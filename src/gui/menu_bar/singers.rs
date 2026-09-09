use super::*;

impl KamafeuStudioApp {
    pub(super) fn menu_singers(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.language;
        ui.menu_button(lang.tr("Cantores", "Singers"), |ui| {
            if ui
                .button(lang.tr("Galeria de Cantores...", "Singers Gallery..."))
                .clicked()
            {
                self.singers_gallery_window_open = true;
                ui.close_menu();
            }
            if ui
                .button(lang.tr(
                    "Diagnóstico de Integridade do Voicebank...",
                    "Voicebank Integrity Diagnostic...",
                ))
                .clicked()
            {
                self.voicebank_diagnostic_open = true;
                ui.close_menu();
            }
            ui.separator();
            if ui
                .button(lang.tr("Carregar Voicebank Único...", "Load Single Voicebank..."))
                .clicked()
            {
                #[cfg(not(target_os = "android"))]
                if let Some(folder) = crate::dialogs::FileDialog::new().pick_folder() {
                    if let Ok(vb) = Voicebank::new(&folder) {
                        self.transport_state.status_message = if lang.is_en() {
                            format!("Voicebank Loaded: {}", vb.name)
                        } else {
                            format!("Voicebank Carregado: {}", vb.name)
                        };
                        self.transport_state.voicebank_name = vb.name.clone();
                        self.transport_state.voicebank_path = Some(vb.root_path.clone());
                        self.config.add_recent_voicebank(vb.root_path.clone());
                        self.voicebank = Some(vb);
                    }
                } else {
                    self.folder_picker_open = true;
                }
                #[cfg(target_os = "android")]
                {
                    self.folder_picker_open = true;
                }
                ui.close_menu();
            }
            if ui
                .button(lang.tr(
                    "Registrar Pasta do OpenUtau / Singers...",
                    "Register OpenUtau / Singers Folder...",
                ))
                .clicked()
            {
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
                ui.close_menu();
            }
            if ui
                .button(lang.tr("Recarregar Cantores", "Reload Singers"))
                .clicked()
            {
                self.reload_singers();
                ui.close_menu();
            }

            ui.menu_button(lang.tr("Voicebanks Recentes", "Recent Voicebanks"), |ui| {
                if self.config.recent_voicebanks.is_empty() {
                    ui.label(
                        egui::RichText::new(
                            lang.tr("Nenhum voicebank recente", "No recent voicebanks"),
                        )
                        .size(11.0)
                        .color(MelodyneTheme::TEXT_MUTED),
                    );
                } else {
                    let mut to_load: Option<PathBuf> = None;
                    for vb_path in &self.config.recent_voicebanks {
                        let label = vb_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Voicebank");
                        if ui
                            .button(egui::RichText::new(label).size(11.0))
                            .on_hover_text(vb_path.to_string_lossy())
                            .clicked()
                        {
                            to_load = Some(vb_path.clone());
                            ui.close_menu();
                        }
                    }
                    if let Some(vb_dir) = to_load {
                        if let Ok(vb) = Voicebank::new(&vb_dir) {
                            self.transport_state.status_message = if lang.is_en() {
                                format!("Voicebank Loaded: {}", vb.name)
                            } else {
                                format!("Voicebank Carregado: {}", vb.name)
                            };
                            self.transport_state.voicebank_name = vb.name.clone();
                            self.transport_state.voicebank_path = Some(vb.root_path.clone());
                            self.config.add_recent_voicebank(vb.root_path.clone());
                            self.voicebank = Some(vb);
                        }
                    }
                    ui.separator();
                    if ui
                        .button(
                            lang.tr("Limpar Histórico de Voicebanks", "Clear Voicebanks History"),
                        )
                        .clicked()
                    {
                        self.config.recent_voicebanks.clear();
                        self.persist_config();
                        ui.close_menu();
                    }
                }
            });

            ui.separator();
            if ui.button("Copaiba Voicebank Toolkit").clicked() {
                self.copaiba_window_open = true;
                ui.close_menu();
            }
        });
    }
}
