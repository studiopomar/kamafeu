mod envelope;
mod expressions;

use super::*;

pub(super) fn draw(
    ui: &mut egui::Ui,
    notes: &mut [UNote],
    state: &mut PianoRollState,
    keyboard_width: f32,
    timeline_scroll_x: f32,
    on_before_change: &mut dyn FnMut(),
    theme: &ThemeConfig,
    ruler_rect: Rect,
    bpm: f64,
    lang: crate::config::AppLanguage,
) {
    if state.show_envelope_handles {
        envelope::draw(
            ui,
            notes,
            state,
            keyboard_width,
            timeline_scroll_x,
            on_before_change,
            theme,
            ruler_rect,
            bpm,
        );
    } else if state.show_parameters_drawer {
        expressions::draw(
            ui,
            notes,
            state,
            keyboard_width,
            timeline_scroll_x,
            on_before_change,
            theme,
            ruler_rect,
            bpm,
            lang,
        );
    }
}
