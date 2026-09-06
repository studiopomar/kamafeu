use crate::gui::phoneme_palette::{draw_phoneme_palette, PhonemePaletteState};
use crate::gui::theme::MelodyneTheme;
use crate::gui::types::RightSidebarTab;
use crate::oto::Voicebank;
use crate::project::model::UNote;
pub use crate::renderer::RenderOptions as VocalModeParams;
use eframe::egui::{self, Color32, Frame, Pos2, Rect, RichText, Rounding, Stroke, Vec2};
use std::path::PathBuf;

pub fn draw_unified_panel(
    ui: &mut egui::Ui,
    voicebank: Option<&Voicebank>,
    recent_voicebanks: &[PathBuf],
    singers_list: &[crate::oto::SingerInfo],
    singer_search_query: &mut String,
    singers_paths: &mut Vec<PathBuf>,
    vocal_mode_params: &mut VocalModeParams,
    selected_note_idx: Option<usize>,
    notes: &mut [UNote],
    selected_indices: &std::collections::HashSet<usize>,
    selected_ruler_alias: Option<&str>,
    active_tab: &mut RightSidebarTab,
    phoneme_state: &mut PhonemePaletteState,
    render_threads: &mut u32,
    sample_rate: &mut u32,
    selected_resampler: &mut String,
    selected_wavtool: &mut String,
    custom_resampler_path: &mut Option<PathBuf>,
    custom_wavtool_path: &mut Option<PathBuf>,
    discord_rpc_enabled: &mut bool,
    on_load_vb: &mut dyn FnMut(Option<PathBuf>),
    on_add_singers_dir: &mut dyn FnMut(),
    on_reload_singers: &mut dyn FnMut(),
    on_open_gallery: &mut dyn FnMut(),
    on_preview_phoneme: &mut dyn FnMut(&str),
    on_insert_phoneme: &mut dyn FnMut(&str),
    on_edit_phoneme: &mut dyn FnMut(&str),
    on_edit_selected_ruler_alias: &mut dyn FnMut(),
) {
    let vb_name = voicebank
        .map(|v| v.name.as_str())
        .unwrap_or("Cantor Padrão");
    let vb_author = voicebank
        .map(|v| v.author.as_str())
        .unwrap_or("UTAU Voicebank");
    let initial_letter = vb_name
        .chars()
        .next()
        .unwrap_or('V')
        .to_uppercase()
        .to_string();

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            let tabs = [
                (RightSidebarTab::SingerTrack, "👤 Cantor"),
                (RightSidebarTab::Note, "📝 Nota"),
                (RightSidebarTab::Phonemes, "🔤 Fonemas"),
                (RightSidebarTab::Engine, "⚙️ Motor"),
            ];

            for (tab, label) in tabs {
                let is_selected = *active_tab == tab;
                let text_color = if is_selected {
                    Color32::from_rgb(0, 255, 157)
                } else {
                    MelodyneTheme::TEXT_MUTED
                };
                if ui.selectable_label(is_selected, RichText::new(label).color(text_color).strong()).clicked() {
                    *active_tab = tab;
                }
            }
        });

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);

        match active_tab {
            RightSidebarTab::SingerTrack => {
                egui::ScrollArea::vertical().id_salt("right_panel_singer_scroll").show(ui, |ui| {
                    Frame::none()
                        .fill(Color32::from_rgb(26, 20, 38))
                        .rounding(Rounding::same(6.0))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(61, 46, 84)))
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.add_space((ui.available_width() - 80.0).max(0.0) * 0.5);
                                let (avatar_rect, _) = ui.allocate_exact_size(Vec2::new(80.0, 80.0), egui::Sense::hover());
                                let painter = ui.painter_at(avatar_rect);

                                painter.rect_filled(avatar_rect, Rounding::same(6.0), Color32::from_rgb(36, 27, 53));
                                painter.rect_stroke(avatar_rect, Rounding::same(6.0), Stroke::new(1.2_f32, Color32::from_rgb(192, 132, 252)));

                                let mut loaded_image = false;
                                if let Some(vb) = voicebank {
                                    if let Some(ref img_path) = vb.image_path {
                                        if let Some(texture) = crate::gui::image_cache::texture_for_path(ui.ctx(), img_path) {
                                            let [img_w, img_h] = texture.size();
                                            let img_w = img_w as f32;
                                            let img_h = img_h as f32;
                                            if img_w > 0.0 && img_h > 0.0 {
                                                let aspect = img_w / img_h;
                                                let (draw_w, draw_h) = if aspect >= 1.0 {
                                                    (76.0, (76.0 / aspect).min(76.0))
                                                } else {
                                                    ((76.0 * aspect).min(76.0), 76.0)
                                                };

                                                let draw_rect = Rect::from_center_size(avatar_rect.center(), Vec2::new(draw_w, draw_h));
                                                let uv = egui::Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0));
                                                painter.image(texture.id(), draw_rect, uv, Color32::WHITE);
                                                loaded_image = true;
                                            }
                                        }
                                    }
                                }

                                if !loaded_image {
                                    let center = avatar_rect.center();
                                    painter.circle_filled(Pos2::new(center.x, center.y - 8.0), 16.0, Color32::from_rgb(0, 255, 157));
                                    painter.text(
                                        Pos2::new(center.x, center.y - 8.0),
                                        egui::Align2::CENTER_CENTER,
                                        &initial_letter,
                                        egui::FontId::proportional(16.0),
                                        Color32::from_rgb(20, 16, 28),
                                    );

                                    painter.text(
                                        Pos2::new(center.x, center.y + 16.0),
                                        egui::Align2::CENTER_CENTER,
                                        vb_name,
                                        egui::FontId::proportional(10.0),
                                        Color32::WHITE,
                                    );
                                }
                            });

                            ui.add_space(6.0);
                            ui.vertical_centered(|ui| {
                                ui.label(RichText::new(vb_name).strong().size(12.5).color(MelodyneTheme::TEXT_GOLD_LABEL));
                                ui.label(RichText::new(format!("Autor: {}", vb_author)).size(9.5).color(MelodyneTheme::TEXT_MUTED));
                            });

                            if let Some(vb) = voicebank {
                                if !vb.character_info.is_empty() || !vb.readme_info.is_empty() {
                                    ui.add_space(4.0);
                                    egui::CollapsingHeader::new(RichText::new("Descrição do Cantor").size(10.0).color(MelodyneTheme::TEXT_GOLD_LABEL))
                                        .show(ui, |ui| {
                                            egui::ScrollArea::vertical().id_salt("right_panel_vb_scroll").max_height(80.0).show(ui, |ui| {
                                                if !vb.character_info.is_empty() {
                                                    ui.label(RichText::new(&vb.character_info).size(9.0).color(Color32::from_rgb(216, 180, 254)));
                                                }
                                                if !vb.readme_info.is_empty() {
                                                    ui.separator();
                                                    ui.label(RichText::new(&vb.readme_info).size(9.0).color(Color32::from_rgb(180, 220, 254)));
                                                }
                                            });
                                        });
                                }
                            }
                        });

                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("🎭 Abrir Galeria").size(10.5).color(Color32::from_rgb(0, 220, 255))).clicked() {
                            on_open_gallery();
                        }
                        if ui.button(RichText::new("📁 Outro Cantor...").size(10.5)).clicked() {
                            on_load_vb(None);
                        }
                    });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        ui.heading(RichText::new(format!("🎤 Cantores Instalados ({})", singers_list.len())).strong().size(11.5).color(MelodyneTheme::TEXT_GOLD_LABEL));
                        ui.add_space(ui.available_width() - 28.0);
                        if ui.small_button("🔄").on_hover_text("Recarregar lista de cantores").clicked() {
                            on_reload_singers();
                        }
                    });

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("🔍").size(11.0));
                        ui.add(egui::TextEdit::singleline(singer_search_query).hint_text("Filtrar cantores...").desired_width(ui.available_width()));
                    });

                    let query_lower = singer_search_query.to_lowercase();
                    let filtered_singers: Vec<_> = singers_list
                        .iter()
                        .filter(|s| {
                            query_lower.is_empty()
                                || s.name.to_lowercase().contains(&query_lower)
                                || s.author.to_lowercase().contains(&query_lower)
                        })
                        .collect();

                    ui.add_space(6.0);
                    egui::ScrollArea::vertical()
                        .id_salt("singers_quick_list_scroll")
                        .max_height(140.0)
                        .show(ui, |ui| {
                            if filtered_singers.is_empty() {
                                ui.label(RichText::new("Nenhum cantor encontrado nesta pasta.").size(9.5).italics().color(MelodyneTheme::TEXT_MUTED));
                            } else {
                                for singer in filtered_singers {
                                    let is_current = voicebank.is_some_and(|v| v.root_path == singer.path);
                                    let item_bg = if is_current {
                                        Color32::from_rgb(45, 30, 70)
                                    } else {
                                        Color32::from_rgb(28, 22, 40)
                                    };

                                    Frame::none()
                                        .fill(item_bg)
                                        .rounding(Rounding::same(4.0))
                                        .stroke(Stroke::new(1.0_f32, if is_current { Color32::from_rgb(192, 132, 252) } else { Color32::from_rgb(48, 38, 68) }))
                                        .inner_margin(egui::Margin::symmetric(6.0, 4.0))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                let (thumb_rect, _) = ui.allocate_exact_size(Vec2::new(24.0, 24.0), egui::Sense::hover());
                                                let thumb_painter = ui.painter_at(thumb_rect);
                                                thumb_painter.rect_filled(thumb_rect, Rounding::same(3.0), Color32::from_rgb(40, 30, 58));

                                                let mut thumb_loaded = false;
                                                if let Some(ref img_path) = singer.image_path {
                                                    if let Some(tex) = crate::gui::image_cache::texture_for_path(ui.ctx(), img_path) {
                                                        thumb_painter.image(
                                                            tex.id(),
                                                            thumb_rect,
                                                            Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0)),
                                                            Color32::WHITE,
                                                        );
                                                        thumb_loaded = true;
                                                    }
                                                }
                                                if !thumb_loaded {
                                                    let initial = singer.name.chars().next().unwrap_or('V');
                                                    thumb_painter.text(
                                                        thumb_rect.center(),
                                                        egui::Align2::CENTER_CENTER,
                                                        initial.to_string(),
                                                        egui::FontId::proportional(12.0),
                                                        Color32::from_rgb(0, 255, 157),
                                                    );
                                                }

                                                ui.add_space(4.0);
                                                ui.vertical(|ui| {
                                                    ui.label(RichText::new(&singer.name).strong().size(10.5).color(if is_current { Color32::from_rgb(0, 255, 180) } else { Color32::WHITE }));
                                                    ui.label(RichText::new(&singer.author).size(8.5).color(MelodyneTheme::TEXT_MUTED));
                                                });

                                                ui.add_space(ui.available_width() - 40.0);
                                                if !is_current {
                                                    if ui.small_button("Usar").clicked() {
                                                        on_load_vb(Some(singer.path.clone()));
                                                    }
                                                } else {
                                                    ui.label(RichText::new("✓").strong().size(11.0).color(Color32::from_rgb(0, 255, 180)));
                                                }
                                            });
                                        });
                                    ui.add_space(2.0);
                                }
                            }
                        });

                    ui.add_space(6.0);
                    egui::CollapsingHeader::new(RichText::new("📁 Gerenciar Pastas do OpenUtau / Singers").size(10.0).color(MelodyneTheme::TEXT_GOLD_LABEL))
                        .show(ui, |ui| {
                            if ui.button(RichText::new("➕ Adicionar Pasta de Cantores...").size(9.5)).clicked() {
                                on_add_singers_dir();
                            }
                            ui.add_space(4.0);
                            let mut to_remove = None;
                            for (p_idx, p) in singers_paths.iter().enumerate() {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new(format!("• {}", p.display())).size(8.5).color(MelodyneTheme::TEXT_MUTED));
                                    if ui.small_button("✖").on_hover_text("Remover esta pasta").clicked() {
                                        to_remove = Some(p_idx);
                                    }
                                });
                            }
                            if let Some(idx) = to_remove {
                                singers_paths.remove(idx);
                                on_reload_singers();
                            }
                        });

                    if !recent_voicebanks.is_empty() {
                        ui.add_space(4.0);
                        ui.label(RichText::new("Recentes:").size(9.5).color(MelodyneTheme::TEXT_MUTED));
                        ui.horizontal_wrapped(|ui| {
                            for path in recent_voicebanks {
                                let folder_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("VB");
                                if ui.button(RichText::new(folder_name).size(9.0)).clicked() {
                                    on_load_vb(Some(path.clone()));
                                }
                            }
                        });
                    }

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(6.0);

                    ui.heading(RichText::new("Modo Vocal (Global)").strong().size(12.5).color(MelodyneTheme::TEXT_GOLD_LABEL));
                    ui.add_space(4.0);

                    ui.label(RichText::new("Fonetizador:").size(10.5).color(MelodyneTheme::TEXT_MUTED));
                    egui::ComboBox::from_id_salt("phonemizer_mode_cb_unified")
                        .selected_text(match vocal_mode_params.phonemizer_mode {
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
                            ui.selectable_value(&mut vocal_mode_params.phonemizer_mode, crate::phonemizer::PhonemizerMode::None, "🚫 Sem Fonemizador (Manual)");
                            ui.separator();
                            ui.label(RichText::new("[JA] Japonês").strong().color(Color32::from_rgb(255, 215, 0)));
                            ui.selectable_value(&mut vocal_mode_params.phonemizer_mode, crate::phonemizer::PhonemizerMode::BasicCV, "  JA: Basic CV");
                            ui.selectable_value(&mut vocal_mode_params.phonemizer_mode, crate::phonemizer::PhonemizerMode::VCV, "  JA: Japanese VCV");
                            ui.selectable_value(&mut vocal_mode_params.phonemizer_mode, crate::phonemizer::PhonemizerMode::CVVC, "  JA: Japanese CVVC");
                            ui.separator();
                            ui.label(RichText::new("[EN] Inglês").strong().color(Color32::from_rgb(255, 215, 0)));
                            ui.selectable_value(&mut vocal_mode_params.phonemizer_mode, crate::phonemizer::PhonemizerMode::EnglishG2P, "  ✨ EN: English G2P (Palavras -> Fonemas)");
                            ui.selectable_value(&mut vocal_mode_params.phonemizer_mode, crate::phonemizer::PhonemizerMode::EnglishArpasing, "  🔤 EN: English Arpasing (Fonética Direta)");
                            ui.selectable_value(&mut vocal_mode_params.phonemizer_mode, crate::phonemizer::PhonemizerMode::EnglishVCCV, "  🔤 EN: English VCCV (Fonética Direta)");
                            ui.separator();
                            ui.label(RichText::new("[PT] Português").strong().color(Color32::from_rgb(255, 215, 0)));
                            ui.selectable_value(&mut vocal_mode_params.phonemizer_mode, crate::phonemizer::PhonemizerMode::PortugueseBrapaVCCV, "  🔥 PT: VCCV BRAPA (xiao / PT-BR 3.7)");
                            ui.selectable_value(&mut vocal_mode_params.phonemizer_mode, crate::phonemizer::PhonemizerMode::PortugueseG2P, "  ✨ PT: Português G2P (Palavras -> Fonemas)");
                            ui.selectable_value(&mut vocal_mode_params.phonemizer_mode, crate::phonemizer::PhonemizerMode::PortugueseBrapaCVC, "  🔤 PT: BRAPA CVC (Fonética Direta)");
                            ui.selectable_value(&mut vocal_mode_params.phonemizer_mode, crate::phonemizer::PhonemizerMode::PortugueseCVVC, "  🔤 PT: Portuguese CVVC (Fonética Direta)");
                            ui.selectable_value(&mut vocal_mode_params.phonemizer_mode, crate::phonemizer::PhonemizerMode::PortugueseVCV, "  🔤 PT: Portuguese VCV (Fonética Direta)");
                        });
                    ui.add_space(4.0);

                    ui.label(RichText::new("Volume / Ganho").size(10.5).color(MelodyneTheme::TEXT_MUTED));
                    ui.add(egui::Slider::new(&mut vocal_mode_params.loudness, -12.0..=12.0).suffix(" dB"));
                    ui.add_space(4.0);

                    ui.label(RichText::new("Tensão").size(10.5).color(MelodyneTheme::TEXT_MUTED));
                    ui.add(egui::Slider::new(&mut vocal_mode_params.tension, 0.0..=100.0).suffix(" %"));
                    ui.add_space(4.0);

                    ui.label(RichText::new("Soprosidade").size(10.5).color(MelodyneTheme::TEXT_MUTED));
                    ui.add(egui::Slider::new(&mut vocal_mode_params.breathiness, 0.0..=100.0).suffix(" %"));
                    ui.add_space(4.0);

                    ui.label(RichText::new("Gênero").size(10.5).color(MelodyneTheme::TEXT_MUTED));
                    ui.add(egui::Slider::new(&mut vocal_mode_params.gender, -100.0..=100.0).suffix(" %"));
                    ui.add_space(4.0);

                    ui.label(RichText::new("Deslocamento de Tom").size(10.5).color(MelodyneTheme::TEXT_MUTED));
                    ui.add(egui::Slider::new(&mut vocal_mode_params.tone_shift, -12.0..=12.0).suffix(" smt"));
                    ui.add_space(4.0);

                    ui.label(RichText::new("Sobreposição (Consoantes)").size(10.5).color(MelodyneTheme::TEXT_MUTED));
                    ui.add(egui::Slider::new(&mut vocal_mode_params.crossfade_ms, 0.0..=200.0).suffix(" ms"));

                    ui.add_space(6.0);
                    ui.label(RichText::new("Predefinições de Transição:").size(10.0).color(MelodyneTheme::TEXT_GOLD_LABEL));
                    ui.horizontal(|ui| {
                        if ui.button(RichText::new("Orgânico").size(9.5)).clicked() {
                            vocal_mode_params.loudness = 1.0;
                            vocal_mode_params.tension = 40.0;
                            vocal_mode_params.breathiness = 10.0;
                            vocal_mode_params.gender = 0.0;
                            vocal_mode_params.crossfade_ms = 60.0;
                        }
                        if ui.button(RichText::new("Pop Natural").size(9.5)).clicked() {
                            vocal_mode_params.loudness = 2.0;
                            vocal_mode_params.tension = 70.0;
                            vocal_mode_params.breathiness = 0.0;
                            vocal_mode_params.gender = 0.0;
                            vocal_mode_params.crossfade_ms = 40.0;
                        }
                        if ui.button(RichText::new("Robótico").size(9.5)).clicked() {
                            vocal_mode_params.loudness = 0.0;
                            vocal_mode_params.tension = 90.0;
                            vocal_mode_params.breathiness = 0.0;
                            vocal_mode_params.gender = 0.0;
                            vocal_mode_params.crossfade_ms = 5.0;
                        }
                    });
                });
            }

            RightSidebarTab::Note => {
                egui::ScrollArea::vertical().id_salt("right_panel_note_scroll").show(ui, |ui| {
                    if let Some(target_idx) = selected_note_idx {
                        if target_idx < notes.len() {
                            if selected_indices.len() > 1 {
                                Frame::none()
                                    .fill(Color32::from_rgb(40, 30, 10))
                                    .rounding(Rounding::same(4.0))
                                    .stroke(Stroke::new(1.0_f32, MelodyneTheme::ACCENT_GOLD))
                                    .inner_margin(egui::Margin::same(6.0))
                                    .show(ui, |ui| {
                                        ui.label(RichText::new(format!("Grupo: {} notas", selected_indices.len())).strong().size(11.0).color(MelodyneTheme::ACCENT_GOLD));
                                        ui.label(RichText::new("Alterações aplicam-se a todas as notas selecionadas").size(10.0).color(MelodyneTheme::TEXT_MUTED));
                                    });
                                ui.add_space(6.0);
                            }

                            let mut lyric = notes[target_idx].lyric.clone();
                            let pitch_str = notes[target_idx].pitch.clone();
                            let pos_ms = notes[target_idx].position_ms;
                            let mut dur_ms = notes[target_idx].duration_ms;
                            let mut gender = notes[target_idx].expressions.gender;
                            let mut dynamics = notes[target_idx].expressions.dynamics;
                            let mut pitch_delta = notes[target_idx].expressions.pitch_delta;
                            let mut breathiness = notes[target_idx].expressions.breathiness;
                            let mut consonant_velocity = notes[target_idx].expressions.consonant_velocity;
                            let mut volume = notes[target_idx].expressions.volume;
                            let mut attack = notes[target_idx].expressions.attack;
                            let mut decay = notes[target_idx].expressions.decay;
                            let mut fade_in_ms = notes[target_idx].envelope.p2;
                            let mut fade_out_ms = notes[target_idx].envelope.p5;
                            let mut note_crossfade_ms = notes[target_idx].envelope.crossfade_ms;
                            let mut vibrato = notes[target_idx].vibrato.clone();
                            let mut portamento_start = notes[target_idx].pitch_bend.portamento_start_ms;
                            let mut portamento_length = notes[target_idx].pitch_bend.portamento_length_ms;
                            let mut portamento_shape = notes[target_idx].pitch_bend.portamento_shape.clone();
                            let mut snap_first = notes[target_idx].pitch_bend.snap_first;

                            let mut changed_lyric = false;
                            let mut changed_dur = false;
                            let mut changed_gender = false;
                            let mut changed_dynamics = false;
                            let mut changed_pitch = false;
                            let mut changed_breath = false;
                            let mut changed_timing = false;
                            let mut changed_amplitude = false;
                            let mut changed_fades = false;
                            let mut changed_vibrato = false;
                            let mut changed_portamento = false;

                            Frame::none()
                                .fill(Color32::from_rgb(36, 27, 53))
                                .rounding(Rounding::same(4.0))
                                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(61, 46, 84)))
                                .inner_margin(egui::Margin::same(8.0))
                                .show(ui, |ui| {
                                    ui.label(RichText::new("Informações Básicas").strong().size(11.0).color(Color32::from_rgb(0, 255, 157)));
                                    ui.separator();

                                    ui.horizontal(|ui| {
                                        ui.label("Letra:");
                                        if ui.text_edit_singleline(&mut lyric).changed() {
                                            changed_lyric = true;
                                        }
                                    });
                                    ui.add_enabled_ui(selected_ruler_alias.is_some(), |ui| {
                                        let label = selected_ruler_alias
                                            .map(|alias| format!("Editar {alias} no Copaiba NEO"))
                                            .unwrap_or_else(|| {
                                                "Selecione um alias na régua inferior".to_string()
                                            });
                                        if ui.button(label).clicked() {
                                            on_edit_selected_ruler_alias();
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Tom / Nota:");
                                        ui.label(RichText::new(&pitch_str).strong().color(Color32::from_rgb(216, 180, 254)));
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Posição Inicial:");
                                        ui.label(format!("{:.1} ms", pos_ms));
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Duração:");
                                        if ui.add(egui::DragValue::new(&mut dur_ms).range(20.0..=10000.0).speed(5.0).suffix(" ms")).changed() {
                                            changed_dur = true;
                                        }
                                    });
                                });

                            ui.add_space(8.0);

                            Frame::none()
                                .fill(Color32::from_rgb(26, 20, 38))
                                .rounding(Rounding::same(4.0))
                                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(61, 46, 84)))
                                .inner_margin(egui::Margin::same(8.0))
                                .show(ui, |ui| {
                                    ui.label(RichText::new("Parâmetros da Nota").strong().size(11.0).color(Color32::from_rgb(0, 255, 157)));
                                    ui.separator();

                                    ui.horizontal(|ui| {
                                        ui.label("Gênero:");
                                        if ui.add(egui::Slider::new(&mut gender, -100.0..=100.0)).changed() {
                                            changed_gender = true;
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Dinâmica:");
                                        if ui.add(egui::Slider::new(&mut dynamics, -240.0..=120.0).suffix(" (0.1 dB)")).changed() {
                                            changed_dynamics = true;
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Pitch Cents:");
                                        if ui.add(egui::Slider::new(&mut pitch_delta, -100.0..=100.0)).changed() {
                                            changed_pitch = true;
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Soprosidade:");
                                        if ui.add(egui::Slider::new(&mut breathiness, 0.0..=100.0)).changed() {
                                            changed_breath = true;
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Vel. Consoante:");
                                        if ui.add(egui::Slider::new(&mut consonant_velocity, 0.0..=200.0).suffix(" %")).changed() {
                                            changed_timing = true;
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("VOL/ATK/DEC:");
                                        changed_amplitude |= ui.add(egui::DragValue::new(&mut volume).range(0.0..=200.0).prefix("V:")).changed();
                                        changed_amplitude |= ui.add(egui::DragValue::new(&mut attack).range(0.0..=200.0).prefix(" A:")).changed();
                                        changed_amplitude |= ui.add(egui::DragValue::new(&mut decay).range(0.0..=100.0).prefix(" D:")).changed();
                                    });
                                });

                            ui.add_space(8.0);

                            Frame::none()
                                .fill(Color32::from_rgb(26, 20, 38))
                                .rounding(Rounding::same(4.0))
                                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(160, 55, 105)))
                                .inner_margin(egui::Margin::same(8.0))
                                .show(ui, |ui| {
                                    ui.label(RichText::new("Fade e Crossfade da Nota").strong().size(11.0).color(Color32::from_rgb(255, 115, 185)));
                                    ui.label(RichText::new("O crossfade cruza a saída anterior com a entrada desta nota.").size(9.5).color(MelodyneTheme::TEXT_MUTED));
                                    ui.separator();
                                    changed_fades |= ui.add(egui::Slider::new(&mut fade_in_ms, 0.0..=500.0).text("Fade-in").suffix(" ms")).changed();
                                    changed_fades |= ui.add(egui::Slider::new(&mut fade_out_ms, 0.0..=500.0).text("Fade-out").suffix(" ms")).changed();
                                    changed_fades |= ui.add(egui::Slider::new(&mut note_crossfade_ms, 0.0..=500.0).text("Crossfade").suffix(" ms")).changed();
                                    ui.horizontal(|ui| {
                                        if ui.small_button("Suave 60 ms").clicked() {
                                            fade_in_ms = 60.0;
                                            fade_out_ms = 60.0;
                                            note_crossfade_ms = 60.0;
                                            changed_fades = true;
                                        }
                                        if ui.small_button("Seco 5 ms").clicked() {
                                            fade_in_ms = 5.0;
                                            fade_out_ms = 5.0;
                                            note_crossfade_ms = 5.0;
                                            changed_fades = true;
                                        }
                                        if ui.small_button("Automático").clicked() {
                                            note_crossfade_ms = 0.0;
                                            changed_fades = true;
                                        }
                                    });
                                });

                            ui.add_space(8.0);

                            Frame::none()
                                .fill(Color32::from_rgb(26, 20, 38))
                                .rounding(Rounding::same(4.0))
                                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(61, 46, 84)))
                                .inner_margin(egui::Margin::same(8.0))
                                .show(ui, |ui| {
                                    ui.label(RichText::new("Portamento").strong().size(11.0).color(Color32::from_rgb(0, 255, 157)));
                                    ui.separator();
                                    changed_portamento |= ui.checkbox(&mut snap_first, "Ligar à nota anterior").changed();
                                    changed_portamento |= ui.add(egui::Slider::new(&mut portamento_length, 1.0..=500.0).text("Comprimento").suffix(" ms")).changed();
                                    changed_portamento |= ui.add(egui::Slider::new(&mut portamento_start, -500.0..=500.0).text("Início").suffix(" ms")).changed();
                                    egui::ComboBox::from_label("Formato da curva")
                                        .selected_text(&portamento_shape)
                                        .show_ui(ui, |ui| {
                                            for (value, label) in [("io", "S suave"), ("l", "Linear"), ("i", "Entrada"), ("o", "Saída")] {
                                                if ui.selectable_value(&mut portamento_shape, value.to_string(), label).changed() {
                                                    changed_portamento = true;
                                                }
                                            }
                                        });
                                });

                            ui.add_space(8.0);

                            Frame::none()
                                .fill(Color32::from_rgb(26, 20, 38))
                                .rounding(Rounding::same(4.0))
                                .stroke(Stroke::new(1.0_f32, Color32::from_rgb(61, 46, 84)))
                                .inner_margin(egui::Margin::same(8.0))
                                .show(ui, |ui| {
                                    ui.label(RichText::new("Vibrato OpenUtau").strong().size(11.0).color(Color32::from_rgb(0, 255, 157)));
                                    ui.separator();
                                    changed_vibrato |= ui.add(egui::Slider::new(&mut vibrato.length_pct, 0.0..=100.0).text("Comprimento").suffix(" %")).changed();
                                    changed_vibrato |= ui.add(egui::Slider::new(&mut vibrato.period_ms, 5.0..=500.0).text("Período").suffix(" ms")).changed();
                                    changed_vibrato |= ui.add(egui::Slider::new(&mut vibrato.depth_cents, 0.0..=200.0).text("Profundidade").suffix(" c")).changed();
                                    ui.horizontal(|ui| {
                                        ui.label("Fade in/out:");
                                        changed_vibrato |= ui.add(egui::DragValue::new(&mut vibrato.fade_in_pct).range(0.0..=100.0).suffix("%")).changed();
                                        changed_vibrato |= ui.add(egui::DragValue::new(&mut vibrato.fade_out_pct).range(0.0..=100.0).suffix("%")).changed();
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Fase/Drift/Link:");
                                        changed_vibrato |= ui.add(egui::DragValue::new(&mut vibrato.shift_pct).range(0.0..=100.0)).changed();
                                        changed_vibrato |= ui.add(egui::DragValue::new(&mut vibrato.drift_pct).range(-100.0..=100.0)).changed();
                                        changed_vibrato |= ui.add(egui::DragValue::new(&mut vibrato.volume_link_pct).range(-100.0..=100.0)).changed();
                                    });
                                });

                            let update_targets: Vec<usize> = if selected_indices.is_empty() {
                                vec![target_idx]
                            } else {
                                selected_indices.iter().copied().collect()
                            };

                            for idx in update_targets {
                                if idx < notes.len() {
                                    if changed_lyric { notes[idx].lyric = lyric.clone(); }
                                    if changed_dur { notes[idx].duration_ms = dur_ms; }
                                    if changed_gender { notes[idx].expressions.gender = gender; }
                                    if changed_dynamics { notes[idx].expressions.dynamics = dynamics; }
                                    if changed_pitch { notes[idx].expressions.pitch_delta = pitch_delta; }
                                    if changed_breath { notes[idx].expressions.breathiness = breathiness; }
                                    if changed_timing { notes[idx].expressions.consonant_velocity = consonant_velocity; }
                                    if changed_amplitude {
                                        notes[idx].expressions.volume = volume;
                                        notes[idx].expressions.attack = attack;
                                        notes[idx].expressions.decay = decay;
                                    }
                                    if changed_fades {
                                        notes[idx].envelope.p2 = fade_in_ms;
                                        notes[idx].envelope.p5 = fade_out_ms;
                                        notes[idx].envelope.crossfade_ms = note_crossfade_ms;
                                    }
                                    if changed_vibrato { notes[idx].vibrato = vibrato.clone(); }
                                    if changed_portamento {
                                        notes[idx].pitch_bend.snap_first = snap_first;
                                        notes[idx].pitch_bend.portamento_start_ms = portamento_start;
                                        notes[idx].pitch_bend.portamento_length_ms = portamento_length;
                                        notes[idx].pitch_bend.portamento_shape = portamento_shape.clone();
                                        if notes[idx].pitch_bend.points.len() >= 2 {
                                            notes[idx].pitch_bend.points[0].time_offset_ms = portamento_start;
                                            notes[idx].pitch_bend.points[0].shape = portamento_shape.clone();
                                            notes[idx].pitch_bend.points[1].time_offset_ms = portamento_start + portamento_length;
                                            notes[idx].pitch_bend.points.sort_by(|left, right| {
                                                left.time_offset_ms.partial_cmp(&right.time_offset_ms).unwrap_or(std::cmp::Ordering::Equal)
                                            });
                                        } else {
                                            notes[idx].pitch_bend.points.clear();
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        ui.vertical_centered(|ui| {
                            ui.add_space(20.0);
                            ui.label(RichText::new("Nenhuma Nota Selecionada").italics().color(MelodyneTheme::TEXT_MUTED));
                            ui.label(RichText::new("Clique em uma nota para editar.").size(10.0).color(MelodyneTheme::TEXT_MUTED));
                        });
                    }
                });
            }

            RightSidebarTab::Phonemes => {
                draw_phoneme_palette(
                    ui,
                    voicebank,
                    phoneme_state,
                    on_preview_phoneme,
                    on_insert_phoneme,
                    on_edit_phoneme,
                );
            }

            RightSidebarTab::Engine => {
                egui::ScrollArea::vertical().id_salt("right_panel_settings_scroll").show(ui, |ui| {
                    ui.label(RichText::new("Motor Resampler").strong().color(Color32::from_rgb(0, 255, 157)));

                    ui.horizontal(|ui| {
                        if ui.radio_value(selected_resampler, "straycat-rs (UtaUtaUtau) [Padrão Recomendado]".to_string(), "straycat-rs (UtaUtaUtau) [Padrão Recomendado]").clicked() {
                            let profile = crate::drivers::KnownResampler::StraycatRs;
                            *custom_resampler_path = Some(
                                profile.find_executable().unwrap_or_else(|| profile.default_path()),
                            );
                        }
                        if crate::drivers::KnownResampler::StraycatRs.default_path().is_file() {
                            ui.label(RichText::new("instalado").size(9.0).color(Color32::from_rgb(0, 255, 157)));
                        }
                    });

                    ui.horizontal(|ui| {
                        if ui.radio_value(selected_resampler, "Nativo (TD-PSOLA)".to_string(), "Nativo (TD-PSOLA)").clicked() {
                            *custom_resampler_path = None;
                        }
                        ui.label(RichText::new("incluso").size(9.0).color(Color32::from_rgb(0, 255, 157)));
                    });

                    ui.label(RichText::new("Modos SOLA / Stretch").size(10.5).color(Color32::from_rgb(255, 190, 80)));
                    for (value, label) in [
                        ("Nativo (SOLA Stretch)", "SOLA Stretch — natural"),
                        ("Nativo (SOLA Loop)", "Loop — sustentação estável"),
                        ("Nativo (SOLA Spline)", "Spline — transição suave"),
                        ("Nativo (SOLA Híbrido)", "SOLA Híbrido — notas longas"),
                    ] {
                        ui.horizontal(|ui| {
                            if ui.radio_value(selected_resampler, value.to_string(), label).clicked() {
                                *custom_resampler_path = None;
                            }
                            ui.label(RichText::new("incluso").size(9.0).color(Color32::from_rgb(180, 180, 180)));
                        });
                    }

                    ui.add_space(6.0);
                    ui.label(RichText::new("Outros Resamplers").size(11.0).color(Color32::from_rgb(180, 170, 200)));

                    for profile in crate::drivers::KnownResampler::ALL {
                        if profile == crate::drivers::KnownResampler::StraycatRs {
                            continue;
                        }
                        ui.horizontal(|ui| {
                            if ui.radio_value(selected_resampler, profile.label().to_string(), profile.label()).clicked() {
                                *custom_resampler_path = Some(
                                    profile.find_executable().unwrap_or_else(|| profile.default_path()),
                                );
                            }
                            if profile.find_executable().is_some() || profile.default_path().is_file() {
                                ui.label(RichText::new("encontrado").size(9.0).color(Color32::from_rgb(0, 255, 157)));
                            }
                        });
                    }

                    if !selected_resampler.contains("Nativo") && !selected_resampler.contains("Native") {
                        let selected_profile = crate::drivers::KnownResampler::from_label(selected_resampler);
                        let resolved_path = custom_resampler_path
                            .clone()
                            .filter(|path| path.is_file())
                            .or_else(|| {
                                selected_profile.map(|profile| profile.default_path()).filter(|path| path.is_file())
                            });
                        if let Some(ref path) = resolved_path {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Executável:").size(10.5).color(Color32::from_rgb(0, 255, 157)));
                                ui.label(RichText::new(path.to_string_lossy().to_string()).size(10.0).monospace().color(Color32::from_rgb(200, 190, 220)));
                            });
                        } else {
                            ui.label(RichText::new("Executável não encontrado.").size(10.0).italics().color(Color32::from_rgb(255, 200, 100)));
                        }

                        ui.horizontal(|ui| {
                            if ui.button(RichText::new("Procurar Resampler...").size(10.5)).clicked() {
                                if let Some(file) = crate::dialogs::FileDialog::new()
                                    .set_title("Selecionar executável de resampler UTAU")
                                    .pick_file()
                                {
                                    *custom_resampler_path = Some(file);
                                    *selected_resampler = "Personalizado (UTAU CLI)".to_string();
                                }
                            }
                        });
                    }

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);

                    ui.label(RichText::new("Motor Wavtool").strong().color(Color32::from_rgb(0, 255, 157)));

                    ui.horizontal(|ui| {
                        if ui.radio_value(selected_wavtool, "Native Rust (Crossfader)".to_string(), "Nativo em Rust (Crossfader) [Recomendado]").clicked() {
                            *custom_wavtool_path = None;
                        }
                        ui.label(RichText::new("incluso").size(9.0).color(Color32::from_rgb(180, 180, 180)));
                    });

                    ui.add_space(4.0);
                    ui.label(RichText::new("Wavtools Externos (UTAU CLI)").size(11.0).color(Color32::from_rgb(180, 170, 200)));

                    for profile in crate::drivers::KnownWavtool::ALL {
                        ui.horizontal(|ui| {
                            if ui.radio_value(selected_wavtool, profile.label().to_string(), profile.label()).clicked() {
                                *custom_wavtool_path = Some(
                                    profile.find_executable().unwrap_or_else(|| profile.default_path()),
                                );
                            }
                            if profile.find_executable().is_some() || profile.default_path().is_file() {
                                ui.label(RichText::new("encontrado").size(9.0).color(Color32::from_rgb(0, 255, 157)));
                            }
                        });
                    }

                    if !selected_wavtool.contains("Native") && !selected_wavtool.contains("Nativo") {
                        let selected_profile = crate::drivers::KnownWavtool::from_label(selected_wavtool);
                        let resolved_path = custom_wavtool_path
                            .clone()
                            .filter(|path| path.is_file())
                            .or_else(|| {
                                selected_profile.map(|profile| profile.default_path()).filter(|path| path.is_file())
                            });
                        if let Some(ref path) = resolved_path {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Executável:").size(10.5).color(Color32::from_rgb(0, 255, 157)));
                                ui.label(RichText::new(path.to_string_lossy().to_string()).size(10.0).monospace().color(Color32::from_rgb(200, 190, 220)));
                            });
                        } else {
                            ui.label(RichText::new("Executável não encontrado.").size(10.0).italics().color(Color32::from_rgb(255, 200, 100)));
                        }

                        ui.horizontal(|ui| {
                            if ui.button(RichText::new("Procurar Wavtool...").size(10.5)).clicked() {
                                if let Some(file) = crate::dialogs::FileDialog::new()
                                    .set_title("Selecionar executável de wavtool UTAU")
                                    .pick_file()
                                {
                                    *custom_wavtool_path = Some(file);
                                    *selected_wavtool = "Personalizado (UTAU CLI)".to_string();
                                }
                            }
                        });
                    }

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);

                    ui.label(RichText::new("Saída de Áudio").strong().color(Color32::from_rgb(0, 255, 157)));
                    ui.horizontal(|ui| {
                        ui.label("Taxa de Amostragem:");
                        for rate in [44100, 48000] {
                            let is_selected = *sample_rate == rate;
                            let (bg_color, text_color, stroke) = if is_selected {
                                (Color32::from_rgb(60, 42, 90), Color32::from_rgb(0, 255, 157), Stroke::new(1.5_f32, Color32::from_rgb(0, 255, 157)))
                            } else {
                                (Color32::from_rgb(32, 24, 46), Color32::from_rgb(200, 190, 220), Stroke::new(1.0_f32, Color32::from_rgb(50, 40, 70)))
                            };
                            let btn = egui::Button::new(RichText::new(format!("{} Hz", rate)).size(11.0).color(text_color).strong())
                                .fill(bg_color)
                                .stroke(stroke)
                                .rounding(Rounding::same(4.0));

                            if ui.add(btn).clicked() {
                                *sample_rate = rate;
                            }
                        }
                    });

                    ui.horizontal(|ui| {
                        ui.label("Threads:");
                        ui.add(egui::Slider::new(render_threads, 1..=16));
                    });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);

                    ui.label(RichText::new("Integração Discord").strong().color(Color32::from_rgb(0, 255, 157)));
                    ui.checkbox(discord_rpc_enabled, RichText::new("Ativar Discord Rich Presence").size(11.0).color(Color32::from_rgb(220, 210, 240)));
                });
            }
        }
    });
}
