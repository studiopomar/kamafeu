use crate::gui::types::GridSnapOption;

pub fn apply_snap(val_ms: f64, snap: GridSnapOption, bpm: f64) -> f64 {
    apply_snap_with_zoom(val_ms, snap, bpm, 0.25)
}

pub fn apply_snap_with_zoom(val_ms: f64, snap: GridSnapOption, bpm: f64, px_per_ms: f32) -> f64 {
    if let Some(step) = snap.step_ms_with_zoom(bpm, px_per_ms) {
        (val_ms / step).round() * step
    } else {
        val_ms
    }
}
