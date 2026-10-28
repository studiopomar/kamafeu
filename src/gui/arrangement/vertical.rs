use super::*;

#[allow(clippy::too_many_arguments)]
pub fn draw_arrangement_view(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    tracks: &mut Vec<UTrack>,
    parts: &mut Vec<UVoicePart>,
    waves: &mut Vec<UWavePart>,
    active: &mut usize,
    playhead: &mut f64,
    zoom: &mut f32,
    bpm: f64,
    scroll_x: &mut f32,
    scroll_y: &mut f32,
    manual: &mut bool,
    drag: &mut Option<usize>,
    meters: &mut Vec<ArrangementMeterState>,
    playing: bool,
    fx: &mut crate::gui::fx_rack_dialog::FxRackDialogState,
    lang: crate::config::AppLanguage,
    master: &mut crate::audio::FxRackConfig,
) -> bool {
    let mut changed = false;
    meters.resize(tracks.len(), ArrangementMeterState::default());
    ui.horizontal(|ui| {
        ui.strong(lang.tr("Arranjo", "Arrangement"));
        if ui.small_button(lang.tr("FX Master", "Master FX"))
            .on_hover_text(lang.tr("Abrir rack de efeitos do master", "Open master effects rack"))
            .clicked()
        {
            fx.target_track = None;
            fx.is_open = true;
        }
        if ui
            .small_button("+")
            .on_hover_text(lang.tr("Nova faixa", "New track"))
            .clicked()
        {
            tracks.push(UTrack {
                name: format!("Track {}", tracks.len() + 1),
                ..Default::default()
            });
            *active = tracks.len() - 1;
            let mut part = UVoicePart::new(format!("Parte vocal {}", tracks.len()), *active);
            part.position_ms = *playhead;
            parts.push(part);
            changed = true;
        }
        let mut follow = !*manual;
        if ui
            .checkbox(&mut follow, lang.tr("Seguir", "Follow"))
            .changed()
        {
            *manual = !follow;
        }
    });
    meters.resize(tracks.len(), ArrangementMeterState::default());
    let widths_id = ui.id().with("track_column_widths");
    let default_width =
        ((ui.available_width() - 32.0) / tracks.len().max(1) as f32).clamp(72.0, 112.0);
    let mut widths = ui
        .ctx()
        .data_mut(|d| d.get_persisted::<Vec<f32>>(widths_id).unwrap_or_default());
    widths.resize(tracks.len(), default_width);
    let mut common_width = widths.first().copied().unwrap_or(default_width);
    ui.horizontal(|ui| {
        ui.label(lang.tr("Largura das faixas", "Track width"));
        ui.spacing_mut().slider_width = 70.0;
        if ui
            .add(egui::Slider::new(&mut common_width, 72.0..=240.0).suffix(" px"))
            .changed()
        {
            widths.fill(common_width);
        }
    });
    let offsets: Vec<f32> = widths
        .iter()
        .scan(0.0, |sum, &w| {
            let x = *sum;
            *sum += w;
            Some(x)
        })
        .collect();
    let total_width: f32 = widths.iter().sum();
    let overflow = (total_width - (ui.available_width() - 32.0)).max(0.0);
    if overflow > 0.0 {
        if ui
            .add(egui::Slider::new(scroll_x, 0.0..=overflow).show_value(false))
            .on_hover_text("Faixas / Tracks")
            .changed()
        {
            *manual = true;
        }
    }
    let lower_height = (ui.available_height() * 0.52).clamp(100.0, 360.0);
    let height = (ui.available_height() - lower_height - 74.0).max(60.0);
    let (area, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), height + 66.0),
        egui::Sense::hover(),
    );
    let timeline = Rect::from_min_size(
        area.min + Vec2::new(32.0, 66.0),
        Vec2::new(area.width() - 32.0, height),
    );
    let duration = parts
        .iter()
        .map(|p| {
            p.position_ms
                + p.notes
                    .iter()
                    .map(|n| n.position_ms + n.duration_ms)
                    .fold(1000.0, f64::max)
        })
        .chain(waves.iter().map(|w| w.position_ms + w.duration_ms))
        .fold(30000.0, f64::max)
        + 10000.0;
    if ui.input(|i| i.pointer.hover_pos().is_some_and(|p| area.contains(p))) {
        let (delta, modifiers, pointer) = ui.input(|i| {
            (
                if i.modifiers.ctrl || i.modifiers.command {
                    i.raw_scroll_delta
                } else {
                    i.smooth_scroll_delta
                },
                i.modifiers,
                i.pointer.hover_pos(),
            )
        });
        if delta != Vec2::ZERO {
            if modifiers.command || modifiers.ctrl {
                let anchor = pointer
                    .map(|p| (p.y - timeline.top()).clamp(0.0, height))
                    .unwrap_or(height / 2.0);
                let time = (*scroll_y + anchor) / *zoom;
                *zoom = (*zoom * (delta.y * 0.005).exp()).clamp(0.005, 0.5);
                *scroll_y = time * *zoom - anchor;
            } else {
                *scroll_x -= delta.x + if modifiers.shift { delta.y } else { 0.0 };
                if !modifiers.shift {
                    *scroll_y -= delta.y;
                }
            }
            *manual = true;
        }
    }
    if playing && !*manual {
        let y = *playhead as f32 * *zoom;
        if y < *scroll_y || y > *scroll_y + height * 0.85 {
            *scroll_y = (y - height * 0.2).max(0.0);
        }
    }
    *scroll_x = scroll_x.clamp(0.0, (total_width - timeline.width()).max(0.0));
    *scroll_y = scroll_y.clamp(0.0, (duration as f32 * *zoom - height).max(0.0));
    let painter = ui.painter().with_clip_rect(area);
    painter.rect_filled(timeline, 0.0, theme.bg_panel_c32());
    let step = (60000.0 / bpm.max(1.0) * 4.0) as f32;
    for beat in ((*scroll_y / *zoom / step).floor() as usize)
        ..=(((*scroll_y + height) / *zoom / step).ceil() as usize)
    {
        let y = timeline.top() + beat as f32 * step * *zoom - *scroll_y;
        painter.line_segment(
            [Pos2::new(area.left(), y), Pos2::new(area.right(), y)],
            Stroke::new(0.5_f32, theme.grid_line_sub_c32()),
        );
        painter.text(
            Pos2::new(area.left() + 2.0, y),
            egui::Align2::LEFT_TOP,
            format!("{}", beat + 1),
            egui::FontId::monospace(9.0),
            theme.text_muted_c32(),
        );
    }
    let seek = ui.interact(
        Rect::from_min_max(Pos2::new(area.left(), timeline.top()), timeline.max),
        ui.id().with("seek"),
        egui::Sense::click(),
    );
    if seek.clicked() {
        if let Some(p) = seek.interact_pointer_pos() {
            *playhead = ((p.y - timeline.top() + *scroll_y) / *zoom) as f64;
        }
    }
    let mut new_part = None;
    let mut delete_track = None;
    let mut duplicate_track = None;
    let mut reorder = None;
    for (i, track) in tracks.iter_mut().enumerate() {
        let width = widths[i];
        let x = timeline.left() + offsets[i] - *scroll_x;
        if x + width < timeline.left() || x > area.right() {
            continue;
        }
        let header = Rect::from_min_size(Pos2::new(x, area.top()), Vec2::new(width, 64.0));
        let active_t = ui
            .ctx()
            .animate_bool(ui.id().with(("track_active_animation", i)), *active == i);
        let hover_t = ui.ctx().animate_bool(
            ui.id().with(("track_hover_animation", i)),
            header.contains(ui.input(|input| input.pointer.hover_pos().unwrap_or(Pos2::ZERO))),
        );
        if active_t > 0.001 || hover_t > 0.001 {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(16));
            painter.with_clip_rect(header.intersect(area)).rect_filled(
                header,
                3.0,
                theme
                    .accent_c32()
                    .gamma_multiply(0.14 * active_t + 0.07 * hover_t),
            );
        }
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(("track_header", i))
                .max_rect(header)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        child.set_clip_rect(header.intersect(area));
        let rename_id = ui.id().with(("rename", i));
        let renaming = ui
            .ctx()
            .data_mut(|d| d.get_temp::<bool>(rename_id).unwrap_or(false));
        if renaming {
            let r = child.add_sized(
                [width - 4.0, 18.0],
                egui::TextEdit::singleline(&mut track.name),
            );
            changed |= r.changed();
            if !r.has_focus() && !r.lost_focus() {
                r.request_focus();
            }
            if r.lost_focus() {
                ui.ctx().data_mut(|d| d.insert_temp(rename_id, false));
            }
        } else {
            let r = child
                .add(
                    egui::Label::new(egui::RichText::new(&track.name).strong())
                        .truncate()
                        .sense(egui::Sense::click()),
                )
                .on_hover_text(&track.name);
            if r.clicked() {
                *active = i;
            }
            if r.double_clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(rename_id, true));
            }
            r.context_menu(|ui| {
                if ui
                    .button(lang.tr("Nova parte vocal", "New vocal part"))
                    .clicked()
                {
                    new_part = Some(i);
                    ui.close_menu();
                }
                if ui.button(lang.tr("Duplicar", "Duplicate")).clicked() {
                    duplicate_track = Some(i);
                    ui.close_menu();
                }
                if ui.button(lang.tr("Excluir", "Delete")).clicked() {
                    delete_track = Some(i);
                    ui.close_menu();
                }
            });
        }
        child
            .add(egui::Label::new(&track.singer).truncate())
            .on_hover_text(&track.singer);
        child.horizontal(|ui| {
            if ui.small_button("FX").clicked() {
                fx.target_track = Some(i);
                fx.is_open = true;
            }
            let r = ui
                .add(egui::Label::new("::").sense(egui::Sense::drag()))
                .on_hover_text("Arrastar para reordenar / Drag to reorder");
            if r.drag_started() {
                *drag = Some(i);
            }
        });
        let edge = Rect::from_min_max(
            Pos2::new(header.right() - 5.0, header.top()),
            header.right_bottom(),
        );
        let resize = ui
            .interact(
                edge.intersect(area),
                ui.id().with(("resize_track", i)),
                egui::Sense::drag(),
            )
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal)
            .on_hover_text(lang.tr("Arraste para ajustar a largura", "Drag to resize track"));
        painter.line_segment(
            [edge.center_top(), edge.center_bottom()],
            Stroke::new(1.0_f32, theme.text_muted_c32()),
        );
        if resize.dragged() {
            widths[i] = (width + resize.drag_delta().x).clamp(72.0, 240.0);
            ui.ctx().request_repaint();
        }
        if ui.input(|i| i.pointer.any_released())
            && header.contains(ui.input(|i| i.pointer.hover_pos()).unwrap_or(Pos2::ZERO))
        {
            if let Some(from) = drag.take() {
                reorder = Some((from, i));
            }
        }
        painter.with_clip_rect(timeline).line_segment(
            [
                Pos2::new(x, timeline.top()),
                Pos2::new(x, timeline.bottom()),
            ],
            Stroke::new(1.0_f32, theme.grid_line_sub_c32()),
        );
    }
    let mut delete_voice = None;
    let mut duplicate_voice = None;
    let mut split_voice = None;
    for (i, part) in parts.iter_mut().enumerate() {
        let width = widths
            .get(part.track_index)
            .copied()
            .unwrap_or(default_width);
        let duration = part
            .notes
            .iter()
            .map(|n| n.position_ms + n.duration_ms)
            .fold(500.0, f64::max);
        let rect = Rect::from_min_size(
            Pos2::new(
                timeline.left() + offsets.get(part.track_index).copied().unwrap_or(0.0) - *scroll_x
                    + 3.0,
                timeline.top() + part.position_ms as f32 * *zoom - *scroll_y,
            ),
            Vec2::new(width - 6.0, (duration as f32 * *zoom).max(8.0)),
        );
        if !rect.intersects(timeline) {
            continue;
        }
        let r = ui.interact(
            rect.intersect(timeline),
            ui.id().with(("voice", i)),
            egui::Sense::click_and_drag(),
        );
        let p = painter.with_clip_rect(timeline);
        p.rect_filled(rect, 3.0, theme.accent_c32().gamma_multiply(0.45));
        for n in &part.notes {
            let nx = rect.left() + n.midi_key() as f32 / 127.0 * rect.width();
            let y = rect.top() + n.position_ms as f32 * *zoom;
            p.line_segment(
                [
                    Pos2::new(nx, y),
                    Pos2::new(nx, y + n.duration_ms as f32 * *zoom),
                ],
                Stroke::new(1.5_f32, theme.text_primary_c32()),
            );
        }
        p.text(
            rect.left_top(),
            egui::Align2::LEFT_TOP,
            &part.name,
            egui::FontId::proportional(10.0),
            theme.text_primary_c32(),
        );
        if r.clicked() {
            *active = part.track_index;
        }
        if r.dragged() {
            part.position_ms = move_clip_by_frame_delta(
                part.position_ms,
                ui.input(|i| i.pointer.delta().y),
                *zoom,
            );
            changed = true;
        }
        r.context_menu(|ui| {
            if ui.button("Duplicar").clicked() {
                duplicate_voice = Some(i);
                ui.close_menu();
            }
            if ui.button("Dividir no cursor").clicked() {
                split_voice = Some(i);
                ui.close_menu();
            }
            if ui.button("Excluir").clicked() {
                delete_voice = Some(i);
                ui.close_menu();
            }
        });
    }
    if let Some(i) = duplicate_voice {
        let mut p = parts[i].clone();
        p.position_ms += p
            .notes
            .iter()
            .map(|n| n.position_ms + n.duration_ms)
            .fold(500.0, f64::max);
        parts.push(p);
        changed = true;
    }
    if let Some(i) = split_voice {
        let rel = *playhead - parts[i].position_ms;
        if rel > 0.0 {
            let mut right = parts[i].clone();
            right.position_ms = *playhead;
            right.notes.clear();
            parts[i].notes.retain_mut(|n| {
                if n.position_ms >= rel {
                    let mut n = n.clone();
                    n.position_ms -= rel;
                    right.notes.push(n);
                    false
                } else {
                    true
                }
            });
            if !right.notes.is_empty() {
                parts.push(right);
                changed = true;
            }
        }
    }
    if let Some(i) = delete_voice {
        parts.remove(i);
        changed = true;
    }
    let mut delete_wave = None;
    let mut duplicate_wave = None;
    let mut split_wave = None;
    for (i, w) in waves.iter_mut().enumerate() {
        let width = widths.get(w.track_index).copied().unwrap_or(default_width);
        let rect = Rect::from_min_size(
            Pos2::new(
                timeline.left() + offsets.get(w.track_index).copied().unwrap_or(0.0) - *scroll_x
                    + 3.0,
                timeline.top() + w.position_ms as f32 * *zoom - *scroll_y,
            ),
            Vec2::new(width - 6.0, (w.duration_ms as f32 * *zoom).max(8.0)),
        );
        if !rect.intersects(timeline) {
            continue;
        }
        let r = ui.interact(
            rect.intersect(timeline),
            ui.id().with(("wave", i)),
            egui::Sense::click_and_drag(),
        );
        let p = painter.with_clip_rect(timeline);
        p.rect_filled(rect, 3.0, theme.accent_c32().gamma_multiply(0.3));
        if let Some(preview) = cached_waveform(&w.file_path) {
            for y in (timeline.top() - rect.top()).max(0.0) as usize
                ..rect.height().min(timeline.bottom() - rect.top()).max(0.0) as usize
            {
                let source_ms = w.file_offset_ms + y as f64 / *zoom as f64;
                if let Some((lo, hi)) = preview.peaks.get((source_ms / preview.bucket_ms) as usize)
                {
                    p.line_segment(
                        [
                            Pos2::new(
                                rect.center().x + lo * rect.width() * 0.45,
                                rect.top() + y as f32,
                            ),
                            Pos2::new(
                                rect.center().x + hi * rect.width() * 0.45,
                                rect.top() + y as f32,
                            ),
                        ],
                        Stroke::new(1.0_f32, theme.text_primary_c32()),
                    );
                }
            }
        } else {
            p.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                lang.tr("Waveform indisponível", "Waveform unavailable"),
                egui::FontId::proportional(10.0),
                theme.text_muted_c32(),
            );
        }
        if r.clicked() {
            *active = w.track_index;
        }
        if r.dragged() {
            w.position_ms =
                move_clip_by_frame_delta(w.position_ms, ui.input(|i| i.pointer.delta().y), *zoom);
            changed = true;
        }
        r.context_menu(|ui| {
            if ui.button("Duplicar").clicked() {
                duplicate_wave = Some(i);
                ui.close_menu();
            }
            if ui.button("Dividir no cursor").clicked() {
                split_wave = Some(i);
                ui.close_menu();
            }
            if ui.button("Excluir").clicked() {
                delete_wave = Some(i);
                ui.close_menu();
            }
        });
    }
    if let Some(i) = duplicate_wave {
        let mut w = waves[i].clone();
        w.position_ms += w.duration_ms;
        waves.push(w);
        changed = true;
    }
    if let Some(i) = split_wave {
        let rel = *playhead - waves[i].position_ms;
        if rel > 0.0 && rel < waves[i].duration_ms {
            let mut w = waves[i].clone();
            w.position_ms = *playhead;
            w.file_offset_ms += rel;
            w.duration_ms -= rel;
            waves[i].duration_ms = rel;
            waves.push(w);
            changed = true;
        }
    }
    if let Some(i) = delete_wave {
        waves.remove(i);
        changed = true;
    }
    egui::ScrollArea::vertical()
        .id_salt("arrangement_lower_controls")
        .max_height(lower_height)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            let (controls_area, _) = ui
                .allocate_exact_size(Vec2::new(ui.available_width(), 166.0), egui::Sense::hover());
            let any_solo = tracks.iter().any(|track| track.solo);
            for (i, track) in tracks.iter_mut().enumerate() {
                let width = widths[i];
                let x = timeline.left() + offsets[i] - *scroll_x;
                if x + width < timeline.left() || x > area.right() {
                    continue;
                }
                let controls =
                    Rect::from_min_size(Pos2::new(x, controls_area.top()), Vec2::new(width, 166.0));
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .id_salt(("track_controls", i))
                        .max_rect(controls)
                        .layout(egui::Layout::top_down(egui::Align::Min)),
                );
                child.set_clip_rect(controls.intersect(controls_area).intersect(ui.clip_rect()));
                child.spacing_mut().slider_width = width - 16.0;
                child.horizontal(|ui| {
                    changed |= ui
                        .toggle_value(&mut track.mute, "M")
                        .on_hover_text(lang.tr("Mudo", "Mute"))
                        .changed();
                    changed |= ui
                        .toggle_value(&mut track.solo, "S")
                        .on_hover_text("Solo")
                        .changed();
                });
                child.horizontal(|ui| {
                    ui.small(lang.tr("Volume", "Volume"));
                    ui.small(format!("{:.1} dB", track.volume_db).replace('-', "−").replace('.', ","));
                });
                changed |= child
                    .add(egui::Slider::new(&mut track.volume_db, -60.0..=12.0).show_value(false))
                    .on_hover_text(format!("{:.1} dB", track.volume_db))
                    .changed();
                child.small(lang.tr("Panorama", "Pan"));
                changed |= child
                    .add(egui::Slider::new(&mut track.pan, -1.0..=1.0).show_value(false))
                    .on_hover_text(format!("Pan {:.2}", track.pan))
                    .changed();
                let audible = !track.mute && (!any_solo || track.solo);
                let pan = track.pan.clamp(-1.0, 1.0) as f32;
                let pan_gain = if pan < 0.0 { (1.0 + pan).sqrt() } else { (1.0 - pan).sqrt() };
                let level = if audible {
                    waves
                        .iter()
                        .filter(|w| w.track_index == i)
                        .filter_map(|w| audio_part_level(w, track, *playhead))
                        .fold(0.0, f32::max) * pan_gain
                } else {
                    0.0
                };
                let meter = &mut meters[i];
                if playing {
                    meter.level = meter.level.max(level);
                    meter.peak = meter.peak.max(meter.level);
                }
                let level = if playing { meter.level } else { 0.0 };
                child.small(lang.tr("Nível", "Level"));
                let bar = child.add(egui::ProgressBar::new(level).desired_width(width - 8.0));
                let peak_x = bar.rect.left() + meter.peak.clamp(0.0, 1.0) * bar.rect.width();
                child.painter().line_segment(
                    [
                        Pos2::new(peak_x, bar.rect.top()),
                        Pos2::new(peak_x, bar.rect.bottom()),
                    ],
                    Stroke::new(1.0_f32, theme.text_primary_c32()),
                );
            }
            egui::CollapsingHeader::new("EQ global")
                .default_open(true)
                .show(ui, |ui| {
                    changed |= draw_eq(ui, theme, master);
                });
        });
    if let Some(i) = new_part {
        let mut p = UVoicePart::new("Parte vocal", i);
        p.position_ms = *playhead;
        parts.push(p);
        changed = true;
    }
    if let Some(i) = duplicate_track {
        let target = tracks.len();
        tracks.push(tracks[i].clone());
        widths.push(widths[i]);
        let vp: Vec<_> = parts
            .iter()
            .filter(|p| p.track_index == i)
            .cloned()
            .map(|mut p| {
                p.track_index = target;
                p
            })
            .collect();
        parts.extend(vp);
        let wp: Vec<_> = waves
            .iter()
            .filter(|p| p.track_index == i)
            .cloned()
            .map(|mut p| {
                p.track_index = target;
                p
            })
            .collect();
        waves.extend(wp);
        changed = true;
    }
    if let Some(i) = delete_track {
        tracks.remove(i);
        widths.remove(i);
        parts.retain(|p| p.track_index != i);
        waves.retain(|p| p.track_index != i);
        for p in parts {
            if p.track_index > i {
                p.track_index -= 1;
            }
        }
        for w in waves {
            if w.track_index > i {
                w.track_index -= 1;
            }
        }
        *active = active
            .saturating_sub(usize::from(*active >= i))
            .min(tracks.len().saturating_sub(1));
        changed = true;
    } else if let Some((from, to)) = reorder {
        if move_track_and_remap_parts(tracks, parts, waves, meters, active, from, to) {
            let width = widths.remove(from);
            widths.insert(to, width);
            changed = true;
        }
    }
    if ui.input(|i| i.pointer.any_released()) {
        *drag = None;
    }
    ui.ctx().data_mut(|d| d.insert_persisted(widths_id, widths));
    let y = timeline.top() + *playhead as f32 * *zoom - *scroll_y;
    painter.with_clip_rect(timeline).line_segment(
        [
            Pos2::new(timeline.left(), y),
            Pos2::new(timeline.right(), y),
        ],
        Stroke::new(2.0_f32, theme.playhead_c32()),
    );
    changed
}

