use crate::gui::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(crate) fn render_voicebank_diagnostic_dialog(&mut self, ctx: &egui::Context) {
        if !self.voicebank_diagnostic_open {
            return;
        }

        let lang = self.config.language;
        let mut is_open = self.voicebank_diagnostic_open;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("voicebank_diagnostic_native_viewport"),
            egui::ViewportBuilder::default()
                .with_title(lang.tr("Diagnóstico de Integridade do Voicebank - Kamafeu Studio", "Voicebank Integrity Diagnostics - Kamafeu Studio"))
                .with_inner_size([600.0, 480.0])
                .with_min_inner_size([450.0, 340.0]),
            |ctx, _class| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading(
                            egui::RichText::new(lang.tr("Diagnóstico do Voicebank", "Voicebank Diagnostics"))
                                .strong()
                                .color(egui::Color32::from_rgb(0, 255, 180)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(lang.tr("Fechar", "Close")).clicked() {
                                is_open = false;
                            }
                        });
                    });
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    if let Some(ref vb) = self.voicebank {
                        ui.label(egui::RichText::new(format!("{}: {}", lang.tr("Cantor", "Singer"), vb.name)).strong().size(13.0));
                        ui.label(egui::RichText::new(format!("{}: {}", lang.tr("Pasta", "Folder"), vb.root_path.display())).size(10.0).monospace().color(crate::gui::theme::MelodyneTheme::TEXT_MUTED));
                        ui.add_space(8.0);

                        let total_entries = vb.entries.len();
                        let mut missing_wav_count = 0;
                        for entry in vb.entries.values() {
                            let wav_path = vb.root_path.join(&entry.wav_filename);
                            if !wav_path.exists() {
                                missing_wav_count += 1;
                            }
                        }

                        egui::Grid::new("diag_grid")
                            .num_columns(2)
                            .spacing([16.0, 8.0])
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(lang.tr("Total de Entradas no oto.ini:", "Total oto.ini Entries:")).strong());
                                ui.label(format!("{}", total_entries));
                                ui.end_row();

                                ui.label(egui::RichText::new(lang.tr("Arquivos WAV Encontrados:", "WAV Files Found:")).strong());
                                let ok_count = total_entries.saturating_sub(missing_wav_count);
                                ui.label(egui::RichText::new(format!("{}/{}", ok_count, total_entries)).color(if missing_wav_count == 0 { egui::Color32::from_rgb(0, 255, 180) } else { egui::Color32::from_rgb(255, 200, 50) }));
                                ui.end_row();

                                ui.label(egui::RichText::new(lang.tr("Arquivos WAV Ausentes:", "Missing WAV Files:")).strong());
                                ui.label(egui::RichText::new(format!("{}", missing_wav_count)).color(if missing_wav_count == 0 { egui::Color32::from_rgb(0, 255, 180) } else { egui::Color32::from_rgb(255, 80, 80) }));
                                ui.end_row();

                                ui.label(egui::RichText::new(lang.tr("Tabela de Prefix/Suffix:", "Prefix/Suffix Table:")).strong());
                                ui.label(if vb.prefix_map.is_empty() { lang.tr("Nenhum prefix.map configurado", "No prefix.map configured") } else { lang.tr("prefix.map carregado", "prefix.map loaded") });
                                ui.end_row();
                            });

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(8.0);

                        ui.label(egui::RichText::new(lang.tr("Status de Integridade:", "Integrity Status:")).strong());
                        if missing_wav_count == 0 {
                            ui.label(egui::RichText::new(lang.tr("[OK] Todas as amostras de áudio e configurações do oto.ini estão íntegras.", "[OK] All audio samples and oto.ini settings are intact.")).color(egui::Color32::from_rgb(0, 255, 180)));
                        } else {
                            ui.label(egui::RichText::new(format!("{}: {} {}", lang.tr("[Aviso]", "[Warning]"), missing_wav_count, lang.tr("arquivos de áudio citados no oto.ini não foram localizados na pasta.", "audio files cited in oto.ini were not found in folder."))).color(egui::Color32::from_rgb(255, 120, 80)));
                        }
                    } else {
                        ui.label(egui::RichText::new(lang.tr("Nenhum voicebank carregado no momento.", "No voicebank currently loaded.")).italics().color(crate::gui::theme::MelodyneTheme::TEXT_MUTED));
                    }
                });
                if ctx.input(|i| i.viewport().close_requested()) {
                    is_open = false;
                }
            },
        );
        self.voicebank_diagnostic_open = is_open;
    }
}
