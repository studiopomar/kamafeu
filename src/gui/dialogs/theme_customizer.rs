use crate::gui::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(crate) fn render_theme_customizer_dialog(&mut self, ctx: &egui::Context) {
        if !self.theme_customizer_open {
            return;
        }

        let lang = self.config.language;
        let mut is_open = self.theme_customizer_open;
        let mut close_requested = false;
        let mut theme_modified = false;
        let mut visuals_modified = false;

        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("theme_customizer_native_viewport"),
            egui::ViewportBuilder::default()
                .with_title(lang.tr(
                    "Personalizar Tema e Aparência - Kamafeu Studio",
                    "Customize Theme & Appearance - Kamafeu Studio",
                ))
                .with_inner_size([720.0, 590.0])
                .with_min_inner_size([580.0, 480.0]),
            |ctx, _class| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading(
                            egui::RichText::new(lang.tr("Presets de Tema", "Theme Presets"))
                                .strong()
                                .color(self.config.theme.accent_c32()),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .button(
                                    lang.tr(
                                        "Restaurar Padrão do Preset",
                                        "Restore Preset Defaults",
                                    ),
                                )
                                .on_hover_text(lang.tr(
                                    "Restaura as cores e parâmetros padrão do preset ativo",
                                    "Restores default colors and parameters of active preset",
                                ))
                                .clicked()
                            {
                                let preset = self.config.theme.preset;
                                self.config.theme.apply_preset(preset);
                                theme_modified = true;
                                visuals_modified = true;
                            }
                        });
                    });

                    ui.add_space(4.0);

                    // Presets quick-bar
                    egui::ScrollArea::horizontal()
                        .id_salt("theme_presets_scroll")
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for preset in crate::gui::theme::ThemePreset::ALL {
                                    let is_active = self.config.theme.preset == preset;
                                    let btn_text = if is_active {
                                        egui::RichText::new(format!(
                                            "{} {}",
                                            lang.tr("[Ativo]", "[Active]"),
                                            preset.display_name_for(lang)
                                        ))
                                        .strong()
                                        .color(egui::Color32::WHITE)
                                    } else {
                                        egui::RichText::new(preset.display_name_for(lang))
                                            .color(self.config.theme.text_primary_c32())
                                    };

                                    if ui
                                        .selectable_label(is_active, btn_text)
                                        .on_hover_text(preset.description())
                                        .clicked()
                                    {
                                        self.config.theme.apply_preset(preset);
                                        theme_modified = true;
                                        visuals_modified = true;
                                    }
                                }
                            });
                        });

                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);

                    // Tab Selector
                    ui.horizontal(|ui| {
                        let tabs = [
                            lang.tr("Cores Gerais", "General Colors"),
                            lang.tr("Notas & Bordas", "Notes & Borders"),
                            lang.tr("Teclado & Grid", "Keyboard & Grid"),
                            lang.tr("Pitch & Playhead", "Pitch & Playhead"),
                            lang.tr("Janelas & Painéis", "Windows & Panels"),
                        ];
                        for (i, tab_label) in tabs.iter().enumerate() {
                            let selected = self.theme_customizer_tab == i;
                            if ui.selectable_label(selected, *tab_label).clicked() {
                                self.theme_customizer_tab = i;
                            }
                        }
                    });

                    ui.add_space(8.0);

                    // Tab Content Area in a vertical scroll
                    egui::ScrollArea::vertical()
                        .id_salt("theme_customizer_tab_scroll")
                        .max_height(260.0)
                        .show(ui, |ui| {
                            match self.theme_customizer_tab {
                                0 => {
                                    // Cores Gerais
                                    egui::Grid::new("theme_general_grid")
                                        .num_columns(2)
                                        .spacing([24.0, 10.0])
                                        .show(ui, |ui| {
                                            ui.label(lang.tr(
                                                "Fundo do Piano Roll (Canvas):",
                                                "Piano Roll Background (Canvas):",
                                            ));
                                            if egui::color_picker::color_edit_button_srgb(
                                                ui,
                                                &mut self.config.theme.bg_canvas,
                                            )
                                            .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                            }
                                            ui.end_row();

                                            ui.label(lang.tr(
                                                "Fundo de Painéis e Janelas:",
                                                "Panels & Windows Background:",
                                            ));
                                            if egui::color_picker::color_edit_button_srgb(
                                                ui,
                                                &mut self.config.theme.bg_panel,
                                            )
                                            .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                                visuals_modified = true;
                                            }
                                            ui.end_row();

                                            ui.label(lang.tr(
                                                "Barra Superior & Cabeçalhos:",
                                                "Top Bar & Headers:",
                                            ));
                                            if egui::color_picker::color_edit_button_srgb(
                                                ui,
                                                &mut self.config.theme.bg_header,
                                            )
                                            .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                                visuals_modified = true;
                                            }
                                            ui.end_row();

                                            ui.label(
                                                lang.tr(
                                                    "Cor de Destaque (Accent):",
                                                    "Accent Color:",
                                                ),
                                            );
                                            if egui::color_picker::color_edit_button_srgb(
                                                ui,
                                                &mut self.config.theme.accent_color,
                                            )
                                            .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                                visuals_modified = true;
                                            }
                                            ui.end_row();

                                            ui.label(lang.tr("Texto Principal:", "Primary Text:"));
                                            if egui::color_picker::color_edit_button_srgb(
                                                ui,
                                                &mut self.config.theme.text_primary,
                                            )
                                            .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                                visuals_modified = true;
                                            }
                                            ui.end_row();

                                            ui.label(lang.tr(
                                                "Texto Secundário / Esmaecido:",
                                                "Secondary / Muted Text:",
                                            ));
                                            if egui::color_picker::color_edit_button_srgb(
                                                ui,
                                                &mut self.config.theme.text_muted,
                                            )
                                            .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                                visuals_modified = true;
                                            }
                                            ui.end_row();
                                        });
                                }
                                1 => {
                                    // Notas & Bordas
                                    egui::Grid::new("theme_notes_grid")
                                        .num_columns(2)
                                        .spacing([24.0, 10.0])
                                        .show(ui, |ui| {
                                            ui.label(
                                                lang.tr(
                                                    "Cor do Corpo da Nota:",
                                                    "Note Body Color:",
                                                ),
                                            );
                                            if egui::color_picker::color_edit_button_srgb(
                                                ui,
                                                &mut self.config.theme.note_fill,
                                            )
                                            .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                            }
                                            ui.end_row();

                                            ui.label(
                                                lang.tr(
                                                    "Cor da Borda da Nota:",
                                                    "Note Border Color:",
                                                ),
                                            );
                                            if egui::color_picker::color_edit_button_srgb(
                                                ui,
                                                &mut self.config.theme.note_stroke,
                                            )
                                            .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                            }
                                            ui.end_row();

                                            ui.label(lang.tr(
                                                "Nota Selecionada (Corpo):",
                                                "Selected Note (Body):",
                                            ));
                                            if egui::color_picker::color_edit_button_srgb(
                                                ui,
                                                &mut self.config.theme.note_selected_fill,
                                            )
                                            .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                            }
                                            ui.end_row();

                                            ui.label(lang.tr(
                                                "Nota Selecionada (Borda):",
                                                "Selected Note (Border):",
                                            ));
                                            if egui::color_picker::color_edit_button_srgb(
                                                ui,
                                                &mut self.config.theme.note_selected_stroke,
                                            )
                                            .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                            }
                                            ui.end_row();

                                            ui.label(lang.tr(
                                                "Cor do Texto da Letra na Nota:",
                                                "Note Lyric Text Color:",
                                            ));
                                            if egui::color_picker::color_edit_button_srgb(
                                                ui,
                                                &mut self.config.theme.text_note_tag,
                                            )
                                            .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                            }
                                            ui.end_row();

                                            ui.label(lang.tr(
                                                "Arredondamento dos Cantos (Radius):",
                                                "Corner Rounding (Radius):",
                                            ));
                                            if ui
                                                .add(
                                                    egui::Slider::new(
                                                        &mut self.config.theme.note_corner_radius,
                                                        0.0..=14.0,
                                                    )
                                                    .suffix(" px"),
                                                )
                                                .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                            }
                                            ui.end_row();

                                            ui.label(
                                                lang.tr(
                                                    "Espessura da Borda:",
                                                    "Border Stroke Width:",
                                                ),
                                            );
                                            if ui
                                                .add(
                                                    egui::Slider::new(
                                                        &mut self.config.theme.note_stroke_width,
                                                        0.5..=4.0,
                                                    )
                                                    .suffix(" px"),
                                                )
                                                .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                            }
                                            ui.end_row();

                                            ui.label(lang.tr(
                                                "Opacidade do Bloco da Nota:",
                                                "Note Block Opacity:",
                                            ));
                                            if ui
                                                .add(
                                                    egui::Slider::new(
                                                        &mut self.config.theme.note_opacity,
                                                        0.2..=1.0,
                                                    )
                                                    .custom_formatter(|v, _| {
                                                        format!("{:.0}%", v * 100.0)
                                                    }),
                                                )
                                                .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                            }
                                            ui.end_row();

                                            ui.label(lang.tr(
                                                "Cor da Forma de Onda (Waveform):",
                                                "Waveform Color:",
                                            ));
                                            if egui::color_picker::color_edit_button_srgb(
                                                ui,
                                                &mut self.config.theme.note_waveform_color,
                                            )
                                            .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                            }
                                            ui.end_row();

                                            ui.label(lang.tr(
                                                "Opacidade da Waveform Interna:",
                                                "Internal Waveform Opacity:",
                                            ));
                                            if ui
                                                .add(
                                                    egui::Slider::new(
                                                        &mut self
                                                            .config
                                                            .theme
                                                            .note_waveform_opacity,
                                                        0.0..=1.0,
                                                    )
                                                    .custom_formatter(|v, _| {
                                                        format!("{:.0}%", v * 100.0)
                                                    }),
                                                )
                                                .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                            }
                                            ui.end_row();
                                        });
                                }
                                2 => {
                                    // Teclado & Grid
                                    egui::Grid::new("theme_keyboard_grid")
                                        .num_columns(2)
                                        .spacing([24.0, 10.0])
                                        .show(ui, |ui| {
                                            ui.label(lang.tr(
                                                "Teclas Brancas (Teclado Lateral):",
                                                "White Keys (Side Keyboard):",
                                            ));
                                            if egui::color_picker::color_edit_button_srgb(
                                                ui,
                                                &mut self.config.theme.bg_keyboard_white,
                                            )
                                            .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                            }
                                            ui.end_row();

                                            ui.label(lang.tr(
                                                "Teclas Pretas (Teclado Lateral):",
                                                "Black Keys (Side Keyboard):",
                                            ));
                                            if egui::color_picker::color_edit_button_srgb(
                                                ui,
                                                &mut self.config.theme.bg_keyboard_black,
                                            )
                                            .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                            }
                                            ui.end_row();

                                            ui.label(lang.tr(
                                                "Linhas de Compasso (Bar Lines):",
                                                "Bar Lines:",
                                            ));
                                            if egui::color_picker::color_edit_button_srgb(
                                                ui,
                                                &mut self.config.theme.grid_line_bar,
                                            )
                                            .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                            }
                                            ui.end_row();

                                            ui.label(lang.tr(
                                                "Subdivisões do Grid (Sub Lines):",
                                                "Grid Subdivisions (Sub Lines):",
                                            ));
                                            if egui::color_picker::color_edit_button_srgb(
                                                ui,
                                                &mut self.config.theme.grid_line_sub,
                                            )
                                            .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                            }
                                            ui.end_row();

                                            ui.label(lang.tr(
                                                "Linhas de Fundo das Teclas Brancas:",
                                                "White Key Row Background Lines:",
                                            ));
                                            if egui::color_picker::color_edit_button_srgb(
                                                ui,
                                                &mut self.config.theme.bg_row_white_key,
                                            )
                                            .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                            }
                                            ui.end_row();

                                            ui.label(lang.tr(
                                                "Linhas de Fundo das Teclas Pretas:",
                                                "Black Key Row Background Lines:",
                                            ));
                                            if egui::color_picker::color_edit_button_srgb(
                                                ui,
                                                &mut self.config.theme.bg_row_black_key,
                                            )
                                            .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                            }
                                            ui.end_row();
                                        });
                                }
                                3 => {
                                    // Pitch & Playhead
                                    egui::Grid::new("theme_pitch_grid")
                                        .num_columns(2)
                                        .spacing([24.0, 10.0])
                                        .show(ui, |ui| {
                                            ui.label(
                                                lang.tr(
                                                    "Curva de Pitch Bend:",
                                                    "Pitch Bend Curve:",
                                                ),
                                            );
                                            if egui::color_picker::color_edit_button_srgb(
                                                ui,
                                                &mut self.config.theme.pitch_curve_color,
                                            )
                                            .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                            }
                                            ui.end_row();

                                            ui.label(lang.tr(
                                                "Pontos de Ancoragem do Pitch:",
                                                "Pitch Anchor Points:",
                                            ));
                                            if egui::color_picker::color_edit_button_srgb(
                                                ui,
                                                &mut self.config.theme.pitch_anchor_color,
                                            )
                                            .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                            }
                                            ui.end_row();

                                            ui.label(lang.tr(
                                                "Cursor de Reprodução (Playhead):",
                                                "Playhead Cursor:",
                                            ));
                                            if egui::color_picker::color_edit_button_srgb(
                                                ui,
                                                &mut self.config.theme.playhead_color,
                                            )
                                            .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                            }
                                            ui.end_row();
                                        });
                                }
                                _ => {
                                    // Janelas & Painéis
                                    egui::Grid::new("theme_panels_grid")
                                        .num_columns(2)
                                        .spacing([24.0, 10.0])
                                        .show(ui, |ui| {
                                            ui.label(lang.tr(
                                                "Cantos Arredondados da Interface:",
                                                "UI Corner Rounding:",
                                            ));
                                            if ui
                                                .add(
                                                    egui::Slider::new(
                                                        &mut self.config.theme.ui_corner_radius,
                                                        0.0..=16.0,
                                                    )
                                                    .suffix(" px"),
                                                )
                                                .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                                visuals_modified = true;
                                            }
                                            ui.end_row();

                                            ui.label(lang.tr(
                                                "Opacidade de Fundo dos Painéis:",
                                                "Panels Background Opacity:",
                                            ));
                                            if ui
                                                .add(
                                                    egui::Slider::new(
                                                        &mut self.config.theme.panel_opacity,
                                                        0.5..=1.0,
                                                    )
                                                    .custom_formatter(|v, _| {
                                                        format!("{:.0}%", v * 100.0)
                                                    }),
                                                )
                                                .changed()
                                            {
                                                self.config.theme.preset =
                                                    crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                                visuals_modified = true;
                                            }
                                            ui.end_row();
                                        });
                                }
                            }
                        });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(6.0);

                    // Live Preview Canvas Widget
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(
                                    lang.tr("Pré-visualização em Tempo Real", "Real-Time Preview"),
                                )
                                .strong()
                                .size(11.5),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{}: {:.1}px · {}: {:.1}px · {}: {:.0}%",
                                            lang.tr("Cantos", "Corners"),
                                            self.config.theme.note_corner_radius,
                                            lang.tr("Borda", "Border"),
                                            self.config.theme.note_stroke_width,
                                            lang.tr("Opacidade", "Opacity"),
                                            self.config.theme.note_opacity * 100.0
                                        ))
                                        .size(10.5)
                                        .color(self.config.theme.text_muted_c32()),
                                    );
                                },
                            );
                        });

                        ui.add_space(4.0);

                        let (rect, _) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), 95.0),
                            egui::Sense::hover(),
                        );
                        let painter = ui.painter_at(rect);

                        // Background canvas
                        painter.rect_filled(
                            rect,
                            egui::Rounding::same(6.0),
                            self.config.theme.bg_canvas_c32(),
                        );
                        painter.rect_stroke(
                            rect,
                            egui::Rounding::same(6.0),
                            egui::Stroke::new(1.0, self.config.theme.grid_line_sub_c32()),
                        );

                        // Piano keys preview on left
                        let key_w = 40.0;
                        let k_rect = egui::Rect::from_min_max(
                            rect.min,
                            egui::pos2(rect.min.x + key_w, rect.max.y),
                        );
                        painter.rect_filled(
                            k_rect,
                            egui::Rounding::ZERO,
                            self.config.theme.bg_panel_c32(),
                        );
                        painter.line_segment(
                            [
                                egui::pos2(rect.min.x + key_w, rect.min.y),
                                egui::pos2(rect.min.x + key_w, rect.max.y),
                            ],
                            egui::Stroke::new(1.5, self.config.theme.accent_c32()),
                        );

                        // Grid lines
                        let g_x1 = rect.min.x + key_w + 60.0;
                        let g_x2 = rect.min.x + key_w + 140.0;
                        let g_x3 = rect.min.x + key_w + 220.0;
                        let g_x4 = rect.min.x + key_w + 300.0;
                        painter.line_segment(
                            [egui::pos2(g_x1, rect.min.y), egui::pos2(g_x1, rect.max.y)],
                            egui::Stroke::new(1.5, self.config.theme.grid_line_bar_c32()),
                        );
                        painter.line_segment(
                            [egui::pos2(g_x2, rect.min.y), egui::pos2(g_x2, rect.max.y)],
                            egui::Stroke::new(1.0, self.config.theme.grid_line_sub_c32()),
                        );
                        painter.line_segment(
                            [egui::pos2(g_x3, rect.min.y), egui::pos2(g_x3, rect.max.y)],
                            egui::Stroke::new(1.5, self.config.theme.grid_line_bar_c32()),
                        );
                        painter.line_segment(
                            [egui::pos2(g_x4, rect.min.y), egui::pos2(g_x4, rect.max.y)],
                            egui::Stroke::new(1.0, self.config.theme.grid_line_sub_c32()),
                        );

                        // Sample Note 1 (Normal)
                        let n1_rect = egui::Rect::from_min_size(
                            egui::pos2(rect.min.x + key_w + 20.0, rect.min.y + 40.0),
                            egui::vec2(100.0, 32.0),
                        );
                        painter.rect_filled(
                            n1_rect,
                            self.config.theme.note_rounding(),
                            self.config.theme.note_fill_c32(),
                        );
                        painter.rect_stroke(
                            n1_rect,
                            self.config.theme.note_rounding(),
                            self.config.theme.note_stroke(false),
                        );

                        // Waveform in Note 1
                        if self.config.theme.note_waveform_opacity > 0.01 {
                            let wf_c = self.config.theme.c32_alpha(
                                self.config.theme.note_waveform_color,
                                self.config.theme.note_waveform_opacity,
                            );
                            let cy = n1_rect.center().y;
                            painter.line_segment(
                                [
                                    egui::pos2(n1_rect.min.x + 8.0, cy),
                                    egui::pos2(n1_rect.min.x + 25.0, cy - 8.0),
                                ],
                                egui::Stroke::new(1.5, wf_c),
                            );
                            painter.line_segment(
                                [
                                    egui::pos2(n1_rect.min.x + 25.0, cy - 8.0),
                                    egui::pos2(n1_rect.min.x + 45.0, cy + 9.0),
                                ],
                                egui::Stroke::new(1.5, wf_c),
                            );
                            painter.line_segment(
                                [
                                    egui::pos2(n1_rect.min.x + 45.0, cy + 9.0),
                                    egui::pos2(n1_rect.min.x + 70.0, cy - 6.0),
                                ],
                                egui::Stroke::new(1.5, wf_c),
                            );
                            painter.line_segment(
                                [
                                    egui::pos2(n1_rect.min.x + 70.0, cy - 6.0),
                                    egui::pos2(n1_rect.max.x - 8.0, cy),
                                ],
                                egui::Stroke::new(1.5, wf_c),
                            );
                        }
                        painter.text(
                            egui::pos2(n1_rect.min.x + 8.0, n1_rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            "ka [C4]",
                            egui::FontId::proportional(11.0),
                            self.config.theme.text_note_tag_c32(),
                        );

                        // Sample Note 2 (Selected)
                        let n2_rect = egui::Rect::from_min_size(
                            egui::pos2(rect.min.x + key_w + 140.0, rect.min.y + 20.0),
                            egui::vec2(130.0, 32.0),
                        );
                        painter.rect_filled(
                            n2_rect,
                            self.config.theme.note_rounding(),
                            self.config.theme.note_selected_fill_c32(),
                        );
                        painter.rect_stroke(
                            n2_rect,
                            self.config.theme.note_rounding(),
                            self.config.theme.note_stroke(true),
                        );

                        // Waveform in Note 2
                        if self.config.theme.note_waveform_opacity > 0.01 {
                            let wf_c = self.config.theme.c32_alpha(
                                self.config.theme.note_waveform_color,
                                (self.config.theme.note_waveform_opacity * 1.2).min(1.0),
                            );
                            let cy = n2_rect.center().y;
                            painter.line_segment(
                                [
                                    egui::pos2(n2_rect.min.x + 8.0, cy),
                                    egui::pos2(n2_rect.min.x + 35.0, cy - 10.0),
                                ],
                                egui::Stroke::new(1.8, wf_c),
                            );
                            painter.line_segment(
                                [
                                    egui::pos2(n2_rect.min.x + 35.0, cy - 10.0),
                                    egui::pos2(n2_rect.min.x + 65.0, cy + 11.0),
                                ],
                                egui::Stroke::new(1.8, wf_c),
                            );
                            painter.line_segment(
                                [
                                    egui::pos2(n2_rect.min.x + 65.0, cy + 11.0),
                                    egui::pos2(n2_rect.min.x + 95.0, cy - 8.0),
                                ],
                                egui::Stroke::new(1.8, wf_c),
                            );
                            painter.line_segment(
                                [
                                    egui::pos2(n2_rect.min.x + 95.0, cy - 8.0),
                                    egui::pos2(n2_rect.max.x - 8.0, cy),
                                ],
                                egui::Stroke::new(1.8, wf_c),
                            );
                        }
                        painter.text(
                            egui::pos2(n2_rect.min.x + 8.0, n2_rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            "ma [E4] *",
                            egui::FontId::proportional(11.0),
                            self.config.theme.text_note_tag_c32(),
                        );

                        // Sample Pitch curve
                        let p1 = egui::pos2(n1_rect.max.x - 10.0, n1_rect.center().y);
                        let p2 = egui::pos2(n2_rect.min.x + 20.0, n2_rect.center().y);
                        painter.line_segment(
                            [p1, p2],
                            egui::Stroke::new(2.0, self.config.theme.pitch_curve_c32()),
                        );
                        painter.circle_filled(p1, 4.0, self.config.theme.pitch_anchor_c32());
                        painter.circle_filled(p2, 4.0, self.config.theme.pitch_anchor_c32());

                        // Sample Playhead
                        let ph_x = rect.min.x + key_w + 180.0;
                        painter.line_segment(
                            [egui::pos2(ph_x, rect.min.y), egui::pos2(ph_x, rect.max.y)],
                            egui::Stroke::new(
                                3.0,
                                self.config
                                    .theme
                                    .c32_alpha(self.config.theme.playhead_color, 0.35),
                            ),
                        );
                        painter.line_segment(
                            [egui::pos2(ph_x, rect.min.y), egui::pos2(ph_x, rect.max.y)],
                            egui::Stroke::new(1.2, self.config.theme.playhead_c32()),
                        );
                    });

                    ui.add_space(8.0);

                    // Action Footer
                    ui.horizontal(|ui| {
                        if ui
                            .button(lang.tr("Importar Tema (.json)...", "Import Theme (.json)..."))
                            .clicked()
                        {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter(
                                    lang.tr("Tema JSON (*.json)", "Theme JSON (*.json)"),
                                    &["json"],
                                )
                                .pick_file()
                            {
                                if let Ok(content) = std::fs::read_to_string(&path) {
                                    if let Ok(imported) =
                                        serde_json::from_str::<crate::gui::theme::ThemeConfig>(
                                            &content,
                                        )
                                    {
                                        self.config.theme = imported;
                                        theme_modified = true;
                                        visuals_modified = true;
                                        self.transport_state.status_message = format!(
                                            "{}: {}",
                                            lang.tr("Tema importado de", "Theme imported from"),
                                            path.display()
                                        );
                                    } else {
                                        self.transport_state.status_message = lang
                                            .tr("Arquivo de tema inválido.", "Invalid theme file.")
                                            .to_string();
                                    }
                                }
                            }
                        }

                        if ui
                            .button(lang.tr("Exportar Tema (.json)...", "Export Theme (.json)..."))
                            .clicked()
                        {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter(
                                    lang.tr("Tema JSON (*.json)", "Theme JSON (*.json)"),
                                    &["json"],
                                )
                                .set_file_name("kamafeu_custom_theme.json")
                                .save_file()
                            {
                                if let Ok(json) = serde_json::to_string_pretty(&self.config.theme) {
                                    if std::fs::write(&path, json).is_ok() {
                                        self.transport_state.status_message = format!(
                                            "{}: {}",
                                            lang.tr("Tema exportado para", "Theme exported to"),
                                            path.display()
                                        );
                                    }
                                }
                            }
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(lang.tr("Fechar", "Close")).clicked() {
                                close_requested = true;
                            }
                        });
                    });
                });
                if ctx.input(|i| i.viewport().close_requested()) {
                    close_requested = true;
                }
            },
        );

        if close_requested {
            is_open = false;
        }

        if visuals_modified {
            ctx.set_visuals(self.config.theme.create_egui_visuals());
        }

        if theme_modified {
            self.persist_config();
        }

        self.theme_customizer_open = is_open;
    }
}
