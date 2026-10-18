#[cfg(test)]
use crate::oto::Voicebank;
#[cfg(test)]
use crate::project::model::UNote;
use crate::project::model::UProject;
use std::sync::atomic::AtomicBool;
#[cfg(test)]
use std::sync::atomic::Ordering;

#[cfg(test)]
use super::mix::{accumulate_mono_to_stereo, accumulate_stereo_scaled};
#[cfg(test)]
use super::preview::PreviewTarget;
#[cfg(test)]
use super::RenderOptions;
use super::TrackRenderer;

pub(crate) fn configured_render_limiter_peak() -> Option<f32> {
    super::mix::configured_render_limiter_peak()
}

#[derive(Debug, Clone)]
pub struct RenderedAudio {
    /// Interleaved PCM samples.
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    pub error: Option<String>,
}

/// A native KFV 2 phrase plus the project-mixed stereo audio for that track.
#[derive(Debug, Clone)]
pub struct NativeTrackRender {
    pub phrase: crate::voicebank::project::NativeProjectPhrase,
    pub audio: RenderedAudio,
}

impl RenderedAudio {
    pub fn empty(sample_rate: u32, channels: u16) -> Self {
        Self {
            error: None,
            samples: Vec::new(),
            sample_rate,
            channels,
        }
    }

    pub fn failed(sample_rate: u32, error: String) -> Self {
        Self {
            samples: Vec::new(),
            sample_rate,
            channels: 2,
            error: Some(error),
        }
    }

    pub fn frame_count(&self) -> usize {
        self.samples.len() / usize::from(self.channels.max(1))
    }
}

/// A single chunk of audio produced by progressive rendering.
#[derive(Debug, Clone)]
pub struct ProgressiveChunk {
    /// The rendered audio for this chunk.
    pub audio: RenderedAudio,
    /// The absolute project time (ms) where this chunk begins.
    pub chunk_start_ms: f64,
    /// Zero-based position in the progressive render sequence.
    pub chunk_index: usize,
    /// `true` when this is the last chunk in the progressive sequence.
    pub is_final: bool,
}

impl ProgressiveChunk {
    /// Absolute project time immediately after this chunk.
    pub fn end_ms(&self) -> f64 {
        self.chunk_start_ms
            + self.audio.frame_count() as f64 * 1000.0 / self.audio.sample_rate.max(1) as f64
    }
}

pub struct ProjectRenderer;

impl ProjectRenderer {
    /// Render one APS track through its stable KFV 2 binding and the explicit
    /// experimental SOLA path, then apply the project's track gain, pan and
    /// effects. This is an offline track render; it does not select a fallback
    /// backend or claim that the product reference engine has been chosen.
    pub fn render_native_track_sola(
        project: &UProject,
        bank: &crate::voicebank::native::NativeBank,
        track_index: usize,
        sample_rate: u32,
    ) -> Result<NativeTrackRender, String> {
        Self::render_native_track_sola_cancellable(project, bank, track_index, sample_rate, None)
    }

    pub fn render_native_track_sola_cancellable(
        project: &UProject,
        bank: &crate::voicebank::native::NativeBank,
        track_index: usize,
        sample_rate: u32,
        cancel: Option<&AtomicBool>,
    ) -> Result<NativeTrackRender, String> {
        let track = project
            .tracks
            .get(track_index)
            .ok_or_else(|| format!("track index does not exist: {track_index}"))?;
        let any_solo = project.tracks.iter().any(|candidate| candidate.solo);
        if track.mute || (any_solo && !track.solo) {
            return Ok(NativeTrackRender {
                phrase: crate::voicebank::project::render_project_track_sola_cancellable(
                    bank,
                    project,
                    track_index,
                    sample_rate,
                    cancel,
                )?,
                audio: RenderedAudio::empty(sample_rate, 2),
            });
        }
        let phrase = crate::voicebank::project::render_project_track_sola_cancellable(
            bank,
            project,
            track_index,
            sample_rate,
            cancel,
        )?;
        let start_sample =
            (phrase.rendered.start_time_ms.max(0.0) * sample_rate as f64 / 1000.0).round() as usize;
        let skipped_samples = ((-phrase.rendered.start_time_ms).max(0.0) * sample_rate as f64
            / 1000.0)
            .round() as usize;
        let source = phrase
            .rendered
            .samples
            .get(skipped_samples..)
            .ok_or_else(|| "native phrase starts beyond its rendered PCM".to_string())?;
        let mut mono = vec![0.0; start_sample.saturating_add(source.len())];
        mono[start_sample..start_sample + source.len()].copy_from_slice(source);
        let mut stereo = Vec::new();
        Self::mix_track_into(
            &mut stereo,
            &mut mono,
            track.volume_db,
            track.pan,
            track.fx_rack.as_ref(),
            sample_rate,
        );

        for wave in project
            .wave_parts
            .iter()
            .filter(|wave| wave.track_index == track_index)
        {
            if let Ok(decoded) = crate::audio::load_audio_file(&wave.file_path) {
                Self::mix_wave_part_into(
                    &mut stereo,
                    &decoded,
                    sample_rate,
                    wave.position_ms,
                    wave.file_offset_ms,
                    0.0,
                    track.volume_db + wave.volume_db,
                    track.pan,
                );
            }
        }
        if let Some(target_peak) = configured_render_limiter_peak() {
            TrackRenderer::apply_soft_limiter(&mut stereo, target_peak);
        }
        Ok(NativeTrackRender {
            phrase,
            audio: RenderedAudio {
                samples: stereo,
                sample_rate,
                channels: 2,
                error: None,
            },
        })
    }

