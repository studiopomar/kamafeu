use crate::gui::KamafeuStudioApp;
use crate::oto::{analyze_voicebank, apply_utau_guide, UtauGuideAnalysis, VoicebankStyle};
use eframe::egui::{self, Color32, Frame, Margin, RichText, Rounding, Stroke, Vec2};

impl KamafeuStudioApp {
    pub(super) fn show_utau_guide(&mut self, ctx: &egui::Context) {
        if !self.utau_guide_window_open {
            return;
        }

        let mut is_open = self.utau_guide_window_open;
        let lang = self.config.language;
        let theme = self.config.theme.clone();

        let analysis = if let Some(vb) = self.voicebank.as_ref() {
            analyze_voicebank(vb)
        } else {
            UtauGuideAnalysis::default()
        };

        let vb_name = self
            .voicebank
            .as_ref()
            .map(|v| v.name.clone())
            .unwrap_or_else(|| lang.tr("Voicebank Padrão", "Default Voicebank").to_string());

        let mut applied = false;
        let mut closed = false;

        egui::Window::new(lang.tr("Guia UTAU - Otimização Acústica", "UTAU Guide - Acoustic Calibration"))
            .open(&mut is_open)
            .collapsible(false)
            .resizable(false)
            .default_size(Vec2::new(500.0, 460.0))
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(theme.bg_panel_c32())
                    .rounding(theme.ui_rounding())
                    .stroke(theme.card_stroke()),
            )
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(8.0, 10.0);

                // Voicebank Analysis Card
                Frame::none()
                    .fill(theme.elevated_surface_c32())
                    .rounding(Rounding::same(6.0))
                    .stroke(theme.card_stroke())
                    .inner_margin(Margin::same(12.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("🎤 {}", vb_name))
                                    .size(13.0)
                                    .strong()
                                    .color(theme.text_primary_c32()),
                            );

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let style_label = if lang == crate::config::AppLanguage::PtBr {
                                    analysis.style.label()
                                } else {
                                    analysis.style.label_en()
                                };

                                let (badge_fill, badge_stroke) = match analysis.style {
                                    VoicebankStyle::Vcv => (theme.c32_alpha(theme.accent_color, 0.20), theme.accent_c32()),
                                    VoicebankStyle::Cv => (Color32::from_rgba_unmultiplied(80, 180, 240, 45), Color32::from_rgb(80, 180, 240)),
                                    VoicebankStyle::Cvvc => (Color32::from_rgba_unmultiplied(240, 160, 60, 45), Color32::from_rgb(240, 160, 60)),
                                    VoicebankStyle::Unknown => (Color32::from_rgba_unmultiplied(128, 128, 128, 40), Color32::GRAY),
                                };

