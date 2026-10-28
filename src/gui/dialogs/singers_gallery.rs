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
            let mut trigger_install_kfv2 = false;

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
                                    .color(self.config.theme.success_c32()),
                            );
                            ui.label(
                                egui::RichText::new(format!("({} {})", self.singers_list.len(), lang.tr("encontrados", "found")))
                                    .size(11.0)
                                    .color(self.config.theme.text_muted_c32()),
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button(lang.tr("Recarregar", "Reload")).clicked() {
                                    trigger_reload = true;
                                }
                                if ui.button(lang.tr("+ Registrar Pasta...", "+ Add Folder...")).clicked() {
                                    trigger_add_dir = true;
                                }
                                if ui.button("Instalar KFV 2...").clicked() {
                                    trigger_install_kfv2 = true;
                                }
                            });
                        });

                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(lang.tr("Buscar:", "Search:"))
                                    .size(11.0)
                                    .color(self.config.theme.text_muted_c32()),
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
                                                .color(self.config.theme.text_muted_c32()),
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
                                            .color(ui.visuals().text_color().linear_multiply(0.72)),
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
                                                let active_track = self.project.tracks.get(self.active_track_index);
                                                let is_current = if singer.is_native_kfv2 {
                                                    active_track
                                                        .and_then(|track| track.native_voicebank.as_ref())
                                                        .and_then(|reference| reference.source_path_hint.as_deref())
                                                        .is_some_and(|path| {
                                                            let p = std::path::Path::new(path);
                                                            p == singer.path
                                                                || std::fs::canonicalize(p).ok()
                                                                    == std::fs::canonicalize(&singer.path).ok()
                                                        })
                                                } else {
                                                    active_track
                                                        .and_then(|track| track.voicebank_path.as_deref())
                                                        .map(std::path::Path::new)
                                                        .is_some_and(|path| {
                                                            path == singer.path
                                                                || std::fs::canonicalize(path).ok()
                                                                    == std::fs::canonicalize(&singer.path).ok()
                                                        })
                                                        || (active_track
                                                            .and_then(|track| track.voicebank_path.as_deref())
                                                            .is_none()
                                                            && self.voicebank.as_ref().is_some_and(|v| {
                                                                v.root_path == singer.path
                                                                    || std::fs::canonicalize(&v.root_path).ok()
                                                                        == std::fs::canonicalize(&singer.path).ok()
                                                            }))
                                                };

                                                let bg_color = if is_current {
                                                    ui.visuals().hyperlink_color
                                                } else {
                                                    ui.visuals().panel_fill
                                                };
                                                let stroke_color = if is_current {
                                                    self.config.theme.success_c32()
                                                } else {
                                                    ui.visuals().hyperlink_color
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
                                                                ui.visuals().panel_fill,
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
                                                                        ui.visuals().text_color(),
                                                                    );
                                                                    loaded = true;
                                                                }
                                                            }

                                                            if !loaded {
                                                                let initial = singer.name.chars().next().unwrap_or('V');
                                                                painter.circle_filled(
                                                                    avatar_rect.center(),
                                                                    18.0,
                                                                    ui.visuals().hyperlink_color,
                                                                );
                                                                painter.text(
                                                                    avatar_rect.center(),
                                                                    egui::Align2::CENTER_CENTER,
                                                                    initial.to_string(),
                                                                    egui::FontId::proportional(16.0),
                                                                    ui.visuals().panel_fill,
                                                                );
                                                            }

                                                            ui.add_space(6.0);
                                                            ui.vertical(|ui| {
                                                                ui.label(
                                                                    egui::RichText::new(&singer.name)
                                                                        .strong()
                                                                        .size(11.5)
                                                                        .color(if is_current {
                                                                            self.config.theme.success_c32()
                                                                        } else {
                                                                            ui.visuals().text_color()
                                                                        }),
                                                                );
                                                                ui.label(
                                                                    egui::RichText::new(&singer.author)
                                                                        .size(9.0)
                                                                        .color(self.config.theme.text_muted_c32()),
                                                                );
                                                                let format_label = ui.label(
                                                                    egui::RichText::new(&singer.voice_type)
                                                                        .size(8.5)
                                                                        .color(ui.visuals().hyperlink_color),
                                                                );
                                                                if let Some(diagnostic) = &singer.diagnostic {
                                                                    format_label.on_hover_text(diagnostic);
                                                                }

                                                                if let Some(web) = singer.web.as_deref() {
                                                                    if ui.small_button(lang.tr("Site", "Website")).clicked() {
                                                                        ui.ctx().open_url(egui::OpenUrl {
                                                                            url: web.to_string(),
                                                                            new_tab: true,
                                                                        });
                                                                    }
                                                                }

                                                                ui.add_space(4.0);
                                                                if is_current {
                                                                    ui.label(
                                                                        egui::RichText::new(lang.tr("[Em Uso]", "[In Use]"))
                                                                            .strong()
                                                                            .size(10.0)
                                                                            .color(self.config.theme.success_c32()),
                                                                    );
                                                                } else if singer.selectable && ui.small_button(lang.tr("Selecionar", "Select")).clicked() {
                                                                    singer_to_load = Some(singer.path.clone());
                                                                } else if !singer.selectable {
                                                                    ui.add_enabled(false, egui::Button::new(lang.tr("Formato indisponível", "Format unavailable")));
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

            if trigger_install_kfv2 {
                if let Some(package) = crate::dialogs::FileDialog::new()
                    .add_filter("Kamafeu Voicebank (*.kfv)", &["kfv"])
                    .pick_file()
                {
                    if let Some(parent) = crate::dialogs::FileDialog::new().pick_folder() {
                        let install_result = (|| {
                            let summary =
                                crate::voicebank::package::inspect_package_metadata(&package)?;
                            let safe_component: String =
                                format!("{}_{}", summary.bank_id, summary.bank_version)
                                    .chars()
                                    .map(|character| {
                                        if character.is_ascii_alphanumeric()
                                            || character == '-'
                                            || character == '_'
                                        {
                                            character
                                        } else {
                                            '_'
                                        }
                                    })
                                    .collect();
                            let destination = parent.join(safe_component);
                            let report = crate::voicebank::package::install_package_atomic(
                                &package,
                                &destination,
                            )?;
                            Ok::<_, String>((summary, report))
                        })();
                        match install_result {
                            Ok((summary, report)) => {
                                if !self.config.singers_paths.contains(&parent) {
                                    self.config.singers_paths.push(parent);
                                    self.persist_config();
                                }
                                self.reload_singers();
                                self.transport_state.status_message = format!(
                                    "KFV 2 instalado: {} · {} · sha256 {} · catálogo atualizado; selecione-o para vincular à faixa ativa (preview SOLA experimental)",
                                    summary.display_name,
                                    report.bank_version,
                                    report.content_sha256.trim_start_matches("sha256:")
                                );
                            }
                            Err(error) => {
                                self.transport_state.status_message =
                                    format!("Falha ao instalar KFV 2: {error}");
                            }
                        }
                    }
                }
            }

            if let Some(path) = singer_to_load {
                self.activate_singer_path(&path);
                is_open = false;
            }

            self.singers_gallery_window_open = is_open;
        }
    }
}
