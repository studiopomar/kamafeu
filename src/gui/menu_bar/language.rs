use super::*;
use crate::config::AppLanguage;

impl KamafeuStudioApp {
    pub(super) fn menu_language(&mut self, ui: &mut egui::Ui) {
        let is_pt = self.config.language == AppLanguage::PtBr;
        let menu_label = if is_pt { "Idioma" } else { "Language" };

        ui.menu_button(menu_label, |ui| {
            ui.label(
                egui::RichText::new(if is_pt {
                    "Idioma da Interface"
                } else {
                    "Interface Language"
                })
                .strong()
                .size(11.5)
                .color(egui::Color32::from_rgb(130, 200, 255)),
            );
            ui.separator();

            let languages = [
                (AppLanguage::PtBr, "Brasileiro (Brasil)"),
                (AppLanguage::EnUs, "Inglês (Global) / English"),
            ];

            for (lang, label) in languages {
                let is_active = self.config.language == lang;
                if ui.selectable_label(is_active, label).clicked() {
                    self.config.language = lang;
                    self.persist_config();
                    self.transport_state.status_message = match lang {
                        AppLanguage::PtBr => "Idioma alterado para Brasileiro (Brasil)".to_string(),
                        AppLanguage::EnUs => "Language switched to English (Global)".to_string(),
                    };
                    ui.close_menu();
                }
            }
        });
    }
}
