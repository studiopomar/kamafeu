mod context_menu;
mod ghost_notes;
#[cfg(test)]
mod interaction_tests;
mod lyric;
pub use lyric::{active_phoneme_query, replace_active_phoneme, PHONEME_SEPARATORS};
mod minimap;
mod note_interaction;
mod note_overlays;
pub(crate) use minimap::TimelinePlaybackStatus;
mod navigation;
mod note_properties;
mod parameter_drawer;
mod phoneme_cache;
mod pitch_data;
mod pitch_interaction;
mod processed_waveform;
mod ruler;
mod vibrato_button;
use crate::dsp::pitch::{midi_to_freq, midi_to_note_name};
use crate::dsp::pitch_bend::PitchBendSolver;
use crate::gui::phoneme_palette::PhonemePaletteState;
use crate::gui::types::{AutoScrollMode, EditTool, GridSnapOption};
use crate::oto::Voicebank;
use crate::project::model::{UNote, UPitchBendPoint, UProjectMarker, UProjectSection, UVoicePart};
use eframe::egui::{self, Color32, Key, Pos2, Rect, Rounding, Sense, Stroke, Vec2};

/// One fifth is intentionally reserved for deliberate imported pitch bends,
/// but mouse editing should not let a tiny vertical miss jump an octave.
use std::collections::HashSet;
pub mod grid;
pub mod state;
pub use grid::{draw_piano_keys, draw_timeline_grid};
pub use state::{
    apply_autopitch_to_selection, apply_snap, apply_snap_with_zoom, canonical_pitch_shape,
    insert_pitch_point, set_waveform_cache_resolution, smooth_pitch_points, ArrangementMeterState,
    AutoPitchStyle, CachedPhoneme, ContextMenuCategory, MusicalScale, ParameterTab, PianoRollState,
    ROOT_NOTE_NAMES,
};

use crate::gui::theme::ThemeConfig;

