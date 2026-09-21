use crate::gui::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(super) fn show_lyrics_dialog(&mut self, ctx: &egui::Context) {
        if self.lyrics_dialog_state.is_open {
            let lang = self.config.language;
            let playhead = self.piano_roll_state.playhead_ms;
            let sel = self.piano_roll_state.selected_note_indices.clone();
            let theme = self.config.theme.clone();
            let mut state = self.lyrics_dialog_state.clone();
            let pre_project = self.project.clone();
            let mut applied = false;
            {
                let notes = self.current_notes_mut();
                crate::gui::lyrics_dialog::draw_lyrics_dialog(
                    ctx,
                    lang,
                    &theme,
                    &mut state,
                    notes,
                    &sel,
                    playhead,
                    &mut || {
                        applied = true;
                    },
                );
            }
            self.lyrics_dialog_state = state;
            if applied {
                self.undo_manager.push_state(pre_project);
                self.is_dirty = true;
                self.piano_roll_state.phoneme_cache.clear();
                self.transport_state.status_message = lang
                    .tr(
                        "Letras distribuídas pelas notas com sucesso!",
                        "Lyrics successfully distributed across notes!",
                    )
                    .to_string();
            }
        }
    }
}
