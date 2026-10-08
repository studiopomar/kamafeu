use crate::gui::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(crate) fn render_voicebank_diagnostic_dialog(&mut self, ctx: &egui::Context) {
        if !self.voicebank_diagnostic_open {
            return;
        }

        let is_diffsinger = self
            .voicebank
            .as_ref()
            .is_some_and(crate::oto::Voicebank::is_diffsinger);
        let refresh_report = !is_diffsinger
            && self.voicebank.as_ref().is_some_and(|voicebank| {
                self.voicebank_diagnostic_report
                    .as_ref()
                    .is_none_or(|report| report.root_path != voicebank.root_path)
            });
        if refresh_report {
            self.voicebank_diagnostic_report = self
                .voicebank
                .as_ref()
                .map(crate::oto::Voicebank::diagnostic_report);
        }
        if is_diffsinger {
            self.voicebank_diagnostic_report = None;
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
                            if !is_diffsinger && ui.button(lang.tr("Atualizar", "Refresh")).clicked() {
                                self.voicebank_diagnostic_report = self
                                    .voicebank
                                    .as_ref()
                                    .map(crate::oto::Voicebank::diagnostic_report);
                            }
                            if ui.button(lang.tr("Fechar", "Close")).clicked() {
                                is_open = false;
                            }
                        });
                    });
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    if let Some(vb) = self.voicebank.as_ref() {
                        if vb.is_diffsinger() {
                            ui.label(egui::RichText::new(format!("{}: {}", lang.tr("Cantor", "Singer"), vb.name)).strong().size(13.0));
                            ui.label(egui::RichText::new(lang.tr("Tipo: DiffSinger (síntese neural por frase)", "Type: DiffSinger (neural phrase synthesis)")).color(egui::Color32::from_rgb(0, 220, 255)));
                            ui.add_space(8.0);
                            match vb.diffsinger_config() {
                                Ok(config) => {
                                    egui::Grid::new("diffsinger_diag_grid")
                                        .num_columns(2)
                                        .spacing([16.0, 8.0])
                                        .show(ui, |ui| {
                                            ui.label(egui::RichText::new(lang.tr("Modelo acústico:", "Acoustic model:")).strong());
                                            ui.label(config.acoustic.display().to_string());
                                            ui.end_row();
                                            ui.label(egui::RichText::new(lang.tr("Vocoder:", "Vocoder:")).strong());
                                            ui.label(config.vocoder.display().to_string());
                                            ui.end_row();
                                            ui.label(egui::RichText::new(lang.tr("Taxa / hop:", "Rate / hop:")).strong());
                                            ui.label(format!("{} Hz / {}", config.sample_rate, config.hop_size));
                                            ui.end_row();
                                            ui.label(egui::RichText::new(lang.tr("Falantes:", "Speakers:")).strong());
                                            ui.label(if config.speakers.is_empty() { "—".to_string() } else { config.speakers.join(", ") });
                                            ui.end_row();
                                            ui.label(egui::RichText::new(lang.tr("Idiomas:", "Languages:")).strong());
                                            ui.label(if config.language_ids.is_empty() { "—".to_string() } else { config.language_ids.keys().cloned().collect::<Vec<_>>().join(", ") });
                                            ui.end_row();
                                        });
                                    ui.add_space(10.0);
                                    ui.label(egui::RichText::new(lang.tr("[OK] Configuração DiffSinger encontrada. A validação completa ocorre ao carregar os modelos ONNX para renderização.", "[OK] DiffSinger configuration found. Full validation occurs when ONNX models load for rendering.")).color(egui::Color32::from_rgb(0, 255, 180)));
                                }
                                Err(error) => {
                                    ui.label(egui::RichText::new(format!("{}: {error}", lang.tr("[Erro] Configuração DiffSinger inválida", "[Error] Invalid DiffSinger configuration"))).color(egui::Color32::from_rgb(255, 100, 90)));
                                    if error.contains(".oudep") {
                                        ui.add_space(6.0);
                                        ui.label(lang.tr(
                                            "Para usar este banco, instale manualmente o vocoder DiffSinger compatível:",
                                            "To use this singer, manually install the compatible DiffSinger vocoder:",
                                        ));
                                        ui.hyperlink(crate::oto::DIFFSINGER_DEPENDENCY_URL);
                                    }
                                }
                            }
                        } else if let Some(report) = self.voicebank_diagnostic_report.as_ref() {
                        ui.label(egui::RichText::new(format!("{}: {}", lang.tr("Cantor", "Singer"), vb.name)).strong().size(13.0));
                        ui.label(egui::RichText::new(format!("{}: {}", lang.tr("Pasta", "Folder"), vb.root_path.display())).size(10.0).monospace().color(crate::gui::theme::MelodyneTheme::TEXT_MUTED));
                        ui.add_space(8.0);

                        egui::Grid::new("diag_grid")
                            .num_columns(2)
                            .spacing([16.0, 8.0])
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(lang.tr("Total de Entradas no oto.ini:", "Total oto.ini Entries:")).strong());
                                ui.label(format!("{}", report.entry_count));
                                ui.end_row();

                                ui.label(egui::RichText::new(lang.tr("WAVs Válidos:", "Valid WAVs:")).strong());
                                ui.label(egui::RichText::new(format!("{}/{}", report.readable_wav_count, report.unique_wav_count)).color(if report.is_healthy() { egui::Color32::from_rgb(0, 255, 180) } else { egui::Color32::from_rgb(255, 200, 50) }));
                                ui.end_row();

                                ui.label(egui::RichText::new(lang.tr("Arquivos WAV Ausentes:", "Missing WAV Files:")).strong());
                                ui.label(egui::RichText::new(format!("{}", report.missing_wav_count)).color(if report.missing_wav_count == 0 { egui::Color32::from_rgb(0, 255, 180) } else { egui::Color32::from_rgb(255, 80, 80) }));
                                ui.end_row();

                                ui.label(egui::RichText::new(lang.tr("WAVs Inválidos/Vazios:", "Invalid/Empty WAVs:")).strong());
                                ui.label(format!("{} / {}", report.invalid_wav_count, report.empty_wav_count));
                                ui.end_row();

                                ui.label(egui::RichText::new(lang.tr("Tempos do oto.ini suspeitos:", "Suspicious oto.ini timings:")).strong());
                                ui.label(format!("{}", report.invalid_oto_timing_count));
                                ui.end_row();

                                ui.label(egui::RichText::new(lang.tr("Taxas de Amostragem:", "Sample Rates:")).strong());
                                let rates = report.sample_rates.iter().map(|(rate, count)| format!("{rate} Hz ({count})")).collect::<Vec<_>>().join(", ");
                                ui.label(if rates.is_empty() { "—".to_string() } else { rates });
                                ui.end_row();

                                ui.label(egui::RichText::new(lang.tr("Tabela de Prefix/Suffix:", "Prefix/Suffix Table:")).strong());
                                ui.label(if vb.prefix_map.is_empty() { lang.tr("Nenhum prefix.map configurado", "No prefix.map configured") } else { lang.tr("prefix.map carregado", "prefix.map loaded") });
                                ui.end_row();
                            });

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(8.0);

                        ui.label(egui::RichText::new(lang.tr("Status de Integridade:", "Integrity Status:")).strong());
                        if report.is_healthy() {
                            ui.label(egui::RichText::new(lang.tr("[OK] Todas as amostras de áudio e configurações do oto.ini estão íntegras.", "[OK] All audio samples and oto.ini settings are intact.")).color(egui::Color32::from_rgb(0, 255, 180)));
                        } else {
                            ui.label(egui::RichText::new(format!("{}: {} {}", lang.tr("[Aviso]", "[Warning]"), report.issues.len(), lang.tr("problemas encontrados. Veja a lista abaixo.", "problems found. See the list below."))).color(egui::Color32::from_rgb(255, 120, 80)));
                            egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                                for issue in report.issues.iter().take(100) {
                                    ui.label(egui::RichText::new(format!("{} · {} · {}", issue.alias, issue.wav_filename, issue.detail)).size(10.0).color(egui::Color32::from_rgb(255, 190, 120)));
                                    ui.label(egui::RichText::new(format!("  Sugestão: {}", issue.suggestion())).size(9.0).italics().color(egui::Color32::from_rgb(180, 200, 220)));
                                }
                            });
                        }
                        } else {
                            ui.label(egui::RichText::new(lang.tr("Não foi possível gerar o relatório deste voicebank.", "Could not generate a report for this voicebank.")).italics().color(crate::gui::theme::MelodyneTheme::TEXT_MUTED));
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