pub fn draw_eq(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    rack: &mut crate::audio::FxRackConfig,
) -> bool {
    let before = rack.clone();
    ui.horizontal(|ui| {
        ui.checkbox(&mut rack.master_enabled, "Master");
        ui.checkbox(&mut rack.eq.enabled, "EQ ativo");
        if ui.small_button("Reset").clicked() {
            rack.eq.gains = [0.0; 31];
        }
    });
    ui.columns(4, |columns| {
        for (column, (label, range)) in [
            ("Graves", 0..9),
            ("Médios graves", 9..16),
            ("Médios agudos", 16..22),
            ("Agudos", 22..31),
        ]
        .into_iter()
        .enumerate()
        {
            columns[column].vertical(|ui| {
                let average = rack.eq.gains[range.clone()].iter().sum::<f32>() / range.len() as f32;
                let (rect, r) =
                    ui.allocate_exact_size(Vec2::splat(42.0), egui::Sense::click_and_drag());
                let mut target = average;
                if r.dragged() {
                    target =
                        (average - ui.input(|i| i.pointer.delta().y) * 0.15).clamp(-12.0, 12.0);
                }
                if r.double_clicked() {
                    for g in &mut rack.eq.gains[range.clone()] {
                        *g = 0.0;
                    }
                } else if r.dragged() {
                    let delta = target - average;
                    for g in &mut rack.eq.gains[range.clone()] {
                        *g = (*g + delta).clamp(-12.0, 12.0);
                    }
                }
                let angle = std::f32::consts::PI * (0.75 + (average + 12.0) / 24.0 * 1.5);
                ui.painter().circle_stroke(
                    rect.center(),
                    17.0,
                    Stroke::new(2.0_f32, theme.text_muted_c32()),
                );
                ui.painter().line_segment(
                    [rect.center(), rect.center() + Vec2::angled(angle) * 15.0],
                    Stroke::new(2.0_f32, theme.accent_c32()),
                );
                r.on_hover_text(format!("{label}: {average:+.1} dB"));
                ui.add(egui::Label::new(egui::RichText::new(label).small()).wrap());
            });
        }
    });
    let (outer, r) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 110.0),
        egui::Sense::click_and_drag(),
    );
    let rect = Rect::from_min_max(
        outer.min + Vec2::new(27.0, 6.0),
        outer.max - Vec2::new(2.0, 16.0),
    );
    let freqs = crate::audio::ISO_31_BAND_FREQS;
    let x = |f: f32| rect.left() + (f / 20.0).ln() / 1000.0f32.ln() * rect.width();
    for (freq, label) in [
        (20.0, "20 Hz"),
        (100.0, "100"),
        (1000.0, "1k"),
        (5000.0, "5k"),
        (20000.0, "20k"),
    ] {
        ui.painter().text(
            Pos2::new(
                x(freq).clamp(rect.left() + 18.0, rect.right() - 14.0),
                outer.bottom(),
            ),
            egui::Align2::CENTER_BOTTOM,
            label,
            egui::FontId::monospace(8.0),
            theme.text_muted_c32(),
        );
    }
    for gain in [-12.0, 0.0, 12.0] {
        let y = rect.center().y - gain / 24.0 * rect.height();
        ui.painter().text(
            Pos2::new(outer.left(), y.clamp(rect.top() + 6.0, rect.bottom() - 6.0)),
            egui::Align2::LEFT_CENTER,
            format!("{gain:+.0} dB"),
            egui::FontId::monospace(8.0),
            theme.text_muted_c32(),
        );
        ui.painter().line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(0.5_f32, theme.grid_line_sub_c32()),
        );
    }
    if let Some(p) = r.hover_pos().or(r.interact_pointer_pos()) {
        let f = 20.0 * 1000.0f32.powf(((p.x - rect.left()) / rect.width()).clamp(0.0, 1.0));
        let idx = (0..31)
            .min_by(|&a, &b| {
                (freqs[a] / f)
                    .ln()
                    .abs()
                    .total_cmp(&(freqs[b] / f).ln().abs())
            })
            .unwrap();
        if r.double_clicked() {
            rack.eq.gains[idx] = 0.0;
        } else if r.dragged() || r.clicked() {
            let previous = if r.dragged() {
                p.x - ui.input(|i| i.pointer.delta().x)
            } else {
                p.x
            };
            let lo = previous.min(p.x);
            let hi = previous.max(p.x);
            let gain = ((rect.center().y - p.y) / rect.height() * 24.0).clamp(-12.0, 12.0);
            for i in 0..31 {
                if i == idx || (x(freqs[i]) >= lo && x(freqs[i]) <= hi) {
                    rack.eq.gains[i] = gain;
                }
            }
        }
        r.on_hover_text(format!("{} Hz · {:+.1} dB", freqs[idx], rack.eq.gains[idx]));
    }
    let points = (0..31)
        .map(|i| {
            Pos2::new(
                x(freqs[i]),
                rect.center().y - rack.eq.gains[i] / 24.0 * rect.height(),
            )
        })
        .collect();
    ui.painter().add(egui::Shape::line(
        points,
        Stroke::new(2.0_f32, theme.accent_c32()),
    ));
    *rack != before
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn narrow_and_short_layouts_keep_the_timeline_and_do_not_edit_eq_when_idle() {
        for preset in [
            crate::gui::theme::ThemePreset::CleanLight,
            crate::gui::theme::ThemePreset::NordicSlate,
        ] {
            for count in [1, 4, 24] {
                for size in [
                    Vec2::new(280.0, 400.0),
                    Vec2::new(360.0, 700.0),
                    Vec2::new(600.0, 500.0),
                ] {
                    let ctx = egui::Context::default();
                    let mut tracks = vec![UTrack::default(); count];
                    tracks[0].name =
                        "Um nome muito longo para testar reticências e alinhamento".into();
                    let mut parts = vec![UVoicePart::new("Voz", 0)];
                    parts[0]
                        .notes
                        .push(crate::project::model::UNote::new("a", "C4", 100.0, 900.0));
                    let mut waves = vec![];
                    let mut active = 0;
                    let mut playhead = 0.0;
                    let mut zoom = 0.05;
                    let mut x = 0.0;
                    let mut y = 0.0;
                    let mut manual = false;
                    let mut drag = None;
                    let mut meters = vec![];
                    let mut fx = crate::gui::fx_rack_dialog::FxRackDialogState::default();
                    let mut master = crate::audio::FxRackConfig::default();
                    master.eq.gains = std::array::from_fn(|i| i as f32 * 0.173 - 3.1);
                    let original = master.clone();
                    for _ in 0..3 {
                        let output = ctx.run(
                            egui::RawInput {
                                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                                ..Default::default()
                            },
                            |ctx| {
                                egui::CentralPanel::default().show(ctx, |ui| {
                                    assert!(!draw_arrangement_view(
                                        ui,
                                        &ThemeConfig::from_preset(preset),
                                        &mut tracks,
                                        &mut parts,
                                        &mut waves,
                                        &mut active,
                                        &mut playhead,
                                        &mut zoom,
                                        120.0,
                                        &mut x,
                                        &mut y,
                                        &mut manual,
                                        &mut drag,
                                        &mut meters,
                                        false,
                                        &mut fx,
                                        crate::config::AppLanguage::PtBr,
                                        &mut master
                                    ));
                                    assert!(
                                        ui.min_rect().height() <= size.y,
                                        "layout overflow: {:?}",
                                        size
                                    );
                                });
                            },
                        );
                        assert!(!output.shapes.is_empty());
                    }
                    assert_eq!(master, original);
                }
            }
        }
    }
}

