use crate::gui::arrangement::draw_arrangement_view;
use crate::gui::toolbar::draw_unified_toolbar;
use crate::gui::unified_panel::draw_unified_panel;
use crate::gui::KamafeuStudioApp;
use crate::oto::Voicebank;
use crate::project::model::UNote;
use crate::renderer::TrackRenderer;
use eframe::egui;
use eframe::egui::Frame;
use eframe::egui::SidePanel;
use eframe::egui::TopBottomPanel;

impl KamafeuStudioApp {
    pub(super) fn update_editor_panels(&mut self, ctx: &egui::Context) {
        self.render_menu_bar(ctx);

        let is_modular = self.config.layout.modular_workspace;
        let toolbar_title = self.config.language.tr("Controles", "Controls");
        let toolbar_fill = self.config.theme.bg_panel_c32();
        let mut draw_toolbar = |ui: &mut egui::Ui| {
            let transport_active = self.audio_player.is_playing() || self.render_rx.is_some();
            let bpm_before = self.project.bpm;
            let mut play_clicked = false;
            let mut stop_clicked = false;
            let mut export_clicked = false;
            let mut quantize_clicked = false;
            let mut fix_overlaps_clicked = false;
            let mut env_acpt_clicked = false;
            let mut env_p2p3_clicked = false;
            let mut env_p1p4_clicked = false;
            let mut env_opt_clicked = false;
            let mut env_reset_clicked = false;

            let lang = self.config.language;
            draw_unified_toolbar(
                ui,
                &self.config.theme,
                lang,
                &mut self.transport_state,
                transport_active,
                &mut self.render_log_window_open,
                &mut self.piano_roll_state.active_tool,
                &mut self.piano_roll_state.pitch_sub_tool,
                &mut self.piano_roll_state.auto_scroll_mode,
                &mut self.piano_roll_state.active_scale,
                &mut self.piano_roll_state.scale_root_key,
                &mut self.piano_roll_state.px_per_ms,
                &mut self.piano_roll_state.row_height,
                &mut self.piano_roll_state.show_arrangement_view,
                &mut self.piano_roll_state.show_parameters_drawer,
                &mut self.piano_roll_state.show_phoneme_ruler,
                &mut self.piano_roll_state.show_inspector,
                &mut self.piano_roll_state.is_maximized,
                &mut || play_clicked = true,
                &mut || stop_clicked = true,
                &mut || export_clicked = true,
                &mut || self.autopitch_window_open = true,
                &mut || quantize_clicked = true,
                &mut || fix_overlaps_clicked = true,
                &mut || env_acpt_clicked = true,
                &mut || env_p2p3_clicked = true,
                &mut || env_p1p4_clicked = true,
                &mut || env_opt_clicked = true,
                &mut || env_reset_clicked = true,
            );

            if quantize_clicked {
                self.quantize_positions();
            }
            if fix_overlaps_clicked {
                self.fix_overlapping_notes();
            }
            if env_acpt_clicked {
                self.apply_envelope_acpt();
            }
            if env_p2p3_clicked {
                self.apply_envelope_p2p3();
            }
            if env_p1p4_clicked {
                self.apply_envelope_p1p4();
            }
            if env_opt_clicked {
                self.apply_envelope_opt();
            }
            if env_reset_clicked {
                self.apply_envelope_reset();
            }

            self.audio_player
                .set_volume(self.transport_state.master_volume);

            let requested_bpm = self.transport_state.bpm;
            if (requested_bpm - bpm_before).abs() > f64::EPSILON {
                let was_active = self.audio_player.is_playing() || self.render_rx.is_some();
                if was_active {
                    self.pause_audio();
                }
                if let Some(time_scale) = self.project.set_bpm_preserving_beats(requested_bpm) {
                    self.piano_roll_state.playhead_ms *= time_scale;
                    self.playback_start_offset_ms *= time_scale;
                    self.transport_state.bpm = self.project.bpm;
                    self.transport_state.status_message = if was_active {
                        if lang.is_en() {
                            format!("BPM changed to {:.0}; playback stopped", self.project.bpm)
                        } else {
                            format!(
                                "BPM alterado para {:.0}; reprodução interrompida",
                                self.project.bpm
                            )
                        }
                    } else {
                        if lang.is_en() {
                            format!("BPM changed to {:.0}", self.project.bpm)
                        } else {
                            format!("BPM alterado para {:.0}", self.project.bpm)
                        }
                    };
                } else {
                    self.transport_state.bpm = self.project.bpm;
                    self.transport_state.status_message =
                        lang.tr("BPM inválido", "Invalid BPM").to_string();
                }
            }

            if play_clicked {
                if transport_active {
                    self.pause_audio();
                } else {
                    self.play_current_track();
                }
            }

            if stop_clicked {
                self.stop_audio();
            }

            if export_clicked {
                self.export_wav();
            }
        };
        if is_modular {
            egui::Window::new(toolbar_title)
                .id(egui::Id::new("workspace_toolbar"))
                .default_pos((40.0, 28.0))
                .default_size((820.0, 74.0))
                .min_size((360.0, 48.0))
                .frame(Frame::none().fill(toolbar_fill))
                .show(ctx, &mut draw_toolbar);
        } else {
            TopBottomPanel::top("top_unified_control_panel")
                .exact_height(36.0)
                .frame(Frame::none().fill(toolbar_fill))
                .show(ctx, &mut draw_toolbar);
        }

        let vocal_mode_params_before = self.vocal_mode_params.clone();
        if self.piano_roll_state.show_inspector && !self.piano_roll_state.is_maximized {
            let is_modular = self.config.layout.modular_workspace;
            let inspector_title = self.config.language.tr("Inspetor", "Inspector");
            let bg_panel = self.config.theme.bg_panel_c32();
            let inspector_snap = match self.workspace_snap_rect.take() {
                Some((crate::gui::workspace::WorkspacePane::Inspector, rect)) if is_modular => {
                    Some(rect)
                }
                other => {
                    self.workspace_snap_rect = other;
                    None
                }
            };
            let mut inspector_snap_after = None;

            let mut draw_inspector = |ui: &mut egui::Ui| {
                let mut color_changed = false;
                if let Some(vb) = self.voicebank.as_mut() {
                    let colors: Vec<String> = vb.prefix_map.colors().map(str::to_string).collect();
                    if colors.len() > 1 {
                        let mut selected = vb.prefix_map.selected_color().to_string();
                        ui.label(self.config.language.tr("Timbre padrão", "Default timbre"));
                        egui::ComboBox::from_id_salt("voicecolor_selector")
                            .selected_text(if selected.is_empty() {
                                self.config.language.tr("Neutro", "Neutral")
                            } else {
                                &selected
                            })
                            .show_ui(ui, |ui| {
                                for color in colors {
                                    let label = if color.is_empty() { "Default" } else { &color };
                                    color_changed |= ui
                                        .selectable_value(&mut selected, color.clone(), label)
                                        .changed();
                                }
                            });
                        if color_changed {
                            vb.prefix_map.select_color(&selected);
                        }
                    }
                }
                if color_changed {
                    self.pause_audio();
                    self.piano_roll_state.phoneme_cache_hash = 0;
                }
                let selected_indices = self.piano_roll_state.selected_note_indices.clone();
                // Marquee/multi-selection may populate the set without
                // assigning a primary index. Use a stable selected note
                // as the inspector target so the panel never appears
                // empty while notes are visibly highlighted.
                let selected_idx = self.piano_roll_state.selected_note_index.or_else(|| {
                    self.piano_roll_state
                        .selected_note_indices
                        .iter()
                        .copied()
                        .min()
                });
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

                let mut loaded_vb: Option<Voicebank> = None;
                let mut preview_alias: Option<String> = None;
                let mut insert_alias: Option<String> = None;
                let mut palette_edit_alias: Option<String> = None;
                let mut selected_ruler_alias_to_edit: Option<(String, String)> = None;
                let selected_ruler_alias = self.piano_roll_state.selected_copaiba_alias.clone();
                // The inspector is rendered every frame. A full project clone is
                // only useful while an edit gesture can begin; cloning it while
                // merely inspecting a note made large projects allocate at the
                // display refresh rate.
                let project_snapshot_before_panel = ui
                    .input(|i| i.pointer.primary_down() || i.pointer.secondary_down())
                    .then(|| self.project.clone());

                let notes = &mut self.project.parts[part_idx].notes[..];

                let mut open_singers_gallery = false;
                let mut reload_singers_flag = false;
                let mut add_singers_dir_flag = false;
                let mut open_folder_picker = false;

                draw_unified_panel(
                    ui,
                    &self.config.theme,
                    self.config.language,
                    self.voicebank.as_ref(),
                    &self.config.recent_voicebanks,
                    &self.singers_list,
                    &mut self.singer_search_query,
                    &mut self.config.singers_paths,
                    &mut self.vocal_mode_params,
                    selected_idx,
                    notes,
                    &selected_indices,
                    selected_ruler_alias
                        .as_ref()
                        .map(|(alias, _)| alias.as_str()),
                    &mut self.right_sidebar_tab,
                    &mut self.phoneme_palette_state,
                    &mut self.render_threads,
                    &mut self.sample_rate,
                    &mut self.selected_resampler,
                    &mut self.selected_wavtool,
                    &mut self.custom_resampler_path,
                    &mut self.custom_wavtool_path,
                    &mut self.config.discord_rpc_enabled,
                    &mut |opt_path| {
                        if let Some(p) = opt_path {
                            if let Ok(vb) = Voicebank::new(&p) {
                                loaded_vb = Some(vb);
                            }
                        } else {
                            #[cfg(not(target_os = "android"))]
                            {
                                if let Some(folder) =
                                    crate::dialogs::FileDialog::new().pick_folder()
                                {
                                    if let Ok(vb) = Voicebank::new(&folder) {
                                        loaded_vb = Some(vb);
                                    }
                                } else {
                                    open_folder_picker = true;
                                }
                            }
                            #[cfg(target_os = "android")]
                            {
                                open_folder_picker = true;
                            }
                        }
                    },
                    &mut || add_singers_dir_flag = true,
                    &mut || reload_singers_flag = true,
                    &mut || open_singers_gallery = true,
                    &mut |alias| preview_alias = Some(alias.to_string()),
                    &mut |alias| insert_alias = Some(alias.to_string()),
                    &mut |alias| palette_edit_alias = Some(alias.to_string()),
                    &mut || selected_ruler_alias_to_edit = selected_ruler_alias.clone(),
                );

                if project_snapshot_before_panel
                    .as_ref()
                    .is_some_and(|snapshot| self.project != *snapshot)
                {
                    let pointer_down =
                        ui.input(|i| i.pointer.primary_down() || i.pointer.secondary_down());
                    if pointer_down {
                        if self.pending_edit_snapshot.is_none() {
                            self.pending_edit_snapshot = project_snapshot_before_panel.clone();
                        }
                    } else {
                        let snapshot = self
                            .pending_edit_snapshot
                            .take()
                            .or_else(|| project_snapshot_before_panel.clone())
                            .expect("inspector edits always retain their gesture snapshot");
                        self.undo_manager.push_state(snapshot);
                        self.piano_roll_state.phoneme_cache.clear();
                        self.piano_roll_state.note_phonemes_cache.clear();
                    }
                    self.is_dirty = true;
                }

                if open_folder_picker {
                    self.folder_picker_open = true;
                }

                if add_singers_dir_flag {
                    #[cfg(not(target_os = "android"))]
                    if let Some(folder) = crate::dialogs::FileDialog::new().pick_folder() {
                        if !self.config.singers_paths.contains(&folder) {
                            self.config.singers_paths.push(folder);
                            self.persist_config();
                            self.reload_singers();
                        }
                    } else {
                        self.folder_picker_open = true;
                    }
                    #[cfg(target_os = "android")]
                    {
                        self.folder_picker_open = true;
                    }
                }

                if reload_singers_flag {
                    self.persist_config();
                    self.reload_singers();
                }

                if open_singers_gallery {
                    self.singers_gallery_window_open = true;
                }

                self.persist_config();

                if let Some(vb) = loaded_vb {
                    self.activate_voicebank(vb);
                }

                if let Some(alias) = preview_alias {
                    let mut played = false;
                    if let Some(ref vb) = self.voicebank {
                        if let Some(entry) = vb
                            .find_entry(&alias, "C4")
                            .or_else(|| vb.find_entry(&alias, "A3"))
                        {
                            let wav_path = vb.root_path.join(&entry.wav_filename);
                            if let Ok((samples, sr)) = TrackRenderer::load_wav_samples(&wav_path) {
                                let max_s = (sr as usize).min(samples.len());
                                self.audio_player
                                    .play_samples(samples[..max_s].to_vec(), sr);
                                played = true;
                            }
                        }
                    }
                    if !played {
                        self.preview_tone(440.0);
                    }
                }

                if let Some(alias) = insert_alias {
                    self.push_history();
                    let playhead_ms = self.piano_roll_state.playhead_ms;
                    let sel_idx = self.piano_roll_state.selected_note_index;
                    let notes_mut = self.current_notes_mut();
                    if let Some(idx) = sel_idx {
                        if idx < notes_mut.len() {
                            notes_mut[idx].lyric = alias;
                        }
                    } else {
                        let new_note = UNote::new(&alias, "C4", playhead_ms, 400.0);
                        notes_mut.push(new_note);
                    }
                }

                #[cfg(not(target_os = "android"))]
                let edit_alias = selected_ruler_alias_to_edit
                    .or_else(|| palette_edit_alias.map(|alias| (alias, "C4".to_string())));
                if let Some((alias, pitch)) = edit_alias {
                    self.open_copaiba_for_alias(&alias, &pitch);
                }
            };

            if is_modular {
                let mut window = egui::Window::new(inspector_title)
                    .id(egui::Id::new("workspace_inspector"))
                    .default_pos((900.0, 110.0))
                    .default_size((300.0, 620.0))
                    .min_width(200.0)
                    .frame(Frame::none().fill(bg_panel));
                if let Some(rect) = inspector_snap {
                    window = window.fixed_rect(rect);
                }
                if let Some(response) = window.show(ctx, &mut draw_inspector) {
                    if response.response.drag_stopped() {
                        inspector_snap_after = crate::gui::workspace::snap_target(
                            response.response.rect,
                            ctx.available_rect(),
                            20.0,
                        );
                    }
                }
            } else {
                SidePanel::right("right_inspector_panel")
                    .resizable(true)
                    .default_width(240.0)
                    .min_width(200.0)
                    .max_width(480.0)
                    .frame(Frame::none().fill(bg_panel))
                    .show(ctx, &mut draw_inspector);
            }
            drop(draw_inspector);
            if inspector_snap_after.is_some() {
                self.workspace_snap_rect = inspector_snap_after
                    .map(|rect| (crate::gui::workspace::WorkspacePane::Inspector, rect));
            }
        }

        if self.vocal_mode_params != vocal_mode_params_before
            && (self.audio_player.is_playing() || self.render_rx.is_some())
        {
            let lang = self.config.language;
            self.pause_audio();
            self.transport_state.status_message = lang
                .tr(
                    "Predefinição alterada; prévia pronta para renderizar novamente",
                    "Preset changed; preview ready to re-render",
                )
                .to_string();
        }

        if self.piano_roll_state.show_arrangement_view && !self.piano_roll_state.is_maximized {
            // Cloning the complete project every frame made the arrangement
            // panel increasingly expensive for larger projects. A snapshot
            // is only needed when an edit gesture can actually begin.
            let arrangement_snapshot_before = if self.pending_edit_snapshot.is_none()
                && ctx.input(|i| i.pointer.primary_down() || i.pointer.secondary_down())
            {
                Some(self.project.clone())
            } else {
                None
            };
            let is_modular = self.config.layout.modular_workspace;
            let arrangement_title = self
                .config
                .language
                .tr("Arranjo e faixas", "Arrangement & Tracks");
            let bg_panel = self.config.theme.bg_panel_c32();
            let border_stroke = self.config.theme.active_border_stroke();
            let default_arr_h = self.piano_roll_state.arrangement_height;

            let mut draw_arrangement = |ui: &mut egui::Ui| {
                let actual_h = ui.max_rect().height().clamp(60.0, 500.0);
                self.piano_roll_state.arrangement_height = actual_h;

                let arrangement_changed = draw_arrangement_view(
                    ui,
                    &self.config.theme,
                    &mut self.project.tracks,
                    &mut self.project.parts,
                    &mut self.project.wave_parts,
                    &mut self.active_track_index,
                    &mut self.piano_roll_state.playhead_ms,
                    self.piano_roll_state.px_per_ms,
                    self.transport_state.bpm,
                    &mut self.piano_roll_state.horizontal_scroll_offset,
                    &mut self.fx_rack_dialog_state,
                    self.config.language,
                );
                if arrangement_changed {
                    let pointer_down =
                        ui.input(|i| i.pointer.primary_down() || i.pointer.secondary_down());
                    if pointer_down {
                        if self.pending_edit_snapshot.is_none() {
                            if let Some(snapshot) = arrangement_snapshot_before.as_ref() {
                                self.pending_edit_snapshot = Some(snapshot.clone());
                            }
                        }
                    } else {
                        if let Some(snapshot) = self.pending_edit_snapshot.take() {
                            self.undo_manager.push_state(snapshot);
                        } else if let Some(snapshot) = arrangement_snapshot_before.as_ref() {
                            self.undo_manager.push_state(snapshot.clone());
                        }
                    }
                    self.is_dirty = true;
                }
            };
            if is_modular {
                egui::Window::new(arrangement_title)
                    .id(egui::Id::new("workspace_arrangement"))
                    .default_pos((40.0, 90.0))
                    .default_size((820.0, 260.0))
                    .min_size((360.0, 120.0))
                    .frame(Frame::none().fill(bg_panel).stroke(border_stroke))
                    .show(ctx, &mut draw_arrangement);
            } else {
                TopBottomPanel::top("arrangement_multitrack_panel")
                    .resizable(true)
                    .height_range(60.0..=500.0)
                    .default_height(default_arr_h)
                    .frame(Frame::none().fill(bg_panel).stroke(border_stroke))
                    .show(ctx, &mut draw_arrangement);
            }
        }
    }
}
