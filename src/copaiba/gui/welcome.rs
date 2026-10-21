use super::*;

pub(super) fn draw_empty_state(app: &mut CopaibaToolkitApp, ui: &mut egui::Ui) {
    ui.centered_and_justified(|ui| {
        ui.vertical_centered(|ui| {
            egui::Frame::none()
                .fill(visual_tokens::PANEL)
                .stroke(Stroke::new(1.0_f32, visual_tokens::BORDER))
                .rounding(Rounding::same(18.0))
                .inner_margin(egui::Margin::same(32.0))
                .show(ui, |ui| {
                    ui.set_max_width(640.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("STUDIO POMAR  /  KAMAFEU")
                                .strong()
                                .size(11.0)
                                .color(visual_tokens::ACCENT),
                        );
                        ui.add_space(14.0);
                        ui.heading(
                            RichText::new("Seus voicebanks,\ncom espaço para respirar.")
                                .strong()
                                .size(30.0)
                                .color(visual_tokens::TEXT),
                        );
                        ui.add_space(10.0);
                        ui.label(
                            RichText::new(
                                "Crie um cantor nativo, importe gravações e revise as marcações no KAMUSE.",
                            )
                            .size(14.0)
                            .color(visual_tokens::MUTED),
                        );
                        ui.add_space(24.0);
                        if ui
                            .add_sized(
                                Vec2::new(280.0, 46.0),
                                egui::Button::new(
                                    RichText::new("Abrir pasta de voicebank")
                                        .strong()
                                        .color(visual_tokens::BACKGROUND),
                                )
                                .fill(visual_tokens::ACCENT)
                                .stroke(Stroke::new(1.0_f32, visual_tokens::ACCENT_HOVER))
                                .rounding(Rounding::same(10.0)),
                            )
                            .clicked()
                        {
                            if let Some(folder) = crate::dialogs::FileDialog::new().pick_folder() {
                                app.open_dir(folder);
                            }
                        }
                        ui.add_space(10.0);
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Abrir pacote .kfv").clicked() {
                                if let Some(package) = crate::dialogs::FileDialog::new()
                                    .add_filter("Kamafeu Voicebank (*.kfv)", &["kfv"])
                                    .pick_file()
                                {
                                    app.open_dir(package);
                                }
                            }
                            if ui.button("Criar cantor com KAMUSE").clicked() {
                                app.native_create_wizard_open = true;
                            }
                        });
                        if !app.status_message.is_empty() {
                            ui.add_space(12.0);
                            ui.colored_label(visual_tokens::WARNING, &app.status_message);
                        }
                    });
                });
            if app.native_create_wizard_open {
                ui.add_space(16.0);
                wizard::draw_native_create_wizard(app, ui);
            }
        });
    });
}
