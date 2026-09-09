use super::*;

impl KamafeuStudioApp {
    pub(super) fn menu_vocal_modes(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.language;
        ui.menu_button(lang.tr("Modos Vocais", "Vocal Modes"), |ui| {
            ui.menu_button(
                lang.tr("Presets Vocais Rápidos", "Quick Vocal Presets"),
                |ui| {
                    if ui
                        .button(lang.tr("Sussurrado / Whisper", "Whisper"))
                        .clicked()
                    {
                        self.apply_vocal_preset("whisper");
                        ui.close_menu();
                    }
                    if ui
                        .button(lang.tr("Belting / Potente", "Belting / Powerful"))
                        .clicked()
                    {
                        self.apply_vocal_preset("belting");
                        ui.close_menu();
                    }
                    if ui
                        .button(lang.tr("Robótico / Hard-Tune", "Robotic / Hard-Tune"))
                        .clicked()
                    {
                        self.apply_vocal_preset("robotic");
                        ui.close_menu();
                    }
                    if ui
                        .button(lang.tr("Natural / Estúdio", "Natural / Studio"))
                        .clicked()
                    {
                        self.apply_vocal_preset("natural");
                        ui.close_menu();
                    }
                },
            );
            ui.separator();

            let mode = &mut self.vocal_mode_params.phonemizer_mode;
            if ui
                .radio_value(
                    mode,
                    crate::phonemizer::PhonemizerMode::None,
                    lang.tr("• Sem Fonemizador (Manual)", "• No Phonemizer (Manual)"),
                )
                .clicked()
            {
                ui.close_menu();
            }
            ui.separator();
            ui.label(
                egui::RichText::new(lang.tr("[JA] Japonês", "[JA] Japanese"))
                    .strong()
                    .color(egui::Color32::from_rgb(255, 215, 0)),
            );
            if ui
                .radio_value(
                    mode,
                    crate::phonemizer::PhonemizerMode::BasicCV,
                    "  JA: Basic CV (Hiragana)",
                )
                .clicked()
            {
                ui.close_menu();
            }
            if ui
                .radio_value(
                    mode,
                    crate::phonemizer::PhonemizerMode::VCV,
                    "  JA: Japanese VCV (- あ, a か)",
                )
                .clicked()
            {
                ui.close_menu();
            }
            if ui
                .radio_value(
                    mode,
                    crate::phonemizer::PhonemizerMode::CVVC,
                    "  JA: Japanese CVVC",
                )
                .clicked()
            {
                ui.close_menu();
            }
            ui.separator();
            ui.label(
                egui::RichText::new(lang.tr("[EN] Inglês", "[EN] English"))
                    .strong()
                    .color(egui::Color32::from_rgb(255, 215, 0)),
            );
            if ui
                .radio_value(
                    mode,
                    crate::phonemizer::PhonemizerMode::EnglishG2P,
                    lang.tr(
                        "  EN: English G2P (Palavras -> Fonemas)",
                        "  EN: English G2P (Words -> Phonemes)",
                    ),
                )
                .clicked()
            {
                ui.close_menu();
            }
            if ui
                .radio_value(
                    mode,
                    crate::phonemizer::PhonemizerMode::EnglishArpasing,
                    lang.tr(
                        "  EN: English Arpasing (Fonética Direta)",
                        "  EN: English Arpasing (Direct Phonetics)",
                    ),
                )
                .clicked()
            {
                ui.close_menu();
            }
            if ui
                .radio_value(
                    mode,
                    crate::phonemizer::PhonemizerMode::EnglishVCCV,
                    lang.tr(
                        "  EN: English VCCV (Fonética Direta)",
                        "  EN: English VCCV (Direct Phonetics)",
                    ),
                )
                .clicked()
            {
                ui.close_menu();
            }
            ui.separator();
            ui.label(
                egui::RichText::new(lang.tr("[PT] Português", "[PT] Portuguese"))
                    .strong()
                    .color(egui::Color32::from_rgb(255, 215, 0)),
            );
            if ui
                .radio_value(
                    mode,
                    crate::phonemizer::PhonemizerMode::PortugueseG2P,
                    lang.tr(
                        "  PT: Português G2P (Palavras -> Fonemas)",
                        "  PT: Portuguese G2P (Words -> Phonemes)",
                    ),
                )
                .clicked()
            {
                ui.close_menu();
            }
            if ui
                .radio_value(
                    mode,
                    crate::phonemizer::PhonemizerMode::PortugueseBrapaVCCV,
                    "  PT: VCCV BRAPA (xiao / PT-BR 3.7)",
                )
                .clicked()
            {
                ui.close_menu();
            }
            if ui
                .radio_value(
                    mode,
                    crate::phonemizer::PhonemizerMode::PortugueseBrapaCVC,
                    lang.tr(
                        "  PT: BRAPA CVC (Fonética Direta)",
                        "  PT: BRAPA CVC (Direct Phonetics)",
                    ),
                )
                .clicked()
            {
                ui.close_menu();
            }
            if ui
                .radio_value(
                    mode,
                    crate::phonemizer::PhonemizerMode::PortugueseCVVC,
                    lang.tr(
                        "  PT: Portuguese CVVC (Fonética Direta)",
                        "  PT: Portuguese CVVC (Direct Phonetics)",
                    ),
                )
                .clicked()
            {
                ui.close_menu();
            }
            if ui
                .radio_value(
                    mode,
                    crate::phonemizer::PhonemizerMode::PortugueseVCV,
                    lang.tr(
                        "  PT: Portuguese VCV (Fonética Direta)",
                        "  PT: Portuguese VCV (Direct Phonetics)",
                    ),
                )
                .clicked()
            {
                ui.close_menu();
            }
            ui.separator();
            if ui
                .button(lang.tr("Forçar Atualização de Fonemas", "Force Phonemes Refresh"))
                .clicked()
            {
                self.rephonemize_all_notes();
                ui.close_menu();
            }
        });
    }
}
