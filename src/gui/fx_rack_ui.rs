//! DAW-style visual layer for the native DSP effects rack.
//!
//! The audio contract stays in `FxRackConfig` and `FxRackProcessor`. This
//! module owns only selection, presentation, SVG skins, and egui controls.

use crate::audio::{FxRackConfig, ISO_31_BAND_FREQS, ISO_31_BAND_LABELS};
use crate::config::AppLanguage;
use crate::gui::fx_rack_dialog::FxRackDialogState;
use crate::gui::theme::ThemeConfig;
use crate::project::model::UTrack;
use eframe::egui::{self, Frame, RichText, Stroke, Vec2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FxModule {
    Eq,
    Gate,
    Compressor,
    Delay,
    Reverb,
    Saturation,
    Chorus,
    AutoPan,
    Limiter,
    Phaser,
    Flanger,
    Widener,
}

impl FxModule {
    const ALL: [Self; 12] = [
        Self::Eq,
        Self::Gate,
        Self::Compressor,
        Self::Delay,
        Self::Reverb,
        Self::Saturation,
        Self::Chorus,
        Self::AutoPan,
        Self::Limiter,
        Self::Phaser,
        Self::Flanger,
        Self::Widener,
    ];

    fn index(self) -> u8 {
        Self::ALL.iter().position(|item| *item == self).unwrap_or(0) as u8
    }

    fn from_index(index: u8) -> Self {
        Self::ALL.get(index as usize).copied().unwrap_or(Self::Eq)
    }

    fn id(self) -> &'static str {
        match self {
            Self::Eq => "eq",
            Self::Gate => "gate",
            Self::Compressor => "compressor",
            Self::Delay => "delay",
            Self::Reverb => "reverb",
            Self::Saturation => "saturation",
            Self::Chorus => "chorus",
            Self::AutoPan => "autopan",
            Self::Limiter => "limiter",
            Self::Phaser => "phaser",
            Self::Flanger => "flanger",
            Self::Widener => "widener",
        }
    }

    fn name(self, lang: AppLanguage) -> &'static str {
        match self {
            Self::Eq => lang.tr("EQ 31", "EQ 31"),
            Self::Gate => lang.tr("GATE", "GATE"),
            Self::Compressor => lang.tr("COMP", "COMP"),
            Self::Delay => lang.tr("DELAY", "DELAY"),
            Self::Reverb => lang.tr("REVERB", "REVERB"),
            Self::Saturation => lang.tr("SAT", "SAT"),
            Self::Chorus => lang.tr("CHORUS", "CHORUS"),
            Self::AutoPan => lang.tr("PAN", "PAN"),
            Self::Limiter => lang.tr("LIMITER", "LIMITER"),
            Self::Phaser => "PHASER",
            Self::Flanger => "FLANGER",
            Self::Widener => "WIDENER",
        }
    }

    fn description(self, lang: AppLanguage) -> &'static str {
        match self {
            Self::Eq => lang.tr("31 bandas / 1⁄3 oitava", "31 bands / 1⁄3 octave"),
            Self::Gate => lang.tr("limpeza de ruído", "noise cleanup"),
            Self::Compressor => lang.tr("controle de dinâmica", "dynamic control"),
            Self::Delay => lang.tr("ecos estéreo", "stereo echoes"),
            Self::Reverb => lang.tr("espaço e cauda", "room and tail"),
            Self::Saturation => lang.tr("harmônicos suaves", "soft harmonics"),
            Self::Chorus => lang.tr("largura modulada", "modulated width"),
            Self::AutoPan => lang.tr("movimento L/R", "L/R movement"),
            Self::Limiter => lang.tr("teto de segurança", "safety ceiling"),
            Self::Phaser => lang.tr("modulação de fase", "phase modulation"),
            Self::Flanger => lang.tr("combinação de atraso modulada", "modulated delay comb"),
            Self::Widener => lang.tr("largura mid-side", "mid-side width"),
        }
    }

    fn enabled(self, config: &FxRackConfig) -> bool {
        match self {
            Self::Eq => config.eq.enabled,
            Self::Gate => config.gate.enabled,
            Self::Compressor => config.compressor.enabled,
            Self::Delay => config.delay.enabled,
            Self::Reverb => config.reverb.enabled,
            Self::Saturation => config.saturation.enabled,
            Self::Chorus => config.chorus.enabled,
            Self::AutoPan => config.auto_pan.enabled,
            Self::Limiter => config.limiter.enabled,
            Self::Phaser => config.phaser.enabled,
            Self::Flanger => config.flanger.enabled,
            Self::Widener => config.stereo_widener.enabled,
        }
    }

    fn svg(self) -> &'static [u8] {
        match self {
            Self::Eq => include_bytes!("../../assets/fx/eq.svg"),
            Self::Gate => include_bytes!("../../assets/fx/gate.svg"),
            Self::Compressor => include_bytes!("../../assets/fx/compressor.svg"),
            Self::Delay => include_bytes!("../../assets/fx/delay.svg"),
            Self::Reverb => include_bytes!("../../assets/fx/reverb.svg"),
            Self::Saturation => include_bytes!("../../assets/fx/saturation.svg"),
            Self::Chorus => include_bytes!("../../assets/fx/chorus.svg"),
            Self::AutoPan => include_bytes!("../../assets/fx/autopan.svg"),
            Self::Limiter => include_bytes!("../../assets/fx/limiter.svg"),
            Self::Phaser | Self::Flanger | Self::Widener => include_bytes!("../../assets/fx/chorus.svg"),
        }
    }
}

