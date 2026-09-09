use super::section_card;
use super::KamafeuStudioApp;
use crate::gui::theme::MelodyneTheme;
use eframe::egui;
use eframe::egui::Color32;
use eframe::egui::RichText;

impl KamafeuStudioApp {
    pub(in crate::gui) fn render_presets_and_help_tab(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.language;
        section_card(ui, lang.tr("Predefinições de desempenho", "Performance Presets"), |ui| {
            ui.label(
                RichText::new(lang.tr(
                    "Estas predefinições ajustam apenas reprodução, renderização e cache. O cantor, o projeto e a exportação permanecem como estão.",
                    "These presets adjust playback, rendering, and cache only. Singer, project, and export settings remain unchanged.",
                ))
                .size(11.0)
                .color(MelodyneTheme::TEXT_MUTED),
            );
            ui.add_space(8.0);

            ui.columns(3, |columns| {
                columns[0].vertical(|ui| {
                    ui.label(
                        RichText::new(lang.tr("Equilibrado", "Balanced"))
                            .strong()
                            .color(Color32::from_rgb(0, 255, 157)),
                    );
                    ui.label(lang.tr(
                        "Boa resposta e estabilidade para a maioria dos projetos.",
                        "Good responsiveness and stability for most projects.",
                    ));
                    if ui.button(lang.tr("Aplicar equilibrado", "Apply balanced")).clicked() {
                        self.config.audio.buffer_size_frames = 512;
                        self.config.audio.buffer_periods = 2;
                        self.config.dsp.render_threads = 0;
                        self.config.dsp.render_lookahead_ms = 2_000.0;
                        self.config.dsp.render_chunk_bars = 4;
                        self.config.memory.ram_preload_strategy = "Lazy On-Demand".to_string();
                        self.render_threads = std::thread::available_parallelism()
                            .map(|count| count.get() as u32)
                            .unwrap_or(4);
                        self.persist_config();
                    }
                });
                columns[1].vertical(|ui| {
                    ui.label(
                        RichText::new(lang.tr("Baixa latência", "Low Latency"))
                            .strong()
                            .color(Color32::from_rgb(180, 220, 255)),
                    );
                    ui.label(lang.tr(
                        "Resposta mais rápida em projetos pequenos e máquinas estáveis.",
                        "Faster response for smaller projects and stable systems.",
                    ));
                    if ui.button(lang.tr("Aplicar baixa latência", "Apply low latency")).clicked() {
                        self.config.audio.buffer_size_frames = 256;
                        self.config.audio.buffer_periods = 2;
                        self.config.dsp.render_threads = 0;
                        self.config.dsp.render_lookahead_ms = 1_000.0;
                        self.config.dsp.render_chunk_bars = 2;
                        self.config.memory.ram_preload_strategy =
                            "Aquecimento de Fonemas".to_string();
                        self.render_threads = std::thread::available_parallelism()
                            .map(|count| count.get() as u32)
                            .unwrap_or(4);
                        self.persist_config();
                    }
                });
                columns[2].vertical(|ui| {
                    ui.label(
                        RichText::new(lang.tr("Projeto pesado", "Heavy Project"))
                            .strong()
                            .color(Color32::from_rgb(255, 205, 110)),
                    );
                    ui.label(lang.tr(
                        "Mais tolerância a muitas notas, faixas e resamplers externos.",
                        "Higher tolerance for many notes, tracks, and external resamplers.",
                    ));
                    if ui.button(lang.tr("Aplicar projeto pesado", "Apply heavy project")).clicked() {
                        self.config.audio.buffer_size_frames = 1_024;
                        self.config.audio.buffer_periods = 3;
                        self.config.dsp.render_threads = 0;
                        self.config.dsp.render_lookahead_ms = 6_000.0;
                        self.config.dsp.render_chunk_bars = 8;
                        self.config.memory.max_ram_cache_mb = 4_096;
                        self.config.memory.ram_preload_strategy = "Lazy On-Demand".to_string();
                        self.render_threads = std::thread::available_parallelism()
                            .map(|count| count.get() as u32)
                            .unwrap_or(4);
                        self.persist_config();
                    }
                });
            });
        });

        section_card(ui, lang.tr("O que cada componente faz", "What Each Component Does"), |ui| {
            let references = [
                (
                    lang.tr("Resampler e wavtool", "Resampler and wavtool"),
                    lang.tr(
                        "O resampler adapta altura e duração da amostra; o wavtool posiciona e une os fonemas.",
                        "Resampler adapts sample pitch and duration; wavtool positions and stitches phonemes.",
                    ),
                    lang.tr("Guia oficial do OpenUtau", "Official OpenUtau Guide"),
                    "https://github.com/openutau/OpenUtau/wiki/Resamplers-and-Wavtools",
                ),
                (
                    lang.tr("Fonemizadores", "Phonemizers"),
                    lang.tr(
                        "Transformam letras ou palavras nos aliases esperados pelo voicebank.",
                        "Transform lyrics or words into aliases expected by the voicebank.",
                    ),
                    lang.tr("Documentação do OpenUtau", "OpenUtau Documentation"),
                    "https://github.com/openutau/OpenUtau/wiki/Phonemizers",
                ),
                (
                    "straycat-rs",
                    lang.tr(
                        "Resampler externo multiplataforma disponível no Kamafeu Studio.",
                        "Cross-platform external resampler available in Kamafeu Studio.",
                    ),
                    lang.tr("Repositório oficial", "Official Repository"),
                    "https://github.com/UtaUtaUtau/straycat-rs",
                ),
                (
                    "wavtool-yawu",
                    lang.tr(
                        "Wavtool multiplataforma compatível com o contrato clássico do UTAU.",
                        "Cross-platform wavtool compatible with classic UTAU specifications.",
                    ),
                    lang.tr("Repositório oficial", "Official Repository"),
                    "https://github.com/m13253/wavtool-yawu",
                ),
            ];

            for (name, description, link_label, url) in references {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(name).strong());
                    ui.label(description);
                    ui.hyperlink_to(format!("{link_label} ->"), url);
                });
                ui.add_space(5.0);
            }
        });
    }
}
