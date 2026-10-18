use crate::drivers::{ResamplerDriver, WavtoolDriver};
use crate::oto::Voicebank;
use crate::project::model::UProject;
use std::sync::atomic::AtomicBool;

use super::project::{ProjectRenderer, RenderedAudio};
use super::{RenderOptions, RenderProgressCallback};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PreviewTarget {
    Range {
        start_ms: f64,
        end_ms: f64,
    },
    Note {
        part_index: usize,
        note_index: usize,
    },
    Phoneme {
        part_index: usize,
        note_index: usize,
        phoneme_index: usize,
    },
    Transition {
        part_index: usize,
        left_note_index: usize,
        right_note_index: usize,
    },
}

impl ProjectRenderer {
    /// Resolves an isolated preview target to an absolute project interval.
    /// This is geometry-only and intentionally does not invoke a phonemizer or
    /// engine, so every frontend can present the same selection before audio
    /// work begins.
    pub fn preview_bounds(project: &UProject, target: PreviewTarget) -> Option<(f64, f64)> {
        let bounds = |part_index: usize, note_index: usize| {
            let part = project.parts.get(part_index)?;
            let note = part.notes.get(note_index)?;
            let start = part.position_ms + note.position_ms;
            Some((start, start + note.duration_ms.max(0.0)))
        };

        match target {
            PreviewTarget::Range { start_ms, end_ms }
                if start_ms.is_finite() && end_ms.is_finite() && end_ms > start_ms =>
            {
                Some((start_ms.max(0.0), end_ms.max(0.0)))
            }
            PreviewTarget::Range { .. } => None,
            PreviewTarget::Note {
                part_index,
                note_index,
            } => bounds(part_index, note_index),
            PreviewTarget::Phoneme {
                part_index,
                note_index,
                phoneme_index,
            } => {
                let part = project.parts.get(part_index)?;
                let note = part.notes.get(note_index)?;
                let (note_start, note_end) = bounds(part_index, note_index)?;
                let durations =
                    note.resolved_phoneme_durations(note.phoneme_durations_ms.len().max(1));
                if phoneme_index >= durations.len() {
                    return None;
                }
                let offset: f64 = durations[..phoneme_index].iter().sum();
                let start = (note_start + offset).min(note_end);
                Some((start, (start + durations[phoneme_index]).min(note_end)))
            }
            PreviewTarget::Transition {
                part_index,
                left_note_index,
                right_note_index,
            } => {
                let (left_start, left_end) = bounds(part_index, left_note_index)?;
                let (right_start, right_end) = bounds(part_index, right_note_index)?;
                (right_start >= left_start && right_end >= left_end)
                    .then_some((left_start.min(right_start), left_end.max(right_end)))
            }
        }
    }

    /// Renders only the requested preview target while retaining enough
    /// preceding context for VCV/VCVC transitions. The returned buffer starts
    /// exactly at the target start, never at the internal context boundary.
    #[allow(clippy::too_many_arguments)]
    pub fn render_preview_target_with_drivers(
        project: &UProject,
        voicebank: &Voicebank,
        sample_rate: u32,
        target: PreviewTarget,
        resampler_driver: &dyn ResamplerDriver,
        wavtool_driver: &dyn WavtoolDriver,
        options: &RenderOptions,
        on_progress: Option<&RenderProgressCallback<'_>>,
        cancel: Option<&AtomicBool>,
    ) -> RenderedAudio {
        let Some((start_ms, end_ms)) = Self::preview_bounds(project, target) else {
            return RenderedAudio::failed(sample_rate, "alvo de prévia inválido".to_string());
        };
        let context_start = Self::preview_context_start(project, start_ms, 0.0);
        let rendered = Self::render_project_range_with_drivers_cancellable(
            project,
            voicebank,
            sample_rate,
            context_start,
            Some(end_ms),
            resampler_driver,
            wavtool_driver,
            options,
            on_progress,
            cancel,
        );
        if rendered.error.is_some() {
            return rendered;
        }
        let channels = usize::from(rendered.channels.max(1));
        let offset_frames = (((start_ms - context_start).max(0.0) / 1000.0)
            * f64::from(sample_rate))
        .round() as usize;
        let target_frames =
            (((end_ms - start_ms).max(0.0) / 1000.0) * f64::from(sample_rate)).round() as usize;
        let offset = offset_frames.saturating_mul(channels);
        let length = target_frames.saturating_mul(channels);
        let mut samples = if offset < rendered.samples.len() {
            rendered.samples[offset..]
                .iter()
                .copied()
                .take(length)
                .collect()
        } else {
            Vec::new()
        };
        samples.resize(length, 0.0);
        RenderedAudio {
            samples,
            sample_rate: rendered.sample_rate,
            channels: rendered.channels,
            error: None,
        }
    }

    /// Earliest musical context required to continue notes that cross a
    /// progressive-preview boundary. The result never precedes `floor_ms`, so
    /// playback started in the middle of a note remains internally consistent.
    /// Also includes the complete contiguous phoneme chain preceding the
    /// boundary, so CVVC/VCV aliases are generated exactly as they are in a
    /// full render instead of losing their VC lead at a preview boundary.
    pub fn preview_context_start(project: &UProject, boundary_ms: f64, floor_ms: f64) -> f64 {
        let mut min_context = boundary_ms;
        for part in &project.parts {
            let Some(mut index) = part.notes.iter().position(|note| {
                let start = part.position_ms + note.position_ms;
                let end = start + note.duration_ms;
                end > boundary_ms && start <= boundary_ms + 500.0
            }) else {
                continue;
            };

            // A phonemizer needs the preceding vowel to produce aliases such
            // as `o n`. Walk back through the whole connected phrase rather
            // than retaining only one note, which could make the renderer
            // synthesize a different phoneme sequence during playback.
            while index > 0 {
                let previous = &part.notes[index - 1];
                let current = &part.notes[index];
                let previous_end = part.position_ms + previous.position_ms + previous.duration_ms;
                let current_start = part.position_ms + current.position_ms;
                if current_start - previous_end > 400.0 {
                    break;
                }
                index -= 1;
            }
            let start = part.position_ms + part.notes[index].position_ms;
            min_context = min_context.min(start.max(floor_ms));
        }
        min_context
    }
}