pub(crate) fn render_fx_rack_ui(
    ui: &mut egui::Ui,
    lang: AppLanguage,
    theme: &ThemeConfig,
    dialog_title: &str,
    state: &mut FxRackDialogState,
    tracks: &[UTrack],
    fx_config: &mut FxRackConfig,
    trigger_close: &mut bool,
) {
    let selected = FxModule::from_index(state.selected_module);
    let surface = theme.elevated_surface_c32();
    let border = theme.card_stroke();

    ui.horizontal(|ui| {
        ui.label(
            RichText::new("KAMAFEU / FX CONSOLE")
                .monospace()
                .size(10.0)
                .color(theme.accent_c32()),
        );
        ui.label(RichText::new("·").color(theme.text_muted_c32()));
        ui.label(
            RichText::new(dialog_title)
                .strong()
                .size(15.0)
                .color(theme.text_primary_c32()),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .button(RichText::new("×").size(18.0).color(theme.text_muted_c32()))
                .clicked()
            {
                *trigger_close = true;
            }
        });
    });

    ui.add_space(7.0);
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("TARGET")
                .monospace()
                .size(9.0)
                .color(theme.text_muted_c32()),
        );
        let target_name = match state.target_track {
            Some(index) => tracks
                .get(index)
                .map(|track| format!("TRACK {:02} / {}", index + 1, track.name))
                .unwrap_or_else(|| "TRACK".to_string()),
            None => "MASTER / GLOBAL".to_string(),
        };
        egui::ComboBox::from_id_salt("fx_modern_target")
            .selected_text(target_name)
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut state.target_track, None, "MASTER / GLOBAL");
                for (index, track) in tracks.iter().enumerate() {
                    ui.selectable_value(
                        &mut state.target_track,
                        Some(index),
                        format!("TRACK {:02} / {}", index + 1, track.name),
                    );
                }
            });
        ui.separator();
        let enabled_color = if fx_config.master_enabled {
            theme.success_c32()
        } else {
            theme.text_muted_c32()
        };
        ui.checkbox(
            &mut fx_config.master_enabled,
            RichText::new(lang.tr("RACK ON", "RACK ON"))
                .monospace()
                .size(10.0)
                .color(enabled_color),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .small_button(lang.tr("Resetar tudo", "Reset all"))
                .clicked()
            {
                *fx_config = FxRackConfig::default();
            }
        });
    });

    ui.add_space(9.0);
    ui.horizontal_wrapped(|ui| {
        for module in FxModule::ALL {
            let is_selected = selected == module;
            let is_enabled = module.enabled(fx_config);
            let color = if is_selected {
                theme.accent_c32()
            } else if is_enabled {
                theme.success_c32()
            } else {
                theme.text_muted_c32()
            };
            let label = format!(
                "{}  {}",
                if is_enabled { "●" } else { "○" },
                module.name(lang)
            );
            let button =
                egui::Button::new(RichText::new(label).monospace().size(10.0).color(color))
                    .fill(if is_selected {
                        theme.focus_glow_c32()
                    } else {
                        surface
                    })
                    .stroke(if is_selected {
                        Stroke::new(1.2_f32, theme.accent_c32())
                    } else {
                        border
                    });
            if ui.add(button).clicked() {
                state.selected_module = module.index();
            }
        }
    });

    ui.add_space(9.0);
    ui.separator();
    ui.add_space(9.0);

    egui::ScrollArea::vertical()
        .id_salt("fx_modern_editor_scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let available = ui.available_width();
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.set_width((available * 0.56).max(360.0));
                    ui.label(
                        RichText::new(selected.name(lang))
                            .heading()
                            .strong()
                            .color(theme.accent_c32()),
                    );
                    ui.label(
                        RichText::new(selected.description(lang))
                            .monospace()
                            .size(10.0)
                            .color(theme.text_muted_c32()),
                    );
                    ui.add_space(9.0);
                    render_editor(ui, selected, lang, theme, fx_config);
                });
                ui.add_space(14.0);
                ui.vertical(|ui| {
                    ui.set_width((available * 0.38).max(240.0));
                    Frame::none()
                        .fill(surface)
                        .stroke(border)
                        .rounding(theme.ui_rounding())
                        .inner_margin(10.0)
                        .show(ui, |ui| {
                            show_svg(ui, selected, Vec2::new(ui.available_width(), 112.0));
                            ui.add_space(8.0);
                            ui.label(
                                RichText::new("SIGNAL PATH")
                                    .monospace()
                                    .size(9.0)
                                    .color(theme.text_muted_c32()),
                            );
                            ui.add_space(4.0);
                            signal_path(ui, selected, theme);
                            ui.add_space(10.0);
                            meter(ui, theme, if fx_config.master_enabled { 0.72 } else { 0.0 });
                        });
                });
            });
        });
}

