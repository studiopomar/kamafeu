use crate::gui::KamafeuStudioApp;
use crate::oto::Voicebank;
use web_time::Instant;

impl KamafeuStudioApp {
    /// Makes every voicebank entry point restore the singer-specific render
    /// profile (including phonemizer), invalidate aliases, and persist the
    /// selection consistently.
    pub(crate) fn activate_voicebank(&mut self, voicebank: Voicebank) {
        let lang = self.config.language;
        self.transport_state.status_message = if lang.is_en() {
            format!("Voicebank Loaded: {}", voicebank.name)
        } else {
            format!("Voicebank Carregado: {}", voicebank.name)
        };
        self.transport_state.voicebank_name = voicebank.name.clone();
        self.transport_state.voicebank_path = Some(voicebank.root_path.clone());
        self.config
            .add_recent_voicebank(voicebank.root_path.clone());
        self.config.last_voicebank = Some(voicebank.root_path.clone());

        if let Some(track) = self.project.tracks.get_mut(self.active_track_index) {
            track.singer = voicebank.name.clone();
            track.voicebank_path = Some(voicebank.root_path.display().to_string());
            track.native_voicebank = None;
            track.diffsinger_language = None;
            track.diffsinger_speaker = None;
            track.diffsinger_phonemizer = None;
            track.diffsinger_provider = None;
        }
        self.project.voicebank = Some(voicebank.name.clone());
        self.project.voicebank_path = Some(voicebank.root_path.display().to_string());
        self.project.render_state = Some(Default::default());
        self.is_dirty = true;

        self.voicebank = Some(voicebank);
        self.restore_render_profile_for_active_context();
        self.piano_roll_state.phoneme_cache_hash = 0;
        self.piano_roll_state.phoneme_cache.clear();
        self.piano_roll_state.note_phonemes_cache.clear();
        self.persist_config();
    }

