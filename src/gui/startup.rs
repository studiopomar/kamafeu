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
use crate::gui::unified_panel::VocalModeParams;
use crate::gui::KamafeuStudioApp;
use crate::gui::RenderLogFilter;
use crate::oto::Voicebank;
use crate::renderer::AudioExportFormat;
use std::path::PathBuf;
use std::time::Instant;

impl KamafeuStudioApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_fonts(&cc.egui_ctx);

        let (mut config, config_error) = match KamafeuConfig::load() {
            Ok(config) => (config, None),
            Err(error) => (KamafeuConfig::default(), Some(error)),
        };

        cc.egui_ctx.set_visuals(config.theme.create_egui_visuals());

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

        if let Some(ref vb) = voicebank {
            config.add_recent_voicebank(vb.root_path.clone());
        }

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
            piano_roll_state: PianoRollState::default(),
            transport_state,
            right_sidebar_tab: RightSidebarTab::default(),
            vocal_mode_params: VocalModeParams::default(),
            phoneme_palette_state: PhonemePaletteState::default(),
            undo_manager: UndoManager::default(),
            pending_edit_snapshot: None,
            clipboard: Vec::new(),
            audio_player: AudioPlayer::new(),
            sample_rate: 44100,
            render_threads: 4,
            selected_resampler: "straycat-rs (UtaUtaUtau) [Padrão Recomendado]".to_string(),
            selected_wavtool: "Native Rust (Crossfader)".to_string(),
            custom_resampler_path: None,
            custom_wavtool_path: None,

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
            export_options_dialog_open: false,
            export_audio_scope: ExportAudioScope::default(),
            export_audio_format: AudioExportFormat::default(),
            export_dialog_open: false,
            export_save_path: None,
            export_progress: 1.0,
            export_status_detail: String::new(),
            export_result: None,
            export_in_progress: false,
            folder_picker_open: false,
            folder_picker_current_dir: {
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
            },
            project_properties_open: false,
            project_comment_buffer: String::new(),
            templates_dialog_open: false,
            voicebank_diagnostic_open: false,
            playback_speed_rate: 1.0,
            last_snapshot_time: None,
            last_exported_notification: None,
            lyrics_dialog_state: lyrics_dialog::LyricsDialogState::default(),
            humanize_dialog_state: humanize_dialog::HumanizeDialogState::default(),
            fx_rack_dialog_state: fx_rack_dialog::FxRackDialogState::default(),
            theme_editor_dialog_state: theme_editor_dialog::ThemeEditorDialogState::default(),
            fx_rack_config: crate::audio::FxRackConfig::default(),
            frame_time_ema_ms: 16.67,
            last_frame_instant: Instant::now(),
            is_dirty: false,
            preview_waveform_cache_hash: 0,
            preview_waveform_rx: None,
            preview_waveform_cancel: None,
        }
    }
}
