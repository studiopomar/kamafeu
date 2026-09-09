use super::help_marker;
use super::section_card;
use super::KamafeuStudioApp;
use crate::audio::player::AudioPlayer;
use crate::gui::theme::MelodyneTheme;
use eframe::egui;
use eframe::egui::Color32;
use eframe::egui::RichText;
use eframe::egui::Rounding;
use eframe::egui::Stroke;

impl KamafeuStudioApp {
    pub(in crate::gui) fn render_audio_settings_tab(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.language;
        section_card(
            ui,
            lang.tr(
                "Placa de Som & Dispositivos de Saída",
                "Sound Card & Output Devices",
            ),
            |ui| {
                let host_name = AudioPlayer::default_host_name();
                ui.horizontal(|ui| {
                ui.label(lang.tr("Host de Áudio Ativo:", "Active Audio Host:"));
                ui.label(RichText::new(&host_name).strong().color(Color32::from_rgb(180, 230, 255)));
                help_marker(
                    ui,
                    lang.tr(
                        "API do sistema operacional utilizada para comunicação de áudio (CoreAudio no macOS, WASAPI no Windows, ALSA/PulseAudio/JACK/PipeWire no Linux).",
                        "Operating system API used for audio communication (CoreAudio on macOS, WASAPI on Windows, ALSA/PulseAudio/JACK/PipeWire on Linux).",
                    ),
                );
            });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                ui.label(lang.tr("Dispositivo de Saída:", "Output Device:"));
                let default_device_str = lang.tr("Padrão do Sistema Operacional", "Operating System Default");
                let current_device = self
                    .config
                    .audio
                    .device_name
                    .as_deref()
                    .unwrap_or(default_device_str);

                egui::ComboBox::from_id_salt("pref_audio_device_combo")
                    .selected_text(current_device)
                    .width(340.0)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_label(
                                self.config.audio.device_name.is_none(),
                                default_device_str,
                            )
                            .clicked()
                        {
                            self.config.audio.device_name = None;
                        }
                        for dev in AudioPlayer::list_output_devices() {
                            let is_sel = self.config.audio.device_name.as_deref() == Some(&dev);
                            if ui.selectable_label(is_sel, &dev).clicked() {
                                self.config.audio.device_name = Some(dev);
                            }
                        }
                    });

                if ui
                    .button(RichText::new(lang.tr("Redetectar", "Rescan")).size(10.5))
                    .on_hover_text(
                        lang.tr(
                            "Escanear novos dispositivos de áudio, caixas de som e fones conectados",
                            "Scan for newly connected audio devices, speakers, and headphones",
                        ),
                    )
                    .clicked()
                {
                    self.preferences_state.refresh_devices();
                }

                help_marker(
                    ui,
                    lang.tr(
                        "Selecione a interface de áudio ou fone de ouvido para reprodução do preview.",
                        "Select the audio interface or headphones for preview playback.",
                    ),
                );
            });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                ui.label(lang.tr("Roteamento de Canais:", "Channel Routing:"));
                let channel_modes = [
                    ("Estéreo (L+R)", lang.tr("Estéreo (L+R)", "Stereo (L+R)")),
                    ("Apenas Esquerda (Mono L)", lang.tr("Apenas Esquerda (Mono L)", "Left Only (Mono L)")),
                    ("Apenas Direita (Mono R)", lang.tr("Apenas Direita (Mono R)", "Right Only (Mono R)")),
                    ("Mixdown Mono", lang.tr("Mixdown Mono", "Mono Mixdown")),
                ];
                for (val, label) in channel_modes {
                    if ui.selectable_label(self.config.audio.channel_mode == val, label).clicked() {
                        self.config.audio.channel_mode = val.to_string();
                    }
                }
                help_marker(
                    ui,
                    lang.tr(
                        "Configura como o áudio sintetizado e as faixas instrumentais são mapeadas nos canais de saída estéreo.",
                        "Configures how synthesized audio and instrumental tracks are routed to stereo output channels.",
                    ),
                );
            });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.config.audio.exclusive_mode,
                    lang.tr("Modo Exclusivo de Baixa Latência (Exclusive Device Access)", "Low-Latency Exclusive Mode (Exclusive Device Access)"),
                );
                help_marker(
                    ui,
                    lang.tr(
                        "Solicita acesso exclusivo e prioritário ao hardware de áudio, ignorando o mixer do sistema para menor jitter e latência mínima.",
                        "Requests exclusive and prioritized access to audio hardware, bypassing system mixer for lower jitter and minimal latency.",
                    ),
                );
            });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.config.audio.auto_mute_on_device_change,
                    lang.tr("Silenciar temporariamente ao desconectar/trocar dispositivo de áudio", "Temporarily mute when disconnecting/switching audio devices"),
                );
                help_marker(
                    ui,
                    lang.tr(
                        "Evita estalos ou ruídos bruscos durante a desconexão ou troca de fones/placas de som.",
                        "Prevents pops or abrupt noises when disconnecting or switching headphones/sound cards.",
                    ),
                );
            });
            },
        );

        section_card(
            ui,
            lang.tr(
                "Taxa de Amostragem, Buffer & Latência do DAC",
                "Sample Rate, Buffer & DAC Latency",
            ),
            |ui| {
                ui.horizontal(|ui| {
                ui.label(lang.tr("Taxa de Amostragem (DAC Playback):", "Sample Rate (DAC Playback):"));
                for rate in [44100, 48000, 88200, 96000, 192000] {
                    let is_sel = self.config.audio.sample_rate == rate;
                    let (bg, text, stroke) = if is_sel {
                        (Color32::from_rgb(60, 42, 90), Color32::from_rgb(0, 255, 157), Stroke::new(1.5, Color32::from_rgb(0, 255, 157)))
                    } else {
                        (Color32::from_rgb(32, 24, 46), Color32::from_rgb(200, 190, 220), Stroke::new(1.0, Color32::from_rgb(50, 40, 70)))
                    };
                    if ui.add(egui::Button::new(RichText::new(format!("{:.1} kHz", rate as f32 / 1000.0)).size(11.0).color(text)).fill(bg).stroke(stroke).rounding(Rounding::same(4.0))).clicked() {
                        self.config.audio.sample_rate = rate;
                        self.sample_rate = rate;
                    }
                }
                help_marker(
                    ui,
                    lang.tr(
                        "Frequência de amostragem enviada para o conversor Digital-Analógico (DAC). 44.1 kHz é o padrão de CD/UTAU; 48.0 kHz e 96.0 kHz são padrões profissionais de estúdio.",
                        "Sampling frequency sent to Digital-to-Analog Converter (DAC). 44.1 kHz is standard for CD/UTAU; 48.0 kHz and 96.0 kHz are studio standards.",
                    ),
                );
            });

                ui.add_space(6.0);

                ui.horizontal(|ui| {
                ui.label(lang.tr("Tamanho do Buffer (Frames):", "Buffer Size (Frames):"));
                for buf in [64, 128, 256, 512, 1024, 2048] {
                    let is_sel = self.config.audio.buffer_size_frames == buf;
                    let (bg, text, stroke) = if is_sel {
                        (Color32::from_rgb(60, 42, 90), Color32::from_rgb(0, 255, 157), Stroke::new(1.5, Color32::from_rgb(0, 255, 157)))
                    } else {
                        (Color32::from_rgb(32, 24, 46), Color32::from_rgb(200, 190, 220), Stroke::new(1.0, Color32::from_rgb(50, 40, 70)))
                    };
                    if ui.add(egui::Button::new(RichText::new(format!("{buf}")).size(11.0).color(text)).fill(bg).stroke(stroke).rounding(Rounding::same(4.0))).clicked() {
                        self.config.audio.buffer_size_frames = buf;
                    }
                }

                let latency_ms = (self.config.audio.buffer_size_frames as f32 / self.config.audio.sample_rate.max(1) as f32) * 1000.0;
                ui.label(RichText::new(format!("(~{:.1} ms {})", latency_ms, lang.tr("latência", "latency"))).size(10.5).color(MelodyneTheme::TEXT_MUTED));
                help_marker(
                    ui,
                    lang.tr(
                        "Buffers menores (128-256) oferecem resposta instantânea ao tocar. Buffers maiores (512-1024) evitam engasgos em projetos muito pesados.",
                        "Smaller buffers (128-256) provide instant playback response. Larger buffers (512-1024) avoid dropouts on heavy projects.",
                    ),
                );
            });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                ui.label(lang.tr("Número de Períodos de Buffer:", "Number of Buffer Periods:"));
                for p in [1, 2, 3, 4] {
                    let is_sel = self.config.audio.buffer_periods == p;
                    let label = match p {
                        1 => lang.tr("1 (Buffer Único)", "1 (Single Buffer)"),
                        2 => lang.tr("2 (Buffer Duplo)", "2 (Double Buffer)"),
                        3 => lang.tr("3 (Buffer Triplo)", "3 (Triple Buffer)"),
                        _ => lang.tr("4 (Buffer Quádruplo)", "4 (Quad Buffer)"),
                    };
                    if ui.selectable_label(is_sel, label).clicked() {
                        self.config.audio.buffer_periods = p;
                    }
                }
                help_marker(
                    ui,
                    lang.tr(
                        "Double buffer (2) é o equilíbrio perfeito entre estabilidade e latência. Triple buffer (3) previne dropouts de áudio em CPUs sobrecarregadas.",
                        "Double buffer (2) offers the ideal balance between stability and latency. Triple buffer (3) prevents audio dropouts on loaded CPUs.",
                    ),
                );
            });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                ui.label(lang.tr("Compensação Manual de Latência de Monitoração:", "Manual Monitoring Latency Compensation:"));
                ui.add(egui::Slider::new(&mut self.config.audio.latency_compensation_ms, -100.0..=100.0).suffix(" ms"));
                help_marker(
                    ui,
                    lang.tr(
                        "Ajuste fino em milissegundos para sincronizar perfeitamente áudio de monitores externos com atraso de processamento de hardware.",
                        "Fine adjustment in milliseconds to perfectly sync external monitor audio with hardware processing delays.",
                    ),
                );
            });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                ui.label(lang.tr("Algoritmo de Dither de Saída:", "Output Dither Algorithm:"));
                let dither_modes = [
                    ("TPDF (Triangular)", lang.tr("TPDF (Triangular)", "TPDF (Triangular)")),
                    ("Noise Shaping (Fletcher)", lang.tr("Noise Shaping (Fletcher)", "Noise Shaping (Fletcher)")),
                    ("Desativado", lang.tr("Desativado", "Disabled")),
                ];
                for (dit, label) in dither_modes {
                    if ui.selectable_label(self.config.audio.dither_algorithm == dit, label).clicked() {
                        self.config.audio.dither_algorithm = dit.to_string();
                    }
                }
                help_marker(
                    ui,
                    lang.tr(
                        "Aplica ruído pseudo-aleatório triangular de baixíssimo nível para eliminar distorções de quantização em placas de 16-bit ou 24-bit.",
                        "Applies ultra-low level triangular pseudo-random noise to eliminate quantization distortion on 16-bit or 24-bit sound cards.",
                    ),
                );
            });

                ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.config.audio.high_quality_resampling,
                    lang.tr("Reamostragem Sinc em Tempo Real para Preview", "Real-Time Sinc Resampling for Preview"),
                );
                help_marker(
                    ui,
                    lang.tr(
                        "Aplica interpolação sinc com filtro passa-baixa anti-aliasing na reprodução caso o áudio original do voicebank esteja em frequência diferente da saída.",
                        "Applies sinc interpolation with anti-aliasing low-pass filter during playback when voicebank sample rate differs from output.",
                    ),
                );
            });
            },
        );

        section_card(
            ui,
            lang.tr(
                "Volume Master & Calibração de Pré-amplificação",
                "Master Volume & Preamp Calibration",
            ),
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(lang.tr("Volume Master Padrão:", "Default Master Volume:"));
                    ui.add(
                        egui::Slider::new(&mut self.config.audio.master_volume, 0.0..=2.0)
                            .text("x")
                            .custom_formatter(|v, _| format!("{:.0}%", v * 100.0)),
                    );
                    help_marker(
                        ui,
                        lang.tr(
                            "Multiplicador de ganho do barramento principal de reprodução.",
                            "Gain multiplier for the main playback bus.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label(lang.tr("Pré-amplificação de Monitoração:", "Monitoring Preamp:"));
                    ui.add(egui::Slider::new(&mut self.config.audio.preamp_gain_db, -12.0..=12.0).suffix(" dB"));
                    help_marker(
                        ui,
                        lang.tr(
                            "Ajuste fino de ganho em decibéis aplicado na saída para calibrar caixas ou fones sem alterar a exportação final.",
                            "Fine gain adjustment in decibels applied to output to calibrate monitors or headphones without altering the final export.",
                        ),
                    );
                });
            },
        );
    }
}
