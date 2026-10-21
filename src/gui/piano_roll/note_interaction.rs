use super::pitch_interaction;
use super::*;
use crate::gui::phoneme_palette::PhonemePaletteState;
use crate::gui::theme::ThemeConfig;
use crate::project::model::UNote;
use eframe::egui::{Painter, Rect};

pub(super) struct Context<'a> {
    pub(super) painter: &'a Painter,
    pub(super) rect: Rect,
    pub(super) visible_clip: Rect,
    pub(super) grid_start_y: f32,
    pub(super) grid_end_y: f32,
    pub(super) keyboard_width: f32,
    pub(super) theme: &'a ThemeConfig,
    pub(super) was_editing_lyric: bool,
    pub(super) playback_pulse: f32,
    pub(super) touch_first: bool,
    pub(super) snap_option: GridSnapOption,
    pub(super) bpm: f64,
}

pub(super) struct Callbacks<'a> {
    pub(super) on_preview_freq: &'a mut dyn FnMut(f64),
    pub(super) on_before_change: &'a mut dyn FnMut(),
    pub(super) on_note_changed: &'a mut dyn FnMut(),
}

pub(super) struct Result {
    pub(super) note_to_delete: Option<usize>,
    pub(super) note_to_slice: Option<(usize, f64)>,
    pub(super) commit_lyric_edit: Option<(usize, String, Option<usize>)>,
    pub(super) pending_lyric_edit: Option<(usize, Rect, f32, f32)>,
    pub(super) pending_lyric_tags: Vec<(Rect, Color32, String, Color32, bool, Rect)>,
    pub(super) pending_phoneme_badges: Vec<(Rect, String, bool, Rect, bool)>,
    pub(super) pending_mode_badges: Vec<(Rect, String)>,
    pub(super) vibrato_buttons: Vec<(usize, Rect)>,
    pub(super) mouse_interact_pos: Option<Pos2>,
    pub(super) interacted_with_note_or_ui: bool,
}