fn render_editor(
    ui: &mut egui::Ui,
    module: FxModule,
    lang: AppLanguage,
    theme: &ThemeConfig,
    config: &mut FxRackConfig,
) {
    let frame = Frame::none()
        .fill(theme.card_bg_c32())
        .stroke(theme.active_border_stroke())
        .rounding(theme.ui_rounding())
        .inner_margin(12.0);
    frame.show(ui, |ui| match module {
        FxModule::Eq => render_eq(ui, lang, theme, config),
        FxModule::Gate => {
            module_header(ui, &mut config.gate.enabled, "NOISE GATE", theme);
            knob_row(
                ui,
                theme,
                [
                    ("THRESH", &mut config.gate.threshold_db, -80.0..=0.0, " dB"),
                    ("ATTACK", &mut config.gate.attack_ms, 0.1..=100.0, " ms"),
                    ("RELEASE", &mut config.gate.release_ms, 5.0..=2000.0, " ms"),
                ],
            );
        }
        FxModule::Compressor => {
            module_header(ui, &mut config.compressor.enabled, "COMPRESSOR", theme);
            knob_row(
                ui,
                theme,
                [
                    (
                        "THRESH",
                        &mut config.compressor.threshold_db,
                        -40.0..=0.0,
                        " dB",
                    ),
                    ("RATIO", &mut config.compressor.ratio, 1.0..=20.0, ":1"),
                    (
                        "ATTACK",
                        &mut config.compressor.attack_ms,
                        0.5..=100.0,
                        " ms",
                    ),
                ],
            );
            knob_row(
                ui,
                theme,
                [
                    (
                        "RELEASE",
                        &mut config.compressor.release_ms,
                        10.0..=1000.0,
                        " ms",
                    ),
                    (
                        "MAKEUP",
                        &mut config.compressor.makeup_gain_db,
                        0.0..=24.0,
                        " dB",
                    ),
                    (
                        "GR",
                        &mut config.compressor.threshold_db,
                        -40.0..=0.0,
                        " dB",
                    ),
                ],
            );
        }
        FxModule::Delay => {
            module_header(ui, &mut config.delay.enabled, "STEREO DELAY", theme);
            knob_row(
                ui,
                theme,
                [
                    ("TIME", &mut config.delay.time_ms, 20.0..=1000.0, " ms"),
                    ("FEEDBACK", &mut config.delay.feedback, 0.0..=0.95, ""),
                    ("WET", &mut config.delay.wet_level, 0.0..=1.0, ""),
                ],
            );
            ui.checkbox(
                &mut config.delay.ping_pong,
                lang.tr("Ping-pong L/R", "Ping-pong L/R"),
            );
        }
        FxModule::Reverb => {
            module_header(ui, &mut config.reverb.enabled, "REVERB", theme);
            knob_row(
                ui,
                theme,
                [
                    ("ROOM", &mut config.reverb.room_size, 0.0..=1.0, ""),
                    ("DAMP", &mut config.reverb.damping, 0.0..=1.0, ""),
                    ("WIDTH", &mut config.reverb.width, 0.0..=1.0, ""),
                ],
            );
            knob_row(
                ui,
                theme,
                [
                    ("WET", &mut config.reverb.wet_level, 0.0..=1.0, ""),
                    ("DRY", &mut config.reverb.dry_level, 0.0..=1.0, ""),
                    ("ROOM", &mut config.reverb.room_size, 0.0..=1.0, ""),
                ],
            );
        }
        FxModule::Saturation => {
            module_header(ui, &mut config.saturation.enabled, "SOFT SATURATION", theme);
            knob_row(
                ui,
                theme,
                [
                    ("DRIVE", &mut config.saturation.drive_db, 0.0..=30.0, " dB"),
                    ("MIX", &mut config.saturation.mix, 0.0..=1.0, ""),
                ],
            );
        }
        FxModule::Chorus => {
            module_header(ui, &mut config.chorus.enabled, "STEREO CHORUS", theme);
            knob_row(
                ui,
                theme,
                [
                    ("RATE", &mut config.chorus.rate_hz, 0.05..=8.0, " Hz"),
                    ("DEPTH", &mut config.chorus.depth_ms, 0.1..=25.0, " ms"),
                    ("MIX", &mut config.chorus.mix, 0.0..=1.0, ""),
                ],
            );
        }
        FxModule::AutoPan => {
            module_header(ui, &mut config.auto_pan.enabled, "AUTO-PAN", theme);
            knob_row(
                ui,
                theme,
                [
                    ("RATE", &mut config.auto_pan.rate_hz, 0.05..=8.0, " Hz"),
                    ("DEPTH", &mut config.auto_pan.depth, 0.0..=1.0, ""),
                ],
            );
        }
        FxModule::Limiter => {
            module_header(ui, &mut config.limiter.enabled, "OUTPUT LIMITER", theme);
            knob_row(
                ui,
                theme,
                [(
                    "CEILING",
                    &mut config.limiter.ceiling_db,
                    -24.0..=0.0,
                    " dB",
                )],
            );
        }
        FxModule::Phaser => {
            module_header(ui, &mut config.phaser.enabled, "PHASER", theme);
            knob_row(ui, theme, [("RATE", &mut config.phaser.rate_hz, 0.03..=8.0, " Hz"), ("DEPTH", &mut config.phaser.depth, 0.0..=1.0, ""), ("MIX", &mut config.phaser.mix, 0.0..=1.0, "")]);
        }
        FxModule::Flanger => {
            module_header(ui, &mut config.flanger.enabled, "FLANGER", theme);
            knob_row(ui, theme, [("RATE", &mut config.flanger.rate_hz, 0.03..=8.0, " Hz"), ("DEPTH", &mut config.flanger.depth_ms, 0.1..=15.0, " ms"), ("FEEDBACK", &mut config.flanger.feedback, -0.9..=0.9, "")]);
            ui.add(egui::Slider::new(&mut config.flanger.mix, 0.0..=1.0).text("MIX"));
        }
        FxModule::Widener => {
            module_header(ui, &mut config.stereo_widener.enabled, "STEREO WIDENER", theme);
            ui.add(egui::Slider::new(&mut config.stereo_widener.width, 0.0..=2.0).text("WIDTH"));
        }
    });
}

