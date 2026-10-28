//! Application state and module entry points. Frame orchestration lives in `app_frame`.

mod activity;
pub(crate) mod animation;
mod app_frame;
pub mod arrangement;
mod audio_export;
mod background_tasks;
pub mod dialogs;
mod editor_canvas;
mod editor_panels;
#[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
mod engine_install;
mod engine_setup;
pub mod fonts;
mod format_actions;
pub mod fx_rack_dialog;
pub(super) mod fx_rack_ui;
pub(crate) mod visual_fx;
pub mod history;
pub mod humanize_dialog;
pub mod i18n;
pub mod image_cache;
pub mod inspector;
mod keyboard_shortcuts;
pub mod lyrics_dialog;
mod marquee;
pub mod menu_bar;
mod note_actions;
mod notifications;
pub mod phoneme_palette;
pub mod phoneme_ruler;
pub mod piano_roll;
mod playback;
mod playback_buffer;
pub mod preferences_dialog;
mod project_files;
mod render_log;
mod startup;
pub mod theme;
pub mod theme_editor_dialog;
pub mod toolbar;
pub mod types;
pub mod unified_panel;
mod voicebank_actions;
pub mod window_icon;
mod workspace;

use crate::audio::AudioPlayer;
use crate::config::KamafeuConfig;
use crate::gui::history::UndoManager;
use crate::gui::phoneme_palette::PhonemePaletteState;
use crate::gui::piano_roll::PianoRollState;
use crate::gui::types::ExportAudioScope;
use crate::gui::types::RightSidebarTab;
use crate::gui::types::TransportState;
use crate::gui::unified_panel::VocalModeParams;
use crate::oto::Voicebank;
use crate::project::model::UNote;
use crate::project::model::UProject;
use crate::renderer::AudioExportFormat;
use crate::renderer::ProgressiveChunk;
use crate::renderer::RenderedAudio;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use web_time::Instant;

/// Whether the current target should use touch-sized controls. A narrow native
/// desktop window remains a compact desktop layout; only Android and narrow
/// browser viewports opt into touch-first spacing/navigation.
pub(crate) fn touch_first_layout(ctx: &eframe::egui::Context) -> bool {
    #[cfg(target_os = "android")]
    {
        let _ = ctx;
        true
    }
    #[cfg(all(not(target_os = "android"), target_arch = "wasm32"))]
    {
        let viewport = ctx.screen_rect();
        viewport.width() < 760.0 || (viewport.height() < 520.0 && viewport.width() < 900.0)
    }
    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    {
        let _ = ctx;
        false
    }
}

pub fn open_external_url(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd")
        .args(["/C", "start", "", url])
        .spawn();
}

