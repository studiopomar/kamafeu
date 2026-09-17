use crate::gui::piano_roll::draw_piano_roll;
use crate::gui::KamafeuStudioApp;
use eframe::egui;
use eframe::egui::CentralPanel;
use eframe::egui::Frame;

impl KamafeuStudioApp {
    pub(super) fn update_editor_canvas(&mut self, ctx: &egui::Context) {
        // Capture o projeto antes do evento que pode iniciar uma edição. O
        // callback do piano roll é processado depois do desenho do frame, quando
        // a nota já pode ter sido alterada; clonar ali produzia estados parciais.
        let piano_edit_snapshot = ctx
            .input(|input| {
                let pointer_started =
                    input.pointer.primary_pressed() || input.pointer.secondary_pressed();
                let edit_key_started = [
                    egui::Key::Enter,
                    egui::Key::Tab,
                    egui::Key::Delete,
                    egui::Key::Backspace,
                    egui::Key::ArrowUp,
                    egui::Key::ArrowDown,
                ]
                .into_iter()
                .any(|key| input.key_pressed(key));
                pointer_started || edit_key_started
            })
            .then(|| self.project.clone());

        let mut ruler_alias_to_edit: Option<(String, String)> = None;
        let is_modular =
            self.config.layout.modular_workspace && !self.piano_roll_state.is_maximized;
        let window_title = self.config.language.tr("Piano Roll", "Piano Roll");
        let bg_canvas = self.config.theme.bg_canvas_c32();
        let pending_snap = match self.workspace_snap_rect.take() {
            Some((crate::gui::workspace::WorkspacePane::PianoRoll, rect)) if is_modular => {
                Some(rect)
            }
            other => {
                self.workspace_snap_rect = other;
                None
            }
        };
        let mut snap_after_drag = None;

        let mut draw_piano_roll = |ui: &mut egui::Ui| {
            let mut preview_freq: Option<f64> = None;
            let mut before_changed = false;
            let mut note_changed = false;
            let mut scrubbed_t: Option<f64> = None;

            let active_track = self.active_track_index;
            if self.project.parts.is_empty() {
                self.project
                    .parts
                    .push(crate::project::model::UVoicePart::new("Part 1", 0));
            }
            let part_idx = self
                .project
                .parts
                .iter()
                .position(|p| p.track_index == active_track)
                .unwrap_or(0);

            let active_notes = &mut self.project.parts[part_idx].notes;

            // Sync loop state to piano_roll_state
            self.piano_roll_state.loop_enabled = self.transport_state.loop_enabled;
            self.piano_roll_state.loop_start_ms = self.transport_state.loop_start_ms;
            self.piano_roll_state.loop_end_ms = self.transport_state.loop_end_ms;

            draw_piano_roll(
                ui,
                active_notes,
                &mut self.piano_roll_state,
                &self.config.theme,
                self.voicebank.as_ref(),
                &mut self.phoneme_palette_state,
                self.transport_state.grid_snap,
                self.transport_state.bpm,
                self.vocal_mode_params.phonemizer_mode,
                self.config.language,
                &mut |freq| preview_freq = Some(freq),
                &mut || before_changed = true,
                &mut || note_changed = true,
                &mut |t| scrubbed_t = Some(t),
                &mut |alias, pitch| {
                    ruler_alias_to_edit = Some((alias.to_string(), pitch.to_string()));
                },
            );

            // Sync loop state back to transport_state if edited in piano roll
            self.transport_state.loop_enabled = self.piano_roll_state.loop_enabled;
            self.transport_state.loop_start_ms = self.piano_roll_state.loop_start_ms;
            self.transport_state.loop_end_ms = self.piano_roll_state.loop_end_ms;

            if let Some(t) = scrubbed_t {
                if self.piano_roll_state.is_playing {
                    self.stop_audio();
                }
                self.piano_roll_state.playhead_ms = t;
                self.playback_start_offset_ms = t;
            }

            if let Some(freq) = preview_freq {
                self.preview_tone(freq);
            }

            if before_changed {
                if let Some(snapshot) = piano_edit_snapshot.clone() {
                    // Um novo gesto sempre substitui qualquer snapshot de um
                    // clique anterior que não chegou a modificar o projeto.
                    self.pending_edit_snapshot = Some(snapshot);
                }
            }

            if note_changed {
                if let Some(snapshot) = self.pending_edit_snapshot.take() {
                    self.undo_manager.push_state(snapshot);
                } else if let Some(snapshot) = piano_edit_snapshot.clone() {
                    // Operações instantâneas (por exemplo, apagar um ponto)
                    // podem confirmar no mesmo frame sem callback inicial.
                    self.undo_manager.push_state(snapshot);
                }
            }

            if note_changed || self.piano_roll_state.continuous_edit_dirty {
                self.ensure_default_portamento();
                if !self.config.workflow.allow_overlapping_notes {
                    self.resolve_note_overlaps();
                }
                self.piano_roll_state.continuous_edit_dirty = false;
                self.is_dirty = true;
            }

            if self.piano_roll_state.request_undo {
                self.piano_roll_state.request_undo = false;
                self.pending_edit_snapshot = None;
                if let Some(prev) = self.undo_manager.undo(self.project.clone()) {
                    self.project = prev;
                    self.piano_roll_state.phoneme_cache.clear();
                    self.piano_roll_state.note_phonemes_cache.clear();
                    self.piano_roll_state.continuous_edit_dirty = false;
                    self.is_dirty = true;
                    self.transport_state.status_message = "Desfeito (Undo)".to_string();
                }
            }
            if self.piano_roll_state.request_redo {
                self.piano_roll_state.request_redo = false;
                self.pending_edit_snapshot = None;
                if let Some(next) = self.undo_manager.redo(self.project.clone()) {
                    self.project = next;
                    self.piano_roll_state.phoneme_cache.clear();
                    self.piano_roll_state.note_phonemes_cache.clear();
                    self.piano_roll_state.continuous_edit_dirty = false;
                    self.is_dirty = true;
                    self.transport_state.status_message = "Refeito (Redo)".to_string();
                }
            }
            if self.piano_roll_state.request_cut {
                self.piano_roll_state.request_cut = false;
                self.cut_selected_notes();
            }
            if self.piano_roll_state.request_copy {
                self.piano_roll_state.request_copy = false;
                self.copy_selected_notes();
            }
            if self.piano_roll_state.request_paste {
                self.piano_roll_state.request_paste = false;
                self.paste_notes();
            }
            if self.piano_roll_state.request_batch_lyrics {
                self.piano_roll_state.request_batch_lyrics = false;
                self.batch_lyrics_open = true;
            }
            if self.piano_roll_state.request_quantize_snap {
                self.piano_roll_state.request_quantize_snap = false;
                self.quantize_positions();
            }
            if self.piano_roll_state.request_quantize_durations {
                self.piano_roll_state.request_quantize_durations = false;
                self.quantize_durations();
            }
            if self.piano_roll_state.request_fix_overlaps {
                self.piano_roll_state.request_fix_overlaps = false;
                self.fix_overlapping_notes();
            }
            if self.piano_roll_state.request_legato {
                self.piano_roll_state.request_legato = false;
                self.legato_connect_notes();
            }
            if self.piano_roll_state.request_clean_pitch_suffixes {
                self.piano_roll_state.request_clean_pitch_suffixes = false;
                self.clean_pitch_suffixes_from_lyrics();
            }
            if self.piano_roll_state.request_rephonemize {
                self.piano_roll_state.request_rephonemize = false;
                self.rephonemize_all_notes();
            }
            if self.piano_roll_state.request_humanize {
                self.piano_roll_state.request_humanize = false;
                self.humanize_selection();
            }
            if self.piano_roll_state.request_invert_retrograde {
                self.piano_roll_state.request_invert_retrograde = false;
                self.invert_melody_retrograde();
            }
            if self.piano_roll_state.request_invert_intervals {
                self.piano_roll_state.request_invert_intervals = false;
                self.invert_melody_intervals();
            }
            if self.piano_roll_state.request_select_overlapping {
                self.piano_roll_state.request_select_overlapping = false;
                self.select_overlapping_notes();
            }
            if self.piano_roll_state.request_select_out_of_scale {
                self.piano_roll_state.request_select_out_of_scale = false;
                self.select_out_of_scale_notes();
            }
            if self.piano_roll_state.request_autopitch_all {
                self.piano_roll_state.request_autopitch_all = false;
                self.apply_autopitch_all();
            }
            if self.piano_roll_state.request_autopitch_window {
                self.piano_roll_state.request_autopitch_window = false;
                self.autopitch_window_open = true;
            }
            if self.piano_roll_state.request_export_selection_audio {
                self.piano_roll_state.request_export_selection_audio = false;
                self.export_selected_notes_audio();
            }
            if self.piano_roll_state.request_clear_selected_render_cache {
                self.piano_roll_state.request_clear_selected_render_cache = false;
                let was_previewing = self.audio_player.is_playing() || self.render_rx.is_some();
                self.pause_audio();
                crate::renderer::resampler_cache::clear_memory_cache();
                let _ = crate::renderer::resampler_cache::clear_disk_cache();
                self.piano_roll_state.phoneme_cache_hash = 0;
                self.piano_roll_state.phoneme_cache.clear();
                self.piano_roll_state.note_phonemes_cache.clear();
                self.piano_roll_state.rendered_waveform_peaks.clear();
                self.preview_waveform_cache_hash = 0;
                self.transport_state.status_message = self
                    .config
                    .language
                    .tr(
                        "Cache das notas selecionadas limpo; renderização renovada",
                        "Selected-note cache cleared; render refreshed",
                    )
                    .to_string();
                if was_previewing {
                    self.play_current_track();
                }
            }
        };

        if is_modular {
            let mut window = egui::Window::new(window_title)
                .id(egui::Id::new("workspace_piano_roll"))
                .default_pos((40.0, 370.0))
                .default_size((820.0, 560.0))
                .min_size((420.0, 260.0))
                .frame(Frame::none().fill(bg_canvas));
            if let Some(rect) = pending_snap {
                window = window.fixed_rect(rect);
            }
            if let Some(response) = window.show(ctx, &mut draw_piano_roll) {
                if response.response.drag_stopped() {
                    snap_after_drag = crate::gui::workspace::snap_target(
                        response.response.rect,
                        ctx.available_rect(),
                        20.0,
                    );
                }
            }
        } else {
            CentralPanel::default()
                .frame(Frame::none().fill(bg_canvas))
                .show(ctx, &mut draw_piano_roll);
        }
        drop(draw_piano_roll);
        if snap_after_drag.is_some() {
            self.workspace_snap_rect =
                snap_after_drag.map(|rect| (crate::gui::workspace::WorkspacePane::PianoRoll, rect));
        }

        #[cfg(not(target_os = "android"))]
        if let Some((alias, pitch)) = ruler_alias_to_edit {
            self.open_copaiba_for_alias(&alias, &pitch);
        }

        // Commit qualquer snapshot pendente de arraste assim que o usuário solta o botão do mouse
        let pointer_released = ctx.input(|i| i.pointer.any_released());
        if pointer_released {
            if let Some(snapshot) = self.pending_edit_snapshot.take() {
                self.undo_manager.push_state(snapshot);
            }
        }
    }
}
