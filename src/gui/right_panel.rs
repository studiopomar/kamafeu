use crate::gui::theme::ThemeConfig;
use crate::project::model::UNote;
use eframe::egui::{self, Color32, Frame, Pos2, Rect, RichText, Rounding, Stroke, Vec2};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RightSidebarTab {
    #[default]
    NoteProperties,
    Settings,
}

pub fn draw_right_panel(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    voicebank: Option<&crate::oto::Voicebank>,
    selected_note_idx: Option<usize>,
    notes: &mut [UNote],
    selected_indices: &std::collections::HashSet<usize>,
    active_tab: &mut RightSidebarTab,
    render_threads: &mut u32,
    sample_rate: &mut u32,
    selected_resampler: &mut String,
    selected_wavtool: &mut String,
    custom_resampler_path: &mut Option<PathBuf>,
    custom_wavtool_path: &mut Option<PathBuf>,
) {
    let vb_name = voicebank
        .map(|v| v.name.as_str())
        .unwrap_or("Default Singer");
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
        Frame::none()
            .fill(theme.card_bg_c32())
            .rounding(theme.ui_rounding())
            .stroke(theme.card_stroke())
            .inner_margin(egui::Margin::same(8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Avatar do Cantor:").strong().size(11.0).color(theme.accent_c32()));
                });

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    ui.add_space((ui.available_width() - 100.0).max(0.0) * 0.5);
                    let (avatar_rect, _) = ui.allocate_exact_size(Vec2::new(100.0, 100.0), egui::Sense::hover());
                    let painter = ui.painter_at(avatar_rect);

                    painter.rect_filled(avatar_rect, Rounding::same(6.0), theme.bg_header_c32());
                    painter.rect_stroke(avatar_rect, Rounding::same(6.0), Stroke::new(1.2, theme.accent_c32()));

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
                                        (96.0, (96.0 / aspect).min(96.0))
                                    } else {
                                        ((96.0 * aspect).min(96.0), 96.0)
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
                        painter.circle_filled(Pos2::new(center.x, center.y - 10.0), 20.0, theme.accent_c32());
                        painter.text(
                            Pos2::new(center.x, center.y - 10.0),
                            egui::Align2::CENTER_CENTER,
                            &initial_letter,
                            egui::FontId::proportional(18.0),
                            if theme.is_light() { Color32::WHITE } else { Color32::from_rgb(20, 16, 28) },
                        );

                        painter.text(
                            Pos2::new(center.x, center.y + 18.0),
                            egui::Align2::CENTER_CENTER,
                            vb_name,
                            egui::FontId::proportional(11.0),
                            theme.text_primary_c32(),
                        );
                    }
                });

                ui.add_space(6.0);
                ui.label(RichText::new(format!("Autor: {}", vb_author)).size(10.0).color(theme.text_muted_c32()));

                if let Some(vb) = voicebank {
                    if !vb.character_info.is_empty() || !vb.readme_info.is_empty() {
                        ui.add_space(4.0);
                        egui::CollapsingHeader::new(RichText::new("character.txt / readme.txt").size(10.0).color(theme.text_primary_c32()))
                            .show(ui, |ui| {
                                egui::ScrollArea::vertical().id_salt("right_panel_vb_scroll").max_height(100.0).show(ui, |ui| {
                                    if !vb.character_info.is_empty() {
                                        ui.label(RichText::new(&vb.character_info).size(9.0).color(theme.accent_c32()));
                                    }
                                    if !vb.readme_info.is_empty() {
                                        ui.separator();
                                        ui.label(RichText::new(&vb.readme_info).size(9.0).color(theme.text_muted_c32()));
                                    }
                                });
                            });
                    }
                }
            });

        ui.add_space(8.0);

        ui.horizontal(|ui| {
            let note_tab_color = if *active_tab == RightSidebarTab::NoteProperties {
                theme.accent_c32()
            } else {
                theme.text_muted_c32()
            };
            if ui.selectable_label(*active_tab == RightSidebarTab::NoteProperties, RichText::new("Informações da Nota").color(note_tab_color)).clicked() {
                *active_tab = RightSidebarTab::NoteProperties;
            }

            let settings_tab_color = if *active_tab == RightSidebarTab::Settings {
                theme.accent_c32()
            } else {
                theme.text_muted_c32()
            };
            if ui.selectable_label(*active_tab == RightSidebarTab::Settings, RichText::new("Configurações do Motor").color(settings_tab_color)).clicked() {
                *active_tab = RightSidebarTab::Settings;
            }
        });

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(6.0);

        match active_tab {
            RightSidebarTab::NoteProperties => {
                egui::ScrollArea::vertical().id_salt("right_panel_note_props_scroll").show(ui, |ui| {
                    if let Some(target_idx) = selected_note_idx {
                        if target_idx < notes.len() {
                            if selected_indices.len() > 1 {
                                Frame::none()
                                    .fill(theme.c32_alpha(theme.accent_color, 0.2))
                                    .rounding(theme.ui_rounding())
                                    .stroke(Stroke::new(1.0, theme.accent_c32()))
                                    .inner_margin(egui::Margin::same(6.0))
                                    .show(ui, |ui| {
                                        ui.label(RichText::new(format!("Seleção em Grupo: {} notas selecionadas", selected_indices.len())).strong().size(11.0).color(theme.accent_c32()));
                                        ui.label(RichText::new("As alterações dos sliders aplicam-se a todas as notas selecionadas").size(10.0).color(theme.text_muted_c32()));
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
                            let mut vibrato = notes[target_idx].vibrato.clone();
                            let mut portamento_start = notes[target_idx].pitch_bend.portamento_start_ms;
                            let mut portamento_length = notes[target_idx].pitch_bend.portamento_length_ms;
                            let mut portamento_shape = notes[target_idx].pitch_bend.portamento_shape.clone();
                            let mut snap_first = notes[target_idx].pitch_bend.snap_first;
                            let mut phonemizer_override = notes[target_idx].phonemizer_override.clone();
                            let mut fade_in_ms = notes[target_idx].envelope.p2;
                            let mut fade_out_ms = notes[target_idx].envelope.p5;
                            let mut crossfade_ms = notes[target_idx].envelope.crossfade_ms;

                            let mut changed_lyric = false;
                            let mut changed_dur = false;
                            let mut changed_gender = false;
                            let mut changed_dynamics = false;
                            let mut changed_pitch = false;
                            let mut changed_breath = false;
                            let mut changed_timing = false;
                            let mut changed_amplitude = false;
                            let mut changed_vibrato = false;
                            let mut changed_portamento = false;
                            let mut changed_envelope = false;
                            let mut changed_phonemizer = false;

                            Frame::none()
                                .fill(Color32::from_rgb(36, 27, 53))
                                .rounding(Rounding::same(4.0))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(61, 46, 84)))
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
                                    egui::ComboBox::from_label("Fonemizador desta nota")
                                        .selected_text(phonemizer_override.as_deref().unwrap_or("Global"))
                                        .show_ui(ui, |ui| {
                                            changed_phonemizer |= ui.selectable_value(&mut phonemizer_override, None, "Global").changed();
                                            for mode in crate::phonemizer::PhonemizerMode::ALL {
                                                let name = format!("{mode:?}");
                                                changed_phonemizer |= ui.selectable_value(&mut phonemizer_override, Some(name.clone()), name).changed();
                                            }
                                        });

                                    ui.horizontal(|ui| {
                                        ui.label("Tom / Nota:");
                                        ui.label(RichText::new(&pitch_str).strong().color(Color32::from_rgb(216, 180, 254)));
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Posição Inicial (ms):");
                                        ui.label(format!("{:.1}", pos_ms));
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Duração (ms):");
                                        if ui.add(egui::DragValue::new(&mut dur_ms).range(20.0..=10000.0).speed(5.0)).changed() {
                                            changed_dur = true;
                                        }
                                    });
                                });

                            ui.add_space(8.0);

                            Frame::none()
                                .fill(Color32::from_rgb(26, 20, 38))
                                .rounding(Rounding::same(4.0))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(61, 46, 84)))
                                .inner_margin(egui::Margin::same(8.0))
                                .show(ui, |ui| {
                                    ui.label(RichText::new("Parâmetros Vocais").strong().size(11.0).color(Color32::from_rgb(0, 255, 157)));
                                    ui.separator();

                                    ui.horizontal(|ui| {
                                        ui.label("Fator de Gênero:");
                                        if ui.add(egui::Slider::new(&mut gender, -100.0..=100.0).show_value(true)).changed() {
                                            changed_gender = true;
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Dinâmica (0,1 dB):");
                                        if ui.add(egui::Slider::new(&mut dynamics, -240.0..=120.0).show_value(true)).changed() {
                                            changed_dynamics = true;
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Deslocamento de Pitch (Cents):");
                                        if ui.add(egui::Slider::new(&mut pitch_delta, -100.0..=100.0).show_value(true)).changed() {
                                            changed_pitch = true;
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Soprosidade:");
                                        if ui.add(egui::Slider::new(&mut breathiness, 0.0..=100.0).show_value(true)).changed() {
                                            changed_breath = true;
                                        }
                                    });

                                    ui.horizontal(|ui| {
                                        ui.label("Velocidade da consoante:");
                                        if ui.add(egui::Slider::new(&mut consonant_velocity, -100.0..=200.0).suffix("%")).changed() {
                                            changed_timing = true;
                                        }
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("VOL / ATK / DEC:");
                                        changed_amplitude |= ui.add(egui::DragValue::new(&mut volume).range(0.0..=200.0)).changed();
                                        changed_amplitude |= ui.add(egui::DragValue::new(&mut attack).range(0.0..=200.0)).changed();
                                        changed_amplitude |= ui.add(egui::DragValue::new(&mut decay).range(0.0..=100.0)).changed();
                                    });
                                });

                            ui.add_space(8.0);
                            Frame::none()
                                .fill(Color32::from_rgb(26, 20, 38))
                                .rounding(Rounding::same(4.0))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(61, 46, 84)))
                                .inner_margin(egui::Margin::same(8.0))
                                .show(ui, |ui| {
                                    ui.label(RichText::new("Envelope / Fades").strong().size(11.0).color(Color32::from_rgb(0, 255, 157)));
                                    ui.separator();
                                    changed_envelope |= ui.add(egui::Slider::new(&mut fade_in_ms, 0.0..=500.0).text("Fade In").suffix(" ms")).changed();
                                    changed_envelope |= ui.add(egui::Slider::new(&mut fade_out_ms, 0.0..=500.0).text("Fade Out").suffix(" ms")).changed();
                                    changed_envelope |= ui.add(egui::Slider::new(&mut crossfade_ms, 0.0..=500.0).text("Crossfade").suffix(" ms")).changed();
                                    ui.label(RichText::new("Os valores são aplicados explicitamente à nota e ao render.").size(9.0).color(theme.text_muted_c32()));
                                });

                            ui.add_space(8.0);
                            Frame::none()
                                .fill(Color32::from_rgb(26, 20, 38))
                                .rounding(Rounding::same(4.0))
                                .stroke(Stroke::new(1.0, Color32::from_rgb(61, 46, 84)))
                                .inner_margin(egui::Margin::same(8.0))
                                .show(ui, |ui| {
                                    ui.label(RichText::new("Portamento").strong().size(11.0).color(Color32::from_rgb(0, 255, 157)));
                                    ui.separator();
                                    ui.horizontal(|ui| {
                                        ui.label("Presets:");
                                        for (label, start, length, shape, snap) in [
                                            ("Suave", -55.0, 110.0, "io", true),
                                            ("Natural", -40.0, 80.0, "io", true),
                                            ("Rápido", -20.0, 45.0, "l", true),
                                            ("Deslizante", -100.0, 220.0, "s", true),
                                            ("Sem snap", -25.0, 60.0, "l", false),
                                        ] {
                                            if ui.small_button(label).clicked() {
                                                portamento_start = start;
                                                portamento_length = length;
                                                portamento_shape = shape.to_string();
                                                snap_first = snap;
                                                changed_portamento = true;
                                            }
                                        }
                                    });
                                    changed_portamento |= ui.checkbox(&mut snap_first, "Ligar à nota anterior").changed();
                                    changed_portamento |= ui.add(egui::Slider::new(&mut portamento_length, 1.0..=500.0).text("Comprimento").suffix(" ms")).changed();
                                    changed_portamento |= ui.add(egui::Slider::new(&mut portamento_start, -500.0..=500.0).text("Início").suffix(" ms")).changed();
                                    egui::ComboBox::from_label("Formato da curva")
                                        .selected_text(&portamento_shape)
                                        .show_ui(ui, |ui| {
                                            for (value, label) in [
                                                ("io", "S suave"),
                                                ("l", "Linear"),
                                                ("i", "Entrada"),
                                                ("o", "Saída"),
                                                ("j", "Exponencial"),
                                                ("r", "Logarítmica"),
                                            ] {
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
                                .stroke(Stroke::new(1.0, Color32::from_rgb(61, 46, 84)))
                                .inner_margin(egui::Margin::same(8.0))
                                .show(ui, |ui| {
                                    ui.label(RichText::new("Vibrato OpenUtau").strong().size(11.0).color(Color32::from_rgb(0, 255, 157)));
                                    ui.separator();
                                    changed_vibrato |= ui.add(egui::Slider::new(&mut vibrato.length_pct, 0.0..=100.0).text("Comprimento").suffix("%")) .changed();
                                    changed_vibrato |= ui.add(egui::Slider::new(&mut vibrato.period_ms, 5.0..=500.0).text("Período").suffix(" ms")).changed();
                                    changed_vibrato |= ui.add(egui::Slider::new(&mut vibrato.depth_cents, 0.0..=200.0).text("Profundidade").suffix(" c")).changed();
                                    ui.horizontal(|ui| {
                                        ui.label("Fade in/out:");
                                        changed_vibrato |= ui.add(egui::DragValue::new(&mut vibrato.fade_in_pct).range(0.0..=100.0).suffix("%")).changed();
                                        changed_vibrato |= ui.add(egui::DragValue::new(&mut vibrato.fade_out_pct).range(0.0..=100.0).suffix("%")).changed();
                                    });
                                    ui.horizontal(|ui| {
                                        ui.label("Fase / drift / VOL link:");
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
                                    if changed_envelope {
                                        notes[idx].envelope.p2 = fade_in_ms;
                                        notes[idx].envelope.p5 = fade_out_ms;
                                        notes[idx].envelope.crossfade_ms = crossfade_ms;
                                    }
                                    if changed_phonemizer {
                                        notes[idx].phonemizer_override = phonemizer_override.clone();
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
                            ui.label(RichText::new("Nenhuma Nota Selecionada").italics().color(theme.text_muted_c32()));
                            ui.label(RichText::new("Clique em qualquer nota na grade do Piano Roll para inspecionar e editar suas propriedades.").size(10.0).color(theme.text_muted_c32()));
                        });
                    }
                });
            }
            RightSidebarTab::Settings => {
                egui::ScrollArea::vertical().id_salt("right_panel_settings_scroll").show(ui, |ui| {
                    ui.label(RichText::new("Motor Resampler").strong().color(theme.accent_c32()));

                    for profile in crate::drivers::KnownResampler::ALL {
                        ui.horizontal(|ui| {
                            if ui
                                .radio_value(
                                    selected_resampler,
                                    profile.label().to_string(),
                                    profile.label(),
                                )
                                .clicked()
                            {
                                *custom_resampler_path = Some(
                                    profile
                                        .find_executable()
                                        .unwrap_or_else(|| profile.default_path()),
                                );
                                if profile == crate::drivers::KnownResampler::HifisamplerRs {
                                    let _ = crate::drivers::resampler_driver::ensure_hifisampler_ready();
                                }
                            }

                            if profile == crate::drivers::KnownResampler::HifisamplerRs {
                                if let Ok(ref msg) = crate::drivers::resampler_driver::ensure_hifisampler_ready() {
                                    ui.label(
                                        RichText::new("pronto (ONNX)")
                                            .size(9.0)
                                            .color(theme.note_fill_c32()),
                                    )
                                    .on_hover_text(msg);
                                } else if profile.find_executable().is_some() || profile.default_path().is_file() {
                                    ui.label(
                                        RichText::new("encontrado")
                                            .size(9.0)
                                            .color(theme.note_fill_c32()),
                                    );
                                }
                            } else if profile.find_executable().is_some() || profile.default_path().is_file() {
                                ui.label(
                                    RichText::new("encontrado")
                                        .size(9.0)
                                        .color(theme.note_fill_c32()),
                                );
                            }
                        });
                    }
                    ui.radio_value(
                        selected_resampler,
                        "Nativo em Rust (TD-PSOLA)".to_string(),
                        "Nativo em Rust (TD-PSOLA)",
                    );

                    ui.add_space(4.0);

                    if !selected_resampler.contains("Nativo") {
                        let selected_profile =
                            crate::drivers::KnownResampler::from_label(selected_resampler);
                        let resolved_path = custom_resampler_path
                            .clone()
                            .filter(|path| path.is_file())
                            .or_else(|| {
                                selected_profile
                                    .map(|profile| profile.default_path())
                                    .filter(|path| path.is_file())
                            });
                        if let Some(ref path) = resolved_path {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("Executável:").size(10.5).color(Color32::from_rgb(0, 255, 157)));
                                ui.label(RichText::new(path.to_string_lossy().to_string()).size(10.0).monospace().color(Color32::from_rgb(200, 190, 220)));
                            });

                            let is_exe = path
                                .extension()
                                .and_then(|s| s.to_str())
                                .map(|ext| ext.eq_ignore_ascii_case("exe"))
                                .unwrap_or(false);

                            if is_exe {
                                #[cfg(unix)]
                                {
                                    if let Some(wine_bin) = crate::drivers::process::find_wine_executable() {
                                        let ver = crate::drivers::process::wine_version().unwrap_or_else(|| "Wine".to_string());
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new("[Wine]").size(10.0).strong().color(Color32::from_rgb(0, 255, 180)));
                                            ui.label(RichText::new(format!("{ver} pronto ({})", wine_bin.display())).size(9.5).color(Color32::from_rgb(180, 230, 255)));
                                        });
                                    } else {
                                        ui.label(
                                            RichText::new("[Aviso] Executável .exe selecionado, mas o Wine não foi encontrado.\nInstale o Wine (ex: 'brew install --cask wine-stable' no Mac ou 'sudo apt install wine' no Linux) para usá-lo.")
                                                .size(9.5)
                                                .color(Color32::from_rgb(255, 140, 140))
                                        );
                                    }
                                }
                            }
                        } else {
                            ui.label(RichText::new("Executável não encontrado. O Kamafeu Studio usará o fallback Native TD-PSOLA.").size(10.0).italics().color(Color32::from_rgb(255, 200, 100)));
                        }
                    }

                    #[cfg(unix)]
                    {
                        if let Some(wine_bin) = crate::drivers::process::find_wine_executable() {
                            let ver = crate::drivers::process::wine_version().unwrap_or_else(|| "Wine".to_string());
                            ui.horizontal(|ui| {
                                ui.label(RichText::new("[Wine]").size(9.5).color(Color32::from_rgb(0, 255, 180)));
                                ui.label(RichText::new(format!("Suporte a resamplers .exe ativo ({ver})")).size(9.5).color(Color32::from_rgb(170, 220, 255)))
                                    .on_hover_text(format!("Wine detectado automaticamente em: {}", wine_bin.display()));
                            });
                        }
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

                        if custom_resampler_path.is_some()
                            && ui.button(RichText::new("Restaurar Padrão").size(10.0)).clicked() {
                                if let Some(profile) =
                                    crate::drivers::KnownResampler::from_label(selected_resampler)
                                {
                                    *custom_resampler_path = Some(
                                        profile
                                            .find_executable()
                                            .unwrap_or_else(|| profile.default_path()),
                                    );
                                } else {
                                    *custom_resampler_path = None;
                                }
                            }
                    });

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

                    for profile in crate::drivers::KnownWavtool::ALL {
                        if let Some(exe_path) = profile.find_executable().filter(|p| p.is_file()) {
                            ui.horizontal(|ui| {
                                if ui.radio_value(selected_wavtool, profile.label().to_string(), profile.label()).clicked() {
                                    *custom_wavtool_path = Some(exe_path);
                                }
                                ui.label(
                                    RichText::new("detectado")
                                        .size(9.0)
                                        .color(Color32::from_rgb(0, 255, 157)),
                                );
                            });
                        }
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

                            let is_exe = path
                                .extension()
                                .and_then(|s| s.to_str())
                                .map(|ext| ext.eq_ignore_ascii_case("exe"))
                                .unwrap_or(false);

                            if is_exe {
                                #[cfg(unix)]
                                {
                                    if let Some(wine_bin) = crate::drivers::process::find_wine_executable() {
                                        let ver = crate::drivers::process::wine_version().unwrap_or_else(|| "Wine".to_string());
                                        ui.horizontal(|ui| {
                                            ui.label(RichText::new("[Wine]").size(10.0).strong().color(Color32::from_rgb(0, 255, 180)));
                                            ui.label(RichText::new(format!("{ver} pronto ({})", wine_bin.display())).size(9.5).color(Color32::from_rgb(180, 230, 255)));
                                        });
                                    } else {
                                        ui.label(
                                            RichText::new("[Aviso] Executável .exe selecionado, mas o Wine não foi encontrado.\nInstale o Wine (ex: 'brew install --cask wine-stable' no Mac ou 'sudo apt install wine' no Linux) para usá-lo.")
                                                .size(9.5)
                                                .color(Color32::from_rgb(255, 140, 140))
                                        );
                                    }
                                }
                            }
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
                                (Color32::from_rgb(60, 42, 90), Color32::from_rgb(0, 255, 157), Stroke::new(1.5, Color32::from_rgb(0, 255, 157)))
                            } else {
                                (Color32::from_rgb(32, 24, 46), Color32::from_rgb(200, 190, 220), Stroke::new(1.0, Color32::from_rgb(50, 40, 70)))
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
                        ui.label("Threads de Renderização:");
                        ui.add(egui::Slider::new(render_threads, 1..=16));
                    });
                });
            }
        }
    });
}
