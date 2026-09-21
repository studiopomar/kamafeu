use crate::gui::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(super) fn show_autopitch(&mut self, ctx: &egui::Context) {
        if self.autopitch_window_open {
            let mut is_open = self.autopitch_window_open;
            let mut trigger_apply = false;
            let mut trigger_close = false;
            let lang = self.config.language;

            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("autopitch_native_viewport"),
                egui::ViewportBuilder::default()
                    .with_title(format!(
                        "{} - Kamafeu",
                        lang.tr("Pre-tunning - Afinador & Expressão", "Pre-tunning - Tuner & Expression")
                    ))
                    .with_inner_size([620.0, 650.0])
                    .with_min_inner_size([520.0, 460.0]),
                |ctx, _class| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

                                ui.horizontal(|ui| {
                                    ui.heading(
                                        egui::RichText::new("Pre-tunning")
                                            .strong()
                                            .size(18.0)
                                            .color(egui::Color32::from_rgb(0, 255, 180)),
                                    );
                                });
                                ui.label(
                                    egui::RichText::new(
                                        lang.tr(
                                            "Modelagem de curvas de afinação orgânicas, vibrato humano e parâmetros de expressão vocal.",
                                            "Organic pitch curve modeling, human vibrato, and vocal expression parameters."
                                        ),
                                    )
                                    .size(11.0)
                                    .color(egui::Color32::from_rgb(180, 175, 200)),
                                );

                                ui.add_space(2.0);
                                ui.separator();
                                ui.add_space(2.0);

                                // --- PRESETS & ESTILO ---
                                ui.label(
                                    egui::RichText::new(lang.tr("ESTILO / PRESET VOCAL:", "VOCAL STYLE / PRESET:"))
                                        .strong()
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(255, 215, 0)),
                                );

                                ui.horizontal_wrapped(|ui| {
                                    for preset in crate::dsp::AutoPitchPreset::all() {
                                        let is_selected = self.autopitch_options.preset == *preset;
                                        let (bg_color, stroke_color, text_color) = if is_selected {
                                            (
                                                egui::Color32::from_rgb(50, 40, 75),
                                                egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(0, 255, 180)),
                                                egui::Color32::from_rgb(0, 255, 220),
                                            )
                                        } else {
                                            (
                                                egui::Color32::from_rgb(26, 20, 36),
                                                egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(50, 40, 65)),
                                                egui::Color32::from_rgb(200, 195, 215),
                                            )
                                        };

                                        let btn_label = preset.display_name(lang);
                                        let btn = egui::Button::new(
                                            egui::RichText::new(btn_label)
                                                .strong()
                                                .size(11.0)
                                                .color(text_color),
                                        )
                                        .fill(bg_color)
                                        .stroke(stroke_color)
                                        .rounding(egui::Rounding::same(4.0))
                                        .min_size(egui::vec2(95.0, 28.0));

                                        if ui.add(btn).clicked() {
                                            self.autopitch_options.preset = *preset;
                                        }
                                    }
                                });

                                egui::Frame::none()
                                    .fill(egui::Color32::from_rgb(20, 16, 28))
                                    .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(45, 38, 60)))
                                    .rounding(egui::Rounding::same(4.0))
                                    .inner_margin(egui::Margin::same(8.0))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(self.autopitch_options.preset.description_for(lang))
                                                    .size(11.0)
                                                    .italics()
                                                    .color(egui::Color32::from_rgb(220, 215, 240)),
                                            );
                                        });
                                    });

                                ui.add_space(2.0);
                                ui.separator();
                                ui.add_space(2.0);

                                // --- CURVAS DE AFINAÇÃO & VIBRATO ---
                                ui.label(
                                    egui::RichText::new(lang.tr("CURVAS DE AFINAÇÃO & VIBRATO:", "PITCH CURVES & VIBRATO:"))
                                        .strong()
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(255, 215, 0)),
                                );

                                ui.horizontal(|ui| {
                                    ui.label(lang.tr("Intensidade Global de Curvas:", "Global Curve Intensity:"));
                                    let mut pct = (self.autopitch_options.intensity * 100.0).round() as i32;
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut pct, 0..=200)
                                                .suffix("%")
                                                .show_value(true),
                                        )
                                        .changed()
                                    {
                                        self.autopitch_options.intensity = pct as f64 / 100.0;
                                    }
                                });

                                egui::Grid::new("autopitch_checkboxes_grid")
                                    .spacing([24.0, 6.0])
                                    .show(ui, |ui| {
                                        ui.checkbox(
                                            &mut self.autopitch_options.enable_attack_scoop,
                                            lang.tr("Attack Scoop (Ataque inicial)", "Attack Scoop (Initial Attack)"),
                                        )
                                        .on_hover_text(lang.tr(
                                            "Inicia notas de começo de frase ligeiramente abaixo do tom, subindo suavemente.",
                                            "Starts phrase-initial notes slightly flat, scooping up smoothly."
                                        ));

                                        ui.checkbox(
                                            &mut self.autopitch_options.enable_overshoot,
                                            lang.tr("Overshoot de Portamento", "Portamento Overshoot"),
                                        )
                                        .on_hover_text(lang.tr(
                                            "Ultrapassa levemente o tom em saltos ascendentes antes de estabilizar.",
                                            "Slightly overshoots target pitch on upward leaps before settling."
                                        ));
                                        ui.end_row();

                                        ui.checkbox(
                                            &mut self.autopitch_options.enable_release_drop,
                                            lang.tr("Release Drop (Queda final)", "Release Drop (Final Drop)"),
                                        )
                                        .on_hover_text(lang.tr(
                                            "Queda natural de afinação no final de frases/silêncios.",
                                            "Natural pitch drop at phrase ends / silences."
                                        ));

                                        ui.checkbox(
                                            &mut self.autopitch_options.enable_vibrato,
                                            lang.tr("Vibrato Inteligente", "Smart Vibrato"),
                                        )
                                        .on_hover_text(lang.tr(
                                            "Aplica vibrato natural com fade-in e período adequado ao estilo.",
                                            "Applies natural vibrato with fade-in and period tuned to style."
                                        ));
                                        ui.end_row();
                                    });

                                ui.collapsing(lang.tr("Ajustes Finos de Portamento & Vibrato", "Fine Portamento & Vibrato Tuning"), |ui| {
                                    ui.horizontal(|ui| {
                                        ui.label(lang.tr("Tempo de Portamento (ms):", "Portamento Speed (ms):"));
                                        let mut p_ms = self.autopitch_options.portamento_speed_ms.round() as i32;
                                        let auto_str = lang.tr(" (Automático)", " (Auto)");
                                        let text_suffix = if p_ms == 0 { auto_str } else { " ms" };
                                        if ui.add(egui::Slider::new(&mut p_ms, 0..=250).suffix(text_suffix)).changed() {
                                            self.autopitch_options.portamento_speed_ms = p_ms as f64;
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label(lang.tr("Profundidade de Vibrato:", "Vibrato Depth:"));
                                        let mut v_depth = (self.autopitch_options.vibrato_depth_mult * 100.0).round() as i32;
                                        if ui.add(egui::Slider::new(&mut v_depth, 0..=200).suffix("%")).changed() {
                                            self.autopitch_options.vibrato_depth_mult = v_depth as f64 / 100.0;
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label(lang.tr("Velocidade de Vibrato:", "Vibrato Speed:"));
                                        let mut v_speed = (self.autopitch_options.vibrato_period_mult * 100.0).round() as i32;
                                        if ui.add(egui::Slider::new(&mut v_speed, 50..=200).suffix("%")).changed() {
                                            self.autopitch_options.vibrato_period_mult = v_speed as f64 / 100.0;
                                        }
                                    });
                                });

                                ui.add_space(2.0);
                                ui.separator();
                                ui.add_space(2.0);

                                // --- PARÂMETROS DE EXPRESSÃO VOCAL ---
                                ui.label(
                                    egui::RichText::new(lang.tr(
                                        "PARÂMETROS DE EXPRESSÃO VOCAL (DYNAMICS, AR & ARTICULAÇÃO):",
                                        "VOCAL EXPRESSION PARAMETERS (DYNAMICS, BREATH & ARTICULATION):"
                                    ))
                                        .strong()
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(255, 215, 0)),
                                );

                                ui.horizontal(|ui| {
                                    ui.label(lang.tr("Intensidade das Expressões:", "Expression Intensity:"));
                                    let mut exp_pct = (self.autopitch_options.expression_amount * 100.0).round() as i32;
                                    if ui
                                        .add(
                                            egui::Slider::new(&mut exp_pct, 0..=200)
                                                .suffix("%")
                                                .show_value(true),
                                        )
                                        .changed()
                                    {
                                        self.autopitch_options.expression_amount = exp_pct as f64 / 100.0;
                                    }
                                });

                                egui::Grid::new("autopitch_expression_params_grid")
                                    .spacing([24.0, 6.0])
                                    .show(ui, |ui| {
                                        ui.checkbox(
                                            &mut self.autopitch_options.enable_dynamics_shaping,
                                            lang.tr("Dinâmica & Volume Orgânico", "Organic Dynamics & Volume"),
                                        )
                                        .on_hover_text(lang.tr(
                                            "Modela a curva de dinâmica (DYN) e volume baseando-se no registro de tom, agudos e ênfase.",
                                            "Shapes dynamics (DYN) and volume curves based on pitch register and emphasis."
                                        ));

                                        ui.checkbox(
                                            &mut self.autopitch_options.enable_breathiness,
                                            lang.tr("Ar & Respiração Natural (BRE)", "Natural Breathiness (BRE)"),
                                        )
                                        .on_hover_text(lang.tr(
                                            "Insere suspiro e ar dinâmico de acordo com o estilo e inícios/fins de frase.",
                                            "Injects dynamic breathiness according to style and phrase boundaries."
                                        ));
                                        ui.end_row();

                                        ui.checkbox(
                                            &mut self.autopitch_options.enable_consonant_velocity,
                                            lang.tr("Velocidade de Consoante (VEL)", "Consonant Velocity (VEL)"),
                                        )
                                        .on_hover_text(lang.tr(
                                            "Ajusta a articulação consoante de acordo com o estilo e andamento de notas rápidas.",
                                            "Adjusts consonant articulation for fast phrases and stylistic pacing."
                                        ));

                                        ui.checkbox(
                                            &mut self.autopitch_options.enable_attack_decay,
                                            lang.tr("Ataque & Decaimento Vocal (ATK/DEC)", "Vocal Attack & Decay (ATK/DEC)"),
                                        )
                                        .on_hover_text(lang.tr(
                                            "Ajusta punch de ataque em consoantes e decaimento suave em finais de frase.",
                                            "Adjusts attack punch on consonants and soft decay at phrase endings."
                                        ));
                                        ui.end_row();
                                    });

                                ui.add_space(4.0);
                                ui.separator();
                                ui.add_space(2.0);

                                // --- ESCOPO ---
                                ui.label(
                                    egui::RichText::new(lang.tr("APLICAR EM:", "APPLY TO:"))
                                        .strong()
                                        .size(12.0)
                                        .color(egui::Color32::from_rgb(255, 215, 0)),
                                );

                                let sel_count = self.piano_roll_state.selected_note_indices.len();
                                ui.horizontal(|ui| {
                                    let sel_label = if sel_count > 0 {
                                        format!("{} ({} {})", lang.tr("Notas Selecionadas", "Selected Notes"), sel_count, lang.tr("notas", "notes"))
                                    } else if self.piano_roll_state.selected_note_index.is_some() {
                                        format!("{} (1 {})", lang.tr("Nota Selecionada", "Selected Note"), lang.tr("nota", "note"))
                                    } else {
                                        format!("{} ({})", lang.tr("Notas Selecionadas", "Selected Notes"), lang.tr("Nenhuma selecionada", "None selected"))
                                    };

                                    ui.radio_value(
                                        &mut self.autopitch_scope,
                                        crate::dsp::AutoPitchScope::SelectedOnly,
                                        sel_label,
                                    );

                                    ui.radio_value(
                                        &mut self.autopitch_scope,
                                        crate::dsp::AutoPitchScope::AllNotes,
                                        lang.tr("Todas as Notas da Faixa", "All Track Notes"),
                                    );
                                });

                                ui.add_space(8.0);
                                ui.separator();
                                ui.add_space(4.0);

                                // --- BOTÕES DE AÇÃO ---
                                ui.horizontal(|ui| {
                                    if ui.button(lang.tr("Cancelar", "Cancel")).clicked() {
                                        trigger_close = true;
                                    }

                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        let apply_btn = egui::Button::new(
                                            egui::RichText::new(lang.tr("Aplicar Pre-tunning", "Apply Pre-tunning"))
                                                .strong()
                                                .size(13.0)
                                                .color(egui::Color32::BLACK),
                                        )
                                        .fill(egui::Color32::from_rgb(0, 255, 180))
                                        .rounding(egui::Rounding::same(4.0))
                                        .min_size(egui::vec2(160.0, 30.0));

                                        if ui.add(apply_btn).clicked() {
                                            trigger_apply = true;
                                            trigger_close = true;
                                        }
                                    });
                                });
                            });
                    });
                    if ctx.input(|i| i.viewport().close_requested()) {
                        trigger_close = true;
                    }
                },
            );

            if trigger_close {
                is_open = false;
            }

            if trigger_apply {
                self.apply_autopitch();
            }

            self.autopitch_window_open = is_open;
        }
    }
}
