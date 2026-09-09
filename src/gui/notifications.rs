use crate::gui::KamafeuStudioApp;
use eframe::egui;
use std::path::PathBuf;

impl KamafeuStudioApp {
    pub(super) fn draw_export_notification_toast(&mut self, ctx: &egui::Context) {
        let mut dismiss = false;
        let mut reveal_path: Option<PathBuf> = None;

        if let Some((ref path, ref text, created_at)) = self.last_exported_notification {
            if created_at.elapsed().as_secs_f32() > 25.0 {
                dismiss = true;
            } else {
                egui::Area::new(egui::Id::new("export_floating_toast_area"))
                    .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-20.0, -20.0))
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(26, 22, 38))
                            .rounding(egui::Rounding::same(8.0))
                            .stroke(egui::Stroke::new(1.5, egui::Color32::from_rgb(0, 255, 180)))
                            .shadow(egui::epaint::Shadow {
                                offset: egui::Vec2::new(0.0, 4.0),
                                blur: 12.0,
                                spread: 0.0,
                                color: egui::Color32::from_black_alpha(180),
                            })
                            .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new("[OK]")
                                            .strong()
                                            .size(12.0)
                                            .color(egui::Color32::from_rgb(0, 255, 180)),
                                    );
                                    ui.vertical(|ui| {
                                        ui.label(
                                            egui::RichText::new(text)
                                                .strong()
                                                .size(12.0)
                                                .color(egui::Color32::WHITE),
                                        );
                                        ui.label(
                                            egui::RichText::new(path.to_string_lossy())
                                                .size(10.0)
                                                .color(egui::Color32::from_rgb(160, 150, 180)),
                                        );
                                    });

                                    ui.add_space(8.0);

                                    let lang = self.config.language;
                                    let reveal_label = crate::gui::dialogs::reveal_in_file_manager_label_for(lang);
                                    let reveal_btn = egui::Button::new(
                                        egui::RichText::new(reveal_label)
                                            .size(11.0)
                                            .strong()
                                            .color(egui::Color32::BLACK),
                                    )
                                    .fill(egui::Color32::from_rgb(0, 255, 180))
                                    .min_size(egui::vec2(130.0, 24.0));

                                    if ui.add(reveal_btn).clicked() {
                                        reveal_path = Some(path.clone());
                                    }

                                    let copy_btn =
                                        egui::Button::new(egui::RichText::new(lang.tr("Copiar", "Copy")).size(10.0))
                                            .fill(egui::Color32::from_rgb(45, 40, 60))
                                            .min_size(egui::vec2(45.0, 24.0));

                                    if ui
                                        .add(copy_btn)
                                        .on_hover_text(lang.tr("Copiar caminho do arquivo", "Copy file path"))
                                        .clicked()
                                    {
                                        ui.output_mut(|o| {
                                            o.copied_text = path.to_string_lossy().to_string()
                                        });
                                        self.transport_state.status_message =
                                            lang.tr("Caminho do arquivo copiado!", "File path copied!").to_string();
                                    }

                                    let close_btn = egui::Button::new(
                                        egui::RichText::new("X")
                                            .size(11.0)
                                            .color(egui::Color32::from_rgb(180, 175, 195)),
                                    )
                                    .fill(egui::Color32::from_rgb(45, 40, 60))
                                    .min_size(egui::vec2(24.0, 24.0));

                                    if ui.add(close_btn).clicked() {
                                        dismiss = true;
                                    }
                                });
                            });
                    });
            }
        }

        if let Some(p) = reveal_path {
            crate::gui::dialogs::open_file_in_folder(&p);
        }

        if dismiss {
            self.last_exported_notification = None;
        }
    }
}
