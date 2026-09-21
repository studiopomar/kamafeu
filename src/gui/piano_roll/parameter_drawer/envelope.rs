use super::*;

pub(super) fn draw(
    ui: &mut egui::Ui,
    notes: &mut [UNote],
    state: &mut PianoRollState,
    keyboard_width: f32,
    timeline_scroll_x: f32,
    on_before_change: &mut dyn FnMut(),
    theme: &ThemeConfig,
    ruler_rect: Rect,
    _bpm: f64,
) {
    let panel_response = egui::TopBottomPanel::bottom("bottom_expanded_envelope_editor")
        .resizable(true)
        .height_range(130.0..=500.0)
        .default_height(state.drawer_height.max(180.0))
        .frame(
            egui::Frame::none()
                .fill(theme.bg_panel_c32())
                .stroke(Stroke::new(1.5_f32, theme.accent_c32())),
        )
        .show_inside(ui, |ui| {
            ui.expand_to_include_rect(ui.max_rect());
            ui.set_min_height(ui.available_height());
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("ENVELOPES DA FRASE — linha do tempo")
                        .strong()
                        .color(Color32::from_rgb(255, 120, 205)),
                );
                ui.label(
                    egui::RichText::new(
                        "Arraste horizontalmente para o tempo e verticalmente para o volume",
                    )
                    .size(9.5)
                    .color(MelodyneTheme::TEXT_MUTED),
                );
                if ui.button("Fechar").clicked() {
                    state.show_envelope_handles = false;
                    state.dragging_envelope_pt = None;
                }
            });

            let graph_size = Vec2::new(ui.available_width(), ui.available_height().max(90.0));
            let (graph_rect, response) =
                ui.allocate_exact_size(graph_size, Sense::click_and_drag());
            let painter = ui.painter_at(graph_rect);
            painter.rect_filled(graph_rect, Rounding::same(4.0), MelodyneTheme::BG_CANVAS);
            painter.rect_stroke(
                graph_rect,
                Rounding::same(4.0),
                Stroke::new(1.0_f32, MelodyneTheme::GRID_LINE_BAR),
            );
            let inner = graph_rect.shrink2(Vec2::new(6.0, 16.0));
            for level in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
                let y = inner.max.y - level * inner.height();
                painter.line_segment(
                    [Pos2::new(inner.min.x, y), Pos2::new(inner.max.x, y)],
                    Stroke::new(0.7_f32, MelodyneTheme::GRID_LINE_SUB),
                );
                painter.text(
                    Pos2::new(inner.min.x + 3.0, y),
                    egui::Align2::LEFT_CENTER,
                    format!("{:.0}%", level * 100.0),
                    egui::FontId::proportional(8.5),
                    MelodyneTheme::TEXT_MUTED,
                );
            }

            let timeline_origin_x = ruler_rect.min.x + keyboard_width - timeline_scroll_x;
            let pointer = response.interact_pointer_pos();
            let mut hovered_handle = None;

            for (note_index, note) in notes.iter().enumerate() {
                let duration = note.duration_ms.max(1.0);
                let note_start_x =
                    timeline_origin_x + (note.position_ms * state.px_per_ms as f64) as f32;
                let note_end_x = note_start_x + (duration * state.px_per_ms as f64) as f32;
                if note_end_x < graph_rect.min.x || note_start_x > graph_rect.max.x {
                    continue;
                }
                let selected = state.selected_note_index == Some(note_index)
                    || state.selected_note_indices.contains(&note_index);
                let env = &note.envelope;
                let points = [
                    (env.p1, env.v1),
                    (env.p1 + env.p2, env.v2),
                    (env.p1 + env.p2 + env.p3, env.v3),
                    ((duration - env.p4).max(0.0), env.v4),
                    ((duration - env.p4 + env.p5).max(0.0), env.v5),
                ];
                let screen_points = points.map(|(time, volume)| {
                    Pos2::new(
                        note_start_x + (time * state.px_per_ms as f64) as f32,
                        inner.max.y - (volume / 100.0).clamp(0.0, 1.0) as f32 * inner.height(),
                    )
                });

                let note_band = Rect::from_min_max(
                    Pos2::new(note_start_x.max(graph_rect.min.x), inner.min.y),
                    Pos2::new(note_end_x.min(graph_rect.max.x), inner.max.y),
                );
                painter.rect_filled(
                    note_band,
                    Rounding::ZERO,
                    if selected {
                        Color32::from_rgba_unmultiplied(0, 180, 220, 24)
                    } else {
                        Color32::from_rgba_unmultiplied(40, 80, 130, 14)
                    },
                );
                painter.text(
                    Pos2::new(note_start_x + 4.0, inner.min.y + 2.0),
                    egui::Align2::LEFT_TOP,
                    &note.lyric,
                    egui::FontId::proportional(9.0),
                    if selected {
                        Color32::from_rgb(0, 235, 255)
                    } else {
                        MelodyneTheme::TEXT_MUTED
                    },
                );

                let adjacent_previous = note_index.checked_sub(1).and_then(|previous_index| {
                    notes.get(previous_index).filter(|previous| {
                        let gap = note.position_ms - (previous.position_ms + previous.duration_ms);
                        gap.abs() <= 200.0
                    })
                });
                let automatic_crossfade = state
                    .oto_overlap_cache
                    .get(note_index)
                    .copied()
                    .unwrap_or(0.0)
                    .abs()
                    .max(env.p2.min(45.0));
                let visual_crossfade_ms = if env.crossfade_ms > 0.0 {
                    env.crossfade_ms
                } else if adjacent_previous.is_some() {
                    automatic_crossfade
                } else {
                    0.0
                };
                if visual_crossfade_ms > 0.0 {
                    let cross_start =
                        note_start_x - (visual_crossfade_ms * state.px_per_ms as f64) as f32;
                    let left = cross_start.max(graph_rect.min.x);
                    let right = note_start_x.min(graph_rect.max.x);
                    if right > left {
                        painter.rect_filled(
                            Rect::from_min_max(
                                Pos2::new(left, inner.min.y),
                                Pos2::new(right, inner.max.y),
                            ),
                            Rounding::ZERO,
                            Color32::from_rgba_unmultiplied(0, 200, 180, 26),
                        );
                        let color = Color32::from_rgb(0, 225, 200);
                        painter.line_segment(
                            [Pos2::new(left, inner.max.y), Pos2::new(right, inner.min.y)],
                            Stroke::new(1.8_f32, color),
                        );
                        painter.line_segment(
                            [Pos2::new(left, inner.min.y), Pos2::new(right, inner.max.y)],
                            Stroke::new(1.8_f32, color),
                        );
                        let handle = Pos2::new(cross_start, inner.center().y);
                        if graph_rect.expand(15.0).contains(handle) {
                            let hovered =
                                pointer.is_some_and(|position| position.distance(handle) <= 13.0);
                            let dragging = state.dragging_envelope_pt == Some((note_index, 5));
                            painter.circle_filled(
                                handle,
                                if hovered || dragging { 8.5 } else { 6.0 },
                                if dragging {
                                    Color32::WHITE
                                } else {
                                    Color32::from_rgb(0, 235, 210)
                                },
                            );
                            painter.circle_stroke(
                                handle,
                                if hovered || dragging { 8.5 } else { 6.0 },
                                Stroke::new(1.2_f32, Color32::WHITE),
                            );
                            if hovered || dragging {
                                hovered_handle = Some((note_index, 5));
                                painter.text(
                                    Pos2::new(handle.x, handle.y - 14.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    format!("Crossfade · {:.0}ms", visual_crossfade_ms),
                                    egui::FontId::proportional(9.5),
                                    Color32::WHITE,
                                );
                            }
                        }
                    }
                }

                painter.add(egui::Shape::line(
                    screen_points.to_vec(),
                    Stroke::new(
                        if selected { 2.6_f32 } else { 1.5_f32 },
                        if selected {
                            Color32::from_rgb(0, 220, 250)
                        } else {
                            Color32::from_rgb(65, 145, 185)
                        },
                    ),
                ));
                for (point_index, point) in screen_points.iter().enumerate() {
                    if !graph_rect.expand(15.0).contains(*point) {
                        continue;
                    }
                    let hovered = pointer.is_some_and(|position| position.distance(*point) <= 13.0);
                    if hovered {
                        hovered_handle = Some((note_index, point_index));
                    }
                    let dragging = state.dragging_envelope_pt == Some((note_index, point_index));
                    painter.circle_filled(
                        *point,
                        if hovered || dragging { 8.0 } else { 5.5 },
                        if dragging {
                            Color32::WHITE
                        } else if selected {
                            Color32::from_rgb(0, 235, 255)
                        } else {
                            Color32::from_rgb(70, 160, 200)
                        },
                    );
                    painter.circle_stroke(
                        *point,
                        if hovered || dragging { 8.0 } else { 5.5 },
                        Stroke::new(
                            1.0_f32,
                            if dragging {
                                Color32::from_rgb(0, 220, 255)
                            } else {
                                Color32::from_rgb(20, 30, 45)
                            },
                        ),
                    );
                    if hovered || dragging {
                        let label_text = format!(
                            "{} · P{} · {:.0}ms · {:.0}%",
                            note.lyric,
                            point_index + 1,
                            points[point_index].0,
                            points[point_index].1
                        );
                        let text_shape = painter.layout_no_wrap(
                            label_text,
                            egui::FontId::proportional(9.5),
                            Color32::WHITE,
                        );
                        let pill_rect = Rect::from_center_size(
                            Pos2::new(point.x, point.y - 14.0),
                            Vec2::new(text_shape.size().x + 10.0, 16.0),
                        );
                        painter.rect_filled(
                            pill_rect,
                            Rounding::same(4.0),
                            Color32::from_rgba_unmultiplied(15, 22, 36, 230),
                        );
                        painter.rect_stroke(
                            pill_rect,
                            Rounding::same(4.0),
                            Stroke::new(1.0_f32, Color32::from_rgb(0, 210, 240)),
                        );
                        painter.galley(
                            Pos2::new(
                                pill_rect.center().x - text_shape.size().x * 0.5,
                                pill_rect.center().y - text_shape.size().y * 0.5,
                            ),
                            text_shape,
                            Color32::WHITE,
                        );
                    }
                }
            }

            if response.drag_started() {
                if let Some(handle) = hovered_handle {
                    on_before_change();
                    state.dragging_envelope_pt = Some(handle);
                    state.selected_note_index = Some(handle.0);
                    state.selected_note_indices.clear();
                    state.selected_note_indices.insert(handle.0);
                }
            }
            if let (Some((note_index, point_index)), Some(position)) =
                (state.dragging_envelope_pt, pointer)
            {
                if let Some(note) = notes.get_mut(note_index) {
                    let duration = note.duration_ms.max(1.0);
                    let note_start_x =
                        timeline_origin_x + (note.position_ms * state.px_per_ms as f64) as f32;
                    let time = (f64::from(position.x - note_start_x) / f64::from(state.px_per_ms))
                        .max(0.0);
                    let volume = ((inner.max.y - position.y) / inner.height()).clamp(0.0, 1.0)
                        as f64
                        * 100.0;
                    match point_index {
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
                                (p3_time - note.envelope.p1 - note.envelope.p2).max(0.0);
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
                            note.envelope.crossfade_ms = (f64::from(note_start_x - position.x)
                                / f64::from(state.px_per_ms))
                            .clamp(0.0, 600.0);
                        }
                        _ => {}
                    }
                    state.continuous_edit_dirty = true;
                }
            }
            if response.drag_stopped() {
                state.dragging_envelope_pt = None;
            }
        });
    // `ui.max_rect()` inside the closure still reflects the layout before
    // the resize interaction is finalized. Persist the resulting panel
    // rectangle instead, otherwise the next frame restores the old size.
    state.drawer_height = panel_response.response.rect.height().clamp(130.0, 500.0);
}