    pub(crate) fn activate_singer_path(&mut self, path: &std::path::Path) {
        let is_diffsinger = path.join("character.yaml").is_file()
            && (path.join("dsconfig.yaml").is_file() || path.join("configs/dsconfig.yaml").is_file());

        let is_native_kfv2 = path.join("cantor.toml").is_file()
            || (path.join("kamafeu_voicebank.json").is_file() && {
                std::fs::read_to_string(path.join("kamafeu_voicebank.json"))
                    .ok()
                    .and_then(|c| serde_json::from_str::<serde_json::Value>(&c).ok())
                    .and_then(|h| h.get("format").and_then(|v| v.as_str()).map(|s| s == "kamafeu.voicebank"))
                    .unwrap_or(false)
            });

        if is_diffsinger {
            match crate::voicebank::diffsinger::DiffSingerBank::inspect(path) {
                Ok(bank) => {
                    let bank_name = bank.name.clone();
                    if let Some(track) = self.project.tracks.get_mut(self.active_track_index) {
                        track.singer = bank_name.clone();
                        track.voicebank_path = Some(path.display().to_string());
                        track.native_voicebank = None;
                        track.diffsinger_language = bank
                            .language_ids
                            .contains_key("pt")
                            .then(|| "pt".into())
                            .or_else(|| bank.language_ids.contains_key("en").then(|| "en".into()))
                            .or_else(|| bank.languages.first().cloned());
                        track.diffsinger_speaker = bank.speakers.first().cloned();
                        track.diffsinger_phonemizer = Some(if bank.language_ids.contains_key("pt") {
                            crate::voicebank::diffsinger::DiffSingerPhonemizer::BrapaPhonetic
                        } else {
                            crate::voicebank::diffsinger::DiffSingerPhonemizer::Automatic
                        });
                        track.diffsinger_provider =
                            Some(crate::voicebank::diffsinger::ExecutionProvider::Cpu);
                    }
                    self.voicebank = None;
                    self.transport_state.voicebank_name = bank_name.clone();
                    self.transport_state.voicebank_path = Some(path.to_path_buf());
                    let lang = self.config.language;
                    self.transport_state.status_message = if lang.is_en() {
                        format!("DiffSinger Loaded: {}", bank_name)
                    } else {
                        format!("DiffSinger Carregado: {}", bank_name)
                    };
                    self.project.voicebank = Some(bank_name);
                    self.project.voicebank_path = Some(path.display().to_string());
                    self.project.render_state = Some(Default::default());
                    self.is_dirty = true;
                    self.config.add_recent_voicebank(path.to_path_buf());
                    self.config.last_voicebank = Some(path.to_path_buf());
                    self.piano_roll_state.show_phoneme_ruler = true;
                    self.config.layout.show_phoneme_ruler = true;
                    self.piano_roll_state.phoneme_cache_hash = 0;
                    self.piano_roll_state.phoneme_cache.clear();
                    self.piano_roll_state.note_phonemes_cache.clear();
                    self.persist_config();
                }
                Err(err) => {
                    let lang = self.config.language;
                    self.transport_state.status_message = format!(
                        "{}: {}",
                        lang.tr(
                            "Não foi possível carregar o banco DiffSinger",
                            "Could not load DiffSinger bank"
                        ),
                        err
                    );
                }
            }
        } else if is_native_kfv2 {
            match crate::voicebank::native::NativeBank::load_path(path).and_then(|bank| {
                let reference = bank.project_reference(Some(path.display().to_string()))?;
                let coverage =
                    bank.coverage_summary(&reference.phonetic_profile_id, &reference.style_id)?;
                Ok((bank.manifest.display_name.clone(), reference, coverage))
            }) {
                Ok((display_name, reference, coverage)) => {
                    let track_name = self
                        .project
                        .tracks
                        .get(self.active_track_index)
                        .map(|t| t.name.clone())
                        .unwrap_or_default();
                    if let Some(track) = self.project.tracks.get_mut(self.active_track_index) {
                        track.native_voicebank = Some(reference);
                        track.voicebank_path = Some(path.display().to_string());
                        track.singer = display_name.clone();
                        track.phonemizer = Some(crate::phonemizer::PhonemizerMode::None);
                        track.diffsinger_language = None;
                        track.diffsinger_speaker = None;
                        track.diffsinger_phonemizer = None;
                        track.diffsinger_provider = None;
                    }
                    self.voicebank = None;
                    self.transport_state.voicebank_name = display_name.clone();
                    self.transport_state.voicebank_path = Some(path.to_path_buf());
                    self.transport_state.status_message = format!(
                        "{} vinculada à faixa {} · cobertura estrutural: {}/{} phones, {} unidades, {} revisadas · preview SOLA experimental",
                        display_name,
                        track_name,
                        coverage.covered_phone_symbols,
                        coverage.total_phone_symbols,
                        coverage.unit_count,
                        coverage.reviewed_unit_count,
                    );
                    self.project.voicebank = Some(display_name);
                    self.project.voicebank_path = Some(path.display().to_string());
                    self.project.render_state = Some(Default::default());
                    self.is_dirty = true;
                    self.config.add_recent_voicebank(path.to_path_buf());
                    self.config.last_voicebank = Some(path.to_path_buf());
                    self.piano_roll_state.phoneme_cache_hash = 0;
                    self.piano_roll_state.phoneme_cache.clear();
                    self.piano_roll_state.note_phonemes_cache.clear();
                    self.persist_config();
                }
                Err(err) => {
                    let lang = self.config.language;
                    self.transport_state.status_message = format!(
                        "{}: {}",
                        lang.tr(
                            "Não foi possível vincular o voicebank nativo",
                            "Could not bind native voicebank"
                        ),
                        err
                    );
                }
            }
        } else {
            match crate::oto::Voicebank::new(path) {
                Ok(vb) => {
                    self.activate_voicebank(vb);
                }
                Err(err) => {
                    let lang = self.config.language;
                    self.transport_state.status_message = format!(
                        "{}: {}",
                        lang.tr(
                            "Não foi possível usar o voicebank",
                            "Could not use voicebank"
                        ),
                        err
                    );
                }
            }
        }
    }

    pub fn reload_singers(&mut self) {
        crate::voicebank::diffsinger::DiffSingerBank::clear_inspection_cache();
        let mut directories = self.config.singers_paths.clone();
        if let Some(parent) = self
            .config
            .last_voicebank
            .as_ref()
            .and_then(|path| path.parent())
            .map(std::path::Path::to_path_buf)
        {
            if !directories.contains(&parent) {
                directories.push(parent);
            }
        }
        self.singers_list = crate::oto::SingerScanner::scan_directories(&directories);
    }

