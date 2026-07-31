use eframe::egui::{self, Color32, RichText, Rounding, Stroke, Vec2};
use crate::gui::phoneme_palette::{draw_phoneme_palette, PhonemePaletteState};
use crate::gui::theme::MelodyneTheme;
use crate::oto::Voicebank;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeftSidebarTab {
    VoiceMode,
    Phonemes,
}

impl Default for LeftSidebarTab {
    fn default() -> Self {
        LeftSidebarTab::VoiceMode
    }
}

#[derive(Debug, Clone)]
pub struct VocalModeParams {
    pub loudness: f64,       // -12.0 to +12.0 dB
    pub tension: f64,        // 0.0 to 100.0 %
    pub breathiness: f64,    // 0.0 to 100.0 %
    pub gender: f64,         // -100.0 to +100.0 %
    pub tone_shift: f64,     // -12.0 to +12.0 semitones
    pub crossfade_ms: f64,   // 0.0 to 200.0 ms (Phoneme transition crossfade)
    pub legato_glide_ms: f64,// 0.0 to 300.0 ms (Legato pitch portamento glide)
    pub phonemizer_mode: crate::phonemizer::PhonemizerMode,
}

impl Default for VocalModeParams {
    fn default() -> Self {
        Self {
            loudness: 0.0,
            tension: 20.0,
            breathiness: 15.0,
            gender: 0.0,
            tone_shift: 0.0,
            crossfade_ms: 45.0,
            legato_glide_ms: 85.0,
            phonemizer_mode: crate::phonemizer::PhonemizerMode::BasicCV,
        }
    }
}

