use super::*;
use crate::dsp::pitch_bend::PitchBendSolver;
use crate::gui::types::PitchSubTool;
use crate::project::model::UPitchBendPoint;

const MAX_MANUAL_PITCH_CENTS: f64 = 2400.0;

pub(super) struct PendingPitchHandle {
    pub(super) note_idx: usize,
    pub(super) pt_idx: usize,
    pub(super) pos: Pos2,
    pub(super) cents: f64,
    pub(super) shape: String,
    pub(super) is_custom: bool,
}

pub(super) struct PitchLayerContext<'a> {
    pub(super) rect: Rect,
    pub(super) keyboard_width: f32,
    pub(super) visible_clip: Rect,
    pub(super) grid_start_y: f32,
    pub(super) note_info: &'a [(u8, f64, f64, f64)],
    pub(super) pending_pitch_handles: &'a [PendingPitchHandle],
    pub(super) mouse_interact_pos: Option<Pos2>,
    pub(super) theme: &'a ThemeConfig,
    pub(super) is_alt: bool,
    pub(super) is_shift: bool,
}

pub(super) fn draw_and_interact(
    ui: &mut egui::Ui,
    notes: &mut [UNote],
    state: &mut PianoRollState,
    context: PitchLayerContext<'_>,
    on_before_change: &mut dyn FnMut(),
    on_note_changed: &mut dyn FnMut(),
) {
    let PitchLayerContext {
        rect,
        keyboard_width,
        visible_clip,
        grid_start_y,
        note_info,
        pending_pitch_handles,
        mouse_interact_pos,
        theme,
        is_alt,
        is_shift,
    } = context;
    let painter = ui.painter_at(rect);
    // ====================================================================
    // OPENUTAU-GRADE CANVAS-WIDE PITCH ENGINE & INTERACTION
    // ====================================================================
    let can_interact_pitch = state.active_tool == EditTool::PitchDraw;

    // 1. Compute & Render Multi-Pass Glowing Vocal Pitch Curve
    {
        let mut global_segments: Vec<Vec<Pos2>> = Vec::new();
        let mut current_segment: Vec<Pos2> = Vec::new();

        for (idx, note) in notes.iter().enumerate() {
            let note_midi = note.midi_key();
            if note_midi < state.min_midi || note_midi > state.max_midi {
                continue;
            }

            let x_start_note =
                rect.min.x + keyboard_width + (note.position_ms * state.px_per_ms as f64) as f32;
            let x_end_note = x_start_note + (note.duration_ms * state.px_per_ms as f64) as f32;

            if x_end_note < visible_clip.min.x - 200.0 || x_start_note > visible_clip.max.x + 200.0
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
                let next_adjacent = (note.position_ms + note.duration_ms - next_pos).abs() <= 1.0;
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

            let visible_note_start = ((visible_clip.min.x - x_start_note) / state.px_per_ms) as f64;
            let visible_note_end = ((visible_clip.max.x - x_start_note) / state.px_per_ms) as f64;
            let draw_min_t = min_t.max(visible_note_start - 20.0);
            let draw_max_t = max_t.min(visible_note_end + 20.0);
            let step = (2.0 / state.px_per_ms.max(0.001)) as f64;

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

                let absolute_midi =
                    note_midi as f64 + (offset_cents + note.expressions.pitch_delta) / 100.0;
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

        // Uma linha principal e um halo discreto mantêm a curva legível
        // sem o efeito neon de múltiplas camadas sobrepostas.
        let bloom_stroke = Stroke::new(2.8_f32, theme.c32_alpha(theme.pitch_curve_color, 0.22));
        let core_pitch_stroke = Stroke::new(1.35_f32, theme.pitch_curve_c32());

        for segment in &global_segments {
            if segment.len() >= 2 {
                painter.add(egui::Shape::line(segment.clone(), bloom_stroke));
                painter.add(egui::Shape::line(segment.clone(), core_pitch_stroke));
            }
        }
    }

    // 2. Real-time Live Drawing & Tool Previews
    if let Some(mpos) = mouse_interact_pos {
        if state.active_tool == EditTool::PitchDraw {
            if state.pitch_sub_tool == PitchSubTool::Freehand
                && !state.pitch_brush_raw_stroke.is_empty()
            {
                let preview_pts: Vec<Pos2> = state
                    .pitch_brush_raw_stroke
                    .iter()
                    .map(|&(abs_t, abs_m)| {
                        let px_x =
                            rect.min.x + keyboard_width + (abs_t * state.px_per_ms as f64) as f32;
                        let px_y = grid_start_y
                            + (state.max_midi as f64 - abs_m) as f32 * state.row_height
                            + state.row_height * 0.5;
                        Pos2::new(px_x, px_y)
                    })
                    .collect();
                if preview_pts.len() >= 2 {
                    painter.add(egui::Shape::line(
                        preview_pts.clone(),
                        Stroke::new(
                            3.5_f32,
                            theme.c32_alpha(theme.pitch_curve_color, 70.0 / 255.0),
                        ),
                    ));
                    painter.add(egui::Shape::line(
                        preview_pts,
                        Stroke::new(1.8_f32, theme.pitch_curve_c32()),
                    ));
                }
            } else if state.pitch_sub_tool == PitchSubTool::Line {
                if let Some((start_t, start_m)) = state.pitch_line_start {
                    let p1_x =
                        rect.min.x + keyboard_width + (start_t * state.px_per_ms as f64) as f32;
                    let p1_y = grid_start_y
                        + (state.max_midi as f64 - start_m) as f32 * state.row_height
                        + state.row_height * 0.5;
                    let p1 = Pos2::new(p1_x, p1_y);
                    let p2 = mpos;
                    painter.line_segment(
                        [p1, p2],
                        Stroke::new(
                            3.0_f32,
                            theme.c32_alpha(theme.pitch_curve_color, 70.0 / 255.0),
                        ),
                    );
                    painter.line_segment([p1, p2], Stroke::new(1.6_f32, theme.pitch_anchor_c32()));
                    painter.circle_filled(p1, 4.0, ui.visuals().hyperlink_color);
                    painter.circle_filled(p2, 4.0, ui.visuals().hyperlink_color);
                }
            }
        }
    }

    // 3. Canvas Pitch Pointer Interaction
    if can_interact_pitch {
        if let Some(mpos) = mouse_interact_pos {
            let timeline_time_ms =
                ((mpos.x - (rect.min.x + keyboard_width)) / state.px_per_ms) as f64;
            let timeline_abs_midi = state.max_midi as f64
                - ((mpos.y - grid_start_y - state.row_height * 0.5) / state.row_height) as f64;

            // Find hovered pitch handle
            let mut hovered_handle_idx: Option<usize> = None;
            for (h_idx, handle) in pending_pitch_handles.iter().enumerate() {
                if mpos.distance(handle.pos) <= 13.0 {
                    hovered_handle_idx = Some(h_idx);
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                    break;
                }
            }

            // A segment uses the same hit target as OpenUtau's curve
            // itself. The nearest segment is selected only when no point
            // is under the pointer, so point deletion remains unambiguous.
            let hovered_segment = if hovered_handle_idx.is_none() {
                pending_pitch_handles
                    .windows(2)
                    .filter(|pair| pair[0].note_idx == pair[1].note_idx)
                    .filter_map(|pair| {
                        let start = pair[0].pos;
                        let end = pair[1].pos;
                        let delta = end - start;
                        let length_sq = delta.dot(delta);
                        if length_sq <= f32::EPSILON {
                            return None;
                        }
                        let projection = ((mpos - start).dot(delta) / length_sq).clamp(0.0, 1.0);
                        let closest = start + delta * projection;
                        let distance = mpos.distance(closest);
                        (distance <= 10.0).then_some((distance, pair[0].note_idx, pair[0].pt_idx))
                    })
                    .min_by(|left, right| {
                        left.0
                            .partial_cmp(&right.0)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    })
                    .map(|(_, note_idx, point_idx)| (note_idx, point_idx))
            } else {
                None
            };

            // Handle right-click / Alt+Click on point (Delete) or canvas right-click drag (Erase)
            if let Some(h_idx) = hovered_handle_idx {
                let handle = &pending_pitch_handles[h_idx];
                if ui.input(|i| i.pointer.primary_clicked() && i.modifiers.alt)
                    || ui.input(|i| i.pointer.secondary_clicked())
                {
                    let n_idx = handle.note_idx;
                    let pt_idx = handle.pt_idx;
                    if n_idx < notes.len() {
                        if notes[n_idx].pitch_bend.points.is_empty() {
                            let (prev_m, is_adj) = if n_idx > 0 {
                                let (pm, pp, pd, _) = note_info[n_idx - 1];
                                (Some(pm), (pp + pd - notes[n_idx].position_ms).abs() <= 1.0)
                            } else {
                                (None, false)
                            };
                            notes[n_idx].pitch_bend.points = notes[n_idx]
                                .pitch_bend
                                .effective_points(prev_m, notes[n_idx].midi_key(), is_adj);
                        }
                        if pt_idx < notes[n_idx].pitch_bend.points.len() {
                            notes[n_idx].pitch_bend.points.remove(pt_idx);
                            state.dragging_pitch_pt = None;
                            on_note_changed();
                        }
                    }
                } else if ui.input(|i| {
                    i.pointer
                        .button_double_clicked(egui::PointerButton::Primary)
                }) {
                    // Double clicking an existing point cycles its interpolation shape:
                    // S -> Linear -> EaseIn/J -> EaseOut/R -> Hermite/H -> S
                    let n_idx = handle.note_idx;
                    let pt_idx = handle.pt_idx;
                    if n_idx < notes.len() {
                        if notes[n_idx].pitch_bend.points.is_empty() {
                            let (prev_m, is_adj) = if n_idx > 0 {
                                let (pm, pp, pd, _) = note_info[n_idx - 1];
                                (Some(pm), (pp + pd - notes[n_idx].position_ms).abs() <= 1.0)
                            } else {
                                (None, false)
                            };
                            notes[n_idx].pitch_bend.points = notes[n_idx]
                                .pitch_bend
                                .effective_points(prev_m, notes[n_idx].midi_key(), is_adj);
                        }
                        if pt_idx < notes[n_idx].pitch_bend.points.len() {
                            let cur_shape = notes[n_idx].pitch_bend.points[pt_idx].shape.clone();
                            let nxt_shape =
                                crate::dsp::pitch_bend::PitchBendSolver::next_pitch_point_shape(
                                    &cur_shape,
                                );
                            notes[n_idx].pitch_bend.points[pt_idx].shape = nxt_shape.to_string();
                            on_note_changed();
                        }
                    }
                }
            } else if let Some((n_idx, pt_idx)) = hovered_segment {
                if ui.input(|i| i.pointer.secondary_clicked()) && n_idx < notes.len() {
                    if notes[n_idx].pitch_bend.points.is_empty() {
                        let (prev_m, is_adj) = if n_idx > 0 {
                            let (pm, pp, pd, _) = note_info[n_idx - 1];
                            (Some(pm), (pp + pd - notes[n_idx].position_ms).abs() <= 1.0)
                        } else {
                            (None, false)
                        };
                        notes[n_idx].pitch_bend.points = notes[n_idx].pitch_bend.effective_points(
                            prev_m,
                            notes[n_idx].midi_key(),
                            is_adj,
                        );
                    }
                    if pt_idx < notes[n_idx].pitch_bend.points.len() {
                        let current = notes[n_idx].pitch_bend.points[pt_idx].shape.clone();
                        notes[n_idx].pitch_bend.points[pt_idx].shape =
                            PitchBendSolver::next_pitch_point_shape(&current).to_string();
                        on_note_changed();
                    }
                }
            }

            // Primary Pressed: OpenUtau-style point creation or point drag.
            if ui.input(|i| i.pointer.primary_pressed()) && !is_alt {
                on_before_change();
                if let Some(h_idx) = hovered_handle_idx {
                    let handle = &pending_pitch_handles[h_idx];
                    let n_idx = handle.note_idx;
                    let pt_idx = handle.pt_idx;
                    if n_idx < notes.len() {
                        if notes[n_idx].pitch_bend.points.is_empty() {
                            let (prev_m, is_adj) = if n_idx > 0 {
                                let (pm, pp, pd, _) = note_info[n_idx - 1];
                                (Some(pm), (pp + pd - notes[n_idx].position_ms).abs() <= 1.0)
                            } else {
                                (None, false)
                            };
                            notes[n_idx].pitch_bend.points = notes[n_idx]
                                .pitch_bend
                                .effective_points(prev_m, notes[n_idx].midi_key(), is_adj);
                            notes[n_idx].pitch_bend.snap_first = false;
                        }
                        if pt_idx < notes[n_idx].pitch_bend.points.len() {
                            state.dragging_pitch_pt = Some((n_idx, pt_idx));
                        }
                    }
                } else if state.active_tool == EditTool::PitchDraw {
                    // A click on the curve creates a point, and the same
                    // gesture immediately enters point-drag mode. This is
                    // the important behavioral difference from the old
                    // sampled freehand brush.
                    let notes_len = notes.len();
                    for (n_idx, note) in notes.iter_mut().enumerate() {
                        let note_end = note.position_ms + note.duration_ms;
                        if timeline_time_ms < note.position_ms
                            || timeline_time_ms > note_end
                            || (n_idx + 1 < notes_len && timeline_time_ms >= note_end)
                        {
                            continue;
                        }

                        if note.pitch_bend.points.is_empty() {
                            let (prev_m, is_adj) = if n_idx > 0 {
                                let (pm, pp, pd, _) = note_info[n_idx - 1];
                                (Some(pm), (pp + pd - note.position_ms).abs() <= 1.0)
                            } else {
                                (None, false)
                            };
                            note.pitch_bend.points =
                                note.pitch_bend
                                    .effective_points(prev_m, note.midi_key(), is_adj);
                        }

                        let rel_t =
                            (timeline_time_ms - note.position_ms).clamp(0.0, note.duration_ms);
                        let cents = ((timeline_abs_midi - note.midi_key() as f64) * 100.0
                            - note.expressions.pitch_delta)
                            .clamp(-MAX_MANUAL_PITCH_CENTS, MAX_MANUAL_PITCH_CENTS);
                        note.pitch_bend.snap_first = false;
                        let point_idx =
                            crate::gui::piano_roll::state::insert_pitch_point(note, rel_t, cents);
                        state.dragging_pitch_pt = Some((n_idx, point_idx));
                        state.selected_note_index = Some(n_idx);
                        state.selected_note_indices.clear();
                        state.selected_note_indices.insert(n_idx);
                        break;
                    }
                }
            }

            // Primary Down: dragging points or active brush drawing
            if ui.input(|i| i.pointer.primary_down()) && !is_alt {
                if let Some((d_note_idx, d_pt_idx)) = state.dragging_pitch_pt {
                    if d_note_idx < notes.len() {
                        let note = &mut notes[d_note_idx];
                        if d_pt_idx < note.pitch_bend.points.len() {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                            let rel_t = timeline_time_ms - note.position_ms;

                            // Fine microtonal dragging: Shift gives 100% continuous microtonal precision;
                            // standard dragging has gentle +/-8c magnetic semitone attraction.
                            let snapped_abs_midi = if !is_shift {
                                let rounded_midi = timeline_abs_midi.round();
                                let diff_cents = (timeline_abs_midi - rounded_midi) * 100.0;
                                if diff_cents.abs() <= 8.0 {
                                    rounded_midi
                                } else {
                                    timeline_abs_midi
                                }
                            } else {
                                timeline_abs_midi
                            };

                            let cents = ((snapped_abs_midi - note.midi_key() as f64) * 100.0
                                - note.expressions.pitch_delta)
                                .clamp(-MAX_MANUAL_PITCH_CENTS, MAX_MANUAL_PITCH_CENTS);

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
                            let should_add = state.pitch_brush_raw_stroke.last().is_none_or(
                                |&(last_t, last_m)| {
                                    (last_t - timeline_time_ms).abs() >= 3.0
                                        || (last_m - timeline_abs_midi).abs() >= 0.04
                                },
                            );
                            if should_add {
                                state
                                    .pitch_brush_raw_stroke
                                    .push((timeline_time_ms, timeline_abs_midi));
                            }
                        }
                        PitchSubTool::Line => {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                        }
                        PitchSubTool::Vibrato => {
                            let min_dist_ms = 15.0;
                            let vibrato_freq = 5.5;
                            let center_m = state.vibrato_brush_center.unwrap_or(timeline_abs_midi);
                            let vibrato_amp_cents = 70.0;

                            for (n_idx, note) in notes.iter_mut().enumerate() {
                                if timeline_time_ms >= note.position_ms - 50.0
                                    && timeline_time_ms
                                        <= note.position_ms + note.duration_ms + 50.0
                                {
                                    if note.pitch_bend.points.is_empty() {
                                        let (prev_m, is_adj) = if n_idx > 0 {
                                            let (pm, pp, pd, _) = note_info[n_idx - 1];
                                            (Some(pm), (pp + pd - note.position_ms).abs() <= 1.0)
                                        } else {
                                            (None, false)
                                        };
                                        note.pitch_bend.points = note.pitch_bend.effective_points(
                                            prev_m,
                                            note.midi_key(),
                                            is_adj,
                                        );
                                    }
                                    let rel_t = timeline_time_ms - note.position_ms;
                                    let phase =
                                        (rel_t / 1000.0) * vibrato_freq * std::f64::consts::TAU;
                                    let base_cents = (center_m - note.midi_key() as f64) * 100.0;
                                    let vib_cents = (base_cents + phase.sin() * vibrato_amp_cents)
                                        .clamp(-MAX_MANUAL_PITCH_CENTS, MAX_MANUAL_PITCH_CENTS);

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
                            }
                        }
                        PitchSubTool::Smooth => {
                            let radius_ms = 60.0;
                            for note in notes.iter_mut() {
                                if !note.pitch_bend.points.is_empty() {
                                    let rel_t = timeline_time_ms - note.position_ms;
                                    let n_pts = note.pitch_bend.points.len();
                                    for pt_i in 0..n_pts {
                                        let p_time = note.pitch_bend.points[pt_i].time_offset_ms;
                                        if (p_time - rel_t).abs() <= radius_ms {
                                            let prev_cents = if pt_i > 0 {
                                                note.pitch_bend.points[pt_i - 1].pitch_offset_cents
                                            } else {
                                                note.pitch_bend.points[pt_i].pitch_offset_cents
                                            };
                                            let next_cents = if pt_i + 1 < n_pts {
                                                note.pitch_bend.points[pt_i + 1].pitch_offset_cents
                                            } else {
                                                note.pitch_bend.points[pt_i].pitch_offset_cents
                                            };
                                            let curr =
                                                note.pitch_bend.points[pt_i].pitch_offset_cents;
                                            let target = (prev_cents + next_cents) * 0.5;
                                            let intensity = state.pitch_smooth_intensity as f64;
                                            note.pitch_bend.points[pt_i].pitch_offset_cents =
                                                curr * (1.0 - intensity) + target * intensity;
                                        }
                                    }
                                    state.continuous_edit_dirty = true;
                                }
                            }
                        }
                    }
                }
            }

            // Primary Released: commit strokes, lines, or points
            if ui.input(|i| i.pointer.primary_released()) {
                state.vibrato_brush_center = None;
                if let Some((d_note_idx, _)) = state.dragging_pitch_pt {
                    state.dragging_pitch_pt = None;
                    if d_note_idx < notes.len() {
                        notes[d_note_idx].pitch_bend.points.sort_by(|a, b| {
                            a.time_offset_ms
                                .partial_cmp(&b.time_offset_ms)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                        state.continuous_edit_dirty = true;
                        on_note_changed();
                    }
                } else if state.pitch_sub_tool == PitchSubTool::Freehand
                    && !state.pitch_brush_raw_stroke.is_empty()
                {
                    let stroke = state.pitch_brush_raw_stroke.clone();
                    state.pitch_brush_raw_stroke.clear();

                    if stroke.len() >= 2 {
                        let s_min_t = stroke
                            .iter()
                            .map(|point| point.0)
                            .fold(f64::INFINITY, f64::min);
                        let s_max_t = stroke
                            .iter()
                            .map(|point| point.0)
                            .fold(f64::NEG_INFINITY, f64::max);

                        for (n_idx, note) in notes.iter_mut().enumerate() {
                            let n_start = note.position_ms;
                            let n_end = note.position_ms + note.duration_ms;

                            if s_max_t >= n_start - 100.0 && s_min_t <= n_end + 100.0 {
                                if note.pitch_bend.points.is_empty() {
                                    let (prev_m, is_adj) = if n_idx > 0 {
                                        let (pm, pp, pd, _) = note_info[n_idx - 1];
                                        (Some(pm), (pp + pd - note.position_ms).abs() <= 1.0)
                                    } else {
                                        (None, false)
                                    };
                                    note.pitch_bend.points = note.pitch_bend.effective_points(
                                        prev_m,
                                        note.midi_key(),
                                        is_adj,
                                    );
                                }

                                // Extract stroke segments that belong to this note
                                let note_stroke_pts: Vec<(f64, f64)> = stroke
                                    .iter()
                                    .filter(|&(t, _)| *t >= n_start - 150.0 && *t <= n_end + 150.0)
                                    .map(|&(t, m)| {
                                        let rel_t = t - note.position_ms;
                                        let cents = (m - note.midi_key() as f64) * 100.0;
                                        (rel_t, cents)
                                    })
                                    .collect();

                                if !note_stroke_pts.is_empty() {
                                    let smoothed = smooth_pitch_points(&note_stroke_pts);
                                    if !smoothed.is_empty() {
                                        let sub_min_t = smoothed.first().unwrap().time_offset_ms;
                                        let sub_max_t = smoothed.last().unwrap().time_offset_ms;

                                        note.pitch_bend.points.retain(|pt| {
                                            pt.time_offset_ms < sub_min_t - 6.0
                                                || pt.time_offset_ms > sub_max_t + 6.0
                                        });

                                        note.pitch_bend.points.extend(smoothed);
                                        note.pitch_bend.points.sort_by(|a, b| {
                                            a.time_offset_ms
                                                .partial_cmp(&b.time_offset_ms)
                                                .unwrap_or(std::cmp::Ordering::Equal)
                                        });

                                        let simplified = PitchBendSolver::simplify_pitch_points(
                                            &note.pitch_bend.points,
                                            3.5,
                                        );
                                        note.pitch_bend.points = simplified;
                                        state.continuous_edit_dirty = true;
                                        on_note_changed();
                                    }
                                }
                            }
                        }
                    }
                } else if state.pitch_sub_tool == PitchSubTool::Line {
                    if let Some((start_t, start_m)) = state.pitch_line_start {
                        let end_t = timeline_time_ms;
                        let end_m = timeline_abs_midi;
                        state.pitch_line_start = None;

                        let (l_min_t, l_max_t) = if start_t <= end_t {
                            (start_t, end_t)
                        } else {
                            (end_t, start_t)
                        };

                        for (n_idx, note) in notes.iter_mut().enumerate() {
                            let n_start = note.position_ms;
                            let n_end = note.position_ms + note.duration_ms;

                            if l_max_t >= n_start - 50.0 && l_min_t <= n_end + 50.0 {
                                if note.pitch_bend.points.is_empty() {
                                    let (prev_m, is_adj) = if n_idx > 0 {
                                        let (pm, pp, pd, _) = note_info[n_idx - 1];
                                        (Some(pm), (pp + pd - note.position_ms).abs() <= 1.0)
                                    } else {
                                        (None, false)
                                    };
                                    note.pitch_bend.points = note.pitch_bend.effective_points(
                                        prev_m,
                                        note.midi_key(),
                                        is_adj,
                                    );
                                }

                                let rel_start_t = start_t - note.position_ms;
                                let rel_start_cents = (start_m - note.midi_key() as f64) * 100.0;
                                let rel_end_t = end_t - note.position_ms;
                                let rel_end_cents = (end_m - note.midi_key() as f64) * 100.0;

                                let (min_rel, max_rel) = if rel_start_t <= rel_end_t {
                                    (rel_start_t, rel_end_t)
                                } else {
                                    (rel_end_t, rel_start_t)
                                };

                                note.pitch_bend.points.retain(|pt| {
                                    pt.time_offset_ms < min_rel || pt.time_offset_ms > max_rel
                                });

                                note.pitch_bend.points.push(UPitchBendPoint {
                                    time_offset_ms: rel_start_t,
                                    pitch_offset_cents: rel_start_cents,
                                    shape: "l".to_string(),
                                });
                                note.pitch_bend.points.push(UPitchBendPoint {
                                    time_offset_ms: rel_end_t,
                                    pitch_offset_cents: rel_end_cents,
                                    shape: "s".to_string(),
                                });

                                note.pitch_bend.points.sort_by(|a, b| {
                                    a.time_offset_ms
                                        .partial_cmp(&b.time_offset_ms)
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                });

                                let simplified = PitchBendSolver::simplify_pitch_points(
                                    &note.pitch_bend.points,
                                    3.0,
                                );
                                note.pitch_bend.points = simplified;
                                state.continuous_edit_dirty = true;
                                on_note_changed();
                            }
                        }
                    }
                }
            }
        }
    }

    // 4. Render OpenUtau-Grade Interactive Pitch Control Handles
    let show_pitch_nodes = state.active_tool == EditTool::PitchDraw;

    if show_pitch_nodes {
        for handle in pending_pitch_handles.iter() {
            let is_hovered =
                mouse_interact_pos.is_some_and(|mpos| mpos.distance(handle.pos) <= 13.0);
            let is_dragged = state.dragging_pitch_pt == Some((handle.note_idx, handle.pt_idx));

            let radius = if is_dragged {
                6.5
            } else if is_hovered {
                5.5
            } else {
                4.0
            };

            let shape_char = match handle.shape.to_lowercase().as_str() {
                "l" => "L",
                "i" => "I",
                "o" => "O",
                "sp" | "h" => "SP",
                _ => "IO",
            };

            let (bg_color, ring_color) = if is_dragged {
                (theme.accent_c32(), theme.note_stroke_c32())
            } else if is_hovered {
                (theme.note_hover_c32(), theme.note_stroke_c32())
            } else if handle.is_custom {
                (
                    theme.note_stroke_c32().linear_multiply(0.72),
                    theme.note_stroke_c32(),
                )
            } else {
                (
                    theme.note_stroke_c32().linear_multiply(0.52),
                    theme.note_stroke_c32().linear_multiply(0.82),
                )
            };

            // Node outer glow when hovered or dragging
            if is_hovered || is_dragged {
                painter.circle_filled(
                    handle.pos,
                    radius + 4.0,
                    theme.note_stroke_c32().linear_multiply(0.18),
                );
            }

            painter.circle_filled(handle.pos, radius, bg_color);
            painter.circle_stroke(handle.pos, radius, Stroke::new(1.2_f32, ring_color));

            // Display shape badge text with exact cents if hovered or dragging
            if is_hovered || is_dragged {
                let badge_text = if is_shift {
                    format!("[{}] {:+.1}c", shape_char, handle.cents)
                } else {
                    format!("[{}] {:+.0}c", shape_char, handle.cents)
                };
                let badge_font = egui::FontId::proportional(10.0);
                let badge_galley = painter.layout_no_wrap(
                    badge_text.clone(),
                    badge_font.clone(),
                    theme.text_note_tag_c32(),
                );
                let badge_pos = Pos2::new(
                    handle.pos.x - badge_galley.size().x * 0.5,
                    handle.pos.y - radius - 18.0,
                );
                let badge_rect = Rect::from_min_size(badge_pos, badge_galley.size()).expand(3.0);

                painter.rect_filled(
                    badge_rect,
                    Rounding::same(3.0),
                    theme.c32_alpha(theme.bg_panel, 225.0 / 255.0),
                );
                painter.rect_stroke(
                    badge_rect,
                    Rounding::same(3.0),
                    Stroke::new(1.0_f32, theme.note_stroke_c32().linear_multiply(0.62)),
                );
                painter.text(
                    badge_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    badge_text,
                    badge_font,
                    theme.text_note_tag_c32(),
                );
            }
        }
    }
}
