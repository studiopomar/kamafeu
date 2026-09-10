use super::help_marker;
use super::section_card;
use super::KamafeuStudioApp;
use crate::gui::theme::MelodyneTheme;
use eframe::egui;
use eframe::egui::Color32;
use eframe::egui::Frame;
use eframe::egui::Margin;
use eframe::egui::RichText;
use eframe::egui::Rounding;
use eframe::egui::Stroke;

impl KamafeuStudioApp {
    pub(in crate::gui) fn render_experimental_tab(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.language;
        Frame::none()
            .fill(Color32::from_rgb(58, 38, 16))
            .stroke(Stroke::new(1.2, Color32::from_rgb(255, 180, 70)))
            .rounding(Rounding::same(6.0))
            .inner_margin(Margin::same(10.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(lang.tr("Opções avançadas", "Advanced Options"))
                        .strong()
                        .color(Color32::from_rgb(255, 210, 120)),
                );
                ui.label(
                    RichText::new(lang.tr(
                        "Altere os controles desta página somente se você entende o parâmetro e o efeito dele no áudio, na memória ou na estabilidade. Valores inadequados podem causar falhas de reprodução e renderização.",
                        "Change settings on this page only if you understand the parameter and its effect on audio, memory, or stability. Improper values may cause playback and rendering issues.",
                    ))
                    .size(11.0)
                    .color(Color32::from_rgb(255, 225, 175)),
                );
            });
        ui.add_space(8.0);

        section_card(
            ui,
            lang.tr(
                "Suíte de Testes & Calibração Ativa de Hardware",
                "Hardware Testing Suite & Active Calibration",
            ),
            |ui| {
                ui.label(RichText::new(lang.tr(
                    "Execute diagnósticos em tempo real para verificar latência de áudio, poder de cálculo DSP, taxa de alocação de memória e integridade dos bancos de voz.",
                    "Run real-time diagnostics to check audio latency, DSP compute power, memory allocation rate, and voicebank integrity.",
                )).size(11.0).color(MelodyneTheme::TEXT_MUTED));
                ui.add_space(6.0);

                // 1. Live Signal Generator
                ui.group(|ui| {
                    ui.label(RichText::new(lang.tr("Gerador de Sinal de Teste & Calibração Acústica", "Test Signal Generator & Acoustic Calibration")).strong().size(12.0).color(Color32::from_rgb(0, 255, 180)));
                    ui.add_space(2.0);

                    ui.horizontal(|ui| {
                        ui.label(lang.tr("Tipo de Sinal:", "Signal Type:"));
                        let sig_types = [
                            (0, lang.tr("Senóide (Frequência Ajustável)", "Sine Wave (Adjustable Freq)")),
                            (1, lang.tr("Senóide 1000 Hz (0 dBFS Calib)", "Sine Wave 1000 Hz (0 dBFS Calib)")),
                            (2, lang.tr("Ruído Rosa (1/f)", "Pink Noise (1/f)")),
                            (3, lang.tr("Ruído Branco", "White Noise")),
                            (4, lang.tr("Sweep Linear (20Hz - 20kHz)", "Linear Sweep (20Hz - 20kHz)")),
                        ];
                        for (idx, name) in sig_types {
                            if ui.selectable_label(self.preferences_state.test_signal_type == idx, name).clicked() {
                                self.preferences_state.test_signal_type = idx;
                            }
                        }
                    });

                    if self.preferences_state.test_signal_type == 0 {
                        ui.horizontal(|ui| {
                            ui.label(lang.tr("Frequência:", "Frequency:"));
                            ui.add(egui::Slider::new(&mut self.preferences_state.test_signal_freq, 20.0..=20000.0).suffix(" Hz").logarithmic(true));
                        });
                    }

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let sig_type = self.preferences_state.test_signal_type;
                        let freq = self.preferences_state.test_signal_freq;
                        let play_btn = egui::Button::new(
                            RichText::new(lang.tr("Disparar Sinal de Calibração (0.5s)", "Trigger Calibration Signal (0.5s)")).strong().size(11.5).color(Color32::BLACK)
                        )
                        .fill(Color32::from_rgb(0, 255, 180))
                        .min_size(egui::vec2(220.0, 26.0));

                        if ui.add(play_btn).clicked() {
                            self.play_calibration_signal(sig_type, freq);
                        }
                        help_marker(
                            ui,
                            lang.tr(
                                "Gera e reproduz uma rajada de sinal de áudio puro calibrado para teste de fase, equalização de monitores e latência.",
                                "Generates and plays a calibrated burst of test audio signal for phase checking, monitor EQ, and latency.",
                            ),
                        );
                    });
                });

