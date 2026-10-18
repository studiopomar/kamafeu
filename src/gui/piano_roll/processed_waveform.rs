use super::*;
use crate::gui::theme::ThemeConfig;
use eframe::egui::{self, Id, Pos2, Rect, Sense, Stroke};

#[allow(clippy::too_many_arguments)]
pub(super) fn draw(
    ui: &mut egui::Ui,
    state: &mut PianoRollState,
    viewport_rect: Rect,
    keyboard_width: f32,
    timeline_scroll_x: f32,
    theme: &ThemeConfig,
    ruler_rect: Rect,
    bpm: f64,
    lang: crate::config::AppLanguage,
    on_playhead_scrubbed: &mut dyn FnMut(f64),
) {
    if !state.show_waveform_area || state.is_maximized || !viewport_rect.is_positive() {
        return;
    }

    // Float over the bottom of the piano-roll viewport instead of reserving a
    // second docked panel. This keeps the waveform above the parameter drawer
    // and gives the note grid back the space the old footer consumed.
    let strip_height = 40.0_f32.min(viewport_rect.height());
    let strip_rect = Rect::from_min_max(
        Pos2::new(viewport_rect.min.x, viewport_rect.max.y - strip_height),
        viewport_rect.max,
    );
    let content_left =
        crate::gui::piano_roll::state::timeline_content_left(ruler_rect.min.x, keyboard_width);
    let timeline_rect = strip_rect.intersect(Rect::from_min_max(
        Pos2::new(content_left, strip_rect.min.y),
        strip_rect.max,
    ));
    if !timeline_rect.is_positive() {
        return;
    }

    let painter = ui.painter_at(strip_rect);
    let mid_y = strip_rect.center().y;
    let coral = theme.c32_alpha(theme.semantic.waveform, 92.0 / 255.0);
    let muted_line = theme.c32_alpha(theme.semantic.waveform_grid, 36.0 / 255.0);

    // A quiet baseline and beat marks help place the floating waveform without
    // creating a panel background or a glow that competes with the notes.
    painter.line_segment(
        [
            Pos2::new(timeline_rect.left(), mid_y),
            Pos2::new(timeline_rect.right(), mid_y),
        ],
        Stroke::new(0.7_f32, muted_line),
    );

    let beat_duration_ms = 60_000.0 / bpm.max(10.0);
    let bar_duration_ms = beat_duration_ms * 4.0;
    let (visible_start_ms, visible_end_ms) =
        crate::gui::piano_roll::state::timeline_visible_time_range(
            timeline_rect,
            content_left,
            state.px_per_ms,
            timeline_scroll_x,
        )
        .unwrap_or((0.0, 0.0));
    let start_bar = (visible_start_ms / bar_duration_ms).floor() as i64;
    let end_bar = (visible_end_ms / bar_duration_ms).ceil() as i64 + 1;
    for bar in start_bar..=end_bar {
        let bar_x = crate::gui::piano_roll::state::timeline_time_to_screen_x(
            content_left,
            bar as f64 * bar_duration_ms,
            state.px_per_ms,
            timeline_scroll_x,
        );
        if bar_x >= timeline_rect.left() && bar_x <= timeline_rect.right() {
            painter.line_segment(
                [
                    Pos2::new(bar_x, strip_rect.top() + 3.0),
                    Pos2::new(bar_x, strip_rect.bottom() - 3.0),
                ],
                Stroke::new(0.7_f32, muted_line),
            );
        }
    }

    if !state.rendered_waveform_peaks.is_empty() {
        let px_per_ms = state.px_per_ms.max(0.001) as f64;
        let half_height = strip_height * 0.34;
        // One restrained stroke per screen column avoids the former luminous
        // double pass and keeps the real waveform amplitude visible.
        let step_px = 1.0_f32;
        let steps = timeline_rect.width().ceil().max(1.0) as usize;
        let timeline_origin_x = crate::gui::piano_roll::state::timeline_time_to_screen_x(
            content_left,
            0.0,
            state.px_per_ms,
            timeline_scroll_x,
        );

        for step in 0..steps {
            let x = timeline_rect.left() + step as f32 * step_px;
            if x > timeline_rect.right() {
                break;
            }
            let next_x = (x + step_px).min(timeline_rect.right());
            let start_ms = ((x - timeline_origin_x) as f64 / px_per_ms) as f32;
            let end_ms = ((next_x - timeline_origin_x) as f64 / px_per_ms) as f32;
            if let Some((minimum, maximum)) = state.waveform_min_max_between(start_ms, end_ms) {
                let top = mid_y - maximum.clamp(0.0, 1.0) * half_height;
                let bottom = mid_y - minimum.clamp(-1.0, 0.0) * half_height;
                if bottom - top > 0.25 {
                    painter.line_segment(
                        [Pos2::new(x, top), Pos2::new(x, bottom)],
                        Stroke::new(1.15_f32, coral),
                    );
                }
            }
        }
    } else {
        painter.text(
            Pos2::new(timeline_rect.left() + 8.0, strip_rect.top() + 4.0),
            egui::Align2::LEFT_TOP,
            lang.tr("Waveform vocal", "Vocal waveform"),
            egui::FontId::proportional(9.0),
            theme.c32_alpha(theme.semantic.waveform, 112.0 / 255.0),
        );
    }

    let playhead_x = crate::gui::piano_roll::state::timeline_time_to_screen_x(
        content_left,
        state.playhead_ms,
        state.px_per_ms,
        timeline_scroll_x,
    );
    if playhead_x >= timeline_rect.left() && playhead_x <= timeline_rect.right() {
        painter.line_segment(
            [
                Pos2::new(playhead_x, strip_rect.top()),
                Pos2::new(playhead_x, strip_rect.bottom()),
            ],
            Stroke::new(1.2_f32, theme.playhead_c32()),
        );
    }

    // Only the narrow timeline strip takes pointer input; notes and piano keys
    // elsewhere keep their existing edit and navigation gestures.
    let interaction = ui.interact(
        timeline_rect,
        Id::new("processed_waveform_scrub_overlay"),
        Sense::click_and_drag(),
    );
    if interaction.clicked() || interaction.dragged() {
        if let Some(position) = interaction.interact_pointer_pos() {
            let timeline_origin_x = crate::gui::piano_roll::state::timeline_time_to_screen_x(
                content_left,
                0.0,
                state.px_per_ms,
                timeline_scroll_x,
            );
            let scrubbed_ms = ((position.x - timeline_origin_x) as f64
                / state.px_per_ms.max(0.001) as f64)
                .max(0.0);
            state.playhead_ms = scrubbed_ms;
            on_playhead_scrubbed(scrubbed_ms);
        }
    }
}
