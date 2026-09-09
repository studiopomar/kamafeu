use super::*;

pub(super) fn draw(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    lang: crate::config::AppLanguage,
    render_threads: &mut u32,
    sample_rate: &mut u32,
    selected_resampler: &mut String,
    selected_wavtool: &mut String,
    custom_resampler_path: &mut Option<PathBuf>,
    custom_wavtool_path: &mut Option<PathBuf>,
    discord_rpc_enabled: &mut bool,
) {
    egui::ScrollArea::vertical()
        .id_salt("right_panel_settings_scroll")
        .show(ui, |ui| {
            // --- MOTOR RESAMPLER ---
            Frame::none()
                .fill(theme.card_bg_c32())
                .rounding(theme.ui_rounding())
                .stroke(theme.card_stroke())
                .inner_margin(egui::Margin::same(8.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(lang.tr("Resampler (Afinador / DSP)", "Resampler (Tuner / DSP)"))
                            .strong()
                            .size(11.0)
                            .color(theme.accent_c32()),
                    );
                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui
                            .radio_value(
                                selected_resampler,
                                "straycat-rs (UtaUtaUtau) [Padrão Recomendado]"
                                    .to_string(),
                                lang.tr("straycat-rs (Recomendado)", "straycat-rs (Recommended)"),
                            )
                            .clicked()
                        {
                            let profile = crate::drivers::KnownResampler::StraycatRs;
                            *custom_resampler_path = Some(
                                profile
                                    .find_executable()
                                    .unwrap_or_else(|| profile.default_path()),
                            );
                        }
                        if crate::drivers::KnownResampler::StraycatRs
                            .default_path()
                            .is_file()
                        {
                            ui.label(
                                RichText::new(lang.tr("● pronto", "● ready"))
                                    .size(9.0)
                                    .color(theme.accent_c32()),
                            );
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui
                            .radio_value(
                                selected_resampler,
                                "VENUS (Nativo)".to_string(),
                                lang.tr("VENUS (Nativo)", "VENUS (Native)"),
                            )
                            .clicked()
                        {
                            *custom_resampler_path = None;
                        }
                        ui.label(
                            RichText::new(lang.tr("incluso", "included"))
                                .size(9.0)
                                .color(theme.accent_c32()),
                        );
                    });

                    ui.add_space(4.0);
                    egui::CollapsingHeader::new(
                        RichText::new(lang.tr("Outros Motores Externos (CLI)", "Other External Engines (CLI)"))
                            .size(10.0)
                            .color(theme.text_muted_c32()),
                    )
                    .show(ui, |ui| {
                        for profile in crate::drivers::KnownResampler::ALL {
                            if profile == crate::drivers::KnownResampler::StraycatRs {
                                continue;
                            }
                            ui.horizontal(|ui| {
                                if ui
                                    .radio_value(
                                        selected_resampler,
                                        profile.label().to_string(),
                                        profile.label(),
                                    )
                                    .clicked()
                                {
                                    *custom_resampler_path = Some(
                                        profile
                                            .find_executable()
                                            .unwrap_or_else(|| profile.default_path()),
                                    );
                                    if profile
                                        == crate::drivers::KnownResampler::HifisamplerRs
                                    {
                                        let _ =
                                            crate::drivers::resampler_driver::ensure_hifisampler_ready();
                                    }
                                }
                                if profile
                                    == crate::drivers::KnownResampler::HifisamplerRs
                                {
                                    if let Ok(ref msg) =
                                        crate::drivers::resampler_driver::ensure_hifisampler_ready()
                                    {
                                        ui.label(
                                            RichText::new("ONNX pronto")
                                                .size(8.5)
                                                .color(theme.accent_c32()),
                                        )
                                        .on_hover_text(msg);
                                    }
                                }
                            });
                        }
                    });

                    if !selected_resampler.contains("Nativo")
                        && !selected_resampler.contains("Native")
                    {
                        ui.add_space(3.0);
                        if ui
                            .add_sized(
                                Vec2::new(ui.available_width(), 20.0),
                                egui::Button::new(
                                    RichText::new(lang.tr(
                                        "Procurar Resampler (.exe / bin)...",
                                        "Browse Resampler (.exe / bin)...",
                                    ))
                                    .size(9.5),
                                ),
                            )
                            .clicked()
                        {
                            if let Some(file) = crate::dialogs::FileDialog::new()
                                .set_title(lang.tr("Selecionar executável de resampler", "Select resampler executable"))
                                .pick_file()
                            {
                                *custom_resampler_path = Some(file);
                                *selected_resampler =
                                    "Personalizado (UTAU CLI)".to_string();
                            }
                        }
                    }
                });

            ui.add_space(8.0);

            // --- MOTOR WAVTOOL ---
            Frame::none()
                .fill(theme.card_bg_c32())
                .rounding(theme.ui_rounding())
                .stroke(theme.card_stroke())
                .inner_margin(egui::Margin::same(8.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new("Wavtool (Splicer / Render)")
                            .strong()
                            .size(11.0)
                            .color(theme.accent_c32()),
                    );
                    ui.separator();

                    ui.horizontal(|ui| {
                        if ui
                            .radio_value(
                                selected_wavtool,
                                "Andromeda (Nativo)".to_string(),
                                lang.tr("Andromeda (Recomendado)", "Andromeda (Recommended)"),
                            )
                            .clicked()
                        {
                            *custom_wavtool_path = None;
                        }
                        ui.label(
                            RichText::new(lang.tr("incluso", "included"))
                                .size(9.0)
                                .color(theme.accent_c32()),
                        );
                    });

                    for profile in crate::drivers::KnownWavtool::ALL {
                        if let Some(exe_path) = profile.find_executable().filter(|p| p.is_file()) {
                            ui.horizontal(|ui| {
                                if ui
                                    .radio_value(
                                        selected_wavtool,
                                        profile.label().to_string(),
                                        profile.label(),
                                    )
                                    .clicked()
                                {
                                    *custom_wavtool_path = Some(exe_path);
                                }
                                ui.label(
                                    RichText::new(lang.tr("detectado", "detected"))
                                        .size(8.5)
                                        .color(theme.accent_c32()),
                                );
                            });
                        }
                    }

                    if !selected_wavtool.contains("Nativo") && !selected_wavtool.contains("Native") {
                        ui.add_space(3.0);
                        if ui
                            .add_sized(
                                Vec2::new(ui.available_width(), 20.0),
                                egui::Button::new(
                                    RichText::new(lang.tr(
                                        "Procurar Wavtool (.exe / bin)...",
                                        "Browse Wavtool (.exe / bin)...",
                                    ))
                                    .size(9.5),
                                ),
                            )
                            .clicked()
                        {
                            if let Some(file) = crate::dialogs::FileDialog::new()
                                .set_title(lang.tr("Selecionar executável de wavtool", "Select wavtool executable"))
                                .pick_file()
                            {
                                *custom_wavtool_path = Some(file);
                                *selected_wavtool = "Personalizado (UTAU CLI)".to_string();
                            }
                        }
                    }
                });

            ui.add_space(8.0);

            // --- PERFORMANCE & ÁUDIO ---
            Frame::none()
                .fill(theme.card_bg_c32())
                .rounding(theme.ui_rounding())
                .stroke(theme.card_stroke())
                .inner_margin(egui::Margin::same(8.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(lang.tr("Saída de Áudio & Hardware", "Audio Output & Hardware"))
                            .strong()
                            .size(11.0)
                            .color(theme.accent_c32()),
                    );
                    ui.separator();

                    ui.label(RichText::new("Sample Rate:").size(10.0));
                    ui.columns(2, |cols| {
                        for (i, rate) in [44100, 48000].iter().enumerate() {
                            let is_selected = *sample_rate == *rate;
                            let (bg_color, text_color, stroke) = if is_selected {
                                (
                                    theme.c32_alpha(theme.accent_color, 0.25),
                                    theme.accent_c32(),
                                    Stroke::new(1.2_f32, theme.accent_c32()),
                                )
                            } else {
                                (
                                    theme.bg_header_c32(),
                                    theme.text_primary_c32(),
                                    Stroke::NONE,
                                )
                            };
                            let btn = egui::Button::new(
                                RichText::new(format!("{} Hz", rate))
                                    .size(10.0)
                                    .color(text_color)
                                    .strong(),
                            )
                            .fill(bg_color)
                            .stroke(stroke)
                            .rounding(Rounding::same(3.0));

                            if cols[i].add_sized(Vec2::new(cols[i].available_width(), 20.0), btn).clicked() {
                                *sample_rate = *rate;
                            }
                        }
                    });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(lang.tr("Threads Render:", "Render Threads:")).size(10.0));
                        ui.add_sized(
                            Vec2::new(ui.available_width(), 18.0),
                            egui::Slider::new(render_threads, 1..=16),
                        );
                    });

                    ui.add_space(2.0);
                    ui.checkbox(
                        discord_rpc_enabled,
                        RichText::new("Discord Rich Presence").size(10.0),
                    );
                });
        });
}
