use crate::gui::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(crate) fn render_project_properties_dialog(&mut self, ctx: &egui::Context) {
        if !self.project_properties_open {
            return;
        }

        let lang = self.config.language;
        let mut is_open = self.project_properties_open;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("project_properties_native_viewport"),
            egui::ViewportBuilder::default()
                .with_title(lang.tr("Propriedades do Projeto - Kamafeu Studio", "Project Properties - Kamafeu Studio"))
                .with_inner_size([540.0, 480.0])
                .with_min_inner_size([420.0, 360.0]),
            |ctx, _class| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading(
                            egui::RichText::new(lang.tr("Propriedades do Projeto", "Project Properties"))
                                .strong()
                                .color(egui::Color32::from_rgb(0, 255, 180)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(lang.tr("Fechar", "Close")).clicked() {
                                is_open = false;
                            }
                        });
                    });
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    egui::ScrollArea::vertical().show(ui, |ui| {
                        egui::Grid::new("project_props_grid")
                            .num_columns(2)
                            .spacing([16.0, 10.0])
                            .show(ui, |ui| {
                                ui.label(egui::RichText::new(lang.tr("Nome do Projeto:", "Project Name:")).strong());
                                ui.add(egui::TextEdit::singleline(&mut self.project.name).desired_width(260.0));
                                ui.end_row();

                                ui.label(egui::RichText::new(lang.tr("Andamento (BPM):", "Tempo (BPM):")).strong());
                                ui.add(egui::DragValue::new(&mut self.project.bpm).range(20.0..=999.0).speed(0.5));
                                ui.end_row();

                                ui.label(egui::RichText::new(lang.tr("Arquivo Salvo:", "Saved File:")).strong());
                                let path_str = self.current_project_path.as_ref().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| lang.tr("Não salvo no disco", "Not saved to disk").to_string());
                                ui.label(egui::RichText::new(path_str).monospace().size(10.5));
                                ui.end_row();

                                ui.label(egui::RichText::new(lang.tr("Total de Pistas Vocais:", "Total Vocal Tracks:")).strong());
                                ui.label(format!("{}", self.project.tracks.len()));
                                ui.end_row();

                                ui.label(egui::RichText::new(lang.tr("Total de Áudios (Wave Parts):", "Total Audio Parts (Wave Parts):")).strong());
                                ui.label(format!("{}", self.project.wave_parts.len()));
                                ui.end_row();

                                let total_notes: usize = self.project.parts.iter().map(|p| p.notes.len()).sum();
                                ui.label(egui::RichText::new(lang.tr("Total de Notas:", "Total Notes:")).strong());
                                ui.label(format!("{} {}", total_notes, lang.tr("notas", "notes")));
                                ui.end_row();

                                let max_end_ms = self.project.parts.iter().flat_map(|p| &p.notes).map(|n| n.position_ms + n.duration_ms).fold(0.0f64, f64::max);
                                let mins = (max_end_ms / 60000.0) as usize;
                                let secs = ((max_end_ms % 60000.0) / 1000.0) as usize;
                                ui.label(egui::RichText::new(lang.tr("Duração Estimada:", "Estimated Duration:")).strong());
                                ui.label(format!("{:02}:{:02} ({:.1}s)", mins, secs, max_end_ms / 1000.0));
                                ui.end_row();

                                ui.label(egui::RichText::new(lang.tr("Voicebank Ativo:", "Active Voicebank:")).strong());
                                let vb_name = self.voicebank.as_ref().map(|v| v.name.as_str()).unwrap_or(lang.tr("Nenhum", "None"));
                                ui.label(egui::RichText::new(vb_name).color(egui::Color32::from_rgb(0, 255, 200)));
                                ui.end_row();
                            });

                        ui.add_space(12.0);
                        ui.separator();
                        ui.add_space(8.0);

                        ui.label(egui::RichText::new(lang.tr("Anotações da Sessão / Letra do Projeto:", "Session Notes / Project Lyrics:")).strong());
                        ui.add(
                            egui::TextEdit::multiline(&mut self.project_comment_buffer)
                                .desired_rows(6)
                                .desired_width(f32::INFINITY)
                                .hint_text(lang.tr(
                                    "Insira anotações de mixagem, referências harmônicas ou rascunho de letra...",
                                    "Insert mixing notes, harmonic references or lyrics draft...",
                                )),
                        );
                    });
                });
                if ctx.input(|i| i.viewport().close_requested()) {
                    is_open = false;
                }
            },
        );
        self.project_properties_open = is_open;
    }
}
