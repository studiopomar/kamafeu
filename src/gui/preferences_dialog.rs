mod audio_settings;
mod diagnostics;
mod dsp_settings;
mod experimental;
mod export_defaults;
mod memory_cache;
mod packages;
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
    pub last_device_scan: Option<web_time::Instant>,
    pub cache_status_message: Option<String>,
    pub benchmark_result: Option<String>,
    pub memory_stress_result: Option<String>,
    pub voicebank_audit_result: Option<String>,
    pub test_signal_type: usize,
    pub test_signal_freq: f32,
    pub phonemizer_rule_mode: usize,
    pub update_check_in_progress: bool,
    pub update_result: Option<Result<crate::updater::GitHubRelease, String>>,
    pub update_receiver:
        Option<std::sync::mpsc::Receiver<Result<crate::updater::GitHubRelease, String>>>,
    pub automatic_check_started: bool,
    pub update_opened_automatically: bool,
}

impl Default for PreferencesDialogState {
    fn default() -> Self {
        Self {
            active_tab: 0,
            detected_audio_devices: AudioPlayer::list_output_devices(),
            last_device_scan: Some(web_time::Instant::now()),
            cache_status_message: None,
            benchmark_result: None,
            memory_stress_result: None,
            voicebank_audit_result: None,
            test_signal_type: 0,
            test_signal_freq: 440.0,
            phonemizer_rule_mode: 0,
            update_check_in_progress: false,
            update_result: None,
            update_receiver: None,
            automatic_check_started: false,
            update_opened_automatically: false,
        }
    }
}

impl PreferencesDialogState {
    pub fn refresh_devices(&mut self) {
        self.detected_audio_devices = AudioPlayer::list_output_devices();
        self.last_device_scan = Some(web_time::Instant::now());
    }

    pub fn request_update_check(&mut self) {
        if self.update_check_in_progress {
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (sender, receiver) = std::sync::mpsc::channel();
            self.update_receiver = Some(receiver);
            self.update_check_in_progress = true;
            std::thread::spawn(move || {
                let _ = sender.send(crate::updater::check_latest_release());
            });
        }
    }

    pub fn poll_update_check(&mut self) {
        if let Some(receiver) = self.update_receiver.take() {
            match receiver.try_recv() {
                Ok(result) => {
                    self.update_result = Some(result);
                    self.update_check_in_progress = false;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    self.update_receiver = Some(receiver);
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.update_result = Some(Err("verificação interrompida".to_string()));
                    self.update_check_in_progress = false;
                }
            }
        }
    }
}

