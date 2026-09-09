use super::*;

impl KamafeuStudioApp {
    pub(super) fn menu_tracks(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.language;
        let is_pt = lang.is_pt();

        ui.menu_button(lang.tr("Faixas", "Tracks"), |ui| {
            if ui
                .button(lang.tr("+ Nova Faixa (Track)", "+ New Track"))
                .clicked()
            {
                let new_idx = self.project.tracks.len();
                let track_name = format!("Track {}", new_idx + 1);
                self.project.tracks.push(crate::project::model::UTrack {
                    name: track_name.clone(),
                    singer: if is_pt {
                        "Cantor Padrão".to_string()
                    } else {
                        "Default Singer".to_string()
                    },
                    volume_db: 0.0,
                    pan: 0.0,
                    mute: false,
                    solo: false,
                    ..crate::project::model::UTrack::default()
                });
                self.project
                    .parts
                    .push(crate::project::model::UVoicePart::new(
                        format!("{} {}", lang.tr("Parte", "Part"), new_idx + 1),
                        new_idx,
                    ));
                self.active_track_index = new_idx;
                ui.close_menu();
            }
            if ui
                .button(lang.tr("Duplicar Faixa Ativa", "Duplicate Active Track"))
                .clicked()
            {
                let active_idx = self.active_track_index;
                if active_idx < self.project.tracks.len() {
                    let mut new_track = self.project.tracks[active_idx].clone();
                    new_track.name = format!("{} ({})", new_track.name, lang.tr("Cópia", "Copy"));
                    let new_track_idx = self.project.tracks.len();
                    self.project.tracks.push(new_track);

                    let matching_parts: Vec<_> = self
                        .project
                        .parts
                        .iter()
                        .filter(|p| p.track_index == active_idx)
                        .cloned()
                        .collect();
                    for mut part in matching_parts {
                        part.track_index = new_track_idx;
                        part.name = format!("{} ({})", part.name, lang.tr("Cópia", "Copy"));
                        self.project.parts.push(part);
                    }
                    self.active_track_index = new_track_idx;
                    self.push_history();
                    self.transport_state.status_message = lang
                        .tr(
                            "Faixa duplicada com sucesso",
                            "Track duplicated successfully",
                        )
                        .to_string();
                }
                ui.close_menu();
            }
            if self.project.tracks.len() > 1
                && ui
                    .button(lang.tr("Excluir Faixa Ativa", "Delete Active Track"))
                    .clicked()
            {
                let del_idx = self.active_track_index;
                if del_idx < self.project.tracks.len() {
                    self.project.tracks.remove(del_idx);
                    self.project.parts.retain(|p| p.track_index != del_idx);
                    for p in self.project.parts.iter_mut() {
                        if p.track_index > del_idx {
                            p.track_index -= 1;
                        }
                    }
                    if self.active_track_index >= self.project.tracks.len() {
                        self.active_track_index = self.project.tracks.len().saturating_sub(1);
                    }
                }
                ui.close_menu();
            }
            ui.separator();
            if let Some(track) = self.project.tracks.get_mut(self.active_track_index) {
                if ui
                    .checkbox(&mut track.mute, lang.tr("Mudo (Mute) [M]", "Mute [M]"))
                    .clicked()
                {
                    ui.close_menu();
                }
                if ui.checkbox(&mut track.solo, "Solo").clicked() {
                    ui.close_menu();
                }
            }
            ui.separator();
            if ui
                .button(lang.tr(
                    "Refonetizar Toda a Faixa Ativa",
                    "Re-phonemize Entire Active Track",
                ))
                .clicked()
            {
                self.rephonemize_all_notes();
                ui.close_menu();
            }
        });
    }
}
