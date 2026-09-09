use crate::gui::types::ExportAudioScope;
use crate::gui::KamafeuStudioApp;
use crate::oto::Voicebank;
use crate::renderer::AudioExportFormat;
use crate::renderer::AudioExporter;
use crate::renderer::ProjectRenderer;
use std::path::PathBuf;
use std::sync::Arc;

impl KamafeuStudioApp {
    pub fn export_wav(&mut self) {
        self.export_options_dialog_open = true;
    }

    pub fn execute_export_wav(&mut self, scope: ExportAudioScope) {
        let ext = self.export_audio_format.extension();
        let default_name = match scope {
            ExportAudioScope::VocalsOnly if !self.project.wave_parts.is_empty() => {
                format!("vocals.{ext}")
            }
            _ => format!("output.{ext}"),
        };

        if let Some(save_path) = crate::dialogs::FileDialog::new()
            .add_filter(
                "Todos os Formatos Suportados (*.wav, *.flac, *.raw)",
                &["wav", "flac", "raw", "pcm"],
            )
            .add_filter("WAV Audio (*.wav)", &["wav"])
            .add_filter("FLAC Lossless Audio (*.flac)", &["flac"])
            .add_filter("RAW PCM Audio (*.raw)", &["raw", "pcm"])
            .set_file_name(&default_name)
            .save_file()
        {
            let has_notes = self.project.parts.iter().any(|part| !part.notes.is_empty());
            let has_waves = !self.project.wave_parts.is_empty();

            if !has_notes && (scope == ExportAudioScope::VocalsOnly || !has_waves) {
                self.transport_state.status_message = "Nenhuma nota para exportar".to_string();
                return;
            }

            let export_format = if let Some(ext) = save_path.extension().and_then(|e| e.to_str()) {
                match ext.to_ascii_lowercase().as_str() {
                    "flac" => match self.export_audio_format {
                        AudioExportFormat::Flac24 => AudioExportFormat::Flac24,
                        _ => AudioExportFormat::Flac16,
                    },
                    "raw" | "pcm" => AudioExportFormat::RawF32,
                    _ => match self.export_audio_format {
                        AudioExportFormat::Wav24 => AudioExportFormat::Wav24,
                        AudioExportFormat::Wav32Float => AudioExportFormat::Wav32Float,
                        _ => AudioExportFormat::Wav16,
                    },
                }
            } else {
                self.export_audio_format
            };

            let resampler_driver = self.create_resampler_driver();
            let wavtool_driver = self.create_wavtool_driver();

            let bpm = self.transport_state.bpm;
            let sample_rate = self.sample_rate;
            let dummy_vb = Voicebank {
                root_path: PathBuf::from("."),
                name: "Synthetic Fallback".to_string(),
                author: "System".to_string(),
                character_info: String::new(),
                readme_info: String::new(),
                image_path: None,
                entries: std::collections::HashMap::new(),
                case_insensitive_entries: Default::default(),
                prefix_map: crate::oto::PrefixMap::default(),
                temp_dir: None,
            };

            let active_vb = self.voicebank.clone().unwrap_or(dummy_vb);
            let vocal_mode_params = self.vocal_mode_params.clone();
            let render_threads = self.render_threads.clamp(1, 16) as usize;
            let mut project = self.project.clone();
            project.bpm = bpm;

            if scope == ExportAudioScope::VocalsOnly {
                project.wave_parts.clear();
            }

            self.export_dialog_open = true;
            self.export_in_progress = true;
            self.export_progress = 0.0;
            self.export_save_path = Some(save_path.clone());
            self.export_status_detail = match scope {
                ExportAudioScope::VocalsOnly => {
                    format!(
                        "Iniciando exportação Acapella ({})",
                        export_format.display_name()
                    )
                }
                ExportAudioScope::VocalsAndAudio => {
                    format!(
                        "Iniciando exportação Mix Completa ({})",
                        export_format.display_name()
                    )
                }
                ExportAudioScope::SeparateTrackStems => {
                    format!(
                        "Iniciando exportação Multifaixas / Stems ({})",
                        export_format.display_name()
                    )
                }
            };
            self.export_result = None;

            self.render_log_window_open = false;
            self.render_progress = 0.0;
            let start_log = format!(
                "[Export] Iniciando exportação ({:?} - {}) para {:?}...",
                scope,
                export_format.display_name(),
                save_path
            );
            self.render_log_messages.push(start_log);
            self.render_status_title = format!(
                "Exportando {} ({})",
                export_format.extension().to_uppercase(),
                save_path.file_name().unwrap_or_default().to_string_lossy()
            );

            let fx_config = self.fx_rack_config.clone();

            let (tx, rx) = std::sync::mpsc::channel();
            self.render_log_channel_rx = Some(rx);

            let (export_tx, export_rx) = std::sync::mpsc::channel();
            self.export_rx = Some(export_rx);

            let tx = Arc::new(std::sync::Mutex::new(tx));
            let tx_cb = tx.clone();
            std::thread::spawn(move || {
                let report_progress = move |prog, msg: &str| {
                    if let Ok(guard) = tx_cb.lock() {
                        let _ = guard.send((prog, msg.to_string()));
                    }
                };

                let mut fx_proc = crate::audio::FxRackProcessor::new(fx_config, sample_rate);

                if scope == ExportAudioScope::SeparateTrackStems {
                    // Export individual stems
                    let parent = save_path
                        .parent()
                        .unwrap_or_else(|| std::path::Path::new("."));
                    let file_stem = save_path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("stem");
                    let ext = export_format.extension();

                    let track_count = project.tracks.len();
                    for (t_idx, track) in project.tracks.iter().enumerate() {
                        let mut track_proj = project.clone();
                        track_proj.parts.retain(|p| p.track_index == t_idx);
                        track_proj.wave_parts.clear();

                        if track_proj.parts.iter().any(|p| !p.notes.is_empty()) {
                            let track_save_path = parent.join(format!(
                                "{file_stem}_stem_{:02}_{}.{ext}",
                                t_idx + 1,
                                track.name.replace(' ', "_")
                            ));
                            report_progress(
                                (t_idx as f32) / (track_count.max(1) as f32),
                                &format!(
                                    "[Stem] Renderizando faixa {}: {}...",
                                    t_idx + 1,
                                    &track.name
                                ),
                            );

                            let mut audio = ProjectRenderer::render_project_with_drivers(
                                &track_proj,
                                &active_vb,
                                sample_rate,
                                0.0,
                                resampler_driver.as_ref(),
                                wavtool_driver.as_ref(),
                                &vocal_mode_params,
                                None,
                            );
                            if let Some(error) = audio.error.take() {
                                let _ = export_tx.send(Err(error));
                                return;
                            }
                            fx_proc.process_interleaved(&mut audio.samples, audio.sample_rate);
                            let _ = AudioExporter::export_audio(
                                &track_save_path,
                                &audio.samples,
                                audio.sample_rate,
                                audio.channels,
                                export_format,
                            );
                        }
                    }

                    // Also export full mixdown
                    let mut audio = ProjectRenderer::render_project_with_drivers(
                        &project,
                        &active_vb,
                        sample_rate,
                        0.0,
                        resampler_driver.as_ref(),
                        wavtool_driver.as_ref(),
                        &vocal_mode_params,
                        None,
                    );
                    if let Some(error) = audio.error.take() {
                        let _ = export_tx.send(Err(error));
                        return;
                    }
                    fx_proc.process_interleaved(&mut audio.samples, audio.sample_rate);
                    let result = AudioExporter::export_audio(
                        &save_path,
                        &audio.samples,
                        audio.sample_rate,
                        audio.channels,
                        export_format,
                    );
                    match &result {
                        Ok(()) => {
                            report_progress(
                                1.0,
                                &format!(
                                    "[Export Stems Concluído] Stems gravados na pasta {:?}",
                                    parent
                                ),
                            );
                        }
                        Err(error) => {
                            report_progress(1.0, &format!("[Export Erro] {}", error));
                        }
                    }
                    let _ = export_tx.send(result);
                } else {
                    let render = || {
                        ProjectRenderer::render_project_with_drivers(
                            &project,
                            &active_vb,
                            sample_rate,
                            0.0,
                            resampler_driver.as_ref(),
                            wavtool_driver.as_ref(),
                            &vocal_mode_params,
                            Some(&report_progress),
                        )
                    };
                    let render_pool = rayon::ThreadPoolBuilder::new()
                        .num_threads(render_threads)
                        .thread_name(|index| format!("kamafeu-export-{index}"))
                        .build();
                    let mut audio = match render_pool {
                        Ok(pool) => pool.install(render),
                        Err(_) => render(),
                    };

                    if let Some(error) = audio.error.take() {
                        let _ = export_tx.send(Err(error));
                        return;
                    }
                    fx_proc.process_interleaved(&mut audio.samples, audio.sample_rate);

                    let result = AudioExporter::export_audio(
                        &save_path,
                        &audio.samples,
                        audio.sample_rate,
                        audio.channels,
                        export_format,
                    );
                    match &result {
                        Ok(()) => {
                            report_progress(
                                1.0,
                                &format!(
                                    "[Export Concluído] Áudio gravado com sucesso em {:?}",
                                    save_path
                                ),
                            );
                        }
                        Err(error) => {
                            report_progress(1.0, &format!("[Export Erro] {}", error));
                        }
                    }
                    let _ = export_tx.send(result);
                }
            });
        }
    }

