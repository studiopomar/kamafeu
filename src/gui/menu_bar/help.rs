use super::*;

impl KamafeuStudioApp {
    pub(super) fn menu_help(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.language;
        ui.menu_button(lang.tr("Ajuda", "Help"), |ui| {
            if ui
                .button(lang.tr(
                    "Guia de Teclas de Atalho... (F1 / Cmd+?)",
                    "Keyboard Shortcuts Guide... (F1 / Cmd+?)",
                ))
                .clicked()
            {
                self.shortcuts_guide_open = true;
                ui.close_menu();
            }
            if ui
                .button(lang.tr("Modelos de Projeto...", "Project Templates..."))
                .clicked()
            {
                self.templates_dialog_open = true;
                ui.close_menu();
            }
            if ui
                .button(lang.tr("Diagnóstico do Voicebank...", "Voicebank Diagnostics..."))
                .clicked()
            {
                self.voicebank_diagnostic_open = true;
                ui.close_menu();
            }
            if ui
                .checkbox(
                    &mut self.config.discord_rpc_enabled,
                    "Discord Rich Presence",
                )
                .clicked()
            {
                self.persist_config();
                ui.close_menu();
            }
            ui.separator();
            ui.label(
                egui::RichText::new(lang.tr(
                    "Kamafeu Studio v1.0.0-A (Âmbar)",
                    "Kamafeu Studio v1.0.0-A (Amber)",
                ))
                .strong()
                .size(11.5)
                .color(egui::Color32::from_rgb(255, 191, 0)),
            );
            ui.label(
                egui::RichText::new(lang.tr(
                    "Motor Vocal OpenUTAU / UTAU em Rust",
                    "OpenUTAU / UTAU Vocal Engine in Rust",
                ))
                .size(9.5)
                .color(MelodyneTheme::TEXT_MUTED),
            );
        });
    }
}