/// Procedural vector help badge (clean circle with vector glyph, no emoji)
fn help_marker(ui: &mut egui::Ui, text: &str) {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(15.0, 15.0), egui::Sense::hover());
    let is_hovered = response.hovered();
    let visuals = ui.visuals();
    let bg_color = if is_hovered {
        visuals.selection.bg_fill
    } else {
        visuals.widgets.inactive.bg_fill
    };
    let border_color = if is_hovered {
        visuals.selection.stroke.color
    } else {
        visuals.widgets.inactive.fg_stroke.color
    };
    let text_color = if is_hovered {
        visuals.selection.stroke.color
    } else {
        visuals.text_color()
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
            .fill(ui.visuals().window_fill())
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .rounding(Rounding::same(5.0))
            .inner_margin(Margin::same(8.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(text)
                        .size(11.0)
                        .color(ui.visuals().text_color()),
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
    let visuals = ui.visuals().clone();
    Frame::none()
        .fill(visuals.panel_fill)
        .rounding(visuals.window_rounding)
        .stroke(visuals.widgets.noninteractive.bg_stroke)
        .inner_margin(Margin::same(10.0))
        .show(ui, |ui| {
            ui.label(
                RichText::new(title)
                    .strong()
                    .size(12.5)
                    .color(visuals.selection.stroke.color),
            );
            ui.add_space(2.0);
            ui.separator();
            ui.add_space(6.0);
            add_contents(ui)
        })
        .inner
}

impl KamafeuStudioApp {
    fn render_updates_tab(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.language;
        self.preferences_state.poll_update_check();
        section_card(
            ui,
            lang.tr("Atualizações do Kamafeu Studio", "Kamafeu Studio Updates"),
            |ui| {
                ui.label(lang.tr(
                "O Kamafeu consulta as releases oficiais no GitHub. Nenhum arquivo é instalado sem sua confirmação.",
                "Kamafeu checks official GitHub releases. No file is installed without your confirmation.",
            ));
                ui.add_space(8.0);
                ui.checkbox(
                    &mut self.config.updates.check_enabled,
                    lang.tr("Verificar atualizações", "Check for updates"),
                );
                ui.horizontal(|ui| {
                    ui.label(lang.tr("Frequência:", "Frequency:"));
                    egui::ComboBox::from_id_salt("update_frequency")
                        .selected_text(match self.config.updates.frequency.as_str() {
                            "daily" => lang.tr("Diária", "Daily"),
                            "monthly" => lang.tr("Mensal", "Monthly"),
                            "never" => lang.tr("Nunca", "Never"),
                            _ => lang.tr("Semanal", "Weekly"),
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.config.updates.frequency,
                                "daily".into(),
                                lang.tr("Diária", "Daily"),
                            );
                            ui.selectable_value(
                                &mut self.config.updates.frequency,
                                "weekly".into(),
                                lang.tr("Semanal", "Weekly"),
                            );
                            ui.selectable_value(
                                &mut self.config.updates.frequency,
                                "monthly".into(),
                                lang.tr("Mensal", "Monthly"),
                            );
                            ui.selectable_value(
                                &mut self.config.updates.frequency,
                                "never".into(),
                                lang.tr("Nunca", "Never"),
                            );
                        });
                });
                ui.checkbox(
                    &mut self.config.updates.automatic_download,
                    lang.tr(
                        "Abrir automaticamente a página da atualização",
                        "Automatically open the update page",
                    ),
                );
                if ui.button(lang.tr("Verificar agora", "Check now")).clicked() {
                    self.preferences_state.request_update_check();
                }
                if self.preferences_state.update_check_in_progress {
                    ui.spinner();
                    ui.label(lang.tr("Consultando GitHub…", "Checking GitHub…"));
                }
                if let Some(result) = &self.preferences_state.update_result {
                    match result {
                        Ok(release)
                            if crate::updater::is_newer(&release.tag_name, crate::APP_VERSION) =>
                        {
                            ui.separator();
                            ui.colored_label(
                                Color32::from_rgb(100, 220, 150),
                                format!(
                                    "{} {}",
                                    lang.tr("Atualização disponível:", "Update available:"),
                                    release.tag_name
                                ),
                            );
                            if let Some(name) = &release.name {
                                ui.label(name);
                            }
                            if ui
                                .button(lang.tr("Abrir página de download", "Open download page"))
                                .clicked()
                            {
                                crate::gui::open_external_url(&release.html_url);
                            }
                        }
                        Ok(_) => {
                            ui.label(lang.tr(
                                "Você está usando a versão mais recente.",
                                "You are using the latest version.",
                            ));
                        }
                        Err(error) => {
                            ui.colored_label(Color32::from_rgb(240, 150, 120), error);
                        }
                    }
                }
                if ui
                    .button(lang.tr("Salvar preferências", "Save preferences"))
                    .clicked()
                {
                    self.persist_config();
                }
            },
        );
    }

    fn phonemizer_rule_template(key: &str) -> String {
        format!(
            "# Kamafeu Phonemizer Rules\n# Método: {key}\n# Linguagem: DSL de substituição do Kamafeu (não é Rust/Python).\n# Uma regra por linha: alias_original => alias_novo\n# A comparação é exata e ocorre antes da busca no oto.ini.\n# Use # para comentários; linhas inválidas são ignoradas.\n#\n# Exemplos (descomente e altere conforme seu voicebank):\n# ka => ka\n# shi => si\n# -a => - a\n#\n# Dica: mantenha o alias à direita exatamente como existe no oto.ini.\n"
        )
    }

    fn render_phonemizer_rules_tab(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.language;
        let modes = [
            ("BasicCV", "Japonês CV(VV)"),
            ("VCV", "Japonês VCV"),
            ("CVVC", "Japonês CVVC"),
            ("EnglishArpasing", "Inglês Arpasing"),
            ("EnglishVCCV", "Inglês VCCV"),
            ("PortugueseBrapaVCCV", "BRAPA VCCV"),
            ("PortugueseBrapaCVC", "BRAPA CVC"),
        ];
        section_card(
            ui,
            lang.tr(
                "Regras editáveis do fonemizador",
                "Editable phonemizer rules",
            ),
            |ui| {
                Frame::none()
                    .fill(Color32::from_rgb(72, 48, 22))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(235, 165, 70)))
                    .rounding(Rounding::same(4.0))
                    .inner_margin(Margin::same(7.0))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(lang.tr(
                                "AVISO — Função experimental: as regras podem não funcionar em todos os voicebanks. Regras inválidas são ignoradas ou podem gerar aliases sem correspondência no oto.ini.",
                                "WARNING — Experimental feature: rules may not work with every voicebank. Invalid rules are ignored or may produce aliases with no oto.ini match.",
                            ))
                            .strong()
                            .color(Color32::from_rgb(255, 220, 150)),
                        );
                    });
                ui.add_space(6.0);
                ui.label(lang.tr("Uma regra por linha: alias_original => alias_novo. As alterações entram em vigor imediatamente.", "One rule per line: original_alias => new_alias. Changes apply immediately."));
                ui.label(lang.tr("Use # para comentários. O botão Restaurar remove as regras personalizadas deste método.", "Use # for comments. Restore removes custom rules for this method."));
                ui.label(lang.tr("A regra transforma somente o lyric exato antes do fonemizador procurar o alias no oto.ini; não use sintaxe Rust, Python ou regex.", "Rules transform only the exact lyric before the phonemizer searches oto.ini; do not use Rust, Python, or regex syntax."));
                let selected = self
                    .preferences_state
                    .phonemizer_rule_mode
                    .min(modes.len() - 1);
                self.preferences_state.phonemizer_rule_mode = selected;
                egui::ComboBox::from_id_salt("phonemizer_rule_mode")
                    .selected_text(modes[selected].1)
                    .show_ui(ui, |ui| {
                        for (idx, (_, label)) in modes.iter().enumerate() {
                            ui.selectable_value(
                                &mut self.preferences_state.phonemizer_rule_mode,
                                idx,
                                *label,
                            );
                        }
                    });
                let key = modes[self.preferences_state.phonemizer_rule_mode]
                    .0
                    .to_string();
                let template = Self::phonemizer_rule_template(&key);
                let script = self
                    .config
                    .phonemizer_rules
                    .entry(key.clone())
                    .or_insert(template);
                let changed = ui
                    .add(
                        egui::TextEdit::multiline(script)
                            .desired_rows(12)
                            .desired_width(f32::INFINITY),
                    )
                    .changed();
                ui.horizontal(|ui| {
                    if ui
                        .button(lang.tr(
                            "Restaurar padrão deste fonemizador",
                            "Restore this phonemizer default",
                        ))
                        .clicked()
                    {
                        self.config
                            .phonemizer_rules
                            .insert(key.clone(), Self::phonemizer_rule_template(&key));
                        crate::phonemizer::set_custom_rules(self.config.phonemizer_rules.clone());
                        self.piano_roll_state.phoneme_cache_hash = 0;
                        self.persist_config();
                    }
                    if changed {
                        crate::phonemizer::set_custom_rules(self.config.phonemizer_rules.clone());
                        self.piano_roll_state.phoneme_cache_hash = 0;
                        self.persist_config();
                    }
                });
            },
        );
    }

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
                    .frame(Frame::none().fill(ctx.style().visuals.window_fill()).inner_margin(Margin::same(12.0)))
                    .show(ctx, |ui| {
                        // Header
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(lang.tr("Preferências & Configurações de Engenharia", "Engineering Preferences & Settings"))
                                    .strong()
                                    .size(16.0)
                                    .color(ctx.style().visuals.selection.stroke.color),
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
                            (lang.tr("Regras de Fonemização", "Phonemizer Rules"), 8),
                            (lang.tr("Pacotes adicionais", "Additional Packages"), 9),
                            (lang.tr("Atualizações", "Updates"), 10),
                        ];

                        ui.horizontal_wrapped(|ui| {
                            for (label, idx) in tabs {
                                let is_active = self.preferences_tab == idx;
                                let bg = if is_active {
                                    ctx.style().visuals.selection.bg_fill
                                } else {
                                    ctx.style().visuals.widgets.inactive.bg_fill
                                };
                                let text_color = if is_active {
                                    ctx.style().visuals.selection.stroke.color
                                } else {
                                    ctx.style().visuals.text_color()
                                };
                                let stroke = if is_active {
                                    ctx.style().visuals.selection.stroke
                                } else {
                                    ctx.style().visuals.widgets.inactive.bg_stroke
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
                                    8 => self.render_phonemizer_rules_tab(ui),
                                    9 => self.render_packages_tab(ui),
                                    10 => self.render_updates_tab(ui),
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
                                self.config.layout = default_conf.layout;
                                self.config.ui_scale_factor = default_conf.ui_scale_factor;
                                self.config.workflow = default_conf.workflow;
                                self.config.voicebank_tuning = default_conf.voicebank_tuning;
                                self.config.experimental = default_conf.experimental;
                                self.config.phonemizer_rules = default_conf.phonemizer_rules;
                                crate::phonemizer::set_custom_rules(self.config.phonemizer_rules.clone());
                                self.piano_roll_state.show_arrangement_view = self.config.layout.show_arrangement_view;
                                self.piano_roll_state.show_parameters_drawer = self.config.layout.show_parameters_drawer;
                                self.piano_roll_state.show_phoneme_ruler = self.config.layout.show_phoneme_ruler;
                                self.piano_roll_state.show_inspector = self.config.layout.show_inspector;
                                self.piano_roll_state.px_per_ms = self.config.layout.px_per_ms;
                                self.piano_roll_state.row_height = self.config.layout.row_height;
                                self.piano_roll_state.show_minimap = self.config.layout.show_minimap;
                                self.piano_roll_state.show_waveform_area = self.config.layout.show_waveform_area;
                                self.piano_roll_state.show_envelope_handles = self.config.layout.show_envelope_handles;
                                self.piano_roll_state.vertical_pitch_follow = self.config.layout.vertical_pitch_follow;
                                self.piano_roll_state.active_scale = match self.config.layout.default_scale.as_str() {
                                    "Major" => crate::gui::piano_roll::MusicalScale::Major,
                                    "NaturalMinor" => crate::gui::piano_roll::MusicalScale::NaturalMinor,
                                    "HarmonicMinor" => crate::gui::piano_roll::MusicalScale::HarmonicMinor,
                                    "MelodicMinor" => crate::gui::piano_roll::MusicalScale::MelodicMinor,
                                    "PentatonicMajor" => crate::gui::piano_roll::MusicalScale::PentatonicMajor,
                                    "PentatonicMinor" => crate::gui::piano_roll::MusicalScale::PentatonicMinor,
                                    "Blues" => crate::gui::piano_roll::MusicalScale::Blues,
                                    "Dorian" => crate::gui::piano_roll::MusicalScale::Dorian,
                                    "Mixolydian" => crate::gui::piano_roll::MusicalScale::Mixolydian,
                                    _ => crate::gui::piano_roll::MusicalScale::Chromatic,
                                };
                                self.piano_roll_state.scale_root_key = self.config.layout.default_scale_root_key;
                                self.piano_roll_state.active_tool = match self.config.workflow.default_edit_tool.as_str() {
                                    "Pencil" => crate::gui::types::EditTool::Pencil,
                                    "PitchDraw" => crate::gui::types::EditTool::PitchDraw,
                                    "Slice" => crate::gui::types::EditTool::Slice,
                                    "Eraser" => crate::gui::types::EditTool::Eraser,
                                    _ => crate::gui::types::EditTool::Pointer,
                                };
                                self.piano_roll_state.pitch_sub_tool = match self.config.workflow.default_pitch_sub_tool.as_str() {
                                    "Smooth" => crate::gui::types::PitchSubTool::Smooth,
                                    "Line" => crate::gui::types::PitchSubTool::Line,
                                    "Vibrato" => crate::gui::types::PitchSubTool::Vibrato,
                                    _ => crate::gui::types::PitchSubTool::Freehand,
                                };
                                self.piano_roll_state.default_note_lyric =
                                    self.config.workflow.default_note_lyric.clone();
                                self.piano_roll_state.default_note_dynamics =
                                    self.config.workflow.default_note_dynamics;
                                self.piano_roll_state.default_note_volume =
                                    self.config.workflow.default_note_volume;
                                self.piano_roll_state.default_note_attack =
                                    self.config.workflow.default_note_attack;
                                self.piano_roll_state.default_note_decay =
                                    self.config.workflow.default_note_decay;
                                self.piano_roll_state.ui_animations_enabled =
                                    self.config.workflow.ui_animations_enabled;
                                self.piano_roll_state.ui_animation_speed =
                                    self.config.workflow.ui_animation_speed;
                                self.humanize_dialog_state.humanize_params.timing_jitter_ms =
                                    self.config.workflow.humanize_timing_jitter_ms;
                                self.humanize_dialog_state.humanize_params.pitch_cents_jitter =
                                    self.config.workflow.humanize_pitch_cents_jitter;
                                self.humanize_dialog_state.humanize_params.volume_jitter_pct =
                                    self.config.workflow.humanize_volume_jitter_pct;
                                self.humanize_dialog_state.humanize_params.breathiness_jitter_pct =
                                    self.config.workflow.humanize_breathiness_jitter_pct;
                                self.humanize_dialog_state.vibrato_params.min_duration_ms = self.config.workflow.auto_vibrato_min_duration_ms;
                                self.humanize_dialog_state.vibrato_params.length_pct = self.config.workflow.auto_vibrato_length_pct;
                                self.humanize_dialog_state.vibrato_params.depth_cents = self.config.workflow.auto_vibrato_depth_cents;
                                self.humanize_dialog_state.vibrato_params.period_ms = self.config.workflow.auto_vibrato_period_ms;
                                self.humanize_dialog_state.vibrato_params.fade_in_pct = self.config.workflow.auto_vibrato_fade_in_pct;
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
