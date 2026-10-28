use super::*;
use eframe::egui::{Color32, RichText, Rounding};

const DAW_TRACK_PALETTE: &[[u8; 3]] = &[
    [70, 160, 225],  // Azul estúdio
    [85, 190, 115],  // Verde esmeralda
    [225, 125, 55],  // Âmbar / Laranja
    [170, 110, 215], // Púrpura / Violeta
    [230, 90, 130],  // Rosa coral
    [45, 185, 175],  // Turquesa / Ciano
    [220, 185, 65],  // Ouro / Amarelo
    [225, 60, 75],   // Vermelho / Escarlate
];

fn track_accent_color(index: usize, theme: &ThemeConfig) -> Color32 {
    let palette = [
        theme.accent_c32(),
        Color32::from_rgb(70, 160, 225),  // Azul estúdio
        Color32::from_rgb(225, 125, 55),  // Âmbar / Laranja
        Color32::from_rgb(85, 190, 115),  // Verde sálvia
        Color32::from_rgb(170, 110, 215), // Púrpura
        Color32::from_rgb(230, 90, 130),  // Rosa coral
        Color32::from_rgb(45, 185, 175),  // Turquesa
        Color32::from_rgb(220, 185, 65),  // Ouro
    ];
    palette[index % palette.len()]
}

fn track_color(track: &UTrack, index: usize, theme: &ThemeConfig) -> Color32 {
    if let Some([r, g, b]) = track.color_rgb {
        Color32::from_rgb(r, g, b)
    } else {
        track_accent_color(index, theme)
    }
}

fn format_time_sec_min(ms: f64) -> String {
    let total_secs = (ms / 1000.0).max(0.0);
    if total_secs < 60.0 {
        format!("{:.1}s", total_secs)
    } else {
        let mins = (total_secs / 60.0).floor() as u32;
        let secs = total_secs % 60.0;
        format!("{:02}:{:04.1}", mins, secs)
    }
}

fn draw_color_palette(ui: &mut egui::Ui, current: &mut Option<[u8; 3]>) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        for &color in DAW_TRACK_PALETTE {
            let c32 = Color32::from_rgb(color[0], color[1], color[2]);
            let (rect, resp) = ui.allocate_exact_size(Vec2::splat(16.0), egui::Sense::click());
            ui.painter().rect_filled(rect, Rounding::same(3.0), c32);
            let is_cur = *current == Some(color);
            if is_cur {
                ui.painter().rect_stroke(
                    rect.expand(1.5),
                    Rounding::same(4.0),
                    Stroke::new(1.5_f32, Color32::WHITE),
                );
            } else if resp.hovered() {
                ui.painter().rect_stroke(
                    rect.expand(1.0),
                    Rounding::same(4.0),
                    Stroke::new(1.0_f32, Color32::from_white_alpha(200)),
                );
            }
            if resp.clicked() {
                *current = Some(color);
                changed = true;
            }
        }
    });
    if current.is_some() && ui.button("Restaurar cor padrão").clicked() {
        *current = None;
        changed = true;
    }
    changed
}

fn draw_track_mute_btn(ui: &mut egui::Ui, mute: &mut bool, theme: &ThemeConfig) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(26.0, 20.0), egui::Sense::click());
    if response.clicked() {
        *mute = !*mute;
    }
    let is_hovered = response.hovered();
    let (bg, text_color, stroke) = if *mute {
        (
            Color32::from_rgb(215, 55, 55),
            Color32::WHITE,
            Stroke::new(1.0_f32, Color32::from_rgb(245, 110, 110)),
        )
    } else if is_hovered {
        (
            theme.bg_canvas_c32().linear_multiply(1.3),
            theme.text_primary_c32(),
            Stroke::new(1.0_f32, theme.grid_line_sub_c32()),
        )
    } else {
        (
            theme.bg_canvas_c32(),
            theme.text_muted_c32(),
            Stroke::new(1.0_f32, theme.grid_line_sub_c32().gamma_multiply(0.6)),
        )
    };
    ui.painter().rect_filled(rect, Rounding::same(3.0), bg);
    ui.painter().rect_stroke(rect, Rounding::same(3.0), stroke);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "M",
        egui::FontId::proportional(11.0),
        text_color,
    );
    response.clicked()
}

fn draw_track_solo_btn(ui: &mut egui::Ui, solo: &mut bool, theme: &ThemeConfig) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(26.0, 20.0), egui::Sense::click());
    if response.clicked() {
        *solo = !*solo;
    }
    let is_hovered = response.hovered();
    let (bg, text_color, stroke) = if *solo {
        (
            Color32::from_rgb(235, 185, 45),
            Color32::from_rgb(20, 20, 20),
            Stroke::new(1.0_f32, Color32::from_rgb(255, 220, 90)),
        )
    } else if is_hovered {
        (
            theme.bg_canvas_c32().linear_multiply(1.3),
            theme.text_primary_c32(),
            Stroke::new(1.0_f32, theme.grid_line_sub_c32()),
        )
    } else {
        (
            theme.bg_canvas_c32(),
            theme.text_muted_c32(),
            Stroke::new(1.0_f32, theme.grid_line_sub_c32().gamma_multiply(0.6)),
        )
    };
    ui.painter().rect_filled(rect, Rounding::same(3.0), bg);
    ui.painter().rect_stroke(rect, Rounding::same(3.0), stroke);
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "S",
        egui::FontId::proportional(11.0),
        text_color,
    );
    response.clicked()
}