#[allow(dead_code)]
fn playback_sample_offset(audio: &RenderedAudio, start_ms: f64) -> usize {
    let frame = ((start_ms.max(0.0) / 1_000.0) * audio.sample_rate as f64).round() as usize;
    frame
        .saturating_mul(usize::from(audio.channels.max(1)))
        .min(audio.samples.len())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderLogFilter {
    All,
    ErrorsWarnings,
    DspResampler,
    WavOto,
    Info,
    Timing,
    Chunks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RenderTaskKind {
    Preview,
    Export,
}

#[derive(Debug, Clone)]
pub(crate) enum PendingProjectAction {
    New,
    Open(PathBuf),
}

pub struct KamafeuStudioApp {
    project: UProject,
    current_project_path: Option<PathBuf>,
    voicebank: Option<Voicebank>,
    voicebank_oto_signature: Option<crate::copaiba_bridge::OtoSignature>,
    last_voicebank_oto_check: Instant,
    piano_roll_state: PianoRollState,
    transport_state: TransportState,
    right_sidebar_tab: RightSidebarTab,
    vocal_mode_params: VocalModeParams,
    /// Policy selected for the next project import. Preserve is the safe
    /// default because explicit aliases are authored render decisions.
    imported_phoneme_policy: crate::project::model::ImportedPhonemePolicy,
    phoneme_palette_state: PhonemePaletteState,
    undo_manager: UndoManager,
    pending_edit_snapshot: Option<UProject>,
    clipboard: Vec<UNote>,
    audio_player: AudioPlayer,
    sample_rate: u32,
    render_threads: u32,
    selected_resampler: String,
    selected_wavtool: String,
    custom_resampler_path: Option<PathBuf>,
    custom_wavtool_path: Option<PathBuf>,
    playback_start_instant: Option<Instant>,
    playback_start_offset_ms: f64,
    playback_duration_ms: f64,
    render_rx: Option<std::sync::mpsc::Receiver<ProgressiveChunk>>,
    direct_playback_rx: Option<std::sync::mpsc::Receiver<RenderedAudio>>,
    direct_playback_active: bool,
    render_cancel: Option<Arc<AtomicBool>>,
    retry_project: Option<UProject>,
    retry_voicebank: Option<Voicebank>,
    retry_options: Option<crate::renderer::RenderOptions>,
    failed_chunk: Option<(usize, f64, f64, bool)>,
    progressive_playback_started: bool,
    progressive_startup_audio: Vec<(Vec<f32>, u32, u16, bool)>,
    playback_chunk_buffer: playback_buffer::PlaybackChunkBuffer,
    render_log_window_open: bool,
    render_log_messages: Vec<String>,
    render_log_filter: RenderLogFilter,
    render_log_search_query: String,
    render_log_font_size: f32,
    render_log_show_line_numbers: bool,
    render_log_wrap_lines: bool,
    auto_scroll_log: bool,
    render_progress: f32,
    render_status_title: String,
    render_log_channel_rx: Option<std::sync::mpsc::Receiver<(f32, String)>>,
    render_progress_event: Option<crate::renderer::RenderProgressEvent>,
    render_progress_rx: Option<std::sync::mpsc::Receiver<crate::renderer::RenderProgressEvent>>,
    render_task_kind: Option<RenderTaskKind>,
    render_progress_started_at: Option<Instant>,
    render_progress_finished_at: Option<Instant>,
    export_rx: Option<std::sync::mpsc::Receiver<Result<(), String>>>,
    export_cancel: Option<Arc<AtomicBool>>,
    active_track_index: usize,
    config: KamafeuConfig,
    singers_list: Vec<crate::oto::SingerInfo>,
    singer_search_query: String,
    singers_gallery_window_open: bool,
    discord_rpc: crate::discord_rpc::DiscordRpcManager,
    copaiba_app: crate::copaiba::gui::CopaibaToolkitApp,
    copaiba_window_open: bool,
    pub packages_window_open: bool,
    pub packages_target_os: usize,
    pub packages_search: String,
    pub preferences_window_open: bool,
    pub preferences_tab: usize,
    pub preferences_state: crate::gui::preferences_dialog::PreferencesDialogState,
    pub theme_customizer_open: bool,
    pub theme_customizer_tab: usize,
    pub saved_theme_selected: Option<usize>,
    pub saved_theme_name_draft: String,
    pub pending_theme_import: Option<(String, String)>,
    pub pending_theme_delete: Option<usize>,
    shortcuts_guide_open: bool,
    command_palette_open: bool,
    command_palette_query: String,
    command_palette_selected: usize,
    pub autopitch_window_open: bool,
    pub autopitch_options: crate::dsp::AutoPitchOptions,
    pub autopitch_scope: crate::dsp::AutoPitchScope,
    pub batch_lyrics_open: bool,
    pub batch_lyrics_buffer: String,
    last_window_title: String,
    pub(crate) startup_maximize_requested: bool,
    pub export_options_dialog_open: bool,
    pub export_audio_scope: ExportAudioScope,
    pub export_audio_format: AudioExportFormat,
    pub export_dialog_open: bool,
    pub export_save_path: Option<PathBuf>,
    pub export_progress: f32,
    pub export_status_detail: String,
    pub export_result: Option<Result<(), String>>,
    pub export_in_progress: bool,
    pub folder_picker_open: bool,
    pub folder_picker_current_dir: PathBuf,
    pub project_properties_open: bool,
    pub project_comment_buffer: String,
    pub templates_dialog_open: bool,
    pub voicebank_diagnostic_open: bool,
    pub voicebank_diagnostic_report: Option<crate::oto::VoicebankDiagnosticReport>,
    pub project_diagnostic_open: bool,
    pub project_diagnostic_fix_confirmation: Option<crate::project::ProjectIssueKind>,
    pub markers_dialog_open: bool,
    pub recovery_snapshots_open: bool,
    pub(crate) snapshot_restore_confirmation: Option<(std::path::PathBuf, SnapshotRestoreKind)>,
    pub playback_speed_rate: f64,
    pub last_snapshot_time: Option<String>,
    #[cfg(not(target_arch = "wasm32"))]
    last_autosave_at: Instant,
    pub last_exported_notification: Option<(PathBuf, String, Instant)>,
    pub lyrics_dialog_state: lyrics_dialog::LyricsDialogState,
    pub humanize_dialog_state: humanize_dialog::HumanizeDialogState,
    pub fx_rack_dialog_state: fx_rack_dialog::FxRackDialogState,
    /// A slider/knob may emit many changes during one drag; preview refresh is
    /// deferred until that gesture ends so rendering never starves the UI.
    fx_preview_refresh_pending: bool,
    frame_time_ema_ms: f32,
    last_frame_instant: Instant,
    pub is_dirty: bool,
    /// Intercepts the OS close request until the user explicitly saves,
    /// discards, or returns to the editor.
    exit_confirmation_open: bool,
    pub(crate) pending_project_action: Option<PendingProjectAction>,
    pub panel_tips_created_at: Option<Instant>,
    pub phonemizer_warning_dismissed: bool,
    pub phonemizer_warning_expanded: bool,
    preview_waveform_cache_hash: u64,
    preview_waveform_last_hash_check: Option<Instant>,
    preview_waveform_pending: Option<(u64, Instant)>,
    preview_waveform_rx: Option<std::sync::mpsc::Receiver<(u64, Vec<(f32, f32, f32)>)>>,
    preview_waveform_cancel: Option<Arc<AtomicBool>>,
    workspace_snap_rect: Option<(workspace::WorkspacePane, eframe::egui::Rect)>,
    pub active_mobile_tab: types::MobileViewTab,
    pub mobile_menu_open: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SnapshotRestoreKind {
    Notes,
    Audio,
    Markers,
    Sections,
}

impl Drop for KamafeuStudioApp {
    fn drop(&mut self) {
        self.persist_config();
        if self.config.memory.clear_cache_on_exit {
            let _ = crate::renderer::resampler_cache::clear_disk_cache();
        }
    }
}

#[cfg(test)]
mod preview_tests {
    use super::*;

    #[test]
    fn playback_offset_uses_frames_channels_and_clamps() {
        let audio = RenderedAudio {
            error: None,
            samples: vec![0.0; 4_000],
            sample_rate: 1_000,
            channels: 2,
        };
        assert_eq!(playback_sample_offset(&audio, 750.0), 1_500);
        assert_eq!(playback_sample_offset(&audio, -100.0), 0);
        assert_eq!(playback_sample_offset(&audio, 3_000.0), 4_000);
    }
}
