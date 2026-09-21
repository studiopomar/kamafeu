use super::help_marker;
use super::section_card;
use super::KamafeuStudioApp;
use eframe::egui;
use eframe::egui::Color32;
use eframe::egui::RichText;

impl KamafeuStudioApp {
    pub(in crate::gui) fn render_voicebank_tuning_tab(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.language;
        section_card(
            ui,
            lang.tr(
                "Diretórios Globais de Cantores & Voicebanks",
                "Global Singer & Voicebank Directories",
            ),
            |ui| {
                let mut remove_idx = None;
                for (i, folder) in self.config.singers_paths.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new("dir:")
                                .size(10.5)
                                .color(Color32::from_rgb(180, 200, 240)),
                        );
                        ui.label(
                            RichText::new(folder.to_string_lossy().to_string())
                                .size(10.5)
                                .monospace(),
                        );

                        if ui
                            .button(RichText::new(lang.tr("Abrir", "Open")).size(10.0))
                            .on_hover_text(crate::gui::dialogs::reveal_in_file_manager_label_for(
                                lang,
                            ))
                            .clicked()
                        {
                            crate::gui::dialogs::open_file_in_folder(folder);
                        }

                        if ui
                            .button(
                                RichText::new(lang.tr("Remover", "Remove"))
                                    .size(10.0)
                                    .color(Color32::from_rgb(255, 120, 120)),
                            )
                            .on_hover_text(
                                lang.tr("Remover da lista de busca", "Remove from search paths"),
                            )
                            .clicked()
                        {
                            remove_idx = Some(i);
                        }
                    });
                }

                if let Some(i) = remove_idx {
                    self.config.singers_paths.remove(i);
                    self.persist_config();
                }

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                if ui.button(RichText::new(lang.tr("+ Adicionar Nova Pasta de Voicebanks...", "+ Add New Voicebank Folder...")).size(11.0)).clicked() {
                    if let Some(folder) = crate::dialogs::FileDialog::new()
                        .set_title(lang.tr("Selecionar Diretório Raiz de Voicebanks", "Select Voicebank Root Directory"))
                        .pick_folder()
                    {
                        if !self.config.singers_paths.contains(&folder) {
                            self.config.singers_paths.push(folder);
                            self.persist_config();
                        }
                    }
                }
                help_marker(
                    ui,
                    lang.tr(
                        "O Kamafeu Studio varre automaticamente todas essas pastas procurando bancos de voz UTAU, CV, VCV, CVVC, VCCV e multi-pitch.",
                        "Kamafeu Studio automatically scans all these folders for UTAU, CV, VCV, CVVC, VCCV, and multi-pitch voicebanks.",
                    ),
                );
            });

                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.config.voicebank_tuning.recursive_voicebank_scan,
                        lang.tr(
                            "Escanear subpastas recursivamente dentro dos diretórios de cantores",
                            "Recursively scan subfolders inside singer directories",
                        ),
                    );
                    help_marker(
                        ui,
                        lang.tr(
                            "Permite organizar bancos de voz em subpastas por autor ou idioma.",
                            "Allows organizing voicebanks in subfolders by author or language.",
                        ),
                    );
                });
            },
        );

        section_card(
            ui,
            lang.tr(
                "Resolução de Aliases, Fonemizadores & oto.ini",
                "Alias Resolution, Phonemizers & oto.ini",
            ),
            |ui| {
                ui.horizontal(|ui| {
                ui.label(lang.tr("Estratégia de Busca de Aliases:", "Alias Search Strategy:"));
                let strat_list = [
                    ("Prioritário (Prefixo/Sufixo -> Exato -> Romaji)", lang.tr("Prioritário (Prefixo/Sufixo -> Exato -> Romaji)", "Priority (Prefix/Suffix -> Exact -> Romaji)")),
                    ("Estrito (Apenas Exato)", lang.tr("Estrito (Apenas Exato)", "Strict (Exact Only)")),
                    ("Tolerante com Fallback Fonético", lang.tr("Tolerante com Fallback Fonético", "Tolerant with Phonetic Fallback")),
                ];
                for (val, label) in strat_list {
                    if ui.selectable_label(self.config.voicebank_tuning.alias_search_strategy == val, label).clicked() {
                        self.config.voicebank_tuning.alias_search_strategy = val.to_string();
                    }
                }
                help_marker(
                    ui,
                    lang.tr(
                        "Define a ordem de busca quando uma nota é digitada. 'Prioritário' tenta prefixos de tom (ex: C4, D4, falsete) antes de usar o alias base.",
                        "Sets search order when note is entered. 'Priority' attempts pitch prefixes (e.g. C4, D4, falsetto) before falling back to base alias.",
                    ),
                );
            });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                ui.label(lang.tr("Algoritmo de Estiramento de Consoante:", "Consonant Stretch Algorithm:"));
                for alg in ["Adaptive SOLA", "Hybrid PSOLA", "WSOLA"] {
                    if ui.selectable_label(self.config.voicebank_tuning.consonant_stretch_mode == alg, alg).clicked() {
                        self.config.voicebank_tuning.consonant_stretch_mode = alg.to_string();
                    }
                }
                help_marker(
                    ui,
                    lang.tr(
                        "Algoritmo DSP usado para esticar ou encurtar a consoante sem alterar o tom da voz. 'Adaptive SOLA' é o padrão de alta fidelidade.",
                        "DSP algorithm used to stretch or shorten consonants without altering pitch. 'Adaptive SOLA' is high-fidelity standard.",
                    ),
                );
            });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.config.voicebank_tuning.auto_reload_oto,
                    lang.tr("Recarregar oto.ini automaticamente quando modificado no disco", "Automatically reload oto.ini when modified on disk"),
                );
                help_marker(
                    ui,
                    lang.tr(
                        "Atualiza a afinação e transições instantaneamente se você editar o oto.ini no Copaiba ou editor externo.",
                        "Updates tuning and transitions instantly if you edit oto.ini in Copaiba or an external editor.",
                    ),
                );
            });

                ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.config.voicebank_tuning.strict_oto_parsing,
                    lang.tr("Modo Estrito de Validação do oto.ini (Ignorar valores negativos ilegais)", "Strict oto.ini Validation Mode (Ignore illegal negative values)"),
                );
                help_marker(
                    ui,
                    lang.tr(
                        "Corrige automaticamente erros comuns de configuração em voicebanks antigos sem quebrar a síntese.",
                        "Automatically handles common configuration errors in legacy voicebanks without crashing synthesis.",
                    ),
                );
            });

                ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.config.voicebank_tuning.auto_phonetic_g2p,
                    lang.tr("Ativar conversão Grapheme-to-Phoneme (G2P) automática", "Enable automatic Grapheme-to-Phoneme (G2P) conversion"),
                );
                help_marker(
                    ui,
                    lang.tr(
                        "Converte palavras escritas em português, japonês ou inglês para os fonemas corretos da reclist do voicebank.",
                        "Converts written words in Portuguese, Japanese, or English into correct reclist phonemes for the voicebank.",
                    ),
                );
            });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                ui.label(lang.tr("Escala Global de Pré-enunciado (Preutterance Scale):", "Global Preutterance Scale:"));
                ui.add(egui::Slider::new(&mut self.config.voicebank_tuning.preutterance_scale, 0.2..=3.0).suffix("x"));
                help_marker(
                    ui,
                    lang.tr(
                        "Multiplica o tempo que a consoante começa a ser pronunciada antes da linha da batida.",
                        "Multiplies the time the consonant begins pronunciation before the beat line.",
                    ),
                );
            });

                ui.horizontal(|ui| {
                ui.label(lang.tr("Escala Global de Sobreposição (Overlap Scale):", "Global Overlap Scale:"));
                ui.add(egui::Slider::new(&mut self.config.voicebank_tuning.overlap_scale, 0.2..=3.0).suffix("x"));
                help_marker(
                    ui,
                    lang.tr(
                        "Multiplica o tempo de crossfade entre o fim da nota anterior e o início da nota atual.",
                        "Multiplies the crossfade time between the end of the previous note and start of the current note.",
                    ),
                );
            });

                ui.horizontal(|ui| {
                    ui.label(lang.tr(
                        "Multiplicador Global de Consoante Fixa (Fixed Consonant):",
                        "Global Fixed Consonant Multiplier:",
                    ));
                    ui.add(
                        egui::Slider::new(
                            &mut self.config.voicebank_tuning.fixed_consonant_scale,
                            0.2..=3.0,
                        )
                        .suffix("x"),
                    );
                    help_marker(
                    ui,
                    lang.tr(
                        "Controla a porção não estirável da consoante (ataque oclusivo / plosivo).",
                        "Controls the non-stretchable portion of the consonant (plosive / attack).",
                    ),
                );
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                ui.label(lang.tr("Inclinação de Transição Vocálica (Vowel Crossfade Slope):", "Vowel Crossfade Slope:"));
                ui.add(
                    egui::Slider::new(
                        &mut self.config.voicebank_tuning.vowel_crossfade_slope,
                        0.2..=3.0,
                    )
                    .suffix("x"),
                );
                help_marker(
                    ui,
                    lang.tr(
                        "Ajusta a suavidade da rampa de transição em frases com vogais continuadas.",
                        "Adjusts the smoothness of transition ramp in phrases with sustained vowels.",
                    ),
                );
            });

                ui.horizontal(|ui| {
                ui.label(lang.tr("Redução de Ruído de Respiração (Breath Reduction):", "Breath Noise Reduction:"));
                ui.add(egui::Slider::new(&mut self.config.voicebank_tuning.breath_noise_reduction_db, -24.0..=0.0).suffix(" dB"));
                help_marker(
                    ui,
                    lang.tr(
                        "Atenua ruídos de respiração aspirados ou ruídos de fundo em amostras sem interferir nos formantes principais.",
                        "Attenuates aspirated breath noise or background noise in samples without interfering with key formants.",
                    ),
                );
            });
            },
        );
    }
}
