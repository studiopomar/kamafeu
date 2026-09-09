mod audio_settings;
mod diagnostics;
mod dsp_settings;
mod experimental;
mod export_defaults;
mod memory_cache;
mod presets_and_help;
mod ui_workflow;
mod voicebank_tuning;

use super::theme::MelodyneTheme;
use super::KamafeuStudioApp;
use crate::audio::player::AudioPlayer;
use crate::config::KamafeuConfig;
use eframe::egui::{self, Color32, Frame, Margin, RichText, Rounding, Stroke};

pub struct PreferencesDialogState {
    pub active_tab: usize,
    pub detected_audio_devices: Vec<String>,
    pub last_device_scan: Option<std::time::Instant>,
    pub cache_status_message: Option<String>,
    pub benchmark_result: Option<String>,
    pub memory_stress_result: Option<String>,
    pub voicebank_audit_result: Option<String>,
    pub test_signal_type: usize,
    pub test_signal_freq: f32,
}

impl Default for PreferencesDialogState {
    fn default() -> Self {
        Self {
            active_tab: 0,
            detected_audio_devices: AudioPlayer::list_output_devices(),
            last_device_scan: Some(std::time::Instant::now()),
            cache_status_message: None,
            benchmark_result: None,
            memory_stress_result: None,
            voicebank_audit_result: None,
            test_signal_type: 0,
            test_signal_freq: 440.0,
        }
    }
}

impl PreferencesDialogState {
    pub fn refresh_devices(&mut self) {
        self.detected_audio_devices = AudioPlayer::list_output_devices();
        self.last_device_scan = Some(std::time::Instant::now());
    }
}

/// Procedural vector help badge (clean circle with vector glyph, no emoji)
fn help_marker(ui: &mut egui::Ui, text: &str) {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(15.0, 15.0), egui::Sense::hover());
    let is_hovered = response.hovered();
    let bg_color = if is_hovered {
        Color32::from_rgb(50, 70, 130)
    } else {
        Color32::from_rgb(32, 38, 58)
    };
    let border_color = if is_hovered {
        Color32::from_rgb(120, 190, 255)
    } else {
        Color32::from_rgb(60, 75, 110)
    };
    let text_color = if is_hovered {
        Color32::from_rgb(255, 255, 255)
    } else {
        Color32::from_rgb(160, 200, 255)
    };

    ui.painter()
        .circle(rect.center(), 7.0, bg_color, Stroke::new(1.0, border_color));
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "?",
        egui::FontId::monospace(10.0),
        text_color,
    );

    response.on_hover_ui(|ui| {
        ui.set_max_width(360.0);
        Frame::none()
            .fill(Color32::from_rgb(18, 14, 28))
            .stroke(Stroke::new(1.0, Color32::from_rgb(80, 60, 120)))
            .rounding(Rounding::same(5.0))
            .inner_margin(Margin::same(8.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(text)
                        .size(11.0)
                        .color(Color32::from_rgb(230, 235, 250)),
                );
            });
    });
}

fn section_card<R>(
    ui: &mut egui::Ui,
    title: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    ui.add_space(4.0);
    Frame::none()
        .fill(Color32::from_rgb(24, 18, 36))
        .rounding(Rounding::same(6.0))
        .stroke(Stroke::new(1.0, Color32::from_rgb(50, 40, 70)))
        .inner_margin(Margin::same(10.0))
        .show(ui, |ui| {
            ui.label(
                RichText::new(title)
                    .strong()
                    .size(12.5)
                    .color(Color32::from_rgb(0, 255, 157)),
            );
            ui.add_space(2.0);
            ui.separator();
            ui.add_space(6.0);
            add_contents(ui)
        })
        .inner
}

