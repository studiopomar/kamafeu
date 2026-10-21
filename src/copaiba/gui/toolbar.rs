use super::*;

pub(super) fn draw_toolbar(
    app: &mut CopaibaToolkitApp,
    audio_player: &mut AudioPlayer,
    ui: &mut egui::Ui,
) {
    egui::Frame::none()
        .fill(visual_tokens::PANEL)
        .stroke(Stroke::new(1.0_f32, visual_tokens::BORDER))
        .rounding(Rounding::same(10.0))
        .inner_margin(egui::Margin::symmetric(16.0, 12.0))
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.vertical(|ui| {
                    ui.heading(
                        RichText::new("KAMUSE")
                            .strong()
                            .size(22.0)
                            .color(visual_tokens::ACCENT),
                    );
                    ui.label(
                        RichText::new("NATIVE SINGER AUTHORING · KAMAFEU")
                            .size(10.5)
                            .strong()
                            .color(visual_tokens::MUTED),
                    );
                });
                ui.add_space(10.0);

                if app.current_dir.is_some() {
                    if ui.button("Abrir pasta").clicked() {
                        if let Some(folder) = crate::dialogs::FileDialog::new().pick_folder() {
                            app.open_dir(folder);
                        }
                    }

                    if ui.button("Abrir pacote .kfv").clicked() {
                        if let Some(package) = crate::dialogs::FileDialog::new()
                            .add_filter("Kamafeu Voicebank (*.kfv)", &["kfv"])
                            .pick_file()
                        {
                            app.open_dir(package);
                        }
                    }
                }

                if app.current_dir.is_some()
                    && app.kamuse_project.is_none()
                    && app.native_project.is_none()
                    && ui.button("Criar cantor nativo").clicked()
                {
                    app.native_create_wizard_open = !app.native_create_wizard_open;
                }

                if app.kamuse_project.is_some() {
                    if ui.button("Salvar cantor.toml").clicked() {
                        app.status_message = match app.save_config() {
                            Ok(()) => "cantor.toml salvo".into(),
                            Err(error) => format!("Falha ao salvar: {error}"),
                        };
                    }
                    if ui.button("Empacotar .kfv").clicked() {
                        let default_name = app
                            .kamuse_project
                            .as_ref()
                            .map(|project| format!("{}.kfv", project.cantor.id.replace('.', "_")))
                            .unwrap_or_else(|| "cantor.kfv".into());
                        if let Some(path) = crate::dialogs::FileDialog::new()
                            .set_file_name(&default_name)
                            .add_filter("Kamafeu Singer (*.kfv)", &["kfv"])
                            .save_file()
                        {
                            app.status_message =
                                match app.kamuse_project.as_ref().unwrap().package(&path) {
                                    Ok(()) => format!("Banco empacotado: {}", path.display()),
                                    Err(error) => format!("Falha ao empacotar: {error}"),
                                };
                        }
                    }
                    if ui.button("Recarregar").clicked() {
                        app.reload_current();
                    }
                }

                if app.native_project.is_some() {
                    if ui.button("Salvar KFV 2").clicked() {
                        app.status_message = match app.save_config() {
                            Ok(()) => "Marcas KFV 2 salvas".into(),
                            Err(error) => format!("Falha ao salvar KFV 2: {error}"),
                        };
                    }
                    if ui.button("Recarregar").clicked() {
                        app.reload_current();
                    }
                    if ui.button("Desfazer").clicked() {
                        let result = app.native_project.as_mut().unwrap().undo();
                        app.status_message = match result {
                            Ok(true) => {
                                app.load_selected_wav_samples();
                                "Edição desfeita".into()
                            }
                            Ok(false) => "Não há edição para desfazer".into(),
                            Err(error) => format!("Falha ao desfazer: {error}"),
                        };
                    }
                    if ui.button("Refazer").clicked() {
                        let result = app.native_project.as_mut().unwrap().redo();
                        app.status_message = match result {
                            Ok(true) => {
                                app.load_selected_wav_samples();
                                "Edição refeita".into()
                            }
                            Ok(false) => "Não há edição para refazer".into(),
                            Err(error) => format!("Falha ao refazer: {error}"),
                        };
                    }
                    if ui.button("Importar WAV").clicked() {
                        if let Some(source) = crate::dialogs::FileDialog::new()
                            .add_filter("Áudio WAV (*.wav)", &["wav"])
                            .pick_file()
                        {
                            let stem = source
                                .file_stem()
                                .and_then(|name| name.to_str())
                                .unwrap_or("take");
                            let asset_id: String = stem
                                .chars()
                                .map(|ch| {
                                    if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                                        ch
                                    } else {
                                        '-'
                                    }
                                })
                                .collect();
                            if let Err(error) = app.import_native_recording(source, &asset_id) {
                                app.status_message = error;
                            }
                        }
                    }
                    if ui.button("Ouvir unidade").clicked() {
                        app.status_message = match app.preview_native_unit(audio_player) {
                            Ok(()) => app.status_message.clone(),
                            Err(error) => format!("Preview experimental: {error}"),
                        };
                    }
                    if ui.button("Empacotar .kfv").clicked() {
                        let default_name = app
                            .native_project
                            .as_ref()
                            .map(|project| {
                                format!("{}.kfv", project.manifest.bank_id.replace('.', "_"))
                            })
                            .unwrap_or_else(|| "voicebank.kfv".into());
                        if let Some(destination) = crate::dialogs::FileDialog::new()
                            .set_file_name(&default_name)
                            .add_filter("Kamafeu Voicebank (*.kfv)", &["kfv"])
                            .save_file()
                        {
                            app.status_message = match app.package_native_draft(&destination) {
                                Ok(()) => {
                                    format!("Rascunho KFV 2 verificado: {}", destination.display())
                                }
                                Err(error) => format!("Falha ao empacotar rascunho: {error}"),
                            };
                        }
                    }
                } else if app.current_dir.is_some() && app.kamuse_project.is_none() {
                    if ui.button("Salvar").clicked() {
                        app.status_message = match app.save_config() {
                            Ok(()) => "Configurações salvas!".to_string(),
                            Err(err) => format!("Erro ao salvar: {err}"),
                        };
                    }

                    if ui.button("Recarregar").clicked() {
                        app.reload_current();
                    }

                    if ui.button("Duplicar alias").clicked() {
                        app.duplicate_selected_entry();
                    }

                    if app.selected_entry_index.is_some() && ui.button("Excluir alias").clicked() {
                        app.delete_selected_entry();
                    }
                }

                if let Some(path) = &app.current_dir {
                    ui.separator();
                    let bank_name = app
                        .kamuse_project
                        .as_ref()
                        .map(|project| project.cantor.nome.as_str())
                        .or_else(|| {
                            app.native_project
                                .as_ref()
                                .map(|project| project.manifest.display_name.as_str())
                        })
                        .unwrap_or(app.config.voicebank_name.as_str());
                    let folder_name = path
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_else(|| path.display().to_string());
                    ui.vertical(|ui| {
                        ui.add_sized(
                            Vec2::new(240.0, 20.0),
                            egui::Label::new(
                                RichText::new(bank_name).strong().color(visual_tokens::TEXT),
                            )
                            .truncate(),
                        )
                        .on_hover_text(bank_name);
                        ui.add_sized(
                            Vec2::new(240.0, 18.0),
                            egui::Label::new(
                                RichText::new(folder_name)
                                    .size(10.5)
                                    .color(visual_tokens::MUTED),
                            )
                            .truncate(),
                        )
                        .on_hover_text(path.display().to_string());
                    });
                }

                if !app.status_message.is_empty() {
                    let status_color = if app.status_message.contains("Falha")
                        || app.status_message.contains("Erro")
                    {
                        visual_tokens::ERROR
                    } else {
                        visual_tokens::SUCCESS
                    };
                    let summary: String = app
                        .status_message
                        .lines()
                        .next()
                        .unwrap_or_default()
                        .chars()
                        .take(100)
                        .collect();
                    ui.colored_label(status_color, summary)
                        .on_hover_text(&app.status_message);
                }
            });
        });
}
