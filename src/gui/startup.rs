use crate::audio::AudioPlayer;
use crate::config::KamafeuConfig;
use crate::gui::fonts::setup_custom_fonts;
use crate::gui::fx_rack_dialog;
use crate::gui::history::UndoManager;
use crate::gui::humanize_dialog;
use crate::gui::lyrics_dialog;
use crate::gui::phoneme_palette::PhonemePaletteState;
use crate::gui::piano_roll::PianoRollState;
use crate::gui::theme_editor_dialog;
use crate::gui::types::ExportAudioScope;
use crate::gui::types::RightSidebarTab;
use crate::gui::types::TransportState;
use crate::gui::KamafeuStudioApp;
use crate::gui::RenderLogFilter;
use crate::oto::Voicebank;
use crate::renderer::AudioExportFormat;
use std::path::PathBuf;
use web_time::Instant;

fn configured_export_format(export: &crate::config::ExportDefaultsConfig) -> AudioExportFormat {
    match export.format.as_str() {
        "FLAC (Lossless)" => {
            if export.bit_depth >= 24 {
                AudioExportFormat::Flac24
            } else {
                AudioExportFormat::Flac16
            }
        }
        "RAW PCM" => AudioExportFormat::RawF32,
        _ if export.bit_depth >= 32 => AudioExportFormat::Wav32Float,
        _ if export.bit_depth >= 24 => AudioExportFormat::Wav24,
        _ => AudioExportFormat::Wav16,
    }
}

fn configured_export_scope(export: &crate::config::ExportDefaultsConfig) -> ExportAudioScope {
    if export.export_stems_default {
        ExportAudioScope::SeparateTrackStems
    } else if export.include_instrumental {
        ExportAudioScope::VocalsAndAudio
    } else {
        ExportAudioScope::VocalsOnly
    }
}

