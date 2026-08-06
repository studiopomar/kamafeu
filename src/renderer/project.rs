use crate::drivers::{ResamplerDriver, WavtoolDriver};
use crate::oto::Voicebank;
use crate::project::model::{UNote, UProject, UTrack};
use std::sync::atomic::{AtomicBool, Ordering};

use super::{RenderOptions, TrackRenderer};

#[derive(Debug, Clone)]
pub struct RenderedAudio {
    /// Interleaved PCM samples.
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
}

impl RenderedAudio {
    pub fn empty(sample_rate: u32, channels: u16) -> Self {
        Self {
            samples: Vec::new(),
            sample_rate,
            channels,
        }
    }

    pub fn frame_count(&self) -> usize {
        self.samples.len() / usize::from(self.channels.max(1))
    }
}

pub struct ProjectRenderer;

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
        on_progress: Option<&dyn Fn(f32, &str)>,
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
        on_progress: Option<&dyn Fn(f32, &str)>,
        cancel: Option<&AtomicBool>,
    ) -> RenderedAudio {
        let any_solo = project.tracks.iter().any(|track| track.solo);
        let audible_tracks: Vec<(usize, &UTrack)> = project
            .tracks
            .iter()
            .enumerate()
            .filter(|(_, track)| !track.mute && (!any_solo || track.solo))
            .collect();

        if audible_tracks.is_empty() {
            return RenderedAudio::empty(sample_rate, 2);
        }

        let mut stereo = Vec::<f32>::new();
        let track_count = audible_tracks.len();

        for (audible_index, (track_index, track)) in audible_tracks.into_iter().enumerate() {
            if cancel.is_some_and(|token| token.load(Ordering::Relaxed)) {
                return RenderedAudio::empty(sample_rate, 2);
            }
            let notes = Self::notes_for_track(project, track_index, start_ms);
            if notes.is_empty() {
                continue;
            }

            let base_progress = audible_index as f32 / track_count as f32;
            let progress_span = 1.0 / track_count as f32;
            let track_progress = |progress: f32, message: &str| {
                if let Some(callback) = on_progress {
                    let prefixed = format!("[{}] {}", track.name, message);
                    callback(base_progress + progress * progress_span, &prefixed);
                }
            };

            let mono = TrackRenderer::render_track_with_progress_cancellable(
                &notes,
                voicebank,
                sample_rate,
                project.bpm,
                resampler_driver,
                wavtool_driver,
                Some(options),
                Some(&track_progress),
                cancel,
            );

            if cancel.is_some_and(|token| token.load(Ordering::Relaxed)) {
                return RenderedAudio::empty(sample_rate, 2);
            }

            Self::mix_track_into(&mut stereo, &mono, track.volume_db, track.pan);
        }

        for sample in &mut stereo {
            *sample = sample.clamp(-1.0, 1.0);
        }

        if let Some(callback) = on_progress {
            callback(1.0, "[Mixer] Renderização multifaixa concluída");
        }

        RenderedAudio {
            samples: stereo,
            sample_rate,
            channels: 2,
        }
    }

    fn notes_for_track(project: &UProject, track_index: usize, start_ms: f64) -> Vec<UNote> {
        let mut notes = Vec::new();

        for part in project
            .parts
            .iter()
            .filter(|part| part.track_index == track_index)
        {
            for note in &part.notes {
                let mut shifted = note.clone();
                shifted.position_ms += part.position_ms;
                let note_end = shifted.position_ms + shifted.duration_ms;
                if note_end <= start_ms {
                    continue;
                }

                if shifted.position_ms < start_ms {
                    shifted.duration_ms -= start_ms - shifted.position_ms;
                    shifted.position_ms = 0.0;
                } else {
                    shifted.position_ms -= start_ms;
                }
                notes.push(shifted);
            }
        }

        notes.sort_by(|left, right| left.position_ms.total_cmp(&right.position_ms));
        notes
    }

    fn mix_track_into(stereo: &mut Vec<f32>, mono: &[f32], volume_db: f64, pan: f64) {
        let required_len = mono.len().saturating_mul(2);
        if stereo.len() < required_len {
            stereo.resize(required_len, 0.0);
        }

        let gain = 10.0f32.powf((volume_db / 20.0) as f32);
        let pan = pan.clamp(-1.0, 1.0) as f32;
        let left_gain = gain * (1.0 - pan.max(0.0));
        let right_gain = gain * (1.0 + pan.min(0.0));

        for (frame, sample) in mono.iter().enumerate() {
            stereo[frame * 2] += sample * left_gain;
            stereo[frame * 2 + 1] += sample * right_gain;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixer_applies_volume_and_pan() {
        let mut stereo = Vec::new();
        ProjectRenderer::mix_track_into(&mut stereo, &[1.0, 0.5], -6.0206, 1.0);

        assert_eq!(stereo.len(), 4);
        assert!(stereo[0].abs() < f32::EPSILON);
        assert!((stereo[1] - 0.5).abs() < 0.001);
        assert!(stereo[2].abs() < f32::EPSILON);
        assert!((stereo[3] - 0.25).abs() < 0.001);
    }
}
