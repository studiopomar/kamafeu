use crate::gui::theme::ThemeConfig;
use crate::project::model::{UTrack, UVoicePart, UWavePart};
use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Stroke, Vec2};

pub fn draw_arrangement_view(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    tracks: &mut Vec<UTrack>,
    parts: &mut Vec<UVoicePart>,
    wave_parts: &mut Vec<UWavePart>,
    active_track_index: &mut usize,
    playhead_ms: &mut f64,
    px_per_ms: f32,
    bpm: f64,
    horizontal_scroll_offset: &mut f32,
    fx_rack_dialog_state: &mut crate::gui::fx_rack_dialog::FxRackDialogState,
    lang: crate::config::AppLanguage,
) -> bool {
    let mut changed = false;
    let header_width = 220.0f32;
    let track_height = 42.0f32;
    let ruler_height = 24.0f32;

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new(lang.tr("Arrangement (Faixas)", "Arrangement (Tracks)"))
                    .strong()
                    .size(12.0)
                    .color(theme.text_primary_c32()),
            );
            ui.add_space(10.0);

            if ui
                .button(
                    egui::RichText::new(lang.tr("Nova Track", "New Track"))
                        .size(10.5)
                        .color(theme.accent_c32()),
                )
                .clicked()
            {
                let new_idx = tracks.len();
                let track_name = format!("Track {}", new_idx + 1);
                tracks.push(UTrack {
                    name: track_name.clone(),
                    singer: lang.tr("Cantor Padrão", "Default Singer").to_string(),
                    volume_db: 0.0,
                    pan: 0.0,
                    mute: false,
                    solo: false,
                    ..UTrack::default()
                });
                parts.push(UVoicePart::new(format!("{} {}", lang.tr("Parte", "Part"), new_idx + 1), new_idx));
                *active_track_index = new_idx;
                changed = true;
            }

            if ui
                .button(
                    egui::RichText::new(lang.tr("Adicionar Áudio...", "Add Audio..."))
                        .size(10.5)
                        .color(Color32::from_rgb(100, 200, 255)),
                )
                .clicked()
            {
                if let Some(path) = crate::dialogs::FileDialog::new()
                    .add_filter(
                        lang.tr("Áudio (*.wav, *.mp3, *.ogg, *.flac)", "Audio (*.wav, *.mp3, *.ogg, *.flac)"),
                        &["wav", "mp3", "ogg", "flac"],
                    )
                    .pick_file()
                {
                    let file_stem = path
                        .file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Audio Track")
                        .to_string();
                    let file_name = path
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("Audio Track")
                        .to_string();
                    let file_path_str = path.to_string_lossy().to_string();

                    let new_idx = tracks.len();
                    tracks.push(UTrack {
                        name: file_stem,
                        singer: lang.tr("Instrumental / Áudio", "Instrumental / Audio").to_string(),
                        volume_db: 0.0,
                        pan: 0.0,
                        mute: false,
                        solo: false,
                        ..UTrack::default()
                    });
                    let wave = UWavePart::new(file_name, file_path_str, new_idx);
                    wave_parts.push(wave);
                    *active_track_index = new_idx;
                    changed = true;
                }
            }

            if tracks.len() > 1
                && ui
                    .button(
                        egui::RichText::new(lang.tr("Excluir Track", "Delete Track"))
                            .size(10.5)
                            .color(Color32::from_rgb(255, 100, 100)),
                    )
                    .clicked()
            {
                let del_idx = *active_track_index;
                if del_idx < tracks.len() {
                    tracks.remove(del_idx);
                    parts.retain(|p| p.track_index != del_idx);
                    wave_parts.retain(|w| w.track_index != del_idx);
                    for p in parts.iter_mut() {
                        if p.track_index > del_idx {
                            p.track_index -= 1;
                        }
                    }
                    for w in wave_parts.iter_mut() {
                        if w.track_index > del_idx {
                            w.track_index -= 1;
                        }
                    }
                    if *active_track_index >= tracks.len() {
                        *active_track_index = tracks.len().saturating_sub(1);
                    }
                    changed = true;
                }
            }

            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(format!(
                    "{}: {} ({})",
                    lang.tr("Track Ativa", "Active Track"),
                    *active_track_index + 1,
                    tracks
                        .get(*active_track_index)
                        .map(|t| t.name.as_str())
                        .unwrap_or("Track")
                ))
                .size(11.0)
                .color(Color32::from_rgb(0, 255, 157)),
            );
        });

        ui.add_space(4.0);

        if *active_track_index >= tracks.len() && !tracks.is_empty() {
            *active_track_index = 0;
        }

        let mut max_audio_end_ms = 0.0f64;
        for part in parts.iter() {
            let part_end = part.position_ms
                + part
                    .notes
                    .iter()
                    .map(|n| n.position_ms + n.duration_ms)
                    .fold(0.0f64, f64::max);
            max_audio_end_ms = max_audio_end_ms.max(part_end);
        }
        for wave in wave_parts.iter() {
            let wave_dur = if wave.duration_ms > 0.0 {
                wave.duration_ms
            } else {
                30_000.0
            };
            max_audio_end_ms = max_audio_end_ms.max(wave.position_ms + wave_dur);
        }
        let total_canvas_ms = (max_audio_end_ms + 30_000.0).max(60_000.0);
        let timeline_width = (total_canvas_ms * px_per_ms as f64) as f32;

        egui::ScrollArea::vertical()
            .id_salt("arrangement_tracks_scroll_v")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    enum HeaderAction {
                        AddVoiceTrack,
                        AddAudioTrack(String, String, String),
                        DeleteTrack(usize),
                        DuplicateTrack(usize),
                    }
                    let mut header_action: Option<HeaderAction> = None;
                    let tracks_count = tracks.len();

                    ui.vertical(|ui| {
                        for (idx, track) in tracks.iter_mut().enumerate() {
                            ui.push_id(("header", idx), |ui| {
                                let is_active = idx == *active_track_index;
                                let track_y_offset = if idx == 0 { ruler_height } else { 0.0 };
                                let card_total_h = track_height + track_y_offset;

                                let (card_rect, card_resp) = ui.allocate_exact_size(
                                    Vec2::new(header_width, card_total_h),
                                    egui::Sense::click(),
                                );

                                if card_resp.clicked() {
                                    *active_track_index = idx;
                                }

                                card_resp.context_menu(|ui| {
                                    *active_track_index = idx;
                                    ui.set_min_width(175.0);

                                    ui.label(
                                        egui::RichText::new(format!("{}: {}", lang.tr("Faixa", "Track"), &track.name))
                                            .strong()
                                            .size(11.5)
                                            .color(theme.text_primary_c32()),
                                    );
                                    ui.separator();

                                    let mute_text = if track.mute { lang.tr("Desmutar Faixa", "Unmute Track") } else { lang.tr("Mutar Faixa", "Mute Track") };
                                    if ui.button(mute_text).clicked() {
                                        track.mute = !track.mute;
                                        changed = true;
                                        ui.close_menu();
                                    }

                                    let solo_text = if track.solo { lang.tr("Desativar Solo", "Unsolo Track") } else { lang.tr("Solo", "Solo") };
                                    if ui.button(solo_text).clicked() {
                                        track.solo = !track.solo;
                                        changed = true;
                                        ui.close_menu();
                                    }

                                    let has_fx_enabled = track.fx_rack.as_ref().map_or(false, |fx| fx.master_enabled);
                                    let fx_menu_text = if has_fx_enabled {
                                        lang.tr("Rack de Efeitos DSP (Ativo)...", "DSP FX Rack (Active)...")
                                    } else {
                                        lang.tr("Rack de Efeitos DSP...", "DSP FX Rack...")
                                    };
                                    if ui.button(fx_menu_text).clicked() {
                                        fx_rack_dialog_state.target_track = Some(idx);
                                        fx_rack_dialog_state.is_open = true;
                                        ui.close_menu();
                                    }

                                    if ui.button(lang.tr("Resetar Volume e Pan", "Reset Volume and Pan")).clicked() {
                                        track.volume_db = 0.0;
                                        track.pan = 0.0;
                                        changed = true;
                                        ui.close_menu();
                                    }

                                    ui.separator();

                                    if ui.button(lang.tr("Duplicar Faixa", "Duplicate Track")).clicked() {
                                        header_action = Some(HeaderAction::DuplicateTrack(idx));
                                        ui.close_menu();
                                    }

                                    if ui.button(lang.tr("Nova Faixa Vocal", "New Vocal Track")).clicked() {
                                        header_action = Some(HeaderAction::AddVoiceTrack);
                                        ui.close_menu();
                                    }

                                    if ui.button(lang.tr("Adicionar Faixa de Áudio...", "Add Audio Track...")).clicked() {
                                        if let Some(path) = crate::dialogs::FileDialog::new()
                                            .add_filter(
                                                lang.tr("Áudio (*.wav, *.mp3, *.ogg, *.flac)", "Audio (*.wav, *.mp3, *.ogg, *.flac)"),
                                                &["wav", "mp3", "ogg", "flac"],
                                            )
                                            .pick_file()
                                        {
                                            let file_stem = path
                                                .file_stem()
                                                .and_then(|s| s.to_str())
                                                .unwrap_or("Audio Track")
                                                .to_string();
                                            let file_name = path
                                                .file_name()
                                                .and_then(|s| s.to_str())
                                                .unwrap_or("Audio Track")
                                                .to_string();
                                            let file_path_str = path.to_string_lossy().to_string();
                                            header_action = Some(HeaderAction::AddAudioTrack(file_stem, file_name, file_path_str));
                                        }
                                        ui.close_menu();
                                    }

                                    if tracks_count > 1 {
                                        ui.separator();
                                        if ui.button(
                                            egui::RichText::new(lang.tr("Excluir esta Faixa", "Delete this Track"))
                                                .color(Color32::from_rgb(255, 100, 100)),
                                        ).clicked() {
                                            header_action = Some(HeaderAction::DeleteTrack(idx));
                                            ui.close_menu();
                                        }
                                    }
                                });

                                let card_bg = if is_active {
                                    theme.c32_alpha(theme.accent_color, 0.25)
                                } else if idx == 0 {
                                    theme.bg_panel_c32()
                                } else {
                                    theme.bg_canvas_c32()
                                };

                                let card_stroke = if is_active {
                                    Stroke::new(2.0_f32, theme.accent_c32())
                                } else {
                                    Stroke::new(1.0_f32, theme.grid_line_bar_c32())
                                };

                                ui.painter()
                                    .rect_filled(card_rect, Rounding::same(4.0), card_bg);
                                ui.painter().rect_stroke(
                                    card_rect,
                                    Rounding::same(4.0),
                                    card_stroke,
                                );

                                let color_badge = match idx % 4 {
                                    0 => theme.note_fill_c32(),
                                    1 => Color32::from_rgb(230, 126, 34),
                                    2 => theme.accent_c32(),
                                    _ => Color32::from_rgb(241, 196, 15),
                                };

                                ui.painter().rect_filled(
                                    Rect::from_min_size(
                                        card_rect.min + Vec2::new(4.0, 4.0 + track_y_offset),
                                        Vec2::new(6.0, track_height - 8.0),
                                    ),
                                    Rounding::same(2.0),
                                    color_badge,
                                );

                                let mute_rect = Rect::from_min_size(
                                    card_rect.min + Vec2::new(130.0, 6.0 + track_y_offset),
                                    Vec2::new(24.0, 16.0),
                                );
                                let solo_rect = Rect::from_min_size(
                                    card_rect.min + Vec2::new(158.0, 6.0 + track_y_offset),
                                    Vec2::new(24.0, 16.0),
                                );
                                let fx_rect = Rect::from_min_size(
                                    card_rect.min + Vec2::new(186.0, 6.0 + track_y_offset),
                                    Vec2::new(28.0, 16.0),
                                );

                                let mute_resp = ui.allocate_rect(mute_rect, egui::Sense::click());
                                if mute_resp.clicked() {
                                    track.mute = !track.mute;
                                    changed = true;
                                }
                                let mute_bg = if track.mute {
                                    Color32::from_rgb(220, 50, 50)
                                } else {
                                    theme.bg_header_c32()
                                };
                                ui.painter()
                                    .rect_filled(mute_rect, Rounding::same(3.0), mute_bg);
                                ui.painter().text(
                                    mute_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "M",
                                    egui::FontId::proportional(10.0),
                                    Color32::WHITE,
                                );

                                let solo_resp = ui.allocate_rect(solo_rect, egui::Sense::click());
                                if solo_resp.clicked() {
                                    track.solo = !track.solo;
                                    changed = true;
                                }
                                let solo_bg = if track.solo {
                                    theme.accent_c32()
                                } else {
                                    theme.bg_header_c32()
                                };
                                ui.painter()
                                    .rect_filled(solo_rect, Rounding::same(3.0), solo_bg);
                                ui.painter().text(
                                    solo_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "S",
                                    egui::FontId::proportional(10.0),
                                    if track.solo {
                                        Color32::BLACK
                                    } else {
                                        Color32::WHITE
                                    },
                                );

                                let has_fx = track.fx_rack.as_ref().map_or(false, |fx| fx.master_enabled);
                                let fx_resp = ui.allocate_rect(fx_rect, egui::Sense::click());
                                if fx_resp.clicked() {
                                    *active_track_index = idx;
                                    fx_rack_dialog_state.target_track = Some(idx);
                                    fx_rack_dialog_state.is_open = true;
                                }
                                let fx_bg = if has_fx {
                                    theme.accent_c32()
                                } else {
                                    theme.bg_header_c32()
                                };
                                ui.painter().rect_filled(fx_rect, Rounding::same(3.0), fx_bg);
                                if has_fx {
                                    ui.painter().rect_stroke(
                                        fx_rect,
                                        Rounding::same(3.0),
                                        Stroke::new(1.0, Color32::WHITE),
                                    );
                                }
                                ui.painter().text(
                                    fx_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "FX",
                                    egui::FontId::proportional(9.5),
                                    if has_fx {
                                        Color32::BLACK
                                    } else {
                                        theme.text_primary_c32()
                                    },
                                );
                                fx_resp.on_hover_text(if has_fx {
                                    "Rack de Efeitos DSP (Ativo) - Clique para editar"
                                } else {
                                    "Abrir Rack de Efeitos DSP para esta faixa"
                                });

                                let name_rect = Rect::from_min_size(
                                    card_rect.min + Vec2::new(16.0, 4.0 + track_y_offset),
                                    Vec2::new(110.0, 18.0),
                                );
                                ui.allocate_new_ui(
                                    egui::UiBuilder::new().max_rect(name_rect),
                                    |ui| {
                                        let name_resp = ui.add(
                                            egui::TextEdit::singleline(&mut track.name)
                                                .desired_width(110.0)
                                                .font(egui::FontId::proportional(11.0)),
                                        );
                                        if name_resp.changed() {
                                            changed = true;
                                        }
                                    },
                                );

                                let vol_rect = Rect::from_min_size(
                                    card_rect.min + Vec2::new(16.0, 24.0 + track_y_offset),
                                    Vec2::new(190.0, 14.0),
                                );
                                ui.allocate_new_ui(
                                    egui::UiBuilder::new().max_rect(vol_rect),
                                    |ui| {
                                        ui.horizontal(|ui| {
                                            let vol_lbl = ui.label(
                                                egui::RichText::new("Vol:")
                                                    .size(9.0)
                                                    .color(theme.text_muted_c32()),
                                            ).on_hover_text("Clique direito para resetar para 0 dB");
                                            if vol_lbl.clicked_by(egui::PointerButton::Secondary) {
                                                track.volume_db = 0.0;
                                                changed = true;
                                            }
                                            ui.spacing_mut().slider_width = 55.0;
                                            let vol_resp = ui.add(
                                                egui::Slider::new(
                                                    &mut track.volume_db,
                                                    -20.0..=6.0,
                                                )
                                                .show_value(false),
                                            ).on_hover_text(format!("{:.1} dB (Clique direito para resetar)", track.volume_db));
                                            if vol_resp.changed() {
                                                changed = true;
                                            }
                                            if vol_resp.clicked_by(egui::PointerButton::Secondary) {
                                                track.volume_db = 0.0;
                                                changed = true;
                                            }

                                            let pan_lbl = ui.label(
                                                egui::RichText::new("Pan:")
                                                    .size(9.0)
                                                    .color(theme.text_muted_c32()),
                                            ).on_hover_text("Clique direito para centralizar");
                                            if pan_lbl.clicked_by(egui::PointerButton::Secondary) {
                                                track.pan = 0.0;
                                                changed = true;
                                            }
                                            ui.spacing_mut().slider_width = 55.0;
                                            let pan_resp = ui.add(
                                                egui::Slider::new(&mut track.pan, -1.0..=1.0)
                                                    .show_value(false),
                                            ).on_hover_text(format!("Pan: {:.2} (Clique direito para centralizar)", track.pan));
                                            if pan_resp.changed() {
                                                changed = true;
                                            }
                                            if pan_resp.clicked_by(egui::PointerButton::Secondary) {
                                                track.pan = 0.0;
                                                changed = true;
                                            }
                                        });
                                    },
                                );
                            });
                        }
                    });

                    if let Some(action) = header_action {
                        match action {
                            HeaderAction::AddVoiceTrack => {
                                let new_idx = tracks.len();
                                tracks.push(UTrack {
                                    name: format!("Track {}", new_idx + 1),
                                    singer: "Cantor Padrão".to_string(),
                                    volume_db: 0.0,
                                    pan: 0.0,
                                    mute: false,
                                    solo: false,
                                    ..UTrack::default()
                                });
                                parts.push(UVoicePart::new(format!("Parte {}", new_idx + 1), new_idx));
                                *active_track_index = new_idx;
                                changed = true;
                            }
                            HeaderAction::AddAudioTrack(stem, file_name, file_path_str) => {
                                let new_idx = tracks.len();
                                tracks.push(UTrack {
                                    name: stem,
                                    singer: "Instrumental / Áudio".to_string(),
                                    volume_db: 0.0,
                                    pan: 0.0,
                                    mute: false,
                                    solo: false,
                                    ..UTrack::default()
                                });
                                let wave = UWavePart::new(file_name, file_path_str, new_idx);
                                wave_parts.push(wave);
                                *active_track_index = new_idx;
                                changed = true;
                            }
                            HeaderAction::DuplicateTrack(idx) => {
                                if let Some(src) = tracks.get(idx) {
                                    let new_idx = tracks.len();
                                    let mut cloned_track = src.clone();
                                    cloned_track.name = format!("{} (Cópia)", cloned_track.name);
                                    tracks.push(cloned_track);

                                    // Duplicate parts that belong to this track
                                    let src_parts: Vec<UVoicePart> = parts
                                        .iter()
                                        .filter(|p| p.track_index == idx)
                                        .cloned()
                                        .map(|mut p| {
                                            p.track_index = new_idx;
                                            p
                                        })
                                        .collect();
                                    parts.extend(src_parts);

                                    // Duplicate wave parts that belong to this track
                                    let src_waves: Vec<UWavePart> = wave_parts
                                        .iter()
                                        .filter(|w| w.track_index == idx)
                                        .cloned()
                                        .map(|mut w| {
                                            w.track_index = new_idx;
                                            w
                                        })
                                        .collect();
                                    wave_parts.extend(src_waves);

                                    *active_track_index = new_idx;
                                    changed = true;
                                }
                            }
                            HeaderAction::DeleteTrack(del_idx) => {
                                if del_idx < tracks.len() && tracks.len() > 1 {
                                    tracks.remove(del_idx);
                                    parts.retain(|p| p.track_index != del_idx);
                                    wave_parts.retain(|w| w.track_index != del_idx);
                                    for p in parts.iter_mut() {
                                        if p.track_index > del_idx {
                                            p.track_index -= 1;
                                        }
                                    }
                                    for w in wave_parts.iter_mut() {
                                        if w.track_index > del_idx {
                                            w.track_index -= 1;
                                        }
                                    }
                                    if *active_track_index >= tracks.len() {
                                        *active_track_index = tracks.len().saturating_sub(1);
                                    }
                                    changed = true;
                                }
                            }
                        }
                    }

                    // Handle horizontal mouse wheel / trackpad / Shift+wheel scroll over the arrangement timeline
                    let (scroll_dx, is_middle_drag, middle_drag_delta) = ui.input(|i| {
                        let mut h_delta = 0.0f32;
                        if i.modifiers.shift {
                            // When Shift is held, vertical wheel becomes horizontal scroll
                            if i.smooth_scroll_delta.y.abs() > 1e-3 {
                                h_delta += i.smooth_scroll_delta.y;
                            } else if i.raw_scroll_delta.y.abs() > 1e-3 {
                                h_delta += i.raw_scroll_delta.y;
                            }
                        }
                        if i.smooth_scroll_delta.x.abs() > 1e-3 {
                            h_delta += i.smooth_scroll_delta.x;
                        } else if i.raw_scroll_delta.x.abs() > 1e-3 {
                            h_delta += i.raw_scroll_delta.x;
                        }
                        for ev in &i.events {
                            if let egui::Event::MouseWheel { delta, modifiers, .. } = ev {
                                if modifiers.shift && delta.y.abs() > h_delta.abs() {
                                    h_delta = delta.y;
                                } else if delta.x.abs() > h_delta.abs() {
                                    h_delta = delta.x;
                                }
                            }
                        }
                        let is_m_drag = i.pointer.button_down(egui::PointerButton::Middle);
                        let m_delta = i.pointer.delta();
                        (h_delta, is_m_drag, m_delta)
                    });

                    if scroll_dx.abs() > 1e-3 {
                        *horizontal_scroll_offset = (*horizontal_scroll_offset - scroll_dx).max(0.0);
                    }
                    if is_middle_drag && middle_drag_delta.x.abs() > 1e-3 {
                        *horizontal_scroll_offset = (*horizontal_scroll_offset - middle_drag_delta.x).max(0.0);
                    }

                    let scroll_id = egui::Id::new("arrangement_timeline_h_scroll");
                    let scroll_output = egui::ScrollArea::horizontal()
                        .id_salt(scroll_id)
                        .horizontal_scroll_offset(*horizontal_scroll_offset)
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                for (idx, _track) in tracks.iter_mut().enumerate() {
                                    let is_active = idx == *active_track_index;
                                    ui.push_id(("strip", idx), |ui| {
                                        let track_y_offset =
                                            if idx == 0 { ruler_height } else { 0.0 };
                                        let strip_total_h = track_height + track_y_offset;

                                        let (strip_rect, strip_resp) = ui.allocate_exact_size(
                                            Vec2::new(timeline_width, strip_total_h),
                                            egui::Sense::click_and_drag(),
                                        );

                                        let is_primary_action = ui.input(|i| {
                                            i.pointer.primary_down()
                                                || i.pointer.primary_clicked()
                                                || i.pointer.primary_released()
                                        });

                                        if (strip_resp.clicked() || strip_resp.dragged()) && is_primary_action {
                                            *active_track_index = idx;
                                            if let Some(pos) = strip_resp.interact_pointer_pos() {
                                                let clicked_ms = ((pos.x - strip_rect.min.x)
                                                    / px_per_ms)
                                                    .max(0.0)
                                                    as f64;
                                                *playhead_ms = clicked_ms;
                                            }
                                        }

                                        ui.painter().rect_filled(
                                            strip_rect,
                                            Rounding::ZERO,
                                            theme.bg_canvas_c32(),
                                        );
                                        ui.painter().rect_stroke(
                                            strip_rect,
                                            Rounding::ZERO,
                                            Stroke::new(1.0_f32, theme.grid_line_sub_c32()),
                                        );

                                        if idx == 0 {
                                            let ruler_rect = Rect::from_min_size(
                                                strip_rect.min,
                                                Vec2::new(strip_rect.width(), ruler_height),
                                            );
                                            let ruler_resp = ui.allocate_rect(
                                                ruler_rect,
                                                egui::Sense::click_and_drag(),
                                            );

                                            if ruler_resp.clicked() || ruler_resp.dragged() {
                                                if let Some(pos) = ruler_resp.interact_pointer_pos()
                                                {
                                                    let clicked_ms = ((pos.x - strip_rect.min.x)
                                                        / px_per_ms)
                                                        .max(0.0)
                                                        as f64;
                                                    *playhead_ms = clicked_ms;
                                                }
                                            }

                                            ui.painter().rect_filled(
                                                ruler_rect,
                                                Rounding::ZERO,
                                                theme.bg_panel_c32(),
                                            );
                                            ui.painter().rect_stroke(
                                                ruler_rect,
                                                Rounding::ZERO,
                                                Stroke::new(1.0_f32, theme.grid_line_bar_c32()),
                                            );

                                            let beat_ms = 60000.0 / bpm;
                                            let max_beats =
                                                (total_canvas_ms / beat_ms).ceil() as i32;
                                            for b in 0..=max_beats {
                                                let x = ruler_rect.min.x
                                                    + (b as f64 * beat_ms * px_per_ms as f64)
                                                        as f32;
                                                if x > ruler_rect.min.x && x < ruler_rect.max.x {
                                                    ui.painter().line_segment(
                                                        [
                                                            Pos2::new(x, ruler_rect.min.y + 14.0),
                                                            Pos2::new(x, ruler_rect.max.y),
                                                        ],
                                                        Stroke::new(
                                                            1.0_f32,
                                                            theme.text_muted_c32(),
                                                        ),
                                                    );
                                                    if b % 4 == 0 {
                                                        let bar = (b / 4) + 1;
                                                        ui.painter().text(
                                                            Pos2::new(
                                                                x + 4.0,
                                                                ruler_rect.min.y + 2.0,
                                                            ),
                                                            egui::Align2::LEFT_TOP,
                                                            format!("m{}", bar),
                                                            egui::FontId::proportional(11.0),
                                                            theme.text_primary_c32(),
                                                        );
                                                        ui.painter().line_segment(
                                                            [
                                                                Pos2::new(
                                                                    x,
                                                                    ruler_rect.min.y + 8.0,
                                                                ),
                                                                Pos2::new(x, ruler_rect.max.y),
                                                            ],
                                                            Stroke::new(
                                                                1.0_f32,
                                                                theme.text_muted_c32(),
                                                            ),
                                                        );
                                                    }
                                                }
                                            }
                                        }

                                        let color_badge = match idx % 4 {
                                            0 => theme.note_fill_c32(),
                                            1 => Color32::from_rgb(230, 126, 34),
                                            2 => theme.accent_c32(),
                                            _ => Color32::from_rgb(241, 196, 15),
                                        };

                                        let mut part_to_remove: Option<usize> = None;
                                        let mut part_to_duplicate: Option<usize> = None;
                                        let mut part_to_split: Option<(usize, f64)> = None;

                                        for (p_idx, part) in parts.iter_mut().enumerate() {
                                            if part.track_index != idx {
                                                continue;
                                            }
                                            let part_start_ms = part.position_ms;
                                            let part_duration_ms = part
                                                .notes
                                                .iter()
                                                .map(|n| n.position_ms + n.duration_ms)
                                                .fold(2000.0f64, f64::max);

                                            let part_x = strip_rect.min.x
                                                + (part_start_ms * px_per_ms as f64) as f32;
                                            let part_w =
                                                (part_duration_ms * px_per_ms as f64) as f32;

                                            let part_rect = Rect::from_min_size(
                                                Pos2::new(
                                                    part_x,
                                                    strip_rect.min.y + track_y_offset + 2.0,
                                                ),
                                                Vec2::new(part_w.max(60.0), track_height - 4.0),
                                            );

                                            if part_rect.max.x > strip_rect.min.x
                                                && part_rect.min.x < strip_rect.max.x
                                            {
                                                let part_resp = ui.allocate_rect(
                                                    part_rect,
                                                    egui::Sense::click_and_drag(),
                                                );

                                                if part_resp.clicked() {
                                                    *active_track_index = idx;
                                                }

                                                if part_resp.dragged() {
                                                    *active_track_index = idx;
                                                    let delta_x = part_resp.drag_delta().x;
                                                    let delta_ms = (delta_x / px_per_ms) as f64;
                                                    part.position_ms =
                                                        (part.position_ms + delta_ms).max(0.0);
                                                    changed = true;
                                                }

                                                if part_resp.double_clicked() {
                                                    *active_track_index = idx;
                                                    *playhead_ms = part.position_ms;
                                                }

                                                let current_playhead = *playhead_ms;
                                                part_resp.context_menu(|ui| {
                                                    *active_track_index = idx;
                                                    ui.set_min_width(180.0);

                                                    ui.label(
                                                        egui::RichText::new(format!("{}: {}", lang.tr("Parte MIDI", "MIDI Part"), &part.name))
                                                            .strong()
                                                            .size(11.5)
                                                            .color(Color32::from_rgb(0, 255, 157)),
                                                    );
                                                    ui.separator();

                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            egui::RichText::new(lang.tr("Nome:", "Name:"))
                                                                .size(10.0)
                                                                .color(theme.text_muted_c32()),
                                                        );
                                                        let name_edit = ui.add(
                                                            egui::TextEdit::singleline(&mut part.name)
                                                                .desired_width(110.0)
                                                                .font(egui::FontId::proportional(11.0)),
                                                        );
                                                        if name_edit.changed() {
                                                            changed = true;
                                                        }
                                                    });

                                                    ui.separator();

                                                    if ui.button(lang.tr("Duplicar Parte", "Duplicate Part")).clicked() {
                                                        part_to_duplicate = Some(p_idx);
                                                        ui.close_menu();
                                                    }

                                                    if ui.button(lang.tr("Mover Início para 0ms", "Move Start to 0ms")).clicked() {
                                                        part.position_ms = 0.0;
                                                        changed = true;
                                                        ui.close_menu();
                                                    }

                                                    if ui.button(lang.tr("Dividir no Cursor", "Split at Playhead")).clicked() {
                                                        part_to_split = Some((p_idx, current_playhead));
                                                        ui.close_menu();
                                                    }

                                                    if !part.notes.is_empty() {
                                                        if ui.button(lang.tr("Limpar Todas as Notas", "Clear All Notes")).clicked() {
                                                            part.notes.clear();
                                                            changed = true;
                                                            ui.close_menu();
                                                        }
                                                    }

                                                    ui.separator();

                                                    if ui.button(
                                                        egui::RichText::new(lang.tr("Excluir Parte Vocal", "Delete Vocal Part"))
                                                            .color(Color32::from_rgb(255, 100, 100)),
                                                    ).clicked() {
                                                        part_to_remove = Some(p_idx);
                                                        ui.close_menu();
                                                    }
                                                });

                                                let clip_bg = if is_active {
                                                    Color32::from_rgb(10, 60, 40)
                                                } else {
                                                    Color32::from_rgb(20, 45, 35)
                                                };
                                                let clip_stroke = if is_active {
                                                    Stroke::new(
                                                        1.5_f32,
                                                        Color32::from_rgb(0, 255, 157),
                                                    )
                                                } else {
                                                    Stroke::new(1.0_f32, color_badge)
                                                };

                                                ui.painter().rect_filled(
                                                    part_rect,
                                                    Rounding::same(4.0),
                                                    clip_bg,
                                                );
                                                ui.painter().rect_stroke(
                                                    part_rect,
                                                    Rounding::same(4.0),
                                                    clip_stroke,
                                                );

                                                ui.painter().text(
                                                    part_rect.min + Vec2::new(6.0, 2.0),
                                                    egui::Align2::LEFT_TOP,
                                                    &part.name,
                                                    egui::FontId::proportional(10.0),
                                                    Color32::from_rgb(0, 255, 157),
                                                );

                                                for note in &part.notes {
                                                    let start_x = part_rect.min.x
                                                        + (note.position_ms * px_per_ms as f64)
                                                            as f32;
                                                    let width = (note.duration_ms
                                                        * px_per_ms as f64)
                                                        as f32;

                                                    let note_rect = Rect::from_min_size(
                                                        Pos2::new(start_x, part_rect.min.y + 14.0),
                                                        Vec2::new(
                                                            width.max(2.0),
                                                            track_height - 20.0,
                                                        ),
                                                    );

                                                    if note_rect.max.x > part_rect.min.x
                                                        && note_rect.min.x < part_rect.max.x
                                                    {
                                                        ui.painter().rect_filled(
                                                            note_rect,
                                                            Rounding::same(2.0),
                                                            color_badge.linear_multiply(0.9),
                                                        );
                                                        ui.painter().rect_stroke(
                                                            note_rect,
                                                            Rounding::same(1.0),
                                                            Stroke::new(
                                                                1.0_f32,
                                                                Color32::from_rgb(0, 255, 157),
                                                            ),
                                                        );
                                                    }
                                                }
                                            }
                                        }

                                        if let Some(d_idx) = part_to_duplicate {
                                            if let Some(src) = parts.get(d_idx) {
                                                let mut cloned = src.clone();
                                                let dur = cloned.notes.iter().map(|n| n.position_ms + n.duration_ms).fold(2000.0f64, f64::max);
                                                cloned.name = format!("{} (Cópia)", cloned.name);
                                                cloned.position_ms += dur;
                                                parts.push(cloned);
                                                changed = true;
                                            }
                                        }

                                        if let Some((s_idx, split_at_ms)) = part_to_split {
                                            if let Some(src) = parts.get_mut(s_idx) {
                                                let split_rel = split_at_ms - src.position_ms;
                                                if split_rel > 0.0 {
                                                    let mut right_notes = Vec::new();
                                                    src.notes.retain(|n| {
                                                        if n.position_ms >= split_rel {
                                                            let mut moved = n.clone();
                                                            moved.position_ms -= split_rel;
                                                            right_notes.push(moved);
                                                            false
                                                        } else {
                                                            true
                                                        }
                                                    });
                                                    let right_part = UVoicePart {
                                                        name: format!("{} (2)", src.name),
                                                        track_index: src.track_index,
                                                        position_ms: split_at_ms,
                                                        notes: right_notes,
                                                    };
                                                    parts.push(right_part);
                                                    changed = true;
                                                }
                                            }
                                        }

                                        if let Some(r_idx) = part_to_remove {
                                            if r_idx < parts.len() {
                                                parts.remove(r_idx);
                                                changed = true;
                                            }
                                        }

                                        let mut wave_to_remove: Option<usize> = None;
                                        let mut wave_to_duplicate: Option<usize> = None;
                                        let mut wave_to_split: Option<(usize, f64)> = None;

                                        for (w_idx, wave) in wave_parts
                                            .iter_mut()
                                            .enumerate()
                                            .filter(|(_, w)| w.track_index == idx)
                                        {
                                            let wave_dur = if wave.duration_ms > 0.0 {
                                                wave.duration_ms
                                            } else {
                                                if let Some(info) =
                                                    crate::audio::probe_audio_file(&wave.file_path)
                                                {
                                                    wave.duration_ms = info.duration_ms;
                                                    info.duration_ms
                                                } else {
                                                    30_000.0
                                                }
                                            };

                                            let wave_x = strip_rect.min.x
                                                + (wave.position_ms * px_per_ms as f64) as f32;
                                            let wave_w = (wave_dur * px_per_ms as f64) as f32;

                                            let wave_rect = Rect::from_min_size(
                                                Pos2::new(
                                                    wave_x,
                                                    strip_rect.min.y + track_y_offset + 2.0,
                                                ),
                                                Vec2::new(wave_w.max(60.0), track_height - 4.0),
                                            );

                                            if wave_rect.max.x > strip_rect.min.x
                                                && wave_rect.min.x < strip_rect.max.x
                                            {
                                                let wave_resp = ui.allocate_rect(
                                                    wave_rect,
                                                    egui::Sense::click_and_drag(),
                                                );
                                                if wave_resp.clicked() {
                                                    *active_track_index = idx;
                                                }
                                                if wave_resp.dragged() {
                                                    *active_track_index = idx;
                                                    let delta_ms = (wave_resp.drag_delta().x
                                                        / px_per_ms)
                                                        as f64;
                                                    wave.position_ms =
                                                        (wave.position_ms + delta_ms).max(0.0);
                                                    changed = true;
                                                }

                                                let current_playhead = *playhead_ms;
                                                wave_resp.context_menu(|ui| {
                                                    *active_track_index = idx;
                                                    ui.set_min_width(190.0);

                                                    ui.label(
                                                        egui::RichText::new(format!("{}: {}", lang.tr("Áudio", "Audio"), &wave.name))
                                                            .strong()
                                                            .size(11.5)
                                                            .color(Color32::from_rgb(120, 225, 255)),
                                                    );
                                                    ui.separator();

                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            egui::RichText::new(lang.tr("Nome:", "Name:"))
                                                                .size(10.0)
                                                                .color(theme.text_muted_c32()),
                                                        );
                                                        let name_edit = ui.add(
                                                            egui::TextEdit::singleline(&mut wave.name)
                                                                .desired_width(110.0)
                                                                .font(egui::FontId::proportional(11.0)),
                                                        );
                                                        if name_edit.changed() {
                                                            changed = true;
                                                        }
                                                    });

                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            egui::RichText::new(lang.tr("Volume (dB):", "Volume (dB):"))
                                                                .size(10.0)
                                                                .color(theme.text_muted_c32()),
                                                        );
                                                        let vol_edit = ui.add(
                                                            egui::Slider::new(&mut wave.volume_db, -30.0..=12.0)
                                                                .show_value(true)
                                                                .suffix(" dB"),
                                                        );
                                                        if vol_edit.changed() {
                                                            changed = true;
                                                        }
                                                    });

                                                    ui.separator();

                                                    if ui.button(lang.tr("Duplicar Clipe", "Duplicate Clip")).clicked() {
                                                        wave_to_duplicate = Some(w_idx);
                                                        ui.close_menu();
                                                    }

                                                    if ui.button(lang.tr("Mover Início para 0ms", "Move Start to 0ms")).clicked() {
                                                        wave.position_ms = 0.0;
                                                        changed = true;
                                                        ui.close_menu();
                                                    }

                                                    if ui.button(lang.tr("Dividir no Cursor", "Split at Playhead")).clicked() {
                                                        wave_to_split = Some((w_idx, current_playhead));
                                                        ui.close_menu();
                                                    }

                                                    if !wave.file_path.is_empty() {
                                                        let file_path_clone = wave.file_path.clone();
                                                        if ui.button(lang.tr("Revelar Arquivo de Áudio", "Reveal Audio File")).clicked() {
                                                            #[cfg(target_os = "macos")]
                                                            {
                                                                let _ = std::process::Command::new("open")
                                                                    .arg("-R")
                                                                    .arg(&file_path_clone)
                                                                    .spawn();
                                                            }
                                                            #[cfg(target_os = "windows")]
                                                            {
                                                                let _ = std::process::Command::new("explorer")
                                                                    .arg("/select,")
                                                                    .arg(&file_path_clone)
                                                                    .spawn();
                                                            }
                                                            #[cfg(target_os = "linux")]
                                                            {
                                                                let _ = std::process::Command::new("xdg-open")
                                                                    .arg(std::path::Path::new(&file_path_clone).parent().unwrap_or(std::path::Path::new(".")))
                                                                    .spawn();
                                                            }
                                                            ui.close_menu();
                                                        }
                                                    }

                                                    ui.separator();

                                                    if ui.button(
                                                        egui::RichText::new(lang.tr("Excluir Clipe de Áudio", "Delete Audio Clip"))
                                                            .color(Color32::from_rgb(255, 100, 100)),
                                                    ).clicked() {
                                                        wave_to_remove = Some(w_idx);
                                                        ui.close_menu();
                                                    }
                                                });

                                                let wave_bg = if is_active {
                                                    theme.accent_c32().linear_multiply(0.2)
                                                } else {
                                                    theme.card_bg_c32()
                                                };
                                                let wave_stroke = if is_active {
                                                    Stroke::new(1.8_f32, theme.accent_c32())
                                                } else {
                                                    Stroke::new(1.2_f32, theme.card_stroke().color)
                                                };
                                                ui.painter().rect_filled(
                                                    wave_rect,
                                                    Rounding::same(4.0),
                                                    wave_bg,
                                                );
                                                ui.painter().rect_stroke(
                                                    wave_rect,
                                                    Rounding::same(4.0),
                                                    wave_stroke,
                                                );

                                                let inner_w = wave_rect.width() - 8.0;
                                                if inner_w > 10.0 {
                                                    let bar_step = 6.0f32;
                                                    let num_bars =
                                                        (inner_w / bar_step).floor() as usize;
                                                    let center_y = wave_rect.center().y + 6.0;
                                                    for b in 0..num_bars {
                                                        let bx = wave_rect.min.x
                                                            + 6.0
                                                            + b as f32 * bar_step;
                                                        let pseudo_h = 4.0
                                                            + 8.0
                                                                * ((b as f32 * 0.7).sin().abs()
                                                                    * 0.6
                                                                    + (b as f32 * 1.3).cos().abs()
                                                                        * 0.4);
                                                        ui.painter().line_segment(
                                                            [
                                                                Pos2::new(bx, center_y - pseudo_h),
                                                                Pos2::new(bx, center_y + pseudo_h),
                                                            ],
                                                            Stroke::new(
                                                                2.0_f32,
                                                                Color32::from_rgb(0, 160, 240)
                                                                    .linear_multiply(0.6),
                                                            ),
                                                        );
                                                    }
                                                }

                                                let mins =
                                                    (wave_dur / 1000.0 / 60.0).floor() as u32;
                                                let secs =
                                                    ((wave_dur / 1000.0) % 60.0).floor() as u32;
                                                ui.painter().text(
                                                    wave_rect.min + Vec2::new(6.0, 2.0),
                                                    egui::Align2::LEFT_TOP,
                                                    format!(
                                                        "[Áudio] {} [{:02}:{:02}]",
                                                        &wave.name, mins, secs
                                                    ),
                                                    egui::FontId::proportional(10.0),
                                                    Color32::from_rgb(120, 225, 255),
                                                );
                                            }
                                        }

                                        if let Some(d_idx) = wave_to_duplicate {
                                            if let Some(src) = wave_parts.get(d_idx) {
                                                let mut cloned = src.clone();
                                                cloned.name = format!("{} (Cópia)", cloned.name);
                                                cloned.position_ms += cloned.duration_ms.max(1000.0);
                                                wave_parts.push(cloned);
                                                changed = true;
                                            }
                                        }

                                        if let Some((s_idx, split_at_ms)) = wave_to_split {
                                            if let Some(src) = wave_parts.get_mut(s_idx) {
                                                let split_rel = split_at_ms - src.position_ms;
                                                if split_rel > 0.0 && split_rel < src.duration_ms {
                                                    let original_dur = src.duration_ms;
                                                    let original_offset = src.file_offset_ms;

                                                    // Left part keeps 0..split_rel
                                                    src.duration_ms = split_rel;

                                                    // Right part has remaining duration
                                                    let right_wave = UWavePart {
                                                        name: format!("{} (2)", src.name),
                                                        file_path: src.file_path.clone(),
                                                        track_index: src.track_index,
                                                        position_ms: split_at_ms,
                                                        file_offset_ms: original_offset + split_rel,
                                                        volume_db: src.volume_db,
                                                        duration_ms: (original_dur - split_rel).max(0.0),
                                                    };
                                                    wave_parts.push(right_wave);
                                                    changed = true;
                                                }
                                            }
                                        }

                                        if let Some(r_idx) = wave_to_remove {
                                            if r_idx < wave_parts.len() {
                                                wave_parts.remove(r_idx);
                                                changed = true;
                                            }
                                        }

                                        let playhead_x = strip_rect.min.x
                                            + (*playhead_ms * px_per_ms as f64) as f32;
                                        if playhead_x >= strip_rect.min.x
                                            && playhead_x <= strip_rect.max.x
                                        {
                                            ui.painter().line_segment(
                                                [
                                                    Pos2::new(playhead_x, strip_rect.min.y),
                                                    Pos2::new(playhead_x, strip_rect.max.y),
                                                ],
                                                Stroke::new(2.5_f32, theme.playhead_c32()),
                                            );
                                        }
                                    });
                                }
                            });
                        });
                    *horizontal_scroll_offset = scroll_output.state.offset.x;
                });
            });
    });
    changed
}
