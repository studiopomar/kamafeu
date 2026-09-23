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
                        (lang.tr("150% (Muito Grande)", "150% (Very Large)"), 1.50f32),
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
                    &mut self.config.layout.modular_workspace,
                    lang.tr(
                        "Workspace modular (painéis movíveis)",
                        "Modular workspace (movable panels)",
                    ),
                )
                .changed()
            {
                self.persist_config();
            }
            if self.config.layout.modular_workspace
                && ui
                    .button(lang.tr(
                        "Restaurar posições do workspace",
                        "Restore workspace positions",
                    ))
                    .clicked()
            {
                ctx.memory_mut(|memory| memory.reset_areas());
                self.workspace_snap_rect = None;
                ui.close_menu();
            }
            ui.menu_button(lang.tr("Painéis", "Panels"), |ui| {
                let panels = [
                    (
                        &mut self.piano_roll_state.show_arrangement_view,
                        "Painel de Arranjo / Multifaixas (A)",
                        "Arrangement / Multitrack Panel (A)",
                    ),
                    (
                        &mut self.piano_roll_state.show_parameters_drawer,
                        "Painel de Parâmetros e Expressões (Tab)",
                        "Parameters & Expressions Panel (Tab)",
                    ),
                    (
                        &mut self.piano_roll_state.show_phoneme_ruler,
                        "Régua de Fonemas (Alt+O)",
                        "Phoneme Ruler (Alt+O)",
                    ),
                    (
                        &mut self.piano_roll_state.show_envelope_handles,
                        "Envelopes de Volume (O)",
                        "Volume Envelopes (O)",
                    ),
                    (
                        &mut self.piano_roll_state.show_inspector,
                        "Inspetor contextual (Cmd+B / Ctrl+B)",
                        "Contextual Inspector (Cmd+B / Ctrl+B)",
                    ),
                ];
                let mut panels_changed = false;
                for (visible, pt, en) in panels {
                    if ui.checkbox(visible, lang.tr(pt, en)).clicked() {
                        panels_changed = true;
                    }
                }
                if panels_changed {
                    self.persist_config();
                }
            });
            if ui
                .checkbox(
                    &mut self.piano_roll_state.is_maximized,
                    lang.tr(
                        "Maximizar Piano Roll / Otimizar Espaço (Shift+F / Alt+M)",
                        "Maximize Piano Roll / Optimize Space (Shift+F / Alt+M)",
                    ),
                )
                .clicked()
            {
                self.persist_config();
            }
            let is_fs = ctx.input(|i| i.viewport().fullscreen.unwrap_or(false));
            let fs_label = if is_fs {
                lang.tr("Sair de Tela Cheia (F11)", "Exit Fullscreen (F11)")
            } else {
                lang.tr("Janela em Tela Cheia (F11)", "Fullscreen Window (F11)")
            };
            if ui.button(fs_label).clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(!is_fs));
                ui.close_menu();
            }
            let is_max_win = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
            let max_win_label = if is_max_win {
                lang.tr("Restaurar Tamanho da Janela", "Restore Window Size")
            } else {
                lang.tr("Maximizar Janela do Sistema", "Maximize System Window")
            };
            if ui.button(max_win_label).clicked() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!is_max_win));
                ui.close_menu();
            }
            ui.checkbox(
                &mut self.render_log_window_open,
                lang.tr(
                    "Janela de Log do Engine DSP (Ctrl+L)",
                    "DSP Engine Log Window (Ctrl+L)",
                ),
            );
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
