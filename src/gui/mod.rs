pub mod arrangement;
pub mod dialogs;
pub mod fonts;
pub mod history;
pub mod image_cache;
pub mod inspector;
pub mod menu_bar;
pub mod phoneme_palette;
pub mod phoneme_ruler;
pub mod piano_roll;
pub mod theme;
pub mod toolbar;
pub mod types;
pub mod unified_panel;
pub mod window_icon;

use crate::config::KamafeuConfig;
use eframe::egui::{self, CentralPanel, Frame, Key, SidePanel, TopBottomPanel};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::audio::AudioPlayer;
use crate::drivers::{
    ExternalResamplerDriver, ExternalWavtoolDriver, KnownResampler, KnownWavtool, MacResDriver,
    NativeResamplerDriver, NativeSolaResamplerDriver, NativeWavtoolDriver, ResamplerDriver,
    WavtoolDriver, WavtoolYawuDriver,
};

use crate::formats::{
    ApsFormat, MidiFormat, SvpFormat, UfdataFormat, UstFormat, UstxFormat, VsqxFormat,
};
use crate::gui::arrangement::draw_arrangement_view;
use crate::gui::fonts::setup_custom_fonts;
use crate::gui::history::UndoManager;
use crate::gui::phoneme_palette::PhonemePaletteState;
use crate::gui::piano_roll::{draw_piano_roll, PianoRollState};
use crate::gui::theme::MelodyneTheme;
use crate::gui::toolbar::draw_unified_toolbar;
use crate::gui::types::{EditTool, ExportAudioScope, RightSidebarTab, TransportState};
use crate::gui::unified_panel::{draw_unified_panel, VocalModeParams};
use crate::oto::Voicebank;
use crate::project::model::{UNote, UProject};
use crate::renderer::{ProjectRenderer, RenderedAudio, TrackRenderer};

struct PreviewRender {
    audio: RenderedAudio,
}

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
    render_rx: Option<std::sync::mpsc::Receiver<PreviewRender>>,
    render_cancel: Option<Arc<AtomicBool>>,
    render_log_window_open: bool,
    render_log_messages: Vec<String>,
    render_log_filter: RenderLogFilter,
    auto_scroll_log: bool,
    render_progress: f32,
    render_status_title: String,
    render_log_channel_rx: Option<std::sync::mpsc::Receiver<(f32, String)>>,
    export_rx: Option<std::sync::mpsc::Receiver<Result<(), String>>>,
    active_track_index: usize,
    config: KamafeuConfig,
    singers_list: Vec<crate::oto::SingerInfo>,
    singer_search_query: String,
    singers_gallery_window_open: bool,
    discord_rpc: crate::discord_rpc::DiscordRpcManager,
    copaiba_app: crate::copaiba::gui::CopaibaToolkitApp,
    copaiba_window_open: bool,
    shortcuts_guide_open: bool,
    pub autopitch_window_open: bool,
    pub autopitch_options: crate::dsp::AutoPitchOptions,
    pub autopitch_scope: crate::dsp::AutoPitchScope,
    pub batch_lyrics_open: bool,
    pub batch_lyrics_buffer: String,
    last_window_title: String,
    pub export_options_dialog_open: bool,
    pub export_audio_scope: ExportAudioScope,
    pub export_dialog_open: bool,
    pub export_save_path: Option<PathBuf>,
    pub export_progress: f32,
    pub export_status_detail: String,
    pub export_result: Option<Result<(), String>>,
    pub export_in_progress: bool,
    pub folder_picker_open: bool,
    pub folder_picker_current_dir: PathBuf,
    frame_time_ema_ms: f32,
    last_frame_instant: Instant,
    pub is_dirty: bool,
}

impl KamafeuStudioApp {
    fn persist_config(&mut self) {
        if let Err(error) = self.config.save() {
            self.transport_state.status_message = error;
        }
    }
    fn create_resampler_driver(&self) -> Box<dyn ResamplerDriver> {
        #[cfg(target_os = "android")]
        {
            return Box::new(NativeSolaResamplerDriver {
                mode: crate::dsp::SolaStretchMode::Hybrid,
            });
        }

        #[cfg(not(target_os = "android"))]
        {
            if self.selected_resampler.contains("straycat") {
                let profile = KnownResampler::StraycatRs;
                return Box::new(ExternalResamplerDriver::for_known(
                    profile,
                    self.custom_resampler_path.clone(),
                ));
            }

            if self.selected_resampler.contains("Phase Vocoder")
                || self.selected_resampler.contains("SOLA Híbrido")
            {
                return Box::new(NativeSolaResamplerDriver {
                    mode: crate::dsp::SolaStretchMode::Hybrid,
                });
            }

            if self.selected_resampler.contains("SOLA") || self.selected_resampler.contains("WSOLA")
            {
                let mode = if self.selected_resampler.contains("Loop") {
                    crate::dsp::SolaStretchMode::Loop
                } else if self.selected_resampler.contains("Spline") {
                    crate::dsp::SolaStretchMode::Spline
                } else {
                    crate::dsp::SolaStretchMode::Stretch
                };
                return Box::new(NativeSolaResamplerDriver { mode });
            }

            if self.selected_resampler.contains("TD-PSOLA")
                || self.selected_resampler.contains("PSOLA")
            {
                return Box::new(NativeResamplerDriver);
            }

            if self.selected_resampler.contains("Native")
                || self.selected_resampler.contains("Nativo")
            {
                return Box::new(NativeSolaResamplerDriver::default());
            }

            if let Some(profile) = KnownResampler::from_label(&self.selected_resampler) {
                if profile == KnownResampler::MacRes {
                    let path = self
                        .custom_resampler_path
                        .clone()
                        .filter(|path| path.is_file())
                        .or_else(|| profile.find_executable())
                        .unwrap_or_else(|| profile.default_path());
                    return Box::new(MacResDriver::new(path));
                }
                return Box::new(ExternalResamplerDriver::for_known(
                    profile,
                    self.custom_resampler_path.clone(),
                ));
            }

            let path = self
                .custom_resampler_path
                .clone()
                .unwrap_or_else(|| PathBuf::from(&self.selected_resampler));
            Box::new(ExternalResamplerDriver::new(path))
        }
    }

