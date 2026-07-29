use eframe::egui::{self, Color32, Frame, Pos2, RichText, Rounding, Stroke, Vec2};
use std::path::PathBuf;
use crate::gui::theme::MelodyneTheme;
use crate::project::model::UNote;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RightSidebarTab {
    NoteProperties,
    Settings,
}

impl Default for RightSidebarTab {
    fn default() -> Self {
        RightSidebarTab::NoteProperties
    }
}

pub fn draw_right_panel(
    ui: &mut egui::Ui,
    voicebank: Option<&crate::oto::Voicebank>,
    selected_note_idx: Option<usize>,
    notes: &mut [UNote],
    selected_indices: &std::collections::HashSet<usize>,
    active_tab: &mut RightSidebarTab,
    render_threads: &mut u32,
    sample_rate: &mut u32,
    selected_resampler: &mut String,
    selected_wavtool: &mut String,
    custom_resampler_path: &mut Option<PathBuf>,
    custom_wavtool_path: &mut Option<PathBuf>,
) {
    let vb_name = voicebank.map(|v| v.name.as_str()).unwrap_or("Default Singer");
    let vb_author = voicebank.map(|v| v.author.as_str()).unwrap_or("UTAU Voicebank");
    let initial_letter = vb_name.chars().next().unwrap_or('V').to_uppercase().to_string();

    ui.vertical(|ui| {
        // 1. Voicebank Singer Info & Character.txt Box
        Frame::none()
            .fill(Color32::from_rgb(26, 20, 38))
            .rounding(Rounding::same(6.0))
            .stroke(Stroke::new(1.0, Color32::from_rgb(61, 46, 84)))
            .inner_margin(egui::Margin::same(8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Singer Avatar:").strong().size(11.0).color(Color32::from_rgb(0, 255, 157)));
                });

                ui.add_space(4.0);

                // Draw Avatar Picture Box
                let (avatar_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 70.0), egui::Sense::hover());
                let painter = ui.painter_at(avatar_rect);

                painter.rect_filled(avatar_rect, Rounding::same(4.0), Color32::from_rgb(36, 27, 53));
                painter.rect_stroke(avatar_rect, Rounding::same(4.0), Stroke::new(1.2, Color32::from_rgb(192, 132, 252)));

                let center = avatar_rect.center();
                painter.circle_filled(Pos2::new(center.x, center.y - 8.0), 18.0, Color32::from_rgb(0, 255, 157));
                painter.text(
                    Pos2::new(center.x, center.y - 8.0),
                    egui::Align2::CENTER_CENTER,
                    &initial_letter,
                    egui::FontId::proportional(16.0),
                    Color32::from_rgb(20, 16, 28),
                );

                painter.text(
                    Pos2::new(center.x, center.y + 16.0),
                    egui::Align2::CENTER_CENTER,
                    vb_name,
                    egui::FontId::proportional(11.0),
                    Color32::WHITE,
                );

                ui.add_space(6.0);
                ui.label(RichText::new(format!("Author: {}", vb_author)).size(10.0).color(MelodyneTheme::TEXT_MUTED));

                if let Some(vb) = voicebank {
                    if !vb.character_info.is_empty() || !vb.readme_info.is_empty() {
                        ui.add_space(4.0);
                        egui::CollapsingHeader::new(RichText::new("📄 character.txt / readme.txt").size(10.0).color(MelodyneTheme::TEXT_GOLD_LABEL))
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical().max_height(100.0).show(ui, |ui| {
                                    if !vb.character_info.is_empty() {
                                        ui.label(RichText::new(&vb.character_info).size(9.0).color(Color32::from_rgb(216, 180, 254)));
                                    }
                                    if !vb.readme_info.is_empty() {
                                        ui.separator();
                                        ui.label(RichText::new(&vb.readme_info).size(9.0).color(Color32::from_rgb(180, 220, 254)));
                                    }
                                });
                            });
                    }
                }
            });

        ui.add_space(8.0);

        // Sidebar Mode Tabs: Note Properties vs Engine Settings
        ui.horizontal(|ui| {
            let note_tab_color = if *active_tab == RightSidebarTab::NoteProperties {
                Color32::from_rgb(0, 255, 157)
            } else {
                MelodyneTheme::TEXT_MUTED
            };
            if ui.selectable_label(*active_tab == RightSidebarTab::NoteProperties, RichText::new("🎵 Note Info").color(note_tab_color)).clicked() {
                *active_tab = RightSidebarTab::NoteProperties;
            }

            let settings_tab_color = if *active_tab == RightSidebarTab::Settings {
                Color32::from_rgb(0, 255, 157)
            } else {
                MelodyneTheme::TEXT_MUTED
            };
            if ui.selectable_label(*active_tab == RightSidebarTab::Settings, RichText::new("⚙️ Engine Settings").color(settings_tab_color)).clicked() {
                *active_tab = RightSidebarTab::Settings;
            }
        });

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);

        match active_tab {
            RightSidebarTab::NoteProperties => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if let Some(target_idx) = selected_note_idx {
                        if target_idx < notes.len() {
                            if selected_indices.len() > 1 {
                                Frame::none()
                                    .fill(Color32::from_rgb(40, 30, 10))
                                    .rounding(Rounding::same(4.0))
                                    .stroke(Stroke::new(1.0, MelodyneTheme::ACCENT_GOLD))
                                    .inner_margin(egui::Margin::same(6.0))
                                    .show(ui, |ui| {
                                        ui.label(RichText::new(format!("✨ Group Selection: {} notes selected", selected_indices.len())).strong().size(11.0).color(MelodyneTheme::ACCENT_GOLD));
                                        ui.label(RichText::new("Slider changes apply to all selected notes").size(10.0).color(MelodyneTheme::TEXT_MUTED));
                                    });
                                ui.add_space(6.0);
                            }

                            let mut lyric = notes[target_idx].lyric.clone();
                            let pitch_str = notes[target_idx].pitch.clone();
                            let pos_ms = notes[target_idx].position_ms;
                            let mut dur_ms = notes[target_idx].duration_ms;
                            let mut gender = notes[target_idx].expressions.gender;
                            let mut dynamics = notes[target_idx].expressions.dynamics;
                            let mut pitch_delta = notes[target_idx].expressions.pitch_delta;
                            let mut breathiness = notes[target_idx].expressions.breathiness;

                            let mut changed_lyric = false;
                            let mut changed_dur = false;
                            let mut changed_gender = false;
                            let mut changed_dynamics = false;
                            let mut changed_pitch = false;
                            let mut changed_breath = false;

                            // Basic Note Info Section
                            Frame::none()
                                .fill(Color32::from_rgb(36, 27, 53))
                                .rounding(Rounding::same(4.0))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(61, 46, 84)))
                                .inner_margin(egui::Margin::same(8.0))
                                .show(ui, |ui| {
                                    ui.label(RichText::new("Basic Information").strong().size(11.0).color(Color32::from_rgb(0, 255, 157)));
                                    ui.separator();

                                    ui.horizontal(|ui| {
                                        ui.label("Lyric:");
                                        if ui.text_edit_singleline(&mut lyric).changed() {
                                            changed_lyric = true;
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Pitch Key:");
                                        ui.label(RichText::new(&pitch_str).strong().color(Color32::from_rgb(216, 180, 254)));
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Start Pos (ms):");
                                        ui.label(format!("{:.1}", pos_ms));
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Duration (ms):");
                                        if ui.add(egui::DragValue::new(&mut dur_ms).range(20.0..=10000.0).speed(5.0)).changed() {
                                            changed_dur = true;
                                        }
                                    });
                                });

                            ui.add_space(8.0);

                            // Voice Parameter Sliders
                            Frame::none()
                                .fill(Color32::from_rgb(26, 20, 38))
                                .rounding(Rounding::same(4.0))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(61, 46, 84)))
                                .inner_margin(egui::Margin::same(8.0))
                                .show(ui, |ui| {
                                    ui.label(RichText::new("Voice Parameters").strong().size(11.0).color(Color32::from_rgb(0, 255, 157)));
                                    ui.separator();

                                    ui.horizontal(|ui| {
                                        ui.label("Gender Factor:");
                                        if ui.add(egui::Slider::new(&mut gender, -100.0..=100.0).show_value(true)).changed() {
                                            changed_gender = true;
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Dynamics (Volume):");
                                        if ui.add(egui::Slider::new(&mut dynamics, -20.0..=20.0).show_value(true)).changed() {
                                            changed_dynamics = true;
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Pitch Delta (Cents):");
                                        if ui.add(egui::Slider::new(&mut pitch_delta, -100.0..=100.0).show_value(true)).changed() {
                                            changed_pitch = true;
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Breathiness:");
                                        if ui.add(egui::Slider::new(&mut breathiness, 0.0..=100.0).show_value(true)).changed() {
                                            changed_breath = true;
                                        }
                                    });
                                });

                            // Synchronize slider edits to selected notes
                            let update_targets: Vec<usize> = if selected_indices.is_empty() {
                                vec![target_idx]
                            } else {
                                selected_indices.iter().copied().collect()
                            };

                            for idx in update_targets {
                                if idx < notes.len() {
                                    if changed_lyric { notes[idx].lyric = lyric.clone(); }
                                    if changed_dur { notes[idx].duration_ms = dur_ms; }
                                    if changed_gender { notes[idx].expressions.gender = gender; }
                                    if changed_dynamics { notes[idx].expressions.dynamics = dynamics; }
                                    if changed_pitch { notes[idx].expressions.pitch_delta = pitch_delta; }
                                    if changed_breath { notes[idx].expressions.breathiness = breathiness; }
                                }
                            }
                        }
                    } else {
                        ui.vertical_centered(|ui| {
                            ui.add_space(20.0);
                            ui.label(RichText::new("No Note Selected").italics().color(MelodyneTheme::TEXT_MUTED));
                            ui.label(RichText::new("Click any note on the Piano Roll grid to inspect note properties.").size(10.0).color(MelodyneTheme::TEXT_MUTED));
                        });
                    }
                });
            }
            RightSidebarTab::Settings => {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.label(RichText::new("Resampler Engine").strong().color(Color32::from_rgb(0, 255, 157)));
                    let resamplers = ["macres (titinko/macres)", "Native Rust (TD-PSOLA)"];
                    for res in resamplers {
                        if ui.radio_value(selected_resampler, res.to_string(), res).clicked() {
                            if res.contains("macres") {
                                *custom_resampler_path = Some(PathBuf::from("./resamplers/macres"));
                            }
                        }
                    }

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);

                    ui.label(RichText::new("Wavtool Engine").strong().color(Color32::from_rgb(0, 255, 157)));
                    let wavtools = ["wavtool-yawu (m13253/wavtool-yawu)", "Native Rust (TD-PSOLA)"];
                    for wt in wavtools {
                        if ui.radio_value(selected_wavtool, wt.to_string(), wt).clicked() {
                            if wt.contains("yawu") {
                                *custom_wavtool_path = Some(PathBuf::from("./wavtools/wavtool-yawu"));
                            }
                        }
                    }

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);

                    ui.label(RichText::new("Audio Output").strong().color(Color32::from_rgb(0, 255, 157)));
                    ui.horizontal(|ui| {
                        ui.label("Sample Rate:");
                        ui.selectable_value(sample_rate, 44100, "44100 Hz");
                        ui.selectable_value(sample_rate, 48000, "48000 Hz");
                    });

                    ui.horizontal(|ui| {
                        ui.label("Render Threads:");
                        ui.add(egui::Slider::new(render_threads, 1..=16));
                    });
                });
            }
        }
    });
}