fn draw_vu_meter_led(
    ui: &mut egui::Ui,
    level: f32,
    peak: f32,
    width: f32,
    theme: &ThemeConfig,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width.max(20.0), 10.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(2.0), theme.bg_canvas_c32());
    painter.rect_stroke(
        rect,
        Rounding::same(2.0),
        Stroke::new(0.5_f32, theme.grid_line_sub_c32().gamma_multiply(0.6)),
    );

    let inner = rect.shrink(1.0);
    let level_clamp = level.clamp(0.0, 1.0);
    if level_clamp > 0.005 {
        let fill_w = inner.width() * level_clamp;
        let seg_rect = Rect::from_min_size(inner.min, Vec2::new(fill_w, inner.height()));
        let color = if level_clamp < 0.70 {
            Color32::from_rgb(90, 195, 110)
        } else if level_clamp < 0.90 {
            Color32::from_rgb(230, 190, 50)
        } else {
            Color32::from_rgb(235, 70, 70)
        };
        painter.rect_filled(seg_rect, Rounding::same(1.5), color);
    }

    let peak_clamp = peak.clamp(0.0, 1.0);
    if peak_clamp > 0.01 {
        let peak_x = (inner.left() + inner.width() * peak_clamp).clamp(inner.left(), inner.right() - 1.5);
        let peak_color = if peak_clamp >= 0.95 {
            Color32::from_rgb(255, 60, 60)
        } else {
            Color32::from_rgb(240, 240, 240)
        };
        painter.line_segment(
            [Pos2::new(peak_x, inner.top()), Pos2::new(peak_x, inner.bottom())],
            Stroke::new(1.5_f32, peak_color),
        );
    }
}

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
    available_singers: &[String],
) -> bool {
    let mut changed = false;
    meters.resize(tracks.len(), ArrangementMeterState::default());
    ui.horizontal(|ui| {
        ui.heading(
            RichText::new(lang.tr("Arranjo", "Arrangement"))
                .size(13.0)
                .strong()
                .color(theme.text_primary_c32()),
        );

        let badge_text = if tracks.len() == 1 {
            lang.tr("1 faixa", "1 track").to_string()
        } else {
            format!("{} {}", tracks.len(), lang.tr("faixas", "tracks"))
        };
        let (badge_rect, _) = ui.allocate_exact_size(
            Vec2::new(badge_text.len() as f32 * 6.0 + 12.0, 16.0),
            egui::Sense::hover(),
        );
        ui.painter().rect_filled(
            badge_rect,
            Rounding::same(8.0),
            theme.bg_canvas_c32(),
        );
        ui.painter().text(
            badge_rect.center(),
            egui::Align2::CENTER_CENTER,
            badge_text,
            egui::FontId::proportional(9.0),
            theme.text_muted_c32(),
        );

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let follow = !*manual;
            let follow_btn = ui.selectable_label(
                follow,
                RichText::new(lang.tr("Seguir", "Follow")).size(10.0),
            );
            if follow_btn.clicked() {
                *manual = follow;
            }

            if ui
                .button(RichText::new(lang.tr("FX Master", "Master FX")).size(10.0))
                .on_hover_text(lang.tr("Abrir rack de efeitos do master", "Open master effects rack"))
                .clicked()
            {
                fx.target_track = None;
                fx.is_open = true;
            }

            if ui
                .button(RichText::new("+ Faixa").size(10.0).strong())
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
                part.duration_ms = 30_000.0;
                parts.push(part);
                changed = true;
            }
        });
    });
    meters.resize(tracks.len(), ArrangementMeterState::default());
    let widths_id = ui.id().with("track_column_widths");
    let default_width =
        ((ui.available_width() - 32.0) / tracks.len().max(1) as f32).clamp(72.0, 160.0);
    let mut widths = ui
        .ctx()
        .data_mut(|d| d.get_persisted::<Vec<f32>>(widths_id).unwrap_or_default());
    widths.resize(tracks.len(), default_width);
    let mut common_width = widths.first().copied().unwrap_or(default_width);
    ui.horizontal(|ui| {
        ui.label(RichText::new(lang.tr("Largura", "Width")).size(9.5).color(theme.text_muted_c32()));
        ui.spacing_mut().slider_width = 75.0;
        if ui
            .add(egui::Slider::new(&mut common_width, 48.0..=600.0).suffix("px").show_value(false))
            .changed()
        {
            widths.fill(common_width);
        }
        ui.label(RichText::new(format!("{:.0}px", common_width)).size(9.0).monospace().color(theme.text_muted_c32()));
        if ui
            .small_button(lang.tr("Padrão", "Reset"))
            .on_hover_text(lang.tr(
                "Redefinir larguras das faixas para o padrão",
                "Reset track widths to default",
            ))
            .clicked()
        {
            widths.fill(default_width);
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
    let timeline_height_ratio_id = ui.id().with("arrangement_timeline_height_ratio");
    let mut timeline_ratio = ui
        .ctx()
        .data_mut(|d| d.get_persisted::<f32>(timeline_height_ratio_id).unwrap_or(0.55));
    let available_h = ui.available_height();
    let lower_height = (available_h * (1.0 - timeline_ratio)).clamp(90.0, (available_h - 110.0).max(90.0));
    let height = (available_h - lower_height - 78.0).max(60.0);
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
    let mut new_part: Option<(usize, f64)> = None;
    let mut delete_track = None;
    let mut duplicate_track = None;
    let mut reorder = None;
    let track_count = tracks.len();
    for (i, track) in tracks.iter_mut().enumerate() {
        let width = widths[i];
        let x = timeline.left() + offsets[i] - *scroll_x;
        if x + width < timeline.left() || x > area.right() {
            continue;
        }
        let track_color = track_color(track, i, theme);
        let header = Rect::from_min_size(Pos2::new(x, area.top()), Vec2::new(width, 64.0));
        let header_inner = header.shrink(1.5);
        let is_selected = *active == i;

        let active_t = ui
            .ctx()
            .animate_bool(ui.id().with(("track_active_animation", i)), is_selected);
        let hover_t = ui.ctx().animate_bool(
            ui.id().with(("track_hover_animation", i)),
            header.contains(ui.input(|input| input.pointer.hover_pos().unwrap_or(Pos2::ZERO))),
        );
        if active_t > 0.001 || hover_t > 0.001 {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(16));
        }

        // Fundo do card do cabeçalho da faixa
        let card_bg = if is_selected {
            theme.card_bg_c32().linear_multiply(1.18)
        } else {
            theme.card_bg_c32()
        };
        painter.with_clip_rect(header.intersect(area)).rect_filled(
            header_inner,
            Rounding::same(4.0),
            card_bg,
        );
        let card_stroke = if is_selected {
            Stroke::new(1.5_f32, track_color)
        } else if hover_t > 0.001 {
            Stroke::new(1.0_f32, theme.grid_line_bar_c32())
        } else {
            Stroke::new(1.0_f32, theme.grid_line_sub_c32().gamma_multiply(0.45))
        };
        painter.with_clip_rect(header.intersect(area)).rect_stroke(
            header_inner,
            Rounding::same(4.0),
            card_stroke,
        );

        // Tarja colorida de identificação da faixa à esquerda
        let stripe_rect = Rect::from_min_size(
            header_inner.min,
            Vec2::new(3.5, header_inner.height()),
        );
        painter.with_clip_rect(header.intersect(area)).rect_filled(
            stripe_rect,
            Rounding { nw: 4.0, sw: 4.0, ne: 0.0, se: 0.0 },
            track_color,
        );

        let content_rect = Rect::from_min_max(
            header_inner.min + Vec2::new(6.0, 3.0),
            header_inner.max - Vec2::new(4.0, 3.0),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .id_salt(("track_header", i))
                .max_rect(content_rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
        );
        child.set_clip_rect(content_rect.intersect(area));
        let rename_id = ui.id().with(("rename", i));
        let renaming = ui
            .ctx()
            .data_mut(|d| d.get_temp::<bool>(rename_id).unwrap_or(false));
        if renaming {
            let r = child.add_sized(
                [content_rect.width(), 18.0],
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
                    egui::Label::new(
                        egui::RichText::new(&track.name)
                            .strong()
                            .size(11.0)
                            .color(theme.text_primary_c32()),
                    )
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
                ui.label(
                    RichText::new(&track.name)
                        .strong()
                        .color(theme.text_primary_c32()),
                );
                ui.separator();
                if ui.button(lang.tr("Renomear faixa", "Rename track")).clicked() {
                    ui.ctx().data_mut(|d| d.insert_temp(rename_id, true));
                    ui.close_menu();
                }
                if !available_singers.is_empty() {
                    ui.menu_button(lang.tr("Mudar cantor", "Change singer"), |ui| {
                        for s in available_singers {
                            let is_curr = &track.singer == s;
                            if ui.selectable_label(is_curr, s).clicked() {
                                track.singer = s.clone();
                                changed = true;
                                ui.close_menu();
                            }
                        }
                    });
                }
                ui.menu_button(lang.tr("Cor da faixa", "Track color"), |ui| {
                    if draw_color_palette(ui, &mut track.color_rgb) {
                        changed = true;
                        ui.close_menu();
                    }
                });
                ui.separator();
                if ui
                    .button(lang.tr("Nova parte vocal (30s)", "New vocal part (30s)"))
                    .clicked()
                {
                    new_part = Some((i, 30_000.0));
                    ui.close_menu();
                }
                ui.menu_button(lang.tr("Nova parte (duração...)", "New part (duration...)"), |ui| {
                    for (label, dur_ms) in [
                        ("5 segundos", 5_000.0),
                        ("10 segundos", 10_000.0),
                        ("30 segundos", 30_000.0),
                        ("1 minuto", 60_000.0),
                        ("2 minutos", 120_000.0),
                        ("5 minutos", 300_000.0),
                    ] {
                        if ui.button(label).clicked() {
                            new_part = Some((i, dur_ms));
                            ui.close_menu();
                        }
                    }
                });
                ui.separator();
                if ui.button(lang.tr("Duplicar faixa", "Duplicate track")).clicked() {
                    duplicate_track = Some(i);
                    ui.close_menu();
                }
                if ui.button(lang.tr("Excluir faixa", "Delete track")).clicked() {
                    delete_track = Some(i);
                    ui.close_menu();
                }
            });
        }
        child
            .add(
                egui::Label::new(
                    egui::RichText::new(&track.singer)
                        .size(9.5)
                        .color(theme.text_muted_c32()),
                )
                .truncate(),
            )
            .on_hover_text(&track.singer);

        child.horizontal(|ui| {
            let has_fx = track.fx_rack.is_some();
            let (fx_bg, fx_fg) = if has_fx {
                (Color32::from_rgb(60, 140, 215), Color32::WHITE)
            } else {
                (theme.bg_canvas_c32(), theme.text_muted_c32())
            };
            let (fx_rect, fx_resp) = ui.allocate_exact_size(Vec2::new(24.0, 16.0), egui::Sense::click());
            ui.painter().rect_filled(fx_rect, Rounding::same(3.0), fx_bg);
            ui.painter().rect_stroke(fx_rect, Rounding::same(3.0), Stroke::new(0.5_f32, theme.grid_line_sub_c32()));
            ui.painter().text(fx_rect.center(), egui::Align2::CENTER_CENTER, "FX", egui::FontId::proportional(9.0), fx_fg);
            if fx_resp.clicked() {
                fx.target_track = Some(i);
                fx.is_open = true;
            }

            let r = ui
                .add(
                    egui::Label::new(
                        RichText::new("⠿")
                            .size(12.0)
                            .color(theme.text_muted_c32()),
                    )
                    .sense(egui::Sense::drag()),
                )
                .on_hover_cursor(egui::CursorIcon::Grab)
                .on_hover_text("Arrastar para reordenar / Drag to reorder");
            if r.drag_started() {
                *drag = Some(i);
            }
        });
        let edge = Rect::from_min_max(
            Pos2::new(header.right() - 4.0, area.top()),
            Pos2::new(header.right() + 4.0, timeline.bottom()),
        );
        let resize = ui
            .interact(
                edge.intersect(area),
                ui.id().with(("resize_track", i)),
                egui::Sense::click_and_drag(),
            )
            .on_hover_cursor(egui::CursorIcon::ResizeHorizontal)
            .on_hover_text(lang.tr(
                "Arraste para ajustar a largura da faixa (duplo clique para redefinir)",
                "Drag to resize track (double-click to reset)",
            ));
        let is_edge_active = resize.hovered() || resize.dragged();
        let edge_stroke = if is_edge_active {
            Stroke::new(2.0_f32, theme.accent_c32())
        } else {
            Stroke::new(1.0_f32, theme.grid_line_sub_c32())
        };
        painter.line_segment(
            [Pos2::new(header.right(), area.top()), Pos2::new(header.right(), timeline.bottom())],
            edge_stroke,
        );
        if resize.double_clicked() {
            widths[i] = default_width;
            ui.ctx().request_repaint();
        } else if resize.dragged() {
            widths[i] = (width + resize.drag_delta().x).clamp(48.0, 600.0);
            ui.ctx().request_repaint();
        }
        if let Some(from) = *drag {
            if from != i {
                let hover_pos = ui.input(|inp| inp.pointer.hover_pos()).unwrap_or(Pos2::ZERO);
                if header.contains(hover_pos) {
                    painter.with_clip_rect(header.intersect(area)).rect_filled(
                        header_inner,
                        Rounding::same(4.0),
                        theme.accent_c32().gamma_multiply(0.25),
                    );
                    let insert_x = if hover_pos.x < header.center().x {
                        header.left()
                    } else {
                        header.right()
                    };
                    painter.with_clip_rect(area).line_segment(
                        [Pos2::new(insert_x, area.top()), Pos2::new(insert_x, timeline.bottom())],
                        Stroke::new(3.0_f32, theme.accent_c32()),
                    );
                }
            }
        }
        if ui.input(|inp| inp.pointer.any_released()) {
            if let Some(hover_pos) = ui.input(|inp| inp.pointer.hover_pos()) {
                if header.contains(hover_pos) {
                    if let Some(from) = drag.take() {
                        let to = if hover_pos.x < header.center().x {
                            if from < i { i.saturating_sub(1) } else { i }
                        } else {
                            if from > i { (i + 1).min(track_count.saturating_sub(1)) } else { i }
                        };
                        if from != to {
                            reorder = Some((from, to));
                        }
                    }
                }
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
    if let Some(from) = *drag {
        if let Some(mpos) = ui.input(|inp| inp.pointer.hover_pos()) {
            let ghost_rect = Rect::from_center_size(mpos + Vec2::new(0.0, -20.0), Vec2::new(120.0, 26.0));
            painter.rect_filled(ghost_rect, Rounding::same(4.0), theme.card_bg_c32().linear_multiply(1.3));
            painter.rect_stroke(ghost_rect, Rounding::same(4.0), Stroke::new(1.5_f32, theme.accent_c32()));
            let tname = tracks.get(from).map(|t| t.name.as_str()).unwrap_or("Faixa");
            painter.text(
                ghost_rect.center(),
                egui::Align2::CENTER_CENTER,
                format!("⠿ {}", tname),
                egui::FontId::proportional(11.0),
                theme.text_primary_c32(),
            );
        }
    }
    let mut delete_voice = None;
    let mut duplicate_voice = None;
    let mut split_voice = None;
    for (i, part) in parts.iter_mut().enumerate() {
        let width = widths
            .get(part.track_index)
            .copied()
            .unwrap_or(default_width);
        let duration = part.effective_duration_ms();
        let rect = Rect::from_min_size(
            Pos2::new(
                timeline.left() + offsets.get(part.track_index).copied().unwrap_or(0.0) - *scroll_x
                    + 3.0,
                timeline.top() + part.position_ms as f32 * *zoom - *scroll_y,
            ),
            Vec2::new(width - 6.0, (duration as f32 * *zoom).max(18.0)),
        );
        if !rect.intersects(timeline) {
            continue;
        }

        let p = painter.with_clip_rect(timeline);
        let track_color = tracks
            .get(part.track_index)
            .map(|t| track_color(t, part.track_index, theme))
            .unwrap_or_else(|| track_accent_color(part.track_index, theme));
        let is_part_selected = *active == part.track_index;
        let clip_bg = if is_part_selected {
            track_color.gamma_multiply(0.38)
        } else {
            track_color.gamma_multiply(0.24)
        };
        p.rect_filled(rect, Rounding::same(4.0), clip_bg);
        p.rect_stroke(
            rect,
            Rounding::same(4.0),
            Stroke::new(
                1.0_f32,
                if is_part_selected {
                    track_color
                } else {
                    track_color.gamma_multiply(0.55)
                },
            ),
        );

        // Cabeçalho do clipe vocal
        let clip_hdr_rect = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 16.0));
        p.rect_filled(
            clip_hdr_rect,
            Rounding {
                nw: 4.0,
                ne: 4.0,
                sw: 0.0,
                se: 0.0,
            },
            track_color.gamma_multiply(0.55),
        );
        p.text(
            Pos2::new(clip_hdr_rect.left() + 4.0, clip_hdr_rect.center().y),
            egui::Align2::LEFT_CENTER,
            &part.name,
            egui::FontId::proportional(9.0),
            Color32::WHITE,
        );
        let dur_str = format_time_sec_min(duration);
        p.text(
            Pos2::new(clip_hdr_rect.right() - 4.0, clip_hdr_rect.center().y),
            egui::Align2::RIGHT_CENTER,
            dur_str,
            egui::FontId::monospace(8.5),
            Color32::from_white_alpha(220),
        );

        for n in &part.notes {
            let nx = rect.left()
                + 4.0
                + (n.midi_key() as f32 / 127.0) * (rect.width() - 8.0).max(1.0);
            let y = rect.top() + 17.0 + n.position_ms as f32 * *zoom;
            let note_len = (n.duration_ms as f32 * *zoom).max(2.0);
            p.line_segment(
                [Pos2::new(nx, y), Pos2::new(nx, y + note_len)],
                Stroke::new(2.5_f32, Color32::from_rgb(250, 248, 240)),
            );
        }

        // Alça inferior de redimensionamento de duração (bottom 7px)
        let bottom_handle_rect = Rect::from_min_max(
            Pos2::new(rect.left(), rect.bottom() - 7.0),
            rect.max,
        );
        let bottom_resp = ui
            .interact(
                bottom_handle_rect.intersect(timeline),
                ui.id().with(("voice_resize", i)),
                egui::Sense::click_and_drag(),
            )
            .on_hover_cursor(egui::CursorIcon::ResizeVertical)
            .on_hover_text(lang.tr(
                "Arraste para redimensionar duração (duplo clique para ajustar às notas)",
                "Drag to resize duration (double-click to fit to notes)",
            ));

        let grip_w = (rect.width() * 0.35).clamp(12.0, 40.0);
        let grip_y = rect.bottom() - 3.5;
        let grip_color = if bottom_resp.hovered() || bottom_resp.dragged() {
            theme.accent_c32()
        } else {
            track_color.gamma_multiply(0.75)
        };
        p.line_segment(
            [
                Pos2::new(rect.center().x - grip_w * 0.5, grip_y),
                Pos2::new(rect.center().x + grip_w * 0.5, grip_y),
            ],
            Stroke::new(2.0_f32, grip_color),
        );

        if bottom_resp.double_clicked() {
            part.duration_ms = 0.0;
            changed = true;
        } else if bottom_resp.dragged() {
            let delta_y = bottom_resp.drag_delta().y;
            let delta_ms = delta_y as f64 / *zoom as f64;
            let notes_end = part
                .notes
                .iter()
                .map(|n| n.position_ms + n.duration_ms)
                .fold(500.0, f64::max);
            part.duration_ms = (part.effective_duration_ms() + delta_ms)
                .max(notes_end)
                .max(500.0);
            changed = true;
        }

        // Corpo do clipe (área acima da alça inferior)
        let body_rect = Rect::from_min_max(
            rect.min,
            Pos2::new(rect.right(), (rect.bottom() - 7.0).max(rect.top())),
        );
        let r = ui.interact(
            body_rect.intersect(timeline),
            ui.id().with(("voice", i)),
            egui::Sense::click_and_drag(),
        );

        if r.clicked() {
            *active = part.track_index;
        }
        if r.dragged() {
            part.position_ms = move_clip_by_frame_delta(
                part.position_ms,
                ui.input(|inp| inp.pointer.delta().y),
                *zoom,
            );
            changed = true;
        }
        r.context_menu(|ui| {
            ui.label(
                RichText::new(&part.name)
                    .strong()
                    .color(theme.text_primary_c32()),
            );
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(lang.tr("Nome:", "Name:"));
                if ui.text_edit_singleline(&mut part.name).changed() {
                    changed = true;
                }
            });
            ui.menu_button(lang.tr("Duração da parte", "Part duration"), |ui| {
                if ui
                    .button(lang.tr("Ajustar às notas (Auto)", "Fit to notes (Auto)"))
                    .clicked()
                {
                    part.duration_ms = 0.0;
                    changed = true;
                    ui.close_menu();
                }
                for (label, dur_ms) in [
                    ("5 segundos", 5_000.0),
                    ("10 segundos", 10_000.0),
                    ("30 segundos", 30_000.0),
                    ("1 minuto", 60_000.0),
                    ("2 minutos", 120_000.0),
                    ("5 minutos", 300_000.0),
                ] {
                    let is_sel = (part.effective_duration_ms() - dur_ms).abs() < 50.0;
                    if ui.selectable_label(is_sel, label).clicked() {
                        part.duration_ms = dur_ms;
                        changed = true;
                        ui.close_menu();
                    }
                }
            });
            if tracks.len() > 1 {
                ui.menu_button(lang.tr("Mover para faixa", "Move to track"), |ui| {
                    for (t_idx, t) in tracks.iter().enumerate() {
                        let is_current = t_idx == part.track_index;
                        if ui
                            .selectable_label(is_current, format!("{}: {}", t_idx + 1, t.name))
                            .clicked()
                        {
                            part.track_index = t_idx;
                            changed = true;
                            ui.close_menu();
                        }
                    }
                });
            }
            if !available_singers.is_empty() {
                ui.menu_button(lang.tr("Mudar cantor da faixa", "Change track singer"), |ui| {
                    if let Some(t) = tracks.get_mut(part.track_index) {
                        for s in available_singers {
                            let is_curr = &t.singer == s;
                            if ui.selectable_label(is_curr, s).clicked() {
                                t.singer = s.clone();
                                changed = true;
                                ui.close_menu();
                            }
                        }
                    }
                });
            }
            ui.menu_button(lang.tr("Cor da faixa", "Track color"), |ui| {
                if let Some(t) = tracks.get_mut(part.track_index) {
                    if draw_color_palette(ui, &mut t.color_rgb) {
                        changed = true;
                        ui.close_menu();
                    }
                }
            });
            ui.separator();
            if ui.button(lang.tr("Duplicar", "Duplicate")).clicked() {
                duplicate_voice = Some(i);
                ui.close_menu();
            }
            if ui
                .button(lang.tr("Dividir no cursor", "Split at playhead"))
                .clicked()
            {
                split_voice = Some(i);
                ui.close_menu();
            }
            if ui.button(lang.tr("Excluir", "Delete")).clicked() {
                delete_voice = Some(i);
                ui.close_menu();
            }
        });
    }
    if let Some(i) = duplicate_voice {
        let mut p = parts[i].clone();
        p.position_ms += p.effective_duration_ms();
        parts.push(p);
        changed = true;
    }
    if let Some(i) = split_voice {
        let rel = *playhead - parts[i].position_ms;
        let old_dur = parts[i].effective_duration_ms();
        if rel > 0.0 && rel < old_dur {
            let mut right = parts[i].clone();
            right.position_ms = *playhead;
            right.duration_ms = (old_dur - rel).max(500.0);
            parts[i].duration_ms = rel.max(500.0);
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
            parts.push(right);
            changed = true;
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
        let track_color = tracks
            .get(w.track_index)
            .map(|t| track_color(t, w.track_index, theme))
            .unwrap_or_else(|| track_accent_color(w.track_index, theme));
        p.rect_filled(rect, Rounding::same(4.0), track_color.gamma_multiply(0.22));
        p.rect_stroke(
            rect,
            Rounding::same(4.0),
            Stroke::new(1.0_f32, track_color.gamma_multiply(0.55)),
        );
        let clip_hdr_rect = Rect::from_min_size(rect.min, Vec2::new(rect.width(), 16.0));
        p.rect_filled(
            clip_hdr_rect,
            Rounding { nw: 4.0, ne: 4.0, sw: 0.0, se: 0.0 },
            track_color.gamma_multiply(0.48),
        );
        p.text(
            Pos2::new(clip_hdr_rect.left() + 4.0, clip_hdr_rect.center().y),
            egui::Align2::LEFT_CENTER,
            &w.name,
            egui::FontId::proportional(9.0),
            Color32::WHITE,
        );
        let wave_dur_str = format_time_sec_min(w.duration_ms);
        p.text(
            Pos2::new(clip_hdr_rect.right() - 4.0, clip_hdr_rect.center().y),
            egui::Align2::RIGHT_CENTER,
            wave_dur_str,
            egui::FontId::monospace(8.5),
            Color32::from_white_alpha(220),
        );
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
    let (splitter_rect, splitter) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), 7.0),
        egui::Sense::click_and_drag(),
    );
    let splitter = splitter
        .on_hover_cursor(egui::CursorIcon::ResizeVertical)
        .on_hover_text(lang.tr(
            "Arraste para ajustar o tamanho da timeline e controles da faixa (duplo clique para redefinir)",
            "Drag to resize timeline vs track controls (double-click to reset)",
        ));
    let splitter_hover = splitter.hovered() || splitter.dragged();
    ui.painter().rect_filled(
        splitter_rect,
        1.0,
        if splitter_hover {
            theme.accent_c32().gamma_multiply(0.35)
        } else {
            theme.grid_line_sub_c32().gamma_multiply(0.30)
        },
    );
    ui.painter().line_segment(
        [
            Pos2::new(splitter_rect.center().x - 20.0, splitter_rect.center().y),
            Pos2::new(splitter_rect.center().x + 20.0, splitter_rect.center().y),
        ],
        Stroke::new(1.5_f32, if splitter_hover { theme.accent_c32() } else { theme.text_muted_c32() }),
    );
    if splitter.double_clicked() {
        timeline_ratio = 0.55;
        ui.ctx().data_mut(|d| d.insert_persisted(timeline_height_ratio_id, timeline_ratio));
        ui.ctx().request_repaint();
    } else if splitter.dragged() {
        let delta_y = splitter.drag_delta().y;
        let total_avail = (available_h - 78.0).max(100.0);
        let cur_h = height + delta_y;
        timeline_ratio = (cur_h / total_avail).clamp(0.15, 0.85);
        ui.ctx().data_mut(|d| d.insert_persisted(timeline_height_ratio_id, timeline_ratio));
        ui.ctx().request_repaint();
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
                    changed |= draw_track_mute_btn(ui, &mut track.mute, theme);
                    changed |= draw_track_solo_btn(ui, &mut track.solo, theme);
                });
                child.add_space(2.0);
                child.horizontal(|ui| {
                    ui.label(RichText::new(lang.tr("VOL", "VOL")).size(9.0).color(theme.text_muted_c32()));
                    let vol_str = if track.volume_db <= -59.0 {
                        "-∞ dB".to_string()
                    } else {
                        format!("{:+.1} dB", track.volume_db).replace('-', "−").replace('.', ",")
                    };
                    ui.label(RichText::new(vol_str).size(9.0).monospace().color(theme.text_primary_c32()));
                });
                changed |= child
                    .add(egui::Slider::new(&mut track.volume_db, -60.0..=12.0).show_value(false))
                    .on_hover_text(format!("{:.1} dB", track.volume_db))
                    .changed();

                child.horizontal(|ui| {
                    ui.label(RichText::new(lang.tr("PAN", "PAN")).size(9.0).color(theme.text_muted_c32()));
                    let pan_str = if track.pan.abs() < 0.03 {
                        "C".to_string()
                    } else if track.pan < 0.0 {
                        format!("L{:.0}%", -track.pan * 100.0)
                    } else {
                        format!("R{:.0}%", track.pan * 100.0)
                    };
                    ui.label(RichText::new(pan_str).size(9.0).monospace().color(theme.text_primary_c32()));
                });
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
                child.label(RichText::new(lang.tr("NÍVEL", "LEVEL")).size(9.0).color(theme.text_muted_c32()));
                draw_vu_meter_led(&mut child, level, meter.peak, width - 12.0, theme);
            }
            egui::CollapsingHeader::new("EQ global")
                .default_open(true)
                .show(ui, |ui| {
                    changed |= draw_eq(ui, theme, master);
                });
        });
    if let Some((i, dur)) = new_part {
        let mut p = UVoicePart::new(format!("Parte vocal {}", parts.len() + 1), i);
        p.position_ms = *playhead;
        p.duration_ms = dur;
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

fn draw_eq_knob(
    ui: &mut egui::Ui,
    label: &str,
    average: &mut f32,
    theme: &ThemeConfig,
) -> (bool, egui::Response) {
    let (rect, r) = ui.allocate_exact_size(Vec2::splat(44.0), egui::Sense::click_and_drag());
    let mut changed = false;
    if r.dragged() {
        let delta = ui.input(|i| i.pointer.delta().y) * 0.15;
        *average = (*average - delta).clamp(-12.0, 12.0);
        changed = true;
    }
    if r.double_clicked() {
        *average = 0.0;
        changed = true;
    }

    let center = rect.center() + Vec2::new(0.0, -3.0);
    let radius = 13.5;
    let t = ((*average + 12.0) / 24.0).clamp(0.0, 1.0);
    let start_angle = std::f32::consts::PI * 0.75;
    let total_angle = std::f32::consts::PI * 1.5;
    let current_angle = start_angle + total_angle * t;

    let painter = ui.painter();
    painter.circle_filled(center, radius, theme.bg_header_c32());
    painter.circle_stroke(
        center,
        radius,
        Stroke::new(1.5_f32, theme.grid_line_bar_c32()),
    );

    let steps = 18;
    let inactive_arc: Vec<Pos2> = (0..=steps)
        .map(|s| {
            let a = start_angle + total_angle * (s as f32 / steps as f32);
            center + Vec2::new(a.cos(), a.sin()) * (radius + 2.5)
        })
        .collect();
    painter.add(egui::Shape::line(
        inactive_arc,
        Stroke::new(1.8_f32, theme.grid_line_sub_c32().gamma_multiply(0.4)),
    ));

    let center_angle = start_angle + total_angle * 0.5;
    let (arc_from, arc_to) = if current_angle >= center_angle {
        (center_angle, current_angle)
    } else {
        (current_angle, center_angle)
    };
    let active_steps = 12;
    let active_arc: Vec<Pos2> = (0..=active_steps)
        .map(|s| {
            let a = arc_from + (arc_to - arc_from) * (s as f32 / active_steps as f32);
            center + Vec2::new(a.cos(), a.sin()) * (radius + 2.5)
        })
        .collect();
    painter.add(egui::Shape::line(
        active_arc,
        Stroke::new(2.5_f32, theme.accent_c32()),
    ));

    painter.line_segment(
        [center, center + Vec2::new(current_angle.cos(), current_angle.sin()) * (radius - 3.0)],
        Stroke::new(2.0_f32, theme.text_primary_c32()),
    );

    let val_str = format!("{:+.1}dB", *average);
    painter.text(
        Pos2::new(rect.center().x, rect.bottom() - 1.0),
        egui::Align2::CENTER_BOTTOM,
        val_str,
        egui::FontId::monospace(8.0),
        theme.text_primary_c32(),
    );

    let r = r.on_hover_text(format!("{label}: {:+.1} dB (duplo clique para 0 dB)", *average));
    (changed, r)
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
            ("Médios B.", 9..16),
            ("Médios A.", 16..22),
            ("Agudos", 22..31),
        ]
        .into_iter()
        .enumerate()
        {
            columns[column].vertical(|ui| {
                let mut average = rack.eq.gains[range.clone()].iter().sum::<f32>() / range.len() as f32;
                let (changed, _r) = draw_eq_knob(ui, label, &mut average, theme);
                if changed {
                    let old_avg = rack.eq.gains[range.clone()].iter().sum::<f32>() / range.len() as f32;
                    let delta = average - old_avg;
                    for g in &mut rack.eq.gains[range.clone()] {
                        *g = (*g + delta).clamp(-12.0, 12.0);
                    }
                }
                ui.add(
                    egui::Label::new(
                        RichText::new(label)
                            .size(9.0)
                            .color(theme.text_muted_c32()),
                    )
                    .truncate(),
                );
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

    // Fundo escuro do analisador com borda suave
    ui.painter().rect_filled(rect, Rounding::same(3.0), theme.bg_canvas_c32());
    ui.painter().rect_stroke(
        rect,
        Rounding::same(3.0),
        Stroke::new(0.5_f32, theme.grid_line_sub_c32().gamma_multiply(0.6)),
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
        let fx = x(freq);
        ui.painter().line_segment(
            [Pos2::new(fx, rect.top()), Pos2::new(fx, rect.bottom())],
            Stroke::new(0.5_f32, theme.grid_line_sub_c32().gamma_multiply(0.35)),
        );
        ui.painter().text(
            Pos2::new(
                fx.clamp(rect.left() + 18.0, rect.right() - 14.0),
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
        let stroke_w = if gain == 0.0 { 0.8_f32 } else { 0.5_f32 };
        let stroke_c = if gain == 0.0 { theme.grid_line_bar_c32() } else { theme.grid_line_sub_c32().gamma_multiply(0.4) };
        ui.painter().line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(stroke_w, stroke_c),
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
    let points: Vec<Pos2> = (0..31)
        .map(|i| {
            Pos2::new(
                x(freqs[i]),
                rect.center().y - rack.eq.gains[i] / 24.0 * rect.height(),
            )
        })
        .collect();

    // Preenchimento de gradiente espectral translúcido sob a curva
    let zero_y = rect.center().y;
    for i in 0..30 {
        let p1 = points[i];
        let p2 = points[i + 1];
        let poly = vec![
            Pos2::new(p1.x, zero_y),
            p1,
            p2,
            Pos2::new(p2.x, zero_y),
        ];
        ui.painter().add(egui::Shape::convex_polygon(
            poly,
            theme.accent_c32().gamma_multiply(0.18),
            Stroke::NONE,
        ));
    }

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
                                        &mut master,
                                        &[],
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
                            &[],
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
                            &mut master,
                            &[],
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
