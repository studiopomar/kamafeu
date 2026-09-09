use super::*;

pub(super) fn draw(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    lang: crate::config::AppLanguage,
    voicebank: Option<&Voicebank>,
    selected_note_idx: Option<usize>,
    notes: &mut [UNote],
    selected_indices: &std::collections::HashSet<usize>,
    selected_ruler_alias: Option<&str>,
    selected_resampler: &mut String,
    selected_wavtool: &mut String,
    on_edit_selected_ruler_alias: &mut dyn FnMut(),
) {
    egui::ScrollArea::vertical()
        .id_salt("right_panel_note_scroll")
        .show(ui, |ui| {
            if let Some(target_idx) = selected_note_idx {
                if target_idx < notes.len() {
                    let is_group = selected_indices.len() > 1;
                    if is_group {
                        Frame::none()
                            .fill(theme.c32_alpha(theme.accent_color, 0.2))
                            .rounding(theme.ui_rounding())
                            .stroke(Stroke::new(1.0_f32, theme.accent_c32()))
                            .inner_margin(egui::Margin::symmetric(8.0, 6.0))
                            .show(ui, |ui| {
                                ui.label(
                                    RichText::new(format!(
                                        "{} ({} {})",
                                        lang.tr("Multi-Seleção", "Multi-Selection"),
                                        selected_indices.len(),
                                        lang.tr("notas", "notes")
                                    ))
                                    .strong()
                                    .size(11.0)
                                    .color(theme.accent_c32()),
                                );
                                ui.label(
                                    RichText::new(lang.tr(
                                        "Alterações aplicam-se a todas as notas selecionadas.",
                                        "Changes apply to all selected notes.",
                                    ))
                                    .size(9.5)
                                    .color(theme.text_muted_c32()),
                                );
                            });
                        ui.add_space(6.0);
                    }

                    let mut lyric = notes[target_idx].lyric.clone();
                    let pitch_str = notes[target_idx].pitch.clone();
                    let pos_ms = notes[target_idx].position_ms;
                    let mut dur_ms = notes[target_idx].duration_ms;
                    let mut flags = notes[target_idx].flags.clone();

                    let mut gender = notes[target_idx].expressions.gender;
                    let mut dynamics = notes[target_idx].expressions.dynamics;
                    let mut pitch_delta = notes[target_idx].expressions.pitch_delta;
                    let mut breathiness = notes[target_idx].expressions.breathiness;
                    let mut velocity = notes[target_idx].expressions.velocity;
                    let mut consonant_velocity =
                        notes[target_idx].expressions.consonant_velocity;
                    let mut modulation = notes[target_idx].expressions.modulation;
                    let mut volume = notes[target_idx].expressions.volume;
                    let mut attack = notes[target_idx].expressions.attack;
                    let mut decay = notes[target_idx].expressions.decay;
                    let mut consonant_timing =
                        notes[target_idx].expressions.consonant_timing_offset_ms;

                    let mut fade_in_ms = notes[target_idx].envelope.p2;
                    let mut fade_out_ms = notes[target_idx].envelope.p5;
                    let mut note_crossfade_ms = notes[target_idx].envelope.crossfade_ms;

                    let mut vibrato = notes[target_idx].vibrato.clone();
                    let mut portamento_start =
                        notes[target_idx].pitch_bend.portamento_start_ms;
                    let mut portamento_length =
                        notes[target_idx].pitch_bend.portamento_length_ms;
                    let mut portamento_shape =
                        notes[target_idx].pitch_bend.portamento_shape.clone();
                    let mut snap_first = notes[target_idx].pitch_bend.snap_first;

                    let mut changed_lyric = false;
                    let mut changed_dur = false;
                    let mut changed_flags = false;
                    let mut changed_gender = false;
                    let mut changed_dynamics = false;
                    let mut changed_pitch = false;
                    let mut changed_breath = false;
                    let mut changed_vel = false;
                    let mut changed_c_vel = false;
                    let mut changed_mod = false;
                    let mut changed_c_timing = false;
                    let mut changed_amplitude = false;
                    let mut changed_fades = false;
                    let mut changed_vibrato = false;
                    let mut changed_portamento = false;
                    let mut reset_all = false;

                    // --- 1. PROPRIEDADES PRINCIPAIS ---
                    Frame::none()
                        .fill(theme.card_bg_c32())
                        .rounding(theme.ui_rounding())
                        .stroke(theme.card_stroke())
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(lang.tr("Propriedades Principais", "Main Properties"))
                                    .strong()
                                    .size(11.0)
                                    .color(theme.accent_c32()),
                            );
                            ui.separator();

                            ui.columns(2, |cols| {
                                cols[0].horizontal(|ui| {
                                    ui.label(RichText::new(lang.tr("Letra:", "Lyric:")).size(10.5));
                                    let lyric_resp = ui.add_sized(
                                        Vec2::new(ui.available_width(), 18.0),
                                        egui::TextEdit::singleline(&mut lyric),
                                    );
                                    if lyric_resp.changed() {
                                        changed_lyric = true;
                                    }
                                });

                                cols[1].horizontal(|ui| {
                                    ui.label(RichText::new(lang.tr("Tom:", "Pitch:")).size(10.5));
                                    ui.label(
                                        RichText::new(&pitch_str)
                                            .strong()
                                            .size(11.0)
                                            .color(theme.accent_c32()),
                                    );
                                });
                            });

                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(lang.tr("Flags UTAU:", "UTAU Flags:")).size(10.0));
                                let flags_resp = ui.add_sized(
                                    Vec2::new(ui.available_width(), 18.0),
                                    egui::TextEdit::singleline(&mut flags)
                                        .hint_text("ex: g-5BRE50"),
                                );
                                if flags_resp.changed() {
                                    changed_flags = true;
                                }
                            });

                            ui.add_space(2.0);
                            ui.columns(2, |cols| {
                                cols[0].horizontal(|ui| {
                                    ui.label(RichText::new(lang.tr("Início:", "Start:")).size(10.0));
                                    ui.label(
                                        RichText::new(format!("{:.0} ms", pos_ms))
                                            .monospace()
                                            .size(10.0)
                                            .color(theme.text_muted_c32()),
                                    );
                                });

                                cols[1].horizontal(|ui| {
                                    ui.label(RichText::new(lang.tr("Duração:", "Duration:")).size(10.0));
                                    let dur_resp = ui.add_sized(
                                        Vec2::new(ui.available_width(), 18.0),
                                        egui::DragValue::new(&mut dur_ms)
                                            .range(10.0..=10000.0)
                                            .speed(5.0)
                                            .suffix(" ms"),
                                    );
                                    if dur_resp.changed() {
                                        changed_dur = true;
                                    }
                                });
                            });

                            if selected_ruler_alias.is_some() {
                                ui.add_space(4.0);
                                let ruler_name = selected_ruler_alias.unwrap_or("");
                                if ui
                                    .add_sized(
                                        Vec2::new(ui.available_width(), 20.0),
                                        egui::Button::new(
                                            RichText::new(lang.tr("Editar no Copaiba NEO", "Edit in Copaiba NEO"))
                                                .size(9.5)
                                                .color(Color32::from_rgb(0, 220, 255)),
                                        ),
                                    )
                                    .on_hover_text(format!("{} '{}' {}", lang.tr("Editar alias", "Edit alias"), ruler_name, lang.tr("no editor Copaiba NEO", "in Copaiba NEO editor")))
                                    .clicked()
                                {
                                    on_edit_selected_ruler_alias();
                                }
                            }

                            ui.add_space(4.0);
                            if ui
                                .add_sized(
                                    Vec2::new(ui.available_width(), 22.0),
                                    egui::Button::new(
                                        RichText::new(lang.tr("Resetar Parâmetros da Nota", "Reset Note Parameters"))
                                            .size(10.0)
                                            .color(Color32::from_rgb(255, 180, 90)),
                                    ),
                                )
                                .on_hover_text(lang.tr(
                                    "Restaura envelopes UTAU, vibrato, curvas de afinação, flags e todas as expressões aos valores padrão",
                                    "Restores UTAU envelopes, vibrato, pitch curves, flags and expressions to default values",
                                ))
                                .clicked()
                            {
                                reset_all = true;
                            }
                        });

                    ui.add_space(6.0);

                    // --- 2. EXPRESSÕES & DINÂMICA (DYN, GEN, PITD, VEL, BRE) ---
                    Frame::none()
                        .fill(theme.card_bg_c32())
                        .rounding(theme.ui_rounding())
                        .stroke(theme.card_stroke())
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(lang.tr("Expressões & Articulação", "Expressions & Articulation"))
                                    .strong()
                                    .size(11.0)
                                    .color(theme.accent_c32()),
                            );
                            ui.separator();

                            ui.horizontal(|ui| {
                                ui.label(RichText::new(lang.tr("Dinâmica (DYN):", "Dynamics (DYN):")).size(10.0));
                                let dyn_s = ui.add_sized(
                                    Vec2::new(ui.available_width(), 18.0),
                                    egui::Slider::new(&mut dynamics, -240.0..=120.0)
                                        .suffix(" 0.1dB"),
                                );
                                if dyn_s.changed() {
                                    changed_dynamics = true;
                                }
                                if dyn_s.clicked_by(egui::PointerButton::Secondary) {
                                    dynamics = 0.0;
                                    changed_dynamics = true;
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label(RichText::new(lang.tr("Gênero (GEN):", "Gender (GEN):")).size(10.0));
                                let gen_s = ui.add_sized(
                                    Vec2::new(ui.available_width(), 18.0),
                                    egui::Slider::new(&mut gender, -100.0..=100.0)
                                        .suffix(" %"),
                                );
                                if gen_s.changed() {
                                    changed_gender = true;
                                }
                                if gen_s.clicked_by(egui::PointerButton::Secondary) {
                                    gender = 0.0;
                                    changed_gender = true;
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label(RichText::new(lang.tr("Pitch (PITD):", "Pitch (PITD):")).size(10.0));
                                let pit_s = ui.add_sized(
                                    Vec2::new(ui.available_width(), 18.0),
                                    egui::Slider::new(&mut pitch_delta, -100.0..=100.0)
                                        .suffix(" c"),
                                );
                                if pit_s.changed() {
                                    changed_pitch = true;
                                }
                                if pit_s.clicked_by(egui::PointerButton::Secondary) {
                                    pitch_delta = 0.0;
                                    changed_pitch = true;
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label(RichText::new(lang.tr("Sopro (BRE):", "Breath (BRE):")).size(10.0));
                                let bre_s = ui.add_sized(
                                    Vec2::new(ui.available_width(), 18.0),
                                    egui::Slider::new(&mut breathiness, -100.0..=100.0)
                                        .suffix(" %"),
                                );
                                if bre_s.changed() {
                                    changed_breath = true;
                                }
                                if bre_s.clicked_by(egui::PointerButton::Secondary) {
                                    breathiness = 0.0;
                                    changed_breath = true;
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label(RichText::new(lang.tr("Velocidade (VEL):", "Velocity (VEL):")).size(10.0));
                                let vel_s = ui.add_sized(
                                    Vec2::new(ui.available_width(), 18.0),
                                    egui::Slider::new(&mut velocity, 0.0..=200.0)
                                        .suffix(" %"),
                                );
                                if vel_s.changed() {
                                    changed_vel = true;
                                }
                                if vel_s.clicked_by(egui::PointerButton::Secondary) {
                                    velocity = 100.0;
                                    changed_vel = true;
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label(RichText::new(lang.tr("Vel. Consoante:", "Consonant Vel.:")).size(10.0));
                                let cvel_s = ui.add_sized(
                                    Vec2::new(ui.available_width(), 18.0),
                                    egui::Slider::new(
                                        &mut consonant_velocity,
                                        0.0..=200.0,
                                    )
                                    .suffix(" %"),
                                );
                                if cvel_s.changed() {
                                    changed_c_vel = true;
                                }
                                if cvel_s.clicked_by(egui::PointerButton::Secondary) {
                                    consonant_velocity = 100.0;
                                    changed_c_vel = true;
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label(RichText::new(lang.tr("Modulação (MOD):", "Modulation (MOD):")).size(10.0));
                                let mod_s = ui.add_sized(
                                    Vec2::new(ui.available_width(), 18.0),
                                    egui::Slider::new(&mut modulation, 0.0..=200.0)
                                        .suffix(" %"),
                                );
                                if mod_s.changed() {
                                    changed_mod = true;
                                }
                                if mod_s.clicked_by(egui::PointerButton::Secondary) {
                                    modulation = 0.0;
                                    changed_mod = true;
                                }
                            });

                            ui.horizontal(|ui| {
                                ui.label(RichText::new(lang.tr("Shift Consoante:", "Consonant Shift:")).size(10.0));
                                let ct_s = ui.add_sized(
                                    Vec2::new(ui.available_width(), 18.0),
                                    egui::Slider::new(
                                        &mut consonant_timing,
                                        -200.0..=200.0,
                                    )
                                    .suffix(" ms"),
                                );
                                if ct_s.changed() {
                                    changed_c_timing = true;
                                }
                                if ct_s.clicked_by(egui::PointerButton::Secondary) {
                                    consonant_timing = 0.0;
                                    changed_c_timing = true;
                                }
                            });
                        });

                    ui.add_space(6.0);

                    // --- 3. ENVELOPE, VOL, ATK, DEC & CROSSFADE ---
                    Frame::none()
                        .fill(theme.card_bg_c32())
                        .rounding(theme.ui_rounding())
                        .stroke(theme.card_stroke())
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(lang.tr("Envelope & Fades", "Envelope & Fades"))
                                    .strong()
                                    .size(11.0)
                                    .color(theme.accent_c32()),
                            );
                            ui.separator();

                            ui.label(RichText::new(lang.tr("Volume / Attack / Decay:", "Volume / Attack / Decay:")).size(10.0));
                            ui.columns(3, |cols| {
                                changed_amplitude |= cols[0]
                                    .add_sized(
                                        Vec2::new(cols[0].available_width(), 18.0),
                                        egui::DragValue::new(&mut volume)
                                            .range(0.0..=200.0)
                                            .prefix("V:"),
                                    )
                                    .changed();
                                changed_amplitude |= cols[1]
                                    .add_sized(
                                        Vec2::new(cols[1].available_width(), 18.0),
                                        egui::DragValue::new(&mut attack)
                                            .range(0.0..=200.0)
                                            .prefix("A:"),
                                    )
                                    .changed();
                                changed_amplitude |= cols[2]
                                    .add_sized(
                                        Vec2::new(cols[2].available_width(), 18.0),
                                        egui::DragValue::new(&mut decay)
                                            .range(0.0..=100.0)
                                            .prefix("D:"),
                                    )
                                    .changed();
                            });

                            ui.add_space(2.0);
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Fade-In:").size(10.0));
                                changed_fades |= ui
                                    .add_sized(
                                        Vec2::new(ui.available_width(), 18.0),
                                        egui::Slider::new(
                                            &mut fade_in_ms,
                                            0.0..=300.0,
                                        )
                                        .suffix(" ms"),
                                    )
                                    .changed();
                            });

                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Fade-Out:").size(10.0));
                                changed_fades |= ui
                                    .add_sized(
                                        Vec2::new(ui.available_width(), 18.0),
                                        egui::Slider::new(
                                            &mut fade_out_ms,
                                            0.0..=300.0,
                                        )
                                        .suffix(" ms"),
                                    )
                                    .changed();
                            });

                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Crossfade:").size(10.0));
                                changed_fades |= ui
                                    .add_sized(
                                        Vec2::new(ui.available_width(), 18.0),
                                        egui::Slider::new(
                                            &mut note_crossfade_ms,
                                            0.0..=300.0,
                                        )
                                        .suffix(" ms"),
                                    )
                                    .changed();
                            });

                            ui.add_space(2.0);
                            ui.columns(3, |cols| {
                                if cols[0]
                                    .add_sized(
                                        Vec2::new(cols[0].available_width(), 19.0),
                                        egui::Button::new(RichText::new(lang.tr("Suave (50ms)", "Smooth (50ms)")).size(9.0)),
                                    )
                                    .clicked()
                                {
                                    fade_in_ms = 50.0;
                                    fade_out_ms = 50.0;
                                    note_crossfade_ms = 50.0;
                                    changed_fades = true;
                                }
                                if cols[1]
                                    .add_sized(
                                        Vec2::new(cols[1].available_width(), 19.0),
                                        egui::Button::new(RichText::new(lang.tr("Seco (5ms)", "Dry (5ms)")).size(9.0)),
                                    )
                                    .clicked()
                                {
                                    fade_in_ms = 5.0;
                                    fade_out_ms = 5.0;
                                    note_crossfade_ms = 5.0;
                                    changed_fades = true;
                                }
                                if cols[2]
                                    .add_sized(
                                        Vec2::new(cols[2].available_width(), 19.0),
                                        egui::Button::new(RichText::new("Auto").size(9.0)),
                                    )
                                    .clicked()
                                {
                                    note_crossfade_ms = 0.0;
                                    changed_fades = true;
                                }
                            });
                        });

                    ui.add_space(6.0);

                    // --- 4. PORTAMENTO ---
                    Frame::none()
                        .fill(theme.card_bg_c32())
                        .rounding(theme.ui_rounding())
                        .stroke(theme.card_stroke())
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("Portamento")
                                    .strong()
                                    .size(11.0)
                                    .color(theme.accent_c32()),
                            );
                            ui.separator();

                            changed_portamento |= ui
                                .checkbox(&mut snap_first, lang.tr("Ligar à nota anterior", "Snap to previous note"))
                                .changed();

                            ui.horizontal(|ui| {
                                ui.label(RichText::new(lang.tr("Comprimento:", "Length:")).size(10.0));
                                changed_portamento |= ui
                                    .add_sized(
                                        Vec2::new(ui.available_width(), 18.0),
                                        egui::Slider::new(
                                            &mut portamento_length,
                                            1.0..=500.0,
                                        )
                                        .suffix(" ms"),
                                    )
                                    .changed();
                            });

                            ui.horizontal(|ui| {
                                ui.label(RichText::new(lang.tr("Início Offset:", "Start Offset:")).size(10.0));
                                changed_portamento |= ui
                                    .add_sized(
                                        Vec2::new(ui.available_width(), 18.0),
                                        egui::Slider::new(
                                            &mut portamento_start,
                                            -400.0..=400.0,
                                        )
                                        .suffix(" ms"),
                                    )
                                    .changed();
                            });

                            ui.horizontal(|ui| {
                                ui.label(RichText::new(lang.tr("Formato:", "Shape:")).size(10.0));
                                egui::ComboBox::from_id_salt("portamento_shape_cb")
                                    .selected_text(match portamento_shape.as_str() {
                                        "io" => lang.tr("Curva S (Suave)", "S-Curve (Smooth)"),
                                        "l" => lang.tr("Linear", "Linear"),
                                        "i" => lang.tr("Ease In (Entrada)", "Ease In"),
                                        "o" => lang.tr("Ease Out (Saída)", "Ease Out"),
                                        "j" => lang.tr("Exponencial", "Exponential"),
                                        "r" => lang.tr("Logarítmico", "Logarithmic"),
                                        _ => lang.tr("Padrão", "Default"),
                                    })
                                    .show_ui(ui, |ui| {
                                        for (value, label) in [
                                            ("io", lang.tr("Curva S (Suave)", "S-Curve (Smooth)")),
                                            ("l", lang.tr("Linear", "Linear")),
                                            ("i", lang.tr("Ease In (Entrada)", "Ease In")),
                                            ("o", lang.tr("Ease Out (Saída)", "Ease Out")),
                                            ("j", lang.tr("Exponencial", "Exponential")),
                                            ("r", lang.tr("Logarítmico", "Logarithmic")),
                                        ] {
                                            if ui
                                                .selectable_value(
                                                    &mut portamento_shape,
                                                    value.to_string(),
                                                    label,
                                                )
                                                .changed()
                                            {
                                                changed_portamento = true;
                                            }
                                        }
                                    });
                            });
                        });

                    ui.add_space(6.0);

                    // --- 5. VIBRATO OPENUTAU ---
                    Frame::none()
                        .fill(theme.card_bg_c32())
                        .rounding(theme.ui_rounding())
                        .stroke(theme.card_stroke())
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("Vibrato")
                                    .strong()
                                    .size(11.0)
                                    .color(theme.accent_c32()),
                            );
                            ui.separator();

                            ui.horizontal(|ui| {
                                ui.label(RichText::new(lang.tr("Comprimento:", "Length:")).size(10.0));
                                changed_vibrato |= ui
                                    .add_sized(
                                        Vec2::new(ui.available_width(), 18.0),
                                        egui::Slider::new(
                                            &mut vibrato.length_pct,
                                            0.0..=100.0,
                                        )
                                        .suffix(" %"),
                                    )
                                    .changed();
                            });

                            ui.horizontal(|ui| {
                                ui.label(RichText::new(lang.tr("Profundidade:", "Depth:")).size(10.0));
                                changed_vibrato |= ui
                                    .add_sized(
                                        Vec2::new(ui.available_width(), 18.0),
                                        egui::Slider::new(
                                            &mut vibrato.depth_cents,
                                            0.0..=200.0,
                                        )
                                        .suffix(" c"),
                                    )
                                    .changed();
                            });

                            ui.horizontal(|ui| {
                                ui.label(RichText::new(lang.tr("Período (Vel.):", "Period (Speed):")).size(10.0));
                                changed_vibrato |= ui
                                    .add_sized(
                                        Vec2::new(ui.available_width(), 18.0),
                                        egui::Slider::new(
                                            &mut vibrato.period_ms,
                                            5.0..=400.0,
                                        )
                                        .suffix(" ms"),
                                    )
                                    .changed();
                            });

                            ui.add_space(2.0);
                            ui.columns(2, |cols| {
                                changed_vibrato |= cols[0]
                                    .add_sized(
                                        Vec2::new(cols[0].available_width(), 18.0),
                                        egui::DragValue::new(&mut vibrato.fade_in_pct)
                                            .range(0.0..=100.0)
                                            .prefix("Fade In: ")
                                            .suffix("%"),
                                    )
                                    .changed();
                                changed_vibrato |= cols[1]
                                    .add_sized(
                                        Vec2::new(cols[1].available_width(), 18.0),
                                        egui::DragValue::new(&mut vibrato.fade_out_pct)
                                            .range(0.0..=100.0)
                                            .prefix("Fade Out: ")
                                            .suffix("%"),
                                    )
                                    .changed();
                            });

                            ui.add_space(2.0);
                            ui.columns(2, |cols| {
                                changed_vibrato |= cols[0]
                                    .add_sized(
                                        Vec2::new(cols[0].available_width(), 18.0),
                                        egui::DragValue::new(&mut vibrato.shift_pct)
                                            .range(0.0..=100.0)
                                            .prefix(lang.tr("Fase: ", "Phase: "))
                                            .suffix("%"),
                                    )
                                    .changed();
                                changed_vibrato |= cols[1]
                                    .add_sized(
                                        Vec2::new(cols[1].available_width(), 18.0),
                                        egui::DragValue::new(&mut vibrato.drift_pct)
                                            .range(-100.0..=100.0)
                                            .prefix("Drift: ")
                                            .suffix("%"),
                                    )
                                    .changed();
                            });
                        });

                    ui.add_space(6.0);

                    // --- 6. DIAGNÓSTICO FONÉTICO & OTO ---
                    Frame::none()
                        .fill(theme.card_bg_c32())
                        .rounding(theme.ui_rounding())
                        .stroke(theme.card_stroke())
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(lang.tr("Diagnóstico OTO", "OTO Diagnostics"))
                                        .strong()
                                        .size(11.0)
                                        .color(theme.accent_c32()),
                                );
                                if selected_ruler_alias.is_some() {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if ui.small_button(lang.tr("Editar", "Edit")).clicked() {
                                                on_edit_selected_ruler_alias();
                                            }
                                        },
                                    );
                                }
                            });
                            ui.separator();

                            let query_alias = selected_ruler_alias.unwrap_or_else(|| lyric.trim());
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new("Alias:")
                                        .size(10.0)
                                        .color(theme.text_muted_c32()),
                                );
                                ui.label(
                                    RichText::new(query_alias)
                                        .strong()
                                        .size(10.5)
                                        .color(Color32::WHITE),
                                );
                            });

                            if let Some(vb) = voicebank {
                                if let Some(entry) = vb.find_mapped_entry(query_alias, &pitch_str) {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(lang.tr("Mapeado:", "Mapped:"))
                                                .size(10.0)
                                                .color(theme.text_muted_c32()),
                                        );
                                        ui.label(
                                            RichText::new(&entry.alias)
                                                .strong()
                                                .size(10.5)
                                                .color(Color32::from_rgb(180, 220, 255)),
                                        );
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(lang.tr("Arquivo:", "File:"))
                                                .size(10.0)
                                                .color(theme.text_muted_c32()),
                                        );
                                        let exists = vb.root_path.join(&entry.wav_filename).exists();
                                        let file_color = if exists {
                                            Color32::from_rgb(140, 230, 160)
                                        } else {
                                            Color32::from_rgb(255, 120, 120)
                                        };
                                        let status_txt = if exists {
                                            format!("{} (OK)", entry.wav_filename)
                                        } else {
                                            format!("{} ({})", entry.wav_filename, lang.tr("Ausente", "Missing"))
                                        };
                                        ui.label(
                                            RichText::new(status_txt)
                                                .size(9.5)
                                                .color(file_color),
                                        );
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!("Offset: {:.1}ms | Cutoff: {:.1}ms", entry.offset, entry.cutoff))
                                                .size(9.0)
                                                .color(theme.text_muted_c32()),
                                        );
                                    });

                                    let c_vel_scale = if consonant_velocity != 100.0 && consonant_velocity > 0.0 {
                                        (2.0f64).powf((100.0 - consonant_velocity) / 100.0)
                                    } else {
                                        1.0
                                    };
                                    let res_preutter = (entry.preutterance * c_vel_scale
                                        + notes[target_idx].expressions.preutter_offset_ms)
                                        .max(0.0);
                                    let res_overlap = entry.overlap * c_vel_scale
                                        + notes[target_idx].expressions.overlap_offset_ms;
                                    let res_consonant = (entry.consonant * c_vel_scale
                                        + consonant_timing)
                                        .clamp(0.0, dur_ms.max(10.0) * 0.95);

                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!("Preutter: {:.1}ms (base {:.1}ms)", res_preutter, entry.preutterance))
                                                .size(9.0)
                                                .color(theme.text_muted_c32()),
                                        );
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!("Overlap: {:.1}ms (base {:.1}ms)", res_overlap, entry.overlap))
                                                .size(9.0)
                                                .color(theme.text_muted_c32()),
                                        );
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new(format!("{}: {:.1}ms (base {:.1}ms)", lang.tr("Consoante", "Consonant"), res_consonant, entry.consonant))
                                                .size(9.0)
                                                .color(theme.text_muted_c32()),
                                        );
                                    });
                                } else {
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            RichText::new("Status:")
                                                .size(10.0)
                                                .color(theme.text_muted_c32()),
                                        );
                                        ui.label(
                                            RichText::new(lang.tr("Não encontrado no banco", "Not found in voicebank"))
                                                .size(9.5)
                                                .color(Color32::from_rgb(255, 160, 80)),
                                        );
                                    });
                                }
                            } else {
                                ui.label(
                                    RichText::new(lang.tr("Nenhum banco carregado.", "No voicebank loaded."))
                                        .size(9.5)
                                        .color(theme.text_muted_c32()),
                                );
                            }

                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("{}: {} / {}", lang.tr("Motores", "Engines"), selected_resampler, selected_wavtool))
                                        .size(9.0)
                                        .color(theme.text_muted_c32()),
                                );
                            });
                        });

                    // --- APLICAÇÃO DOS DADOS ATUALIZADOS ---
                    let update_targets: Vec<usize> = if selected_indices.is_empty() {
                        vec![target_idx]
                    } else {
                        selected_indices.iter().copied().collect()
                    };

                    for idx in update_targets {
                        if idx < notes.len() {
                            if reset_all {
                                notes[idx].reset_all_parameters();
                            } else {
                                if changed_lyric {
                                    notes[idx].lyric = lyric.clone();
                                }
                                if changed_dur {
                                    notes[idx].duration_ms = dur_ms;
                                }
                                if changed_flags {
                                    notes[idx].flags = flags.clone();
                                }
                                if changed_gender {
                                    notes[idx].expressions.gender = gender;
                                }
                                if changed_dynamics {
                                    notes[idx].expressions.dynamics = dynamics;
                                }
                                if changed_pitch {
                                    notes[idx].expressions.pitch_delta = pitch_delta;
                                }
                                if changed_breath {
                                    notes[idx].expressions.breathiness = breathiness;
                                }
                                if changed_vel {
                                    notes[idx].expressions.velocity = velocity;
                                }
                                if changed_c_vel {
                                    notes[idx].expressions.consonant_velocity =
                                        consonant_velocity;
                                }
                                if changed_mod {
                                    notes[idx].expressions.modulation = modulation;
                                }
                                if changed_c_timing {
                                    notes[idx].expressions.consonant_timing_offset_ms =
                                        consonant_timing;
                                }
                                if changed_amplitude {
                                    notes[idx].expressions.volume = volume;
                                    notes[idx].expressions.attack = attack;
                                    notes[idx].expressions.decay = decay;
                                }
                                if changed_fades {
                                    notes[idx].envelope.p2 = fade_in_ms;
                                    notes[idx].envelope.p5 = fade_out_ms;
                                    notes[idx].envelope.crossfade_ms = note_crossfade_ms;
                                }
                                if changed_vibrato {
                                    notes[idx].vibrato = vibrato.clone();
                                }
                                if changed_portamento {
                                    notes[idx].pitch_bend.snap_first = snap_first;
                                    notes[idx].pitch_bend.portamento_start_ms =
                                        portamento_start;
                                    notes[idx].pitch_bend.portamento_length_ms =
                                        portamento_length;
                                    notes[idx].pitch_bend.portamento_shape =
                                        portamento_shape.clone();
                                    if notes[idx].pitch_bend.points.len() > 1 {
                                        notes[idx].pitch_bend.points[1].time_offset_ms =
                                            portamento_start + portamento_length;
                                        notes[idx].pitch_bend.points.sort_by(
                                            |left, right| {
                                                left.time_offset_ms
                                                    .partial_cmp(&right.time_offset_ms)
                                                    .unwrap_or(std::cmp::Ordering::Equal)
                                            },
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                ui.vertical_centered(|ui| {
                    ui.add_space(40.0);
                    ui.label(
                        RichText::new(lang.tr("Nenhuma Nota Selecionada", "No Note Selected"))
                            .strong()
                            .size(12.0)
                            .color(theme.text_muted_c32()),
                    );
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(lang.tr(
                            "Clique em uma nota no Piano Roll para inspecionar e editar propriedades detalhadas.",
                            "Click a note in the Piano Roll to inspect and edit detailed properties.",
                        ))
                        .size(10.0)
                        .color(theme.text_muted_c32()),
                    );
                });
            }
        });
}
