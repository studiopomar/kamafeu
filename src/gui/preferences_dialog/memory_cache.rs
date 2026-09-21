use super::help_marker;
use super::section_card;
use super::KamafeuStudioApp;
use crate::gui::theme::MelodyneTheme;
use crate::renderer::resampler_cache;
use eframe::egui;
use eframe::egui::Color32;
use eframe::egui::RichText;

impl KamafeuStudioApp {
    pub(in crate::gui) fn render_memory_cache_tab(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.language;
        section_card(
            ui,
            lang.tr(
                "Estatísticas de Cache em Disco & Memória RAM",
                "Disk Cache & RAM Memory Statistics",
            ),
            |ui| {
                let (file_count, disk_bytes) = resampler_cache::get_disk_cache_stats();
                let (mem_entries, mem_samples) = resampler_cache::get_memory_cache_stats();
                let disk_mb = disk_bytes as f64 / (1024.0 * 1024.0);
                let mem_mb = (mem_samples * 4) as f64 / (1024.0 * 1024.0);

                ui.horizontal(|ui| {
                ui.label(lang.tr("Armazenamento em Disco:", "Disk Storage:"));
                ui.label(RichText::new(format!("{file_count} {} ({:.2} MB)", lang.tr("arquivos", "files"), disk_mb)).strong().color(Color32::from_rgb(0, 255, 157)));
                help_marker(
                    ui,
                    lang.tr(
                        "Espaço em disco ocupado pelas amostras sintetizadas salvas para reutilização imediata.",
                        "Disk space occupied by synthesized samples cached for immediate reuse.",
                    ),
                );
            });

                ui.horizontal(|ui| {
                ui.label(lang.tr("Memória RAM (DSP Pool):", "RAM Memory (DSP Pool):"));
                ui.label(RichText::new(format!("{mem_entries} {} ({:.2} MB)", lang.tr("trechos", "slices"), mem_mb)).strong().color(Color32::from_rgb(180, 230, 255)));
                help_marker(
                    ui,
                    lang.tr(
                        "Memória RAM utilizada para manter trechos de áudio decodificados e formas de onda prontas para tocar sem acessar o disco.",
                        "RAM used to keep decoded audio slices and waveforms ready to play without disk access.",
                    ),
                );
            });

                ui.add_space(4.0);
                let cache_path = resampler_cache::persistent_cache_dir();
                ui.horizontal(|ui| {
                    ui.label(lang.tr("Pasta de Cache:", "Cache Directory:"));
                    ui.label(
                        RichText::new(cache_path.to_string_lossy().to_string())
                            .size(9.5)
                            .monospace()
                            .color(MelodyneTheme::TEXT_MUTED),
                    );
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                if ui
                    .button(
                        RichText::new(lang.tr("Limpar Todo o Cache em Disco Agora", "Clear All Disk Cache Now"))
                            .size(11.0)
                            .color(Color32::from_rgb(255, 140, 140)),
                    )
                    .clicked()
                {
                    let _ = resampler_cache::clear_disk_cache();
                }

                if ui
                    .button(
                        RichText::new(lang.tr("Esvaziar Pool de Memória RAM", "Empty RAM Memory Pool"))
                            .size(11.0)
                            .color(Color32::from_rgb(255, 180, 120)),
                    )
                    .clicked()
                {
                    resampler_cache::clear_memory_cache();
                }

                if ui
                    .button(RichText::new(crate::gui::dialogs::reveal_in_file_manager_label_for(lang)).size(11.0))
                    .clicked()
                {
                    let _ = std::fs::create_dir_all(&cache_path);
                    crate::gui::dialogs::open_file_in_folder(&cache_path);
                }

                help_marker(
                    ui,
                    lang.tr(
                        "Limpar o cache libera espaço em disco e força a re-síntese de todas as notas.",
                        "Clearing cache frees disk space and forces re-synthesis of all notes.",
                    ),
                );
            });
            },
        );

        section_card(
            ui,
            lang.tr(
                "Limites e Políticas de Armazenamento",
                "Limits and Storage Policies",
            ),
            |ui| {
                ui.horizontal(|ui| {
                ui.label(lang.tr("Limite Máximo de Memória RAM para Cache:", "Maximum RAM Cache Limit:"));
                ui.add(egui::Slider::new(&mut self.config.memory.max_ram_cache_mb, 256..=32768).suffix(" MB").logarithmic(true));
                help_marker(
                    ui,
                    lang.tr(
                        "Quantidade máxima de memória RAM dedicada ao cache de notas decodificadas antes de liberar as mais antigas.",
                        "Maximum amount of RAM dedicated to decoded note cache before evicting older entries.",
                    ),
                );
            });

                ui.horizontal(|ui| {
                    ui.label(lang.tr(
                        "Limite Máximo de Armazenamento em Disco:",
                        "Maximum Disk Storage Limit:",
                    ));
                    ui.add(
                        egui::Slider::new(&mut self.config.memory.max_disk_cache_mb, 512..=102400)
                            .suffix(" MB")
                            .logarithmic(true),
                    );
                    help_marker(
                        ui,
                        lang.tr(
                            "Teto de espaço em disco para arquivos temporários .wav renderizados.",
                            "Disk space ceiling for temporary rendered .wav audio files.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                ui.label(lang.tr("Estratégia de Pré-carregamento em RAM:", "RAM Preload Strategy:"));
                let strat_list = [
                    ("Lazy On-Demand", "Lazy On-Demand"),
                    ("Aquecimento de Fonemas", lang.tr("Aquecimento de Fonemas", "Phoneme Warming")),
                    ("Pré-carregamento Total", lang.tr("Pré-carregamento Total", "Total Preload")),
                ];
                for (val, label) in strat_list {
                    if ui.selectable_label(self.config.memory.ram_preload_strategy == val, label).clicked() {
                        self.config.memory.ram_preload_strategy = val.to_string();
                    }
                }
                help_marker(
                    ui,
                    lang.tr(
                        "Define se as amostras do voicebank são carregadas apenas quando tocadas ou pré-aquecidas na RAM.",
                        "Defines whether voicebank samples are loaded on-demand or pre-warmed in RAM.",
                    ),
                );
            });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                ui.label(lang.tr("Concorrência de Leitura em Disco (I/O Threads):", "Disk Read Concurrency (I/O Threads):"));
                ui.add(egui::Slider::new(&mut self.config.memory.io_thread_concurrency, 1..=16).suffix(" threads"));
                help_marker(
                    ui,
                    lang.tr(
                        "Número de threads assíncronas para carregar arquivos de áudio WAV do disco rígido/SSD simultaneamente.",
                        "Number of asynchronous threads to load WAV audio files from hard drive/SSD concurrently.",
                    ),
                );
            });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                ui.label(lang.tr("Compressão de Cache na Memória RAM:", "RAM Cache Compression:"));
                let comp_list = [
                    ("Nenhuma (Float32)", lang.tr("Nenhuma (Float32)", "None (Float32)")),
                    ("Compactação Float16", lang.tr("Compactação Float16", "Float16 Compression")),
                    ("LZ4 Áudio Rápido", lang.tr("LZ4 Áudio Rápido", "Fast Audio LZ4")),
                ];
                for (val, label) in comp_list {
                    if ui.selectable_label(self.config.memory.ram_cache_compression == val, label).clicked() {
                        self.config.memory.ram_cache_compression = val.to_string();
                    }
                }
                help_marker(
                    ui,
                    lang.tr(
                        "Permite armazenar o dobro de trechos de áudio na memória RAM com perda inaudível de resolução.",
                        "Allows storing double the audio slices in RAM with inaudible resolution trade-off.",
                    ),
                );
            });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.config.memory.clear_cache_on_exit,
                    lang.tr("Limpar cache temporário de renderização automaticamente ao fechar o Kamafeu Studio", "Automatically clear temporary render cache on closing Kamafeu Studio"),
                );
                help_marker(
                    ui,
                    lang.tr(
                        "Exclui arquivos de cache temporários do disco ao fechar o aplicativo, mantendo o armazenamento limpo.",
                        "Deletes temporary cache files from disk when closing the application, keeping storage clean.",
                    ),
                );
            });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                ui.label(lang.tr("Limpeza Automática de Arquivos Antigos:", "Automatic Cleanup of Old Files:"));
                ui.add(egui::Slider::new(&mut self.config.memory.auto_cleanup_days, 1..=60).suffix(lang.tr(" dias", " days")));
                help_marker(
                    ui,
                    lang.tr(
                        "Arquivos de cache que não forem acessados por esse número de dias serão excluídos automaticamente.",
                        "Cache files not accessed for this number of days will be deleted automatically.",
                    ),
                );
            });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(lang.tr(
                        "Resolução do Cache de Forma de Onda:",
                        "Waveform Cache Resolution:",
                    ));
                    ui.add(
                        egui::Slider::new(&mut self.config.memory.waveform_cache_resolution, 1..=8)
                            .text(lang.tr("detalhes", "details")),
                    );
                    help_marker(
                        ui,
                        lang.tr(
                            "Fator de mipmapping para exibição gráfica de waveforms na timeline.",
                            "Mipmapping factor for graphical waveform display on the timeline.",
                        ),
                    );
                });
            },
        );
    }
}
