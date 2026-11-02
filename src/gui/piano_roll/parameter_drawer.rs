mod expressions;
mod timbre;
mod voicebank_pitch;

use super::*;

pub(super) fn draw(
    ui: &mut egui::Ui,
    notes: &mut [UNote],
    timbre_curve: Option<&mut Vec<crate::project::model::UTimbrePoint>>,
    state: &mut PianoRollState,
    keyboard_width: f32,
    timeline_scroll_x: f32,
    on_before_change: &mut dyn FnMut(),
    theme: &ThemeConfig,
    ruler_rect: Rect,
    bpm: f64,
    time_signature_numerator: u8,
    time_signature_denominator: u8,
    lang: crate::config::AppLanguage,
    voicebank: Option<&Voicebank>,
    selected_resampler: &str,
    on_note_changed: &mut dyn FnMut(),
) {
    if state.show_parameters_drawer {
        expressions::draw(
            ui,
            notes,
            timbre_curve,
            state,
            keyboard_width,
            timeline_scroll_x,
            on_before_change,
            theme,
            ruler_rect,
            bpm,
            time_signature_numerator,
            time_signature_denominator,
            lang,
            voicebank,
            selected_resampler,
            on_note_changed,
        );
    }
}
