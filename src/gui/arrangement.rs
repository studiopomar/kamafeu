use crate::gui::theme::MelodyneTheme;
use crate::project::model::{UTrack, UVoicePart, UWavePart};
use eframe::egui::{self, Color32, Pos2, Rect, Rounding, Stroke, Vec2};

pub fn draw_arrangement_view(
    ui: &mut egui::Ui,
    tracks: &mut Vec<UTrack>,
    parts: &mut Vec<UVoicePart>,
    wave_parts: &mut Vec<UWavePart>,
    active_track_index: &mut usize,
    playhead_ms: &mut f64,
    px_per_ms: f32,
    bpm: f64,
    horizontal_scroll_offset: &mut f32,
) {
    let header_width = 220.0f32;
    let track_height = 42.0f32;
    let ruler_height = 24.0f32;

    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.heading(
                egui::RichText::new("Arrangement (Faixas)")
                    .strong()
                    .size(12.0)
                    .color(MelodyneTheme::TEXT_GOLD_LABEL),
            );
            ui.add_space(10.0);

            if ui
                .button(
                    egui::RichText::new("Nova Track")
                        .size(10.5)
                        .color(Color32::from_rgb(0, 255, 157)),
                )
                .clicked()
            {
                let new_idx = tracks.len();
                let track_name = format!("Track {}", new_idx + 1);
                tracks.push(UTrack {
                    name: track_name.clone(),
                    singer: "Cantor Padrão".to_string(),
                    volume_db: 0.0,
                    pan: 0.0,
                    mute: false,
                    solo: false,
                    ..UTrack::default()
                });
                parts.push(UVoicePart::new(format!("Parte {}", new_idx + 1), new_idx));
                *active_track_index = new_idx;
            }

            if ui
                .button(
                    egui::RichText::new("🎵 Adicionar Áudio...")
                        .size(10.5)
                        .color(Color32::from_rgb(100, 200, 255)),
                )
                .clicked()
            {
                if let Some(path) = crate::dialogs::FileDialog::new()
                    .add_filter(
                        "Áudio (*.wav, *.mp3, *.ogg, *.flac)",
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
                }
            }

            if tracks.len() > 1
                && ui
                    .button(
                        egui::RichText::new("Excluir Track")
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
                }
            }

            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(format!(
                    "Track Ativa: {} ({})",
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

                                let card_bg = if is_active {
                                    Color32::from_rgb(45, 34, 65)
                                } else if idx == 0 {
                                    MelodyneTheme::BG_PANEL
                                } else {
                                    MelodyneTheme::BG_CANVAS
                                };

                                let card_stroke = if is_active {
                                    Stroke::new(2.0_f32, Color32::from_rgb(0, 255, 157))
                                } else {
                                    Stroke::new(1.0_f32, MelodyneTheme::GRID_LINE_BAR)
                                };

                                ui.painter()
                                    .rect_filled(card_rect, Rounding::same(4.0), card_bg);
                                ui.painter().rect_stroke(
                                    card_rect,
                                    Rounding::same(4.0),
                                    card_stroke,
                                );

                                let color_badge = match idx % 4 {
                                    0 => MelodyneTheme::NOTE_GOLD_FILL,
                                    1 => Color32::from_rgb(230, 126, 34),
                                    2 => MelodyneTheme::ACCENT_GOLD,
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
                                    card_rect.min + Vec2::new(145.0, 6.0 + track_y_offset),
                                    Vec2::new(28.0, 16.0),
                                );
                                let solo_rect = Rect::from_min_size(
                                    card_rect.min + Vec2::new(180.0, 6.0 + track_y_offset),
                                    Vec2::new(28.0, 16.0),
                                );

                                let mute_resp = ui.allocate_rect(mute_rect, egui::Sense::click());
                                if mute_resp.clicked() {
                                    track.mute = !track.mute;
                                }
                                let mute_bg = if track.mute {
                                    Color32::from_rgb(220, 50, 50)
                                } else {
                                    Color32::from_rgb(45, 35, 60)
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
                                }
                                let solo_bg = if track.solo {
                                    MelodyneTheme::ACCENT_GOLD
                                } else {
                                    Color32::from_rgb(45, 35, 60)
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

                                let name_rect = Rect::from_min_size(
                                    card_rect.min + Vec2::new(16.0, 4.0 + track_y_offset),
                                    Vec2::new(120.0, 18.0),
                                );
                                ui.allocate_new_ui(
                                    egui::UiBuilder::new().max_rect(name_rect),
                                    |ui| {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut track.name)
                                                .desired_width(120.0)
                                                .font(egui::FontId::proportional(11.0)),
                                        );
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
                                            ui.label(
                                                egui::RichText::new("Vol:")
                                                    .size(9.0)
                                                    .color(MelodyneTheme::TEXT_MUTED),
                                            );
                                            ui.spacing_mut().slider_width = 55.0;
                                            ui.add(
                                                egui::Slider::new(
                                                    &mut track.volume_db,
                                                    -20.0..=6.0,
                                                )
                                                .show_value(false),
                                            );
                                            ui.label(
                                                egui::RichText::new("Pan:")
                                                    .size(9.0)
                                                    .color(MelodyneTheme::TEXT_MUTED),
                                            );
                                            ui.add(
                                                egui::Slider::new(&mut track.pan, -1.0..=1.0)
                                                    .show_value(false),
                                            );
                                        });
                                    },
                                );
                            });
                        }
                    });

                    let scroll_id = ui.make_persistent_id("arrangement_timeline_h_scroll");
                    let mut scroll_area = egui::ScrollArea::horizontal()
                        .id_salt(scroll_id)
                        .auto_shrink([false, false]);

                    if !ui.ctx().is_being_dragged(scroll_id) {
                        scroll_area =
                            scroll_area.horizontal_scroll_offset(*horizontal_scroll_offset);
                    }

                    let scroll_output = scroll_area.show(ui, |ui| {
                        ui.vertical(|ui| {
                            for (idx, _track) in tracks.iter_mut().enumerate() {
                                ui.push_id(("strip", idx), |ui| {
                                    let is_active = idx == *active_track_index;
                                    let track_y_offset = if idx == 0 { ruler_height } else { 0.0 };
                                    let card_total_h = track_height + track_y_offset;

                                    let (strip_rect, strip_resp) = ui.allocate_exact_size(
                                        Vec2::new(timeline_width, card_total_h),
                                        egui::Sense::click_and_drag(),
                                    );

                                    if strip_resp.clicked() || strip_resp.dragged() {
                                        *active_track_index = idx;
                                        if let Some(pos) = strip_resp.interact_pointer_pos() {
                                            let clicked_ms =
                                                ((pos.x - strip_rect.min.x) / px_per_ms).max(0.0)
                                                    as f64;
                                            *playhead_ms = clicked_ms;
                                        }
                                    }

                                    ui.painter().rect_filled(
                                        strip_rect,
                                        Rounding::ZERO,
                                        MelodyneTheme::BG_CANVAS,
                                    );
                                    ui.painter().rect_stroke(
                                        strip_rect,
                                        Rounding::ZERO,
                                        Stroke::new(1.0_f32, MelodyneTheme::GRID_LINE_SUB),
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
                                            if let Some(pos) = ruler_resp.interact_pointer_pos() {
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
                                            MelodyneTheme::BG_PANEL,
                                        );
                                        ui.painter().rect_stroke(
                                            ruler_rect,
                                            Rounding::ZERO,
                                            Stroke::new(1.0_f32, MelodyneTheme::GRID_LINE_BAR),
                                        );

                                        let beat_ms = 60000.0 / bpm;
                                        let max_beats = (total_canvas_ms / beat_ms).ceil() as i32;
                                        for b in 0..=max_beats {
                                            let x = ruler_rect.min.x
                                                + (b as f64 * beat_ms * px_per_ms as f64) as f32;
                                            if x > ruler_rect.min.x && x < ruler_rect.max.x {
                                                ui.painter().line_segment(
                                                    [
                                                        Pos2::new(x, ruler_rect.min.y + 14.0),
                                                        Pos2::new(x, ruler_rect.max.y),
                                                    ],
                                                    Stroke::new(1.0_f32, MelodyneTheme::TEXT_MUTED),
                                                );
                                                if b % 4 == 0 {
                                                    let bar = (b / 4) + 1;
                                                    ui.painter().text(
                                                        Pos2::new(x + 4.0, ruler_rect.min.y + 2.0),
                                                        egui::Align2::LEFT_TOP,
                                                        format!("m{}", bar),
                                                        egui::FontId::proportional(11.0),
                                                        MelodyneTheme::TEXT_NOTE_TAG,
                                                    );
                                                    ui.painter().line_segment(
                                                        [
                                                            Pos2::new(x, ruler_rect.min.y + 8.0),
                                                            Pos2::new(x, ruler_rect.max.y),
                                                        ],
                                                        Stroke::new(
                                                            1.0_f32,
                                                            MelodyneTheme::TEXT_NOTE_TAG,
                                                        ),
                                                    );
                                                }
                                            }
                                        }
                                    }

                                    let color_badge = match idx % 4 {
                                        0 => MelodyneTheme::NOTE_GOLD_FILL,
                                        1 => Color32::from_rgb(230, 126, 34),
                                        2 => MelodyneTheme::ACCENT_GOLD,
                                        _ => Color32::from_rgb(241, 196, 15),
                                    };

                                    for part in parts.iter_mut().filter(|p| p.track_index == idx) {
                                        let part_start_ms = part.position_ms;
                                        let part_duration_ms = part
                                            .notes
                                            .iter()
                                            .map(|n| n.position_ms + n.duration_ms)
                                            .fold(2000.0f64, f64::max);

                                        let part_x = strip_rect.min.x
                                            + (part_start_ms * px_per_ms as f64) as f32;
                                        let part_w = (part_duration_ms * px_per_ms as f64) as f32;

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
                                            }

                                            if part_resp.double_clicked() {
                                                *active_track_index = idx;
                                                *playhead_ms = part.position_ms;
                                            }

                                            let clip_bg = if is_active {
                                                Color32::from_rgb(10, 60, 40)
                                            } else {
                                                Color32::from_rgb(20, 45, 35)
                                            };
                                            let clip_stroke = if is_active {
                                                Stroke::new(1.5_f32, Color32::from_rgb(0, 255, 157))
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
                                                    + (note.position_ms * px_per_ms as f64) as f32;
                                                let width =
                                                    (note.duration_ms * px_per_ms as f64) as f32;

                                                let note_rect = Rect::from_min_size(
                                                    Pos2::new(start_x, part_rect.min.y + 14.0),
                                                    Vec2::new(width.max(2.0), track_height - 20.0),
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

                                    let mut wave_to_remove: Option<usize> = None;
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
                                                let delta_ms =
                                                    (wave_resp.drag_delta().x / px_per_ms) as f64;
                                                wave.position_ms =
                                                    (wave.position_ms + delta_ms).max(0.0);
                                            }
                                            if wave_resp.secondary_clicked() {
                                                wave_to_remove = Some(w_idx);
                                            }

                                            let wave_bg = if is_active {
                                                Color32::from_rgb(15, 45, 75)
                                            } else {
                                                Color32::from_rgb(10, 30, 50)
                                            };
                                            let wave_stroke = if is_active {
                                                Stroke::new(
                                                    1.8_f32,
                                                    Color32::from_rgb(50, 200, 255),
                                                )
                                            } else {
                                                Stroke::new(1.2_f32, Color32::from_rgb(0, 140, 220))
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
                                                    let bx =
                                                        wave_rect.min.x + 6.0 + b as f32 * bar_step;
                                                    let pseudo_h = 4.0
                                                        + 8.0
                                                            * ((b as f32 * 0.7).sin().abs() * 0.6
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

                                            let mins = (wave_dur / 1000.0 / 60.0).floor() as u32;
                                            let secs = ((wave_dur / 1000.0) % 60.0).floor() as u32;
                                            ui.painter().text(
                                                wave_rect.min + Vec2::new(6.0, 2.0),
                                                egui::Align2::LEFT_TOP,
                                                format!(
                                                    "🎵 {} [{:02}:{:02}]",
                                                    &wave.name, mins, secs
                                                ),
                                                egui::FontId::proportional(10.0),
                                                Color32::from_rgb(120, 225, 255),
                                            );
                                        }
                                    }
                                    if let Some(r_idx) = wave_to_remove {
                                        if r_idx < wave_parts.len() {
                                            wave_parts.remove(r_idx);
                                        }
                                    }

                                    let playhead_x =
                                        strip_rect.min.x + (*playhead_ms * px_per_ms as f64) as f32;
                                    if playhead_x >= strip_rect.min.x
                                        && playhead_x <= strip_rect.max.x
                                    {
                                        ui.painter().line_segment(
                                            [
                                                Pos2::new(playhead_x, strip_rect.min.y),
                                                Pos2::new(playhead_x, strip_rect.max.y),
                                            ],
                                            Stroke::new(2.5_f32, MelodyneTheme::PLAYHEAD_RED),
                                        );
                                    }
                                });
                            }
                        });
                    });
                    if ui.ctx().is_being_dragged(scroll_id) {
                        *horizontal_scroll_offset = scroll_output.state.offset.x;
                    }
                });
            });
    });
}
