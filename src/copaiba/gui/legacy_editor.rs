use super::*;

pub(super) fn draw_legacy_editor(
    app: &mut CopaibaToolkitApp,
    audio_player: &mut AudioPlayer,
    ui: &mut egui::Ui,
) {
    egui::Frame::none()
        .fill(Color32::from_rgb(24, 24, 24))
        .rounding(Rounding::ZERO)
        .stroke(Stroke::new(2.5_f32, Color32::BLACK))
        .inner_margin(egui::Margin::same(8.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let (avatar_rect, _) =
                    ui.allocate_exact_size(Vec2::new(100.0, 100.0), Sense::hover());
                let painter = ui.painter_at(avatar_rect);
                painter.rect_filled(avatar_rect, Rounding::same(8.0), visual_tokens::BACKGROUND);
                painter.rect_stroke(
                    avatar_rect,
                    Rounding::ZERO,
                    Stroke::new(2.0_f32, Color32::BLACK),
                );

                if app.avatar_texture.is_none() {
                    app.reload_avatar_texture(ui.ctx());
                }

                if let Some(ref texture) = app.avatar_texture {
                    let uv = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0));
                    painter.image(texture.id(), avatar_rect.shrink(2.0), uv, Color32::WHITE);
                } else {
                    let center = avatar_rect.center();
                    painter.text(
                        Pos2::new(center.x, center.y - 6.0),
                        egui::Align2::CENTER_CENTER,
                        "[Foto]",
                        egui::FontId::proportional(12.0),
                        Color32::from_rgb(165, 148, 201),
                    );
                    painter.text(
                        Pos2::new(center.x, center.y + 12.0),
                        egui::Align2::CENTER_CENTER,
                        "Sem Imagem",
                        egui::FontId::proportional(10.0),
                        Color32::from_rgb(165, 148, 201),
                    );
                }

                ui.add_space(10.0);

                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("Nome do Voicebank:")
                                .size(11.0)
                                .strong()
                                .color(visual_tokens::ACCENT),
                        );
                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut app.config.voicebank_name)
                                    .desired_width(180.0),
                            )
                            .changed()
                        {
                            if let Err(err) = app.save_config() {
                                app.status_message = format!("Erro ao salvar: {err}");
                            }
                        }

                        ui.add_space(12.0);
                        ui.label(
                            RichText::new("Autor / Criador:")
                                .size(11.0)
                                .strong()
                                .color(Color32::from_rgb(216, 180, 254)),
                        );
                        if ui
                            .add(
                                egui::TextEdit::singleline(&mut app.config.author)
                                    .desired_width(160.0),
                            )
                            .changed()
                        {
                            if let Err(err) = app.save_config() {
                                app.status_message = format!("Erro ao salvar: {err}");
                            }
                        }
                    });

                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        let import_btn = egui::Button::new(
                            RichText::new("Importar Foto / Avatar (100x100)...")
                                .size(11.0)
                                .strong()
                                .color(visual_tokens::ACCENT),
                        )
                        .fill(Color32::from_rgb(10, 48, 30))
                        .stroke(Stroke::new(1.0_f32, visual_tokens::ACCENT))
                        .rounding(Rounding::same(4.0));

                        if ui.add(import_btn).clicked() {
                            if let Some(file) = crate::dialogs::FileDialog::new()
                                .add_filter(
                                    "Imagens (*.png, *.jpg, *.jpeg, *.bmp, *.webp)",
                                    &["png", "jpg", "jpeg", "bmp", "webp"],
                                )
                                .pick_file()
                            {
                                if let Err(err) = app.import_avatar_image(ui.ctx(), file) {
                                    app.status_message = err;
                                }
                            }
                        }

                        if (app.config.image_filename.is_some() || app.avatar_texture.is_some())
                            && ui
                                .button(
                                    RichText::new("Remover")
                                        .size(11.0)
                                        .color(Color32::from_rgb(255, 100, 100)),
                                )
                                .clicked()
                        {
                            app.status_message = match app.remove_avatar_image(ui.ctx()) {
                                Ok(()) => "Avatar removido".to_string(),
                                Err(err) => err,
                            };
                        }

                        if let Some(ref img_name) = app.config.image_filename {
                            ui.label(
                                RichText::new(format!("Arquivo: {}", img_name))
                                    .size(10.0)
                                    .italics()
                                    .color(Color32::from_rgb(165, 148, 201)),
                            );
                        }
                    });
                });
            });
        });

    ui.add_space(4.0);

    let full_avail_h = ui.available_height();

    ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(Vec2::new(220.0, full_avail_h), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    if ui.selectable_label(app.filter_mode == ListFilterMode::ByAlias, "Alias").clicked() {
                        app.filter_mode = ListFilterMode::ByAlias;
                    }
                    if ui.selectable_label(app.filter_mode == ListFilterMode::ByWavFile, "WAVs").clicked() {
                        app.filter_mode = ListFilterMode::ByWavFile;
                    }
                    if ui.selectable_label(app.filter_mode == ListFilterMode::PrefixMap, "Tons").clicked() {
                        app.filter_mode = ListFilterMode::PrefixMap;
                    }
                });

                ui.add_space(4.0);
                ui.add(egui::TextEdit::singleline(&mut app.search_query).hint_text("Filtrar alias ou wav...").desired_width(210.0));
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if ui.button(RichText::new("+ Duplicar").size(10.5).color(visual_tokens::ACCENT)).clicked() {
                        app.duplicate_selected_entry();
                    }
                    if ui.button(RichText::new("Excluir").size(10.5).color(Color32::from_rgb(255, 100, 100))).clicked() {
                        app.delete_selected_entry();
                    }
                });

                ui.add_space(4.0);
                ui.separator();

                let mut new_selection = app.selected_entry_index;

                egui::ScrollArea::vertical()
                    .id_salt("copaiba_alias_scroll_list")
                    .max_height(ui.available_height())
                    .show(ui, |ui| {
                        for (idx, entry) in app.config.entries.iter().enumerate() {
                            let query = app.search_query.to_lowercase();
                            if !query.is_empty() {
                                let match_alias = entry.alias.to_lowercase().contains(&query);
                                let match_wav = entry.wav_filename.to_lowercase().contains(&query);
                                if !match_alias && !match_wav {
                                    continue;
                                }
                            }

                            let is_sel = app.selected_entry_index == Some(idx);
                            let display_title = if app.filter_mode == ListFilterMode::ByAlias {
                                format!("{} ({})", entry.alias, entry.wav_filename)
                            } else {
                                format!("{} -> {}", entry.wav_filename, entry.alias)
                            };

                            let text_color = if is_sel {
                                visual_tokens::ACCENT
                            } else {
                                Color32::from_rgb(210, 200, 230)
                            };

                            if ui.selectable_label(is_sel, RichText::new(display_title).size(11.0).color(text_color)).clicked() {
                                new_selection = Some(idx);
                                if app.filter_mode == ListFilterMode::PrefixMap {
                                    app.filter_mode = ListFilterMode::ByAlias;
                                }
                            }
                        }
                    });

                if new_selection != app.selected_entry_index {
                    app.selected_entry_index = new_selection;
                    app.load_selected_wav_samples();
                }
            });

            ui.separator();

            if app.filter_mode == ListFilterMode::PrefixMap {
                ui.allocate_ui_with_layout(Vec2::new(ui.available_width(), full_avail_h), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.vertical(|ui| {
                        ui.heading(RichText::new("Mapeamento Multitom (Prefix Map / Subbanks)").strong().color(Color32::from_rgb(180, 255, 0)));
                        ui.label("Configure prefixos e sufixos para tons específicos (subbanks) do seu voicebank.");
                        ui.add_space(10.0);

                        egui::Frame::none()
                            .fill(Color32::from_rgb(24, 24, 24))
                            .rounding(Rounding::ZERO)
                            .stroke(Stroke::new(2.5_f32, Color32::BLACK))
                            .inner_margin(egui::Margin::same(8.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Tom (ex: C4):");
                                    ui.add(egui::TextEdit::singleline(&mut app.new_mapping_pitch).desired_width(60.0));
                                    ui.label("Prefixo:");
                                    ui.add(egui::TextEdit::singleline(&mut app.new_mapping_prefix).desired_width(80.0));
                                    ui.label("Sufixo:");
                                    ui.add(egui::TextEdit::singleline(&mut app.new_mapping_suffix).desired_width(80.0));

                                    let add_btn = egui::Button::new(RichText::new("+ Adicionar").strong().color(Color32::BLACK))
                                        .fill(Color32::from_rgb(180, 255, 0))
                                        .stroke(Stroke::new(2.0_f32, Color32::BLACK));

                                    if ui.add(add_btn).clicked() {

                                        let pitch = app.new_mapping_pitch.trim().to_string();
                                        if !pitch.is_empty() {
                                            app.config.prefix_map.insert(
                                                pitch,
                                                (app.new_mapping_prefix.clone(), app.new_mapping_suffix.clone()),
                                            );
                                            app.new_mapping_pitch.clear();
                                            app.new_mapping_prefix.clear();
                                            app.new_mapping_suffix.clear();
                                            if let Err(err) = app.save_config() {
                                                app.status_message = format!("Erro ao salvar: {err}");
                                            }
                                        }
                                    }

                                    ui.add_space(10.0);
                                    if ui.button("Gerar Oitavas Padrão").clicked() {
                                        let octaves = vec!["C3", "F3", "C4", "F4", "C5", "F5"];
                                        for oct in octaves {
                                            app.config.prefix_map.insert(
                                                oct.to_string(),
                                                (String::new(), format!("_{}", oct)),
                                            );
                                        }
                                        if let Err(err) = app.save_config() {
                                            app.status_message = format!("Erro ao salvar: {err}");
                                        }
                                    }
                                });
                            });

                        ui.add_space(15.0);

                        egui::ScrollArea::vertical()
                            .id_salt("prefix_map_rules_scroll")
                            .show(ui, |ui| {
                                egui::Grid::new("prefix_map_grid")
                                    .striped(true)
                                    .spacing([20.0, 10.0])
                                    .show(ui, |ui| {
                                        ui.label(RichText::new("Tom").strong().color(Color32::from_rgb(255, 215, 0)));
                                        ui.label(RichText::new("Prefixo").strong().color(Color32::from_rgb(216, 180, 254)));
                                        ui.label(RichText::new("Sufixo").strong().color(Color32::from_rgb(216, 180, 254)));
                                        ui.label(RichText::new("Ações").strong().color(Color32::WHITE));
                                        ui.end_row();

                                        let mut to_remove = None;
                                        let mut keys: Vec<String> = app.config.prefix_map.keys().cloned().collect();
                                        keys.sort();

                                        let mut changed = false;
                                        for key in keys {
                                            let mut entry_removed = false;
                                            if let Some((pref, suff)) = app.config.prefix_map.get_mut(&key) {
                                                ui.label(RichText::new(&key).strong().color(visual_tokens::ACCENT));

                                                if ui.add(egui::TextEdit::singleline(pref).desired_width(100.0)).changed() {
                                                    changed = true;
                                                }
                                                if ui.add(egui::TextEdit::singleline(suff).desired_width(100.0)).changed() {
                                                    changed = true;
                                                }

                                                if ui.button(RichText::new("Excluir").color(Color32::from_rgb(255, 100, 100))).clicked() {
                                                    to_remove = Some(key.clone());
                                                    entry_removed = true;
                                                }
                                                ui.end_row();
                                            }
                                            if entry_removed {
                                                break;
                                            }
                                        }

                                        if let Some(rem_key) = to_remove {
                                            app.config.prefix_map.remove(&rem_key);
                                            changed = true;
                                        }

                                        if changed {
                                            if let Err(err) = app.save_config() {
                                                app.status_message = format!("Erro ao salvar: {err}");
                                            }
                                        }
                                    });
                            });
                    });
                });
            } else {
                let center_w = (ui.available_width() - 270.0).max(300.0);
                ui.allocate_ui_with_layout(Vec2::new(center_w, full_avail_h), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    let current_idx = app.selected_entry_index.unwrap_or(0);
                    let prev_entry = if current_idx > 0 { app.config.entries.get(current_idx - 1).cloned() } else { None };

                let next_entry = app.config.entries.get(current_idx + 1).cloned();

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Editor de Forma de Onda (Triplo Stack)").strong().size(12.0).color(visual_tokens::ACCENT));
                    ui.add_space(10.0);

                    if let Some((ref samples, sr)) = app.loaded_waveform {
                        if ui.button("▶ Tocar").clicked() {
                            audio_player.play_samples(samples.clone(), sr);
                        }
                    }

                    ui.add_space(10.0);
                    ui.label(RichText::new("Zoom X:").size(10.0).color(Color32::from_rgb(180, 170, 190)));
                    ui.add(egui::Slider::new(&mut app.zoom_x, 1.0..=5.0).show_value(false));
                    ui.label(RichText::new("Zoom Y:").size(10.0).color(Color32::from_rgb(180, 170, 190)));
                    ui.add(egui::Slider::new(&mut app.zoom_y, 0.5..=4.0).show_value(false));
                });

                ui.add_space(4.0);

                let available_center_h = ui.available_height() - 10.0;
                let prev_next_h = (available_center_h * 0.18).clamp(55.0, 90.0);
                let main_h = available_center_h - (prev_next_h * 2.0) - 12.0;

                ui.horizontal(|ui| {
                    if let Some(ref p_entry) = prev_entry {
                        ui.label(RichText::new(format!("▲ Anterior: {} ({})", p_entry.alias, p_entry.wav_filename)).size(10.0).color(Color32::from_rgb(150, 140, 170)));
                    } else {
                        ui.label(RichText::new("▲ (Sem alias anterior)").size(10.0).color(Color32::from_rgb(90, 80, 110)));
                    }
                });

                let (prev_rect, prev_resp) = ui.allocate_exact_size(Vec2::new(center_w, prev_next_h), Sense::click());
                let prev_painter = ui.painter_at(prev_rect);
                prev_painter.rect_filled(prev_rect, Rounding::same(4.0), Color32::from_rgb(14, 11, 22));
                prev_painter.rect_stroke(prev_rect, Rounding::same(4.0), Stroke::new(1.0_f32, Color32::from_rgb(35, 28, 48)));

                if let Some(ref p_entry) = prev_entry {
                    if prev_resp.clicked() {
                        app.selected_entry_index = Some(current_idx - 1);
                        app.load_selected_wav_samples();
                    }
                    if let Some((cached_name, samples, _)) = &app.previous_waveform {
                        if cached_name == &p_entry.wav_filename {
                            draw_waveform_peak_envelope(&prev_painter, prev_rect, samples, Color32::from_rgb(0, 150, 100), app.zoom_y * 0.8, 1.0);
                        }
                    }
                }

                ui.add_space(4.0);

                let (canvas_rect, canvas_resp) = ui.allocate_exact_size(Vec2::new(center_w, main_h.max(120.0)), Sense::click_and_drag());
                let painter = ui.painter_at(canvas_rect);

                painter.rect_filled(canvas_rect, Rounding::same(6.0), visual_tokens::BACKGROUND);
                painter.rect_stroke(canvas_rect, Rounding::ZERO, Stroke::new(2.5_f32, Color32::BLACK));


                let zoom_y = app.zoom_y;
                let sel_idx = app.selected_entry_index;

                if let (Some((ref samples, sr)), Some(idx)) = (&app.loaded_waveform, sel_idx) {
                    if idx < app.config.entries.len() {
                        let entry = &mut app.config.entries[idx];
                        let total_duration_ms = (samples.len() as f64 / *sr as f64) * 1000.0;

                        let visible_duration_ms = total_duration_ms / app.zoom_x.max(1.0) as f64;
                        let ms_to_x = |ms: f64| -> f32 {
                            let ratio = (ms / visible_duration_ms).clamp(0.0, 1.0) as f32;
                            canvas_rect.min.x + ratio * canvas_rect.width()
                        };

                        let x_to_ms = |x: f32| -> f64 {
                            let ratio = ((x - canvas_rect.min.x) / canvas_rect.width()).clamp(0.0, 1.0) as f64;
                            ratio * visible_duration_ms
                        };

                        let ruler_h = 16.0f32;
                        let ruler_rect = Rect::from_min_size(canvas_rect.min, Vec2::new(canvas_rect.width(), ruler_h));
                        painter.rect_filled(ruler_rect, Rounding::same(2.0), Color32::from_rgb(22, 18, 34));
                        painter.line_segment([Pos2::new(ruler_rect.min.x, ruler_rect.max.y), Pos2::new(ruler_rect.max.x, ruler_rect.max.y)], Stroke::new(1.0_f32, Color32::from_rgb(60, 50, 80)));

                        let step_ms = if total_duration_ms > 2000.0 { 200.0 } else { 50.0 };
                        let mut ms_tick = 0.0f64;
                        while ms_tick <= visible_duration_ms {
                            let tick_x = ms_to_x(ms_tick);
                            if tick_x >= canvas_rect.min.x && tick_x <= canvas_rect.max.x {
                                painter.line_segment(
                                    [Pos2::new(tick_x, ruler_rect.min.y + 8.0), Pos2::new(tick_x, canvas_rect.max.y)],
                                    Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(80, 70, 110, 100)),
                                );
                                painter.text(
                                    Pos2::new(tick_x + 3.0, ruler_rect.min.y + 2.0),
                                    egui::Align2::LEFT_TOP,
                                    format!("{:.0}ms", ms_tick),
                                    egui::FontId::proportional(9.0),
                                    Color32::from_rgb(160, 150, 190),
                                );
                            }
                            ms_tick += step_ms;
                        }

                        let wave_area_min_y = canvas_rect.min.y + ruler_h;

                        let x_offset = ms_to_x(entry.corte_inicial_ms);
                        let x_consonant = ms_to_x(entry.corte_inicial_ms + entry.consoante_ms);
                        let cutoff_marker_ms = if entry.corte_final_ms > 0.0 {
                            total_duration_ms - entry.corte_final_ms
                        } else if entry.corte_final_ms < 0.0 {
                            entry.corte_inicial_ms - entry.corte_final_ms
                        } else {
                            total_duration_ms
                        };
                        let x_cutoff = ms_to_x(cutoff_marker_ms);
                        let x_tail = entry.cauda_final_ms.map(&ms_to_x);
                        let x_preutterance = ms_to_x(entry.corte_inicial_ms + entry.preutterance_ms);
                        let x_overlap = ms_to_x(entry.corte_inicial_ms + entry.overlap_ms);


                        let left_cut_rect = Rect::from_min_max(Pos2::new(canvas_rect.min.x, wave_area_min_y), Pos2::new(x_offset, canvas_rect.max.y));
                        painter.rect_filled(left_cut_rect, Rounding::ZERO, Color32::from_rgba_premultiplied(0, 60, 150, 45));

                        let cons_rect = Rect::from_min_max(Pos2::new(x_offset, wave_area_min_y), Pos2::new(x_consonant, canvas_rect.max.y));
                        painter.rect_filled(cons_rect, Rounding::ZERO, Color32::from_rgba_premultiplied(0, 150, 80, 25));

                        let right_cut_rect = Rect::from_min_max(Pos2::new(x_cutoff, wave_area_min_y), canvas_rect.max);
                        painter.rect_filled(right_cut_rect, Rounding::ZERO, Color32::from_rgba_premultiplied(150, 40, 40, 45));

                        if let (Some(l_start), Some(l_end)) = (entry.loop_inicio_ms, entry.loop_fim_ms) {
                            let x_lstart = ms_to_x(l_start);
                            let x_lend = ms_to_x(l_end);
                            let loop_rect = Rect::from_min_max(Pos2::new(x_lstart, wave_area_min_y), Pos2::new(x_lend, canvas_rect.max.y));
                            painter.rect_filled(loop_rect, Rounding::ZERO, Color32::from_rgba_premultiplied(160, 140, 0, 25));
                            painter.line_segment([Pos2::new(x_lstart, wave_area_min_y), Pos2::new(x_lstart, canvas_rect.max.y)], Stroke::new(1.5_f32, Color32::from_rgb(220, 200, 0)));
                            painter.line_segment([Pos2::new(x_lend, wave_area_min_y), Pos2::new(x_lend, canvas_rect.max.y)], Stroke::new(1.5_f32, Color32::from_rgb(220, 200, 0)));
                        }

                        if let Some(xt) = x_tail {
                            let tail_rect = Rect::from_min_max(Pos2::new(xt, wave_area_min_y), Pos2::new((xt + 40.0).min(canvas_rect.max.x), canvas_rect.max.y));
                            painter.rect_filled(tail_rect, Rounding::ZERO, Color32::from_rgba_premultiplied(120, 30, 180, 25));
                            painter.line_segment([Pos2::new(xt, wave_area_min_y), Pos2::new(xt, canvas_rect.max.y)], Stroke::new(1.5_f32, Color32::from_rgb(160, 50, 220)));
                            painter.text(Pos2::new(xt + 3.0, wave_area_min_y + 4.0), egui::Align2::LEFT_TOP, "Cauda Final", egui::FontId::proportional(10.0), Color32::from_rgb(180, 60, 240));
                        }

                        draw_waveform_peak_envelope(&painter, Rect::from_min_max(Pos2::new(canvas_rect.min.x, wave_area_min_y), canvas_rect.max), samples, visual_tokens::ACCENT, zoom_y, app.zoom_x);


                        painter.line_segment([Pos2::new(x_offset, wave_area_min_y), Pos2::new(x_offset, canvas_rect.max.y)], Stroke::new(2.5_f32, Color32::from_rgb(0, 150, 255)));
                        painter.text(Pos2::new(x_offset + 3.0, wave_area_min_y + 4.0), egui::Align2::LEFT_TOP, "Corte Inicial", egui::FontId::proportional(10.0), Color32::from_rgb(0, 180, 255));

                        painter.line_segment([Pos2::new(x_consonant, wave_area_min_y), Pos2::new(x_consonant, canvas_rect.max.y)], Stroke::new(2.0_f32, visual_tokens::SUCCESS));
                        painter.text(Pos2::new(x_consonant + 3.0, wave_area_min_y + 4.0), egui::Align2::LEFT_TOP, "Início Consoante", egui::FontId::proportional(10.0), visual_tokens::SUCCESS);

                        painter.line_segment([Pos2::new(x_cutoff, wave_area_min_y), Pos2::new(x_cutoff, canvas_rect.max.y)], Stroke::new(2.5_f32, Color32::from_rgb(255, 60, 60)));
                        painter.text(Pos2::new(x_cutoff + 3.0, wave_area_min_y + 4.0), egui::Align2::LEFT_TOP, "Corte Final", egui::FontId::proportional(10.0), Color32::from_rgb(255, 80, 80));

                        painter.line_segment([Pos2::new(x_preutterance, wave_area_min_y), Pos2::new(x_preutterance, canvas_rect.max.y)], Stroke::new(2.5_f32, Color32::from_rgb(255, 0, 180)));
                        painter.text(Pos2::new(x_preutterance + 3.0, wave_area_min_y + 16.0), egui::Align2::LEFT_TOP, "Pre-utterance", egui::FontId::proportional(10.0), Color32::from_rgb(255, 0, 180));

                        painter.line_segment([Pos2::new(x_overlap, wave_area_min_y), Pos2::new(x_overlap, canvas_rect.max.y)], Stroke::new(2.5_f32, Color32::from_rgb(255, 140, 0)));
                        painter.text(Pos2::new(x_overlap + 3.0, wave_area_min_y + 28.0), egui::Align2::LEFT_TOP, "Overlap", egui::FontId::proportional(10.0), Color32::from_rgb(255, 140, 0));

                        let is_typing = ui.ctx().wants_keyboard_input();
                        if !is_typing {
                            if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
                                if canvas_rect.contains(hover_pos) {
                                    let hovered_ms = x_to_ms(hover_pos.x);
                                    ui.input(|i| {
                                        if i.key_down(egui::Key::Q) {
                                            entry.corte_inicial_ms = hovered_ms.clamp(0.0, total_duration_ms);
                                        } else if i.key_down(egui::Key::W) {
                                            entry.consoante_ms = (hovered_ms - entry.corte_inicial_ms).max(1.0);
                                        } else if i.key_down(egui::Key::E) {
                                            entry.corte_final_ms = if entry.corte_final_ms < 0.0 {
                                                -(hovered_ms - entry.corte_inicial_ms).max(0.0)
                                            } else {
                                                (total_duration_ms - hovered_ms).max(0.0)
                                            };
                                        } else if i.key_down(egui::Key::R) {
                                            entry.preutterance_ms = (hovered_ms - entry.corte_inicial_ms).max(0.0);
                                        } else if i.key_down(egui::Key::T) {
                                            entry.overlap_ms = (hovered_ms - entry.corte_inicial_ms).max(0.0);
                                        } else if i.key_down(egui::Key::Y) {
                                            if entry.loop_inicio_ms.is_some() {
                                                entry.loop_inicio_ms = Some(hovered_ms.clamp(0.0, total_duration_ms));
                                            } else if entry.cauda_final_ms.is_some() {
                                                entry.cauda_final_ms = Some(hovered_ms.clamp(0.0, total_duration_ms));
                                            }
                                        } else if i.key_down(egui::Key::U)
                                            && entry.loop_fim_ms.is_some()
                                        {
                                            entry.loop_fim_ms =
                                                Some(hovered_ms.clamp(0.0, total_duration_ms));
                                        }
                                    });
                                }
                            }
                        }

                        if let Some(mpos) = canvas_resp.interact_pointer_pos() {
                            if canvas_resp.dragged() || canvas_resp.clicked() {
                                let clicked_ms = x_to_ms(mpos.x);

                                let dist_offset = (mpos.x - x_offset).abs();
                                let dist_cons = (mpos.x - x_consonant).abs();
                                let dist_cutoff = (mpos.x - x_cutoff).abs();
                                let dist_pre = (mpos.x - x_preutterance).abs();
                                let dist_over = (mpos.x - x_overlap).abs();
                                let dist_tail = x_tail.map(|xt| (mpos.x - xt).abs()).unwrap_or(f32::MAX);

                                let min_dist = dist_offset.min(dist_cons).min(dist_cutoff).min(dist_pre).min(dist_over).min(dist_tail);

                                if min_dist == dist_tail && entry.cauda_final_ms.is_some() {
                                    entry.cauda_final_ms = Some(clicked_ms.clamp(0.0, total_duration_ms));
                                } else if min_dist == dist_offset {
                                    entry.corte_inicial_ms = clicked_ms.clamp(0.0, total_duration_ms);
                                } else if min_dist == dist_cons {
                                    entry.consoante_ms = (clicked_ms - entry.corte_inicial_ms).max(1.0);
                                } else if min_dist == dist_cutoff {
                                    entry.corte_final_ms = if entry.corte_final_ms < 0.0 {
                                        -(clicked_ms - entry.corte_inicial_ms).max(0.0)
                                    } else {
                                        (total_duration_ms - clicked_ms).max(0.0)
                                    };
                                } else if min_dist == dist_pre {
                                    entry.preutterance_ms = (clicked_ms - entry.corte_inicial_ms).max(0.0);
                                } else {
                                    entry.overlap_ms = (clicked_ms - entry.corte_inicial_ms).max(0.0);
                                }
                            }
                        }

                    }
                }

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if let Some(ref n_entry) = next_entry {
                        ui.label(RichText::new(format!("▼ Próxima: {} ({})", n_entry.alias, n_entry.wav_filename)).size(10.0).color(Color32::from_rgb(150, 140, 170)));
                    } else {
                        ui.label(RichText::new("▼ (Sem próxima alias)").size(10.0).color(Color32::from_rgb(90, 80, 110)));
                    }
                });

                let (next_rect, next_resp) = ui.allocate_exact_size(Vec2::new(center_w, prev_next_h), Sense::click());
                let next_painter = ui.painter_at(next_rect);
                next_painter.rect_filled(next_rect, Rounding::same(4.0), Color32::from_rgb(14, 11, 22));
                next_painter.rect_stroke(next_rect, Rounding::same(4.0), Stroke::new(1.0_f32, Color32::from_rgb(35, 28, 48)));

                if let Some(ref n_entry) = next_entry {
                    if next_resp.clicked() {
                        app.selected_entry_index = Some(current_idx + 1);
                        app.load_selected_wav_samples();
                    }
                    if let Some((cached_name, samples, _)) = &app.next_waveform {
                        if cached_name == &n_entry.wav_filename {
                            draw_waveform_peak_envelope(&next_painter, next_rect, samples, Color32::from_rgb(0, 150, 100), app.zoom_y * 0.8, 1.0);
                        }
                    }
                }
            });

            ui.separator();

            ui.allocate_ui_with_layout(Vec2::new(260.0, full_avail_h), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.label(RichText::new("Controles & Sliders").strong().size(12.0).color(visual_tokens::ACCENT));
                ui.add_space(6.0);

                egui::ScrollArea::vertical()
                    .id_salt("copaiba_right_controls_scroll")
                    .max_height(ui.available_height())
                    .show(ui, |ui| {
                        let selected_index = app
                            .selected_entry_index
                            .filter(|idx| *idx < app.config.entries.len());
                        let wav_duration_ms = app
                            .loaded_waveform
                            .as_ref()
                            .map(|(samples, sample_rate)| {
                                (samples.len() as f64 / *sample_rate as f64) * 1000.0
                            })
                            .filter(|duration_ms| duration_ms.is_finite() && *duration_ms > 0.0);

                        if let Some(idx) = selected_index {
                            // Keep alias parameters editable even when the referenced WAV is
                            // missing or unreadable. In that case use a temporary range based
                            // on the stored values and clearly label it as an estimate.
                            let has_waveform = wav_duration_ms.is_some();
                            let duration_ms = wav_duration_ms.unwrap_or_else(|| {
                                let entry = &app.config.entries[idx];
                                [
                                    entry.corte_inicial_ms.abs(),
                                    entry.consoante_ms.abs(),
                                    entry.corte_final_ms.abs(),
                                    entry.preutterance_ms.abs(),
                                    entry.overlap_ms.abs(),
                                    entry.loop_inicio_ms.unwrap_or_default().abs(),
                                    entry.loop_fim_ms.unwrap_or_default().abs(),
                                    entry.cauda_final_ms.unwrap_or_default().abs(),
                                ]
                                .into_iter()
                                .fold(1_000.0_f64, f64::max)
                            });
                            let entry = &mut app.config.entries[idx];

                                ui.label(RichText::new("Nome do Alias:").size(11.0).color(Color32::from_rgb(200, 190, 230)));
                                ui.add(egui::TextEdit::singleline(&mut entry.alias).desired_width(240.0));

                                ui.add_space(4.0);
                                ui.label(RichText::new("Arquivo WAV:").size(10.0).color(Color32::from_rgb(160, 150, 180)));
                                ui.label(RichText::new(&entry.wav_filename).size(11.0).color(Color32::WHITE));
                                if !has_waveform {
                                    ui.colored_label(
                                        visual_tokens::WARNING,
                                        "WAV indisponível; faixa dos sliders é provisória.",
                                    )
                                    .on_hover_text(
                                        "Os parâmetros continuam editáveis. Reabra o WAV para ajustar os limites à duração real da gravação.",
                                    );
                                }

                                ui.add_space(8.0);
                                ui.separator();
                                ui.add_space(6.0);

                                ui.collapsing("Análise VENUS (.venus)", |ui| {
                                let mut venus_changed = false;
                                if let Some(analysis) = app.venus_analysis.as_mut() {
                                    let voiced = analysis.frames.iter().filter(|frame| frame.voiced).count();
                                    ui.label(
                                        RichText::new(format!(
                                            "{} frames · {} vozeados · {} marcas",
                                            analysis.frames.len(),
                                            voiced,
                                            analysis.pitch_marks.len()
                                        ))
                                        .size(10.0)
                                        .color(Color32::from_rgb(160, 180, 205)),
                                    );
                                    venus_changed |= ui
                                        .add(
                                            egui::Slider::new(&mut analysis.gain_db, -24.0..=24.0)
                                                .text("Ganho por alias")
                                                .suffix(" dB"),
                                        )
                                        .changed();
                                    venus_changed |= ui
                                        .add(
                                            egui::Slider::new(
                                                &mut analysis.formant_shift_cents,
                                                -2_400.0..=2_400.0,
                                            )
                                            .text("Correção de formante")
                                            .suffix(" cents"),
                                        )
                                        .changed();
                                    venus_changed |= ui
                                        .add(
                                            egui::Slider::new(&mut analysis.breathiness, 0.0..=100.0)
                                                .text("Respiração adicional"),
                                        )
                                        .changed();
                                    venus_changed |= ui
                                        .add(
                                            egui::TextEdit::multiline(&mut analysis.notes)
                                                .hint_text("Notas para este alias")
                                                .desired_rows(2),
                                        )
                                        .changed();
                                    if ui.button("Reanalisar WAV").clicked() {
                                        if let Some((samples, sample_rate)) = app.loaded_waveform.as_ref() {
                                            let mut refreshed = crate::dsp::venus_analysis::analyze_samples(samples, *sample_rate);
                                            refreshed.gain_db = analysis.gain_db;
                                            refreshed.formant_shift_cents = analysis.formant_shift_cents;
                                            refreshed.breathiness = analysis.breathiness;
                                            refreshed.notes.clone_from(&analysis.notes);
                                            *analysis = refreshed;
                                            venus_changed = true;
                                        }
                                    }
                                } else {
                                    ui.label(
                                        RichText::new("Não foi possível criar a análise deste WAV.")
                                            .size(10.0)
                                            .color(Color32::from_rgb(255, 140, 100)),
                                    );
                                }
                                if venus_changed {
                                    if let (Some(analysis), Some(path)) = (
                                        app.venus_analysis.as_ref(),
                                        app.venus_analysis_path.as_ref(),
                                    ) {
                                        let _ = crate::dsp::venus_analysis::save(path, analysis);
                                    }
                                }
                                });

                                ui.add_space(8.0);
                                ui.separator();
                                ui.add_space(6.0);

                                ui.label(
                                    RichText::new("Parâmetros de oto.ini")
                                        .strong()
                                        .size(12.0)
                                        .color(visual_tokens::ACCENT),
                                );
                                ui.label(RichText::new("Corte Inicial (ms):").size(11.0).color(Color32::from_rgb(0, 150, 255)));
                                ui.add(egui::Slider::new(&mut entry.corte_inicial_ms, 0.0..=duration_ms).suffix(" ms"));

                                ui.add_space(8.0);
                                ui.label(RichText::new("Início Consoante (ms):").size(11.0).color(visual_tokens::SUCCESS));
                                ui.add(egui::Slider::new(&mut entry.consoante_ms, 0.0..=duration_ms).suffix(" ms"));

                                ui.add_space(8.0);
                                ui.label(RichText::new("Corte Final (ms):").size(11.0).color(Color32::from_rgb(255, 60, 60)));
                                ui.add(egui::Slider::new(&mut entry.corte_final_ms, -duration_ms..=duration_ms).suffix(" ms"));

                                ui.add_space(8.0);
                                ui.label(RichText::new("Pre-utterance (ms):").size(11.0).color(Color32::from_rgb(255, 0, 180)));
                                ui.add(egui::Slider::new(&mut entry.preutterance_ms, 0.0..=duration_ms).suffix(" ms"));

                                ui.add_space(8.0);
                                ui.label(RichText::new("Overlap (ms):").size(11.0).color(Color32::from_rgb(255, 140, 0)));
                                ui.add(egui::Slider::new(&mut entry.overlap_ms, 0.0..=duration_ms).suffix(" ms"));

                                ui.add_space(10.0);

                                ui.separator();
                                ui.add_space(6.0);

                                ui.label(RichText::new("Parte de Loop (Sustentação)").strong().size(11.0).color(visual_tokens::WARNING));
                                let mut has_loop = entry.loop_inicio_ms.is_some();
                                if ui.checkbox(&mut has_loop, "Habilitar Loop").changed() {
                                    if has_loop {
                                        entry.loop_inicio_ms = Some(entry.corte_inicial_ms + entry.consoante_ms);
                                        entry.loop_fim_ms = Some(entry.corte_inicial_ms + entry.consoante_ms + 200.0);
                                    } else {
                                        entry.loop_inicio_ms = None;
                                        entry.loop_fim_ms = None;
                                    }
                                }

                                if has_loop {
                                    let l_start = entry.loop_inicio_ms.get_or_insert(100.0);
                                    ui.label(RichText::new("Loop Início:").size(10.0).color(visual_tokens::WARNING));
                                    ui.add(egui::Slider::new(l_start, 0.0..=duration_ms).suffix(" ms"));

                                    let l_end = entry.loop_fim_ms.get_or_insert(300.0);
                                    ui.label(RichText::new("Loop Fim:").size(10.0).color(visual_tokens::WARNING));
                                    ui.add(egui::Slider::new(l_end, 0.0..=duration_ms).suffix(" ms"));
                                }

                                ui.add_space(10.0);
                                ui.separator();
                                ui.add_space(6.0);

                                ui.label(RichText::new("Cauda Final (Ditongos)").strong().size(11.0).color(Color32::from_rgb(180, 50, 255)));
                                let mut has_tail = entry.cauda_final_ms.is_some();
                                if ui.checkbox(&mut has_tail, "Habilitar Cauda Final").changed() {
                                    if has_tail {
                                        entry.cauda_final_ms = Some(entry.corte_inicial_ms + entry.consoante_ms + 100.0);
                                    } else {
                                        entry.cauda_final_ms = None;
                                    }
                                }

                                if has_tail {
                                    let tail = entry.cauda_final_ms.get_or_insert(150.0);
                                    ui.label(RichText::new("Cauda Final:").size(10.0).color(Color32::from_rgb(180, 50, 255)));
                                    ui.add(egui::Slider::new(tail, 0.0..=duration_ms).suffix(" ms"));
                                }
                        } else if app.config.entries.is_empty() {
                            ui.label(
                                RichText::new("Nenhum alias disponível neste voicebank.")
                                    .size(11.0)
                                    .color(visual_tokens::MUTED),
                            );
                        } else {
                            ui.label(
                                RichText::new("Selecione um alias para editar seus parâmetros.")
                                    .size(11.0)
                                    .color(visual_tokens::MUTED),
                            );
                        }
                    });
            });
        }
    });
}
