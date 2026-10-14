use crate::gui::KamafeuStudioApp;

impl KamafeuStudioApp {
    pub(super) fn update_activity(&mut self) {
        let note_count = self.current_notes().len();
        let active_part_name = self
            .project
            .parts
            .iter()
            .find(|part| part.track_index == self.active_track_index)
            .or_else(|| self.project.parts.first())
            .map(|part| part.name.as_str())
            .unwrap_or("Parte vocal");
        let selected_count = self
            .piano_roll_state
            .selected_note_indices
            .len()
            .max(usize::from(
                self.piano_roll_state.selected_note_index.is_some(),
            ));

        let is_rendering = self.render_rx.is_some() || self.export_rx.is_some();
        let is_playing = self.piano_roll_state.is_playing;

        let rpc_state = crate::discord_rpc::activity_presentation(
            is_rendering,
            is_playing,
            self.render_progress,
            &self.project.name,
            active_part_name,
            self.voicebank
                .as_ref()
                .map(|v| v.name.as_str())
                .unwrap_or(""),
            note_count,
            selected_count,
            self.transport_state.bpm,
            self.piano_roll_state.playhead_ms,
            self.current_notes()
                .iter()
                .map(|note| note.position_ms + note.duration_ms)
                .fold(0.0_f64, f64::max),
            self.is_dirty,
            self.config.discord_rpc_enabled,
        );

        self.discord_rpc.update(rpc_state);
    }
}