pub fn draw_left_panel(
    ui: &mut egui::Ui,
    voicebank: Option<&Voicebank>,
    active_tab: &mut LeftSidebarTab,
    params: &mut VocalModeParams,
    phoneme_state: &mut PhonemePaletteState,
    on_load_vb: &mut dyn FnMut(),
    on_preview_phoneme: &mut dyn FnMut(&str),
    on_insert_phoneme: &mut dyn FnMut(&str),
) {
    ui.vertical(|ui| {
        // Tab Header: Voice Mode vs Phonemes (oto.ini)
        ui.horizontal(|ui| {
            let voice_btn = egui::Button::new(
                RichText::new("🎙 Cantor & Voz")
                    .strong()
                    .size(11.0)
                    .color(if *active_tab == LeftSidebarTab::VoiceMode { Color32::BLACK } else { MelodyneTheme::TEXT_SECONDARY })
            )
            .fill(if *active_tab == LeftSidebarTab::VoiceMode { MelodyneTheme::ACCENT_GOLD } else { MelodyneTheme::BG_CARD })
            .stroke(Stroke::new(1.0, if *active_tab == LeftSidebarTab::VoiceMode { Color32::BLACK } else { MelodyneTheme::BORDER_FINE }))
            .rounding(Rounding::same(3.0));

            if ui.add(voice_btn).clicked() {
                *active_tab = LeftSidebarTab::VoiceMode;
            }

            let phonemes_btn = egui::Button::new(
                RichText::new("🔤 Fonemas")
                    .strong()
                    .size(11.0)
                    .color(if *active_tab == LeftSidebarTab::Phonemes { Color32::BLACK } else { MelodyneTheme::TEXT_SECONDARY })
            )
            .fill(if *active_tab == LeftSidebarTab::Phonemes { MelodyneTheme::ACCENT_CYAN } else { MelodyneTheme::BG_CARD })
            .stroke(Stroke::new(1.0, if *active_tab == LeftSidebarTab::Phonemes { Color32::BLACK } else { MelodyneTheme::BORDER_FINE }))
            .rounding(Rounding::same(3.0));

            if ui.add(phonemes_btn).clicked() {
                *active_tab = LeftSidebarTab::Phonemes;
            }
        });

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);

        match active_tab {
            LeftSidebarTab::VoiceMode => {
                let singer_name = voicebank.map(|v| v.name.as_str()).unwrap_or("Singer Padrão");

                // Singer Card (Delicate Neo-Brutalist Card)
                let (card_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 65.0), egui::Sense::hover());
                ui.painter().rect_filled(card_rect, Rounding::same(3.0), MelodyneTheme::BG_CARD);
                ui.painter().rect_stroke(card_rect, Rounding::same(3.0), Stroke::new(1.0, MelodyneTheme::BORDER_GOLD));

                // Singer Avatar Badge
                let avatar_rect = egui::Rect::from_min_size(
                    card_rect.min + Vec2::new(8.0, 8.0),
                    Vec2::new(48.0, 48.0),
                );
                ui.painter().rect_filled(avatar_rect, Rounding::same(2.0), MelodyneTheme::ACCENT_GOLD);
                ui.painter().rect_stroke(avatar_rect, Rounding::same(2.0), Stroke::new(1.0, Color32::BLACK));
                ui.painter().text(
                    avatar_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "K",
                    egui::FontId::proportional(24.0),
                    MelodyneTheme::TEXT_NOTE_TAG,
                );

                // Singer Details
                ui.painter().text(
                    card_rect.min + Vec2::new(64.0, 12.0),
                    egui::Align2::LEFT_TOP,
                    singer_name,
                    egui::FontId::proportional(13.0),
                    MelodyneTheme::TEXT_GOLD_LABEL,
                );
                ui.painter().text(
                    card_rect.min + Vec2::new(64.0, 32.0),
                    egui::Align2::LEFT_TOP,
                    "UTAU Voicebank",
                    egui::FontId::proportional(11.0),
                    MelodyneTheme::TEXT_MUTED,
                );

                ui.add_space(8.0);
                let load_btn = egui::Button::new(RichText::new("📁 Carregar Voicebank...").strong().size(11.0).color(Color32::BLACK))
                    .fill(MelodyneTheme::ACCENT_GOLD)
                    .stroke(Stroke::new(1.0, Color32::BLACK))
                    .rounding(Rounding::same(3.0));

                if ui.add(load_btn).clicked() {
                    on_load_vb();
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                // Vocal Mode Parameters
                ui.label(
                    RichText::new(" MODO VOCAL ")
                        .strong()
                        .size(9.5)
                        .color(MelodyneTheme::TEXT_NOTE_TAG)
                        .background_color(MelodyneTheme::ACCENT_GOLD)
                );
                ui.add_space(6.0);

                ui.label(RichText::new("Modo Fonemizador").size(10.5).color(MelodyneTheme::TEXT_SECONDARY));
                egui::ComboBox::from_id_salt("phonemizer_mode_cb")
                    .selected_text(format!("{:?}", params.phonemizer_mode))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut params.phonemizer_mode, crate::phonemizer::PhonemizerMode::BasicCV, "Basic CV");
                        ui.selectable_value(&mut params.phonemizer_mode, crate::phonemizer::PhonemizerMode::VCV, "VCV");
                        ui.selectable_value(&mut params.phonemizer_mode, crate::phonemizer::PhonemizerMode::CVVC, "CVVC");
                    });
                ui.add_space(4.0);

                ui.label(RichText::new("Volume (Loudness)").size(10.5).color(MelodyneTheme::TEXT_SECONDARY));
                ui.add(egui::Slider::new(&mut params.loudness, -12.0..=12.0).suffix(" dB"));
                ui.add_space(4.0);

                ui.label(RichText::new("Tensão (Tension)").size(10.5).color(MelodyneTheme::TEXT_SECONDARY));
                ui.add(egui::Slider::new(&mut params.tension, 0.0..=100.0).suffix(" %"));
                ui.add_space(4.0);

                ui.label(RichText::new("Soprosidade (Breathiness)").size(10.5).color(MelodyneTheme::TEXT_SECONDARY));
                ui.add(egui::Slider::new(&mut params.breathiness, 0.0..=100.0).suffix(" %"));
                ui.add_space(4.0);

                ui.label(RichText::new("Gênero (Gender)").size(10.5).color(MelodyneTheme::TEXT_SECONDARY));
                ui.add(egui::Slider::new(&mut params.gender, -100.0..=100.0).suffix(" %"));
                ui.add_space(4.0);

                ui.label(RichText::new("Transposição de Tom").size(10.5).color(MelodyneTheme::TEXT_SECONDARY));
                ui.add(egui::Slider::new(&mut params.tone_shift, -12.0..=12.0).suffix(" smt"));

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                // Phoneme Transition & Legato Smoothing Panel
                ui.label(
                    RichText::new(" TRANSIÇÕES & LEGATO ")
                        .strong()
                        .size(9.5)
                        .color(Color32::BLACK)
                        .background_color(MelodyneTheme::ACCENT_CYAN)
                );
                ui.add_space(6.0);

                ui.label(RichText::new("Crossfade Overlap").size(10.5).color(MelodyneTheme::TEXT_SECONDARY));
                ui.add(egui::Slider::new(&mut params.crossfade_ms, 0.0..=200.0).suffix(" ms"));
                ui.add_space(4.0);

                ui.label(RichText::new("Deslizamento Legato").size(10.5).color(MelodyneTheme::TEXT_SECONDARY));
                ui.add(egui::Slider::new(&mut params.legato_glide_ms, 0.0..=300.0).suffix(" ms"));
                ui.add_space(6.0);

                ui.label(RichText::new("Presets de Transição:").size(10.5).color(MelodyneTheme::TEXT_GOLD_LABEL));
                ui.horizontal(|ui| {
                    if ui.button(RichText::new("Orgânico").size(10.0)).clicked() {
                        params.loudness = 1.0;
                        params.tension = 40.0;
                        params.breathiness = 10.0;
                        params.gender = 0.0;
                        params.crossfade_ms = 60.0;
                        params.legato_glide_ms = 120.0;
                    }
                    if ui.button(RichText::new("Pop Natural").size(10.0)).clicked() {
                        params.loudness = 2.0;
                        params.tension = 70.0;
                        params.breathiness = 0.0;
                        params.gender = 0.0;
                        params.crossfade_ms = 40.0;
                        params.legato_glide_ms = 80.0;
                    }
                    if ui.button(RichText::new("Robótico").size(10.0)).clicked() {
                        params.loudness = 0.0;
                        params.tension = 90.0;
                        params.breathiness = 0.0;
                        params.gender = 0.0;
                        params.crossfade_ms = 5.0;
                        params.legato_glide_ms = 10.0;
                    }
                });
            }

            LeftSidebarTab::Phonemes => {
                draw_phoneme_palette(
                    ui,
                    voicebank,
                    phoneme_state,
                    on_preview_phoneme,
                    on_insert_phoneme,
                );
            }
        }
    });
}
