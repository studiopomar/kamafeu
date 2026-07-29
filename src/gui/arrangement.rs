use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Stroke, Vec2};
use crate::gui::theme::MelodyneTheme;
use crate::project::model::UTrack;

pub fn draw_arrangement_view(
    ui: &mut egui::Ui,
    tracks: &mut [UTrack],
    playhead_ms: f64,
    px_per_ms: f32,
) {
    let header_width = 180.0f32;
    let track_height = 34.0f32;

    ui.vertical(|ui| {
        ui.add_space(2.0);
        ui.heading(egui::RichText::new(" 🎚 Arrangement").strong().size(12.0).color(MelodyneTheme::TEXT_GOLD_LABEL));
        ui.add_space(2.0);

        for (idx, track) in tracks.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                // 1. Left Track Card Header
                let (card_rect, _) = ui.allocate_exact_size(Vec2::new(header_width, track_height), egui::Sense::hover());
                let card_bg = if idx == 0 { MelodyneTheme::BG_PANEL } else { MelodyneTheme::BG_CANVAS };
                ui.painter().rect_filled(card_rect, Rounding::same(4.0), card_bg);
                ui.painter().rect_stroke(card_rect, Rounding::same(4.0), Stroke::new(1.0, MelodyneTheme::GRID_LINE_BAR));

                // Metallic Gold Track badge
                let color_badge = match idx % 4 {
                    0 => MelodyneTheme::NOTE_GOLD_FILL,
                    1 => Color32::from_rgb(230, 126, 34),
                    2 => MelodyneTheme::ACCENT_GOLD,
                    _ => Color32::from_rgb(241, 196, 15),
                };
                ui.painter().rect_filled(
                    Rect::from_min_size(card_rect.min + Vec2::new(4.0, 4.0), Vec2::new(6.0, track_height - 8.0)),
                    Rounding::same(2.0),
                    color_badge,
                );

                // Track Name
                ui.painter().text(
                    card_rect.min + Vec2::new(16.0, 6.0),
                    egui::Align2::LEFT_TOP,
                    &track.name,
                    egui::FontId::proportional(12.0),
                    MelodyneTheme::TEXT_GOLD_LABEL,
                );

                // Mute (M) / Solo (S) Buttons
                let mute_bg = if track.mute { Color32::from_rgb(220, 50, 50) } else { MelodyneTheme::BG_CANVAS };
                let solo_bg = if track.solo { MelodyneTheme::ACCENT_GOLD } else { MelodyneTheme::BG_CANVAS };

                let mute_rect = Rect::from_min_size(card_rect.min + Vec2::new(125.0, 7.0), Vec2::new(20.0, 20.0));
                let solo_rect = Rect::from_min_size(card_rect.min + Vec2::new(150.0, 7.0), Vec2::new(20.0, 20.0));

                ui.painter().rect_filled(mute_rect, Rounding::same(3.0), mute_bg);
                ui.painter().text(mute_rect.center(), egui::Align2::CENTER_CENTER, "M", egui::FontId::proportional(11.0), Color32::WHITE);

                ui.painter().rect_filled(solo_rect, Rounding::same(3.0), solo_bg);
                ui.painter().text(solo_rect.center(), egui::Align2::CENTER_CENTER, "S", egui::FontId::proportional(11.0), Color32::BLACK);

                // 2. Right Track Audio Waveform Strip (Melodyne Gold Waveforms)
                let (strip_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), track_height), egui::Sense::hover());
                ui.painter().rect_filled(strip_rect, Rounding::ZERO, MelodyneTheme::BG_CANVAS);
                ui.painter().rect_stroke(strip_rect, Rounding::ZERO, Stroke::new(1.0, MelodyneTheme::GRID_LINE_SUB));

                let mut wave_x = strip_rect.min.x;
                let end_x = strip_rect.max.x;
                let step = 4.0f32;
                let mid_y = strip_rect.center().y;

                while wave_x < end_x {
                    let rel_i = ((wave_x - strip_rect.min.x) * 0.05) as f32;
                    let amp = (rel_i.sin().abs() * 0.8 + (rel_i * 2.5).cos().abs() * 0.4) * (track_height * 0.35);
                    ui.painter().line_segment(
                        [Pos2::new(wave_x, mid_y - amp), Pos2::new(wave_x, mid_y + amp)],
                        Stroke::new(1.5, color_badge.linear_multiply(0.7)),
                    );
                    wave_x += step;
                }

                // Playhead line indicator in Arrangement view
                let playhead_x = strip_rect.min.x + (playhead_ms * px_per_ms as f64) as f32;
                if playhead_x >= strip_rect.min.x && playhead_x <= strip_rect.max.x {
                    ui.painter().line_segment(
                        [Pos2::new(playhead_x, strip_rect.min.y), Pos2::new(playhead_x, strip_rect.max.y)],
                        Stroke::new(2.0, MelodyneTheme::PLAYHEAD_RED),
                    );
                }
            });
        }
    });
}
