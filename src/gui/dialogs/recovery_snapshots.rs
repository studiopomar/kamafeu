use crate::gui::KamafeuStudioApp;
#[cfg(not(target_arch = "wasm32"))]
use crate::gui::SnapshotRestoreKind;
use eframe::egui;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy)]
struct ProjectShape {
    tracks: usize,
    notes: usize,
    audio_parts: usize,
    markers: usize,
}

#[cfg(not(target_arch = "wasm32"))]
impl ProjectShape {
    fn from_project(project: &crate::project::UProject) -> Self {
        Self {
            tracks: project.tracks.len(),
            notes: project.parts.iter().map(|part| part.notes.len()).sum(),
            audio_parts: project.wave_parts.len(),
            markers: project.markers.len(),
        }
    }

    fn compact_label(self) -> String {
        format!(
            "{} pistas · {} notas · {} áudios · {} marcadores",
            self.tracks, self.notes, self.audio_parts, self.markers
        )
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn snapshot_shape(path: &std::path::Path) -> Result<ProjectShape, String> {
    crate::formats::ApsFormat::load_file(path)
        .map(|project| ProjectShape::from_project(&project))
        .map_err(|error| error.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn snapshot_diff(
    path: &std::path::Path,
    current: &crate::project::UProject,
) -> Result<crate::project::ProjectDiff, String> {
    crate::formats::ApsFormat::load_file(path)
        .map(|project| crate::project::ProjectDiff::between(current, &project))
        .map_err(|error| error.to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn snapshot_age_label(path: &std::path::Path) -> String {
    let elapsed = std::fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.elapsed().ok());
    match elapsed {
        Some(age) if age.as_secs() < 60 => "agora mesmo".to_string(),
        Some(age) if age.as_secs() < 3_600 => format!("há {} min", age.as_secs() / 60),
        Some(age) if age.as_secs() < 86_400 => format!("há {} h", age.as_secs() / 3_600),
        Some(age) => format!("há {} dias", age.as_secs() / 86_400),
        None => "data indisponível".to_string(),
    }
}

impl KamafeuStudioApp {
    pub(crate) fn render_recovery_snapshots_dialog(&mut self, ctx: &egui::Context) {
        if !self.recovery_snapshots_open {
            return;
        }

        let lang = self.config.language;
        let mut is_open = self.recovery_snapshots_open;
        #[cfg(not(target_arch = "wasm32"))]
        let mut snapshot_to_restore: Option<std::path::PathBuf> = None;
        egui::Window::new(lang.tr("Recuperar Snapshot", "Recover Snapshot"))
            .id(egui::Id::new("recovery_snapshots_dialog"))
            .open(&mut is_open)
            .default_width(560.0)
            .default_height(300.0)
            .resizable(true)
            .show(ctx, |ui| {
                #[cfg(target_arch = "wasm32")]
                {
                    ui.label(lang.tr(
                        "A edição Web não pode criar ou acessar snapshots locais. Use Exportar para guardar uma cópia do projeto.",
                        "The Web edition cannot create or access local snapshots. Use Export to keep a project copy.",
                    ));
                }

                #[cfg(not(target_arch = "wasm32"))]
                {
                    let current_shape = ProjectShape::from_project(&self.project);
                    ui.label(
                        egui::RichText::new(lang.tr(
                            "Snapshots são cópias automáticas. Abrir um deles não substitui seu projeto atual; salve o resultado com um novo nome.",
                            "Snapshots are automatic copies. Opening one does not replace your current project; save the result under a new name.",
                        ))
                        .color(self.config.theme.text_muted_c32()),
                    );
                    ui.add_space(8.0);

                    match self.recovery_snapshot_paths() {
                        Ok(snapshots) if snapshots.is_empty() => {
                            ui.label(lang.tr(
                                "Nenhum snapshot foi encontrado nesta pasta de projeto.",
                                "No snapshots were found in this project folder.",
                            ));
                        }
                        Ok(snapshots) => {
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                for path in snapshots {
                                    let filename = path
                                        .file_name()
                                        .and_then(|name| name.to_str())
                                        .unwrap_or("snapshot.aps");
                                    let bytes = std::fs::metadata(&path)
                                        .map(|metadata| metadata.len())
                                        .unwrap_or(0);
                                    let shape = snapshot_shape(&path);
                                    let diff = snapshot_diff(&path, &self.project);
                                    ui.group(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.label(egui::RichText::new(filename).strong());
                                                ui.label(
                                                    egui::RichText::new(format!(
                                                        "{} · {:.1} KB",
                                                        snapshot_age_label(&path),
                                                        bytes as f64 / 1024.0
                                                    ))
                                                    .small()
                                                    .color(self.config.theme.text_muted_c32()),
                                                );
                                                match shape {
                                                    Ok(shape) => {
                                                        let same_shape = shape.tracks == current_shape.tracks
                                                            && shape.notes == current_shape.notes
                                                            && shape.audio_parts == current_shape.audio_parts
                                                            && shape.markers == current_shape.markers;
                                                        let color = if same_shape {
                                                            self.config.theme.text_muted_c32()
                                                        } else {
                                                            egui::Color32::from_rgb(255, 194, 82)
                                                        };
                                                        ui.label(
                                                            egui::RichText::new(format!(
                                                                "Snapshot: {}{}",
                                                                shape.compact_label(),
                                                                if same_shape { "" } else { " · diferente do projeto aberto" }
                                                            ))
                                                            .small()
                                                            .color(color),
                                                        );
                                                        if let Ok(diff) = &diff {
                                                            let diff_hint = format!(
                                                                "Notas: {:?}\nÁudios: {:?}",
                                                                diff.changed_note_indices,
                                                                diff.changed_wave_part_indices,
                                                            );
                                                            ui.label(
                                                                egui::RichText::new(format!(
                                                                    "Alterações: {} notas · {} fonemas · {} áudio{}",
                                                                    diff.added_notes + diff.removed_notes + diff.changed_notes,
                                                                    diff.changed_phoneme_overrides,
                                                                    diff.added_wave_parts + diff.removed_wave_parts + diff.changed_wave_parts,
                                                                    if diff.added_wave_parts + diff.removed_wave_parts + diff.changed_wave_parts == 1 { "" } else { "s" },
                                                                ))
                                                                .small()
                                                                .color(self.config.theme.text_muted_c32()),
                                                            )
                                                            .on_hover_text(diff_hint);
                                                        }
                                                    }
                                                    Err(error) => {
                                                        ui.colored_label(
                                                            egui::Color32::from_rgb(255, 120, 120),
                                                            format!("Snapshot inválido: {error}"),
                                                        );
                                                    }
                                                }
                                            });
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if ui
                                                        .button(lang.tr("Abrir cópia", "Open copy"))
                                                        .clicked()
                                                    {
                                                        snapshot_to_restore = Some(path.clone());
                                                    }
                                                    if let Ok(diff) = &diff {
                                                        if !diff.changed_note_indices.is_empty()
                                                            && ui
                                                                .button(lang.tr(
                                                                    "Restaurar notas",
                                                                    "Restore notes",
                                                                ))
                                                                .on_hover_text(lang.tr(
                                                                    "Pede confirmação e registra undo antes de substituir somente as notas alteradas.",
                                                                    "Asks for confirmation and records undo before replacing only changed notes.",
                                                                ))
                                                                .clicked()
                                                        {
                                                            self.snapshot_restore_confirmation =
                                                                Some((path.clone(), SnapshotRestoreKind::Notes));
                                                        }
                                                        if !diff.changed_wave_part_indices.is_empty()
                                                            && ui
                                                                .button(lang.tr(
                                                                    "Restaurar áudio",
                                                                    "Restore audio",
                                                                ))
                                                                .on_hover_text(lang.tr(
                                                                    "Pede confirmação e restaura somente as partes de áudio alteradas.",
                                                                    "Asks for confirmation and restores only changed audio parts.",
                                                                ))
                                                                .clicked()
                                                        {
                                                            self.snapshot_restore_confirmation =
                                                                Some((path.clone(), SnapshotRestoreKind::Audio));
                                                        }
                                                        if diff.changed_arrangement {
                                                            if !diff.changed_marker_indices.is_empty()
                                                                && ui.button(lang.tr("Restaurar marcadores", "Restore markers")).clicked() {
                                                                self.snapshot_restore_confirmation =
                                                                    Some((path.clone(), SnapshotRestoreKind::Markers));
                                                            }
                                                            if !diff.changed_section_indices.is_empty()
                                                                && ui.button(lang.tr("Restaurar seções", "Restore sections")).clicked() {
                                                                self.snapshot_restore_confirmation =
                                                                    Some((path.clone(), SnapshotRestoreKind::Sections));
                                                            }
                                                        }
                                                    }
                                                },
                                            );
                                        });
                                    });
                                    ui.add_space(4.0);
                                }
                            });
                        }
                        Err(error) => {
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 120, 120),
                                format!("{}: {error}", lang.tr("Não foi possível listar snapshots", "Could not list snapshots")),
                            );
                        }
                    }
                }
            });

        #[cfg(not(target_arch = "wasm32"))]
        if let Some(path) = snapshot_to_restore {
            self.open_project_from_path(&path);
            is_open = false;
        }
        self.recovery_snapshots_open = is_open;

        #[cfg(not(target_arch = "wasm32"))]
        if let Some((path, restore_kind)) = self.snapshot_restore_confirmation.clone() {
            let mut confirm_open = true;
            egui::Window::new(lang.tr(
                "Confirmar restauração seletiva",
                "Confirm selective restore",
            ))
            .id(egui::Id::new("snapshot_selective_restore_confirmation"))
            .open(&mut confirm_open)
            .show(ctx, |ui| {
                let diff = snapshot_diff(&path, &self.project);
                match diff {
                    Ok(diff)
                        if !(match restore_kind {
                            SnapshotRestoreKind::Notes => diff.changed_note_indices.is_empty(),
                            SnapshotRestoreKind::Audio => diff.changed_wave_part_indices.is_empty(),
                            SnapshotRestoreKind::Markers => diff.changed_marker_indices.is_empty(),
                            SnapshotRestoreKind::Sections => diff.changed_section_indices.is_empty(),
                        }) =>
                    {
                        let indices = match restore_kind {
                            SnapshotRestoreKind::Notes => &diff.changed_note_indices,
                            SnapshotRestoreKind::Audio => &diff.changed_wave_part_indices,
                            SnapshotRestoreKind::Markers => &diff.changed_marker_indices,
                            SnapshotRestoreKind::Sections => &diff.changed_section_indices,
                        };
                        let label = match restore_kind {
                            SnapshotRestoreKind::Notes => "nota(s)",
                            SnapshotRestoreKind::Audio => "parte(s) de áudio",
                            SnapshotRestoreKind::Markers => "marcador(es)",
                            SnapshotRestoreKind::Sections => "seção(ões)",
                        };
                        ui.label(format!(
                            "Substituir {} {} do projeto atual pelo snapshot?",
                            indices.len(), label
                        ));
                        ui.label(
                            egui::RichText::new(match restore_kind {
                                SnapshotRestoreKind::Notes => "O arranjo, o áudio e as configurações de render serão preservados.",
                                SnapshotRestoreKind::Audio => "As notas, fonemas e configurações de render serão preservados.",
                                SnapshotRestoreKind::Markers | SnapshotRestoreKind::Sections => "Notas, fonemas e áudio serão preservados.",
                            })
                            .small(),
                        );
                        ui.horizontal(|ui| {
                            if ui.button(lang.tr("Cancelar", "Cancel")).clicked() {
                                self.snapshot_restore_confirmation = None;
                            }
                            if ui
                                .button(lang.tr("Restaurar e criar undo", "Restore and undo"))
                                .clicked()
                            {
                                let original = self.project.clone();
                                let snapshot_project =
                                    match crate::formats::ApsFormat::load_file(&path) {
                                        Ok(project) => project,
                                        Err(error) => {
                                            self.transport_state.status_message =
                                                format!("Snapshot inválido: {error}");
                                            return;
                                        }
                                    };
                                let restored = match restore_kind {
                                    SnapshotRestoreKind::Notes => crate::project::restore_note_indices(&self.project, &snapshot_project, indices),
                                    SnapshotRestoreKind::Audio => crate::project::restore_wave_part_indices(&self.project, &snapshot_project, indices),
                                    SnapshotRestoreKind::Markers => crate::project::restore_marker_indices(&self.project, &snapshot_project, indices),
                                    SnapshotRestoreKind::Sections => crate::project::restore_section_indices(&self.project, &snapshot_project, indices),
                                };
                                match restored {
                                    Ok(restored) => {
                                        self.undo_manager.push_state(original);
                                        self.project = restored;
                                        self.is_dirty = true;
                                        self.piano_roll_state.phoneme_cache_hash = 0;
                                        self.snapshot_restore_confirmation = None;
                                        self.transport_state.status_message = match restore_kind {
                                            SnapshotRestoreKind::Notes => "Notas restauradas do snapshot; undo disponível.",
                                            SnapshotRestoreKind::Audio => "Áudio restaurado do snapshot; undo disponível.",
                                            SnapshotRestoreKind::Markers => "Marcadores restaurados do snapshot; undo disponível.",
                                            SnapshotRestoreKind::Sections => "Seções restauradas do snapshot; undo disponível.",
                                        }
                                        .to_string();
                                    }
                                    Err(error) => {
                                        self.transport_state.status_message = error;
                                    }
                                }
                            }
                        });
                    }
                    Ok(_) => {
                        ui.label(lang.tr(
                            "Não há notas alteradas para restaurar.",
                            "There are no changed notes to restore.",
                        ));
                    }
                    Err(error) => {
                        ui.colored_label(egui::Color32::RED, error);
                    }
                }
            });
            if !confirm_open {
                self.snapshot_restore_confirmation = None;
            }
        }
    }
}
