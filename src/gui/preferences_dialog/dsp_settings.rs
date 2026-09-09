use super::help_marker;
use super::section_card;
use super::KamafeuStudioApp;
use eframe::egui;
use eframe::egui::Color32;
use eframe::egui::RichText;

impl KamafeuStudioApp {
    pub(in crate::gui) fn render_dsp_settings_tab(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.language;
        section_card(
            ui,
            lang.tr(
                "Desempenho & Paralelismo de Renderização",
                "Rendering Performance & Parallelism",
            ),
            |ui| {
                let cpu_threads = std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(4);

                ui.horizontal(|ui| {
                ui.label(lang.tr("Threads de Renderização:", "Render Threads:"));
                let threads_val = self.config.dsp.render_threads;
                if threads_val == 0 {
                    ui.label(RichText::new(format!("{cpu_threads} {}", lang.tr("Automático (threads detectadas)", "Auto (threads detected)"))).strong().color(Color32::from_rgb(0, 255, 157)));
                } else {
                    ui.label(format!("{threads_val} {}", lang.tr("threads dedicadas", "dedicated threads")));
                }
                help_marker(
                    ui,
                    lang.tr(
                        "Quantidade de tarefas paralelas simultâneas de processamento DSP e resampler. '0' detecta automaticamente os núcleos da sua CPU.",
                        "Number of concurrent parallel DSP and resampler tasks. '0' automatically detects your CPU cores.",
                    ),
                );
            });

                ui.horizontal(|ui| {
                    ui.add(
                        egui::Slider::new(&mut self.config.dsp.render_threads, 0..=32)
                            .text(lang.tr("threads (0 = auto)", "threads (0 = auto)")),
                    );
                    if ui
                        .button(
                            RichText::new(format!(
                                "{} ({cpu_threads} {})",
                                lang.tr("Auto-detectar", "Auto-detect"),
                                lang.tr("núcleos", "cores")
                            ))
                            .size(10.5),
                        )
                        .clicked()
                    {
                        self.config.dsp.render_threads = 0;
                        self.render_threads = cpu_threads as u32;
                    }
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                ui.label(lang.tr("Prioridade das Threads:", "Thread Priority:"));
                let prio_list = [
                    ("Normal", lang.tr("Normal", "Normal")),
                    ("Alta (High Priority)", lang.tr("Alta (High Priority)", "High Priority")),
                    ("Tempo Real (Realtime Audio)", lang.tr("Tempo Real (Realtime Audio)", "Realtime Audio")),
                ];
                for (val, label) in prio_list {
                    if ui.selectable_label(self.config.dsp.thread_priority == val, label).clicked() {
                        self.config.dsp.thread_priority = val.to_string();
                    }
                }
                help_marker(
                    ui,
                    lang.tr(
                        "Prioridade atribuída às threads de síntese pelo agendador do sistema operacional.",
                        "Priority assigned to synthesis threads by the operating system scheduler.",
                    ),
                );
            });
            },
        );

        section_card(
            ui,
            lang.tr(
                "Algoritmos DSP, Interpolação & Curvas de Pitch",
                "DSP Algorithms, Interpolation & Pitch Curves",
            ),
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(lang.tr("Resolução Temporal da Curva de Pitch:", "Pitch Curve Time Resolution:"));
                    ui.add(egui::Slider::new(&mut self.config.dsp.pitch_curve_step_ms, 1.0..=10.0).suffix(lang.tr(" ms / ponto", " ms / point")));
                    help_marker(
                        ui,
                        lang.tr(
                            "Intervalo em milissegundos entre pontos de cálculo da curva de afinação (portamento, vibrato, modulação). Valores menores (1-2ms) produzem afinações hiper-suaves.",
                            "Interval in milliseconds between pitch curve calculation points (portamento, vibrato, modulation). Smaller values (1-2ms) yield ultra-smooth pitch.",
                        ),
                    );
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(lang.tr("Método de Interpolação de Pitch:", "Pitch Interpolation Method:"));
                    for mode in ["Cosine", "Cubic Spline", "Hermite", "Linear"] {
                        if ui.selectable_label(self.config.dsp.pitch_interpolation == mode, mode).clicked() {
                            self.config.dsp.pitch_interpolation = mode.to_string();
                        }
                    }
                    help_marker(
                        ui,
                        lang.tr(
                            "Fórmula matemática usada para conectar os pontos de pitch bend. 'Cosine' e 'Cubic Spline' criam transições orgânicas naturais.",
                            "Mathematical formula used to connect pitch bend points. 'Cosine' and 'Cubic Spline' produce organic natural transitions.",
                        ),
                    );
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(lang.tr("Detecção de Pitch Fundamental (F0 Tracker):", "Fundamental Pitch Detection (F0 Tracker):"));
                    let f0_list = [
                        ("YIN (Pitch Fundamental Adaptativo)", lang.tr("YIN (Pitch Fundamental Adaptativo)", "YIN (Adaptive Fundamental Pitch)")),
                        ("pyIN (Probabilístico)", lang.tr("pyIN (Probabilístico)", "pyIN (Probabilistic)")),
                        ("Harvest/DIO (Espectral)", lang.tr("Harvest/DIO (Espectral)", "Harvest/DIO (Spectral)")),
                    ];
                    for (val, label) in f0_list {
                        if ui.selectable_label(self.config.dsp.f0_detection_method == val, label).clicked() {
                            self.config.dsp.f0_detection_method = val.to_string();
                        }
                    }
                    help_marker(
                        ui,
                        lang.tr(
                            "Algoritmo de estimação de frequência fundamental F0 da voz humana para tracking e resynthesis.",
                            "Fundamental frequency F0 estimation algorithm of human voice for tracking and resynthesis.",
                        ),
                    );
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(lang.tr("Faixa de Busca de Frequência F0:", "F0 Frequency Search Range:"));
                    ui.add(egui::Slider::new(&mut self.config.dsp.f0_min_hz, 20.0..=200.0).prefix(lang.tr("Min: ", "Min: ")).suffix(" Hz"));
                    ui.add(egui::Slider::new(&mut self.config.dsp.f0_max_hz, 300.0..=2000.0).prefix(lang.tr("Max: ", "Max: ")).suffix(" Hz"));
                    help_marker(
                        ui,
                        lang.tr(
                            "Limites inferior e superior para a detecção de pitch da voz, evitando oitavações falsas em vozes muito graves ou agudas.",
                            "Lower and upper boundaries for vocal pitch detection, preventing octave jumps on deep or high voices.",
                        ),
                    );
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(lang.tr("Janela de Análise FFT / Espectral:", "FFT / Spectral Analysis Window:"));
                    for win in ["Blackman-Harris (4-term)", "Hann", "Hamming", "Kaiser-Bessel"] {
                        if ui.selectable_label(self.config.dsp.fft_window_type == win, win).clicked() {
                            self.config.dsp.fft_window_type = win.to_string();
                        }
                    }
                    help_marker(
                        ui,
                        lang.tr(
                            "Função de janelamento aplicada em transformadas de Fourier e análise TD-PSOLA.",
                            "Windowing function applied in Fourier transforms and TD-PSOLA analysis.",
                        ),
                    );
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(lang.tr("Fator de Oversampling Interno:", "Internal Oversampling Factor:"));
                    let os_list = [
                        (1, lang.tr("1x (Nativo 44.1/48k)", "1x (Native 44.1/48k)")),
                        (2, lang.tr("2x (Alta Definição 88.2/96k)", "2x (High Definition 88.2/96k)")),
                        (4, lang.tr("4x (Ultra HD 176.4/192k)", "4x (Ultra HD 176.4/192k)")),
                    ];
                    for (val, label) in os_list {
                        if ui.selectable_label(self.config.dsp.oversampling_factor == val, label).clicked() {
                            self.config.dsp.oversampling_factor = val;
                        }
                    }
                    help_marker(
                        ui,
                        lang.tr(
                            "Multiplica internamente a taxa de processamento para eliminar completamente distorções de aliasing e intermodulação não linear.",
                            "Internally multiplies the processing sample rate to eliminate aliasing and non-linear intermodulation distortion.",
                        ),
                    );
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(lang.tr("Preservação de Formantes:", "Formant Preservation:"));
                    let formant_list = [
                        ("LPC Spectral Envelope", "LPC Spectral Envelope"),
                        ("True Envelope", "True Envelope"),
                        ("Desativado", lang.tr("Desativado", "Disabled")),
                    ];
                    for (val, label) in formant_list {
                        if ui.selectable_label(self.config.dsp.formant_preservation_mode == val, label).clicked() {
                            self.config.dsp.formant_preservation_mode = val.to_string();
                        }
                    }
                    help_marker(
                        ui,
                        lang.tr(
                            "Mantém a característica tímbrica única do cantor (formantes vocais) estável mesmo durante transposições de oitavas extremas.",
                            "Maintains the unique timbre characteristics of the singer (vocal formants) even during extreme octave shifts.",
                        ),
                    );
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.config.dsp.anti_aliasing_filter,
                        lang.tr("Filtro Anti-Aliasing DSP (Proteção contra dobras de Nyquist em agudos)", "DSP Anti-Aliasing Filter (Protection against Nyquist folding on high pitches)"),
                    );
                    help_marker(
                        ui,
                        lang.tr(
                            "Aplica corte passa-baixa dinâmico proporcional à transposição para evitar artefatos metálicos desagradáveis quando notas agudas são cantadas.",
                            "Applies dynamic low-pass filtering proportional to transposition to prevent harsh metallic artifacts on high notes.",
                        ),
                    );
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(lang.tr("Janela de Crossfade de Fonemas:", "Phoneme Crossfade Window:"));
                    ui.add(egui::Slider::new(&mut self.config.dsp.crossfade_window_ms, 5.0..=50.0).suffix(" ms"));
                    help_marker(
                        ui,
                        lang.tr(
                            "Duração da sobreposição suave entre consoante e vogal / fonemas adjacentes para evitar estalos na junção.",
                            "Duration of smooth overlap between consonant and vowel / adjacent phonemes to prevent junction clicks.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label(lang.tr("Curva de Crossfade de Áudio:", "Audio Crossfade Curve:"));
                    let curve_list = [
                        ("Equal Power S-Curve", "Equal Power S-Curve"),
                        ("Linear", lang.tr("Linear", "Linear")),
                        ("Logarítmica", lang.tr("Logarítmica", "Logarithmic")),
                    ];
                    for (curve, label) in curve_list {
                        if ui.selectable_label(self.config.dsp.crossfade_curve == curve, label).clicked() {
                            self.config.dsp.crossfade_curve = curve.to_string();
                        }
                    }
                    help_marker(
                        ui,
                        lang.tr(
                            "'Equal Power S-Curve' mantém o volume acústico constante sem perda de energia durante a transição de fonemas.",
                            "'Equal Power S-Curve' preserves constant acoustic energy without dips during phoneme transitions.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label(lang.tr("Lookahead de Pré-renderização:", "Prerender Lookahead:"));
                    ui.add(egui::Slider::new(&mut self.config.dsp.render_lookahead_ms, 500.0..=10000.0).suffix(" ms"));
                    help_marker(
                        ui,
                        lang.tr(
                            "Distância em milissegundos à frente do cursor de reprodução que o motor sintetiza antecipadamente para tocar sem pausas.",
                            "Distance in milliseconds ahead of the playhead that the engine prerenders for seamless playback.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label(lang.tr("Tamanho do Bloco de Renderização:", "Render Chunk Size:"));
                    ui.add(egui::Slider::new(&mut self.config.dsp.render_chunk_bars, 1..=16).suffix(lang.tr(" compassos", " bars")));
                    help_marker(
                        ui,
                        lang.tr(
                            "Quantidade de compassos processados em cada lote de renderização de fundo.",
                            "Number of bars processed in each background render batch.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.config.dsp.background_prerender,
                        lang.tr("Renderizar notas em segundo plano automaticamente durante a edição", "Automatically prerender notes in background during editing"),
                    );
                    help_marker(
                        ui,
                        lang.tr(
                            "Sintetiza as notas assim que são desenhadas ou alteradas no piano roll para que a reprodução seja instantânea.",
                            "Synthesizes notes as soon as they are drawn or modified in the piano roll for instant playback.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.config.dsp.verbose_dsp_logging,
                        lang.tr("Logs detalhados de depuração DSP no console", "Detailed DSP debug logs in console"),
                    );
                    help_marker(
                        ui,
                        lang.tr(
                            "Exibe cada chamada matemática, frequência fundamental F0 calculada e tempos de resampler no terminal.",
                            "Outputs mathematical calls, calculated fundamental frequency F0, and resampler timing to terminal.",
                        ),
                    );
                });
            },
        );

        section_card(
            ui,
            lang.tr(
                "Motores de Síntese Padrão (Resampler & Wavtool)",
                "Default Synthesis Engines (Resampler & Wavtool)",
            ),
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(lang.tr("Resampler Padrão:", "Default Resampler:"));
                    ui.label(RichText::new(&self.selected_resampler).strong().color(Color32::from_rgb(216, 180, 254)));
                    help_marker(
                        ui,
                        lang.tr(
                            "Motor de alteração de altura tonal e estiramento temporal. O straycat-rs e o Native Hybrid TD-PSOLA oferecem alta fidelidade vocal.",
                            "Pitch-shifting and time-stretching engine. straycat-rs and Native Hybrid TD-PSOLA provide high vocal fidelity.",
                        ),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label(lang.tr("Wavtool Padrão:", "Default Wavtool:"));
                    ui.label(
                        RichText::new(&self.selected_wavtool)
                            .strong()
                            .color(Color32::from_rgb(216, 180, 254)),
                    );
                    help_marker(
                        ui,
                        lang.tr(
                            "Ferramenta de concatenação e fusão das fatias de áudio renderizadas.",
                            "Tool for concatenating and splicing rendered audio slices.",
                        ),
                    );
                });
            },
        );
    }
}
