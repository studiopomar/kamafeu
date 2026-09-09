use crate::gui::theme::MelodyneTheme;
use crate::gui::KamafeuStudioApp;
use crate::gui::RenderLogFilter;
use eframe::egui;

impl KamafeuStudioApp {
    pub(super) fn draw_mini_log_window(&mut self, ctx: &egui::Context) {
        if !self.render_log_window_open {
            return;
        }

        let lang = self.config.language;
        let mut is_open = self.render_log_window_open;
        let mut trigger_save_log = false;

        // Calculate counts per category
        let total_count = self.render_log_messages.len();
        let mut errors_count = 0;
        let mut dsp_count = 0;
        let mut wav_count = 0;
        let mut info_count = 0;

        for msg in &self.render_log_messages {
            let msg_upper = msg.to_uppercase();
            if msg_upper.contains("FAIL")
                || msg_upper.contains("ERR")
                || msg_upper.contains("WARN")
                || msg_upper.contains("FALLBACK")
            {
                errors_count += 1;
            }
            if msg.contains("[Resampler]") || msg.contains("[Wavtool]") || msg.contains("[DSP]") {
                dsp_count += 1;
            }
            if msg.contains("[WAV]") || msg.contains("[Render]") || msg.contains("oto=") {
                wav_count += 1;
            }
            if msg.contains("[INFO]")
                || msg.contains("Iniciando")
                || msg.contains("Concluído")
                || msg.contains("Carregado")
            {
                info_count += 1;
            }
        }

        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("engine_log_native_viewport"),
            egui::ViewportBuilder::default()
                .with_title(lang.tr("Kamafeu Studio - Terminal & Console de Renderização", "Kamafeu Studio - Terminal & Rendering Console"))
                .with_inner_size([780.0, 520.0])
                .with_min_inner_size([500.0, 320.0]),
            |ctx, _class| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    let is_rendering = self.render_progress < 1.0 || self.render_rx.is_some() || self.export_in_progress;

                    // 1. Header Toolbar
                    ui.horizontal(|ui| {
                        ui.heading(
                            egui::RichText::new(lang.tr("Terminal do Engine", "Engine Terminal"))
                                .strong()
                                .size(15.0)
                                .color(egui::Color32::from_rgb(0, 255, 180)),
                        );

                        // Real-time Status Badge
                        let (badge_bg, badge_stroke, badge_text, badge_color) = if is_rendering {
                            (
                                egui::Color32::from_rgb(50, 40, 20),
                                egui::Color32::from_rgb(255, 200, 50),
                                format!("{} ({:.0}%)", lang.tr("Renderizando", "Rendering"), self.render_progress * 100.0),
                                egui::Color32::from_rgb(255, 220, 100),
                            )
                        } else {
                            (
                                egui::Color32::from_rgb(18, 45, 32),
                                egui::Color32::from_rgb(0, 255, 150),
                                lang.tr("Engine Pronto", "Engine Ready").to_string(),
                                egui::Color32::from_rgb(0, 255, 180),
                            )
                        };

                        egui::Frame::none()
                            .fill(badge_bg)
                            .stroke(egui::Stroke::new(1.0, badge_stroke))
                            .rounding(egui::Rounding::same(4.0))
                            .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(badge_text)
                                        .size(10.5)
                                        .strong()
                                        .color(badge_color),
                                );
                            });

                        // Action Buttons
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .button(egui::RichText::new(lang.tr("Limpar", "Clear")).size(11.0))
                                .on_hover_text(lang.tr("Limpar histórico de mensagens do console", "Clear console message history"))
                                .clicked()
                            {
                                self.render_log_messages.clear();
                            }

                            if ui
                                .button(egui::RichText::new(lang.tr("Salvar Log...", "Save Log...")).size(11.0))
                                .on_hover_text(lang.tr("Salvar todo o log em arquivo .txt ou .log no disco", "Save entire log to .txt or .log file on disk"))
                                .clicked()
                            {
                                trigger_save_log = true;
                            }

