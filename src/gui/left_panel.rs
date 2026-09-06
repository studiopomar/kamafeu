use crate::gui::phoneme_palette::{draw_phoneme_palette, PhonemePaletteState};
use crate::gui::theme::MelodyneTheme;
use crate::oto::Voicebank;
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Rounding, Stroke, Vec2};

pub use crate::renderer::RenderOptions as VocalModeParams;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LeftSidebarTab {
    #[default]
    VoiceMode,
    Phonemes,
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
        ui.horizontal(|ui| {
            let voice_tab_text = if *active_tab == LeftSidebarTab::VoiceMode {
                RichText::new("Voz")
                    .strong()
                    .color(MelodyneTheme::ACCENT_GOLD)
            } else {
                RichText::new("Voz").color(MelodyneTheme::TEXT_MUTED)
            };
            if ui.button(voice_tab_text).clicked() {
                *active_tab = LeftSidebarTab::VoiceMode;
            }

            let phonemes_tab_text = if *active_tab == LeftSidebarTab::Phonemes {
                RichText::new("Fonemas")
                    .strong()
                    .color(MelodyneTheme::ACCENT_GOLD)
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
                egui::ScrollArea::vertical()
                    .id_salt("left_panel_voice_scroll")
                    .show(ui, |ui| {
                        let singer_name = voicebank
                            .map(|v| v.name.as_str())
                            .unwrap_or("Cantor Padrão");

                        let (card_rect, _) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 65.0),
                            egui::Sense::hover(),
                        );
                        let painter = ui.painter_at(card_rect);
                        painter.rect_filled(
                            card_rect,
                            Rounding::same(6.0),
                            MelodyneTheme::BG_PANEL,
                        );
                        painter.rect_stroke(
                            card_rect,
                            Rounding::same(6.0),
                            Stroke::new(1.0, MelodyneTheme::GRID_LINE_BAR),
                        );

                        let avatar_rect = egui::Rect::from_min_size(
                            card_rect.min + Vec2::new(8.0, 8.0),
                            Vec2::new(48.0, 48.0),
                        );

                        let mut loaded_image = false;
                        if let Some(vb) = voicebank {
                            if let Some(ref img_path) = vb.image_path {
                                if let Some(handle) =
                                    crate::gui::image_cache::texture_for_path(ui.ctx(), img_path)
                                {
                                    painter.image(
                                        handle.id(),
                                        avatar_rect,
                                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                        Color32::WHITE,
                                    );
                                    loaded_image = true;
                                }
                            }
                        }

                        if !loaded_image {
                            painter.rect_filled(
                                avatar_rect,
                                Rounding::same(4.0),
                                Color32::from_rgb(45, 35, 60),
                            );
                            painter.rect_stroke(
                                avatar_rect,
                                Rounding::same(4.0),
                                Stroke::new(1.0, MelodyneTheme::ACCENT_GOLD),
                            );
                            let initial = singer_name
                                .chars()
                                .next()
                                .unwrap_or('V')
                                .to_uppercase()
                                .to_string();
                            painter.text(
                                avatar_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                initial,
                                egui::FontId::proportional(22.0),
                                MelodyneTheme::ACCENT_GOLD,
                            );
                        }

                        let info_rect = Rect::from_min_max(
                            card_rect.min + Vec2::new(64.0, 10.0),
                            card_rect.max - Vec2::new(8.0, 8.0),
                        );

                        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(info_rect), |ui| {
                            ui.vertical(|ui| {
                                ui.label(
                                    RichText::new(singer_name)
                                        .strong()
                                        .size(13.0)
                                        .color(MelodyneTheme::TEXT_GOLD_LABEL),
                                );
                                let vb_status = if voicebank.is_some() {
                                    "Voicebank Ativo"
                                } else {
                                    "Modo Padrão (Sem VB)"
                                };
                                ui.label(
                                    RichText::new(vb_status)
                                        .size(10.0)
                                        .color(Color32::from_rgb(0, 255, 157)),
                                );
                            });
                        });

                        ui.add_space(8.0);

                        if ui
                            .button(RichText::new("Trocar Voicebank...").size(11.0))
                            .clicked()
                        {
                            if let Some(folder) = crate::dialogs::FileDialog::new().pick_folder() {
                                on_load_vb(Some(folder));
                            }
                        }

                        if !recent_voicebanks.is_empty() {
                            ui.add_space(4.0);
                            ui.label(
                                RichText::new("Recentes:")
                                    .size(10.0)
                                    .color(MelodyneTheme::TEXT_MUTED),
                            );
                            ui.horizontal_wrapped(|ui| {
                                for path in recent_voicebanks {
                                    let folder_name =
                                        path.file_name().and_then(|s| s.to_str()).unwrap_or("VB");
                                    if ui.button(RichText::new(folder_name).size(9.5)).clicked() {
                                        on_load_vb(Some(path.clone()));
                                    }
                                }
                            });
                        }

                        ui.add_space(10.0);
                        ui.separator();
                        ui.add_space(6.0);

                        ui.heading(
                            RichText::new("Modo Vocal")
                                .strong()
                                .size(13.0)
                                .color(MelodyneTheme::TEXT_GOLD_LABEL),
                        );
                        ui.add_space(6.0);

                        ui.label(
                            RichText::new("Modo do Fonemizador").color(MelodyneTheme::TEXT_MUTED),
                        );
                        egui::ComboBox::from_id_salt("phonemizer_mode_cb")
                            .selected_text(match params.phonemizer_mode {
                                crate::phonemizer::PhonemizerMode::None => "Sem Fonemizador (Manual)",
                                crate::phonemizer::PhonemizerMode::BasicCV => "JA: Basic CV",
                                crate::phonemizer::PhonemizerMode::VCV => "JA: Japanese VCV",
                                crate::phonemizer::PhonemizerMode::CVVC => "JA: Japanese CVVC",
                                crate::phonemizer::PhonemizerMode::EnglishArpasing => "EN: English Arpasing (Fonética)",
                                crate::phonemizer::PhonemizerMode::EnglishVCCV => "EN: English VCCV (Fonética)",
                                crate::phonemizer::PhonemizerMode::EnglishG2P => "EN: English G2P (Palavras -> Fonemas)",
                                crate::phonemizer::PhonemizerMode::PortugueseBrapaVCCV => "PT: VCCV BRAPA (xiao / PT-BR 3.7)",
                                crate::phonemizer::PhonemizerMode::PortugueseBrapaCVC => "PT: BRAPA CVC (Fonética)",
                                crate::phonemizer::PhonemizerMode::PortugueseCVVC => "PT: Portuguese CVVC (Fonética)",
                                crate::phonemizer::PhonemizerMode::PortugueseVCV => "PT: Portuguese VCV (Fonética)",
                                crate::phonemizer::PhonemizerMode::PortugueseG2P => "PT: Português G2P (Palavras -> Fonemas)",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut params.phonemizer_mode, crate::phonemizer::PhonemizerMode::None, "🚫 Sem Fonemizador (Manual)");
                                ui.separator();
                                ui.label(RichText::new("[JA] Japonês").strong().color(Color32::from_rgb(255, 215, 0)));
                                ui.selectable_value(&mut params.phonemizer_mode, crate::phonemizer::PhonemizerMode::BasicCV, "  JA: Basic CV");
                                ui.selectable_value(&mut params.phonemizer_mode, crate::phonemizer::PhonemizerMode::VCV, "  JA: Japanese VCV");
                                ui.selectable_value(&mut params.phonemizer_mode, crate::phonemizer::PhonemizerMode::CVVC, "  JA: Japanese CVVC");
                                ui.separator();
                                ui.label(RichText::new("[EN] Inglês").strong().color(Color32::from_rgb(255, 215, 0)));
                                ui.selectable_value(&mut params.phonemizer_mode, crate::phonemizer::PhonemizerMode::EnglishG2P, "  ✨ EN: English G2P (Palavras -> Fonemas)");
                                ui.selectable_value(&mut params.phonemizer_mode, crate::phonemizer::PhonemizerMode::EnglishArpasing, "  🔤 EN: English Arpasing (Fonética Direta)");
                                ui.selectable_value(&mut params.phonemizer_mode, crate::phonemizer::PhonemizerMode::EnglishVCCV, "  🔤 EN: English VCCV (Fonética Direta)");
                                ui.separator();
                                ui.label(RichText::new("[PT] Português").strong().color(Color32::from_rgb(255, 215, 0)));
                                ui.selectable_value(&mut params.phonemizer_mode, crate::phonemizer::PhonemizerMode::PortugueseBrapaVCCV, "  🔥 PT: VCCV BRAPA (xiao / PT-BR 3.7)");
                                ui.selectable_value(&mut params.phonemizer_mode, crate::phonemizer::PhonemizerMode::PortugueseG2P, "  ✨ PT: Português G2P (Palavras -> Fonemas)");
                                ui.selectable_value(&mut params.phonemizer_mode, crate::phonemizer::PhonemizerMode::PortugueseBrapaCVC, "  🔤 PT: BRAPA CVC (Fonética Direta)");
                                ui.selectable_value(&mut params.phonemizer_mode, crate::phonemizer::PhonemizerMode::PortugueseCVVC, "  🔤 PT: Portuguese CVVC (Fonética Direta)");
                                ui.selectable_value(&mut params.phonemizer_mode, crate::phonemizer::PhonemizerMode::PortugueseVCV, "  🔤 PT: Portuguese VCV (Fonética Direta)");
                            });
                        ui.add_space(4.0);

                        ui.label(RichText::new("Volume / Ganho").color(MelodyneTheme::TEXT_MUTED));
                        ui.add(egui::Slider::new(&mut params.loudness, -12.0..=12.0).suffix(" dB"));
                        ui.add_space(4.0);

                        ui.label(RichText::new("Tensão").color(MelodyneTheme::TEXT_MUTED));
                        ui.add(egui::Slider::new(&mut params.tension, 0.0..=100.0).suffix(" %"));
                        ui.add_space(4.0);

                        ui.label(RichText::new("Soprosidade").color(MelodyneTheme::TEXT_MUTED));
                        ui.add(
                            egui::Slider::new(&mut params.breathiness, 0.0..=100.0).suffix(" %"),
                        );
                        ui.add_space(4.0);

                        ui.label(RichText::new("Gênero").color(MelodyneTheme::TEXT_MUTED));
                        ui.add(egui::Slider::new(&mut params.gender, -100.0..=100.0).suffix(" %"));
                        ui.add_space(4.0);

                        ui.label(
                            RichText::new("Deslocamento de Tom").color(MelodyneTheme::TEXT_MUTED),
                        );
                        ui.add(
                            egui::Slider::new(&mut params.tone_shift, -12.0..=12.0).suffix(" smt"),
                        );

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);

                        ui.heading(
                            RichText::new("Suavização de Transições")
                                .strong()
                                .size(13.0)
                                .color(egui::Color32::from_rgb(0, 255, 157)),
                        );
                        ui.add_space(4.0);

                        ui.label(
                            RichText::new("Sobreposição (Fonemas)")
                                .color(MelodyneTheme::TEXT_MUTED),
                        );
                        ui.add(
                            egui::Slider::new(&mut params.crossfade_ms, 0.0..=200.0).suffix(" ms"),
                        );
                        ui.add_space(4.0);

                        ui.label(
                            RichText::new("Predefinições de Transição:")
                                .size(11.0)
                                .color(MelodyneTheme::TEXT_GOLD_LABEL),
                        );
                        ui.horizontal(|ui| {
                            if ui.button(RichText::new("Orgânico").size(10.0)).clicked() {
                                params.loudness = 1.0;
                                params.tension = 40.0;
                                params.breathiness = 10.0;
                                params.gender = 0.0;
                                params.crossfade_ms = 60.0;
                            }
                            if ui.button(RichText::new("Pop Natural").size(10.0)).clicked() {
                                params.loudness = 2.0;
                                params.tension = 70.0;
                                params.breathiness = 0.0;
                                params.gender = 0.0;
                                params.crossfade_ms = 40.0;
                            }
                            if ui
                                .button(RichText::new("Direto / Robótico").size(10.0))
                                .clicked()
                            {
                                params.loudness = 0.0;
                                params.tension = 90.0;
                                params.breathiness = 0.0;
                                params.gender = 0.0;
                                params.crossfade_ms = 5.0;
                            }
                        });
                    });
            }

            LeftSidebarTab::Phonemes => {
                draw_phoneme_palette(
                    ui,
                    voicebank,
                    phoneme_state,
                    on_preview_phoneme,
                    on_insert_phoneme,
                    &mut |_| {},
                );
            }
        }
    });
}
