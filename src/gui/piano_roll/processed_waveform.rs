use super::*;
use crate::gui::theme::ThemeConfig;
use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Sense, Stroke, Vec2};

pub(super) fn draw(
    ui: &mut egui::Ui,
    state: &mut PianoRollState,
    keyboard_width: f32,
    timeline_scroll_x: f32,
    theme: &ThemeConfig,
    ruler_rect: Rect,
    bpm: f64,
    lang: crate::config::AppLanguage,
    on_playhead_scrubbed: &mut dyn FnMut(f64),
) {
    if !state.show_waveform_area || state.is_maximized {
        return;
    }

    let strip_h = 72.0f32;

    egui::TopBottomPanel::bottom("bottom_processed_waveform_strip")
        .resizable(false)
        .exact_height(strip_h)
        .frame(egui::Frame::none().fill(theme.bg_panel_c32()))
        .show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                // 1. Left sidebar gutter matching piano keys width
                let (gutter_rect, _gutter_resp) =
                    ui.allocate_exact_size(Vec2::new(keyboard_width, strip_h), Sense::hover());
                let gutter_painter = ui.painter_at(gutter_rect);
                gutter_painter.rect_filled(gutter_rect, Rounding::ZERO, theme.bg_panel_c32());
                gutter_painter.line_segment(
                    [
                        Pos2::new(gutter_rect.max.x, gutter_rect.min.y),
                        Pos2::new(gutter_rect.max.x, gutter_rect.max.y),
                    ],
                    Stroke::new(1.0, theme.grid_line_bar_c32()),
                );

                // Gutter label & status
                let badge_rect = Rect::from_min_size(
                    Pos2::new(gutter_rect.min.x + 4.0, gutter_rect.min.y + 6.0),
                    Vec2::new(gutter_rect.width() - 8.0, 16.0),
                );
                gutter_painter.rect_filled(
                    badge_rect,
                    Rounding::same(3.0),
                    Color32::from_rgba_unmultiplied(0, 220, 180, 28),
                );
                gutter_painter.rect_stroke(
                    badge_rect,
                    Rounding::same(3.0),
                    Stroke::new(0.8, Color32::from_rgb(0, 220, 180)),
                );
                gutter_painter.text(
                    badge_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "WAVEFORM",
                    egui::FontId::monospace(8.0),
                    Color32::from_rgb(0, 255, 200),
                );

                let has_waveform = !state.rendered_waveform_peaks.is_empty();
                let status_text = if has_waveform {
                    let total_ms = state
                        .rendered_waveform_peaks
                        .last()
                        .map(|p| p.0)
                        .unwrap_or(0.0);
                    format!("{:.1}s", total_ms / 1000.0)
                } else {
                    lang.tr("Áudio", "Audio").to_string()
                };
                gutter_painter.text(
                    Pos2::new(gutter_rect.center().x, gutter_rect.max.y - 8.0),
                    egui::Align2::CENTER_CENTER,
                    status_text,
                    egui::FontId::proportional(8.5),
                    Color32::from_rgb(120, 120, 140),
                );

                // 2. Timeline Waveform Display Canvas
                let available_w = (ui.available_width() - 4.0).max(50.0);
                let (strip_rect, strip_response) = ui
                    .allocate_exact_size(Vec2::new(available_w, strip_h), Sense::click_and_drag());

                let painter = ui.painter_at(strip_rect);
                painter.rect_filled(strip_rect, Rounding::ZERO, Color32::from_rgb(14, 14, 18));
                painter.line_segment(
                    [
                        Pos2::new(strip_rect.min.x, strip_rect.min.y),
                        Pos2::new(strip_rect.max.x, strip_rect.min.y),
                    ],
                    Stroke::new(1.0, Color32::from_rgb(32, 32, 42)),
                );
                painter.line_segment(
                    [
                        Pos2::new(strip_rect.min.x, strip_rect.max.y),
                        Pos2::new(strip_rect.max.x, strip_rect.max.y),
                    ],
                    Stroke::new(1.0, Color32::from_rgb(32, 32, 42)),
                );

                let timeline_origin_x = ruler_rect.min.x + keyboard_width - timeline_scroll_x;
                let px_per_ms = state.px_per_ms as f64;
                let mid_y = strip_rect.center().y;

                // Center Zero-Reference Line
                painter.line_segment(
                    [
                        Pos2::new(strip_rect.min.x, mid_y),
                        Pos2::new(strip_rect.max.x, mid_y),
                    ],
                    Stroke::new(0.8, Color32::from_rgba_unmultiplied(60, 60, 80, 80)),
                );

                // Musical Bars / Beats Grid Markers
                let beat_dur_ms = 60_000.0 / bpm.max(10.0);
                let beats_per_bar = 4.0_f64;
                let bar_dur_ms = beat_dur_ms * beats_per_bar;

                let t_min_visible = (strip_rect.min.x - timeline_origin_x) as f64 / px_per_ms;
                let t_max_visible = (strip_rect.max.x - timeline_origin_x) as f64 / px_per_ms;

                let start_bar = (t_min_visible / bar_dur_ms).floor() as i64;
                let end_bar = (t_max_visible / bar_dur_ms).ceil() as i64 + 1;

                for b in start_bar..=end_bar {
                    let bar_t = b as f64 * bar_dur_ms;
                    let bar_x = timeline_origin_x + (bar_t * px_per_ms) as f32;

                    if bar_x >= strip_rect.min.x && bar_x <= strip_rect.max.x {
                        painter.line_segment(
                            [
                                Pos2::new(bar_x, strip_rect.min.y),
                                Pos2::new(bar_x, strip_rect.max.y),
                            ],
                            Stroke::new(1.0, Color32::from_rgba_unmultiplied(70, 65, 95, 110)),
                        );
                    }
                }

                // Render High-Resolution Waveform Peak Envelope
                if has_waveform {
                    let half_h = strip_h * 0.43;
                    let step_px = 1.0f32;
                    let num_steps = ((strip_rect.width() / step_px).ceil() as usize).max(1);
                    let display_peak = state.waveform_display_peak();
                    let display_gain = if display_peak > 1e-5 {
                        (0.92 / display_peak).clamp(1.0, 12.0)
                    } else {
                        1.0
                    };

                    for step in 0..num_steps {
                        let x_pos = strip_rect.min.x + step as f32 * step_px;
                        let next_x = (x_pos + step_px).min(strip_rect.max.x);
                        let t_start_ms = ((x_pos - timeline_origin_x) as f64 / px_per_ms) as f32;
                        let t_end_ms = ((next_x - timeline_origin_x) as f64 / px_per_ms) as f32;

                        if let Some((min_val, max_val)) =
                            state.waveform_min_max_between(t_start_ms, t_end_ms)
                        {
                            let top_amp = (max_val * display_gain).clamp(0.0, 1.0);
                            let bot_amp = (min_val * display_gain).clamp(-1.0, 0.0);
                            let y_top = mid_y - top_amp * half_h;
                            let y_bot = mid_y - bot_amp * half_h;

                            if (y_bot - y_top).abs() > 0.25 {
                                painter.line_segment(
                                    [Pos2::new(x_pos, y_top), Pos2::new(x_pos, y_bot)],
                                    Stroke::new(1.0, Color32::from_rgb(0, 210, 255)),
                                );
                            }
                        }
                    }
                } else {
                    // Empty placeholder hint
                    painter.text(
                        Pos2::new(strip_rect.center().x, mid_y),
                        egui::Align2::CENTER_CENTER,
                        lang.tr(
                            "Área de áudio renderizado / reprodução (Waveform)",
                            "Rendered / playback audio area (Waveform)",
                        ),
                        egui::FontId::proportional(10.0),
                        Color32::from_rgba_unmultiplied(100, 100, 130, 120),
                    );
                }

                // Playhead Needle
                let playhead_x = timeline_origin_x + (state.playhead_ms * px_per_ms) as f32;
                if playhead_x >= strip_rect.min.x && playhead_x <= strip_rect.max.x {
                    painter.line_segment(
                        [
                            Pos2::new(playhead_x, strip_rect.min.y),
                            Pos2::new(playhead_x, strip_rect.max.y),
                        ],
                        Stroke::new(1.5, Color32::from_rgb(255, 215, 0)),
                    );
                    painter.circle_filled(
                        Pos2::new(playhead_x, strip_rect.min.y + 4.0),
                        3.0,
                        Color32::from_rgb(255, 215, 0),
                    );
                }

                // Interactive Timeline Scrubbing on Click/Drag
                let is_primary_down = ui.input(|i| i.pointer.primary_down());
                let pointer_pos = ui.input(|i| i.pointer.latest_pos());

                if (strip_response.dragged() || strip_response.clicked()) && is_primary_down {
                    if let Some(pos) = pointer_pos {
                        let scrubbed_t = ((pos.x - timeline_origin_x) as f64 / px_per_ms).max(0.0);
                        state.playhead_ms = scrubbed_t;
                        on_playhead_scrubbed(scrubbed_t);
                    }
                }
            });
        });
}