                            if ui
                                .button(egui::RichText::new(lang.tr("Copiar Tudo", "Copy All")).size(11.0))
                                .on_hover_text(lang.tr("Copiar todas as mensagens do console para a área de transferência", "Copy all console messages to clipboard"))
                                .clicked()
                            {
                                let full_log = self.render_log_messages.join("\n");
                                ui.output_mut(|o| o.copied_text = full_log);
                                self.transport_state.status_message = lang.tr("Log copiado para a área de transferência!", "Log copied to clipboard!").to_string();
                            }
                        });
                    });

                    ui.add_space(4.0);

                    // 2. Engine Subheader & Progress Bar
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::ProgressBar::new(self.render_progress)
                                .text(format!("{:.0}%", self.render_progress * 100.0))
                                .fill(egui::Color32::from_rgb(0, 230, 138))
                                .animate(is_rendering)
                                .desired_width(ui.available_width() - 320.0),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!(
                                    "Resampler: {} · Wavtool: {} · Threads: {}t",
                                    self.selected_resampler, self.selected_wavtool, self.render_threads
                                ))
                                .size(10.0)
                                .color(egui::Color32::from_rgb(180, 170, 210)),
                            );
                        });
                    });

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // 3. Filter Pills and Search Bar
                    ui.horizontal(|ui| {
                        let filter_all_label = format!("{} ({})", lang.tr("Todos", "All"), total_count);
                        if ui.selectable_label(self.render_log_filter == RenderLogFilter::All, filter_all_label).clicked() {
                            self.render_log_filter = RenderLogFilter::All;
                        }

                        let filter_err_label = format!("{} ({})", lang.tr("Erros", "Errors"), errors_count);
                        if ui.selectable_label(self.render_log_filter == RenderLogFilter::ErrorsWarnings, filter_err_label).clicked() {
                            self.render_log_filter = RenderLogFilter::ErrorsWarnings;
                        }

                        let filter_dsp_label = format!("DSP ({})", dsp_count);
                        if ui.selectable_label(self.render_log_filter == RenderLogFilter::DspResampler, filter_dsp_label).clicked() {
                            self.render_log_filter = RenderLogFilter::DspResampler;
                        }

                        let filter_wav_label = format!("WAV ({})", wav_count);
                        if ui.selectable_label(self.render_log_filter == RenderLogFilter::WavOto, filter_wav_label).clicked() {
                            self.render_log_filter = RenderLogFilter::WavOto;
                        }

                        let filter_info_label = format!("Info ({})", info_count);
                        if ui.selectable_label(self.render_log_filter == RenderLogFilter::Info, filter_info_label).clicked() {
                            self.render_log_filter = RenderLogFilter::Info;
                        }

                        ui.add_space(8.0);
                        ui.label(egui::RichText::new(lang.tr("Buscar:", "Search:")).size(11.0).color(MelodyneTheme::TEXT_MUTED));
                        ui.add(
                            egui::TextEdit::singleline(&mut self.render_log_search_query)
                                .hint_text(lang.tr("Buscar mensagem ou nota...", "Search message or note..."))
                                .desired_width(160.0),
                        );
                        if !self.render_log_search_query.is_empty() {
                            if ui.small_button(lang.tr("Limpar", "Clear")).on_hover_text(lang.tr("Limpar busca", "Clear search")).clicked() {
                                self.render_log_search_query.clear();
                            }
                        }
                    });

                    ui.add_space(4.0);

                    // 4. View Preferences (Auto-scroll, Line Numbers, Word Wrap, Font Size)
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut self.auto_scroll_log, lang.tr("Rolar auto", "Auto-scroll"));
                        ui.checkbox(&mut self.render_log_show_line_numbers, lang.tr("Nº Linha", "Line #"));
                        ui.checkbox(&mut self.render_log_wrap_lines, lang.tr("Quebrar", "Word Wrap"));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("A+").on_hover_text(lang.tr("Aumentar tamanho da fonte", "Increase font size")).clicked() {
                                self.render_log_font_size = (self.render_log_font_size + 0.5).min(18.0);
                            }
                            if ui.small_button("A-").on_hover_text(lang.tr("Diminuir tamanho da fonte", "Decrease font size")).clicked() {
                                self.render_log_font_size = (self.render_log_font_size - 0.5).max(8.5);
                            }
                            ui.label(
                                egui::RichText::new(format!("{:.1}px", self.render_log_font_size))
                                    .size(10.0)
                                    .color(egui::Color32::from_rgb(170, 160, 190)),
                            );
                            ui.label(egui::RichText::new(format!("{}:", lang.tr("Fonte", "Font"))).size(10.0));
                        });
                    });

                    ui.add_space(4.0);

                    // 5. Terminal Console Body
                    let search_lower = self.render_log_search_query.trim().to_lowercase();
                    let font_sz = self.render_log_font_size;
                    let show_line_num = self.render_log_show_line_numbers;
                    let wrap_text = self.render_log_wrap_lines;

                    let mut displayed_lines = 0;

                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(10, 8, 16))
                        .rounding(egui::Rounding::same(6.0))
                        .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(45, 35, 65)))
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            egui::ScrollArea::both()
                                .id_salt("render_log_native_scroll")
                                .stick_to_bottom(self.auto_scroll_log)
                                .max_height(ui.available_height() - 28.0)
                                .show(ui, |ui| {
                                    if self.render_log_messages.is_empty() {
                                        ui.vertical_centered(|ui| {
                                            ui.add_space(30.0);
                                            ui.label(
                                                egui::RichText::new(format!(">_ {}", lang.tr("Console do Engine Kamafeu Studio", "Kamafeu Studio Engine Console")))
                                                    .size(13.0)
                                                    .monospace()
                                                    .strong()
                                                    .color(egui::Color32::from_rgb(0, 255, 180)),
                                            );
                                            ui.add_space(4.0);
                                            ui.label(
                                                egui::RichText::new(lang.tr(
                                                    "Aguardando tarefas de síntese, resampler e renderização...",
                                                    "Awaiting synthesis, resampler, and rendering tasks...",
                                                ))
                                                .size(11.0)
                                                .italics()
                                                .color(egui::Color32::from_rgb(140, 130, 160)),
                                            );
                                            ui.add_space(30.0);
                                        });
                                    } else {
                                        for (i, msg) in self.render_log_messages.iter().enumerate() {
                                            let msg_upper = msg.to_uppercase();
                                            let is_err = msg_upper.contains("FAIL") || msg_upper.contains("ERR") || msg_upper.contains("FALLBACK");
                                            let is_warn = msg_upper.contains("WARN");
                                            let is_dsp = msg.contains("[Resampler]") || msg.contains("[Wavtool]") || msg.contains("[DSP]");
                                            let is_wav = msg.contains("[WAV]") || msg.contains("[Render]") || msg.contains("oto=");
                                            let is_info = msg.contains("[INFO]") || msg.contains("Iniciando") || msg.contains("Concluído") || msg.contains("Carregado");

                                            let matches_filter = match self.render_log_filter {
                                                RenderLogFilter::All => true,
                                                RenderLogFilter::ErrorsWarnings => is_err || is_warn,
                                                RenderLogFilter::DspResampler => is_dsp,
                                                RenderLogFilter::WavOto => is_wav,
                                                RenderLogFilter::Info => is_info,
                                            };

                                            let matches_search = search_lower.is_empty()
                                                || msg.to_lowercase().contains(&search_lower);

                                            if matches_filter && matches_search {
                                                displayed_lines += 1;

                                                ui.horizontal(|ui| {
                                                    // Line Number
                                                    if show_line_num {
                                                        ui.label(
                                                            egui::RichText::new(format!("{:04} ", i + 1))
                                                                .size(font_sz * 0.9)
                                                                .monospace()
                                                                .color(egui::Color32::from_rgb(85, 75, 105)),
                                                        );
                                                    }

                                                    // Category Badge Pill
                                                    let (pill_text, pill_color) = if is_err {
                                                        ("[ERR]", egui::Color32::from_rgb(255, 90, 90))
                                                    } else if is_warn {
                                                        ("[WARN]", egui::Color32::from_rgb(255, 200, 60))
                                                    } else if is_dsp {
                                                        ("[DSP]", egui::Color32::from_rgb(216, 180, 254))
                                                    } else if is_wav {
                                                        ("[WAV]", egui::Color32::from_rgb(120, 220, 255))
                                                    } else if msg.contains("Concluído") || msg.contains("Pronto") {
                                                        ("[OK]", egui::Color32::from_rgb(0, 255, 157))
                                                    } else {
                                                        ("[LOG]", egui::Color32::from_rgb(150, 160, 190))
                                                    };

                                                    ui.label(
                                                        egui::RichText::new(pill_text)
                                                            .size(font_sz * 0.85)
                                                            .strong()
                                                            .monospace()
                                                            .color(pill_color),
                                                    );

                                                    // Text Body with Syntax Coloring
                                                    let text_color = if is_err {
                                                        egui::Color32::from_rgb(255, 130, 130)
                                                    } else if is_warn {
                                                        egui::Color32::from_rgb(255, 225, 140)
                                                    } else if is_dsp {
                                                        egui::Color32::from_rgb(230, 200, 255)
                                                    } else if is_wav {
                                                        egui::Color32::from_rgb(180, 235, 255)
                                                    } else if msg.contains("Concluído") || msg.contains("Pronto") {
                                                        egui::Color32::from_rgb(140, 255, 190)
                                                    } else {
                                                        egui::Color32::from_rgb(215, 210, 230)
                                                    };

                                                    let mut lbl = egui::Label::new(
                                                        egui::RichText::new(msg)
                                                            .size(font_sz)
                                                            .monospace()
                                                            .color(text_color),
                                                    );
                                                    if wrap_text {
                                                        lbl = lbl.wrap();
                                                    }
                                                    ui.add(lbl);
                                                });
                                            }
                                        }

                                        if displayed_lines == 0 && !self.render_log_messages.is_empty() {
                                            ui.vertical_centered(|ui| {
                                                ui.add_space(20.0);
                                                ui.label(
                                                    egui::RichText::new(lang.tr(
                                                        "Nenhuma mensagem encontrada com o filtro / busca atual.",
                                                        "No messages found with current filter / search.",
                                                    ))
                                                    .size(11.5)
                                                    .italics()
                                                    .color(egui::Color32::from_rgb(160, 150, 180)),
                                                );
                                                ui.add_space(20.0);
                                            });
                                        }
                                    }
                                });
                        });

                    ui.add_space(4.0);

                    // 6. Footer Status Info
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} {} {} {} {}",
                                lang.tr("Exibindo", "Showing"),
                                displayed_lines,
                                lang.tr("de", "of"),
                                total_count,
                                lang.tr("mensagens", "messages")
                            ))
                            .size(10.5)
                            .color(egui::Color32::from_rgb(160, 150, 180)),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(
                                egui::RichText::new(format!("Kamafeu Engine v1.0.0-A ({})", lang.tr("Âmbar", "Amber")))
                                    .size(10.0)
                                    .monospace()
                                    .color(egui::Color32::from_rgb(110, 100, 130)),
                            );
                        });
                    });
                });

                if ctx.input(|i| i.viewport().close_requested()) {
                    is_open = false;
                }
            },
        );

        if trigger_save_log {
            if let Some(path) = crate::dialogs::FileDialog::new()
                .add_filter(lang.tr("Arquivo de Log (*.txt; *.log)", "Log File (*.txt; *.log)"), &["txt", "log"])
                .set_file_name("kamafeu_engine.log")
                .save_file()
            {
                let header = format!(
                    "=== Kamafeu Studio Engine Log ===\nResampler: {}\nWavtool: {}\nThreads: {}\n{}: {}\n=================================\n\n",
                    self.selected_resampler,
                    self.selected_wavtool,
                    self.render_threads,
                    lang.tr("Total de Mensagens", "Total Messages"),
                    self.render_log_messages.len()
                );
                let content = header + &self.render_log_messages.join("\n");
                if std::fs::write(&path, content).is_ok() {
                    self.transport_state.status_message =
                        format!("{}: {}", lang.tr("Log exportado com sucesso para", "Log successfully exported to"), path.display());
                }
            }
        }

        self.render_log_window_open = is_open;
    }
}
