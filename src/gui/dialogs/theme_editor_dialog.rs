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
                // The theme editor mutates the persisted config, but egui
                // keeps its own Visuals snapshot. Apply the new snapshot
                // immediately so global text and widget colors change without
                // requiring an application restart.
                ctx.set_visuals(self.config.theme.create_egui_visuals());
                self.persist_config();
            }
        }
    }
}
