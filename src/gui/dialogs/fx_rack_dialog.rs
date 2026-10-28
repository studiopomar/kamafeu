use crate::gui::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(super) fn show_fx_rack_dialog(&mut self, ctx: &egui::Context) {
        if self.fx_rack_dialog_state.is_open {
            let lang = self.config.language;
            let theme = self.config.theme.clone();
            let mut state = self.fx_rack_dialog_state;
            let mut fx_config = self.project.master_fx.clone().unwrap_or_default();
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
            if changed && state.target_track.is_none() {
                let snapshot = self.project.clone();
                if ctx.input(|i| i.pointer.primary_down()) {
                    if self.pending_edit_snapshot.is_none() {
                        self.pending_edit_snapshot = Some(snapshot);
                    }
                } else {
                    self.undo_manager
                        .push_state(self.pending_edit_snapshot.take().unwrap_or(snapshot));
                }
                self.project.master_fx = Some(fx_config);
            }
            if changed {
                self.is_dirty = true;
                self.fx_preview_refresh_pending = state.target_track.is_some();
            }
            if self.fx_preview_refresh_pending && !ctx.input(|input| input.pointer.primary_down()) {
                self.fx_preview_refresh_pending = false;
                if self.transport_is_active() {
                    self.pause_audio();
                    self.play_current_track();
                    self.transport_state.status_message = lang
                        .tr(
                            "FX alterado; prévia atualizada na posição atual",
                            "FX changed; preview refreshed at the current position",
                        )
                        .to_string();
                }
            }
        }
    }
}