pub(super) fn draw_and_interact(
    ui: &mut egui::Ui,
    notes: &mut [UNote],
    state: &mut PianoRollState,
    phoneme_state: &mut PhonemePaletteState,
    context: Context<'_>,
    callbacks: Callbacks<'_>,
) -> Result {
    let Callbacks {
        on_preview_freq,
        on_before_change,
        on_note_changed,
    } = callbacks;
    let Context {
        painter,
        rect,
        visible_clip,
        grid_start_y,
        grid_end_y,
        keyboard_width,
        theme,
        was_editing_lyric,
        playback_pulse,
        touch_first,
        snap_option,
        bpm,
    } = context;
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
    let note_neighbors = UNote::time_neighbors(notes);

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
        || pointer.is_some_and(|pos| vibrato_buttons.iter().any(|(_, rect)| rect.contains(pos)));
    let mouse_interact_pos =
        pointer.filter(|_| !vibrato_interaction && !state.is_dragging_in_drawer);
    let mut interacted_with_note_or_ui = vibrato_interaction;
    let mut pending_lyric_tags: Vec<(Rect, Color32, String, Color32, bool, Rect)> = Vec::new();
    let mut pending_phoneme_badges: Vec<(Rect, String, bool, Rect, bool)> = Vec::new();
    let mut pending_mode_badges: Vec<(Rect, String)> = Vec::new();

    let mut pending_pitch_handles: Vec<pitch_interaction::PendingPitchHandle> = Vec::new();

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

        let note_rect = Rect::from_min_max(Pos2::new(x_start, y_top), Pos2::new(x_end, y_bottom));

        let extended_clip = visible_clip.expand2(Vec2::new(300.0, 40.0));
        if !note_rect.intersects(extended_clip) {
            continue;
        }

        let is_selected =
            state.selected_note_index == Some(idx) || state.selected_note_indices.contains(&idx);
        let is_editing_lyric = state.editing_lyric_index == Some(idx);
        let is_active_playback = state.is_playing
            && state.playhead_ms >= note.position_ms
            && state.playhead_ms <= note.position_ms + note.duration_ms;

        // Velocity dynamic modulation
        let vel_factor = (note.expressions.velocity as f32 / 100.0).clamp(0.65, 1.35);
        let regular = theme.note_fill_c32();
        // Use the same selected-note color exposed by the theme editor.
        // Previously this accidentally pointed back to `regular`, making
        // the selected-note color appear to have no effect in the Piano Roll.
        let selected = theme.note_selected_fill_c32();
        let base_color = if is_selected { selected } else { regular };
        let note_color = Color32::from_rgba_unmultiplied(
            ((base_color.r() as f32 * vel_factor).min(255.0)) as u8,
            ((base_color.g() as f32 * vel_factor).min(255.0)) as u8,
            ((base_color.b() as f32 * vel_factor).min(255.0)) as u8,
            base_color.a(),
        );

        // Active playback pulsing glow halo
        if is_active_playback {
            painter.rect_stroke(
                note_rect.expand(2.0 + playback_pulse * 1.5),
                theme.note_rounding(),
                Stroke::new(
                    1.8 + playback_pulse * 1.4,
                    theme.c32_alpha(theme.playhead_color, 0.30 + playback_pulse * 0.35),
                ),
            );
        }

        // Corpo da nota estilo moderno (DAW profissional):
        // Fundo da nota com contorno limpo
        painter.rect_filled(note_rect, theme.note_rounding(), note_color);

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
            let pill_bg = theme.c32_alpha(theme.bg_panel, 0.42);

            // Keep the lyric color theme-controlled. A fixed dark color
            // made labels unreadable on dark themes and made the theme
            // editor's text controls appear to do nothing.
            let text_color = theme.text_note_tag_c32();

            pending_lyric_tags.push((
                pill_rect,
                pill_bg,
                note.lyric.clone(),
                text_color,
                is_selected,
                note_rect,
            ));
        }

        let lyric_trimmed = note.lyric.trim();
        if let Some(mode) = note.phonemizer_override.as_deref() {
            let badge_label = crate::phonemizer::PhonemizerMode::from_name(mode)
                .map(|mode| mode.info().name.to_string())
                .unwrap_or_else(|| mode.to_string());
            let badge_rect = Rect::from_min_size(
                // Keep the phonemizer marker in the lane below the note.
                // The alias remains the only content inside the note body,
                // so choosing a per-note phonemizer cannot cover it.
                Pos2::new(x_start + 4.0, y_bottom + 2.0),
                Vec2::new(18.0, 11.0),
            );
            pending_mode_badges.push((badge_rect, badge_label));
        }
        if lyric_trimmed != "+" && !lyric_trimmed.starts_with("+ ") {
            if let Some(phoneme) = state.phoneme_cache.get(idx) {
                if !phoneme.is_empty() && phoneme != lyric_trimmed {
                    let note_width = note_rect.width();
                    if note_width >= 42.0 {
                        let pill_rect = Rect::from_min_max(
                            // Keep the resolved phoneme outside the note
                            // body.  The lyric is the primary content of
                            // a selected note and must not be covered by
                            // the secondary phoneme label.
                            Pos2::new(x_start + 5.0, y_top - 16.0),
                            Pos2::new(x_end - 5.0, y_top - 2.0),
                        );
                        pending_phoneme_badges.push((
                            pill_rect,
                            phoneme.clone(),
                            true,
                            note_rect,
                            is_selected,
                        ));
                    } else {
                        let marker_rect = Rect::from_center_size(
                            Pos2::new(x_end - 5.0, y_center),
                            Vec2::splat(6.0),
                        );
                        pending_phoneme_badges.push((
                            marker_rect,
                            phoneme.clone(),
                            false,
                            note_rect,
                            is_selected,
                        ));
                    }
                }
            }
        }

        let e = &note.envelope;
        if e.crossfade_ms > 0.0 {
            let crossfade_px = (e.crossfade_ms * state.px_per_ms as f64) as f32;
            let crossfade_left_x = (x_start - crossfade_px).max(rect.min.x + keyboard_width);
            let crossfade_right_x =
                (x_start + crossfade_px.min((x_end - x_start) * 0.8)).min(x_end);

            let crossfade_rect = Rect::from_min_max(
                Pos2::new(crossfade_left_x, y_top),
                Pos2::new(crossfade_right_x, y_bottom),
            );
            painter.rect_filled(
                crossfade_rect,
                Rounding::same(2.0),
                theme.c32_alpha(
                    theme.note_stroke,
                    (if is_selected { 0.18 } else { 0.10 })
                        + playback_pulse * if is_selected { 0.08 } else { 0.04 },
                ),
            );
            let cross_color = theme.c32_alpha(
                theme.note_stroke,
                (if is_selected { 0.72 } else { 0.48 })
                    + playback_pulse * if is_selected { 0.10 } else { 0.06 },
            );
            painter.line_segment(
                [
                    Pos2::new(crossfade_left_x, y_bottom),
                    Pos2::new(crossfade_right_x, y_top),
                ],
                Stroke::new(1.4_f32, cross_color),
            );
            painter.line_segment(
                [
                    Pos2::new(crossfade_left_x, y_top),
                    Pos2::new(crossfade_right_x, y_bottom),
                ],
                Stroke::new(1.4_f32, cross_color),
            );
            if is_selected && crossfade_right_x - crossfade_left_x > 20.0 {
                painter.text(
                    Pos2::new(
                        (crossfade_left_x + x_start) * 0.5,
                        crossfade_rect.center().y,
                    ),
                    egui::Align2::CENTER_CENTER,
                    format!("ovl {:.0}ms", e.crossfade_ms),
                    egui::FontId::proportional(8.5),
                    theme.text_note_tag_c32().linear_multiply(0.82),
                );
            }
        }
        // --- OpenUtau-Style Smooth Volume Envelope & Point Controls ---
        let env_pts = e.get_effective_points(note.duration_ms);
        let mut env_screen_pts = Vec::with_capacity(5);
        for (t_ms, vol) in env_pts.iter() {
            let px_x = x_start + (*t_ms * state.px_per_ms as f64) as f32;
            let px_y = y_bottom - (*vol / 100.0).clamp(0.0, 2.0) as f32 * (y_bottom - y_top);
            env_screen_pts.push(Pos2::new(px_x, px_y));
        }

        // High-density smooth cosine-interpolated curve for silky visual curve
        let visual_curve = e.generate_visual_curve(note.duration_ms, 12);
        let mut curve_screen_pts = Vec::with_capacity(visual_curve.len());
        for (t_ms, vol) in &visual_curve {
            let px_x = x_start + (*t_ms * state.px_per_ms as f64) as f32;
            let px_y = y_bottom - (*vol / 100.0).clamp(0.0, 2.0) as f32 * (y_bottom - y_top);
            curve_screen_pts.push(Pos2::new(px_x, px_y));
        }

        if state.show_envelope_handles {
            // 100% (0 dB) Nominal Reference Guide
            let ref_100_y = y_top;
            painter.line_segment(
                [Pos2::new(x_start, ref_100_y), Pos2::new(x_end, ref_100_y)],
                Stroke::new(0.8_f32, theme.note_stroke_c32().linear_multiply(0.22)),
            );

            // Translucent filled area under the curve
            let fill_color = if is_selected {
                theme.c32_alpha(theme.note_stroke, 0.10)
            } else {
                theme.c32_alpha(theme.note_stroke, 0.07)
            };

            let mut poly_pts = Vec::with_capacity(curve_screen_pts.len() + 2);
            poly_pts.push(Pos2::new(x_start, y_bottom));
            for pt in &curve_screen_pts {
                poly_pts.push(*pt);
            }
            let last_x = curve_screen_pts.last().map(|p| p.x).unwrap_or(x_end);
            poly_pts.push(Pos2::new(last_x, y_bottom));

            if poly_pts.len() >= 3 {
                painter.add(egui::Shape::convex_polygon(
                    poly_pts,
                    fill_color,
                    Stroke::NONE,
                ));
            }

            // Smooth glowing outline curve
            let env_color = if is_selected {
                theme.c32_alpha(theme.note_stroke, 0.62 + playback_pulse * 0.08)
            } else {
                theme.c32_alpha(theme.note_stroke, 0.48 + playback_pulse * 0.05)
            };

            for i in 0..curve_screen_pts.len().saturating_sub(1) {
                painter.line_segment(
                    [curve_screen_pts[i], curve_screen_pts[i + 1]],
                    Stroke::new(if is_selected { 1.35_f32 } else { 1.3_f32 }, env_color),
                );
            }
        }

        if is_selected && state.show_envelope_handles {
            let handle_labels = [
                "P1 (Ataque)",
                "P2 (Pico)",
                "P3 (Decaimento)",
                "P4 (Sustentação)",
                "P5 (Soltura)",
            ];
            let handle_descs = ["P1", "P2", "P3", "P4", "P5"];

            for (pt_i, pt) in env_screen_pts.iter().enumerate() {
                let is_pt_hover = mouse_interact_pos
                    .is_some_and(|m| m.distance(*pt) <= if touch_first { 20.0 } else { 13.0 });
                let is_pt_drag = state.dragging_envelope_pt == Some((idx, pt_i));
                let radius = if is_pt_drag {
                    if touch_first {
                        9.0
                    } else {
                        7.0
                    }
                } else if is_pt_hover {
                    if touch_first {
                        8.0
                    } else {
                        6.0
                    }
                } else if touch_first {
                    5.0
                } else {
                    3.5
                };

                // Outer halo glow
                if is_pt_hover || is_pt_drag {
                    painter.circle_filled(
                        *pt,
                        radius + 4.0,
                        theme.c32_alpha(theme.note_stroke, 0.22),
                    );
                }

                let fill = if is_pt_drag {
                    theme.accent_c32()
                } else if is_pt_hover {
                    theme.note_hover_c32()
                } else {
                    theme.note_stroke_c32().linear_multiply(0.78)
                };

                painter.circle_filled(*pt, radius, fill);
                painter.circle_stroke(
                    *pt,
                    radius,
                    Stroke::new(
                        1.4_f32,
                        if is_pt_drag {
                            theme.note_stroke_c32()
                        } else {
                            theme.bg_header_c32()
                        },
                    ),
                );

                if is_pt_hover || is_pt_drag {
                    let tag = handle_descs.get(pt_i).copied().unwrap_or("");
                    let name = handle_labels.get(pt_i).copied().unwrap_or("");
                    let (t_val, vol_val) = env_pts[pt_i];
                    let text = format!("{tag} [{name}]: {:.1}ms • {:.0}%", t_val, vol_val);
                    let label_pos = Pos2::new(pt.x, (pt.y - 16.0).max(y_top - 8.0));
                    let text_shape = painter.layout_no_wrap(
                        text,
                        egui::FontId::proportional(9.5),
                        theme.text_note_tag_c32(),
                    );
                    let pill_rect = Rect::from_center_size(
                        label_pos,
                        Vec2::new(text_shape.size().x + 10.0, 16.0),
                    );
                    painter.rect_filled(
                        pill_rect,
                        Rounding::same(4.0),
                        theme.c32_alpha(theme.bg_panel, 235.0 / 255.0),
                    );
                    painter.rect_stroke(
                        pill_rect,
                        Rounding::same(4.0),
                        Stroke::new(1.0_f32, theme.note_stroke_c32()),
                    );
                    painter.galley(
                        Pos2::new(
                            pill_rect.center().x - text_shape.size().x * 0.5,
                            pill_rect.center().y - text_shape.size().y * 0.5,
                        ),
                        text_shape,
                        theme.text_note_tag_c32(),
                    );
                }
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
                    theme.c32_alpha(
                        theme.pitch_curve_color,
                        (if is_selected { 35.0 } else { 18.0 }) / 255.0,
                    ),
                );
            }
        }

        let (previous_midi, is_adjacent) = if let Some(previous_idx) = note_neighbors[idx].0 {
            let (prev_m, prev_pos, prev_dur, _) = note_info[previous_idx];
            let adj = UNote::are_adjacent(prev_pos + prev_dur, note.position_ms);
            (Some(prev_m), adj)
        } else {
            (None, false)
        };

        let pitch_curve = note
            .pitch_bend
            .effective_points(previous_midi, note_midi, is_adjacent);

        let is_custom_pitch = !note.pitch_bend.points.is_empty();
        let show_pitch_anchors = state.active_tool == EditTool::PitchDraw;
        if show_pitch_anchors {
            for (pt_idx, pt) in pitch_curve.iter().enumerate() {
                let total_cents = pt.pitch_offset_cents + note.expressions.pitch_delta;
                let px_x = x_start + (pt.time_offset_ms * state.px_per_ms as f64) as f32;
                let px_y = y_center - (total_cents / 100.0) as f32 * state.row_height;
                let pt_pos = Pos2::new(px_x, px_y);
                if visible_clip.expand(50.0).contains(pt_pos) {
                    pending_pitch_handles.push(pitch_interaction::PendingPitchHandle {
                        note_idx: idx,
                        pt_idx,
                        pos: pt_pos,
                        cents: total_cents,
                        shape: pt.shape.clone(),
                        is_custom: is_custom_pitch,
                    });
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
                && (state.active_tool == EditTool::Pointer || state.active_tool == EditTool::Pencil)
                && (resize_handle_left.contains(mpos) || resize_handle_right.contains(mpos))
            {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
            }

            // Pitch anchors have exclusive pointer ownership. Mark the
            // canvas as consumed before note selection/marquee handling
            // runs in this frame.
            let pitch_anchor_hit = state.active_tool == EditTool::PitchDraw
                && pitch_curve.iter().any(|pt| {
                    let px_x = x_start + (pt.time_offset_ms * state.px_per_ms as f64) as f32;
                    let px_y = y_center
                        - ((pt.pitch_offset_cents + note.expressions.pitch_delta) / 100.0) as f32
                            * state.row_height;
                    mpos.distance(Pos2::new(px_x, px_y)) <= if touch_first { 22.0 } else { 16.0 }
                });
            if pitch_anchor_hit {
                interacted_with_note_or_ui = true;
            }

            let is_drag_this_env = state
                .dragging_envelope_pt
                .is_some_and(|(d_idx, _)| d_idx == idx);

            let hovered_env_pt = if (is_selected || is_drag_this_env) && state.show_envelope_handles
            {
                env_screen_pts
                    .iter()
                    .position(|pt| mpos.distance(*pt) <= if touch_first { 21.0 } else { 14.0 })
            } else {
                None
            };

            let is_over_envelope_handle = is_selected
                && state.show_envelope_handles
                && (is_drag_this_env || hovered_env_pt.is_some());

            if is_over_envelope_handle {
                interacted_with_note_or_ui = true;
                if state.dragging_envelope_pt.is_some() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                } else {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }
            }

            let just_pressed = ui.input(|i| i.pointer.primary_pressed());
            if just_pressed && hovered_env_pt.is_some() {
                if let Some(pt_idx) = hovered_env_pt {
                    on_before_change();
                    state.dragging_envelope_pt = Some((idx, pt_idx));
                    state.selected_note_index = Some(idx);
                    if !state.selected_note_indices.contains(&idx) {
                        state.selected_note_indices.clear();
                        state.selected_note_indices.insert(idx);
                    }
                }
            }

            if let Some((drag_idx, pt_idx)) = state.dragging_envelope_pt {
                if drag_idx == idx && ui.input(|i| i.pointer.primary_down()) {
                    interacted_with_note_or_ui = true;
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    let duration = note.duration_ms.max(1.0);
                    let time = f64::from(mpos.x - x_start) / f64::from(state.px_per_ms);
                    let volume =
                        ((y_bottom - mpos.y) / (y_bottom - y_top)).clamp(0.0, 2.0) as f64 * 100.0;
                    note.envelope.set_point(pt_idx, time, volume, duration);
                    state.continuous_edit_dirty = true;
                }
            }

            if hovered_env_pt.is_some()
                && ui.input(|i| {
                    i.pointer
                        .button_double_clicked(egui::PointerButton::Primary)
                })
            {
                if let Some(pt_i) = hovered_env_pt {
                    on_before_change();
                    note.envelope.reset_point(pt_i);
                    state.continuous_edit_dirty = true;
                    on_note_changed();
                }
            }

            if hovered_env_pt.is_some() && ui.input(|i| i.pointer.secondary_clicked()) {
                if let Some(pt_i) = hovered_env_pt {
                    on_before_change();
                    note.envelope.reset_point(pt_i);
                    state.continuous_edit_dirty = true;
                    on_note_changed();
                }
            }

            if note_rect.contains(mpos)
                && !pitch_anchor_hit
                && !is_over_envelope_handle
                && mpos.y > grid_start_y
                && mpos.y < grid_end_y
            {
                interacted_with_note_or_ui = true;

                if state.active_tool == EditTool::Pointer || state.active_tool == EditTool::Pencil {
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

                    let is_over_pitch_anchor = state.active_tool == EditTool::PitchDraw
                        && (state.dragging_pitch_pt.is_some()
                            || pitch_curve.iter().any(|pt| {
                                let px_x =
                                    x_start + (pt.time_offset_ms * state.px_per_ms as f64) as f32;
                                let px_y = y_center
                                    - ((pt.pitch_offset_cents + note.expressions.pitch_delta)
                                        / 100.0) as f32
                                        * state.row_height;
                                mpos.distance(Pos2::new(px_x, px_y)) <= 16.0
                            }));

                    // Use primary_pressed() (not primary_clicked()) so drag starts
                    // on mouse-down, enabling click-and-drag for resize/move
                    if just_pressed
                        && !is_editing_lyric
                        && state.dragging_note_idx.is_none()
                        && state.dragging_pitch_pt.is_none()
                        && state.dragging_envelope_pt.is_none()
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
                        state.drag_start_pos =
                            Some(Pos2::new(mpos.x - rect.min.x, mpos.y - rect.min.y));
                        state.note_original_start_ms = note.position_ms;
                        state.note_original_duration_ms = note.duration_ms;
                        state.note_original_midi = note_midi;
                        state.dragging_is_resize = resize_handle_right.contains(mpos);
                        state.dragging_is_left_resize = resize_handle_left.contains(mpos);

                        if !state.dragging_is_resize && !state.dragging_is_left_resize {
                            on_preview_freq(midi_to_freq(note_midi as f64));
                        }

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
                } else if state.active_tool == EditTool::Eraser
                    && ui.input(|i| i.pointer.primary_clicked())
                {
                    note_to_delete = Some(idx);
                }
            }
        }
    }

    pitch_interaction::draw_and_interact(
        ui,
        notes,
        state,
        pitch_interaction::PitchLayerContext {
            rect,
            keyboard_width,
            visible_clip,
            grid_start_y,
            note_info: &note_info,
            note_neighbors: &note_neighbors,
            pending_pitch_handles: &pending_pitch_handles,
            mouse_interact_pos,
            theme,
            is_alt: ui.input(|i| i.modifiers.alt),
            is_shift: ui.input(|i| i.modifiers.shift),
        },
        on_before_change,
        on_note_changed,
    );
    Result {
        note_to_delete,
        note_to_slice,
        commit_lyric_edit,
        pending_lyric_edit,
        pending_lyric_tags,
        pending_phoneme_badges,
        pending_mode_badges,
        vibrato_buttons,
        mouse_interact_pos,
        interacted_with_note_or_ui,
    }
}
