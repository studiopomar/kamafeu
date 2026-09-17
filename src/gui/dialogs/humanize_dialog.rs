use crate::gui::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(super) fn show_humanize_dialog(&mut self, ctx: &egui::Context) {
        if self.humanize_dialog_state.is_open {
            let lang = self.config.language;
            let sel = self.piano_roll_state.selected_note_indices.clone();
            let theme = self.config.theme.clone();
            let mut state = self.humanize_dialog_state.clone();
            let pre_project = self.project.clone();
            let mut applied = false;
            {
                let notes = self.current_notes_mut();
                crate::gui::humanize_dialog::draw_humanize_dialog(
                    ctx,
                    lang,
                    &theme,
                    &mut state,
                    notes,
                    &sel,
                    &mut || {
                        applied = true;
                    },
                );
            }
            self.humanize_dialog_state = state;
            if applied {
                self.undo_manager.push_state(pre_project);
                self.is_dirty = true;
                self.transport_state.status_message = lang
                    .tr(
                        "Humanização e vibrato aplicados com sucesso!",
                        "Humanization and vibrato applied successfully!",
                    )
                    .to_string();
            }
        }
    }
}
