use crate::gui::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(super) fn show_singers_gallery(&mut self, ctx: &egui::Context) {
        if self.singers_gallery_window_open {
            let lang = self.config.language;
            let mut is_open = self.singers_gallery_window_open;
            let mut singer_to_load: Option<std::path::PathBuf> = None;
            let mut trigger_add_dir = false;
            let mut trigger_reload = false;

            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("singers_gallery_native_viewport"),
                egui::ViewportBuilder::default()
                    .with_title(lang.tr("Galeria de Cantores (OpenUtau / UTAU) - Kamafeu Studio", "Singers Gallery (OpenUtau / UTAU) - Kamafeu Studio"))
                    .with_inner_size([760.0, 560.0])
                    .with_min_inner_size([500.0, 380.0]),
                |ctx, _class| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.heading(
                                egui::RichText::new(lang.tr("Cantores Instalados", "Installed Singers"))
                                    .strong()
                                    .size(16.0)
                                    .color(egui::Color32::from_rgb(0, 255, 180)),
                            );
                            ui.label(
                                egui::RichText::new(format!("({} {})", self.singers_list.len(), lang.tr("encontrados", "found")))
                                    .size(11.0)
                                    .color(crate::gui::theme::MelodyneTheme::TEXT_MUTED),
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button(lang.tr("Recarregar", "Reload")).clicked() {
                                    trigger_reload = true;
                                }
                                if ui.button(lang.tr("+ Registrar Pasta...", "+ Add Folder...")).clicked() {
                                    trigger_add_dir = true;
                                }
                            });
                        });

                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(lang.tr("Buscar:", "Search:"))
                                    .size(11.0)
                                    .color(crate::gui::theme::MelodyneTheme::TEXT_MUTED),
                            );
                            ui.add(
                                egui::TextEdit::singleline(&mut self.singer_search_query)
                                    .hint_text(lang.tr("Buscar por nome do cantor ou autor...", "Search by singer name or author..."))
                                    .desired_width(ui.available_width()),
                            );
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        let query_lower = self.singer_search_query.to_lowercase();
                        let filtered: Vec<_> = self
                            .singers_list
                            .iter()
                            .filter(|s| {
                                query_lower.is_empty()
                                    || s.name.to_lowercase().contains(&query_lower)
                                    || s.author.to_lowercase().contains(&query_lower)
                                    || s.voice_type.to_lowercase().contains(&query_lower)
                            })
                            .collect();

                        egui::ScrollArea::vertical()
                            .id_salt("singers_gallery_grid_scroll")
                            .show(ui, |ui| {
                                if filtered.is_empty() {
                                    ui.vertical_centered(|ui| {
                                        ui.add_space(40.0);
                                        ui.label(
                                            egui::RichText::new(lang.tr("Nenhum cantor encontrado.", "No singers found."))
                                                .size(13.0)
                                                .color(crate::gui::theme::MelodyneTheme::TEXT_MUTED),
                                        );
                                        ui.label(
                                            egui::RichText::new(
                                                lang.tr(
                                                    "Clique em '+ Registrar Pasta...' para apontar para a pasta 'Singers' do OpenUtau.",
                                                    "Click '+ Add Folder...' to point to the OpenUtau 'Singers' folder.",
                                                ),
                                            )
                                            .size(10.5)
                                            .italics()
                                            .color(egui::Color32::from_rgb(180, 190, 220)),
                                        );
                                    });
                                } else {
                                    let item_w = 210.0f32;
                                    let item_h = 100.0f32;
                                    let cols = ((ui.available_width() / (item_w + 12.0)).floor() as usize).max(1);

                                    egui::Grid::new("singers_gallery_grid")
                                        .spacing([12.0, 12.0])
                                        .show(ui, |ui| {
                                            for (idx, singer) in filtered.iter().enumerate() {
                                                let is_current = self
                                                    .voicebank
                                                    .as_ref()
                                                    .is_some_and(|v| v.root_path == singer.path);

                                                let bg_color = if is_current {
                                                    egui::Color32::from_rgb(45, 30, 68)
                                                } else {
                                                    egui::Color32::from_rgb(24, 18, 34)
                                                };
                                                let stroke_color = if is_current {
                                                    egui::Color32::from_rgb(0, 255, 180)
                                                } else {
                                                    egui::Color32::from_rgb(52, 40, 72)
                                                };

                                                egui::Frame::none()
                                                    .fill(bg_color)
                                                    .rounding(egui::Rounding::same(8.0))
                                                    .stroke(egui::Stroke::new(1.2_f32, stroke_color))
                                                    .inner_margin(egui::Margin::same(8.0))
                                                    .show(ui, |ui| {
                                                        ui.set_width(item_w);
                                                        ui.set_height(item_h);

                                                        ui.horizontal(|ui| {
                                                            let (avatar_rect, _) = ui.allocate_exact_size(
                                                                egui::Vec2::new(56.0, 56.0),
                                                                egui::Sense::hover(),
                                                            );
                                                            let painter = ui.painter_at(avatar_rect);
                                                            painter.rect_filled(
                                                                avatar_rect,
                                                                egui::Rounding::same(6.0),
                                                                egui::Color32::from_rgb(36, 26, 52),
                                                            );

                                                            let mut loaded = false;
                                                            if let Some(ref img_p) = singer.image_path {
                                                                if let Some(tex) = crate::gui::image_cache::texture_for_path(ui.ctx(), img_p) {
                                                                    painter.image(
                                                                        tex.id(),
                                                                        avatar_rect,
                                                                        egui::Rect::from_min_max(
                                                                            egui::Pos2::new(0.0, 0.0),
                                                                            egui::Pos2::new(1.0, 1.0),
                                                                        ),
                                                                        egui::Color32::WHITE,
                                                                    );
                                                                    loaded = true;
                                                                }
                                                            }

                                                            if !loaded {
                                                                let initial = singer.name.chars().next().unwrap_or('V');
                                                                painter.circle_filled(
                                                                    avatar_rect.center(),
                                                                    18.0,
                                                                    egui::Color32::from_rgb(192, 132, 252),
                                                                );
                                                                painter.text(
                                                                    avatar_rect.center(),
                                                                    egui::Align2::CENTER_CENTER,
                                                                    initial.to_string(),
                                                                    egui::FontId::proportional(16.0),
                                                                    egui::Color32::from_rgb(20, 14, 30),
                                                                );
                                                            }

                                                            ui.add_space(6.0);
                                                            ui.vertical(|ui| {
                                                                ui.label(
                                                                    egui::RichText::new(&singer.name)
                                                                        .strong()
                                                                        .size(11.5)
                                                                        .color(if is_current {
                                                                            egui::Color32::from_rgb(0, 255, 180)
                                                                        } else {
                                                                            egui::Color32::WHITE
                                                                        }),
                                                                );
                                                                ui.label(
                                                                    egui::RichText::new(&singer.author)
                                                                        .size(9.0)
                                                                        .color(crate::gui::theme::MelodyneTheme::TEXT_MUTED),
                                                                );
                                                                ui.label(
                                                                    egui::RichText::new(&singer.voice_type)
                                                                        .size(8.5)
                                                                        .color(egui::Color32::from_rgb(216, 180, 254)),
                                                                );

                                                                ui.add_space(4.0);
                                                                if is_current {
                                                                    ui.label(
                                                                        egui::RichText::new(lang.tr("[Em Uso]", "[In Use]"))
                                                                            .strong()
                                                                            .size(10.0)
                                                                            .color(egui::Color32::from_rgb(0, 255, 180)),
                                                                    );
                                                                } else if ui.small_button(lang.tr("Selecionar", "Select")).clicked() {
                                                                    singer_to_load = Some(singer.path.clone());
                                                                }
                                                            });
                                                        });
                                                    });

                                                if (idx + 1) % cols == 0 {
                                                    ui.end_row();
                                                }
                                            }
                                        });
                                }
                            });
                    });
                    if ctx.input(|i| i.viewport().close_requested()) {
                        is_open = false;
                    }
                },
            );

            if trigger_add_dir {
                if let Some(folder) = crate::dialogs::FileDialog::new().pick_folder() {
                    if !self.config.singers_paths.contains(&folder) {
                        self.config.singers_paths.push(folder);
                        self.persist_config();
                        self.reload_singers();
                    }
                }
            }

            if trigger_reload {
                self.persist_config();
                self.reload_singers();
            }

            if let Some(path) = singer_to_load {
                if let Ok(vb) = crate::oto::Voicebank::new(&path) {
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
                }
                is_open = false;
            }

            self.singers_gallery_window_open = is_open;
        }
    }
}
