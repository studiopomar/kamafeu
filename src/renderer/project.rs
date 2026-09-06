use crate::drivers::{ResamplerDriver, WavtoolDriver};
use crate::oto::Voicebank;
use crate::project::model::{UNote, UProject, UTrack};
use std::sync::atomic::{AtomicBool, Ordering};

use rayon::prelude::*;

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
    /// Earliest musical context required to continue notes that cross a
    /// progressive-preview boundary. The result never precedes `floor_ms`, so
    /// playback started in the middle of a note remains internally consistent.
    /// Also includes the immediately preceding note if within a transition window
    /// so that VCV / phonetic transitions are not ignored across chunk/page boundaries.
    pub fn preview_context_start(project: &UProject, boundary_ms: f64, floor_ms: f64) -> f64 {
        let mut min_context = boundary_ms;
        for part in &project.parts {
            for (idx, note) in part.notes.iter().enumerate() {
                let start = part.position_ms + note.position_ms;
                let end = start + note.duration_ms;
                if start < boundary_ms && end > boundary_ms {
                    min_context = min_context.min(start.max(floor_ms));
                } else if start >= boundary_ms && start <= boundary_ms + 500.0 {
                    if idx > 0 {
                        let prev_note = &part.notes[idx - 1];
                        let prev_start = part.position_ms + prev_note.position_ms;
                        let prev_end = prev_start + prev_note.duration_ms;
                        if start - prev_end < 400.0 {
                            min_context = min_context.min(prev_start.max(floor_ms));
                        }
                    }
                }
            }
        }
        min_context
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_project_with_drivers(
        project: &UProject,
        voicebank: &Voicebank,
        sample_rate: u32,
        start_ms: f64,
        resampler_driver: &dyn ResamplerDriver,
        wavtool_driver: &dyn WavtoolDriver,
        options: &RenderOptions,
        on_progress: Option<&(dyn Fn(f32, &str) + Send + Sync)>,
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
        on_progress: Option<&(dyn Fn(f32, &str) + Send + Sync)>,
        cancel: Option<&AtomicBool>,
    ) -> RenderedAudio {
        Self::render_project_range_with_drivers_cancellable(
            project,
            voicebank,
            sample_rate,
            start_ms,
            None,
            resampler_driver,
            wavtool_driver,
            options,
            on_progress,
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
        on_progress: Option<&(dyn Fn(f32, &str) + Send + Sync)>,
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

        let track_count = audible_tracks.len();

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
            mono: Vec<f32>,
        }

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

                let mono = TrackRenderer::render_track_with_progress_cancellable(
                    &notes,
                    voicebank,
                    sample_rate,
                    project.bpm,
                    resampler_driver,
                    wavtool_driver,
                    Some(options),
                    Some(&track_cb),
                    cancel,
                );

                Some(TrackResult {
                    _track_index: track_index,
                    audible_index,
                    track_name: track.name.clone(),
                    volume_db: track.volume_db,
                    pan: track.pan,
                    mono,
                })
            })
            .collect();

        if cancel.is_some_and(|token| token.load(Ordering::Relaxed)) {
            return RenderedAudio::empty(sample_rate, 2);
        }

        // Sort by audible_index so mixing order is deterministic.
        track_results.sort_unstable_by_key(|r| r.audible_index);

        let mut stereo = Vec::<f32>::new();

        for result in track_results {
            if let Some(callback) = on_progress {
                let progress = result.audible_index as f32 / track_count as f32;
                callback(
                    progress,
                    &format!("[{}] Renderização concluída", result.track_name),
                );
            }
            Self::mix_track_into(&mut stereo, &result.mono, result.volume_db, result.pan);
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

        for sample in &mut stereo {
            *sample = sample.clamp(-1.0, 1.0);
        }

        if let Some(callback) = on_progress {
            callback(1.0, "[Mixer] Renderização multifaixa concluída");
        }

        if let Some(end_ms) = end_ms {
            let requested_frames =
                (((end_ms - start_ms).max(0.0) / 1000.0) * sample_rate as f64).round() as usize;
            stereo.truncate(requested_frames.saturating_mul(2));
            stereo.resize(requested_frames.saturating_mul(2), 0.0);
        }

        RenderedAudio {
            samples: stereo,
            sample_rate,
            channels: 2,
        }
    }

    fn notes_for_track(
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

    #[allow(clippy::too_many_arguments)]
    fn mix_wave_part_into(
        stereo: &mut Vec<f32>,
        decoded: &crate::audio::DecodedAudio,
        sample_rate: u32,
        wave_position_ms: f64,
        wave_file_offset_ms: f64,
        start_ms: f64,
        volume_db: f64,
        pan: f64,
    ) {
        if decoded.samples.is_empty() || decoded.sample_rate == 0 || decoded.channels == 0 {
            return;
        }

        let decoded_stereo = decoded.to_stereo_at_sample_rate(sample_rate);
        let src_frames = decoded_stereo.len() / 2;
        if src_frames == 0 {
            return;
        }

        let gain = 10.0f32.powf((volume_db / 20.0) as f32);
        let pan = pan.clamp(-1.0, 1.0) as f32;
        let left_gain = gain * (1.0 - pan.max(0.0));
        let right_gain = gain * (1.0 + pan.min(0.0));

        let offset_ms = wave_position_ms - start_ms;
        let file_offset_frames =
            ((wave_file_offset_ms / 1000.0) * sample_rate as f64).round() as isize;

        let dest_start_frame = if offset_ms >= 0.0 {
            ((offset_ms / 1000.0) * sample_rate as f64).round() as usize
        } else {
            0
        };

        let src_start_frame = if offset_ms >= 0.0 {
            file_offset_frames.max(0) as usize
        } else {
            (file_offset_frames + ((-offset_ms / 1000.0) * sample_rate as f64).round() as isize)
                .max(0) as usize
        };

        if src_start_frame >= src_frames {
            return;
        }

        let frames_to_mix = src_frames - src_start_frame;
        let required_len = (dest_start_frame + frames_to_mix) * 2;
        if stereo.len() < required_len {
            stereo.resize(required_len, 0.0);
        }

        for i in 0..frames_to_mix {
            let src_f = src_start_frame + i;
            let dest_f = dest_start_frame + i;
            let l = decoded_stereo[src_f * 2];
            let r = decoded_stereo[src_f * 2 + 1];
            stereo[dest_f * 2] += l * left_gain;
            stereo[dest_f * 2 + 1] += r * right_gain;
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

    #[test]
    fn preview_range_only_selects_notes_needed_by_the_chunk() {
        let mut project = UProject::default();
        project.parts[0].notes = vec![
            UNote::new("before", "C4", 0.0, 500.0),
            UNote::new("crossing", "C4", 900.0, 300.0),
            UNote::new("inside", "C4", 1_500.0, 300.0),
            UNote::new("after", "C4", 5_100.0, 300.0),
        ];

        let notes = ProjectRenderer::notes_for_track(&project, 0, 1_000.0, Some(5_000.0));
        assert_eq!(notes.len(), 2);
        assert_eq!(notes[0].lyric, "crossing");
        assert_eq!(notes[0].position_ms, 0.0);
        assert_eq!(notes[0].duration_ms, 200.0);
        assert_eq!(notes[1].lyric, "inside");
        assert_eq!(notes[1].position_ms, 500.0);
    }

    #[test]
    fn full_preview_is_independent_of_a_distant_viewport() {
        let mut project = UProject::default();
        project.parts[0].notes = vec![
            UNote::new("start", "C4", 0.0, 500.0),
            UNote::new("middle", "D4", 12_000.0, 500.0),
            UNote::new("distant", "E4", 24_000.0, 500.0),
        ];

        let notes = ProjectRenderer::notes_for_track(&project, 0, 0.0, Some(25_000.0));
        assert_eq!(notes.len(), 3);
        assert_eq!(notes[0].lyric, "start");
        assert_eq!(notes[2].lyric, "distant");
        assert_eq!(notes[2].position_ms, 24_000.0);
    }

    #[test]
    fn progressive_preview_keeps_context_for_long_notes() {
        let mut project = UProject::default();
        project.parts[0].notes = vec![
            UNote::new("long", "C4", 1_000.0, 10_000.0),
            UNote::new("later", "D4", 12_000.0, 500.0),
        ];

        assert_eq!(
            ProjectRenderer::preview_context_start(&project, 4_000.0, 0.0),
            1_000.0
        );
        assert_eq!(
            ProjectRenderer::preview_context_start(&project, 8_000.0, 5_000.0),
            5_000.0
        );
        assert_eq!(
            ProjectRenderer::preview_context_start(&project, 12_000.0, 0.0),
            12_000.0
        );
    }

    #[test]
    fn test_mix_wave_part() {
        let mut stereo = Vec::new();
        let decoded = crate::audio::DecodedAudio {
            samples: vec![1.0, 1.0, 0.5, 0.5], // 2 stereo frames at 44100
            sample_rate: 44100,
            channels: 2,
            duration_ms: 2.0 / 44100.0 * 1000.0,
        };
        ProjectRenderer::mix_wave_part_into(&mut stereo, &decoded, 44100, 0.0, 0.0, 0.0, 0.0, 0.0);
        assert_eq!(stereo.len(), 4);
        assert!((stereo[0] - 1.0).abs() < 1e-4);
        assert!((stereo[1] - 1.0).abs() < 1e-4);
        assert!((stereo[2] - 0.5).abs() < 1e-4);
        assert!((stereo[3] - 0.5).abs() < 1e-4);
    }
}
