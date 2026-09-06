use eframe::egui::{self, Color32, Frame, RichText, Rounding, Stroke};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridSnapOption {
    Freeform,
    Snap1_1,
    Snap1_2,
    Snap1_4,
    Snap1_8,
    Snap1_16,
    Snap1_32,
    Snap1_64,
    Snap1_128,
    Snap1_4T,
    Snap1_8T,
    Snap1_16T,
    Snap1_32T,
    Snap1_64T,
}

impl GridSnapOption {
    pub fn step_ms(&self, bpm: f64) -> Option<f64> {
        let beat_ms = 60000.0 / bpm;
        match self {
            GridSnapOption::Freeform => None,
            GridSnapOption::Snap1_1 => Some(beat_ms * 4.0),
            GridSnapOption::Snap1_2 => Some(beat_ms * 2.0),
            GridSnapOption::Snap1_4 => Some(beat_ms),
            GridSnapOption::Snap1_8 => Some(beat_ms / 2.0),
            GridSnapOption::Snap1_16 => Some(beat_ms / 4.0),
            GridSnapOption::Snap1_32 => Some(beat_ms / 8.0),
            GridSnapOption::Snap1_64 => Some(beat_ms / 16.0),
            GridSnapOption::Snap1_128 => Some(beat_ms / 32.0),
            GridSnapOption::Snap1_4T => Some(beat_ms * 4.0 / 6.0),
            GridSnapOption::Snap1_8T => Some(beat_ms * 4.0 / 12.0),
            GridSnapOption::Snap1_16T => Some(beat_ms * 4.0 / 24.0),
            GridSnapOption::Snap1_32T => Some(beat_ms * 4.0 / 48.0),
            GridSnapOption::Snap1_64T => Some(beat_ms * 4.0 / 96.0),
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            GridSnapOption::Freeform => "Livre",
            GridSnapOption::Snap1_1 => "1/1",
            GridSnapOption::Snap1_2 => "1/2",
            GridSnapOption::Snap1_4 => "1/4",
            GridSnapOption::Snap1_8 => "1/8",
            GridSnapOption::Snap1_16 => "1/16",
            GridSnapOption::Snap1_32 => "1/32",
            GridSnapOption::Snap1_64 => "1/64",
            GridSnapOption::Snap1_128 => "1/128",
            GridSnapOption::Snap1_4T => "1/4T",
            GridSnapOption::Snap1_8T => "1/8T",
            GridSnapOption::Snap1_16T => "1/16T",
            GridSnapOption::Snap1_32T => "1/32T",
            GridSnapOption::Snap1_64T => "1/64T",
        }
    }
}

pub struct TransportState {
    pub bpm: f64,
    pub voicebank_name: String,
    pub voicebank_path: Option<PathBuf>,
    pub status_message: String,
    pub grid_snap: GridSnapOption,
    pub playhead_time_str: String,
    pub render_progress: f32, // 0.0 to 1.0 (0% to 100%)
}

impl Default for TransportState {
    fn default() -> Self {
        Self {
            bpm: 120.0,
            voicebank_name: "Nenhum Voicebank Carregado".to_string(),
            voicebank_path: None,
            status_message: "Pronto".to_string(),
            grid_snap: GridSnapOption::Snap1_16,
            playhead_time_str: "00:00.000".to_string(),
            render_progress: 1.0,
        }
    }
}

