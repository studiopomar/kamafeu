use crate::dsp::pitch::{midi_to_freq, midi_to_note_name};
use crate::gui::piano_roll::state::PianoRollState;
use crate::gui::theme::ThemeConfig;
use crate::gui::types::GridSnapOption;
use eframe::egui::{self, Color32, Painter, Pos2, Rect, Rounding, Stroke};

pub fn draw_piano_keys(
    painter: &Painter,
    ui: &egui::Ui,
    state: &PianoRollState,
    theme: &ThemeConfig,
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
    painter.rect_filled(keys_bg_rect, Rounding::ZERO, theme.bg_panel_c32());

    let mouse_pos = ui.input(|i| i.pointer.interact_pos());
    let mouse_down = ui.input(|i| i.pointer.primary_down() || i.pointer.primary_pressed());

    // 1. First pass: Draw white keys background and tactile bevel
    for key_idx in first_visible_key..last_visible_key {
        let midi = state.max_midi - key_idx as u8;
        let y_top = grid_start_y + key_idx as f32 * state.row_height;
        let y_bottom = y_top + state.row_height;

        let is_black_key = matches!(midi % 12, 1 | 3 | 6 | 8 | 10);
        if is_black_key {
            continue;
        }

        let key_rect = Rect::from_min_max(
            Pos2::new(sticky_key_x, y_top),
            Pos2::new(sticky_key_x + keyboard_width, y_bottom),
        );

        let is_hovered = mouse_pos.is_some_and(|pos| key_rect.contains(pos));
        let is_active = state.active_sounding_keys.contains(&midi) || (is_hovered && mouse_down);

        if is_hovered && ui.input(|i| i.pointer.primary_pressed()) {
            let freq = midi_to_freq(midi as f64);
            on_preview_freq(freq);
        }

        let key_color = if is_active {
            theme.key_active_c32()
        } else if is_hovered {
            Color32::from_rgb(250, 248, 255)
        } else {
            theme.bg_keyboard_white_c32()
        };

        painter.rect_filled(key_rect, Rounding::ZERO, key_color);

        // Tactile white key bevel (highlight on top edge, subtle shadow on bottom edge)
        painter.line_segment(
            [
                Pos2::new(sticky_key_x, y_top + 0.5),
                Pos2::new(sticky_key_x + keyboard_width - 1.0, y_top + 0.5),
            ],
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 120)),
        );
        painter.line_segment(
            [
                Pos2::new(sticky_key_x, y_bottom - 0.5),
                Pos2::new(sticky_key_x + keyboard_width, y_bottom - 0.5),
            ],
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(20, 16, 28, 90)),
        );

        // Tonic & note labels
        let is_tonic = state.active_scale != crate::gui::piano_roll::state::MusicalScale::Chromatic
            && state.active_scale.is_tonic(state.scale_root_key, midi);
        let note_str = midi_to_note_name(midi);
        let is_c = midi % 12 == 0;

        let text_color = if is_active {
            Color32::WHITE
        } else if is_tonic {
            Color32::from_rgb(180, 70, 0)
        } else if is_c {
            Color32::from_rgb(20, 15, 30)
        } else {
            Color32::from_rgb(60, 55, 75)
        };

        let font_size = if is_c || is_tonic { 11.5 } else { 10.0 };
        painter.text(
            Pos2::new(sticky_key_x + 6.0, y_top + state.row_height * 0.5),
            egui::Align2::LEFT_CENTER,
            &note_str,
            egui::FontId::proportional(font_size),
            text_color,
        );

        if is_tonic {
            painter.circle_filled(
                Pos2::new(
                    sticky_key_x + keyboard_width - 10.0,
                    y_top + state.row_height * 0.5,
                ),
                3.0,
                Color32::from_rgb(255, 180, 0),
            );
        }
    }

    // 2. Second pass: Draw black keys with drop-shadow and rounded bevel
    let black_key_width = (keyboard_width * 0.65).min(keyboard_width - 16.0);
    for key_idx in first_visible_key..last_visible_key {
        let midi = state.max_midi - key_idx as u8;
        let y_top = grid_start_y + key_idx as f32 * state.row_height;
        let y_bottom = y_top + state.row_height;

        let is_black_key = matches!(midi % 12, 1 | 3 | 6 | 8 | 10);
        if !is_black_key {
            continue;
        }

        let key_rect = Rect::from_min_max(
            Pos2::new(sticky_key_x, y_top + 1.0),
            Pos2::new(sticky_key_x + black_key_width, y_bottom - 1.0),
        );

        let is_hovered = mouse_pos.is_some_and(|pos| key_rect.contains(pos));
        let is_active = state.active_sounding_keys.contains(&midi) || (is_hovered && mouse_down);

        if is_hovered && ui.input(|i| i.pointer.primary_pressed()) {
            let freq = midi_to_freq(midi as f64);
            on_preview_freq(freq);
        }

        // Drop shadow under black key
        let shadow_rect = key_rect.translate(egui::Vec2::new(1.5, 1.5));
        painter.rect_filled(
            shadow_rect,
            Rounding {
                nw: 0.0,
                ne: 3.0,
                se: 3.0,
                sw: 0.0,
            },
            Color32::from_rgba_unmultiplied(0, 0, 0, 110),
        );

        let key_color = if is_active {
            theme.key_active_c32()
        } else if is_hovered {
            Color32::from_rgb(50, 42, 68)
        } else {
            theme.bg_keyboard_black_c32()
        };

        painter.rect_filled(
            key_rect,
            Rounding {
                nw: 0.0,
                ne: 2.5,
                se: 2.5,
                sw: 0.0,
            },
            key_color,
        );

        // Tactile black key bevel highlight
        painter.line_segment(
            [
                Pos2::new(sticky_key_x, y_top + 1.5),
                Pos2::new(sticky_key_x + black_key_width - 2.0, y_top + 1.5),
            ],
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 45)),
        );

        let is_tonic = state.active_scale != crate::gui::piano_roll::state::MusicalScale::Chromatic
            && state.active_scale.is_tonic(state.scale_root_key, midi);
        let note_str = midi_to_note_name(midi);

        let text_color = if is_active {
            Color32::WHITE
        } else if is_tonic {
            Color32::from_rgb(255, 215, 0)
        } else {
            Color32::from_rgb(220, 215, 235)
        };

        painter.text(
            Pos2::new(sticky_key_x + 5.0, y_top + state.row_height * 0.5),
            egui::Align2::LEFT_CENTER,
            &note_str,
            egui::FontId::proportional(8.5),
            text_color,
        );
    }

    painter.line_segment(
        [
            Pos2::new(sticky_key_x + keyboard_width, keys_y_min),
            Pos2::new(sticky_key_x + keyboard_width, keys_y_max),
        ],
        Stroke::new(2.0_f32, theme.accent_c32()),
    );
}

pub fn draw_timeline_grid(
    painter: &Painter,
    state: &PianoRollState,
    theme: &ThemeConfig,
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
        _ => snap_option
            .step_ms_with_zoom(bpm, state.px_per_ms)
            .unwrap_or(beat_ms / 4.0),
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
                (1.8_f32, theme.grid_line_bar_c32())
            } else if is_beat {
                (1.2_f32, theme.grid_line_sub_c32())
            } else {
                (
                    0.8_f32,
                    Color32::from_rgba_unmultiplied(
                        theme.grid_line_sub[0],
                        theme.grid_line_sub[1],
                        theme.grid_line_sub[2],
                        140,
                    ),
                )
            };

            painter.line_segment(
                [Pos2::new(x, y_line_top), Pos2::new(x, y_line_bottom)],
                Stroke::new(line_width, line_color),
            );
        }

        time_ms += visual_step_ms;
    }
}