impl KamafeuStudioApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_fonts(&cc.egui_ctx);

        let (config, config_error) = match KamafeuConfig::load() {
            Ok(config) => (config, None),
            Err(error) => (KamafeuConfig::default(), Some(error)),
        };
        let export_audio_format = configured_export_format(&config.export);
        let export_audio_scope = configured_export_scope(&config.export);

        cc.egui_ctx.set_visuals(config.theme.create_egui_visuals());
        crate::phonemizer::set_custom_rules(config.phonemizer_rules.clone());

        if let Some(ref wine_path) = config.dsp.custom_wine_path {
            crate::drivers::process::set_custom_wine_path(Some(wine_path.clone()));
        }
        crate::renderer::resampler_cache::set_persistent_cache_dir_override(
            config.memory.custom_cache_dir.clone(),
        );
        crate::renderer::resampler_cache::set_cache_limits(
            config.memory.max_ram_cache_mb,
            config.memory.max_disk_cache_mb,
        );
        let _ =
            crate::renderer::resampler_cache::cleanup_older_than(config.memory.auto_cleanup_days);

        let project = crate::project::model::UProject::default();

        let mut voicebank: Option<Voicebank> = None;
        if let Some(ref last_path) = config.last_voicebank {
            if last_path.exists() {
                if let Ok(vb) = Voicebank::new(last_path) {
                    voicebank = Some(vb);
                }
            }
        }

        if voicebank.is_none() {
            voicebank = Voicebank::new("demo_vb")
                .or_else(|_| Voicebank::new("sample_vb"))
                .ok();
        }

        // Configurações anteriores ao perfil vocal persistente só tinham a
        // janela DSP global. Preserve-a como ponto de partida, sem sobrescrever
        // um perfil que o usuário já tenha ajustado explicitamente.
        let mut vocal_render = voicebank
            .as_ref()
            .and_then(|voicebank| config.voicebank_render_profiles.get(&voicebank.root_path))
            .cloned()
            .unwrap_or_else(|| config.vocal_render.clone());
        if (vocal_render.crossfade_ms - crate::renderer::RenderOptions::default().crossfade_ms)
            .abs()
            <= f64::EPSILON
        {
            vocal_render.crossfade_ms = f64::from(config.dsp.crossfade_window_ms).clamp(0.0, 200.0);
        }
        let startup_engine_profile = voicebank
            .as_ref()
            .and_then(|voicebank| config.voicebank_engine_profiles.get(&voicebank.root_path));
        let selected_resampler = startup_engine_profile
            .and_then(|profile| profile.resampler.clone())
            .unwrap_or_else(|| config.dsp.default_resampler.clone());
        let selected_wavtool = startup_engine_profile
            .and_then(|profile| profile.wavtool.clone())
            .unwrap_or_else(|| config.dsp.default_wavtool.clone());

        let mut transport_state = TransportState {
            bpm: project.bpm,
            ..TransportState::default()
        };
        if let Some(error) = config_error {
            transport_state.status_message = error;
        }
        if let Some(ref vb) = voicebank {
            transport_state.voicebank_name = vb.name.clone();
            transport_state.voicebank_path = Some(vb.root_path.clone());
        }
        let voicebank_oto_signature = voicebank
            .as_ref()
            .and_then(|vb| crate::copaiba_bridge::oto_signature(&vb.root_path).ok());

        Self {
            project,
            current_project_path: None,
            voicebank,
            voicebank_oto_signature,
            last_voicebank_oto_check: Instant::now(),
            piano_roll_state: {
                let mut state = PianoRollState::default();
                state.show_arrangement_view = config.layout.show_arrangement_view;
                state.show_parameters_drawer = config.layout.show_parameters_drawer;
                state.show_phoneme_ruler = config.layout.show_phoneme_ruler;
                state.show_inspector = config.layout.show_inspector;
                state.is_maximized = config.layout.is_maximized;
                state.px_per_ms = config.layout.px_per_ms;
                state.row_height = config.layout.row_height;
                state
            },
            transport_state,
            right_sidebar_tab: RightSidebarTab::default(),
            vocal_mode_params: vocal_render,
            phoneme_palette_state: PhonemePaletteState::default(),
            undo_manager: UndoManager::default(),
            pending_edit_snapshot: None,
            clipboard: Vec::new(),
            audio_player: AudioPlayer::new(),
            sample_rate: config.audio.sample_rate.clamp(8_000, 192_000),
            render_threads: if config.dsp.render_threads == 0 {
                4
            } else {
                config.dsp.render_threads
            },
            selected_resampler,
            selected_wavtool,
            custom_resampler_path: config.dsp.custom_resampler_path.clone(),
            custom_wavtool_path: config.dsp.custom_wavtool_path.clone(),

            playback_start_instant: None,
            playback_start_offset_ms: 0.0,
            render_rx: None,
            progressive_playback_started: false,
            render_cancel: None,
            render_log_window_open: false,
            render_log_messages: Vec::new(),
            render_log_filter: RenderLogFilter::All,
            render_log_search_query: String::new(),
            render_log_font_size: 11.5,
            render_log_show_line_numbers: true,
            render_log_wrap_lines: true,
            auto_scroll_log: true,
            render_progress: 1.0,
            render_status_title: "Pronto".to_string(),
            render_log_channel_rx: None,
            export_rx: None,
            active_track_index: 0,
            singers_list: crate::oto::SingerScanner::scan_directories(&config.singers_paths),
            singer_search_query: String::new(),
            singers_gallery_window_open: false,
            config,
            discord_rpc: crate::discord_rpc::DiscordRpcManager::new(),
            copaiba_app: crate::copaiba::gui::CopaibaToolkitApp::default(),
            copaiba_window_open: false,
            packages_window_open: false,
            packages_target_os: 0,
            packages_search: String::new(),
            preferences_window_open: false,
            preferences_tab: 7,
            preferences_state: crate::gui::preferences_dialog::PreferencesDialogState::default(),
            theme_customizer_open: false,
            theme_customizer_tab: 0,
            shortcuts_guide_open: false,
            autopitch_window_open: false,
            autopitch_options: crate::dsp::AutoPitchOptions::default(),
            autopitch_scope: crate::dsp::AutoPitchScope::SelectedOnly,
            batch_lyrics_open: false,
            batch_lyrics_buffer: String::new(),
            last_window_title: String::new(),
            startup_maximize_requested: true,
            export_options_dialog_open: false,
            export_audio_scope,
            export_audio_format,
            export_dialog_open: false,
            export_save_path: None,
            export_progress: 1.0,
            export_status_detail: String::new(),
            export_result: None,
            export_in_progress: false,
            folder_picker_open: false,
            folder_picker_current_dir: {
                #[cfg(target_arch = "wasm32")]
                {
                    PathBuf::from("/")
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let default_dir = PathBuf::from("/sdcard/Download");
                    if default_dir.exists() {
                        default_dir
                    } else {
                        let sdcard = PathBuf::from("/sdcard");
                        if sdcard.exists() {
                            sdcard
                        } else {
                            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                        }
                    }
                }
            },
            project_properties_open: false,
            project_comment_buffer: String::new(),
            templates_dialog_open: false,
            voicebank_diagnostic_open: false,
            voicebank_diagnostic_report: None,
            playback_speed_rate: 1.0,
            last_snapshot_time: None,
            last_exported_notification: None,
            lyrics_dialog_state: lyrics_dialog::LyricsDialogState::default(),
            humanize_dialog_state: humanize_dialog::HumanizeDialogState::default(),
            fx_rack_dialog_state: fx_rack_dialog::FxRackDialogState::default(),
            theme_editor_dialog_state: theme_editor_dialog::ThemeEditorDialogState::default(),
            fx_rack_config: crate::audio::FxRackConfig::default(),
            fx_preview_refresh_pending: false,
            frame_time_ema_ms: 16.67,
            last_frame_instant: Instant::now(),
            is_dirty: false,
            exit_confirmation_open: false,
            panel_tips_created_at: Some(Instant::now()),
            phonemizer_warning_dismissed: false,
            phonemizer_warning_expanded: false,
            preview_waveform_cache_hash: 0,
            preview_waveform_rx: None,
            preview_waveform_cancel: None,
            workspace_snap_rect: None,
            active_mobile_tab: crate::gui::types::MobileViewTab::default(),
            mobile_menu_open: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{configured_export_format, configured_export_scope};
    use crate::config::ExportDefaultsConfig;
    use crate::gui::types::ExportAudioScope;
    use crate::renderer::AudioExportFormat;

    #[test]
    fn export_defaults_choose_matching_output_format() {
        let mut config = ExportDefaultsConfig::default();
        config.format = "FLAC (Lossless)".to_string();
        config.bit_depth = 24;
        assert_eq!(configured_export_format(&config), AudioExportFormat::Flac24);
        config.format = "WAV (Lossless PCM)".to_string();
        config.bit_depth = 32;
        assert_eq!(
            configured_export_format(&config),
            AudioExportFormat::Wav32Float
        );
    }

    #[test]
    fn export_defaults_choose_matching_scope() {
        let mut config = ExportDefaultsConfig::default();
        assert_eq!(
            configured_export_scope(&config),
            ExportAudioScope::VocalsAndAudio
        );

        config.include_instrumental = false;
        assert_eq!(
            configured_export_scope(&config),
            ExportAudioScope::VocalsOnly
        );

        config.export_stems_default = true;
        assert_eq!(
            configured_export_scope(&config),
            ExportAudioScope::SeparateTrackStems
        );
    }
}
