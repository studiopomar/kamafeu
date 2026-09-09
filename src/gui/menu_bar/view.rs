use super::*;

impl KamafeuStudioApp {
    pub(super) fn menu_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let lang = self.config.language;
        ui.menu_button(lang.tr("Exibir", "View"), |ui| {
            if ui
                .button(lang.tr(
                    "Aumentar Zoom X  (Ctrl+= / Cmd+=)",
                    "Zoom In X  (Ctrl+= / Cmd+=)",
                ))
                .clicked()
            {
                self.piano_roll_state.px_per_ms = (self.piano_roll_state.px_per_ms * 1.25).min(1.5);
                ui.close_menu();
            }
            if ui
                .button(lang.tr(
                    "Diminuir Zoom X  (Ctrl+- / Cmd+-)",
                    "Zoom Out X  (Ctrl+- / Cmd+-)",
                ))
                .clicked()
            {
                self.piano_roll_state.px_per_ms = (self.piano_roll_state.px_per_ms * 0.8).max(0.05);
                ui.close_menu();
            }
            if ui
                .button(lang.tr(
                    "Redefinir Zoom Padrão (Ctrl+0 / Cmd+0)",
                    "Reset Default Zoom (Ctrl+0 / Cmd+0)",
                ))
                .clicked()
            {
                self.piano_roll_state.px_per_ms = 0.25;
                self.piano_roll_state.row_height = 22.0;
                ui.close_menu();
            }
            if ui
                .button(lang.tr(
                    "Enquadrar Projeto Inteiro (Zoom Fit)",
                    "Fit Project (Zoom Fit)",
                ))
                .clicked()
            {
                self.zoom_fit_all_notes();
                ui.close_menu();
            }
            ui.separator();
            ui.menu_button(
                lang.tr(
                    "Escala de Interface (DPI / Zoom)",
                    "Interface Scale (DPI / Zoom)",
                ),
                |ui| {
                    let scales = [
                        (
                            lang.tr("75% (Muito Compacto)", "75% (Very Compact)"),
                            0.75f32,
                        ),
                        (lang.tr("85% (Compacto)", "85% (Compact)"), 0.85f32),
                        (lang.tr("90% (Espaçoso)", "90% (Spacious)"), 0.90f32),
                        (lang.tr("100% (Padrão)", "100% (Default)"), 1.00f32),
                        (lang.tr("110% (Ampliado)", "110% (Enlarged)"), 1.10f32),
                        (
                            lang.tr("125% (Grande / HiDPI)", "125% (Large / HiDPI)"),
                            1.25f32,
                        ),
                        (
                            lang.tr("150% (Muito Grande)", "150% (Very Large)"),
                            1.50f32,
                        ),
                    ];
                    for (label, s) in scales {
                        let is_active = (self.config.ui_scale_factor - s).abs() < 0.02;
                        if ui.selectable_label(is_active, label).clicked() {
                            self.config.ui_scale_factor = s;
                            self.persist_config();
                            ui.close_menu();
                        }
                    }
                },
            );
            ui.separator();
            if ui
                .checkbox(
                    &mut self.piano_roll_state.show_arrangement_view,
                    lang.tr(
                        "Painel de Multifaixas / Arrangement (A)",
                        "Multitrack / Arrangement View (A)",
                    ),
                )
                .clicked()
            {
                ui.close_menu();
            }
            if ui
                .checkbox(
                    &mut self.piano_roll_state.show_parameters_drawer,
                    lang.tr(
                        "Painel de Parâmetros e Expressões (Tab)",
                        "Parameters & Expressions Drawer (Tab)",
                    ),
                )
                .clicked()
            {
                ui.close_menu();
            }
            if ui
                .checkbox(
                    &mut self.piano_roll_state.show_phoneme_ruler,
                    lang.tr(
                        "Régua de Fonemas e Envelopes OTO (Alt+O)",
                        "Phoneme & OTO Envelope Ruler (Alt+O)",
                    ),
                )
                .clicked()
            {
                ui.close_menu();
            }
            if ui
                .checkbox(
                    &mut self.piano_roll_state.show_inspector,
                    lang.tr(
                        "Painel Lateral / Inspetor (Cmd+B / Ctrl+B)",
                        "Right Sidebar / Inspector (Cmd+B / Ctrl+B)",
                    ),
                )
                .clicked()
            {
                ui.close_menu();
            }
            if ui
                .checkbox(
                    &mut self.piano_roll_state.is_maximized,
                    lang.tr(
                        "Maximizar Piano Roll / Otimizar Espaço (F11 / Shift+F)",
                        "Maximize Piano Roll / Optimize Space (F11 / Shift+F)",
                    ),
                )
                .clicked()
            {
                ui.close_menu();
            }
            if ui
                .checkbox(
                    &mut self.render_log_window_open,
                    lang.tr(
                        "Janela de Log do Engine DSP (Ctrl+L)",
                        "DSP Engine Log Window (Ctrl+L)",
                    ),
                )
                .clicked()
            {
                ui.close_menu();
            }
            ui.separator();
            ui.menu_button(lang.tr("Tema do Aplicativo", "App Theme"), |ui| {
                for preset in crate::gui::theme::ThemePreset::ALL {
                    let is_active = self.config.theme.preset == preset;
                    if ui
                        .selectable_label(is_active, preset.display_name_for(lang))
                        .clicked()
                    {
                        self.config.theme.apply_preset(preset);
                        ctx.set_visuals(self.config.theme.create_egui_visuals());
                        self.persist_config();
                        ui.close_menu();
                    }
                }
            });
            if ui
                .button(lang.tr(
                    "Editor Visual de Temas (Tempo Real)...  (Ctrl+Alt+T)",
                    "Visual Theme Editor (Real-Time)...  (Ctrl+Alt+T)",
                ))
                .clicked()
            {
                self.theme_editor_dialog_state.is_open = true;
                ui.close_menu();
            }
            if ui
                .button(lang.tr(
                    "Personalizar Tema e Cores...",
                    "Customize Theme & Colors...",
                ))
                .clicked()
            {
                self.theme_customizer_open = true;
                ui.close_menu();
            }
            if ui
                .button(lang.tr(
                    "Preferências & Configurações... (Ctrl+,)",
                    "Preferences & Settings... (Ctrl+,)",
                ))
                .clicked()
            {
                self.preferences_window_open = true;
                ui.close_menu();
            }
            ui.separator();
            ui.menu_button(lang.tr("Auto-Rolagem", "Auto-Scroll"), |ui| {
                let mode = &mut self.piano_roll_state.auto_scroll_mode;
                let is_off = *mode == crate::gui::types::AutoScrollMode::Off;
                let is_stationary = *mode == crate::gui::types::AutoScrollMode::StationaryCursor;
                let is_page = *mode == crate::gui::types::AutoScrollMode::PageScroll;

                if ui
                    .selectable_label(is_off, lang.tr("Desligar", "Off"))
                    .clicked()
                {
                    *mode = crate::gui::types::AutoScrollMode::Off;
                    ui.close_menu();
                }
                if ui
                    .selectable_label(
                        is_stationary,
                        lang.tr("Cursor Estacionário", "Stationary Cursor"),
                    )
                    .clicked()
                {
                    *mode = crate::gui::types::AutoScrollMode::StationaryCursor;
                    ui.close_menu();
                }
                if ui
                    .selectable_label(is_page, lang.tr("Rolagem de Página", "Page Scroll"))
                    .clicked()
                {
                    *mode = crate::gui::types::AutoScrollMode::PageScroll;
                    ui.close_menu();
                }
            });
        });
    }
}
