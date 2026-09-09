use crate::gui::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(super) fn show_folder_picker(&mut self, ctx: &egui::Context) {
        if self.folder_picker_open {
            let lang = self.config.language;
            let mut is_open = self.folder_picker_open;
            let mut selected_dir: Option<std::path::PathBuf> = None;
            let mut trigger_close = false;

            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("folder_picker_native_viewport"),
                egui::ViewportBuilder::default()
                    .with_title(lang.tr("Escolher Pasta do Cantor / Voicebank - Kamafeu Studio", "Choose Singer / Voicebank Folder - Kamafeu Studio"))
                    .with_inner_size([680.0, 460.0])
                    .with_min_inner_size([500.0, 340.0]),
                |ctx, _class| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);

                        ui.horizontal(|ui| {
                            ui.heading(
                                egui::RichText::new(lang.tr("Explorador de Pastas (Android / Local)", "Folder Explorer (Android / Local)"))
                                    .strong()
                                    .color(egui::Color32::from_rgb(0, 255, 180)),
                            );
                        });
                        ui.add_space(4.0);

                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                egui::RichText::new(lang.tr("Atalhos:", "Shortcuts:"))
                                    .strong()
                                    .color(egui::Color32::from_rgb(255, 215, 0)),
                            );
                            for (label, path_str) in [
                                (lang.tr("Downloads", "Downloads"), "/sdcard/Download"),
                                (lang.tr("Músicas", "Music"), "/sdcard/Music"),
                                (lang.tr("Documentos", "Documents"), "/sdcard/Documents"),
                                ("OpenUtau/Singers", "/sdcard/OpenUtau/Singers"),
                                ("/sdcard", "/sdcard"),
                                (lang.tr("Início", "Home"), "."),
                            ] {
                                let p = std::path::PathBuf::from(path_str);
                                if p.exists() {
                                    if ui.small_button(label).clicked() {
                                        self.folder_picker_current_dir = p;
                                    }
                                }
                            }
                        });
                        ui.separator();

                        ui.horizontal(|ui| {
                            if let Some(parent) = self.folder_picker_current_dir.parent() {
                                if ui.button(lang.tr("Subir Nível", "Up One Level")).clicked() {
                                    self.folder_picker_current_dir = parent.to_path_buf();
                                }
                            }
                            ui.label(
                                egui::RichText::new(self.folder_picker_current_dir.display().to_string())
                                    .monospace()
                                    .color(egui::Color32::from_rgb(200, 220, 255)),
                            );
                        });

                        let has_oto = self.folder_picker_current_dir.join("oto.ini").is_file()
                            || self.folder_picker_current_dir.join("character.txt").is_file();
                        if has_oto {
                            ui.colored_label(
                                egui::Color32::from_rgb(0, 255, 150),
                                lang.tr(
                                    "[Voicebank] Voicebank detectado nesta pasta (oto.ini / character.txt encontrado)!",
                                    "[Voicebank] Voicebank detected in this folder (oto.ini / character.txt found)!",
                                ),
                            );
                        }

                        ui.add_space(4.0);
                        egui::ScrollArea::vertical()
                            .id_salt("folder_picker_entries_scroll")
                            .max_height(240.0)
                            .show(ui, |ui| {
                                if let Ok(entries) = std::fs::read_dir(&self.folder_picker_current_dir) {
                                    let mut dirs = Vec::new();
                                    for entry in entries.flatten() {
                                        if let Ok(file_type) = entry.file_type() {
                                            if file_type.is_dir() {
                                                dirs.push(entry.path());
                                            }
                                        }
                                    }
                                    dirs.sort();
                                    if dirs.is_empty() {
                                        ui.label(
                                            egui::RichText::new(lang.tr("(Nenhuma subpasta encontrada aqui)", "(No subfolders found here)")).italics(),
                                        );
                                    } else {
                                        for dir in dirs {
                                            let name = dir
                                                .file_name()
                                                .and_then(|n| n.to_str())
                                                .unwrap_or("?");
                                            let is_vb = dir.join("oto.ini").is_file()
                                                || dir.join("character.txt").is_file();
                                            let prefix = if is_vb { "[VB]" } else { "[Dir]" };
                                            let text = if is_vb {
                                                format!("{prefix} {name} (Voicebank)")
                                            } else {
                                                format!("{prefix} {name}")
                                            };
                                            let color = if is_vb {
                                                egui::Color32::from_rgb(0, 255, 200)
                                            } else {
                                                egui::Color32::WHITE
                                            };
                                            if ui.button(egui::RichText::new(text).color(color)).clicked() {
                                                self.folder_picker_current_dir = dir;
                                                break;
                                            }
                                        }
                                    }
                                } else {
                                    ui.colored_label(egui::Color32::RED, lang.tr("Acesso restrito ou pasta vazia.", "Restricted access or empty folder."));
                                }
                            });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.horizontal(|ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        egui::RichText::new(lang.tr("Selecionar Esta Pasta", "Select This Folder"))
                                            .strong()
                                            .color(egui::Color32::from_rgb(0, 255, 180)),
                                    )
                                    .fill(egui::Color32::from_rgb(20, 60, 45)),
                                )
                                .clicked()
                            {
                                selected_dir = Some(self.folder_picker_current_dir.clone());
                                trigger_close = true;
                            }
                            if ui.button(lang.tr("Cancelar", "Cancel")).clicked() {
                                trigger_close = true;
                            }
                        });
                    });
                    if ctx.input(|i| i.viewport().close_requested()) {
                        trigger_close = true;
                    }
                },
            );

            if trigger_close {
                is_open = false;
            }

            self.folder_picker_open = is_open;

            if let Some(folder) = selected_dir {
                if let Ok(vb) = crate::oto::Voicebank::new(&folder) {
                    self.transport_state.status_message = format!(
                        "{}: {}",
                        lang.tr("Voicebank Carregado", "Voicebank Loaded"),
                        vb.name
                    );
                    self.transport_state.voicebank_name = vb.name.clone();
                    self.transport_state.voicebank_path = Some(vb.root_path.clone());
                    self.config.add_recent_voicebank(vb.root_path.clone());
                    self.voicebank = Some(vb);
                    self.persist_config();
                } else {
                    self.transport_state.status_message = format!(
                        "{}: {}",
                        lang.tr("Pasta adicionada", "Folder added"),
                        folder.display()
                    );
                }
                if !self.config.singers_paths.contains(&folder) {
                    self.config.singers_paths.push(folder);
                    self.persist_config();
                    self.reload_singers();
                }
            }
        }
    }
}