impl KamafeuStudioApp {
    pub fn render_preferences_dialog(&mut self, ctx: &egui::Context) {
        if !self.preferences_window_open {
            return;
        }

        let lang = self.config.language;
        let mut is_open = self.preferences_window_open;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("kamafeu_preferences_viewport"),
            egui::ViewportBuilder::default()
                .with_title(lang.tr("Preferências & Configurações Avançadas - Kamafeu Studio", "Preferences & Advanced Settings - Kamafeu Studio"))
                .with_inner_size([980.0, 720.0])
                .with_min_inner_size([800.0, 560.0]),
            |ctx, _class| {
                egui::CentralPanel::default()
                    .frame(Frame::none().fill(MelodyneTheme::BG_CANVAS).inner_margin(Margin::same(12.0)))
                    .show(ctx, |ui| {
                        // Header
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(lang.tr("Preferências & Configurações de Engenharia", "Engineering Preferences & Settings"))
                                    .strong()
                                    .size(16.0)
                                    .color(Color32::from_rgb(0, 255, 157)),
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button(RichText::new(lang.tr("Fechar", "Close")).size(11.0)).clicked() {
                                    is_open = false;
                                }
                            });
                        });
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(lang.tr(
                                "Comece pelas predefinições ou ajuste áudio, voicebanks e exportação. Os controles técnicos ficam reunidos no final. Passe o mouse sobre [?] para entender cada parâmetro.",
                                "Start with presets or adjust audio, voicebanks and export. Technical controls are grouped at the end. Hover over [?] to understand each parameter.",
                            ))
                            .size(11.0)
                            .color(MelodyneTheme::TEXT_MUTED),
                        );
                        ui.add_space(6.0);
                        ui.separator();
                        ui.add_space(6.0);

                        // Main tab selector
                        let tabs = [
                            (lang.tr("Início & Predefinições", "Home & Presets"), 7),
                            (lang.tr("Interface", "Interface"), 4),
                            (lang.tr("Áudio & Hardware", "Audio & Hardware"), 0),
                            (lang.tr("Voicebanks & oto.ini", "Voicebanks & oto.ini"), 5),
                            (lang.tr("Motor DSP & Síntese", "DSP Engine & Synthesis"), 1),
                            (lang.tr("Exportação", "Export"), 3),
                            (lang.tr("Cache", "Cache"), 2),
                            (lang.tr("Avançado", "Advanced"), 6),
                        ];

                        ui.horizontal_wrapped(|ui| {
                            for (label, idx) in tabs {
                                let is_active = self.preferences_tab == idx;
                                let bg = if is_active {
                                    Color32::from_rgb(60, 42, 90)
                                } else {
                                    Color32::from_rgb(32, 24, 46)
                                };
                                let text_color = if is_active {
                                    Color32::from_rgb(0, 255, 157)
                                } else {
                                    Color32::from_rgb(180, 170, 200)
                                };
                                let stroke = if is_active {
                                    Stroke::new(1.5, Color32::from_rgb(0, 255, 157))
                                } else {
                                    Stroke::new(1.0, Color32::from_rgb(50, 40, 70))
                                };

                                let btn = egui::Button::new(RichText::new(label).strong().size(11.5).color(text_color))
                                    .fill(bg)
                                    .stroke(stroke)
                                    .rounding(Rounding::same(4.0));

                                if ui.add(btn).clicked() {
                                    self.preferences_tab = idx;
                                }
                            }
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);

                        // Tab Content Area
                        egui::ScrollArea::vertical()
                            .id_salt("preferences_content_scroll")
                            .max_height(530.0)
                            .show(ui, |ui| {
                                match self.preferences_tab {
                                    0 => self.render_audio_settings_tab(ui),
                                    1 => self.render_dsp_settings_tab(ui),
                                    2 => self.render_memory_cache_tab(ui),
                                    3 => self.render_export_defaults_tab(ui),
                                    4 => self.render_ui_workflow_tab(ui, ctx),
                                    5 => self.render_voicebank_tuning_tab(ui),
                                    6 => self.render_experimental_tab(ui),
                                    7 => self.render_presets_and_help_tab(ui),
                                    _ => {}
                                }
                            });

                        // Footer with action buttons
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            if ui.button(RichText::new(lang.tr("Restaurar Padrões de Fábrica", "Restore Factory Defaults")).size(11.0).color(Color32::from_rgb(255, 180, 100))).clicked() {
                                let default_conf = KamafeuConfig::default();
                                self.config.audio = default_conf.audio;
                                self.config.dsp = default_conf.dsp;
                                self.config.memory = default_conf.memory;
                                self.config.export = default_conf.export;
                                self.config.workflow = default_conf.workflow;
                                self.config.voicebank_tuning = default_conf.voicebank_tuning;
                                self.config.experimental = default_conf.experimental;
                                self.persist_config();
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button(RichText::new(lang.tr("Aplicar & Fechar", "Apply & Close")).strong().size(11.5).color(Color32::from_rgb(0, 255, 180))).clicked() {
                                    self.persist_config();
                                    is_open = false;
                                }

                                if ui.button(RichText::new(lang.tr("Salvar Alterações", "Save Changes")).size(11.0)).clicked() {
                                    self.persist_config();
                                }
                            });
                        });
                    });

                if ctx.input(|i| i.viewport().close_requested()) {
                    is_open = false;
                }
            },
        );
        self.preferences_window_open = is_open;
    }
}