    pub(super) fn refresh_voicebank_oto(&mut self) {
        if self.last_voicebank_oto_check.elapsed() < std::time::Duration::from_millis(350) {
            return;
        }
        self.last_voicebank_oto_check = Instant::now();

        let Some(root_path) = self
            .voicebank
            .as_ref()
            .map(|voicebank| voicebank.root_path.clone())
        else {
            return;
        };
        let Ok(signature) = crate::copaiba_bridge::oto_signature(&root_path) else {
            return;
        };
        if self.voicebank_oto_signature.is_none() {
            self.voicebank_oto_signature = Some(signature);
            return;
        }
        if self.voicebank_oto_signature == Some(signature) {
            return;
        }

        if let Ok(mut voicebank) = Voicebank::new(&root_path) {
            if let Some(previous) = self.voicebank.as_ref() {
                voicebank
                    .prefix_map
                    .select_color(previous.prefix_map.selected_color());
            }
            self.transport_state.voicebank_name = voicebank.name.clone();
            self.transport_state.voicebank_path = Some(voicebank.root_path.clone());
            self.voicebank = Some(voicebank);
            self.voicebank_oto_signature = Some(signature);
            self.piano_roll_state.phoneme_cache_hash = 0;
            self.piano_roll_state.phoneme_cache.clear();
            self.piano_roll_state.note_phonemes_cache.clear();
            self.transport_state.status_message = "oto.ini atualizado pelo Copaiba".to_string();
        }
    }

    pub(super) fn open_copaiba_for_alias(&mut self, requested_alias: &str, pitch: &str) {
        if requested_alias.trim().is_empty() || requested_alias.trim() == "+" {
            return;
        }
        if self
            .project
            .tracks
            .get(self.active_track_index)
            .and_then(|track| track.native_voicebank.as_ref())
            .is_some()
        {
            self.open_kamuse_for_active_native_bank();
            return;
        }
        let Some(voicebank) = self.voicebank.as_ref() else {
            self.transport_state.status_message =
                "Carregue um voicebank para editar o oto.ini.".to_string();
            return;
        };
        let alias = voicebank
            .entries
            .keys()
            .find(|alias| alias.trim().eq_ignore_ascii_case(requested_alias.trim()))
            .cloned()
            .or_else(|| {
                voicebank
                    .find_entry(requested_alias, pitch)
                    .map(|entry| entry.alias.clone())
            })
            .unwrap_or_else(|| requested_alias.trim().to_string());
        // Use the embedded Copaiba editor so the selected phoneme, its WAV and
        // all oto.ini timing fields are immediately editable. The external NEO
        // launcher remains available to callers that explicitly need it.
        self.copaiba_app
            .open_alias(voicebank.root_path.clone(), &alias);
        self.copaiba_window_open = true;
        self.transport_state.status_message =
            format!("Copaiba aberto no alias {alias}; ajuste os parâmetros e clique em Salvar.");
    }

