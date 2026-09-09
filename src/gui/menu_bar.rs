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
        TopBottomPanel::top("top_menu_bar")
            .exact_height(26.0)
            .frame(Frame::none().fill(MelodyneTheme::BG_PANEL))
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    // ==========================================
                    // 1. ARQUIVO
                    // ==========================================
                    self.menu_file(ui);

                    // ==========================================
                    // 2. EDITAR
                    // ==========================================
                    self.menu_edit(ui);

                    self.menu_tracks(ui);

                    // ==========================================
                    // 4. MODOS VOCAIS
                    // ==========================================
                    self.menu_vocal_modes(ui);

                    // ==========================================
                    // 5. CANTORES
                    // ==========================================
                    self.menu_singers(ui);

                    // ==========================================
                    // 6. FERRAMENTAS
                    // ==========================================
                    self.menu_tools(ui);

                    // ==========================================
                    // 7. EXIBIR
                    // ==========================================
                    self.menu_view(ui, ctx);

                    // ==========================================
                    // 8. REPRODUÇÃO
                    // ==========================================
                    self.menu_playback(ui);

                    // ==========================================
                    // 9. IDIOMA / LANGUAGE
                    // ==========================================
                    self.menu_language(ui);

                    // ==========================================
                    // 10. AJUDA
                    // ==========================================
                    self.menu_help(ui);

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

                    let (badge_bg, badge_border, badge_fg, badge_icon, badge_tooltip) = if self.is_dirty {
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
                        let version_badge = egui::Button::new(
                            egui::RichText::new("v1.0.0-A")
                                .size(10.0)
                                .strong()
                                .color(egui::Color32::from_rgb(255, 205, 100)),
                        )
                        .fill(egui::Color32::from_rgb(38, 28, 14))
                        .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(200, 140, 40)))
                        .rounding(egui::Rounding::same(4.0));

                        let version_resp = ui.add(version_badge).on_hover_text(lang.tr(
                            "Kamafeu Studio v1.0.0-A (Âmbar)\nMotor de Síntese de Voz & DAW",
                            "Kamafeu Studio v1.0.0-A (Amber)\nVoice Synthesis Engine & DAW",
                        ));
                        if version_resp.clicked() {
                            self.shortcuts_guide_open = true;
                        }
                    });
                });
            });
    }
}
