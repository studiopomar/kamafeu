use super::*;
use crate::gui::theme::ThemeConfig;
use eframe::egui::{self, Color32, Pos2, Rect, Sense, Stroke, Vec2};

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

    // Keep the waveform as a light footer layer, separate from the note area
    // without making it a second editor panel.
    let strip_h = 52.0f32;

    egui::TopBottomPanel::bottom("bottom_processed_waveform_strip")
        .resizable(false)
        .exact_height(strip_h)
        .frame(egui::Frame::none())
        .show_inside(ui, |ui| {
            // Allocate one full-width, transparent interaction layer. In
            // particular, do not derive the timeline origin from a local
            // horizontal layout: that would reintroduce an egui spacing drift.
            let (strip_rect, strip_response) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), strip_h),
                Sense::click_and_drag(),
            );

            let painter = ui.painter_at(strip_rect);

            // This is intentionally the same origin used by notes, ruler and
            // playhead: ruler left plus the piano keyboard width.
            let content_left = crate::gui::piano_roll::state::timeline_content_left(
                ruler_rect.min.x,
                keyboard_width,
            );
            let px_per_ms = state.px_per_ms as f64;
            let timeline_rect = strip_rect.intersect(Rect::from_min_max(
                Pos2::new(content_left, strip_rect.min.y),
                strip_rect.max,
            ));
            let mid_y = strip_rect.center().y;
            let has_waveform = !state.rendered_waveform_peaks.is_empty();
            let animation_policy = crate::gui::animation::AnimationPolicy {
                enabled: state.ui_animations_enabled,
                reduce_motion: state.ui_reduce_motion,
                speed: state.ui_animation_speed,
                frame_time_ema_ms: if state.animation_quality_reduced {
                    24.0
                } else {
                    16.0
                },
            };
            let phase = ui.input(|input| input.time) as f32
                * std::f32::consts::TAU
                * 0.9
                * animation_policy.visual_speed();
            let footer_pulse = if animation_policy.enabled {
                0.5 + 0.5 * animation_policy.pulse_amplitude() * phase.sin()
            } else {
                0.0
            };
            if animation_policy.enabled && !animation_policy.reduce_motion && has_waveform {
                if let Some(interval) = animation_policy.interval() {
                    ui.ctx().request_repaint_after(interval);
                }
            }

            if timeline_rect.is_positive() {
                painter.line_segment(
                    [
                        Pos2::new(timeline_rect.min.x, mid_y),
                        Pos2::new(timeline_rect.max.x, mid_y),
                    ],
                    Stroke::new(0.7, Color32::from_rgba_unmultiplied(214, 133, 102, 70)),
                );
            }

            // Musical Bars / Beats Grid Markers
            let beat_dur_ms = 60_000.0 / bpm.max(10.0);
            let beats_per_bar = 4.0_f64;
            let bar_dur_ms = beat_dur_ms * beats_per_bar;

            let (t_min_visible, t_max_visible) =
                crate::gui::piano_roll::state::timeline_visible_time_range(
                    timeline_rect,
                    content_left,
                    state.px_per_ms,
                    timeline_scroll_x,
                )
                .unwrap_or((0.0, 0.0));

            let start_bar = (t_min_visible / bar_dur_ms).floor() as i64;
            let end_bar = (t_max_visible / bar_dur_ms).ceil() as i64 + 1;

            for b in start_bar..=end_bar {
                let bar_t = b as f64 * bar_dur_ms;
                let bar_x = crate::gui::piano_roll::state::timeline_time_to_screen_x(
                    content_left,
                    bar_t,
                    state.px_per_ms,
                    timeline_scroll_x,
                );

                if bar_x >= timeline_rect.min.x && bar_x <= timeline_rect.max.x {
                    painter.line_segment(
                        [
                            Pos2::new(bar_x, strip_rect.min.y),
                            Pos2::new(bar_x, strip_rect.max.y),
                        ],
                        Stroke::new(0.8, theme.c32_alpha(theme.grid_line_bar, 0.55)),
                    );
                }
            }

            // Render High-Resolution Waveform Peak Envelope
            if has_waveform {
                let half_h = strip_h * 0.40;
                // Sub-pixel sampling makes transients and quiet passages
                // much smoother than the former one-column bars.
                let step_px = 0.75f32;
                let num_steps = ((timeline_rect.width() / step_px).ceil() as usize).max(1);
                // Preserve the renderer's real amplitude. Auto-scaling
                // every preview to 92% made volume-link and dynamics
                // edits look identical even though the samples changed.
                let display_gain = 1.0_f32;

                for step in 0..num_steps {
                    let x_pos = timeline_rect.min.x + step as f32 * step_px;
                    let next_x = (x_pos + step_px).min(timeline_rect.max.x);
                    let timeline_origin_x =
                        crate::gui::piano_roll::state::timeline_time_to_screen_x(
                            content_left,
                            0.0,
                            state.px_per_ms,
                            timeline_scroll_x,
                        );
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
                            // The footer has its own visual language: a warm
                            // coral envelope with a restrained halo, rather
                            // than the cool blue waveform treatment associated
                            // with OpenUtau.
                            painter.line_segment(
                                [Pos2::new(x_pos, y_top), Pos2::new(x_pos, y_bot)],
                                Stroke::new(
                                    2.2 + footer_pulse * 0.8,
                                    Color32::from_rgba_unmultiplied(
                                        238,
                                        145,
                                        111,
                                        (24.0 + footer_pulse * 14.0) as u8,
                                    ),
                                ),
                            );
                            painter.line_segment(
                                [Pos2::new(x_pos, y_top), Pos2::new(x_pos, y_bot)],
                                Stroke::new(
                                    0.72 + footer_pulse * 0.16,
                                    Color32::from_rgba_unmultiplied(
                                        238,
                                        145,
                                        111,
                                        (116.0 + footer_pulse * 28.0) as u8,
                                    ),
                                ),
                            );
                        }
                    }
                }
            } else {
                // Empty placeholder hint
                painter.text(
                    Pos2::new(timeline_rect.center().x, mid_y),
                    egui::Align2::CENTER_CENTER,
                    lang.tr(
                        "Envelope de áudio renderizado / reprodução",
                        "Rendered / playback audio envelope",
                    ),
                    egui::FontId::proportional(10.0),
                    Color32::from_rgba_unmultiplied(214, 133, 102, 145),
                );
            }

            // Playhead Needle
            let playhead_x = crate::gui::piano_roll::state::timeline_time_to_screen_x(
                content_left,
                state.playhead_ms,
                state.px_per_ms,
                timeline_scroll_x,
            );
            if playhead_x >= timeline_rect.min.x && playhead_x <= timeline_rect.max.x {
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
                    if pos.x >= timeline_rect.min.x {
                        let timeline_origin_x =
                            crate::gui::piano_roll::state::timeline_time_to_screen_x(
                                content_left,
                                0.0,
                                state.px_per_ms,
                                timeline_scroll_x,
                            );
                        let scrubbed_t = ((pos.x - timeline_origin_x) as f64 / px_per_ms).max(0.0);
                        state.playhead_ms = scrubbed_t;
                        on_playhead_scrubbed(scrubbed_t);
                    }
                }
            }
        });
}
