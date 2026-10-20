use super::super::KamafeuStudioApp;
use crate::extensions::{discover, ExtensionManifest, EXTENSION_MANIFEST_FILE};
use eframe::egui;
use std::path::{Path, PathBuf};

fn plugins_directory() -> PathBuf {
    crate::config::KamafeuConfig::config_path()
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("extensions")
}

fn copy_directory(source: &Path, destination: &Path) -> Result<(), String> {
    std::fs::create_dir(destination).map_err(|error| error.to_string())?;
    for entry in std::fs::read_dir(source).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            copy_directory(&source_path, &destination_path)?;
        } else if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_file()
        {
            std::fs::copy(&source_path, &destination_path).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

impl KamafeuStudioApp {
    pub(crate) fn show_plugins_window(&mut self, ctx: &egui::Context) {
        if !self.plugins_window_open {
            return;
        }

        let directory = plugins_directory();
        let discovery = if directory.exists() {
            discover(&directory)
        } else {
            crate::extensions::ExtensionDiscovery::default()
        };
        let mut open = self.plugins_window_open;
        egui::Window::new("Plugins")
            .open(&mut open)
            .default_size([620.0, 520.0])
            .min_width(420.0)
            .show(ctx, |ui| {
                super::visual::header(
                    ui,
                    super::visual::Art::Plugins,
                    "Plugins",
                    "Extensões locais e ferramentas integradas do Kamafeu.",
                    self.config.theme.success_c32(),
                );
                ui.add_space(8.0);
                ui.label(
                    "Plugins são descobertos pelo manifesto. A descoberta não executa código.",
                );
                super::visual::card(ui, |ui| {
                    ui.heading("Ferramentas integradas");
                    ui.label("Ações locais que já aproveitam os recursos do Kamafeu.");
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("🎙 Vocal para APS — GAME").clicked() {
                            self.game_transcription.open = true;
                        }
                        if ui.button("🧭 Radar de Frase").clicked() {
                            self.project_diagnostic_open = true;
                        }
                        if ui.button("🎨 Curva Viva").clicked() {
                            self.autopitch_window_open = true;
                        }
                    });
                    ui.small("Radar de Frase abre os diagnósticos do projeto; Curva Viva abre o gerador de expressão e pitch já disponível no editor.");
                });
                ui.horizontal(|ui| {
                    ui.label(format!("Pasta: {}", directory.display()));
                    if ui.button("Importar pasta…").clicked() {
                        if let Some(source) = crate::dialogs::FileDialog::new().pick_folder() {
                            let result = (|| {
                                let manifest_path = source.join(EXTENSION_MANIFEST_FILE);
                                let manifest_text = std::fs::read_to_string(&manifest_path)
                                    .map_err(|error| {
                                        format!("Manifesto não encontrado: {error}")
                                    })?;
                                let manifest: ExtensionManifest =
                                    serde_json::from_str(&manifest_text)
                                        .map_err(|error| format!("Manifesto inválido: {error}"))?;
                                manifest.validate()?;
                                let target = directory.join(&manifest.id);
                                if target.exists() {
                                    return Err(format!(
                                        "Já existe um plugin com o ID {}",
                                        manifest.id
                                    ));
                                }
                                std::fs::create_dir_all(&directory)
                                    .map_err(|error| error.to_string())?;
                                copy_directory(&source, &target)?;
                                Ok(format!("Plugin {} importado.", manifest.name))
                            })();
                            self.plugins_status = Some(match result {
                                Ok(message) => message,
                                Err(error) => format!("Falha na importação: {error}"),
                            });
                        }
                    }
                });
                if let Some(message) = &self.plugins_status {
                    ui.label(message);
                }
                ui.separator();
                if discovery.extensions.is_empty() && discovery.diagnostics.is_empty() {
                    ui.label(
                        "Nenhum plugin instalado. Importe uma pasta com kamafeu-extension.json.",
                    );
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for extension in &discovery.extensions {
                        super::visual::card(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.strong(&extension.manifest.name);
                                ui.label(format!("v{}", extension.manifest.version));
                                ui.label(format!("{:?}", extension.manifest.kind));
                            });
                            ui.label(&extension.manifest.description);
                            ui.label(format!("ID: {}", extension.manifest.id));
                            ui.label(format!(
                                "Capacidades: {}",
                                extension
                                    .manifest
                                    .capabilities
                                    .iter()
                                    .cloned()
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ));
                            ui.label(format!(
                                "Permissões solicitadas: {}",
                                extension
                                    .manifest
                                    .permissions
                                    .iter()
                                    .cloned()
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            ));
                            ui.small(extension.manifest_path.display().to_string());
                            let abi_min = extension
                                .manifest
                                .abi_min_version
                                .unwrap_or(extension.manifest.api_version);
                            let abi_max = extension
                                .manifest
                                .abi_max_version
                                .unwrap_or(extension.manifest.api_version);
                            if extension.manifest.kind == crate::extensions::ExtensionKind::Analysis
                                && abi_min <= 2
                                && abi_max >= 2
                                && ui.button("Analisar faixa ativa…").clicked()
                            {
                                let notes: Vec<_> = self
                                    .project
                                    .parts
                                    .iter()
                                    .filter(|part| part.track_index == self.active_track_index)
                                    .flat_map(|part| part.notes.iter())
                                    .map(|note| {
                                        serde_json::json!({
                                            "lyric": note.lyric,
                                            "pitch": note.pitch,
                                            "position_ms": note.position_ms,
                                            "duration_ms": note.duration_ms,
                                            "pitch_delta_cents": note.expressions.pitch_delta,
                                        })
                                    })
                                    .collect();
                                let context = serde_json::json!({
                                    "schema_version": 1,
                                    "project_name": self.project.name,
                                    "track_index": self.active_track_index,
                                    "notes": notes,
                                });
                                let result = crate::extensions::invoke_wasm_json(
                                    extension,
                                    &crate::extensions::ExtensionPermissionPolicy::default(),
                                    "analyze_json",
                                    &context,
                                    &serde_json::Map::new(),
                                );
                                self.plugins_status = Some(match result {
                                    Ok(report) => format!("{}: {}", extension.manifest.name, report),
                                    Err(error) => format!("{}: {error}", extension.manifest.name),
                                });
                            }
                        });
                    }
                    for diagnostic in &discovery.diagnostics {
                        ui.colored_label(ui.visuals().error_fg_color, diagnostic);
                    }
                });
            });
        self.plugins_window_open = open;
    }
}
