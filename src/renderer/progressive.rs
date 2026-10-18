use crate::drivers::{ResamplerDriver, WavtoolDriver};
use crate::oto::Voicebank;
use crate::project::model::UProject;
use std::sync::atomic::{AtomicBool, Ordering};

use super::project::{ProgressiveChunk, ProjectRenderer, RenderedAudio};
use super::{RenderOptions, RenderProgressCallback};

#[cfg(not(target_arch = "wasm32"))]
fn debug_render_chunk_delay() {
    if let Some(delay_ms) = std::env::var("KAMAFEU_DEBUG_RENDER_CHUNK_DELAY_MS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
    {
        std::thread::sleep(std::time::Duration::from_millis(delay_ms));
    }
}

#[cfg(target_arch = "wasm32")]
fn debug_render_chunk_delay() {}

impl ProjectRenderer {
    /// Render the project in progressive chunks and send each through `chunk_tx`.
    ///
    /// The first chunk is rendered before the rest of the project. This is
    /// important for transport responsiveness: synthesizing a distant verse
    /// must not delay audio at the playhead. Once that priority chunk has been
    /// sent, the complete render provides the following chunks with the same
    /// phrase context as export.
    #[allow(clippy::too_many_arguments)]
    pub fn retry_progressive_chunk(
        project: &UProject,
        voicebank: &Voicebank,
        sample_rate: u32,
        chunk_index: usize,
        start_ms: f64,
        end_ms: f64,
        is_final: bool,
        resampler_driver: &dyn ResamplerDriver,
        wavtool_driver: &dyn WavtoolDriver,
        options: &RenderOptions,
        on_progress: Option<&RenderProgressCallback<'_>>,
        cancel: Option<&AtomicBool>,
    ) -> ProgressiveChunk {
        Self::retry_progressive_chunk_with_progress_events(
            project,
            voicebank,
            sample_rate,
            chunk_index,
            start_ms,
            end_ms,
            is_final,
            resampler_driver,
            wavtool_driver,
            options,
            on_progress,
            None,
            cancel,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn retry_progressive_chunk_with_progress_events(
        project: &UProject,
        voicebank: &Voicebank,
        sample_rate: u32,
        chunk_index: usize,
        start_ms: f64,
        end_ms: f64,
        is_final: bool,
        resampler_driver: &dyn ResamplerDriver,
        wavtool_driver: &dyn WavtoolDriver,
        options: &RenderOptions,
        on_progress: Option<&RenderProgressCallback<'_>>,
        on_progress_event: Option<&(dyn Fn(super::RenderProgressEvent) + Send + Sync)>,
        cancel: Option<&AtomicBool>,
    ) -> ProgressiveChunk {
        let start_ms = start_ms.max(0.0);
        let end_ms = end_ms.max(start_ms);
        let context_start = Self::preview_context_start(project, start_ms, 0.0);
        let rendered = Self::render_project_range_with_progress_events_cancellable(
            project,
            voicebank,
            sample_rate,
            context_start,
            Some(end_ms),
            resampler_driver,
            wavtool_driver,
            options,
            on_progress,
            on_progress_event,
            cancel,
        );
        if rendered.error.is_some() {
            return ProgressiveChunk {
                audio: rendered,
                chunk_start_ms: start_ms,
                chunk_index,
                is_final,
            };
        }
        let channels = usize::from(rendered.channels.max(1));
        let offset = (((start_ms - context_start).max(0.0) / 1000.0) * sample_rate as f64).round()
            as usize
            * channels;
        let requested =
            (((end_ms - start_ms) / 1000.0) * sample_rate as f64).round() as usize * channels;
        let mut samples = vec![0.0; requested];
        if offset < rendered.samples.len() {
            let available = samples.len().min(rendered.samples.len() - offset);
            samples[..available].copy_from_slice(&rendered.samples[offset..offset + available]);
        }
        ProgressiveChunk {
            audio: RenderedAudio {
                samples,
                sample_rate,
                channels: rendered.channels,
                error: None,
            },
            chunk_start_ms: start_ms,
            chunk_index,
            is_final,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_project_progressive(
        project: &UProject,
        voicebank: &Voicebank,
        sample_rate: u32,
        playhead_ms: f64,
        max_end_ms: f64,
        chunk_ms: f64,
        resampler_driver: &dyn ResamplerDriver,
        wavtool_driver: &dyn WavtoolDriver,
        options: &RenderOptions,
        on_progress: Option<&RenderProgressCallback<'_>>,
        cancel: Option<&AtomicBool>,
        chunk_tx: &std::sync::mpsc::SyncSender<ProgressiveChunk>,
    ) {
        let chunk_ms = chunk_ms.max(500.0);
        let mut cursor = playhead_ms.max(0.0);
        if cursor >= max_end_ms {
            return;
        }
        let report_chunk_diagnostics = |audio: &RenderedAudio, start_ms: f64| {
            if let Some(callback) = on_progress {
                callback(
                    0.0,
                    &crate::renderer::AudioDiagnostics::analyze(&audio.samples, audio.channels)
                        .render_log_line(&format!("prévia {:.0}ms", start_ms)),
                );
            }
        };

        // Include notes needed to establish an active note or a VCV transition
        // at the playhead, but do not make the first audible chunk wait for
        // unrelated notes elsewhere in the project.
        let first_end = (cursor + chunk_ms).min(max_end_ms);
        let context_start = Self::preview_context_start(project, cursor, 0.0);
        let priority_audio = Self::render_project_range_with_drivers_cancellable(
            project,
            voicebank,
            sample_rate,
            context_start,
            Some(first_end),
            resampler_driver,
            wavtool_driver,
            options,
            on_progress,
            cancel,
        );
        if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
            return;
        }
        if priority_audio.error.is_some() {
            let _ = chunk_tx.send(ProgressiveChunk {
                audio: priority_audio,
                chunk_start_ms: cursor,
                chunk_index: 0,
                is_final: true,
            });
            return;
        }

        let channels = usize::from(priority_audio.channels.max(1));
        let priority_offset = (((cursor - context_start).max(0.0) / 1000.0) * sample_rate as f64)
            .round() as usize
            * channels;
        let requested_samples =
            (((first_end - cursor) / 1000.0) * sample_rate as f64).round() as usize * channels;
        let mut priority_samples = vec![0.0; requested_samples];
        if priority_offset < priority_audio.samples.len() {
            let available = priority_samples
                .len()
                .min(priority_audio.samples.len() - priority_offset);
            priority_samples[..available].copy_from_slice(
                &priority_audio.samples[priority_offset..priority_offset + available],
            );
        }
        let priority_chunk = RenderedAudio {
            samples: priority_samples,
            sample_rate,
            channels: priority_audio.channels,
            error: None,
        };
        report_chunk_diagnostics(&priority_chunk, cursor);
        debug_render_chunk_delay();
        if chunk_tx
            .send(ProgressiveChunk {
                audio: priority_chunk,
                chunk_start_ms: cursor,
                chunk_index: 0,
                is_final: first_end >= max_end_ms,
            })
            .is_err()
        {
            return;
        }
        if first_end >= max_end_ms {
            return;
        }

        // Render each following page immediately instead of completing the
        // entire project before queuing page two. The old path could exhaust
        // the two-second sink buffer and create silence / apparently eaten
        // phonemes while a distant verse was still rendering.
        cursor = first_end;
        let mut chunk_index = 1;
        while cursor < max_end_ms {
            if cancel.is_some_and(|c| c.load(Ordering::Relaxed)) {
                return;
            }
            let end = (cursor + chunk_ms).min(max_end_ms);
            let context_start = Self::preview_context_start(project, cursor, 0.0);
            let page_audio = Self::render_project_range_with_drivers_cancellable(
                project,
                voicebank,
                sample_rate,
                context_start,
                Some(end),
                resampler_driver,
                wavtool_driver,
                options,
                on_progress,
                cancel,
            );
            if page_audio.error.is_some() {
                let _ = chunk_tx.send(ProgressiveChunk {
                    audio: page_audio,
                    chunk_start_ms: cursor,
                    chunk_index,
                    is_final: true,
                });
                return;
            }
            let channels = usize::from(page_audio.channels.max(1));
            let offset = (((cursor - context_start).max(0.0) / 1000.0) * sample_rate as f64).round()
                as usize
                * channels;
            let requested =
                (((end - cursor) / 1000.0) * sample_rate as f64).round() as usize * channels;
            let mut samples = vec![0.0; requested];
            if offset < page_audio.samples.len() {
                let available = samples.len().min(page_audio.samples.len() - offset);
                samples[..available]
                    .copy_from_slice(&page_audio.samples[offset..offset + available]);
            }
            let page_chunk = RenderedAudio {
                samples,
                sample_rate,
                channels: page_audio.channels,
                error: None,
            };
            report_chunk_diagnostics(&page_chunk, cursor);
            debug_render_chunk_delay();
            if chunk_tx
                .send(ProgressiveChunk {
                    audio: page_chunk,
                    chunk_start_ms: cursor,
                    chunk_index,
                    is_final: end >= max_end_ms,
                })
                .is_err()
            {
                return;
            }
            chunk_index += 1;
            cursor = end;
        }
    }
}
