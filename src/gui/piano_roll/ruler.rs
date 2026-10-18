use super::*;

pub(super) struct RulerConfig<'a> {
    pub(super) markers: &'a [UProjectMarker],
    pub(super) sections: &'a [UProjectSection],
    pub(super) theme: &'a ThemeConfig,
    pub(super) keyboard_width: f32,
    pub(super) ruler_height: f32,
    pub(super) timeline_scroll_x: f32,
    pub(super) total_canvas_ms: f64,
    pub(super) bpm: f64,
    pub(super) time_signature_numerator: u8,
    pub(super) time_signature_denominator: u8,
    pub(super) snap_option: GridSnapOption,
    pub(super) lang: crate::config::AppLanguage,
}

pub(super) fn draw(
    ui: &mut egui::Ui,
    state: &mut PianoRollState,
    config: RulerConfig<'_>,
    on_playhead_scrubbed: &mut dyn FnMut(f64),
) -> Rect {
    let RulerConfig {
        markers,
        sections,
        theme,
        keyboard_width,
        ruler_height,
        timeline_scroll_x,
        total_canvas_ms,
        bpm,
        time_signature_numerator,
        time_signature_denominator,
        snap_option,
        lang,
    } = config;
    let (ruler_rect, _ruler_resp) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), ruler_height),
        Sense::click_and_drag(),
    );

    let ruler_painter = ui.painter_at(ruler_rect);
    ruler_painter.rect_filled(ruler_rect, Rounding::ZERO, theme.bg_header_c32());
    ruler_painter.line_segment(
        [
            Pos2::new(ruler_rect.min.x, ruler_rect.max.y),
            Pos2::new(ruler_rect.max.x, ruler_rect.max.y),
        ],
        Stroke::new(1.5_f32, theme.grid_line_bar_c32()),
    );

    let corner_rect = Rect::from_min_max(
        Pos2::new(ruler_rect.min.x, ruler_rect.min.y),
        Pos2::new(ruler_rect.min.x + keyboard_width, ruler_rect.max.y),
    );
    ruler_painter.rect_filled(corner_rect, Rounding::ZERO, theme.bg_panel_c32());
    ruler_painter.text(
        Pos2::new(
            ruler_rect.min.x + 8.0,
            ruler_rect.min.y + ruler_height * 0.5,
        ),
        egui::Align2::LEFT_CENTER,
        "TIME",
        egui::FontId::proportional(10.0),
        theme.accent_c32(),
    );

    let beat_ms = 60000.0 / bpm * 4.0 / f64::from(time_signature_denominator.max(1));
    let bar_ms = beat_ms * f64::from(time_signature_numerator.max(1));
    let ruler_visible_width = (ruler_rect.width() - keyboard_width).max(1.0);
    let ruler_start_ms = (timeline_scroll_x / state.px_per_ms).max(0.0) as f64;
    let ruler_end_ms = ruler_start_ms + (ruler_visible_width / state.px_per_ms) as f64 + bar_ms;
    let first_second = (ruler_start_ms / 1_000.0).floor().max(0.0) as u64;
    let mut tick_ms = first_second as f64 * 1_000.0;

    while tick_ms < total_canvas_ms.min(ruler_end_ms) {
        let x = ruler_rect.min.x + keyboard_width + (tick_ms * state.px_per_ms as f64) as f32
            - timeline_scroll_x;
        if x >= ruler_rect.min.x + keyboard_width && x <= ruler_rect.max.x + 200.0 {
            let is_bar = ((tick_ms / bar_ms).round() - (tick_ms / bar_ms)).abs() < 1e-6;
            ruler_painter.line_segment(
                [
                    Pos2::new(x, ruler_rect.min.y + if is_bar { 10.0 } else { 16.0 }),
                    Pos2::new(x, ruler_rect.max.y),
                ],
                Stroke::new(
                    if is_bar { 1.25_f32 } else { 0.7_f32 },
                    if is_bar {
                        theme.grid_line_bar_c32()
                    } else {
                        theme.grid_line_sub_c32()
                    },
                ),
            );

            ruler_painter.text(
                Pos2::new(x + 4.0, ruler_rect.min.y + 4.0),
                egui::Align2::LEFT_TOP,
                format_timeline_time(tick_ms),
                egui::FontId::proportional(11.0),
                theme.text_primary_c32(),
            );
        }

        tick_ms += 1_000.0;
    }

    // Arrangement sections live independently from notes. Their quiet bands
    // make song structure visible without obscuring beat labels or markers.
    let mut jumped_to_marker = false;
    for (section_index, section) in sections.iter().enumerate() {
        let start_x =
            ruler_rect.min.x + keyboard_width + (section.start_ms * state.px_per_ms as f64) as f32
                - timeline_scroll_x;
        let end_x =
            ruler_rect.min.x + keyboard_width + (section.end_ms * state.px_per_ms as f64) as f32
                - timeline_scroll_x;
        let section_rect = Rect::from_min_max(
            Pos2::new(
                start_x.max(ruler_rect.min.x + keyboard_width),
                ruler_rect.min.y + 1.0,
            ),
            Pos2::new(end_x.min(ruler_rect.max.x), ruler_rect.max.y - 1.0),
        );
        if section_rect.is_positive() {
            let color = marker_color(section.color.as_deref(), theme);
            ruler_painter.rect_filled(
                section_rect,
                Rounding::same(2.0),
                Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 34),
            );
            ruler_painter.text(
                Pos2::new(section_rect.min.x + 4.0, section_rect.center().y),
                egui::Align2::LEFT_CENTER,
                &section.name,
                egui::FontId::proportional(9.0),
                Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 210),
            );
            let section_response = ui.interact(
                section_rect,
                ui.make_persistent_id(("project_section", section_index)),
                Sense::click(),
            );
            if section_response.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if section_response.clicked() {
                state.playhead_ms = section.start_ms;
                state.is_scrubbing_ruler = false;
                on_playhead_scrubbed(section.start_ms);
                jumped_to_marker = true;
            }
        }
    }

    // Arrangement markers live in the project timeline, independently of the
    // current vocal part. Compact flags keep sections readable without
    // competing with measure labels or loop handles.
    for (marker_index, marker) in markers.iter().enumerate() {
        let x = ruler_rect.min.x
            + keyboard_width
            + (marker.position_ms * state.px_per_ms as f64) as f32
            - timeline_scroll_x;
        if x < ruler_rect.min.x + keyboard_width || x > ruler_rect.max.x {
            continue;
        }
        let color = marker_color(marker.color.as_deref(), theme);
        ruler_painter.line_segment(
            [
                Pos2::new(x, ruler_rect.min.y + 1.0),
                Pos2::new(x, ruler_rect.max.y),
            ],
            Stroke::new(1.25_f32, color),
        );
        let flag = [
            Pos2::new(x + 1.0, ruler_rect.min.y + 2.0),
            Pos2::new(x + 9.0, ruler_rect.min.y + 5.5),
            Pos2::new(x + 1.0, ruler_rect.min.y + 9.0),
        ];
        ruler_painter.add(egui::Shape::convex_polygon(
            flag.to_vec(),
            color,
            Stroke::NONE,
        ));
        ruler_painter.text(
            Pos2::new(x + 11.0, ruler_rect.min.y + 3.0),
            egui::Align2::LEFT_TOP,
            &marker.name,
            egui::FontId::proportional(9.0),
            color,
        );
        let marker_response = ui.interact(
            Rect::from_min_max(
                Pos2::new(x - 6.0, ruler_rect.min.y),
                Pos2::new(x + 12.0, ruler_rect.max.y),
            ),
            ui.make_persistent_id(("project_marker", marker_index)),
            Sense::click(),
        );
        if marker_response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if marker_response.clicked() {
            state.playhead_ms = marker.position_ms;
            state.is_scrubbing_ruler = false;
            on_playhead_scrubbed(marker.position_ms);
            jumped_to_marker = true;
        }
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
                theme.c32_alpha(theme.semantic.loop_region, 0.18)
            } else {
                theme.c32_alpha(theme.text_muted, 0.10)
            };
            ruler_painter.rect_filled(loop_banner_rect, theme.ui_rounding(), banner_bg);
        }

        // Loop Start Marker [A
        if loop_start_x >= left_bound && loop_start_x <= right_bound {
            let marker_color = if state.loop_enabled {
                theme.loop_region_c32()
            } else {
                theme.text_muted_c32()
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
                theme.loop_region_c32()
            } else {
                theme.text_muted_c32()
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
            theme.playhead_c32(),
            Stroke::new(0.8_f32, theme.text_primary_c32()),
        ));
        ruler_painter.line_segment(
            [
                Pos2::new(ruler_playhead_x, ruler_rect.min.y + 10.0),
                Pos2::new(ruler_playhead_x, ruler_rect.max.y),
            ],
            Stroke::new(1.0_f32, theme.playhead_c32()),
        );
    }

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
        if !state.is_maximized
            && !state.show_arrangement_view
            && !state.show_inspector
            && !state.show_phoneme_ruler
        {
            state.show_arrangement_view = true;
            state.show_inspector = true;
            state.show_phoneme_ruler = true;
        }
        ui.ctx().request_repaint();
    }

    let scrub_ruler_rect = Rect::from_min_max(
        ruler_rect.min,
        Pos2::new(
            (ruler_rect.max.x - 32.0).max(ruler_rect.min.x),
            ruler_rect.max.y,
        ),
    );

    let ruler_response = ui.interact(
        scrub_ruler_rect,
        ui.make_persistent_id("piano_roll_ruler_interaction"),
        Sense::click_and_drag(),
    );

    let is_hovered = max_btn_resp.hovered();
    let (btn_bg, btn_stroke, text_color, icon_symbol) = if state.is_maximized {
        (
            if is_hovered {
                theme.c32_alpha(theme.accent_color, 0.28)
            } else {
                theme.c32_alpha(theme.bg_canvas, 0.82)
            },
            Stroke::new(1.0_f32, theme.accent_c32()),
            theme.accent_c32(),
            "[v]",
        )
    } else {
        (
            if is_hovered {
                theme.c32_alpha(theme.accent_color, 0.22)
            } else {
                theme.c32_alpha(theme.bg_canvas, 0.82)
            },
            Stroke::new(
                1.0_f32,
                if is_hovered {
                    theme.accent_c32()
                } else {
                    theme.accent_soft_c32()
                },
            ),
            if is_hovered {
                theme.accent_c32()
            } else {
                theme.accent_soft_c32()
            },
            "[^]",
        )
    };

    ruler_painter.rect_filled(max_btn_rect, theme.ui_rounding(), btn_bg);
    ruler_painter.rect_stroke(max_btn_rect, theme.ui_rounding(), btn_stroke);
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

    if !jumped_to_marker && (ruler_response.clicked() || ruler_response.dragged()) {
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

    ruler_rect
}
