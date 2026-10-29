pub mod vertical;
use crate::gui::piano_roll::state::ArrangementMeterState;
use crate::gui::theme::ThemeConfig;
use crate::project::model::{UTrack, UVoicePart, UWavePart};
use eframe::egui::{self, Pos2, Rect, Stroke, Vec2};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};
pub use vertical::draw_arrangement_view;

#[derive(Debug)]
struct WaveformPreview {
    bucket_ms: f64,
    duration_ms: f64,
    peaks: Vec<(f32, f32)>,
}

struct CachedWaveform {
    stamp: u128,
    preview: Arc<WaveformPreview>,
    checked_at: Instant,
}

type WaveformCache = HashMap<String, CachedWaveform>;

#[cfg(test)]
fn horizontal_gesture_delta(delta: Vec2, shift: bool) -> f32 {
    delta.x + if shift { delta.y } else { 0.0 }
}

#[allow(dead_code)]
fn move_clip_by_frame_delta(position_ms: f64, delta_x: f32, px_per_ms: f32) -> f64 {
    (position_ms + (delta_x / px_per_ms.max(f32::EPSILON)) as f64).max(0.0)
}

fn move_track_and_remap_parts(
    tracks: &mut Vec<UTrack>,
    parts: &mut [UVoicePart],
    wave_parts: &mut [UWavePart],
    meters: &mut Vec<ArrangementMeterState>,
    active_track_index: &mut usize,
    from: usize,
    to: usize,
) -> bool {
    if from >= tracks.len() || to >= tracks.len() || from == to {
        return false;
    }

    let track = tracks.remove(from);
    tracks.insert(to, track);
    if from < meters.len() {
        let meter = meters.remove(from);
        meters.insert(to.min(meters.len()), meter);
    }
    for part in parts {
        part.track_index = remap_track_index(part.track_index, from, to);
    }
    for wave in wave_parts {
        wave.track_index = remap_track_index(wave.track_index, from, to);
    }
    *active_track_index = remap_track_index(*active_track_index, from, to);
    true
}

fn remap_track_index(index: usize, from: usize, to: usize) -> usize {
    if index == from {
        return to;
    }
    if from < to && (from..=to).contains(&index) {
        return index.saturating_sub(1);
    }
    if to < from && (to..=from).contains(&index) {
        return index.saturating_add(1);
    }
    index
}

fn audio_part_level(wave: &UWavePart, track: &UTrack, playhead_ms: f64) -> Option<f32> {
    let duration = wave.duration_ms.max(0.0);
    if duration <= 0.0
        || playhead_ms < wave.position_ms
        || playhead_ms >= wave.position_ms + duration
    {
        return None;
    }
    let preview = cached_waveform(&wave.file_path)?;
    let source_ms =
        (wave.file_offset_ms + playhead_ms - wave.position_ms).clamp(0.0, preview.duration_ms);
    let bucket = (source_ms / preview.bucket_ms).floor() as usize;
    let (low, high) = *preview
        .peaks
        .get(bucket.min(preview.peaks.len().saturating_sub(1)))?;
    let linear_gain = 10.0f32.powf(((wave.volume_db + track.volume_db) as f32) / 20.0);
    Some((low.abs().max(high.abs()) * linear_gain).clamp(0.0, 1.0))
}

