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
    inherited_flags: &str,
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
                    let selected_phoneme_index = selected_ruler_alias.and_then(|alias| {
                        notes[target_idx]
                            .lyric
                            .split(['.', ';', ',', '|', '/'])
                            .map(str::trim)
                            .position(|part| part == alias)
                    });
                    let mut phoneme_flags = selected_phoneme_index
                        .and_then(|index| {
                            notes[target_idx]
                                .phoneme_overrides
                                .iter()
                                .find(|item| item.index == index)
                        })
                        .and_then(|item| item.flags.clone())
                        .unwrap_or_default();
                    let mut advanced_phoneme_scope = selected_phoneme_index.is_some_and(|index| {
                        notes[target_idx]
                            .phoneme_overrides
                            .iter()
                            .find(|item| item.index == index)
                            .is_some_and(|item| {
                                item.envelope.is_some()
                                    || item.vibrato.is_some()
                                    || item.pitch_bend.is_some()
                            })
                    });

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
                    let orig_fade_in = notes[target_idx].envelope.p2;
                    let orig_fade_out = notes[target_idx].envelope.p5;
                    let orig_crossfade = notes[target_idx].envelope.crossfade_ms;

                    let mut fade_in_ms = orig_fade_in;
                    let mut fade_out_ms = orig_fade_out;
                    let mut note_crossfade_ms = orig_crossfade;

                    let orig_volume = notes[target_idx].expressions.volume;
                    let orig_attack = notes[target_idx].expressions.attack;
                    let orig_decay = notes[target_idx].expressions.decay;
                    let orig_gender = notes[target_idx].expressions.gender;
                    let orig_dynamics = notes[target_idx].expressions.dynamics;
                    let orig_pitch_delta = notes[target_idx].expressions.pitch_delta;
                    let orig_breathiness = notes[target_idx].expressions.breathiness;
                    let orig_velocity = notes[target_idx].expressions.velocity;
                    let orig_consonant_velocity = notes[target_idx].expressions.consonant_velocity;
                    let orig_modulation = notes[target_idx].expressions.modulation;
                    let orig_consonant_timing = notes[target_idx].expressions.consonant_timing_offset_ms;
                    let mut consonant_timing = orig_consonant_timing;

                    let original_vibrato = notes[target_idx].vibrato.clone();
                    let mut vibrato = original_vibrato.clone();

                    let orig_portamento_start = notes[target_idx].pitch_bend.portamento_start_ms;
                    let orig_portamento_length = notes[target_idx].pitch_bend.portamento_length_ms;
                    let orig_portamento_shape = notes[target_idx].pitch_bend.portamento_shape.clone();
                    let orig_snap_first = notes[target_idx].pitch_bend.snap_first;

                    let mut portamento_start = orig_portamento_start;
                    let mut portamento_length = orig_portamento_length;
                    let mut portamento_shape = orig_portamento_shape.clone();
                    let mut snap_first = orig_snap_first;
                    if let Some(phoneme_index) = selected_phoneme_index.filter(|_| advanced_phoneme_scope) {
                        if let Some(item) = notes[target_idx]
                            .phoneme_overrides
                            .iter()
                            .find(|item| item.index == phoneme_index)
                        {
                            if let Some(envelope) = &item.envelope {
                                fade_in_ms = envelope.p2;
                                fade_out_ms = envelope.p5;
                                note_crossfade_ms = envelope.crossfade_ms;
                            }
                            if let Some(item_vibrato) = &item.vibrato {
                                vibrato = item_vibrato.clone();
                            }
                            if let Some(item_pitch_bend) = &item.pitch_bend {
                                portamento_start = item_pitch_bend.portamento_start_ms;
                                portamento_length = item_pitch_bend.portamento_length_ms;
                                portamento_shape = item_pitch_bend.portamento_shape.clone();
                                snap_first = item_pitch_bend.snap_first;
                            }
                            if let Some(value) = item.gender { gender = value; }
                            if let Some(value) = item.dynamics { dynamics = value; }
                            if let Some(value) = item.pitch_delta { pitch_delta = value; }
                            if let Some(value) = item.breathiness { breathiness = value; }
                            if let Some(value) = item.velocity { velocity = value; consonant_velocity = value; }
                            if let Some(value) = item.modulation { modulation = value; }
                            if let Some(value) = item.volume { volume = value; }
                            if let Some(value) = item.attack { attack = value; }
                            if let Some(value) = item.decay { decay = value; }
                        }
                    }

                    let mut changed_lyric = false;
                    let mut changed_dur = false;
                    let mut changed_flags = false;
                    let mut changed_phoneme_flags = false;
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
                    let mut clear_phoneme_advanced = false;
                    let mut reset_all = false;

                    // --- 1. PROPRIEDADES PRINCIPAIS ---
                    Frame::none()
                        .fill(theme.elevated_surface_c32())
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

                            let effective_flags =
                                crate::drivers::resampler_driver::merge_utau_flags(&[
                                    inherited_flags,
                                    &flags,
                                ]);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(
                                    RichText::new(lang.tr(
                                        "Flags efetivas:",
                                        "Effective flags:",
                                    ))
                                    .size(9.5)
                                    .color(theme.text_muted_c32()),
                                );
                                ui.label(
                                    RichText::new(if effective_flags.is_empty() {
                                        "(padrão do resampler)".to_string()
                                    } else {
                                        effective_flags.clone()
                                    })
                                    .monospace()
                                    .size(9.5)
                                    .color(theme.accent_c32()),
                                )
                                .on_hover_text(lang.tr(
                                    "A linha final preserva flags desconhecidas e será combinada com gênero e respiração antes do resampler.",
                                    "The final line preserves unknown flags and is combined with gender and breathiness before the resampler.",
                                ));
                            });

                            ui.collapsing(
                                lang.tr("Editor estruturado de flags", "Structured flag editor"),
                                |ui| {
                                    ui.label(
                                        RichText::new(lang.tr(
                                            "Altere flags conhecidas sem perder tokens proprietários.",
                                            "Edit known flags without losing proprietary tokens.",
                                        ))
                                        .size(9.0)
                                        .color(theme.text_muted_c32()),
                                    );
                                    // Flags numéricas comuns do ecossistema UTAU.
                                    // A linha literal continua sendo a fonte de
                                    // escape para flags proprietárias ou de um
                                    // resampler específico.
                                    for key in [
                                        "g", "B", "b", "C", "D", "E", "e", "H", "Hb", "Mb",
                                        "Mt", "N", "O", "P", "R", "S", "T", "V", "W", "Y",
                                    ] {
                                        let original = crate::drivers::resampler_driver::parse_flag_numeric(
                                            &flags, key,
                                        );
                                        let mut enabled = original.is_some();
                                        let mut value = original.unwrap_or(0.0);
                                        ui.horizontal(|ui| {
                                            let mut changed = ui.checkbox(&mut enabled, key).changed();
                                            if enabled {
                                                changed |= ui
                                                    .add(
                                                        egui::DragValue::new(&mut value)
                                                            .speed(0.5)
                                                            .range(-200.0..=200.0),
                                                    )
                                                    .changed();
                                            }
                                            if changed {
                                                flags = crate::drivers::resampler_driver::set_utau_flag(
                                                    &flags,
                                                    key,
                                                    enabled.then_some(value),
                                                );
                                                changed_flags = true;
                                            }
                                        });
                                    }
                                },
                            );

                            if let Some(phoneme_index) = selected_phoneme_index {
                                ui.add_space(3.0);
                                ui.label(
                                    RichText::new(format!(
                                        "{} #{}",
                                        lang.tr("Flags do fonema selecionado", "Selected phoneme flags"),
                                        phoneme_index + 1
                                    ))
                                    .size(9.5)
                                    .color(theme.accent_c32()),
                                );
                                let response = ui.add_sized(
                                    Vec2::new(ui.available_width(), 18.0),
                                    egui::TextEdit::singleline(&mut phoneme_flags)
                                        .hint_text("flags literais do fonema"),
                                );
                                if response.changed() {
                                    changed_phoneme_flags = true;
                                }
                                let effective_phoneme_flags =
                                    crate::drivers::resampler_driver::merge_utau_flags(&[
                                        inherited_flags,
                                        &flags,
                                        &phoneme_flags,
                                    ]);
                                ui.label(
                                    RichText::new(format!(
                                        "{} {}",
                                        lang.tr("Efetivas:", "Effective:"),
                                        if effective_phoneme_flags.is_empty() {
                                            "(padrão)"
                                        } else {
                                            effective_phoneme_flags.as_str()
                                        }
                                    ))
                                    .monospace()
                                    .size(9.0)
                                    .color(theme.text_muted_c32()),
                                );
                                ui.checkbox(
                                    &mut advanced_phoneme_scope,
                                    lang.tr(
                                        "Editar envelope, vibrato e portamento deste fonema",
                                        "Edit this phoneme's envelope, vibrato and portamento",
                                    ),
                                );
                                if ui
                                    .small_button(lang.tr(
                                        "Limpar controles avançados do fonema",
                                        "Clear phoneme advanced controls",
                                    ))
                                    .clicked()
                                {
                                    clear_phoneme_advanced = true;
                                }
                            }

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
                        .fill(theme.elevated_surface_c32())
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
                                        -100.0..=200.0,
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
                        .fill(theme.elevated_surface_c32())
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

                            ui.add_space(3.0);
                            ui.label(RichText::new("UTAU Envelope Presets:").size(9.5).color(theme.text_muted_c32()));
                            ui.horizontal_wrapped(|ui| {
                                let overlap = voicebank.and_then(|vb| vb.find_mapped_entry(&lyric, &pitch_str).map(|e| e.overlap)).unwrap_or(fade_in_ms.max(20.0));
                                if ui.small_button(RichText::new("ACPT").strong().color(Color32::from_rgb(0, 255, 157))).on_hover_text("ACPT: Ajusta crossfade e ataque para o overlap do oto.ini").clicked() {
                                    fade_in_ms = overlap.max(5.0);
                                    note_crossfade_ms = 0.0;
                                    changed_fades = true;
                                }
                                if ui.small_button(RichText::new("P2P3").strong().color(Color32::from_rgb(147, 197, 253))).on_hover_text("P2P3: Suaviza ataque e decaimento para transições de fonemas VC").clicked() {
                                    fade_in_ms = overlap.max(5.0);
                                    note_crossfade_ms = 0.0;
                                    changed_fades = true;
                                }
                                if ui.small_button(RichText::new("P1P4").strong().color(Color32::from_rgb(216, 180, 254))).on_hover_text("P1P4: Fixa margens limpas no início e final da nota").clicked() {
                                    changed_fades = true;
                                }
                                if ui.small_button(RichText::new("OPT").strong().color(Color32::from_rgb(253, 224, 71))).on_hover_text("OPT: Otimiza envelope proporcionalmente à duração").clicked() {
                                    fade_in_ms = (dur_ms * 0.25).min(overlap).max(5.0);
                                    fade_out_ms = (dur_ms * 0.2).clamp(5.0, 35.0);
                                    note_crossfade_ms = 0.0;
                                    changed_fades = true;
                                }
                                if ui.small_button(RichText::new("RESET").strong().color(Color32::from_rgb(248, 113, 113))).on_hover_text("RESET: Restaura envelope padrão UTAU").clicked() {
                                    fade_in_ms = 5.0;
                                    fade_out_ms = 35.0;
                                    note_crossfade_ms = 0.0;
                                    changed_fades = true;
                                }
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
                        .fill(theme.elevated_surface_c32())
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

                            ui.horizontal_wrapped(|ui| {
                                ui.label(RichText::new(lang.tr("Presets:", "Presets:")).size(10.0));
                                for (label, start, length, shape, snap) in [
                                    ("Suave", -55.0, 110.0, "io", true),
                                    ("Natural", -40.0, 80.0, "io", true),
                                    ("Rápido", -20.0, 45.0, "l", true),
                                    ("Deslizante", -100.0, 220.0, "s", true),
                                    ("Sem snap", -25.0, 60.0, "l", false),
                                ] {
                                    if ui.small_button(label).clicked() {
                                        portamento_start = start;
                                        portamento_length = length;
                                        portamento_shape = shape.to_string();
                                        snap_first = snap;
                                        changed_portamento = true;
                                    }
                                }
                            });

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
                                        "h" | "hermite" => lang.tr("Spline Hermite (Ultra-Suave)", "Hermite Spline (Ultra-Smooth)"),
                                        "io" | "s" => lang.tr("Curva S (Suave)", "S-Curve (Smooth)"),
                                        "l" => lang.tr("Linear", "Linear"),
                                        "i" => lang.tr("Ease In (Entrada)", "Ease In"),
                                        "o" => lang.tr("Ease Out (Saída)", "Ease Out"),
                                        "j" => lang.tr("Exponencial", "Exponential"),
                                        "r" => lang.tr("Logarítmico", "Logarithmic"),
                                        _ => lang.tr("Padrão", "Default"),
                                    })
                                    .show_ui(ui, |ui| {
                                        for (value, label) in [
                                            ("h", lang.tr("Spline Hermite (Ultra-Suave)", "Hermite Spline (Ultra-Smooth)")),
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
                        .fill(theme.elevated_surface_c32())
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
                                ui.label(RichText::new(lang.tr("Presets:", "Presets:")).size(10.0));
                                for (label, length, depth, period, fade) in [
                                    ("Suave", 55.0, 28.0, 190.0, 30.0),
                                    ("Natural", 70.0, 42.0, 175.0, 20.0),
                                    ("Intenso", 85.0, 70.0, 140.0, 12.0),
                                ] {
                                    if ui.small_button(label).clicked() {
                                        vibrato.length_pct = length;
                                        vibrato.depth_cents = depth;
                                        vibrato.period_ms = period;
                                        vibrato.fade_in_pct = fade;
                                        vibrato.fade_out_pct = 15.0;
                                        changed_vibrato = true;
                                    }
                                }
                            });

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

                            changed_vibrato |= ui
                                .add_sized(
                                    Vec2::new(ui.available_width(), 18.0),
                                    egui::Slider::new(&mut vibrato.volume_link_pct, -100.0..=100.0)
                                        .text(lang.tr("Vínculo de volume", "Volume link"))
                                        .suffix(" %"),
                                )
                                .changed();

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

                    // --- 6. VALIDAÇÃO DO VOICEBANK ---
                    Frame::none()
                        .fill(theme.elevated_surface_c32())
                        .rounding(theme.ui_rounding())
                        .stroke(theme.card_stroke())
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(lang.tr(
                                        "Validação do Voicebank",
                                        "Voicebank Validation",
                                    ))
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
                            let command_flags =
                                crate::drivers::resampler_driver::merge_utau_flags(&[
                                    inherited_flags,
                                    &flags,
                                    &phoneme_flags,
                                ]);
                            let command_args = crate::drivers::resampler_driver::ResamplerArgs {
                                input_wav: std::path::PathBuf::from("<oto.wav>"),
                                output_wav: std::path::PathBuf::from("<rendered-phone.wav>"),
                                pitch_name: pitch_str.clone(),
                                pitch_freq: crate::dsp::pitch::midi_to_freq(
                                    notes[target_idx].midi_key() as f64,
                                ),
                                velocity,
                                flags: command_flags,
                                offset_ms: 0.0,
                                duration_ms: dur_ms,
                                source_consonant_ms: 0.0,
                                consonant_ms: consonant_timing,
                                cutoff_ms: 0.0,
                                volume,
                                modulation,
                                tempo: 120.0,
                                pitch_bend_str: String::new(),
                                pitch_points: notes[target_idx].pitch_bend.points.clone(),
                                loop_start_ms: None,
                                loop_end_ms: None,
                                tail_start_ms: None,
                            };
                            ui.collapsing(
                                lang.tr("Comandos finais (prévia)", "Final commands (preview)"),
                                |ui| {
                                    ui.label(
                                        RichText::new(
                                            crate::drivers::resampler_driver::describe_resampler_command(
                                                selected_resampler,
                                                &command_args,
                                            ),
                                        )
                                        .monospace()
                                        .size(8.5),
                                    );
                                    let mut command_envelope = notes[target_idx].envelope.clone();
                                    command_envelope.p2 = fade_in_ms;
                                    command_envelope.p5 = fade_out_ms;
                                    command_envelope.crossfade_ms = note_crossfade_ms;
                                    let command_phoneme_envelope =
                                        command_envelope.get_effective_points(dur_ms);
                                    let wavtool_args = crate::drivers::wavtool_driver::WavtoolArgs {
                                        output_wav: std::path::PathBuf::from("<track.wav>"),
                                        input_rendered_wav: std::path::PathBuf::from(
                                            "<rendered-phone.wav>",
                                        ),
                                        skip_over_ms: 0.0,
                                        duration_ms: dur_ms,
                                        envelope: command_envelope,
                                        overlap_ms: 0.0,
                                        phoneme_envelope: command_phoneme_envelope,
                                        sample_time_zero_ms: 0.0,
                                    };
                                    ui.label(
                                        RichText::new(
                                            crate::drivers::wavtool_driver::describe_wavtool_command(
                                                selected_wavtool,
                                                &wavtool_args,
                                            ),
                                        )
                                        .monospace()
                                        .size(8.5),
                                    );
                                    ui.label(
                                        RichText::new(lang.tr(
                                            "Caminhos entre < > são placeholders; a prévia é apenas diagnóstica e nunca executa shell.",
                                            "Paths between < > are placeholders; this is diagnostic only and never executes a shell.",
                                        ))
                                        .size(8.5)
                                        .color(theme.text_muted_c32()),
                                    );
                                },
                            );
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
                                if changed_phoneme_flags && idx == target_idx {
                                    if let Some(phoneme_index) = selected_phoneme_index {
                                        let override_item = notes[idx]
                                            .phoneme_overrides
                                            .iter_mut()
                                            .find(|item| item.index == phoneme_index);
                                        let item = match override_item {
                                            Some(item) => item,
                                            None => {
                                                notes[idx].phoneme_overrides.push(
                                                    crate::project::model::UPhonemeOverride {
                                                        index: phoneme_index,
                                                        ..Default::default()
                                                    },
                                                );
                                                notes[idx]
                                                    .phoneme_overrides
                                                    .last_mut()
                                                    .expect("phoneme override was just inserted")
                                            }
                                        };
                                        item.flags = (!phoneme_flags.trim().is_empty())
                                            .then(|| phoneme_flags.clone());
                                    }
                                }
                                if changed_gender && (gender - orig_gender).abs() > f64::EPSILON {
                                    notes[idx].expressions.gender = gender;
                                }
                                if changed_dynamics && (dynamics - orig_dynamics).abs() > f64::EPSILON {
                                    notes[idx].expressions.dynamics = dynamics;
                                }
                                if changed_pitch && (pitch_delta - orig_pitch_delta).abs() > f64::EPSILON {
                                    notes[idx].expressions.pitch_delta = pitch_delta;
                                }
                                if changed_breath && (breathiness - orig_breathiness).abs() > f64::EPSILON {
                                    notes[idx].expressions.breathiness = breathiness;
                                }
                                if changed_vel && (velocity - orig_velocity).abs() > f64::EPSILON {
                                    notes[idx].expressions.velocity = velocity;
                                }
                                if changed_c_vel && (consonant_velocity - orig_consonant_velocity).abs() > f64::EPSILON {
                                    notes[idx].expressions.consonant_velocity =
                                        consonant_velocity;
                                }
                                if changed_mod && (modulation - orig_modulation).abs() > f64::EPSILON {
                                    notes[idx].expressions.modulation = modulation;
                                }
                                if changed_c_timing && (consonant_timing - orig_consonant_timing).abs() > f64::EPSILON {
                                    if advanced_phoneme_scope && idx == target_idx {
                                        let phoneme_index = selected_phoneme_index.unwrap();
                                        let item = notes[idx]
                                            .phoneme_overrides
                                            .iter_mut()
                                            .find(|item| item.index == phoneme_index);
                                        let item = match item {
                                            Some(item) => item,
                                            None => {
                                                notes[idx].phoneme_overrides.push(
                                                    crate::project::model::UPhonemeOverride {
                                                        index: phoneme_index,
                                                        ..Default::default()
                                                    },
                                                );
                                                notes[idx].phoneme_overrides.last_mut().unwrap()
                                            }
                                        };
                                        item.consonant_timing_offset_ms = Some(consonant_timing);
                                    } else {
                                        notes[idx].expressions.consonant_timing_offset_ms = consonant_timing;
                                    }
                                }
                                if changed_amplitude {
                                    if (volume - orig_volume).abs() > f64::EPSILON {
                                        notes[idx].expressions.volume = volume;
                                    }
                                    if (attack - orig_attack).abs() > f64::EPSILON {
                                        notes[idx].expressions.attack = attack;
                                    }
                                    if (decay - orig_decay).abs() > f64::EPSILON {
                                        notes[idx].expressions.decay = decay;
                                    }
                                }
                                if changed_fades {
                                    if advanced_phoneme_scope && idx == target_idx {
                                        let phoneme_index = selected_phoneme_index.unwrap();
                                        let item = notes[idx]
                                            .phoneme_overrides
                                            .iter_mut()
                                            .find(|item| item.index == phoneme_index);
                                        let item = match item {
                                            Some(item) => item,
                                            None => {
                                                notes[idx].phoneme_overrides.push(
                                                    crate::project::model::UPhonemeOverride {
                                                        index: phoneme_index,
                                                        envelope: Some(notes[idx].envelope.clone()),
                                                        ..Default::default()
                                                    },
                                                );
                                                notes[idx].phoneme_overrides.last_mut().unwrap()
                                            }
                                        };
                                        let envelope = item.envelope.get_or_insert_with(Default::default);
                                        envelope.p2 = fade_in_ms;
                                        envelope.p5 = fade_out_ms;
                                        envelope.crossfade_ms = note_crossfade_ms;
                                    } else {
                                        notes[idx].envelope.p2 = fade_in_ms;
                                        notes[idx].envelope.p5 = fade_out_ms;
                                        notes[idx].envelope.crossfade_ms = note_crossfade_ms;
                                    }
                                }
                                if changed_vibrato {
                                    let target = if advanced_phoneme_scope && idx == target_idx {
                                        let phoneme_index = selected_phoneme_index.unwrap();
                                        let item = notes[idx]
                                            .phoneme_overrides
                                            .iter_mut()
                                            .find(|item| item.index == phoneme_index);
                                        let item = match item {
                                            Some(item) => item,
                                            None => {
                                                notes[idx].phoneme_overrides.push(
                                                    crate::project::model::UPhonemeOverride {
                                                        index: phoneme_index,
                                                        vibrato: Some(notes[idx].vibrato.clone()),
                                                        ..Default::default()
                                                    },
                                                );
                                                notes[idx].phoneme_overrides.last_mut().unwrap()
                                            }
                                        };
                                        item.vibrato.get_or_insert_with(Default::default)
                                    } else {
                                        &mut notes[idx].vibrato
                                    };
                                    if (vibrato.length_pct - original_vibrato.length_pct).abs() > f64::EPSILON { target.length_pct = vibrato.length_pct; }
                                    if (vibrato.period_ms - original_vibrato.period_ms).abs() > f64::EPSILON { target.period_ms = vibrato.period_ms; }
                                    if (vibrato.depth_cents - original_vibrato.depth_cents).abs() > f64::EPSILON { target.depth_cents = vibrato.depth_cents; }
                                    if (vibrato.fade_in_pct - original_vibrato.fade_in_pct).abs() > f64::EPSILON { target.fade_in_pct = vibrato.fade_in_pct; }
                                    if (vibrato.fade_out_pct - original_vibrato.fade_out_pct).abs() > f64::EPSILON { target.fade_out_pct = vibrato.fade_out_pct; }
                                    if (vibrato.shift_pct - original_vibrato.shift_pct).abs() > f64::EPSILON { target.shift_pct = vibrato.shift_pct; }
                                    if (vibrato.drift_pct - original_vibrato.drift_pct).abs() > f64::EPSILON { target.drift_pct = vibrato.drift_pct; }
                                    if (vibrato.volume_link_pct - original_vibrato.volume_link_pct).abs() > f64::EPSILON { target.volume_link_pct = vibrato.volume_link_pct; }
                                }
                                if changed_portamento {
                                    let target = if advanced_phoneme_scope && idx == target_idx {
                                        let phoneme_index = selected_phoneme_index.unwrap();
                                        let item = notes[idx]
                                            .phoneme_overrides
                                            .iter_mut()
                                            .find(|item| item.index == phoneme_index);
                                        let item = match item {
                                            Some(item) => item,
                                            None => {
                                                notes[idx].phoneme_overrides.push(
                                                    crate::project::model::UPhonemeOverride {
                                                        index: phoneme_index,
                                                        pitch_bend: Some(notes[idx].pitch_bend.clone()),
                                                        ..Default::default()
                                                    },
                                                );
                                                notes[idx].phoneme_overrides.last_mut().unwrap()
                                            }
                                        };
                                        item.pitch_bend.get_or_insert_with(Default::default)
                                    } else {
                                        &mut notes[idx].pitch_bend
                                    };
                                    if snap_first != orig_snap_first { target.snap_first = snap_first; }
                                    if (portamento_start - orig_portamento_start).abs() > f64::EPSILON {
                                        target.portamento_start_ms = portamento_start;
                                    }
                                    if (portamento_length - orig_portamento_length).abs() > f64::EPSILON {
                                        target.portamento_length_ms = portamento_length;
                                    }
                                    if portamento_shape != orig_portamento_shape {
                                        target.portamento_shape = portamento_shape.clone();
                                    }
                                    if target.points.len() > 1 {
                                        target.points[1].time_offset_ms =
                                            target.portamento_start_ms + target.portamento_length_ms;
                                        target.points.sort_by(
                                            |left, right| {
                                                left.time_offset_ms
                                                    .partial_cmp(&right.time_offset_ms)
                                                    .unwrap_or(std::cmp::Ordering::Equal)
                                            },
                                        );
                                    }
                                }
                                if clear_phoneme_advanced && idx == target_idx {
                                    if let Some(phoneme_index) = selected_phoneme_index {
                                        if let Some(item) = notes[idx]
                                            .phoneme_overrides
                                            .iter_mut()
                                            .find(|item| item.index == phoneme_index)
                                        {
                                            item.envelope = None;
                                            item.vibrato = None;
                                            item.pitch_bend = None;
                                            item.consonant_timing_offset_ms = None;
                                        }
                                    }
                                }
                                if advanced_phoneme_scope && idx == target_idx {
                                    if let Some(phoneme_index) = selected_phoneme_index {
                                        let item = notes[idx]
                                            .phoneme_overrides
                                            .iter_mut()
                                            .find(|item| item.index == phoneme_index);
                                        let item = match item {
                                            Some(item) => item,
                                            None => {
                                                notes[idx].phoneme_overrides.push(
                                                    crate::project::model::UPhonemeOverride {
                                                        index: phoneme_index,
                                                        ..Default::default()
                                                    },
                                                );
                                                notes[idx].phoneme_overrides.last_mut().unwrap()
                                            }
                                        };
                                        if changed_gender { item.gender = Some(gender); notes[idx].expressions.gender = orig_gender; }
                                        if changed_dynamics { item.dynamics = Some(dynamics); notes[idx].expressions.dynamics = orig_dynamics; }
                                        if changed_pitch { item.pitch_delta = Some(pitch_delta); notes[idx].expressions.pitch_delta = orig_pitch_delta; }
                                        if changed_breath { item.breathiness = Some(breathiness); notes[idx].expressions.breathiness = orig_breathiness; }
                                        if changed_vel { item.velocity = Some(velocity); notes[idx].expressions.velocity = orig_velocity; }
                                        if changed_c_vel { item.velocity = Some(consonant_velocity); notes[idx].expressions.consonant_velocity = orig_consonant_velocity; }
                                        if changed_mod { item.modulation = Some(modulation); notes[idx].expressions.modulation = orig_modulation; }
                                        if changed_amplitude {
                                            if (volume - orig_volume).abs() > f64::EPSILON { item.volume = Some(volume); notes[idx].expressions.volume = orig_volume; }
                                            if (attack - orig_attack).abs() > f64::EPSILON { item.attack = Some(attack); notes[idx].expressions.attack = orig_attack; }
                                            if (decay - orig_decay).abs() > f64::EPSILON { item.decay = Some(decay); notes[idx].expressions.decay = orig_decay; }
                                        }
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
