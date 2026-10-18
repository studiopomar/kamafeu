use super::*;
use crate::gui::phoneme_palette::draw_phoneme_palette;

#[allow(clippy::too_many_arguments)]
pub(super) fn draw(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    lang: crate::config::AppLanguage,
    voicebank: Option<&Voicebank>,
    phoneme_state: &mut PhonemePaletteState,
    on_preview_phoneme: &mut dyn FnMut(&str),
    on_insert_phoneme: &mut dyn FnMut(&str),
    on_edit_phoneme: &mut dyn FnMut(&str),
) {
    draw_phoneme_palette(
        ui,
        theme,
        lang,
        voicebank,
        phoneme_state,
        on_preview_phoneme,
        on_insert_phoneme,
        on_edit_phoneme,
    );
}
