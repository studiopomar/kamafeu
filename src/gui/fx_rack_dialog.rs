use crate::audio::FxRackConfig;
use crate::config::AppLanguage;
use crate::gui::theme::ThemeConfig;
use crate::project::model::UTrack;
use eframe::egui::{self, Frame, RichText};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FxRackDialogState {
    pub is_open: bool,
    pub target_track: Option<usize>, // None = Master FX, Some(idx) = Track FX
}

impl Default for FxRackDialogState {
    fn default() -> Self {
        Self {
            is_open: false,
            target_track: None,
        }
    }
}

pub fn draw_fx_rack_dialog(
    ctx: &egui::Context,
    lang: AppLanguage,
    theme: &ThemeConfig,
    state: &mut FxRackDialogState,
    master_fx_config: &mut FxRackConfig,
    tracks: &mut [UTrack],
    project_changed: &mut bool,
) {
    if !state.is_open {
        return;
    }

    // Determine target track and config to edit
    let num_tracks = tracks.len();
    if let Some(t_idx) = state.target_track {
        if t_idx >= num_tracks {
            state.target_track = if num_tracks > 0 { Some(0) } else { None };
        }
    }

    let dialog_title = match state.target_track {
        Some(idx) => {
            let name = tracks.get(idx).map(|t| t.name.as_str()).unwrap_or(lang.tr("Faixa", "Track"));
            format!("{} - {} {}: {}", lang.tr("Rack de Efeitos DSP", "DSP Effects Rack"), lang.tr("Faixa", "Track"), idx + 1, name)
        }
        None => format!("{} - Master", lang.tr("Rack de Efeitos DSP", "DSP Effects Rack")),
    };

    let mut trigger_close = false;

    let mut working_fx = match state.target_track {
        Some(idx) => tracks.get(idx).and_then(|t| t.fx_rack.clone()).unwrap_or_default(),
        None => master_fx_config.clone(),
    };
    let initial_fx = working_fx.clone();

    ctx.show_viewport_immediate(
        egui::ViewportId::from_hash_of("fx_rack_native_viewport"),
        egui::ViewportBuilder::default()
            .with_title(format!("{} - Kamafeu", dialog_title))
            .with_inner_size([760.0, 620.0])
            .with_min_inner_size([540.0, 480.0]),
        |ctx, _class| {
            egui::CentralPanel::default()
                .frame(
                    Frame::none()
                        .fill(theme.bg_panel_c32())
                        .inner_margin(egui::Margin::same(12.0)),
                )
                .show(ctx, |ui| {
                    render_fx_rack_ui(
                        ui,
                        lang,
                        theme,
                        &dialog_title,
                        state,
                        tracks,
                        &mut working_fx,
                        &mut trigger_close,
                    );
                });

            if ctx.input(|i| i.viewport().close_requested()) {
                trigger_close = true;
            }
        },
    );

    if working_fx != initial_fx {
        match state.target_track {
            Some(idx) => {
                if let Some(track) = tracks.get_mut(idx) {
                    track.fx_rack = Some(working_fx);
                }
            }
            None => {
                *master_fx_config = working_fx;
            }
        }
        *project_changed = true;
    }

    if trigger_close {
        state.is_open = false;
    }
}

