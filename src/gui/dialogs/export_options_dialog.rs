use crate::gui::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(super) fn show_export_options_dialog(&mut self, ctx: &egui::Context) {
        if self.export_options_dialog_open {
            let lang = self.config.language;
            let mut is_open = self.export_options_dialog_open;
            let mut trigger_export = false;
            let mut trigger_close = false;

            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("export_options_native_viewport"),
                egui::ViewportBuilder::default()
                    .with_title(lang.tr("Opções de Exportação WAV - Kamafeu Studio", "WAV Export Options - Kamafeu Studio"))
                    .with_inner_size([500.0, 300.0])
                    .with_min_inner_size([440.0, 240.0]),
                |ctx, _class| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.add_space(4.0);
                        ui.heading(
                            egui::RichText::new(lang.tr("Opções e Configurações de Exportação", "Export Options & Settings"))
                                .strong()
                                .size(15.0)
                                .color(egui::Color32::from_rgb(0, 220, 255)),
                        );
                        ui.label(
                            egui::RichText::new(lang.tr(
                                "Escolha o escopo de áudio, formato e qualidade desejados para a exportação:",
                                "Choose the audio scope, format, and quality desired for export:",
                            ))
                            .size(11.0)
                            .color(egui::Color32::from_rgb(170, 160, 190)),
                        );

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        ui.radio_value(
                            &mut self.export_audio_scope,
                            crate::gui::types::ExportAudioScope::VocalsAndAudio,
                            egui::RichText::new(lang.tr("Mix Completa (Vocais + Instrumental/Áudios)", "Full Mix (Vocals + Instrumental/Audio)"))
                                .strong()
                                .size(13.0)
                                .color(egui::Color32::from_rgb(230, 240, 255)),
                        );
                        ui.indent("opt_full_mix", |ui| {
                            ui.label(
                                egui::RichText::new(lang.tr(
                                    "Exporta a música finalizada, combinando as vozes sintetizadas com as faixas de áudio importadas.",
                                    "Exports final song, combining synthesized vocals with imported audio tracks.",
                                ))
                                .size(11.0)
                                .color(egui::Color32::from_rgb(150, 145, 175)),
                            );
                        });

                        ui.add_space(6.0);

                        ui.radio_value(
                            &mut self.export_audio_scope,
                            crate::gui::types::ExportAudioScope::VocalsOnly,
                            egui::RichText::new(lang.tr("Apenas Vocais (Acapella)", "Vocals Only (Acapella)"))
                                .strong()
                                .size(13.0)
                                .color(egui::Color32::from_rgb(230, 240, 255)),
                        );
                        ui.indent("opt_vocals_only", |ui| {
                            ui.label(
                                egui::RichText::new(lang.tr(
                                    "Exporta apenas a síntese das vozes (stems/acapella), silenciando as faixas instrumentais.",
                                    "Exports synthesized voices only (stems/acapella), muting instrumental tracks.",
                                ))
                                .size(11.0)
                                .color(egui::Color32::from_rgb(150, 145, 175)),
                            );
                        });

                        ui.add_space(6.0);

                        ui.radio_value(
                            &mut self.export_audio_scope,
                            crate::gui::types::ExportAudioScope::SeparateTrackStems,
                            egui::RichText::new(lang.tr("Exportar Faixas Separadas (Stems / Multitrack)", "Export Separate Tracks (Stems / Multitrack)"))
                                .strong()
                                .size(13.0)
                                .color(egui::Color32::from_rgb(230, 240, 255)),
                        );
                        ui.indent("opt_stems", |ui| {
                            ui.label(
                                egui::RichText::new(lang.tr(
                                    "Gera arquivos de áudio individuais para cada faixa de voz e instrumental separadamente.",
                                    "Generates individual audio files for each vocal and instrumental track separately.",
                                ))
                                .size(11.0)
                                .color(egui::Color32::from_rgb(150, 145, 175)),
                            );
                        });

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        ui.label(
                            egui::RichText::new(lang.tr("Formato do Arquivo de Áudio:", "Audio File Format:"))
                                .strong()
                                .size(12.5)
                                .color(egui::Color32::from_rgb(0, 220, 255)),
                        );

                        egui::ComboBox::from_id_salt("export_format_options_combo")
                            .selected_text(self.export_audio_format.display_name())
                            .width(320.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.export_audio_format,
                                    crate::renderer::AudioExportFormat::Wav16,
                                    crate::renderer::AudioExportFormat::Wav16.display_name(),
                                );
                                ui.selectable_value(
                                    &mut self.export_audio_format,
                                    crate::renderer::AudioExportFormat::Wav24,
                                    crate::renderer::AudioExportFormat::Wav24.display_name(),
                                );
                                ui.selectable_value(
                                    &mut self.export_audio_format,
                                    crate::renderer::AudioExportFormat::Wav32Float,
                                    crate::renderer::AudioExportFormat::Wav32Float.display_name(),
                                );
                                ui.selectable_value(
                                    &mut self.export_audio_format,
                                    crate::renderer::AudioExportFormat::Flac16,
                                    crate::renderer::AudioExportFormat::Flac16.display_name(),
                                );
                                ui.selectable_value(
                                    &mut self.export_audio_format,
                                    crate::renderer::AudioExportFormat::Flac24,
                                    crate::renderer::AudioExportFormat::Flac24.display_name(),
                                );
                                ui.selectable_value(
                                    &mut self.export_audio_format,
                                    crate::renderer::AudioExportFormat::RawF32,
                                    crate::renderer::AudioExportFormat::RawF32.display_name(),
                                );
                            });

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            if ui.button(lang.tr("Cancelar", "Cancel")).clicked() {
                                trigger_close = true;
                            }

                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                let export_btn = egui::Button::new(
                                    egui::RichText::new(lang.tr("Escolher Destino e Exportar...", "Choose Destination & Export..."))
                                        .strong()
                                        .size(12.0)
                                        .color(egui::Color32::BLACK),
                                )
                                .fill(egui::Color32::from_rgb(0, 255, 180))
                                .min_size(egui::vec2(160.0, 28.0));

                                if ui.add(export_btn).clicked() {
                                    trigger_export = true;
                                    trigger_close = true;
                                }
                            });
                        });
                    });
                    if ctx.input(|i| i.viewport().close_requested()) {
                        trigger_close = true;
                    }
                },
            );

            if trigger_close {
                is_open = false;
            }
            self.export_options_dialog_open = is_open;

            if trigger_export {
                let scope = self.export_audio_scope;
                self.execute_export_wav(scope);
            }
        }
    }
}
