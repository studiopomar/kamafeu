use super::*;

pub(super) fn draw(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    lang: crate::config::AppLanguage,
    voicebank: Option<&Voicebank>,
    recent_voicebanks: &[PathBuf],
    singers_list: &[crate::oto::SingerInfo],
    singer_search_query: &mut String,
    singers_paths: &mut Vec<PathBuf>,
    vocal_mode_params: &mut VocalModeParams,
    on_load_vb: &mut dyn FnMut(Option<PathBuf>),
    on_add_singers_dir: &mut dyn FnMut(),
    on_reload_singers: &mut dyn FnMut(),
    on_open_gallery: &mut dyn FnMut(),
) {
    let vb_name = voicebank
        .map(|v| v.name.as_str())
        .unwrap_or_else(|| lang.tr("Cantor Padrão", "Default Singer"));
    let vb_author = voicebank
        .map(|v| v.author.as_str())
        .unwrap_or("UTAU Voicebank");
    let initial_letter = vb_name
        .chars()
        .next()
        .unwrap_or('V')
        .to_uppercase()
        .to_string();

    egui::ScrollArea::vertical()
        .id_salt("right_panel_singer_scroll")
        .show(ui, |ui| {
            // --- CARD DO CANTOR ATIVO ---
            Frame::none()
                .fill(theme.card_bg_c32())
                .rounding(theme.ui_rounding())
                .stroke(theme.card_stroke())
                .inner_margin(egui::Margin::same(10.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let (avatar_rect, _) =
                            ui.allocate_exact_size(Vec2::new(56.0, 56.0), egui::Sense::hover());
                        let painter = ui.painter_at(avatar_rect);

                        painter.rect_filled(
                            avatar_rect,
                            Rounding::same(6.0),
                            theme.bg_header_c32(),
                        );
                        painter.rect_stroke(
                            avatar_rect,
                            Rounding::same(6.0),
                            Stroke::new(1.2_f32, theme.accent_c32()),
                        );

                        let mut loaded_image = false;
                        if let Some(vb) = voicebank {
                            if let Some(ref img_path) = vb.image_path {
                                if let Some(texture) =
                                    crate::gui::image_cache::texture_for_path(ui.ctx(), img_path)
                                {
                                    let [img_w, img_h] = texture.size();
                                    let img_w = img_w as f32;
                                    let img_h = img_h as f32;
                                    if img_w > 0.0 && img_h > 0.0 {
                                        let aspect = img_w / img_h;
                                        let (draw_w, draw_h) = if aspect >= 1.0 {
                                            (54.0, (54.0 / aspect).min(54.0))
                                        } else {
                                            ((54.0 * aspect).min(54.0), 54.0)
                                        };

                                        let draw_rect = Rect::from_center_size(
                                            avatar_rect.center(),
                                            Vec2::new(draw_w, draw_h),
                                        );
                                        let uv = egui::Rect::from_min_max(
                                            Pos2::new(0.0, 0.0),
                                            Pos2::new(1.0, 1.0),
                                        );
                                        painter.image(texture.id(), draw_rect, uv, Color32::WHITE);
                                        loaded_image = true;
                                    }
                                }
                            }
                        }

                        if !loaded_image {
                            let center = avatar_rect.center();
                            painter.circle_filled(
                                center,
                                16.0,
                                theme.accent_c32().linear_multiply(0.4),
                            );
                            painter.text(
                                center,
                                egui::Align2::CENTER_CENTER,
                                &initial_letter,
                                egui::FontId::proportional(16.0),
                                theme.accent_c32(),
                            );
                        }

                        ui.add_space(6.0);
                        ui.vertical(|ui| {
                            ui.label(
                                RichText::new(vb_name)
                                    .strong()
                                    .size(13.0)
                                    .color(theme.text_primary_c32()),
                            );
                            ui.label(
                                RichText::new(vb_author)
                                    .size(10.0)
                                    .color(theme.text_muted_c32()),
                            );
                            if let Some(vb) = voicebank {
                                ui.label(
                                    RichText::new(format!(
                                        "{} {}",
                                        vb.entries.len(),
                                        lang.tr("amostras", "samples")
                                    ))
                                    .size(9.0)
                                    .color(theme.text_muted_c32()),
                                );
                            }
                        });
                    });

                    ui.add_space(6.0);
                    ui.columns(2, |cols| {
                        if cols[0]
                            .add_sized(
                                Vec2::new(cols[0].available_width(), 22.0),
                                egui::Button::new(
                                    RichText::new(lang.tr("Galeria", "Gallery"))
                                        .size(10.5)
                                        .color(Color32::from_rgb(0, 220, 255)),
                                ),
                            )
                            .on_hover_text(lang.tr(
                                "Abrir Galeria Visual de Cantores",
                                "Open Visual Singers Gallery",
                            ))
                            .clicked()
                        {
                            on_open_gallery();
                        }
                        if cols[1]
                            .add_sized(
                                Vec2::new(cols[1].available_width(), 22.0),
                                egui::Button::new(
                                    RichText::new(lang.tr("Carregar...", "Load..."))
                                        .size(10.5)
                                        .color(theme.text_primary_c32()),
                                ),
                            )
                            .on_hover_text(lang.tr(
                                "Carregar banco de voz a partir de pasta",
                                "Load voicebank from folder",
                            ))
                            .clicked()
                        {
                            on_load_vb(None);
                        }
                    });

                    if let Some(vb) = voicebank {
                        if !vb.character_info.is_empty() || !vb.readme_info.is_empty() {
                            ui.add_space(4.0);
                            egui::CollapsingHeader::new(
                                RichText::new(lang.tr("Detalhes & Readme", "Details & Readme"))
                                    .size(10.0)
                                    .color(theme.text_muted_c32()),
                            )
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical()
                                    .id_salt("right_panel_vb_desc_scroll")
                                    .max_height(90.0)
                                    .show(ui, |ui| {
                                        if !vb.character_info.is_empty() {
                                            ui.label(
                                                RichText::new(&vb.character_info)
                                                    .size(9.5)
                                                    .color(theme.accent_c32()),
                                            );
                                        }
                                        if !vb.readme_info.is_empty() {
                                            ui.separator();
                                            ui.label(
                                                RichText::new(&vb.readme_info)
                                                    .size(9.0)
                                                    .color(theme.text_muted_c32()),
                                            );
                                        }
                                    });
                            });
                        }
                    }
                    if !recent_voicebanks.is_empty() {
                        ui.add_space(4.0);
                        egui::CollapsingHeader::new(
                            RichText::new(lang.tr("Recentes", "Recent"))
                                .size(9.5)
                                .color(theme.text_muted_c32()),
                        )
                        .show(ui, |ui| {
                            for recent_p in recent_voicebanks.iter().take(5) {
                                let name = recent_p
                                    .file_name()
                                    .map(|n| n.to_string_lossy().to_string())
                                    .unwrap_or_else(|| recent_p.display().to_string());
                                if ui
                                    .button(RichText::new(format!("• {}", name)).size(9.0))
                                    .on_hover_text(recent_p.display().to_string())
                                    .clicked()
                                {
                                    on_load_vb(Some(recent_p.clone()));
                                }
                            }
                        });
                    }
                });

            ui.add_space(8.0);

            // --- LISTA RÁPIDA DE CANTORES ---
            Frame::none()
                .fill(theme.card_bg_c32())
                .rounding(theme.ui_rounding())
                .stroke(theme.card_stroke())
                .inner_margin(egui::Margin::same(8.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} ({})",
                                lang.tr("Biblioteca", "Library"),
                                singers_list.len()
                            ))
                            .strong()
                            .size(11.5)
                            .color(theme.accent_c32()),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .small_button(lang.tr("Recarregar", "Reload"))
                                .on_hover_text(lang.tr(
                                    "Recarregar lista de cantores",
                                    "Reload singers list",
                                ))
                                .clicked()
                            {
                                on_reload_singers();
                            }
                        });
                    });

                    ui.add_space(3.0);
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(singer_search_query)
                                .hint_text(lang.tr("Buscar cantor...", "Search singer..."))
                                .desired_width(ui.available_width()),
                        );
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

                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .id_salt("singers_quick_list_scroll")
                        .max_height(130.0)
                        .show(ui, |ui| {
                            if filtered_singers.is_empty() {
                                ui.label(
                                    RichText::new(lang.tr(
                                        "Nenhum cantor encontrado.",
                                        "No singers found.",
                                    ))
                                    .size(9.5)
                                    .italics()
                                    .color(theme.text_muted_c32()),
                                );
                            } else {
                                for singer in filtered_singers {
                                    let is_current =
                                        voicebank.is_some_and(|v| v.root_path == singer.path);
                                    let item_bg = if is_current {
                                        theme.accent_c32().linear_multiply(0.2)
                                    } else {
                                        theme.bg_header_c32()
                                    };

                                    Frame::none()
                                        .fill(item_bg)
                                        .rounding(Rounding::same(4.0))
                                        .stroke(if is_current {
                                            theme.active_border_stroke()
                                        } else {
                                            Stroke::NONE
                                        })
                                        .inner_margin(egui::Margin::symmetric(6.0, 4.0))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                let initial =
                                                    singer.name.chars().next().unwrap_or('V');
                                                ui.label(
                                                    RichText::new(format!("[{}]", initial))
                                                        .size(9.0)
                                                        .monospace()
                                                        .color(theme.accent_c32()),
                                                );
                                                ui.vertical(|ui| {
                                                    ui.label(
                                                        RichText::new(&singer.name)
                                                            .strong()
                                                            .size(10.5)
                                                            .color(if is_current {
                                                                theme.accent_c32()
                                                            } else {
                                                                theme.text_primary_c32()
                                                            }),
                                                    );
                                                    if !singer.author.is_empty() {
                                                        ui.label(
                                                            RichText::new(&singer.author)
                                                                .size(8.5)
                                                                .color(theme.text_muted_c32()),
                                                        );
                                                    }
                                                });

                                                ui.with_layout(
                                                    egui::Layout::right_to_left(
                                                        egui::Align::Center,
                                                    ),
                                                    |ui| {
                                                        if !is_current {
                                                            if ui.small_button(lang.tr("Usar", "Use")).clicked() {
                                                                on_load_vb(Some(
                                                                    singer.path.clone(),
                                                                ));
                                                            }
                                                        } else {
                                                            ui.label(
                                                                RichText::new("[OK]")
                                                                    .strong()
                                                                    .size(11.0)
                                                                    .color(theme.accent_c32()),
                                                            );
                                                        }
                                                    },
                                                );
                                            });
                                        });
                                    ui.add_space(2.0);
                                }
                            }
                        });

                    ui.add_space(4.0);
                    egui::CollapsingHeader::new(
                        RichText::new(lang.tr("Pastas de Cantores", "Singers Folders"))
                            .size(9.5)
                            .color(theme.text_muted_c32()),
                    )
                    .show(ui, |ui| {
                        if ui
                            .button(RichText::new(lang.tr("+ Adicionar Pasta...", "+ Add Folder...")).size(9.5))
                            .clicked()
                        {
                            on_add_singers_dir();
                        }
                        let mut to_remove = None;
                        for (p_idx, p) in singers_paths.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.label(
                                    RichText::new(format!("• {}", p.display()))
                                        .size(8.5)
                                        .color(theme.text_muted_c32()),
                                );
                                if ui.small_button("X").clicked() {
                                    to_remove = Some(p_idx);
                                }
                            });
                        }
                        if let Some(idx) = to_remove {
                            singers_paths.remove(idx);
                            on_reload_singers();
                        }
                    });
                });

            ui.add_space(8.0);

            // --- MODO VOCAL & EXPRESSÃO GLOBAL ---
            Frame::none()
                .fill(theme.card_bg_c32())
                .rounding(theme.ui_rounding())
                .stroke(theme.card_stroke())
                .inner_margin(egui::Margin::same(8.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(lang.tr("Modo Vocal & Timbre", "Vocal Mode & Timbre"))
                            .strong()
                            .size(11.5)
                            .color(theme.accent_c32()),
                    );
                    ui.separator();

                    ui.label(
                        RichText::new(lang.tr("Fonetizador:", "Phonemizer:"))
                            .size(10.0)
                            .color(theme.text_muted_c32()),
                    );
                    egui::ComboBox::from_id_salt("phonemizer_mode_cb_unified")
                        .selected_text(match vocal_mode_params.phonemizer_mode {
                            crate::phonemizer::PhonemizerMode::None => {
                                lang.tr("Sem Fonemizador (Manual)", "No Phonemizer (Manual)")
                            }
                            crate::phonemizer::PhonemizerMode::BasicCV => "JA: Basic CV",
                            crate::phonemizer::PhonemizerMode::VCV => "JA: Japanese VCV",
                            crate::phonemizer::PhonemizerMode::CVVC => "JA: Japanese CVVC",
                            crate::phonemizer::PhonemizerMode::EnglishArpasing => {
                                "EN: English Arpasing"
                            }
                            crate::phonemizer::PhonemizerMode::EnglishVCCV => "EN: English VCCV",
                            crate::phonemizer::PhonemizerMode::EnglishG2P => {
                                lang.tr("EN: English G2P (Palavras)", "EN: English G2P (Words)")
                            }
                            crate::phonemizer::PhonemizerMode::PortugueseBrapaVCCV => {
                                "PT: VCCV BRAPA (xiao / 3.7)"
                            }
                            crate::phonemizer::PhonemizerMode::PortugueseBrapaCVC => {
                                "PT: BRAPA CVC"
                            }
                            crate::phonemizer::PhonemizerMode::PortugueseCVVC => {
                                "PT: Portuguese CVVC"
                            }
                            crate::phonemizer::PhonemizerMode::PortugueseVCV => {
                                "PT: Portuguese VCV"
                            }
                            crate::phonemizer::PhonemizerMode::PortugueseG2P => {
                                lang.tr("PT: Português G2P", "PT: Portuguese G2P")
                            }
                        })
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut vocal_mode_params.phonemizer_mode,
                                crate::phonemizer::PhonemizerMode::None,
                                lang.tr("• Sem Fonemizador (Manual)", "• No Phonemizer (Manual)"),
                            );
                            ui.separator();
                            ui.label(
                                RichText::new(lang.tr("[JA] Japonês", "[JA] Japanese"))
                                    .strong()
                                    .color(theme.accent_c32()),
                            );
                            ui.selectable_value(
                                &mut vocal_mode_params.phonemizer_mode,
                                crate::phonemizer::PhonemizerMode::BasicCV,
                                "  JA: Basic CV",
                            );
                            ui.selectable_value(
                                &mut vocal_mode_params.phonemizer_mode,
                                crate::phonemizer::PhonemizerMode::VCV,
                                "  JA: Japanese VCV",
                            );
                            ui.selectable_value(
                                &mut vocal_mode_params.phonemizer_mode,
                                crate::phonemizer::PhonemizerMode::CVVC,
                                "  JA: Japanese CVVC",
                            );
                            ui.separator();
                            ui.label(
                                RichText::new(lang.tr("[PT] Português", "[PT] Portuguese"))
                                    .strong()
                                    .color(theme.accent_c32()),
                            );
                            ui.selectable_value(
                                &mut vocal_mode_params.phonemizer_mode,
                                crate::phonemizer::PhonemizerMode::PortugueseBrapaVCCV,
                                "  PT: VCCV BRAPA (xiao / 3.7)",
                            );
                            ui.selectable_value(
                                &mut vocal_mode_params.phonemizer_mode,
                                crate::phonemizer::PhonemizerMode::PortugueseG2P,
                                lang.tr(
                                    "  PT: Português G2P (Palavras -> Fonemas)",
                                    "  PT: Portuguese G2P (Words -> Phonemes)",
                                ),
                            );
                            ui.selectable_value(
                                &mut vocal_mode_params.phonemizer_mode,
                                crate::phonemizer::PhonemizerMode::PortugueseBrapaCVC,
                                "  PT: BRAPA CVC",
                            );
                            ui.selectable_value(
                                &mut vocal_mode_params.phonemizer_mode,
                                crate::phonemizer::PhonemizerMode::PortugueseCVVC,
                                "  PT: Portuguese CVVC",
                            );
                            ui.selectable_value(
                                &mut vocal_mode_params.phonemizer_mode,
                                crate::phonemizer::PhonemizerMode::PortugueseVCV,
                                "  PT: Portuguese VCV",
                            );
                            ui.separator();
                            ui.label(
                                RichText::new(lang.tr("[EN] Inglês", "[EN] English"))
                                    .strong()
                                    .color(theme.accent_c32()),
                            );
                            ui.selectable_value(
                                &mut vocal_mode_params.phonemizer_mode,
                                crate::phonemizer::PhonemizerMode::EnglishG2P,
                                "  EN: English G2P",
                            );
                            ui.selectable_value(
                                &mut vocal_mode_params.phonemizer_mode,
                                crate::phonemizer::PhonemizerMode::EnglishArpasing,
                                "  EN: English Arpasing",
                            );
                            ui.selectable_value(
                                &mut vocal_mode_params.phonemizer_mode,
                                crate::phonemizer::PhonemizerMode::EnglishVCCV,
                                "  EN: English VCCV",
                            );
                        });

                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(lang.tr("Tensão Vocal:", "Vocal Tension:"))
                            .size(10.0)
                            .color(theme.text_muted_c32()),
                    );
                    let tens_slider = ui.add_sized(
                        Vec2::new(ui.available_width(), 18.0),
                        egui::Slider::new(&mut vocal_mode_params.tension, 0.0..=100.0).suffix(" %"),
                    );
                    if tens_slider.clicked_by(egui::PointerButton::Secondary) {
                        vocal_mode_params.tension = 50.0;
                    }

                    ui.label(
                        RichText::new(lang.tr("Soprosidade (Breath):", "Breathiness:"))
                            .size(10.0)
                            .color(theme.text_muted_c32()),
                    );
                    let breath_slider = ui.add_sized(
                        Vec2::new(ui.available_width(), 18.0),
                        egui::Slider::new(&mut vocal_mode_params.breathiness, 0.0..=100.0)
                            .suffix(" %"),
                    );
                    if breath_slider.clicked_by(egui::PointerButton::Secondary) {
                        vocal_mode_params.breathiness = 0.0;
                    }

                    ui.label(
                        RichText::new(lang.tr("Formante / Gênero (GEN):", "Formant / Gender (GEN):"))
                            .size(10.0)
                            .color(theme.text_muted_c32()),
                    );
                    let gen_slider = ui.add_sized(
                        Vec2::new(ui.available_width(), 18.0),
                        egui::Slider::new(&mut vocal_mode_params.gender, -100.0..=100.0)
                            .suffix(" %"),
                    );
                    if gen_slider.clicked_by(egui::PointerButton::Secondary) {
                        vocal_mode_params.gender = 0.0;
                    }

                    ui.label(
                        RichText::new(lang.tr("Ganho de Saída:", "Output Gain:"))
                            .size(10.0)
                            .color(theme.text_muted_c32()),
                    );
                    let loud_slider = ui.add_sized(
                        Vec2::new(ui.available_width(), 18.0),
                        egui::Slider::new(&mut vocal_mode_params.loudness, -12.0..=12.0)
                            .suffix(" dB"),
                    );
                    if loud_slider.clicked_by(egui::PointerButton::Secondary) {
                        vocal_mode_params.loudness = 0.0;
                    }

                    ui.label(
                        RichText::new(lang.tr("Crossfade de Consoantes:", "Consonant Crossfade:"))
                            .size(10.0)
                            .color(theme.text_muted_c32()),
                    );
                    let cross_slider = ui.add_sized(
                        Vec2::new(ui.available_width(), 18.0),
                        egui::Slider::new(&mut vocal_mode_params.crossfade_ms, 0.0..=200.0)
                            .suffix(" ms"),
                    );
                    if cross_slider.clicked_by(egui::PointerButton::Secondary) {
                        vocal_mode_params.crossfade_ms = 0.0;
                    }

                    ui.add_space(6.0);
                    ui.label(
                        RichText::new(lang.tr("Predefinições:", "Presets:"))
                            .size(9.5)
                            .color(theme.accent_c32()),
                    );
                    ui.columns(3, |cols| {
                        if cols[0]
                            .add_sized(
                                Vec2::new(cols[0].available_width(), 20.0),
                                egui::Button::new(RichText::new(lang.tr("Orgânico", "Organic")).size(9.5)),
                            )
                            .clicked()
                        {
                            vocal_mode_params.loudness = 0.0;
                            vocal_mode_params.tension = 40.0;
                            vocal_mode_params.breathiness = 10.0;
                            vocal_mode_params.gender = 0.0;
                            vocal_mode_params.crossfade_ms = 50.0;
                        }
                        if cols[1]
                            .add_sized(
                                Vec2::new(cols[1].available_width(), 20.0),
                                egui::Button::new(RichText::new(lang.tr("Pop", "Pop")).size(9.5)),
                            )
                            .clicked()
                        {
                            vocal_mode_params.loudness = 1.5;
                            vocal_mode_params.tension = 70.0;
                            vocal_mode_params.breathiness = 0.0;
                            vocal_mode_params.gender = 0.0;
                            vocal_mode_params.crossfade_ms = 35.0;
                        }
                        if cols[2]
                            .add_sized(
                                Vec2::new(cols[2].available_width(), 20.0),
                                egui::Button::new(RichText::new(lang.tr("Robô", "Robot")).size(9.5)),
                            )
                            .clicked()
                        {
                            vocal_mode_params.loudness = 0.0;
                            vocal_mode_params.tension = 95.0;
                            vocal_mode_params.breathiness = 0.0;
                            vocal_mode_params.gender = 0.0;
                            vocal_mode_params.crossfade_ms = 5.0;
                        }
                    });
                });
        });
}
