use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Stroke, Vec2};
use crate::gui::theme::MelodyneTheme;
use crate::project::model::{UTrack, UVoicePart};

pub fn draw_arrangement_view(
    ui: &mut egui::Ui,
    tracks: &mut Vec<UTrack>,
    parts: &mut Vec<UVoicePart>,
    active_track_index: &mut usize,
    playhead_ms: f64,
    px_per_ms: f32,
    bpm: f64,
) {
    let header_width = 220.0f32;
    let track_height = 42.0f32;
    let ruler_height = 24.0f32;

    ui.vertical(|ui| {
        // 1. Top Track Management Bar
        ui.horizontal(|ui| {
            ui.heading(egui::RichText::new("🎚 Arrangement (Faixas)").strong().size(12.0).color(MelodyneTheme::TEXT_GOLD_LABEL));
            ui.add_space(10.0);

            if ui.button(egui::RichText::new("➕ Nova Track").size(10.5).color(Color32::from_rgb(0, 255, 157))).clicked() {
                let new_idx = tracks.len();
                let track_name = format!("Track {}", new_idx + 1);
                tracks.push(UTrack {
                    name: track_name.clone(),
                    singer: "Default Singer".to_string(),
                    volume_db: 0.0,
                    pan: 0.0,
                    mute: false,
                    solo: false,
                });
                parts.push(UVoicePart::new(format!("Part {}", new_idx + 1), new_idx));
                *active_track_index = new_idx;
            }

            if tracks.len() > 1 {
                if ui.button(egui::RichText::new("🗑 Excluir Track").size(10.5).color(Color32::from_rgb(255, 100, 100))).clicked() {
                    let del_idx = *active_track_index;
                    if del_idx < tracks.len() {
                        tracks.remove(del_idx);
                        parts.retain(|p| p.track_index != del_idx);
                        for p in parts.iter_mut() {
                            if p.track_index > del_idx {
                                p.track_index -= 1;
                            }
                        }
                        if *active_track_index >= tracks.len() {
                            *active_track_index = tracks.len().saturating_sub(1);
                        }
                    }
                }
            }

            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(format!("Track Ativa: {} ({})", *active_track_index + 1, tracks.get(*active_track_index).map(|t| t.name.as_str()).unwrap_or("Track")))
                    .size(11.0)
                    .color(Color32::from_rgb(200, 190, 230))
            );
        });

        ui.add_space(4.0);

        if *active_track_index >= tracks.len() && !tracks.is_empty() {
            *active_track_index = 0;
        }

        for idx in 0..tracks.len() {
            let is_active = idx == *active_track_index;
            let track_y_offset = if idx == 0 { ruler_height } else { 0.0 };
            let card_total_h = track_height + track_y_offset;

            ui.horizontal(|ui| {
                // 1. Left Track Card Header
                let (card_rect, card_resp) = ui.allocate_exact_size(Vec2::new(header_width, card_total_h), egui::Sense::click());
                
                if card_resp.clicked() {
                    *active_track_index = idx;
                }

                let card_bg = if is_active {
                    Color32::from_rgb(36, 27, 53)
                } else if idx == 0 {
                    MelodyneTheme::BG_PANEL
                } else {
                    MelodyneTheme::BG_CANVAS
                };

                let card_stroke = if is_active {
                    Stroke::new(1.8, Color32::from_rgb(0, 255, 157))
                } else {
                    Stroke::new(1.0, MelodyneTheme::GRID_LINE_BAR)
                };

                ui.painter().rect_filled(card_rect, Rounding::same(4.0), card_bg);
                ui.painter().rect_stroke(card_rect, Rounding::same(4.0), card_stroke);

                let color_badge = match idx % 4 {
                    0 => MelodyneTheme::NOTE_GOLD_FILL,
                    1 => Color32::from_rgb(230, 126, 34),
                    2 => MelodyneTheme::ACCENT_GOLD,
                    _ => Color32::from_rgb(241, 196, 15),
                };

                ui.painter().rect_filled(
                    Rect::from_min_size(card_rect.min + Vec2::new(4.0, 4.0 + track_y_offset), Vec2::new(6.0, track_height - 8.0)),
                    Rounding::same(2.0),
                    color_badge,
                );

                // Track Controls UI
                let track = &mut tracks[idx];

                // Interactive Mute & Solo Buttons
                let mute_rect = Rect::from_min_size(card_rect.min + Vec2::new(145.0, 6.0 + track_y_offset), Vec2::new(28.0, 15.0));
                let solo_rect = Rect::from_min_size(card_rect.min + Vec2::new(180.0, 6.0 + track_y_offset), Vec2::new(28.0, 15.0));

                let mute_resp = ui.allocate_rect(mute_rect, egui::Sense::click());
                if mute_resp.clicked() {
                    track.mute = !track.mute;
                }
                let mute_bg = if track.mute { Color32::from_rgb(220, 50, 50) } else { Color32::from_rgb(45, 35, 60) };
                ui.painter().rect_filled(mute_rect, Rounding::same(3.0), mute_bg);
                ui.painter().text(mute_rect.center(), egui::Align2::CENTER_CENTER, "M", egui::FontId::proportional(10.0), Color32::WHITE);

                let solo_resp = ui.allocate_rect(solo_rect, egui::Sense::click());
                if solo_resp.clicked() {
                    track.solo = !track.solo;
                }
                let solo_bg = if track.solo { MelodyneTheme::ACCENT_GOLD } else { Color32::from_rgb(45, 35, 60) };
                ui.painter().rect_filled(solo_rect, Rounding::same(3.0), solo_bg);
                ui.painter().text(solo_rect.center(), egui::Align2::CENTER_CENTER, "S", egui::FontId::proportional(10.0), if track.solo { Color32::BLACK } else { Color32::WHITE });

                // Track Name text edit
                let name_rect = Rect::from_min_size(card_rect.min + Vec2::new(16.0, 4.0 + track_y_offset), Vec2::new(120.0, 18.0));
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(name_rect), |ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut track.name)
                            .desired_width(120.0)
                            .font(egui::FontId::proportional(11.0))
                    );
                });

                // Track Volume Slider
                let vol_rect = Rect::from_min_size(card_rect.min + Vec2::new(16.0, 23.0 + track_y_offset), Vec2::new(190.0, 14.0));
                ui.allocate_new_ui(egui::UiBuilder::new().max_rect(vol_rect), |ui| {
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Vol:").size(9.0).color(MelodyneTheme::TEXT_MUTED));
                        ui.add(egui::Slider::new(&mut track.volume_db, -20.0..=6.0).show_value(false));
                    });
                });

                // 2. Right Track Audio / Note Strip
                let available_width = ui.available_width();
                let scroll_id = ui.make_persistent_id("piano_roll_scroll");

                egui::ScrollArea::horizontal()
                    .id_salt(scroll_id)
                    .enable_scrolling(false)
                    .show(ui, |ui| {
                        let (strip_rect, _) = ui.allocate_exact_size(Vec2::new(available_width.max(3000.0), card_total_h), egui::Sense::hover());
                        ui.painter().rect_filled(strip_rect, Rounding::ZERO, MelodyneTheme::BG_CANVAS);
                        ui.painter().rect_stroke(strip_rect, Rounding::ZERO, Stroke::new(1.0, MelodyneTheme::GRID_LINE_SUB));

                        // Draw Timeline Ruler
                        if idx == 0 {
                            let ruler_rect = Rect::from_min_size(strip_rect.min, Vec2::new(strip_rect.width(), ruler_height));
                            ui.painter().rect_filled(ruler_rect, Rounding::ZERO, MelodyneTheme::BG_PANEL);
                            ui.painter().rect_stroke(ruler_rect, Rounding::ZERO, Stroke::new(1.0, MelodyneTheme::GRID_LINE_BAR));
                            
                            let beat_ms = 60000.0 / bpm;
                            let max_beats = (30000.0 / beat_ms).ceil() as i32;
                            for b in 0..=max_beats {
                                let x = ruler_rect.min.x + (b as f64 * beat_ms * px_per_ms as f64) as f32;
                                if x > ruler_rect.min.x && x < ruler_rect.max.x {
                                    ui.painter().line_segment(
                                        [Pos2::new(x, ruler_rect.min.y + 14.0), Pos2::new(x, ruler_rect.max.y)],
                                        Stroke::new(1.0, MelodyneTheme::TEXT_MUTED)
                                    );
                                    if b % 4 == 0 {
                                        let bar = (b / 4) + 1;
                                        ui.painter().text(
                                            Pos2::new(x + 4.0, ruler_rect.min.y + 2.0),
                                            egui::Align2::LEFT_TOP,
                                            format!("{}", bar),
                                            egui::FontId::proportional(11.0),
                                            MelodyneTheme::TEXT_NOTE_TAG,
                                        );
                                        ui.painter().line_segment(
                                            [Pos2::new(x, ruler_rect.min.y + 8.0), Pos2::new(x, ruler_rect.max.y)],
                                            Stroke::new(1.0, MelodyneTheme::TEXT_NOTE_TAG)
                                        );
                                    }
                                }
                            }
                        }

                        // Render notes for this track
                        for part in parts.iter().filter(|p| p.track_index == idx) {
                            let part_offset_x = (part.position_ms * px_per_ms as f64) as f32;
                            for note in &part.notes {
                                let start_x = strip_rect.min.x + part_offset_x + (note.position_ms * px_per_ms as f64) as f32;
                                let width = (note.duration_ms * px_per_ms as f64) as f32;
                                
                                let note_rect = Rect::from_min_size(
                                    Pos2::new(start_x, strip_rect.min.y + track_y_offset + 4.0),
                                    Vec2::new(width.max(2.0), track_height - 8.0)
                                );
                                
                                if note_rect.max.x > strip_rect.min.x && note_rect.min.x < strip_rect.max.x {
                                    ui.painter().rect_filled(note_rect, Rounding::same(2.0), color_badge.linear_multiply(0.8));
                                    ui.painter().rect_stroke(note_rect, Rounding::same(2.0), Stroke::new(1.0, MelodyneTheme::NOTE_GOLD_STROKE));
                                }
                            }
                        }

                        // Playhead line indicator
                        let playhead_x = strip_rect.min.x + (playhead_ms * px_per_ms as f64) as f32;
                        if playhead_x >= strip_rect.min.x && playhead_x <= strip_rect.max.x {
                            ui.painter().line_segment(
                                [Pos2::new(playhead_x, strip_rect.min.y), Pos2::new(playhead_x, strip_rect.max.y)],
                                Stroke::new(2.0, MelodyneTheme::PLAYHEAD_RED),
                            );
                        }
                    });
            });
        }
    });
}
