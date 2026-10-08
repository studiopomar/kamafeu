use crate::gui::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(crate) fn render_project_diagnostic_dialog(&mut self, ctx: &egui::Context) {
        if !self.project_diagnostic_open {
            return;
        }

        let lang = self.config.language;
        let report = self.project.diagnostic_report();
        let mut is_open = self.project_diagnostic_open;
        let mut issue_to_focus = None;
        let mut requested_fix = None;
        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("project_diagnostic_native_viewport"),
            egui::ViewportBuilder::default()
                .with_title(lang.tr(
                    "Validação do Projeto - Kamafeu Studio",
                    "Project Validation - Kamafeu Studio",
                ))
                .with_inner_size([620.0, 440.0])
                .with_min_inner_size([460.0, 300.0]),
            |ctx, _class| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading(
                            egui::RichText::new(
                                lang.tr("Validação do Projeto", "Project Validation"),
                            )
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
                    ui.label(egui::RichText::new(&self.project.name).strong());
                    ui.label(
                        egui::RichText::new(format!(
                            "{} · {} {} · {} {} · {} {} · {} {}",
                            format!("{:.2} BPM", self.project.bpm),
                            report.track_count,
                            lang.tr("faixas", "tracks"),
                            report.note_count,
                            lang.tr("notas", "notes"),
                            report.wave_part_count,
                            lang.tr("áudios", "audio parts"),
                            self.project.sections.len(),
                            lang.tr("seções", "sections"),
                        ))
                        .size(10.0)
                        .color(crate::gui::theme::MelodyneTheme::TEXT_MUTED),
                    );
                    let health_color = if report.error_count() > 0 {
                        egui::Color32::from_rgb(255, 120, 100)
                    } else if report.warning_count() > 0 {
                        egui::Color32::from_rgb(255, 205, 100)
                    } else {
                        egui::Color32::from_rgb(0, 255, 180)
                    };
                    ui.label(
                        egui::RichText::new(format!(
                            "{}: {}/100 · {} {} · {} {}",
                            lang.tr("Saúde vocal", "Vocal health"),
                            report.vocal_health_score(),
                            report.error_count(),
                            lang.tr("erros", "errors"),
                            report.warning_count(),
                            lang.tr("avisos", "warnings"),
                        ))
                        .size(10.5)
                        .color(health_color),
                    );
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    if report.is_valid() {
                        ui.label(
                            egui::RichText::new(lang.tr(
                                "[OK] A estrutura do projeto está pronta para renderização.",
                                "[OK] The project structure is ready for rendering.",
                            ))
                            .color(egui::Color32::from_rgb(0, 255, 180)),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} {}",
                                lang.tr("[Erro]", "[Error]"),
                                lang.tr(
                                    "Corrija os problemas abaixo antes de renderizar.",
                                    "Fix the issues below before rendering."
                                )
                            ))
                            .color(egui::Color32::from_rgb(255, 100, 90)),
                        );
                    }
                    if report.issues.is_empty() {
                        return;
                    }
                    ui.add_space(8.0);
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for issue in &report.issues {
                            let color = match issue.severity {
                                crate::project::ProjectIssueSeverity::Error => {
                                    egui::Color32::from_rgb(255, 120, 100)
                                }
                                crate::project::ProjectIssueSeverity::Warning => {
                                    egui::Color32::from_rgb(255, 205, 100)
                                }
                            };
                            ui.group(|ui| {
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{:?} · {}",
                                            issue.kind, issue.location
                                        ))
                                        .strong()
                                        .size(10.5)
                                        .color(color),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            if let Some(label) =
                                                suggested_fix_label(issue.kind, lang)
                                            {
                                                if ui.small_button(label).clicked() {
                                                    requested_fix = Some(issue.kind);
                                                }
                                            }
                                            if issue_target(&issue.location).is_some()
                                                && ui
                                                    .small_button(lang.tr("Localizar", "Locate"))
                                                    .clicked()
                                            {
                                                issue_to_focus = Some(issue.clone());
                                            }
                                        },
                                    );
                                });
                                ui.label(egui::RichText::new(&issue.detail).size(10.0));
                                if let Some(suggestion) = issue.suggestion() {
                                    ui.label(
                                        egui::RichText::new(format!("Sugestão: {suggestion}"))
                                            .italics()
                                            .size(9.5)
                                            .color(egui::Color32::from_rgb(180, 200, 220)),
                                    );
                                }
                            });
                            ui.add_space(4.0);
                        }
                    });
                });
                if ctx.input(|input| input.viewport().close_requested()) {
                    is_open = false;
                }
            },
        );
        if let Some(issue) = issue_to_focus {
            self.focus_project_diagnostic_issue(&issue);
        }
        if let Some(kind) = requested_fix {
            self.project_diagnostic_fix_confirmation = Some(kind);
        }
        self.project_diagnostic_open = is_open;
        self.render_project_diagnostic_fix_confirmation(ctx);
    }

    fn render_project_diagnostic_fix_confirmation(&mut self, ctx: &egui::Context) {
        let Some(kind) = self.project_diagnostic_fix_confirmation else {
            return;
        };
        let lang = self.config.language;
        let mut keep_open = true;
        egui::Window::new(lang.tr("Confirmar correção", "Confirm fix"))
            .id(egui::Id::new("project_diagnostic_fix_confirmation"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.label(diagnostic_fix_description(kind, lang));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button(lang.tr("Cancelar", "Cancel")).clicked() {
                        keep_open = false;
                    }
                    if ui
                        .button(lang.tr("Aplicar correção", "Apply fix"))
                        .clicked()
                    {
                        self.apply_project_diagnostic_fix(kind);
                        keep_open = false;
                    }
                });
            });
        if !keep_open {
            self.project_diagnostic_fix_confirmation = None;
        }
    }

    fn apply_project_diagnostic_fix(&mut self, kind: crate::project::ProjectIssueKind) {
        match kind {
            crate::project::ProjectIssueKind::NoteOverlap => self.fix_overlapping_notes(),
            crate::project::ProjectIssueKind::ExtremePhonemeTiming => {
                let original = self.project.clone();
                let mut changed = 0usize;
                for note in self
                    .project
                    .parts
                    .iter_mut()
                    .flat_map(|part| &mut part.notes)
                {
                    for value in [
                        &mut note.expressions.consonant_timing_offset_ms,
                        &mut note.expressions.preutter_offset_ms,
                        &mut note.expressions.overlap_offset_ms,
                    ] {
                        let sanitized = if value.is_finite() {
                            (*value).clamp(-300.0, 300.0)
                        } else {
                            0.0
                        };
                        if (*value - sanitized).abs() > f64::EPSILON || !value.is_finite() {
                            *value = sanitized;
                            changed += 1;
                        }
                    }
                }
                if changed > 0 {
                    self.undo_manager.push_state(original);
                    self.is_dirty = true;
                    self.piano_roll_state.phoneme_cache_hash = 0;
                    self.transport_state.status_message = format!(
                        "{} offsets de fonema ajustados para a faixa segura",
                        changed
                    );
                }
            }
            crate::project::ProjectIssueKind::InvalidMarkerPosition => {
                let original = self.project.clone();
                let mut changed = 0usize;
                for marker in &mut self.project.markers {
                    let position = if marker.position_ms.is_finite() {
                        marker.position_ms.max(0.0)
                    } else {
                        0.0
                    };
                    if marker.position_ms != position {
                        marker.position_ms = position;
                        changed += 1;
                    }
                }
                if changed > 0 {
                    self.undo_manager.push_state(original);
                    self.is_dirty = true;
                    self.transport_state.status_message =
                        format!("{} marcador(es) reposicionado(s) no início seguro", changed);
                }
            }
            crate::project::ProjectIssueKind::InvalidSectionRange => {
                let original = self.project.clone();
                let mut changed = 0usize;
                for section in &mut self.project.sections {
                    let start = if section.start_ms.is_finite() {
                        section.start_ms.max(0.0)
                    } else {
                        0.0
                    };
                    let end = if section.end_ms.is_finite() {
                        section.end_ms.max(start)
                    } else {
                        start
                    };
                    if section.start_ms != start || section.end_ms != end {
                        section.start_ms = start;
                        section.end_ms = end;
                        changed += 1;
                    }
                }
                if changed > 0 {
                    self.undo_manager.push_state(original);
                    self.is_dirty = true;
                    self.transport_state.status_message = format!(
                        "{} seção(ões) ajustada(s) para um intervalo seguro",
                        changed
                    );
                }
            }
            _ => {}
        }
    }

    fn focus_project_diagnostic_issue(&mut self, issue: &crate::project::ProjectIssue) {
        let Some(target) = issue_target(&issue.location) else {
            return;
        };

        match target {
            DiagnosticTarget::VoiceNote {
                part_index,
                note_index,
            } => {
                let Some(part) = self.project.parts.get(part_index) else {
                    return;
                };
                let Some(note) = part.notes.get(note_index) else {
                    return;
                };
                self.active_track_index = part
                    .track_index
                    .min(self.project.tracks.len().saturating_sub(1));
                self.piano_roll_state.selected_note_indices.clear();
                self.piano_roll_state
                    .selected_note_indices
                    .insert(note_index);
                self.piano_roll_state.selected_note_index = Some(note_index);
                let time_ms = part.position_ms + note.position_ms;
                self.piano_roll_state.playhead_ms = time_ms;
                self.piano_roll_state.horizontal_scroll_offset =
                    (time_ms as f32 * self.piano_roll_state.px_per_ms - 140.0).max(0.0);
                self.piano_roll_state.show_arrangement_view = false;
                self.transport_state.status_message = format!(
                    "Diagnóstico: nota {} da parte {} selecionada",
                    note_index + 1,
                    part_index + 1
                );
            }
            DiagnosticTarget::VoicePart { part_index } => {
                let Some(part) = self.project.parts.get(part_index) else {
                    return;
                };
                self.active_track_index = part
                    .track_index
                    .min(self.project.tracks.len().saturating_sub(1));
                self.piano_roll_state.playhead_ms = part.position_ms;
                self.piano_roll_state.horizontal_scroll_offset =
                    (part.position_ms as f32 * self.piano_roll_state.px_per_ms - 140.0).max(0.0);
                self.transport_state.status_message =
                    format!("Diagnóstico: parte vocal {} localizada", part_index + 1);
            }
            DiagnosticTarget::WavePart { wave_index } => {
                let Some(wave) = self.project.wave_parts.get(wave_index) else {
                    return;
                };
                self.active_track_index = wave
                    .track_index
                    .min(self.project.tracks.len().saturating_sub(1));
                self.piano_roll_state.playhead_ms = wave.position_ms;
                self.piano_roll_state.arrangement_horizontal_scroll_offset =
                    (wave.position_ms as f32 * self.piano_roll_state.px_per_ms - 140.0).max(0.0);
                self.piano_roll_state.show_arrangement_view = true;
                self.transport_state.status_message =
                    format!("Diagnóstico: áudio {} localizado", wave_index + 1);
            }
            DiagnosticTarget::Marker { marker_index } => {
                let Some(marker) = self.project.markers.get(marker_index) else {
                    return;
                };
                self.piano_roll_state.playhead_ms = marker.position_ms.max(0.0);
                self.piano_roll_state.horizontal_scroll_offset =
                    (self.piano_roll_state.playhead_ms as f32 * self.piano_roll_state.px_per_ms
                        - 140.0)
                        .max(0.0);
                self.transport_state.status_message =
                    format!("Diagnóstico: marcador {} localizado", marker_index + 1);
            }
            DiagnosticTarget::Section { section_index } => {
                let Some(section) = self.project.sections.get(section_index) else {
                    return;
                };
                self.piano_roll_state.playhead_ms = section.start_ms.max(0.0);
                self.piano_roll_state.horizontal_scroll_offset =
                    (self.piano_roll_state.playhead_ms as f32 * self.piano_roll_state.px_per_ms
                        - 140.0)
                        .max(0.0);
                self.transport_state.status_message =
                    format!("Diagnóstico: seção {} localizada", section_index + 1);
            }
        }
    }
}

