mod edit;
mod file;
mod help;
mod language;
mod playback;
mod singers;
mod tools;
mod tracks;
mod view;
mod vocal_modes;
use super::KamafeuStudioApp;
use crate::gui::theme::MelodyneTheme;
use crate::gui::types::EditTool;
use crate::oto::Voicebank;
use eframe::egui::{self, Frame, TopBottomPanel};
use std::path::PathBuf;

impl KamafeuStudioApp {
    pub(crate) fn render_menu_bar(&mut self, ctx: &egui::Context) {
        let screen_width = ctx.screen_rect().width();
        let is_mobile = screen_width < 768.0;

        if is_mobile {
            self.render_mobile_menu_bar(ctx);
            if self.mobile_menu_open {
                self.render_mobile_menu_drawer(ctx);
            }
            return;
        }

        TopBottomPanel::top("top_menu_bar")
            .exact_height(26.0)
            .frame(Frame::none().fill(self.config.theme.bg_panel_c32()))
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    self.menu_file(ui);
                    self.menu_edit(ui);
                    self.menu_tracks(ui);
                    self.menu_vocal_modes(ui);
                    self.menu_singers(ui);
                    self.menu_view(ui, ctx);
                    self.menu_playback(ui);

                    ui.menu_button("⋯", |ui| {
                        self.menu_tools(ui);
                        self.menu_language(ui);
                        if ui
                            .button(self.config.language.tr("Pacotes", "Packages"))
                            .clicked()
                        {
                            self.packages_window_open = true;
                            ui.close_menu();
                        }
                        self.menu_help(ui);
                    })
                    .response
                    .on_hover_text(self.config.language.tr(
                        "Mais opções: ferramentas, idioma, pacotes e ajuda",
                        "More options: tools, language, packages and help",
                    ));

                    // ==========================================
                    // DIREITA: BADGE DO PROJETO
                    // ==========================================
                    let lang = self.config.language;
                    let project_display_name = if let Some(ref path) = self.current_project_path {
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("project.aps")
                            .to_string()
                    } else {
                        let stem = self.project.name.trim();
                        if stem.is_empty() {
                            lang.tr("Novo Projeto", "New Project").to_string()
                        } else {
                            stem.to_string()
                        }
                    };

                    let title_text = if self.is_dirty {
                        format!("* {} {}", project_display_name, lang.tr("(Não salvo)", "(Unsaved)"))
                    } else {
                        project_display_name.clone()
                    };

                    let is_light = self.config.theme.is_light();
                    let (badge_bg, badge_border, badge_fg, badge_icon, badge_tooltip) = if self.is_dirty {
                        if is_light {
                            (
                                egui::Color32::from_rgb(255, 235, 220),
                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(230, 120, 20)),
                                egui::Color32::from_rgb(180, 70, 0),
                                "●",
                                lang.tr(
                                    "Projeto modificado (não salvo). Clique aqui ou use Ctrl+S / Cmd+S para salvar agora.",
                                    "Project modified (unsaved). Click here or press Ctrl+S / Cmd+S to save now.",
                                ),
                            )
                        } else {
                            (
                                egui::Color32::from_rgb(58, 30, 16),
                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(255, 170, 50)),
                                egui::Color32::from_rgb(255, 200, 100),
                                "●",
                                lang.tr(
                                    "Projeto modificado (não salvo). Clique aqui ou use Ctrl+S / Cmd+S para salvar agora.",
                                    "Project modified (unsaved). Click here or press Ctrl+S / Cmd+S to save now.",
                                ),
                            )
                        }
                    } else {
                        if is_light {
                            (
                                egui::Color32::from_rgb(230, 248, 238),
                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(40, 170, 100)),
                                egui::Color32::from_rgb(20, 120, 60),
                                "•",
                                lang.tr(
                                    "Projeto salvo no disco. Clique para salvar novamente ou salvar cópia.",
                                    "Project saved to disk. Click to save again or save copy.",
                                ),
                            )
                        } else {
                            (
                                egui::Color32::from_rgb(18, 32, 28),
                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(45, 180, 120)),
                                egui::Color32::from_rgb(130, 240, 190),
                                "•",
                                lang.tr(
                                    "Projeto salvo no disco. Clique para salvar novamente ou salvar cópia.",
                                    "Project saved to disk. Click to save again or save copy.",
                                ),
                            )
                        }
                    };

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(4.0);
                        let badge_btn = egui::Button::new(
                            egui::RichText::new(format!("{} {}", badge_icon, title_text))
                                .size(10.5)
                                .strong()
                                .color(badge_fg),
                        )
                        .fill(badge_bg)
                        .stroke(badge_border)
                        .rounding(egui::Rounding::same(4.0));

                        let resp = ui.add(badge_btn).on_hover_text(badge_tooltip);
                        if resp.clicked() {
                            self.save_project();
                        }

                        ui.add_space(4.0);
                        let (v_bg, v_stroke, v_fg) = if is_light {
                            (
                                egui::Color32::from_rgb(255, 245, 220),
                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(220, 160, 40)),
                                egui::Color32::from_rgb(160, 95, 0),
                            )
                        } else {
                            (
                                egui::Color32::from_rgb(38, 28, 14),
                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(200, 140, 40)),
                                egui::Color32::from_rgb(255, 205, 100),
                            )
                        };
                        let version_badge = egui::Button::new(
                            egui::RichText::new(format!("v{}", crate::APP_VERSION))
                                .size(10.0)
                                .strong()
                                .color(v_fg),
                        )
                        .fill(v_bg)
                        .stroke(v_stroke)
                        .rounding(egui::Rounding::same(4.0));

                        let version_resp = ui.add(version_badge).on_hover_text(lang.tr(
                            &format!("Kamafeu Studio v{} (Bariloche)\nCodinome interno: {}\nMotor de Síntese de Voz & DAW", crate::APP_VERSION, crate::APP_CODENAME),
                            &format!("Kamafeu Studio v{} (Bariloche)\nInternal codename: {}\nVoice Synthesis Engine & DAW", crate::APP_VERSION, crate::APP_CODENAME),
                        ));
                        if version_resp.clicked() {
                            self.shortcuts_guide_open = true;
                        }
                    });
                });
            });
    }

    fn render_mobile_menu_bar(&mut self, ctx: &egui::Context) {
        let lang = self.config.language;
        let bg_panel = self.config.theme.bg_panel_c32();
        let accent = self.config.theme.accent_c32();
        let card_bg = self.config.theme.card_bg_c32();
        let text_primary = self.config.theme.text_primary_c32();

        TopBottomPanel::top("mobile_top_bar")
            .exact_height(34.0)
            .frame(Frame::none().fill(bg_panel))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.add_space(4.0);
                    let menu_btn = egui::Button::new(
                        egui::RichText::new("☰ Menu")
                            .strong()
                            .size(12.0)
                            .color(accent),
                    )
                    .fill(card_bg)
                    .stroke(egui::Stroke::new(1.0, accent))
                    .rounding(egui::Rounding::same(5.0));

                    if ui.add(menu_btn).clicked() {
                        self.mobile_menu_open = !self.mobile_menu_open;
                    }

                    ui.add_space(4.0);

                    // Project Title
                    let project_name = if let Some(ref path) = self.current_project_path {
                        path.file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("project.aps")
                            .to_string()
                    } else {
                        let stem = self.project.name.trim();
                        if stem.is_empty() {
                            lang.tr("Novo Projeto", "New Project").to_string()
                        } else {
                            stem.to_string()
                        }
                    };

                    let title_text = if self.is_dirty {
                        format!("* {project_name}")
                    } else {
                        project_name
                    };

                    ui.label(egui::RichText::new(title_text).strong().size(11.5).color(
                        if self.is_dirty {
                            egui::Color32::from_rgb(255, 170, 50)
                        } else {
                            text_primary
                        },
                    ));

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(4.0);
                        if ui
                            .button(egui::RichText::new("💾").size(12.0))
                            .on_hover_text(lang.tr("Salvar Projeto", "Save Project"))
                            .clicked()
                        {
                            self.save_project();
                        }

                        if ui
                            .button(egui::RichText::new("⚙").size(12.0))
                            .on_hover_text(lang.tr("Preferências", "Preferences"))
                            .clicked()
                        {
                            self.preferences_window_open = true;
                        }
                    });
                });
            });
    }

    fn render_mobile_menu_drawer(&mut self, ctx: &egui::Context) {
        let lang = self.config.language;
        let theme = &self.config.theme;

        egui::Window::new(lang.tr("Menu Principal", "Main Menu"))
            .id(egui::Id::new("mobile_main_menu_drawer"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::LEFT_TOP, egui::Vec2::new(0.0, 34.0))
            .default_size(egui::Vec2::new(280.0, ctx.screen_rect().height() - 34.0))
            .frame(
                Frame::window(&ctx.style())
                    .fill(theme.bg_panel_c32())
                    .rounding(egui::Rounding::same(0.0)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.spacing_mut().item_spacing = egui::Vec2::new(0.0, 6.0);

                    ui.horizontal(|ui| {
                        ui.heading(lang.tr("Navegação", "Navigation"));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button("✖").clicked() {
                                self.mobile_menu_open = false;
                            }
                        });
                    });
                    ui.separator();

                    // Categorias em CollapsingHeader com touch targets confortáveis
                    ui.collapsing(lang.tr("📁 Arquivo", "📁 File"), |ui| {
                        self.menu_file(ui);
                    });

                    ui.collapsing(lang.tr("✏️ Editar", "✏️ Edit"), |ui| {
                        self.menu_edit(ui);
                    });

                    ui.collapsing(lang.tr("🎚️ Faixas", "🎚️ Tracks"), |ui| {
                        self.menu_tracks(ui);
                    });

                    ui.collapsing(lang.tr("🎤 Modos Vocais", "🎤 Vocal Modes"), |ui| {
                        self.menu_vocal_modes(ui);
                    });

                    ui.collapsing(
                        lang.tr("👤 Cantores & Voicebanks", "👤 Singers & Voicebanks"),
                        |ui| {
                            self.menu_singers(ui);
                        },
                    );

                    ui.collapsing(lang.tr("🛠️ Ferramentas", "🛠️ Tools"), |ui| {
                        self.menu_tools(ui);
                    });

                    ui.collapsing(lang.tr("👁️ Exibir", "👁️ View"), |ui| {
                        self.menu_view(ui, ctx);
                    });

                    ui.collapsing(lang.tr("▶️ Reprodução", "▶️ Playback"), |ui| {
                        self.menu_playback(ui);
                    });

                    ui.collapsing(lang.tr("🌐 Idioma / Language", "🌐 Language"), |ui| {
                        self.menu_language(ui);
                    });

                    ui.separator();

                    if ui
                        .button(lang.tr("📦 Gerenciador de Pacotes", "📦 Package Manager"))
                        .clicked()
                    {
                        self.packages_window_open = true;
                        self.mobile_menu_open = false;
                    }

                    if ui
                        .button(lang.tr("❓ Guia de Atalhos & Ajuda", "❓ Shortcuts & Help"))
                        .clicked()
                    {
                        self.shortcuts_guide_open = true;
                        self.mobile_menu_open = false;
                    }
                });
            });
    }
}
