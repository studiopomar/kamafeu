use crate::project::model::UNote;
use eframe::egui::{self, Color32, RichText};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InspectorTab {
    #[default]
    NoteProperties,
    Dynamics,
    Gender,
    PitchDelta,
    Breathiness,
    Vibrato,
}

pub fn draw_inspector_panel(
    ui: &mut egui::Ui,
    selected_note: Option<&mut UNote>,
    active_tab: &mut InspectorTab,
) {
    ui.horizontal(|ui| {
        ui.heading(
            RichText::new("🎛 Inspector")
                .strong()
                .size(14.0)
                .color(Color32::from_rgb(200, 215, 235)),
        );
        ui.add_space(12.0);

        // Tab selection buttons
        let tabs = [
            (InspectorTab::NoteProperties, "Note"),
            (InspectorTab::Dynamics, "DYN (Volume)"),
            (InspectorTab::Gender, "GEN (Formant)"),
            (InspectorTab::PitchDelta, "PITD (Pitch)"),
            (InspectorTab::Breathiness, "BRE (Breath)"),
            (InspectorTab::Vibrato, "Vibrato"),
        ];

        for (tab, label) in tabs {
            let is_sel = *active_tab == tab;
            let text = if is_sel {
                RichText::new(label)
                    .strong()
                    .color(Color32::from_rgb(0, 210, 255))
            } else {
                RichText::new(label).color(Color32::from_rgb(160, 175, 195))
            };
            if ui.button(text).clicked() {
                *active_tab = tab;
            }
        }

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(8.0);

        if let Some(note) = selected_note {
            match active_tab {
                InspectorTab::NoteProperties => {
                    ui.label("Lyric:");
                    ui.add(egui::TextEdit::singleline(&mut note.lyric).desired_width(55.0));

                    ui.label("Pitch:");
                    ui.add(egui::TextEdit::singleline(&mut note.pitch).desired_width(45.0));

                    ui.label("Start:");
                    ui.add(
                        egui::DragValue::new(&mut note.position_ms)
                            .range(0.0..=100000.0)
                            .speed(10.0)
                            .suffix("ms"),
                    );

                    ui.label("Duration:");
                    ui.add(
                        egui::DragValue::new(&mut note.duration_ms)
                            .range(20.0..=10000.0)
                            .speed(10.0)
                            .suffix("ms"),
                    );

                    ui.label("Vel. consoante:");
                    ui.add(
                        egui::Slider::new(&mut note.expressions.consonant_velocity, 0.0..=200.0)
                            .suffix(" %"),
                    )
                    .on_hover_text(
                        "100% mantém a duração original; valores maiores aceleram a consoante",
                    );

                    ui.separator();

                    ui.label("Env (p1..p5):");
                    ui.add(
                        egui::DragValue::new(&mut note.envelope.p1)
                            .range(0.0..=200.0)
                            .speed(1.0)
                            .prefix("p1:"),
                    );
                    ui.add(
                        egui::DragValue::new(&mut note.envelope.p2)
                            .range(0.0..=200.0)
                            .speed(1.0)
                            .prefix("p2:"),
                    );
                    ui.add(
                        egui::DragValue::new(&mut note.envelope.p3)
                            .range(0.0..=500.0)
                            .speed(1.0)
                            .prefix("p3:"),
                    );
                    ui.add(
                        egui::DragValue::new(&mut note.envelope.p4)
                            .range(0.0..=500.0)
                            .speed(1.0)
                            .prefix("p4:"),
                    );
                    ui.add(
                        egui::DragValue::new(&mut note.envelope.p5)
                            .range(0.0..=500.0)
                            .speed(1.0)
                            .prefix("p5:"),
                    );
                }
                InspectorTab::Dynamics => {
                    ui.label("Dynamics Offset (DYN):");
                    ui.add(
                        egui::Slider::new(&mut note.expressions.dynamics, -240.0..=120.0)
                            .suffix(" ×0.1 dB"),
                    );
                    ui.label("VOL:");
                    ui.add(egui::Slider::new(&mut note.expressions.volume, 0.0..=200.0));
                    ui.label("ATK:");
                    ui.add(egui::Slider::new(&mut note.expressions.attack, 0.0..=200.0));
                    ui.label("DEC:");
                    ui.add(egui::Slider::new(&mut note.expressions.decay, 0.0..=100.0));
                }
                InspectorTab::Gender => {
                    ui.label("Gender / Formant Shift (GEN):");
                    ui.add(egui::Slider::new(
                        &mut note.expressions.gender,
                        -100.0..=100.0,
                    ));
                }
                InspectorTab::PitchDelta => {
                    ui.label("Pitch Shift Delta (PITD):");
                    ui.add(
                        egui::Slider::new(&mut note.expressions.pitch_delta, -1200.0..=1200.0)
                            .suffix(" cents"),
                    );
                }
                InspectorTab::Breathiness => {
                    ui.label("Breathiness (BRE):");
                    ui.add(egui::Slider::new(
                        &mut note.expressions.breathiness,
                        0.0..=100.0,
                    ));
                }
                InspectorTab::Vibrato => {
                    ui.label("Comprimento:");
                    ui.add(
                        egui::Slider::new(&mut note.vibrato.length_pct, 0.0..=100.0).suffix("%"),
                    );
                    ui.label("Período:");
                    ui.add(
                        egui::DragValue::new(&mut note.vibrato.period_ms)
                            .range(5.0..=500.0)
                            .suffix(" ms"),
                    );
                    ui.label("Profundidade:");
                    ui.add(
                        egui::Slider::new(&mut note.vibrato.depth_cents, 0.0..=200.0).suffix(" c"),
                    );
                    ui.label("Entrada / saída:");
                    ui.add(
                        egui::DragValue::new(&mut note.vibrato.fade_in_pct)
                            .range(0.0..=100.0)
                            .suffix("%"),
                    );
                    ui.add(
                        egui::DragValue::new(&mut note.vibrato.fade_out_pct)
                            .range(0.0..=100.0)
                            .suffix("%"),
                    );
                    ui.label("Fase / drift / VOL link:");
                    ui.add(
                        egui::DragValue::new(&mut note.vibrato.shift_pct)
                            .range(0.0..=100.0)
                            .suffix("%"),
                    );
                    ui.add(egui::DragValue::new(&mut note.vibrato.drift_pct).range(-100.0..=100.0));
                    ui.add(
                        egui::DragValue::new(&mut note.vibrato.volume_link_pct)
                            .range(-100.0..=100.0)
                            .suffix("%"),
                    );
                }
            }
        } else {
            ui.label(
                RichText::new("Select a note on the Piano Roll to adjust its expressions.")
                    .italics()
                    .color(Color32::from_rgb(120, 135, 155)),
            );
        }
    });
}
