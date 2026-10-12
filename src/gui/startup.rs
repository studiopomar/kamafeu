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
use crate::gui::types::RightSidebarTab;
use crate::gui::types::TransportState;
use crate::gui::types::{AutoScrollMode, ExportAudioScope, GridSnapOption};
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
        crate::dsp::world_resampler::set_runtime_config(
            crate::dsp::world_resampler::WorldRuntimeConfig {
                f0_floor_hz: config.dsp.f0_min_hz as f64,
                f0_ceil_hz: config.dsp.f0_max_hz as f64,
                voiced_aperiodicity: config.dsp.world_voiced_aperiodicity as f64,
                formant_preservation_mode: config.dsp.formant_preservation_mode.clone(),
                f0_detection_method: config.dsp.f0_detection_method.clone(),
                frame_period_ms: config.dsp.world_frame_period_ms as f64,
            },
        );
        crate::renderer::TrackRenderer::set_pitch_sampling_config(
            config.dsp.pitch_curve_step_ms,
            &config.dsp.pitch_interpolation,
        );

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
        crate::gui::piano_roll::state::set_waveform_cache_resolution(
            config.memory.waveform_cache_resolution,
        );
        crate::renderer::TrackRenderer::set_io_thread_concurrency(
            config.memory.io_thread_concurrency,
        );
        crate::renderer::TrackRenderer::set_preload_strategy(&config.memory.ram_preload_strategy);
        crate::renderer::TrackRenderer::set_anti_aliasing_filter(config.dsp.anti_aliasing_filter);
        crate::renderer::ProjectRenderer::set_render_limiter(
            config.dsp.render_limiter_enabled,
            config.dsp.render_limiter_peak_db,
        );
        crate::audio::player::set_flush_denormals_to_zero(
            config.experimental.flush_denormals_to_zero,
        );
        crate::renderer::TrackRenderer::set_crossfade_curve(&config.dsp.crossfade_curve);
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
        transport_state.grid_snap = match config.workflow.default_grid_snap.as_str() {
            "Freeform" => GridSnapOption::Freeform,
            "1/4" => GridSnapOption::Snap1_4,
            "1/8" => GridSnapOption::Snap1_8,
            "1/32" => GridSnapOption::Snap1_32,
            _ => GridSnapOption::Snap1_16,
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
        let mut humanize_dialog_state = humanize_dialog::HumanizeDialogState::default();
        humanize_dialog_state.humanize_params.timing_jitter_ms =
            config.workflow.humanize_timing_jitter_ms.clamp(0.0, 100.0);
        humanize_dialog_state.humanize_params.pitch_cents_jitter = config
            .workflow
            .humanize_pitch_cents_jitter
            .clamp(0.0, 100.0);
        humanize_dialog_state.humanize_params.volume_jitter_pct =
            config.workflow.humanize_volume_jitter_pct.clamp(0.0, 50.0);
        humanize_dialog_state.humanize_params.breathiness_jitter_pct = config
            .workflow
            .humanize_breathiness_jitter_pct
            .clamp(0.0, 50.0);
        humanize_dialog_state.vibrato_params.min_duration_ms = config
            .workflow
            .auto_vibrato_min_duration_ms
            .clamp(80.0, 2000.0);
        humanize_dialog_state.vibrato_params.length_pct =
            config.workflow.auto_vibrato_length_pct.clamp(20.0, 100.0);
        humanize_dialog_state.vibrato_params.depth_cents =
            config.workflow.auto_vibrato_depth_cents.clamp(10.0, 150.0);
        humanize_dialog_state.vibrato_params.period_ms =
            config.workflow.auto_vibrato_period_ms.clamp(80.0, 300.0);
        humanize_dialog_state.vibrato_params.fade_in_pct =
            config.workflow.auto_vibrato_fade_in_pct.clamp(5.0, 60.0);

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
                state.show_minimap = config.layout.show_minimap;
                state.show_waveform_area = config.layout.show_waveform_area;
                state.show_envelope_handles = config.layout.show_envelope_handles;
                state.vertical_pitch_follow = config.layout.vertical_pitch_follow;
                state.is_maximized = config.layout.is_maximized;
                state.px_per_ms = config.layout.px_per_ms;
                state.row_height = config.layout.row_height;
                state.min_midi = config
                    .layout
                    .default_min_midi
                    .min(config.layout.default_max_midi.saturating_sub(1));
                state.max_midi = config
                    .layout
                    .default_max_midi
                    .max(state.min_midi.saturating_add(1));
                state.default_note_duration_ms = config.workflow.default_note_duration_ms;
                state.default_note_lyric = if config.workflow.default_note_lyric.trim().is_empty() {
                    "ka".to_string()
                } else {
                    config.workflow.default_note_lyric.clone()
                };
                state.default_note_dynamics =
                    config.workflow.default_note_dynamics.clamp(-100.0, 100.0);
                state.default_note_volume = config.workflow.default_note_volume.clamp(0.0, 200.0);
                state.default_note_attack = config.workflow.default_note_attack.clamp(0.0, 200.0);
                state.default_note_decay = config.workflow.default_note_decay.clamp(0.0, 100.0);
                state.ui_animations_enabled = config.workflow.ui_animations_enabled;
                state.ui_animation_speed = config.workflow.ui_animation_speed.clamp(0.25, 4.0);
                state.active_tool = match config.workflow.default_edit_tool.as_str() {
                    "Pencil" => crate::gui::types::EditTool::Pencil,
                    "PitchDraw" => crate::gui::types::EditTool::PitchDraw,
                    "Slice" => crate::gui::types::EditTool::Slice,
                    "Eraser" => crate::gui::types::EditTool::Eraser,
                    _ => crate::gui::types::EditTool::Pointer,
                };
                state.pitch_sub_tool = match config.workflow.default_pitch_sub_tool.as_str() {
                    "Smooth" => crate::gui::types::PitchSubTool::Smooth,
                    "Line" => crate::gui::types::PitchSubTool::Line,
                    "Vibrato" => crate::gui::types::PitchSubTool::Vibrato,
                    _ => crate::gui::types::PitchSubTool::Freehand,
                };
                state.active_scale = match config.layout.default_scale.as_str() {
                    "Major" => crate::gui::piano_roll::MusicalScale::Major,
                    "NaturalMinor" => crate::gui::piano_roll::MusicalScale::NaturalMinor,
                    "HarmonicMinor" => crate::gui::piano_roll::MusicalScale::HarmonicMinor,
                    "MelodicMinor" => crate::gui::piano_roll::MusicalScale::MelodicMinor,
                    "PentatonicMajor" => crate::gui::piano_roll::MusicalScale::PentatonicMajor,
                    "PentatonicMinor" => crate::gui::piano_roll::MusicalScale::PentatonicMinor,
                    "Blues" => crate::gui::piano_roll::MusicalScale::Blues,
                    "Dorian" => crate::gui::piano_roll::MusicalScale::Dorian,
                    "Mixolydian" => crate::gui::piano_roll::MusicalScale::Mixolydian,
                    _ => crate::gui::piano_roll::MusicalScale::Chromatic,
                };
                state.scale_root_key = config.layout.default_scale_root_key.min(11);
                state.auto_scroll_mode = match config.workflow.default_auto_scroll.as_str() {
                    "Desligado" => AutoScrollMode::Off,
                    "Seguir Cabeça (Cursor)" => AutoScrollMode::StationaryCursor,
                    _ => AutoScrollMode::PageScroll,
                };
                state
            },
            transport_state,
            right_sidebar_tab: RightSidebarTab::default(),
            vocal_mode_params: vocal_render,
            phoneme_palette_state: PhonemePaletteState::default(),
            undo_manager: UndoManager::default(),
            pending_edit_snapshot: None,
            clipboard: Vec::new(),
            audio_player: {
                let mut player = AudioPlayer::new();
                player.set_volume(config.audio.master_volume);
                player
            },
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
            retry_project: None,
            retry_voicebank: None,
            retry_options: None,
            failed_chunk: None,
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
            export_cancel: None,
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
            command_palette_open: false,
            command_palette_query: String::new(),
            command_palette_selected: 0,
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
            pending_project_action: None,
            lyrics_dialog_state: lyrics_dialog::LyricsDialogState::default(),
            humanize_dialog_state,
            fx_rack_dialog_state: fx_rack_dialog::FxRackDialogState::default(),
            theme_editor_dialog_state: theme_editor_dialog::ThemeEditorDialogState::default(),
            fx_rack_config: crate::audio::FxRackConfig::default(),
            fx_preview_refresh_pending: false,
            frame_time_ema_ms: 16.67,
            last_frame_instant: Instant::now(),
            is_dirty: false,
            exit_confirmation_open: false,
            // A dica continua disponível em Ajuda, mas não cobre o editor na
            // primeira abertura, especialmente em telas pequenas.
            panel_tips_created_at: None,
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
