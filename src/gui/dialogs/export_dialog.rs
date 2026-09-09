use super::{open_file_in_folder, reveal_in_file_manager_label_for};
use crate::gui::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(super) fn show_export_dialog(&mut self, ctx: &egui::Context) {
        if self.export_dialog_open {
            let lang = self.config.language;
            let mut is_open = self.export_dialog_open;
            let export_finished = !self.export_in_progress && self.export_result.is_some();
            let has_error = matches!(&self.export_result, Some(Err(_)));

            let title = if export_finished {
                if has_error {
                    lang.tr(
                        "Falha na Exportação de Áudio - Kamafeu Studio",
                        "Audio Export Failed - Kamafeu Studio",
                    )
                } else {
                    lang.tr(
                        "Exportação de Áudio Concluída - Kamafeu Studio",
                        "Audio Export Completed - Kamafeu Studio",
                    )
                }
            } else {
                lang.tr(
                    "Exportando Áudio... - Kamafeu Studio",
                    "Exporting Audio... - Kamafeu Studio",
                )
            };

            let mut trigger_close = false;

            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("export_progress_native_viewport"),
                egui::ViewportBuilder::default()
                    .with_title(title)
                    .with_inner_size([580.0, 310.0])
                    .with_min_inner_size([480.0, 240.0]),
                |ctx, _class| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.add_space(6.0);

                        let filename_text = self
                            .export_save_path
                            .as_ref()
                            .and_then(|p| p.file_name())
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| "output.wav".to_string());

                        ui.horizontal(|ui| {
                            let status_badge = if export_finished {
                                if has_error {
                                    lang.tr("[ERRO]", "[ERROR]")
                                } else {
                                    lang.tr("[CONCLUÍDO]", "[COMPLETED]")
                                }
                            } else {
                                lang.tr("[EXPORTANDO]", "[EXPORTING]")
                            };
                            let badge_color = if has_error {
                                egui::Color32::from_rgb(255, 90, 90)
                            } else if export_finished {
                                egui::Color32::from_rgb(0, 255, 180)
                            } else {
                                egui::Color32::from_rgb(0, 220, 255)
                            };

                            ui.label(
                                egui::RichText::new(status_badge)
                                    .strong()
                                    .size(13.0)
                                    .color(badge_color),
                            );
                            ui.vertical(|ui| {
                                ui.heading(
                                    egui::RichText::new(&filename_text)
                                        .strong()
                                        .size(15.0)
                                        .color(badge_color),
                                );
                            });
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        let pct = (self.export_progress * 100.0).clamp(0.0, 100.0);
                        let bar_color = if has_error {
                            egui::Color32::from_rgb(255, 80, 80)
                        } else if export_finished {
                            egui::Color32::from_rgb(0, 230, 140)
                        } else {
                            egui::Color32::from_rgb(0, 215, 255)
                        };

                        let progress_bar = egui::ProgressBar::new(self.export_progress)
                            .show_percentage()
                            .text(format!("{:.1}%", pct))
                            .fill(bar_color)
                            .animate(self.export_in_progress);

                        ui.add_sized([ui.available_width(), 22.0], progress_bar);

                        ui.add_space(8.0);

                        let status_color = if has_error {
                            egui::Color32::from_rgb(255, 120, 120)
                        } else if export_finished {
                            egui::Color32::from_rgb(180, 255, 200)
                        } else {
                            egui::Color32::from_rgb(200, 200, 230)
                        };

                        let display_msg = if let Some(Err(ref err)) = self.export_result {
                            format!("{}: {}", lang.tr("Erro", "Error"), err)
                        } else if export_finished {
                            lang.tr(
                                "Renderização concluída e gravada em disco com sucesso!",
                                "Rendering completed and successfully written to disk!",
                            )
                            .to_string()
                        } else if !self.export_status_detail.is_empty() {
                            self.export_status_detail.clone()
                        } else {
                            lang.tr(
                                "Processando amostras de áudio...",
                                "Processing audio samples...",
                            )
                            .to_string()
                        };

                        ui.label(
                            egui::RichText::new(display_msg)
                                .size(11.5)
                                .color(status_color),
                        );

                        if let Some(ref path) = self.export_save_path {
                            ui.add_space(8.0);
                            egui::Frame::none()
                                .fill(egui::Color32::from_rgb(22, 19, 32))
                                .rounding(egui::Rounding::same(6.0))
                                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(55, 48, 75)))
                                .inner_margin(egui::Margin::same(8.0))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new(format!(
                                                "{}:",
                                                lang.tr("Destino", "Destination")
                                            ))
                                            .strong()
                                            .size(11.0)
                                            .color(egui::Color32::from_rgb(180, 175, 200)),
                                        );
                                        ui.label(
                                            egui::RichText::new(path.to_string_lossy())
                                                .size(11.0)
                                                .color(egui::Color32::from_rgb(230, 225, 245)),
                                        );
                                    });

                                    if export_finished && !has_error {
                                        ui.add_space(4.0);
                                        ui.horizontal(|ui| {
                                            let reveal_label =
                                                reveal_in_file_manager_label_for(lang);
                                            let reveal_btn = egui::Button::new(
                                                egui::RichText::new(reveal_label)
                                                    .size(11.5)
                                                    .strong()
                                                    .color(egui::Color32::BLACK),
                                            )
                                            .fill(egui::Color32::from_rgb(0, 255, 180))
                                            .min_size(egui::vec2(160.0, 24.0));

                                            if ui.add(reveal_btn).clicked() {
                                                open_file_in_folder(path);
                                            }

                                            let copy_btn = egui::Button::new(
                                                egui::RichText::new(
                                                    lang.tr("Copiar Caminho", "Copy Path"),
                                                )
                                                .size(11.5),
                                            )
                                            .fill(egui::Color32::from_rgb(45, 40, 60))
                                            .min_size(egui::vec2(120.0, 24.0));

                                            if ui.add(copy_btn).clicked() {
                                                ui.output_mut(|o| {
                                                    o.copied_text =
                                                        path.to_string_lossy().to_string()
                                                });
                                                self.transport_state.status_message = lang
                                                    .tr(
                                                        "Caminho do arquivo copiado!",
                                                        "File path copied!",
                                                    )
                                                    .to_string();
                                            }
                                        });
                                    }
                                });
                        }

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            if let Some(ref path) = self.export_save_path {
                                if export_finished && !has_error {
                                    let reveal_label = reveal_in_file_manager_label_for(lang);
                                    let open_folder_btn = egui::Button::new(
                                        egui::RichText::new(reveal_label).size(12.0),
                                    )
                                    .min_size(egui::vec2(150.0, 28.0));

                                    if ui.add(open_folder_btn).clicked() {
                                        open_file_in_folder(path);
                                    }
                                }
                            }

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let btn_label = if export_finished {
                                        lang.tr("Concluir", "Finish")
                                    } else {
                                        lang.tr("Ocultar em Segundo Plano", "Hide in Background")
                                    };

                                    let close_btn = egui::Button::new(
                                        egui::RichText::new(btn_label).strong().size(12.0).color(
                                            if export_finished && !has_error {
                                                egui::Color32::BLACK
                                            } else {
                                                egui::Color32::WHITE
                                            },
                                        ),
                                    )
                                    .fill(if export_finished && !has_error {
                                        egui::Color32::from_rgb(0, 255, 180)
                                    } else {
                                        egui::Color32::from_rgb(60, 50, 80)
                                    })
                                    .min_size(egui::vec2(100.0, 28.0));

                                    if ui.add(close_btn).clicked() {
                                        trigger_close = true;
                                    }
                                },
                            );
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

            self.export_dialog_open = is_open;
        }
    }
}
