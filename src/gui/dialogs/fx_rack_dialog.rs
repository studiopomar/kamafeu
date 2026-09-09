use crate::gui::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(super) fn show_fx_rack_dialog(&mut self, ctx: &egui::Context) {
        if self.fx_rack_dialog_state.is_open {
            let lang = self.config.language;
            let theme = self.config.theme.clone();
            let mut state = self.fx_rack_dialog_state;
            let mut fx_config = self.fx_rack_config.clone();
            let mut changed = false;
            crate::gui::fx_rack_dialog::draw_fx_rack_dialog(
                ctx,
                lang,
                &theme,
                &mut state,
                &mut fx_config,
                &mut self.project.tracks,
                &mut changed,
            );
            self.fx_rack_dialog_state = state;
            self.fx_rack_config = fx_config;
            if changed {
                self.is_dirty = true;
            }
        }
    }
}
