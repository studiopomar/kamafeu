use crate::formats::ApsFormat;
use crate::formats::MidiFormat;
use crate::formats::SvpFormat;
use crate::formats::UfdataFormat;
use crate::formats::UstFormat;
use crate::formats::UstxFormat;
use crate::formats::VsqxFormat;
use crate::gui::history::UndoManager;
use crate::gui::KamafeuStudioApp;
use crate::oto::Voicebank;
use std::path::Path;
use std::path::PathBuf;

impl KamafeuStudioApp {
    pub fn create_project_snapshot(&mut self) {
        let snapshot_dir = PathBuf::from(".kamafeu_snapshots");
        let _ = std::fs::create_dir_all(&snapshot_dir);
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let filename = format!("snapshot_{}.kamafeu", secs);
        let path = snapshot_dir.join(filename);
        if let Ok(json) = serde_json::to_string_pretty(&self.project) {
            if std::fs::write(&path, json).is_ok() {
                let time_str = format!("timestamp {}", secs);
                self.last_snapshot_time = Some(time_str.clone());
                self.transport_state.status_message =
                    format!("Snapshot salvo com sucesso ({})", time_str);
            }
        }
    }

    pub fn reveal_project_in_finder(&self) {
        if let Some(ref path) = self.current_project_path {
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("open")
                    .arg("-R")
                    .arg(path)
                    .spawn();
            }
            #[cfg(target_os = "windows")]
            {
                let _ = std::process::Command::new("explorer")
                    .arg(format!("/select,\"{}\"", path.display()))
                    .spawn();
            }
            #[cfg(target_os = "linux")]
            {
                if let Some(parent) = path.parent() {
                    let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
                }
            }
        } else {
            #[cfg(target_os = "macos")]
            let _ = std::process::Command::new("open").arg(".").spawn();
            #[cfg(target_os = "windows")]
            let _ = std::process::Command::new("explorer").arg(".").spawn();
            #[cfg(target_os = "linux")]
            let _ = std::process::Command::new("xdg-open").arg(".").spawn();
        }
    }

    pub fn load_project_template(&mut self, template_name: &str) {
        self.push_history();
        let mut p = crate::project::model::UProject::default();
        match template_name {
            "pop" => {
                p.name = "Novo Projeto Pop".to_string();
                p.bpm = 120.0;
                p.tracks = vec![
                    crate::project::model::UTrack {
                        name: "Vocal Principal (Lead)".to_string(),
                        singer: "Default Singer".to_string(),
                        volume_db: 0.0,
                        pan: 0.0,
                        ..Default::default()
                    },
                    crate::project::model::UTrack {
                        name: "Vocal Guia (Chords)".to_string(),
                        singer: "Default Singer".to_string(),
                        volume_db: -3.0,
                        pan: 0.0,
                        ..Default::default()
                    },
                ];
                p.parts = vec![
                    crate::project::model::UVoicePart::new("Lead Part", 0),
                    crate::project::model::UVoicePart::new("Guide Part", 1),
                ];
                self.piano_roll_state.active_scale =
                    crate::gui::piano_roll::state::MusicalScale::Major;
                self.piano_roll_state.scale_root_key = 0;
            }
            "ballad" => {
                p.name = "Nova Balada Acústica".to_string();
                p.bpm = 80.0;
                p.tracks = vec![
                    crate::project::model::UTrack {
                        name: "Vocal Principal".to_string(),
                        singer: "Default Singer".to_string(),
                        volume_db: 0.0,
                        pan: 0.0,
                        ..Default::default()
                    },
                    crate::project::model::UTrack {
                        name: "Harmonia Suave".to_string(),
                        singer: "Default Singer".to_string(),
                        volume_db: -4.0,
                        pan: -0.2,
                        ..Default::default()
                    },
                ];
                p.parts = vec![
                    crate::project::model::UVoicePart::new("Lead Part", 0),
                    crate::project::model::UVoicePart::new("Harm Part", 1),
                ];
                self.piano_roll_state.active_scale =
                    crate::gui::piano_roll::state::MusicalScale::NaturalMinor;
                self.piano_roll_state.scale_root_key = 9;
            }
            "rock" => {
                p.name = "Novo Projeto Rock".to_string();
                p.bpm = 160.0;
                p.tracks = vec![
                    crate::project::model::UTrack {
                        name: "Lead Vocal (Drive)".to_string(),
                        singer: "Default Singer".to_string(),
                        volume_db: 0.0,
                        pan: 0.0,
                        ..Default::default()
                    },
                    crate::project::model::UTrack {
                        name: "Backing Vocal L".to_string(),
                        singer: "Default Singer".to_string(),
                        volume_db: -3.5,
                        pan: -0.6,
                        ..Default::default()
                    },
                    crate::project::model::UTrack {
                        name: "Backing Vocal R".to_string(),
                        singer: "Default Singer".to_string(),
                        volume_db: -3.5,
                        pan: 0.6,
                        ..Default::default()
                    },
                ];
                p.parts = vec![
                    crate::project::model::UVoicePart::new("Lead Part", 0),
                    crate::project::model::UVoicePart::new("Back L Part", 1),
                    crate::project::model::UVoicePart::new("Back R Part", 2),
                ];
                self.piano_roll_state.active_scale =
                    crate::gui::piano_roll::state::MusicalScale::NaturalMinor;
                self.piano_roll_state.scale_root_key = 4;
            }
            "choir" => {
                p.name = "Novo Arranjo Coral".to_string();
                p.bpm = 100.0;
                p.tracks = vec![
                    crate::project::model::UTrack {
                        name: "Soprano".to_string(),
                        volume_db: 0.0,
                        pan: -0.4,
                        ..Default::default()
                    },
                    crate::project::model::UTrack {
                        name: "Alto".to_string(),
                        volume_db: 0.0,
                        pan: -0.15,
                        ..Default::default()
                    },
                    crate::project::model::UTrack {
                        name: "Tenor".to_string(),
                        volume_db: 0.0,
                        pan: 0.15,
                        ..Default::default()
                    },
                    crate::project::model::UTrack {
                        name: "Baixo".to_string(),
                        volume_db: 0.0,
                        pan: 0.4,
                        ..Default::default()
                    },
                ];
                p.parts = vec![
                    crate::project::model::UVoicePart::new("Soprano Part", 0),
                    crate::project::model::UVoicePart::new("Alto Part", 1),
                    crate::project::model::UVoicePart::new("Tenor Part", 2),
                    crate::project::model::UVoicePart::new("Bass Part", 3),
                ];
            }
            _ => {
                p.name = "Novo Projeto".to_string();
                p.bpm = 120.0;
                p.tracks = vec![crate::project::model::UTrack::default()];
                p.parts = vec![crate::project::model::UVoicePart::new("Part 1", 0)];
            }
        }
        self.project = p;
        self.transport_state.bpm = self.project.bpm;
        self.current_project_path = None;
        self.piano_roll_state.selected_note_index = None;
        self.piano_roll_state.selected_note_indices.clear();
        self.is_dirty = false;
        self.transport_state.status_message =
            format!("Modelo '{}' carregado com sucesso", template_name);
    }

    pub fn new_project(&mut self) {
        self.audio_player.stop();
        self.project = crate::project::model::UProject::default();
        self.current_project_path = None;
        self.transport_state.bpm = self.project.bpm;
        self.piano_roll_state.playhead_ms = 0.0;
        self.piano_roll_state.selected_note_index = None;
        self.piano_roll_state.selected_note_indices.clear();
        self.piano_roll_state.initial_scrolled = false;
        self.undo_manager = UndoManager::default();
        self.is_dirty = false;
        self.transport_state.status_message = "Novo projeto criado".to_string();
    }

    pub fn open_project_from_path(&mut self, path: &Path) {
        let extension = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();

        if matches!(extension.as_str(), "wav" | "mp3" | "ogg" | "flac") {
            self.import_audio_track_from_path(path);
            return;
        }

        let loaded = match extension.as_str() {
            "aps" => ApsFormat::load_file(path)
                .or_else(|_| UstxFormat::load_file(path))
                .or_else(|_| UfdataFormat::load_file(path)),
            "mid" | "midi" => MidiFormat::load_file(path).or_else(|_| VsqxFormat::load_file(path)),
            "ust" => UstFormat::load_file(path).or_else(|_| UstxFormat::load_file(path)),
            "ustx" => UstxFormat::load_file(path)
                .or_else(|_| ApsFormat::load_file(path))
                .or_else(|_| UstFormat::load_file(path)),
            "ufdata" => UfdataFormat::load_file(path)
                .or_else(|_| SvpFormat::load_file(path))
                .or_else(|_| ApsFormat::load_file(path)),
            "svp" => SvpFormat::load_file(path)
                .or_else(|_| UfdataFormat::load_file(path))
                .or_else(|_| ApsFormat::load_file(path)),
            "vsqx" | "vsq" => VsqxFormat::load_file(path)
                .or_else(|_| MidiFormat::load_file(path))
                .or_else(|_| UfdataFormat::load_file(path)),
            "json" => UfdataFormat::load_file(path)
                .or_else(|_| SvpFormat::load_file(path))
                .or_else(|_| ApsFormat::load_file(path))
                .or_else(|_| UstxFormat::load_file(path)),
            _ => ApsFormat::load_file(path)
                .or_else(|_| SvpFormat::load_file(path))
                .or_else(|_| UstxFormat::load_file(path))
                .or_else(|_| UfdataFormat::load_file(path))
                .or_else(|_| UstFormat::load_file(path))
                .or_else(|_| VsqxFormat::load_file(path))
                .or_else(|_| MidiFormat::load_file(path)),
        };
        match loaded {
            Ok(mut proj) => {
                proj.normalize();
                self.audio_player.stop();
                self.project = proj;
                self.transport_state.bpm = self.project.bpm;
                self.piano_roll_state.selected_note_index = None;
                self.piano_roll_state.selected_note_indices.clear();
                self.undo_manager = UndoManager::default();
                self.is_dirty = false;

                if extension == "aps" {
                    self.current_project_path = Some(path.to_path_buf());
                } else {
                    self.current_project_path = None;
                }

                if let Some(mode) = self
                    .project
                    .phonemizer
                    .or_else(|| self.project.tracks.first().and_then(|t| t.phonemizer))
                {
                    self.vocal_mode_params.phonemizer_mode = mode;
                }

                let vb_target = self
                    .project
                    .voicebank_path
                    .as_deref()
                    .or(self.project.voicebank.as_deref())
                    .or_else(|| {
                        self.project
                            .tracks
                            .first()
                            .and_then(|t| t.voicebank_path.as_deref())
                    })
                    .or_else(|| self.project.tracks.first().map(|t| t.singer.as_str()));

                if let Some(target) = vb_target {
                    if let Some(found_path) = crate::oto::SingerScanner::find_singer_by_name_or_path(
                        target,
                        &self.config.recent_voicebanks,
                        &self.config.singers_paths,
                    ) {
                        if let Ok(vb) = Voicebank::new(&found_path) {
                            self.transport_state.voicebank_name = vb.name.clone();
                            self.config.add_recent_voicebank(vb.root_path.clone());
                            self.voicebank = Some(vb);
                        }
                    }
                }

                if let Some(ref resampler) = self.project.resampler.clone().or_else(|| {
                    self.project
                        .tracks
                        .first()
                        .and_then(|t| t.resampler.clone())
                }) {
                    self.selected_resampler = resampler.clone();
                }

                if let Some(ref wavtool) = self
                    .project
                    .wavtool
                    .clone()
                    .or_else(|| self.project.tracks.first().and_then(|t| t.wavtool.clone()))
                {
                    self.selected_wavtool = wavtool.clone();
                }

                if let Some(sr) = self.project.sample_rate {
                    self.sample_rate = sr;
                }
                if let Some(threads) = self.project.render_threads {
                    self.render_threads = threads;
                }

                let first_pos = self
                    .project
                    .parts
                    .iter()
                    .flat_map(|p| p.notes.iter())
                    .map(|n| n.position_ms)
                    .fold(f64::INFINITY, f64::min);

                if first_pos.is_finite() && first_pos > 0.0 {
                    self.piano_roll_state.playhead_ms = first_pos;
                } else {
                    self.piano_roll_state.playhead_ms = 0.0;
                }

                self.piano_roll_state.initial_scrolled = false;
                self.config.add_recent_project(path.to_path_buf());
                self.transport_state.status_message =
                    format!("Projeto aberto: {:?}", path.file_name().unwrap_or_default());
            }
            Err(e) => {
                self.transport_state.status_message = format!("Erro ao abrir projeto: {}", e);
            }
        }
    }

    pub fn open_project_dialog(&mut self) {
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Abrir Projeto / Importar Formato")
            .add_filter(
                "Todos os Formatos Suportados (*.aps, *.ustx, *.ust, *.ufdata, *.svp, *.vsqx, *.vsq, *.mid, *.json)",
                &[
                    "aps", "APS",
                    "ustx", "USTX",
                    "ust", "UST",
                    "ufdata", "UFDATA",
                    "svp", "SVP",
                    "vsqx", "VSQX",
                    "vsq", "VSQ",
                    "mid", "MID",
                    "midi", "MIDI",
                    "json", "JSON",
                ],
            )
            .add_filter("Arquivo Projeto Saturno (*.aps)", &["aps", "APS"])
            .add_filter("UtaFormatix Data (*.ufdata, *.json)", &["ufdata", "UFDATA", "json", "JSON"])
            .add_filter("Projeto OpenUTAU (*.ustx)", &["ustx", "USTX"])
            .add_filter("Sequência UTAU (*.ust)", &["ust", "UST"])
            .add_filter("Projeto Synthesizer V (*.svp)", &["svp", "SVP"])
            .add_filter("Sequência Vocaloid (*.vsqx, *.vsq)", &["vsqx", "VSQX", "vsq", "VSQ"])
            .add_filter("Arquivo MIDI Padrão (*.mid, *.midi)", &["mid", "MID", "midi", "MIDI"])
            .pick_file()
        {
            self.open_project_from_path(&path);
        }
    }

    pub(super) fn sync_project_meta_before_save(&mut self) {
        self.project.bpm = self.transport_state.bpm;
        if self.project.name.trim().is_empty() || self.project.name == "Novo Projeto" {
            if let Some(ref path) = self.current_project_path {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    self.project.name = stem.to_string();
                }
            }
        }
        self.project.phonemizer = Some(self.vocal_mode_params.phonemizer_mode);
        self.project.resampler = Some(self.selected_resampler.clone());
        self.project.wavtool = Some(self.selected_wavtool.clone());
        self.project.sample_rate = Some(self.sample_rate);
        self.project.render_threads = Some(self.render_threads);

        if let Some(ref vb) = self.voicebank {
            self.project.voicebank = Some(vb.name.clone());
            self.project.voicebank_path = Some(vb.root_path.to_string_lossy().to_string());
        }

        for track in &mut self.project.tracks {
            track.phonemizer = Some(self.vocal_mode_params.phonemizer_mode);
            track.resampler = Some(self.selected_resampler.clone());
            track.wavtool = Some(self.selected_wavtool.clone());
            if let Some(ref vb) = self.voicebank {
                if track.singer.is_empty()
                    || track.singer == "Default Singer"
                    || track.singer == "Cantor Padrão"
                {
                    track.singer = vb.name.clone();
                }
                track.voicebank_path = Some(vb.root_path.to_string_lossy().to_string());
            }
        }
    }

    pub fn compute_next_incremental_path(current: &std::path::Path) -> std::path::PathBuf {
        let parent = current
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));
        let ext = current
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("aps");
        let stem = current
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("projeto");

        let (base_stem, start_num) = if let Some(idx) = stem.rfind("_v") {
            let suffix = &stem[idx + 2..];
            if let Ok(num) = suffix.parse::<u32>() {
                (&stem[..idx], num + 1)
            } else {
                (stem, 2)
            }
        } else {
            (stem, 2)
        };

        let mut candidate_num = start_num;
        loop {
            let candidate_name = format!("{}_v{}.{}", base_stem, candidate_num, ext);
            let candidate_path = parent.join(&candidate_name);
            if !candidate_path.exists() {
                return candidate_path;
            }
            candidate_num += 1;
        }
    }

    pub fn save_project_incremental_version(&mut self) {
        self.sync_project_meta_before_save();
        let next_path = if let Some(ref current) = self.current_project_path {
            Self::compute_next_incremental_path(current)
        } else {
            let stem = self.project.name.trim();
            let base = if stem.is_empty() || stem == "Novo Projeto" {
                "projeto".to_string()
            } else {
                stem.to_string()
            };
            std::path::PathBuf::from(format!("{}_v1.aps", base))
        };

        if let Err(e) = ApsFormat::save_file(&self.project, &next_path) {
            self.transport_state.status_message =
                format!("Erro ao salvar versão incremental: {}", e);
        } else {
            self.current_project_path = Some(next_path.clone());
            if let Some(stem) = next_path.file_stem().and_then(|s| s.to_str()) {
                self.project.name = stem.to_string();
            }
            self.is_dirty = false;
            self.config.add_recent_project(next_path.clone());
            self.persist_config();
            let name_str = next_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("arquivo.aps");
            self.transport_state.status_message =
                format!("Nova versão incremental salva: {}", name_str);
        }
    }

    pub fn save_project_backup_copy(&mut self) {
        self.sync_project_meta_before_save();
        let parent = self
            .current_project_path
            .as_ref()
            .and_then(|p| p.parent().map(|d| d.to_path_buf()))
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        let stem = self
            .current_project_path
            .as_ref()
            .and_then(|p| p.file_stem().and_then(|s| s.to_str()))
            .unwrap_or_else(|| self.project.name.trim());
        let stem_clean = if stem.is_empty() || stem == "Novo Projeto" {
            "projeto"
        } else {
            stem
        };

        let backup_dir = parent.join(".kamafeu_backups");
        let _ = std::fs::create_dir_all(&backup_dir);

        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let backup_filename = format!("{}_backup_{}.aps", stem_clean, secs);
        let backup_path = backup_dir.join(&backup_filename);

        if let Err(e) = ApsFormat::save_file(&self.project, &backup_path) {
            self.transport_state.status_message = format!("Erro ao salvar cópia de backup: {}", e);
        } else {
            self.transport_state.status_message = format!(
                "Cópia de backup salva em: .kamafeu_backups/{}",
                backup_filename
            );
        }
    }

    pub fn save_project(&mut self) {
        self.sync_project_meta_before_save();
        if let Some(ref path) = self.current_project_path.clone() {
            if path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase()
                == "aps"
            {
                if let Err(e) = ApsFormat::save_file(&self.project, path) {
                    self.transport_state.status_message = format!("Erro ao salvar projeto: {}", e);
                } else {
                    self.is_dirty = false;
                    self.config.add_recent_project(path.clone());
                    self.transport_state.status_message =
                        format!("Projeto salvo: {:?}", path.file_name().unwrap_or_default());
                }
                return;
            }
        }
        self.save_project_as_dialog();
    }

    pub fn save_project_as_dialog(&mut self) {
        self.sync_project_meta_before_save();
        let default_name = if let Some(ref p) = self.current_project_path {
            p.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("project.aps")
                .to_string()
        } else {
            let stem = self.project.name.trim();
            if stem.is_empty() || stem == "Novo Projeto" {
                "project.aps".to_string()
            } else {
                format!("{}.aps", stem)
            }
        };

        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Salvar Projeto Como")
            .set_file_name(&default_name)
            .add_filter("Arquivo Projeto Saturno (*.aps)", &["aps"])
            .add_filter("UtaFormatix Data (*.ufdata)", &["ufdata"])
            .add_filter("Projeto OpenUTAU (*.ustx)", &["ustx"])
            .add_filter("Sequência UTAU (*.ust)", &["ust"])
            .add_filter("Projeto Synthesizer V (*.svp)", &["svp"])
            .add_filter("Sequência Vocaloid (*.vsqx)", &["vsqx"])
            .add_filter("Arquivo MIDI Padrão (*.mid)", &["mid", "midi"])
            .save_file()
        {
            let extension = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            let saved = match extension.as_str() {
                "aps" => ApsFormat::save_file(&self.project, &path),
                "ufdata" | "json" => UfdataFormat::save_file(&self.project, &path),
                "mid" | "midi" => MidiFormat::save_file(&self.project, &path),
                "ust" => UstFormat::save_file(&self.project, &path),
                "ustx" => UstxFormat::save_file(&self.project, &path),
                "svp" => SvpFormat::save_file(&self.project, &path),
                "vsqx" | "vsq" => VsqxFormat::save_file(&self.project, &path),
                _ => ApsFormat::save_file(&self.project, &path),
            };
            match saved {
                Ok(()) => {
                    self.is_dirty = false;
                    if extension == "aps" {
                        self.current_project_path = Some(path.clone());
                    }
                    self.config.add_recent_project(path.clone());
                    self.transport_state.status_message = format!(
                        "Projeto salvo com sucesso: {:?}",
                        path.file_name().unwrap_or_default()
                    );
                }
                Err(e) => {
                    self.transport_state.status_message = format!("Erro ao salvar projeto: {}", e);
                }
            }
        }
    }
}
