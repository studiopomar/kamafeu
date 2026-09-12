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
                                    let reveal_label =
                                        crate::gui::dialogs::reveal_in_file_manager_label_for(lang);
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

                                    let copy_btn = egui::Button::new(
                                        egui::RichText::new(lang.tr("Copiar", "Copy")).size(10.0),
                                    )
                                    .fill(egui::Color32::from_rgb(45, 40, 60))
                                    .min_size(egui::vec2(45.0, 24.0));

                                    if ui
                                        .add(copy_btn)
                                        .on_hover_text(
                                            lang.tr("Copiar caminho do arquivo", "Copy file path"),
                                        )
                                        .clicked()
                                    {
                                        ui.output_mut(|o| {
                                            o.copied_text = path.to_string_lossy().to_string()
                                        });
                                        self.transport_state.status_message = lang
                                            .tr("Caminho do arquivo copiado!", "File path copied!")
                                            .to_string();
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

    pub(super) fn draw_panel_tips_bubble(&mut self, ctx: &egui::Context) {
        let Some(created_at) = self.panel_tips_created_at else {
            return;
        };

        let elapsed = created_at.elapsed().as_secs_f32();
        const TOTAL_DURATION_SECS: f32 = 10.0;
        const FADE_DURATION_SECS: f32 = 2.0;

        if elapsed >= TOTAL_DURATION_SECS {
            self.panel_tips_created_at = None;
            return;
        }

        // Request repaint while tips are alive to ensure smooth fade animation
        ctx.request_repaint();

        let alpha = if elapsed > TOTAL_DURATION_SECS - FADE_DURATION_SECS {
            ((TOTAL_DURATION_SECS - elapsed) / FADE_DURATION_SECS).clamp(0.0, 1.0)
        } else {
            (elapsed * 4.0).clamp(0.0, 1.0) // Quick 0.25s fade-in
        };

        let btn_rects: Vec<egui::Rect> = [
            egui::Id::new("panel_btn_arrangement"),
            egui::Id::new("panel_btn_drawer"),
            egui::Id::new("panel_btn_phonemes"),
            egui::Id::new("panel_btn_inspector"),
        ]
        .iter()
        .filter_map(|id| ctx.data(|d| d.get_temp(*id)))
        .collect();

        let anchor_pos = if let (Some(first), Some(last)) = (btn_rects.first(), btn_rects.last()) {
            egui::pos2((first.min.x + last.max.x) * 0.5, last.max.y + 10.0)
        } else {
            egui::pos2(ctx.screen_rect().center().x, 48.0)
        };

        let mut dismiss = false;
        let lang = self.config.language;

        egui::Area::new(egui::Id::new("panel_tips_bubble_area"))
            .fixed_pos(anchor_pos)
            .pivot(egui::Align2::CENTER_TOP)
            .order(egui::Order::Foreground)
            .interactable(true)
            .show(ctx, |ui| {
                let bg_color = egui::Color32::from_rgba_unmultiplied(22, 20, 32, (245.0 * alpha) as u8);
                let stroke_color = egui::Color32::from_rgba_unmultiplied(0, 255, 180, (230.0 * alpha) as u8);
                let text_color = egui::Color32::from_rgba_unmultiplied(240, 240, 250, (255.0 * alpha) as u8);
                let text_sec = egui::Color32::from_rgba_unmultiplied(165, 160, 190, (230.0 * alpha) as u8);
                let shortcut_color = egui::Color32::from_rgba_unmultiplied(255, 205, 75, (255.0 * alpha) as u8);
                let arrow_color = stroke_color;

                // Draw upward pointing tooltip arrow pointing directly to the toggle buttons
                let tip_width = 370.0;
                let painter = ui.painter();
                let arrow_tip = egui::pos2(anchor_pos.x, anchor_pos.y - 8.0);
                let arrow_left = egui::pos2(anchor_pos.x - 7.0, anchor_pos.y);
                let arrow_right = egui::pos2(anchor_pos.x + 7.0, anchor_pos.y);
                painter.add(egui::epaint::Shape::convex_polygon(
                    vec![arrow_tip, arrow_left, arrow_right],
                    bg_color,
                    egui::Stroke::new(1.0, arrow_color),
                ));

                egui::Frame::none()
                    .fill(bg_color)
                    .rounding(egui::Rounding::same(8.0))
                    .stroke(egui::Stroke::new(1.2, stroke_color))
                    .shadow(egui::epaint::Shadow {
                        offset: egui::Vec2::new(0.0, 4.0),
                        blur: 14.0,
                        spread: 1.0,
                        color: egui::Color32::from_black_alpha((160.0 * alpha) as u8),
                    })
                    .inner_margin(egui::Margin::symmetric(14.0, 10.0))
                    .show(ui, |ui| {
                        ui.set_max_width(tip_width);
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("💡")
                                        .size(13.0)
                                        .color(text_color),
                                );
                                ui.label(
                                    egui::RichText::new(lang.tr(
                                        "Onde estão os painéis?",
                                        "Where are the panels?",
                                    ))
                                    .strong()
                                    .size(12.0)
                                    .color(stroke_color),
                                );
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let close_btn = egui::Button::new(
                                        egui::RichText::new("✕")
                                            .size(10.0)
                                            .color(text_sec),
                                    )
                                    .fill(egui::Color32::TRANSPARENT)
                                    .min_size(egui::vec2(16.0, 16.0));
                                    if ui.add(close_btn).on_hover_text(lang.tr("Fechar dica", "Dismiss tip")).clicked() {
                                        dismiss = true;
                                    }
                                });
                            });

                            ui.add_space(3.0);

                            ui.label(
                                egui::RichText::new(lang.tr(
                                    "A interface agora é limpa e focada no Piano Roll. Clique nestes botões para abrir cada painel:",
                                    "The interface is now clean and focused on the Piano Roll. Click these buttons to open each panel:",
                                ))
                                .size(10.5)
                                .color(text_sec),
                            );

                            ui.add_space(5.0);

                            // Panel shortcut pills
                            ui.horizontal_wrapped(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);

                                let items = [
                                    (lang.tr("Fxs", "Trk"), "A", lang.tr("Faixas", "Tracks")),
                                    (lang.tr("Exp", "Exp"), "Tab", lang.tr("Expressões", "Expressions")),
                                    (lang.tr("Fon", "Pho"), "Alt+O", lang.tr("Fonemas", "Phonemes")),
                                    (lang.tr("Ins", "Ins"), "Cmd+B", lang.tr("Inspetor", "Inspector")),
                                ];

                                for (btn_name, key, desc) in items {
                                    egui::Frame::none()
                                        .fill(egui::Color32::from_rgba_unmultiplied(35, 32, 50, (230.0 * alpha) as u8))
                                        .rounding(egui::Rounding::same(4.0))
                                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgba_unmultiplied(65, 60, 85, (200.0 * alpha) as u8)))
                                        .inner_margin(egui::Margin::symmetric(6.0, 3.0))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(
                                                    egui::RichText::new(btn_name)
                                                        .strong()
                                                        .size(10.0)
                                                        .color(stroke_color),
                                                );
                                                ui.label(
                                                    egui::RichText::new(format!("({})", key))
                                                        .monospace()
                                                        .size(9.5)
                                                        .color(shortcut_color),
                                                );
                                                ui.label(
                                                    egui::RichText::new(desc)
                                                        .size(9.5)
                                                        .color(text_color),
                                                );
                                            });
                                        });
                                }
                            });

                            ui.add_space(4.0);

                            ui.horizontal(|ui| {
                                let seconds_left = (TOTAL_DURATION_SECS - elapsed).max(0.0).ceil() as u32;
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} ({}s)",
                                        lang.tr("Some automaticamente em", "Disappears in"),
                                        seconds_left
                                    ))
                                    .size(9.0)
                                    .color(text_sec),
                                );
                            });
                        });
                    });
            });

        if dismiss {
            self.panel_tips_created_at = None;
        }
    }
}
