use eframe::egui::{self, Color32, Frame, RichText, Rounding, Stroke};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridSnapOption {
    Freeform,
    Snap1_16,
    Snap1_8,
    Snap1_4,
}

impl GridSnapOption {
    pub fn step_ms(&self, bpm: f64) -> Option<f64> {
        let beat_ms = 60000.0 / bpm;
        match self {
            GridSnapOption::Freeform => None,
            GridSnapOption::Snap1_16 => Some(beat_ms / 4.0), // 1/16 note
            GridSnapOption::Snap1_8 => Some(beat_ms / 2.0),  // 1/8 note
            GridSnapOption::Snap1_4 => Some(beat_ms),        // 1/4 note
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
    on_play: &mut dyn FnMut(),
    on_stop: &mut dyn FnMut(),
    on_load_vb: &mut dyn FnMut(PathBuf),
    on_export_wav: &mut dyn FnMut(),
) {
    ui.vertical(|ui| {
        // 1. Classic DAW Menu Bar (Dark Cyberpunk Style)
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            let menu_items = ["Arquivo", "Editar", "Exibir", "Ferramentas", "Tocar", "Exportar", "Ajuda"];
            for item in menu_items {
                let menu_btn = egui::Button::new(RichText::new(item).size(11.0).color(Color32::from_rgb(220, 210, 240)))
                    .fill(Color32::TRANSPARENT);
                ui.add(menu_btn);
            }
            ui.add_space(8.0);
            ui.label(
                RichText::new("kamafeu - sintetizador de voz")
                    .size(11.0)
                    .strong()
                    .color(Color32::from_rgb(0, 255, 157)),
            );
        });

        ui.separator();

        // 2. High-Contrast Cyberpunk Green/Purple Transport Tool Strip
        ui.horizontal(|ui| {
            ui.add_space(6.0);

            // Transport Controls: Play / Stop
            let (play_bg, play_text, play_color) = if is_playing {
                (Color32::from_rgb(36, 27, 53), "Pausar", Color32::from_rgb(216, 180, 254))
            } else {
                (Color32::from_rgb(10, 48, 30), "Tocar", Color32::from_rgb(0, 255, 157))
            };

            let play_btn = egui::Button::new(RichText::new(play_text).strong().size(12.0).color(play_color))
                .fill(play_bg)
                .stroke(Stroke::new(1.2, Color32::from_rgb(0, 255, 157)))
                .rounding(Rounding::same(4.0));

            if ui.add(play_btn).clicked() {
                on_play();
            }

            let stop_btn = egui::Button::new(RichText::new("Parar").size(12.0).color(Color32::from_rgb(255, 110, 110)))
                .fill(Color32::from_rgb(45, 20, 26))
                .stroke(Stroke::new(1.0, Color32::from_rgb(200, 80, 80)))
                .rounding(Rounding::same(4.0));

            if ui.add(stop_btn).clicked() {
                on_stop();
            }

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Time Readout Pill (00:01.250)
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

            // Tempo / BPM
            ui.label(RichText::new("BPM:").size(11.0).color(Color32::from_rgb(220, 210, 240)));
            ui.add(egui::DragValue::new(&mut state.bpm).range(40.0..=300.0).speed(1.0));

            ui.label(RichText::new("Compasso:").size(11.0).color(Color32::from_rgb(220, 210, 240)));
            ui.label(RichText::new("4/4").strong().size(11.0).color(Color32::from_rgb(0, 255, 157)));

            ui.label(RichText::new("Compassos:").size(11.0).color(Color32::from_rgb(220, 210, 240)));
            ui.label(RichText::new("24").strong().size(11.0).color(Color32::from_rgb(0, 255, 157)));

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Grid Snap Selector
            ui.label(RichText::new("Grade:").size(11.0).color(Color32::from_rgb(220, 210, 240)));
            let snap_options = [
                (GridSnapOption::Freeform, "Livre"),
                (GridSnapOption::Snap1_16, "1/16"),
                (GridSnapOption::Snap1_8, "1/8"),
                (GridSnapOption::Snap1_4, "1/4"),
            ];

            egui::ComboBox::from_id_salt("grid_snap_combo_cyberpunk")
                .selected_text(match state.grid_snap {
                    GridSnapOption::Freeform => "Livre",
                    GridSnapOption::Snap1_16 => "1/16 Nota",
                    GridSnapOption::Snap1_8 => "1/8 Nota",
                    GridSnapOption::Snap1_4 => "1/4 Nota",
                })
                .show_ui(ui, |ui| {
                    for (opt, label) in snap_options {
                        ui.selectable_value(&mut state.grid_snap, opt, label);
                    }
                });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            // Render Progress Bar (Neon Green Glow)
            if state.render_progress < 0.99 {
                ui.label(RichText::new("Renderizando:").size(10.0).color(Color32::from_rgb(0, 255, 157)));
                ui.add(
                    egui::ProgressBar::new(state.render_progress)
                        .desired_width(90.0)
                        .text(format!("{:.0}%", state.render_progress * 100.0))
                        .fill(Color32::from_rgb(0, 230, 138)),
                );
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(6.0);
            }

            // Load Voicebank Button
            if ui.button(RichText::new("Voicebank...").size(11.0)).clicked() {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    state.voicebank_name = folder
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Voicebank")
                        .to_string();
                    state.voicebank_path = Some(folder.clone());
                    on_load_vb(folder);
                }
            }

            ui.label(RichText::new(&state.voicebank_name).size(11.0).color(Color32::from_rgb(216, 180, 254)));

            ui.add_space(8.0);
            ui.separator();

            // Export WAV / MP3 Buttons
            if ui.button(RichText::new("➡ WAV").size(11.0)).clicked() {
                on_export_wav();
            }
            if ui.button(RichText::new("➡ MP3").size(11.0)).clicked() {
                on_export_wav();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(RichText::new(&state.status_message).size(11.0).italics().color(Color32::from_rgb(165, 148, 201)));
            });
        });
    });
}