fn render_eq(ui: &mut egui::Ui, lang: AppLanguage, theme: &ThemeConfig, config: &mut FxRackConfig) {
    module_header(ui, &mut config.eq.enabled, "GRAPHIC EQ / 31 BANDS", theme);
    ui.horizontal(|ui| {
        if ui.small_button(lang.tr("Flat", "Flat")).clicked() {
            config.eq.gains = [0.0; 31];
        }
        ui.label(
            RichText::new("-12 dB   0   +12 dB")
                .monospace()
                .size(9.0)
                .color(theme.text_muted_c32()),
        );
    });
    egui::ScrollArea::horizontal()
        .id_salt("fx_eq_bands")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                for index in 0..31 {
                    ui.vertical(|ui| {
                        let value = &mut config.eq.gains[index];
                        ui.label(
                            RichText::new(format!("{:+.1}", *value))
                                .monospace()
                                .size(8.0)
                                .color(theme.accent_c32()),
                        );
                        ui.add(
                            egui::Slider::new(value, -12.0..=12.0)
                                .vertical()
                                .show_value(false)
                                .text(""),
                        );
                        ui.label(
                            RichText::new(ISO_31_BAND_LABELS[index])
                                .monospace()
                                .size(8.0)
                                .color(theme.text_muted_c32()),
                        )
                        .on_hover_text(format!("{} Hz", ISO_31_BAND_FREQS[index]));
                    });
                }
            });
        });
}

