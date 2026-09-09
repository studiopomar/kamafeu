use crate::config::AppLanguage;
use crate::gui::theme::{ThemeConfig, ThemePreset};
use eframe::egui::{self, Frame, RichText, Window};

#[derive(Debug, Clone, Copy)]
pub struct ThemeEditorDialogState {
    pub is_open: bool,
}

impl Default for ThemeEditorDialogState {
    fn default() -> Self {
        Self { is_open: false }
    }
}

pub fn draw_theme_editor_dialog(
    ctx: &egui::Context,
    lang: AppLanguage,
    theme: &mut ThemeConfig,
    state: &mut ThemeEditorDialogState,
    on_theme_changed: &mut dyn FnMut(),
) {
    if !state.is_open {
        return;
    }

    let mut window_open = state.is_open;
    let mut close_clicked = false;
    let mut modified = false;

    Window::new(
        RichText::new(lang.tr("Editor Visual de Temas", "Visual Theme Editor"))
            .strong()
            .color(theme.accent_c32()),
    )
    .open(&mut window_open)
    .resizable(true)
    .default_width(500.0)
    .default_height(520.0)
    .frame(
        Frame::window(&ctx.style())
            .fill(theme.bg_panel_c32())
            .stroke(theme.card_stroke()),
    )
    .show(ctx, |ui| {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(lang.tr("Predefinição Base:", "Base Preset:"))
                        .strong()
                        .size(11.0)
                        .color(theme.accent_c32()),
                );
                let mut selected_preset = theme.preset;
                egui::ComboBox::from_id_salt("theme_editor_preset_cb")
                    .selected_text(selected_preset.display_name_for(lang))
                    .show_ui(ui, |ui| {
                        for preset in ThemePreset::ALL {
                            if ui
                                .selectable_value(
                                    &mut selected_preset,
                                    preset,
                                    preset.display_name_for(lang),
                                )
                                .clicked()
                            {
                                *theme = ThemeConfig::from_preset(preset);
                                modified = true;
                            }
                        }
                    });
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            egui::ScrollArea::vertical()
                .id_salt("theme_editor_scroll")
                .show(ui, |ui| {
                    // --- Cores Principais de Fundo ---
                    Frame::none()
                        .fill(theme.card_bg_c32())
                        .rounding(theme.ui_rounding())
                        .stroke(theme.card_stroke())
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(lang.tr("Fundo e Painéis", "Background & Panels"))
                                    .strong()
                                    .size(11.0)
                                    .color(theme.accent_c32()),
                            );
                            ui.add_space(4.0);

                            ui.horizontal(|ui| {
                                modified |=
                                    ui.color_edit_button_srgb(&mut theme.bg_canvas).changed();
                                ui.label(lang.tr(
                                    "Fundo do Canvas / Piano Roll",
                                    "Canvas / Piano Roll Background",
                                ));
                            });
                            ui.horizontal(|ui| {
                                modified |=
                                    ui.color_edit_button_srgb(&mut theme.bg_panel).changed();
                                ui.label(lang.tr(
                                    "Fundo dos Painéis e Toolbars",
                                    "Panels & Toolbars Background",
                                ));
                            });
                            ui.horizontal(|ui| {
                                modified |=
                                    ui.color_edit_button_srgb(&mut theme.bg_header).changed();
                                ui.label(lang.tr(
                                    "Fundo de Cabeçalhos e Réguas",
                                    "Headers & Rulers Background",
                                ));
                            });
                            ui.horizontal(|ui| {
                                modified |=
                                    ui.color_edit_button_srgb(&mut theme.accent_color).changed();
                                ui.label(
                                    lang.tr("Cor de Acento / Destaque", "Accent / Highlight Color"),
                                );
                            });
                            ui.horizontal(|ui| {
                                modified |= ui
                                    .color_edit_button_srgb(&mut theme.playhead_color)
                                    .changed();
                                ui.label(
                                    lang.tr("Cursor de Reprodução (Playhead)", "Playhead Cursor"),
                                );
                            });
                        });

                    ui.add_space(8.0);

                    // --- Cores das Notas ---
                    Frame::none()
                        .fill(theme.card_bg_c32())
                        .rounding(theme.ui_rounding())
                        .stroke(theme.card_stroke())
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(lang.tr("Notas Musicais", "Musical Notes"))
                                    .strong()
                                    .size(11.0)
                                    .color(theme.accent_c32()),
                            );
                            ui.add_space(4.0);

                            ui.horizontal(|ui| {
                                modified |=
                                    ui.color_edit_button_srgb(&mut theme.note_fill).changed();
                                ui.label(
                                    lang.tr("Preenchimento da Nota Normal", "Normal Note Fill"),
                                );
                            });
                            ui.horizontal(|ui| {
                                modified |=
                                    ui.color_edit_button_srgb(&mut theme.note_stroke).changed();
                                ui.label(lang.tr("Borda da Nota Normal", "Normal Note Border"));
                            });
                            ui.horizontal(|ui| {
                                modified |= ui
                                    .color_edit_button_srgb(&mut theme.note_selected_fill)
                                    .changed();
                                ui.label(
                                    lang.tr(
                                        "Preenchimento da Nota Selecionada",
                                        "Selected Note Fill",
                                    ),
                                );
                            });
                            ui.horizontal(|ui| {
                                modified |= ui
                                    .color_edit_button_srgb(&mut theme.note_selected_stroke)
                                    .changed();
                                ui.label(
                                    lang.tr("Borda da Nota Selecionada", "Selected Note Border"),
                                );
                            });
                            ui.horizontal(|ui| {
                                modified |=
                                    ui.color_edit_button_srgb(&mut theme.note_hover).changed();
                                ui.label(lang.tr(
                                    "Nota em Hover (Mouse Sobre)",
                                    "Hovered Note (Mouse Over)",
                                ));
                            });
                            ui.horizontal(|ui| {
                                modified |= ui
                                    .color_edit_button_srgb(&mut theme.pitch_curve_color)
                                    .changed();
                                ui.label(lang.tr("Curva de Pitch Bend", "Pitch Bend Curve"));
                            });
                        });

                    ui.add_space(8.0);

                    // --- Tipografia & Grade ---
                    Frame::none()
                        .fill(theme.card_bg_c32())
                        .rounding(theme.ui_rounding())
                        .stroke(theme.card_stroke())
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(lang.tr("Tipografia & Grade", "Typography & Grid"))
                                    .strong()
                                    .size(11.0)
                                    .color(theme.accent_c32()),
                            );
                            ui.add_space(4.0);

                            ui.horizontal(|ui| {
                                modified |=
                                    ui.color_edit_button_srgb(&mut theme.text_primary).changed();
                                ui.label(lang.tr("Texto Principal", "Primary Text"));
                            });
                            ui.horizontal(|ui| {
                                modified |=
                                    ui.color_edit_button_srgb(&mut theme.text_muted).changed();
                                ui.label(
                                    lang.tr("Texto Secundário / Muted", "Secondary / Muted Text"),
                                );
                            });
                            ui.horizontal(|ui| {
                                modified |= ui
                                    .color_edit_button_srgb(&mut theme.grid_line_bar)
                                    .changed();
                                ui.label(lang.tr("Linhas de Compasso", "Bar Lines"));
                            });
                            ui.horizontal(|ui| {
                                modified |= ui
                                    .color_edit_button_srgb(&mut theme.grid_line_sub)
                                    .changed();
                                ui.label(
                                    lang.tr(
                                        "Linhas de Subdivisão de Tempo",
                                        "Beat Subdivision Lines",
                                    ),
                                );
                            });
                        });

                    ui.add_space(8.0);

                    // --- Geometria e Estilo ---
                    Frame::none()
                        .fill(theme.card_bg_c32())
                        .rounding(theme.ui_rounding())
                        .stroke(theme.card_stroke())
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(
                                    lang.tr("Geometria & Arredondamentos", "Geometry & Rounding"),
                                )
                                .strong()
                                .size(11.0)
                                .color(theme.accent_c32()),
                            );
                            ui.add_space(4.0);

                            ui.horizontal(|ui| {
                                ui.label(
                                    lang.tr("Arredondamento das Notas:", "Note Corner Radius:"),
                                );
                                modified |= ui
                                    .add(
                                        egui::Slider::new(
                                            &mut theme.note_corner_radius,
                                            0.0..=12.0,
                                        )
                                        .suffix(" px"),
                                    )
                                    .changed();
                            });
                            ui.horizontal(|ui| {
                                ui.label(lang.tr(
                                    "Arredondamento da UI / Cards:",
                                    "UI / Card Corner Radius:",
                                ));
                                modified |= ui
                                    .add(
                                        egui::Slider::new(&mut theme.ui_corner_radius, 0.0..=12.0)
                                            .suffix(" px"),
                                    )
                                    .changed();
                            });
                            ui.horizontal(|ui| {
                                ui.label(lang.tr(
                                    "Espessura da Borda das Notas:",
                                    "Note Border Stroke Width:",
                                ));
                                modified |= ui
                                    .add(
                                        egui::Slider::new(&mut theme.note_stroke_width, 0.5..=3.0)
                                            .suffix(" px"),
                                    )
                                    .changed();
                            });
                            ui.horizontal(|ui| {
                                ui.label(lang.tr("Opacidade das Notas:", "Note Opacity:"));
                                modified |= ui
                                    .add(egui::Slider::new(&mut theme.note_opacity, 0.3..=1.0))
                                    .changed();
                            });
                        });
                });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                if ui
                    .button(
                        RichText::new(
                            lang.tr("Salvar como Tema Personalizado", "Save as Custom Theme"),
                        )
                        .strong()
                        .size(11.5),
                    )
                    .clicked()
                {
                    theme.preset = ThemePreset::Custom;
                    modified = true;
                    close_clicked = true;
                }
                if ui
                    .button(RichText::new(lang.tr("Fechar", "Close")).size(11.0))
                    .clicked()
                {
                    close_clicked = true;
                }
            });
        });
    });

    if modified {
        theme.preset = ThemePreset::Custom;
        on_theme_changed();
    }

    state.is_open = window_open && !close_clicked;
}