    fn create_wavtool_driver(&self) -> Box<dyn WavtoolDriver> {
        #[cfg(target_os = "android")]
        {
            return Box::new(NativeWavtoolDriver);
        }

        #[cfg(not(target_os = "android"))]
        {
            if self.selected_wavtool.contains("Native") || self.selected_wavtool.contains("Nativo")
            {
                return Box::new(NativeWavtoolDriver);
            }

            if let Some(profile) = KnownWavtool::from_label(&self.selected_wavtool) {
                if profile == KnownWavtool::WavtoolYawu {
                    let path = self
                        .custom_wavtool_path
                        .clone()
                        .filter(|p| p.is_file())
                        .or_else(|| profile.find_executable())
                        .unwrap_or_else(|| profile.default_path());
                    return Box::new(WavtoolYawuDriver::new(path));
                }
                return Box::new(ExternalWavtoolDriver::for_known(
                    profile,
                    self.custom_wavtool_path.clone(),
                ));
            }

            let path = self
                .custom_wavtool_path
                .clone()
                .unwrap_or_else(|| PathBuf::from(&self.selected_wavtool));
            Box::new(ExternalWavtoolDriver::new(path))
        }
    }

    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_fonts(&cc.egui_ctx);

        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = MelodyneTheme::BG_PANEL;
        visuals.window_fill = MelodyneTheme::BG_PANEL;
        visuals.widgets.noninteractive.bg_fill = MelodyneTheme::BG_PANEL;
        visuals.widgets.noninteractive.fg_stroke =
            egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(220, 215, 235));

        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(32, 24, 46);
        visuals.widgets.inactive.fg_stroke =
            egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(210, 200, 230));

        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(48, 35, 72);
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.2_f32, egui::Color32::WHITE);

        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(60, 42, 90);
        visuals.widgets.active.fg_stroke =
            egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(0, 255, 157));

        visuals.selection.bg_fill = egui::Color32::from_rgb(60, 42, 90);
        visuals.selection.stroke = egui::Stroke::new(1.2_f32, egui::Color32::from_rgb(0, 255, 157));
        cc.egui_ctx.set_visuals(visuals);

        let project = crate::project::model::UProject::default();

        let (mut config, config_error) = match KamafeuConfig::load() {
            Ok(config) => (config, None),
            Err(error) => (KamafeuConfig::default(), Some(error)),
        };

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

        let wavtool_default_path = PathBuf::from("./wavtools/wavtool-yawu");

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
            selected_wavtool: "wavtool-yawu (m13253/wavtool-yawu)".to_string(),
            custom_resampler_path: None,
            custom_wavtool_path: Some(wavtool_default_path),

            playback_start_instant: None,
            playback_start_offset_ms: 0.0,
            render_rx: None,
            render_cancel: None,
            render_log_window_open: false,
            render_log_messages: Vec::new(),
            render_log_filter: RenderLogFilter::All,
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
            shortcuts_guide_open: false,
            autopitch_window_open: false,
            autopitch_options: crate::dsp::AutoPitchOptions::default(),
            autopitch_scope: crate::dsp::AutoPitchScope::SelectedOnly,
            batch_lyrics_open: false,
            batch_lyrics_buffer: String::new(),
            last_window_title: String::new(),
            export_options_dialog_open: false,
            export_audio_scope: ExportAudioScope::default(),
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
            frame_time_ema_ms: 16.67,
            last_frame_instant: Instant::now(),
            is_dirty: false,
        }
    }

    pub fn apply_autopitch(&mut self) {
        self.push_history();

        let selected: Option<Vec<usize>> = match self.autopitch_scope {
            crate::dsp::AutoPitchScope::SelectedOnly => {
                if self.piano_roll_state.selected_note_indices.is_empty() {
                    if let Some(idx) = self.piano_roll_state.selected_note_index {
                        Some(vec![idx])
                    } else {
                        None
                    }
                } else {
                    Some(
                        self.piano_roll_state
                            .selected_note_indices
                            .iter()
                            .copied()
                            .collect(),
                    )
                }
            }
            crate::dsp::AutoPitchScope::AllNotes => None,
        };

        let options = self.autopitch_options.clone();
        let notes = self.current_notes_mut();
        crate::dsp::AutoPitchEngine::apply_to_notes(notes, selected.as_deref(), &options);
        self.piano_roll_state.continuous_edit_dirty = true;
        self.is_dirty = true;
        self.transport_state.status_message = format!(
            "✨ AutoPitch aplicado com sucesso ({})",
            options.preset.name()
        );
    }

    pub fn current_notes_mut(&mut self) -> &mut Vec<UNote> {
        if self.project.tracks.is_empty() {
            self.project
                .tracks
                .push(crate::project::model::UTrack::default());
        }
        if self.active_track_index >= self.project.tracks.len() {
            self.active_track_index = 0;
        }
        let track_idx = self.active_track_index;

        if let Some(part_idx) = self
            .project
            .parts
            .iter()
            .position(|p| p.track_index == track_idx)
        {
            &mut self.project.parts[part_idx].notes
        } else {
            let part_name = format!("Part Track {}", track_idx + 1);
            let new_part = crate::project::model::UVoicePart::new(part_name, track_idx);
            self.project.parts.push(new_part);
            let last_idx = self.project.parts.len() - 1;
            &mut self.project.parts[last_idx].notes
        }
    }

    pub fn reload_singers(&mut self) {
        self.singers_list = crate::oto::SingerScanner::scan_directories(&self.config.singers_paths);
    }

    fn refresh_voicebank_oto(&mut self) {
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

        if let Ok(voicebank) = Voicebank::new(&root_path) {
            self.transport_state.voicebank_name = voicebank.name.clone();
            self.transport_state.voicebank_path = Some(voicebank.root_path.clone());
            self.voicebank = Some(voicebank);
            self.voicebank_oto_signature = Some(signature);
            self.piano_roll_state.phoneme_cache_hash = 0;
            self.piano_roll_state.phoneme_cache.clear();
            self.piano_roll_state.note_phonemes_cache.clear();
            self.transport_state.status_message = "oto.ini atualizado pelo Copaiba NEO".to_string();
        }
    }

    #[cfg(not(target_os = "android"))]
    fn open_copaiba_for_alias(&mut self, requested_alias: &str) {
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
                    .find_entry(requested_alias, "C4")
                    .map(|entry| entry.alias.clone())
            })
            .unwrap_or_else(|| requested_alias.trim().to_string());
        let result = crate::copaiba_bridge::launch_editor(&voicebank.root_path, &alias);
        self.transport_state.status_message = match result {
            Ok(()) => format!("Copaiba NEO aberto em: {alias}"),
            Err(error) => error,
        };
    }

    pub fn current_notes(&self) -> &[UNote] {
        let track_idx = self.active_track_index;
        if let Some(part) = self
            .project
            .parts
            .iter()
            .find(|p| p.track_index == track_idx)
        {
            &part.notes
        } else if !self.project.parts.is_empty() {
            &self.project.parts[0].notes
        } else {
            &[]
        }
    }

    pub fn push_history(&mut self) {
        self.undo_manager.push_state(self.project.clone());
        self.is_dirty = true;
    }

    pub fn copy_selected_notes(&mut self) {
        let selected_indices = self.piano_roll_state.selected_note_indices.clone();
        let notes = self.current_notes();

        let indices_to_copy: Vec<usize> = if !selected_indices.is_empty() {
            let mut v: Vec<usize> = selected_indices.into_iter().collect();
            v.sort();
            v
        } else if let Some(sel_idx) = self.piano_roll_state.selected_note_index {
            if sel_idx < notes.len() {
                vec![sel_idx]
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        if !indices_to_copy.is_empty() {
            self.clipboard = indices_to_copy
                .iter()
                .filter_map(|&idx| notes.get(idx).cloned())
                .collect();
            self.transport_state.status_message =
                format!("{} nota(s) copiada(s)", self.clipboard.len());
        }
    }

    pub fn cut_selected_notes(&mut self) {
        self.copy_selected_notes();
        if !self.clipboard.is_empty() {
            let count = self.clipboard.len();
            self.delete_selected_notes();
            self.transport_state.status_message = format!("{} nota(s) recortada(s)", count);
        }
    }

    pub fn delete_selected_notes(&mut self) {
        let selected_indices = self.piano_roll_state.selected_note_indices.clone();
        let sel_idx_opt = self.piano_roll_state.selected_note_index;
        let total_notes = self.current_notes().len();

        let mut to_delete: Vec<usize> = if !selected_indices.is_empty() {
            selected_indices.into_iter().collect()
        } else if let Some(sel_idx) = sel_idx_opt {
            if sel_idx < total_notes {
                vec![sel_idx]
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        if !to_delete.is_empty() {
            self.push_history();
            to_delete.sort_by(|a, b| b.cmp(a));
            let notes = self.current_notes_mut();
            for d_idx in to_delete {
                if d_idx < notes.len() {
                    notes.remove(d_idx);
                }
            }
            self.piano_roll_state.selected_note_index = None;
            self.piano_roll_state.selected_note_indices.clear();
        }
    }

    pub fn paste_notes(&mut self) {
        if self.clipboard.is_empty() {
            return;
        }

        self.push_history();
        let target_pos = self.piano_roll_state.playhead_ms.max(0.0);
        let min_pos = self
            .clipboard
            .iter()
            .map(|n| n.position_ms)
            .fold(f64::INFINITY, f64::min);
        let offset = if min_pos.is_finite() {
            target_pos - min_pos
        } else {
            0.0
        };

        let start_idx = self.current_notes().len();
        let mut pasted_notes = Vec::new();

        for note in &self.clipboard {
            let mut pasted = note.clone();
            pasted.position_ms = (pasted.position_ms + offset).max(0.0);
            pasted_notes.push(pasted);
        }

        let new_count = pasted_notes.len();
        self.current_notes_mut().extend(pasted_notes);

        self.piano_roll_state.selected_note_indices.clear();
        for i in 0..new_count {
            self.piano_roll_state
                .selected_note_indices
                .insert(start_idx + i);
        }
        if new_count > 0 {
            self.piano_roll_state.selected_note_index = Some(start_idx);
        }

        self.transport_state.status_message = format!("{} nota(s) colada(s)", new_count);
    }

    pub fn transpose_selected_note(&mut self, semitones: i32) {
        let selected_indices = self.piano_roll_state.selected_note_indices.clone();
        let min_m = self.piano_roll_state.min_midi as i32;
        let max_m = self.piano_roll_state.max_midi as i32;

        let target_indices: Vec<usize> = if !selected_indices.is_empty() {
            selected_indices.into_iter().collect()
        } else if let Some(sel_idx) = self.piano_roll_state.selected_note_index {
            vec![sel_idx]
        } else {
            vec![]
        };

        if !target_indices.is_empty() {
            self.push_history();
            let notes = self.current_notes_mut();
            for idx in target_indices {
                if idx < notes.len() {
                    let cur_m = notes[idx].midi_key() as i32;
                    let new_m = (cur_m + semitones).clamp(min_m, max_m) as u8;
                    notes[idx].set_midi_key(new_m);
                }
            }
        }
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
        let loaded = match extension.as_str() {
            "aps" => ApsFormat::load_file(path),
            "mid" | "midi" => MidiFormat::load_file(path),
            "ust" => UstFormat::load_file(path),
            "ustx" => UstxFormat::load_file(path),
            "ufdata" => UfdataFormat::load_file(path),
            "svp" => SvpFormat::load_file(path),
            "vsqx" | "vsq" => VsqxFormat::load_file(path),
            "json" => UfdataFormat::load_file(path)
                .or_else(|_| SvpFormat::load_file(path))
                .or_else(|_| ApsFormat::load_file(path)),
            _ => ApsFormat::load_file(path)
                .or_else(|_| UstxFormat::load_file(path))
                .or_else(|_| UfdataFormat::load_file(path))
                .or_else(|_| UstFormat::load_file(path))
                .or_else(|_| SvpFormat::load_file(path))
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

    fn sync_project_meta_before_save(&mut self) {
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
                track.singer = vb.name.clone();
                track.voicebank_path = Some(vb.root_path.to_string_lossy().to_string());
            }
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

    pub fn export_midi_dialog(&mut self) {
        self.sync_project_meta_before_save();
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Exportar Arquivo MIDI")
            .set_file_name("export.mid")
            .add_filter("Arquivo MIDI (*.mid, *.midi)", &["mid", "midi"])
            .save_file()
        {
            if let Err(e) = MidiFormat::save_file(&self.project, &path) {
                self.transport_state.status_message = format!("Erro ao exportar MIDI: {}", e);
            } else {
                self.transport_state.status_message =
                    format!("MIDI exportado: {:?}", path.file_name().unwrap_or_default());
            }
        }
    }

    pub fn export_ust_dialog(&mut self) {
        self.sync_project_meta_before_save();
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Exportar Sequência UTAU")
            .set_file_name("export.ust")
            .add_filter("Sequência UTAU (*.ust)", &["ust"])
            .save_file()
        {
            if let Err(e) = UstFormat::save_file(&self.project, &path) {
                self.transport_state.status_message = format!("Erro ao exportar UST: {}", e);
            } else {
                self.transport_state.status_message =
                    format!("UST exportado: {:?}", path.file_name().unwrap_or_default());
            }
        }
    }

    pub fn export_ustx_dialog(&mut self) {
        self.sync_project_meta_before_save();
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Exportar Projeto OpenUTAU")
            .set_file_name("export.ustx")
            .add_filter("Projeto OpenUTAU (*.ustx)", &["ustx"])
            .save_file()
        {
            if let Err(e) = UstxFormat::save_file(&self.project, &path) {
                self.transport_state.status_message = format!("Erro ao exportar USTX: {}", e);
            } else {
                self.transport_state.status_message =
                    format!("USTX exportado: {:?}", path.file_name().unwrap_or_default());
            }
        }
    }

    pub fn export_ufdata_dialog(&mut self) {
        self.sync_project_meta_before_save();
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Exportar UtaFormatix Data")
            .set_file_name("export.ufdata")
            .add_filter("UtaFormatix Data (*.ufdata, *.json)", &["ufdata", "json"])
            .save_file()
        {
            if let Err(e) = UfdataFormat::save_file(&self.project, &path) {
                self.transport_state.status_message = format!("Erro ao exportar UFData: {}", e);
            } else {
                self.transport_state.status_message = format!(
                    "UFData exportado: {:?}",
                    path.file_name().unwrap_or_default()
                );
            }
        }
    }

    pub fn export_svp_dialog(&mut self) {
        self.sync_project_meta_before_save();
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Exportar Projeto Synthesizer V")
            .set_file_name("export.svp")
            .add_filter("Projeto Synthesizer V (*.svp)", &["svp"])
            .save_file()
        {
            if let Err(e) = SvpFormat::save_file(&self.project, &path) {
                self.transport_state.status_message = format!("Erro ao exportar SVP: {}", e);
            } else {
                self.transport_state.status_message =
                    format!("SVP exportado: {:?}", path.file_name().unwrap_or_default());
            }
        }
    }

    pub fn export_vsqx_dialog(&mut self) {
        self.sync_project_meta_before_save();
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Exportar Sequência Vocaloid")
            .set_file_name("export.vsqx")
            .add_filter("Sequência Vocaloid (*.vsqx)", &["vsqx"])
            .save_file()
        {
            if let Err(e) = VsqxFormat::save_file(&self.project, &path) {
                self.transport_state.status_message = format!("Erro ao exportar VSQX: {}", e);
            } else {
                self.transport_state.status_message =
                    format!("VSQX exportado: {:?}", path.file_name().unwrap_or_default());
            }
        }
    }

    pub fn import_midi_dialog(&mut self) {
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Importar Arquivo MIDI")
            .add_filter("Arquivo MIDI (*.mid, *.midi)", &["mid", "midi"])
            .pick_file()
        {
            self.open_project_from_path(&path);
        }
    }

    pub fn import_ust_dialog(&mut self) {
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Importar Sequência UTAU")
            .add_filter("Sequência UTAU (*.ust)", &["ust"])
            .pick_file()
        {
            self.open_project_from_path(&path);
        }
    }

    pub fn import_ustx_dialog(&mut self) {
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Importar Projeto OpenUTAU")
            .add_filter("Projeto OpenUTAU (*.ustx)", &["ustx"])
            .pick_file()
        {
            self.open_project_from_path(&path);
        }
    }

    pub fn import_ufdata_dialog(&mut self) {
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Importar UtaFormatix Data")
            .add_filter("UtaFormatix Data (*.ufdata, *.json)", &["ufdata", "json"])
            .pick_file()
        {
            self.open_project_from_path(&path);
        }
    }

    pub fn import_svp_dialog(&mut self) {
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Importar Projeto Synthesizer V")
            .add_filter("Projeto Synthesizer V (*.svp)", &["svp"])
            .pick_file()
        {
            self.open_project_from_path(&path);
        }
    }

    pub fn import_vsqx_dialog(&mut self) {
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Importar Sequência Vocaloid")
            .add_filter("Sequência Vocaloid (*.vsqx, *.vsq)", &["vsqx", "vsq"])
            .pick_file()
        {
            self.open_project_from_path(&path);
        }
    }

    pub fn import_audio_track_dialog(&mut self) {
        if let Some(path) = crate::dialogs::FileDialog::new()
            .set_title("Importar Faixa de Áudio")
            .add_filter(
                "Áudio (*.wav, *.mp3, *.ogg, *.flac)",
                &["wav", "mp3", "ogg", "flac"],
            )
            .pick_file()
        {
            self.push_history();
            let file_stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("Audio Track")
                .to_string();
            let file_name = path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("Audio Track")
                .to_string();
            let file_path_str = path.to_string_lossy().to_string();

            let new_idx = self.project.tracks.len();
            self.project.tracks.push(crate::project::model::UTrack {
                name: file_stem,
                singer: "Instrumental / Áudio".to_string(),
                volume_db: 0.0,
                pan: 0.0,
                mute: false,
                solo: false,
                ..crate::project::model::UTrack::default()
            });
            let wave = crate::project::model::UWavePart::new(file_name, file_path_str, new_idx);
            self.project.wave_parts.push(wave);
            self.active_track_index = new_idx;
            self.transport_state.status_message =
                "Faixa de áudio adicionada com sucesso!".to_string();
        }
    }

    pub fn rephonemize_all_notes(&mut self) {
        self.push_history();
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
        let vb = self.voicebank.as_ref().unwrap_or(&dummy_vb);
        let mode = self.vocal_mode_params.phonemizer_mode;

        for part in self.project.parts.iter_mut() {
            let phones =
                crate::phonemizer::JapanesePhonemizer::apply_phonemizer(&part.notes, vb, mode);
            for (idx, phone) in phones.iter().enumerate() {
                if idx < part.notes.len() {
                    part.notes[idx].lyric = phone.lyric.clone();
                }
            }
        }
        self.piano_roll_state.phoneme_cache.clear();
        self.transport_state.status_message =
            "Todas as notas foram refonetizadas com sucesso!".to_string();
    }

    pub fn clear_all_pitch_curves(&mut self) {
        self.push_history();
        for part in self.project.parts.iter_mut() {
            for note in part.notes.iter_mut() {
                note.pitch_bend.points.clear();
                note.vibrato.length_pct = 0.0;
            }
        }
        self.transport_state.status_message = "Curvas de pitch e vibrato limpas!".to_string();
    }

    pub fn reset_all_volume_envelopes(&mut self) {
        self.push_history();
        for part in self.project.parts.iter_mut() {
            for note in part.notes.iter_mut() {
                note.envelope = crate::dsp::envelope::UtauEnvelope::default();
            }
        }
        self.transport_state.status_message =
            "Envelopes de volume resetados para o padrão!".to_string();
    }

    pub fn apply_autopitch_all(&mut self) {
        self.push_history();
        for part in self.project.parts.iter_mut() {
            let all_indices: std::collections::HashSet<usize> = (0..part.notes.len()).collect();
            crate::gui::piano_roll::apply_autopitch_to_selection(
                &mut part.notes,
                &all_indices,
                crate::gui::piano_roll::AutoPitchStyle::SmoothPop,
            );
        }
        self.transport_state.status_message =
            "AutoPitch Suave/Pop aplicado a todas as notas!".to_string();
    }

    pub fn play_current_track(&mut self) {
        if self.render_rx.is_some() {
            self.transport_state.status_message = "A prévia já está sendo renderizada".to_string();
            return;
        }

        if self.project.parts.iter().all(|part| part.notes.is_empty())
            && self.project.wave_parts.is_empty()
        {
            self.transport_state.status_message =
                "Nenhum áudio ou nota para reproduzir".to_string();
            return;
        }

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
        let render_threads = self.render_threads.clamp(1, 16) as usize;
        let mut project = self.project.clone();
        project.bpm = self.transport_state.bpm;
        if self.transport_state.preview_selection_only {
            let selected = &self.piano_roll_state.selected_note_indices;
            if !selected.is_empty() {
                let active_part_idx = self
                    .project
                    .parts
                    .iter()
                    .position(|part| part.track_index == self.active_track_index)
                    .unwrap_or(0);
                for (part_idx, part) in project.parts.iter_mut().enumerate() {
                    if part_idx == active_part_idx {
                        part.notes = part
                            .notes
                            .iter()
                            .enumerate()
                            .filter(|(index, _)| selected.contains(index))
                            .map(|(_, note)| note.clone())
                            .collect();
                    } else {
                        part.notes.clear();
                    }
                }
            }
        }
        let mut max_project_end = project
            .parts
            .iter()
            .flat_map(|part| {
                part.notes
                    .iter()
                    .map(move |note| part.position_ms + note.position_ms + note.duration_ms)
            })
            .fold(0.0f64, f64::max);

        for wave in &project.wave_parts {
            let dur = if wave.duration_ms > 0.0 {
                wave.duration_ms
            } else {
                30_000.0
            };
            max_project_end = max_project_end.max(wave.position_ms + dur);
        }

        if max_project_end <= 0.0 {
            self.transport_state.status_message =
                "Nenhum áudio ou nota para reproduzir".to_string();
            return;
        }
        let mut playhead_ms = self.piano_roll_state.playhead_ms;
        if playhead_ms >= max_project_end {
            playhead_ms = 0.0;
            self.piano_roll_state.playhead_ms = 0.0;
        }

        self.render_log_window_open = false;
        self.render_progress = 0.0;
        self.render_status_title = format!("⚡ {} • {:.0}ms", resampler_driver.name(), playhead_ms);

        let (tx, rx) = std::sync::mpsc::channel();
        self.render_log_channel_rx = Some(rx);
        let (audio_tx, audio_rx) = std::sync::mpsc::sync_channel(1);
        self.render_rx = Some(audio_rx);
        let cancel = Arc::new(AtomicBool::new(false));
        self.render_cancel = Some(cancel.clone());
        self.playback_start_offset_ms = playhead_ms;
        self.piano_roll_state.is_playing = false;
        self.piano_roll_state.rendered_waveform_peaks.clear();
        self.playback_start_instant = None;
        let tx = Arc::new(std::sync::Mutex::new(tx));
        let tx_cb = tx.clone();
        std::thread::spawn(move || {
            let report_progress = move |progress, message: &str| {
                if let Ok(guard) = tx_cb.lock() {
                    let _ = guard.send((progress, message.to_string()));
                }
            };

            let render = || {
                ProjectRenderer::render_project_range_with_drivers_cancellable(
                    &project,
                    &active_vb,
                    sample_rate,
                    0.0,
                    Some(max_project_end + 250.0),
                    resampler_driver.as_ref(),
                    wavtool_driver.as_ref(),
                    &vocal_mode_params,
                    Some(&report_progress),
                    Some(cancel.as_ref()),
                )
            };
            let render_pool = rayon::ThreadPoolBuilder::new()
                .num_threads(render_threads)
                .thread_name(|index| format!("kamafeu-render-{index}"))
                .build();
            let rendered = match render_pool {
                Ok(pool) => pool.install(render),
                Err(_) => render(),
            };
            if !cancel.load(Ordering::Relaxed) {
                let _ = audio_tx.send(PreviewRender { audio: rendered });
            }
        });
    }

    pub fn pause_audio(&mut self) {
        if let Some(cancel) = self.render_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.audio_player.stop();
        self.piano_roll_state.is_playing = false;
        self.playback_start_instant = None;
        self.render_rx = None;
        self.transport_state.status_message = "Pausado".to_string();
    }

    pub fn stop_audio(&mut self) {
        self.pause_audio();
        self.piano_roll_state.playhead_ms = 0.0;
        self.transport_state.status_message = "Parado".to_string();
    }

    pub fn export_wav(&mut self) {
        if !self.project.wave_parts.is_empty() {
            self.export_options_dialog_open = true;
        } else {
            self.execute_export_wav(ExportAudioScope::VocalsOnly);
        }
    }

    pub fn execute_export_wav(&mut self, scope: ExportAudioScope) {
        let default_name = match scope {
            ExportAudioScope::VocalsOnly if !self.project.wave_parts.is_empty() => "vocals.wav",
            _ => "output.wav",
        };

        if let Some(save_path) = crate::dialogs::FileDialog::new()
            .add_filter("WAV Audio", &["wav"])
            .set_file_name(default_name)
            .save_file()
        {
            let has_notes = self.project.parts.iter().any(|part| !part.notes.is_empty());
            let has_waves = !self.project.wave_parts.is_empty();

            if !has_notes && (scope == ExportAudioScope::VocalsOnly || !has_waves) {
                self.transport_state.status_message = "Nenhuma nota para exportar".to_string();
                return;
            }

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
                    "Iniciando exportação (Apenas Vocais / Acapella)...".to_string()
                }
                ExportAudioScope::VocalsAndAudio => {
                    "Iniciando exportação (Vocais + Áudios Importados)...".to_string()
                }
            };
            self.export_result = None;

            self.render_log_window_open = false;
            self.render_progress = 0.0;
            let start_log = format!(
                "[Export] Iniciando exportação ({:?}) para {:?}...",
                scope, save_path
            );
            self.render_log_messages.push(start_log);
            self.render_status_title = format!(
                "Exportando WAV ({})",
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
                let audio = match render_pool {
                    Ok(pool) => pool.install(render),
                    Err(_) => render(),
                };

                let result = TrackRenderer::save_wav_samples_with_channels(
                    &save_path,
                    &audio.samples,
                    audio.sample_rate,
                    audio.channels,
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
                        report_progress(1.0, &format!("[Export ERROR] {}", error));
                    }
                }
                let _ = export_tx.send(result);
            });
        }
    }

    pub fn preview_tone(&mut self, freq: f64) {
        let sample_rate = 44100;
        let num_samples = (sample_rate as f64 * 0.3) as usize;
        let mut raw_samples: Vec<f32> = (0..num_samples)
            .map(|i| {
                (i as f64 * 2.0 * std::f64::consts::PI * freq / sample_rate as f64).sin() as f32
                    * 0.4
            })
            .collect();

        let len = raw_samples.len();
        for (i, sample) in raw_samples.iter_mut().enumerate() {
            let fade = (len - i) as f32 / len as f32;
            *sample *= fade;
        }

        self.audio_player.play_samples(raw_samples, sample_rate);
    }

    fn draw_led_marquee(&self, ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();

        ui.painter().rect_filled(
            rect,
            egui::Rounding::same(3.0),
            egui::Color32::from_rgb(18, 16, 26),
        );
        ui.painter().rect_stroke(
            rect,
            egui::Rounding::same(3.0),
            egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(45, 40, 60)),
        );

        ui.horizontal(|ui| {
            ui.add_space(8.0);

            let is_rendering = self.render_progress < 0.99;
            let is_playing = self.audio_player.is_playing();

            let (status_dot_color, status_text) = if is_rendering {
                (
                    egui::Color32::from_rgb(0, 229, 255), // Ciano
                    "⚡ RENDERIZANDO ÁUDIO",
                )
            } else if is_playing {
                (
                    egui::Color32::from_rgb(0, 255, 157), // Verde
                    "▶ REPRODUZINDO",
                )
            } else {
                (
                    egui::Color32::from_rgb(140, 220, 100), // Verde suave
                    "● PRONTO",
                )
            };

            ui.label(
                egui::RichText::new(status_text)
                    .strong()
                    .size(11.0)
                    .color(status_dot_color),
            );

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            let vb_name = self
                .voicebank
                .as_ref()
                .map(|v| v.name.as_str())
                .unwrap_or("Nenhum Voicebank");

            let resampler_short = if self.selected_resampler.contains("straycat") {
                "straycat-rs"
            } else if self.selected_resampler.contains("TD-PSOLA") {
                "TD-PSOLA Nativo"
            } else if self.selected_resampler.contains("SOLA") {
                "SOLA Nativo"
            } else {
                &self.selected_resampler
            };

            let phonemizer_label = match self.vocal_mode_params.phonemizer_mode {
                crate::phonemizer::PhonemizerMode::None => "Manual (Sem Fonemizador)",
                crate::phonemizer::PhonemizerMode::BasicCV => "JA: Basic CV",
                crate::phonemizer::PhonemizerMode::VCV => "JA: VCV",
                crate::phonemizer::PhonemizerMode::CVVC => "JA: CVVC",
                crate::phonemizer::PhonemizerMode::EnglishArpasing => "EN: Arpasing (Fonética)",
                crate::phonemizer::PhonemizerMode::EnglishVCCV => "EN: VCCV (Fonética)",
                crate::phonemizer::PhonemizerMode::EnglishG2P => "EN: English G2P",
                crate::phonemizer::PhonemizerMode::PortugueseBrapaVCCV => "PT: VCCV BRAPA (xiao)",
                crate::phonemizer::PhonemizerMode::PortugueseBrapaCVC => "PT: BRAPA CVC (Fonética)",
                crate::phonemizer::PhonemizerMode::PortugueseCVVC => "PT: CVVC (Fonética)",
                crate::phonemizer::PhonemizerMode::PortugueseVCV => "PT: VCV (Fonética)",
                crate::phonemizer::PhonemizerMode::PortugueseG2P => "PT: Português G2P",
            };

            ui.label(
                egui::RichText::new("Voicebank:")
                    .size(10.5)
                    .color(egui::Color32::from_rgb(160, 155, 180)),
            );
            ui.label(
                egui::RichText::new(vb_name)
                    .strong()
                    .size(10.5)
                    .color(egui::Color32::WHITE),
            );

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            ui.label(
                egui::RichText::new("Engine:")
                    .size(10.5)
                    .color(egui::Color32::from_rgb(160, 155, 180)),
            );
            ui.label(
                egui::RichText::new(resampler_short)
                    .strong()
                    .size(10.5)
                    .color(egui::Color32::WHITE),
            );

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);

            ui.label(
                egui::RichText::new("Fonetizador:")
                    .size(10.5)
                    .color(egui::Color32::from_rgb(160, 155, 180)),
            );
            ui.label(
                egui::RichText::new(phonemizer_label)
                    .strong()
                    .size(10.5)
                    .color(egui::Color32::from_rgb(255, 220, 120)),
            );

            if is_playing {
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);
                let cur_ms = self.piano_roll_state.playhead_ms.max(0.0);
                let mins = (cur_ms / 60000.0) as usize;
                let secs = ((cur_ms % 60000.0) / 1000.0) as usize;
                let ms_rem = (cur_ms % 1000.0) as usize;
                ui.label(
                    egui::RichText::new(format!("Playhead: {:02}:{:02}.{:03}", mins, secs, ms_rem))
                        .strong()
                        .size(10.5)
                        .color(egui::Color32::from_rgb(0, 255, 157)),
                );
            }

            if self.transport_state.metronome_enabled {
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);
                let beat_ms = 60_000.0 / self.transport_state.bpm.max(1.0);
                let beat = ((self.piano_roll_state.playhead_ms / beat_ms).floor() as i64)
                    .rem_euclid(4)
                    + 1;
                let accent = if beat == 1 { "●" } else { "○" };
                ui.label(
                    egui::RichText::new(format!("{accent} METRÔNOMO {beat}/4"))
                        .size(10.0)
                        .color(if beat == 1 {
                            egui::Color32::from_rgb(255, 215, 100)
                        } else {
                            egui::Color32::from_rgb(180, 175, 195)
                        }),
                );
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);

                let fps = if self.frame_time_ema_ms > 0.0 {
                    1_000.0 / self.frame_time_ema_ms
                } else {
                    0.0
                };
                let fps_color = if fps >= 55.0 {
                    egui::Color32::from_rgb(120, 230, 150)
                } else if fps >= 30.0 {
                    egui::Color32::from_rgb(255, 210, 100)
                } else {
                    egui::Color32::from_rgb(255, 120, 120)
                };
                ui.label(
                    egui::RichText::new(format!("{fps:.0} FPS • {:.1} ms", self.frame_time_ema_ms))
                        .monospace()
                        .size(9.5)
                        .color(fps_color),
                )
                .on_hover_text("Média móvel do tempo de quadro da interface");
                ui.separator();

                let (progress_fraction, progress_color, pct_text) = if is_rendering {
                    let p = self.render_progress.clamp(0.0, 1.0) as f32;
                    (
                        p,
                        egui::Color32::from_rgb(0, 229, 255),
                        format!("{:.0}%", p * 100.0),
                    )
                } else if is_playing {
                    let total_ms = self
                        .project
                        .parts
                        .iter()
                        .flat_map(|p| p.notes.iter())
                        .map(|n| n.position_ms + n.duration_ms)
                        .fold(0.0_f64, f64::max)
                        .max(1000.0);
                    let cur_ms = self.piano_roll_state.playhead_ms.max(0.0);
                    let p = (cur_ms / total_ms).clamp(0.0, 1.0) as f32;
                    (
                        p,
                        egui::Color32::from_rgb(0, 255, 157),
                        format!("{:.0}%", p * 100.0),
                    )
                } else {
                    (
                        1.0,
                        egui::Color32::from_rgb(90, 80, 120),
                        "100%".to_string(),
                    )
                };

                ui.label(
                    egui::RichText::new(pct_text)
                        .strong()
                        .size(10.5)
                        .color(egui::Color32::WHITE),
                );

                ui.add_space(4.0);

                let bar_width = 180.0;
                let bar_height = 8.0;
                let (bar_rect, _) = ui.allocate_exact_size(
                    egui::Vec2::new(bar_width, bar_height),
                    egui::Sense::hover(),
                );

                ui.painter().rect_filled(
                    bar_rect,
                    egui::Rounding::same(4.0),
                    egui::Color32::from_rgb(30, 25, 40),
                );

                if progress_fraction > 0.001 {
                    let filled_width = bar_width * progress_fraction;
                    let filled_rect = egui::Rect::from_min_size(
                        bar_rect.min,
                        egui::Vec2::new(filled_width, bar_height),
                    );
                    ui.painter().rect_filled(
                        filled_rect,
                        egui::Rounding::same(4.0),
                        progress_color,
                    );
                }

                ui.painter().rect_stroke(
                    bar_rect,
                    egui::Rounding::same(4.0),
                    egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(55, 48, 72)),
                );

                if is_rendering {
                    ui.add_space(6.0);
                    ui.label(
                        egui::RichText::new(&self.render_status_title)
                            .size(9.5)
                            .color(egui::Color32::from_rgb(0, 200, 230)),
                    );
                }
            });
        });
    }

    fn draw_mini_log_window(&mut self, ctx: &egui::Context) {
        if !self.render_log_window_open {
            return;
        }

        let mut is_open = self.render_log_window_open;

        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("engine_log_native_viewport"),
            egui::ViewportBuilder::default()
                .with_title("⚡ Kamafeu Studio Engine Log & Terminal")
                .with_inner_size([680.0, 420.0])
                .with_min_inner_size([400.0, 240.0]),
            |ctx, _class| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading(
                            egui::RichText::new("⚡ Kamafeu Studio Engine Log")
                                .strong()
                                .size(14.0)
                                .color(egui::Color32::from_rgb(0, 255, 157)),
                        );

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .button(egui::RichText::new("🗑️ Limpar").size(11.0))
                                .clicked()
                            {
                                self.render_log_messages.clear();
                            }
                            if ui
                                .button(egui::RichText::new("📋 Copiar Log").size(11.0))
                                .clicked()
                            {
                                let full_log = self.render_log_messages.join("\n");
                                ui.output_mut(|o| o.copied_text = full_log);
                            }
                        });
                    });

                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.add(
                            egui::ProgressBar::new(self.render_progress)
                                .text(format!("{:.0}%", self.render_progress * 100.0))
                                .fill(egui::Color32::from_rgb(0, 230, 138))
                                .animate(self.render_progress < 1.0)
                                .desired_width(ui.available_width() - 140.0),
                        );
                        ui.label(
                            egui::RichText::new(&self.render_status_title)
                                .size(11.0)
                                .color(egui::Color32::from_rgb(216, 180, 254)),
                        );
                    });

                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("Filtro:")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(180, 170, 190)),
                        );
                        if ui
                            .selectable_label(
                                self.render_log_filter == RenderLogFilter::All,
                                "Todos",
                            )
                            .clicked()
                        {
                            self.render_log_filter = RenderLogFilter::All;
                        }
                        if ui
                            .selectable_label(
                                self.render_log_filter == RenderLogFilter::ErrorsWarnings,
                                "⚠️ Erros",
                            )
                            .clicked()
                        {
                            self.render_log_filter = RenderLogFilter::ErrorsWarnings;
                        }
                        if ui
                            .selectable_label(
                                self.render_log_filter == RenderLogFilter::DspResampler,
                                "🎛️ DSP",
                            )
                            .clicked()
                        {
                            self.render_log_filter = RenderLogFilter::DspResampler;
                        }
                        if ui
                            .selectable_label(
                                self.render_log_filter == RenderLogFilter::WavOto,
                                "🔊 WAV",
                            )
                            .clicked()
                        {
                            self.render_log_filter = RenderLogFilter::WavOto;
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.checkbox(&mut self.auto_scroll_log, "Rolar auto.");
                        });
                    });

                    ui.add_space(4.0);

                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(12, 10, 18))
                        .rounding(egui::Rounding::same(4.0))
                        .stroke(egui::Stroke::new(
                            1.0_f32,
                            egui::Color32::from_rgb(45, 35, 65),
                        ))
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("render_log_native_scroll")
                                .stick_to_bottom(self.auto_scroll_log)
                                .show(ui, |ui| {
                                    if self.render_log_messages.is_empty() {
                                        ui.label(
                                            egui::RichText::new(
                                                "Aguardando tarefas de renderização...",
                                            )
                                            .size(11.0)
                                            .italics()
                                            .color(egui::Color32::from_rgb(120, 110, 140)),
                                        );
                                    } else {
                                        for msg in &self.render_log_messages {
                                            let matches_filter = match self.render_log_filter {
                                                RenderLogFilter::All => true,
                                                RenderLogFilter::ErrorsWarnings => {
                                                    msg.contains("FAILED")
                                                        || msg.contains("ERROR")
                                                        || msg.contains("warning")
                                                        || msg.contains("fallback")
                                                }
                                                RenderLogFilter::DspResampler => {
                                                    msg.contains("[Resampler]")
                                                        || msg.contains("[Wavtool]")
                                                        || msg.contains("[DSP]")
                                                }
                                                RenderLogFilter::WavOto => {
                                                    msg.contains("[WAV]")
                                                        || msg.contains("[Render]")
                                                        || msg.contains("oto=")
                                                }
                                            };

                                            if matches_filter {
                                                let text_color = if msg.contains("FAILED")
                                                    || msg.contains("ERROR")
                                                {
                                                    egui::Color32::from_rgb(255, 100, 100)
                                                } else if msg.contains("[WAV]") {
                                                    egui::Color32::from_rgb(180, 230, 255)
                                                } else if msg.contains("[Resampler]")
                                                    || msg.contains("[Wavtool]")
                                                {
                                                    egui::Color32::from_rgb(216, 180, 254)
                                                } else if msg.contains("Truncated")
                                                    || msg.contains("Concluído")
                                                {
                                                    egui::Color32::from_rgb(0, 255, 157)
                                                } else {
                                                    egui::Color32::from_rgb(200, 190, 220)
                                                };
                                                ui.label(
                                                    egui::RichText::new(msg)
                                                        .size(10.5)
                                                        .monospace()
                                                        .color(text_color),
                                                );
                                            }
                                        }
                                    }
                                });
                        });
                });

                if ctx.input(|i| i.viewport().close_requested()) {
                    is_open = false;
                }
            },
        );

        self.render_log_window_open = is_open;
    }
}

