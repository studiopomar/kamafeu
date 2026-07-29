use eframe::egui::{self, RichText, Rounding, Stroke, Vec2};
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
            let voice_tab_text = if *active_tab == LeftSidebarTab::VoiceMode {
                RichText::new("🎙 Voice").strong().color(MelodyneTheme::ACCENT_GOLD)
            } else {
                RichText::new("🎙 Voice").color(MelodyneTheme::TEXT_MUTED)
            };
            if ui.button(voice_tab_text).clicked() {
                *active_tab = LeftSidebarTab::VoiceMode;
            }

            let phonemes_tab_text = if *active_tab == LeftSidebarTab::Phonemes {
                RichText::new("🔤 Phonemes").strong().color(MelodyneTheme::ACCENT_GOLD)
            } else {
                RichText::new("🔤 Phonemes").color(MelodyneTheme::TEXT_MUTED)
            };
            if ui.button(phonemes_tab_text).clicked() {
                *active_tab = LeftSidebarTab::Phonemes;
            }
        });

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);

        match active_tab {
            LeftSidebarTab::VoiceMode => {
                let singer_name = voicebank.map(|v| v.name.as_str()).unwrap_or("Default Singer");

                // Singer Card (Melodyne Metallic Gold Style)
                let (card_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 65.0), egui::Sense::hover());
                ui.painter().rect_filled(card_rect, Rounding::same(6.0), MelodyneTheme::BG_PANEL);
                ui.painter().rect_stroke(card_rect, Rounding::same(6.0), Stroke::new(1.0, MelodyneTheme::GRID_LINE_BAR));

                // Singer Avatar Badge (Metallic Gold)
                let avatar_rect = egui::Rect::from_min_size(
                    card_rect.min + Vec2::new(8.0, 8.0),
                    Vec2::new(48.0, 48.0),
                );
                ui.painter().rect_filled(avatar_rect, Rounding::same(4.0), MelodyneTheme::NOTE_GOLD_FILL);
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
                if ui.button("Load Voicebank...").clicked() {
                    on_load_vb();
                }

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                // Vocal Mode Parameters
                ui.heading(RichText::new("Vocal Mode").strong().size(13.0).color(MelodyneTheme::TEXT_GOLD_LABEL));
                ui.add_space(6.0);

                ui.label(RichText::new("Loudness").color(MelodyneTheme::TEXT_MUTED));
                ui.add(egui::Slider::new(&mut params.loudness, -12.0..=12.0).suffix(" dB"));
                ui.add_space(4.0);

                ui.label(RichText::new("Tension").color(MelodyneTheme::TEXT_MUTED));
                ui.add(egui::Slider::new(&mut params.tension, 0.0..=100.0).suffix(" %"));
                ui.add_space(4.0);

                ui.label(RichText::new("Breathiness").color(MelodyneTheme::TEXT_MUTED));
                ui.add(egui::Slider::new(&mut params.breathiness, 0.0..=100.0).suffix(" %"));
                ui.add_space(4.0);

                ui.label(RichText::new("Gender").color(MelodyneTheme::TEXT_MUTED));
                ui.add(egui::Slider::new(&mut params.gender, -100.0..=100.0).suffix(" %"));
                ui.add_space(4.0);

                ui.label(RichText::new("Tone Shift").color(MelodyneTheme::TEXT_MUTED));
                ui.add(egui::Slider::new(&mut params.tone_shift, -12.0..=12.0).suffix(" smt"));

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                // Phoneme Transition & Legato Smoothing Panel
                ui.heading(RichText::new("🌊 Suavização de Transições").strong().size(13.0).color(egui::Color32::from_rgb(0, 255, 157)));
                ui.add_space(4.0);

                ui.label(RichText::new("Crossfade Overlap (Fonemas)").color(MelodyneTheme::TEXT_MUTED));
                ui.add(egui::Slider::new(&mut params.crossfade_ms, 0.0..=200.0).suffix(" ms"));
                ui.add_space(4.0);

                ui.label(RichText::new("Deslizamento Legato (Portamento)").color(MelodyneTheme::TEXT_MUTED));
                ui.add(egui::Slider::new(&mut params.legato_glide_ms, 0.0..=300.0).suffix(" ms"));
                ui.add_space(6.0);

                ui.label(RichText::new("Presets de Transição:").size(11.0).color(MelodyneTheme::TEXT_GOLD_LABEL));
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
                    if ui.button(RichText::new("Direto / Robótico").size(10.0)).clicked() {
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
