mod context_menu;
#[cfg(test)]
mod interaction_tests;
mod minimap;
mod note_properties;
mod parameter_drawer;
mod phoneme_cache;
mod vibrato_button;
use crate::dsp::pitch::{midi_to_freq, midi_to_note_name};
use crate::dsp::pitch_bend::PitchBendSolver;
use crate::gui::phoneme_palette::PhonemePaletteState;
use crate::gui::theme::MelodyneTheme;
use crate::gui::types::{AutoScrollMode, EditTool, GridSnapOption, PitchSubTool};
use crate::oto::Voicebank;
use crate::project::model::{UNote, UPitchBendPoint};
use eframe::egui::{self, Color32, Key, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use std::collections::HashSet;
pub mod grid;
pub mod state;
pub use grid::*;
pub use state::*;

use crate::gui::theme::ThemeConfig;

pub fn draw_piano_roll(
    ui: &mut egui::Ui,
    notes: &mut Vec<UNote>,
    state: &mut PianoRollState,
    theme: &ThemeConfig,
    voicebank: Option<&Voicebank>,
    phoneme_state: &mut PhonemePaletteState,
    snap_option: GridSnapOption,
    bpm: f64,
    phonemizer_mode: crate::phonemizer::PhonemizerMode,
    lang: crate::config::AppLanguage,
    on_preview_freq: &mut dyn FnMut(f64),
    on_before_change: &mut dyn FnMut(),
    on_note_changed: &mut dyn FnMut(),
    on_playhead_scrubbed: &mut dyn FnMut(f64),
    on_edit_oto_alias: &mut dyn FnMut(&str, &str),
) {
    let was_editing_lyric = state.editing_lyric_index.is_some();
    let key_count = (state.max_midi - state.min_midi + 1) as usize;
    let ruler_height = 28.0f32;
    let param_drawer_height = if state.show_parameters_drawer {
        state.drawer_height
    } else {
        0.0f32
    };
    let _total_height = key_count as f32 * state.row_height + ruler_height + param_drawer_height;
    let keyboard_width = 65.0f32;
    let (ctrl_pressed, alt_pressed, cmd_pressed) = ui.input(|i| {
        (
            i.modifiers.ctrl,
            i.modifiers.alt,
            i.modifiers.command || i.modifiers.mac_cmd,
        )
    });
    let is_mod_zoom = ctrl_pressed || alt_pressed || cmd_pressed;

    let (is_middle_down, mouse_delta) = ui.input(|i| {
        (
            i.pointer.middle_down() || i.pointer.button_down(egui::PointerButton::Middle),
            i.pointer.delta(),
        )
    });

    let available_viewport = ui.available_rect_before_wrap();
    let is_hovering_piano_roll = ui.input(|i| {
        i.pointer
            .hover_pos()
            .or(i.pointer.latest_pos())
            .map(|pos| available_viewport.contains(pos))
            .unwrap_or(false)
    });

    let mut is_middle_panning = false;
    if is_middle_down && (is_hovering_piano_roll || state.is_middle_panning) {
        state.is_middle_panning = true;
        is_middle_panning = true;
        if mouse_delta.length_sq() > 0.0 {
            state.horizontal_scroll_offset =
                (state.horizontal_scroll_offset - mouse_delta.x).max(0.0);
            state.vertical_scroll_offset = (state.vertical_scroll_offset - mouse_delta.y).max(0.0);
        }
        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grabbing);
    } else {
        state.is_middle_panning = false;
    }

    // Consume Ctrl/Alt zoom input only while the pointer is actually over the
    // piano-roll viewport.  Reading and clearing global scroll deltas here
    // otherwise steals wheel events from the inspector and other windows.
    if is_mod_zoom && is_hovering_piano_roll {
        let wheel_delta = ui.input(|i| {
            // Only consume a scroll/zoom event that belongs to this frame.
            // Reading the accumulated scroll delta while Ctrl is merely
            // pressed makes playback jump to an apparently random position.
            let mut dy: f32 = 0.0;
            for ev in &i.events {
                if let egui::Event::MouseWheel { delta, .. } = ev {
                    if delta.y.abs() > dy.abs() {
                        dy = delta.y;
                    }
                } else if let egui::Event::Zoom(factor) = ev {
                    let derived = (*factor - 1.0) * 80.0;
                    if derived.abs() > dy.abs() {
                        dy = derived;
                    }
                }
            }
            if dy.abs() < 1e-3 {
                dy = (i.zoom_delta() - 1.0) * 80.0;
            }
            dy
        });

        if wheel_delta != 0.0 {
            let zoom_factor = 1.05f32.powf(wheel_delta * 0.05);

            let mouse_pos = ui.input(|i| {
                i.pointer
                    .hover_pos()
                    .or(i.pointer.interact_pos())
                    .or(i.pointer.latest_pos())
            });

            if alt_pressed {
                let old_row_h = state.row_height;
                let new_row_h = (old_row_h * zoom_factor).clamp(12.0, 70.0);

                if let Some(mpos) = mouse_pos {
                    let grid_screen_top = ui.available_rect_before_wrap().min.y + ruler_height;
                    let mouse_y_in_viewport = (mpos.y - grid_screen_top).max(0.0);
                    let canvas_y_under_mouse = mouse_y_in_viewport + state.vertical_scroll_offset;
                    let key_ratio_under_mouse = canvas_y_under_mouse / old_row_h;
                    let new_canvas_y = key_ratio_under_mouse * new_row_h;
                    state.vertical_scroll_offset = (new_canvas_y - mouse_y_in_viewport).max(0.0);
                }

                state.row_height = new_row_h;
            } else if ctrl_pressed || cmd_pressed {
                let old_px_per_ms = state.px_per_ms;
                let new_px_per_ms = (old_px_per_ms * zoom_factor).clamp(0.04, 2.5);

                if let Some(mpos) = mouse_pos {
                    let canvas_screen_left = ui.available_rect_before_wrap().min.x + keyboard_width;
                    let mouse_x_in_viewport = (mpos.x - canvas_screen_left).max(0.0);
                    let time_under_mouse = (mouse_x_in_viewport + state.horizontal_scroll_offset)
                        as f64
                        / old_px_per_ms as f64;
                    let new_mouse_x_in_canvas = (time_under_mouse * new_px_per_ms as f64) as f32;
                    state.horizontal_scroll_offset =
                        (new_mouse_x_in_canvas - mouse_x_in_viewport).max(0.0);
                }

                state.px_per_ms = new_px_per_ms;
            }
        }

        ui.ctx().input_mut(|i| {
            i.smooth_scroll_delta = egui::Vec2::ZERO;
            i.raw_scroll_delta = egui::Vec2::ZERO;
        });
    }

    if state.is_playing {
        let playhead_x = (state.playhead_ms * state.px_per_ms as f64) as f32;
        let visible_width = (ui.available_width() - keyboard_width).max(100.0);

        match state.auto_scroll_mode {
            AutoScrollMode::StationaryCursor => {
                let target_x = (playhead_x - visible_width * 0.30).max(0.0);
                state.horizontal_scroll_offset = target_x;
            }
            AutoScrollMode::PageScroll => {
                let curr_offset = state.horizontal_scroll_offset;
                if playhead_x > curr_offset + visible_width * 0.92 || playhead_x < curr_offset {
                    let target_x = (playhead_x - visible_width * 0.08).max(0.0);
                    state.horizontal_scroll_offset = target_x;
                }
            }
            AutoScrollMode::Off => {}
        }
    }

    let timeline_scroll_x = state.horizontal_scroll_offset;
    let max_note_end_ms = notes
        .iter()
        .map(|note| note.position_ms + note.duration_ms)
        .fold(0.0f64, f64::max);
    let total_canvas_ms = (max_note_end_ms + 30_000.0).max(60_000.0);

    phoneme_cache::draw(notes, state, voicebank, phonemizer_mode);

    // Update active playback sounding keys
    state.active_sounding_keys.clear();
    if state.is_playing {
        for n in notes.iter() {
            if state.playhead_ms >= n.position_ms
                && state.playhead_ms <= n.position_ms + n.duration_ms
            {
                state.active_sounding_keys.insert(n.midi_key());
            }
        }
    }

    minimap::draw(
        ui,
        notes,
        state,
        theme,
        keyboard_width,
        timeline_scroll_x,
        total_canvas_ms,
    );

    let (ruler_rect, _ruler_resp) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), ruler_height),
        Sense::click_and_drag(),
    );

    let ruler_painter = ui.painter_at(ruler_rect);
    ruler_painter.rect_filled(ruler_rect, Rounding::ZERO, MelodyneTheme::BG_HEADER);
    ruler_painter.line_segment(
        [
            Pos2::new(ruler_rect.min.x, ruler_rect.max.y),
            Pos2::new(ruler_rect.max.x, ruler_rect.max.y),
        ],
        Stroke::new(1.5_f32, MelodyneTheme::GRID_LINE_BAR),
    );

    let corner_rect = Rect::from_min_max(
        Pos2::new(ruler_rect.min.x, ruler_rect.min.y),
        Pos2::new(ruler_rect.min.x + keyboard_width, ruler_rect.max.y),
    );
    ruler_painter.rect_filled(corner_rect, Rounding::ZERO, MelodyneTheme::BG_PANEL);
    ruler_painter.text(
        Pos2::new(
            ruler_rect.min.x + 8.0,
            ruler_rect.min.y + ruler_height * 0.5,
        ),
        egui::Align2::LEFT_CENTER,
        "RULER",
        egui::FontId::proportional(10.0),
        MelodyneTheme::TEXT_GOLD_LABEL,
    );

    let beat_ms = 60000.0 / bpm;
    let bar_ms = beat_ms * 4.0;
    let ruler_visible_width = (ruler_rect.width() - keyboard_width).max(1.0);
    let ruler_start_ms = (timeline_scroll_x / state.px_per_ms).max(0.0) as f64;
    let ruler_end_ms = ruler_start_ms + (ruler_visible_width / state.px_per_ms) as f64 + bar_ms;
    let first_measure = (ruler_start_ms / bar_ms).floor().max(0.0) as usize;
    let mut measure_idx = first_measure + 1;
    let mut m_ms = first_measure as f64 * bar_ms;

    while m_ms < total_canvas_ms.min(ruler_end_ms) {
        let x = ruler_rect.min.x + keyboard_width + (m_ms * state.px_per_ms as f64) as f32
            - timeline_scroll_x;
        if x >= ruler_rect.min.x + keyboard_width && x <= ruler_rect.max.x + 200.0 {
            ruler_painter.line_segment(
                [
                    Pos2::new(x, ruler_rect.min.y + 12.0),
                    Pos2::new(x, ruler_rect.max.y),
                ],
                Stroke::new(1.5_f32, MelodyneTheme::ACCENT_GOLD),
            );

            ruler_painter.text(
                Pos2::new(x + 4.0, ruler_rect.min.y + 4.0),
                egui::Align2::LEFT_TOP,
                format!("m{}", measure_idx),
                egui::FontId::proportional(11.0),
                MelodyneTheme::TEXT_GOLD_LABEL,
            );
        }

        measure_idx += 1;
        m_ms += bar_ms;
    }

    // Draw Loop Region on Ruler
    if state.loop_end_ms > state.loop_start_ms {
        let loop_start_x = ruler_rect.min.x
            + keyboard_width
            + (state.loop_start_ms * state.px_per_ms as f64) as f32
            - timeline_scroll_x;
        let loop_end_x =
            ruler_rect.min.x + keyboard_width + (state.loop_end_ms * state.px_per_ms as f64) as f32
                - timeline_scroll_x;

        let left_bound = ruler_rect.min.x + keyboard_width;
        let right_bound = ruler_rect.max.x;

        let visible_loop_left = loop_start_x.clamp(left_bound, right_bound);
        let visible_loop_right = loop_end_x.clamp(left_bound, right_bound);

        if visible_loop_right > visible_loop_left {
            let loop_banner_rect = Rect::from_min_max(
                Pos2::new(visible_loop_left, ruler_rect.min.y + 1.0),
                Pos2::new(visible_loop_right, ruler_rect.max.y - 1.0),
            );
            let banner_bg = if state.loop_enabled {
                Color32::from_rgba_unmultiplied(0, 255, 157, 45)
            } else {
                Color32::from_rgba_unmultiplied(120, 140, 160, 25)
            };
            ruler_painter.rect_filled(loop_banner_rect, Rounding::same(2.0), banner_bg);
        }

        // Loop Start Marker [A
        if loop_start_x >= left_bound && loop_start_x <= right_bound {
            let marker_color = if state.loop_enabled {
                Color32::from_rgb(0, 255, 157)
            } else {
                Color32::from_rgb(160, 175, 190)
            };
            ruler_painter.line_segment(
                [
                    Pos2::new(loop_start_x, ruler_rect.min.y + 2.0),
                    Pos2::new(loop_start_x, ruler_rect.max.y - 1.0),
                ],
                Stroke::new(2.0_f32, marker_color),
            );
            ruler_painter.text(
                Pos2::new(loop_start_x + 3.0, ruler_rect.min.y + 2.0),
                egui::Align2::LEFT_TOP,
                "A",
                egui::FontId::monospace(9.0),
                marker_color,
            );
        }

        // Loop End Marker B]
        if loop_end_x >= left_bound && loop_end_x <= right_bound {
            let marker_color = if state.loop_enabled {
                Color32::from_rgb(0, 255, 157)
            } else {
                Color32::from_rgb(160, 175, 190)
            };
            ruler_painter.line_segment(
                [
                    Pos2::new(loop_end_x, ruler_rect.min.y + 2.0),
                    Pos2::new(loop_end_x, ruler_rect.max.y - 1.0),
                ],
                Stroke::new(2.0_f32, marker_color),
            );
            ruler_painter.text(
                Pos2::new(loop_end_x - 3.0, ruler_rect.min.y + 2.0),
                egui::Align2::RIGHT_TOP,
                "B",
                egui::FontId::monospace(9.0),
                marker_color,
            );
        }
    }

    let ruler_playhead_x =
        ruler_rect.min.x + keyboard_width + (state.playhead_ms * state.px_per_ms as f64) as f32
            - timeline_scroll_x;
    if ruler_playhead_x >= ruler_rect.min.x + keyboard_width && ruler_playhead_x <= ruler_rect.max.x
    {
        let tri = vec![
            Pos2::new(ruler_playhead_x - 3.5, ruler_rect.min.y + 1.0),
            Pos2::new(ruler_playhead_x + 3.5, ruler_rect.min.y + 1.0),
            Pos2::new(ruler_playhead_x, ruler_rect.min.y + 11.0),
        ];
        ruler_painter.add(egui::Shape::convex_polygon(
            tri,
            Color32::from_rgb(255, 65, 85),
            Stroke::new(0.8_f32, Color32::WHITE),
        ));
        ruler_painter.line_segment(
            [
                Pos2::new(ruler_playhead_x, ruler_rect.min.y + 10.0),
                Pos2::new(ruler_playhead_x, ruler_rect.max.y),
            ],
            Stroke::new(1.0_f32, Color32::from_rgb(255, 65, 85)),
        );
    }

    let ruler_response = ui.interact(
        ruler_rect,
        ui.make_persistent_id("piano_roll_ruler_interaction"),
        Sense::click_and_drag(),
    );

    // Botão ícone compacto de Maximizar / Restaurar no canto da régua
    let max_btn_size = 22.0_f32;
    let max_btn_rect = Rect::from_center_size(
        Pos2::new(ruler_rect.max.x - 16.0, ruler_rect.center().y),
        Vec2::splat(max_btn_size),
    );

    let max_btn_resp = ui.interact(
        max_btn_rect,
        ui.make_persistent_id("piano_roll_maximize_toggle_btn"),
        Sense::click(),
    );
    if max_btn_resp.clicked() {
        state.is_maximized = !state.is_maximized;
    }

    let is_hovered = max_btn_resp.hovered();
    let (btn_bg, btn_stroke, text_color, icon_symbol) = if state.is_maximized {
        (
            if is_hovered {
                Color32::from_rgb(52, 42, 20)
            } else {
                Color32::from_rgb(32, 25, 12)
            },
            Stroke::new(1.0_f32, MelodyneTheme::ACCENT_GOLD),
            MelodyneTheme::ACCENT_GOLD,
            "[v]",
        )
    } else {
        (
            if is_hovered {
                Color32::from_rgb(25, 45, 38)
            } else {
                Color32::from_rgb(18, 28, 24)
            },
            Stroke::new(
                1.0_f32,
                if is_hovered {
                    Color32::from_rgb(0, 255, 157)
                } else {
                    Color32::from_rgba_unmultiplied(0, 255, 157, 100)
                },
            ),
            if is_hovered {
                Color32::from_rgb(0, 255, 157)
            } else {
                Color32::from_rgba_unmultiplied(0, 255, 157, 200)
            },
            "[^]",
        )
    };

    ruler_painter.rect_filled(max_btn_rect, Rounding::same(4.0), btn_bg);
    ruler_painter.rect_stroke(max_btn_rect, Rounding::same(4.0), btn_stroke);
    ruler_painter.text(
        max_btn_rect.center(),
        egui::Align2::CENTER_CENTER,
        icon_symbol,
        egui::FontId::proportional(12.0),
        text_color,
    );

    if is_hovered {
        max_btn_resp.clone().on_hover_text(if state.is_maximized {
            lang.tr(
                "Restaurar layout padrão (F11 / Shift+F)",
                "Restore default layout (F11 / Shift+F)",
            )
        } else {
            lang.tr(
                "Maximizar Piano Roll / Otimizar espaço (F11 / Shift+F)",
                "Maximize Piano Roll / Optimize space (F11 / Shift+F)",
            )
        });
    }

    if ruler_response.clicked() || ruler_response.dragged() {
        if let Some(mpos) = ruler_response.interact_pointer_pos() {
            let shift_held = ui.input(|i| i.modifiers.shift);
            let raw_t = (mpos.x - (ruler_rect.min.x + keyboard_width) + timeline_scroll_x) as f64
                / state.px_per_ms as f64;
            let scrubbed_t =
                apply_snap_with_zoom(raw_t.max(0.0), snap_option, bpm, state.px_per_ms);

            if shift_held {
                // Shift+Click/Drag sets loop boundary
                if ruler_response.clicked() {
                    state.loop_start_ms = scrubbed_t;
                    state.loop_end_ms = scrubbed_t + (60000.0 / bpm) * 4.0;
                    state.loop_enabled = true;
                } else if ruler_response.dragged() {
                    if scrubbed_t > state.loop_start_ms {
                        state.loop_end_ms = scrubbed_t;
                    } else {
                        state.loop_start_ms = scrubbed_t;
                    }
                    state.loop_enabled = true;
                }
            } else {
                state.is_scrubbing_ruler = true;
                state.playhead_ms = scrubbed_t;
                on_playhead_scrubbed(scrubbed_t);

                let playhead_canvas_x = (scrubbed_t * state.px_per_ms as f64) as f32;
                let visible_w = (ui.available_width() - keyboard_width).max(100.0);
                if playhead_canvas_x < state.horizontal_scroll_offset + 50.0 {
                    state.horizontal_scroll_offset = (playhead_canvas_x - 50.0).max(0.0);
                } else if playhead_canvas_x > state.horizontal_scroll_offset + visible_w - 70.0 {
                    state.horizontal_scroll_offset =
                        (playhead_canvas_x - visible_w + 70.0).max(0.0);
                }
            }
        }
    }

    if state.is_scrubbing_ruler && !ui.input(|i| i.pointer.primary_down()) {
        state.is_scrubbing_ruler = false;
    }

    crate::gui::phoneme_ruler::draw_phoneme_ruler(
        ui,
        theme,
        state,
        notes,
        voicebank,
        ruler_rect,
        keyboard_width,
        timeline_scroll_x,
        bpm,
        on_before_change,
        on_note_changed,
        on_edit_oto_alias,
    );

    parameter_drawer::draw(
        ui,
        notes,
        state,
        keyboard_width,
        timeline_scroll_x,
        on_before_change,
        theme,
        ruler_rect,
        bpm,
        lang,
    );

    let grid_width = (keyboard_width as f64 + total_canvas_ms * state.px_per_ms as f64) as f32;
    let grid_width = grid_width.max(3000.0);
    let grid_height = key_count as f32 * state.row_height;

    let mut scroll_area = egui::ScrollArea::both()
        .id_salt("piano_roll_scroll")
        .auto_shrink([false, false])
        .enable_scrolling(!is_mod_zoom)
        // Primary-button drags edit notes; wheel and scrollbars still navigate.
        .drag_to_scroll(false);

    if !state.initial_scrolled {
        let (first_note_pos, target_midi) = if let Some(first) = notes.iter().min_by(|a, b| {
            a.position_ms
                .partial_cmp(&b.position_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            (first.position_ms, first.midi_key())
        } else {
            (0.0, 60)
        };

        let row_idx = (state.max_midi.saturating_sub(target_midi)) as f32;
        let target_y = (row_idx * state.row_height - 180.0).max(0.0);
        let target_x = ((first_note_pos * state.px_per_ms as f64) as f32 - 100.0).max(0.0);

        state.vertical_scroll_offset = target_y;
        state.horizontal_scroll_offset = target_x;
        scroll_area = scroll_area
            .vertical_scroll_offset(state.vertical_scroll_offset)
            .horizontal_scroll_offset(state.horizontal_scroll_offset);
        state.initial_scrolled = true;
    }

    if state.is_playing && state.auto_scroll_mode != AutoScrollMode::Off {
        scroll_area = scroll_area.horizontal_scroll_offset(state.horizontal_scroll_offset);
    }

    if is_mod_zoom || state.is_scrubbing_ruler || is_middle_panning {
        scroll_area = scroll_area
            .horizontal_scroll_offset(state.horizontal_scroll_offset)
            .vertical_scroll_offset(state.vertical_scroll_offset);
    }

    let scroll_output = scroll_area.show(ui, |ui| {
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(grid_width, grid_height), Sense::click_and_drag());

        let painter = ui.painter_at(rect);

        painter.rect_filled(rect, Rounding::ZERO, theme.bg_canvas_c32());

        let grid_start_y = rect.min.y;
        let grid_end_y = rect.max.y;

        let visible_clip = ui.clip_rect();
        let first_visible_key = (((visible_clip.min.y - grid_start_y) / state.row_height).floor()
            as isize)
            .max(0) as usize;
        let last_visible_key = (((visible_clip.max.y - grid_start_y) / state.row_height).ceil()
            as isize)
            .clamp(0, key_count as isize) as usize;

        for key_idx in first_visible_key..last_visible_key {
            let midi = state.max_midi - key_idx as u8;
            let y_top = grid_start_y + key_idx as f32 * state.row_height;
            let y_bottom = y_top + state.row_height;

            let is_black_key = matches!(midi % 12, 1 | 3 | 6 | 8 | 10);
            let in_scale = state.active_scale.is_in_scale(state.scale_root_key, midi);
            let is_tonic = state.active_scale
                != crate::gui::piano_roll::state::MusicalScale::Chromatic
                && state.active_scale.is_tonic(state.scale_root_key, midi);

            let row_color = if is_black_key {
                theme.bg_row_black_key_c32()
            } else {
                theme.bg_row_white_key_c32()
            };

            let row_x_min = rect.min.x + keyboard_width;
            let row_x_max = rect.max.x;

            if row_x_min < row_x_max {
                let row_rect =
                    Rect::from_min_max(Pos2::new(row_x_min, y_top), Pos2::new(row_x_max, y_bottom));
                painter.rect_filled(row_rect, Rounding::ZERO, row_color);

                // Scale Assistant background tint
                if state.active_scale != crate::gui::piano_roll::state::MusicalScale::Chromatic {
                    if !in_scale {
                        painter.rect_filled(
                            row_rect,
                            Rounding::ZERO,
                            theme.scale_out_of_key_tint(),
                        );
                    } else if is_tonic {
                        painter.rect_filled(row_rect, Rounding::ZERO, theme.scale_tonic_tint());
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

        grid::draw_timeline_grid(
            &painter,
            state,
            theme,
            rect,
            visible_clip,
            keyboard_width,
            grid_start_y,
            grid_end_y,
            total_canvas_ms,
            bpm,
            snap_option,
        );

        let mut note_to_delete: Option<usize> = None;
        let mut note_to_slice: Option<(usize, f64)> = None;
        let mut commit_lyric_edit: Option<(usize, String, Option<usize>)> = None;
        let mut pending_lyric_edit: Option<(usize, Rect, f32, f32)> = None;

        let note_info: Vec<(u8, f64, f64, f64)> = notes
            .iter()
            .map(|n| {
                let first_t = n
                    .pitch_bend
                    .points
                    .first()
                    .map(|p| p.time_offset_ms)
                    .unwrap_or_else(|| n.pitch_bend.portamento_start_ms.clamp(-2000.0, 2000.0));
                (n.midi_key(), n.position_ms, n.duration_ms, first_t)
            })
            .collect();

        let vibrato_buttons: Vec<_> = notes
            .iter()
            .enumerate()
            .filter(|(index, note)| {
                note.midi_key() >= state.min_midi
                    && note.midi_key() <= state.max_midi
                    && state.editing_lyric_index != Some(*index)
            })
            .map(|(index, note)| {
                (
                    index,
                    vibrato_button::rect(
                        note,
                        state,
                        Pos2::new(rect.min.x + keyboard_width, grid_start_y),
                    ),
                )
            })
            .filter(|(_, button)| {
                visible_clip.contains_rect(*button)
                    && button.left() >= visible_clip.left() + keyboard_width
            })
            .collect();
        let pointer = ui.input(|i| i.pointer.interact_pos());
        let vibrato_interaction = ui.memory(|memory| memory.any_popup_open())
            || pointer
                .is_some_and(|pos| vibrato_buttons.iter().any(|(_, rect)| rect.contains(pos)));
        let mouse_interact_pos = pointer.filter(|_| !vibrato_interaction);
        let mut interacted_with_note_or_ui = vibrato_interaction;
        let mut pending_lyric_tags: Vec<(Rect, Color32, String, Color32)> = Vec::new();
        let mut pending_phoneme_badges: Vec<(Rect, String)> = Vec::new();
        let mut pending_mode_badges: Vec<(Rect, String)> = Vec::new();

        for (idx, note) in notes.iter_mut().enumerate() {
            let note_midi = note.midi_key();
            if note_midi < state.min_midi || note_midi > state.max_midi {
                continue;
            }

            let key_idx = (state.max_midi - note_midi) as f32;
            let y_top = grid_start_y + key_idx * state.row_height + 1.0;
            let y_bottom = y_top + state.row_height - 2.0;
            let y_center = (y_top + y_bottom) * 0.5;

            let x_start =
                rect.min.x + keyboard_width + (note.position_ms * state.px_per_ms as f64) as f32;
            let x_end = x_start + (note.duration_ms * state.px_per_ms as f64) as f32;

            let note_rect =
                Rect::from_min_max(Pos2::new(x_start, y_top), Pos2::new(x_end, y_bottom));

            let extended_clip = visible_clip.expand2(Vec2::new(300.0, 40.0));
            if !note_rect.intersects(extended_clip) {
                continue;
            }

            let is_selected = state.selected_note_index == Some(idx)
                || state.selected_note_indices.contains(&idx);
            let is_editing_lyric = state.editing_lyric_index == Some(idx);
            let is_active_playback = state.is_playing
                && state.playhead_ms >= note.position_ms
                && state.playhead_ms <= note.position_ms + note.duration_ms;

            // Velocity dynamic modulation
            let vel_factor = (note.expressions.velocity as f32 / 100.0).clamp(0.65, 1.35);
            let base_color = if is_selected {
                theme.note_selected_fill_c32()
            } else {
                theme.note_fill_c32()
            };
            let note_color = Color32::from_rgba_unmultiplied(
                ((base_color.r() as f32 * vel_factor).min(255.0)) as u8,
                ((base_color.g() as f32 * vel_factor).min(255.0)) as u8,
                ((base_color.b() as f32 * vel_factor).min(255.0)) as u8,
                base_color.a(),
            );

            // Active playback pulsing glow halo
            if is_active_playback {
                painter.rect_stroke(
                    note_rect.expand(2.5),
                    theme.note_rounding(),
                    Stroke::new(3.0_f32, theme.playhead_c32()),
                );
            }

            // Main body
            painter.rect_filled(note_rect, theme.note_rounding(), note_color);

            // Top highlight bevel strip (glassmorphic shine)
            if note_rect.height() > 8.0 && note_rect.width() > 6.0 {
                let highlight_h = (note_rect.height() * 0.35).min(5.0);
                let highlight_rect = Rect::from_min_max(
                    Pos2::new(note_rect.min.x + 1.0, note_rect.min.y + 1.0),
                    Pos2::new(note_rect.max.x - 1.0, note_rect.min.y + highlight_h),
                );
                painter.rect_filled(
                    highlight_rect,
                    Rounding {
                        nw: theme.note_corner_radius.max(0.0),
                        ne: theme.note_corner_radius.max(0.0),
                        se: 0.0,
                        sw: 0.0,
                    },
                    theme.note_highlight_c32(is_selected),
                );

                // Bottom shadow bevel line
                let shadow_rect = Rect::from_min_max(
                    Pos2::new(note_rect.min.x + 1.0, note_rect.max.y - 2.0),
                    Pos2::new(note_rect.max.x - 1.0, note_rect.max.y - 0.5),
                );
                painter.rect_filled(
                    shadow_rect,
                    Rounding {
                        nw: 0.0,
                        ne: 0.0,
                        se: theme.note_corner_radius.max(0.0),
                        sw: theme.note_corner_radius.max(0.0),
                    },
                    theme.note_shadow_c32(is_selected),
                );
            }

            painter.rect_stroke(
                note_rect,
                theme.note_rounding(),
                theme.note_stroke(is_selected),
            );

            if let Some(ref dragged_alias) = phoneme_state.dragged_phoneme {
                if let Some(mpos) = mouse_interact_pos {
                    if note_rect.contains(mpos) {
                        painter.rect_stroke(
                            note_rect,
                            theme.note_rounding(),
                            Stroke::new(2.5_f32, theme.accent_c32()),
                        );

                        if !ui.input(|i| i.pointer.primary_down()) {
                            commit_lyric_edit = Some((idx, dragged_alias.clone(), None));
                            phoneme_state.dragged_phoneme = None;
                        }
                    }
                }
            }

            if is_editing_lyric {
                let edit_rect = Rect::from_min_size(
                    Pos2::new(x_start + 1.0, y_top + 1.0),
                    Vec2::new((note_rect.width() - 2.0).max(95.0), state.row_height - 2.0),
                );
                pending_lyric_edit = Some((idx, edit_rect, x_start, y_bottom));
            } else {
                let pill_rect = Rect::from_min_size(
                    Pos2::new(x_start + 4.0, y_top + 3.0),
                    Vec2::new(
                        (note.lyric.len() as f32 * 8.0 + 12.0).min(note_rect.width() - 8.0),
                        state.row_height - 10.0,
                    ),
                );
                let pill_bg = if is_selected {
                    Color32::from_rgb(15, 15, 20) // Solid dark for high contrast
                } else {
                    Color32::from_rgb(26, 18, 8) // Opaque dark so pitch line NEVER shows through
                };

                let text_color = if is_selected {
                    Color32::WHITE
                } else {
                    MelodyneTheme::TEXT_GOLD_LABEL
                };

                pending_lyric_tags.push((pill_rect, pill_bg, note.lyric.clone(), text_color));
            }

            let lyric_trimmed = note.lyric.trim();
            if let Some(mode) = note.phonemizer_override.as_deref() {
                let badge_rect = Rect::from_min_size(
                    Pos2::new(x_start + 4.0, y_top - 17.0),
                    Vec2::new(18.0, 13.0),
                );
                pending_mode_badges.push((badge_rect, mode.to_string()));
            }
            if lyric_trimmed != "+" && !lyric_trimmed.starts_with("+ ") {
                if let Some(phoneme) = state.phoneme_cache.get(idx) {
                    if !phoneme.is_empty() && phoneme != lyric_trimmed {
                        let pill_w = (phoneme.len() as f32 * 6.5 + 10.0).max(20.0);
                        let pill_h = 15.0f32;
                        let pill_cx = (x_start + x_end) * 0.5;
                        let pill_rect = Rect::from_center_size(
                            Pos2::new(pill_cx, y_top - pill_h * 0.5 - 1.0),
                            Vec2::new(pill_w, pill_h),
                        );
                        pending_phoneme_badges.push((pill_rect, phoneme.clone()));
                    }
                }
            }

            let e = &note.envelope;
            if e.crossfade_ms > 0.0 {
                let crossfade_end_x = (x_start
                    + (e.crossfade_ms.min(note.duration_ms) * state.px_per_ms as f64) as f32)
                    .min(x_end);
                let crossfade_rect = Rect::from_min_max(
                    Pos2::new(x_start, y_top),
                    Pos2::new(crossfade_end_x, y_bottom),
                );
                painter.rect_filled(
                    crossfade_rect,
                    Rounding::ZERO,
                    Color32::from_rgba_unmultiplied(0, 200, 180, 26),
                );
                let cross_color = Color32::from_rgb(0, 220, 195);
                painter.line_segment(
                    [
                        Pos2::new(x_start, y_bottom),
                        Pos2::new(crossfade_end_x, y_top),
                    ],
                    Stroke::new(1.5_f32, cross_color),
                );
                painter.line_segment(
                    [
                        Pos2::new(x_start, y_top),
                        Pos2::new(crossfade_end_x, y_bottom),
                    ],
                    Stroke::new(1.5_f32, cross_color),
                );
                if is_selected && crossfade_end_x - x_start > 24.0 {
                    painter.text(
                        crossfade_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("X {:.0}ms", e.crossfade_ms),
                        egui::FontId::proportional(9.0),
                        Color32::from_rgb(220, 255, 245),
                    );
                }
            }
            let env_pts = [
                (e.p1, e.v1),
                (e.p1 + e.p2, e.v2),
                (e.p1 + e.p2 + e.p3, e.v3),
                ((note.duration_ms - e.p4).max(0.0), e.v4),
                ((note.duration_ms - e.p4 + e.p5).max(0.0), e.v5),
            ];

            let mut env_screen_pts = Vec::with_capacity(5);
            for (t_ms, vol) in env_pts.iter() {
                let px_x = x_start + (*t_ms * state.px_per_ms as f64) as f32;
                let px_y = y_bottom - (*vol / 100.0).clamp(0.0, 1.0) as f32 * (y_bottom - y_top);
                env_screen_pts.push(Pos2::new(px_x, px_y));
            }

            let env_color = if is_selected {
                Color32::from_rgba_unmultiplied(0, 225, 250, 175)
            } else {
                Color32::from_rgba_unmultiplied(0, 180, 220, 50)
            };

            if let Some(first) = env_screen_pts.first() {
                painter.line_segment(
                    [Pos2::new(x_start, y_bottom), *first],
                    Stroke::new(1.3_f32, env_color),
                );
            }
            for i in 0..env_screen_pts.len().saturating_sub(1) {
                painter.line_segment(
                    [env_screen_pts[i], env_screen_pts[i + 1]],
                    Stroke::new(1.3_f32, env_color),
                );
            }
            if let Some(last) = env_screen_pts.last() {
                painter.line_segment(
                    [*last, Pos2::new(x_end, y_bottom)],
                    Stroke::new(1.3_f32, env_color),
                );
            }

            if is_selected && state.show_envelope_handles {
                for (pt_i, pt) in env_screen_pts.iter().enumerate() {
                    let is_pt_hover = mouse_interact_pos.is_some_and(|m| m.distance(*pt) <= 12.0);
                    let is_pt_drag = state.dragging_envelope_pt == Some((idx, pt_i));
                    painter.circle_filled(
                        *pt,
                        if is_pt_hover || is_pt_drag { 7.5 } else { 5.0 },
                        if is_pt_drag {
                            Color32::WHITE
                        } else {
                            Color32::from_rgb(0, 235, 255)
                        },
                    );
                    painter.circle_stroke(
                        *pt,
                        if is_pt_hover || is_pt_drag { 7.5 } else { 5.0 },
                        Stroke::new(
                            1.0_f32,
                            if is_pt_drag {
                                Color32::from_rgb(0, 210, 240)
                            } else {
                                Color32::from_rgb(15, 25, 40)
                            },
                        ),
                    );
                }
            }

            let has_vibrato = note.vibrato.length_pct > 0.0 && note.vibrato.depth_cents > 0.0;
            let vib_start_t = if has_vibrato {
                note.duration_ms * (1.0 - (note.vibrato.length_pct / 100.0).clamp(0.0, 1.0))
            } else {
                f64::INFINITY
            };

            if has_vibrato && vib_start_t < note.duration_ms {
                let vib_px_start =
                    (x_start + (vib_start_t * state.px_per_ms as f64) as f32).max(x_start);
                let vib_px_end = x_end;
                if vib_px_end > vib_px_start {
                    let vib_rect = Rect::from_min_max(
                        Pos2::new(vib_px_start, y_top),
                        Pos2::new(vib_px_end, y_bottom),
                    );
                    painter.rect_filled(
                        vib_rect,
                        Rounding::same(4.0),
                        Color32::from_rgba_unmultiplied(
                            0,
                            220,
                            255,
                            if is_selected { 35 } else { 18 },
                        ),
                    );
                }
            }

            // Find chronologically previous and next connected notes in the phrase
            let prev_connected = note_info
                .iter()
                .enumerate()
                .filter(|(i, &(_, p_pos, _, _))| *i != idx && p_pos <= note.position_ms)
                .max_by(|(_, (_, a_pos, a_dur, _)), (_, (_, b_pos, b_dur, _))| {
                    (a_pos + a_dur)
                        .partial_cmp(&(b_pos + b_dur))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

            let (previous_midi, is_adjacent, prev_end_offset) =
                if let Some((_, &(prev_m, prev_pos, prev_dur, _))) = prev_connected {
                    let gap = note.position_ms - (prev_pos + prev_dur);
                    let connected = gap <= 400.0 && note.position_ms >= prev_pos;
                    (Some(prev_m), connected, -(gap.max(0.0)))
                } else {
                    (None, false, 0.0)
                };

            let pitch_curve =
                note.pitch_bend
                    .effective_points(previous_midi, note.midi_key(), is_adjacent);

            let first_pt_t = pitch_curve.first().map(|p| p.time_offset_ms).unwrap_or(0.0);
            let min_t = if is_adjacent {
                first_pt_t.min(prev_end_offset)
            } else {
                first_pt_t.min(0.0)
            };

            let next_connected = note_info
                .iter()
                .enumerate()
                .filter(|(i, &(_, n_pos, _, _))| *i != idx && n_pos >= note.position_ms)
                .min_by(|(_, (_, a_pos, _, _)), (_, (_, b_pos, _, _))| {
                    a_pos
                        .partial_cmp(b_pos)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });

            let max_t =
                if let Some((_, &(_next_m, next_pos, _next_dur, next_first_t))) = next_connected {
                    let next_gap = next_pos - (note.position_ms + note.duration_ms);
                    if next_gap <= 400.0 {
                        let next_start = next_pos + next_first_t;
                        (next_start - note.position_ms)
                            .max(min_t)
                            .max(note.duration_ms)
                    } else {
                        note.duration_ms
                    }
                } else {
                    note.duration_ms
                };

            let visible_note_start = ((visible_clip.min.x - x_start) / state.px_per_ms) as f64;
            let visible_note_end = ((visible_clip.max.x - x_start) / state.px_per_ms) as f64;
            let draw_min_t = min_t.max(visible_note_start);
            let draw_max_t = max_t.min(visible_note_end);
            let step = (2.0 / state.px_per_ms.max(0.001)) as f64;
            let capacity = if draw_max_t > draw_min_t {
                ((draw_max_t - draw_min_t) / step).ceil() as usize + 2
            } else {
                0
            };
            let mut spline_points = Vec::with_capacity(capacity);
            let mut t = draw_min_t;
            while t <= draw_max_t {
                let mut offset_cents =
                    PitchBendSolver::get_pitch_offset_cents_sorted(t, &pitch_curve);

                if has_vibrato && t >= vib_start_t {
                    let vib_t = t - vib_start_t;
                    let vib_dur = (note.duration_ms - vib_start_t).max(1.0);
                    let fade_in_dur = vib_dur * (note.vibrato.fade_in_pct / 100.0).clamp(0.0, 1.0);
                    let fade_in = if fade_in_dur > 0.0 {
                        (vib_t / fade_in_dur).min(1.0)
                    } else {
                        1.0
                    };
                    let fade_out_dur =
                        vib_dur * (note.vibrato.fade_out_pct / 100.0).clamp(0.0, 1.0);
                    let fade_out_start = vib_dur - fade_out_dur;
                    let fade_out = if vib_t >= fade_out_start && fade_out_dur > 0.0 {
                        ((vib_dur - vib_t) / fade_out_dur).clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
                    let period_ms = note.vibrato.period_ms.clamp(40.0, 1000.0);
                    let phase = (vib_t / period_ms) * std::f64::consts::TAU
                        + (note.vibrato.shift_pct / 100.0) * std::f64::consts::TAU;
                    let vib_mod = phase.sin() * note.vibrato.depth_cents * fade_in * fade_out;
                    offset_cents += vib_mod;
                }

                let px_x = x_start + (t * state.px_per_ms as f64) as f32;
                let px_y = y_center - (offset_cents / 100.0) as f32 * state.row_height;
                spline_points.push(Pos2::new(px_x, px_y));
                t += step;
            }

            let in_pitch_mode = state.active_tool == EditTool::PitchDraw;
            if spline_points.len() >= 2 {
                let (halo_stroke, core_stroke) = if in_pitch_mode {
                    (
                        Stroke::new(3.5_f32, theme.c32_alpha(theme.pitch_curve_color, 0.45)),
                        Stroke::new(2.0_f32, theme.pitch_curve_c32()),
                    )
                } else {
                    (
                        Stroke::new(3.0_f32, theme.c32_alpha(theme.pitch_curve_color, 0.30)),
                        Stroke::new(1.6_f32, theme.pitch_curve_c32()),
                    )
                };
                painter.add(egui::Shape::line(spline_points.clone(), halo_stroke));
                painter.add(egui::Shape::line(spline_points, core_stroke));
            }

            let show_pitch_anchors = in_pitch_mode || state.active_tool == EditTool::Pointer;
            if show_pitch_anchors {
                let mut last_rendered_x: Option<f32> = None;
                for (pt_idx, pt) in pitch_curve.iter().enumerate() {
                    let px_x = x_start + (pt.time_offset_ms * state.px_per_ms as f64) as f32;
                    let px_y = y_center - (pt.pitch_offset_cents / 100.0) as f32 * state.row_height;
                    let pt_pos = Pos2::new(px_x, px_y);
                    if visible_clip.contains(pt_pos)
                        || (px_x >= visible_clip.min.x && px_x <= visible_clip.max.x)
                    {
                        let is_active_pt = state.dragging_pitch_pt == Some((idx, pt_idx));
                        let is_hovered = mouse_interact_pos
                            .map(|m| m.distance(pt_pos) <= 10.0)
                            .unwrap_or(false);

                        let is_first_or_last = pt_idx == 0 || pt_idx + 1 == pitch_curve.len();
                        let dist_ok =
                            last_rendered_x.map_or(true, |last_x| (px_x - last_x).abs() >= 8.0);

                        if in_pitch_mode
                            || is_active_pt
                            || is_hovered
                            || is_first_or_last
                            || dist_ok
                        {
                            last_rendered_x = Some(px_x);

                            let radius = if is_active_pt {
                                4.5
                            } else if is_hovered {
                                4.0
                            } else {
                                2.8
                            };

                            if is_active_pt {
                                painter.circle_filled(
                                    pt_pos,
                                    radius + 3.0,
                                    Color32::from_rgba_unmultiplied(255, 215, 80, 100),
                                );
                                painter.circle_filled(
                                    pt_pos,
                                    radius,
                                    Color32::from_rgb(255, 240, 160),
                                );
                                painter.circle_stroke(
                                    pt_pos,
                                    radius,
                                    Stroke::new(1.2_f32, Color32::WHITE),
                                );
                            } else if is_hovered {
                                painter.circle_filled(
                                    pt_pos,
                                    radius + 3.0,
                                    Color32::from_rgba_unmultiplied(0, 230, 255, 120),
                                );
                                painter.circle_filled(
                                    pt_pos,
                                    radius,
                                    Color32::from_rgb(180, 245, 255),
                                );
                                painter.circle_stroke(
                                    pt_pos,
                                    radius,
                                    Stroke::new(1.2_f32, Color32::WHITE),
                                );
                            } else {
                                painter.circle_filled(
                                    pt_pos,
                                    radius,
                                    Color32::from_rgb(255, 220, 100),
                                );
                                painter.circle_stroke(
                                    pt_pos,
                                    radius,
                                    Stroke::new(
                                        1.0_f32,
                                        Color32::from_rgba_unmultiplied(20, 15, 28, 220),
                                    ),
                                );
                            }

                            if is_hovered || is_active_pt {
                                let shape_txt = match pt.shape.to_lowercase().as_str() {
                                    "l" => "Linear (l)",
                                    "j" | "i" => "J-Curve (j)",
                                    "r" | "o" => "R-Curve (r)",
                                    _ => "S-Curve (s)",
                                };
                                let cents_label = format!("{:+0.0} c", pt.pitch_offset_cents);
                                painter.text(
                                    Pos2::new(pt_pos.x, pt_pos.y - 12.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    format!("{} ({})", cents_label, shape_txt),
                                    egui::FontId::proportional(9.0),
                                    Color32::from_rgb(220, 240, 255),
                                );
                            }
                        }
                    }
                }
            }

            if in_pitch_mode && !state.pitch_brush_raw_stroke.is_empty() {
                let stroke_pts: Vec<Pos2> = state
                    .pitch_brush_raw_stroke
                    .iter()
                    .filter(|(n_idx, _, _)| *n_idx == idx)
                    .map(|(_, t, c)| {
                        let px_x = x_start + (t * state.px_per_ms as f64) as f32;
                        let px_y = y_center - (c / 100.0) as f32 * state.row_height;
                        Pos2::new(px_x, px_y)
                    })
                    .collect();
                if stroke_pts.len() >= 2 {
                    for pair in stroke_pts.windows(2) {
                        painter.line_segment(
                            [pair[0], pair[1]],
                            Stroke::new(3.5_f32, Color32::from_rgba_unmultiplied(255, 215, 0, 160)),
                        );
                        painter.line_segment(
                            [pair[0], pair[1]],
                            Stroke::new(1.8_f32, Color32::from_rgb(255, 255, 255)),
                        );
                    }
                }
            }

            if in_pitch_mode && state.pitch_sub_tool == PitchSubTool::Line {
                if let Some((line_note_idx, l_start_t, l_start_cents)) = state.pitch_line_start {
                    if line_note_idx == idx {
                        if let Some(mpos) = mouse_interact_pos {
                            let start_px_x = x_start + (l_start_t * state.px_per_ms as f64) as f32;
                            let start_px_y =
                                y_center - (l_start_cents / 100.0) as f32 * state.row_height;
                            let end_px_x = mpos.x;
                            let end_px_y = mpos.y;
                            painter.line_segment(
                                [
                                    Pos2::new(start_px_x, start_px_y),
                                    Pos2::new(end_px_x, end_px_y),
                                ],
                                Stroke::new(1.5_f32, Color32::from_rgb(255, 220, 100)),
                            );
                        }
                    }
                }
            }

            if let Some(mpos) = mouse_interact_pos {
                let resize_handle_right = Rect::from_min_max(
                    Pos2::new(x_end - 8.0, y_top),
                    Pos2::new(x_end + 4.0, y_bottom),
                );

                let resize_handle_left = Rect::from_min_max(
                    Pos2::new(x_start - 4.0, y_top),
                    Pos2::new(x_start + 8.0, y_bottom),
                );

                if visible_clip.contains(mpos)
                    && note_rect.contains(mpos)
                    && (state.active_tool == EditTool::Pointer
                        || state.active_tool == EditTool::Pencil)
                    && (resize_handle_left.contains(mpos) || resize_handle_right.contains(mpos))
                {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                }

                let min_pitch_t = pitch_curve
                    .iter()
                    .map(|p| p.time_offset_ms)
                    .fold(0.0f64, f64::min);
                let max_pitch_t = pitch_curve
                    .iter()
                    .map(|p| p.time_offset_ms)
                    .fold(note.duration_ms, f64::max);
                let min_pitch_c = pitch_curve
                    .iter()
                    .map(|p| p.pitch_offset_cents)
                    .fold(0.0f64, f64::min);
                let max_pitch_c = pitch_curve
                    .iter()
                    .map(|p| p.pitch_offset_cents)
                    .fold(0.0f64, f64::max);

                let pitch_draw_target_rect = Rect::from_min_max(
                    Pos2::new(
                        (x_start + (min_pitch_t * state.px_per_ms as f64) as f32 - 150.0)
                            .min(x_start - 300.0),
                        (y_center
                            - (max_pitch_c / 100.0) as f32 * state.row_height
                            - state.row_height * 4.0)
                            .min(y_top - state.row_height * 6.0),
                    ),
                    Pos2::new(
                        (x_start + (max_pitch_t * state.px_per_ms as f64) as f32 + 150.0)
                            .max(x_end + 300.0),
                        (y_center - (min_pitch_c / 100.0) as f32 * state.row_height
                            + state.row_height * 4.0)
                            .max(y_bottom + state.row_height * 6.0),
                    ),
                );

                // Pitch anchors have exclusive pointer ownership. Mark the
                // canvas as consumed before note selection/marquee handling
                // runs in this frame.
                let pitch_anchor_hit = (state.active_tool == EditTool::PitchDraw
                    || state.active_tool == EditTool::Pointer)
                    && pitch_curve.iter().any(|pt| {
                        let px_x = x_start + (pt.time_offset_ms * state.px_per_ms as f64) as f32;
                        let px_y =
                            y_center - (pt.pitch_offset_cents / 100.0) as f32 * state.row_height;
                        mpos.distance(Pos2::new(px_x, px_y)) <= 16.0
                    });
                if pitch_anchor_hit {
                    interacted_with_note_or_ui = true;
                }

                if note_rect.contains(mpos)
                    && !pitch_anchor_hit
                    && mpos.y > grid_start_y
                    && mpos.y < grid_end_y
                {
                    interacted_with_note_or_ui = true;

                    if state.active_tool == EditTool::Pointer
                        || state.active_tool == EditTool::Pencil
                    {
                        let is_double_clicked = ui.input(|i| {
                            i.pointer
                                .button_double_clicked(egui::PointerButton::Primary)
                        });
                        let is_enter_on_note = !was_editing_lyric
                            && ui.input(|i| {
                                i.key_pressed(egui::Key::Enter)
                                    && !i.modifiers.command
                                    && !i.modifiers.alt
                            });
                        if is_double_clicked || is_enter_on_note {
                            state.editing_lyric_index = Some(idx);
                            state.lyric_buffer = note.lyric.clone();
                            state.autocomplete_selected_idx = 0;
                            state.lyric_needs_select_all = true;
                            state.selected_note_index = Some(idx);
                            state.selected_note_indices.clear();
                            state.selected_note_indices.insert(idx);
                        }

                        let hovered_env_pt = if is_selected && state.show_envelope_handles {
                            env_screen_pts
                                .iter()
                                .position(|pt| mpos.distance(*pt) <= 13.0)
                        } else {
                            None
                        };
                        let is_over_envelope_handle = is_selected
                            && state.show_envelope_handles
                            && (state.dragging_envelope_pt.is_some() || hovered_env_pt.is_some());
                        let is_over_pitch_anchor = (state.active_tool == EditTool::PitchDraw
                            || state.active_tool == EditTool::Pointer)
                            && (state.dragging_pitch_pt.is_some()
                                || pitch_curve.iter().any(|pt| {
                                    let px_x = x_start
                                        + (pt.time_offset_ms * state.px_per_ms as f64) as f32;
                                    let px_y = y_center
                                        - (pt.pitch_offset_cents / 100.0) as f32 * state.row_height;
                                    mpos.distance(Pos2::new(px_x, px_y)) <= 16.0
                                }));

                        let just_pressed = ui.input(|i| i.pointer.primary_pressed());
                        if just_pressed && hovered_env_pt.is_some() {
                            if let Some(pt_idx) = hovered_env_pt {
                                on_before_change();
                                state.dragging_envelope_pt = Some((idx, pt_idx));
                            }
                        }

                        if let Some((drag_idx, pt_idx)) = state.dragging_envelope_pt {
                            if drag_idx == idx && ui.input(|i| i.pointer.primary_down()) {
                                let duration = note.duration_ms.max(1.0);
                                let time = (f64::from(mpos.x - x_start)
                                    / f64::from(state.px_per_ms))
                                .max(0.0);
                                let volume = ((y_bottom - mpos.y) / (y_bottom - y_top))
                                    .clamp(0.0, 1.0)
                                    as f64
                                    * 100.0;
                                match pt_idx {
                                    0 => {
                                        note.envelope.p1 = time.clamp(0.0, duration);
                                        note.envelope.v1 = volume.clamp(0.0, 100.0);
                                    }
                                    1 => {
                                        let p2_time = time.max(note.envelope.p1);
                                        note.envelope.p2 = (p2_time - note.envelope.p1).max(0.0);
                                        note.envelope.v2 = volume.clamp(0.0, 100.0);
                                    }
                                    2 => {
                                        let p3_time = time.max(note.envelope.p1 + note.envelope.p2);
                                        note.envelope.p3 =
                                            (p3_time - note.envelope.p1 - note.envelope.p2)
                                                .max(0.0);
                                        note.envelope.v3 = volume.clamp(0.0, 100.0);
                                    }
                                    3 => {
                                        let p4_pos = time.clamp(0.0, duration + 200.0);
                                        note.envelope.p4 = (duration - p4_pos).max(0.0);
                                        note.envelope.v4 = volume.clamp(0.0, 100.0);
                                    }
                                    4 => {
                                        let p4_pos = (duration - note.envelope.p4).max(0.0);
                                        let p5_pos = time.max(p4_pos);
                                        note.envelope.p5 = (p5_pos - p4_pos).max(0.0);
                                        note.envelope.v5 = volume.clamp(0.0, 100.0);
                                    }
                                    5 => {
                                        note.envelope.crossfade_ms = (f64::from(x_start - mpos.x)
                                            / f64::from(state.px_per_ms))
                                        .clamp(0.0, 600.0);
                                    }
                                    _ => {}
                                }
                                state.continuous_edit_dirty = true;
                            }
                        }

                        // Use primary_pressed() (not primary_clicked()) so drag starts
                        // on mouse-down, enabling click-and-drag for resize/move
                        if just_pressed
                            && !is_editing_lyric
                            && state.dragging_note_idx.is_none()
                            && state.dragging_pitch_pt.is_none()
                            && !is_over_envelope_handle
                            && !is_over_pitch_anchor
                        {
                            on_before_change();
                            state.selected_note_index = Some(idx);
                            if !state.selected_note_indices.contains(&idx) {
                                state.selected_note_indices.clear();
                                state.selected_note_indices.insert(idx);
                            }
                            state.dragging_note_idx = Some(idx);
                            state.drag_start_pos = Some(mpos);
                            state.note_original_start_ms = note.position_ms;
                            state.note_original_duration_ms = note.duration_ms;
                            state.note_original_midi = note_midi;
                            state.dragging_is_resize = resize_handle_right.contains(mpos);
                            state.dragging_is_left_resize = resize_handle_left.contains(mpos);

                            state.note_original_states = state
                                .selected_note_indices
                                .iter()
                                .filter_map(|&n_idx| {
                                    note_info
                                        .get(n_idx)
                                        .map(|&(midi, pos, dur, _)| (n_idx, pos, dur, midi))
                                })
                                .collect();
                        }

                        if ui.input(|i| i.pointer.secondary_clicked()) {
                            if !state.selected_note_indices.contains(&idx) {
                                state.selected_note_indices.clear();
                                state.selected_note_indices.insert(idx);
                                state.selected_note_index = Some(idx);
                            }
                            state.context_menu_note_idx = Some(idx);
                            state.context_menu_pos = Some(mpos);
                            state.context_menu_hovered_category = None;
                        }
                    } else if state.active_tool == EditTool::Slice {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                        if ui.input(|i| i.pointer.primary_clicked()) {
                            let click_t =
                                ((mpos.x - (rect.min.x + keyboard_width)) / state.px_per_ms) as f64;
                            let snapped_t =
                                apply_snap_with_zoom(click_t, snap_option, bpm, state.px_per_ms);
                            let slice_t = if (snapped_t - note.position_ms) >= 15.0
                                && (note.position_ms + note.duration_ms - snapped_t) >= 15.0
                            {
                                snapped_t
                            } else {
                                click_t
                            };
                            note_to_slice = Some((idx, slice_t));
                        }
                    } else if state.active_tool == EditTool::Eraser {
                        if ui.input(|i| i.pointer.primary_clicked()) {
                            note_to_delete = Some(idx);
                        }
                    }
                }

                let can_interact_pitch = state.active_tool == EditTool::PitchDraw
                    || state.active_tool == EditTool::Pointer;

                let is_hovering_pitch_pt = pitch_curve.iter().any(|pt| {
                    let px_x = x_start + (pt.time_offset_ms * state.px_per_ms as f64) as f32;
                    let px_y = y_center - (pt.pitch_offset_cents / 100.0) as f32 * state.row_height;
                    mpos.distance(Pos2::new(px_x, px_y)) <= 12.0
                });

                if can_interact_pitch
                    && (state.dragging_pitch_pt.is_some()
                        || is_hovering_pitch_pt
                        || pitch_draw_target_rect.contains(mpos))
                {
                    let mut hovered_pitch_pt: Option<usize> = None;
                    for (pt_idx, pt) in pitch_curve.iter().enumerate() {
                        let px_x = x_start + (pt.time_offset_ms * state.px_per_ms as f64) as f32;
                        let px_y =
                            y_center - (pt.pitch_offset_cents / 100.0) as f32 * state.row_height;
                        let pt_pos = Pos2::new(px_x, px_y);
                        if mpos.distance(pt_pos) <= 16.0 {
                            hovered_pitch_pt = Some(pt_idx);
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                            break;
                        }
                    }

                    let rel_t = ((mpos.x - x_start) / state.px_per_ms) as f64;
                    let delta_y = y_center - mpos.y;
                    let cents =
                        (((delta_y / state.row_height) * 100.0) as f64).clamp(-1200.0, 1200.0);
                    let is_alt = ui.input(|i| i.modifiers.alt);
                    let is_shift = ui.input(|i| i.modifiers.shift);

                    if let Some(pt_idx) = hovered_pitch_pt {
                        if ui.input(|i| i.pointer.primary_clicked() && i.modifiers.alt)
                            || ui.input(|i| i.pointer.secondary_clicked())
                        {
                            if note.pitch_bend.points.is_empty() {
                                note.pitch_bend.points = pitch_curve.clone();
                            }
                            if pt_idx < note.pitch_bend.points.len() {
                                note.pitch_bend.points.remove(pt_idx);
                                state.dragging_pitch_pt = None;
                                on_note_changed();
                            }
                        }
                    } else if state.active_tool == EditTool::PitchDraw
                        && ui.input(|i| i.pointer.secondary_clicked())
                    {
                        note.pitch_bend.points.clear();
                        state.dragging_pitch_pt = None;
                        state.pitch_line_start = None;
                        on_note_changed();
                    }

                    if state.active_tool == EditTool::PitchDraw
                        && hovered_pitch_pt.is_none()
                        && (ui.input(|i| {
                            i.pointer
                                .button_double_clicked(egui::PointerButton::Primary)
                        }) || (ui.input(|i| i.pointer.primary_clicked()) && is_shift))
                    {
                        if note.pitch_bend.points.is_empty() {
                            note.pitch_bend.points = pitch_curve.clone();
                        }
                        note.pitch_bend.points.push(UPitchBendPoint {
                            time_offset_ms: rel_t,
                            pitch_offset_cents: cents,
                            shape: "s".to_string(),
                        });
                        note.pitch_bend.points.sort_by(|a, b| {
                            a.time_offset_ms
                                .partial_cmp(&b.time_offset_ms)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                        on_note_changed();
                    }

                    if ui.input(|i| i.pointer.primary_pressed()) && !is_alt && !is_shift {
                        on_before_change();
                        if let Some(pt_idx) = hovered_pitch_pt {
                            if note.pitch_bend.points.is_empty() {
                                note.pitch_bend.points = pitch_curve.clone();
                            }
                            if pt_idx < note.pitch_bend.points.len() {
                                state.dragging_pitch_pt = Some((idx, pt_idx));
                            }
                        } else if state.active_tool == EditTool::PitchDraw
                            && state.pitch_sub_tool == PitchSubTool::Line
                        {
                            state.pitch_line_start = Some((idx, rel_t, cents));
                        } else if state.active_tool == EditTool::PitchDraw
                            && state.pitch_sub_tool == PitchSubTool::Freehand
                        {
                            state.pitch_brush_raw_stroke.clear();
                            state.pitch_brush_raw_stroke.push((idx, rel_t, cents));
                        }
                    }

                    if ui.input(|i| i.pointer.primary_down()) && !is_alt {
                        if let Some((d_note_idx, d_pt_idx)) = state.dragging_pitch_pt {
                            if d_note_idx == idx {
                                if note.pitch_bend.points.is_empty() {
                                    note.pitch_bend.points = pitch_curve.clone();
                                }
                                if d_pt_idx < note.pitch_bend.points.len() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                    note.pitch_bend.points[d_pt_idx].time_offset_ms = rel_t;
                                    note.pitch_bend.points[d_pt_idx].pitch_offset_cents = cents;
                                    note.pitch_bend.snap_first = false;
                                    state.continuous_edit_dirty = true;
                                }
                            }
                        } else if state.active_tool == EditTool::PitchDraw {
                            match state.pitch_sub_tool {
                                PitchSubTool::Freehand => {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                                    let should_add = state.pitch_brush_raw_stroke.last().map_or(
                                        true,
                                        |&(_, last_t, last_c)| {
                                            (last_t - rel_t).abs() >= 4.0
                                                || (last_c - cents).abs() >= 5.0
                                        },
                                    );
                                    if should_add {
                                        state.pitch_brush_raw_stroke.push((idx, rel_t, cents));
                                    }
                                }
                                PitchSubTool::Line => {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                                }
                                PitchSubTool::Vibrato => {
                                    if note.pitch_bend.points.is_empty() {
                                        note.pitch_bend.points = pitch_curve.clone();
                                    }
                                    let min_dist_ms = 15.0;
                                    let vibrato_freq = 5.5; // ~5.5 Hz
                                    let vibrato_amp =
                                        (delta_y.abs() * 2.5).clamp(25.0, 180.0) as f64;
                                    let phase =
                                        (rel_t / 1000.0) * vibrato_freq * std::f64::consts::TAU;
                                    let vib_cents =
                                        (phase.sin() * vibrato_amp).clamp(-1200.0, 1200.0);

                                    note.pitch_bend.points.retain(|pt| {
                                        (pt.time_offset_ms - rel_t).abs() >= min_dist_ms
                                    });
                                    note.pitch_bend.points.push(UPitchBendPoint {
                                        time_offset_ms: rel_t,
                                        pitch_offset_cents: vib_cents,
                                        shape: "s".to_string(),
                                    });
                                    note.pitch_bend.points.sort_by(|a, b| {
                                        a.time_offset_ms
                                            .partial_cmp(&b.time_offset_ms)
                                            .unwrap_or(std::cmp::Ordering::Equal)
                                    });
                                    state.continuous_edit_dirty = true;
                                }
                                PitchSubTool::Smooth => {
                                    if !note.pitch_bend.points.is_empty() {
                                        let radius_ms = 60.0;
                                        for pt_i in 0..note.pitch_bend.points.len() {
                                            let p_time =
                                                note.pitch_bend.points[pt_i].time_offset_ms;
                                            if (p_time - rel_t).abs() <= radius_ms {
                                                let prev_cents = if pt_i > 0 {
                                                    note.pitch_bend.points[pt_i - 1]
                                                        .pitch_offset_cents
                                                } else {
                                                    note.pitch_bend.points[pt_i].pitch_offset_cents
                                                };
                                                let next_cents = if pt_i + 1
                                                    < note.pitch_bend.points.len()
                                                {
                                                    note.pitch_bend.points[pt_i + 1]
                                                        .pitch_offset_cents
                                                } else {
                                                    note.pitch_bend.points[pt_i].pitch_offset_cents
                                                };
                                                let curr =
                                                    note.pitch_bend.points[pt_i].pitch_offset_cents;
                                                note.pitch_bend.points[pt_i].pitch_offset_cents =
                                                    curr * 0.7 + (prev_cents + next_cents) * 0.15;
                                            }
                                        }
                                        state.continuous_edit_dirty = true;
                                    }
                                }
                            }
                        }
                    }

                    if ui.input(|i| i.pointer.primary_released()) {
                        if state.dragging_pitch_pt.is_some() {
                            state.dragging_pitch_pt = None;
                            note.pitch_bend.points.sort_by(|a, b| {
                                a.time_offset_ms
                                    .partial_cmp(&b.time_offset_ms)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            });
                            state.continuous_edit_dirty = true;
                        } else if state.pitch_sub_tool == PitchSubTool::Freehand
                            && !state.pitch_brush_raw_stroke.is_empty()
                        {
                            let raw_pts: Vec<(f64, f64)> = state
                                .pitch_brush_raw_stroke
                                .iter()
                                .filter(|(n_idx, _, _)| *n_idx == idx)
                                .map(|(_, t, c)| (*t, *c))
                                .collect();

                            if !raw_pts.is_empty() {
                                let smoothed = smooth_pitch_points(&raw_pts);
                                if !smoothed.is_empty() {
                                    if note.pitch_bend.points.is_empty() {
                                        note.pitch_bend.points = pitch_curve.clone();
                                    }
                                    let t_min =
                                        smoothed.first().map(|p| p.time_offset_ms).unwrap_or(0.0);
                                    let t_max =
                                        smoothed.last().map(|p| p.time_offset_ms).unwrap_or(0.0);

                                    note.pitch_bend.points.retain(|pt| {
                                        pt.time_offset_ms < t_min - 8.0
                                            || pt.time_offset_ms > t_max + 8.0
                                    });

                                    note.pitch_bend.points.extend(smoothed);
                                    note.pitch_bend.points.sort_by(|a, b| {
                                        a.time_offset_ms
                                            .partial_cmp(&b.time_offset_ms)
                                            .unwrap_or(std::cmp::Ordering::Equal)
                                    });
                                    state.continuous_edit_dirty = true;
                                    on_note_changed();
                                }
                            }
                            state.pitch_brush_raw_stroke.clear();
                        } else if state.pitch_sub_tool == PitchSubTool::Line {
                            if let Some((l_note_idx, l_start_t, l_start_cents)) =
                                state.pitch_line_start
                            {
                                if l_note_idx == idx {
                                    if note.pitch_bend.points.is_empty() {
                                        note.pitch_bend.points = pitch_curve.clone();
                                    }
                                    let (min_t, max_t) = if l_start_t <= rel_t {
                                        (l_start_t, rel_t)
                                    } else {
                                        (rel_t, l_start_t)
                                    };
                                    note.pitch_bend.points.retain(|pt| {
                                        pt.time_offset_ms < min_t || pt.time_offset_ms > max_t
                                    });

                                    note.pitch_bend.points.push(UPitchBendPoint {
                                        time_offset_ms: l_start_t,
                                        pitch_offset_cents: l_start_cents,
                                        shape: "l".to_string(),
                                    });
                                    note.pitch_bend.points.push(UPitchBendPoint {
                                        time_offset_ms: rel_t,
                                        pitch_offset_cents: cents,
                                        shape: "s".to_string(),
                                    });
                                    note.pitch_bend.points.sort_by(|a, b| {
                                        a.time_offset_ms
                                            .partial_cmp(&b.time_offset_ms)
                                            .unwrap_or(std::cmp::Ordering::Equal)
                                    });
                                    state.continuous_edit_dirty = true;
                                }
                            }
                            state.pitch_line_start = None;
                        } else if note.pitch_bend.points.len() > 2 {
                            let simplify_tol = match state.pitch_sub_tool {
                                PitchSubTool::Vibrato => 2.0,
                                PitchSubTool::Smooth => 3.0,
                                _ => 4.0,
                            };
                            let simplified =
                                crate::dsp::pitch_bend::PitchBendSolver::simplify_pitch_points(
                                    &note.pitch_bend.points,
                                    simplify_tol,
                                );
                            note.pitch_bend.points = simplified;
                            state.continuous_edit_dirty = true;
                        }
                    }
                }
            }
        }

        {
            let mut global_segments: Vec<Vec<Pos2>> = Vec::new();
            let mut current_segment: Vec<Pos2> = Vec::new();

            for (idx, note) in notes.iter().enumerate() {
                let note_midi = note.midi_key();
                if note_midi < state.min_midi || note_midi > state.max_midi {
                    continue;
                }

                let x_start_note = rect.min.x
                    + keyboard_width
                    + (note.position_ms * state.px_per_ms as f64) as f32;
                let x_end_note = x_start_note + (note.duration_ms * state.px_per_ms as f64) as f32;

                // Skip notes completely outside visible area (with margin for
                // portamento that starts before the note)
                if x_end_note < visible_clip.min.x - 200.0
                    || x_start_note > visible_clip.max.x + 200.0
                {
                    if !current_segment.is_empty() {
                        global_segments.push(std::mem::take(&mut current_segment));
                    }
                    continue;
                }

                let (previous_midi, is_adjacent) = if idx > 0 {
                    let (prev_m, prev_pos, prev_dur, _) = note_info[idx - 1];
                    let adj = (prev_pos + prev_dur - note.position_ms).abs() <= 1.0;
                    (Some(prev_m), adj)
                } else {
                    (None, false)
                };

                // Break the segment at gaps between non-adjacent notes
                if !is_adjacent && !current_segment.is_empty() {
                    global_segments.push(std::mem::take(&mut current_segment));
                }

                let pitch_curve =
                    note.pitch_bend
                        .effective_points(previous_midi, note_midi, is_adjacent);

                let min_t = pitch_curve.first().map(|p| p.time_offset_ms).unwrap_or(0.0);
                let max_t = if let Some(&(_next_m, next_pos, _next_dur, next_first_t)) =
                    note_info.get(idx + 1)
                {
                    let next_adjacent =
                        (note.position_ms + note.duration_ms - next_pos).abs() <= 1.0;
                    let next_start = next_pos + next_first_t;
                    if next_adjacent {
                        (next_start - note.position_ms).max(min_t)
                    } else {
                        note.duration_ms
                    }
                } else {
                    note.duration_ms
                };

                let has_vibrato = note.vibrato.length_pct > 0.0 && note.vibrato.depth_cents > 0.0;
                let vib_start_t = if has_vibrato {
                    note.duration_ms * (1.0 - (note.vibrato.length_pct / 100.0).clamp(0.0, 1.0))
                } else {
                    f64::INFINITY
                };

                // Only sample within the visible pixel range for performance
                let visible_note_start =
                    ((visible_clip.min.x - x_start_note) / state.px_per_ms) as f64;
                let visible_note_end =
                    ((visible_clip.max.x - x_start_note) / state.px_per_ms) as f64;
                let draw_min_t = min_t.max(visible_note_start - 20.0);
                let draw_max_t = max_t.min(visible_note_end + 20.0);
                let step = (2.0 / state.px_per_ms.max(0.001)) as f64;

                let mut t = draw_min_t;
                while t <= draw_max_t {
                    let mut offset_cents =
                        PitchBendSolver::get_pitch_offset_cents_sorted(t, &pitch_curve);

                    // Include vibrato modulation
                    if has_vibrato && t >= vib_start_t {
                        let vib_t = t - vib_start_t;
                        let vib_dur = (note.duration_ms - vib_start_t).max(1.0);
                        let fade_in_dur =
                            vib_dur * (note.vibrato.fade_in_pct / 100.0).clamp(0.0, 1.0);
                        let fade_in = if fade_in_dur > 0.0 {
                            (vib_t / fade_in_dur).min(1.0)
                        } else {
                            1.0
                        };
                        let fade_out_dur =
                            vib_dur * (note.vibrato.fade_out_pct / 100.0).clamp(0.0, 1.0);
                        let fade_out_start = vib_dur - fade_out_dur;
                        let fade_out = if vib_t >= fade_out_start && fade_out_dur > 0.0 {
                            ((vib_dur - vib_t) / fade_out_dur).clamp(0.0, 1.0)
                        } else {
                            1.0
                        };
                        let period_ms = note.vibrato.period_ms.clamp(40.0, 1000.0);
                        let phase = (vib_t / period_ms) * std::f64::consts::TAU
                            + (note.vibrato.shift_pct / 100.0) * std::f64::consts::TAU;
                        let vib_mod = phase.sin() * note.vibrato.depth_cents * fade_in * fade_out;
                        offset_cents += vib_mod;
                    }

                    let absolute_midi = note_midi as f64 + offset_cents / 100.0;
                    let px_y = grid_start_y
                        + (state.max_midi as f64 - absolute_midi) as f32 * state.row_height
                        + state.row_height * 0.5;
                    let px_x = x_start_note + (t * state.px_per_ms as f64) as f32;

                    current_segment.push(Pos2::new(px_x, px_y));
                    t += step;
                }
            }

            if !current_segment.is_empty() {
                global_segments.push(current_segment);
            }

            let global_pitch_stroke =
                Stroke::new(1.8_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 175));
            for segment in global_segments {
                if segment.len() >= 2 {
                    painter.add(egui::Shape::line(segment, global_pitch_stroke));
                }
            }
        }

        for (pill_rect, pill_bg, lyric, text_color) in pending_lyric_tags {
            painter.rect_filled(pill_rect, Rounding::same(3.0), pill_bg);
            painter.text(
                pill_rect.center(),
                egui::Align2::CENTER_CENTER,
                &lyric,
                egui::FontId::proportional(11.0),
                text_color,
            );
        }

        for (pill_rect, phoneme) in pending_phoneme_badges {
            painter.rect_filled(pill_rect, Rounding::same(4.0), Color32::from_rgb(10, 8, 20));
            painter.rect_stroke(
                pill_rect,
                Rounding::same(4.0),
                Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(0, 220, 255, 140)),
            );
            painter.text(
                pill_rect.center(),
                egui::Align2::CENTER_CENTER,
                phoneme,
                egui::FontId::proportional(10.5),
                Color32::from_rgb(0, 235, 255),
            );
        }

        for (badge_rect, mode) in pending_mode_badges {
            painter.rect_filled(
                badge_rect,
                Rounding::same(6.0),
                Color32::from_rgba_unmultiplied(80, 180, 220, 170),
            );
            painter.text(
                badge_rect.center(),
                egui::Align2::CENTER_CENTER,
                "ƒ",
                egui::FontId::proportional(10.0),
                Color32::WHITE,
            );
            ui.interact(
                badge_rect,
                ui.make_persistent_id(("phonemizer_mode_badge", &mode)),
                egui::Sense::hover(),
            )
            .on_hover_text(format!("Fonemizador: {mode}"));
        }

        for (index, button_rect) in vibrato_buttons {
            if let Some(note) = notes.get_mut(index) {
                vibrato_button::draw(
                    ui,
                    note,
                    index,
                    button_rect,
                    theme,
                    lang,
                    on_before_change,
                    on_note_changed,
                );
            }
        }

        if let Some((idx, edit_rect, x_start, y_bottom)) = pending_lyric_edit {
            painter.rect_filled(
                edit_rect.expand(2.0),
                Rounding::same(5.0),
                Color32::from_rgb(14, 12, 22),
            );
            painter.rect_stroke(
                edit_rect.expand(2.0),
                Rounding::same(5.0),
                Stroke::new(2.0_f32, Color32::from_rgb(0, 220, 255)),
            );

            let mut text_lost_focus = false;
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(edit_rect), |ui| {
                let text_id = ui.make_persistent_id(format!("note_lyric_text_edit_{}", idx));

                if state.lyric_needs_select_all {
                    let char_count = state.lyric_buffer.chars().count();
                    let mut te_state = egui::text_edit::TextEditState::default();
                    te_state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::two(
                            egui::text::CCursor::new(0),
                            egui::text::CCursor::new(char_count),
                        )));
                    te_state.store(ui.ctx(), text_id);
                    state.lyric_needs_select_all = false;
                }

                let text_resp = ui.add(
                    egui::TextEdit::singleline(&mut state.lyric_buffer)
                        .id(text_id)
                        .text_color(Color32::WHITE)
                        .desired_width(edit_rect.width())
                        .font(egui::FontId::proportional(13.0)),
                );

                let enter_pressed =
                    text_resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                if !enter_pressed
                    && ui.input(|i| !i.key_pressed(Key::Enter) && !i.key_pressed(Key::Escape))
                {
                    text_resp.request_focus();
                }
                text_lost_focus = text_resp.lost_focus();
            });

            if let Some(vb) = voicebank {
                let query = active_phoneme_query(&state.lyric_buffer);
                let has_separator = state
                    .lyric_buffer
                    .chars()
                    .any(|character| PHONEME_SEPARATORS.contains(&character));
                let cache_key = format!(
                    "{}\0{}\0{}\0{}",
                    vb.root_path.display(),
                    vb.entries.len(),
                    has_separator,
                    query
                );
                let query_changed = cache_key != state.autocomplete_cache_key;
                if query_changed {
                    state.autocomplete_cache_key = cache_key;
                    state.autocomplete_candidates.clear();
                }
                if query_changed && (!query.is_empty() || has_separator) {
                    let mut seen = HashSet::new();
                    let matches = vb.search_entries(query, "All Folders");
                    for (alias, _) in matches {
                        if seen.insert(alias.clone()) {
                            state.autocomplete_candidates.push(alias.clone());
                        }
                    }
                    state.autocomplete_candidates.sort();
                    // A janela exibe poucas linhas; limitar o cache evita cópias
                    // enormes ao listar um voicebank inteiro após um separador.
                    state.autocomplete_candidates.truncate(200);
                }
            } else {
                state.autocomplete_cache_key.clear();
                state.autocomplete_candidates.clear();
            }
            let candidates = state.autocomplete_candidates.clone();
            if !candidates.is_empty() {
                state.autocomplete_selected_idx %= candidates.len();
            }

            let popup_rect = if !candidates.is_empty() {
                let popup_pos = Pos2::new(x_start, y_bottom + 4.0);
                let popup_height = (candidates.len() as f32 * 20.0 + 24.0).min(170.0);
                Some(Rect::from_min_size(
                    popup_pos,
                    Vec2::new(180.0, popup_height),
                ))
            } else {
                None
            };

            if let Some(p_rect) = popup_rect {
                egui::Area::new(egui::Id::new(format!("oto_autocomplete_popup_{}", idx)))
                    .fixed_pos(p_rect.min)
                    .order(egui::Order::Foreground)
                    .show(ui.ctx(), |ui| {
                        egui::Frame::popup(ui.style())
                            .fill(MelodyneTheme::BG_PANEL)
                            .show(ui, |ui| {
                                ui.set_max_width(180.0);
                                ui.label(
                                    egui::RichText::new(format!(
                                        "oto.ini Suggestions ({})",
                                        candidates.len()
                                    ))
                                    .size(10.0)
                                    .color(MelodyneTheme::TEXT_GOLD_LABEL),
                                );
                                ui.separator();

                                egui::ScrollArea::vertical()
                                    .max_height(140.0)
                                    .show(ui, |ui| {
                                        for (cand_i, cand) in candidates.iter().enumerate() {
                                            let is_cand_sel =
                                                state.autocomplete_selected_idx == cand_i;
                                            let text_widget = if is_cand_sel {
                                                egui::RichText::new(cand)
                                                    .strong()
                                                    .color(MelodyneTheme::NOTE_SELECTED_GOLD)
                                            } else {
                                                egui::RichText::new(cand)
                                                    .color(Color32::from_rgb(240, 230, 210))
                                            };

                                            if ui
                                                .selectable_label(is_cand_sel, text_widget)
                                                .clicked()
                                            {
                                                commit_lyric_edit = Some((
                                                    idx,
                                                    replace_active_phoneme(
                                                        &state.lyric_buffer,
                                                        cand,
                                                    ),
                                                    None,
                                                ));
                                            }
                                        }
                                    });
                            });
                    });
            }

            if ui.input(|i| i.key_pressed(Key::ArrowDown)) && !candidates.is_empty() {
                state.autocomplete_selected_idx =
                    (state.autocomplete_selected_idx + 1) % candidates.len();
            }
            if ui.input(|i| i.key_pressed(Key::ArrowUp)) && !candidates.is_empty() {
                state.autocomplete_selected_idx = if state.autocomplete_selected_idx == 0 {
                    candidates.len() - 1
                } else {
                    state.autocomplete_selected_idx - 1
                };
            }

            let is_clicked_outside = ui.input(|i| i.pointer.primary_clicked())
                && mouse_interact_pos.is_some_and(|mpos| {
                    !edit_rect.contains(mpos) && popup_rect.is_none_or(|pr| !pr.contains(mpos))
                });

            let is_escape = ui.input(|i| i.key_pressed(Key::Escape));
            let is_enter = ui.input(|i| i.key_pressed(Key::Enter));
            let is_tab = ui.input(|i| i.key_pressed(Key::Tab));
            let is_shift = ui.input(|i| i.modifiers.shift);

            if is_escape {
                state.editing_lyric_index = None;
            } else if commit_lyric_edit.is_none()
                && (text_lost_focus || is_clicked_outside || is_enter || is_tab)
            {
                let final_lyric = if !state.lyric_buffer.trim().is_empty() {
                    state.lyric_buffer.trim().to_string()
                } else {
                    notes[idx].lyric.clone()
                };
                let next_edit_idx = if is_tab {
                    if is_shift {
                        if idx > 0 {
                            Some(idx - 1)
                        } else {
                            None
                        }
                    } else if idx + 1 < notes.len() {
                        Some(idx + 1)
                    } else {
                        None
                    }
                } else {
                    None
                };
                commit_lyric_edit = Some((idx, final_lyric, next_edit_idx));
            }
        }

        if let Some((idx, new_lyric, next_edit_idx)) = commit_lyric_edit {
            if idx < notes.len() && notes[idx].lyric != new_lyric {
                on_before_change();
                notes[idx].lyric = new_lyric;
                on_note_changed();
            }
            if let Some(next_idx) = next_edit_idx {
                if next_idx < notes.len() {
                    state.editing_lyric_index = Some(next_idx);
                    state.lyric_buffer = notes[next_idx].lyric.clone();
                    state.autocomplete_selected_idx = 0;
                    state.lyric_needs_select_all = true;
                    state.selected_note_index = Some(next_idx);
                    state.selected_note_indices.clear();
                    state.selected_note_indices.insert(next_idx);
                } else {
                    state.editing_lyric_index = None;
                }
            } else {
                state.editing_lyric_index = None;
            }
        }

        if ui.input(|i| i.pointer.primary_released() || i.pointer.secondary_released()) {
            state.shift_locked_drawer_norm = None;
            if state.dragging_envelope_pt.is_some() {
                state.dragging_envelope_pt = None;
            }
            if state.dragging_pitch_pt.is_some() {
                state.dragging_pitch_pt = None;
            }
            if state.continuous_edit_dirty {
                state.continuous_edit_dirty = false;
                on_note_changed();
            }
        }

        if let Some(del_idx) = note_to_delete {
            on_before_change();
            notes.remove(del_idx);
            state.selected_note_indices.remove(&del_idx);
            if state.selected_note_index == Some(del_idx) {
                state.selected_note_index = None;
            }
            on_note_changed();
        }

        if let Some((slice_idx, slice_time_ms)) = note_to_slice {
            if slice_idx < notes.len() {
                let orig_pos = notes[slice_idx].position_ms;
                let orig_dur = notes[slice_idx].duration_ms;
                let split_offset = slice_time_ms - orig_pos;
                if split_offset >= 15.0 && split_offset <= orig_dur - 15.0 {
                    on_before_change();
                    let mut second_note = notes[slice_idx].clone();
                    notes[slice_idx].duration_ms = split_offset;
                    notes[slice_idx].phoneme_durations_ms.clear();
                    second_note.position_ms = slice_time_ms;
                    second_note.duration_ms = orig_dur - split_offset;
                    second_note.lyric = "+".to_string();
                    second_note.phoneme_durations_ms.clear();
                    notes.insert(slice_idx + 1, second_note);
                    state.selected_note_index = Some(slice_idx + 1);
                    state.selected_note_indices.clear();
                    state.selected_note_indices.insert(slice_idx + 1);
                    on_note_changed();
                }
            }
        }

        if !was_editing_lyric && state.editing_lyric_index.is_none() {
            let (select_all, delete_sel, arrow_up, arrow_down, is_shift, press_enter) =
                ui.input(|i| {
                    (
                        i.modifiers.command && i.key_pressed(egui::Key::A),
                        i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace),
                        i.key_pressed(egui::Key::ArrowUp),
                        i.key_pressed(egui::Key::ArrowDown),
                        i.modifiers.shift,
                        i.key_pressed(egui::Key::Enter) && !i.modifiers.command && !i.modifiers.alt,
                    )
                });

            if press_enter {
                let target_idx = state
                    .selected_note_index
                    .or_else(|| state.selected_note_indices.iter().next().copied());
                if let Some(sel_idx) = target_idx {
                    if sel_idx < notes.len() {
                        state.editing_lyric_index = Some(sel_idx);
                        state.lyric_buffer = notes[sel_idx].lyric.clone();
                        state.autocomplete_selected_idx = 0;
                        state.lyric_needs_select_all = true;
                    }
                }
            }

            if select_all {
                state.selected_note_indices = (0..notes.len()).collect();
                if !notes.is_empty() {
                    state.selected_note_index = Some(0);
                }
            }

            if delete_sel && !state.selected_note_indices.is_empty() {
                on_before_change();
                let mut to_delete: Vec<usize> =
                    state.selected_note_indices.iter().copied().collect();
                to_delete.sort_by(|a, b| b.cmp(a));
                for d_idx in to_delete {
                    if d_idx < notes.len() {
                        notes.remove(d_idx);
                    }
                }
                state.selected_note_indices.clear();
                state.selected_note_index = None;
                on_note_changed();
            }

            if (arrow_up || arrow_down) && !state.selected_note_indices.is_empty() {
                on_before_change();
                let shift_amt = if is_shift { 12 } else { 1 };
                let delta = if arrow_up { shift_amt } else { -shift_amt };
                for &n_idx in &state.selected_note_indices {
                    if n_idx < notes.len() {
                        let curr_midi = notes[n_idx].midi_key();
                        let new_midi = (curr_midi as i16 + delta)
                            .clamp(state.min_midi as i16, state.max_midi as i16)
                            as u8;
                        notes[n_idx].pitch = midi_to_note_name(new_midi);
                    }
                }
                on_note_changed();
            }
        }

        if state.active_tool == EditTool::Pointer || state.active_tool == EditTool::Pencil {
            if let (Some(drag_idx), Some(start_pos)) =
                (state.dragging_note_idx, state.drag_start_pos)
            {
                if ui.input(|i| i.pointer.primary_down()) {
                    if let Some(current_pos) = ui.input(|i| i.pointer.interact_pos()) {
                        let delta_x = current_pos.x - start_pos.x;
                        let delta_y = current_pos.y - start_pos.y;
                        let delta_ms = delta_x as f64 / state.px_per_ms as f64;

                        // A click selects a note; only an actual pointer movement
                        // starts an edit.  This prevents selection from silently
                        // quantizing the note onto the grid.
                        if drag_idx < notes.len() && (delta_x.abs() > 2.0 || delta_y.abs() > 2.0) {
                            if state.dragging_is_resize {
                                let raw_dur =
                                    (state.note_original_duration_ms + delta_ms).max(20.0);
                                let new_main_dur = apply_snap_with_zoom(
                                    raw_dur,
                                    snap_option,
                                    bpm,
                                    state.px_per_ms,
                                )
                                .max(20.0);
                                let dur_diff = new_main_dur - state.note_original_duration_ms;

                                if state.note_original_states.len() > 1 {
                                    for &(n_idx, _, orig_dur, _) in &state.note_original_states {
                                        if n_idx < notes.len() {
                                            notes[n_idx].duration_ms =
                                                (orig_dur + dur_diff).max(20.0);
                                        }
                                    }
                                } else {
                                    notes[drag_idx].duration_ms = new_main_dur;
                                }
                            } else if state.dragging_is_left_resize {
                                let original_end_ms =
                                    state.note_original_start_ms + state.note_original_duration_ms;
                                let raw_new_start =
                                    (state.note_original_start_ms + delta_ms).max(0.0);
                                let new_start = apply_snap_with_zoom(
                                    raw_new_start,
                                    snap_option,
                                    bpm,
                                    state.px_per_ms,
                                )
                                .max(0.0);
                                let new_dur = (original_end_ms - new_start).max(20.0);
                                let start_diff = new_start - state.note_original_start_ms;

                                if state.note_original_states.len() > 1 {
                                    for &(n_idx, orig_pos, orig_dur, _) in
                                        &state.note_original_states
                                    {
                                        if n_idx < notes.len() {
                                            let n_orig_end = orig_pos + orig_dur;
                                            let n_new_start = (orig_pos + start_diff).max(0.0);
                                            notes[n_idx].position_ms = n_new_start;
                                            notes[n_idx].duration_ms =
                                                (n_orig_end - n_new_start).max(20.0);
                                        }
                                    }
                                } else {
                                    notes[drag_idx].position_ms = new_start;
                                    notes[drag_idx].duration_ms = new_dur;
                                }
                            } else {
                                // Full note move (position + pitch relative to original drag start snapshot)
                                let delta_semitones = -(delta_y / state.row_height).round() as i32;
                                let raw_pos = (state.note_original_start_ms + delta_ms).max(0.0);
                                let new_pos = apply_snap_with_zoom(
                                    raw_pos,
                                    snap_option,
                                    bpm,
                                    state.px_per_ms,
                                )
                                .max(0.0);
                                let pos_diff = new_pos - state.note_original_start_ms;

                                if state.note_original_states.len() > 1 {
                                    // Apply one shared, grid-snapped delta to the
                                    // whole selection.  Clamp the delta once at
                                    // the left edge so notes never lose their
                                    // relative spacing when the group reaches 0.
                                    let min_orig_pos = state
                                        .note_original_states
                                        .iter()
                                        .map(|&(_, orig_pos, _, _)| orig_pos)
                                        .fold(f64::INFINITY, f64::min);
                                    let group_pos_diff = pos_diff.max(-min_orig_pos);
                                    for &(n_idx, orig_pos, _, orig_midi) in
                                        &state.note_original_states
                                    {
                                        if n_idx < notes.len() {
                                            let n_new_pos = orig_pos + group_pos_diff;
                                            let n_new_midi = (orig_midi as i32 + delta_semitones)
                                                .clamp(state.min_midi as i32, state.max_midi as i32)
                                                as u8;
                                            notes[n_idx].position_ms = n_new_pos;
                                            notes[n_idx].set_midi_key(n_new_midi);
                                        }
                                    }
                                } else {
                                    let new_m = (state.note_original_midi as i32 + delta_semitones)
                                        .clamp(state.min_midi as i32, state.max_midi as i32)
                                        as u8;
                                    notes[drag_idx].position_ms = new_pos;
                                    notes[drag_idx].set_midi_key(new_m);
                                }
                            }
                        }
                    }
                } else {
                    let note_was_changed = if state.note_original_states.len() > 1 {
                        state.note_original_states.iter().any(
                            |&(n_idx, orig_pos, orig_dur, orig_midi)| {
                                notes.get(n_idx).is_some_and(|note| {
                                    (note.position_ms - orig_pos).abs() > f64::EPSILON
                                        || (note.duration_ms - orig_dur).abs() > f64::EPSILON
                                        || note.midi_key() != orig_midi
                                })
                            },
                        )
                    } else {
                        notes.get(drag_idx).is_some_and(|note| {
                            (note.position_ms - state.note_original_start_ms).abs() > f64::EPSILON
                                || (note.duration_ms - state.note_original_duration_ms).abs()
                                    > f64::EPSILON
                                || note.midi_key() != state.note_original_midi
                        })
                    };
                    state.dragging_note_idx = None;
                    state.drag_start_pos = None;
                    state.dragging_is_left_resize = false;
                    state.dragging_is_resize = false;
                    state.note_original_states.clear();
                    if note_was_changed {
                        on_note_changed();
                    }
                }
            }
        } else {
            state.dragging_note_idx = None;
            state.drag_start_pos = None;
            state.dragging_is_left_resize = false;
            state.dragging_is_resize = false;
            state.note_original_states.clear();
        }

        if let Some(c_idx) = state.creating_note_idx {
            if let (Some(note), Some(mpos)) = (notes.get_mut(c_idx), mouse_interact_pos) {
                let curr_x = mpos.x - (rect.min.x + keyboard_width);
                let curr_ms = (curr_x / state.px_per_ms) as f64;
                note.duration_ms = apply_snap_with_zoom(
                    (curr_ms - note.position_ms).max(50.0),
                    snap_option,
                    bpm,
                    state.px_per_ms,
                )
                .max(50.0);
            }
            if !ui.input(|i| i.pointer.primary_down()) {
                state.creating_note_idx = None;
                state.drag_start_pos = None;
                if let Some(note) = notes.get(c_idx) {
                    on_preview_freq(midi_to_freq(note.midi_key() as f64));
                    on_note_changed();
                }
            }
        }

        if let Some(mpos) = mouse_interact_pos {
            let is_hovering_canvas = response.hovered() && ui.clip_rect().contains(mpos);
            if is_hovering_canvas
                && mpos.x > rect.min.x + keyboard_width
                && mpos.y > grid_start_y
                && mpos.y < grid_end_y
                && state.editing_lyric_index.is_none()
            {
                match state.active_tool {
                    EditTool::Pointer => {
                        if response.drag_started()
                            && !interacted_with_note_or_ui
                            && state.dragging_note_idx.is_none()
                        {
                            state.marquee_start = Some(mpos);
                            state.marquee_current = Some(mpos);
                        }

                        if response.dragged() && state.marquee_start.is_some() {
                            state.marquee_current = Some(mpos);

                            if let (Some(m_start), Some(m_curr)) =
                                (state.marquee_start, state.marquee_current)
                            {
                                let sel_rect = Rect::from_two_pos(m_start, m_curr);
                                painter.rect_filled(
                                    sel_rect,
                                    Rounding::same(2.0),
                                    Color32::from_rgba_premultiplied(245, 176, 65, 35),
                                );
                                painter.rect_stroke(
                                    sel_rect,
                                    Rounding::same(2.0),
                                    Stroke::new(1.2_f32, MelodyneTheme::NOTE_SELECTED_GOLD),
                                );

                                state.selected_note_indices.clear();
                                for (n_i, note) in notes.iter().enumerate() {
                                    let n_midi = note.midi_key();
                                    if n_midi >= state.min_midi && n_midi <= state.max_midi {
                                        let key_i = (state.max_midi - n_midi) as f32;
                                        let y_t = grid_start_y + key_i * state.row_height + 2.0;
                                        let y_b = y_t + state.row_height - 4.0;
                                        let x_s = rect.min.x
                                            + keyboard_width
                                            + (note.position_ms * state.px_per_ms as f64) as f32;
                                        let x_e = x_s
                                            + (note.duration_ms * state.px_per_ms as f64) as f32;
                                        let n_rect = Rect::from_min_max(
                                            Pos2::new(x_s, y_t),
                                            Pos2::new(x_e, y_b),
                                        );

                                        if sel_rect.intersects(n_rect) {
                                            state.selected_note_indices.insert(n_i);
                                        }
                                    }
                                }
                            }
                        }

                        if response.clicked()
                            && !interacted_with_note_or_ui
                            && state.dragging_note_idx.is_none()
                        {
                            let click_x = mpos.x - (rect.min.x + keyboard_width);
                            let raw_t = (click_x / state.px_per_ms) as f64;
                            let scrubbed_t = apply_snap_with_zoom(
                                raw_t.max(0.0),
                                snap_option,
                                bpm,
                                state.px_per_ms,
                            );
                            state.playhead_ms = scrubbed_t;
                            on_playhead_scrubbed(scrubbed_t);
                            state.selected_note_indices.clear();
                            state.selected_note_index = None;
                        }

                        if !ui.input(|i| i.pointer.primary_down()) {
                            state.marquee_start = None;
                            state.marquee_current = None;
                        }
                    }

                    EditTool::Pencil => {
                        if ui.input(|i| i.pointer.primary_pressed())
                            && state.creating_note_idx.is_none()
                            && state.dragging_note_idx.is_none()
                            && !interacted_with_note_or_ui
                        {
                            on_before_change();
                            let click_x = mpos.x - (rect.min.x + keyboard_width);
                            let raw_start_ms = (click_x / state.px_per_ms) as f64;
                            let click_start_ms = apply_snap_with_zoom(
                                raw_start_ms,
                                snap_option,
                                bpm,
                                state.px_per_ms,
                            )
                            .max(0.0);
                            let key_idx =
                                ((mpos.y - grid_start_y) / state.row_height).floor() as u8;
                            let click_midi = (state.max_midi.saturating_sub(key_idx))
                                .clamp(state.min_midi, state.max_midi);

                            let new_note = UNote::new(
                                "ka",
                                midi_to_note_name(click_midi),
                                click_start_ms,
                                50.0,
                            );
                            notes.push(new_note);
                            let new_idx = notes.len() - 1;
                            state.creating_note_idx = Some(new_idx);
                            state.selected_note_index = Some(new_idx);
                            state.drag_start_pos = Some(mpos);
                        }
                    }
                    _ => {}
                }

                if let Some(mpos) = mouse_interact_pos {
                    if !interacted_with_note_or_ui
                        && state.dragging_note_idx.is_none()
                        && mpos.x > rect.min.x + keyboard_width
                        && mpos.y > grid_start_y
                        && mpos.y < grid_end_y
                        && ui.input(|i| i.pointer.secondary_clicked())
                    {
                        state.context_menu_note_idx = None;
                        state.context_menu_pos = Some(mpos);
                        state.context_menu_hovered_category = None;
                    }
                }
            }
        }

        if let Some(ref dragged_alias) = phoneme_state.dragged_phoneme.clone() {
            if let Some(mpos) = mouse_interact_pos {
                let is_hovering_canvas = response.hovered() && ui.clip_rect().contains(mpos);
                if is_hovering_canvas
                    && mpos.x > rect.min.x + keyboard_width
                    && mpos.y > grid_start_y
                    && mpos.y < grid_end_y
                {
                    let key_idx = ((mpos.y - grid_start_y) / state.row_height).floor() as u8;
                    let hover_midi = (state.max_midi.saturating_sub(key_idx))
                        .clamp(state.min_midi, state.max_midi);

                    let y_top = grid_start_y
                        + (state.max_midi - hover_midi) as f32 * state.row_height
                        + 2.0;
                    let y_bottom = y_top + state.row_height - 4.0;
                    let hover_rect = Rect::from_min_max(
                        Pos2::new(rect.min.x + keyboard_width, y_top),
                        Pos2::new(rect.max.x, y_bottom),
                    );

                    painter.rect_filled(
                        hover_rect,
                        Rounding::ZERO,
                        Color32::from_rgba_premultiplied(0, 255, 157, 18),
                    );
                    painter.rect_stroke(
                        hover_rect,
                        Rounding::ZERO,
                        Stroke::new(1.0_f32, Color32::from_rgb(0, 255, 157)),
                    );

                    if !ui.input(|i| i.pointer.primary_down()) {
                        let mut dropped_on_note = false;
                        for (idx, note) in notes.iter_mut().enumerate() {
                            let note_midi = note.midi_key();
                            let k_i = (state.max_midi - note_midi) as f32;
                            let n_yt = grid_start_y + k_i * state.row_height + 2.0;
                            let n_yb = n_yt + state.row_height - 4.0;
                            let n_xs = rect.min.x
                                + keyboard_width
                                + (note.position_ms * state.px_per_ms as f64) as f32;
                            let n_xe = n_xs + (note.duration_ms * state.px_per_ms as f64) as f32;
                            let n_rect =
                                Rect::from_min_max(Pos2::new(n_xs, n_yt), Pos2::new(n_xe, n_yb));

                            if n_rect.contains(mpos) {
                                note.lyric = dragged_alias.clone();
                                state.selected_note_index = Some(idx);
                                dropped_on_note = true;
                                break;
                            }
                        }

                        if !dropped_on_note {
                            let click_x = mpos.x - (rect.min.x + keyboard_width);
                            let raw_start_ms = (click_x / state.px_per_ms) as f64;
                            let drop_start_ms = apply_snap_with_zoom(
                                raw_start_ms,
                                snap_option,
                                bpm,
                                state.px_per_ms,
                            )
                            .max(0.0);
                            let new_note = UNote::new(
                                dragged_alias,
                                midi_to_note_name(hover_midi),
                                drop_start_ms,
                                400.0,
                            );
                            notes.push(new_note);
                            state.selected_note_index = Some(notes.len() - 1);
                        }

                        let freq = midi_to_freq(hover_midi as f64);
                        on_preview_freq(freq);
                        on_note_changed();
                        phoneme_state.dragged_phoneme = None;
                    }
                }
            }

            if !ui.input(|i| i.pointer.primary_down()) {
                phoneme_state.dragged_phoneme = None;
            }
        }

        let sticky_key_x = rect.min.x.max(visible_clip.min.x);
        let keys_y_min = visible_clip.min.y.max(grid_start_y);
        let keys_y_max = visible_clip.max.y.min(grid_end_y);

        grid::draw_piano_keys(
            &painter,
            ui,
            state,
            theme,
            rect,
            visible_clip,
            keyboard_width,
            grid_start_y,
            grid_end_y,
            first_visible_key,
            last_visible_key,
            on_preview_freq,
        );

        let playhead_x =
            rect.min.x + keyboard_width + (state.playhead_ms * state.px_per_ms as f64) as f32;
        if playhead_x >= sticky_key_x + keyboard_width && playhead_x <= visible_clip.max.x {
            let handle_points = vec![
                Pos2::new(playhead_x - 3.5, keys_y_min + 1.0),
                Pos2::new(playhead_x + 3.5, keys_y_min + 1.0),
                Pos2::new(playhead_x, keys_y_min + 9.0),
            ];
            painter.add(egui::Shape::convex_polygon(
                handle_points,
                theme.playhead_c32(),
                Stroke::new(0.8_f32, Color32::WHITE),
            ));

            painter.line_segment(
                [
                    Pos2::new(playhead_x, keys_y_min),
                    Pos2::new(playhead_x, keys_y_max),
                ],
                Stroke::new(3.0_f32, theme.c32_alpha(theme.playhead_color, 0.25)),
            );

            painter.line_segment(
                [
                    Pos2::new(playhead_x, keys_y_min),
                    Pos2::new(playhead_x, keys_y_max),
                ],
                Stroke::new(1.2_f32, theme.playhead_c32()),
            );
        }
    });
    if !state.is_playing && !is_mod_zoom && !state.is_scrubbing_ruler && !is_middle_panning {
        state.horizontal_scroll_offset = scroll_output.state.offset.x;
        state.vertical_scroll_offset = scroll_output.state.offset.y;
    }

    context_menu::draw(ui, notes, state, lang, on_before_change, on_note_changed);

    note_properties::draw(ui, notes, state, lang, on_note_changed);
}