impl eframe::App for KamafeuStudioApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let frame_ms = now.duration_since(self.last_frame_instant).as_secs_f32() * 1_000.0;
        self.last_frame_instant = now;
        if frame_ms.is_finite() && frame_ms < 1_000.0 {
            self.frame_time_ema_ms = self.frame_time_ema_ms * 0.9 + frame_ms * 0.1;
        }
        let active_scale = self.config.ui_scale_factor.clamp(0.7, 2.0);
        if (ctx.zoom_factor() - active_scale).abs() > 0.001 {
            ctx.set_zoom_factor(active_scale);
        }

        let project_name = if let Some(ref path) = self.current_project_path {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("project.aps")
                .to_string()
        } else {
            let stem = self.project.name.trim();
            if stem.is_empty() {
                "Novo Projeto".to_string()
            } else {
                stem.to_string()
            }
        };
        let window_title = if self.is_dirty {
            format!("* {} - Kamafeu Studio", project_name)
        } else {
            format!("{} - Kamafeu Studio", project_name)
        };
        if window_title != self.last_window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(window_title.clone()));
            self.last_window_title = window_title;
        }

        // Keep the editor invariant: every active track owns an editable part.
        let _ = self.current_notes_mut();
        self.refresh_voicebank_oto();

        if let Some(ref rx) = self.render_log_channel_rx {
            // Keep UI frames responsive even if a renderer emits a burst of logs.
            for _ in 0..64 {
                let Ok((prog, msg)) = rx.try_recv() else {
                    break;
                };
                self.render_progress = prog;
                self.transport_state.render_progress = prog;
                if self.export_in_progress {
                    self.export_progress = prog;
                    self.export_status_detail = msg.clone();
                } else if self.render_rx.is_some() {
                    self.transport_state.status_message = msg.clone();
                    self.render_status_title = format!("⚡ {:.0}% • {}", prog * 100.0, msg);
                }
                self.render_log_messages.push(msg);
                if self.render_log_messages.len() > 300 {
                    self.render_log_messages.remove(0);
                }
            }
        }

        if let Some(ref rx) = self.export_rx {
            if let Ok(result) = rx.try_recv() {
                self.export_in_progress = false;
                self.export_progress = 1.0;
                self.transport_state.status_message = match &result {
                    Ok(()) => "Exportação WAV concluída!".to_string(),
                    Err(error) => format!("Erro na exportação WAV: {error}"),
                };
                self.export_result = Some(result);
                self.export_rx = None;
            }
        }

        let preview_message = self.render_rx.as_ref().map(|rx| rx.try_recv());
        match preview_message {
            Some(Ok(preview)) => {
                let mut audio = preview.audio;
                eprintln!(
                    "[Kamafeu] Received complete preview ({} frames)",
                    audio.frame_count(),
                );
                self.piano_roll_state.update_rendered_waveform(
                    &audio.samples,
                    audio.sample_rate,
                    audio.channels,
                    0.0,
                );

                let playback_sample = playback_sample_offset(&audio, self.playback_start_offset_ms);
                let playback_samples = audio.samples.split_off(playback_sample);
                self.audio_player.play_samples_with_channels(
                    playback_samples,
                    audio.sample_rate,
                    audio.channels,
                );
                self.piano_roll_state.is_playing = true;
                self.playback_start_instant = Some(Instant::now());
                self.transport_state.render_progress = 1.0;
                self.render_progress = 1.0;
                self.transport_state.status_message = "Tocando prévia completa...".to_string();
            }
            Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
                self.render_rx = None;
                self.render_cancel = None;
            }
            _ => {}
        }

        if self.piano_roll_state.is_playing {
            if let Some(start_t) = self.playback_start_instant {
                let elapsed_ms = start_t.elapsed().as_secs_f64() * 1000.0;
                self.piano_roll_state.playhead_ms = self.playback_start_offset_ms + elapsed_ms;

                let max_end_ms = self
                    .project
                    .parts
                    .iter()
                    .flat_map(|part| {
                        part.notes
                            .iter()
                            .map(move |note| part.position_ms + note.position_ms + note.duration_ms)
                    })
                    .fold(0.0f64, f64::max);

                if elapsed_ms > 1000.0
                    && self.piano_roll_state.playhead_ms > max_end_ms + 1000.0
                    && !self.audio_player.is_playing()
                {
                    self.pause_audio();
                }

                if self.transport_state.loop_enabled
                    && self.transport_state.loop_end_ms > self.transport_state.loop_start_ms
                    && self.piano_roll_state.playhead_ms >= self.transport_state.loop_end_ms
                {
                    self.pause_audio();
                    self.piano_roll_state.playhead_ms = self.transport_state.loop_start_ms;
                    self.playback_start_offset_ms = self.transport_state.loop_start_ms;
                    self.play_current_track();
                }
            }
            ctx.request_repaint();
        } else if self.piano_roll_state.is_scrubbing_ruler {
            ctx.request_repaint();
        }

        if self.render_rx.is_some() || self.export_rx.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }

        let cur_ms = self.piano_roll_state.playhead_ms.max(0.0);
        let total_sec = (cur_ms / 1000.0) as u32;
        let mins = total_sec / 60;
        let secs = total_sec % 60;
        let ms_rem = (cur_ms % 1000.0) as u32;
        self.transport_state.playhead_time_str = format!("{:02}:{:02}.{:03}", mins, secs, ms_rem);

        let is_editing_lyric = self.piano_roll_state.editing_lyric_index.is_some();
        if !is_editing_lyric {
            let mut toggle_play = false;
            ctx.input(|i| {
                if i.key_pressed(Key::Space) {
                    toggle_play = true;
                }
            });
            if toggle_play {
                if self.piano_roll_state.is_playing || self.render_rx.is_some() {
                    self.pause_audio();
                } else {
                    self.play_current_track();
                }
            }
        }

        let is_editing_lyric = self.piano_roll_state.editing_lyric_index.is_some();

        if !is_editing_lyric {
            let mut do_undo = false;
            let mut do_redo = false;
            let mut do_cut = false;
            let mut do_copy = false;
            let mut do_paste = false;
            let mut do_duplicate = false;
            let mut do_delete = false;
            let mut do_new = false;
            let mut do_open = false;
            let mut do_save = false;
            let mut do_save_as = false;
            let mut do_export = false;
            let mut transpose_semitones: i32 = 0;
            let mut nudge_ms: f64 = 0.0;
            let mut duration_nudge_ms: f64 = 0.0;

            let mut do_toggle_log = false;
            let mut do_select_all = false;
            let mut do_deselect_all = false;
            let mut do_toggle_drawer = false;
            let mut do_toggle_mute = false;
            let mut do_reset_zoom = false;
            let mut do_open_help = false;

            ctx.input(|i| {
                let has_cmd_or_ctrl = i.modifiers.command || i.modifiers.ctrl;

                if has_cmd_or_ctrl && i.key_pressed(Key::Z) {
                    if i.modifiers.shift {
                        do_redo = true;
                    } else {
                        do_undo = true;
                    }
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::Y) {
                    do_redo = true;
                }

                if has_cmd_or_ctrl && i.key_pressed(Key::N) {
                    do_new = true;
                } else if i.key_pressed(Key::N) || i.key_pressed(Key::Num2) {
                    self.piano_roll_state.active_tool = EditTool::Pencil;
                }
                if i.key_pressed(Key::V) || i.key_pressed(Key::Num1) {
                    self.piano_roll_state.active_tool = EditTool::Pointer;
                }
                if i.key_pressed(Key::P) || i.key_pressed(Key::Num3) {
                    if i.modifiers.shift && self.piano_roll_state.active_tool == EditTool::PitchDraw
                    {
                        self.piano_roll_state.pitch_sub_tool =
                            match self.piano_roll_state.pitch_sub_tool {
                                crate::gui::types::PitchSubTool::Freehand => {
                                    crate::gui::types::PitchSubTool::Line
                                }
                                crate::gui::types::PitchSubTool::Line => {
                                    crate::gui::types::PitchSubTool::Vibrato
                                }
                                crate::gui::types::PitchSubTool::Vibrato => {
                                    crate::gui::types::PitchSubTool::Smooth
                                }
                                crate::gui::types::PitchSubTool::Smooth => {
                                    crate::gui::types::PitchSubTool::Freehand
                                }
                            };
                    } else {
                        self.piano_roll_state.active_tool = EditTool::PitchDraw;
                    }
                }
                if i.key_pressed(Key::C) && !has_cmd_or_ctrl || i.key_pressed(Key::Num4) {
                    self.piano_roll_state.active_tool = EditTool::Slice;
                }
                if i.key_pressed(Key::E) && !has_cmd_or_ctrl || i.key_pressed(Key::Num5) {
                    self.piano_roll_state.active_tool = EditTool::Eraser;
                }
                if has_cmd_or_ctrl && i.modifiers.shift && i.key_pressed(Key::L) {
                    self.batch_lyrics_open = true;
                }
                if has_cmd_or_ctrl && i.modifiers.alt && i.key_pressed(Key::P) {
                    self.autopitch_window_open = true;
                }

                if has_cmd_or_ctrl && i.key_pressed(Key::O) {
                    do_open = true;
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::S) {
                    if i.modifiers.shift {
                        do_save_as = true;
                    } else {
                        do_save = true;
                    }
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::E) {
                    do_export = true;
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::L) {
                    do_toggle_log = true;
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::A) {
                    if i.modifiers.shift {
                        do_deselect_all = true;
                    } else {
                        do_select_all = true;
                    }
                }
                if i.key_pressed(Key::F1) || (has_cmd_or_ctrl && i.key_pressed(Key::Slash)) {
                    do_open_help = true;
                }
                if i.key_pressed(Key::Tab) {
                    do_toggle_drawer = true;
                }
                if i.key_pressed(Key::M) && !has_cmd_or_ctrl {
                    do_toggle_mute = true;
                }

                if has_cmd_or_ctrl && i.key_pressed(Key::X) {
                    do_cut = true;
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::C) {
                    do_copy = true;
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::V) {
                    do_paste = true;
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::D) {
                    do_duplicate = true;
                }
                if i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace) {
                    do_delete = true;
                }

                let step = if i.modifiers.shift { 12 } else { 1 };
                if i.key_pressed(Key::ArrowUp) {
                    transpose_semitones += step;
                }
                if i.key_pressed(Key::ArrowDown) {
                    transpose_semitones -= step;
                }

                if i.modifiers.shift {
                    if i.key_pressed(Key::ArrowLeft) {
                        duration_nudge_ms -= 50.0;
                    }
                    if i.key_pressed(Key::ArrowRight) {
                        duration_nudge_ms += 50.0;
                    }
                } else {
                    if i.key_pressed(Key::ArrowLeft) {
                        nudge_ms -= 50.0;
                    }
                    if i.key_pressed(Key::ArrowRight) {
                        nudge_ms += 50.0;
                    }
                }

                if has_cmd_or_ctrl && (i.key_pressed(Key::Equals) || i.key_pressed(Key::Plus)) {
                    self.piano_roll_state.px_per_ms =
                        (self.piano_roll_state.px_per_ms * 1.25).min(1.0);
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::Minus) {
                    self.piano_roll_state.px_per_ms =
                        (self.piano_roll_state.px_per_ms * 0.8).max(0.05);
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::Num0) {
                    do_reset_zoom = true;
                }
            });

            if do_new {
                self.new_project();
            }
            if do_open {
                self.open_project_dialog();
            }
            if do_save {
                self.save_project();
            }
            if do_save_as {
                self.save_project_as_dialog();
            }
            if do_export {
                self.export_wav();
            }
            if do_toggle_log {
                self.render_log_window_open = !self.render_log_window_open;
            }
            if do_open_help {
                self.shortcuts_guide_open = true;
            }
            if do_toggle_drawer {
                self.piano_roll_state.show_parameters_drawer =
                    !self.piano_roll_state.show_parameters_drawer;
            }
            if do_toggle_mute {
                if let Some(track) = self.project.tracks.get_mut(self.active_track_index) {
                    track.mute = !track.mute;
                }
            }
            if do_reset_zoom {
                self.piano_roll_state.px_per_ms = 0.25;
                self.piano_roll_state.row_height = 22.0;
            }
            if do_select_all {
                let note_count = self.current_notes().len();
                self.piano_roll_state.selected_note_indices = (0..note_count).collect();
                self.piano_roll_state.selected_note_index =
                    if note_count > 0 { Some(0) } else { None };
            }
            if do_deselect_all {
                self.piano_roll_state.selected_note_indices.clear();
                self.piano_roll_state.selected_note_index = None;
            }

            if do_undo {
                self.pending_edit_snapshot = None;
                if let Some(prev) = self.undo_manager.undo(self.project.clone()) {
                    self.project = prev;
                    self.transport_state.status_message = "Desfeito (Undo)".to_string();
                }
            }
            if do_redo {
                self.pending_edit_snapshot = None;
                if let Some(next) = self.undo_manager.redo(self.project.clone()) {
                    self.project = next;
                    self.transport_state.status_message = "Refeito (Redo)".to_string();
                }
            }
            if do_cut {
                self.cut_selected_notes();
            }
            if do_copy {
                self.copy_selected_notes();
            }
            if do_paste {
                self.paste_notes();
            }
            if do_duplicate {
                if let Some(sel_idx) = self.piano_roll_state.selected_note_index {
                    let notes = self.current_notes();
                    if sel_idx < notes.len() {
                        let mut dup = notes[sel_idx].clone();
                        dup.position_ms += dup.duration_ms;
                        self.push_history();
                        self.current_notes_mut().push(dup);
                        self.piano_roll_state.selected_note_index =
                            Some(self.current_notes().len() - 1);
                    }
                }
            }
            if do_delete {
                self.delete_selected_notes();
            }
            if transpose_semitones != 0 {
                self.transpose_selected_note(transpose_semitones);
            }
            if nudge_ms != 0.0 {
                if let Some(sel_idx) = self.piano_roll_state.selected_note_index {
                    self.push_history();
                    let notes = self.current_notes_mut();
                    if sel_idx < notes.len() {
                        notes[sel_idx].position_ms =
                            (notes[sel_idx].position_ms + nudge_ms).max(0.0);
                    }
                }
            }
            if duration_nudge_ms != 0.0 {
                if let Some(sel_idx) = self.piano_roll_state.selected_note_index {
                    self.push_history();
                    let notes = self.current_notes_mut();
                    if sel_idx < notes.len() {
                        notes[sel_idx].duration_ms =
                            (notes[sel_idx].duration_ms + duration_nudge_ms).max(50.0);
                    }
                }
            }
        }

        self.render_menu_bar(ctx);

        TopBottomPanel::top("top_unified_control_panel")
            .exact_height(78.0)
            .frame(Frame::none().fill(MelodyneTheme::BG_PANEL))
            .show(ctx, |ui| {
                let transport_active = self.audio_player.is_playing() || self.render_rx.is_some();
                let bpm_before = self.project.bpm;
                let mut play_clicked = false;
                let mut stop_clicked = false;
                let mut export_clicked = false;

                draw_unified_toolbar(
                    ui,
                    &mut self.transport_state,
                    transport_active,
                    &mut self.render_log_window_open,
                    &mut self.piano_roll_state.active_tool,
                    &mut self.piano_roll_state.pitch_sub_tool,
                    &mut self.piano_roll_state.auto_scroll_mode,
                    &mut self.piano_roll_state.px_per_ms,
                    &mut self.piano_roll_state.row_height,
                    &mut || play_clicked = true,
                    &mut || stop_clicked = true,
                    &mut || export_clicked = true,
                    &mut || self.copaiba_window_open = true,
                    &mut || self.autopitch_window_open = true,
                );

                self.audio_player
                    .set_volume(self.transport_state.master_volume);

                let requested_bpm = self.transport_state.bpm;
                if (requested_bpm - bpm_before).abs() > f64::EPSILON {
                    let was_active = self.audio_player.is_playing() || self.render_rx.is_some();
                    if was_active {
                        self.pause_audio();
                    }
                    if let Some(time_scale) = self.project.set_bpm_preserving_beats(requested_bpm) {
                        self.piano_roll_state.playhead_ms *= time_scale;
                        self.playback_start_offset_ms *= time_scale;
                        self.transport_state.bpm = self.project.bpm;
                        self.transport_state.status_message = if was_active {
                            format!(
                                "BPM alterado para {:.0}; reprodução interrompida",
                                self.project.bpm
                            )
                        } else {
                            format!("BPM alterado para {:.0}", self.project.bpm)
                        };
                    } else {
                        self.transport_state.bpm = self.project.bpm;
                        self.transport_state.status_message = "BPM inválido".to_string();
                    }
                }

                if play_clicked {
                    if transport_active {
                        self.pause_audio();
                    } else {
                        self.play_current_track();
                    }
                }

                if stop_clicked {
                    self.stop_audio();
                }

                if export_clicked {
                    self.export_wav();
                }
            });

        TopBottomPanel::top("top_led_marquee_panel")
            .exact_height(24.0)
            .frame(Frame::none().fill(MelodyneTheme::BG_PANEL))
            .show(ctx, |ui| {
                self.draw_led_marquee(ui);
            });

        SidePanel::right("right_inspector_panel")
            .resizable(true)
            .default_width(240.0)
            .frame(Frame::none().fill(MelodyneTheme::BG_PANEL))
            .show(ctx, |ui| {
                let selected_indices = self.piano_roll_state.selected_note_indices.clone();
                let selected_idx = self.piano_roll_state.selected_note_index;
                let active_track = self.active_track_index;
                if self.project.parts.is_empty() {
                    self.project
                        .parts
                        .push(crate::project::model::UVoicePart::new("Part 1", 0));
                }
                let part_idx = self
                    .project
                    .parts
                    .iter()
                    .position(|p| p.track_index == active_track)
                    .unwrap_or(0);

                let mut loaded_vb: Option<Voicebank> = None;
                let mut preview_alias: Option<String> = None;
                let mut insert_alias: Option<String> = None;
                let mut edit_alias: Option<String> = None;

                let notes = &mut self.project.parts[part_idx].notes[..];

                let mut open_singers_gallery = false;
                let mut reload_singers_flag = false;
                let mut add_singers_dir_flag = false;
                let mut open_folder_picker = false;

                draw_unified_panel(
                    ui,
                    self.voicebank.as_ref(),
                    &self.config.recent_voicebanks,
                    &self.singers_list,
                    &mut self.singer_search_query,
                    &mut self.config.singers_paths,
                    &mut self.vocal_mode_params,
                    selected_idx,
                    notes,
                    &selected_indices,
                    &mut self.right_sidebar_tab,
                    &mut self.phoneme_palette_state,
                    &mut self.render_threads,
                    &mut self.sample_rate,
                    &mut self.selected_resampler,
                    &mut self.selected_wavtool,
                    &mut self.custom_resampler_path,
                    &mut self.custom_wavtool_path,
                    &mut self.config.discord_rpc_enabled,
                    &mut |opt_path| {
                        if let Some(p) = opt_path {
                            if let Ok(vb) = Voicebank::new(&p) {
                                loaded_vb = Some(vb);
                            }
                        } else {
                            #[cfg(not(target_os = "android"))]
                            {
                                if let Some(folder) =
                                    crate::dialogs::FileDialog::new().pick_folder()
                                {
                                    if let Ok(vb) = Voicebank::new(&folder) {
                                        loaded_vb = Some(vb);
                                    }
                                } else {
                                    open_folder_picker = true;
                                }
                            }
                            #[cfg(target_os = "android")]
                            {
                                open_folder_picker = true;
                            }
                        }
                    },
                    &mut || add_singers_dir_flag = true,
                    &mut || reload_singers_flag = true,
                    &mut || open_singers_gallery = true,
                    &mut |alias| preview_alias = Some(alias.to_string()),
                    &mut |alias| insert_alias = Some(alias.to_string()),
                    &mut |alias| edit_alias = Some(alias.to_string()),
                );

                if open_folder_picker {
                    self.folder_picker_open = true;
                }

                if add_singers_dir_flag {
                    #[cfg(not(target_os = "android"))]
                    if let Some(folder) = crate::dialogs::FileDialog::new().pick_folder() {
                        if !self.config.singers_paths.contains(&folder) {
                            self.config.singers_paths.push(folder);
                            self.persist_config();
                            self.reload_singers();
                        }
                    } else {
                        self.folder_picker_open = true;
                    }
                    #[cfg(target_os = "android")]
                    {
                        self.folder_picker_open = true;
                    }
                }

                if reload_singers_flag {
                    self.persist_config();
                    self.reload_singers();
                }

                if open_singers_gallery {
                    self.singers_gallery_window_open = true;
                }

                self.persist_config();

                if let Some(vb) = loaded_vb {
                    self.transport_state.status_message =
                        format!("Voicebank Carregado: {}", vb.name);
                    self.transport_state.voicebank_name = vb.name.clone();
                    self.transport_state.voicebank_path = Some(vb.root_path.clone());
                    self.config.add_recent_voicebank(vb.root_path.clone());
                    self.voicebank = Some(vb);
                }

                if let Some(alias) = preview_alias {
                    let mut played = false;
                    if let Some(ref vb) = self.voicebank {
                        if let Some(entry) = vb
                            .find_entry(&alias, "C4")
                            .or_else(|| vb.find_entry(&alias, "A3"))
                        {
                            let wav_path = vb.root_path.join(&entry.wav_filename);
                            if let Ok((samples, sr)) = TrackRenderer::load_wav_samples(&wav_path) {
                                let max_s = (sr as usize).min(samples.len());
                                self.audio_player
                                    .play_samples(samples[..max_s].to_vec(), sr);
                                played = true;
                            }
                        }
                    }
                    if !played {
                        self.preview_tone(440.0);
                    }
                }

                if let Some(alias) = insert_alias {
                    self.push_history();
                    let playhead_ms = self.piano_roll_state.playhead_ms;
                    let sel_idx = self.piano_roll_state.selected_note_index;
                    let notes_mut = self.current_notes_mut();
                    if let Some(idx) = sel_idx {
                        if idx < notes_mut.len() {
                            notes_mut[idx].lyric = alias;
                        }
                    } else {
                        let new_note = UNote::new(&alias, "C4", playhead_ms, 400.0);
                        notes_mut.push(new_note);
                    }
                }

                #[cfg(not(target_os = "android"))]
                if let Some(alias) = edit_alias {
                    self.open_copaiba_for_alias(&alias);
                }
            });

        if self.piano_roll_state.show_arrangement_view {
            TopBottomPanel::top("arrangement_multitrack_panel")
                .resizable(true)
                .height_range(60.0..=500.0)
                .default_height(self.piano_roll_state.arrangement_height)
                .frame(
                    Frame::none()
                        .fill(MelodyneTheme::BG_PANEL)
                        .stroke(egui::Stroke::new(1.5_f32, MelodyneTheme::ACCENT_GOLD)),
                )
                .show(ctx, |ui| {
                    let actual_h = ui.max_rect().height().clamp(60.0, 500.0);
                    self.piano_roll_state.arrangement_height = actual_h;

                    draw_arrangement_view(
                        ui,
                        &mut self.project.tracks,
                        &mut self.project.parts,
                        &mut self.project.wave_parts,
                        &mut self.active_track_index,
                        &mut self.piano_roll_state.playhead_ms,
                        self.piano_roll_state.px_per_ms,
                        self.transport_state.bpm,
                        &mut self.piano_roll_state.horizontal_scroll_offset,
                    );
                });
        }

        // Capture o projeto antes do evento que pode iniciar uma edição. O
        // callback do piano roll é processado depois do desenho do frame, quando
        // a nota já pode ter sido alterada; clonar ali produzia estados parciais.
        let piano_edit_snapshot = ctx
            .input(|input| {
                let pointer_started =
                    input.pointer.primary_pressed() || input.pointer.secondary_pressed();
                let edit_key_started = [
                    egui::Key::Enter,
                    egui::Key::Tab,
                    egui::Key::Delete,
                    egui::Key::Backspace,
                    egui::Key::ArrowUp,
                    egui::Key::ArrowDown,
                ]
                .into_iter()
                .any(|key| input.key_pressed(key));
                pointer_started || edit_key_started
            })
            .then(|| self.project.clone());

        CentralPanel::default()
            .frame(Frame::none().fill(MelodyneTheme::BG_CANVAS))
            .show(ctx, |ui| {
                let mut preview_freq: Option<f64> = None;
                let mut before_changed = false;
                let mut note_changed = false;
                let mut scrubbed_t: Option<f64> = None;

                let active_track = self.active_track_index;
                if self.project.parts.is_empty() {
                    self.project
                        .parts
                        .push(crate::project::model::UVoicePart::new("Part 1", 0));
                }
                let part_idx = self
                    .project
                    .parts
                    .iter()
                    .position(|p| p.track_index == active_track)
                    .unwrap_or(0);
                let active_notes = &mut self.project.parts[part_idx].notes;

                draw_piano_roll(
                    ui,
                    active_notes,
                    &mut self.piano_roll_state,
                    self.voicebank.as_ref(),
                    &mut self.phoneme_palette_state,
                    self.transport_state.grid_snap,
                    self.transport_state.bpm,
                    self.vocal_mode_params.phonemizer_mode,
                    &mut |freq| preview_freq = Some(freq),
                    &mut || before_changed = true,
                    &mut || note_changed = true,
                    &mut |t| scrubbed_t = Some(t),
                );

                if let Some(t) = scrubbed_t {
                    if self.piano_roll_state.is_playing {
                        self.stop_audio();
                    }
                    self.piano_roll_state.playhead_ms = t;
                    self.playback_start_offset_ms = t;
                }

                if let Some(freq) = preview_freq {
                    self.preview_tone(freq);
                }

                if before_changed {
                    if let Some(snapshot) = piano_edit_snapshot.clone() {
                        // Um novo gesto sempre substitui qualquer snapshot de um
                        // clique anterior que não chegou a modificar o projeto.
                        self.pending_edit_snapshot = Some(snapshot);
                    }
                }

                if note_changed {
                    if let Some(snapshot) = self.pending_edit_snapshot.take() {
                        self.undo_manager.push_state(snapshot);
                    } else if let Some(snapshot) = piano_edit_snapshot.clone() {
                        // Operações instantâneas (por exemplo, apagar um ponto)
                        // podem confirmar no mesmo frame sem callback inicial.
                        self.undo_manager.push_state(snapshot);
                    }
                }

                if note_changed || self.piano_roll_state.continuous_edit_dirty {
                    self.is_dirty = true;
                }

                if self.render_rx.is_some() {
                    let p = self.render_progress.clamp(0.0, 1.0);
                    let resampler_driver = self.create_resampler_driver();
                    let resampler_name = resampler_driver.name();
                    let painter = ui.painter();
                    let clip_rect = ui.clip_rect();
                    let center_x = clip_rect.center().x;
                    let hud_w = 440.0f32.min((clip_rect.width() - 40.0).max(100.0));
                    let hud_h = 60.0f32;
                    let hud_rect = egui::Rect::from_center_size(
                        egui::Pos2::new(center_x, clip_rect.min.y + 48.0),
                        egui::Vec2::new(hud_w, hud_h),
                    );

                    painter.rect_filled(
                        hud_rect,
                        egui::Rounding::same(8.0),
                        egui::Color32::from_rgba_premultiplied(16, 14, 26, 238),
                    );
                    painter.rect_stroke(
                        hud_rect,
                        egui::Rounding::same(8.0),
                        egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(0, 229, 255)),
                    );

                    let title_text = format!("⚡ Renderizando Prévia [{}]", resampler_name);
                    let pct_text = format!("{:.0}%", p * 100.0);

                    ui.allocate_new_ui(
                        egui::UiBuilder::new().max_rect(hud_rect.shrink(8.0)),
                        |hud_ui| {
                            hud_ui.horizontal(|h_ui| {
                                h_ui.label(
                                    egui::RichText::new(title_text)
                                        .size(12.0)
                                        .strong()
                                        .color(egui::Color32::from_rgb(0, 229, 255)),
                                );
                                h_ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |r_ui| {
                                        r_ui.label(
                                            egui::RichText::new(pct_text)
                                                .size(12.0)
                                                .strong()
                                                .color(egui::Color32::WHITE),
                                        );
                                        r_ui.label(
                                            egui::RichText::new("[Espaço para cancelar]")
                                                .size(9.5)
                                                .color(egui::Color32::from_rgb(160, 150, 180)),
                                        );
                                    },
                                );
                            });

                            hud_ui.add_space(4.0);
                            hud_ui.add(
                                egui::ProgressBar::new(p)
                                    .animate(true)
                                    .fill(egui::Color32::from_rgb(0, 230, 138))
                                    .desired_width(hud_w - 20.0),
                            );
                        },
                    );
                }
            });

        self.draw_mini_log_window(ctx);

        self.render_dialogs(ctx);

        let note_count = self.current_notes().len();
        let active_part_name = self
            .project
            .parts
            .iter()
            .find(|part| part.track_index == self.active_track_index)
            .or_else(|| self.project.parts.first())
            .map(|part| part.name.as_str())
            .unwrap_or("Parte vocal");
        let selected_count = self
            .piano_roll_state
            .selected_note_indices
            .len()
            .max(usize::from(
                self.piano_roll_state.selected_note_index.is_some(),
            ));

        let is_rendering = self.render_rx.is_some() || self.export_rx.is_some();
        let is_playing = self.piano_roll_state.is_playing;

        let rpc_state = crate::discord_rpc::activity_presentation(
            is_rendering,
            is_playing,
            self.render_progress,
            &self.project.name,
            active_part_name,
            self.voicebank
                .as_ref()
                .map(|v| v.name.as_str())
                .unwrap_or(""),
            note_count,
            selected_count,
            self.transport_state.bpm,
            self.config.discord_rpc_enabled,
        );

        self.discord_rpc.update(rpc_state);
    }
}

#[cfg(test)]
mod preview_tests {
    use super::*;

    #[test]
    fn playback_offset_uses_frames_channels_and_clamps() {
        let audio = RenderedAudio {
            samples: vec![0.0; 4_000],
            sample_rate: 1_000,
            channels: 2,
        };
        assert_eq!(playback_sample_offset(&audio, 750.0), 1_500);
        assert_eq!(playback_sample_offset(&audio, -100.0), 0);
        assert_eq!(playback_sample_offset(&audio, 3_000.0), 4_000);
    }
}
