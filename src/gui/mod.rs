pub mod arrangement;
pub mod fonts;
pub mod history;
pub mod inspector;
pub mod left_panel;
pub mod piano_roll;
pub mod phoneme_palette;
pub mod right_panel;
pub mod theme;
pub mod toolbar;
pub mod transport;

use std::path::PathBuf;
use std::time::Instant;
use eframe::egui::{self, TopBottomPanel, SidePanel, CentralPanel, Frame, Key};

use crate::audio::AudioPlayer;
use crate::drivers::{
    ExternalResamplerDriver, ExternalWavtoolDriver, MacResDriver, NativeResamplerDriver, NativeWavtoolDriver,
    ResamplerDriver, WavtoolDriver, WavtoolYawuDriver,
};
use crate::formats::{UstFormat, UstxFormat, MidiFormat};
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
use crate::renderer::TrackRenderer;

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
    render_rx: Option<std::sync::mpsc::Receiver<(Vec<f32>, u32)>>,
}

impl KamafeuStudioApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load Japanese CJK system fonts for egui
        setup_custom_fonts(&cc.egui_ctx);

        // Apply Melodyne Gold Visuals Theme
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = MelodyneTheme::BG_PANEL;
        visuals.window_fill = MelodyneTheme::BG_PANEL;
        visuals.widgets.noninteractive.bg_fill = MelodyneTheme::BG_PANEL;
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(36, 27, 53);
        visuals.selection.bg_fill = MelodyneTheme::NOTE_SELECTED_GOLD;
        visuals.selection.stroke.color = egui::Color32::from_rgb(0, 255, 157);
        cc.egui_ctx.set_visuals(visuals);

        let project = crate::project::model::create_astro_boy_1980_project();

        let voicebank = Voicebank::new("demo_vb")
            .or_else(|_| Voicebank::new("sample_vb"))
            .ok();

        let mut transport_state = TransportState::default();
        transport_state.bpm = project.bpm;
        if let Some(ref vb) = voicebank {
            transport_state.voicebank_name = vb.name.clone();
            transport_state.voicebank_path = Some(vb.root_path.clone());
        }

        let resampler_default_path = PathBuf::from("./resamplers/macres");
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
            selected_resampler: "macres (titinko/macres)".to_string(),
            selected_wavtool: "wavtool-yawu (m13253/wavtool-yawu)".to_string(),
            custom_resampler_path: Some(resampler_default_path),
            custom_wavtool_path: Some(wavtool_default_path),
            playback_start_instant: None,
            playback_start_offset_ms: 0.0,
            render_rx: None,
        }
    }

    pub fn current_notes_mut(&mut self) -> &mut Vec<UNote> {
        if self.project.parts.is_empty() {
            self.project.parts.push(crate::project::model::UVoicePart::new("Part 1", 0));
        }
        &mut self.project.parts[0].notes
    }

    pub fn current_notes(&self) -> &[UNote] {
        if self.project.parts.is_empty() {
            &[]
        } else {
            &self.project.parts[0].notes
        }
    }

    pub fn push_history(&mut self) {
        self.undo_manager.push_state(self.project.clone());
    }

    pub fn play_current_track(&mut self) {
        let notes_vec = self.current_notes().to_vec();
        if notes_vec.is_empty() {
            return;
        }

        let native_resampler = NativeResamplerDriver;
        let resampler_driver: Box<dyn ResamplerDriver> = if self.selected_resampler.contains("macres") {
            let path = self.custom_resampler_path.clone().unwrap_or_else(|| PathBuf::from("macres"));
            Box::new(MacResDriver::new(path))
        } else if self.selected_resampler.contains("Native") {
            Box::new(native_resampler)
        } else {
            let path = self.custom_resampler_path.clone().unwrap_or_else(|| PathBuf::from(&self.selected_resampler));
            Box::new(ExternalResamplerDriver::new(path))
        };

        let native_wavtool = NativeWavtoolDriver;
        let wavtool_driver: Box<dyn WavtoolDriver> = if self.selected_wavtool.contains("yawu") {
            let path = self.custom_wavtool_path.clone().unwrap_or_else(|| PathBuf::from("wavtool-yawu"));
            Box::new(WavtoolYawuDriver::new(path))
        } else if self.selected_wavtool.contains("Native") {
            Box::new(native_wavtool)
        } else {
            let path = self.custom_wavtool_path.clone().unwrap_or_else(|| PathBuf::from(&self.selected_wavtool));
            Box::new(ExternalWavtoolDriver::new(path))
        };

        let bpm = self.transport_state.bpm;
        let max_note_end = notes_vec.iter().map(|n| n.position_ms + n.duration_ms).fold(0.0f64, f64::max);
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
            entries: std::collections::HashMap::new(),
            prefix_map: crate::oto::PrefixMap::default(),
        };

        let active_vb = self.voicebank.clone().unwrap_or(dummy_vb);
        let vocal_mode_params = self.vocal_mode_params.clone();

        // Render synchronously — ensures audio is ready immediately
        eprintln!("[Kamafeu] Rendering {} notes synchronously (playhead={:.0}ms)...", notes_vec.len(), playhead_ms);
        let rendered_audio = crate::renderer::ChunkedRenderer::render_playhead_chunk(
            &notes_vec,
            &active_vb,
            sample_rate,
            bpm,
            playhead_ms,
            30000.0,
            resampler_driver.as_ref(),
            wavtool_driver.as_ref(),
            Some(&vocal_mode_params),
        );

        if rendered_audio.is_empty() {
            eprintln!("[Kamafeu] WARNING: Rendered audio buffer is empty! No audio will play.");
            return;
        }

        eprintln!("[Kamafeu] Rendered {} samples, playing immediately!", rendered_audio.len());

        // Start playhead animation and audio playback at the same time
        self.piano_roll_state.is_playing = true;
        self.playback_start_instant = Some(Instant::now());
        self.playback_start_offset_ms = playhead_ms;
        self.transport_state.render_progress = 1.0;
        self.transport_state.status_message = "Playing...".to_string();
        self.render_rx = None;

        self.audio_player.play_samples(rendered_audio, sample_rate);
    }

    pub fn stop_audio(&mut self) {
        self.audio_player.stop();
        self.piano_roll_state.is_playing = false;
        self.playback_start_instant = None;
        self.render_rx = None;
        self.transport_state.status_message = "Stopped".to_string();
    }

    pub fn preview_tone(&mut self, freq: f64) {
        let sample_rate = 44100;
        let num_samples = (sample_rate as f64 * 0.3) as usize;
        let mut raw_samples: Vec<f32> = (0..num_samples)
            .map(|i| (i as f64 * 2.0 * std::f64::consts::PI * freq / sample_rate as f64).sin() as f32 * 0.4)
            .collect();

        let len = raw_samples.len();
        for (i, sample) in raw_samples.iter_mut().enumerate() {
            let fade = (len - i) as f32 / len as f32;
            *sample *= fade;
        }

        self.audio_player.play_samples(raw_samples, sample_rate);
    }
}