                ui.add_space(8.0);

                // 2. Benchmarks & Stress Tests
                ui.group(|ui| {
                    ui.label(
                        RichText::new(lang.tr(
                            "Benchmarks de Alto Desempenho & Testes de Estresse",
                            "High-Performance Benchmarks & Stress Tests",
                        ))
                        .strong()
                        .size(12.0)
                        .color(Color32::from_rgb(255, 215, 100)),
                    );
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        if ui
                            .button(
                                RichText::new(lang.tr(
                                    "Executar Benchmark do Motor DSP (50.000 amostras)",
                                    "Run DSP Engine Benchmark (50,000 samples)",
                                ))
                                .size(11.0),
                            )
                            .clicked()
                        {
                            self.run_dsp_engine_benchmark();
                        }

                        if ui
                            .button(
                                RichText::new(lang.tr(
                                    "Testar Alocação de Memória RAM (64 MB Heap)",
                                    "Test RAM Allocation (64 MB Heap)",
                                ))
                                .size(11.0),
                            )
                            .clicked()
                        {
                            self.run_ram_stress_test();
                        }

                        if ui
                            .button(
                                RichText::new(lang.tr(
                                    "Auditar Integridade de Todos os Cantores",
                                    "Audit All Singer Voicebank Integrity",
                                ))
                                .size(11.0),
                            )
                            .clicked()
                        {
                            self.run_voicebanks_audit();
                        }
                    });