fn cached_waveform(path: &str) -> Option<Arc<WaveformPreview>> {
    static CACHE: OnceLock<Mutex<WaveformCache>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let now = Instant::now();

    // A waveform is painted every frame. Avoid a filesystem metadata request
    // for every audio part at 60 FPS; checking once per second still refreshes
    // an externally replaced file promptly without stalling the UI.
    if let Ok(cache) = cache.lock() {
        if let Some(cached) = cache.get(path) {
            if now.duration_since(cached.checked_at) < Duration::from_secs(1) {
                return Some(Arc::clone(&cached.preview));
            }
        }
    }

    let stamp = std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos();

    if let Ok(mut cache) = cache.lock() {
        if let Some(cached) = cache.get_mut(path) {
            cached.checked_at = now;
            if cached.stamp == stamp {
                return Some(Arc::clone(&cached.preview));
            }
        }
    }

    let audio = crate::audio::load_audio_file(path).ok()?;
    let channels = usize::from(audio.channels.max(1));
    let frames = audio.samples.len() / channels;
    if frames == 0 {
        return None;
    }
    // Store one signed peak envelope per millisecond. The painter aggregates
    // these buckets to the current pixel width, preserving transients when
    // zoomed in and preventing aliased bars when zoomed out.
    let bucket_ms = 1.0f64;
    let frames_per_bucket = ((audio.sample_rate as f64 * bucket_ms) / 1000.0)
        .round()
        .max(1.0) as usize;
    let bucket_count = frames.div_ceil(frames_per_bucket).max(1);
    let mut peaks = Vec::with_capacity(bucket_count);
    for bucket in 0..bucket_count {
        let start = bucket * frames_per_bucket;
        let end = (start + frames_per_bucket).min(frames);
        let (mut min, mut max) = (1.0f32, -1.0f32);
        for frame in start..end {
            let sample = (0..channels)
                .map(|channel| audio.samples[frame * channels + channel])
                .sum::<f32>()
                / channels as f32;
            min = min.min(sample);
            max = max.max(sample);
        }
        peaks.push((min, max));
    }
    if let Ok(mut guard) = cache.lock() {
        let preview = Arc::new(WaveformPreview {
            bucket_ms,
            duration_ms: audio.duration_ms,
            peaks,
        });
        guard.insert(
            path.to_owned(),
            CachedWaveform {
                stamp,
                preview: Arc::clone(&preview),
                checked_at: now,
            },
        );
        return Some(preview);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{
        horizontal_gesture_delta, move_clip_by_frame_delta, move_track_and_remap_parts,
        remap_track_index,
    };
    use crate::gui::piano_roll::state::ArrangementMeterState;
    use crate::project::model::{UTrack, UVoicePart, UWavePart};
    use eframe::egui::Vec2;

    #[test]
    fn clip_drag_applies_only_the_current_frame_delta() {
        let mut position = 1_000.0;
        for _ in 0..10 {
            position = move_clip_by_frame_delta(position, 10.0, 0.1);
        }
        assert_eq!(position, 2_000.0);
    }

    #[test]
    fn arrangement_maps_trackpad_and_shift_wheel_to_horizontal_axis() {
        assert_eq!(horizontal_gesture_delta(Vec2::new(12.0, 40.0), false), 12.0);
        assert_eq!(horizontal_gesture_delta(Vec2::new(12.0, 40.0), true), 52.0);
    }

    #[test]
    fn moving_a_track_remaps_all_part_indices_and_selection() {
        let mut tracks = vec![
            UTrack {
                name: "A".into(),
                ..UTrack::default()
            },
            UTrack {
                name: "B".into(),
                ..UTrack::default()
            },
            UTrack {
                name: "C".into(),
                ..UTrack::default()
            },
        ];
        let mut parts = vec![UVoicePart::new("part-a", 0), UVoicePart::new("part-c", 2)];
        let mut waves = vec![UWavePart {
            name: "audio-b".into(),
            file_path: String::new(),
            track_index: 1,
            position_ms: 0.0,
            file_offset_ms: 0.0,
            volume_db: 0.0,
            duration_ms: 100.0,
        }];
        let mut meters = vec![
            ArrangementMeterState {
                level: 0.1,
                peak: 0.2,
            },
            ArrangementMeterState {
                level: 0.3,
                peak: 0.4,
            },
            ArrangementMeterState {
                level: 0.5,
                peak: 0.6,
            },
        ];
        let mut active = 0;

        assert!(move_track_and_remap_parts(
            &mut tracks,
            &mut parts,
            &mut waves,
            &mut meters,
            &mut active,
            0,
            2,
        ));
        assert_eq!(
            tracks
                .iter()
                .map(|track| track.name.as_str())
                .collect::<Vec<_>>(),
            ["B", "C", "A"]
        );
        assert_eq!(
            parts
                .iter()
                .map(|part| part.track_index)
                .collect::<Vec<_>>(),
            [2, 1]
        );
        assert_eq!(waves[0].track_index, 0);
        assert_eq!(
            meters.iter().map(|meter| meter.level).collect::<Vec<_>>(),
            [0.3, 0.5, 0.1]
        );
        assert_eq!(active, 2);
    }

    #[test]
    fn remap_track_index_preserves_unrelated_tracks() {
        assert_eq!(remap_track_index(4, 1, 3), 4);
        assert_eq!(remap_track_index(2, 1, 3), 1);
        assert_eq!(remap_track_index(2, 3, 1), 3);
    }
}
