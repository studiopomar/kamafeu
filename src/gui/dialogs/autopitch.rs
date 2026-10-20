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

                                super::visual::header(
                                    ui,
                                    super::visual::Art::Pitch,
                                    "Pre-tunning",
                                    lang.tr(
                                        "Curvas de afinação, vibrato e expressão vocal.",
                                        "Pitch curves, vibrato, and vocal expression.",
                                    ),
                                    self.config.theme.success_c32(),
                                );

                                ui.add_space(2.0);
                                ui.separator();
                                ui.add_space(2.0);

                                egui::Frame::group(ui.style())
                                    .fill(ui.visuals().extreme_bg_color)
                                    .rounding(egui::Rounding::same(10.0))
                                    .inner_margin(egui::Margin::same(10.0))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.selectable_value(
                                                &mut self.autopitch_use_audio_reference,
                                                false,
                                                lang.tr("Análise musical", "Musical analysis"),
                                            );
                                            ui.selectable_value(
                                                &mut self.autopitch_use_audio_reference,
                                                true,
                                                lang.tr("Referência vocal", "Vocal reference"),
                                            );
                                        });
                                        if self.autopitch_use_audio_reference {
                                            ui.horizontal_wrapped(|ui| {
                                                if ui.button(lang.tr("Escolher WAV…", "Choose WAV…")).clicked() {
                                                    if let Some(path) = crate::dialogs::FileDialog::new().pick_file() {
                                                        self.autopitch_reference_path = Some(path);
                                                        self.autopitch_reference = None;
                                                        self.autopitch_reference_status = None;
                                                    }
                                                }
                                                if let Some(path) = &self.autopitch_reference_path {
                                                    ui.label(path.file_name().and_then(|name| name.to_str()).unwrap_or("WAV"));
                                                    if ui.button(lang.tr("Analisar", "Analyze")).clicked() {
                                                        let result = crate::dsp::autopitch_audio::PitchReference::analyze_wav(path);
                                                        match result {
                                                            Ok(reference) => {
                                                                self.autopitch_reference_status = Some(format!("{} quadros vozeados detectados.", reference.voiced_frames));
                                                                self.autopitch_reference = Some(reference);
                                                            }
                                                            Err(error) => {
                                                                self.autopitch_reference = None;
                                                                self.autopitch_reference_status = Some(error);
                                                            }
                                                        }
                                                    }
                                                }
                                            });
                                            ui.label(lang.tr(
                                                "A curva é alinhada pelo tempo do projeto. Áudio com ruído pode exigir revisão manual.",
                                                "The curve follows project time. Noisy audio may need manual review.",
                                            ));
                                            if let Some(status) = &self.autopitch_reference_status {
                                                ui.label(status);
                                            }
                                        } else {
                                            ui.label(lang.tr(
                                                "A análise considera frase, intervalos, duração e pausas.",
                                                "Analysis considers phrases, intervals, duration, and gaps.",
                                            ));
                                        }
                                        draw_pitch_preview(ui, self.autopitch_reference.as_ref(), &self.autopitch_options);
                                    });

                                ui.add_space(4.0);

                                // --- PRESETS & ESTILO ---
                                ui.label(
                                    egui::RichText::new(lang.tr("ESTILO / PRESET VOCAL:", "VOCAL STYLE / PRESET:"))
                                        .strong()
                                        .size(12.0)
                                        .color(ui.visuals().warn_fg_color),
                                );

                                ui.horizontal_wrapped(|ui| {
                                    for preset in crate::dsp::AutoPitchPreset::all() {
                                        let is_selected = self.autopitch_options.preset == *preset;
                                        let (bg_color, stroke_color, text_color) = if is_selected {
                                            (
                                                ui.visuals().hyperlink_color,
                                                egui::Stroke::new(1.5_f32, self.config.theme.success_c32()),
                                                ui.visuals().hyperlink_color,
                                            )
                                        } else {
                                            (
                                                ui.visuals().panel_fill,
                                                egui::Stroke::new(1.0_f32, ui.visuals().panel_fill),
                                                ui.visuals().text_color().linear_multiply(0.72),
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
                                    .fill(ui.visuals().panel_fill)
                                    .stroke(egui::Stroke::new(1.0_f32, ui.visuals().panel_fill))
                                    .rounding(egui::Rounding::same(4.0))
                                    .inner_margin(egui::Margin::same(8.0))
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(
                                                egui::RichText::new(self.autopitch_options.preset.description_for(lang))
                                                    .size(11.0)
                                                    .italics()
                                                    .color(ui.visuals().text_color()),
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
                                        .color(ui.visuals().warn_fg_color),
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
                                        .color(ui.visuals().warn_fg_color),
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
                                        .color(ui.visuals().warn_fg_color),
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
                                                .color(ui.visuals().text_color()),
                                        )
                                        .fill(self.config.theme.success_c32())
                                        .rounding(egui::Rounding::same(4.0))
                                        .min_size(egui::vec2(160.0, 30.0));

                                        if ui.add_enabled(!self.autopitch_use_audio_reference || self.autopitch_reference.is_some(), apply_btn).clicked() {
                                            trigger_apply = true;
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

            if trigger_apply {
                if self.apply_autopitch() {
                    is_open = false;
                }
            }

            if trigger_close {
                is_open = false;
            }

            self.autopitch_window_open = is_open;
        }
    }
}

fn draw_pitch_preview(
    ui: &mut egui::Ui,
    reference: Option<&crate::dsp::autopitch_audio::PitchReference>,
    options: &crate::dsp::AutoPitchOptions,
) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().max(240.0), 74.0),
        egui::Sense::hover(),
    );
    let painter = ui.painter_at(rect);
    let visuals = ui.visuals();
    painter.rect_filled(rect, 6.0, visuals.panel_fill);
    let center_y = rect.center().y;
    painter.line_segment(
        [
            egui::pos2(rect.left() + 8.0, center_y),
            egui::pos2(rect.right() - 8.0, center_y),
        ],
        egui::Stroke::new(1.0_f32, visuals.widgets.noninteractive.bg_stroke.color),
    );
    let segments: Vec<Vec<(f64, f64)>> = if let Some(reference) = reference {
        let valid: Vec<f32> = reference
            .f0_hz
            .iter()
            .copied()
            .filter(|f0| f0.is_finite() && *f0 > 0.0)
            .collect();
        let mut sorted = valid.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let baseline = sorted
            .get(sorted.len() / 2)
            .copied()
            .unwrap_or(220.0)
            .max(1.0);
        let mut segments = Vec::<Vec<(f64, f64)>>::new();
        let mut segment = Vec::new();
        for (i, f0) in reference.f0_hz.iter().enumerate() {
            if *f0 > 0.0 && f0.is_finite() {
                segment.push((
                    i as f64 * reference.frame_period_ms,
                    1200.0 * (*f0 as f64 / baseline as f64).log2(),
                ));
            } else if !segment.is_empty() {
                segments.push(std::mem::take(&mut segment));
            }
        }
        if !segment.is_empty() {
            segments.push(segment);
        }
        segments
    } else {
        let (bend, _) = crate::dsp::AutoPitchEngine::generate_pitch_for_note(
            60, 650.0, None, false, false, options,
        );
        vec![bend
            .points
            .iter()
            .map(|point| (point.time_offset_ms, point.pitch_offset_cents))
            .collect()]
    };
    if segments.iter().any(|segment| segment.len() > 1) {
        let max_x = segments
            .iter()
            .filter_map(|segment| segment.last().map(|point| point.0))
            .fold(1.0, f64::max);
        let max_y = segments
            .iter()
            .flatten()
            .map(|point| point.1.abs())
            .fold(100.0, f64::max)
            .clamp(100.0, 900.0);
        for segment in segments.iter().filter(|segment| segment.len() > 1) {
            let points: Vec<egui::Pos2> = segment
                .iter()
                .map(|(x, y)| {
                    egui::pos2(
                        egui::lerp(
                            (rect.left() + 8.0)..=(rect.right() - 8.0),
                            (*x / max_x) as f32,
                        ),
                        center_y - (*y as f32 / max_y as f32) * (rect.height() * 0.42),
                    )
                })
                .collect();
            painter.add(egui::Shape::line(
                points,
                egui::Stroke::new(2.0_f32, visuals.hyperlink_color),
            ));
        }
    }
}