#[allow(clippy::too_many_arguments)]
fn render_fx_rack_ui(
    ui: &mut egui::Ui,
    lang: AppLanguage,
    theme: &ThemeConfig,
    dialog_title: &str,
    state: &mut FxRackDialogState,
    tracks: &[UTrack],
    fx_config: &mut FxRackConfig,
    trigger_close: &mut bool,
) {
    // Header with title and close button
    ui.horizontal(|ui| {
        ui.heading(
            RichText::new(dialog_title)
                .strong()
                .size(16.0)
                .color(theme.accent_c32()),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button(RichText::new("X").size(11.0).color(theme.text_muted_c32()))
                .clicked()
            {
                *trigger_close = true;
            }
        });
    });

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    // Track selector combo
    ui.horizontal(|ui| {
        ui.label(RichText::new(lang.tr("Destino do FX:", "FX Target:")).strong().color(theme.text_muted_c32()));
        let current_target_name = match state.target_track {
            Some(idx) => {
                let name = tracks.get(idx).map(|t| t.name.as_str()).unwrap_or(lang.tr("Faixa", "Track"));
                format!("{} {}: {}", lang.tr("Faixa", "Track"), idx + 1, name)
            }
            None => lang.tr("Master (Geral)", "Master (Global)").to_string(),
        };

        egui::ComboBox::from_id_salt("fx_rack_target_combo")
            .selected_text(current_target_name)
            .show_ui(ui, |ui| {
                if ui.selectable_value(&mut state.target_track, None, lang.tr("Master (Geral)", "Master (Global)")).clicked() {
                    // Switched to Master
                }
                for (i, t) in tracks.iter().enumerate() {
                    let label = format!("{} {}: {}", lang.tr("Faixa", "Track"), i + 1, t.name);
                    if ui.selectable_value(&mut state.target_track, Some(i), label).clicked() {
                        // Switched to Track i
                    }
                }
            });
    });

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            let is_master = fx_config.master_enabled;
            ui.checkbox(
                &mut fx_config.master_enabled,
                RichText::new(lang.tr("Ativar Rack de Efeitos", "Enable Effects Rack"))
                    .strong()
                    .size(13.0)
                    .color(if is_master {
                        theme.accent_c32()
                    } else {
                        theme.text_muted_c32()
                    }),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(RichText::new(lang.tr("Redefinir Tudo", "Reset All")).size(10.0))
                    .clicked()
                {
                    *fx_config = FxRackConfig::default();
                }
            });
        });

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);

        egui::ScrollArea::vertical()
            .id_salt("fx_rack_scroll")
            .show(ui, |ui| {
                let card_frame = |enabled: bool| {
                    Frame::none()
                        .fill(theme.card_bg_c32())
                        .rounding(theme.ui_rounding())
                        .stroke(if enabled && fx_config.master_enabled {
                            theme.active_border_stroke()
                        } else {
                            theme.card_stroke()
                        })
                        .inner_margin(egui::Margin::same(10.0))
                };

                // 1. 31-Band Graphic Equalizer Module Card (Full width top)
                card_frame(fx_config.eq.enabled).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.checkbox(
                            &mut fx_config.eq.enabled,
                            RichText::new(lang.tr("Equalizador Gráfico de 31 Bandas (1/3 Oitava)", "31-Band Graphic Equalizer (1/3 Octave)"))
                                .strong()
                                .size(12.0)
                                .color(theme.accent_c32()),
                        );
                        if !fx_config.eq.enabled {
                            ui.label(
                                RichText::new(lang.tr("(Bypass)", "(Bypass)"))
                                    .italics()
                                    .size(10.0)
                                    .color(theme.text_muted_c32()),
                            );
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(RichText::new(lang.tr("Flat (Zerar)", "Flat (Zero)")).size(10.0)).clicked() {
                                fx_config.eq.gains = [0.0; 31];
                            }

                            egui::ComboBox::from_id_salt("eq_presets_combo")
                                .selected_text(RichText::new(lang.tr("Presets de EQ...", "EQ Presets...")).size(10.0))
                                .show_ui(ui, |ui| {
                                    if ui.button(lang.tr("Flat (Linear)", "Flat (Linear)")).clicked() {
                                        fx_config.eq.gains = [0.0; 31];
                                        ui.close_menu();
                                    }
                                    if ui.button(lang.tr("Vocal - Brilho e Ar (Vocal Air)", "Vocal - Brightness and Air (Vocal Air)")).clicked() {
                                        fx_config.eq.gains = [
                                            -12.0, -10.0, -8.0, -6.0, -4.0, -2.0, 0.0, 0.0,
                                            0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                                            0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5, 5.0, 5.5, 5.0,
                                            4.0, 3.0, 2.0, 1.0, 0.0,
                                        ];
                                        ui.close_menu();
                                    }
                                    if ui.button(lang.tr("Vocal - Calor & Presença (Warmth)", "Vocal - Warmth & Presence")).clicked() {
                                        fx_config.eq.gains = [
                                            -12.0, -10.0, -6.0, -3.0, 0.0, 1.0, 2.0, 2.5,
                                            3.0, 2.5, 2.0, 1.0, 0.0, -1.0,
                                            -1.0, 0.0, 1.0, 2.0, 2.5, 3.0, 3.0, 2.5, 2.0, 1.5, 1.0, 0.5,
                                            0.0, 0.0, 0.0, -1.0, -2.0,
                                        ];
                                        ui.close_menu();
                                    }
                                    if ui.button(lang.tr("Vocal - Clareza (Anti-Muffled)", "Vocal - Clarity (Anti-Muffled)")).clicked() {
                                        fx_config.eq.gains = [
                                            -12.0, -10.0, -8.0, -5.0, -3.0, -1.0, 0.0, 0.0,
                                            -1.0, -2.0, -3.0, -3.5, -3.0, -2.0,
                                            -1.0, 0.0, 1.5, 2.5, 3.5, 4.0, 3.5, 3.0, 2.5, 2.0, 1.5, 1.0,
                                            0.5, 0.0, 0.0, 0.0, 0.0,
                                        ];
                                        ui.close_menu();
                                    }
                                    if ui.button(lang.tr("Corte de Sub-Graves (High-Pass 80Hz)", "Sub-Bass Cut (High-Pass 80Hz)")).clicked() {
                                        fx_config.eq.gains[0] = -12.0;
                                        fx_config.eq.gains[1] = -12.0;
                                        fx_config.eq.gains[2] = -12.0;
                                        fx_config.eq.gains[3] = -12.0;
                                        fx_config.eq.gains[4] = -10.0;
                                        fx_config.eq.gains[5] = -6.0;
                                        fx_config.eq.gains[6] = -2.0;
                                        ui.close_menu();
                                    }
                                });
                        });
                    });

                    ui.add_space(6.0);

                    // Interactive 31-band faders
                    egui::ScrollArea::horizontal()
                        .id_salt("eq_31_bands_scroll")
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = egui::Vec2::new(3.5, 0.0);
                                for i in 0..31 {
                                    let label = crate::audio::ISO_31_BAND_LABELS[i];
                                    let freq = crate::audio::ISO_31_BAND_FREQS[i];
                                    let gain = &mut fx_config.eq.gains[i];

                                    ui.vertical(|ui| {
                                        // Gain text display
                                        let val_str = if *gain > 0.0 {
                                            format!("+{:0.1}", *gain)
                                        } else {
                                            format!("{:0.1}", *gain)
                                        };
                                        ui.label(
                                            RichText::new(val_str)
                                                .size(8.5)
                                                .color(if gain.abs() > 0.1 {
                                                    theme.accent_c32()
                                                } else {
                                                    theme.text_muted_c32()
                                                }),
                                        );

                                        // Vertical slider
                                        let slider_resp = ui.add(
                                            egui::Slider::new(gain, -12.0..=12.0)
                                                .vertical()
                                                .show_value(false),
                                        ).on_hover_text(format!("Freq: {} Hz\n{}: {:+0.1} dB\n({})", freq, lang.tr("Ganho", "Gain"), *gain, lang.tr("Clique direito para resetar a 0 dB", "Right click to reset to 0 dB")));

                                        if slider_resp.clicked_by(egui::PointerButton::Secondary) {
                                            *gain = 0.0;
                                        }

                                        // Frequency label
                                        let lbl_resp = ui.label(
                                            RichText::new(label)
                                                .size(8.5)
                                                .color(theme.text_primary_c32()),
                                        ).on_hover_text(format!("{} Hz ({})", freq, lang.tr("Clique direito para resetar", "Right click to reset")));
                                        if lbl_resp.clicked_by(egui::PointerButton::Secondary) {
                                            *gain = 0.0;
                                        }
                                    });
                                }
                            });
                        });
                });

                ui.add_space(8.0);

                // 2. Dynamics & Spatial Modules Section
                let total_width = ui.available_width();
                let col_width = ((total_width - 8.0) / 2.0).max(280.0);

                ui.columns(2, |cols| {
                    // LEFT COLUMN: Compressor
                    cols[0].vertical(|ui| {
                        card_frame(fx_config.compressor.enabled).show(ui, |ui| {
                            ui.set_width(col_width);
                            ui.horizontal(|ui| {
                                ui.checkbox(
                                    &mut fx_config.compressor.enabled,
                                    RichText::new(lang.tr("Compressor Dinâmico", "Dynamic Compressor"))
                                        .strong()
                                        .size(11.5)
                                        .color(theme.accent_c32()),
                                );
                                if !fx_config.compressor.enabled {
                                    ui.label(
                                        RichText::new(lang.tr("(Bypass)", "(Bypass)"))
                                            .italics()
                                            .size(10.0)
                                            .color(theme.text_muted_c32()),
                                    );
                                }
                            });
                            ui.add_space(4.0);

                            egui::Grid::new("comp_grid")
                                .num_columns(2)
                                .spacing([8.0, 6.0])
                                .show(ui, |ui| {
                                    ui.label("Threshold:");
                                    ui.add(egui::Slider::new(&mut fx_config.compressor.threshold_db, -40.0..=0.0).suffix(" dB"));
                                    ui.end_row();

                                    ui.label("Ratio:");
                                    ui.add(egui::Slider::new(&mut fx_config.compressor.ratio, 1.0..=20.0).suffix(":1"));
                                    ui.end_row();

                                    ui.label(lang.tr("Ataque:", "Attack:"));
                                    ui.add(egui::Slider::new(&mut fx_config.compressor.attack_ms, 0.5..=100.0).suffix(" ms"));
                                    ui.end_row();

                                    ui.label("Release:");
                                    ui.add(egui::Slider::new(&mut fx_config.compressor.release_ms, 10.0..=1000.0).suffix(" ms"));
                                    ui.end_row();

                                    ui.label(lang.tr("Ganho Makeup:", "Makeup Gain:"));
                                    ui.add(egui::Slider::new(&mut fx_config.compressor.makeup_gain_db, 0.0..=24.0).suffix(" dB"));
                                    ui.end_row();
                                });
                        });

                        ui.add_space(8.0);

                        // Stereo Delay
                        card_frame(fx_config.delay.enabled).show(ui, |ui| {
                            ui.set_width(col_width);
                            ui.horizontal(|ui| {
                                ui.checkbox(
                                    &mut fx_config.delay.enabled,
                                    RichText::new(lang.tr("Delay Estéreo / Eco", "Stereo Delay / Echo"))
                                        .strong()
                                        .size(11.5)
                                        .color(theme.accent_c32()),
                                );
                                if !fx_config.delay.enabled {
                                    ui.label(
                                        RichText::new(lang.tr("(Bypass)", "(Bypass)"))
                                            .italics()
                                            .size(10.0)
                                            .color(theme.text_muted_c32()),
                                    );
                                }
                            });
                            ui.add_space(4.0);

                            egui::Grid::new("delay_grid")
                                .num_columns(2)
                                .spacing([8.0, 6.0])
                                .show(ui, |ui| {
                                    ui.label(lang.tr("Tempo:", "Time:"));
                                    ui.add(egui::Slider::new(&mut fx_config.delay.time_ms, 20.0..=1000.0).suffix(" ms"));
                                    ui.end_row();

                                    ui.label("Feedback:");
                                    ui.add(egui::Slider::new(&mut fx_config.delay.feedback, 0.0..=0.95));
                                    ui.end_row();

                                    ui.label("Mix Wet:");
                                    ui.add(egui::Slider::new(&mut fx_config.delay.wet_level, 0.0..=1.0));
                                    ui.end_row();
                                });

                            ui.add_space(2.0);
                            ui.checkbox(&mut fx_config.delay.ping_pong, lang.tr("Modo Ping-Pong (L/R)", "Ping-Pong Mode (L/R)"));
                        });
                    });

                    // RIGHT COLUMN: Reverb
                    cols[1].vertical(|ui| {
                        card_frame(fx_config.reverb.enabled).show(ui, |ui| {
                            ui.set_width(col_width);
                            ui.horizontal(|ui| {
                                ui.checkbox(
                                    &mut fx_config.reverb.enabled,
                                    RichText::new(lang.tr("Reverb Espacial (Freeverb)", "Spatial Reverb (Freeverb)"))
                                        .strong()
                                        .size(11.5)
                                        .color(theme.accent_c32()),
                                );
                                if !fx_config.reverb.enabled {
                                    ui.label(
                                        RichText::new(lang.tr("(Bypass)", "(Bypass)"))
                                            .italics()
                                            .size(10.0)
                                            .color(theme.text_muted_c32()),
                                    );
                                }
                            });
                            ui.add_space(4.0);

                            egui::Grid::new("reverb_grid")
                                .num_columns(2)
                                .spacing([8.0, 6.0])
                                .show(ui, |ui| {
                                    ui.label(lang.tr("Tamanho da Sala:", "Room Size:"));
                                    ui.add(egui::Slider::new(&mut fx_config.reverb.room_size, 0.0..=1.0));
                                    ui.end_row();

                                    ui.label(lang.tr("Amortecimento:", "Damping:"));
                                    ui.add(egui::Slider::new(&mut fx_config.reverb.damping, 0.0..=1.0));
                                    ui.end_row();

                                    ui.label(lang.tr("Largura Estéreo:", "Stereo Width:"));
                                    ui.add(egui::Slider::new(&mut fx_config.reverb.width, 0.0..=1.0));
                                    ui.end_row();

                                    ui.label(lang.tr("Mix Wet (Reverb):", "Wet Mix (Reverb):"));
                                    ui.add(egui::Slider::new(&mut fx_config.reverb.wet_level, 0.0..=1.0));
                                    ui.end_row();

                                    ui.label(lang.tr("Nível Dry:", "Dry Level:"));
                                    ui.add(egui::Slider::new(&mut fx_config.reverb.dry_level, 0.0..=1.0));
                                    ui.end_row();
                                });
                        });
                    });
                });
            });
    });
}
