use crate::gui::piano_roll::PianoRollState;
use crate::gui::theme::MelodyneTheme;
use crate::project::model::UNote;
use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Sense, Stroke, Vec2};

pub fn draw_phoneme_ruler(
    ui: &mut egui::Ui,
    state: &mut PianoRollState,
    notes: &mut [UNote],
    ruler_rect: Rect,
    keyboard_width: f32,
    timeline_scroll_x: f32,
    bpm: f64,
    on_before_change: &mut dyn FnMut(),
    on_note_changed: &mut dyn FnMut(),
) {
    let ruler_h = 76.0f32;

    egui::TopBottomPanel::bottom("bottom_phoneme_envelope_ruler")
        .resizable(false)
        .exact_height(ruler_h)
        .frame(egui::Frame::none().fill(Color32::from_rgb(15, 12, 22)))
        .show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(keyboard_width);

                let was_editing_phoneme = state.editing_phoneme_index.is_some();
                let mut commit_phoneme_edit: Option<(usize, usize, String)> = None;

                let available_w = (ui.available_width() - 4.0).max(100.0);
                let (strip_rect, strip_response) = ui.allocate_exact_size(
                    Vec2::new(available_w, ruler_h - 2.0),
                    Sense::click_and_drag(),
                );

                let painter = ui.painter_at(strip_rect);
                painter.rect_filled(strip_rect, Rounding::ZERO, Color32::from_rgb(20, 16, 30));
                painter.line_segment(
                    [
                        Pos2::new(strip_rect.min.x, strip_rect.min.y),
                        Pos2::new(strip_rect.max.x, strip_rect.min.y),
                    ],
                    Stroke::new(1.0_f32, Color32::from_rgb(48, 38, 68)),
                );

                let is_primary_down = ui.input(|i| i.pointer.primary_down());
                let is_secondary_down = ui.input(|i| i.pointer.secondary_down());
                let pointer_pos = ui.input(|i| i.pointer.latest_pos()).filter(|pos| {
                    strip_rect.contains(*pos)
                        || state.dragging_phoneme_handle.is_some()
                        || state.dragging_subphoneme_boundary.is_some()
                });

                if !is_primary_down {
                    if state.dragging_phoneme_handle.is_some()
                        || state.dragging_subphoneme_boundary.is_some()
                    {
                        state.dragging_phoneme_handle = None;
                        state.dragging_subphoneme_boundary = None;
                        state.continuous_edit_dirty = true;
                        on_note_changed();
                    }
                }

                if is_secondary_down && !state.right_click_reset_active {
                    state.right_click_reset_active = true;
                    on_before_change();
                } else if !is_secondary_down && state.right_click_reset_active {
                    state.right_click_reset_active = false;
                    state.continuous_edit_dirty = true;
                    on_note_changed();
                }

                let px_per_ms = state.px_per_ms as f64;

                let beat_ms = 60000.0 / bpm.max(1.0);
                let bar_ms = beat_ms * 4.0;
                let start_time_ms = (timeline_scroll_x as f64 / px_per_ms).max(0.0);
                let end_time_ms = ((timeline_scroll_x + available_w) as f64 / px_per_ms).max(0.0);
                let first_bar = (start_time_ms / bar_ms).floor() as i64;
                let last_bar = (end_time_ms / bar_ms).ceil() as i64;

                for bar in first_bar..=last_bar {
                    let bar_time = bar as f64 * bar_ms;
                    let x = ruler_rect.min.x + keyboard_width + (bar_time * px_per_ms) as f32
                        - timeline_scroll_x;
                    if x >= strip_rect.min.x && x <= strip_rect.max.x {
                        painter.line_segment(
                            [
                                Pos2::new(x, strip_rect.min.y),
                                Pos2::new(x, strip_rect.max.y),
                            ],
                            Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(65, 52, 90, 140)),
                        );
                        painter.text(
                            Pos2::new(x + 4.0, strip_rect.max.y - 4.0),
                            egui::Align2::LEFT_BOTTOM,
                            format!("{}", bar + 1),
                            egui::FontId::proportional(9.0),
                            Color32::from_rgb(110, 100, 135),
                        );
                    }
                }

                let y_top = strip_rect.min.y + 24.0;
                let y_bottom = strip_rect.max.y - 10.0;

                let mut previous_note_end_ms: Option<f64> = None;
                let mut previous_note_lyric: Option<String> = None;

                for (note_index, note) in notes.iter_mut().enumerate() {
                    let prior_end_ms = previous_note_end_ms;
                    let prior_lyric = previous_note_lyric.clone();
                    previous_note_end_ms = Some(note.position_ms + note.duration_ms);
                    previous_note_lyric = Some(note.lyric.clone());

                    let x_start =
                        ruler_rect.min.x + keyboard_width + (note.position_ms * px_per_ms) as f32
                            - timeline_scroll_x;
                    let x_end = x_start + (note.duration_ms * px_per_ms) as f32;

                    if x_end < strip_rect.min.x - 300.0 || x_start > strip_rect.max.x + 300.0 {
                        continue;
                    }

                    let is_selected = state.selected_note_index == Some(note_index)
                        || state.selected_note_indices.contains(&note_index);

                    let lyric_trimmed = note.lyric.trim().to_string();
                    let is_plus = lyric_trimmed == "+" || lyric_trimmed.starts_with("+ ");

                    let subphonemes: Vec<(String, f64, f64)> =
                        if let Some(cached) = state.note_phonemes_cache.get(note_index) {
                            if !cached.is_empty() {
                                let count = cached.len();
                                let mut cur_offset = cached[0].1;
                                let has_custom = note.phoneme_durations_ms.len() == count;
                                let authored_sum: f64 = if has_custom {
                                    note.phoneme_durations_ms.iter().sum()
                                } else {
                                    0.0
                                };
                                let scale = if has_custom
                                    && authored_sum > 0.0
                                    && (authored_sum - note.duration_ms).abs() > 2.0
                                {
                                    note.duration_ms / authored_sum
                                } else {
                                    1.0
                                };

                                cached
                                    .iter()
                                    .enumerate()
                                    .map(|(idx, (lyric, _rel_pos, default_dur))| {
                                        let dur = if has_custom {
                                            note.phoneme_durations_ms[idx] * scale
                                        } else {
                                            *default_dur
                                        };
                                        let pos = cur_offset;
                                        cur_offset += dur;
                                        (lyric.clone(), pos, dur)
                                    })
                                    .collect()
                            } else {
                                let manual_parts: Vec<&str> = lyric_trimmed
                                    .split(['.', ';', ','])
                                    .map(str::trim)
                                    .filter(|part| !part.is_empty())
                                    .collect();
                                if manual_parts.len() > 1 {
                                    let durs = note.resolved_phoneme_durations(manual_parts.len());
                                    let mut cur_pos = 0.0;
                                    manual_parts
                                        .into_iter()
                                        .zip(durs)
                                        .map(|(s, d)| {
                                            let p = cur_pos;
                                            cur_pos += d;
                                            (s.to_string(), p, d)
                                        })
                                        .collect()
                                } else {
                                    vec![(lyric_trimmed.clone(), 0.0, note.duration_ms)]
                                }
                            }
                        } else {
                            let manual_parts: Vec<&str> = lyric_trimmed
                                .split(['.', ';', ','])
                                .map(str::trim)
                                .filter(|part| !part.is_empty())
                                .collect();
                            if manual_parts.len() > 1 {
                                let durs = note.resolved_phoneme_durations(manual_parts.len());
                                let mut cur_pos = 0.0;
                                manual_parts
                                    .into_iter()
                                    .zip(durs)
                                    .map(|(s, d)| {
                                        let p = cur_pos;
                                        cur_pos += d;
                                        (s.to_string(), p, d)
                                    })
                                    .collect()
                            } else {
                                vec![(lyric_trimmed.clone(), 0.0, note.duration_ms)]
                            }
                        };

                    let oto_consonant = state
                        .oto_consonant_cache
                        .get(note_index)
                        .copied()
                        .unwrap_or(0.0);
                    let oto_preutter = state
                        .oto_preutter_cache
                        .get(note_index)
                        .copied()
                        .unwrap_or(0.0);
                    let oto_overlap = state
                        .oto_overlap_cache
                        .get(note_index)
                        .copied()
                        .unwrap_or(0.0);

                    let consonant_v_scale = crate::phonemizer::consonant_velocity_time_scale(
                        note.expressions.consonant_velocity,
                    );

                    let base_consonant_ms = oto_consonant * consonant_v_scale;
                    let active_consonant_ms = (base_consonant_ms
                        + note.expressions.consonant_timing_offset_ms)
                        .clamp(0.0, note.duration_ms.max(10.0) * 0.95);

                    let base_preutter_ms = oto_preutter * consonant_v_scale;
                    let active_preutter_ms =
                        (base_preutter_ms + note.expressions.preutter_offset_ms).max(0.0);

                    let base_overlap_ms = oto_overlap * consonant_v_scale;
                    let active_overlap_ms = base_overlap_ms + note.expressions.overlap_offset_ms;

                    let preutter_x = x_start - (active_preutter_ms * px_per_ms) as f32;
                    let overlap_x = x_start
                        - ((active_preutter_ms - active_overlap_ms).max(0.0) * px_per_ms) as f32;
                    let consonant_x =
                        (x_start + (active_consonant_ms * px_per_ms) as f32).clamp(x_start, x_end);

                    if state.right_click_reset_active {
                        if let Some(pos) = pointer_pos {
                            let first_sub_x = subphonemes
                                .first()
                                .map(|s| x_start + (s.1 * px_per_ms) as f32)
                                .unwrap_or(x_start);
                            let min_x = preutter_x.min(x_start).min(first_sub_x) - 10.0;
                            let touch_rect = Rect::from_min_max(
                                Pos2::new(min_x, strip_rect.min.y),
                                Pos2::new(x_end + 10.0, strip_rect.max.y),
                            );
                            if touch_rect.contains(pos) {
                                if note.expressions.consonant_timing_offset_ms != 0.0
                                    || note.expressions.preutter_offset_ms != 0.0
                                    || note.expressions.overlap_offset_ms != 0.0
                                {
                                    note.expressions.consonant_timing_offset_ms = 0.0;
                                    note.expressions.preutter_offset_ms = 0.0;
                                    note.expressions.overlap_offset_ms = 0.0;
                                    state.continuous_edit_dirty = true;
                                }
                                if !note.phoneme_durations_ms.is_empty() {
                                    note.phoneme_durations_ms.clear();
                                    state.phoneme_cache_hash = 0;
                                    state.continuous_edit_dirty = true;
                                }
                            }
                        }
                    }

                    let has_previous_adjacent =
                        prior_end_ms.is_some_and(|prev_end| prev_end >= note.position_ms - 2.0);
                    let fill_color = if is_selected {
                        Color32::from_rgba_unmultiplied(115, 95, 175, 120)
                    } else {
                        Color32::from_rgba_unmultiplied(80, 68, 125, 95)
                    };
                    let stroke_color = if is_selected {
                        Color32::from_rgb(255, 215, 80)
                    } else {
                        Color32::from_rgb(140, 125, 190)
                    };

                    let attack_start_x = preutter_x;
                    let attack_end_x = overlap_x.max(preutter_x);
                    let cutoff_start_x = x_end;
                    let cutoff_end_x = x_end;

                    let poly_points = vec![
                        Pos2::new(attack_start_x, y_bottom),
                        Pos2::new(attack_end_x, y_top),
                        Pos2::new(cutoff_start_x, y_top),
                        Pos2::new(cutoff_end_x, y_bottom),
                    ];

                    painter.add(egui::Shape::convex_polygon(
                        poly_points,
                        fill_color,
                        Stroke::new(if is_selected { 1.6_f32 } else { 1.0_f32 }, stroke_color),
                    ));

                    if !is_plus && consonant_x > x_start + 1.0 && consonant_x < x_end {
                        let c_rect = Rect::from_min_max(
                            Pos2::new(attack_end_x.max(x_start), y_top),
                            Pos2::new(consonant_x, y_bottom),
                        );
                        if c_rect.width() > 1.0 {
                            painter.rect_filled(
                                c_rect,
                                Rounding::ZERO,
                                Color32::from_rgba_unmultiplied(40, 140, 180, 45),
                            );
                        }
                        painter.line_segment(
                            [
                                Pos2::new(consonant_x, y_top),
                                Pos2::new(consonant_x, y_bottom),
                            ],
                            Stroke::new(
                                1.2_f32,
                                Color32::from_rgba_unmultiplied(120, 230, 200, 200),
                            ),
                        );
                    }

                    if !is_plus
                        && has_previous_adjacent
                        && active_overlap_ms > 0.0
                        && overlap_x > preutter_x + 0.5
                    {
                        painter.line_segment(
                            [Pos2::new(preutter_x, y_top), Pos2::new(overlap_x, y_bottom)],
                            Stroke::new(if is_selected { 1.8_f32 } else { 1.2_f32 }, stroke_color),
                        );
                        painter.line_segment(
                            [Pos2::new(preutter_x, y_bottom), Pos2::new(overlap_x, y_top)],
                            Stroke::new(if is_selected { 1.8_f32 } else { 1.2_f32 }, stroke_color),
                        );
                        let circle_radius = 3.2f32;
                        painter.circle(
                            Pos2::new(preutter_x, y_top),
                            circle_radius,
                            Color32::from_rgb(22, 17, 34),
                            Stroke::new(1.3_f32, stroke_color),
                        );
                        painter.circle(
                            Pos2::new(overlap_x, y_bottom),
                            circle_radius,
                            Color32::from_rgb(22, 17, 34),
                            Stroke::new(1.3_f32, stroke_color),
                        );
                    }

                    let circle_radius = 3.4f32;
                    let mut draw_anchor_handle =
                        |pos: Pos2, kind: u8, color: Color32, tooltip: &str| {
                            if pos.x >= strip_rect.min.x - 6.0 && pos.x <= strip_rect.max.x + 6.0 {
                                let clamped_x = pos.x.clamp(strip_rect.min.x, strip_rect.max.x);
                                let draw_pos = Pos2::new(clamped_x, pos.y);
                                let mut is_hover = false;
                                if let Some(cursor_pos) = pointer_pos {
                                    is_hover = (cursor_pos.x - clamped_x).abs() <= 8.0
                                        && (cursor_pos.y - pos.y).abs() <= 8.0;
                                }
                                let node_stroke_color = if is_hover {
                                    Color32::from_rgb(255, 235, 100)
                                } else {
                                    color
                                };
                                let node_fill = if is_hover {
                                    Color32::from_rgb(50, 40, 75)
                                } else {
                                    Color32::from_rgb(20, 16, 32)
                                };
                                painter.circle(
                                    draw_pos,
                                    if is_hover {
                                        circle_radius + 1.2
                                    } else {
                                        circle_radius
                                    },
                                    node_fill,
                                    Stroke::new(
                                        if is_hover { 1.8_f32 } else { 1.3_f32 },
                                        node_stroke_color,
                                    ),
                                );
                                if is_hover {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                    painter.text(
                                        Pos2::new(clamped_x, strip_rect.min.y + 4.0),
                                        egui::Align2::CENTER_TOP,
                                        tooltip,
                                        egui::FontId::proportional(10.0),
                                        Color32::from_rgb(255, 240, 160),
                                    );
                                    if strip_response.drag_started() && is_primary_down {
                                        on_before_change();
                                        let init_val = match kind {
                                            0 => note.expressions.preutter_offset_ms,
                                            1 => note.expressions.overlap_offset_ms,
                                            _ => note.expressions.consonant_timing_offset_ms,
                                        };
                                        state.dragging_phoneme_handle =
                                            Some((note_index, kind, draw_pos.x, init_val));
                                        state.selected_note_index = Some(note_index);
                                    }
                                }
                            }
                        };

                    if !is_plus && subphonemes.len() <= 1 {
                        draw_anchor_handle(
                            Pos2::new(preutter_x, y_bottom),
                            0,
                            Color32::from_rgb(0, 220, 255),
                            &format!(
                                "Preutter: {:.1}ms ({:+.0}ms)",
                                active_preutter_ms, note.expressions.preutter_offset_ms
                            ),
                        );
                        draw_anchor_handle(
                            Pos2::new(overlap_x, y_top),
                            1,
                            Color32::from_rgb(255, 120, 200),
                            &format!(
                                "Overlap: {:.1}ms ({:+.0}ms)",
                                active_overlap_ms, note.expressions.overlap_offset_ms
                            ),
                        );
                        draw_anchor_handle(
                            Pos2::new(consonant_x, y_top),
                            2,
                            if is_selected {
                                MelodyneTheme::ACCENT_GOLD
                            } else {
                                Color32::from_rgb(0, 255, 157)
                            },
                            &format!(
                                "Consoante: {:.1}ms ({:+.0}ms)",
                                active_consonant_ms, note.expressions.consonant_timing_offset_ms
                            ),
                        );
                    }

                    painter.circle(
                        Pos2::new(x_end, y_top),
                        circle_radius,
                        Color32::from_rgb(20, 16, 32),
                        Stroke::new(1.2_f32, stroke_color),
                    );
                    painter.circle(
                        Pos2::new(x_end, y_bottom),
                        circle_radius,
                        Color32::from_rgb(20, 16, 32),
                        Stroke::new(1.2_f32, stroke_color),
                    );

                    if subphonemes.len() > 1 {
                        for (index, (label, rel_pos, duration)) in subphonemes.iter().enumerate() {
                            let seg_start_x = x_start + (*rel_pos * px_per_ms) as f32;
                            let seg_end_x = x_start + ((*rel_pos + *duration) * px_per_ms) as f32;
                            let visible_start = seg_start_x.max(strip_rect.min.x);
                            let visible_end = seg_end_x.min(strip_rect.max.x);

                            let badge_x =
                                if seg_start_x < strip_rect.min.x && seg_end_x > strip_rect.min.x {
                                    (strip_rect.min.x + 24.0).min(seg_end_x - 14.0)
                                } else {
                                    ((seg_start_x + seg_end_x) * 0.5)
                                        .clamp(strip_rect.min.x + 18.0, strip_rect.max.x - 18.0)
                                };
                            let badge_y = strip_rect.min.y + 11.0;
                            if badge_x >= strip_rect.min.x && badge_x <= strip_rect.max.x {
                                painter.line_segment(
                                    [
                                        Pos2::new(badge_x, badge_y + 7.0),
                                        Pos2::new(badge_x.clamp(seg_start_x, seg_end_x), y_top),
                                    ],
                                    Stroke::new(
                                        1.0_f32,
                                        Color32::from_rgba_unmultiplied(140, 125, 185, 160),
                                    ),
                                );
                                let is_editing_this =
                                    state.editing_phoneme_index == Some((note_index, index));
                                let text_shape = painter.layout_no_wrap(
                                    label.clone(),
                                    egui::FontId::proportional(10.0),
                                    if is_selected {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_rgb(230, 225, 245)
                                    },
                                );
                                let pill_w = (text_shape.size().x + 12.0).max(28.0);
                                let pill_rect = Rect::from_center_size(
                                    Pos2::new(badge_x, badge_y),
                                    Vec2::new(pill_w, 16.0),
                                );

                                let pill_id = ui.id().with(("subphoneme_pill", note_index, index));
                                let pill_resp = ui.interact(pill_rect, pill_id, Sense::click());
                                if !was_editing_phoneme && pill_resp.double_clicked() {
                                    state.editing_phoneme_index = Some((note_index, index));
                                    state.phoneme_buffer = label.clone();
                                    state.phoneme_needs_select_all = true;
                                    state.selected_note_index = Some(note_index);
                                }
                                if pill_resp.hovered() && !is_editing_this {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }

                                if is_editing_this {
                                    let edit_w = (pill_rect.width().max(65.0)).min(160.0);
                                    let edit_rect = Rect::from_center_size(
                                        Pos2::new(badge_x, badge_y),
                                        Vec2::new(edit_w, 18.0),
                                    );
                                    painter.rect_filled(
                                        edit_rect,
                                        Rounding::same(4.0),
                                        Color32::from_rgb(45, 30, 75),
                                    );
                                    painter.rect_stroke(
                                        edit_rect,
                                        Rounding::same(4.0),
                                        Stroke::new(1.8_f32, Color32::from_rgb(255, 215, 80)),
                                    );

                                    let text_id = ui.make_persistent_id(format!(
                                        "subphoneme_edit_{}_{}",
                                        note_index, index
                                    ));
                                    if state.phoneme_needs_select_all {
                                        let char_count = state.phoneme_buffer.chars().count();
                                        let mut te_state =
                                            egui::text_edit::TextEditState::default();
                                        te_state.cursor.set_char_range(Some(
                                            egui::text::CCursorRange::two(
                                                egui::text::CCursor::new(0),
                                                egui::text::CCursor::new(char_count),
                                            ),
                                        ));
                                        te_state.store(ui.ctx(), text_id);
                                        state.phoneme_needs_select_all = false;
                                    }

                                    let text_resp = ui.put(
                                        edit_rect,
                                        egui::TextEdit::singleline(&mut state.phoneme_buffer)
                                            .id(text_id)
                                            .text_color(Color32::WHITE)
                                            .desired_width(edit_rect.width())
                                            .font(egui::FontId::proportional(11.0))
                                            .margin(egui::Margin::symmetric(4.0, 1.0)),
                                    );

                                    let is_enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                                    let is_escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                                    let is_clicked_outside = ui.input(|i| {
                                        i.pointer.button_pressed(egui::PointerButton::Primary)
                                            && i.pointer
                                                .interact_pos()
                                                .is_some_and(|pos| !edit_rect.contains(pos))
                                    });

                                    if !is_enter && !is_escape && !text_resp.lost_focus() {
                                        text_resp.request_focus();
                                    }

                                    if is_escape {
                                        state.editing_phoneme_index = None;
                                    } else if is_enter
                                        || is_clicked_outside
                                        || text_resp.lost_focus()
                                    {
                                        let new_text = state.phoneme_buffer.trim().to_string();
                                        commit_phoneme_edit = Some((note_index, index, new_text));
                                        state.editing_phoneme_index = None;
                                    }
                                } else {
                                    painter.rect_filled(
                                        pill_rect,
                                        Rounding::same(4.0),
                                        if is_selected {
                                            Color32::from_rgb(45, 32, 70)
                                        } else {
                                            Color32::from_rgb(24, 18, 36)
                                        },
                                    );
                                    painter.rect_stroke(
                                        pill_rect,
                                        Rounding::same(4.0),
                                        Stroke::new(
                                            if is_selected { 1.4_f32 } else { 1.0_f32 },
                                            if is_selected {
                                                Color32::from_rgb(255, 215, 80)
                                            } else {
                                                Color32::from_rgb(120, 100, 170)
                                            },
                                        ),
                                    );
                                    painter.galley(
                                        Pos2::new(
                                            badge_x - text_shape.size().x * 0.5,
                                            badge_y - text_shape.size().y * 0.5,
                                        ),
                                        text_shape,
                                        Color32::WHITE,
                                    );
                                }
                            }

                            if visible_end > visible_start + 14.0 {
                                painter.text(
                                    Pos2::new(
                                        (visible_start + visible_end) * 0.5,
                                        (y_top + y_bottom) * 0.5,
                                    ),
                                    egui::Align2::CENTER_CENTER,
                                    format!("{} ({:.0}ms)", label, duration),
                                    egui::FontId::proportional(9.5),
                                    Color32::from_rgb(235, 225, 250),
                                );
                            }

                            if index + 1 < subphonemes.len() {
                                let boundary_index = index + 1;
                                let boundary_offset_ms = *rel_pos + *duration;
                                let boundary_x = x_start + (boundary_offset_ms * px_per_ms) as f32;
                                if boundary_x >= strip_rect.min.x - 8.0
                                    && boundary_x <= strip_rect.max.x + 8.0
                                {
                                    let boundary_x =
                                        boundary_x.clamp(strip_rect.min.x, strip_rect.max.x);
                                    let color = Color32::from_rgb(255, 205, 70);
                                    let separator_hit_rect = Rect::from_min_max(
                                        Pos2::new(boundary_x - 9.0, y_top - 2.0),
                                        Pos2::new(boundary_x + 9.0, y_bottom + 2.0),
                                    );
                                    let separator_response = ui.interact(
                                        separator_hit_rect,
                                        ui.id().with((
                                            "subphoneme_boundary",
                                            note_index,
                                            boundary_index,
                                        )),
                                        Sense::drag(),
                                    );
                                    let is_hovered = separator_response.hovered();
                                    let is_dragging = state
                                        .dragging_subphoneme_boundary
                                        .is_some_and(|(drag_note, drag_boundary, _, _)| {
                                            drag_note == note_index
                                                && drag_boundary == boundary_index
                                        });
                                    let separator_color = if is_hovered || is_dragging {
                                        Color32::WHITE
                                    } else {
                                        color
                                    };
                                    painter.line_segment(
                                        [
                                            Pos2::new(boundary_x, y_top),
                                            Pos2::new(boundary_x, y_bottom),
                                        ],
                                        Stroke::new(
                                            if is_hovered || is_dragging {
                                                3.5_f32
                                            } else {
                                                2.0_f32
                                            },
                                            separator_color,
                                        ),
                                    );
                                    painter.circle(
                                        Pos2::new(boundary_x, y_top),
                                        4.0,
                                        separator_color,
                                        Stroke::new(1.0_f32, Color32::BLACK),
                                    );
                                    painter.circle(
                                        Pos2::new(boundary_x, y_bottom),
                                        4.0,
                                        separator_color,
                                        Stroke::new(1.0_f32, Color32::BLACK),
                                    );
                                    if is_hovered || is_dragging {
                                        ui.ctx()
                                            .set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                                        if let Some(pos) = pointer_pos {
                                            painter.text(
                                                Pos2::new(boundary_x, y_top - 4.0),
                                                egui::Align2::CENTER_BOTTOM,
                                                format!(
                                                    "Divisão: {:.0}ms (arraste para ajustar)",
                                                    boundary_offset_ms
                                                ),
                                                egui::FontId::proportional(10.0),
                                                Color32::WHITE,
                                            );
                                            if separator_response
                                                .drag_started_by(egui::PointerButton::Primary)
                                            {
                                                on_before_change();
                                                if note.phoneme_durations_ms.len()
                                                    != subphonemes.len()
                                                {
                                                    note.phoneme_durations_ms =
                                                        subphonemes.iter().map(|s| s.2).collect();
                                                }
                                                let init_left_dur = note
                                                    .phoneme_durations_ms
                                                    .get(boundary_index - 1)
                                                    .copied()
                                                    .unwrap_or(50.0);
                                                state.dragging_subphoneme_boundary = Some((
                                                    note_index,
                                                    boundary_index,
                                                    pos.x,
                                                    init_left_dur,
                                                ));
                                                state.selected_note_index = Some(note_index);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        let badge_x = if x_start < strip_rect.min.x && x_end > strip_rect.min.x {
                            (strip_rect.min.x + 28.0).min(x_end - 16.0)
                        } else if x_start > strip_rect.max.x && x_start < strip_rect.max.x + 80.0 {
                            strip_rect.max.x - 28.0
                        } else {
                            x_start + 12.0
                        };
                        let badge_y = strip_rect.min.y + 11.0;
                        if badge_x >= strip_rect.min.x - 10.0 && badge_x <= strip_rect.max.x + 10.0
                        {
                            painter.line_segment(
                                [
                                    Pos2::new(badge_x, badge_y + 7.0),
                                    Pos2::new(badge_x.clamp(x_start, x_end), y_top),
                                ],
                                Stroke::new(
                                    1.0_f32,
                                    Color32::from_rgba_unmultiplied(140, 125, 185, 160),
                                ),
                            );
                        }
                        let prev_v = prior_lyric
                            .as_deref()
                            .and_then(|l| l.chars().last())
                            .unwrap_or('-');
                        let display_alias = if is_plus {
                            format!("+ {}", lyric_trimmed)
                        } else if has_previous_adjacent {
                            format!("{} {}", prev_v, lyric_trimmed)
                        } else {
                            format!("- {}", lyric_trimmed)
                        };
                        let has_mod = note.expressions.consonant_timing_offset_ms.abs() > 0.5
                            || note.expressions.preutter_offset_ms.abs() > 0.5
                            || note.expressions.overlap_offset_ms.abs() > 0.5
                            || !note.phoneme_durations_ms.is_empty();
                        let badge_text = if has_mod {
                            format!("{}*", display_alias)
                        } else {
                            display_alias
                        };
                        let text_shape = painter.layout_no_wrap(
                            badge_text,
                            egui::FontId::proportional(10.0),
                            if is_selected {
                                Color32::WHITE
                            } else {
                                Color32::from_rgb(230, 225, 245)
                            },
                        );
                        let pill_w = (text_shape.size().x + 12.0).max(28.0);
                        let pill_rect = Rect::from_center_size(
                            Pos2::new(badge_x, badge_y),
                            Vec2::new(pill_w, 16.0),
                        );

                        let is_editing_this = state.editing_phoneme_index == Some((note_index, 0));
                        let pill_id = ui.id().with(("phoneme_pill_single", note_index));
                        let pill_resp = ui.interact(pill_rect, pill_id, Sense::click());
                        if !was_editing_phoneme && pill_resp.double_clicked() {
                            state.editing_phoneme_index = Some((note_index, 0));
                            state.phoneme_buffer = lyric_trimmed.clone();
                            state.phoneme_needs_select_all = true;
                            state.selected_note_index = Some(note_index);
                        }
                        if pill_resp.hovered() && !is_editing_this {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                        }

                        if is_editing_this {
                            let edit_w = (pill_rect.width().max(65.0)).min(160.0);
                            let edit_rect = Rect::from_center_size(
                                Pos2::new(badge_x, badge_y),
                                Vec2::new(edit_w, 18.0),
                            );
                            painter.rect_filled(
                                edit_rect,
                                Rounding::same(4.0),
                                Color32::from_rgb(45, 30, 75),
                            );
                            painter.rect_stroke(
                                edit_rect,
                                Rounding::same(4.0),
                                Stroke::new(1.8_f32, Color32::from_rgb(255, 215, 80)),
                            );

                            let text_id = ui.make_persistent_id(format!(
                                "phoneme_pill_single_edit_{}",
                                note_index
                            ));
                            if state.phoneme_needs_select_all {
                                let char_count = state.phoneme_buffer.chars().count();
                                let mut te_state = egui::text_edit::TextEditState::default();
                                te_state.cursor.set_char_range(Some(
                                    egui::text::CCursorRange::two(
                                        egui::text::CCursor::new(0),
                                        egui::text::CCursor::new(char_count),
                                    ),
                                ));
                                te_state.store(ui.ctx(), text_id);
                                state.phoneme_needs_select_all = false;
                            }

                            let text_resp = ui.put(
                                edit_rect,
                                egui::TextEdit::singleline(&mut state.phoneme_buffer)
                                    .id(text_id)
                                    .text_color(Color32::WHITE)
                                    .desired_width(edit_rect.width())
                                    .font(egui::FontId::proportional(11.0))
                                    .margin(egui::Margin::symmetric(4.0, 1.0)),
                            );

                            let is_enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let is_escape = ui.input(|i| i.key_pressed(egui::Key::Escape));
                            let is_clicked_outside = ui.input(|i| {
                                i.pointer.button_pressed(egui::PointerButton::Primary)
                                    && i.pointer
                                        .interact_pos()
                                        .is_some_and(|pos| !edit_rect.contains(pos))
                            });

                            if !is_enter && !is_escape && !text_resp.lost_focus() {
                                text_resp.request_focus();
                            }

                            if is_escape {
                                state.editing_phoneme_index = None;
                            } else if is_enter || is_clicked_outside || text_resp.lost_focus() {
                                let new_text = state.phoneme_buffer.trim().to_string();
                                commit_phoneme_edit = Some((note_index, 0, new_text));
                                state.editing_phoneme_index = None;
                            }
                        } else {
                            painter.rect_filled(
                                pill_rect,
                                Rounding::same(4.0),
                                if is_selected {
                                    Color32::from_rgb(38, 28, 56)
                                } else {
                                    Color32::from_rgb(18, 14, 28)
                                },
                            );
                            painter.rect_stroke(
                                pill_rect,
                                Rounding::same(4.0),
                                Stroke::new(
                                    if is_selected { 1.4_f32 } else { 1.0_f32 },
                                    if is_selected {
                                        Color32::from_rgb(255, 215, 80)
                                    } else {
                                        Color32::from_rgb(110, 95, 150)
                                    },
                                ),
                            );
                            painter.galley(
                                Pos2::new(
                                    badge_x - text_shape.size().x * 0.5,
                                    badge_y - text_shape.size().y * 0.5,
                                ),
                                text_shape,
                                Color32::WHITE,
                            );
                        }
                    }

                    if let Some((drag_idx, drag_kind, init_x, init_val)) =
                        state.dragging_phoneme_handle
                    {
                        if drag_idx == note_index {
                            if let Some(pos) = pointer_pos {
                                let delta_px = pos.x - init_x;
                                let delta_ms = delta_px as f64 / px_per_ms;
                                match drag_kind {
                                    0 => {
                                        let preutter = (base_preutter_ms + init_val - delta_ms)
                                            .clamp(0.0, 500.0);
                                        note.expressions.preutter_offset_ms =
                                            preutter - base_preutter_ms;
                                        let overlap = (base_overlap_ms
                                            + note.expressions.overlap_offset_ms)
                                            .clamp(0.0, preutter);
                                        note.expressions.overlap_offset_ms =
                                            overlap - base_overlap_ms;
                                    }
                                    1 => {
                                        let overlap = (base_overlap_ms + init_val + delta_ms)
                                            .clamp(0.0, active_preutter_ms.max(0.0));
                                        note.expressions.overlap_offset_ms =
                                            overlap - base_overlap_ms;
                                    }
                                    _ => {
                                        note.expressions.consonant_timing_offset_ms =
                                            (init_val + delta_ms).clamp(-500.0, 500.0);
                                    }
                                }
                                state.continuous_edit_dirty = true;
                            }
                        }
                    }

                    if let Some((drag_idx, boundary, init_x, init_left_dur)) =
                        state.dragging_subphoneme_boundary
                    {
                        if drag_idx == note_index
                            && boundary > 0
                            && boundary < note.phoneme_durations_ms.len()
                        {
                            if let Some(pos) = pointer_pos {
                                let delta_px = pos.x - init_x;
                                let delta_ms = delta_px as f64 / px_per_ms;
                                let pair_total = note.phoneme_durations_ms[boundary - 1]
                                    + note.phoneme_durations_ms[boundary];
                                let min_dur = 15.0f64.min(pair_total / 2.0);
                                let new_left =
                                    (init_left_dur + delta_ms).clamp(min_dur, pair_total - min_dur);
                                note.phoneme_durations_ms[boundary - 1] = new_left;
                                note.phoneme_durations_ms[boundary] = pair_total - new_left;
                                state.continuous_edit_dirty = true;
                            }
                        }
                    }
                }

                if let Some((note_idx, sub_idx, new_text)) = commit_phoneme_edit {
                    if !new_text.is_empty() && note_idx < notes.len() {
                        on_before_change();
                        let note = &mut notes[note_idx];

                        let sep_char = if note.lyric.contains('.') {
                            Some('.')
                        } else if note.lyric.contains(';') {
                            Some(';')
                        } else if note.lyric.contains(',') {
                            Some(',')
                        } else {
                            None
                        };

                        if let Some(sep) = sep_char {
                            let mut parts: Vec<String> = note
                                .lyric
                                .split(sep)
                                .map(|p| p.trim().to_string())
                                .collect();
                            if sub_idx < parts.len() {
                                parts[sub_idx] = new_text;
                                note.lyric = parts.join(&sep.to_string());
                            } else {
                                parts.push(new_text);
                                note.lyric = parts.join(&sep.to_string());
                            }
                        } else {
                            if let Some(cached) = state.note_phonemes_cache.get(note_idx) {
                                if cached.len() > 1 && sub_idx < cached.len() {
                                    let mut parts: Vec<String> =
                                        cached.iter().map(|s| s.0.clone()).collect();
                                    parts[sub_idx] = new_text;
                                    note.lyric = parts.join(".");
                                } else {
                                    note.lyric = new_text;
                                }
                            } else {
                                note.lyric = new_text;
                            }
                        }

                        state.phoneme_cache_hash = 0;
                        state.note_phonemes_cache.clear();
                        state.phoneme_cache.clear();
                        on_note_changed();
                    }
                }
            });
        });
}
