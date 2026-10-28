use crate::gui::KamafeuStudioApp;
use eframe::egui;

fn color_row(ui: &mut egui::Ui, label: &str, color: &mut [u8; 3]) -> bool {
    ui.label(label);
    let changed = egui::color_picker::color_edit_button_srgb(ui, color).changed();
    ui.end_row();
    changed
}

fn unique_theme_name(base: &str, themes: &[crate::gui::theme::SavedTheme]) -> String {
    let base = base.trim();
    if !themes
        .iter()
        .any(|entry| entry.name.eq_ignore_ascii_case(base))
    {
        return base.to_string();
    }
    for suffix in 2..=999 {
        let ending = format!(" ({suffix})");
        let prefix: String = base
            .chars()
            .take(64_usize.saturating_sub(ending.chars().count()))
            .collect();
        let candidate = format!("{prefix}{ending}");
        if !themes
            .iter()
            .any(|entry| entry.name.eq_ignore_ascii_case(&candidate))
        {
            return candidate;
        }
    }
    format!("{base} (cópia)")
}

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
                // The tab controls, live preview and action footer need to
                // coexist without the footer being clipped on first launch.
                .with_inner_size([720.0, 720.0])
                .with_min_inner_size([580.0, 600.0]),
            |viewport_ctx, _class| {
                egui::CentralPanel::default().show(viewport_ctx, |ui| {
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
                                        .color(self.config.theme.text_primary_c32())
                                    } else {
                                        egui::RichText::new(preset.display_name_for(lang))
                                            .color(self.config.theme.text_primary_c32())
                                    };

                                    if ui
                                        .selectable_label(is_active, btn_text)
                                        .on_hover_text(preset.description())
                                        .clicked()
                                    {
                                        if preset != crate::gui::theme::ThemePreset::Custom {
                                            self.config.theme.apply_preset(preset);
                                        } else {
                                            self.config.theme.preset = preset;
                                        }
                                        theme_modified = true;
                                        visuals_modified = true;
                                    }
                                }
                            });
                        });

                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);

                    let theme = &self.config.theme;
                    if !crate::gui::theme::meets_text_contrast(theme.text_primary, theme.bg_canvas)
                        || !crate::gui::theme::meets_text_contrast(
                            theme.text_primary,
                            theme.bg_panel,
                        )
                    {
                        ui.colored_label(
                            theme.warning_c32(),
                            lang.tr(
                                "O contraste do texto primário está abaixo de WCAG AA em uma ou mais superfícies.",
                                "Primary text contrast is below WCAG AA on one or more surfaces.",
                            ),
                        );
                    }

                    // Tab Selector
                    egui::ScrollArea::horizontal()
                        .id_salt("theme_customizer_tabs")
                        .show(ui, |ui| {
                        ui.horizontal(|ui| {
                        let tabs = [
                            lang.tr("Cores Gerais", "General Colors"),
                            lang.tr("Notas & Bordas", "Notes & Borders"),
                            lang.tr("Teclado, Linhas & Grid", "Keyboard, Rows & Grid"),
                            lang.tr("Pitch & Playhead", "Pitch & Playhead"),
                            lang.tr("Janelas & Painéis", "Windows & Panels"),
                            lang.tr("Sinais & Waveform", "Status & Waveform"),
                            lang.tr("Meus Temas", "My Themes"),
                        ];
                        for (i, tab_label) in tabs.iter().enumerate() {
                            let selected = self.theme_customizer_tab == i;
                            if ui.selectable_label(selected, *tab_label).clicked() {
                                self.theme_customizer_tab = i;
                            }
                        }
                        });
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
                                                "Fundo do Piano Roll:",
                                                "Piano Roll Background:",
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

                                            ui.label(lang.tr("Nota sob o cursor:", "Hovered Note:"));
                                            if egui::color_picker::color_edit_button_srgb(
                                                ui,
                                                &mut self.config.theme.note_hover,
                                            )
                                            .changed()
                                            {
                                                self.config.theme.preset = crate::gui::theme::ThemePreset::Custom;
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

                                            ui.label(lang.tr("Âncoras de Pitch:", "Pitch Anchors:"));
                                            if egui::color_picker::color_edit_button_srgb(
                                                ui,
                                                &mut self.config.theme.pitch_anchor_color,
                                            )
                                            .changed()
                                            {
                                                self.config.theme.preset = crate::gui::theme::ThemePreset::Custom;
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
                                4 => {
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
                                5 => {
                                    egui::Grid::new("theme_semantic_grid")
                                        .num_columns(2)
                                        .spacing([24.0, 10.0])
                                        .show(ui, |ui| {
                                            let semantic = &mut self.config.theme.semantic;
                                            let mut changed = false;
                                            changed |= color_row(ui, lang.tr("Sucesso", "Success"), &mut semantic.success);
                                            changed |= color_row(ui, lang.tr("Aviso", "Warning"), &mut semantic.warning);
                                            changed |= color_row(ui, lang.tr("Erro", "Error"), &mut semantic.danger);
                                            changed |= color_row(ui, lang.tr("Informação", "Information"), &mut semantic.info);
                                            changed |= color_row(ui, lang.tr("Waveform", "Waveform"), &mut semantic.waveform);
                                            changed |= color_row(ui, lang.tr("Grade da waveform", "Waveform grid"), &mut semantic.waveform_grid);
                                            changed |= color_row(ui, lang.tr("Marcadores", "Markers"), &mut semantic.marker);
                                            changed |= color_row(ui, lang.tr("Região de loop", "Loop region"), &mut semantic.loop_region);
                                            changed |= color_row(ui, lang.tr("Medidores de áudio", "Audio meters"), &mut semantic.meter);
                                            if changed {
                                                self.config.theme.preset = crate::gui::theme::ThemePreset::Custom;
                                                theme_modified = true;
                                            }
                                        });
                                }
                                _ => {
                                    ui.horizontal(|ui| {
                                        ui.label(lang.tr("Nome do tema", "Theme name"));
                                        ui.text_edit_singleline(&mut self.saved_theme_name_draft);
                                        if ui.button(lang.tr("Salvar como novo", "Save as new")).clicked() {
                                            match crate::gui::theme::validate_theme_name(&self.saved_theme_name_draft) {
                                                Ok(name) if !self.config.saved_themes.iter().any(|entry| entry.name.eq_ignore_ascii_case(&name)) => {
                                                    self.config.saved_themes.push(crate::gui::theme::SavedTheme { name: name.clone(), theme: self.config.theme.clone() });
                                                    self.saved_theme_selected = Some(self.config.saved_themes.len() - 1);
                                                    self.saved_theme_name_draft = name;
                                                    theme_modified = true;
                                                }
                                                Ok(_) => self.transport_state.status_message = lang.tr("Já existe um tema com esse nome.", "A theme with that name already exists.").to_string(),
                                                Err(error) => self.transport_state.status_message = error,
                                            }
                                        }
                                    });

                                    egui::ScrollArea::vertical()
                                        .id_salt("saved_theme_list")
                                        .max_height(115.0)
                                        .show(ui, |ui| {
                                            for (index, entry) in self.config.saved_themes.iter().enumerate() {
                                                let selected = self.saved_theme_selected == Some(index);
                                                if ui.selectable_label(selected, &entry.name).clicked() {
                                                    self.saved_theme_selected = Some(index);
                                                    self.saved_theme_name_draft = entry.name.clone();
                                                }
                                            }
                                        });

                                    if let Some(index) = self.saved_theme_selected.filter(|i| *i < self.config.saved_themes.len()) {
                                        ui.horizontal(|ui| {
                                            if ui.button(lang.tr("Aplicar", "Apply")).clicked() {
                                                self.config.theme = self.config.saved_themes[index].theme.clone();
                                                visuals_modified = true;
                                                theme_modified = true;
                                            }
                                            if ui.button(lang.tr("Atualizar com o tema atual", "Update from current theme")).clicked() {
                                                self.config.saved_themes[index].theme = self.config.theme.clone();
                                                theme_modified = true;
                                            }
                                            if ui.button(lang.tr("Renomear", "Rename")).clicked() {
                                                match crate::gui::theme::validate_theme_name(&self.saved_theme_name_draft) {
                                                    Ok(name) if !self.config.saved_themes.iter().enumerate().any(|(i, entry)| i != index && entry.name.eq_ignore_ascii_case(&name)) => {
                                                        self.config.saved_themes[index].name = name.clone();
                                                        self.saved_theme_name_draft = name;
                                                        theme_modified = true;
                                                    }
                                                    Ok(_) => self.transport_state.status_message = lang.tr("Já existe um tema com esse nome.", "A theme with that name already exists.").to_string(),
                                                    Err(error) => self.transport_state.status_message = error,
                                                }
                                            }
                                            if ui.button(lang.tr("Excluir…", "Delete…")).clicked() {
                                                self.pending_theme_delete = Some(index);
                                            }
                                            if ui.button(lang.tr("Duplicar", "Duplicate")).clicked() {
                                                let name = unique_theme_name(&format!("{} (cópia)", self.config.saved_themes[index].name), &self.config.saved_themes);
                                                self.config.saved_themes.push(crate::gui::theme::SavedTheme { name: name.clone(), theme: self.config.saved_themes[index].theme.clone() });
                                                self.saved_theme_selected = Some(self.config.saved_themes.len() - 1);
                                                self.saved_theme_name_draft = name;
                                                theme_modified = true;
                                            }
                                        });
                                    }
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
                            egui::Stroke::new(1.0_f32, self.config.theme.grid_line_sub_c32()),
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
                            egui::Stroke::new(1.5_f32, self.config.theme.accent_c32()),
                        );

                        // Grid lines
                        let g_x1 = rect.min.x + key_w + 60.0;
                        let g_x2 = rect.min.x + key_w + 140.0;
                        let g_x3 = rect.min.x + key_w + 220.0;
                        let g_x4 = rect.min.x + key_w + 300.0;
                        painter.line_segment(
                            [egui::pos2(g_x1, rect.min.y), egui::pos2(g_x1, rect.max.y)],
                            egui::Stroke::new(1.5_f32, self.config.theme.grid_line_bar_c32()),
                        );
                        painter.line_segment(
                            [egui::pos2(g_x2, rect.min.y), egui::pos2(g_x2, rect.max.y)],
                            egui::Stroke::new(1.0_f32, self.config.theme.grid_line_sub_c32()),
                        );
                        painter.line_segment(
                            [egui::pos2(g_x3, rect.min.y), egui::pos2(g_x3, rect.max.y)],
                            egui::Stroke::new(1.5_f32, self.config.theme.grid_line_bar_c32()),
                        );
                        painter.line_segment(
                            [egui::pos2(g_x4, rect.min.y), egui::pos2(g_x4, rect.max.y)],
                            egui::Stroke::new(1.0_f32, self.config.theme.grid_line_sub_c32()),
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
                                egui::Stroke::new(1.5_f32, wf_c),
                            );
                            painter.line_segment(
                                [
                                    egui::pos2(n1_rect.min.x + 25.0, cy - 8.0),
                                    egui::pos2(n1_rect.min.x + 45.0, cy + 9.0),
                                ],
                                egui::Stroke::new(1.5_f32, wf_c),
                            );
                            painter.line_segment(
                                [
                                    egui::pos2(n1_rect.min.x + 45.0, cy + 9.0),
                                    egui::pos2(n1_rect.min.x + 70.0, cy - 6.0),
                                ],
                                egui::Stroke::new(1.5_f32, wf_c),
                            );
                            painter.line_segment(
                                [
                                    egui::pos2(n1_rect.min.x + 70.0, cy - 6.0),
                                    egui::pos2(n1_rect.max.x - 8.0, cy),
                                ],
                                egui::Stroke::new(1.5_f32, wf_c),
                            );
                        }
                        painter.text(
                            egui::pos2(n1_rect.min.x + 8.0, n1_rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            "ka [C4]",
                            egui::FontId::proportional(11.0),
                            self.config.theme.note_lyric_color(false),
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
                                egui::Stroke::new(1.8_f32, wf_c),
                            );
                            painter.line_segment(
                                [
                                    egui::pos2(n2_rect.min.x + 35.0, cy - 10.0),
                                    egui::pos2(n2_rect.min.x + 65.0, cy + 11.0),
                                ],
                                egui::Stroke::new(1.8_f32, wf_c),
                            );
                            painter.line_segment(
                                [
                                    egui::pos2(n2_rect.min.x + 65.0, cy + 11.0),
                                    egui::pos2(n2_rect.min.x + 95.0, cy - 8.0),
                                ],
                                egui::Stroke::new(1.8_f32, wf_c),
                            );
                            painter.line_segment(
                                [
                                    egui::pos2(n2_rect.min.x + 95.0, cy - 8.0),
                                    egui::pos2(n2_rect.max.x - 8.0, cy),
                                ],
                                egui::Stroke::new(1.8_f32, wf_c),
                            );
                        }
                        painter.text(
                            egui::pos2(n2_rect.min.x + 8.0, n2_rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            "ma [E4] *",
                            egui::FontId::proportional(11.0),
                            self.config.theme.note_lyric_color(true),
                        );

                        // Sample Pitch curve
                        let p1 = egui::pos2(n1_rect.max.x - 10.0, n1_rect.center().y);
                        let p2 = egui::pos2(n2_rect.min.x + 20.0, n2_rect.center().y);
                        painter.line_segment(
                            [p1, p2],
                            egui::Stroke::new(2.0_f32, self.config.theme.pitch_curve_c32()),
                        );
                        painter.circle_filled(p1, 4.0, self.config.theme.pitch_curve_c32());
                        painter.circle_filled(p2, 4.0, self.config.theme.pitch_curve_c32());

                        // Sample Playhead
                        let ph_x = rect.min.x + key_w + 180.0;
                        painter.line_segment(
                            [egui::pos2(ph_x, rect.min.y), egui::pos2(ph_x, rect.max.y)],
                            egui::Stroke::new(
                                3.0_f32,
                                self.config
                                    .theme
                                    .c32_alpha(self.config.theme.playhead_color, 0.35),
                            ),
                        );
                        painter.line_segment(
                            [egui::pos2(ph_x, rect.min.y), egui::pos2(ph_x, rect.max.y)],
                            egui::Stroke::new(1.2_f32, self.config.theme.playhead_c32()),
                        );
                    });

                    ui.add_space(8.0);

                    // Action Footer
                    ui.horizontal(|ui| {
                        if ui
                            .button(lang.tr("Importar Tema (.json)...", "Import Theme (.json)..."))
                            .clicked()
                        {
                            if let Some(path) = crate::dialogs::FileDialog::new()
                                .add_filter(
                                    lang.tr("Tema JSON (*.json)", "Theme JSON (*.json)"),
                                    &["json"],
                                )
                                .pick_file()
                            {
                                match std::fs::read_to_string(&path) {
                                    Ok(content) => {
                                        let name = path
                                            .file_stem()
                                            .and_then(|stem| stem.to_str())
                                            .unwrap_or("Tema importado")
                                            .to_string();
                                        self.pending_theme_import = Some((name, content));
                                    }
                                    Err(error) => {
                                        self.transport_state.status_message = format!(
                                            "{}: {error}",
                                            lang.tr("Não foi possível ler o tema", "Could not read theme")
                                        );
                                    }
                                }
                            }
                        }

                        if ui
                            .button(lang.tr("Exportar Tema (.json)...", "Export Theme (.json)..."))
                            .clicked()
                        {
                            if let Some(path) = crate::dialogs::FileDialog::new()
                                .add_filter(
                                    lang.tr("Tema JSON (*.json)", "Theme JSON (*.json)"),
                                    &["json"],
                                )
                                .set_file_name("kamafeu_custom_theme.json")
                                .save_file()
                            {
                                let name = self
                                    .saved_theme_selected
                                    .and_then(|index| self.config.saved_themes.get(index))
                                    .filter(|entry| entry.theme == self.config.theme)
                                    .map(|entry| entry.name.as_str())
                                    .unwrap_or("Tema personalizado");
                                let result = crate::gui::theme::ThemeDocument::from_theme(
                                    name,
                                    &self.config.theme,
                                )
                                .and_then(|document| {
                                    serde_json::to_string_pretty(&document)
                                        .map_err(|error| format!("Falha ao serializar tema: {error}"))
                                })
                                .and_then(|json| {
                                    std::fs::write(&path, json)
                                        .map_err(|error| format!("Falha ao gravar tema: {error}"))
                                });
                                if result.is_ok() {
                                        self.transport_state.status_message = format!(
                                            "{}: {}",
                                            lang.tr("Tema exportado para", "Theme exported to"),
                                            path.display()
                                        );
                                } else if let Err(error) = result {
                                    self.transport_state.status_message = error;
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
                let mut import_action: Option<Option<bool>> = None;
                if let Some((name, content)) = self.pending_theme_import.clone() {
                    egui::Window::new(lang.tr("Importar tema", "Import theme"))
                        .collapsible(false)
                        .resizable(false)
                        .show(viewport_ctx, |ui| {
                            ui.label(lang.tr(
                                "Escolha como aplicar este tema. Ele será salvo na biblioteca.",
                                "Choose how to apply this theme. It will be saved to your library.",
                            ));
                            ui.label(&name);
                            ui.horizontal(|ui| {
                                if ui.button(lang.tr("Substituir", "Replace")).clicked() {
                                    import_action = Some(Some(false));
                                }
                                if ui.button(lang.tr("Mesclar", "Merge")).clicked() {
                                    import_action = Some(Some(true));
                                }
                                if ui.button(lang.tr("Cancelar", "Cancel")).clicked() {
                                    import_action = Some(None);
                                }
                            });
                        });
                    if let Some(action) = import_action {
                        self.pending_theme_import = None;
                        if let Some(merge) = action {
                            match crate::gui::theme::ThemeDocument::import(
                                &content,
                                &self.config.theme,
                                merge,
                                &name,
                            ) {
                                Ok(mut imported) => {
                                    imported.name = unique_theme_name(
                                        &imported.name,
                                        &self.config.saved_themes,
                                    );
                                    self.config.theme = imported.theme.clone();
                                    self.config.saved_themes.push(imported.clone());
                                    self.saved_theme_selected = Some(self.config.saved_themes.len() - 1);
                                    self.saved_theme_name_draft = imported.name;
                                    theme_modified = true;
                                    visuals_modified = true;
                                    self.transport_state.status_message = lang
                                        .tr("Tema importado e aplicado.", "Theme imported and applied.")
                                        .to_string();
                                }
                                Err(error) => self.transport_state.status_message = error,
                            }
                        }
                    }
                }

                if let Some(index) = self
                    .pending_theme_delete
                    .filter(|index| *index < self.config.saved_themes.len())
                {
                    let mut delete_choice: Option<bool> = None;
                    egui::Window::new(lang.tr("Excluir tema salvo?", "Delete saved theme?"))
                        .collapsible(false)
                        .resizable(false)
                        .show(viewport_ctx, |ui| {
                            ui.label(format!(
                                "{} ‘{}’?",
                                lang.tr("Excluir", "Delete"),
                                self.config.saved_themes[index].name
                            ));
                            ui.horizontal(|ui| {
                                if ui.button(lang.tr("Excluir", "Delete")).clicked() {
                                    delete_choice = Some(true);
                                }
                                if ui.button(lang.tr("Cancelar", "Cancel")).clicked() {
                                    delete_choice = Some(false);
                                }
                            });
                        });
                    if let Some(confirm) = delete_choice {
                        self.pending_theme_delete = None;
                        if confirm {
                            self.config.saved_themes.remove(index);
                            self.saved_theme_selected = None;
                            self.saved_theme_name_draft.clear();
                            theme_modified = true;
                        }
                    }
                }

                if viewport_ctx.input(|i| i.viewport().close_requested()) {
                    close_requested = true;
                }
            },
        );

        if close_requested {
            is_open = false;
        }

        if visuals_modified || theme_modified {
            ctx.set_visuals(self.config.theme.create_egui_visuals());
        }

        if theme_modified {
            self.persist_config();
        }

        self.theme_customizer_open = is_open;
    }
}
