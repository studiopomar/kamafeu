use crate::gui::piano_roll::state::ArrangementMeterState;
use crate::gui::theme::ThemeConfig;
use crate::project::model::{UTrack, UVoicePart, UWavePart};
use eframe::egui::{self, Pos2, Rect, Rounding, Stroke, Vec2};

#[path = "arrangement/waveform.rs"]
mod waveform;
use waveform::{audio_part_level, cached_waveform};
#[path = "arrangement/operations.rs"]
mod operations;
#[cfg(test)]
use operations::remap_track_index;
use operations::{horizontal_gesture_delta, move_clip_by_frame_delta, move_track_and_remap_parts};

#[allow(clippy::too_many_arguments)]
pub fn draw_arrangement_view(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    tracks: &mut Vec<UTrack>,
    parts: &mut Vec<UVoicePart>,
    wave_parts: &mut Vec<UWavePart>,
    active_track_index: &mut usize,
    playhead_ms: &mut f64,
    px_per_ms: &mut f32,
    bpm: f64,
    horizontal_scroll_offset: &mut f32,
    vertical_scroll_offset: &mut f32,
    horizontal_follow_user_override: &mut bool,
    arrangement_drag_track: &mut Option<usize>,
    arrangement_track_meters: &mut Vec<ArrangementMeterState>,
    is_playing: bool,
    fx_rack_dialog_state: &mut crate::gui::fx_rack_dialog::FxRackDialogState,
    lang: crate::config::AppLanguage,
) -> bool {
    let mut changed = false;
    let header_width = 310.0f32;
    let track_height = 68.0f32;
    let ruler_height = 26.0f32;
    arrangement_track_meters.resize(tracks.len(), ArrangementMeterState::default());

    ui.vertical(|ui| {
        ui.horizontal_wrapped(|ui| {
            ui.heading(
                egui::RichText::new(lang.tr("Arrangement (Faixas)", "Arrangement (Tracks)"))
                    .strong()
                    .size(12.0)
                    .color(theme.text_primary_c32()),
            );
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new(format!(
                    "{} {}  •  {}",
                    tracks.len(),
                    lang.tr("faixas", "tracks"),
                    if is_playing {
                        lang.tr("TOCANDO", "PLAYING")
                    } else {
                        lang.tr("PRONTO", "READY")
                    }
                ))
                .monospace()
                .size(9.0)
                .color(if is_playing {
                    theme.playhead_c32()
                } else {
                    theme.text_muted_c32()
                }),
            );
            ui.add_space(4.0);

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
                parts.push(UVoicePart::new(
                    format!("{} {}", lang.tr("Parte", "Part"), new_idx + 1),
                    new_idx,
                ));
                *active_track_index = new_idx;
                changed = true;
            }

            if ui
                .button(
                    egui::RichText::new(lang.tr("Adicionar Áudio...", "Add Audio..."))
                        .size(10.5)
                        .color(ui.visuals().hyperlink_color),
                )
                .clicked()
            {
                if let Some(path) = crate::dialogs::FileDialog::new()
                    .add_filter(
                        lang.tr(
                            "Áudio (*.wav, *.mp3, *.ogg, *.flac)",
                            "Audio (*.wav, *.mp3, *.ogg, *.flac)",
                        ),
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
                        singer: lang
                            .tr("Instrumental / Áudio", "Instrumental / Audio")
                            .to_string(),
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
                            .color(ui.visuals().error_fg_color),
                    )
                    .clicked()
            {
                let del_idx = *active_track_index;
                if del_idx < tracks.len() {
                    tracks.remove(del_idx);
                    if del_idx < arrangement_track_meters.len() {
                        arrangement_track_meters.remove(del_idx);
                    }
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
                .color(ui.visuals().hyperlink_color),
            );

            ui.separator();
            ui.label(
                egui::RichText::new(lang.tr("Zoom", "Zoom"))
                    .size(9.5)
                    .color(theme.text_muted_c32()),
            );
            if ui
                .small_button("−")
                .on_hover_text(lang.tr("Afastar timeline", "Zoom out timeline"))
                .clicked()
            {
                *px_per_ms = (*px_per_ms * 0.82).clamp(0.01, 0.5);
            }
            ui.add(
                egui::Slider::new(px_per_ms, 0.01..=0.5)
                    .show_value(false)
                    .clamping(egui::SliderClamping::Always),
            )
            .on_hover_text(lang.tr(
                "Arraste para alterar o zoom",
                "Drag to change timeline zoom",
            ));
            if ui
                .small_button("+")
                .on_hover_text(lang.tr("Aproximar timeline", "Zoom in timeline"))
                .clicked()
            {
                *px_per_ms = (*px_per_ms * 1.22).clamp(0.01, 0.5);
            }
            if ui
                .small_button(lang.tr("Ajustar", "Fit"))
                .on_hover_text(
                    lang.tr("Zoom confortável para a sessão", "Comfortable session zoom"),
                )
                .clicked()
            {
                *px_per_ms = 0.08;
            }
            let mut follow = !*horizontal_follow_user_override;
            if ui
                .checkbox(&mut follow, lang.tr("Seguir", "Follow"))
                .on_hover_text(lang.tr(
                    "Acompanhar o playhead durante a reprodução",
                    "Follow the playhead during playback",
                ))
                .changed()
            {
                *horizontal_follow_user_override = !follow;
            }
        });

        ui.add_space(4.0);

        let px_per_ms = (*px_per_ms).clamp(0.01, 0.5);

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

        let arrangement_viewport = ui.available_rect_before_wrap();
        let vertical_delta = ui.input(|input| {
            let hovered = input
                .pointer
                .hover_pos()
                .is_some_and(|pos| arrangement_viewport.contains(pos));
            if !hovered || input.modifiers.shift {
                return 0.0;
            }
            if input.smooth_scroll_delta.y.abs() > 1e-3 {
                input.smooth_scroll_delta.y
            } else {
                input.raw_scroll_delta.y
            }
        });
        if vertical_delta.abs() > 1e-3 {
            *vertical_scroll_offset = (*vertical_scroll_offset - vertical_delta).max(0.0);
            ui.ctx().input_mut(|input| {
                input.smooth_scroll_delta.y = 0.0;
                input.raw_scroll_delta.y = 0.0;
            });
        }

        let mut reorder_target: Option<usize> = None;
        let vertical_scroll_output = egui::ScrollArea::vertical()
            .id_salt("arrangement_tracks_scroll_v")
            .vertical_scroll_offset(*vertical_scroll_offset)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    enum HeaderAction {
                        AddVoice,
                        AddAudio(String, String, String),
                        Delete(usize),
                        Duplicate(usize),
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
                                if card_resp.hovered() && arrangement_drag_track.is_some() {
                                    reorder_target = Some(idx);
                                }

                                let grip_rect = Rect::from_min_size(
                                    card_rect.min + Vec2::new(7.0, 10.0 + track_y_offset),
                                    Vec2::new(15.0, track_height - 18.0),
                                );
                                let grip_resp = ui.allocate_rect(grip_rect, egui::Sense::drag());
                                if grip_resp.drag_started() {
                                    *arrangement_drag_track = Some(idx);
                                    *active_track_index = idx;
                                }
                                if grip_resp.hovered() || arrangement_drag_track == &Some(idx) {
                                    ui.painter().rect_filled(
                                        grip_rect,
                                        Rounding::same(3.0),
                                        theme.accent_soft_c32(),
                                    );
                                }
                                for line in 0..3 {
                                    let y = grip_rect.center().y - 5.0 + line as f32 * 5.0;
                                    ui.painter().line_segment(
                                        [
                                            Pos2::new(grip_rect.min.x + 3.0, y),
                                            Pos2::new(grip_rect.max.x - 3.0, y),
                                        ],
                                        Stroke::new(1.0_f32, theme.text_muted_c32()),
                                    );
                                }
                                grip_resp.on_hover_text(lang.tr(
                                    "Arraste para reorganizar a faixa",
                                    "Drag to reorder this track",
                                ));

                                if is_active {
                                    ui.painter().rect_filled(
                                        Rect::from_min_max(
                                            Pos2::new(
                                                card_rect.min.x,
                                                card_rect.min.y + track_y_offset,
                                            ),
                                            Pos2::new(card_rect.min.x + 3.0, card_rect.max.y),
                                        ),
                                        Rounding::same(1.0),
                                        theme.accent_c32(),
                                    );
                                }

                                card_resp.context_menu(|ui| {
                                    *active_track_index = idx;
                                    ui.set_min_width(175.0);

                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{}: {}",
                                            lang.tr("Faixa", "Track"),
                                            track.name
                                        ))
                                        .strong()
                                        .size(11.5)
                                        .color(theme.text_primary_c32()),
                                    );
                                    ui.separator();

                                    let mute_text = if track.mute {
                                        lang.tr("Desmutar Faixa", "Unmute Track")
                                    } else {
                                        lang.tr("Mutar Faixa", "Mute Track")
                                    };
                                    if ui.button(mute_text).clicked() {
                                        track.mute = !track.mute;
                                        changed = true;
                                        ui.close_menu();
                                    }

                                    let solo_text = if track.solo {
                                        lang.tr("Desativar Solo", "Unsolo Track")
                                    } else {
                                        lang.tr("Solo", "Solo")
                                    };
                                    if ui.button(solo_text).clicked() {
                                        track.solo = !track.solo;
                                        changed = true;
                                        ui.close_menu();
                                    }

                                    let has_fx_enabled =
                                        track.fx_rack.as_ref().is_some_and(|fx| fx.master_enabled);
                                    let fx_menu_text = if has_fx_enabled {
                                        lang.tr(
                                            "Rack de Efeitos DSP (Ativo)...",
                                            "DSP FX Rack (Active)...",
                                        )
                                    } else {
                                        lang.tr("Rack de Efeitos DSP...", "DSP FX Rack...")
                                    };
                                    if ui.button(fx_menu_text).clicked() {
                                        fx_rack_dialog_state.target_track = Some(idx);
                                        fx_rack_dialog_state.is_open = true;
                                        ui.close_menu();
                                    }

                                    if ui
                                        .button(
                                            lang.tr("Resetar Volume e Pan", "Reset Volume and Pan"),
                                        )
                                        .clicked()
                                    {
                                        track.volume_db = 0.0;
                                        track.pan = 0.0;
                                        changed = true;
                                        ui.close_menu();
                                    }

                                    ui.separator();

                                    if ui
                                        .button(lang.tr("Duplicar Faixa", "Duplicate Track"))
                                        .clicked()
                                    {
                                        header_action = Some(HeaderAction::Duplicate(idx));
                                        ui.close_menu();
                                    }

                                    if ui
                                        .button(lang.tr("Nova Faixa Vocal", "New Vocal Track"))
                                        .clicked()
                                    {
                                        header_action = Some(HeaderAction::AddVoice);
                                        ui.close_menu();
                                    }

                                    if ui
                                        .button(lang.tr(
                                            "Adicionar Faixa de Áudio...",
                                            "Add Audio Track...",
                                        ))
                                        .clicked()
                                    {
                                        if let Some(path) = crate::dialogs::FileDialog::new()
                                            .add_filter(
                                                lang.tr(
                                                    "Áudio (*.wav, *.mp3, *.ogg, *.flac)",
                                                    "Audio (*.wav, *.mp3, *.ogg, *.flac)",
                                                ),
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
                                            header_action = Some(HeaderAction::AddAudio(
                                                file_stem,
                                                file_name,
                                                file_path_str,
                                            ));
                                        }
                                        ui.close_menu();
                                    }

                                    if tracks_count > 1 {
                                        ui.separator();
                                        if ui
                                            .button(
                                                egui::RichText::new(
                                                    lang.tr(
                                                        "Excluir esta Faixa",
                                                        "Delete this Track",
                                                    ),
                                                )
                                                .color(ui.visuals().error_fg_color),
                                            )
                                            .clicked()
                                        {
                                            header_action = Some(HeaderAction::Delete(idx));
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
                                if arrangement_drag_track.is_some_and(|from| from != idx)
                                    && reorder_target == Some(idx)
                                {
                                    let marker_y =
                                        if arrangement_drag_track.as_ref().copied().unwrap_or(idx)
                                            < idx
                                        {
                                            card_rect.max.y
                                        } else {
                                            card_rect.min.y
                                        };
                                    ui.painter().line_segment(
                                        [
                                            Pos2::new(card_rect.min.x + 8.0, marker_y),
                                            Pos2::new(card_rect.max.x - 8.0, marker_y),
                                        ],
                                        Stroke::new(2.0_f32, theme.playhead_c32()),
                                    );
                                }
                                if idx == 0 {
                                    ui.painter().text(
                                        card_rect.min + Vec2::new(12.0, 6.0),
                                        egui::Align2::LEFT_TOP,
                                        lang.tr("FAIXAS / MIXER", "TRACKS / MIXER"),
                                        egui::FontId::monospace(8.0),
                                        theme.text_muted_c32(),
                                    );
                                }

                                let color_badge = match idx % 4 {
                                    0 => theme.note_fill_c32(),
                                    1 => ui.visuals().warn_fg_color,
                                    2 => theme.accent_c32(),
                                    _ => ui.visuals().warn_fg_color,
                                };

                                ui.painter().rect_filled(
                                    Rect::from_min_size(
                                        card_rect.min + Vec2::new(25.0, 4.0 + track_y_offset),
                                        Vec2::new(5.0, track_height - 8.0),
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
                                    ui.visuals().error_fg_color
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
                                    ui.visuals().text_color(),
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
                                    ui.visuals().text_color(),
                                );

                                let has_fx =
                                    track.fx_rack.as_ref().is_some_and(|fx| fx.master_enabled);
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
                                ui.painter()
                                    .rect_filled(fx_rect, Rounding::same(3.0), fx_bg);
                                if has_fx {
                                    ui.painter().rect_stroke(
                                        fx_rect,
                                        Rounding::same(3.0),
                                        Stroke::new(1.0_f32, ui.visuals().text_color()),
                                    );
                                }
                                ui.painter().text(
                                    fx_rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    "FX",
                                    egui::FontId::proportional(9.5),
                                    if has_fx {
                                        ui.visuals().text_color()
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
                                    card_rect.min + Vec2::new(34.0, 4.0 + track_y_offset),
                                    Vec2::new(92.0, 18.0),
                                );
                                ui.allocate_new_ui(
                                    egui::UiBuilder::new().max_rect(name_rect),
                                    |ui| {
                                        let edit_id =
                                            egui::Id::new(("arrangement_track_name", idx));
                                        let editing = ui.memory(|memory| {
                                            memory.data.get_temp::<bool>(edit_id).unwrap_or(false)
                                        });
                                        if editing {
                                            let response = ui.add(
                                                egui::TextEdit::singleline(&mut track.name)
                                                    .desired_width(92.0)
                                                    .font(egui::FontId::proportional(11.0)),
                                            );
                                            response.request_focus();
                                            if response.changed() {
                                                changed = true;
                                            }
                                            if response.lost_focus()
                                                || ui.input(|input| {
                                                    input.key_pressed(egui::Key::Enter)
                                                })
                                            {
                                                ui.memory_mut(|memory| {
                                                    memory.data.insert_temp(edit_id, false)
                                                });
                                            }
                                            if !response.has_focus()
                                                && ui.input(|input| {
                                                    input.key_pressed(egui::Key::Escape)
                                                })
                                            {
                                                ui.memory_mut(|memory| {
                                                    memory.data.insert_temp(edit_id, false)
                                                });
                                            }
                                        } else {
                                            let response = ui.add_sized(
                                                [92.0, 18.0],
                                                egui::Button::new(
                                                    egui::RichText::new(&track.name)
                                                        .size(11.0)
                                                        .color(theme.text_primary_c32()),
                                                )
                                                .frame(false),
                                            );
                                            let double_clicked = response.double_clicked();
                                            response.on_hover_text(lang.tr(
                                                "Duplo clique para renomear",
                                                "Double-click to rename",
                                            ));
                                            if double_clicked {
                                                ui.memory_mut(|memory| {
                                                    memory.data.insert_temp(edit_id, true)
                                                });
                                            }
                                        }
                                    },
                                );

                                ui.painter().text(
                                    card_rect.min + Vec2::new(34.0, 21.0 + track_y_offset),
                                    egui::Align2::LEFT_TOP,
                                    if track.singer.is_empty() {
                                        lang.tr("Sem cantor", "No singer")
                                    } else {
                                        &track.singer
                                    },
                                    egui::FontId::proportional(9.0),
                                    theme.text_muted_c32(),
                                );

                                let vol_rect = Rect::from_min_size(
                                    card_rect.min + Vec2::new(28.0, 34.0 + track_y_offset),
                                    Vec2::new(190.0, 14.0),
                                );
                                ui.allocate_new_ui(
                                    egui::UiBuilder::new().max_rect(vol_rect),
                                    |ui| {
                                        ui.horizontal(|ui| {
                                            let vol_lbl = ui
                                                .label(
                                                    egui::RichText::new("Vol:")
                                                        .size(9.0)
                                                        .color(theme.text_muted_c32()),
                                                )
                                                .on_hover_text(
                                                    "Clique direito para resetar para 0 dB",
                                                );
                                            if vol_lbl.clicked_by(egui::PointerButton::Secondary) {
                                                track.volume_db = 0.0;
                                                changed = true;
                                            }
                                            ui.spacing_mut().slider_width = 55.0;
                                            let vol_resp = ui
                                                .add(
                                                    egui::Slider::new(
                                                        &mut track.volume_db,
                                                        -20.0..=6.0,
                                                    )
                                                    .show_value(false),
                                                )
                                                .on_hover_text(format!(
                                                    "{:.1} dB (Clique direito para resetar)",
                                                    track.volume_db
                                                ));
                                            if vol_resp.changed() {
                                                changed = true;
                                            }
                                            if vol_resp.clicked_by(egui::PointerButton::Secondary) {
                                                track.volume_db = 0.0;
                                                changed = true;
                                            }

                                            let pan_lbl = ui
                                                .label(
                                                    egui::RichText::new("Pan:")
                                                        .size(9.0)
                                                        .color(theme.text_muted_c32()),
                                                )
                                                .on_hover_text("Clique direito para centralizar");
                                            if pan_lbl.clicked_by(egui::PointerButton::Secondary) {
                                                track.pan = 0.0;
                                                changed = true;
                                            }
                                            ui.spacing_mut().slider_width = 55.0;
                                            let pan_resp = ui
                                                .add(
                                                    egui::Slider::new(&mut track.pan, -1.0..=1.0)
                                                        .show_value(false),
                                                )
                                                .on_hover_text(format!(
                                                    "Pan: {:.2} (Clique direito para centralizar)",
                                                    track.pan
                                                ));
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

                                let source_level = if is_playing {
                                    wave_parts
                                        .iter()
                                        .filter(|wave| wave.track_index == idx)
                                        .filter_map(|wave| {
                                            audio_part_level(wave, track, *playhead_ms)
                                        })
                                        .fold(0.0f32, f32::max)
                                } else {
                                    0.0
                                };
                                let meter = arrangement_track_meters
                                    .get(idx)
                                    .copied()
                                    .unwrap_or_default();
                                let level = source_level.max(meter.level).clamp(0.0, 1.0);
                                let meter_rect = Rect::from_min_size(
                                    card_rect.min + Vec2::new(222.0, 54.0 + track_y_offset),
                                    Vec2::new(62.0, 8.0),
                                );
                                ui.painter().rect_filled(
                                    meter_rect,
                                    Rounding::same(2.0),
                                    theme.bg_header_c32(),
                                );
                                if level > 0.001 {
                                    let fill = Rect::from_min_max(
                                        meter_rect.min,
                                        Pos2::new(
                                            meter_rect.min.x + meter_rect.width() * level,
                                            meter_rect.max.y,
                                        ),
                                    );
                                    ui.painter().rect_filled(
                                        fill,
                                        Rounding::same(2.0),
                                        if level > 0.9 {
                                            theme.danger_c32()
                                        } else {
                                            theme.accent_c32()
                                        },
                                    );
                                }
                                let peak_x = meter_rect.min.x
                                    + meter_rect.width() * meter.peak.clamp(0.0, 1.0);
                                ui.painter().line_segment(
                                    [
                                        Pos2::new(peak_x, meter_rect.min.y),
                                        Pos2::new(peak_x, meter_rect.max.y),
                                    ],
                                    Stroke::new(1.0_f32, theme.playhead_c32()),
                                );
                                ui.painter().text(
                                    meter_rect.left_top() + Vec2::new(0.0, -11.0),
                                    egui::Align2::LEFT_TOP,
                                    if level > 0.001 {
                                        format!("{:>3.0}%", level * 100.0)
                                    } else {
                                        "—".to_string()
                                    },
                                    egui::FontId::monospace(8.0),
                                    theme.text_muted_c32(),
                                );
                            });
                        }
                    });

                    if let Some(action) = header_action {
                        match action {
                            HeaderAction::AddVoice => {
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
                                parts.push(UVoicePart::new(
                                    format!("Parte {}", new_idx + 1),
                                    new_idx,
                                ));
                                *active_track_index = new_idx;
                                changed = true;
                            }
                            HeaderAction::AddAudio(stem, file_name, file_path_str) => {
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
                            HeaderAction::Duplicate(idx) => {
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
                            HeaderAction::Delete(del_idx) => {
                                if del_idx < tracks.len() && tracks.len() > 1 {
                                    tracks.remove(del_idx);
                                    if del_idx < arrangement_track_meters.len() {
                                        arrangement_track_meters.remove(del_idx);
                                    }
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

                    // Handle two-axis mouse wheel / trackpad / Shift+wheel scrolling only
                    // while the pointer is over the arrangement. Reading the
                    // frame-global scroll delta here would make a gesture in
                    // the inspector move the tracks as well.
                    let arrangement_rect = ui.clip_rect();
                    let is_hovering_arrangement = ui.input(|i| {
                        i.pointer
                            .hover_pos()
                            .is_some_and(|pos| arrangement_rect.contains(pos))
                    });
                    let (scroll_dx, consume_x, consume_shift_y, is_middle_drag, middle_drag_delta) =
                        ui.input(|i| {
                            if !is_hovering_arrangement {
                                return (0.0, false, false, false, Vec2::ZERO);
                            }
                            let delta = if i.smooth_scroll_delta.length_sq() > 1e-6 {
                                i.smooth_scroll_delta
                            } else {
                                i.raw_scroll_delta
                            };
                            // Shift maps a conventional vertical wheel to the timeline;
                            // native two-axis trackpads keep their horizontal axis.
                            let h_delta = horizontal_gesture_delta(delta, i.modifiers.shift);
                            let is_m_drag = i.pointer.button_down(egui::PointerButton::Middle);
                            let m_delta = i.pointer.delta();
                            (
                                h_delta,
                                delta.x.abs() > 1e-3,
                                i.modifiers.shift && delta.y.abs() > 1e-3,
                                is_m_drag,
                                m_delta,
                            )
                        });

                    if scroll_dx.abs() > 1e-3 {
                        *horizontal_scroll_offset =
                            (*horizontal_scroll_offset - scroll_dx).max(0.0);
                    }
                    if is_middle_drag && middle_drag_delta.x.abs() > 1e-3 {
                        *horizontal_scroll_offset =
                            (*horizontal_scroll_offset - middle_drag_delta.x).max(0.0);
                    }
                    if consume_x || consume_shift_y {
                        ui.ctx().input_mut(|input| {
                            if consume_x {
                                input.smooth_scroll_delta.x = 0.0;
                                input.raw_scroll_delta.x = 0.0;
                            }
                            if consume_shift_y {
                                input.smooth_scroll_delta.y = 0.0;
                                input.raw_scroll_delta.y = 0.0;
                            }
                        });
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

                                        if (strip_resp.clicked() || strip_resp.dragged())
                                            && is_primary_action
                                        {
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
                                            let playhead_seconds = (*playhead_ms / 1000.0).max(0.0);
                                            ui.painter().text(
                                                ruler_rect.right_top() + Vec2::new(-8.0, 3.0),
                                                egui::Align2::RIGHT_TOP,
                                                format!(
                                                    "{:02}:{:05.2}",
                                                    (playhead_seconds / 60.0) as u32,
                                                    playhead_seconds % 60.0
                                                ),
                                                egui::FontId::monospace(9.0),
                                                theme.playhead_c32(),
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
                                            1 => ui.visuals().warn_fg_color,
                                            2 => theme.accent_c32(),
                                            _ => ui.visuals().warn_fg_color,
                                        };

                                        let mut part_to_remove: Option<usize> = None;
                                        let mut part_to_duplicate: Option<usize> = None;
                                        let mut part_to_split: Option<(usize, f64)> = None;

                                        let track_pitch_bounds = parts
                                            .iter()
                                            .filter(|part| part.track_index == idx)
                                            .flat_map(|part| {
                                                part.notes.iter().map(|note| note.midi_key())
                                            })
                                            .fold(None, |bounds, midi| match bounds {
                                                None => Some((midi, midi)),
                                                Some((min, max)) => {
                                                    Some((min.min(midi), max.max(midi)))
                                                }
                                            });

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
                                                    // Pointer delta is incremental for this frame.
                                                    // Response::drag_delta is cumulative since the
                                                    // press and must not be added every frame.
                                                    let delta_x =
                                                        ui.input(|input| input.pointer.delta().x);
                                                    part.position_ms = move_clip_by_frame_delta(
                                                        part.position_ms,
                                                        delta_x,
                                                        px_per_ms,
                                                    );
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
                                                        egui::RichText::new(format!(
                                                            "{}: {}",
                                                            lang.tr("Parte MIDI", "MIDI Part"),
                                                            part.name
                                                        ))
                                                        .strong()
                                                        .size(11.5)
                                                        .color(ui.visuals().hyperlink_color),
                                                    );
                                                    ui.separator();

                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            egui::RichText::new(
                                                                lang.tr("Nome:", "Name:"),
                                                            )
                                                            .size(10.0)
                                                            .color(theme.text_muted_c32()),
                                                        );
                                                        let name_edit = ui.add(
                                                            egui::TextEdit::singleline(
                                                                &mut part.name,
                                                            )
                                                            .desired_width(110.0)
                                                            .font(egui::FontId::proportional(11.0)),
                                                        );
                                                        if name_edit.changed() {
                                                            changed = true;
                                                        }
                                                    });

                                                    ui.separator();

                                                    if ui
                                                        .button(
                                                            lang.tr(
                                                                "Duplicar Parte",
                                                                "Duplicate Part",
                                                            ),
                                                        )
                                                        .clicked()
                                                    {
                                                        part_to_duplicate = Some(p_idx);
                                                        ui.close_menu();
                                                    }

                                                    if ui
                                                        .button(lang.tr(
                                                            "Mover Início para 0ms",
                                                            "Move Start to 0ms",
                                                        ))
                                                        .clicked()
                                                    {
                                                        part.position_ms = 0.0;
                                                        changed = true;
                                                        ui.close_menu();
                                                    }

                                                    if ui
                                                        .button(lang.tr(
                                                            "Dividir no Cursor",
                                                            "Split at Playhead",
                                                        ))
                                                        .clicked()
                                                    {
                                                        part_to_split =
                                                            Some((p_idx, current_playhead));
                                                        ui.close_menu();
                                                    }

                                                    if !part.notes.is_empty()
                                                        && ui
                                                            .button(lang.tr(
                                                                "Limpar Todas as Notas",
                                                                "Clear All Notes",
                                                            ))
                                                            .clicked()
                                                    {
                                                        part.notes.clear();
                                                        changed = true;
                                                        ui.close_menu();
                                                    }

                                                    ui.separator();

                                                    if ui
                                                        .button(
                                                            egui::RichText::new(lang.tr(
                                                                "Excluir Parte Vocal",
                                                                "Delete Vocal Part",
                                                            ))
                                                            .color(ui.visuals().error_fg_color),
                                                        )
                                                        .clicked()
                                                    {
                                                        part_to_remove = Some(p_idx);
                                                        ui.close_menu();
                                                    }
                                                });

                                                let clip_bg = ui.visuals().panel_fill;
                                                let clip_stroke = if is_active {
                                                    Stroke::new(
                                                        1.5_f32,
                                                        ui.visuals().hyperlink_color,
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
                                                    ui.visuals().hyperlink_color,
                                                );

                                                let (pitch_min, pitch_max) =
                                                    track_pitch_bounds.unwrap_or((54, 66));
                                                let center = (u16::from(pitch_min)
                                                    + u16::from(pitch_max))
                                                    as f32
                                                    * 0.5;
                                                let pitch_span =
                                                    (pitch_max.saturating_sub(pitch_min) as f32
                                                        + 4.0)
                                                        .max(12.0);
                                                let display_min = center - pitch_span * 0.5;
                                                let melody_top = part_rect.min.y + 15.0;
                                                let melody_height =
                                                    (part_rect.height() - 19.0).max(12.0);

                                                for note in &part.notes {
                                                    let start_x = part_rect.min.x
                                                        + (note.position_ms * px_per_ms as f64)
                                                            as f32;
                                                    let width = (note.duration_ms
                                                        * px_per_ms as f64)
                                                        as f32;

                                                    let pitch_norm = ((note.midi_key() as f32
                                                        - display_min)
                                                        / pitch_span)
                                                        .clamp(0.0, 1.0);
                                                    let note_y = melody_top
                                                        + (1.0 - pitch_norm)
                                                            * (melody_height - 3.0);

                                                    let note_rect = Rect::from_min_size(
                                                        Pos2::new(start_x, note_y),
                                                        Vec2::new(width.max(2.0), 3.0),
                                                    );

                                                    if note_rect.max.x > part_rect.min.x
                                                        && note_rect.min.x < part_rect.max.x
                                                    {
                                                        ui.painter().rect_filled(
                                                            note_rect,
                                                            Rounding::same(0.5),
                                                            ui.visuals().text_color(),
                                                        );
                                                    }
                                                }
                                            }
                                        }

                                        if let Some(d_idx) = part_to_duplicate {
                                            if let Some(src) = parts.get(d_idx) {
                                                let mut cloned = src.clone();
                                                let dur = cloned
                                                    .notes
                                                    .iter()
                                                    .map(|n| n.position_ms + n.duration_ms)
                                                    .fold(2000.0f64, f64::max);
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
                                                    let delta_x =
                                                        ui.input(|input| input.pointer.delta().x);
                                                    wave.position_ms = move_clip_by_frame_delta(
                                                        wave.position_ms,
                                                        delta_x,
                                                        px_per_ms,
                                                    );
                                                    changed = true;
                                                }

                                                let current_playhead = *playhead_ms;
                                                wave_resp.context_menu(|ui| {
                                                    *active_track_index = idx;
                                                    ui.set_min_width(190.0);

                                                    ui.label(
                                                        egui::RichText::new(format!(
                                                            "{}: {}",
                                                            lang.tr("Áudio", "Audio"),
                                                            wave.name
                                                        ))
                                                        .strong()
                                                        .size(11.5)
                                                        .color(ui.visuals().hyperlink_color),
                                                    );
                                                    ui.separator();

                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            egui::RichText::new(
                                                                lang.tr("Nome:", "Name:"),
                                                            )
                                                            .size(10.0)
                                                            .color(theme.text_muted_c32()),
                                                        );
                                                        let name_edit = ui.add(
                                                            egui::TextEdit::singleline(
                                                                &mut wave.name,
                                                            )
                                                            .desired_width(110.0)
                                                            .font(egui::FontId::proportional(11.0)),
                                                        );
                                                        if name_edit.changed() {
                                                            changed = true;
                                                        }
                                                    });

                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            egui::RichText::new(lang.tr(
                                                                "Volume (dB):",
                                                                "Volume (dB):",
                                                            ))
                                                            .size(10.0)
                                                            .color(theme.text_muted_c32()),
                                                        );
                                                        let vol_edit = ui.add(
                                                            egui::Slider::new(
                                                                &mut wave.volume_db,
                                                                -30.0..=12.0,
                                                            )
                                                            .show_value(true)
                                                            .suffix(" dB"),
                                                        );
                                                        if vol_edit.changed() {
                                                            changed = true;
                                                        }
                                                    });

                                                    ui.separator();

                                                    if ui
                                                        .button(
                                                            lang.tr(
                                                                "Duplicar Clipe",
                                                                "Duplicate Clip",
                                                            ),
                                                        )
                                                        .clicked()
                                                    {
                                                        wave_to_duplicate = Some(w_idx);
                                                        ui.close_menu();
                                                    }

                                                    if ui
                                                        .button(lang.tr(
                                                            "Mover Início para 0ms",
                                                            "Move Start to 0ms",
                                                        ))
                                                        .clicked()
                                                    {
                                                        wave.position_ms = 0.0;
                                                        changed = true;
                                                        ui.close_menu();
                                                    }

                                                    if ui
                                                        .button(lang.tr(
                                                            "Dividir no Cursor",
                                                            "Split at Playhead",
                                                        ))
                                                        .clicked()
                                                    {
                                                        wave_to_split =
                                                            Some((w_idx, current_playhead));
                                                        ui.close_menu();
                                                    }

                                                    if !wave.file_path.is_empty() {
                                                        let file_path_clone =
                                                            wave.file_path.clone();
                                                        if ui
                                                            .button(lang.tr(
                                                                "Revelar Arquivo de Áudio",
                                                                "Reveal Audio File",
                                                            ))
                                                            .clicked()
                                                        {
                                                            #[cfg(target_os = "macos")]
                                                            {
                                                                let _ = std::process::Command::new(
                                                                    "open",
                                                                )
                                                                .arg("-R")
                                                                .arg(&file_path_clone)
                                                                .spawn();
                                                            }
                                                            #[cfg(target_os = "windows")]
                                                            {
                                                                let _ = std::process::Command::new(
                                                                    "explorer",
                                                                )
                                                                .arg("/select,")
                                                                .arg(&file_path_clone)
                                                                .spawn();
                                                            }
                                                            #[cfg(target_os = "linux")]
                                                            {
                                                                let _ = std::process::Command::new(
                                                                    "xdg-open",
                                                                )
                                                                .arg(
                                                                    std::path::Path::new(
                                                                        &file_path_clone,
                                                                    )
                                                                    .parent()
                                                                    .unwrap_or(
                                                                        std::path::Path::new("."),
                                                                    ),
                                                                )
                                                                .spawn();
                                                            }
                                                            ui.close_menu();
                                                        }
                                                    }

                                                    ui.separator();

                                                    if ui
                                                        .button(
                                                            egui::RichText::new(lang.tr(
                                                                "Excluir Clipe de Áudio",
                                                                "Delete Audio Clip",
                                                            ))
                                                            .color(ui.visuals().error_fg_color),
                                                        )
                                                        .clicked()
                                                    {
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
                                                    // Use the actual clip samples, with one
                                                    // min/max pair per display pixel. This keeps
                                                    // transients readable at both zoom levels and
                                                    // avoids the misleading synthetic waveform.
                                                    if let Some(preview) =
                                                        cached_waveform(&wave.file_path)
                                                    {
                                                        let center_y = wave_rect.center().y + 3.0;
                                                        let half_h =
                                                            (wave_rect.height() * 0.40).max(5.0);
                                                        ui.painter().line_segment(
                                                            [
                                                                Pos2::new(
                                                                    wave_rect.min.x + 4.0,
                                                                    center_y,
                                                                ),
                                                                Pos2::new(
                                                                    wave_rect.max.x - 4.0,
                                                                    center_y,
                                                                ),
                                                            ],
                                                            Stroke::new(
                                                                0.5_f32,
                                                                ui.visuals()
                                                                    .hyperlink_color
                                                                    .linear_multiply(70.0 / 255.0),
                                                            ),
                                                        );
                                                        let pixels = inner_w.ceil() as usize;
                                                        for b in 0..pixels {
                                                            let bx = wave_rect.min.x
                                                                + 4.0
                                                                + b as f32 * inner_w
                                                                    / pixels as f32;
                                                            let clip_start_ms =
                                                                wave_dur * b as f64 / pixels as f64;
                                                            let clip_end_ms = wave_dur
                                                                * (b + 1) as f64
                                                                / pixels as f64;
                                                            let source_start_ms = (wave
                                                                .file_offset_ms
                                                                + clip_start_ms)
                                                                .clamp(0.0, preview.duration_ms);
                                                            let source_end_ms =
                                                                (wave.file_offset_ms + clip_end_ms)
                                                                    .clamp(
                                                                        source_start_ms,
                                                                        preview.duration_ms,
                                                                    );
                                                            let first = (source_start_ms
                                                                / preview.bucket_ms)
                                                                .floor()
                                                                as usize;
                                                            let last = ((source_end_ms
                                                                / preview.bucket_ms)
                                                                .ceil()
                                                                as usize)
                                                                .max(first + 1)
                                                                .min(preview.peaks.len());
                                                            let (mut lo, mut hi) =
                                                                (1.0f32, -1.0f32);
                                                            for &(bucket_lo, bucket_hi) in &preview
                                                                .peaks
                                                                [first.min(preview.peaks.len() - 1)
                                                                    ..last]
                                                            {
                                                                lo = lo.min(bucket_lo);
                                                                hi = hi.max(bucket_hi);
                                                            }
                                                            if hi >= lo {
                                                                ui.painter().line_segment(
                                                                    [
                                                                        Pos2::new(
                                                                            bx,
                                                                            center_y - hi * half_h,
                                                                        ),
                                                                        Pos2::new(
                                                                            bx,
                                                                            center_y - lo * half_h,
                                                                        ),
                                                                    ],
                                                                    Stroke::new(
                                                                        1.0_f32,
                                                                        ui.visuals()
                                                                            .hyperlink_color,
                                                                    ),
                                                                );
                                                            }
                                                        }
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
                                                        wave.name, mins, secs
                                                    ),
                                                    egui::FontId::proportional(10.0),
                                                    ui.visuals().hyperlink_color,
                                                );
                                            }
                                        }

                                        if let Some(d_idx) = wave_to_duplicate {
                                            if let Some(src) = wave_parts.get(d_idx) {
                                                let mut cloned = src.clone();
                                                cloned.name = format!("{} (Cópia)", cloned.name);
                                                cloned.position_ms +=
                                                    cloned.duration_ms.max(1000.0);
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
                                                        duration_ms: (original_dur - split_rel)
                                                            .max(0.0),
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
        *vertical_scroll_offset = vertical_scroll_output.state.offset.y;

        if ui.input(|input| input.pointer.any_released()) {
            if let Some(from) = arrangement_drag_track.take() {
                if let Some(to) = reorder_target {
                    if move_track_and_remap_parts(
                        tracks,
                        parts,
                        wave_parts,
                        arrangement_track_meters,
                        active_track_index,
                        from,
                        to,
                    ) {
                        changed = true;
                    }
                }
            }
        }
    });
    changed
}

#[cfg(test)]
mod tests {
    use super::{
        horizontal_gesture_delta, move_clip_by_frame_delta, move_track_and_remap_parts,
        remap_track_index,
    };
    use crate::gui::piano_roll::state::ArrangementMeterState;
    use crate::project::model::{UTrack, UVoicePart, UWavePart};
    use eframe::egui::Vec2;

    #[test]
    fn clip_drag_applies_only_the_current_frame_delta() {
        let mut position = 1_000.0;
        for _ in 0..10 {
            position = move_clip_by_frame_delta(position, 10.0, 0.1);
        }
        assert_eq!(position, 2_000.0);
    }

    #[test]
    fn arrangement_maps_trackpad_and_shift_wheel_to_horizontal_axis() {
        assert_eq!(horizontal_gesture_delta(Vec2::new(12.0, 40.0), false), 12.0);
        assert_eq!(horizontal_gesture_delta(Vec2::new(12.0, 40.0), true), 52.0);
    }

    #[test]
    fn moving_a_track_remaps_all_part_indices_and_selection() {
        let mut tracks = vec![
            UTrack {
                name: "A".into(),
                ..UTrack::default()
            },
            UTrack {
                name: "B".into(),
                ..UTrack::default()
            },
            UTrack {
                name: "C".into(),
                ..UTrack::default()
            },
        ];
        let mut parts = vec![UVoicePart::new("part-a", 0), UVoicePart::new("part-c", 2)];
        let mut waves = vec![UWavePart {
            name: "audio-b".into(),
            file_path: String::new(),
            track_index: 1,
            position_ms: 0.0,
            file_offset_ms: 0.0,
            volume_db: 0.0,
            duration_ms: 100.0,
        }];
        let mut meters = vec![
            ArrangementMeterState {
                level: 0.1,
                peak: 0.2,
            },
            ArrangementMeterState {
                level: 0.3,
                peak: 0.4,
            },
            ArrangementMeterState {
                level: 0.5,
                peak: 0.6,
            },
        ];
        let mut active = 0;

        assert!(move_track_and_remap_parts(
            &mut tracks,
            &mut parts,
            &mut waves,
            &mut meters,
            &mut active,
            0,
            2,
        ));
        assert_eq!(
            tracks
                .iter()
                .map(|track| track.name.as_str())
                .collect::<Vec<_>>(),
            ["B", "C", "A"]
        );
        assert_eq!(
            parts
                .iter()
                .map(|part| part.track_index)
                .collect::<Vec<_>>(),
            [2, 1]
        );
        assert_eq!(waves[0].track_index, 0);
        assert_eq!(
            meters.iter().map(|meter| meter.level).collect::<Vec<_>>(),
            [0.3, 0.5, 0.1]
        );
        assert_eq!(active, 2);
    }

    #[test]
    fn remap_track_index_preserves_unrelated_tracks() {
        assert_eq!(remap_track_index(4, 1, 3), 4);
        assert_eq!(remap_track_index(2, 1, 3), 1);
        assert_eq!(remap_track_index(2, 3, 1), 3);
    }
}