    pub fn export_selected_notes_audio(&mut self) {
        let active_track = self.active_track_index;
        let part_idx = self
            .project
            .parts
            .iter()
            .position(|p| p.track_index == active_track)
            .unwrap_or(0);

        if self.project.parts.is_empty() || part_idx >= self.project.parts.len() {
            self.transport_state.status_message = self.config.language.tr(
                "Nenhuma parte ativa encontrada",
                "No active part found",
            ).to_string();
            return;
        }

        let part = &self.project.parts[part_idx];
        let selected_indices = &self.piano_roll_state.selected_note_indices;
        let single_selected = self.piano_roll_state.selected_note_index;

        let selected_notes: Vec<crate::project::model::UNote> = if !selected_indices.is_empty() {
            let mut indices: Vec<usize> = selected_indices.iter().copied().collect();
            indices.sort_unstable();
            indices
                .into_iter()
                .filter_map(|idx| part.notes.get(idx).cloned())
                .collect()
        } else if let Some(idx) = single_selected {
            if let Some(note) = part.notes.get(idx).cloned() {
                vec![note]
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        if selected_notes.is_empty() {
            self.transport_state.status_message = self.config.language.tr(
                "Selecione uma ou mais notas no piano roll para exportar a demo",
                "Select one or more notes in the piano roll to export demo",
            ).to_string();
            return;
        }

        // Suggested filename: [project] - [voicebank] - wip.wav
        let raw_proj_name = if let Some(ref path) = self.current_project_path {
            path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("project")
        } else {
            let stem = self.project.name.trim();
            if stem.is_empty() {
                "project"
            } else {
                stem
            }
        };

        let raw_vb_name = if let Some(ref vb) = self.voicebank {
            vb.name.trim()
        } else if !self.transport_state.voicebank_name.trim().is_empty() {
            self.transport_state.voicebank_name.trim()
        } else {
            "vocal"
        };

        let clean_name = |s: &str| -> String {
            s.chars()
                .map(|c| if ['/', '\\', ':', '*', '?', '"', '<', '>', '|'].contains(&c) { '_' } else { c })
                .collect::<String>()
                .trim()
                .to_string()
        };

        let clean_proj = clean_name(raw_proj_name);
        let clean_vb = clean_name(raw_vb_name);
        let default_name = format!("{clean_proj} - {clean_vb} - wip.wav");

        let lang = self.config.language;
        if let Some(save_path) = crate::dialogs::FileDialog::new()
            .add_filter(
                lang.tr("Áudio WAV (*.wav)", "WAV Audio (*.wav)"),
                &["wav"],
            )
            .add_filter(
                lang.tr("Áudio FLAC (*.flac)", "FLAC Audio (*.flac)"),
                &["flac"],
            )
            .add_filter(
                lang.tr("Todos os Formatos Suportados (*.wav, *.flac)", "All Supported Formats (*.wav, *.flac)"),
                &["wav", "flac"],
            )
            .set_file_name(&default_name)
            .save_file()
        {
            let min_pos = selected_notes
                .iter()
                .map(|n| n.position_ms)
                .fold(f64::INFINITY, f64::min);
            let mut export_notes = selected_notes;
            for note in &mut export_notes {
                note.position_ms = (note.position_ms - min_pos).max(0.0);
            }

            let export_project = crate::project::model::UProject {
                name: format!("{clean_proj} (WIP Demo)"),
                bpm: self.transport_state.bpm,
                tracks: vec![crate::project::model::UTrack {
                    name: "Vocals".to_string(),
                    ..Default::default()
                }],
                parts: vec![{
                    let mut demo_part = crate::project::model::UVoicePart::new("WIP Demo", 0);
                    demo_part.notes = export_notes;
                    demo_part
                }],
                ..Default::default()
            };

            let resampler_driver = self.create_resampler_driver();
            let wavtool_driver = self.create_wavtool_driver();
            let sample_rate = self.sample_rate;
            let dummy_vb = Voicebank {
                root_path: PathBuf::from("."),
                name: "Synthetic Fallback".to_string(),
                author: "System".to_string(),
                character_info: String::new(),
                readme_info: String::new(),
                image_path: None,
                entries: std::collections::HashMap::new(),
                case_insensitive_entries: Default::default(),
                prefix_map: crate::oto::PrefixMap::default(),
                temp_dir: None,
            };
            let active_vb = self.voicebank.clone().unwrap_or(dummy_vb);
            let vocal_mode_params = self.vocal_mode_params.clone();
            let fx_config = self.fx_rack_config.clone();

            let export_format = if let Some(ext) = save_path.extension().and_then(|e| e.to_str()) {
                match ext.to_ascii_lowercase().as_str() {
                    "flac" => AudioExportFormat::Flac16,
                    "raw" | "pcm" => AudioExportFormat::RawF32,
                    _ => AudioExportFormat::Wav16,
                }
            } else {
                AudioExportFormat::Wav16
            };

            self.export_dialog_open = true;
            self.export_in_progress = true;
            self.export_progress = 0.0;
            self.export_save_path = Some(save_path.clone());
            self.export_status_detail = format!(
                "{} ({})",
                lang.tr("Exportando seleção Demo / WIP", "Exporting WIP / Demo selection"),
                save_path.file_name().unwrap_or_default().to_string_lossy()
            );

            let (tx, rx) = std::sync::mpsc::channel();
            self.render_log_channel_rx = Some(rx);
            let (export_tx, export_rx) = std::sync::mpsc::channel();
            self.export_rx = Some(export_rx);

            let tx = Arc::new(std::sync::Mutex::new(tx));
            let tx_cb = tx.clone();

            std::thread::spawn(move || {
                let report_progress = move |prog, msg: &str| {
                    if let Ok(guard) = tx_cb.lock() {
                        let _ = guard.send((prog, msg.to_string()));
                    }
                };

                report_progress(0.1, "[WIP Demo] Renderizando notas selecionadas...");

                let mut audio = ProjectRenderer::render_project_with_drivers(
                    &export_project,
                    &active_vb,
                    sample_rate,
                    0.0,
                    resampler_driver.as_ref(),
                    wavtool_driver.as_ref(),
                    &vocal_mode_params,
                    None,
                );

                if let Some(error) = audio.error.take() {
                    let _ = export_tx.send(Err(error));
                    return;
                }

                report_progress(0.8, "[WIP Demo] Processando efeitos FX...");
                let mut fx_proc = crate::audio::FxRackProcessor::new(fx_config, sample_rate);
                fx_proc.process_interleaved(&mut audio.samples, audio.sample_rate);

                report_progress(0.9, "[WIP Demo] Gravando arquivo de áudio...");
                let result = AudioExporter::export_audio(
                    &save_path,
                    &audio.samples,
                    audio.sample_rate,
                    audio.channels,
                    export_format,
                );

                match &result {
                    Ok(()) => {
                        report_progress(
                            1.0,
                            &format!(
                                "[Demo WIP Exportada com Sucesso] Arquivo salvo em {:?}",
                                save_path
                            ),
                        );
                    }
                    Err(error) => {
                        report_progress(1.0, &format!("[Export Erro] {}", error));
                    }
                }
                let _ = export_tx.send(result);
            });
        }
    }
}
