use crate::gui::piano_roll::state::PianoRollState;
use crate::gui::theme::ThemeConfig;
use crate::project::model::UVoicePart;
use eframe::egui::{Painter, Pos2, Rect, Rounding, Stroke, Vec2};

pub(super) struct Layer<'a> {
    pub(super) parts: &'a [&'a UVoicePart],
    pub(super) active_part_position_ms: f64,
    pub(super) state: &'a PianoRollState,
    pub(super) theme: &'a ThemeConfig,
    pub(super) rect: Rect,
    pub(super) visible_clip: Rect,
    pub(super) keyboard_width: f32,
    pub(super) grid_start_y: f32,
}

/// Draw reference notes behind the active track without creating interactive
/// responses or mutating editor state.
pub(super) fn draw(painter: &Painter, layer: Layer<'_>) {
    let Layer {
        parts,
        active_part_position_ms,
        state,
        theme,
        rect,
        visible_clip,
        keyboard_width,
        grid_start_y,
    } = layer;
    let ghost_fill = theme.c32_alpha(theme.semantic.info, 0.20);
    let ghost_stroke = theme.c32_alpha(theme.semantic.info, 0.48);
    let timeline_left = rect.min.x + keyboard_width;
    let extended_clip = visible_clip.expand2(Vec2::new(24.0, 2.0));

    for part in parts {
        let part_offset_ms = part.position_ms - active_part_position_ms;
        for note in &part.notes {
            let midi = note.midi_key();
            if midi < state.min_midi
                || midi > state.max_midi
                || !note.position_ms.is_finite()
                || !note.duration_ms.is_finite()
                || note.duration_ms <= 0.0
            {
                continue;
            }

            let x_start = timeline_left
                + ((part_offset_ms + note.position_ms) * state.px_per_ms as f64) as f32;
            let x_end = x_start + (note.duration_ms * state.px_per_ms as f64) as f32;
            let key_idx = (state.max_midi - midi) as f32;
            let y_top = grid_start_y + key_idx * state.row_height + 2.0;
            let y_bottom = y_top + state.row_height - 4.0;
            let note_rect =
                Rect::from_min_max(Pos2::new(x_start, y_top), Pos2::new(x_end, y_bottom));

            if !note_rect.is_positive() || !note_rect.intersects(extended_clip) {
                continue;
            }
            painter.rect_filled(note_rect, Rounding::same(3.0), ghost_fill);
            painter.rect_stroke(
                note_rect,
                Rounding::same(3.0),
                Stroke::new(0.8_f32, ghost_stroke),
            );
        }
    }
}
