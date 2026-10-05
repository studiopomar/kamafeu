use crate::gui::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(crate) fn render_markers_dialog(&mut self, ctx: &egui::Context) {
        if !self.markers_dialog_open {
            return;
        }

        let lang = self.config.language;
        let original_project = self.project.clone();
        let mut is_open = self.markers_dialog_open;
        let mut changed = false;
        let mut delete_index = None;
        let mut add_marker = false;
        let mut delete_section_index = None;
        let mut add_section = false;

        egui::Window::new(lang.tr("Marcadores e Seções", "Markers & Sections"))
            .id(egui::Id::new("project_markers_dialog"))
            .open(&mut is_open)
            .default_width(610.0)
            .default_height(310.0)
            .resizable(true)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(lang.tr(
                        "Use marcadores para versos, refrões, pontes e pontos de revisão.",
                        "Use markers for verses, choruses, bridges, and review points.",
                    ))
                    .color(self.config.theme.text_muted_c32()),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui
                        .button(lang.tr("+ Marcador no playhead", "+ Marker at playhead"))
                        .clicked()
                    {
                        add_marker = true;
                    }
                    if ui
                        .button(lang.tr("+ Seção no playhead", "+ Section at playhead"))
                        .clicked()
                    {
                        add_section = true;
                    }
                });
                ui.add_space(6.0);

                egui::ScrollArea::vertical().show(ui, |ui| {
                    for (index, marker) in self.project.markers.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("{:02}", index + 1))
                                    .monospace()
                                    .color(self.config.theme.text_muted_c32()),
                            );
                            changed |= ui
                                .add(
                                    egui::TextEdit::singleline(&mut marker.name)
                                        .desired_width(200.0),
                                )
                                .changed();
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut marker.position_ms)
                                        .speed(10.0)
                                        .range(0.0..=86_400_000.0)
                                        .suffix(" ms"),
                                )
                                .changed();
                            let mut color = marker.color.clone().unwrap_or_default();
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut color)
                                        .hint_text("#00dccc")
                                        .desired_width(82.0),
                                )
                                .changed()
                            {
                                marker.color = (!color.trim().is_empty()).then_some(color);
                                changed = true;
                            }
                            if ui
                                .small_button("×")
                                .on_hover_text(lang.tr("Remover marcador", "Remove marker"))
                                .clicked()
                            {
                                delete_index = Some(index);
                            }
                        });
                    }
                    if self.project.markers.is_empty() {
                        ui.add_space(12.0);
                        ui.label(
                            egui::RichText::new(lang.tr(
                                "Nenhum marcador ainda. Crie um no playhead para começar.",
                                "No markers yet. Create one at the playhead to begin.",
                            ))
                            .italics()
                            .color(self.config.theme.text_muted_c32()),
                        );
                    }
                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new(lang.tr("Seções", "Sections")).strong());
                    for (index, section) in self.project.sections.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(format!("{:02}", index + 1))
                                    .monospace()
                                    .color(self.config.theme.text_muted_c32()),
                            );
                            changed |= ui
                                .add(
                                    egui::TextEdit::singleline(&mut section.name)
                                        .desired_width(150.0),
                                )
                                .changed();
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut section.start_ms)
                                        .speed(10.0)
                                        .range(0.0..=86_400_000.0)
                                        .suffix(" ms"),
                                )
                                .changed();
                            changed |= ui
                                .add(
                                    egui::DragValue::new(&mut section.end_ms)
                                        .speed(10.0)
                                        .range(0.0..=86_400_000.0)
                                        .suffix(" ms"),
                                )
                                .changed();
                            let mut color = section.color.clone().unwrap_or_default();
                            if ui
                                .add(
                                    egui::TextEdit::singleline(&mut color)
                                        .hint_text("#00dccc")
                                        .desired_width(82.0),
                                )
                                .changed()
                            {
                                section.color = (!color.trim().is_empty()).then_some(color);
                                changed = true;
                            }
                            if ui
                                .small_button("×")
                                .on_hover_text(lang.tr("Remover seção", "Remove section"))
                                .clicked()
                            {
                                delete_section_index = Some(index);
                            }
                        });
                    }
                    if self.project.sections.is_empty() {
                        ui.label(
                            egui::RichText::new(lang.tr(
                                "Seções destacam trechos como verso e refrão na linha do tempo.",
                                "Sections mark ranges such as verses and choruses on the timeline.",
                            ))
                            .italics()
                            .color(self.config.theme.text_muted_c32()),
                        );
                    }
                });
            });

        if add_marker {
            let number = self.project.markers.len() + 1;
            self.project
                .markers
                .push(crate::project::UProjectMarker::new(
                    format!("{} {}", lang.tr("Marcador", "Marker"), number),
                    self.piano_roll_state.playhead_ms.max(0.0),
                ));
            changed = true;
        }
        if let Some(index) = delete_index {
            self.project.markers.remove(index);
            changed = true;
        }
        if add_section {
            let number = self.project.sections.len() + 1;
            let beat_ms = 60_000.0 / self.project.bpm.max(20.0) * 4.0
                / f64::from(self.project.time_signature_denominator.max(1));
            let bar_ms = beat_ms * f64::from(self.project.time_signature_numerator.max(1));
            let start_ms = self.piano_roll_state.playhead_ms.max(0.0);
            self.project
                .sections
                .push(crate::project::UProjectSection::new(
                    format!("{} {}", lang.tr("Seção", "Section"), number),
                    start_ms,
                    start_ms + bar_ms,
                ));
            changed = true;
        }
        if let Some(index) = delete_section_index {
            self.project.sections.remove(index);
            changed = true;
        }
        if changed {
            self.project.normalize();
            self.undo_manager.push_state(original_project);
            self.is_dirty = true;
            self.transport_state.status_message = lang
                .tr(
                    "Marcadores do arranjo atualizados",
                    "Arrangement markers updated",
                )
                .to_string();
        }
        self.markers_dialog_open = is_open;
    }
}