#[cfg(test)]
mod creation_tests {
    use super::*;
    #[test]
    fn clicking_new_track_keeps_meters_in_bounds_in_the_same_frame() {
        let ctx = egui::Context::default();
        let mut tracks = vec![UTrack::default()];
        let mut parts = vec![];
        let mut waves = vec![];
        let mut active = 0;
        let mut playhead = 0.0;
        let mut zoom = 0.05;
        let mut x = 0.0;
        let mut y = 0.0;
        let mut manual = false;
        let mut drag = None;
        let mut meters = vec![];
        let mut fx = crate::gui::fx_rack_dialog::FxRackDialogState::default();
        let mut master = crate::audio::FxRackConfig::default();
        let pos = Pos2::new(71.0, 17.0);
        for events in [
            vec![],
            vec![egui::Event::PointerMoved(pos)],
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            }],
            vec![egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            }],
        ] {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(360.0, 700.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        draw_arrangement_view(
                            ui,
                            &ThemeConfig::default(),
                            &mut tracks,
                            &mut parts,
                            &mut waves,
                            &mut active,
                            &mut playhead,
                            &mut zoom,
                            120.0,
                            &mut x,
                            &mut y,
                            &mut manual,
                            &mut drag,
                            &mut meters,
                            false,
                            &mut fx,
                            crate::config::AppLanguage::PtBr,
                            &mut master,
                        );
                    });
                },
            );
        }
        assert_eq!(tracks.len(), 2, "test must actually click the add button");
        assert_eq!(meters.len(), tracks.len());
        assert_eq!(parts.len(), 1);
        assert_eq!(active, 1);
    }
}

