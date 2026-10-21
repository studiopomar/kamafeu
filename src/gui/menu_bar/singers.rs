use super::*;

impl KamafeuStudioApp {
    pub(super) fn menu_singers(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.language;
        ui.menu_button(lang.tr("Cantores", "Singers"), |ui| {
            let native_bank_active = self
                .project
                .tracks
                .get(self.active_track_index)
                .and_then(|track| track.native_voicebank.as_ref())
                .and_then(|reference| reference.source_path_hint.as_ref())
                .is_some();
            if ui
                .add_enabled(
                    native_bank_active,
                    egui::Button::new(lang.tr(
                        "Configurar cantor Kamafeu da faixa ativa...",
                        "Configure active track's Kamafeu singer...",
                    )),
                )
                .clicked()
            {
                self.open_native_bank_settings();
                ui.close_menu();
            }
            if !native_bank_active {
                ui.label(
                    egui::RichText::new(lang.tr(
                        "Vincule um banco nativo para editar suas marcações e ajustes.",
                        "Bind a native bank to edit its timing and settings.",
                    ))
                    .small()
                    .color(self.config.theme.text_muted_c32()),
                );
            }
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
                        self.activate_voicebank(vb);
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
                        .color(self.config.theme.text_muted_c32()),
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
                            self.activate_voicebank(vb);
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
            if ui
                .button("KAMUSE · Kamafeu Autoria e Modelagem de Unidades Sonoras Editáveis")
                .clicked()
            {
                self.open_kamuse_for_current_context();
                ui.close_menu();
            }
        });
    }
}
