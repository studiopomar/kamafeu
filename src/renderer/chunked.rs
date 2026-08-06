use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::thread;

use crate::drivers::{ResamplerDriver, WavtoolDriver};
use crate::oto::Voicebank;
use crate::project::model::UNote;
use crate::renderer::RenderOptions;
use crate::renderer::TrackRenderer;

pub struct ChunkedRenderer;

impl ChunkedRenderer {
    /// Filter notes that fall within or overlap time window [start_ms, end_ms]
    pub fn filter_notes_in_window(notes: &[UNote], start_ms: f64, end_ms: f64) -> Vec<UNote> {
        notes
            .iter()
            .filter(|n| {
                let note_start = n.position_ms;
                let note_end = n.position_ms + n.duration_ms;
                note_end >= start_ms && note_start <= end_ms
            })
            .cloned()
            .collect()
    }

    /// Render priority playhead chunk first (<50ms) for instant audio playback
    pub fn render_playhead_chunk(
        notes: &[UNote],
        voicebank: &Voicebank,
        sample_rate: u32,
        tempo_bpm: f64,
        playhead_ms: f64,
        window_duration_ms: f64,
        resampler_driver: &dyn ResamplerDriver,
        wavtool_driver: &dyn WavtoolDriver,
        vocal_mode: Option<&RenderOptions>,
    ) -> Vec<f32> {
        let active_window_end = playhead_ms + window_duration_ms;
        let mut playhead_notes =
            Self::filter_notes_in_window(notes, playhead_ms, active_window_end);

        if playhead_notes.is_empty() {
            playhead_notes = notes.to_vec();
        }

        let mut shifted_notes = Vec::new();
        for n in &playhead_notes {
            let mut shifted = n.clone();
            if shifted.position_ms >= playhead_ms {
                shifted.position_ms -= playhead_ms;
                shifted_notes.push(shifted);
            } else {
                let cut_ms = playhead_ms - shifted.position_ms;
                if shifted.duration_ms > cut_ms {
                    shifted.position_ms = 0.0;
                    shifted.duration_ms -= cut_ms;
                    shifted_notes.push(shifted);
                }
            }
        }

        TrackRenderer::render_track_with_drivers(
            &shifted_notes,
            voicebank,
            sample_rate,
            tempo_bpm,
            resampler_driver,
            wavtool_driver,
            vocal_mode,
        )
    }

    /// Spawn background thread to synthesize full track and update progress atomic (0..100)
    pub fn spawn_background_rendering(
        notes: Vec<UNote>,
        voicebank: Voicebank,
        sample_rate: u32,
        tempo_bpm: f64,
        resampler_driver: Box<dyn ResamplerDriver>,
        wavtool_driver: Box<dyn WavtoolDriver>,
        progress_atomic: Arc<AtomicU32>,
        on_complete: impl FnOnce(Vec<f32>) + Send + 'static,
    ) {
        thread::spawn(move || {
            progress_atomic.store(10, Ordering::Relaxed);
            let rendered = TrackRenderer::render_track_with_drivers(
                &notes,
                &voicebank,
                sample_rate,
                tempo_bpm,
                resampler_driver.as_ref(),
                wavtool_driver.as_ref(),
                None,
            );
            progress_atomic.store(100, Ordering::Relaxed);
            on_complete(rendered);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_notes_in_window() {
        let notes = vec![
            UNote::new("ka", "C4", 0.0, 500.0),
            UNote::new("ki", "D4", 1000.0, 500.0),
            UNote::new("ku", "E4", 3000.0, 500.0),
        ];

        let window = ChunkedRenderer::filter_notes_in_window(&notes, 800.0, 2000.0);
        assert_eq!(window.len(), 1);
        assert_eq!(window[0].lyric, "ki");
    }
}