    fn open_kamuse_for_active_native_bank(&mut self) {
        let path = self
            .project
            .tracks
            .get(self.active_track_index)
            .and_then(|track| track.native_voicebank.as_ref())
            .and_then(|reference| reference.source_path_hint.as_deref())
            .map(std::path::PathBuf::from);
        let Some(path) = path else {
            self.transport_state.status_message =
                "A faixa ativa não tem um caminho de origem para o banco nativo.".into();
            return;
        };
        match crate::copaiba::kamuse::KamuseProject::load(&path) {
            Ok(project) => {
                let name = project.cantor.nome.clone();
                let source = project
                    .cantor
                    .tons
                    .first()
                    .and_then(|tone| tone.gravacoes.first())
                    .map(|recording| recording.arquivo.clone())
                    .unwrap_or_else(|| "sem gravação".into());
                self.copaiba_app.activate_kamuse_project(project);
                self.copaiba_window_open = true;
                self.transport_state.status_message =
                    format!("KAMUSE abriu {name} · fonte selecionada: {source}");
            }
            Err(error) => {
                self.transport_state.status_message =
                    format!("Não foi possível abrir o banco no KAMUSE: {error}");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn test_app() -> KamafeuStudioApp {
        let mut config = crate::config::KamafeuConfig::default();
        config.singers_paths.clear();
        let mut project = crate::project::model::UProject::default();
        project.tracks = vec![crate::project::model::UTrack::default()];
        KamafeuStudioApp {
            project,
            current_project_path: None,
            voicebank: None,
            voicebank_oto_signature: None,
            last_voicebank_oto_check: Instant::now(),
            piano_roll_state: crate::gui::piano_roll::PianoRollState::default(),
            transport_state: crate::gui::types::TransportState::default(),
            right_sidebar_tab: crate::gui::types::RightSidebarTab::SingerTrack,
            vocal_mode_params: crate::renderer::RenderOptions::default(),
            imported_phoneme_policy: Default::default(),
            phoneme_palette_state: crate::gui::phoneme_palette::PhonemePaletteState::default(),
            undo_manager: crate::gui::history::UndoManager::default(),
            pending_edit_snapshot: None,
            clipboard: Vec::new(),
            audio_player: crate::audio::AudioPlayer::new(),
            sample_rate: 44100,
            render_threads: 2,
            selected_resampler: "world".into(),
            selected_wavtool: "wavtool".into(),
            custom_resampler_path: None,
            custom_wavtool_path: None,
            playback_start_instant: None,
            playback_start_offset_ms: 0.0,
            playback_duration_ms: 0.0,
            render_rx: None,
            direct_playback_rx: None,
            direct_playback_active: false,
            render_cancel: None,
            retry_project: None,
            retry_voicebank: None,
            retry_options: None,
            failed_chunk: None,
            progressive_playback_started: false,
            progressive_startup_audio: Vec::new(),
            playback_chunk_buffer: Default::default(),
            render_log_window_open: false,
            render_log_messages: Vec::new(),
            render_log_filter: crate::gui::RenderLogFilter::All,
            render_log_search_query: String::new(),
            render_log_font_size: 11.5,
            render_log_show_line_numbers: true,
            render_log_wrap_lines: true,
            auto_scroll_log: true,
            render_progress: 1.0,
            render_status_title: "Pronto".into(),
            render_log_channel_rx: None,
            render_progress_event: None,
            render_progress_rx: None,
            render_task_kind: None,
            render_progress_started_at: None,
            render_progress_finished_at: None,
            export_rx: None,
            export_cancel: None,
            active_track_index: 0,
            config,
            singers_list: Vec::new(),
            singer_search_query: String::new(),
            singers_gallery_window_open: false,
            discord_rpc: crate::discord_rpc::DiscordRpcManager::new(),
            copaiba_app: crate::copaiba::gui::CopaibaToolkitApp::default(),
            copaiba_window_open: false,
            packages_window_open: false,
            packages_target_os: 0,
            packages_search: String::new(),
            preferences_window_open: false,
            preferences_tab: 0,
            preferences_state: Default::default(),
            theme_customizer_open: false,
            theme_customizer_tab: 0,
            saved_theme_selected: None,
            saved_theme_name_draft: String::new(),
            pending_theme_import: None,
            pending_theme_delete: None,
            shortcuts_guide_open: false,
            command_palette_open: false,
            command_palette_query: String::new(),
            command_palette_selected: 0,
            autopitch_window_open: false,
            autopitch_options: Default::default(),
            autopitch_scope: crate::dsp::AutoPitchScope::SelectedOnly,
            batch_lyrics_open: false,
            batch_lyrics_buffer: String::new(),
            last_window_title: String::new(),
            startup_maximize_requested: false,
            export_options_dialog_open: false,
            export_audio_scope: crate::gui::types::ExportAudioScope::VocalsAndAudio,
            export_audio_format: crate::renderer::AudioExportFormat::Wav16,
            export_dialog_open: false,
            export_save_path: None,
            export_progress: 1.0,
            export_status_detail: String::new(),
            export_result: None,
            export_in_progress: false,
            folder_picker_open: false,
            folder_picker_current_dir: std::path::PathBuf::from("/"),
            project_properties_open: false,
            project_comment_buffer: String::new(),
            templates_dialog_open: false,
            voicebank_diagnostic_open: false,
            voicebank_diagnostic_report: None,
            project_diagnostic_open: false,
            project_diagnostic_fix_confirmation: None,
            markers_dialog_open: false,
            recovery_snapshots_open: false,
            snapshot_restore_confirmation: None,
            playback_speed_rate: 1.0,
            last_snapshot_time: None,
            #[cfg(not(target_arch = "wasm32"))]
            last_autosave_at: Instant::now(),
            last_exported_notification: None,
            lyrics_dialog_state: Default::default(),
            humanize_dialog_state: Default::default(),
            fx_rack_dialog_state: Default::default(),
            fx_preview_refresh_pending: false,
            frame_time_ema_ms: 16.0,
            last_frame_instant: Instant::now(),
            is_dirty: false,
            exit_confirmation_open: false,
            pending_project_action: None,
            panel_tips_created_at: None,
            phonemizer_warning_dismissed: false,
            phonemizer_warning_expanded: false,
            preview_waveform_cache_hash: 0,
            preview_waveform_last_hash_check: None,
            preview_waveform_pending: None,
            preview_waveform_rx: None,
            preview_waveform_cancel: None,
            workspace_snap_rect: None,
            active_mobile_tab: crate::gui::types::MobileViewTab::PianoRoll,
            mobile_menu_open: false,
        }
    }

    #[test]
    fn switching_between_diffsinger_and_utau_clears_and_restores_state_consistently() {
        let diff_dir = tempfile::tempdir().unwrap();
        fs::write(
            diff_dir.path().join("character.yaml"),
            "name: DiffSinger Voice\nauthor: Neural Author\n",
        )
        .unwrap();
        fs::write(
            diff_dir.path().join("dsconfig.yaml"),
            "acoustic: acoustic.onnx\nvocoder: vocoder\nphonemes: phonemes.txt\nlanguages: languages.json\nspeakers:\n  - standard\n",
        )
        .unwrap();
        fs::write(diff_dir.path().join("acoustic.onnx"), "fake-onnx").unwrap();
        fs::create_dir_all(diff_dir.path().join("vocoder")).unwrap();
        fs::write(diff_dir.path().join("vocoder/vocoder.yaml"), "model: model.onnx\n").unwrap();
        fs::write(diff_dir.path().join("vocoder/model.onnx"), "fake-vocoder").unwrap();
        fs::write(diff_dir.path().join("phonemes.txt"), "SP\nAP\na\n").unwrap();
        fs::write(diff_dir.path().join("languages.json"), "{\"pt\": 0}").unwrap();

        let utau_dir = tempfile::tempdir().unwrap();
        fs::write(
            utau_dir.path().join("character.txt"),
            "name=UTAU Classic Voice\nauthor=Sampler Author\n",
        )
        .unwrap();
        fs::write(
            utau_dir.path().join("oto.ini"),
            "a.wav=a,0,100,-400,80,30\n",
        )
        .unwrap();

        let mut app = test_app();

        // 1. Activate DiffSinger
        app.activate_singer_path(diff_dir.path());
        assert_eq!(app.project.tracks[0].singer, "DiffSinger Voice");
        assert_eq!(
            app.project.tracks[0].voicebank_path.as_deref(),
            Some(diff_dir.path().display().to_string().as_str())
        );
        assert_eq!(app.project.tracks[0].diffsinger_language.as_deref(), Some("pt"));
        assert!(app.project.tracks[0].diffsinger_phonemizer.is_some());
        assert!(app.project.tracks[0].diffsinger_speaker.is_some());
        assert!(app.project.tracks[0].diffsinger_provider.is_some());
        assert!(app.voicebank.is_none());
        assert_eq!(app.transport_state.voicebank_name, "DiffSinger Voice");
        assert!(app.piano_roll_state.show_phoneme_ruler);
        assert!(app.is_dirty);

        // 2. Switch to UTAU
        app.activate_singer_path(utau_dir.path());
        assert_eq!(app.project.tracks[0].singer, "UTAU Classic Voice");
        assert_eq!(
            app.project.tracks[0].voicebank_path.as_deref(),
            Some(utau_dir.path().display().to_string().as_str())
        );
        assert!(app.project.tracks[0].diffsinger_language.is_none());
        assert!(app.project.tracks[0].diffsinger_phonemizer.is_none());
        assert!(app.project.tracks[0].diffsinger_speaker.is_none());
        assert!(app.project.tracks[0].diffsinger_provider.is_none());
        assert!(app.voicebank.is_some());
        assert_eq!(app.voicebank.as_ref().unwrap().name, "UTAU Classic Voice");
        assert_eq!(app.transport_state.voicebank_name, "UTAU Classic Voice");

        // 3. Switch back to DiffSinger
        app.activate_singer_path(diff_dir.path());
        assert_eq!(app.project.tracks[0].singer, "DiffSinger Voice");
        assert!(app.project.tracks[0].diffsinger_language.is_some());
        assert!(app.voicebank.is_none());
        assert_eq!(app.transport_state.voicebank_name, "DiffSinger Voice");
    }
}