fn module_header(ui: &mut egui::Ui, enabled: &mut bool, title: &str, theme: &ThemeConfig) {
    ui.horizontal(|ui| {
        ui.checkbox(
            enabled,
            RichText::new(title)
                .strong()
                .monospace()
                .size(12.0)
                .color(theme.accent_c32()),
        );
        ui.label(
            RichText::new(if *enabled { "ACTIVE" } else { "BYPASS" })
                .monospace()
                .size(9.0)
                .color(if *enabled {
                    theme.success_c32()
                } else {
                    theme.text_muted_c32()
                }),
        );
    });
    ui.add_space(10.0);
}

fn knob_row<const N: usize>(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    controls: [(&str, &mut f32, std::ops::RangeInclusive<f32>, &str); N],
) {
    ui.horizontal(|ui| {
        for (label, value, range, suffix) in controls {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new(label)
                        .monospace()
                        .size(9.0)
                        .color(theme.text_muted_c32()),
                );
                fx_knob(ui, value, range, suffix, theme);
            });
        }
    });
    ui.add_space(8.0);
}

fn fx_knob(
    ui: &mut egui::Ui,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
    theme: &ThemeConfig,
) {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(76.0, 76.0), egui::Sense::click_and_drag());
    if response.dragged() {
        let span = *range.end() - *range.start();
        *value =
            (*value - response.drag_delta().y * span / 160.0).clamp(*range.start(), *range.end());
    }
    let t = ((*value - *range.start()) / (*range.end() - *range.start()).max(f32::EPSILON))
        .clamp(0.0, 1.0);
    let center = rect.center();
    let radius = 24.0;
    let start = std::f32::consts::PI * 0.75;
    let angle = start + std::f32::consts::PI * 1.5 * t;
    let painter = ui.painter();
    painter.circle_filled(center, radius, theme.bg_header_c32());
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(2.0_f32, theme.grid_line_bar_c32()),
    );
    let arc_points: Vec<egui::Pos2> = (0..=24)
        .map(|step| {
            let progress = step as f32 / 24.0;
            let angle = start + std::f32::consts::PI * 1.5 * t * progress;
            center + Vec2::new(angle.cos(), angle.sin()) * (radius - 2.0)
        })
        .collect();
    painter.add(egui::Shape::line(
        arc_points,
        Stroke::new(3.0_f32, theme.accent_c32()),
    ));
    painter.line_segment(
        [
            center,
            center + Vec2::new(angle.cos(), angle.sin()) * (radius - 6.0),
        ],
        Stroke::new(2.0_f32, theme.text_primary_c32()),
    );
    painter.text(
        rect.center_bottom() + Vec2::new(0.0, -4.0),
        egui::Align2::CENTER_BOTTOM,
        format!("{:.2}{suffix}", *value),
        egui::FontId::monospace(9.0),
        theme.text_primary_c32(),
    );
}

fn show_svg(ui: &mut egui::Ui, module: FxModule, size: Vec2) {
    let uri = format!("bytes://kamafeu-fx-{}.svg", module.id());
    ui.add(egui::Image::from_bytes(uri, module.svg()).fit_to_exact_size(size));
}

fn signal_path(ui: &mut egui::Ui, selected: FxModule, theme: &ThemeConfig) {
    ui.horizontal_wrapped(|ui| {
        for (index, module) in FxModule::ALL.iter().enumerate() {
            let color = if *module == selected {
                theme.accent_c32()
            } else {
                theme.text_muted_c32()
            };
            ui.label(
                RichText::new(format!(
                    "{:02} {}",
                    index + 1,
                    module.name(AppLanguage::EnUs)
                ))
                .monospace()
                .size(9.0)
                .color(color),
            );
            if index + 1 < FxModule::ALL.len() {
                ui.label(RichText::new("→").color(theme.text_muted_c32()));
            }
        }
    });
}

fn meter(ui: &mut egui::Ui, theme: &ThemeConfig, level: f32) {
    let (rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 12.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 2.0, theme.bg_header_c32());
    let fill = egui::Rect::from_min_max(
        rect.min,
        egui::pos2(
            rect.min.x + rect.width() * level.clamp(0.0, 1.0),
            rect.max.y,
        ),
    );
    painter.rect_filled(fill, 2.0, theme.success_c32());
}
