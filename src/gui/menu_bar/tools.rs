use super::*;

impl KamafeuStudioApp {
    pub(super) fn menu_tools(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.language;
        ui.menu_button(lang.tr("Ferramentas", "Tools"), |ui| {
            if ui
                .button(lang.tr("Ponteiro (Seleção) [V / 1]", "Pointer (Selection) [V / 1]"))
                .clicked()
            {
                self.piano_roll_state.active_tool = EditTool::Pointer;
                ui.close_menu();
            }
            if ui
                .button(lang.tr("Lápis (Desenhar)   [N / 2]", "Pencil (Draw)      [N / 2]"))
                .clicked()
            {
                self.piano_roll_state.active_tool = EditTool::Pencil;
                ui.close_menu();
            }
            if ui
                .button(lang.tr("Desenhar Pitch     [P / 3]", "Draw Pitch         [P / 3]"))
                .clicked()
            {
                self.piano_roll_state.active_tool = EditTool::PitchDraw;
                ui.close_menu();
            }
            if self.piano_roll_state.active_tool == EditTool::PitchDraw {
                ui.separator();
                ui.label(lang.tr(
                    "  Pontos/segmentos: clique para criar, arraste para mover",
                    "  Points/segments: click to create, drag to move",
                ));
                ui.separator();
            }
            if ui
                .button(lang.tr("Borracha           [E / 4]", "Eraser             [E / 4]"))
                .clicked()
            {
                self.piano_roll_state.active_tool = EditTool::Eraser;
                ui.close_menu();
            }
            ui.separator();
            if ui
                .button(lang.tr(
                    "Pre-tunning (Afinador Orgânico)...",
                    "Pre-tunning (Organic Tuner)...",
                ))
                .clicked()
            {
                self.autopitch_window_open = true;
                ui.close_menu();
            }
            if ui
                .button(lang.tr(
                    "Rack de Efeitos Vocais DSP (EQ, Comp, Delay, Reverb)...  (Ctrl+Alt+F)",
                    "DSP Vocal FX Rack (EQ, Comp, Delay, Reverb)...  (Ctrl+Alt+F)",
                ))
                .clicked()
            {
                self.fx_rack_dialog_state.target_track = Some(self.active_track_index);
                self.fx_rack_dialog_state.is_open = true;
                ui.close_menu();
            }
            if ui
                .button(lang.tr(
                    "Suavizar Curvas de Pitch (Gaussian 3-pts)",
                    "Smooth Pitch Curves (Gaussian 3-pts)",
                ))
                .clicked()
            {
                self.smooth_selected_pitch_curves();
                ui.close_menu();
            }
            if ui
                .button(lang.tr("Paleta de Fonemas", "Phoneme Palette"))
                .clicked()
            {
                self.right_sidebar_tab = crate::gui::types::RightSidebarTab::Phonemes;
                ui.close_menu();
            }
        });
    }
}
