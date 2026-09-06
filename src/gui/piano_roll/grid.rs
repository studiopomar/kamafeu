use crate::dsp::pitch::{midi_to_freq, midi_to_note_name};
use crate::gui::piano_roll::state::PianoRollState;
use crate::gui::theme::MelodyneTheme;
use crate::gui::types::GridSnapOption;
use eframe::egui::{self, Color32, Painter, Pos2, Rect, Rounding, Stroke};

pub fn draw_piano_keys(
    painter: &Painter,
    ui: &egui::Ui,
    state: &PianoRollState,
    rect: Rect,
    visible_clip: Rect,
    keyboard_width: f32,
    grid_start_y: f32,
    grid_end_y: f32,
    first_visible_key: usize,
    last_visible_key: usize,
    on_preview_freq: &mut dyn FnMut(f64),
) {
    let sticky_key_x = rect.min.x.max(visible_clip.min.x);
    let keys_y_min = visible_clip.min.y.max(grid_start_y);
    let keys_y_max = visible_clip.max.y.min(grid_end_y);

    let keys_bg_rect = Rect::from_min_max(
        Pos2::new(sticky_key_x, keys_y_min),
        Pos2::new(sticky_key_x + keyboard_width, keys_y_max),
    );
    painter.rect_filled(keys_bg_rect, Rounding::ZERO, Color32::from_rgb(20, 16, 28));

    for key_idx in first_visible_key..last_visible_key {
        let midi = state.max_midi - key_idx as u8;
        let y_top = grid_start_y + key_idx as f32 * state.row_height;
        let y_bottom = y_top + state.row_height;

        let is_black_key = matches!(midi % 12, 1 | 3 | 6 | 8 | 10);

        let key_rect = Rect::from_min_max(
            Pos2::new(sticky_key_x, y_top),
            Pos2::new(sticky_key_x + keyboard_width, y_bottom),
        );

        let key_color = if is_black_key {
            MelodyneTheme::BG_KEYBOARD_BLACK
        } else {
            MelodyneTheme::BG_KEYBOARD_WHITE
        };

        let text_color = if is_black_key {
            Color32::from_rgb(240, 235, 255)
        } else if midi % 12 == 0 {
            Color32::from_rgb(20, 15, 30)
        } else {
            Color32::from_rgb(45, 40, 55)
        };

        let mouse_pos = ui.input(|i| i.pointer.interact_pos());
        if let Some(mpos) = mouse_pos {
            if key_rect.contains(mpos) && ui.input(|i| i.pointer.primary_pressed()) {
                let freq = midi_to_freq(midi as f64);
                on_preview_freq(freq);
            }
        }

        painter.rect_filled(key_rect, Rounding::ZERO, key_color);
        painter.rect_stroke(
            key_rect,
            Rounding::ZERO,
            Stroke::new(1.0_f32, Color32::from_rgb(15, 12, 20)),
        );

        let note_str = midi_to_note_name(midi);
        let font_size = if is_black_key {
            9.5
        } else if midi % 12 == 0 {
            11.5
        } else {
            10.5
        };
        painter.text(
            Pos2::new(sticky_key_x + 6.0, y_top + state.row_height * 0.5),
            egui::Align2::LEFT_CENTER,
            note_str,
            egui::FontId::proportional(font_size),
            text_color,
        );
    }

    painter.line_segment(
        [
            Pos2::new(sticky_key_x + keyboard_width, keys_y_min),
            Pos2::new(sticky_key_x + keyboard_width, keys_y_max),
        ],
        Stroke::new(2.0_f32, MelodyneTheme::ACCENT_GOLD),
    );
}

pub fn draw_timeline_grid(
    painter: &Painter,
    state: &PianoRollState,
    rect: Rect,
    visible_clip: Rect,
    keyboard_width: f32,
    grid_start_y: f32,
    grid_end_y: f32,
    total_canvas_ms: f64,
    bpm: f64,
    snap_option: GridSnapOption,
) {
    let beat_ms = 60000.0 / bpm;
    let grid_step_ms = match snap_option {
        GridSnapOption::Freeform => beat_ms / 4.0,
        _ => snap_option.step_ms(bpm).unwrap_or(beat_ms / 4.0),
    };

    let mut visual_step_ms = grid_step_ms;
    while (visual_step_ms * state.px_per_ms as f64) < 5.0 {
        visual_step_ms *= 2.0;
    }

    let bar_ms = beat_ms * 4.0;
    let visible_time_start =
        ((visible_clip.min.x - (rect.min.x + keyboard_width)) / state.px_per_ms).max(0.0) as f64;
    let visible_time_end =
        ((visible_clip.max.x - (rect.min.x + keyboard_width)) / state.px_per_ms).max(0.0) as f64;
    let mut time_ms = (visible_time_start / visual_step_ms).floor() * visual_step_ms;

    let y_line_top = visible_clip.min.y.max(grid_start_y);
    let y_line_bottom = visible_clip.max.y.min(grid_end_y);

    while time_ms <= total_canvas_ms.min(visible_time_end + visual_step_ms) {
        let x = rect.min.x + keyboard_width + (time_ms * state.px_per_ms as f64) as f32;
        if x >= rect.min.x + keyboard_width && x <= rect.max.x {
            let is_bar =
                (time_ms % bar_ms).abs() < 1e-2 || ((time_ms % bar_ms) - bar_ms).abs() < 1e-2;
            let is_beat =
                (time_ms % beat_ms).abs() < 1e-2 || ((time_ms % beat_ms) - beat_ms).abs() < 1e-2;

            let (line_width, line_color) = if is_bar {
                (1.8_f32, Color32::from_rgb(110, 85, 160))
            } else if is_beat {
                (1.2_f32, Color32::from_rgb(70, 52, 105))
            } else {
                (0.8_f32, Color32::from_rgb(48, 36, 72))
            };

            painter.line_segment(
                [Pos2::new(x, y_line_top), Pos2::new(x, y_line_bottom)],
                Stroke::new(line_width, line_color),
            );
        }

        time_ms += visual_step_ms;
    }
}
