use crate::gui::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(super) fn show_theme_editor_dialog(&mut self, ctx: &egui::Context) {
        if self.theme_editor_dialog_state.is_open {
            let lang = self.config.language;
            let mut theme = self.config.theme.clone();
            let mut state = self.theme_editor_dialog_state;
            let mut changed = false;
            crate::gui::theme_editor_dialog::draw_theme_editor_dialog(
                ctx,
                lang,
                &mut theme,
                &mut state,
                &mut || {
                    changed = true;
                },
            );
            self.theme_editor_dialog_state = state;
            if changed {
                self.config.theme = theme;
                self.persist_config();
            }
        }
    }
}
