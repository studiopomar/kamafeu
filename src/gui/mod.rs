pub mod arrangement;
pub mod fonts;
pub mod history;
pub mod inspector;
pub mod left_panel;
pub mod phoneme_palette;
pub mod piano_roll;
pub mod right_panel;
pub mod theme;
pub mod toolbar;
pub mod transport;

use crate::config::KamafeuConfig;
use eframe::egui::{self, CentralPanel, Frame, Key, SidePanel, TopBottomPanel};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use crate::audio::AudioPlayer;
use crate::drivers::{
    ExternalResamplerDriver, ExternalWavtoolDriver, KnownResampler, MacResDriver,
    NativeResamplerDriver, NativeWavtoolDriver, ResamplerDriver, WavtoolDriver, WavtoolYawuDriver,
};
use crate::formats::{MidiFormat, UstFormat, UstxFormat};
use crate::gui::arrangement::draw_arrangement_view;
use crate::gui::fonts::setup_custom_fonts;
use crate::gui::history::UndoManager;
use crate::gui::left_panel::{draw_left_panel, LeftSidebarTab, VocalModeParams};
use crate::gui::phoneme_palette::PhonemePaletteState;
use crate::gui::piano_roll::{draw_piano_roll, PianoRollState};
use crate::gui::right_panel::{draw_right_panel, RightSidebarTab};
use crate::gui::theme::MelodyneTheme;
use crate::gui::toolbar::{draw_toolbar, EditTool};
use crate::gui::transport::{draw_transport_bar, TransportState};
use crate::oto::Voicebank;
use crate::project::model::{UNote, UProject};
use crate::renderer::{ProjectRenderer, RenderedAudio, TrackRenderer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderLogFilter {
    All,
    ErrorsWarnings,
    DspResampler,
    WavOto,
}

pub struct KamafeuStudioApp {
    project: UProject,
    voicebank: Option<Voicebank>,
    piano_roll_state: PianoRollState,
    transport_state: TransportState,
    left_sidebar_tab: LeftSidebarTab,
    right_sidebar_tab: RightSidebarTab,
    vocal_mode_params: VocalModeParams,
    phoneme_palette_state: PhonemePaletteState,
    undo_manager: UndoManager,
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
    render_rx: Option<std::sync::mpsc::Receiver<RenderedAudio>>,
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
    copaiba_app: crate::copaiba::gui::CopaibaToolkitApp,
    copaiba_window_open: bool,
    shortcuts_guide_open: bool,
}

impl KamafeuStudioApp {
    fn create_resampler_driver(&self) -> Box<dyn ResamplerDriver> {
        if self.selected_resampler.contains("Native") || self.selected_resampler.contains("Nativo")
        {
            return Box::new(NativeResamplerDriver);
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

    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load Japanese CJK system fonts for egui
        setup_custom_fonts(&cc.egui_ctx);

        // Apply High-Contrast Dark Visuals Theme
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = MelodyneTheme::BG_PANEL;
        visuals.window_fill = MelodyneTheme::BG_PANEL;
        visuals.widgets.noninteractive.bg_fill = MelodyneTheme::BG_PANEL;
        visuals.widgets.noninteractive.fg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(220, 215, 235));

        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(32, 24, 46);
        visuals.widgets.inactive.fg_stroke =
            egui::Stroke::new(1.0, egui::Color32::from_rgb(210, 200, 230));

        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(48, 35, 72);
        visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.2, egui::Color32::WHITE);

        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(60, 42, 90);
        visuals.widgets.active.fg_stroke =
            egui::Stroke::new(1.5, egui::Color32::from_rgb(0, 255, 157));

        visuals.selection.bg_fill = egui::Color32::from_rgb(60, 42, 90);
        visuals.selection.stroke = egui::Stroke::new(1.2, egui::Color32::from_rgb(0, 255, 157));
        cc.egui_ctx.set_visuals(visuals);

        let project = crate::project::model::create_astro_boy_1980_project();

        let mut config = KamafeuConfig::load();

        // Automatically open the last used voicebank if available
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
        if let Some(ref vb) = voicebank {
            transport_state.voicebank_name = vb.name.clone();
            transport_state.voicebank_path = Some(vb.root_path.clone());
        }

        let default_resampler = if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            KnownResampler::Organum
        } else {
            KnownResampler::MacRes
        };
        let resampler_default_path = default_resampler
            .find_executable()
            .unwrap_or_else(|| default_resampler.default_path());
        let wavtool_default_path = PathBuf::from("./wavtools/wavtool-yawu");

        Self {
            project,
            voicebank,
            piano_roll_state: PianoRollState::default(),
            transport_state,
            left_sidebar_tab: LeftSidebarTab::default(),
            right_sidebar_tab: RightSidebarTab::default(),
            vocal_mode_params: VocalModeParams::default(),
            phoneme_palette_state: PhonemePaletteState::default(),
            undo_manager: UndoManager::default(),
            clipboard: Vec::new(),
            audio_player: AudioPlayer::new(),
            sample_rate: 44100,
            render_threads: 4,
            selected_resampler: default_resampler.label().to_string(),
            selected_wavtool: "wavtool-yawu (m13253/wavtool-yawu)".to_string(),
            custom_resampler_path: Some(resampler_default_path),
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
            config,
            copaiba_app: crate::copaiba::gui::CopaibaToolkitApp::default(),
            copaiba_window_open: false,
            shortcuts_guide_open: false,
        }
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
        self.transport_state.bpm = self.project.bpm;
        self.piano_roll_state.playhead_ms = 0.0;
        self.piano_roll_state.selected_note_index = None;
        self.piano_roll_state.selected_note_indices.clear();
        self.piano_roll_state.initial_scrolled = false;
        self.undo_manager = UndoManager::default();
        self.transport_state.status_message = "Novo projeto criado".to_string();
    }

    pub fn open_project_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Arquivos de Projeto e MIDI (.ustx, .ust, .mid, .midi)",
                &["ustx", "ust", "mid", "midi", "json"],
            )
            .pick_file()
        {
            let extension = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            let loaded = match extension.as_str() {
                "mid" | "midi" => MidiFormat::load_file(&path),
                "ust" => UstFormat::load_file(&path),
                _ => UstxFormat::load_file(&path),
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
                    self.transport_state.status_message =
                        format!("Projeto aberto: {:?}", path.file_name().unwrap_or_default());
                }
                Err(e) => {
                    self.transport_state.status_message = format!("Erro ao abrir projeto: {}", e);
                }
            }
        }
    }

    pub fn save_project_dialog(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name("project.ustx")
            .add_filter("Projeto OpenUTAU (*.ustx)", &["ustx"])
            .add_filter("Sequência UTAU (*.ust)", &["ust"])
            .add_filter("Arquivo MIDI Padrão (*.mid)", &["mid", "midi"])
            .save_file()
        {
            let ext = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            let res = match ext.as_str() {
                "mid" | "midi" => MidiFormat::save_file(&self.project, &path),
                "ust" => UstFormat::save_file(&self.project, &path),
                _ => UstxFormat::save_file(&self.project, &path),
            };

            if res.is_ok() {
                self.transport_state.status_message = format!(
                    "Projeto salvo em: {:?}",
                    path.file_name().unwrap_or_default()
                );
            } else if let Err(e) = res {
                self.transport_state.status_message = format!("Erro ao salvar projeto: {}", e);
            }
        }
    }

    pub fn play_current_track(&mut self) {
        if self.render_rx.is_some() {
            self.transport_state.status_message = "A prévia já está sendo renderizada".to_string();
            return;
        }

        if self.project.parts.iter().all(|part| part.notes.is_empty()) {
            self.transport_state.status_message = "Nenhuma nota para reproduzir".to_string();
            return;
        }

        let resampler_driver = self.create_resampler_driver();

        let native_wavtool = NativeWavtoolDriver;
        let wavtool_driver: Box<dyn WavtoolDriver> = if self.selected_wavtool.contains("yawu") {
            let path = self
                .custom_wavtool_path
                .clone()
                .unwrap_or_else(|| PathBuf::from("wavtool-yawu"));
            Box::new(WavtoolYawuDriver::new(path))
        } else if self.selected_wavtool.contains("Native") {
            Box::new(native_wavtool)
        } else {
            let path = self
                .custom_wavtool_path
                .clone()
                .unwrap_or_else(|| PathBuf::from(&self.selected_wavtool));
            Box::new(ExternalWavtoolDriver::new(path))
        };

        let max_note_end = self
            .project
            .parts
            .iter()
            .flat_map(|part| {
                part.notes
                    .iter()
                    .map(move |note| part.position_ms + note.position_ms + note.duration_ms)
            })
            .fold(0.0f64, f64::max);
        let mut playhead_ms = self.piano_roll_state.playhead_ms;
        if playhead_ms >= max_note_end {
            playhead_ms = 0.0;
            self.piano_roll_state.playhead_ms = 0.0;
        }
        let sample_rate = self.sample_rate;

        let dummy_vb = Voicebank {
            root_path: PathBuf::from("."),
            name: "Synthetic Fallback".to_string(),
            author: "System".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries: std::collections::HashMap::new(),
            prefix_map: crate::oto::PrefixMap::default(),
        };

        let active_vb = self.voicebank.clone().unwrap_or(dummy_vb);
        let vocal_mode_params = self.vocal_mode_params.clone();
        let mut project = self.project.clone();
        project.bpm = self.transport_state.bpm;

        self.render_log_window_open = true;
        self.render_progress = 0.0;
        self.render_status_title = format!("Renderizando prévia ({:.0}ms)...", playhead_ms);

        let (tx, rx) = std::sync::mpsc::channel();
        self.render_log_channel_rx = Some(rx);
        let (audio_tx, audio_rx) = std::sync::mpsc::channel();
        self.render_rx = Some(audio_rx);
        let cancel = Arc::new(AtomicBool::new(false));
        self.render_cancel = Some(cancel.clone());
        self.playback_start_offset_ms = playhead_ms;
        self.piano_roll_state.is_playing = false;
        self.playback_start_instant = None;
        self.transport_state.status_message = "Renderizando prévia multifaixa...".to_string();

        std::thread::spawn(move || {
            let rendered = ProjectRenderer::render_project_with_drivers_cancellable(
                &project,
                &active_vb,
                sample_rate,
                playhead_ms,
                resampler_driver.as_ref(),
                wavtool_driver.as_ref(),
                &vocal_mode_params,
                Some(&|progress, message| {
                    let _ = tx.send((progress, message.to_string()));
                }),
                Some(cancel.as_ref()),
            );
            if !cancel.load(Ordering::Relaxed) {
                let _ = audio_tx.send(rendered);
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
        if let Some(save_path) = rfd::FileDialog::new()
            .add_filter("WAV Audio", &["wav"])
            .set_file_name("output.wav")
            .save_file()
        {
            if self.project.parts.iter().all(|part| part.notes.is_empty()) {
                self.transport_state.status_message = "Nenhuma nota para exportar".to_string();
                return;
            }

            let resampler_driver = self.create_resampler_driver();

            let native_wavtool = NativeWavtoolDriver;
            let wavtool_driver: Box<dyn WavtoolDriver> = if self.selected_wavtool.contains("yawu") {
                let path = self
                    .custom_wavtool_path
                    .clone()
                    .unwrap_or_else(|| PathBuf::from("wavtool-yawu"));
                Box::new(WavtoolYawuDriver::new(path))
            } else if self.selected_wavtool.contains("Native") {
                Box::new(native_wavtool)
            } else {
                let path = self
                    .custom_wavtool_path
                    .clone()
                    .unwrap_or_else(|| PathBuf::from(&self.selected_wavtool));
                Box::new(ExternalWavtoolDriver::new(path))
            };

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
                prefix_map: crate::oto::PrefixMap::default(),
            };
            let active_vb = self.voicebank.clone().unwrap_or(dummy_vb);
            let vocal_mode_params = self.vocal_mode_params.clone();
            let mut project = self.project.clone();
            project.bpm = bpm;

            self.render_log_window_open = true;
            self.render_progress = 0.0;
            let start_log = format!("[Export] Iniciando exportação para {:?}...", save_path);
            self.render_log_messages.push(start_log);
            self.render_status_title = format!(
                "Exportando WAV ({})",
                save_path.file_name().unwrap_or_default().to_string_lossy()
            );

            let (tx, rx) = std::sync::mpsc::channel();
            self.render_log_channel_rx = Some(rx);

            let (export_tx, export_rx) = std::sync::mpsc::channel();
            self.export_rx = Some(export_rx);

            std::thread::spawn(move || {
                let audio = ProjectRenderer::render_project_with_drivers(
                    &project,
                    &active_vb,
                    sample_rate,
                    0.0,
                    resampler_driver.as_ref(),
                    wavtool_driver.as_ref(),
                    &vocal_mode_params,
                    Some(&|prog, msg| {
                        let _ = tx.send((prog, msg.to_string()));
                    }),
                );

                let result = TrackRenderer::save_wav_samples_with_channels(
                    &save_path,
                    &audio.samples,
                    audio.sample_rate,
                    audio.channels,
                );
                match &result {
                    Ok(()) => {
                        let _ = tx.send((
                            1.0,
                            format!(
                                "[Export Concluído] Áudio gravado com sucesso em {:?}",
                                save_path
                            ),
                        ));
                    }
                    Err(error) => {
                        let _ = tx.send((1.0, format!("[Export ERROR] {}", error)));
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

    fn draw_mini_log_window(&mut self, ctx: &egui::Context) {
        if !self.render_log_window_open {
            return;
        }

        let mut is_open = self.render_log_window_open;

        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("engine_log_native_viewport"),
            egui::ViewportBuilder::default()
                .with_title("⚡ Kamafeu Engine Log & Terminal")
                .with_inner_size([680.0, 420.0])
                .with_min_inner_size([400.0, 240.0]),
            |ctx, _class| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading(
                            egui::RichText::new("⚡ Kamafeu Engine Log")
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

                    // Animated Progress Bar
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

                    // Filter Tabs
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

                    // Monospace Console Box
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(12, 10, 18))
                        .rounding(egui::Rounding::same(4.0))
                        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(45, 35, 65)))
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
        // Keep the editor invariant: every active track owns an editable part.
        let _ = self.current_notes_mut();

        // Poll for render log and progress events
        if let Some(ref rx) = self.render_log_channel_rx {
            // Keep UI frames responsive even if a renderer emits a burst of logs.
            for _ in 0..64 {
                let Ok((prog, msg)) = rx.try_recv() else {
                    break;
                };
                self.render_progress = prog;
                self.transport_state.render_progress = prog;
                self.render_log_messages.push(msg);
                if self.render_log_messages.len() > 300 {
                    self.render_log_messages.remove(0);
                }
            }
        }

        if let Some(ref rx) = self.export_rx {
            if let Ok(result) = rx.try_recv() {
                self.transport_state.status_message = match result {
                    Ok(()) => "Exportação WAV concluída!".to_string(),
                    Err(error) => format!("Erro na exportação WAV: {error}"),
                };
                self.export_rx = None;
            }
        }

        // Poll for rendered audio from background thread
        if let Some(ref rx) = self.render_rx {
            if let Ok(audio) = rx.try_recv() {
                eprintln!(
                    "[Kamafeu] Main thread received {} rendered frames! Playing audio...",
                    audio.frame_count()
                );
                self.audio_player.play_samples_with_channels(
                    audio.samples,
                    audio.sample_rate,
                    audio.channels,
                );
                self.render_rx = None;
                self.render_cancel = None;
                self.piano_roll_state.is_playing = true;
                self.playback_start_instant = Some(Instant::now());
                self.transport_state.render_progress = 1.0;
                self.render_progress = 1.0;
                self.transport_state.status_message = "Tocando...".to_string();
            }
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
            }
            ctx.request_repaint();
        }

        if self.render_rx.is_some() || self.export_rx.is_some() {
            ctx.request_repaint();
        }

        let cur_ms = self.piano_roll_state.playhead_ms.max(0.0);
        let total_sec = (cur_ms / 1000.0) as u32;
        let mins = total_sec / 60;
        let secs = total_sec % 60;
        let ms_rem = (cur_ms % 1000.0) as u32;
        self.transport_state.playhead_time_str = format!("{:02}:{:02}.{:03}", mins, secs, ms_rem);

        // Space = Play/Pause — apenas pausa no ponto atual (sem resetar para 0)
        let is_editing_lyric = self.piano_roll_state.editing_lyric_index.is_some();
        if !is_editing_lyric {
            let mut toggle_play = false;
            ctx.input(|i| {
                if i.key_pressed(Key::Space) {
                    toggle_play = true;
                }
            });
            if toggle_play {
                if self.piano_roll_state.is_playing {
                    self.pause_audio();
                } else {
                    self.play_current_track();
                }
            }
        }

        // Keyboard Shortcuts System (Only process if not currently typing in a text widget or editing lyric)
        let is_editing_text =
            ctx.wants_keyboard_input() || self.piano_roll_state.editing_lyric_index.is_some();

        if !is_editing_text {
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
            let mut do_export = false;
            let mut transpose_semitones: i32 = 0;
            let mut nudge_ms: f64 = 0.0;
            let mut duration_nudge_ms: f64 = 0.0;

            let mut do_toggle_log = false;

            ctx.input(|i| {
                let has_cmd_or_ctrl = i.modifiers.command || i.modifiers.ctrl;

                if has_cmd_or_ctrl && i.key_pressed(Key::N) {
                    do_new = true;
                } else if i.key_pressed(Key::N) || i.key_pressed(Key::Num2) {
                    self.piano_roll_state.active_tool = EditTool::Pencil;
                }
                if i.key_pressed(Key::V) || i.key_pressed(Key::Num1) {
                    self.piano_roll_state.active_tool = EditTool::Pointer;
                }
                if i.key_pressed(Key::P) || i.key_pressed(Key::Num3) {
                    self.piano_roll_state.active_tool = EditTool::PitchDraw;
                }
                if i.key_pressed(Key::E) || i.key_pressed(Key::Num4) {
                    self.piano_roll_state.active_tool = EditTool::Eraser;
                }

                if has_cmd_or_ctrl && i.key_pressed(Key::O) {
                    do_open = true;
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::S) {
                    do_save = true;
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::E) {
                    do_export = true;
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::L) {
                    do_toggle_log = true;
                }

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
            });

            if do_new {
                self.new_project();
            }
            if do_open {
                self.open_project_dialog();
            }
            if do_save {
                self.save_project_dialog();
            }
            if do_export {
                self.export_wav();
            }
            if do_toggle_log {
                self.render_log_window_open = !self.render_log_window_open;
            }

            if do_undo {
                if let Some(prev) = self.undo_manager.undo(self.project.clone()) {
                    self.project = prev;
                }
            }
            if do_redo {
                if let Some(next) = self.undo_manager.redo(self.project.clone()) {
                    self.project = next;
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

        // 1. Top Menu Bar (Fixed 26px height)
        TopBottomPanel::top("top_menu_bar")
            .exact_height(26.0)
            .frame(Frame::none().fill(MelodyneTheme::BG_PANEL))
            .show(ctx, |ui| {
                egui::menu::bar(ui, |ui| {
                    ui.menu_button("Arquivo", |ui| {
                        if ui.button("Novo Projeto  (Ctrl+N / Cmd+N)").clicked() {
                            self.new_project();
                            ui.close_menu();
                        }
                        if ui.button("Abrir Projeto...  (Ctrl+O / Cmd+O)").clicked() {
                            self.open_project_dialog();
                            ui.close_menu();
                        }
                        if ui.button("Salvar Projeto... (Ctrl+S / Cmd+S)").clicked() {
                            self.save_project_dialog();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Carregar Voicebank...").clicked() {
                            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                                if let Ok(vb) = Voicebank::new(&folder) {
                                    self.transport_state.status_message =
                                        format!("Voicebank Carregado: {}", vb.name);
                                    self.transport_state.voicebank_name = vb.name.clone();
                                    self.transport_state.voicebank_path =
                                        Some(vb.root_path.clone());
                                    self.config.add_recent_voicebank(vb.root_path.clone());
                                    self.voicebank = Some(vb);
                                }
                            }
                            ui.close_menu();
                        }
                        if ui
                            .button("Exportar Áudio WAV... (Ctrl+E / Cmd+E)")
                            .clicked()
                        {
                            self.export_wav();
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Editar", |ui| {
                        if ui.button("Desfazer (Ctrl+Z / Cmd+Z)").clicked() {
                            if let Some(prev) = self.undo_manager.undo(self.project.clone()) {
                                self.project = prev;
                            }
                            ui.close_menu();
                        }
                        if ui.button("Refazer (Ctrl+Y / Cmd+Shift+Z)").clicked() {
                            if let Some(next) = self.undo_manager.redo(self.project.clone()) {
                                self.project = next;
                            }
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Recortar (Ctrl+X / Cmd+X)").clicked() {
                            self.cut_selected_notes();
                            ui.close_menu();
                        }
                        if ui.button("Copiar   (Ctrl+C / Cmd+C)").clicked() {
                            self.copy_selected_notes();
                            ui.close_menu();
                        }
                        if ui.button("Colar    (Ctrl+V / Cmd+V)").clicked() {
                            self.paste_notes();
                            ui.close_menu();
                        }
                        if ui.button("Duplicar (Ctrl+D / Cmd+D)").clicked() {
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
                            ui.close_menu();
                        }
                        if ui.button("Excluir  (Delete)").clicked() {
                            self.delete_selected_notes();
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Transpor +1 Semitom  (Seta Cima)").clicked() {
                            self.transpose_selected_note(1);
                            ui.close_menu();
                        }
                        if ui.button("Transpor -1 Semitom  (Seta Baixo)").clicked() {
                            self.transpose_selected_note(-1);
                            ui.close_menu();
                        }
                        if ui
                            .button("Transpor +1 Oitava   (Shift + Seta Cima)")
                            .clicked()
                        {
                            self.transpose_selected_note(12);
                            ui.close_menu();
                        }
                        if ui
                            .button("Transpor -1 Oitava   (Shift + Seta Baixo)")
                            .clicked()
                        {
                            self.transpose_selected_note(-12);
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Ferramentas", |ui| {
                        if ui.button("Ponteiro (Seleção) [V / 1]").clicked() {
                            self.piano_roll_state.active_tool = EditTool::Pointer;
                            ui.close_menu();
                        }
                        if ui.button("Lápis (Desenhar)   [N / 2]").clicked() {
                            self.piano_roll_state.active_tool = EditTool::Pencil;
                            ui.close_menu();
                        }
                        if ui.button("Desenhar Pitch     [P / 3]").clicked() {
                            self.piano_roll_state.active_tool = EditTool::PitchDraw;
                            ui.close_menu();
                        }
                        if ui.button("Borracha           [E / 4]").clicked() {
                            self.piano_roll_state.active_tool = EditTool::Eraser;
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui.button("Copaiba Voicebank Toolkit").clicked() {
                            self.copaiba_window_open = true;
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Exibir", |ui| {
                        if ui.button("Aumentar Zoom X  (Ctrl+=)").clicked() {
                            self.piano_roll_state.px_per_ms =
                                (self.piano_roll_state.px_per_ms * 1.25).min(1.0);
                            ui.close_menu();
                        }
                        if ui.button("Diminuir Zoom X  (Ctrl+-)").clicked() {
                            self.piano_roll_state.px_per_ms =
                                (self.piano_roll_state.px_per_ms * 0.8).max(0.05);
                            ui.close_menu();
                        }
                        ui.separator();
                        if ui
                            .checkbox(
                                &mut self.render_log_window_open,
                                "⚡ Janela de Log do Engine (Ctrl+L)",
                            )
                            .clicked()
                        {
                            ui.close_menu();
                        }
                    });

                    ui.menu_button("Ajuda", |ui| {
                        if ui.button("Guia de Teclas de Atalho...").clicked() {
                            self.shortcuts_guide_open = true;
                            ui.close_menu();
                        }
                    });
                });
            });

        // 2. Transport Bar Panel (Height: 32px)
        TopBottomPanel::top("top_transport_panel")
            .exact_height(32.0)
            .frame(Frame::none().fill(MelodyneTheme::BG_PANEL))
            .show(ctx, |ui| {
                let transport_active = self.audio_player.is_playing() || self.render_rx.is_some();
                let bpm_before = self.project.bpm;
                let mut play_clicked = false;
                let mut stop_clicked = false;
                let mut loaded_path: Option<PathBuf> = None;
                let mut export_clicked = false;

                draw_transport_bar(
                    ui,
                    &mut self.transport_state,
                    transport_active,
                    &mut self.render_log_window_open,
                    &mut || play_clicked = true,
                    &mut || stop_clicked = true,
                    &mut |p| loaded_path = Some(p),
                    &mut || export_clicked = true,
                );

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

                if let Some(path) = loaded_path {
                    match Voicebank::new(&path) {
                        Ok(vb) => {
                            self.transport_state.status_message =
                                format!("Voicebank Carregado: {}", vb.name);
                            self.transport_state.voicebank_name = vb.name.clone();
                            self.transport_state.voicebank_path = Some(vb.root_path.clone());
                            self.config.add_recent_voicebank(vb.root_path.clone());
                            self.voicebank = Some(vb);
                        }
                        Err(e) => {
                            self.transport_state.status_message =
                                format!("Erro ao carregar voicebank: {}", e);
                        }
                    }
                }
            });

        // 3. Edit Tools & Zoom Bar Panel (Height: 28px)
        TopBottomPanel::top("top_tools_panel")
            .exact_height(28.0)
            .frame(Frame::none().fill(MelodyneTheme::BG_PANEL))
            .show(ctx, |ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("top_tools_h_scroll")
                    .enable_scrolling(true)
                    .show(ui, |ui| {
                        draw_toolbar(
                            ui,
                            &mut self.piano_roll_state.active_tool,
                            &mut self.piano_roll_state.px_per_ms,
                            &mut self.piano_roll_state.row_height,
                            &mut || self.copaiba_window_open = true,
                        );
                    });
            });

        // 2. Left Panel (Singer Card, Vocal Mode Sliders & Phoneme Palette)
        SidePanel::left("left_voice_panel")
            .resizable(true)
            .default_width(240.0)
            .frame(Frame::none().fill(MelodyneTheme::BG_PANEL))
            .show(ctx, |ui| {
                let mut loaded_vb: Option<Voicebank> = None;
                let mut preview_alias: Option<String> = None;
                let mut insert_alias: Option<String> = None;

                draw_left_panel(
                    ui,
                    self.voicebank.as_ref(),
                    &self.config.recent_voicebanks,
                    &mut self.left_sidebar_tab,
                    &mut self.vocal_mode_params,
                    &mut self.phoneme_palette_state,
                    &mut |opt_path| {
                        let target_path = if let Some(p) = opt_path {
                            Some(p)
                        } else {
                            rfd::FileDialog::new().pick_folder()
                        };
                        if let Some(folder) = target_path {
                            if let Ok(vb) = Voicebank::new(&folder) {
                                loaded_vb = Some(vb);
                            }
                        }
                    },
                    &mut |alias| preview_alias = Some(alias.to_string()),
                    &mut |alias| insert_alias = Some(alias.to_string()),
                );

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
                    let notes = self.current_notes_mut();
                    if let Some(idx) = sel_idx {
                        if idx < notes.len() {
                            notes[idx].lyric = alias;
                        }
                    } else {
                        let new_note = UNote::new(&alias, "C4", playhead_ms, 400.0);
                        notes.push(new_note);
                    }
                }
            });

        // 3. Right Sidebar (Note Properties & Settings with Resampler/Wavtool Selectors)
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
                let notes = &mut self.project.parts[part_idx].notes[..];

                draw_right_panel(
                    ui,
                    self.voicebank.as_ref(),
                    selected_idx,
                    notes,
                    &selected_indices,
                    &mut self.right_sidebar_tab,
                    &mut self.render_threads,
                    &mut self.sample_rate,
                    &mut self.selected_resampler,
                    &mut self.selected_wavtool,
                    &mut self.custom_resampler_path,
                    &mut self.custom_wavtool_path,
                );
            });

        // 4. Center Workspace (Arrangement View & Piano Roll Canvas)
        CentralPanel::default()
            .frame(Frame::none().fill(MelodyneTheme::BG_CANVAS))
            .show(ctx, |ui| {
                draw_arrangement_view(
                    ui,
                    &mut self.project.tracks,
                    &mut self.project.parts,
                    &mut self.active_track_index,
                    &mut self.piano_roll_state.playhead_ms,
                    self.piano_roll_state.px_per_ms,
                    self.transport_state.bpm,
                );

                ui.add_space(2.0);
                ui.separator();
                ui.add_space(2.0);

                let mut preview_freq: Option<f64> = None;
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
                    &mut |freq| preview_freq = Some(freq),
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

                if note_changed {
                    self.push_history();
                }
            });

        self.draw_mini_log_window(ctx);

        if self.copaiba_window_open {
            let mut is_open = self.copaiba_window_open;
            egui::Window::new("Copaiba Voicebank Toolkit")
                .open(&mut is_open)
                .default_size([1000.0, 600.0])
                .show(ctx, |ui| {
                    crate::copaiba::gui::draw_copaiba_toolkit_ui(&mut self.copaiba_app, ui);
                });
            self.copaiba_window_open = is_open;
        }

        if self.shortcuts_guide_open {
            let mut is_open = self.shortcuts_guide_open;
            egui::Window::new("Guia de Teclas de Atalho")
                .open(&mut is_open)
                .default_size([550.0, 480.0])
                .show(ctx, |ui| {
                    ui.heading(
                        egui::RichText::new("Teclas de Atalho do Kamafeu Studio")
                            .strong()
                            .color(egui::Color32::from_rgb(0, 255, 157)),
                    );
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    egui::ScrollArea::vertical()
                        .id_salt("shortcuts_guide_scroll")
                        .show(ui, |ui| {
                            egui::Grid::new("shortcuts_grid")
                                .striped(true)
                                .spacing([20.0, 8.0])
                                .show(ui, |ui| {
                                    ui.label(
                                        egui::RichText::new("Atalho")
                                            .strong()
                                            .color(egui::Color32::from_rgb(255, 215, 0)),
                                    );
                                    ui.label(
                                        egui::RichText::new("Ação / Funcionalidade")
                                            .strong()
                                            .color(egui::Color32::from_rgb(255, 215, 0)),
                                    );
                                    ui.end_row();

                                    ui.label("Espaço (Space)");
                                    ui.label("Tocar / Pausar Reprodução");
                                    ui.end_row();
                                    ui.label("Esc");
                                    ui.label("Parar e Reiniciar Cursor no Início (0ms)");
                                    ui.end_row();
                                    ui.label("V ou 1");
                                    ui.label(
                                        "Ferramenta Ponteiro (Seleção / Mover / Redimensionar)",
                                    );
                                    ui.end_row();
                                    ui.label("N ou 2");
                                    ui.label("Ferramenta Lápis (Desenhar Notas)");
                                    ui.end_row();
                                    ui.label("P ou 3");
                                    ui.label("Ferramenta Desenhar Pitch");
                                    ui.end_row();
                                    ui.label("E ou 4");
                                    ui.label("Ferramenta Borracha (Apagar Notas)");
                                    ui.end_row();
                                    ui.label("Ctrl+Z / Cmd+Z");
                                    ui.label("Desfazer Ação");
                                    ui.end_row();
                                    ui.label("Ctrl+Y / Cmd+Shift+Z");
                                    ui.label("Refazer Ação");
                                    ui.end_row();
                                    ui.label("Ctrl+X / Cmd+X");
                                    ui.label("Recortar Nota(s) Selecionada(s)");
                                    ui.end_row();
                                    ui.label("Ctrl+C / Cmd+C");
                                    ui.label("Copiar Nota(s) Selecionada(s)");
                                    ui.end_row();
                                    ui.label("Ctrl+V / Cmd+V");
                                    ui.label("Colar Nota(s) na Posição do Cursor");
                                    ui.end_row();
                                    ui.label("Ctrl+D / Cmd+D");
                                    ui.label("Duplicar Nota(s)");
                                    ui.end_row();
                                    ui.label("Ctrl+A / Cmd+A");
                                    ui.label("Selecionar Todas as Notas");
                                    ui.end_row();
                                    ui.label("Delete / Backspace");
                                    ui.label("Excluir Nota(s) Selecionada(s)");
                                    ui.end_row();
                                    ui.label("Seta Cima / Baixo");
                                    ui.label("Transpor Nota +1 / -1 Semitom");
                                    ui.end_row();
                                    ui.label("Shift + Seta Cima / Baixo");
                                    ui.label("Transpor Nota +1 / -1 Oitava (+12 / -12 semitones)");
                                    ui.end_row();
                                    ui.label("Seta Esquerda / Direita");
                                    ui.label("Mover Posição da Nota (-50ms / +50ms)");
                                    ui.end_row();
                                    ui.label("Shift + Esquerda / Direita");
                                    ui.label("Redimensionar Duração da Nota (-50ms / +50ms)");
                                    ui.end_row();
                                    ui.label("Ctrl+N / Cmd+N");
                                    ui.label("Novo Projeto (Limpar / Criar projeto vazio)");
                                    ui.end_row();
                                    ui.label("Ctrl+O / Cmd+O");
                                    ui.label("Abrir Projeto (.ustx, .ust, .mid)");
                                    ui.end_row();
                                    ui.label("Ctrl+S / Cmd+S");
                                    ui.label("Salvar Projeto (.ustx)");
                                    ui.end_row();
                                    ui.label("Ctrl+E / Cmd+E");
                                    ui.label("Exportar Áudio WAV");
                                    ui.end_row();
                                    ui.label("Ctrl+L / Cmd+L");
                                    ui.label("Alternar Janela de Log do Engine (ON/OFF)");
                                    ui.end_row();
                                    ui.label("Ctrl+= / Cmd+=");
                                    ui.label("Aumentar Zoom Horizontal");
                                    ui.end_row();
                                    ui.label("Ctrl+- / Cmd+-");
                                    ui.label("Diminuir Zoom Horizontal");
                                    ui.end_row();
                                });
                        });
                });
            self.shortcuts_guide_open = is_open;
        }
    }
}
