use super::ArrangementMeterState;
use crate::project::model::{UTrack, UVoicePart, UWavePart};
use eframe::egui::Vec2;

pub(super) fn horizontal_gesture_delta(delta: Vec2, shift: bool) -> f32 {
    delta.x + if shift { delta.y } else { 0.0 }
}

pub(super) fn move_clip_by_frame_delta(position_ms: f64, delta_x: f32, px_per_ms: f32) -> f64 {
    (position_ms + (delta_x / px_per_ms.max(f32::EPSILON)) as f64).max(0.0)
}

pub(super) fn move_track_and_remap_parts(
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

pub(super) fn remap_track_index(index: usize, from: usize, to: usize) -> usize {
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
