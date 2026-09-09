use super::*;

impl KamafeuStudioApp {
    pub(super) fn menu_edit(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.language;

        ui.menu_button(lang.tr("Editar", "Edit"), |ui| {
            if ui
                .button(lang.tr("Desfazer (Ctrl+Z / Cmd+Z)", "Undo (Ctrl+Z / Cmd+Z)"))
                .clicked()
            {
                self.pending_edit_snapshot = None;
                if let Some(prev) = self.undo_manager.undo(self.project.clone()) {
                    self.project = prev;
                }
                ui.close_menu();
            }
            if ui
                .button(lang.tr(
                    "Refazer (Ctrl+Y / Cmd+Shift+Z)",
                    "Redo (Ctrl+Y / Cmd+Shift+Z)",
                ))
                .clicked()
            {
                self.pending_edit_snapshot = None;
                if let Some(next) = self.undo_manager.redo(self.project.clone()) {
                    self.project = next;
                }
                ui.close_menu();
            }
            ui.separator();
            if ui
                .button(lang.tr("Recortar (Ctrl+X / Cmd+X)", "Cut (Ctrl+X / Cmd+X)"))
                .clicked()
            {
                self.cut_selected_notes();
                ui.close_menu();
            }
            if ui
                .button(lang.tr("Copiar   (Ctrl+C / Cmd+C)", "Copy   (Ctrl+C / Cmd+C)"))
                .clicked()
            {
                self.copy_selected_notes();
                ui.close_menu();
            }
            if ui
                .button(lang.tr("Colar    (Ctrl+V / Cmd+V)", "Paste    (Ctrl+V / Cmd+V)"))
                .clicked()
            {
                self.paste_notes();
                ui.close_menu();
            }
            if ui
                .button(lang.tr(
                    "Duplicar Nota (Ctrl+D / Cmd+D)",
                    "Duplicate Note (Ctrl+D / Cmd+D)",
                ))
                .clicked()
            {
                if let Some(sel_idx) = self.piano_roll_state.selected_note_index {
                    let notes = self.current_notes();
                    if sel_idx < notes.len() {
                        let mut dup = notes[sel_idx].clone();
                        dup.position_ms += dup.duration_ms;
                        self.push_history();
                        self.current_notes_mut().push(dup);
                        self.piano_roll_state.selected_note_index =
                            Some(self.current_notes().len() - 1);
                    }
                }
                ui.close_menu();
            }
            if ui
                .button(lang.tr(
                    "Excluir  (Delete / Backspace)",
                    "Delete  (Delete / Backspace)",
                ))
                .clicked()
            {
                self.delete_selected_notes();
                ui.close_menu();
            }
            ui.separator();
            if ui
                .button(lang.tr(
                    "Selecionar Tudo (Ctrl+A / Cmd+A)",
                    "Select All (Ctrl+A / Cmd+A)",
                ))
                .clicked()
            {
                let note_count = self.current_notes().len();
                self.piano_roll_state.selected_note_indices = (0..note_count).collect();
                if note_count > 0 {
                    self.piano_roll_state.selected_note_index = Some(0);
                }
                ui.close_menu();
            }
            if ui
                .button(lang.tr(
                    "Desmarcar Seleção (Ctrl+Shift+A)",
                    "Deselect All (Ctrl+Shift+A)",
                ))
                .clicked()
            {
                self.piano_roll_state.selected_note_indices.clear();
                self.piano_roll_state.selected_note_index = None;
                ui.close_menu();
            }

            ui.menu_button(lang.tr("Seleção Inteligente", "Smart Selection"), |ui| {
                if ui
                    .button(lang.tr(
                        "Selecionar Notas Curtas (< 120ms)",
                        "Select Short Notes (< 120ms)",
                    ))
                    .clicked()
                {
                    self.select_short_notes(120.0);
                    ui.close_menu();
                }
                if ui
                    .button(lang.tr(
                        "Selecionar Notas Muito Curtas (< 60ms)",
                        "Select Very Short Notes (< 60ms)",
                    ))
                    .clicked()
                {
                    self.select_short_notes(60.0);
                    ui.close_menu();
                }
                if ui
                    .button(lang.tr("Selecionar Notas Sobrepostas", "Select Overlapping Notes"))
                    .clicked()
                {
                    self.select_overlapping_notes();
                    ui.close_menu();
                }
                if ui
                    .button(lang.tr(
                        "Selecionar Fora da Escala Ativa",
                        "Select Notes Out of Active Scale",
                    ))
                    .clicked()
                {
                    self.select_out_of_scale_notes();
                    ui.close_menu();
                }
            });

            ui.separator();
            ui.menu_button(
                lang.tr("Quantização & Legato", "Quantization & Legato"),
                |ui| {
                    if ui
                        .button(lang.tr(
                            "Quantizar Posições (Snap Atual)",
                            "Quantize Positions (Current Snap)",
                        ))
                        .clicked()
                    {
                        self.quantize_positions();
                        ui.close_menu();
                    }
                    if ui
                        .button(lang.tr("Quantizar Durações", "Quantize Durations"))
                        .clicked()
                    {
                        self.quantize_durations();
                        ui.close_menu();
                    }
                    if ui
                        .button(lang.tr(
                            "Corrigir Sobreposição de Notas (Trim Overlaps)",
                            "Fix Note Overlaps (Trim Overlaps)",
                        ))
                        .clicked()
                    {
                        self.fix_overlapping_notes();
                        ui.close_menu();
                    }
                    if ui
                        .button(lang.tr(
                            "Conectar Finais das Notas (Legato)",
                            "Connect Note Ends (Legato)",
                        ))
                        .clicked()
                    {
                        self.legato_connect_notes();
                        ui.close_menu();
                    }
                },
            );

            ui.menu_button(lang.tr("Letras & Fonemas", "Lyrics & Phonemes"), |ui| {
                if ui
                    .button(lang.tr(
                        "Distribuidor Inteligente de Letras... (Ctrl+L / Cmd+L)",
                        "Smart Lyrics Distributor... (Ctrl+L / Cmd+L)",
                    ))
                    .clicked()
                {
                    self.lyrics_dialog_state.is_open = true;
                    ui.close_menu();
                }
                if ui
                    .button(lang.tr(
                        "Inserir Letras em Lote... (Ctrl+Shift+L)",
                        "Insert Batch Lyrics... (Ctrl+Shift+L)",
                    ))
                    .clicked()
                {
                    self.batch_lyrics_open = true;
                    ui.close_menu();
                }
                if ui
                    .button(lang.tr(
                        "Limpar Sufixos de Afinação (_A3, _C4...)",
                        "Clear Pitch Suffixes (_A3, _C4...)",
                    ))
                    .clicked()
                {
                    self.clean_pitch_suffixes_from_lyrics();
                    ui.close_menu();
                }
                if ui
                    .button(lang.tr(
                        "Forçar Atualização de Fonemas da Faixa",
                        "Force Track Phonemes Update",
                    ))
                    .clicked()
                {
                    self.rephonemize_all_notes();
                    ui.close_menu();
                }
            });

            ui.menu_button(
                lang.tr("Transformações Musicais", "Musical Transformations"),
                |ui| {
                    if ui
                        .button(lang.tr(
                            "Humanizador & Auto-Vibrato Inteligente... (Ctrl+H / Cmd+H)",
                            "Smart Humanizer & Auto-Vibrato... (Ctrl+H / Cmd+H)",
                        ))
                        .clicked()
                    {
                        self.humanize_dialog_state.is_open = true;
                        ui.close_menu();
                    }
                    if ui
                        .button(lang.tr(
                            "Humanizar Posições e Velocities",
                            "Humanize Positions & Velocities",
                        ))
                        .clicked()
                    {
                        self.humanize_selection();
                        ui.close_menu();
                    }
                    if ui
                        .button(lang.tr(
                            "Inverter Melodia (Retrógrado Horizontal)",
                            "Invert Melody (Horizontal Retrograde)",
                        ))
                        .clicked()
                    {
                        self.invert_melody_retrograde();
                        ui.close_menu();
                    }
                    if ui
                        .button(lang.tr(
                            "Espelhar Intervalos (Inversão Vertical)",
                            "Mirror Intervals (Vertical Inversion)",
                        ))
                        .clicked()
                    {
                        self.invert_melody_intervals();
                        ui.close_menu();
                    }
                },
            );

            ui.menu_button(lang.tr("Transposição", "Transpose"), |ui| {
                if ui
                    .button(lang.tr(
                        "Transpor +1 Semitom  (Seta Cima)",
                        "Transpose +1 Semitone  (Up Arrow)",
                    ))
                    .clicked()
                {
                    self.transpose_selected_note(1);
                    ui.close_menu();
                }
                if ui
                    .button(lang.tr(
                        "Transpor -1 Semitom  (Seta Baixo)",
                        "Transpose -1 Semitone  (Down Arrow)",
                    ))
                    .clicked()
                {
                    self.transpose_selected_note(-1);
                    ui.close_menu();
                }
                if ui
                    .button(lang.tr(
                        "Transpor +1 Oitava   (Shift + Seta Cima)",
                        "Transpose +1 Octave   (Shift + Up Arrow)",
                    ))
                    .clicked()
                {
                    self.transpose_selected_note(12);
                    ui.close_menu();
                }
                if ui
                    .button(lang.tr(
                        "Transpor -1 Oitava   (Shift + Seta Baixo)",
                        "Transpose -1 Octave   (Shift + Down Arrow)",
                    ))
                    .clicked()
                {
                    self.transpose_selected_note(-12);
                    ui.close_menu();
                }
            });

            ui.separator();
            if ui
                .button(lang.tr(
                    "Pre-tunning...  (Ctrl+Alt+P)",
                    "Pre-tunning...  (Ctrl+Alt+P)",
                ))
                .clicked()
            {
                self.autopitch_window_open = true;
                ui.close_menu();
            }
            if ui
                .button(lang.tr(
                    "Aplicar Pre-tunning Suave/Pop em Tudo",
                    "Apply Soft/Pop Pre-tunning to All",
                ))
                .clicked()
            {
                self.apply_autopitch_all();
                ui.close_menu();
            }
            if ui
                .button(lang.tr(
                    "Limpar Curvas de Pitch de Todas as Notas",
                    "Clear Pitch Curves on All Notes",
                ))
                .clicked()
            {
                self.clear_all_pitch_curves();
                ui.close_menu();
            }
            if ui
                .button(
                    egui::RichText::new(lang.tr(
                        "Limpar Todos os Parâmetros das Notas Selecionadas",
                        "Clear All Parameters of Selected Notes",
                    ))
                    .color(egui::Color32::from_rgb(255, 180, 90)),
                )
                .clicked()
            {
                self.reset_selected_notes_parameters();
                ui.close_menu();
            }
            if ui
                .button(lang.tr(
                    "Resetar Envelopes de Volume para Padrão",
                    "Reset Volume Envelopes to Default",
                ))
                .clicked()
            {
                self.reset_all_volume_envelopes();
                ui.close_menu();
            }
            ui.separator();
            if ui
                .button(lang.tr(
                    "Preferências de Tema e Aparência...  (Ctrl+Alt+T)",
                    "Theme & Appearance Preferences...  (Ctrl+Alt+T)",
                ))
                .clicked()
            {
                self.theme_customizer_open = true;
                ui.close_menu();
            }
        });
    }
}
