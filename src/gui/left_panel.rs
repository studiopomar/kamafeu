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
    recent_voicebanks: &[std::path::PathBuf],
    active_tab: &mut LeftSidebarTab,
    params: &mut VocalModeParams,
    phoneme_state: &mut PhonemePaletteState,
    on_load_vb: &mut dyn FnMut(Option<std::path::PathBuf>),
    on_preview_phoneme: &mut dyn FnMut(&str),
    on_insert_phoneme: &mut dyn FnMut(&str),
) {
    ui.vertical(|ui| {
        // Tab Header: Voice Mode vs Phonemes (oto.ini)
        ui.horizontal(|ui| {
            let voice_tab_text = if *active_tab == LeftSidebarTab::VoiceMode {
                RichText::new("Voz").strong().color(MelodyneTheme::ACCENT_GOLD)
            } else {
                RichText::new("Voz").color(MelodyneTheme::TEXT_MUTED)
            };
            if ui.button(voice_tab_text).clicked() {
                *active_tab = LeftSidebarTab::VoiceMode;
            }

            let phonemes_tab_text = if *active_tab == LeftSidebarTab::Phonemes {
                RichText::new("Fonemas").strong().color(MelodyneTheme::ACCENT_GOLD)
            } else {
                RichText::new("Fonemas").color(MelodyneTheme::TEXT_MUTED)
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
                let singer_name = voicebank.map(|v| v.name.as_str()).unwrap_or("Cantor Padrão");

                // Singer Card (Melodyne Metallic Gold Style)
                let (card_rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 65.0), egui::Sense::hover());
                let painter = ui.painter_at(card_rect);
                painter.rect_filled(card_rect, Rounding::same(6.0), MelodyneTheme::BG_PANEL);
                painter.rect_stroke(card_rect, Rounding::same(6.0), Stroke::new(1.0, MelodyneTheme::GRID_LINE_BAR));

                // Singer Avatar Box (Image or Letter Badge)
                let avatar_rect = egui::Rect::from_min_size(
                    card_rect.min + Vec2::new(8.0, 8.0),
                    Vec2::new(48.0, 48.0),
                );

                let mut loaded_image = false;
                if let Some(vb) = voicebank {
                    if let Some(ref img_path) = vb.image_path {
                        if let Ok(img) = image::open(img_path) {
                            let img_w = img.width() as f32;
                            let img_h = img.height() as f32;
                            if img_w > 0.0 && img_h > 0.0 {
                                let aspect = img_w / img_h;
                                let (draw_w, draw_h) = if aspect >= 1.0 {
                                    (48.0, (48.0 / aspect).min(48.0))
                                } else {
                                    ((48.0 * aspect).min(48.0), 48.0)
                                };
                                let draw_rect = egui::Rect::from_center_size(avatar_rect.center(), Vec2::new(draw_w, draw_h));
                                let color_image = egui::ColorImage::from_rgba_unmultiplied([img.width() as _, img.height() as _], img.to_rgba8().as_flat_samples().as_slice());
                                let texture = ui.ctx().load_texture(&format!("left_vb_avatar_{}", vb.name), color_image, Default::default());
                                let uv = egui::Rect::from_min_max(egui::Pos2::new(0.0, 0.0), egui::Pos2::new(1.0, 1.0));
                                painter.image(texture.id(), draw_rect, uv, egui::Color32::WHITE);
                                loaded_image = true;
                            }
                        }
                    }
                }

                if !loaded_image {
                    let initial_letter = singer_name.chars().next().unwrap_or('K').to_uppercase().to_string();
                    painter.rect_filled(avatar_rect, Rounding::same(4.0), MelodyneTheme::NOTE_GOLD_FILL);
                    painter.text(
                        avatar_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        &initial_letter,
                        egui::FontId::proportional(24.0),
                        MelodyneTheme::TEXT_NOTE_TAG,
                    );
                }

                // Singer Details
                painter.text(
                    card_rect.min + Vec2::new(64.0, 12.0),
                    egui::Align2::LEFT_TOP,
                    singer_name,
                    egui::FontId::proportional(13.0),
                    MelodyneTheme::TEXT_GOLD_LABEL,
                );
                painter.text(
                    card_rect.min + Vec2::new(64.0, 32.0),
                    egui::Align2::LEFT_TOP,
                    "Voicebank UTAU",
                    egui::FontId::proportional(11.0),
                    MelodyneTheme::TEXT_MUTED,
                );

                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button("Carregar Voicebank...").clicked() {
                        on_load_vb(None);
                    }

                    if !recent_voicebanks.is_empty() {
                        egui::ComboBox::from_id_salt("recent_vb_combo_left")
                            .selected_text("Recentes...")
                            .show_ui(ui, |ui| {
                                for path in recent_voicebanks {
                                    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("Voicebank");
                                    if ui.button(RichText::new(name).size(11.0)).clicked() {
                                        on_load_vb(Some(path.clone()));
                                    }
                                }
                            });
                    }
                });

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(6.0);

                // Vocal Mode Parameters
                ui.heading(RichText::new("Modo Vocal").strong().size(13.0).color(MelodyneTheme::TEXT_GOLD_LABEL));
                ui.add_space(6.0);

                ui.label(RichText::new("Modo do Fonemizador").color(MelodyneTheme::TEXT_MUTED));
                egui::ComboBox::from_id_salt("phonemizer_mode_cb")
                    .selected_text(match params.phonemizer_mode {
                        crate::phonemizer::PhonemizerMode::BasicCV => "CV Básico",
                        crate::phonemizer::PhonemizerMode::VCV => "VCV",
                        crate::phonemizer::PhonemizerMode::CVVC => "CVVC",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut params.phonemizer_mode, crate::phonemizer::PhonemizerMode::BasicCV, "CV Básico");
                        ui.selectable_value(&mut params.phonemizer_mode, crate::phonemizer::PhonemizerMode::VCV, "VCV");
                        ui.selectable_value(&mut params.phonemizer_mode, crate::phonemizer::PhonemizerMode::CVVC, "CVVC");
                    });
                ui.add_space(4.0);

                ui.label(RichText::new("Volume / Ganho").color(MelodyneTheme::TEXT_MUTED));
                ui.add(egui::Slider::new(&mut params.loudness, -12.0..=12.0).suffix(" dB"));
                ui.add_space(4.0);

                ui.label(RichText::new("Tensão").color(MelodyneTheme::TEXT_MUTED));
                ui.add(egui::Slider::new(&mut params.tension, 0.0..=100.0).suffix(" %"));
                ui.add_space(4.0);

                ui.label(RichText::new("Soprosidade").color(MelodyneTheme::TEXT_MUTED));
                ui.add(egui::Slider::new(&mut params.breathiness, 0.0..=100.0).suffix(" %"));
                ui.add_space(4.0);

                ui.label(RichText::new("Gênero").color(MelodyneTheme::TEXT_MUTED));
                ui.add(egui::Slider::new(&mut params.gender, -100.0..=100.0).suffix(" %"));
                ui.add_space(4.0);

                ui.label(RichText::new("Deslocamento de Tom").color(MelodyneTheme::TEXT_MUTED));
                ui.add(egui::Slider::new(&mut params.tone_shift, -12.0..=12.0).suffix(" smt"));

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);

                // Phoneme Transition & Legato Smoothing Panel
                ui.heading(RichText::new("Suavização de Transições").strong().size(13.0).color(egui::Color32::from_rgb(0, 255, 157)));
                ui.add_space(4.0);

                ui.label(RichText::new("Sobreposição (Fonemas)").color(MelodyneTheme::TEXT_MUTED));
                ui.add(egui::Slider::new(&mut params.crossfade_ms, 0.0..=200.0).suffix(" ms"));
                ui.add_space(4.0);

                ui.label(RichText::new("Deslizamento Legato (Portamento)").color(MelodyneTheme::TEXT_MUTED));
                ui.add(egui::Slider::new(&mut params.legato_glide_ms, 0.0..=300.0).suffix(" ms"));
                ui.add_space(6.0);

                ui.label(RichText::new("Predefinições de Transição:").size(11.0).color(MelodyneTheme::TEXT_GOLD_LABEL));
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
