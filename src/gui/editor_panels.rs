use crate::gui::arrangement::draw_arrangement_view;
use crate::gui::toolbar::draw_unified_toolbar;
use crate::gui::unified_panel::draw_unified_panel;
use crate::gui::KamafeuStudioApp;
use crate::project::model::UNote;
use crate::renderer::{PreviewTarget, ProjectRenderer, TrackRenderer};
use eframe::egui;
use eframe::egui::Frame;
use eframe::egui::SidePanel;
use eframe::egui::TopBottomPanel;

impl KamafeuStudioApp {
    fn draw_project_harmony(&self, ui: &mut egui::Ui) {
        let names = [
            "C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B",
        ];
        let mut pitch_classes = std::collections::BTreeSet::new();
        let mut note_count = 0usize;
        for part in &self.project.parts {
            for note in &part.notes {
                if let Ok(midi) = note.pitch.parse::<u8>() {
                    pitch_classes.insert(midi % 12);
                    note_count += 1;
                } else if let Some(midi) = crate::dsp::pitch::note_name_to_midi(&note.pitch) {
                    pitch_classes.insert(midi as u8 % 12);
                    note_count += 1;
                }
            }
        }
        ui.label(
            egui::RichText::new(
                self.config
                    .language
                    .tr("Leitura das notas do projeto", "Project note analysis"),
            )
            .strong(),
        );
        if note_count == 0 {
            ui.label(
                self.config
                    .language
                    .tr("Sem notas analisáveis", "No analyzable notes"),
            );
            return;
        }
        ui.label(format!(
            "{}: {}",
            self.config
                .language
                .tr("Classes de altura", "Pitch classes"),
            pitch_classes
                .iter()
                .map(|pc| names[*pc as usize])
                .collect::<Vec<_>>()
                .join(" · ")
        ));
        ui.label(format!(
            "{note_count} {} · {}",
            self.config
                .language
                .tr("notas no projeto", "notes in project"),
            self.config.language.tr(
                "Resumo harmônico não altera o projeto",
                "Harmony summary does not modify the project"
            )
        ));
    }

