use crate::gui::types::EditTool;
use crate::gui::KamafeuStudioApp;
use eframe::egui;
use eframe::egui::Key;

impl KamafeuStudioApp {
    pub(super) fn update_keyboard_shortcuts(&mut self, ctx: &egui::Context) {
        let is_editing_lyric = self.piano_roll_state.editing_lyric_index.is_some();
        if !is_editing_lyric {
            let mut toggle_play = false;
            ctx.input(|i| {
                if i.key_pressed(Key::Space) {
                    toggle_play = true;
                }
            });
            if toggle_play {
                if self.piano_roll_state.is_playing || self.render_rx.is_some() {
                    self.pause_audio();
                } else {
                    self.play_current_track();
                }
            }
        }

        let is_editing_lyric = self.piano_roll_state.editing_lyric_index.is_some();

        if !is_editing_lyric {
            let mut do_undo = false;
            let mut do_redo = false;
            let mut do_cut = false;
            let mut do_copy = false;
            let mut do_paste = false;
            let mut do_duplicate = false;
            let mut do_delete = false;
            let mut do_new = false;
            let mut do_open = false;
            let mut do_save = false;
            let mut do_save_as = false;
            let mut do_export = false;
            let transpose_semitones: i32 = 0;
            let mut nudge_ms: f64 = 0.0;
            let mut duration_nudge_ms: f64 = 0.0;

            let mut do_save_version = false;
            let mut do_toggle_log = false;
            let mut do_toggle_prefs = false;
            let mut do_select_all = false;
            let mut do_deselect_all = false;
            let mut do_toggle_drawer = false;
            let mut do_toggle_phonemes = false;
            let mut do_toggle_inspector = false;
            let mut do_toggle_arrangement = false;
            let mut do_toggle_maximize = false;
            let mut do_toggle_mute = false;
            let mut do_reset_zoom = false;
            let mut do_open_help = false;

            ctx.input(|i| {
                let has_cmd_or_ctrl = i.modifiers.command || i.modifiers.ctrl;

                if has_cmd_or_ctrl && i.key_pressed(Key::Z) {
                    if i.modifiers.shift {
                        do_redo = true;
                    } else {
                        do_undo = true;
                    }
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::Y) {
                    do_redo = true;
                }

                if has_cmd_or_ctrl && i.key_pressed(Key::N) {
                    do_new = true;
                } else if i.key_pressed(Key::N) || i.key_pressed(Key::Num2) {
                    if self.piano_roll_state.active_tool == EditTool::Pencil {
                        self.piano_roll_state.active_tool = EditTool::Pointer;
                    } else {
                        self.piano_roll_state.active_tool = EditTool::Pencil;
                    }
                }
                if i.key_pressed(Key::V) || i.key_pressed(Key::Num1) {
                    if self.piano_roll_state.active_tool != EditTool::Pointer {
                        self.piano_roll_state.active_tool = EditTool::Pointer;
                    } else {
                        // Multi-press on 1 / V cycles playback/selection modes: Loop -> Sel -> Normal
                        if !self.transport_state.loop_enabled
                            && !self.transport_state.preview_selection_only
                        {
                            self.transport_state.loop_enabled = true;
                            self.transport_state.preview_selection_only = false;
                        } else if self.transport_state.loop_enabled {
                            self.transport_state.loop_enabled = false;
                            self.transport_state.preview_selection_only = true;
                        } else {
                            self.transport_state.loop_enabled = false;
                            self.transport_state.preview_selection_only = false;
                        }
                    }
                }
                if i.key_pressed(Key::P) || i.key_pressed(Key::Num3) {
                    if self.piano_roll_state.active_tool == EditTool::PitchDraw {
                        self.piano_roll_state.pitch_sub_tool =
                            match self.piano_roll_state.pitch_sub_tool {
                                crate::gui::types::PitchSubTool::Freehand => {
                                    crate::gui::types::PitchSubTool::Line
                                }
                                crate::gui::types::PitchSubTool::Line => {
                                    crate::gui::types::PitchSubTool::Vibrato
                                }
                                crate::gui::types::PitchSubTool::Vibrato => {
                                    crate::gui::types::PitchSubTool::Smooth
                                }
                                crate::gui::types::PitchSubTool::Smooth => {
                                    crate::gui::types::PitchSubTool::Freehand
                                }
                            };
                    } else {
                        self.piano_roll_state.active_tool = EditTool::PitchDraw;
                    }
                }
                if i.key_pressed(Key::C) && !has_cmd_or_ctrl || i.key_pressed(Key::Num4) {
                    self.piano_roll_state.active_tool = EditTool::Slice;
                }
                if i.key_pressed(Key::E) && !has_cmd_or_ctrl || i.key_pressed(Key::Num5) {
                    self.piano_roll_state.active_tool = EditTool::Eraser;
                }
                if i.key_pressed(Key::L)
                    && !has_cmd_or_ctrl
                    && !i.modifiers.alt
                    && !i.modifiers.shift
                {
                    self.transport_state.loop_enabled = !self.transport_state.loop_enabled;
                }
                if i.key_pressed(Key::S)
                    && !has_cmd_or_ctrl
                    && !i.modifiers.alt
                    && !i.modifiers.shift
                {
                    self.transport_state.preview_selection_only =
                        !self.transport_state.preview_selection_only;
                }
                if has_cmd_or_ctrl && i.modifiers.shift && i.key_pressed(Key::L) {
                    self.lyrics_dialog_state.is_open = true;
                }
                if has_cmd_or_ctrl && i.modifiers.alt && i.key_pressed(Key::L) {
                    self.lyrics_dialog_state.is_open = true;
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::H) {
                    self.humanize_dialog_state.is_open = !self.humanize_dialog_state.is_open;
                }
                if has_cmd_or_ctrl && i.modifiers.alt && i.key_pressed(Key::F) {
                    if !self.fx_rack_dialog_state.is_open {
                        self.fx_rack_dialog_state.target_track = Some(self.active_track_index);
                        self.fx_rack_dialog_state.is_open = true;
                    } else {
                        self.fx_rack_dialog_state.is_open = false;
                    }
                }
                if has_cmd_or_ctrl && i.modifiers.alt && i.key_pressed(Key::P) {
                    self.autopitch_window_open = true;
                }
                if has_cmd_or_ctrl && i.modifiers.alt && i.key_pressed(Key::T) {
                    self.theme_editor_dialog_state.is_open =
                        !self.theme_editor_dialog_state.is_open;
                }

                if has_cmd_or_ctrl && i.key_pressed(Key::O) {
                    do_open = true;
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::S) {
                    if i.modifiers.alt {
                        do_save_version = true;
                    } else if i.modifiers.shift {
                        do_save_as = true;
                    } else {
                        do_save = true;
                    }
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::E) {
                    do_export = true;
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::L) {
                    do_toggle_log = true;
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::Comma) {
                    do_toggle_prefs = true;
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::A) {
                    if i.modifiers.shift {
                        do_deselect_all = true;
                    } else {
                        do_select_all = true;
                    }
                }
                if i.key_pressed(Key::F1) || (has_cmd_or_ctrl && i.key_pressed(Key::Slash)) {
                    do_open_help = true;
                }
                if i.key_pressed(Key::Tab) {
                    do_toggle_drawer = true;
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::B) {
                    do_toggle_inspector = true;
                }
                if i.modifiers.alt && i.key_pressed(Key::A) {
                    do_toggle_arrangement = true;
                }
                if i.modifiers.alt && i.key_pressed(Key::O) {
                    do_toggle_phonemes = true;
                }
                if i.key_pressed(Key::F11) || (i.modifiers.shift && i.key_pressed(Key::F)) {
                    do_toggle_maximize = true;
                }
                if i.key_pressed(Key::M) && !has_cmd_or_ctrl {
                    do_toggle_mute = true;
                }

                if has_cmd_or_ctrl && i.key_pressed(Key::X) {
                    do_cut = true;
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::C) {
                    do_copy = true;
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::V) {
                    do_paste = true;
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::D) {
                    do_duplicate = true;
                }
                if i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace) {
                    do_delete = true;
                }

                if i.modifiers.shift {
                    if i.key_pressed(Key::ArrowLeft) {
                        duration_nudge_ms -= 50.0;
                    }
                    if i.key_pressed(Key::ArrowRight) {
                        duration_nudge_ms += 50.0;
                    }
                } else {
                    if i.key_pressed(Key::ArrowLeft) {
                        nudge_ms -= 50.0;
                    }
                    if i.key_pressed(Key::ArrowRight) {
                        nudge_ms += 50.0;
                    }
                }

                if has_cmd_or_ctrl && (i.key_pressed(Key::Equals) || i.key_pressed(Key::Plus)) {
                    self.piano_roll_state.px_per_ms =
                        (self.piano_roll_state.px_per_ms * 1.25).min(1.0);
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::Minus) {
                    self.piano_roll_state.px_per_ms =
                        (self.piano_roll_state.px_per_ms * 0.8).max(0.05);
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::Num0) {
                    do_reset_zoom = true;
                }
            });

            if do_new {
                self.new_project();
            }
            if do_open {
                self.open_project_dialog();
            }
            if do_save {
                self.save_project();
            }
            if do_save_version {
                self.save_project_incremental_version();
            }
            if do_save_as {
                self.save_project_as_dialog();
            }
            if do_export {
                self.export_wav();
            }
            if do_toggle_log {
                self.render_log_window_open = !self.render_log_window_open;
            }
            if do_toggle_prefs {
                self.preferences_window_open = !self.preferences_window_open;
            }
            if do_open_help {
                self.shortcuts_guide_open = true;
            }
            if do_toggle_drawer {
                self.piano_roll_state.show_parameters_drawer =
                    !self.piano_roll_state.show_parameters_drawer;
            }
            if do_toggle_phonemes {
                self.piano_roll_state.show_phoneme_ruler =
                    !self.piano_roll_state.show_phoneme_ruler;
            }
            if do_toggle_inspector {
                self.piano_roll_state.show_inspector = !self.piano_roll_state.show_inspector;
            }
            if do_toggle_arrangement {
                self.piano_roll_state.show_arrangement_view =
                    !self.piano_roll_state.show_arrangement_view;
            }
            if do_toggle_maximize {
                self.piano_roll_state.is_maximized = !self.piano_roll_state.is_maximized;
            }
            if do_toggle_mute {
                if let Some(track) = self.project.tracks.get_mut(self.active_track_index) {
                    track.mute = !track.mute;
                }
            }
            if do_reset_zoom {
                self.piano_roll_state.px_per_ms = 0.25;
                self.piano_roll_state.row_height = 22.0;
            }
            if do_select_all {
                let note_count = self.current_notes().len();
                self.piano_roll_state.selected_note_indices = (0..note_count).collect();
                self.piano_roll_state.selected_note_index =
                    if note_count > 0 { Some(0) } else { None };
            }
            if do_deselect_all {
                self.piano_roll_state.selected_note_indices.clear();
                self.piano_roll_state.selected_note_index = None;
            }

            if do_undo {
                self.pending_edit_snapshot = None;
                if let Some(prev) = self.undo_manager.undo(self.project.clone()) {
                    self.project = prev;
                    self.transport_state.status_message = "Desfeito (Undo)".to_string();
                }
            }
            if do_redo {
                self.pending_edit_snapshot = None;
                if let Some(next) = self.undo_manager.redo(self.project.clone()) {
                    self.project = next;
                    self.transport_state.status_message = "Refeito (Redo)".to_string();
                }
            }
            if do_cut {
                self.cut_selected_notes();
            }
            if do_copy {
                self.copy_selected_notes();
            }
            if do_paste {
                self.paste_notes();
            }
            if do_duplicate {
                if let Some(sel_idx) = self.piano_roll_state.selected_note_index {
                    let notes = self.current_notes();
                    if sel_idx < notes.len() {
                        let mut dup = notes[sel_idx].clone();
                        dup.position_ms += dup.duration_ms;
                        self.push_history();
                        self.current_notes_mut().push(dup);
                        self.piano_roll_state.selected_note_index =
                            Some(self.current_notes().len() - 1);
                    }
                }
            }
            if do_delete {
                self.delete_selected_notes();
            }
            if transpose_semitones != 0 {
                self.transpose_selected_note(transpose_semitones);
            }
            if nudge_ms != 0.0 {
                if let Some(sel_idx) = self.piano_roll_state.selected_note_index {
                    self.push_history();
                    let notes = self.current_notes_mut();
                    if sel_idx < notes.len() {
                        notes[sel_idx].position_ms =
                            (notes[sel_idx].position_ms + nudge_ms).max(0.0);
                    }
                }
            }
            if duration_nudge_ms != 0.0 {
                if let Some(sel_idx) = self.piano_roll_state.selected_note_index {
                    self.push_history();
                    let notes = self.current_notes_mut();
                    if sel_idx < notes.len() {
                        notes[sel_idx].duration_ms =
                            (notes[sel_idx].duration_ms + duration_nudge_ms).max(50.0);
                    }
                }
            }
        }
    }
}