fn format_timeline_time(ms: f64) -> String {
    let total_ms = ms.max(0.0).round() as u64;
    let minutes = total_ms / 60_000;
    let seconds = (total_ms / 1_000) % 60;
    let millis = total_ms % 1_000;
    format!("{minutes}:{seconds:02}.{millis:03}")
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_piano_roll(
    ui: &mut egui::Ui,
    notes: &mut Vec<UNote>,
    secondary_parts: &[&UVoicePart],
    active_part_position_ms: f64,
    markers: &[UProjectMarker],
    sections: &[UProjectSection],
    state: &mut PianoRollState,
    theme: &ThemeConfig,
    voicebank: Option<&Voicebank>,
    phoneme_state: &mut PhonemePaletteState,
    snap_option: GridSnapOption,
    bpm: f64,
    time_signature_numerator: u8,
    time_signature_denominator: u8,
    phonemizer_mode: crate::phonemizer::PhonemizerMode,
    lang: crate::config::AppLanguage,
    playback: minimap::TimelinePlaybackStatus<'_>,
    on_preview_freq: &mut dyn FnMut(f64),
    on_before_change: &mut dyn FnMut(),
    on_note_changed: &mut dyn FnMut(),
    on_playhead_scrubbed: &mut dyn FnMut(f64),
    on_edit_oto_alias: &mut dyn FnMut(&str, &str),
) {
    let was_editing_lyric = state.editing_lyric_index.is_some();
    let key_count = (state.max_midi - state.min_midi + 1) as usize;
    let ruler_height = 28.0f32;
    let _param_drawer_height = if state.show_parameters_drawer {
        state.drawer_height
    } else {
        0.0f32
    };
    let is_narrow_screen = ui.available_width() < 720.0;
    let touch_first = crate::gui::touch_first_layout(ui.ctx());
    let animation_policy = crate::gui::animation::AnimationPolicy {
        enabled: state.ui_animations_enabled,
        reduce_motion: state.ui_reduce_motion,
        speed: state.ui_animation_speed,
        frame_time_ema_ms: if state.animation_quality_reduced {
            24.0
        } else {
            16.0
        },
    };
    let animation_time = ui.input(|input| input.time) as f32;
    let playback_pulse = if animation_policy.enabled {
        0.5 + 0.5
            * animation_policy.pulse_amplitude()
            * (animation_time * std::f32::consts::TAU * 0.9 * animation_policy.visual_speed()).sin()
    } else {
        0.0
    };
    if state.is_playing && state.ui_animations_enabled {
        let interval = if state.animation_quality_reduced {
            crate::gui::animation::REDUCED_FRAME_MS
        } else {
            crate::gui::animation::NORMAL_FRAME_MS
        };
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs_f32(interval / 1000.0));
    }
    let keyboard_width = if is_narrow_screen { 44.0f32 } else { 62.0f32 };
    let navigation = navigation::update(ui, state, ruler_height, keyboard_width);
    let is_mod_zoom = navigation.is_mod_zoom;
    let is_pinch_zoom = navigation.is_pinch_zoom;
    let is_middle_panning = navigation.is_middle_panning;

    navigation::update_playhead_scroll(ui, state, keyboard_width);

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
        playback,
        on_playhead_scrubbed,
    );

    let ruler_rect = ruler::draw(
        ui,
        state,
        ruler::RulerConfig {
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
        },
        on_playhead_scrubbed,
    );

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
        time_signature_numerator,
        time_signature_denominator,
        lang,
        voicebank,
        on_note_changed,
    );

    navigation::update_pitch_follow(ui, state, notes, ui.available_height().max(100.0), bpm);

    let grid_width = (keyboard_width as f64 + total_canvas_ms * state.px_per_ms as f64) as f32;
    let grid_width = grid_width.max(3000.0);
    let grid_height = key_count as f32 * state.row_height;

    // Touch-first layouts use drag-to-scroll; native desktop keeps the primary
    // drag gesture available for direct note editing, even in a small window.
    let compact_view = crate::gui::touch_first_layout(ui.ctx());
    let mut scroll_area = egui::ScrollArea::both()
        .id_salt("piano_roll_scroll")
        .auto_shrink([false, false])
        .enable_scrolling(!is_mod_zoom && !is_pinch_zoom)
        .drag_to_scroll(compact_view);

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

    let is_primary_down = ui.input(|i| i.pointer.primary_down());
    let latest_pointer = ui.input(|i| i.pointer.latest_pos().or(i.pointer.hover_pos()));
    let is_dragging =
        (state.marquee_start.is_some() || state.dragging_note_idx.is_some()) && is_primary_down;

    if is_dragging && !state.is_middle_panning {
        if let Some(pos) = latest_pointer {
            let avail_rect = ui.available_rect_before_wrap();
            let left_edge = avail_rect.min.x + keyboard_width + 45.0;
            let right_edge = avail_rect.max.x - 45.0;

            if pos.x > right_edge {
                let dist = (pos.x - right_edge).max(0.0);
                let delta = (dist / 35.0).clamp(0.2, 5.0) * 18.0;
                let max_offset = (grid_width - (avail_rect.width() - keyboard_width)).max(0.0);
                state.horizontal_scroll_offset =
                    (state.horizontal_scroll_offset + delta).min(max_offset);
                state.is_edge_autoscrolling = true;
                ui.ctx().request_repaint();
            } else if pos.x < left_edge {
                let dist = (left_edge - pos.x).max(0.0);
                let delta = (dist / 35.0).clamp(0.2, 5.0) * 18.0;
                state.horizontal_scroll_offset = (state.horizontal_scroll_offset - delta).max(0.0);
                state.is_edge_autoscrolling = true;
                ui.ctx().request_repaint();
            }
        }
    }

    if state.is_playing
        && state.auto_scroll_mode != AutoScrollMode::Off
        && !state.horizontal_follow_user_override
        && state.minimap_drag_scroll_origin.is_none()
    {
        scroll_area = scroll_area.horizontal_scroll_offset(state.horizontal_scroll_offset);
    }
    if state.is_playing && state.vertical_pitch_follow && !state.vertical_follow_user_override {
        scroll_area = scroll_area.vertical_scroll_offset(state.vertical_scroll_offset);
    }

    if is_mod_zoom
        || is_pinch_zoom
        || state.is_scrubbing_ruler
        || is_middle_panning
        || state.is_edge_autoscrolling
        || is_dragging
        || state.minimap_navigation_active
        || state.minimap_drag_scroll_origin.is_some()
    {
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

        grid::draw_pitch_rows(
            &painter,
            grid::PitchRowLayer {
                state,
                theme,
                rect,
                grid_start_y,
                first_visible_key,
                last_visible_key,
                keyboard_width,
            },
        );

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
            time_signature_numerator,
            time_signature_denominator,
            snap_option,
        );

        // Secondary tracks are a non-interactive, translucent reference layer.
        // Draw them after the grid and before the active notes so edits and
        // selection remain unambiguous.
        ghost_notes::draw(
            &painter,
            ghost_notes::Layer {
                parts: secondary_parts,
                active_part_position_ms,
                state,
                theme,
                rect,
                visible_clip,
                keyboard_width,
                grid_start_y,
            },
        );

        let note_interaction = note_interaction::draw_and_interact(
            ui,
            notes,
            state,
            phoneme_state,
            note_interaction::Context {
                painter: &painter,
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
            },
            note_interaction::Callbacks {
                on_preview_freq,
                on_before_change,
                on_note_changed,
            },
        );
        let note_interaction::Result {
            note_to_delete,
            note_to_slice,
            mut commit_lyric_edit,
            pending_lyric_edit,
            pending_lyric_tags,
            pending_phoneme_badges,
            pending_mode_badges,
            vibrato_buttons,
            mouse_interact_pos,
            interacted_with_note_or_ui,
        } = note_interaction;

        note_overlays::draw(
            note_overlays::Context {
                ui,
                painter: &painter,
                theme,
                notes,
                lang,
                on_before_change,
                on_note_changed,
            },
            note_overlays::Pending {
                lyric_tags: pending_lyric_tags,
                phoneme_badges: pending_phoneme_badges,
                mode_badges: pending_mode_badges,
                vibrato_buttons,
            },
        );

        if let Some((idx, edit_rect, x_start, y_bottom)) = pending_lyric_edit {
            painter.rect_filled(
                edit_rect.expand(2.0),
                Rounding::same(5.0),
                theme.bg_panel_c32(),
            );
            painter.rect_stroke(
                edit_rect.expand(2.0),
                Rounding::same(5.0),
                Stroke::new(2.0_f32, theme.note_stroke_c32()),
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
                        .text_color(theme.text_primary_c32())
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
                            .fill(theme.bg_panel_c32())
                            .show(ui, |ui| {
                                ui.set_max_width(180.0);
                                ui.label(
                                    egui::RichText::new(format!(
                                        "oto.ini Suggestions ({})",
                                        candidates.len()
                                    ))
                                    .size(10.0)
                                    .color(theme.text_primary_c32()),
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
                                                    .color(theme.note_selected_fill_c32())
                                            } else {
                                                egui::RichText::new(cand)
                                                    .color(theme.text_primary_c32())
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
            if let (Some(drag_idx), Some(start_pos_canvas)) =
                (state.dragging_note_idx, state.drag_start_pos)
            {
                if ui.input(|i| i.pointer.primary_down()) {
                    if let Some(current_screen_pos) =
                        ui.input(|i| i.pointer.interact_pos().or(i.pointer.latest_pos()))
                    {
                        let current_canvas_x = current_screen_pos.x - rect.min.x;
                        let current_canvas_y = current_screen_pos.y - rect.min.y;
                        let delta_x = current_canvas_x - start_pos_canvas.x;
                        let delta_y = current_canvas_y - start_pos_canvas.y;
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
                                    if new_m != notes[drag_idx].midi_key() {
                                        on_preview_freq(midi_to_freq(new_m as f64));
                                    }
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
                        if state.auto_cut {
                            crate::gui::note_actions::resolve_monophonic_overlaps(notes);
                        }
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

        let is_primary_down = ui.input(|i| i.pointer.primary_down());
        let latest_pointer_pos = ui.input(|i| i.pointer.latest_pos()).or(mouse_interact_pos);

        // Edge autoscroll for piano roll marquee selection and note dragging
        let is_marquee_dragging = state.marquee_start.is_some() && is_primary_down;
        let is_note_dragging = (state.dragging_note_idx.is_some()) && is_primary_down;

        if (is_marquee_dragging || is_note_dragging) && !state.is_middle_panning {
            if let Some(pos) = latest_pointer_pos {
                let edge_margin = 40.0f32;
                let left_edge = visible_clip.min.x + keyboard_width + edge_margin;
                let right_edge = visible_clip.max.x - edge_margin;
                let mut scroll_delta = 0.0f32;

                if pos.x > right_edge {
                    let dist = (pos.x - right_edge).max(0.0);
                    scroll_delta = (dist / edge_margin).clamp(0.2, 4.0) * 16.0;
                } else if pos.x < left_edge {
                    let dist = (left_edge - pos.x).max(0.0);
                    scroll_delta = -(dist / edge_margin).clamp(0.2, 4.0) * 16.0;
                }

                if scroll_delta != 0.0 {
                    state.horizontal_scroll_offset =
                        (state.horizontal_scroll_offset + scroll_delta).max(0.0);
                    state.is_edge_autoscrolling = true;
                    ui.ctx().request_repaint();
                }
            }
        }

        if is_marquee_dragging {
            if let Some(pos) = latest_pointer_pos {
                let clamped_pos = Pos2::new(
                    pos.x.max(rect.min.x + keyboard_width),
                    pos.y.clamp(grid_start_y, grid_end_y),
                );
                state.marquee_current = Some(clamped_pos);
            }

            if let (Some(m_start_canvas), Some(m_curr_screen)) =
                (state.marquee_start, state.marquee_current)
            {
                let m_start_screen =
                    Pos2::new(rect.min.x + m_start_canvas.x, rect.min.y + m_start_canvas.y);
                let sel_rect = Rect::from_two_pos(m_start_screen, m_curr_screen);
                painter.rect_filled(
                    sel_rect,
                    Rounding::same(2.0),
                    theme.c32_alpha(theme.accent_color, 35.0 / 255.0),
                );
                painter.rect_stroke(
                    sel_rect,
                    Rounding::same(2.0),
                    Stroke::new(1.2_f32, theme.note_selected_fill_c32()),
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
                        let x_e = x_s + (note.duration_ms * state.px_per_ms as f64) as f32;
                        let n_rect = Rect::from_min_max(Pos2::new(x_s, y_t), Pos2::new(x_e, y_b));

                        if sel_rect.intersects(n_rect) {
                            state.selected_note_indices.insert(n_i);
                        }
                    }
                }
            }
        }

        if !is_primary_down {
            state.marquee_start = None;
            state.marquee_current = None;
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
                            let canvas_start = Pos2::new(mpos.x - rect.min.x, mpos.y - rect.min.y);
                            state.marquee_start = Some(canvas_start);
                            state.marquee_current = Some(mpos);
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
                    }

                    EditTool::Pencil
                        if ui.input(|i| i.pointer.primary_pressed())
                            && state.creating_note_idx.is_none()
                            && state.dragging_note_idx.is_none()
                            && !interacted_with_note_or_ui =>
                    {
                        on_before_change();
                        let click_x = mpos.x - (rect.min.x + keyboard_width);
                        let raw_start_ms = (click_x / state.px_per_ms) as f64;
                        let click_start_ms =
                            apply_snap_with_zoom(raw_start_ms, snap_option, bpm, state.px_per_ms)
                                .max(0.0);
                        let key_idx = ((mpos.y - grid_start_y) / state.row_height).floor() as u8;
                        let click_midi = (state.max_midi.saturating_sub(key_idx))
                            .clamp(state.min_midi, state.max_midi);

                        let new_note = UNote::new(
                            &state.default_note_lyric,
                            midi_to_note_name(click_midi),
                            click_start_ms,
                            state.default_note_duration_ms.max(1.0),
                        );
                        let mut new_note = new_note;
                        new_note.expressions.dynamics = state.default_note_dynamics;
                        new_note.expressions.volume = state.default_note_volume;
                        new_note.expressions.attack = state.default_note_attack;
                        new_note.expressions.decay = state.default_note_decay;
                        notes.push(new_note);
                        let new_idx = notes.len() - 1;
                        state.creating_note_idx = Some(new_idx);
                        state.selected_note_index = Some(new_idx);
                        state.drag_start_pos = Some(mpos);

                        let freq = midi_to_freq(click_midi as f64);
                        on_preview_freq(freq);
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
                        theme.c32_alpha(theme.accent_color, 0.08),
                    );
                    painter.rect_stroke(
                        hover_rect,
                        Rounding::ZERO,
                        Stroke::new(1.0_f32, theme.c32_alpha(theme.accent_color, 0.62)),
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
                Stroke::new(0.8_f32, theme.text_primary_c32()),
            ));

            painter.line_segment(
                [
                    Pos2::new(playhead_x, keys_y_min),
                    Pos2::new(playhead_x, keys_y_max),
                ],
                Stroke::new(
                    2.0 + playback_pulse * 1.2,
                    theme.c32_alpha(theme.playhead_color, 0.18 + playback_pulse * 0.16),
                ),
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
    if !is_mod_zoom
        && !state.is_scrubbing_ruler
        && !is_middle_panning
        && !state.is_edge_autoscrolling
        && state.marquee_start.is_none()
        && state.dragging_note_idx.is_none()
        && !state.minimap_navigation_active
        && state.minimap_drag_scroll_origin.is_none()
    {
        if !state.is_playing || state.horizontal_follow_user_override {
            state.horizontal_scroll_offset = scroll_output.state.offset.x;
        }
        if !state.is_playing || state.vertical_follow_user_override {
            state.vertical_scroll_offset = scroll_output.state.offset.y;
        }
    }
    state.minimap_navigation_active = false;
    state.is_edge_autoscrolling = false;

    processed_waveform::draw(
        ui,
        state,
        scroll_output.inner_rect,
        keyboard_width,
        timeline_scroll_x,
        theme,
        ruler_rect,
        bpm,
        lang,
        on_playhead_scrubbed,
    );

    context_menu::draw(
        ui,
        notes,
        state,
        theme,
        lang,
        on_before_change,
        on_note_changed,
    );

    note_properties::draw(ui, notes, state, theme, lang, on_note_changed);
}

fn marker_color(value: Option<&str>, theme: &ThemeConfig) -> Color32 {
    let Some(value) = value else {
        return theme.marker_c32();
    };
    let bytes = value.as_bytes();
    if bytes.len() == 7 && bytes.first() == Some(&b'#') {
        let parse = |start| {
            std::str::from_utf8(&bytes[start..start + 2])
                .ok()
                .and_then(|chunk| u8::from_str_radix(chunk, 16).ok())
        };
        if let (Some(red), Some(green), Some(blue)) = (parse(1), parse(3), parse(5)) {
            return Color32::from_rgb(red, green, blue);
        }
    }
    theme.marker_c32()
}
