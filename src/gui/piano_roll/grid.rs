use crate::dsp::pitch::{midi_to_freq, midi_to_note_name};
use crate::gui::piano_roll::state::PianoRollState;
use crate::gui::theme::ThemeConfig;
use crate::gui::types::GridSnapOption;
use eframe::egui::{self, Painter, Pos2, Rect, Rounding, Stroke};

fn keyboard_text_color(
    background: egui::Color32,
    underlying: egui::Color32,
    tonic: bool,
) -> egui::Color32 {
    let [r, g, b, alpha] = background.to_array();
    let [ur, ug, ub, _] = underlying.to_array();
    let blend = |top: u8, bottom: u8| {
        ((top as u16 * alpha as u16 + bottom as u16 * (255 - alpha as u16)) / 255) as u8
    };
    let [r, g, b] = [blend(r, ur), blend(g, ug), blend(b, ub)];
    let foregrounds = if tonic {
        [[112, 58, 28], [255, 202, 132]]
    } else {
        [[30, 27, 31], [250, 247, 243]]
    };
    let foreground = foregrounds
        .into_iter()
        .max_by(|left, right| {
            crate::gui::theme::contrast_ratio_rgb(*left, [r, g, b])
                .total_cmp(&crate::gui::theme::contrast_ratio_rgb(*right, [r, g, b]))
        })
        .unwrap_or([30, 27, 31]);
    egui::Color32::from_rgb(foreground[0], foreground[1], foreground[2])
}

pub(super) struct PitchRowLayer<'a> {
    pub(super) state: &'a PianoRollState,
    pub(super) theme: &'a ThemeConfig,
    pub(super) rect: Rect,
    pub(super) grid_start_y: f32,
    pub(super) first_visible_key: usize,
    pub(super) last_visible_key: usize,
    pub(super) keyboard_width: f32,
}

/// Draw the pitch rows and scale shading behind note content.
pub(super) fn draw_pitch_rows(painter: &Painter, layer: PitchRowLayer<'_>) {
    let PitchRowLayer {
        state,
        theme,
        rect,
        grid_start_y,
        first_visible_key,
        last_visible_key,
        keyboard_width,
    } = layer;

    for key_idx in first_visible_key..last_visible_key {
        let midi = state.max_midi - key_idx as u8;
        let y_top = grid_start_y + key_idx as f32 * state.row_height;
        let y_bottom = y_top + state.row_height;

        let is_black_key = matches!(midi % 12, 1 | 3 | 6 | 8 | 10);
        let in_scale = state.active_scale.is_in_scale(state.scale_root_key, midi);
        let is_tonic = state.active_scale != crate::gui::piano_roll::state::MusicalScale::Chromatic
            && state.active_scale.is_tonic(state.scale_root_key, midi);

        let row_color = if is_black_key {
            theme.bg_row_black_key_c32()
        } else {
            theme.bg_row_white_key_c32()
        };

        let row_x_min = rect.min.x + keyboard_width;
        let row_x_max = rect.max.x;

        if row_x_min >= row_x_max {
            continue;
        }

        let row_rect =
            Rect::from_min_max(Pos2::new(row_x_min, y_top), Pos2::new(row_x_max, y_bottom));
        painter.rect_filled(row_rect, Rounding::ZERO, row_color);

        if state.active_scale != crate::gui::piano_roll::state::MusicalScale::Chromatic {
            if !in_scale {
                painter.rect_filled(row_rect, Rounding::ZERO, theme.scale_out_of_key_tint());
            } else if is_tonic {
                painter.rect_filled(row_rect, Rounding::ZERO, theme.scale_tonic_tint());
            } else {
                painter.rect_filled(row_rect, Rounding::ZERO, theme.scale_in_key_tint());
            }
        }

        painter.line_segment(
            [
                Pos2::new(row_x_min, y_bottom),
                Pos2::new(row_x_max, y_bottom),
            ],
            Stroke::new(
                if is_tonic { 1.0_f32 } else { 0.5_f32 },
                if is_tonic {
                    theme.accent_c32()
                } else {
                    theme.grid_line_sub_c32()
                },
            ),
        );
    }
}

#[allow(clippy::too_many_arguments)]
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

    // 1. Flat key surfaces: color and labels carry the hierarchy; avoid
    // bevels, shadows and faux 3-D controls in the piano-roll keyboard.
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
            theme.bg_keyboard_white_c32().linear_multiply(1.08)
        } else {
            theme.bg_keyboard_white_c32()
        };

        painter.rect_filled(key_rect, Rounding::ZERO, key_color);

        // Tonic & note labels
        let is_tonic = state.active_scale != crate::gui::piano_roll::state::MusicalScale::Chromatic
            && state.active_scale.is_tonic(state.scale_root_key, midi);
        let note_str = midi_to_note_name(midi);
        let is_c = midi % 12 == 0;

        let text_color = keyboard_text_color(key_color, theme.bg_keyboard_white_c32(), is_tonic);

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
                keyboard_text_color(key_color, theme.bg_keyboard_white_c32(), true),
            );
        }
    }

    // 2. Black keys use the same flat treatment, only with a darker surface.
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

        let key_color = if is_active {
            theme.key_active_c32()
        } else if is_hovered {
            ui.visuals().hyperlink_color
        } else {
            theme.bg_keyboard_black_c32()
        };

        painter.rect_filled(key_rect, Rounding::ZERO, key_color);

        let is_tonic = state.active_scale != crate::gui::piano_roll::state::MusicalScale::Chromatic
            && state.active_scale.is_tonic(state.scale_root_key, midi);
        let note_str = midi_to_note_name(midi);

        let text_color = keyboard_text_color(key_color, theme.bg_keyboard_black_c32(), is_tonic);

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
        Stroke::new(1.0_f32, theme.grid_line_sub_c32()),
    );
}

#[allow(clippy::too_many_arguments)]
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
    time_signature_numerator: u8,
    time_signature_denominator: u8,
    snap_option: GridSnapOption,
) {
    let quarter_ms = 60000.0 / bpm;
    let beat_ms = quarter_ms * 4.0 / f64::from(time_signature_denominator.max(1));
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

    let bar_ms = beat_ms * f64::from(time_signature_numerator.max(1));
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
                (1.4_f32, theme.c32_alpha(theme.grid_line_bar, 0.72))
            } else if is_beat {
                (1.0_f32, theme.c32_alpha(theme.grid_line_sub, 0.52))
            } else {
                (0.6_f32, theme.c32_alpha(theme.grid_line_sub, 0.28))
            };

            painter.line_segment(
                [Pos2::new(x, y_line_top), Pos2::new(x, y_line_bottom)],
                Stroke::new(line_width, line_color),
            );
        }

        time_ms += visual_step_ms;
    }
}
