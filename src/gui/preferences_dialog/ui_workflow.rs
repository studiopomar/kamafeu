use super::help_marker;
use super::section_card;
use super::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(in crate::gui) fn render_ui_workflow_tab(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
    ) {
        let lang = self.config.language;
        section_card(
            ui,
            lang.tr("Idioma & Localização", "Language & Locale"),
            |ui| {
                section_card(
                    ui,
                    lang.tr("Auto-vibrato padrão", "Default auto-vibrato"),
                    |ui| {
                        let mut changed = false;
                        ui.horizontal(|ui| {
                            ui.label(lang.tr("Duração mínima:", "Minimum duration:"));
                            changed |= ui
                                .add(
                                    egui::Slider::new(
                                        &mut self.config.workflow.auto_vibrato_min_duration_ms,
                                        80.0..=2000.0,
                                    )
                                    .suffix(" ms"),
                                )
                                .changed();
                        });
                        ui.horizontal(|ui| {
                            ui.label(lang.tr("Profundidade:", "Depth:"));
                            changed |= ui
                                .add(
                                    egui::Slider::new(
                                        &mut self.config.workflow.auto_vibrato_depth_cents,
                                        10.0..=150.0,
                                    )
                                    .suffix(" cents"),
                                )
                                .changed();
                        });
                        ui.horizontal(|ui| {
                            ui.label(lang.tr("Período:", "Period:"));
                            changed |= ui
                                .add(
                                    egui::Slider::new(
                                        &mut self.config.workflow.auto_vibrato_period_ms,
                                        80.0..=300.0,
                                    )
                                    .suffix(" ms"),
                                )
                                .changed();
                        });
                        ui.horizontal(|ui| {
                            ui.label(lang.tr("Comprimento:", "Length:"));
                            changed |= ui
                                .add(
                                    egui::Slider::new(
                                        &mut self.config.workflow.auto_vibrato_length_pct,
                                        20.0..=100.0,
                                    )
                                    .suffix(" %"),
                                )
                                .changed();
                        });
                        ui.horizontal(|ui| {
                            ui.label(lang.tr("Fade-in:", "Fade-in:"));
                            changed |= ui
                                .add(
                                    egui::Slider::new(
                                        &mut self.config.workflow.auto_vibrato_fade_in_pct,
                                        5.0..=60.0,
                                    )
                                    .suffix(" %"),
                                )
                                .changed();
                        });
                        if changed {
                            self.config.workflow.auto_vibrato_min_duration_ms = self
                                .config
                                .workflow
                                .auto_vibrato_min_duration_ms
                                .clamp(80.0, 2000.0);
                            self.config.workflow.auto_vibrato_depth_cents = self
                                .config
                                .workflow
                                .auto_vibrato_depth_cents
                                .clamp(10.0, 150.0);
                            self.config.workflow.auto_vibrato_period_ms = self
                                .config
                                .workflow
                                .auto_vibrato_period_ms
                                .clamp(80.0, 300.0);
                            self.config.workflow.auto_vibrato_length_pct = self
                                .config
                                .workflow
                                .auto_vibrato_length_pct
                                .clamp(20.0, 100.0);
                            self.config.workflow.auto_vibrato_fade_in_pct = self
                                .config
                                .workflow
                                .auto_vibrato_fade_in_pct
                                .clamp(5.0, 60.0);
                            self.humanize_dialog_state.vibrato_params.min_duration_ms =
                                self.config.workflow.auto_vibrato_min_duration_ms;
                            self.humanize_dialog_state.vibrato_params.depth_cents =
                                self.config.workflow.auto_vibrato_depth_cents;
                            self.humanize_dialog_state.vibrato_params.period_ms =
                                self.config.workflow.auto_vibrato_period_ms;
                            self.humanize_dialog_state.vibrato_params.length_pct =
                                self.config.workflow.auto_vibrato_length_pct;
                            self.humanize_dialog_state.vibrato_params.fade_in_pct =
                                self.config.workflow.auto_vibrato_fade_in_pct;
                            self.persist_config();
                        }
                        help_marker(
                            ui,
                            lang.tr(
                                "Padrões usados pelo Auto-vibrato Inteligente.",
                                "Defaults used by Smart Auto-Vibrato.",
                            ),
                        );
                    },
                );

                ui.horizontal(|ui| {
                ui.label(lang.tr("Idioma da Interface:", "Interface Language:"));
                for (l, label) in [
                    (crate::config::AppLanguage::PtBr, lang.tr("Brasileiro (Brasil)", "Portuguese (Brazil)")),
                    (crate::config::AppLanguage::EnUs, lang.tr("Inglês (Global)", "English (Global)")),
                ] {
                    let is_active = self.config.language == l;
                    if ui.selectable_label(is_active, label).clicked() {
                        self.config.language = l;
                        self.persist_config();
                    }
                }
                help_marker(
                    ui,
                    lang.tr(
                        "Define o idioma padrão dos menus, tooltips e painéis de controle do Kamafeu Studio.",
                        "Sets the default language for Kamafeu Studio menus, tooltips, and control panels.",
                    ),
                );
            });
            },
        );

        section_card(
            ui,
            lang.tr(
                "Escala da Interface Gráfica & Acessibilidade",
                "UI Scale & Accessibility",
            ),
            |ui| {
                ui.horizontal(|ui| {
                ui.label(lang.tr("Escala de Zoom da Interface (UI Scale):", "UI Scale Factor:"));
                let mut scale = self.config.ui_scale_factor;
                if ui.add(egui::Slider::new(&mut scale, 0.75..=2.0).step_by(0.05).custom_formatter(|v, _| format!("{:.0}%", v * 100.0))).changed() {
                    self.config.ui_scale_factor = scale;
                    ctx.set_pixels_per_point(scale);
                }
                help_marker(
                    ui,
                    lang.tr(
                        "Aumenta ou reduz o tamanho dos textos, botões e controles para monitores 4K ou telas compactas.",
                        "Increases or decreases the size of texts, buttons, and controls for 4K monitors or compact screens.",
                    ),
                );
            });
            },
        );

        section_card(
            ui,
            lang.tr("Movimento da Interface", "Interface Motion"),
            |ui| {
                if ui
                    .checkbox(
                        &mut self.config.workflow.ui_animations_enabled,
                        lang.tr("Ativar animações suaves", "Enable smooth animations"),
                    )
                    .changed()
                {
                    self.piano_roll_state.ui_animations_enabled =
                        self.config.workflow.ui_animations_enabled;
                }
                ui.horizontal(|ui| {
                    ui.label(lang.tr("Velocidade:", "Speed:"));
                    ui.add_enabled(
                        self.config.workflow.ui_animations_enabled,
                        egui::Slider::new(&mut self.config.workflow.ui_animation_speed, 0.25..=4.0)
                            .suffix("x"),
                    );
                    self.config.workflow.ui_animation_speed =
                        self.config.workflow.ui_animation_speed.clamp(0.25, 4.0);
                    self.piano_roll_state.ui_animation_speed =
                        self.config.workflow.ui_animation_speed;
                });
                help_marker(
                    ui,
                    lang.tr(
                        "Controla as transições de hover, seleção e painéis. Desative para reduzir movimento visual.",
                        "Controls hover, selection, and panel transitions. Disable to reduce visual motion.",
                    ),
                );
            },
        );

        section_card(
            ui,
            lang.tr("Canvas do Piano Roll", "Piano Roll Canvas"),
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(lang.tr("Altura das linhas:", "Row height:"));
                    if ui
                        .add(
                            egui::Slider::new(&mut self.config.layout.row_height, 12.0..=48.0)
                                .suffix(" px"),
                        )
                        .changed()
                    {
                        self.piano_roll_state.row_height = self.config.layout.row_height;
                        self.persist_config();
                    }
                    help_marker(
                        ui,
                        lang.tr(
                            "Aumente para editar notas com mais precisão ou reduza para visualizar mais oitavas.",
                            "Increase for precise editing or reduce to see more octaves.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label(lang.tr("Zoom horizontal inicial:", "Initial horizontal zoom:"));
                    if ui
                        .add(
                            egui::Slider::new(&mut self.config.layout.px_per_ms, 0.04..=1.5)
                                .logarithmic(true),
                        )
                        .changed()
                    {
                        self.piano_roll_state.px_per_ms = self.config.layout.px_per_ms;
                        self.persist_config();
                    }
                    help_marker(
                        ui,
                        lang.tr(
                            "Controla quanto tempo cabe na tela quando o projeto é aberto.",
                            "Controls how much time fits on screen when the project opens.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label(lang.tr("Faixa vertical inicial:", "Initial vertical range:"));
                    let mut min_midi = self.config.layout.default_min_midi;
                    let mut max_midi = self.config.layout.default_max_midi;
                    let min_changed = ui
                        .add(egui::Slider::new(&mut min_midi, 0..=126).text(lang.tr("mín", "min")))
                        .changed();
                    let max_changed = ui
                        .add(egui::Slider::new(&mut max_midi, 1..=127).text(lang.tr("máx", "max")))
                        .changed();
                    if min_changed || max_changed {
                        if min_midi >= max_midi {
                            if min_changed {
                                max_midi = min_midi.saturating_add(1).min(127);
                            } else {
                                min_midi = max_midi.saturating_sub(1);
                            }
                        }
                        self.config.layout.default_min_midi = min_midi;
                        self.config.layout.default_max_midi = max_midi;
                        self.piano_roll_state.min_midi = min_midi;
                        self.piano_roll_state.max_midi = max_midi;
                        self.persist_config();
                    }
                    help_marker(
                        ui,
                        lang.tr(
                            "Define as notas mais grave e mais aguda visíveis quando um projeto é aberto.",
                            "Defines the lowest and highest visible notes when a project opens.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label(lang.tr("Escala musical inicial:", "Initial musical scale:"));
                    egui::ComboBox::from_id_salt("preferences_default_scale")
                        .selected_text(
                            crate::gui::piano_roll::MusicalScale::ALL
                                .iter()
                                .find(|scale| {
                                    format!("{:?}", scale) == self.config.layout.default_scale
                                })
                                .map(|scale| scale.display_name())
                                .unwrap_or("Cromática (Livre)"),
                        )
                        .show_ui(ui, |ui| {
                            for scale in crate::gui::piano_roll::MusicalScale::ALL {
                                if ui
                                    .selectable_value(
                                        &mut self.config.layout.default_scale,
                                        format!("{:?}", scale),
                                        scale.display_name(),
                                    )
                                    .changed()
                                {
                                    self.piano_roll_state.active_scale = scale;
                                    self.persist_config();
                                }
                            }
                        });
                });

                ui.horizontal(|ui| {
                    ui.label(lang.tr("Nota raiz:", "Root note:"));
                    egui::ComboBox::from_id_salt("preferences_default_scale_root")
                        .selected_text(
                            crate::gui::piano_roll::ROOT_NOTE_NAMES
                                [self.config.layout.default_scale_root_key.min(11) as usize],
                        )
                        .show_ui(ui, |ui| {
                            for (idx, name) in
                                crate::gui::piano_roll::ROOT_NOTE_NAMES.iter().enumerate()
                            {
                                if ui
                                    .selectable_value(
                                        &mut self.config.layout.default_scale_root_key,
                                        idx as u8,
                                        *name,
                                    )
                                    .changed()
                                {
                                    self.piano_roll_state.scale_root_key = idx as u8;
                                    self.persist_config();
                                }
                            }
                        });
                });

                ui.horizontal(|ui| {
                    if ui
                        .checkbox(
                            &mut self.config.layout.show_minimap,
                            lang.tr("Mostrar minimapa do Piano Roll", "Show Piano Roll minimap"),
                        )
                        .changed()
                    {
                        self.piano_roll_state.show_minimap = self.config.layout.show_minimap;
                        self.persist_config();
                    }
                    help_marker(
                        ui,
                        lang.tr(
                            "Exibe uma visão geral para navegar rapidamente por projetos longos.",
                            "Shows an overview for quickly navigating long projects.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    if ui
                        .checkbox(
                            &mut self.config.layout.show_waveform_area,
                            lang.tr("Mostrar área de waveform", "Show waveform area"),
                        )
                        .changed()
                    {
                        self.piano_roll_state.show_waveform_area =
                            self.config.layout.show_waveform_area;
                        self.persist_config();
                    }
                    help_marker(
                        ui,
                        lang.tr(
                            "Libera espaço vertical quando você quer trabalhar somente com notas e parâmetros.",
                            "Frees vertical space when you want to work only with notes and parameters.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    if ui
                        .checkbox(
                            &mut self.config.layout.show_envelope_handles,
                            lang.tr("Exibir pontos de envelope", "Show envelope handles"),
                        )
                        .changed()
                    {
                        self.piano_roll_state.show_envelope_handles =
                            self.config.layout.show_envelope_handles;
                        self.persist_config();
                    }
                    help_marker(
                        ui,
                        lang.tr(
                            "Mantém os pontos P1–P5 visíveis para edição rápida dos envelopes.",
                            "Keeps P1–P5 points visible for quick envelope editing.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    if ui
                        .checkbox(
                            &mut self.config.layout.vertical_pitch_follow,
                            lang.tr(
                                "Acompanhar notas verticalmente ao tocar",
                                "Follow notes vertically during playback",
                            ),
                        )
                        .changed()
                    {
                        self.piano_roll_state.vertical_pitch_follow =
                            self.config.layout.vertical_pitch_follow;
                        self.persist_config();
                    }
                    help_marker(
                        ui,
                        lang.tr(
                            "Move a visão vertical para manter as notas em reprodução visíveis.",
                            "Moves the vertical view to keep playing notes visible.",
                        ),
                    );
                });
            },
        );

        section_card(
            ui,
            lang.tr(
                "Fluxo de Trabalho, Salvamento & Histórico Undo",
                "Workflow, Saving & Undo History",
            ),
            |ui| {
                ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.config.workflow.auto_save_enabled,
                    lang.tr("Ativar Salvamento Automático Periódico (Auto-save)", "Enable Periodic Auto-save"),
                );
                if self.config.workflow.auto_save_enabled {
                    ui.add(egui::Slider::new(&mut self.config.workflow.auto_save_interval_sec, 30..=600).suffix(lang.tr(" seg", " sec")));
                }
                help_marker(
                    ui,
                    lang.tr(
                        "Salva um arquivo de recuperação do projeto a cada intervalo determinado para proteger contra imprevistos.",
                        "Saves a project recovery file at regular intervals to protect against unforeseen crashes.",
                    ),
                );
            });

                ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.config.workflow.backup_on_save,
                    lang.tr("Criar arquivo de backup (.bak) ao salvar o projeto", "Create backup file (.bak) when saving project"),
                );
                help_marker(
                    ui,
                    lang.tr(
                        "Mantém uma cópia da versão anterior sempre que você salvar manualmente o projeto.",
                        "Keeps a copy of the previous version whenever you manually save the project.",
                    ),
                );
            });

                ui.horizontal(|ui| {
                    ui.label(lang.tr(
                        "Passos Máximos de Desfazer (Undo History):",
                        "Maximum Undo Steps:",
                    ));
                    ui.add(
                        egui::Slider::new(&mut self.config.workflow.max_undo_steps, 20..=500)
                            .suffix(lang.tr(" passos", " steps")),
                    );
                    help_marker(
                        ui,
                        lang.tr(
                            "Número máximo de ações que podem ser desfeitas com Ctrl+Z.",
                            "Maximum number of actions that can be undone with Ctrl+Z.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.config.workflow.note_audition_on_click,
                    lang.tr("Tocar tom da nota ao clicar/arrastar no Piano Roll", "Audition note pitch on click/drag in Piano Roll"),
                );
                help_marker(
                    ui,
                    lang.tr(
                        "Emite o som da nota ao clicar no teclado guia ou selecionar uma nota.",
                        "Plays the note tone when clicking on the piano guide keys or selecting a note.",
                    ),
                );
            });

                ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.config.discord_rpc_enabled,
                    lang.tr("Integração Discord Rich Presence (Exibir projeto e status no Discord)", "Discord Rich Presence Integration (Show project and status in Discord)"),
                );
                help_marker(
                    ui,
                    lang.tr(
                        "Exibe seu status no perfil do Discord mostrando que está trabalhando no Kamafeu Studio.",
                        "Shows your status on your Discord profile indicating you are working in Kamafeu Studio.",
                    ),
                );
            });

                ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.config.workflow.confirm_on_exit_dirty,
                    lang.tr("Pedir confirmação ao fechar o programa caso haja alterações não salvas", "Confirm before closing if there are unsaved changes"),
                );
                help_marker(
                    ui,
                    lang.tr(
                        "Exibe aviso de confirmação para salvar o projeto antes de fechar a janela.",
                        "Prompts for confirmation to save the project before closing the window.",
                    ),
                );
            });

                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.config.workflow.allow_overlapping_notes,
                        lang.tr("Permitir sobreposição de notas", "Allow overlapping notes"),
                    );
                    help_marker(
                        ui,
                        lang.tr(
                            "Desativado: ao mover ou criar notas, a duração da nota anterior é ajustada para evitar sobreposição.",
                            "Disabled: moving or creating notes adjusts the previous note to avoid overlap.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.config.workflow.audition_on_lyric_change,
                        lang.tr(
                            "Tocar nota ao alterar o lyric",
                            "Audition note when changing lyric",
                        ),
                    );
                    help_marker(
                        ui,
                        lang.tr(
                            "Reproduz uma prévia curta quando o texto da nota é alterado.",
                            "Plays a short preview when a note lyric is changed.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label(lang.tr(
                        "Duração padrão de novas notas:",
                        "Default new-note duration:",
                    ));
                    ui.add(
                        egui::Slider::new(
                            &mut self.config.workflow.default_note_duration_ms,
                            30.0..=4000.0,
                        )
                        .suffix(" ms")
                        .logarithmic(true),
                    );
                    help_marker(
                        ui,
                        lang.tr(
                            "Define o tamanho inicial de notas desenhadas com a ferramenta Lápis.",
                            "Sets the initial length of notes drawn with the Pencil tool.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label(lang.tr(
                        "Lyric padrão de novas notas:",
                        "Default lyric for new notes:",
                    ));
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.config.workflow.default_note_lyric)
                            .desired_width(120.0),
                    );
                    if response.changed() {
                        let lyric = self.config.workflow.default_note_lyric.trim();
                        if !lyric.is_empty() {
                            self.piano_roll_state.default_note_lyric = lyric.to_string();
                            self.persist_config();
                        }
                    }
                    help_marker(
                        ui,
                        lang.tr(
                            "Alias usado ao inserir uma nova nota com o Lápis.",
                            "Alias used when inserting a new note with the Pencil tool.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label(lang.tr("Expressão inicial:", "Initial expression:"));
                    let dynamics_changed = ui
                        .add(egui::Slider::new(
                            &mut self.config.workflow.default_note_dynamics,
                            -100.0..=100.0,
                        )
                        .text(lang.tr("dinâmica", "dynamics")))
                        .changed();
                    let volume_changed = ui
                        .add(egui::Slider::new(
                            &mut self.config.workflow.default_note_volume,
                            0.0..=200.0,
                        )
                        .text(lang.tr("volume", "volume")))
                        .changed();
                    if dynamics_changed || volume_changed {
                        self.config.workflow.default_note_dynamics =
                            self.config.workflow.default_note_dynamics.clamp(-100.0, 100.0);
                        self.config.workflow.default_note_volume =
                            self.config.workflow.default_note_volume.clamp(0.0, 200.0);
                        self.piano_roll_state.default_note_dynamics =
                            self.config.workflow.default_note_dynamics;
                        self.piano_roll_state.default_note_volume =
                            self.config.workflow.default_note_volume;
                        self.persist_config();
                    }
                    help_marker(
                        ui,
                        lang.tr(
                            "Valores aplicados automaticamente às novas notas criadas com o Lápis.",
                            "Values automatically applied to new notes created with the Pencil tool.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label(lang.tr("Articulação inicial:", "Initial articulation:"));
                    let attack_changed = ui
                        .add(
                            egui::Slider::new(
                                &mut self.config.workflow.default_note_attack,
                                0.0..=200.0,
                            )
                            .text(lang.tr("ataque", "attack")),
                        )
                        .changed();
                    let decay_changed = ui
                        .add(
                            egui::Slider::new(
                                &mut self.config.workflow.default_note_decay,
                                0.0..=100.0,
                            )
                            .text(lang.tr("decay", "decay")),
                        )
                        .changed();
                    if attack_changed || decay_changed {
                        self.config.workflow.default_note_attack =
                            self.config.workflow.default_note_attack.clamp(0.0, 200.0);
                        self.config.workflow.default_note_decay =
                            self.config.workflow.default_note_decay.clamp(0.0, 100.0);
                        self.piano_roll_state.default_note_attack =
                            self.config.workflow.default_note_attack;
                        self.piano_roll_state.default_note_decay =
                            self.config.workflow.default_note_decay;
                        self.persist_config();
                    }
                    help_marker(
                        ui,
                        lang.tr(
                            "Define o ataque e a queda de volume das novas notas criadas.",
                            "Defines attack and volume decay for newly created notes.",
                        ),
                    );
                });

                section_card(
                    ui,
                    lang.tr("Humanização padrão", "Default humanization"),
                    |ui| {
                        let mut changed = false;
                        ui.horizontal(|ui| {
                            ui.label(lang.tr("Variação de timing:", "Timing variation:"));
                            changed |= ui
                                .add(
                                    egui::Slider::new(
                                        &mut self.config.workflow.humanize_timing_jitter_ms,
                                        0.0..=100.0,
                                    )
                                    .suffix(" ms"),
                                )
                                .changed();
                        });
                        ui.horizontal(|ui| {
                            ui.label(lang.tr("Variação de pitch:", "Pitch variation:"));
                            changed |= ui
                                .add(
                                    egui::Slider::new(
                                        &mut self.config.workflow.humanize_pitch_cents_jitter,
                                        0.0..=100.0,
                                    )
                                    .suffix(" cents"),
                                )
                                .changed();
                        });
                        ui.horizontal(|ui| {
                            ui.label(lang.tr("Variação de volume:", "Volume variation:"));
                            changed |= ui
                                .add(
                                    egui::Slider::new(
                                        &mut self.config.workflow.humanize_volume_jitter_pct,
                                        0.0..=50.0,
                                    )
                                    .suffix(" %"),
                                )
                                .changed();
                        });
                        ui.horizontal(|ui| {
                            ui.label(lang.tr("Variação de breathiness:", "Breathiness variation:"));
                            changed |= ui
                                .add(
                                    egui::Slider::new(
                                        &mut self.config.workflow.humanize_breathiness_jitter_pct,
                                        0.0..=50.0,
                                    )
                                    .suffix(" %"),
                                )
                                .changed();
                        });
                        if changed {
                            self.config.workflow.humanize_timing_jitter_ms = self
                                .config
                                .workflow
                                .humanize_timing_jitter_ms
                                .clamp(0.0, 100.0);
                            self.config.workflow.humanize_pitch_cents_jitter = self
                                .config
                                .workflow
                                .humanize_pitch_cents_jitter
                                .clamp(0.0, 100.0);
                            self.config.workflow.humanize_volume_jitter_pct = self
                                .config
                                .workflow
                                .humanize_volume_jitter_pct
                                .clamp(0.0, 50.0);
                            self.config.workflow.humanize_breathiness_jitter_pct = self
                                .config
                                .workflow
                                .humanize_breathiness_jitter_pct
                                .clamp(0.0, 50.0);
                            self.humanize_dialog_state.humanize_params.timing_jitter_ms =
                                self.config.workflow.humanize_timing_jitter_ms;
                            self.humanize_dialog_state
                                .humanize_params
                                .pitch_cents_jitter =
                                self.config.workflow.humanize_pitch_cents_jitter;
                            self.humanize_dialog_state.humanize_params.volume_jitter_pct =
                                self.config.workflow.humanize_volume_jitter_pct;
                            self.humanize_dialog_state
                                .humanize_params
                                .breathiness_jitter_pct =
                                self.config.workflow.humanize_breathiness_jitter_pct;
                            self.persist_config();
                        }
                        help_marker(
                            ui,
                            lang.tr(
                                "Define os valores iniciais da janela Humanizar; timing fica em zero por padrão para preservar transições.",
                                "Sets the initial Humanize dialog values; timing defaults to zero to preserve transitions.",
                            ),
                        );
                    },
                );

                ui.horizontal(|ui| {
                    ui.label(lang.tr("Grid snap padrão:", "Default grid snap:"));
                    let snap_options = [
                        ("Auto", lang.tr("Automático", "Automatic")),
                        ("Freeform", lang.tr("Livre", "Freeform")),
                        ("1/4", "1/4"),
                        ("1/8", "1/8"),
                        ("1/16", "1/16"),
                        ("1/32", "1/32"),
                    ];
                    egui::ComboBox::from_id_salt("preferences_default_grid_snap")
                        .selected_text(&self.config.workflow.default_grid_snap)
                        .show_ui(ui, |ui| {
                            for (value, label) in snap_options {
                                ui.selectable_value(
                                    &mut self.config.workflow.default_grid_snap,
                                    value.to_string(),
                                    label,
                                );
                            }
                        });
                });

                ui.horizontal(|ui| {
                    ui.label(lang.tr("Auto-scroll padrão:", "Default auto-scroll:"));
                    let scroll_options = [
                        "Desligado",
                        "Seguir Cabeça (Cursor)",
                        "Seguir Cabeça (Página)",
                    ];
                    egui::ComboBox::from_id_salt("preferences_default_auto_scroll")
                        .selected_text(&self.config.workflow.default_auto_scroll)
                        .show_ui(ui, |ui| {
                            for value in scroll_options {
                                ui.selectable_value(
                                    &mut self.config.workflow.default_auto_scroll,
                                    value.to_string(),
                                    value,
                                );
                            }
                        });
                });

                ui.horizontal(|ui| {
                    ui.label(lang.tr(
                        "Ferramenta inicial do Piano Roll:",
                        "Initial Piano Roll tool:",
                    ));
                    let tools = [
                        ("Pointer", lang.tr("Pointer", "Pointer")),
                        ("Pencil", lang.tr("Lápis", "Pencil")),
                        ("PitchDraw", lang.tr("Pitch", "Pitch")),
                        ("Slice", lang.tr("Cortar", "Slice")),
                        ("Eraser", lang.tr("Borracha", "Eraser")),
                    ];
                    egui::ComboBox::from_id_salt("preferences_default_edit_tool")
                        .selected_text(&self.config.workflow.default_edit_tool)
                        .show_ui(ui, |ui| {
                            for (value, label) in tools {
                                ui.selectable_value(
                                    &mut self.config.workflow.default_edit_tool,
                                    value.to_string(),
                                    label,
                                );
                            }
                        });
                });

                ui.horizontal(|ui| {
                    ui.label(lang.tr("Subferramenta inicial de Pitch:", "Initial Pitch sub-tool:"));
                    let tools = [
                        ("Freehand", lang.tr("Livre", "Freehand")),
                        ("Smooth", lang.tr("Suave", "Smooth")),
                        ("Line", lang.tr("Reta", "Line")),
                        ("Vibrato", "Vibrato"),
                    ];
                    egui::ComboBox::from_id_salt("preferences_default_pitch_sub_tool")
                        .selected_text(&self.config.workflow.default_pitch_sub_tool)
                        .show_ui(ui, |ui| {
                            for (value, label) in tools {
                                ui.selectable_value(
                                    &mut self.config.workflow.default_pitch_sub_tool,
                                    value.to_string(),
                                    label,
                                );
                            }
                        });
                });
            },
        );
    }
}