#[cfg(test)]
mod resize_tests {
    use super::*;
    #[test]
    fn dragging_column_edge_resizes_only_that_column_and_keeps_the_width() {
        let ctx = egui::Context::default();
        let mut tracks = vec![UTrack::default(); 4];
        let mut parts = vec![];
        let mut waves = vec![];
        let mut active = 0;
        let mut playhead = 0.0;
        let mut zoom = 0.05;
        let mut x = 0.0;
        let mut y = 0.0;
        let mut manual = false;
        let mut drag = None;
        let mut meters = vec![];
        let mut fx = crate::gui::fx_rack_dialog::FxRackDialogState::default();
        let mut master = crate::audio::FxRackConfig::default();
        let mut draw = |events| {
            let mut edge = Rect::NOTHING;
            let mut widths = Vec::<f32>::new();
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(360.0, 700.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let id = ui.id();
                        assert!(!draw_arrangement_view(
                            ui,
                            &ThemeConfig::default(),
                            &mut tracks,
                            &mut parts,
                            &mut waves,
                            &mut active,
                            &mut playhead,
                            &mut zoom,
                            120.0,
                            &mut x,
                            &mut y,
                            &mut manual,
                            &mut drag,
                            &mut meters,
                            false,
                            &mut fx,
                            crate::config::AppLanguage::PtBr,
                            &mut master
                        ));
                        edge = ctx
                            .read_response(id.with(("resize_track", 0usize)))
                            .unwrap()
                            .rect;
                        widths = ctx.data_mut(|d| {
                            d.get_persisted::<Vec<f32>>(id.with("track_column_widths"))
                                .unwrap()
                        });
                    });
                },
            );
            (edge, widths)
        };
        let (edge, initial) = draw(vec![]);
        let start = edge.center();
        draw(vec![
            egui::Event::PointerMoved(start),
            egui::Event::PointerButton {
                pos: start,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        let moved = start + Vec2::new(35.0, 0.0);
        draw(vec![egui::Event::PointerMoved(moved)]);
        let (_, resized) = draw(vec![egui::Event::PointerButton {
            pos: moved,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }]);
        let (_, held) = draw(vec![]);
        assert!((resized[0] - initial[0] - 35.0).abs() < 0.01);
        assert_eq!(resized[1..], initial[1..]);
        assert_eq!(held, resized);
    }
}
