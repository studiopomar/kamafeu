use super::*;

impl KamafeuStudioApp {
    pub(super) fn menu_file(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.language;
        let is_pt = lang.is_pt();

        ui.menu_button(lang.tr("Arquivo", "File"), |ui| {
            if ui
                .button(lang.tr(
                    "Novo Projeto  (Ctrl+N / Cmd+N)",
                    "New Project  (Ctrl+N / Cmd+N)",
                ))
                .clicked()
            {
                self.new_project();
                ui.close_menu();
            }
            if ui
                .button(lang.tr("Novo a partir de Modelo...", "New from Template..."))
                .clicked()
            {
                self.templates_dialog_open = true;
                ui.close_menu();
            }
            if ui
                .button(lang.tr(
                    "Abrir Projeto...  (Ctrl+O / Cmd+O)",
                    "Open Project...  (Ctrl+O / Cmd+O)",
                ))
                .clicked()
            {
                self.open_project_dialog();
                ui.close_menu();
            }

            ui.menu_button(lang.tr("Projetos Recentes", "Recent Projects"), |ui| {
                if self.config.recent_projects.is_empty() {
                    ui.label(
                        egui::RichText::new(
                            lang.tr("Nenhum projeto recente", "No recent projects"),
                        )
                        .size(11.0)
                        .color(MelodyneTheme::TEXT_MUTED),
                    );
                } else {
                    let mut to_open: Option<PathBuf> = None;
                    for p_path in &self.config.recent_projects {
                        let label = p_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or(lang.tr("Projeto", "Project"));
                        if ui
                            .button(egui::RichText::new(label).size(11.0))
                            .on_hover_text(p_path.to_string_lossy())
                            .clicked()
                        {
                            to_open = Some(p_path.clone());
                            ui.close_menu();
                        }
                    }
                    if let Some(p) = to_open {
                        self.open_project_from_path(&p);
                    }
                    ui.separator();
                    if ui
                        .button(lang.tr("Limpar Histórico de Recentes", "Clear Recent History"))
                        .clicked()
                    {
                        self.config.recent_projects.clear();
                        self.persist_config();
                        ui.close_menu();
                    }
                }
            });

            ui.separator();
            if ui
                .button(lang.tr(
                    "Salvar Projeto  (Ctrl+S / Cmd+S)",
                    "Save Project  (Ctrl+S / Cmd+S)",
                ))
                .clicked()
            {
                self.save_project();
                ui.close_menu();
            }
            if ui
                .button(lang.tr(
                    "Salvar Como...  (Ctrl+Shift+S)",
                    "Save As...  (Ctrl+Shift+S)",
                ))
                .clicked()
            {
                self.save_project_as_dialog();
                ui.close_menu();
            }
            if ui
                .button(lang.tr(
                    "Salvar Nova Versão (Incremental)  (Ctrl+Alt+S)",
                    "Save New Version (Incremental)  (Ctrl+Alt+S)",
                ))
                .clicked()
            {
                self.save_project_incremental_version();
                ui.close_menu();
            }
            if ui
                .button(lang.tr("Salvar Cópia de Backup (.aps)", "Save Backup Copy (.aps)"))
                .clicked()
            {
                self.save_project_backup_copy();
                ui.close_menu();
            }
            if ui
                .button(lang.tr(
                    "Criar Snapshot Interno (.kamafeu_snapshots)",
                    "Create Internal Snapshot (.kamafeu_snapshots)",
                ))
                .clicked()
            {
                self.create_project_snapshot();
                ui.close_menu();
            }
            if ui
                .button(lang.tr("Propriedades do Projeto...", "Project Properties..."))
                .clicked()
            {
                self.project_properties_open = true;
                ui.close_menu();
            }
            if ui
                .button(lang.tr(
                    "Revelar no Gerenciador de Arquivos",
                    "Reveal in File Manager",
                ))
                .clicked()
            {
                self.reveal_project_in_finder();
                ui.close_menu();
            }

            ui.separator();
            ui.menu_button(lang.tr("Importar", "Import"), |ui| {
                if ui
                    .button(lang.tr(
                        "Qualquer Formato Compatível (UtaFormatix)...",
                        "Any Compatible Format (UtaFormatix)...",
                    ))
                    .clicked()
                {
                    self.open_project_dialog();
                    ui.close_menu();
                }
                ui.separator();
                if ui
                    .button(lang.tr(
                        "UtaFormatix Data (.ufdata, .json)...",
                        "UtaFormatix Data (.ufdata, .json)...",
                    ))
                    .clicked()
                {
                    self.import_ufdata_dialog();
                    ui.close_menu();
                }
                if ui
                    .button(lang.tr("Projeto OpenUTAU (.ustx)...", "OpenUTAU Project (.ustx)..."))
                    .clicked()
                {
                    self.import_ustx_dialog();
                    ui.close_menu();
                }
                if ui
                    .button(lang.tr("Sequência UTAU (.ust)...", "UTAU Sequence (.ust)..."))
                    .clicked()
                {
                    self.import_ust_dialog();
                    ui.close_menu();
                }
                if ui
                    .button(lang.tr(
                        "Projeto Synthesizer V (.svp)...",
                        "Synthesizer V Project (.svp)...",
                    ))
                    .clicked()
                {
                    self.import_svp_dialog();
                    ui.close_menu();
                }
                if ui
                    .button(lang.tr(
                        "Sequência Vocaloid (.vsqx, .vsq)...",
                        "Vocaloid Sequence (.vsqx, .vsq)...",
                    ))
                    .clicked()
                {
                    self.import_vsqx_dialog();
                    ui.close_menu();
                }
                if ui
                    .button(lang.tr(
                        "Arquivo MIDI (.mid, .midi)...",
                        "MIDI File (.mid, .midi)...",
                    ))
                    .clicked()
                {
                    self.import_midi_dialog();
                    ui.close_menu();
                }
                ui.separator();
                if ui
                    .button(lang.tr(
                        "Faixa de Áudio / Instrumental (.wav, .mp3, .ogg, .flac)...",
                        "Audio Track / Instrumental (.wav, .mp3, .ogg, .flac)...",
                    ))
                    .clicked()
                {
                    self.import_audio_track_dialog();
                    ui.close_menu();
                }
            });

            ui.menu_button(lang.tr("Exportar", "Export"), |ui| {
                if ui
                    .button(lang.tr(
                        "Exportar Áudio (WAV / FLAC / RAW)... (Ctrl+E / Cmd+E)",
                        "Export Audio (WAV / FLAC / RAW)... (Ctrl+E / Cmd+E)",
                    ))
                    .clicked()
                {
                    self.export_wav();
                    ui.close_menu();
                }
                if !self.project.wave_parts.is_empty() {
                    if ui
                        .button(lang.tr(
                            "Exportar Apenas Vocais / Acapella...",
                            "Export Vocals Only / Acapella...",
                        ))
                        .clicked()
                    {
                        self.execute_export_wav(crate::gui::types::ExportAudioScope::VocalsOnly);
                        ui.close_menu();
                    }
                    if ui
                        .button(lang.tr(
                            "Exportar Mix Completa com Áudios...",
                            "Export Full Mix with Audio...",
                        ))
                        .clicked()
                    {
                        self.execute_export_wav(
                            crate::gui::types::ExportAudioScope::VocalsAndAudio,
                        );
                        ui.close_menu();
                    }
                }
                ui.menu_button(lang.tr("Formato de Áudio", "Audio Format"), |ui| {
                    let formats = [
                        crate::renderer::AudioExportFormat::Wav16,
                        crate::renderer::AudioExportFormat::Wav24,
                        crate::renderer::AudioExportFormat::Wav32Float,
                        crate::renderer::AudioExportFormat::Flac16,
                        crate::renderer::AudioExportFormat::Flac24,
                        crate::renderer::AudioExportFormat::RawF32,
                    ];
                    for fmt in formats {
                        let is_active = self.export_audio_format == fmt;
                        if ui.selectable_label(is_active, fmt.display_name()).clicked() {
                            self.export_audio_format = fmt;
                            self.transport_state.status_message = if is_pt {
                                format!("Formato de exportação: {}", fmt.display_name())
                            } else {
                                format!("Export format: {}", fmt.display_name())
                            };
                            ui.close_menu();
                        }
                    }
                });
                ui.separator();
                if ui
                    .button(lang.tr(
                        "UtaFormatix Data (*.ufdata)...",
                        "UtaFormatix Data (*.ufdata)...",
                    ))
                    .clicked()
                {
                    self.export_ufdata_dialog();
                    ui.close_menu();
                }
                if ui
                    .button(lang.tr(
                        "Projeto OpenUTAU (*.ustx)...",
                        "OpenUTAU Project (*.ustx)...",
                    ))
                    .clicked()
                {
                    self.export_ustx_dialog();
                    ui.close_menu();
                }
                if ui
                    .button(lang.tr("Sequência UTAU (*.ust)...", "UTAU Sequence (*.ust)..."))
                    .clicked()
                {
                    self.export_ust_dialog();
                    ui.close_menu();
                }
                if ui
                    .button(lang.tr(
                        "Projeto Synthesizer V (*.svp)...",
                        "Synthesizer V Project (*.svp)...",
                    ))
                    .clicked()
                {
                    self.export_svp_dialog();
                    ui.close_menu();
                }
                if ui
                    .button(lang.tr(
                        "Sequência Vocaloid (*.vsqx)...",
                        "Vocaloid Sequence (*.vsqx)...",
                    ))
                    .clicked()
                {
                    self.export_vsqx_dialog();
                    ui.close_menu();
                }
                if ui
                    .button(lang.tr("Arquivo MIDI (*.mid)...", "MIDI File (*.mid)..."))
                    .clicked()
                {
                    self.export_midi_dialog();
                    ui.close_menu();
                }
            });

            ui.separator();
            if ui
                .button(lang.tr(
                    "Preferências...  (Ctrl+, / Cmd+,)",
                    "Preferences...  (Ctrl+, / Cmd+,)",
                ))
                .clicked()
            {
                self.preferences_window_open = true;
                ui.close_menu();
            }
            ui.separator();
            if ui.button(lang.tr("Fechar", "Close")).clicked() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }
}
