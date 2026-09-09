use super::*;

pub(super) fn draw(
    ui: &mut egui::Ui,
    notes: &[UNote],
    state: &mut PianoRollState,
    theme: &ThemeConfig,
    keyboard_width: f32,
    timeline_scroll_x: f32,
    total_canvas_ms: f64,
) {
    // Timeline Mini-Map (Radar)
    if state.show_minimap {
        let (minimap_rect, minimap_resp) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), state.minimap_height),
            Sense::click_and_drag(),
        );
        let m_painter = ui.painter_at(minimap_rect);
        m_painter.rect_filled(minimap_rect, Rounding::ZERO, Color32::from_rgb(14, 11, 20));
        m_painter.line_segment(
            [
                Pos2::new(minimap_rect.min.x, minimap_rect.max.y),
                Pos2::new(minimap_rect.max.x, minimap_rect.max.y),
            ],
            Stroke::new(1.0, Color32::from_rgb(38, 30, 52)),
        );

        // Corner label
        m_painter.text(
            Pos2::new(minimap_rect.min.x + 6.0, minimap_rect.center().y),
            egui::Align2::LEFT_CENTER,
            "RADAR",
            egui::FontId::monospace(8.0),
            Color32::from_rgb(150, 130, 180),
        );

        let map_span_x = (minimap_rect.width() - keyboard_width).max(10.0);
        let map_origin_x = minimap_rect.min.x + keyboard_width;

        // Draw miniature notes in radar
        for n in notes.iter() {
            let n_x_norm = (n.position_ms / total_canvas_ms).clamp(0.0, 1.0) as f32;
            let n_w_norm = ((n.duration_ms / total_canvas_ms).clamp(0.001, 1.0)) as f32;
            let nx = map_origin_x + n_x_norm * map_span_x;
            let nw = (n_w_norm * map_span_x).max(2.0);

            let k_norm = (state.max_midi.saturating_sub(n.midi_key()) as f32
                / (state.max_midi - state.min_midi).max(1) as f32)
                .clamp(0.0, 1.0);
            let ny = minimap_rect.min.y + 2.0 + k_norm * (state.minimap_height - 6.0);

            m_painter.rect_filled(
                Rect::from_min_size(Pos2::new(nx, ny), Vec2::new(nw, 2.0)),
                Rounding::same(0.5),
                theme.c32_alpha(theme.note_fill, 0.75),
            );
        }

        // Viewport window overlay
        let visible_w = (ui.available_width() - keyboard_width).max(10.0);
        let vp_start_norm =
            ((timeline_scroll_x / state.px_per_ms) as f64 / total_canvas_ms).clamp(0.0, 1.0) as f32;
        let vp_dur_ms = (visible_w / state.px_per_ms) as f64;
        let vp_w_norm = ((vp_dur_ms / total_canvas_ms).clamp(0.01, 1.0)) as f32;

        let vp_rect = Rect::from_min_max(
            Pos2::new(
                map_origin_x + vp_start_norm * map_span_x,
                minimap_rect.min.y + 1.0,
            ),
            Pos2::new(
                map_origin_x + (vp_start_norm + vp_w_norm) * map_span_x,
                minimap_rect.max.y - 1.0,
            ),
        );
        m_painter.rect_filled(
            vp_rect,
            Rounding::same(2.0),
            Color32::from_rgba_unmultiplied(192, 132, 252, 40),
        );
        m_painter.rect_stroke(
            vp_rect,
            Rounding::same(2.0),
            Stroke::new(1.0, theme.accent_c32()),
        );

        // Playhead in minimap
        let playhead_norm = (state.playhead_ms / total_canvas_ms).clamp(0.0, 1.0) as f32;
        let playhead_map_x = map_origin_x + playhead_norm * map_span_x;
        m_painter.line_segment(
            [
                Pos2::new(playhead_map_x, minimap_rect.min.y),
                Pos2::new(playhead_map_x, minimap_rect.max.y),
            ],
            Stroke::new(1.5, theme.playhead_c32()),
        );

        // Minimap drag interaction
        if minimap_resp.dragged() || minimap_resp.clicked() {
            if let Some(mpos) = minimap_resp.interact_pointer_pos() {
                if mpos.x >= map_origin_x && mpos.x <= map_origin_x + map_span_x {
                    let click_norm = ((mpos.x - map_origin_x) / map_span_x).clamp(0.0, 1.0) as f64;
                    let target_ms = click_norm * total_canvas_ms;
                    let target_scroll_x =
                        (target_ms * state.px_per_ms as f64) as f32 - visible_w * 0.5;
                    state.horizontal_scroll_offset = target_scroll_x.max(0.0);
                }
            }
        }
    }
}