impl eframe::App for KamafeuStudioApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll for rendered audio from background thread
        if let Some(ref rx) = self.render_rx {
            if let Ok((samples, sample_rate)) = rx.try_recv() {
                eprintln!("[Kamafeu] Main thread received {} rendered samples! Playing audio...", samples.len());
                self.audio_player.play_samples(samples, sample_rate);
                self.render_rx = None;
            }
        }

        if self.piano_roll_state.is_playing {
            if let Some(start_t) = self.playback_start_instant {
                let elapsed_ms = start_t.elapsed().as_secs_f64() * 1000.0;
                self.piano_roll_state.playhead_ms = self.playback_start_offset_ms + elapsed_ms;

                let max_end_ms = self.current_notes()
                    .iter()
                    .map(|n| n.position_ms + n.duration_ms)
                    .fold(0.0f64, f64::max);

                if elapsed_ms > 1000.0 && self.piano_roll_state.playhead_ms > max_end_ms + 1000.0 && !self.audio_player.is_playing() {
                    self.stop_audio();
                }
            }
            ctx.request_repaint();
        }

        let cur_ms = self.piano_roll_state.playhead_ms.max(0.0);
        let total_sec = (cur_ms / 1000.0) as u32;
        let mins = total_sec / 60;
        let secs = total_sec % 60;
        let ms_rem = (cur_ms % 1000.0) as u32;
        self.transport_state.playhead_time_str = format!("{:02}:{:02}.{:03}", mins, secs, ms_rem);

        // Space = Play/Stop — sempre funciona, exceto ao editar lyric de nota
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
                    self.stop_audio();
                } else {
                    self.play_current_track();
                }
            }
        }

        // Keyboard Shortcuts System (Only process if not currently typing in a text widget or editing lyric)
        let is_editing_text = ctx.wants_keyboard_input() || self.piano_roll_state.editing_lyric_index.is_some();

        if !is_editing_text {
            let mut do_undo = false;
            let mut do_redo = false;
            let mut do_copy = false;
            let mut do_paste = false;
            let mut do_duplicate = false;
            let mut do_delete = false;
            let mut transpose_semitones: i32 = 0;
            let mut nudge_ms: f64 = 0.0;

            ctx.input(|i| {
                let has_cmd_or_ctrl = i.modifiers.command || i.modifiers.ctrl;

                // Space already handled above

                if i.key_pressed(Key::V) || i.key_pressed(Key::Num1) { self.piano_roll_state.active_tool = EditTool::Pointer; }
                if i.key_pressed(Key::N) || i.key_pressed(Key::Num2) { self.piano_roll_state.active_tool = EditTool::Pencil; }
                if i.key_pressed(Key::P) || i.key_pressed(Key::Num3) { self.piano_roll_state.active_tool = EditTool::PitchDraw; }
                if i.key_pressed(Key::E) || i.key_pressed(Key::Num4) { self.piano_roll_state.active_tool = EditTool::Eraser; }

                if has_cmd_or_ctrl && i.key_pressed(Key::Z) {
                    if i.modifiers.shift { do_redo = true; } else { do_undo = true; }
                }
                if has_cmd_or_ctrl && i.key_pressed(Key::Y) {
                    do_redo = true;
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
                if i.key_pressed(Key::ArrowUp) { transpose_semitones += step; }
                if i.key_pressed(Key::ArrowDown) { transpose_semitones -= step; }

                if i.key_pressed(Key::ArrowLeft) { nudge_ms -= 50.0; }
                if i.key_pressed(Key::ArrowRight) { nudge_ms += 50.0; }
            });

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
            if do_copy {
                if let Some(sel_idx) = self.piano_roll_state.selected_note_index {
                    let notes = self.current_notes();
                    if sel_idx < notes.len() {
                        self.clipboard = vec![notes[sel_idx].clone()];
                    }
                }
            }
            if do_paste {
                if !self.clipboard.is_empty() {
                    self.push_history();
                    let mut pasted = self.clipboard[0].clone();
                    pasted.position_ms = self.piano_roll_state.playhead_ms.max(0.0);
                    self.current_notes_mut().push(pasted);
                }
            }
            if do_duplicate {
                if let Some(sel_idx) = self.piano_roll_state.selected_note_index {
                    let notes = self.current_notes();
                    if sel_idx < notes.len() {
                        let mut dup = notes[sel_idx].clone();
                        dup.position_ms += dup.duration_ms;
                        self.push_history();
                        self.current_notes_mut().push(dup);
                        self.piano_roll_state.selected_note_index = Some(self.current_notes().len() - 1);
                    }
                }
            }
            if do_delete {
                if let Some(sel_idx) = self.piano_roll_state.selected_note_index {
                    self.push_history();
                    let notes = self.current_notes_mut();
                    if sel_idx < notes.len() {
                        notes.remove(sel_idx);
                        self.piano_roll_state.selected_note_index = None;
                    }
                }
            }
            if transpose_semitones != 0 {
                if let Some(sel_idx) = self.piano_roll_state.selected_note_index {
                    let min_m = self.piano_roll_state.min_midi as i32;
                    let max_m = self.piano_roll_state.max_midi as i32;
                    self.push_history();
                    let notes = self.current_notes_mut();
                    if sel_idx < notes.len() {
                        let cur_m = notes[sel_idx].midi_key() as i32;
                        let new_m = (cur_m + transpose_semitones).clamp(min_m, max_m) as u8;
                        notes[sel_idx].set_midi_key(new_m);
                    }
                }
            }
            if nudge_ms != 0.0 {
                if let Some(sel_idx) = self.piano_roll_state.selected_note_index {
                    self.push_history();
                    let notes = self.current_notes_mut();
                    if sel_idx < notes.len() {
                        notes[sel_idx].position_ms = (notes[sel_idx].position_ms + nudge_ms).max(0.0);
                    }
                }
            }
        }

        // 1. Top Transport Bar (Fixed 52px height)
        TopBottomPanel::top("top_transport")
            .exact_height(52.0)
            .frame(Frame::none().fill(MelodyneTheme::BG_PANEL))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("📂 Open Project...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Project & MIDI Files (.ustx, .ust, .mid, .midi)", &["ustx", "ust", "mid", "midi", "json"])
                            .pick_file()
                        {
                            let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                            let loaded = match extension.as_str() {
                                "mid" | "midi" => MidiFormat::load_file(&path),
                                "ust" => UstFormat::load_file(&path),
                                _ => UstxFormat::load_file(&path),
                            };
                            match loaded {
                                Ok(proj) => {
                                    self.project = proj;
                                    self.transport_state.bpm = self.project.bpm;

                                    let first_pos = self.project.parts.iter()
                                        .flat_map(|p| p.notes.iter())
                                        .map(|n| n.position_ms)
                                        .fold(f64::INFINITY, f64::min);

                                    if first_pos.is_finite() && first_pos > 0.0 {
                                        self.piano_roll_state.playhead_ms = first_pos;
                                    } else {
                                        self.piano_roll_state.playhead_ms = 0.0;
                                    }

                                    self.piano_roll_state.initial_scrolled = false;
                                    self.transport_state.status_message = format!("Opened project {:?}", path.file_name().unwrap_or_default());
                                }
                                Err(e) => {
                                    self.transport_state.status_message = format!("Error opening project: {}", e);
                                }
                            }
                        }
                    }

                    if ui.button("💾 Save Project...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name("project.ustx")
                            .add_filter("OpenUTAU Project (*.ustx)", &["ustx"])
                            .add_filter("UTAU Sequence (*.ust)", &["ust"])
                            .add_filter("Standard MIDI File (*.mid)", &["mid", "midi"])
                            .save_file()
                        {
                            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
                            let res = match ext.as_str() {
                                "mid" | "midi" => MidiFormat::save_file(&self.project, &path),
                                "ust" => UstFormat::save_file(&self.project, &path),
                                _ => UstxFormat::save_file(&self.project, &path),
                            };

                            if res.is_ok() {
                                self.transport_state.status_message = format!("Saved project to {:?}", path.file_name().unwrap_or_default());
                            } else if let Err(e) = res {
                                self.transport_state.status_message = format!("Error saving project: {}", e);
                            }
                        }
                    }

                    if ui.button("🎙️ Export WAV...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name("kamafeu_output.wav")
                            .add_filter("PCM Audio Wave (*.wav)", &["wav"])
                            .save_file()
                        {
                            let sample_rate = self.sample_rate;
                            let bpm = self.transport_state.bpm;
                            let notes_vec = if !self.project.parts.is_empty() {
                                self.project.parts[0].notes.clone()
                            } else {
                                Vec::new()
                            };

                            let native_resampler = NativeResamplerDriver;
                            let resampler_driver: Box<dyn ResamplerDriver> = if self.selected_resampler.contains("macres") {
                                let res_path = self.custom_resampler_path.clone().unwrap_or_else(|| PathBuf::from("macres"));
                                Box::new(MacResDriver::new(res_path))
                            } else if self.selected_resampler.contains("Native") {
                                Box::new(native_resampler)
                            } else {
                                let res_path = self.custom_resampler_path.clone().unwrap_or_else(|| PathBuf::from(&self.selected_resampler));
                                Box::new(ExternalResamplerDriver::new(res_path))
                            };

                            let native_wavtool = NativeWavtoolDriver;
                            let wavtool_driver: Box<dyn WavtoolDriver> = if self.selected_wavtool.contains("yawu") {
                                let wav_path = self.custom_wavtool_path.clone().unwrap_or_else(|| PathBuf::from("wavtool-yawu"));
                                Box::new(WavtoolYawuDriver::new(wav_path))
                            } else if self.selected_wavtool.contains("Native") {
                                Box::new(native_wavtool)
                            } else {
                                let wav_path = self.custom_wavtool_path.clone().unwrap_or_else(|| PathBuf::from(&self.selected_wavtool));
                                Box::new(ExternalWavtoolDriver::new(wav_path))
                            };

                            if let Some(ref vb) = self.voicebank {
                                let samples = crate::renderer::TrackRenderer::render_track_with_drivers(
                                    &notes_vec,
                                    vb,
                                    sample_rate,
                                    bpm,
                                    resampler_driver.as_ref(),
                                    wavtool_driver.as_ref(),
                                    Some(&self.vocal_mode_params),
                                );

                                match crate::renderer::exporter::AudioExporter::export_to_wav(&path, &samples, sample_rate) {
                                    Ok(_) => self.transport_state.status_message = format!("Exported WAV audio to {:?}", path.file_name().unwrap_or_default()),
                                    Err(e) => self.transport_state.status_message = format!("Error exporting WAV: {}", e),
                                }
                            }
                        }
                    }

                    ui.separator();

                    draw_toolbar(
                        ui,
                        &mut self.piano_roll_state.active_tool,
                        &mut self.piano_roll_state.px_per_ms,
                        &mut self.piano_roll_state.row_height,
                    );
                });

                ui.separator();

                let is_playing = self.audio_player.is_playing();
                let mut play_clicked = false;
                let mut stop_clicked = false;
                let mut loaded_path: Option<PathBuf> = None;
                let mut export_clicked = false;

                draw_transport_bar(
                    ui,
                    &mut self.transport_state,
                    is_playing,
                    &mut || play_clicked = true,
                    &mut || stop_clicked = true,
                    &mut |p| loaded_path = Some(p),
                    &mut || export_clicked = true,
                );

                if play_clicked {
                    if is_playing {
                        self.stop_audio();
                    } else {
                        self.play_current_track();
                    }
                }

                if stop_clicked {
                    self.stop_audio();
                }

                if let Some(path) = loaded_path {
                    match Voicebank::new(&path) {
                        Ok(vb) => {
                            self.transport_state.status_message = format!("Loaded Voicebank: {}", vb.name);
                            self.voicebank = Some(vb);
                        }
                        Err(e) => {
                            self.transport_state.status_message = format!("Error loading voicebank: {}", e);
                        }
                    }
                }

                if export_clicked {
                    if let Some(file_path) = rfd::FileDialog::new()
                        .set_file_name("output.wav")
                        .add_filter("WAV Audio", &["wav"])
                        .save_file()
                    {
                        let dummy_vb = Voicebank {
                            root_path: PathBuf::from("."),
                            name: "Fallback".to_string(),
                            author: "System".to_string(),
                            character_info: String::new(),
                            readme_info: String::new(),
                            entries: std::collections::HashMap::new(),
                            prefix_map: crate::oto::PrefixMap::default(),
                        };
                        let vb_ref = self.voicebank.as_ref().unwrap_or(&dummy_vb);
                        let buffer = TrackRenderer::render_track(self.current_notes(), vb_ref, self.sample_rate);

                        let spec = hound::WavSpec {
                            channels: 1,
                            sample_rate: self.sample_rate,
                            bits_per_sample: 16,
                            sample_format: hound::SampleFormat::Int,
                        };

                        if let Ok(mut writer) = hound::WavWriter::create(&file_path, spec) {
                            for &s in &buffer {
                                let clamped = s.clamp(-1.0, 1.0);
                                let sample_i16 = (clamped * i16::MAX as f32) as i16;
                                let _ = writer.write_sample(sample_i16);
                            }
                            let _ = writer.finalize();
                            self.transport_state.status_message = format!("Exported WAV to {:?}", file_path.file_name().unwrap_or_default());
                        }
                    }
                }
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
                    &mut self.left_sidebar_tab,
                    &mut self.vocal_mode_params,
                    &mut self.phoneme_palette_state,
                    &mut || {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            if let Ok(vb) = Voicebank::new(&folder) {
                                loaded_vb = Some(vb);
                            }
                        }
                    },
                    &mut |alias| preview_alias = Some(alias.to_string()),
                    &mut |alias| insert_alias = Some(alias.to_string()),
                );

                if let Some(vb) = loaded_vb {
                    self.voicebank = Some(vb);
                }

                if let Some(alias) = preview_alias {
                    let mut played = false;
                    if let Some(ref vb) = self.voicebank {
                        if let Some(entry) = vb.find_entry(&alias, "C4").or_else(|| vb.find_entry(&alias, "A3")) {
                            let wav_path = vb.root_path.join(&entry.wav_filename);
                            if let Ok((samples, sr)) = TrackRenderer::load_wav_samples(&wav_path) {
                                let max_s = (sr as usize).min(samples.len());
                                self.audio_player.play_samples(samples[..max_s].to_vec(), sr);
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
                let notes = if self.project.parts.is_empty() { &mut [] } else { &mut self.project.parts[0].notes[..] };
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
                    self.piano_roll_state.playhead_ms,
                    self.piano_roll_state.px_per_ms,
                );

                ui.add_space(2.0);
                ui.separator();
                ui.add_space(2.0);

                let mut preview_freq: Option<f64> = None;
                let mut note_changed = false;
                let mut scrubbed_t: Option<f64> = None;

                if self.project.parts.is_empty() {
                    self.project.parts.push(crate::project::model::UVoicePart::new("Part 1", 0));
                }

                draw_piano_roll(
                    ui,
                    &mut self.project.parts[0].notes,
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
    }
}