                    if let Some(ref res) = self.preferences_state.benchmark_result {
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(res)
                                .size(10.5)
                                .color(Color32::from_rgb(0, 255, 180))
                                .monospace(),
                        );
                    }

                    if let Some(ref res) = self.preferences_state.memory_stress_result {
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(res)
                                .size(10.5)
                                .color(Color32::from_rgb(180, 230, 255))
                                .monospace(),
                        );
                    }

                    if let Some(ref res) = self.preferences_state.voicebank_audit_result {
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(res)
                                .size(10.5)
                                .color(Color32::from_rgb(255, 220, 140))
                                .monospace(),
                        );
                    }
                });
            },
        );

        section_card(
            ui,
            lang.tr(
                "Otimizações de Baixo Nível & Parâmetros do Kernel",
                "Low-Level Optimizations & Kernel Parameters",
            ),
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(lang.tr("Conjunto de Instruções Vetoriais SIMD:", "SIMD Vector Instruction Set:"));
                    let simd_modes = [
                        ("Auto (AVX2 / NEON / SSE)", lang.tr("Auto (AVX2 / NEON / SSE)", "Auto (AVX2 / NEON / SSE)")),
                        ("Forçar SSE 4.1", lang.tr("Forçar SSE 4.1", "Force SSE 4.1")),
                        ("Forçar Escalar (Sem SIMD)", lang.tr("Forçar Escalar (Sem SIMD)", "Force Scalar (No SIMD)")),
                    ];
                    for (mode, label) in simd_modes {
                        if ui.selectable_label(self.config.experimental.simd_mode == mode, label).clicked() {
                            self.config.experimental.simd_mode = mode.to_string();
                        }
                    }
                    help_marker(
                        ui,
                        lang.tr(
                            "Permite que os algoritmos de convolução, pitch bend e síntese utilizem aceleração de hardware nativa (AVX2 no x86_64, NEON no Apple Silicon / ARM).",
                            "Enables convolution, pitch bend, and synthesis algorithms to use native hardware acceleration (AVX2 on x86_64, NEON on Apple Silicon / ARM).",
                        ),
                    );
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.config.experimental.flush_denormals_to_zero,
                        lang.tr("Flush Subnormals/Denormals to Zero (FTZ & DAZ)", "Flush Subnormals/Denormals to Zero (FTZ & DAZ)"),
                    );
                    help_marker(
                        ui,
                        lang.tr(
                            "Força números de ponto flutuante infinitesimais para zero imediato na FPU, prevenindo quedas graves de performance da CPU em caudas de silêncio acústico.",
                            "Forces infinitesimal floating-point numbers to zero on FPU, preventing severe CPU performance drops in silent acoustic tails.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.config.experimental.realtime_thread_affinity,
                        lang.tr("Afinidade de CPU em Tempo Real (CPU Core Pinning)", "Real-Time CPU Affinity (CPU Core Pinning)"),
                    );
                    help_marker(
                        ui,
                        lang.tr(
                            "Fixa as threads do motor de áudio nos núcleos de maior desempenho da CPU para minimizar latência de troca de contexto do sistema operacional.",
                            "Pins audio engine threads to high-performance CPU cores to minimize OS context-switch latency.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.config.experimental.async_ipc_rendering,
                        lang.tr("Renderização Assíncrona via IPC / Processos Isolados", "Asynchronous IPC / Isolated Process Rendering"),
                    );
                    help_marker(
                        ui,
                        lang.tr(
                            "Executa resamplers pesados em processos de sistema separados para isolar qualquer crash de executáveis legados de terceiros.",
                            "Runs heavy resamplers in separate system processes to isolate crashes from legacy third-party executables.",
                        ),
                    );
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(lang.tr("Timeout de Processo Resampler IPC:", "Resampler IPC Process Timeout:"));
                    ui.add(egui::Slider::new(&mut self.config.experimental.resampler_ipc_timeout_ms, 500..=15000).suffix(" ms"));
                    help_marker(
                        ui,
                        lang.tr(
                            "Tempo limite antes de abortar a execução de um processo de resampler que parou de responder.",
                            "Timeout threshold before aborting a resampler process that became unresponsive.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label(lang.tr("Capacidade do Pipe Buffer de I/O:", "I/O Pipe Buffer Capacity:"));
                    ui.add(egui::Slider::new(&mut self.config.experimental.pipe_buffer_kb, 16..=512).suffix(" KB"));
                    help_marker(
                        ui,
                        lang.tr(
                            "Tamanho do buffer de comunicação entre o Kamafeu Studio e os motores wavtool/resampler.",
                            "Communication buffer size between Kamafeu Studio and wavtool/resampler engines.",
                        ),
                    );
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.config.experimental.dump_chunks_debug,
                        lang.tr("Salvar Dumps de Chunks PCM Brutos para Depuração (/tmp/kamafeu_chunks)", "Save Raw PCM Chunk Dumps for Debugging (/tmp/kamafeu_chunks)"),
                    );
                    help_marker(
                        ui,
                        lang.tr(
                            "Grava fatias de áudio em formato PCM Float32 no disco para análise espectral de fase e crossfade.",
                            "Writes Float32 PCM audio slices to disk for phase and crossfade spectral analysis.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.config.experimental.trace_dsp_timing,
                        lang.tr("Profiling Detalhado de Tempo de Execução por Fonema", "Detailed Phoneme Execution Timing Profiling"),
                    );
                    help_marker(
                        ui,
                        lang.tr(
                            "Emite relatórios com precisão de microssegundos sobre o tempo gasto em cada fonema sintetizado no Console.",
                            "Outputs microsecond-accurate reports on synthesis time per phoneme in console.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.config.experimental.force_opengl_fallback,
                        lang.tr("Forçar Fallback de Renderização de GPU (OpenGL Compat)", "Force GPU Rendering Fallback (OpenGL Compat)"),
                    );
                    help_marker(
                        ui,
                        lang.tr(
                            "Utiliza renderização gráfica conservadora caso seu driver de vídeo apresente artefatos no egui.",
                            "Uses conservative graphics rendering if display drivers show artifacts in egui.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.config.experimental.vst3_plugin_support,
                        lang.tr("Habilitar Host Experimental de Plugins de Efeitos VST3", "Enable Experimental VST3 Effects Plugin Host"),
                    );
                    help_marker(
                        ui,
                        lang.tr(
                            "Ativa suporte a inserção de plugins VST3 de reverberação e equalização no canal master.",
                            "Enables inserting VST3 reverb and EQ plugins on the master channel.",
                        ),
                    );
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    let wine_found = crate::drivers::process::find_wine_executable();
                    if let Some(ref w) = wine_found {
                        let ver = crate::drivers::process::wine_version().unwrap_or_else(|| "Wine".to_string());
                        ui.label(RichText::new(format!("✔ Wine: {} ({})", w.display(), ver)).size(11.0).color(Color32::from_rgb(0, 255, 157)));
                    } else {
                        ui.label(RichText::new(lang.tr("⚠ Wine não detectado (necessário para executar resamplers .exe no macOS/Linux)", "⚠ Wine not detected (needed for Windows .exe resamplers on macOS/Linux)")).size(11.0).color(Color32::from_rgb(255, 180, 70)));
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(lang.tr("Executável Personalizado do Wine:", "Custom Wine Executable:"));
                    if ui.button(lang.tr("🔍 Detectar Wine", "🔍 Detect Wine")).clicked() {
                        crate::drivers::process::rescan_wine_executable();
                    }
                    if ui.button(lang.tr("Procurar Wine...", "Browse Wine...")).clicked() {
                        if let Some(file) = crate::dialogs::FileDialog::new()
                            .set_title("Selecionar binário do Wine")
                            .pick_file()
                        {
                            self.config.dsp.custom_wine_path = Some(file.clone());
                            crate::drivers::process::set_custom_wine_path(Some(file));
                            crate::drivers::process::rescan_wine_executable();
                            self.persist_config();
                        }
                    }
                    if self.config.dsp.custom_wine_path.is_some() {
                        if ui.button(lang.tr("Limpar", "Clear")).clicked() {
                            self.config.dsp.custom_wine_path = None;
                            crate::drivers::process::set_custom_wine_path(None);
                            crate::drivers::process::rescan_wine_executable();
                            self.persist_config();
                        }
                    }
                });

                ui.horizontal(|ui| {
                    ui.label(lang.tr("Canais de Depuração do Wine/Proton:", "Wine/Proton Debug Channels:"));
                    ui.add(egui::TextEdit::singleline(&mut self.config.experimental.wine_debug_channel).desired_width(180.0));
                    help_marker(
                        ui,
                        lang.tr(
                            "Variável WINEDEBUG repassada aos resamplers de Windows (.exe) executados em macOS/Linux.",
                            "WINEDEBUG environment variable passed to Windows resamplers (.exe) on macOS/Linux.",
                        ),
                    );
                });
            },
        );

        section_card(
            ui,
            lang.tr(
                "Telemetria de Hardware & Sistema em Tempo Real",
                "Real-Time Hardware & System Telemetry",
            ),
            |ui| {
                let cpu_threads = std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(4);
                let os_name = std::env::consts::OS;
                let arch_name = std::env::consts::ARCH;

                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "{}: {os_name} ({arch_name})",
                            lang.tr("Sistema", "System")
                        ))
                        .strong()
                        .color(Color32::from_rgb(180, 230, 255)),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!(
                            "{}: {cpu_threads}",
                            lang.tr("Threads Lógicas", "Logical Threads")
                        ))
                        .strong()
                        .color(Color32::from_rgb(0, 255, 157)),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!(
                            "{}: {:.0}",
                            lang.tr("FPS da Interface", "UI FPS"),
                            if self.frame_time_ema_ms > 0.0 {
                                1000.0 / self.frame_time_ema_ms
                            } else {
                                60.0
                            }
                        ))
                        .strong()
                        .color(Color32::from_rgb(255, 215, 100)),
                    );
                    ui.separator();
                    ui.label(
                        RichText::new(format!(
                            "{}: {:.1} ms",
                            lang.tr("Latência de Quadro", "Frame Latency"),
                            self.frame_time_ema_ms
                        ))
                        .monospace()
                        .color(MelodyneTheme::TEXT_MUTED),
                    );
                });
            },
        );
    }
}