    pub(super) fn update_editor_panels(&mut self, ctx: &egui::Context) {
        self.audio_player
            .set_master_fx(&self.project.master_fx.clone().unwrap_or_default());
        self.render_menu_bar(ctx);

        let is_modular = self.config.layout.modular_workspace;
        let toolbar_title = self.config.language.tr("Controles", "Controls");
        let toolbar_fill = self.config.theme.elevated_surface_c32();
        let toolbar_stroke = self.config.theme.card_stroke();
        self.transport_state.motion = self.animation_policy().allows_motion();
        self.transport_state.busy = self.render_rx.is_some() || self.export_rx.is_some();
        let mut draw_toolbar = |ui: &mut egui::Ui| {
            let transport_active = self.transport_is_active();
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
            let follow_before = (
                self.piano_roll_state.vertical_follow_smoothing_ms,
                self.piano_roll_state.horizontal_follow_smoothing_ms,
                self.piano_roll_state.follow_cursor_pct,
                self.piano_roll_state.follow_min_pct,
                self.piano_roll_state.follow_max_pct,
            );

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
                &mut self.piano_roll_state.pitch_smooth_intensity,
                &mut self.piano_roll_state.auto_scroll_mode,
                &mut self.piano_roll_state.vertical_pitch_follow,
                &mut self.piano_roll_state.vertical_follow_smoothing_ms,
                &mut self.piano_roll_state.horizontal_follow_smoothing_ms,
                &mut self.piano_roll_state.follow_cursor_pct,
                &mut self.piano_roll_state.follow_min_pct,
                &mut self.piano_roll_state.follow_max_pct,
                &mut self.piano_roll_state.active_scale,
                &mut self.piano_roll_state.scale_root_key,
                &mut self.piano_roll_state.px_per_ms,
                &mut self.piano_roll_state.row_height,
                &mut self.piano_roll_state.show_arrangement_view,
                &mut self.piano_roll_state.show_parameters_drawer,
                &mut self.piano_roll_state.show_phoneme_ruler,
                &mut self.piano_roll_state.show_envelope_handles,
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
            let follow_after = (
                self.piano_roll_state.vertical_follow_smoothing_ms,
                self.piano_roll_state.horizontal_follow_smoothing_ms,
                self.piano_roll_state.follow_cursor_pct,
                self.piano_roll_state.follow_min_pct,
                self.piano_roll_state.follow_max_pct,
            );
            if follow_after != follow_before {
                self.config.layout.vertical_follow_smoothing_ms = follow_after.0;
                self.config.layout.horizontal_follow_smoothing_ms = follow_after.1;
                self.config.layout.follow_cursor_pct = follow_after.2;
                self.config.layout.follow_min_pct = follow_after.3;
                self.config.layout.follow_max_pct = follow_after.4;
                self.persist_config();
            }

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
                let was_active = self.transport_is_active();
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
        // Portrait tablets and phones use the touch-first navigation instead of
        // squeezing desktop side panels into an unusable column.
        let is_mobile = super::touch_first_layout(ctx);

        if is_mobile {
            TopBottomPanel::top("top_unified_control_panel")
                .exact_height(52.0)
                .frame(Frame::none().fill(toolbar_fill).stroke(toolbar_stroke))
                .show(ctx, |ui| {
                    egui::ScrollArea::horizontal()
                        .id_salt("mobile_toolbar_scroll")
                        .auto_shrink([false, true])
                        .show(ui, |ui| draw_toolbar(ui));
                });

            self.render_mobile_bottom_bar(ctx);
            return;
        }

        if is_modular {
            egui::Window::new(toolbar_title)
                .id(egui::Id::new("workspace_toolbar"))
                .default_pos((40.0, 28.0))
                .default_size((820.0, 74.0))
                .min_size((360.0, 48.0))
                .frame(Frame::none().fill(toolbar_fill).stroke(toolbar_stroke))
                .show(ctx, &mut draw_toolbar);
        } else {
            TopBottomPanel::top("top_unified_control_panel")
                .exact_height(32.0)
                .frame(Frame::none().fill(toolbar_fill).stroke(toolbar_stroke))
                .show(ctx, &mut draw_toolbar);
        }

        let vocal_mode_params_before = self.vocal_mode_params.clone();
        if self.piano_roll_state.show_inspector && !self.piano_roll_state.is_maximized {
            let is_modular = self.config.layout.modular_workspace;
            let inspector_title = self.config.language.tr("Inspetor", "Inspector");
            let bg_panel = self.config.theme.bg_panel_c32();
            let inspector_rounding = self.config.theme.ui_rounding();
            let inspector_stroke = self.config.theme.card_stroke();
            let inspector_default_width = self.config.layout.right_dock_width;
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
            let mut inspector_resized_width = None;

            let mut draw_inspector = |ui: &mut egui::Ui| {
                ui.horizontal(|ui| {
                    if ui
                        .selectable_label(
                            self.config.layout.right_dock_tab == 0,
                            self.config.language.tr("Inspetor", "Inspector"),
                        )
                        .clicked()
                    {
                        self.config.layout.right_dock_tab = 0;
                    }
                    if ui
                        .selectable_label(self.config.layout.right_dock_tab == 1, "FX")
                        .clicked()
                    {
                        self.config.layout.right_dock_tab = 1;
                    }
                });
                ui.separator();
                if self.config.layout.right_dock_tab == 0 {
                    self.draw_inspector_inner(ui);
                } else {
                    let mut close = false;
                    let target = self.active_track_index;
                    let rack_state = &mut self.fx_rack_dialog_state;
                    rack_state.target_track = Some(target);
                    let mut track_fx = self
                        .project
                        .tracks
                        .get(target)
                        .and_then(|track| track.fx_rack.clone())
                        .unwrap_or_default();
                    let original_fx = track_fx.clone();
                    crate::gui::fx_rack_ui::render_fx_rack_ui(
                        ui,
                        self.config.language,
                        &self.config.theme,
                        self.config.language.tr("Efeitos da faixa", "Track effects"),
                        rack_state,
                        &self.project.tracks,
                        &mut track_fx,
                        &mut close,
                    );
                    if track_fx != original_fx {
                        if let Some(track) = self.project.tracks.get_mut(target) {
                            track.fx_rack = Some(track_fx);
                        }
                        self.is_dirty = true;
                        self.fx_preview_refresh_pending = true;
                    }
                }
            };

            if is_modular {
                let mut window = egui::Window::new(inspector_title)
                    .id(egui::Id::new("workspace_inspector"))
                    .default_pos((900.0, 110.0))
                    .default_size((260.0, 560.0))
                    .min_width(180.0)
                    .frame(
                        Frame::none()
                            .fill(bg_panel)
                            .rounding(inspector_rounding)
                            .stroke(inspector_stroke),
                    );
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
                let response = SidePanel::right("right_inspector_panel")
                    .resizable(true)
                    .default_width(inspector_default_width)
                    .min_width(180.0)
                    .max_width(300.0)
                    .frame(Frame::none().fill(bg_panel).stroke(inspector_stroke))
                    .show(ctx, &mut draw_inspector);
                if response.response.drag_stopped() {
                    inspector_resized_width = Some(response.response.rect.width());
                }
            }
            drop(draw_inspector);
            if let Some(width) = inspector_resized_width {
                self.config.layout.right_dock_width = width;
                self.persist_config();
            }
            if inspector_snap_after.is_some() {
                self.workspace_snap_rect = inspector_snap_after
                    .map(|rect| (crate::gui::workspace::WorkspacePane::Inspector, rect));
            }
        }

        if self.vocal_mode_params != vocal_mode_params_before && self.transport_is_active() {
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
            let is_modular = self.config.layout.modular_workspace;
            let arrangement_title = self
                .config
                .language
                .tr("Arranjo e faixas", "Arrangement & Tracks");
            let bg_panel = self.config.theme.bg_panel_c32();
            let border_stroke = self.config.theme.active_border_stroke();
            let arrangement_default_width = self.config.layout.left_dock_width;

            let mut draw_arrangement = |ui: &mut egui::Ui| {
                self.draw_arrangement_inner(ui);
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
                let response = SidePanel::left("arrangement_multitrack_panel")
                    .resizable(true)
                    .default_width(arrangement_default_width)
                    .min_width(280.0)
                    .max_width(600.0)
                    .frame(Frame::none().fill(bg_panel).stroke(border_stroke))
                    .show(ctx, &mut draw_arrangement);
                drop(draw_arrangement);
                if response.response.drag_stopped() {
                    self.config.layout.left_dock_width = response.response.rect.width();
                    self.persist_config();
                }
            }
        }

        if self.config.layout.show_bottom_dock {
            let fill = self.config.theme.bg_panel_c32();
            let response = TopBottomPanel::bottom("editor_analysis_dock")
                .resizable(true)
                .default_height(self.config.layout.bottom_dock_height)
                .height_range(100.0..=360.0)
                .frame(
                    Frame::none()
                        .fill(fill)
                        .stroke(self.config.theme.card_stroke()),
                )
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui
                            .selectable_label(
                                self.config.layout.bottom_dock_tab == 0,
                                self.config.language.tr("Mixer", "Mixer"),
                            )
                            .clicked()
                        {
                            self.config.layout.bottom_dock_tab = 0;
                        }
                        if ui
                            .selectable_label(
                                self.config.layout.bottom_dock_tab == 1,
                                self.config.language.tr("Harmonia", "Harmony"),
                            )
                            .clicked()
                        {
                            self.config.layout.bottom_dock_tab = 1;
                        }
                        if ui
                            .small_button("×")
                            .on_hover_text(
                                self.config.language.tr("Recolher painel", "Collapse panel"),
                            )
                            .clicked()
                        {
                            self.config.layout.show_bottom_dock = false;
                            self.persist_config();
                        }
                    });
                    ui.separator();
                    match self.config.layout.bottom_dock_tab {
                        0 => {
                            egui::ScrollArea::vertical()
                                .id_salt("mixer_track_cascade")
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    let mut mix_changed = false;
                                    for (index, track) in self.project.tracks.iter_mut().enumerate()
                                    {
                                        ui.group(|ui| {
                                            ui.set_width(ui.available_width());
                                            ui.horizontal(|ui| {
                                                ui.label(egui::RichText::new(&track.name).strong());
                                                ui.separator();
                                                mix_changed |= ui
                                                    .checkbox(
                                                        &mut track.mute,
                                                        self.config.language.tr("Mudo", "Mute"),
                                                    )
                                                    .changed();
                                                mix_changed |=
                                                    ui.checkbox(&mut track.solo, "Solo").changed();
                                                if ui.small_button("FX").clicked() {
                                                    self.active_track_index = index;
                                                    self.config.layout.right_dock_tab = 1;
                                                }
                                            });
                                            let slider_width = ui.available_width();
                                            mix_changed |= ui
                                                .add_sized(
                                                    [slider_width, 22.0],
                                                    egui::Slider::new(
                                                        &mut track.volume_db,
                                                        -60.0..=12.0,
                                                    )
                                                    .text(
                                                        self.config
                                                            .language
                                                            .tr("Volume (dB)", "Volume (dB)"),
                                                    ),
                                                )
                                                .changed();
                                            mix_changed |= ui
                                                .add_sized(
                                                    [slider_width, 22.0],
                                                    egui::Slider::new(&mut track.pan, -1.0..=1.0)
                                                        .text("Pan"),
                                                )
                                                .changed();
                                            // Track VU meter
                                            let track_meter = self.piano_roll_state.arrangement_track_meters.get(index);
                                            let level = track_meter.map(|m| m.level).unwrap_or(0.0);
                                            let peak = track_meter.map(|m| m.peak).unwrap_or(0.0);
                                            let (meter_rect, _) = ui.allocate_exact_size(egui::Vec2::new(slider_width, 6.0), egui::Sense::hover());
                                            let painter = ui.painter_at(meter_rect);
                                            let palette = crate::gui::visual_fx::MeterPalette {
                                                low: self.config.theme.meter_c32(),
                                                mid: self.config.theme.warning_c32(),
                                                high: self.config.theme.danger_c32(),
                                                off: self.config.theme.c32_alpha(self.config.theme.grid_line_sub, 0.22),
                                            };
                                            crate::gui::visual_fx::paint_led_meter(&painter, meter_rect, level, peak, &palette);
                                        });
                                    }
                                    if mix_changed {
                                        self.is_dirty = true;
                                        self.fx_preview_refresh_pending = true;
                                    }
                                    ui.group(|ui| {
                                        ui.set_width(ui.available_width());
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new("Master").strong());
                                            ui.separator();
                                            // Live Scope preview in master mixer channel
                                            let (scope_rect, _) = ui.allocate_exact_size(egui::Vec2::new(120.0, 20.0), egui::Sense::hover());
                                            let painter = ui.painter_at(scope_rect);
                                            crate::gui::visual_fx::paint_vertical_gradient(
                                                &painter,
                                                scope_rect,
                                                self.config.theme.c32_alpha(self.config.theme.bg_canvas, 0.95),
                                                self.config.theme.c32_alpha(self.config.theme.bg_header, 0.95),
                                            );
                                            crate::gui::visual_fx::paint_scope(
                                                &painter,
                                                scope_rect.shrink(1.0),
                                                &self.transport_state.scope,
                                                self.config.theme.waveform_c32(),
                                                1.5,
                                            );
                                            painter.rect_stroke(
                                                scope_rect,
                                                self.config.theme.ui_rounding(),
                                                egui::Stroke::new(1.0_f32, self.config.theme.c32_alpha(self.config.theme.grid_line_sub, 0.6)),
                                            );
                                        });
                                        ui.add_sized(
                                            [ui.available_width(), 22.0],
                                            egui::Slider::new(
                                                &mut self.transport_state.master_volume,
                                                0.0..=2.0,
                                            )
                                            .text(self.config.language.tr("Volume", "Volume")),
                                        );
                                        // Master Stereo LED Meter
                                        let (meter_rect_l, _) = ui.allocate_exact_size(egui::Vec2::new(ui.available_width(), 5.0), egui::Sense::hover());
                                        let painter_l = ui.painter_at(meter_rect_l);
                                        let (meter_rect_r, _) = ui.allocate_exact_size(egui::Vec2::new(ui.available_width(), 5.0), egui::Sense::hover());
                                        let painter_r = ui.painter_at(meter_rect_r);
                                        let palette = crate::gui::visual_fx::MeterPalette {
                                            low: self.config.theme.meter_c32(),
                                            mid: self.config.theme.warning_c32(),
                                            high: self.config.theme.danger_c32(),
                                            off: self.config.theme.c32_alpha(self.config.theme.grid_line_sub, 0.22),
                                        };
                                        crate::gui::visual_fx::paint_led_meter(&painter_l, meter_rect_l, self.transport_state.vu_level_l, self.transport_state.vu_peak_l, &palette);
                                        crate::gui::visual_fx::paint_led_meter(&painter_r, meter_rect_r, self.transport_state.vu_level_r, self.transport_state.vu_peak_r, &palette);
                                    });
                                });
                        }
                        _ => self.draw_project_harmony(ui),
                    }
                });
            if response.response.drag_stopped() {
                self.config.layout.bottom_dock_height = response.response.rect.height();
                self.persist_config();
            }
        }

        if self.fx_preview_refresh_pending && !ctx.input(|input| input.pointer.primary_down()) {
            self.fx_preview_refresh_pending = false;
            if self.transport_is_active() {
                self.pause_audio();
                self.play_current_track();
            }
        }
    }

    pub(crate) fn draw_inspector_inner(&mut self, ui: &mut egui::Ui) {
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

        let mut preview_alias: Option<String> = None;
        let mut insert_alias: Option<String> = None;
        let mut palette_edit_alias: Option<String> = None;
        let mut selected_ruler_alias_to_edit: Option<(String, String)> = None;
        let selected_ruler_alias = self.piano_roll_state.selected_copaiba_alias.clone();
        let active_native_singer = self.project.tracks.get(self.active_track_index)
            .and_then(|track| track.native_voicebank.as_ref()
                .and_then(|reference| reference.source_path_hint.clone())
                .or_else(|| track.voicebank_path.clone()))
            .and_then(|path| {
                let p = std::path::PathBuf::from(&path);
                self.singers_list
                    .iter()
                    .find(|singer| singer.path == p || std::fs::canonicalize(&singer.path).ok() == std::fs::canonicalize(&p).ok())
                    .cloned()
                    .or_else(|| crate::oto::SingerScanner::inspect_singer_directory(&p))
            });
        let pointer_down = ui.input(|i| i.pointer.primary_down() || i.pointer.secondary_down());
        let mut notes_to_watch = selected_indices.clone();
        if let Some(index) = selected_idx {
            notes_to_watch.insert(index);
        }
        let watched_note_snapshots: Vec<_> = self
            .project
            .parts
            .get(part_idx)
            .map(|part| {
                notes_to_watch
                    .iter()
                    .filter_map(|index| part.notes.get(*index).map(|note| (*index, note.clone())))
                    .collect()
            })
            .unwrap_or_default();

        let notes = &mut self.project.parts[part_idx].notes[..];

        let mut open_singers_gallery = false;
        let mut reload_singers_flag = false;
        let mut add_singers_dir_flag = false;
        let mut open_folder_picker = false;
        let mut singer_to_load: Option<std::path::PathBuf> = None;
        let phonemizer_before_panel = self.vocal_mode_params.phonemizer_mode;

        draw_unified_panel(
            ui,
            &self.config.theme,
            self.config.language,
            self.voicebank.as_ref(),
            active_native_singer.as_ref(),
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
            self.piano_roll_state.playhead_ms,
            self.piano_roll_state.is_playing,
            &mut |opt_path| {
                if let Some(p) = opt_path {
                    singer_to_load = Some(p);
                } else {
                    #[cfg(not(target_os = "android"))]
                    {
                        if let Some(folder) = crate::dialogs::FileDialog::new().pick_folder() {
                            singer_to_load = Some(folder);
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

        if let Some(path) = singer_to_load {
            self.activate_singer_path(&path);
        }

        let mut diffsinger_settings_changed = false;
        let mut phoneme_ruler_visibility_changed = false;
        if let Some(track) = self.project.tracks.get_mut(self.active_track_index) {
            if let Some(path) = track.voicebank_path.as_deref() {
                let root = std::path::Path::new(path);
                if root.join("character.yaml").is_file()
                    && (root.join("dsconfig.yaml").is_file() || root.join("configs/dsconfig.yaml").is_file())
                {
                    if let Ok(bank) = crate::voicebank::diffsinger::DiffSingerBank::inspect(root) {
                        if track.diffsinger_phonemizer.is_none() {
                            if bank.language_ids.contains_key("pt") {
                                track.diffsinger_language = Some("pt".into());
                                track.diffsinger_phonemizer = Some(crate::voicebank::diffsinger::DiffSingerPhonemizer::BrapaPhonetic);
                            } else {
                                track.diffsinger_phonemizer = Some(crate::voicebank::diffsinger::DiffSingerPhonemizer::Automatic);
                            }
                            self.piano_roll_state.show_phoneme_ruler = true;
                            self.config.layout.show_phoneme_ruler = true;
                            diffsinger_settings_changed = true;
                        }
                        ui.group(|ui| {
                            ui.label(egui::RichText::new("DiffSinger · faixa ativa").strong());
                            ui.label("Idioma");
                            egui::ComboBox::from_id_salt("diffsinger_language")
                                .selected_text(track.diffsinger_language.as_deref().unwrap_or("idioma"))
                                .show_ui(ui, |ui| for language in &bank.languages {
                                    diffsinger_settings_changed |= ui.selectable_value(&mut track.diffsinger_language, Some(language.clone()), language).changed();
                                });
                            if track.diffsinger_language.as_deref() == Some("pt") {
                                ui.label("Fonemizador DiffSinger");
                                egui::ComboBox::from_id_salt("diffsinger_phonemizer")
                                    .selected_text(match track.diffsinger_phonemizer.unwrap_or_default() {
                                        crate::voicebank::diffsinger::DiffSingerPhonemizer::Automatic => "G2P automático",
                                        crate::voicebank::diffsinger::DiffSingerPhonemizer::BrapaPhonetic => "BRAPA · fonético",
                                    })
                                    .show_ui(ui, |ui| {
                                        diffsinger_settings_changed |= ui.selectable_value(
                                            &mut track.diffsinger_phonemizer,
                                            Some(crate::voicebank::diffsinger::DiffSingerPhonemizer::BrapaPhonetic),
                                            "BRAPA · fonético",
                                        ).changed();
                                        diffsinger_settings_changed |= ui.selectable_value(
                                            &mut track.diffsinger_phonemizer,
                                            Some(crate::voicebank::diffsinger::DiffSingerPhonemizer::Automatic),
                                            "G2P automático",
                                        ).changed();
                                    });
                            }
                            egui::ComboBox::from_id_salt("diffsinger_speaker")
                                .selected_text(track.diffsinger_speaker.as_deref().unwrap_or("speaker"))
                                .show_ui(ui, |ui| for speaker in &bank.speakers {
                                    diffsinger_settings_changed |= ui.selectable_value(&mut track.diffsinger_speaker, Some(speaker.clone()), speaker).changed();
                                });
                            egui::ComboBox::from_id_salt("diffsinger_provider")
                                .selected_text(match track.diffsinger_provider.unwrap_or_default() {
                                    crate::voicebank::diffsinger::ExecutionProvider::Cpu => "CPU",
                                    crate::voicebank::diffsinger::ExecutionProvider::CoreMl => "CoreML",
                                })
                                .show_ui(ui, |ui| {
                                    diffsinger_settings_changed |= ui.selectable_value(&mut track.diffsinger_provider, Some(crate::voicebank::diffsinger::ExecutionProvider::Cpu), "CPU").changed();
                                    #[cfg(target_os = "macos")]
                                    {
                                        diffsinger_settings_changed |= ui.selectable_value(&mut track.diffsinger_provider, Some(crate::voicebank::diffsinger::ExecutionProvider::CoreMl), "CoreML").changed();
                                    }
                                });
                            let ruler_toggle = ui.checkbox(
                                &mut self.piano_roll_state.show_phoneme_ruler,
                                "Régua de phones",
                            );
                            phoneme_ruler_visibility_changed = ruler_toggle.changed();
                            ruler_toggle.on_hover_text(
                                "Arraste a divisão entre os phones para ajustar quanto tempo cada um ocupa na nota.",
                            );
                            if let Some(reason) = bank.diagnostic.as_deref() {
                                ui.colored_label(egui::Color32::LIGHT_RED, reason);
                            }
                        });
                    }
                }
            }
        }
        if diffsinger_settings_changed {
            self.is_dirty = true;
            self.piano_roll_state.phoneme_cache_hash = 0;
        }
        if phoneme_ruler_visibility_changed {
            self.config.layout.show_phoneme_ruler = self.piano_roll_state.show_phoneme_ruler;
            self.persist_config();
        }

        // The phonemizer selector lives in the sidebar, outside the note
        // editor's project snapshot. Invalidate phoneme previews immediately
        // when it changes so the next frame cannot keep showing aliases from
        // the previous family.
        if self.vocal_mode_params.phonemizer_mode != phonemizer_before_panel {
            self.piano_roll_state.phoneme_cache_hash = 0;
            self.piano_roll_state.phoneme_cache.clear();
            self.piano_roll_state.note_phonemes_cache.clear();
        }

        let watched_notes_changed = watched_note_snapshots.iter().any(|(index, before)| {
            self.project
                .parts
                .get(part_idx)
                .and_then(|part| part.notes.get(*index))
                != Some(before)
        });
        if watched_notes_changed {
            let snapshot = self.pending_edit_snapshot.take().unwrap_or_else(|| {
                let mut snapshot = self.project.clone();
                if let Some(part) = snapshot.parts.get_mut(part_idx) {
                    for (index, before) in &watched_note_snapshots {
                        if let Some(note) = part.notes.get_mut(*index) {
                            *note = before.clone();
                        }
                    }
                }
                snapshot
            });
            if pointer_down {
                self.pending_edit_snapshot = Some(snapshot);
            } else if self.project != snapshot {
                self.undo_manager.push_state(snapshot);
                self.piano_roll_state.phoneme_cache.clear();
                self.piano_roll_state.note_phonemes_cache.clear();
            }
            self.is_dirty = true;
        } else if !pointer_down {
            if let Some(snapshot) = self.pending_edit_snapshot.take() {
                if self.project != snapshot {
                    self.undo_manager.push_state(snapshot);
                    self.piano_roll_state.phoneme_cache.clear();
                    self.piano_roll_state.note_phonemes_cache.clear();
                }
            }
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

        if let Some(alias) = preview_alias {
            let mut played = false;
            if let (Some(note_index), Some(vb)) = (selected_idx, self.voicebank.as_ref()) {
                let previews_selected_note = self
                    .project
                    .parts
                    .get(part_idx)
                    .and_then(|part| part.notes.get(note_index))
                    .is_some_and(|note| note.lyric == alias);
                if !previews_selected_note {
                    // A palette alias that is not the selected note remains a
                    // direct sample preview; never render unrelated music.
                } else {
                    let resampler = self.create_resampler_driver();
                    let wavtool = self.create_wavtool_driver();
                    let mut options = self.vocal_mode_params.clone();
                    options.resampler_instances = self.config.dsp.resampler_instances.max(1);
                    let preview = ProjectRenderer::render_preview_target_with_drivers(
                        &self.project,
                        vb,
                        self.sample_rate,
                        PreviewTarget::Note {
                            part_index: part_idx,
                            note_index,
                        },
                        resampler.as_ref(),
                        wavtool.as_ref(),
                        &options,
                        None,
                        None,
                    );
                    if preview.error.is_none() && !preview.samples.is_empty() {
                        self.audio_player
                            .play_samples(preview.samples, preview.sample_rate);
                        played = true;
                    }
                }
            }
            if let Some(ref vb) = self.voicebank {
                if !played {
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
        #[cfg(not(target_os = "android"))]
        if let Some((alias, pitch)) = edit_alias {
            self.open_copaiba_for_alias(&alias, &pitch);
        }
    }

    pub(crate) fn draw_arrangement_inner(&mut self, ui: &mut egui::Ui) {
        // Undo preparation used to clone the entire project whenever any
        // primary button was held anywhere in the app. Restrict it to a press
        // that actually began over the arrangement panel.
        let pointer_is_over_arrangement = ui.input(|input| {
            input
                .pointer
                .press_origin()
                .or(input.pointer.interact_pos())
                .is_some_and(|pos| ui.max_rect().contains(pos))
        });
        let mut arrangement_snapshot_before = if self.pending_edit_snapshot.is_none()
            && ui.input(|i| {
                (pointer_is_over_arrangement
                    && (i.pointer.primary_down()
                        || i.pointer.secondary_down()
                        || i.pointer.any_released()))
                    || i.events.iter().any(|event| {
                        matches!(
                            event,
                            egui::Event::Key { pressed: true, .. } | egui::Event::Text(_)
                        )
                    })
            }) {
            Some(self.project.clone())
        } else {
            None
        };

        let actual_h = ui.max_rect().height().clamp(60.0, 500.0);
        self.piano_roll_state.arrangement_height = actual_h;

        let mut master_fx = self.project.master_fx.clone().unwrap_or_default();
        let original_master_fx = master_fx.clone();

        let mut available_singers: Vec<String> = self
            .config
            .recent_voicebanks
            .iter()
            .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()))
            .collect();
        for t in &self.project.tracks {
            if !t.singer.is_empty() && !available_singers.contains(&t.singer) {
                available_singers.push(t.singer.clone());
            }
        }

        let arrangement_changed = draw_arrangement_view(
            ui,
            &self.config.theme,
            &mut self.project.tracks,
            &mut self.project.parts,
            &mut self.project.wave_parts,
            &mut self.active_track_index,
            &mut self.piano_roll_state.playhead_ms,
            &mut self.piano_roll_state.arrangement_zoom,
            self.transport_state.bpm,
            &mut self.piano_roll_state.arrangement_horizontal_scroll_offset,
            &mut self.piano_roll_state.arrangement_vertical_scroll_offset,
            &mut self.piano_roll_state.arrangement_follow_override,
            &mut self.piano_roll_state.arrangement_drag_track,
            &mut self.piano_roll_state.arrangement_track_meters,
            self.piano_roll_state.is_playing,
            &mut self.fx_rack_dialog_state,
            self.config.language,
            &mut master_fx,
            &available_singers,
        );
        if master_fx != original_master_fx {
            self.project.master_fx = Some(master_fx);
        }

        if arrangement_changed {
            let pointer_down = ui.input(|i| i.pointer.primary_down() || i.pointer.secondary_down());
            if pointer_down {
                if self.pending_edit_snapshot.is_none() {
                    if let Some(snapshot) = arrangement_snapshot_before.as_ref() {
                        self.pending_edit_snapshot = Some(snapshot.clone());
                    }
                }
            } else {
                if let Some(snapshot) = self.pending_edit_snapshot.take() {
                    self.undo_manager.push_state(snapshot);
                } else if let Some(snapshot) = arrangement_snapshot_before.take() {
                    // A click commits on release. Move the captured snapshot
                    // into history instead of cloning the whole project again.
                    self.undo_manager.push_state(snapshot);
                }
            }
            self.is_dirty = true;
        }
    }

    fn render_mobile_bottom_bar(&mut self, ctx: &egui::Context) {
        let lang = self.config.language;
        let theme = &self.config.theme;

        TopBottomPanel::bottom("mobile_bottom_nav_bar")
            .exact_height(68.0)
            .frame(
                Frame::none()
                    .fill(theme.bg_panel_c32())
                    .stroke(egui::Stroke::new(1.0_f32, theme.grid_line_sub_c32())),
            )
            .show(ctx, |ui| {
                let tab_width = (ui.available_width() / 4.0).max(64.0);
                egui::ScrollArea::horizontal()
                    .id_salt("mobile_bottom_nav_scroll")
                    .auto_shrink([false, true])
                    .drag_to_scroll(true)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let tabs = [
                                (
                                    crate::gui::types::MobileViewTab::PianoRoll,
                                    "🎹",
                                    lang.tr("Notas", "Notes"),
                                ),
                                (
                                    crate::gui::types::MobileViewTab::Arrangement,
                                    "🎚️",
                                    lang.tr("Faixas", "Tracks"),
                                ),
                                (
                                    crate::gui::types::MobileViewTab::Inspector,
                                    "ℹ️",
                                    lang.tr("Inspetor", "Inspector"),
                                ),
                                (
                                    crate::gui::types::MobileViewTab::Settings,
                                    "⚙️",
                                    lang.tr("Ajustes", "Settings"),
                                ),
                            ];

                            for (tab, icon, label) in tabs {
                                let is_selected = self.active_mobile_tab == tab;
                                let (bg, stroke, text_color) = if is_selected {
                                    (
                                        theme.c32_alpha(theme.accent_color, 0.25),
                                        egui::Stroke::new(1.2_f32, theme.accent_c32()),
                                        theme.accent_c32(),
                                    )
                                } else {
                                    (
                                        egui::Color32::TRANSPARENT,
                                        egui::Stroke::NONE,
                                        theme.text_muted_c32(),
                                    )
                                };

                                let btn = egui::Button::new(
                                    egui::RichText::new(format!("{icon}\n{label}"))
                                        .size(11.0)
                                        .strong()
                                        .color(text_color),
                                )
                                .min_size(egui::Vec2::new(tab_width - 4.0, 56.0))
                                .fill(bg)
                                .stroke(stroke)
                                .rounding(egui::Rounding::same(6.0));

                                if ui.add(btn).clicked() {
                                    self.active_mobile_tab = tab;
                                }
                            }
                        });
                    });
            });
    }
}
