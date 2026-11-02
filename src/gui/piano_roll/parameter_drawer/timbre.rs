use super::*;

fn row_at(rect: Rect, count: usize, y: f32) -> usize {
    (((rect.bottom() - y) / rect.height() * count as f32).floor() as usize)
        .min(count.saturating_sub(1))
}

pub(super) fn draw(
    ui: &mut egui::Ui,
    notes: &mut [UNote],
    mut timbre_curve: Option<&mut Vec<crate::project::model::UTimbrePoint>>,
    state: &mut PianoRollState,
    rect: Rect,
    response: &egui::Response,
    origin_x: f32,
    voicebank: Option<&Voicebank>,
    theme: &ThemeConfig,
    lang: crate::config::AppLanguage,
    on_before_change: &mut dyn FnMut(),
    on_note_changed: &mut dyn FnMut(),
) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, Rounding::same(3.0), theme.bg_canvas_c32());
    let mut options = vec![None];
    if let Some(vb) = voicebank {
        options.extend(vb.prefix_map.colors().map(|name| Some(name.to_string())));
    }
    // Retain authored values even when the current bank lacks that timbre.
    for note in notes.iter() {
        if !options.contains(&note.timbre) {
            options.push(note.timbre.clone());
        }
    }
    if let Some(curve) = timbre_curve.as_ref() {
        for pt in curve.iter() {
            if !options.contains(&pt.timbre) {
                options.push(pt.timbre.clone());
            }
        }
    }

    let row_h = rect.height() / options.len() as f32;
    let (pointer, primary, secondary, pressed) = ui.input(|i| {
        (
            i.pointer.interact_pos(),
            i.pointer.primary_down(),
            i.pointer.secondary_down(),
            i.pointer.primary_pressed() || i.pointer.secondary_pressed(),
        )
    });

    // Automation Curve Drag / Click Handling
    if state.timbre_automation_mode {
        if let Some(curve) = timbre_curve.as_deref_mut() {
            if pressed && (response.is_pointer_button_down_on() || response.contains_pointer()) {
                if let Some(pos) = pointer {
                    if rect.contains(pos) {
                        on_before_change();
                        let time_ms =
                            ((pos.x - origin_x) / state.px_per_ms.max(0.001)).max(0.0) as f64;
                        let target_row = row_at(rect, options.len(), pos.y);
                        let val = options[target_row].clone();

                        // Check if clicked close to an existing point
                        let mut hit_idx = None;
                        for (idx, pt) in curve.iter().enumerate() {
                            let pt_x = origin_x + pt.time_offset_ms as f32 * state.px_per_ms;
                            if (pt_x - pos.x).abs() < 12.0 {
                                hit_idx = Some(idx);
                                break;
                            }
                        }

                        if secondary {
                            if let Some(idx) = hit_idx {
                                curve.remove(idx);
                                on_note_changed();
                            }
                        } else if let Some(idx) = hit_idx {
                            state.timbre_point_drag = Some(idx);
                            curve[idx].timbre = val;
                            state.timbre_changed = true;
                        } else {
                            // Insert new point
                            let new_pt = crate::project::model::UTimbrePoint {
                                time_offset_ms: time_ms,
                                timbre: val,
                                weight: 1.0,
                            };
                            let insert_pos = curve
                                .binary_search_by(|p| p.time_offset_ms.total_cmp(&time_ms))
                                .unwrap_or_else(|pos| pos);
                            curve.insert(insert_pos, new_pt);
                            state.timbre_point_drag = Some(insert_pos);
                            state.timbre_changed = true;
                        }
                    }
                }
            }

            if primary {
                if let (Some(drag_idx), Some(pos)) = (state.timbre_point_drag, pointer) {
                    if drag_idx < curve.len() && rect.contains(pos) {
                        let time_ms =
                            ((pos.x - origin_x) / state.px_per_ms.max(0.001)).max(0.0) as f64;
                        let target_row = row_at(rect, options.len(), pos.y);
                        curve[drag_idx].time_offset_ms = time_ms;
                        curve[drag_idx].timbre = options[target_row].clone();
                        curve.sort_by(|a, b| a.time_offset_ms.total_cmp(&b.time_offset_ms));
                        state.timbre_changed = true;
                    }
                }
            }

            if !primary && !secondary && state.timbre_point_drag.take().is_some() {
                if state.timbre_changed {
                    on_note_changed();
                }
                state.timbre_changed = false;
            }
        }
    } else {
        // Discrete Note Pins Drag / Click Handling
        if pressed && (response.is_pointer_button_down_on() || response.contains_pointer()) {
            if pointer.is_some() {
                on_before_change();
                state.timbre_drag = pointer;
                state.timbre_changed = false;
            }
        }
        if let (Some(previous), Some(pos)) = (state.timbre_drag, pointer) {
            if (primary || secondary) && rect.contains(pos) {
                let value = if secondary {
                    None
                } else {
                    options[row_at(rect, options.len(), pos.y)].clone()
                };
                let left = previous.x.min(pos.x);
                let right = previous.x.max(pos.x);
                let has_selection =
                    !state.selected_note_indices.is_empty() || state.selected_note_index.is_some();
                for (idx, note) in notes.iter_mut().enumerate() {
                    let start = origin_x + note.position_ms as f32 * state.px_per_ms;
                    let end = start + note.duration_ms as f32 * state.px_per_ms;
                    let selected = state.selected_note_indices.contains(&idx)
                        || state.selected_note_index == Some(idx);
                    if end >= left
                        && start <= right
                        && (!has_selection || selected)
                        && note.timbre != value
                    {
                        note.timbre = value.clone();
                        state.timbre_changed = true;
                        state.continuous_edit_dirty = true;
                    }
                }
                state.timbre_drag = Some(pos);
            }
        }
        if !primary && !secondary && state.timbre_drag.take().is_some() {
            if state.timbre_changed {
                on_note_changed();
            }
            state.timbre_changed = false;
        }
    }

    // Draw row grid lines and background labels
    for (row, option) in options.iter().enumerate() {
        let y = rect.bottom() - (row as f32 + 0.5) * row_h;
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            theme.card_stroke(),
        );

        let label = match option.as_deref() {
            None => lang.tr("Padrão da faixa", "Track default"),
            Some("") => lang.tr("Neutro", "Neutral"),
            Some(name) => name,
        };
        let galley = painter.layout_no_wrap(
            label.to_string(),
            egui::FontId::proportional(11.0),
            theme.text_primary_c32(),
        );
        let label_pos = Pos2::new(
            rect.right() - galley.size().x - 8.0,
            y - galley.size().y * 0.5,
        );
        painter.rect_filled(
            Rect::from_min_size(label_pos, galley.size()).expand(2.0),
            Rounding::same(2.0),
            theme.bg_canvas_c32(),
        );
        painter.galley(label_pos, galley, theme.text_primary_c32());
    }

    // Render Automation Curve (Stepped line + gradient fill)
    if let Some(curve) = timbre_curve.as_ref() {
        if !curve.is_empty() {
            let get_y = |timbre: &Option<String>| -> f32 {
                let row = options.iter().position(|o| o == timbre).unwrap_or(0);
                rect.bottom() - (row as f32 + 0.5) * row_h
            };

            let stroke_color = if state.timbre_automation_mode {
                theme.accent_c32()
            } else {
                theme.c32_alpha(theme.accent_color, 0.45)
            };
            let fill_color = theme.c32_alpha(
                theme.accent_color,
                if state.timbre_automation_mode {
                    0.14
                } else {
                    0.05
                },
            );

            // Connect stepped segments across time
            let mut prev_x =
                (origin_x + curve[0].time_offset_ms as f32 * state.px_per_ms).max(rect.left());
            let mut prev_y = get_y(&curve[0].timbre);

            // Shaded fill to baseline
            for (idx, pt) in curve.iter().enumerate() {
                let cur_x = origin_x + pt.time_offset_ms as f32 * state.px_per_ms;
                let cur_y = get_y(&pt.timbre);

                if idx > 0 {
                    // Stepped segment: horizontal from prev_x to cur_x at prev_y, then vertical to cur_y
                    let step_p1 = Pos2::new(prev_x.clamp(rect.left(), rect.right()), prev_y);
                    let step_p2 = Pos2::new(cur_x.clamp(rect.left(), rect.right()), prev_y);
                    let step_p3 = Pos2::new(cur_x.clamp(rect.left(), rect.right()), cur_y);

                    painter.line_segment(
                        [step_p1, step_p2],
                        Stroke::new(
                            if state.timbre_automation_mode {
                                2.0_f32
                            } else {
                                1.2_f32
                            },
                            stroke_color,
                        ),
                    );
                    painter.line_segment(
                        [step_p2, step_p3],
                        Stroke::new(
                            if state.timbre_automation_mode {
                                2.0_f32
                            } else {
                                1.2_f32
                            },
                            stroke_color,
                        ),
                    );

                    // Fill under horizontal segment
                    if step_p2.x > step_p1.x {
                        painter.rect_filled(
                            Rect::from_min_max(
                                Pos2::new(step_p1.x, step_p1.y),
                                Pos2::new(step_p2.x, rect.bottom()),
                            ),
                            Rounding::ZERO,
                            fill_color,
                        );
                    }
                }

                // Node marker
                if cur_x >= rect.left() - 10.0 && cur_x <= rect.right() + 10.0 {
                    let node_pos = Pos2::new(cur_x, cur_y);
                    painter.circle_filled(
                        node_pos,
                        if state.timbre_automation_mode {
                            4.5
                        } else {
                            3.0
                        },
                        stroke_color,
                    );
                    painter.circle_stroke(
                        node_pos,
                        if state.timbre_automation_mode {
                            6.0
                        } else {
                            4.0
                        },
                        Stroke::new(1.0_f32, theme.bg_canvas_c32()),
                    );
                }

                prev_x = cur_x;
                prev_y = cur_y;
            }

            // Extend last point to end of graph
            if prev_x < rect.right() {
                let end_p = Pos2::new(rect.right(), prev_y);
                painter.line_segment(
                    [Pos2::new(prev_x, prev_y), end_p],
                    Stroke::new(
                        if state.timbre_automation_mode {
                            2.0_f32
                        } else {
                            1.2_f32
                        },
                        stroke_color,
                    ),
                );
                painter.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(prev_x, prev_y),
                        Pos2::new(rect.right(), rect.bottom()),
                    ),
                    Rounding::ZERO,
                    fill_color,
                );
            }
        }
    }

    // Render Note Pins
    for (row, option) in options.iter().enumerate() {
        let y = rect.bottom() - (row as f32 + 0.5) * row_h;
        for (idx, note) in notes.iter().enumerate() {
            let x = origin_x + note.position_ms as f32 * state.px_per_ms + 5.0;
            if x < rect.left() || x > rect.right() {
                continue;
            }
            let point = Pos2::new(x, y);
            let selected = state.selected_note_indices.contains(&idx)
                || state.selected_note_index == Some(idx);
            let active = &note.timbre == option;
            let color = if selected {
                theme.accent_c32()
            } else {
                theme.text_primary_c32()
            };
            let alpha = if state.timbre_automation_mode {
                0.45
            } else {
                1.0
            };
            painter.circle_stroke(
                point,
                4.5,
                Stroke::new(
                    if active { 2.0_f32 } else { 1.0_f32 },
                    if active {
                        color.linear_multiply(alpha)
                    } else {
                        theme.text_muted_c32().linear_multiply(alpha * 0.7)
                    },
                ),
            );
            if active && option.is_some() {
                painter.circle_filled(point, 2.5, color.linear_multiply(alpha));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drag_respects_selection_and_finishes_outside_panel() {
        let ctx = egui::Context::default();
        let dir = tempfile::tempdir().unwrap();
        let mut bank = Voicebank::new(dir.path()).unwrap();
        bank.prefix_map = crate::oto::PrefixMap::parse_yaml_str(
            "subbanks:\n  - color: ''\n    suffix: ''\n  - color: Soft\n    suffix: _soft\n",
        );
        let mut notes = vec![
            UNote::new("a", "C4", 100.0, 100.0),
            UNote::new("a", "C4", 300.0, 100.0),
        ];
        let mut state = PianoRollState::default();
        state.px_per_ms = 1.0;
        state.selected_note_indices.insert(0);
        let mut begins = 0;
        let mut commits = 0;
        let mut frame = |events| {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 300.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let (rect, response) = ui
                            .allocate_exact_size(Vec2::new(600.0, 120.0), Sense::click_and_drag());
                        draw(
                            ui,
                            &mut notes,
                            None,
                            &mut state,
                            rect,
                            &response,
                            rect.left(),
                            Some(&bank),
                            &ThemeConfig::default(),
                            crate::config::AppLanguage::PtBr,
                            &mut || begins += 1,
                            &mut || commits += 1,
                        );
                    });
                },
            );
        };
        let button = |pos, pressed, button| egui::Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        let start = Pos2::new(130.0, 28.0);
        frame(vec![]);
        frame(vec![egui::Event::PointerMoved(start)]);
        frame(vec![button(start, true, egui::PointerButton::Primary)]);
        frame(vec![egui::Event::PointerMoved(Pos2::new(330.0, 28.0))]);
        let outside = Pos2::new(700.0, 220.0);
        frame(vec![
            egui::Event::PointerMoved(outside),
            button(outside, false, egui::PointerButton::Primary),
        ]);
        drop(frame);
        assert_eq!((begins, commits), (1, 1));
        assert_eq!(notes[0].timbre.as_deref(), Some("Soft"));
        assert_eq!(notes[1].timbre, None);
        assert_eq!((begins, commits), (1, 1));
        assert!(state.timbre_drag.is_none());
    }
}
