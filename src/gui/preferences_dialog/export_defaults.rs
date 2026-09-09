use super::help_marker;
use super::section_card;
use super::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(in crate::gui) fn render_export_defaults_tab(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.language;
        section_card(
            ui,
            lang.tr("Formato & Qualidade Padrão de Exportação Master", "Master Export Default Format & Quality"),
            |ui| {
                ui.horizontal(|ui| {
                    ui.label(lang.tr("Formato Padrão:", "Default Format:"));
                    for fmt in ["WAV (Lossless PCM)", "FLAC (Lossless)", "RAW PCM"] {
                        if ui
                            .selectable_label(self.config.export.format == fmt, fmt)
                            .clicked()
                        {
                            self.config.export.format = fmt.to_string();
                        }
                    }
                    help_marker(
                        ui,
                        lang.tr(
                            "Formato de arquivo gerado por padrão na exportação master.",
                            "Default file format generated in master export.",
                        ),
                    );
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(lang.tr("Resolução de Bits:", "Bit Depth:"));
                    for bit in [16, 24, 32] {
                        let label = if bit == 32 { "32-bit Float" } else if bit == 24 { "24-bit PCM" } else { "16-bit PCM" };
                        if ui.selectable_label(self.config.export.bit_depth == bit, label).clicked() {
                            self.config.export.bit_depth = bit;
                        }
                    }
                    help_marker(
                        ui,
                        lang.tr(
                            "Profundidade de bits do áudio exportado. 24-bit e 32-bit Float preservam toda a faixa dinâmica de mixagem.",
                            "Bit depth of exported audio. 24-bit and 32-bit Float preserve the full dynamic range of the mix.",
                        ),
                    );
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(lang.tr("Taxa de Amostragem:", "Sample Rate:"));
                    for rate in [44100, 48000, 88200, 96000] {
                        if ui
                            .selectable_label(
                                self.config.export.sample_rate == rate,
                                format!("{rate} Hz"),
                            )
                            .clicked()
                        {
                            self.config.export.sample_rate = rate;
                        }
                    }
                    help_marker(
                        ui,
                        lang.tr(
                            "Frequência de amostragem da exportação master.",
                            "Sample rate for master export.",
                        ),
                    );
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.config.export.normalize_audio,
                        lang.tr("Normalizar Pico de Áudio Automaticamente", "Automatically Normalize Peak Audio"),
                    );
                    if self.config.export.normalize_audio {
                        ui.add(
                            egui::Slider::new(
                                &mut self.config.export.normalize_peak_db,
                                -6.0..=0.0,
                            )
                            .suffix(" dBFS"),
                        );
                    }
                    help_marker(
                        ui,
                        lang.tr(
                            "Ajuste o ganho da master para atingir o nível máximo sem clipar.",
                            "Adjusts master gain to reach maximum level without clipping.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label(lang.tr("Silêncio de Cauda (Tail Silence):", "Tail Silence:"));
                    ui.add(egui::Slider::new(&mut self.config.export.tail_silence_ms, 0.0..=3000.0).suffix(" ms"));
                    help_marker(
                        ui,
                        lang.tr(
                            "Tempo extra de áudio adicionado ao final do arquivo para acomodar caudas naturais de reverberação e releases.",
                            "Extra audio time added at the end of the file to accommodate natural reverb tails and releases.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                    ui.checkbox(
                        &mut self.config.export.embed_project_metadata,
                        lang.tr(
                            "Embutir Metadados do Projeto no Cabeçalho WAV/FLAC (RIFF INFO / Vorbis)",
                            "Embed Project Metadata in WAV/FLAC Headers (RIFF INFO / Vorbis)",
                        ),
                    );
                    help_marker(
                        ui,
                        lang.tr(
                            "Salva nome do projeto, cantor, autor e tempo (BPM) nos metadados do arquivo de áudio.",
                            "Saves project name, singer, author, and tempo (BPM) into audio file metadata.",
                        ),
                    );
                });
            },
        );
    }
}