                                Frame::none()
                                    .fill(badge_fill)
                                    .rounding(Rounding::same(4.0))
                                    .stroke(Stroke::new(1.0_f32, badge_stroke))
                                    .inner_margin(Margin::symmetric(8.0, 3.0))
                                    .show(ui, |ui| {
                                        ui.label(RichText::new(style_label).size(10.5).strong().color(badge_stroke));
                                    });
                            });
                        });

                        ui.add_space(4.0);
                        ui.separator();
                        ui.add_space(4.0);

                        ui.columns(3, |cols| {
                            cols[0].vertical(|ui| {
                                ui.label(RichText::new(lang.tr("Pré-prolação (Preutter)", "Preutterance")).size(9.5).color(theme.text_muted_c32()));
                                ui.label(RichText::new(format!("{:.1} ms", analysis.median_preutterance_ms)).size(13.0).strong().color(theme.text_primary_c32()));
                            });
                            cols[1].vertical(|ui| {
                                ui.label(RichText::new(lang.tr("Sobreposição (Overlap)", "Overlap")).size(9.5).color(theme.text_muted_c32()));
                                ui.label(RichText::new(format!("{:.1} ms", analysis.median_overlap_ms)).size(13.0).strong().color(theme.text_primary_c32()));
                            });
                            cols[2].vertical(|ui| {
                                ui.label(RichText::new(lang.tr("Consoante (Fixo)", "Consonant (Fixed)")).size(9.5).color(theme.text_muted_c32()));
                                ui.label(RichText::new(format!("{:.1} ms", analysis.median_consonant_ms)).size(13.0).strong().color(theme.text_primary_c32()));
                            });
                        });

                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(lang.tr(
                                "O Guia analisa a geometria do oto.ini deste voicebank para alinhar portamento, onset e transições vocais perfeitas sem desafinação mecânica.",
                                "The Guide inspects the oto.ini geometry of this voicebank to align portamento, onset and seamless vocal transitions without robotic artifacts.",
                            ))
                            .size(10.0)
                            .color(theme.text_muted_c32()),
                        );
                    });

                // Configuration Parameters
                Frame::none()
                    .fill(theme.elevated_surface_c32())
                    .rounding(Rounding::same(6.0))
                    .stroke(theme.card_stroke())
                    .inner_margin(Margin::same(12.0))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(lang.tr("Opções de Calibração", "Calibration Options"))
                                .size(11.5)
                                .strong()
                                .color(theme.accent_c32()),
                        );
                        ui.add_space(4.0);

                        ui.horizontal(|ui| {
                            ui.label(RichText::new(lang.tr("Intensidade / Força:", "Strength:")).size(10.5));
                            ui.add(
                                egui::Slider::new(&mut self.utau_guide_options.strength, 0.4..=1.8)
                                    .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
                            );
                        });

                        ui.add_space(4.0);
                        ui.checkbox(
                            &mut self.utau_guide_options.apply_pitch_portamento,
                            RichText::new(lang.tr(
                                "Configurar Portamento & Pitch (curvas alinhadas às vogais do banco)",
                                "Configure Portamento & Pitch (vowel-aligned curves for this bank)",
                            )).size(10.5),
                        );

                        ui.checkbox(
                            &mut self.utau_guide_options.apply_envelope_shape,
                            RichText::new(lang.tr(
                                "Ajustar Envelopes & Ataque (Shape calibrado para evitar estalos)",
                                "Adjust Envelope & Attack (Calibrated Shape to avoid pops/clipping)",
                            )).size(10.5),
                        );

                        ui.checkbox(
                            &mut self.utau_guide_options.apply_consonant_velocity,
                            RichText::new(lang.tr(
                                "Otimizar Velocidade de Consoante (Consonant Velocity)",
                                "Optimize Consonant Velocity",
                            )).size(10.5),
                        );

                        ui.checkbox(
                            &mut self.utau_guide_options.apply_timing,
                            RichText::new(lang.tr(
                                "Compensar Timing de Pré-prolação (Preutterance Offset)",
                                "Compensate Preutterance Timing (Preutterance Offset)",
                            )).size(10.5),
                        );
                    });

                ui.add_space(6.0);

                // Footer Buttons
                ui.horizontal(|ui| {
                    if ui.button(lang.tr("Cancelar", "Cancel")).clicked() {
                        closed = true;
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let apply_btn = egui::Button::new(
                            RichText::new(lang.tr("✨ Aplicar Guia UTAU", "✨ Apply UTAU Guide"))
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(theme.accent_c32())
                        .rounding(Rounding::same(4.0));

                        if ui.add(apply_btn).clicked() {
                            applied = true;
                        }
                    });
                });
            });

        if closed {
            self.utau_guide_window_open = false;
        }

        if applied {
            let pre_project = self.project.clone();
            let sel_indices = self.piano_roll_state.selected_note_indices.clone();
            let mut affected_count = 0usize;

            if let Some(part) = self.project.parts.get_mut(self.active_track_index) {
                if !sel_indices.is_empty() {
                    let selected_notes: Vec<&mut crate::project::model::UNote> = part
                        .notes
                        .iter_mut()
                        .enumerate()
                        .filter(|(idx, _)| sel_indices.contains(idx))
                        .map(|(_, n)| n)
                        .collect();
                    affected_count = selected_notes.len();
                    // Temporary buffer to run apply_utau_guide
                    let mut buf: Vec<crate::project::model::UNote> =
                        selected_notes.iter().map(|n| (*n).clone()).collect();
                    apply_utau_guide(&mut buf, &analysis, &self.utau_guide_options);
                    for (dst, src) in selected_notes.into_iter().zip(buf.into_iter()) {
                        *dst = src;
                    }
                } else {
                    affected_count = part.notes.len();
                    apply_utau_guide(&mut part.notes, &analysis, &self.utau_guide_options);
                }
            }

            self.undo_manager.push_state(pre_project);
            self.is_dirty = true;
            self.utau_guide_window_open = false;
            self.transport_state.status_message = lang
                .tr(
                    &format!(
                        "Guia UTAU aplicado com sucesso: {} notas otimizadas!",
                        affected_count
                    ),
                    &format!(
                        "UTAU Guide applied successfully: {} notes optimized!",
                        affected_count
                    ),
                )
                .to_string();
        } else {
            self.utau_guide_window_open = is_open;
        }
    }
}
