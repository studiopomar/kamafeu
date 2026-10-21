use crate::drivers::{ResamplerDriver, WavtoolDriver};
use crate::oto::Voicebank;
use crate::project::model::{UNote, UProject, UTrack};
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

use super::project::{configured_render_limiter_peak, ProjectRenderer, RenderedAudio};
use super::{RenderOptions, RenderProgressCallback, TrackRenderer};

impl ProjectRenderer {
    #[allow(clippy::too_many_arguments)]
    pub fn render_project_with_drivers(
        project: &UProject,
        voicebank: &Voicebank,
        sample_rate: u32,
        start_ms: f64,
        resampler_driver: &dyn ResamplerDriver,
        wavtool_driver: &dyn WavtoolDriver,
        options: &RenderOptions,
        on_progress: Option<&RenderProgressCallback<'_>>,
    ) -> RenderedAudio {
        Self::render_project_with_drivers_cancellable(
            project,
            voicebank,
            sample_rate,
            start_ms,
            resampler_driver,
            wavtool_driver,
            options,
            on_progress,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_project_with_drivers_cancellable(
        project: &UProject,
        voicebank: &Voicebank,
        sample_rate: u32,
        start_ms: f64,
        resampler_driver: &dyn ResamplerDriver,
        wavtool_driver: &dyn WavtoolDriver,
        options: &RenderOptions,
        on_progress: Option<&RenderProgressCallback<'_>>,
        cancel: Option<&AtomicBool>,
    ) -> RenderedAudio {
        Self::render_project_with_progress_events_cancellable(
            project,
            voicebank,
            sample_rate,
            start_ms,
            resampler_driver,
            wavtool_driver,
            options,
            on_progress,
            None,
            cancel,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_project_with_progress_events_cancellable(
        project: &UProject,
        voicebank: &Voicebank,
        sample_rate: u32,
        start_ms: f64,
        resampler_driver: &dyn ResamplerDriver,
        wavtool_driver: &dyn WavtoolDriver,
        options: &RenderOptions,
        on_progress: Option<&RenderProgressCallback<'_>>,
        on_progress_event: Option<&(dyn Fn(super::RenderProgressEvent) + Send + Sync)>,
        cancel: Option<&AtomicBool>,
    ) -> RenderedAudio {
        Self::render_project_range_with_progress_events_cancellable(
            project,
            voicebank,
            sample_rate,
            start_ms,
            None,
            resampler_driver,
            wavtool_driver,
            options,
            on_progress,
            on_progress_event,
            cancel,
        )
    }

    /// Render the project interval beginning at `start_ms`. When present,
    /// `end_ms` bounds both note selection and the exact output duration.
    #[allow(clippy::too_many_arguments)]
    pub fn render_project_range_with_drivers_cancellable(
        project: &UProject,
        voicebank: &Voicebank,
        sample_rate: u32,
        start_ms: f64,
        end_ms: Option<f64>,
        resampler_driver: &dyn ResamplerDriver,
        wavtool_driver: &dyn WavtoolDriver,
        options: &RenderOptions,
        on_progress: Option<&RenderProgressCallback<'_>>,
        cancel: Option<&AtomicBool>,
    ) -> RenderedAudio {
        Self::render_project_range_with_progress_events_cancellable(
            project,
            voicebank,
            sample_rate,
            start_ms,
            end_ms,
            resampler_driver,
            wavtool_driver,
            options,
            on_progress,
            None,
            cancel,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_project_range_with_progress_events_cancellable(
        project: &UProject,
        voicebank: &Voicebank,
        sample_rate: u32,
        start_ms: f64,
        end_ms: Option<f64>,
        resampler_driver: &dyn ResamplerDriver,
        wavtool_driver: &dyn WavtoolDriver,
        options: &RenderOptions,
        on_progress: Option<&RenderProgressCallback<'_>>,
        on_progress_event: Option<&(dyn Fn(super::RenderProgressEvent) + Send + Sync)>,
        cancel: Option<&AtomicBool>,
    ) -> RenderedAudio {
        if let Some(callback) = on_progress_event {
            callback(super::RenderProgressEvent::stage(
                super::RenderStage::Preparing,
            ));
        }
        let any_solo = project.tracks.iter().any(|track| track.solo);
        let audible_tracks: Vec<(usize, &UTrack)> = project
            .tracks
            .iter()
            .enumerate()
            .filter(|(_, track)| !track.mute && (!any_solo || track.solo))
            .collect();

        let native_track_indices: Vec<usize> = audible_tracks
            .iter()
            .filter_map(|(index, track)| track.native_voicebank.as_ref().map(|_| *index))
            .collect();
        if !native_track_indices.is_empty() {
            if cancel.is_some_and(|token| token.load(Ordering::Relaxed)) {
                return RenderedAudio::empty(sample_rate, 2);
            }
            if let Some(callback) = on_progress {
                callback(0.0, "Resolvendo bancos nativos e bindings do projeto");
            }
            let classic_tracks_with_notes = audible_tracks
                .iter()
                .filter(|(index, track)| {
                    track.native_voicebank.is_none()
                        && project
                            .parts
                            .iter()
                            .any(|part| part.track_index == *index && !part.notes.is_empty())
                })
                .map(|(_, track)| track.name.as_str())
                .collect::<Vec<_>>();
            if !classic_tracks_with_notes.is_empty() {
                if let Some(callback) = on_progress {
                    callback(
                        0.0,
                        &format!(
                            "[Mistura UTAU] faixas {} também serão renderizadas com o banco UTAU carregado: {} ({})",
                            classic_tracks_with_notes.join(", "),
                            voicebank.name,
                            voicebank.root_path.display()
                        ),
                    );
                }
            }
            if let Some(callback) = on_progress_event {
                callback(super::RenderProgressEvent::counted(
                    super::RenderStage::Rendering,
                    0,
                    native_track_indices.len(),
                ));
            }

            // Keep the established classic voicebank renderer responsible for these tracks,
            // while muting native tracks in this clone so their notes cannot be
            // phonemized or rendered through a classic voicebank by accident.
            let mut classic_project = project.clone();
            for &index in &native_track_indices {
                if let Some(track) = classic_project.tracks.get_mut(index) {
                    track.native_voicebank = None;
                    track.mute = true;
                }
            }
            let mut mixed = Self::render_project_range_with_progress_events_cancellable(
                &classic_project,
                voicebank,
                sample_rate,
                start_ms,
                end_ms,
                resampler_driver,
                wavtool_driver,
                options,
                on_progress,
                on_progress_event,
                cancel,
            );
            if mixed.error.is_some() {
                return mixed;
            }

            let mut native_frames = 0usize;
            for (native_index, &track_index) in native_track_indices.iter().enumerate() {
                if cancel.is_some_and(|token| token.load(Ordering::Relaxed)) {
                    return RenderedAudio::empty(sample_rate, 2);
                }
                let track = &project.tracks[track_index];
                let binding = track
                    .native_voicebank
                    .as_ref()
                    .expect("native track index came from a present binding");
                let Some(bank_path) = binding
                    .source_path_hint
                    .as_deref()
                    .or(track.voicebank_path.as_deref())
                else {
                    return RenderedAudio::failed(
                        sample_rate,
                        format!(
                            "faixa '{}': cantor nativo sem caminho local; reinstale ou relocalize o banco",
                            track.name
                        ),
                    );
                };
                let cantor_bank = match crate::voicebank::cantor::CantorBank::load_path(
                    std::path::Path::new(bank_path),
                ) {
                    Ok(bank) => bank,
                    Err(error) => {
                        return RenderedAudio::failed(
                            sample_rate,
                            format!(
                                "faixa '{}': não foi possível abrir cantor.toml: {error}",
                                track.name
                            ),
                        );
                    }
                };
                let source_recordings = cantor_bank
                    .cantor
                    .tons
                    .iter()
                    .flat_map(|tone| {
                        tone.gravacoes.iter().map(move |recording| {
                            format!("{} / {}: {}", tone.id, recording.id, recording.arquivo)
                        })
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                let singer_name = cantor_bank.cantor.nome.clone();
                let bank = match cantor_bank.native_snapshot() {
                    Ok(bank) => bank,
                    Err(error) => {
                        return RenderedAudio::failed(
                            sample_rate,
                            format!(
                                "faixa '{}': falha ao preparar áudio de {}: {error}",
                                track.name, singer_name
                            ),
                        );
                    }
                };
                if let Some(callback) = on_progress {
                    callback(
                        native_index as f32 / native_track_indices.len() as f32,
                        &format!(
                            "[KAMUSE / Tainah WORLD] faixa {} · cantor {} · {}/cantor.toml · fontes originais: {}",
                            track.name,
                            singer_name,
                            bank_path,
                            source_recordings
                        ),
                    );
                }
                let rendered = match Self::render_native_track_world_cancellable(
                    project,
                    &bank,
                    track_index,
                    sample_rate,
                    cancel,
                ) {
                    Ok(rendered) => rendered,
                    Err(error) => {
                        if cancel.is_some_and(|token| token.load(Ordering::Relaxed)) {
                            return RenderedAudio::empty(sample_rate, 2);
                        }
                        return RenderedAudio::failed(
                            sample_rate,
                            format!(
                                "faixa '{}': render nativo Tainah WORLD falhou: {error}",
                                track.name
                            ),
                        );
                    }
                };
                if let Some(callback) = on_progress {
                    let decisions = rendered
                        .phrase
                        .plan
                        .units
                        .iter()
                        .take(6)
                        .map(|unit| {
                            let reason = match unit.reason {
                                crate::voicebank::planner::SelectionReason::Override => "override",
                                crate::voicebank::planner::SelectionReason::ExactContext => {
                                    "contexto exato"
                                }
                                crate::voicebank::planner::SelectionReason::WildcardContext => {
                                    "contexto curinga"
                                }
                            };
                            format!(
                                "{} → {} / {} (MIDI pedido {}, gravação {}, {})",
                                unit.phones.join(" "),
                                unit.unit_id,
                                unit.layer_id,
                                unit.requested_midi,
                                unit.selected_midi,
                                reason
                            )
                        })
                        .collect::<Vec<_>>();
                    let omitted = rendered
                        .phrase
                        .plan
                        .units
                        .len()
                        .saturating_sub(decisions.len());
                    let warnings = if rendered.phrase.warnings.is_empty() {
                        String::new()
                    } else {
                        format!("; avisos: {}", rendered.phrase.warnings.join(" "))
                    };
                    let suffix = if omitted > 0 {
                        format!("; mais {omitted} decisões no plano")
                    } else {
                        String::new()
                    };
                    callback(
                        (native_index + 1) as f32 / native_track_indices.len() as f32,
                        &format!(
                            "[Tainah WORLD][{}] amostras: {}{}",
                            track.name,
                            decisions.join("; "),
                            format!("{suffix}{warnings}")
                        ),
                    );
                }

                let source_start =
                    ((start_ms.max(0.0) * sample_rate as f64) / 1000.0).round() as usize;
                let source_end = end_ms
                    .map(|end| ((end.max(start_ms) * sample_rate as f64) / 1000.0).round() as usize)
                    .unwrap_or_else(|| rendered.audio.frame_count());
                let source_end = source_end.min(rendered.audio.frame_count());
                native_frames = native_frames.max(source_end.saturating_sub(source_start));
                if mixed.samples.len() / 2 < native_frames {
                    mixed.samples.resize(native_frames.saturating_mul(2), 0.0);
                }
                for frame in source_start..source_end {
                    if cancel.is_some_and(|token| token.load(Ordering::Relaxed)) {
                        return RenderedAudio::empty(sample_rate, 2);
                    }
                    let output_frame = frame - source_start;
                    let source_offset = frame.saturating_mul(2);
                    let output_offset = output_frame.saturating_mul(2);
                    if let (Some(source), Some(output)) = (
                        rendered.audio.samples.get(source_offset..source_offset + 2),
                        mixed.samples.get_mut(output_offset..output_offset + 2),
                    ) {
                        output[0] += source[0];
                        output[1] += source[1];
                    }
                }
                if let Some(callback) = on_progress_event {
                    callback(super::RenderProgressEvent::counted(
                        super::RenderStage::Rendering,
                        native_index + 1,
                        native_track_indices.len(),
                    ));
                }
            }

            if let Some(end_ms) = end_ms {
                let requested_frames =
                    (((end_ms - start_ms).max(0.0) / 1000.0) * sample_rate as f64).round() as usize;
                mixed.samples.truncate(requested_frames.saturating_mul(2));
                mixed
                    .samples
                    .resize(requested_frames.saturating_mul(2), 0.0);
            }
            if let Some(target_peak) = configured_render_limiter_peak() {
                if let Some(callback) = on_progress_event {
                    callback(super::RenderProgressEvent::stage(
                        super::RenderStage::Finalizing,
                    ));
                }
                TrackRenderer::apply_soft_limiter(&mut mixed.samples, target_peak);
            }
            if let Some(callback) = on_progress {
                callback(
                    1.0,
                    "[Mixer] Renderização nativa WORLD e clássica concluída",
                );
            }
            if let Some(callback) = on_progress_event {
                callback(super::RenderProgressEvent::stage(
                    super::RenderStage::Complete,
                ));
            }
            return mixed;
        }

        if audible_tracks.is_empty() {
            return RenderedAudio::empty(sample_rate, 2);
        }

        let track_count = audible_tracks.len();
        let progress_weights = audible_tracks
            .iter()
            .map(|(track_index, _)| {
                Self::notes_for_track(project, *track_index, start_ms, end_ms).len()
            })
            .collect::<Vec<_>>();
        let active_track_count = progress_weights
            .iter()
            .filter(|weight| **weight > 0)
            .count();
        let progress_aggregator =
            super::progress::TrackProgressAggregator::new(progress_weights.clone());
        if let Some(callback) = on_progress_event {
            callback(super::RenderProgressEvent::counted(
                super::RenderStage::Rendering,
                0,
                active_track_count,
            ));
        }

        // ------------------------------------------------------------------
        // Render all audible tracks in parallel.  Each track produces an
        // independent mono buffer; the results are mixed into `stereo` below.
        // ------------------------------------------------------------------
        struct TrackResult {
            _track_index: usize,
            audible_index: usize,
            track_name: String,
            volume_db: f64,
            pan: f64,
            fx_rack: Option<crate::audio::FxRackConfig>,
            mono: Result<Vec<f32>, String>,
        }

        let report_track_progress = |audible_index: usize, event: super::RenderProgressEvent| {
            if event.stage != super::RenderStage::Rendering {
                return;
            }
            let Some(fraction) = event.fraction else {
                return;
            };
            if let Some(callback) = on_progress_event {
                if let Some(progress) = progress_aggregator.update(audible_index, fraction) {
                    callback(progress);
                }
            }
        };

        #[cfg(not(target_arch = "wasm32"))]
        let mut track_results: Vec<TrackResult> = audible_tracks
            .par_iter()
            .copied()
            .enumerate()
            .filter_map(|(audible_index, (track_index, track))| {
                if cancel.is_some_and(|token| token.load(Ordering::Relaxed)) {
                    return None;
                }
                let notes = Self::notes_for_track(project, track_index, start_ms, end_ms);
                if notes.is_empty() {
                    return None;
                }

                let track_cb = |prog: f32, msg: &str| {
                    if let Some(cb) = on_progress {
                        let total_prog = (audible_index as f32 + prog) / (track_count as f32);
                        cb(total_prog, msg);
                    }
                };
                let track_progress_cb = |event| report_track_progress(audible_index, event);

                let mut track_options = options.clone();
                track_options.flags = crate::drivers::resampler_driver::merge_utau_flags(&[
                    project.flags.as_str(),
                    track.flags.as_str(),
                    options.flags.as_str(),
                ]);

                let mono = TrackRenderer::try_render_track_with_progress_events(
                    &notes,
                    voicebank,
                    sample_rate,
                    project.bpm,
                    resampler_driver,
                    wavtool_driver,
                    Some(&track_options),
                    Some(&track_cb),
                    Some(&track_progress_cb),
                    cancel,
                );

                Some(TrackResult {
                    _track_index: track_index,
                    audible_index,
                    track_name: track.name.clone(),
                    volume_db: track.volume_db,
                    pan: track.pan,
                    fx_rack: track.fx_rack.clone(),
                    mono,
                })
            })
            .collect();

        #[cfg(target_arch = "wasm32")]
        let mut track_results: Vec<TrackResult> = audible_tracks
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(audible_index, (track_index, track))| {
                if cancel.is_some_and(|token| token.load(Ordering::Relaxed)) {
                    return None;
                }
                let notes = Self::notes_for_track(project, track_index, start_ms, end_ms);
                if notes.is_empty() {
                    return None;
                }

                let track_cb = |prog: f32, msg: &str| {
                    if let Some(cb) = on_progress {
                        let total_prog = (audible_index as f32 + prog) / (track_count as f32);
                        cb(total_prog, msg);
                    }
                };
                let track_progress_cb = |event| report_track_progress(audible_index, event);

                let mut track_options = options.clone();
                track_options.flags = crate::drivers::resampler_driver::merge_utau_flags(&[
                    project.flags.as_str(),
                    track.flags.as_str(),
                    options.flags.as_str(),
                ]);

                let mono = TrackRenderer::try_render_track_with_progress_events(
                    &notes,
                    voicebank,
                    sample_rate,
                    project.bpm,
                    resampler_driver,
                    wavtool_driver,
                    Some(&track_options),
                    Some(&track_cb),
                    Some(&track_progress_cb),
                    cancel,
                );

                Some(TrackResult {
                    _track_index: track_index,
                    audible_index,
                    track_name: track.name.clone(),
                    volume_db: track.volume_db,
                    pan: track.pan,
                    fx_rack: track.fx_rack.clone(),
                    mono,
                })
            })
            .collect();

        if cancel.is_some_and(|token| token.load(Ordering::Relaxed)) {
            return RenderedAudio::empty(sample_rate, 2);
        }

        if let Some(error) = track_results.iter().find_map(|r| r.mono.as_ref().err()) {
            if let Some(callback) = on_progress_event {
                callback(super::RenderProgressEvent::stage(
                    super::RenderStage::Failed,
                ));
            }
            if let Some(cb) = on_progress {
                cb(1.0, error);
            }
            return RenderedAudio::failed(sample_rate, error.to_string());
        }

        // Sort by audible_index so mixing order is deterministic.
        track_results.sort_unstable_by_key(|r| r.audible_index);

        let mut stereo = Vec::<f32>::new();
        let mix_total = track_results.len().max(1);
        if let Some(callback) = on_progress_event {
            callback(super::RenderProgressEvent::counted(
                super::RenderStage::Mixing,
                0,
                mix_total,
            ));
        }

        for (mix_index, mut result) in track_results.into_iter().enumerate() {
            if let Some(callback) = on_progress {
                let progress = result.audible_index as f32 / track_count as f32;
                callback(
                    progress,
                    &format!("[{}] Renderização concluída", result.track_name),
                );
            }
            Self::mix_track_into(
                &mut stereo,
                result.mono.as_mut().expect("validated track result"),
                result.volume_db,
                result.pan,
                result.fx_rack.as_ref(),
                sample_rate,
            );
            if let Some(callback) = on_progress_event {
                callback(super::RenderProgressEvent::counted(
                    super::RenderStage::Mixing,
                    mix_index + 1,
                    mix_total,
                ));
            }
        }

        // Mix all audible wave parts (backing tracks / instrumentals)
        for wave in &project.wave_parts {
            if cancel.is_some_and(|token| token.load(Ordering::Relaxed)) {
                break;
            }
            if let Some((_, track)) = audible_tracks
                .iter()
                .find(|(idx, _)| *idx == wave.track_index)
            {
                if let Ok(decoded) = crate::audio::load_audio_file(&wave.file_path) {
                    let total_vol_db = track.volume_db + wave.volume_db;
                    let pan = track.pan;
                    Self::mix_wave_part_into(
                        &mut stereo,
                        &decoded,
                        sample_rate,
                        wave.position_ms,
                        wave.file_offset_ms,
                        start_ms,
                        total_vol_db,
                        pan,
                    );
                }
            }
        }

        if let Some(callback) = on_progress_event {
            callback(super::RenderProgressEvent::stage(
                super::RenderStage::Finalizing,
            ));
        }
        if let Some(target_peak) = configured_render_limiter_peak() {
            TrackRenderer::apply_soft_limiter(&mut stereo, target_peak);
        }

        if let Some(callback) = on_progress {
            callback(1.0, "[Mixer] Renderização multifaixa concluída");
        }
        if let Some(callback) = on_progress_event {
            callback(super::RenderProgressEvent::stage(
                super::RenderStage::Complete,
            ));
        }

        if let Some(end_ms) = end_ms {
            let requested_frames =
                (((end_ms - start_ms).max(0.0) / 1000.0) * sample_rate as f64).round() as usize;
            stereo.truncate(requested_frames.saturating_mul(2));
            stereo.resize(requested_frames.saturating_mul(2), 0.0);
        }

        RenderedAudio {
            error: None,
            samples: stereo,
            sample_rate,
            channels: 2,
        }
    }

    pub(super) fn notes_for_track(
        project: &UProject,
        track_index: usize,
        start_ms: f64,
        end_ms: Option<f64>,
    ) -> Vec<UNote> {
        let mut raw_track_notes = Vec::new();

        for part in project
            .parts
            .iter()
            .filter(|part| part.track_index == track_index)
        {
            for note in &part.notes {
                let mut shifted = note.clone();
                shifted.position_ms += part.position_ms;
                raw_track_notes.push(shifted);
            }
        }

        raw_track_notes.sort_by(|left, right| left.position_ms.total_cmp(&right.position_ms));

        let mut notes = Vec::new();
        for shifted in raw_track_notes {
            let note_end = shifted.position_ms + shifted.duration_ms;
            if note_end <= start_ms || end_ms.is_some_and(|end| shifted.position_ms >= end) {
                continue;
            }

            let mut n = shifted;
            if n.position_ms < start_ms {
                n.duration_ms -= start_ms - n.position_ms;
                n.position_ms = 0.0;
            } else {
                n.position_ms -= start_ms;
            }
            notes.push(n);
        }

        notes
    }
}
