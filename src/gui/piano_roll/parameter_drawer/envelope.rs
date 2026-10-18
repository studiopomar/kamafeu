use super::*;

/// Dedicated five-point envelope editor for the bottom parameter drawer.
/// The envelope is intentionally rendered as a connected polygon per note,
/// with the same P1..P5 handles used by the note envelope editor.
#[allow(clippy::too_many_arguments)]
pub(super) fn draw(
    ui: &mut egui::Ui,
    graph_rect: Rect,
    response: &egui::Response,
    notes: &mut [UNote],
    state: &mut PianoRollState,
    timeline_origin_x: f32,
    theme: &ThemeConfig,
    on_before_change: &mut dyn FnMut(),
    on_note_changed: &mut dyn FnMut(),
) {
    let painter = ui.painter_at(graph_rect);
    let bg = ui.visuals().panel_fill;
    let lane = theme.c32_alpha(theme.bg_panel, 0.92);
    let lane_stroke = theme.note_stroke_c32();
    let coral = ui.visuals().warn_fg_color;
    let pink = theme.playhead_c32();
    let orange = theme.note_hover_c32();
    let white = theme.text_primary_c32();
    painter.rect_filled(graph_rect, Rounding::ZERO, bg);

    // The lower editor is a set of horizontal lanes, not a generic graph.
    let lane_top = graph_rect.top() + 48.0;
    let lane_bottom = graph_rect.bottom() - 18.0;
    let lane_height = (lane_bottom - lane_top).max(22.0);
    let mid_y = lane_top + lane_height * 0.5;
    for y in [graph_rect.top() + 30.0, lane_top, mid_y, lane_bottom] {
        painter.line_segment(
            [
                Pos2::new(graph_rect.left(), y),
                Pos2::new(graph_rect.right(), y),
            ],
            Stroke::new(
                1.0_f32,
                ui.visuals()
                    .text_color()
                    .linear_multiply(0.72)
                    .linear_multiply(150.0 / 255.0),
            ),
        );
    }
    let px_per_ms = state.px_per_ms.max(0.001) as f64;
    let mut nearest: Option<(usize, usize, f32)> = None;

    for (note_idx, note) in notes.iter().enumerate() {
        let duration = note.duration_ms.max(1.0);
        let x_start = timeline_origin_x + (note.position_ms * px_per_ms) as f32;
        let x_end = timeline_origin_x + ((note.position_ms + duration) * px_per_ms) as f32;
        if x_end < graph_rect.left() - 30.0 || x_start > graph_rect.right() + 30.0 {
            continue;
        }
        let rect = Rect::from_min_max(Pos2::new(x_start, lane_top), Pos2::new(x_end, lane_bottom));
        painter.rect_filled(rect, Rounding::same(2.0), lane);
        painter.rect_stroke(rect, Rounding::same(2.0), Stroke::new(1.0_f32, lane_stroke));
        painter.line_segment(
            [
                Pos2::new(x_start, graph_rect.top()),
                Pos2::new(x_start, graph_rect.bottom()),
            ],
            Stroke::new(1.2_f32, pink),
        );
        // Keep the note identity as a small canvas label. The previous
        // rounded tags consumed a second, overlapping layout on top of every
        // note and made dense passages unreadable.
        painter.text(
            Pos2::new((x_start + x_end) * 0.5, graph_rect.top() + 8.0),
            egui::Align2::CENTER_TOP,
            &note.lyric,
            egui::FontId::proportional(9.0),
            white,
        );

        // `p1..p5` are UTAU duration fields, not absolute X coordinates.
        // Rendering them directly folded every envelope toward note start in
        // the drawer, even though the piano roll used the correct cumulative
        // geometry. Use the shared effective-point conversion so both views
        // describe the same editable curve.
        let points = note.envelope.get_effective_points(duration);
        let screen_points: Vec<Pos2> = points
            .iter()
            .map(|(time, level)| {
                Pos2::new(
                    x_start + (*time * px_per_ms) as f32,
                    lane_bottom - lane_height * (*level as f32 / 200.0).clamp(0.0, 1.0),
                )
            })
            .collect();
        let visual_curve = note.envelope.generate_visual_curve(duration, 12);
        let curve_screen_points: Vec<Pos2> = visual_curve
            .iter()
            .map(|(time, level)| {
                Pos2::new(
                    x_start + (*time * px_per_ms) as f32,
                    lane_bottom - lane_height * (*level as f32 / 200.0).clamp(0.0, 1.0),
                )
            })
            .collect();
        if curve_screen_points.len() >= 2 {
            let mut fill = Vec::with_capacity(curve_screen_points.len() + 2);
            fill.push(Pos2::new(curve_screen_points[0].x, lane_bottom));
            fill.extend(curve_screen_points.iter().copied());
            fill.push(Pos2::new(
                curve_screen_points
                    .last()
                    .map(|point| point.x)
                    .unwrap_or(x_end),
                lane_bottom,
            ));
            painter.add(egui::Shape::convex_polygon(
                fill,
                ui.visuals().warn_fg_color.linear_multiply(23.0 / 255.0),
                Stroke::NONE,
            ));
            painter.add(egui::Shape::line(
                curve_screen_points,
                Stroke::new(2.0_f32, coral),
            ));
        }
        for (point_idx, point) in screen_points.iter().enumerate() {
            if point.x < graph_rect.left() - 12.0 || point.x > graph_rect.right() + 12.0 {
                continue;
            }
            let color = match point_idx {
                0 => coral,
                1 => coral,
                2 | 3 => orange,
                _ => pink,
            };
            let radius = if state.dragging_envelope_pt == Some((note_idx, point_idx)) {
                7.0
            } else {
                5.5
            };
            painter.circle_filled(*point, radius, bg);
            painter.circle_stroke(*point, radius, Stroke::new(2.0_f32, color));
            if let Some(cursor) = response.hover_pos() {
                let distance = cursor.distance(*point);
                if distance < nearest.map(|v| v.2).unwrap_or(15.0) {
                    nearest = Some((note_idx, point_idx, distance));
                }
            }
        }
    }

    if let Some((note_idx, point_idx, _)) = nearest {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
        if response.drag_started() {
            on_before_change();
            state.dragging_envelope_pt = Some((note_idx, point_idx));
            state.is_dragging_in_drawer = true;
        }
    }
    if let Some((note_idx, point_idx)) = state.dragging_envelope_pt {
        if let Some(pos) = response.interact_pointer_pos() {
            if let Some(note) = notes.get_mut(note_idx) {
                let duration = note.duration_ms.max(1.0);
                let time = ((pos.x - timeline_origin_x) as f64 / px_per_ms - note.position_ms)
                    .clamp(0.0, duration);
                let volume =
                    (((lane_bottom - pos.y) / lane_height) * 200.0).clamp(0.0, 200.0) as f64;
                note.envelope.set_point(point_idx, time, volume, duration);
                state.continuous_edit_dirty = true;
                on_note_changed();
            }
        }
        if !ui.input(|input| input.pointer.primary_down()) {
            state.dragging_envelope_pt = None;
            state.is_dragging_in_drawer = false;
        }
    }
}