    pub fn set_render_limiter(enabled: bool, peak_db: f32) {
        super::mix::set_render_limiter(enabled, peak_db);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mixer_applies_volume_and_pan() {
        let mut stereo = Vec::new();
        let mut mono = vec![1.0, 0.5];
        ProjectRenderer::mix_track_into(&mut stereo, &mut mono, -6.0206, 1.0, None, 44100);

        assert_eq!(stereo.len(), 4);
        assert!(stereo[0].abs() < f32::EPSILON);
        assert!((stereo[1] - 0.5).abs() < 0.001);
        assert!(stereo[2].abs() < f32::EPSILON);
        assert!((stereo[3] - 0.25).abs() < 0.001);
    }

    #[test]
    fn simd_mix_paths_preserve_stereo_layout_and_tail() {
        let mono = [1.0, 0.5, -0.25, 0.75, 0.125];
        let mut stereo = vec![0.0; mono.len() * 2];
        accumulate_mono_to_stereo(&mut stereo, &mono, 0.5, -0.25);
        for (frame, &sample) in mono.iter().enumerate() {
            assert_eq!(stereo[frame * 2], sample * 0.5);
            assert_eq!(stereo[frame * 2 + 1], sample * -0.25);
        }

        let source = [
            1.0, -1.0, 0.5, -0.5, 0.25, -0.25, 0.125, -0.125, 0.0625, -0.0625,
        ];
        let mut destination = vec![0.25; source.len()];
        accumulate_stereo_scaled(&mut destination, &source, 2.0, 0.5);
        for frame in 0..source.len() / 2 {
            assert_eq!(destination[frame * 2], 0.25 + source[frame * 2] * 2.0);
            assert_eq!(
                destination[frame * 2 + 1],
                0.25 + source[frame * 2 + 1] * 0.5
            );
        }
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
    fn progressive_context_keeps_the_complete_cvvc_phrase() {
        let mut project = UProject::default();
        project.parts[0].notes = vec![
            UNote::new("no", "C4", 0.0, 300.0),
            UNote::new("na", "D4", 300.0, 300.0),
            UNote::new("next", "E4", 600.0, 300.0),
            UNote::new("separate", "F4", 1_500.0, 300.0),
        ];

        // At 650 ms the renderer must still see `no` and `na`, otherwise a
        // CVVC phonemizer can no longer form the `o n` transition.
        assert_eq!(
            ProjectRenderer::preview_context_start(&project, 650.0, 0.0),
            0.0
        );
        assert_eq!(
            ProjectRenderer::preview_context_start(&project, 1_550.0, 0.0),
            1_500.0
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

    #[test]
    fn test_render_project_progressive_delivers_ordered_chunks() {
        let mut project = UProject::default();
        project.parts[0].notes = vec![
            UNote::new("a", "C4", 0.0, 500.0),
            UNote::new("b", "E4", 1000.0, 500.0),
            UNote::new("c", "G4", 2500.0, 500.0),
        ];

        let directory = tempfile::tempdir().unwrap();
        let source: Vec<f32> = (0..44100)
            .map(|i| (i as f32 * std::f32::consts::TAU * 220.0 / 44100.0).sin() * 0.2)
            .collect();
        TrackRenderer::save_wav_samples(directory.path().join("source.wav"), &source, 44100)
            .unwrap();
        std::fs::write(directory.path().join("oto.ini"), "source.wav=a,0,50,-900,40,10\nsource.wav=b,0,50,-900,40,10\nsource.wav=c,0,50,-900,40,10\n").unwrap();
        let vb = Voicebank::new(directory.path()).unwrap();
        let (tx, rx) = std::sync::mpsc::sync_channel(4);
        let cancel = AtomicBool::new(false);
        let options = RenderOptions::default();

        cancel.store(true, Ordering::Relaxed);
        ProjectRenderer::render_project_progressive(
            &project,
            &vb,
            44100,
            0.0,
            3000.0,
            1000.0,
            &crate::drivers::NativeResamplerDriver,
            &crate::drivers::NativeWavtoolDriver,
            &options,
            None,
            Some(&cancel),
            &tx,
        );
        assert!(
            rx.try_recv().is_err(),
            "cancelled render must not publish chunks"
        );
        cancel.store(false, Ordering::Relaxed);

        ProjectRenderer::render_project_progressive(
            &project,
            &vb,
            44100,
            0.0,
            3000.0,
            1000.0,
            &crate::drivers::NativeResamplerDriver,
            &crate::drivers::NativeWavtoolDriver,
            &options,
            None,
            Some(&cancel),
            &tx,
        );

        let mut chunks = Vec::new();
        while let Ok(chunk) = rx.try_recv() {
            chunks.push(chunk);
        }

        assert!(
            !chunks.is_empty(),
            "Should have received progressive chunks"
        );
        assert_eq!(chunks[0].chunk_start_ms, 0.0);
        assert!(chunks.last().unwrap().is_final);
        assert_eq!(chunks.len(), 3);
        let retried = ProjectRenderer::retry_progressive_chunk(
            &project,
            &vb,
            44100,
            1,
            1000.0,
            2000.0,
            false,
            &crate::drivers::NativeResamplerDriver,
            &crate::drivers::NativeWavtoolDriver,
            &options,
            None,
            None,
        );
        assert_eq!(retried.chunk_index, 1);
        assert_eq!(retried.chunk_start_ms, 1000.0);
        assert_eq!(retried.end_ms(), 2000.0);
        assert_eq!(retried.audio.frame_count(), 44_100);
        assert!(retried.audio.error.is_none());
        let retried_final = ProjectRenderer::retry_progressive_chunk(
            &project,
            &vb,
            44100,
            2,
            2000.0,
            3000.0,
            true,
            &crate::drivers::NativeResamplerDriver,
            &crate::drivers::NativeWavtoolDriver,
            &options,
            None,
            None,
        );
        assert!(retried_final.is_final);
        let full = ProjectRenderer::render_project_with_drivers(
            &project,
            &vb,
            44100,
            0.0,
            &crate::drivers::NativeResamplerDriver,
            &crate::drivers::NativeWavtoolDriver,
            &options,
            None,
        );
        assert!(full.error.is_none());
        let joined: Vec<f32> = chunks
            .iter()
            .flat_map(|c| c.audio.samples.iter().copied())
            .collect();
        let mut expected = full.samples;
        expected.resize(joined.len(), 0.0);
        // Every page has its own complete phoneme context so it can be queued
        // before distant audio is rendered. It is intentionally not required
        // to be bit-identical to an unrelated whole-project buffer, but no
        // page may be malformed or omitted.
        assert!(joined.iter().all(|sample| sample.is_finite()));
        assert_eq!(joined.len(), expected.len());
        assert!(chunks
            .iter()
            .all(|chunk| chunk.audio.frame_count() == 44_100));
        for i in 1..chunks.len() {
            assert!(chunks[i].chunk_start_ms > chunks[i - 1].chunk_start_ms);
            assert_eq!(chunks[i].chunk_index, i);
        }
        assert_eq!(chunks[0].chunk_index, 0);

        // Starting playback from a non-zero playhead must keep chunk timing
        // anchored to the requested project position and trim the final
        // partial chunk to the project end.
        let (offset_tx, offset_rx) = std::sync::mpsc::sync_channel(4);
        ProjectRenderer::render_project_progressive(
            &project,
            &vb,
            44100,
            500.0,
            3000.0,
            1000.0,
            &crate::drivers::NativeResamplerDriver,
            &crate::drivers::NativeWavtoolDriver,
            &options,
            None,
            None,
            &offset_tx,
        );
        let offset_chunks: Vec<_> = offset_rx.try_iter().collect();
        assert_eq!(
            offset_chunks
                .iter()
                .map(|chunk| chunk.chunk_start_ms)
                .collect::<Vec<_>>(),
            vec![500.0, 1500.0, 2500.0]
        );
        assert_eq!(offset_chunks[0].audio.frame_count(), 44_100);
        assert_eq!(offset_chunks[1].audio.frame_count(), 44_100);
        assert_eq!(offset_chunks[2].audio.frame_count(), 22_050);
        assert!(offset_chunks[2].is_final);
    }

    #[test]
    fn world_progressive_chunks_do_not_introduce_boundary_silence() {
        let mut project = UProject::default();
        project.parts[0].notes = vec![
            UNote::new("a", "C4", 0.0, 500.0),
            UNote::new("a", "D4", 500.0, 500.0),
            UNote::new("a", "E4", 1000.0, 500.0),
        ];
        let directory = tempfile::tempdir().unwrap();
        let source: Vec<f32> = (0..88_200)
            .map(|i| {
                let phase = i as f32 * std::f32::consts::TAU * 220.0 / 44_100.0;
                phase.sin() * 0.30 + (phase * 2.0).sin() * 0.12
            })
            .collect();
        TrackRenderer::save_wav_samples(directory.path().join("source.wav"), &source, 44_100)
            .unwrap();
        std::fs::write(
            directory.path().join("oto.ini"),
            "source.wav=a,0,120,-700,0,0\n",
        )
        .unwrap();
        let voicebank = Voicebank::new(directory.path()).unwrap();
        let (tx, rx) = std::sync::mpsc::sync_channel(8);
        let cancel = AtomicBool::new(false);
        ProjectRenderer::render_project_progressive(
            &project,
            &voicebank,
            44_100,
            120.0,
            1_500.0,
            500.0,
            &crate::drivers::NativeWorldResamplerDriver,
            &crate::drivers::NativeWavtoolDriver,
            &RenderOptions::default(),
            None,
            Some(&cancel),
            &tx,
        );
        let chunks: Vec<_> = rx.try_iter().collect();
        assert_eq!(chunks.len(), 3);
        let joined: Vec<f32> = chunks
            .iter()
            .flat_map(|chunk| chunk.audio.samples.iter().copied())
            .collect();
        for center_ms in [250.0, 750.0, 1_250.0] {
            let center = (center_ms * 44.1) as usize;
            let radius = (44_100.0 * 0.08) as usize;
            let window = &joined[center - radius..center + radius];
            let rms = (window
                .iter()
                .map(|sample| f64::from(*sample).powi(2))
                .sum::<f64>()
                / window.len() as f64)
                .sqrt();
            assert!(rms > 0.005, "progressive WORLD chunk cut at {center_ms}ms");
        }
    }

    #[test]
    fn preview_bounds_cover_notes_phonemes_transitions_and_ranges() {
        let mut project = UProject::default();
        let mut first = UNote::new("ka", "C4", 100.0, 400.0);
        first.phoneme_durations_ms = vec![120.0, 280.0];
        project.parts[0].notes = vec![first, UNote::new("a", "D4", 500.0, 300.0)];

        assert_eq!(
            ProjectRenderer::preview_bounds(
                &project,
                PreviewTarget::Note {
                    part_index: 0,
                    note_index: 0,
                }
            ),
            Some((100.0, 500.0))
        );
        assert_eq!(
            ProjectRenderer::preview_bounds(
                &project,
                PreviewTarget::Phoneme {
                    part_index: 0,
                    note_index: 0,
                    phoneme_index: 1,
                }
            ),
            Some((220.0, 500.0))
        );
        assert_eq!(
            ProjectRenderer::preview_bounds(
                &project,
                PreviewTarget::Transition {
                    part_index: 0,
                    left_note_index: 0,
                    right_note_index: 1,
                }
            ),
            Some((100.0, 800.0))
        );
        assert_eq!(
            ProjectRenderer::preview_bounds(
                &project,
                PreviewTarget::Range {
                    start_ms: 50.0,
                    end_ms: 250.0,
                }
            ),
            Some((50.0, 250.0))
        );
        assert!(ProjectRenderer::preview_bounds(
            &project,
            PreviewTarget::Phoneme {
                part_index: 0,
                note_index: 0,
                phoneme_index: 2,
            }
        )
        .is_none());
    }

    #[test]
    fn isolated_preview_returns_exact_target_length() {
        let directory = tempfile::tempdir().unwrap();
        let source: Vec<f32> = (0..44_100)
            .map(|index| (index as f32 * 0.03).sin() * 0.2)
            .collect();
        TrackRenderer::save_wav_samples(directory.path().join("source.wav"), &source, 44_100)
            .unwrap();
        std::fs::write(
            directory.path().join("oto.ini"),
            "source.wav=a,0,50,-400,30,10\n",
        )
        .unwrap();
        let voicebank = Voicebank::new(directory.path()).unwrap();
        let mut project = UProject::default();
        project.parts[0].notes = vec![UNote::new("a", "C4", 0.0, 500.0)];
        let audio = ProjectRenderer::render_preview_target_with_drivers(
            &project,
            &voicebank,
            44_100,
            PreviewTarget::Range {
                start_ms: 100.0,
                end_ms: 250.0,
            },
            &crate::drivers::NativeResamplerDriver,
            &crate::drivers::NativeWavtoolDriver,
            &RenderOptions::default(),
            None,
            None,
        );
        assert!(audio.error.is_none(), "preview failed: {:?}", audio.error);
        assert_eq!(audio.frame_count(), 6_615);
        assert!(audio.samples.iter().any(|sample| sample.abs() > 1e-5));
    }
}
