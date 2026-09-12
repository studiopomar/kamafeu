use super::*;

#[cfg(test)]
mod tests;

pub(super) fn draw(
    ui: &mut egui::Ui,
    notes: &mut [UNote],
    state: &mut PianoRollState,
    keyboard_width: f32,
    timeline_scroll_x: f32,
    on_before_change: &mut dyn FnMut(),
    _theme: &ThemeConfig,
    ruler_rect: Rect,
    bpm: f64,
    lang: crate::config::AppLanguage,
) {
    if !state.show_parameters_drawer || state.is_maximized {
        return;
    }

    let panel_response = egui::TopBottomPanel::bottom("bottom_param_drawer_fixed")
        .resizable(true)
        .height_range(60.0..=750.0)
        .default_height(state.drawer_height)
        .frame(
            egui::Frame::none()
                .fill(MelodyneTheme::BG_PANEL)
                .stroke(Stroke::new(1.5_f32, MelodyneTheme::ACCENT_GOLD)),
        )
        .show_inside(ui, |ui| {
            let drawer_available_h = ui.available_height().max(50.0);
            let available_graph_h = drawer_available_h;

            // A default horizontal row only allocates one widget's height.
            // Reserve the entire panel height and bound the sidebar width so
            // its non-shrinking ScrollArea cannot push the graph offscreen.
            ui.allocate_ui_with_layout(
                Vec2::new(ui.available_width(), drawer_available_h),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    ui.add_space(4.0);
                    ui.allocate_ui_with_layout(
                        Vec2::new(150.0, drawer_available_h),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.add_space(2.0);
                            ui.label(
                                egui::RichText::new(
                                    lang.tr("PARÂMETROS / EXPRESSÕES", "PARAMETERS / EXPRESSIONS"),
                                )
                                .strong()
                                .size(10.0)
                                .color(Color32::from_rgb(0, 255, 157)),
                            );
                            ui.add_space(2.0);

                            let list_height = (ui.available_height() - 4.0).max(40.0);
                            egui::ScrollArea::vertical()
                                .id_salt("param_tabs_scroll")
                                .min_scrolled_height(40.0)
                                .max_height(list_height)
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.set_width(135.0);
                                    let param_tabs = [
                                        (
                                            lang.tr("Dynamics (DYN)", "Dynamics (DYN)"),
                                            ParameterTab::Dynamics,
                                        ),
                                        (
                                            lang.tr("Pitch Offset (PITD)", "Pitch Offset (PITD)"),
                                            ParameterTab::PitchDelta,
                                        ),
                                        (
                                            lang.tr("Gender (GEN/g)", "Gender (GEN/g)"),
                                            ParameterTab::Gender,
                                        ),
                                        (
                                            lang.tr("Vel. Consoante (VEL)", "Consonant Vel. (VEL)"),
                                            ParameterTab::Velocity,
                                        ),
                                        (
                                            lang.tr("Breathiness (BRE/B)", "Breathiness (BRE/B)"),
                                            ParameterTab::Breathiness,
                                        ),
                                        (
                                            lang.tr("Modulação (MOD)", "Modulation (MOD)"),
                                            ParameterTab::Modulation,
                                        ),
                                        (
                                            lang.tr("Volume (VOL)", "Volume (VOL)"),
                                            ParameterTab::Volume,
                                        ),
                                        (
                                            lang.tr("Ataque (ATK)", "Attack (ATK)"),
                                            ParameterTab::Attack,
                                        ),
                                        (
                                            lang.tr("Decaimento (DEC)", "Decay (DEC)"),
                                            ParameterTab::Decay,
                                        ),
                                        (
                                            lang.tr("Vibrato Tam (VIBL)", "Vibrato Len (VIBL)"),
                                            ParameterTab::VibratoLength,
                                        ),
                                        (
                                            lang.tr("Vibrato Prof (VIBD)", "Vibrato Depth (VIBD)"),
                                            ParameterTab::VibratoDepth,
                                        ),
                                        (
                                            lang.tr("Vibrato Per (VIBP)", "Vibrato Per (VIBP)"),
                                            ParameterTab::VibratoPeriod,
                                        ),
                                    ];

                                    for (p_name, tab_val) in param_tabs {
                                        let is_sel = state.selected_parameter == tab_val;
                                        let (text_color, fill_color) = if is_sel {
                                            (
                                                Color32::from_rgb(0, 255, 157),
                                                Color32::from_rgb(36, 27, 53),
                                            )
                                        } else {
                                            (Color32::from_rgb(165, 148, 201), Color32::TRANSPARENT)
                                        };

                                        let btn = egui::Button::new(
                                            egui::RichText::new(p_name).size(9.5).color(text_color),
                                        )
                                        .min_size(Vec2::new(125.0, 18.0))
                                        .fill(fill_color)
                                        .rounding(Rounding::same(3.0));

                                        if ui.add(btn).clicked() {
                                            state.selected_parameter = tab_val;
                                        }
                                    }
                                });
                        },
                    );

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    let graph_width = (ui.available_width() - 8.0).max(10.0);
                    let graph_height = available_graph_h.max(10.0);
                    let (graph_rect, graph_response) = ui.allocate_exact_size(
                        Vec2::new(graph_width, graph_height),
                        Sense::click_and_drag(),
                    );

                    let painter = ui.painter_at(graph_rect);
                    // Fundo escuro estilizado estilo OpenUtau / Synthesizer V
                    painter.rect_filled(
                        graph_rect,
                        Rounding::same(3.0),
                        Color32::from_rgb(18, 18, 22),
                    );
                    painter.rect_stroke(
                        graph_rect,
                        Rounding::same(3.0),
                        Stroke::new(1.0_f32, Color32::from_rgb(38, 38, 48)),
                    );

                    let timeline_origin_x = ruler_rect.min.x + keyboard_width - timeline_scroll_x;
                    let mid_y = graph_rect.min.y + graph_rect.height() * 0.5;
                    let half_span_y = (graph_rect.height() * 0.44).max(10.0);

                    // 1. Grid Musical (Compassos e Tempos sincronizados com o Piano Roll)
                    let beat_dur_ms = 60_000.0 / bpm.max(10.0);
                    let beats_per_bar = 4.0_f64;
                    let bar_dur_ms = beat_dur_ms * beats_per_bar;

                    let t_min_visible =
                        (graph_rect.min.x - timeline_origin_x) as f64 / state.px_per_ms as f64;
                    let t_max_visible =
                        (graph_rect.max.x - timeline_origin_x) as f64 / state.px_per_ms as f64;

                    let start_bar = (t_min_visible / bar_dur_ms).floor() as i64;
                    let end_bar = (t_max_visible / bar_dur_ms).ceil() as i64 + 1;

                    for b in start_bar..=end_bar {
                        let bar_t = b as f64 * bar_dur_ms;
                        let bar_x = timeline_origin_x + (bar_t * state.px_per_ms as f64) as f32;

                        if bar_x >= graph_rect.min.x - 2.0 && bar_x <= graph_rect.max.x + 2.0 {
                            // Linha de compasso
                            painter.line_segment(
                                [
                                    Pos2::new(bar_x, graph_rect.min.y),
                                    Pos2::new(bar_x, graph_rect.max.y),
                                ],
                                Stroke::new(
                                    1.0_f32,
                                    Color32::from_rgba_unmultiplied(85, 85, 110, 85),
                                ),
                            );

                            // Número do compasso no topo do drawer
                            if b >= 0
                                && bar_x >= graph_rect.min.x
                                && bar_x <= graph_rect.max.x - 15.0
                            {
                                painter.text(
                                    Pos2::new(bar_x + 3.0, graph_rect.min.y + 2.0),
                                    egui::Align2::LEFT_TOP,
                                    format!("{}", b + 1),
                                    egui::FontId::proportional(9.0),
                                    Color32::from_rgb(140, 140, 160),
                                );
                            }
                        }

                        // Linhas de tempo (sub-beats)
                        for beat_idx in 1..beats_per_bar as i64 {
                            let beat_t = bar_t + beat_idx as f64 * beat_dur_ms;
                            let beat_x =
                                timeline_origin_x + (beat_t * state.px_per_ms as f64) as f32;
                            if beat_x >= graph_rect.min.x && beat_x <= graph_rect.max.x {
                                painter.line_segment(
                                    [
                                        Pos2::new(beat_x, graph_rect.min.y),
                                        Pos2::new(beat_x, graph_rect.max.y),
                                    ],
                                    Stroke::new(
                                        0.5_f32,
                                        Color32::from_rgba_unmultiplied(50, 50, 68, 45),
                                    ),
                                );
                            }
                        }
                    }

                    // 2. Linha guia central (Zero Baseline: y = 0)
                    painter.line_segment(
                        [
                            Pos2::new(graph_rect.min.x, mid_y),
                            Pos2::new(graph_rect.max.x, mid_y),
                        ],
                        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(160, 175, 210, 100)),
                    );

                    // Linhas guias de limite superior (+MAX) e inferior (-MAX)
                    let top_y = mid_y - half_span_y;
                    let bot_y = mid_y + half_span_y;
                    painter.line_segment(
                        [
                            Pos2::new(graph_rect.min.x, top_y),
                            Pos2::new(graph_rect.max.x, top_y),
                        ],
                        Stroke::new(0.6_f32, Color32::from_rgba_unmultiplied(90, 110, 160, 35)),
                    );
                    painter.line_segment(
                        [
                            Pos2::new(graph_rect.min.x, bot_y),
                            Pos2::new(graph_rect.max.x, bot_y),
                        ],
                        Stroke::new(0.6_f32, Color32::from_rgba_unmultiplied(90, 110, 160, 35)),
                    );

                    // Indicadores numéricos no canto esquerdo
                    painter.text(
                        Pos2::new(graph_rect.min.x + 6.0, top_y + 2.0),
                        egui::Align2::LEFT_TOP,
                        "+MAX",
                        egui::FontId::proportional(8.5),
                        Color32::from_rgba_unmultiplied(0, 255, 157, 180),
                    );
                    painter.text(
                        Pos2::new(graph_rect.min.x + 6.0, mid_y - 2.0),
                        egui::Align2::LEFT_BOTTOM,
                        "0",
                        egui::FontId::proportional(8.5),
                        Color32::from_rgba_unmultiplied(200, 200, 230, 160),
                    );
                    painter.text(
                        Pos2::new(graph_rect.min.x + 6.0, bot_y - 2.0),
                        egui::Align2::LEFT_BOTTOM,
                        "-MAX",
                        egui::FontId::proportional(8.5),
                        Color32::from_rgba_unmultiplied(255, 120, 150, 180),
                    );

                    // 3. Playhead no Drawer (se visível)
                    let playhead_x =
                        timeline_origin_x + (state.playhead_ms * state.px_per_ms as f64) as f32;
                    if playhead_x >= graph_rect.min.x && playhead_x <= graph_rect.max.x {
                        painter.line_segment(
                            [
                                Pos2::new(playhead_x, graph_rect.min.y),
                                Pos2::new(playhead_x, graph_rect.max.y),
                            ],
                            Stroke::new(1.2_f32, Color32::from_rgb(0, 255, 157)),
                        );
                    }

                    // 4. Coletar intervalos dos fonemas e seus valores normalizados
                    struct PhonemeKeyframe {
                        note_index: usize,
                        start_ms: f64,
                        end_ms: f64,
                        norm_val: f64,
                        lyric: String,
                        alias: String,
                    }

                    let mut keyframes: Vec<PhonemeKeyframe> = Vec::new();

                    for (note_index, note) in notes.iter().enumerate() {
                        let norm = match state.selected_parameter {
                            ParameterTab::Dynamics => note.expressions.dynamics / 120.0,
                            ParameterTab::PitchDelta => note.expressions.pitch_delta / 1200.0,
                            ParameterTab::Gender => note.expressions.gender / 100.0,
                            ParameterTab::Velocity => {
                                (note.expressions.consonant_velocity - 100.0) / 100.0
                            }
                            ParameterTab::Breathiness => note.expressions.breathiness / 100.0,
                            ParameterTab::Modulation => note.expressions.modulation / 100.0,
                            ParameterTab::Volume => (note.expressions.volume - 100.0) / 100.0,
                            ParameterTab::Attack => (note.expressions.attack - 100.0) / 100.0,
                            ParameterTab::Decay => note.expressions.decay / 100.0,
                            ParameterTab::VibratoLength => note.vibrato.length_pct / 100.0,
                            ParameterTab::VibratoDepth => note.vibrato.depth_cents / 200.0,
                            ParameterTab::VibratoPeriod => (note.vibrato.period_ms - 250.0) / 200.0,
                        };
                        let norm_clamped = norm.clamp(-1.0, 1.0);

                        // DYN is a note-level continuous expression. Do not
                        // split it into phoneme keyframes; that creates the
                        // staircase shown by the old drawer implementation.
                        if state.selected_parameter == ParameterTab::Dynamics {
                            keyframes.push(PhonemeKeyframe {
                                note_index,
                                start_ms: note.position_ms,
                                end_ms: note.position_ms + note.duration_ms,
                                norm_val: norm_clamped,
                                lyric: note.lyric.clone(),
                                alias: note.lyric.clone(),
                            });
                            continue;
                        }

                        if let Some(cached) = state.note_phonemes_cache.get(note_index) {
                            if !cached.is_empty() {
                                let count = cached.len();
                                let mut cur_offset = cached[0].1;
                                let has_custom = note.phoneme_durations_ms.len() == count;
                                let authored_sum: f64 = if has_custom {
                                    note.phoneme_durations_ms.iter().sum()
                                } else {
                                    0.0
                                };
                                let scale = if has_custom
                                    && authored_sum > 0.0
                                    && (authored_sum - note.duration_ms).abs() > 2.0
                                {
                                    note.duration_ms / authored_sum
                                } else {
                                    1.0
                                };

                                for (idx, (p_alias, _rel_pos, default_dur)) in
                                    cached.iter().enumerate()
                                {
                                    let dur = if has_custom {
                                        note.phoneme_durations_ms[idx] * scale
                                    } else {
                                        *default_dur
                                    };
                                    let p_start = (note.position_ms + cur_offset).max(0.0);
                                    let p_end = p_start + dur.max(10.0);
                                    cur_offset += dur;

                                    keyframes.push(PhonemeKeyframe {
                                        note_index,
                                        start_ms: p_start,
                                        end_ms: p_end,
                                        norm_val: norm_clamped,
                                        lyric: note.lyric.clone(),
                                        alias: p_alias.clone(),
                                    });
                                }
                                continue;
                            }
                        }

                        // Fallback se o cache de fonemas estiver vazio: utiliza preutterance da nota
                        let preutter = state
                            .oto_preutter_cache
                            .get(note_index)
                            .copied()
                            .unwrap_or(0.0);
                        let p_start = (note.position_ms - preutter).max(0.0);
                        let p_end = note.position_ms + note.duration_ms;
                        keyframes.push(PhonemeKeyframe {
                            note_index,
                            start_ms: p_start,
                            end_ms: p_end,
                            norm_val: norm_clamped,
                            lyric: note.lyric.clone(),
                            alias: note.lyric.clone(),
                        });
                    }

                    keyframes.sort_by(|a, b| {
                        a.start_ms
                            .partial_cmp(&b.start_ms)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });

                    // Interação com Ferramenta de Pincel / Desenho Contínuo sobre os fonemas
                    if graph_response.hovered() {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                    }

                    let is_shift_down = ui.input(|i| i.modifiers.shift);
                    if !ui.input(|i| i.pointer.primary_down()) {
                        state.shift_locked_drawer_norm = None;
                    }

                    let mut is_drawing = false;
                    let mut cursor_mpos = Pos2::ZERO;

                    let is_secondary_down = ui.input(|i| i.pointer.secondary_down());
                    let is_primary_down = ui.input(|i| i.pointer.primary_down());
                    let secondary_active = graph_response.secondary_clicked()
                        || graph_response.dragged_by(egui::PointerButton::Secondary)
                        || (graph_response.hovered() && is_secondary_down);
                    let primary_active = (graph_response.clicked() && !is_secondary_down)
                        || graph_response.dragged_by(egui::PointerButton::Primary)
                        || (graph_response.hovered() && is_primary_down && !is_secondary_down);

                    if let Some(mpos) = graph_response.interact_pointer_pos().or_else(|| {
                        if secondary_active || primary_active || graph_response.hovered() {
                            ui.input(|i| i.pointer.hover_pos())
                        } else {
                            None
                        }
                    }) {
                        cursor_mpos = mpos;
                        let raw_norm = ((mid_y - mpos.y) / half_span_y).clamp(-1.0, 1.0) as f64;

                        if secondary_active {
                            if graph_response.secondary_clicked()
                                || graph_response.drag_started_by(egui::PointerButton::Secondary)
                                || ui.input(|i| i.pointer.secondary_pressed())
                            {
                                on_before_change();
                            }
                            is_drawing = true;
                            let click_t =
                                (mpos.x - timeline_origin_x) as f64 / state.px_per_ms as f64;
                            let mut changed = false;
                            for kf in &keyframes {
                                if click_t >= kf.start_ms - 20.0 && click_t <= kf.end_ms + 20.0 {
                                    if let Some(note) = notes.get_mut(kf.note_index) {
                                        match state.selected_parameter {
                                            ParameterTab::Dynamics => {
                                                note.expressions.dynamics = 0.0;
                                                note.expressions.dynamics_curve.clear();
                                            }
                                            ParameterTab::PitchDelta => {
                                                note.pitch_bend.points.clear();
                                                note.expressions.pitch_delta = 0.0;
                                            }
                                            ParameterTab::Gender => {
                                                note.expressions.gender = 0.0;
                                            }
                                            ParameterTab::Velocity => {
                                                note.expressions.consonant_velocity = 100.0;
                                            }
                                            ParameterTab::Breathiness => {
                                                note.expressions.breathiness = 0.0;
                                            }
                                            ParameterTab::Modulation => {
                                                note.expressions.modulation = 0.0;
                                            }
                                            ParameterTab::Volume => {
                                                note.expressions.volume = 100.0;
                                            }
                                            ParameterTab::Attack => {
                                                note.expressions.attack = 100.0;
                                            }
                                            ParameterTab::Decay => {
                                                note.expressions.decay = 0.0;
                                            }
                                            ParameterTab::VibratoLength => {
                                                note.vibrato.length_pct = 0.0;
                                            }
                                            ParameterTab::VibratoDepth => {
                                                note.vibrato.depth_cents = 0.0;
                                            }
                                            ParameterTab::VibratoPeriod => {
                                                note.vibrato.period_ms = 175.0;
                                            }
                                        }
                                        changed = true;
                                    }
                                }
                            }
                            if changed {
                                state.continuous_edit_dirty = true;
                            }
                        } else if primary_active {
                            if graph_response.drag_started_by(egui::PointerButton::Primary)
                                || (graph_response.clicked() && !graph_response.dragged())
                                || ui.input(|i| i.pointer.primary_pressed())
                            {
                                on_before_change();
                                state.shift_locked_drawer_norm = Some(raw_norm);
                            }
                            is_drawing = true;
                            let click_t =
                                (mpos.x - timeline_origin_x) as f64 / state.px_per_ms as f64;

                            let norm = if is_shift_down {
                                *state.shift_locked_drawer_norm.get_or_insert(raw_norm)
                            } else {
                                state.shift_locked_drawer_norm = Some(raw_norm);
                                raw_norm
                            };

                            let mut changed = false;
                            for kf in &keyframes {
                                if click_t >= kf.start_ms - 20.0 && click_t <= kf.end_ms + 20.0 {
                                    if let Some(note) = notes.get_mut(kf.note_index) {
                                        match state.selected_parameter {
                                            ParameterTab::Dynamics => {
                                                note.expressions.dynamics =
                                                    (norm * 120.0).clamp(-240.0, 120.0);
                                                let value = note.expressions.dynamics;
                                                note.expressions.dynamics_curve.retain(|point| {
                                                    (point.time_offset_ms - (click_t - note.position_ms)).abs() >= 8.0
                                                });
                                                note.expressions.dynamics_curve.push(
                                                    crate::project::model::UExpressionPoint {
                                                        time_offset_ms: (click_t - note.position_ms).max(0.0),
                                                        value,
                                                    },
                                                );
                                                note.expressions.dynamics_curve.sort_by(|a, b| a.time_offset_ms.partial_cmp(&b.time_offset_ms).unwrap_or(std::cmp::Ordering::Equal));
                                            }
                                            ParameterTab::PitchDelta => {
                                                let rel_t = click_t - note.position_ms;
                                                let target_cents =
                                                    (norm * 1200.0).clamp(-1200.0, 1200.0);
                                                note.pitch_bend.points.retain(|pt| {
                                                    (pt.time_offset_ms - rel_t).abs() >= 8.0
                                                });
                                                note.pitch_bend.points.push(UPitchBendPoint {
                                                    time_offset_ms: rel_t,
                                                    pitch_offset_cents: target_cents,
                                                    shape: "s".to_string(),
                                                });
                                                note.pitch_bend.points.sort_by(|a, b| {
                                                    a.time_offset_ms
                                                        .partial_cmp(&b.time_offset_ms)
                                                        .unwrap_or(std::cmp::Ordering::Equal)
                                                });
                                                note.expressions.pitch_delta = 0.0;
                                            }
                                            ParameterTab::Gender => {
                                                note.expressions.gender =
                                                    (norm * 100.0).clamp(-100.0, 100.0);
                                            }
                                            ParameterTab::Velocity => {
                                                    note.expressions.consonant_velocity =
                                                    (100.0 + norm * 100.0).clamp(-100.0, 200.0);
                                            }
                                            ParameterTab::Breathiness => {
                                                note.expressions.breathiness =
                                                    (norm * 100.0).clamp(-100.0, 100.0);
                                            }
                                            ParameterTab::Modulation => {
                                                note.expressions.modulation =
                                                    (norm * 100.0).clamp(-100.0, 100.0);
                                            }
                                            ParameterTab::Volume => {
                                                note.expressions.volume =
                                                    (100.0 + norm * 100.0).clamp(0.0, 200.0);
                                            }
                                            ParameterTab::Attack => {
                                                note.expressions.attack =
                                                    (100.0 + norm * 100.0).clamp(0.0, 200.0);
                                            }
                                            ParameterTab::Decay => {
                                                note.expressions.decay =
                                                    (norm * 100.0).clamp(-100.0, 100.0);
                                            }
                                            ParameterTab::VibratoLength => {
                                                note.vibrato.length_pct =
                                                    (norm * 100.0).clamp(0.0, 100.0);
                                            }
                                            ParameterTab::VibratoDepth => {
                                                note.vibrato.depth_cents =
                                                    (norm * 200.0).clamp(0.0, 200.0);
                                            }
                                            ParameterTab::VibratoPeriod => {
                                                note.vibrato.period_ms =
                                                    (250.0 + norm * 200.0).clamp(50.0, 450.0);
                                            }
                                        }
                                        changed = true;
                                    }
                                }
                            }
                            if changed {
                                state.continuous_edit_dirty = true;
                            }
                        }
                    }

                    // 5. Função contínua suave ao longo dos fonemas estilo OpenUtau
                    let eval_curve = |t: f64| -> f64 {
                        if state.selected_parameter == ParameterTab::PitchDelta {
                            for note in notes.iter() {
                                let n_start = note.position_ms - 150.0;
                                let n_end = note.position_ms + note.duration_ms + 150.0;
                                if t >= n_start && t <= n_end {
                                    let rel_t = t - note.position_ms;
                                    let pts_offset = if !note.pitch_bend.points.is_empty() {
                                        PitchBendSolver::get_pitch_offset_cents(
                                            rel_t,
                                            &note.pitch_bend.points,
                                        )
                                    } else {
                                        0.0
                                    };
                                    let total_cents = (pts_offset + note.expressions.pitch_delta)
                                        .clamp(-1200.0, 1200.0);
                                    return total_cents / 1200.0;
                                }
                            }
                            return 0.0;
                        }

                        if state.selected_parameter == ParameterTab::Dynamics {
                            for note in notes.iter() {
                                if t < note.position_ms || t > note.position_ms + note.duration_ms {
                                    continue;
                                }
                                let rel_t = t - note.position_ms;
                                let points = &note.expressions.dynamics_curve;
                                if points.is_empty() {
                                    return (note.expressions.dynamics / 120.0).clamp(-1.0, 1.0);
                                }
                                if let Some(first) = points.first() {
                                    if rel_t <= first.time_offset_ms { return (first.value / 120.0).clamp(-1.0, 1.0); }
                                }
                                for pair in points.windows(2) {
                                    if rel_t <= pair[1].time_offset_ms {
                                        let span = (pair[1].time_offset_ms - pair[0].time_offset_ms).max(1e-6);
                                        let u = ((rel_t - pair[0].time_offset_ms) / span).clamp(0.0, 1.0);
                                        let value = pair[0].value + (pair[1].value - pair[0].value) * u;
                                        return (value / 120.0).clamp(-1.0, 1.0);
                                    }
                                }
                                return (points.last().map(|p| p.value).unwrap_or(note.expressions.dynamics) / 120.0).clamp(-1.0, 1.0);
                            }
                            return 0.0;
                        }

                        if keyframes.is_empty() {
                            return 0.0;
                        }

                        let first_start = keyframes[0].start_ms;
                        if t < first_start - 35.0 {
                            return 0.0;
                        }
                        if t < first_start {
                            let u = ((t - (first_start - 35.0)) / 35.0).clamp(0.0, 1.0);
                            let s = u * u * (3.0 - 2.0 * u);
                            return s * keyframes[0].norm_val;
                        }

                        let last_end = keyframes.last().unwrap().end_ms;
                        if t > last_end + 35.0 {
                            return 0.0;
                        }
                        if t > last_end {
                            let u = ((t - last_end) / 35.0).clamp(0.0, 1.0);
                            let s = u * u * (3.0 - 2.0 * u);
                            return (1.0 - s) * keyframes.last().unwrap().norm_val;
                        }

                        for (i, kf) in keyframes.iter().enumerate() {
                            if t >= kf.start_ms && t <= kf.end_ms {
                                if i + 1 < keyframes.len() {
                                    let next_kf = &keyframes[i + 1];
                                    let trans_win = 25.0_f64.min((kf.end_ms - kf.start_ms) * 0.4);
                                    if t > kf.end_ms - trans_win {
                                        let u = ((t - (kf.end_ms - trans_win))
                                            / (trans_win
                                                + (next_kf.start_ms - kf.end_ms).max(0.0)))
                                        .clamp(0.0, 1.0);
                                        let s = u * u * (3.0 - 2.0 * u);
                                        return (1.0 - s) * kf.norm_val + s * next_kf.norm_val;
                                    }
                                }
                                return kf.norm_val;
                            }

                            if i + 1 < keyframes.len() {
                                let next_kf = &keyframes[i + 1];
                                if t > kf.end_ms && t < next_kf.start_ms {
                                    let gap = next_kf.start_ms - kf.end_ms;
                                    if gap <= 90.0 {
                                        let u = ((t - kf.end_ms) / gap).clamp(0.0, 1.0);
                                        let s = u * u * (3.0 - 2.0 * u);
                                        return (1.0 - s) * kf.norm_val + s * next_kf.norm_val;
                                    } else {
                                        let ease_dur = 30.0_f64;
                                        if t <= kf.end_ms + ease_dur {
                                            let u = ((t - kf.end_ms) / ease_dur).clamp(0.0, 1.0);
                                            let s = u * u * (3.0 - 2.0 * u);
                                            return (1.0 - s) * kf.norm_val;
                                        } else if t >= next_kf.start_ms - ease_dur {
                                            let u = ((t - (next_kf.start_ms - ease_dur))
                                                / ease_dur)
                                                .clamp(0.0, 1.0);
                                            let s = u * u * (3.0 - 2.0 * u);
                                            return s * next_kf.norm_val;
                                        } else {
                                            return 0.0;
                                        }
                                    }
                                }
                            }
                        }

                        0.0
                    };

                    // 6. Desenhar a Curva Contínua Branca (OpenUtau Waveform Style)
                    let sample_step_px = 2.0_f32;
                    let num_samples =
                        ((graph_rect.width() / sample_step_px).ceil() as usize).max(2);
                    let mut curve_points: Vec<Pos2> = Vec::with_capacity(num_samples + 2);

                    for s_idx in 0..=num_samples {
                        let px = (graph_rect.min.x + s_idx as f32 * sample_step_px)
                            .min(graph_rect.max.x);
                        let t_ms = (px - timeline_origin_x) as f64 / state.px_per_ms as f64;
                        let norm = eval_curve(t_ms).clamp(-1.0, 1.0);
                        let py = mid_y - (norm as f32 * half_span_y);
                        curve_points.push(Pos2::new(px, py));
                    }

                    if curve_points.len() >= 2 {
                        // Linha branca suave contínua exatamente como no OpenUtau
                        painter.add(egui::Shape::line(
                            curve_points,
                            Stroke::new(2.2_f32, Color32::WHITE),
                        ));
                    }

                    // 7. Renderizar Marcadores e Letras/Fonemas na base do Drawer
                    for kf in &keyframes {
                        let x_start =
                            timeline_origin_x + (kf.start_ms * state.px_per_ms as f64) as f32;
                        let x_end = timeline_origin_x + (kf.end_ms * state.px_per_ms as f64) as f32;

                        if x_end >= graph_rect.min.x - 20.0 && x_start <= graph_rect.max.x + 20.0 {
                            // Marcador vertical sutil no início do fonema
                            painter.line_segment(
                                [
                                    Pos2::new(x_start, graph_rect.max.y - 14.0),
                                    Pos2::new(x_start, graph_rect.max.y - 2.0),
                                ],
                                Stroke::new(
                                    1.0_f32,
                                    Color32::from_rgba_unmultiplied(130, 130, 180, 80),
                                ),
                            );

                            // Letra / Fonema na base exatamente no vão do fonema
                            let x_mid = (x_start + x_end) * 0.5;
                            if x_mid >= graph_rect.min.x + 8.0 && x_mid <= graph_rect.max.x - 8.0 {
                                let label_display = if kf.alias.starts_with(&kf.lyric)
                                    || kf.alias.contains(&kf.lyric)
                                {
                                    kf.lyric.clone()
                                } else {
                                    kf.alias.clone()
                                };
                                painter.text(
                                    Pos2::new(x_mid, graph_rect.max.y - 12.0),
                                    egui::Align2::CENTER_TOP,
                                    label_display,
                                    egui::FontId::proportional(8.5),
                                    Color32::from_rgba_unmultiplied(170, 170, 210, 160),
                                );
                            }
                        }
                    }

                    // 8. Retículo / Cursor Interativo do Pincel com Tooltip de Valor em Tempo Real
                    if graph_response.hovered() || is_drawing {
                        let hover_t =
                            (cursor_mpos.x - timeline_origin_x) as f64 / state.px_per_ms as f64;
                        let hover_norm = eval_curve(hover_t).clamp(-1.0, 1.0);
                        let hover_curve_y = mid_y - (hover_norm as f32 * half_span_y);

                        // Linha guia horizontal quando SHIFT está travado
                        if is_shift_down {
                            if let Some(locked_norm) = state.shift_locked_drawer_norm {
                                let locked_y = mid_y - (locked_norm as f32 * half_span_y);
                                painter.line_segment(
                                    [
                                        Pos2::new(graph_rect.min.x, locked_y),
                                        Pos2::new(graph_rect.max.x, locked_y),
                                    ],
                                    Stroke::new(
                                        1.0_f32,
                                        Color32::from_rgba_unmultiplied(0, 255, 180, 150),
                                    ),
                                );
                            }
                        }

                        // Ponto cursor na curva
                        painter.circle_filled(
                            Pos2::new(cursor_mpos.x, hover_curve_y),
                            4.0,
                            Color32::from_rgb(0, 255, 157),
                        );
                        painter.circle_stroke(
                            Pos2::new(cursor_mpos.x, hover_curve_y),
                            5.5,
                            Stroke::new(1.2_f32, Color32::WHITE),
                        );

                        // Encontrar fonema sob o cursor para exibir lyric e valor formatado
                        let kf_under_cursor = keyframes
                            .iter()
                            .find(|kf| hover_t >= kf.start_ms && hover_t <= kf.end_ms);
                        let note_under_cursor =
                            kf_under_cursor.and_then(|kf| notes.get(kf.note_index));
                        let val_str = match state.selected_parameter {
                            ParameterTab::Dynamics => {
                                let v = note_under_cursor
                                    .map(|n| n.expressions.dynamics)
                                    .unwrap_or(hover_norm * 120.0);
                                format!("{:+.1} dB", v * 0.1)
                            }
                            ParameterTab::PitchDelta => {
                                format!("{:+.0} c", hover_norm * 1200.0)
                            }
                            ParameterTab::Gender => {
                                let v = note_under_cursor
                                    .map(|n| n.expressions.gender)
                                    .unwrap_or(hover_norm * 100.0);
                                format!("g{:+.0}", v)
                            }
                            ParameterTab::Velocity => {
                                let v = note_under_cursor
                                    .map(|n| n.expressions.consonant_velocity - 100.0)
                                    .unwrap_or(hover_norm * 100.0);
                                format!("VEL {:+.0}%", v)
                            }
                            ParameterTab::Breathiness => {
                                let v = note_under_cursor
                                    .map(|n| n.expressions.breathiness)
                                    .unwrap_or(hover_norm * 100.0);
                                format!("B{:+.0}", v)
                            }
                            ParameterTab::Modulation => {
                                let v = note_under_cursor
                                    .map(|n| n.expressions.modulation)
                                    .unwrap_or(hover_norm * 100.0);
                                format!("MOD {:+.0}%", v)
                            }
                            ParameterTab::Volume => {
                                let v = note_under_cursor
                                    .map(|n| n.expressions.volume - 100.0)
                                    .unwrap_or(hover_norm * 100.0);
                                format!("VOL {:+.0}%", v)
                            }
                            ParameterTab::Attack => {
                                let v = note_under_cursor
                                    .map(|n| n.expressions.attack - 100.0)
                                    .unwrap_or(hover_norm * 100.0);
                                format!("ATK {:+.0}%", v)
                            }
                            ParameterTab::Decay => {
                                let v = note_under_cursor
                                    .map(|n| n.expressions.decay)
                                    .unwrap_or(hover_norm * 100.0);
                                format!("DEC {:+.0}%", v)
                            }
                            ParameterTab::VibratoLength => {
                                let v = note_under_cursor
                                    .map(|n| n.vibrato.length_pct)
                                    .unwrap_or(hover_norm * 100.0);
                                format!("VIBL {:.0}%", v)
                            }
                            ParameterTab::VibratoDepth => {
                                let v = note_under_cursor
                                    .map(|n| n.vibrato.depth_cents)
                                    .unwrap_or(hover_norm * 200.0);
                                format!("VIBD {:.0} c", v)
                            }
                            ParameterTab::VibratoPeriod => {
                                let v = note_under_cursor
                                    .map(|n| n.vibrato.period_ms)
                                    .unwrap_or(250.0 + hover_norm * 200.0);
                                format!("VIBP {:.0} ms", v)
                            }
                        };

                        let badge_text = if secondary_active {
                            if let Some(kf) = kf_under_cursor {
                                format!("{} ({}) · {} [RESET]", kf.lyric, kf.alias, val_str)
                            } else {
                                format!("{} [RESET]", val_str)
                            }
                        } else if is_shift_down {
                            if let Some(kf) = kf_under_cursor {
                                format!("{} ({}) · {} [SHIFT LOCK]", kf.lyric, kf.alias, val_str)
                            } else {
                                format!("{} [SHIFT LOCK]", val_str)
                            }
                        } else if let Some(kf) = kf_under_cursor {
                            format!("{} ({}) · {}", kf.lyric, kf.alias, val_str)
                        } else {
                            val_str
                        };

                        let badge_width = (badge_text.len() as f32 * 5.8 + 16.0).max(90.0);
                        let badge_pos = Pos2::new(
                            (cursor_mpos.x + 12.0).min(graph_rect.max.x - (badge_width + 5.0)),
                            (hover_curve_y - 18.0).max(graph_rect.min.y + 6.0),
                        );

                        // Pill background do tooltip
                        painter.rect_filled(
                            Rect::from_min_size(badge_pos, Vec2::new(badge_width, 16.0)),
                            Rounding::same(3.0),
                            Color32::from_rgba_unmultiplied(20, 20, 30, 220),
                        );
                        painter.rect_stroke(
                            Rect::from_min_size(badge_pos, Vec2::new(badge_width, 16.0)),
                            Rounding::same(3.0),
                            Stroke::new(1.0_f32, Color32::from_rgb(0, 255, 157)),
                        );
                        painter.text(
                            Pos2::new(badge_pos.x + badge_width * 0.5, badge_pos.y + 8.0),
                            egui::Align2::CENTER_CENTER,
                            badge_text,
                            egui::FontId::proportional(8.5),
                            Color32::from_rgb(0, 255, 157),
                        );
                    }
                },
            );
        });
    state.drawer_height = panel_response.response.rect.height().clamp(60.0, 750.0);
}