pub fn draw_transport_bar(
    ui: &mut egui::Ui,
    state: &mut TransportState,
    is_playing: bool,
    log_open: &mut bool,
    on_play: &mut dyn FnMut(),
    on_stop: &mut dyn FnMut(),
    on_load_vb: &mut dyn FnMut(PathBuf),
    on_export_wav: &mut dyn FnMut(),
) {
    egui::ScrollArea::horizontal()
        .id_salt("transport_bar_h_scroll")
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(6.0);

                let (play_bg, play_text, play_color) = if is_playing {
                    (
                        Color32::from_rgb(36, 27, 53),
                        "Pausar",
                        Color32::from_rgb(216, 180, 254),
                    )
                } else {
                    (
                        Color32::from_rgb(10, 48, 30),
                        "Tocar",
                        Color32::from_rgb(0, 255, 157),
                    )
                };

                let play_btn = egui::Button::new(
                    RichText::new(play_text)
                        .strong()
                        .size(12.0)
                        .color(play_color),
                )
                .fill(play_bg)
                .stroke(Stroke::new(1.2, Color32::from_rgb(0, 255, 157)))
                .rounding(Rounding::same(4.0));

                if ui.add(play_btn).clicked() {
                    on_play();
                }

                let stop_btn = egui::Button::new(
                    RichText::new("Parar")
                        .size(12.0)
                        .color(Color32::from_rgb(255, 110, 110)),
                )
                .fill(Color32::from_rgb(45, 20, 26))
                .stroke(Stroke::new(1.0, Color32::from_rgb(200, 80, 80)))
                .rounding(Rounding::same(4.0));

                if ui.add(stop_btn).clicked() {
                    on_stop();
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                Frame::none()
                    .fill(Color32::from_rgb(18, 14, 28))
                    .rounding(Rounding::same(4.0))
                    .stroke(Stroke::new(1.0, Color32::from_rgb(61, 46, 84)))
                    .inner_margin(egui::Margin::symmetric(8.0, 3.0))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(&state.playhead_time_str)
                                .strong()
                                .monospace()
                                .size(12.0)
                                .color(Color32::from_rgb(0, 255, 157)),
                        );
                    });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                ui.label(
                    RichText::new("BPM:")
                        .size(11.0)
                        .color(Color32::from_rgb(220, 210, 240)),
                );
                ui.add(
                    egui::DragValue::new(&mut state.bpm)
                        .range(40.0..=300.0)
                        .speed(1.0),
                );
                ui.label(
                    RichText::new("4/4")
                        .strong()
                        .size(11.0)
                        .color(Color32::from_rgb(0, 255, 157)),
                );

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                ui.label(
                    RichText::new("Grade:")
                        .size(11.0)
                        .color(Color32::from_rgb(220, 210, 240)),
                );
                let snap_options = [
                    (GridSnapOption::Freeform, "Livre"),
                    (GridSnapOption::Snap1_1, "1/1"),
                    (GridSnapOption::Snap1_2, "1/2"),
                    (GridSnapOption::Snap1_4, "1/4"),
                    (GridSnapOption::Snap1_8, "1/8"),
                    (GridSnapOption::Snap1_16, "1/16"),
                    (GridSnapOption::Snap1_32, "1/32"),
                    (GridSnapOption::Snap1_64, "1/64"),
                    (GridSnapOption::Snap1_128, "1/128"),
                    (GridSnapOption::Snap1_4T, "1/4T (1/6)"),
                    (GridSnapOption::Snap1_8T, "1/8T (1/12)"),
                    (GridSnapOption::Snap1_16T, "1/16T (1/24)"),
                    (GridSnapOption::Snap1_32T, "1/32T (1/48)"),
                    (GridSnapOption::Snap1_64T, "1/64T (1/96)"),
                ];

                egui::ComboBox::from_id_salt("grid_snap_combo_cyberpunk")
                    .selected_text(state.grid_snap.label())
                    .show_ui(ui, |ui| {
                        for (opt, label) in snap_options {
                            ui.selectable_value(&mut state.grid_snap, opt, label);
                        }
                    });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                if state.render_progress < 0.99 {
                    ui.label(
                        RichText::new("⚡ Resampler:")
                            .size(10.5)
                            .strong()
                            .color(Color32::from_rgb(0, 229, 255)),
                    );
                    ui.add(
                        egui::ProgressBar::new(state.render_progress)
                            .desired_width(110.0)
                            .text(format!("{:.0}%", state.render_progress * 100.0))
                            .animate(true)
                            .fill(Color32::from_rgb(0, 230, 138)),
                    )
                    .on_hover_text(&state.status_message);
                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(6.0);
                }

                if ui
                    .button(RichText::new("Voicebank...").size(11.0))
                    .clicked()
                {
                    if let Some(folder) = crate::dialogs::FileDialog::new().pick_folder() {
                        state.voicebank_name = folder
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("Voicebank")
                            .to_string();
                        state.voicebank_path = Some(folder.clone());
                        on_load_vb(folder);
                    }
                }

                ui.label(
                    RichText::new(&state.voicebank_name)
                        .size(11.0)
                        .color(Color32::from_rgb(216, 180, 254)),
                );

                ui.add_space(8.0);
                ui.separator();

                if ui.button(RichText::new("➡ WAV").size(11.0)).clicked() {
                    on_export_wav();
                }
                if ui.button(RichText::new("➡ MP3").size(11.0)).clicked() {
                    on_export_wav();
                }

                ui.add_space(8.0);
                ui.separator();

                let log_bg = if *log_open {
                    Color32::from_rgb(10, 48, 30)
                } else {
                    Color32::from_rgb(28, 25, 34)
                };
                let log_stroke = if *log_open {
                    Stroke::new(1.2, Color32::from_rgb(0, 255, 157))
                } else {
                    Stroke::new(1.0, Color32::from_rgb(60, 50, 80))
                };
                let log_text_color = if *log_open {
                    Color32::from_rgb(0, 255, 157)
                } else {
                    Color32::from_rgb(180, 170, 200)
                };

                let log_btn = egui::Button::new(
                    RichText::new("⚡ Engine Log")
                        .size(11.0)
                        .color(log_text_color),
                )
                .fill(log_bg)
                .stroke(log_stroke)
                .rounding(Rounding::same(4.0));

                if ui.add(log_btn).clicked() {
                    *log_open = !*log_open;
                }

                ui.add_space(8.0);
                ui.label(
                    RichText::new(&state.status_message)
                        .size(11.0)
                        .italics()
                        .color(Color32::from_rgb(165, 148, 201)),
                );
            });
        });
}