fn suggested_fix_label(
    kind: crate::project::ProjectIssueKind,
    lang: crate::config::AppLanguage,
) -> Option<&'static str> {
    match kind {
        crate::project::ProjectIssueKind::NoteOverlap => Some(lang.tr("Corrigir todas", "Fix all")),
        crate::project::ProjectIssueKind::ExtremePhonemeTiming => {
            Some(lang.tr("Limitar offsets", "Clamp offsets"))
        }
        crate::project::ProjectIssueKind::InvalidMarkerPosition => {
            Some(lang.tr("Reposicionar marcadores", "Reposition markers"))
        }
        crate::project::ProjectIssueKind::InvalidSectionRange => {
            Some(lang.tr("Ajustar seções", "Fix sections"))
        }
        _ => None,
    }
}

fn diagnostic_fix_description(
    kind: crate::project::ProjectIssueKind,
    lang: crate::config::AppLanguage,
) -> &'static str {
    match kind {
        crate::project::ProjectIssueKind::NoteOverlap => lang.tr(
            "O Kamafeu vai encurtar notas que invadem a próxima nota na mesma parte. Esta ação pode ser desfeita.",
            "Kamafeu will shorten notes that run into the next note in the same part. This action can be undone.",
        ),
        crate::project::ProjectIssueKind::ExtremePhonemeTiming => lang.tr(
            "O Kamafeu vai limitar offsets de consoante, preutterance e overlap a ±300 ms. Esta ação pode ser desfeita.",
            "Kamafeu will limit consonant, preutterance and overlap offsets to ±300 ms. This action can be undone.",
        ),
        crate::project::ProjectIssueKind::InvalidMarkerPosition => lang.tr(
            "O Kamafeu vai mover marcadores com posição inválida para o início da timeline. Esta ação pode ser desfeita.",
            "Kamafeu will move markers with invalid positions to the beginning of the timeline. This action can be undone.",
        ),
        crate::project::ProjectIssueKind::InvalidSectionRange => lang.tr(
            "O Kamafeu vai limitar início e fim de seções a um intervalo seguro. Esta ação pode ser desfeita.",
            "Kamafeu will constrain section start and end to a safe range. This action can be undone.",
        ),
        _ => "",
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DiagnosticTarget {
    VoiceNote {
        part_index: usize,
        note_index: usize,
    },
    VoicePart {
        part_index: usize,
    },
    WavePart {
        wave_index: usize,
    },
    Marker {
        marker_index: usize,
    },
    Section {
        section_index: usize,
    },
}

fn issue_target(location: &str) -> Option<DiagnosticTarget> {
    if let Some(part) = location.strip_prefix("voice part ") {
        if let Some((part, note)) = part.split_once(" / note ") {
            return Some(DiagnosticTarget::VoiceNote {
                part_index: one_based_index(part)?,
                note_index: one_based_index(note)?,
            });
        }
        return Some(DiagnosticTarget::VoicePart {
            part_index: one_based_index(part)?,
        });
    }
    if let Some(wave) = location.strip_prefix("wave part ") {
        return Some(DiagnosticTarget::WavePart {
            wave_index: one_based_index(wave)?,
        });
    }
    if let Some(marker) = location.strip_prefix("marker ") {
        return Some(DiagnosticTarget::Marker {
            marker_index: one_based_index(marker)?,
        });
    }
    if let Some(section) = location.strip_prefix("section ") {
        return Some(DiagnosticTarget::Section {
            section_index: one_based_index(section)?,
        });
    }
    None
}

fn one_based_index(value: &str) -> Option<usize> {
    value.trim().parse::<usize>().ok()?.checked_sub(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_diagnostic_locations_without_guessing() {
        assert_eq!(
            issue_target("voice part 2 / note 4"),
            Some(DiagnosticTarget::VoiceNote {
                part_index: 1,
                note_index: 3
            })
        );
        assert_eq!(
            issue_target("voice part 3"),
            Some(DiagnosticTarget::VoicePart { part_index: 2 })
        );
        assert_eq!(
            issue_target("wave part 1"),
            Some(DiagnosticTarget::WavePart { wave_index: 0 })
        );
        assert_eq!(issue_target("project"), None);
        assert_eq!(issue_target("voice part 0"), None);
        assert_eq!(
            issue_target("marker 2"),
            Some(DiagnosticTarget::Marker { marker_index: 1 })
        );
        assert_eq!(
            issue_target("section 4"),
            Some(DiagnosticTarget::Section { section_index: 3 })
        );
    }
}
