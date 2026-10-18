use super::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct TimelinePlaybackStatus<'a> {
    pub is_rendering: bool,
    pub is_playing: bool,
    pub render_progress: f32,
    pub render_event: Option<crate::renderer::RenderProgressEvent>,
    pub elapsed_seconds: f64,
    pub is_exporting: bool,
    pub language: crate::config::AppLanguage,
    pub title: &'a str,
    pub message: &'a str,
    pub engine: &'a str,
    pub total_duration_ms: f64,
    pub has_error: bool,
}

impl Default for TimelinePlaybackStatus<'static> {
    fn default() -> Self {
        Self {
            is_rendering: false,
            is_playing: false,
            render_progress: 1.0,
            render_event: None,
            elapsed_seconds: 0.0,
            is_exporting: false,
            language: crate::config::AppLanguage::default(),
            title: "Pronto",
            message: "Pronto",
            engine: "",
            total_duration_ms: 0.0,
            has_error: false,
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn draw(
    ui: &mut egui::Ui,
    notes: &[UNote],
    state: &mut PianoRollState,
    theme: &ThemeConfig,
    keyboard_width: f32,
    timeline_scroll_x: f32,
    total_canvas_ms: f64,
    playback: TimelinePlaybackStatus<'_>,
    on_playhead_scrubbed: &mut dyn FnMut(f64),
) {
    // Timeline Mini-Map (Radar)
    if state.show_minimap {
        let (minimap_rect, minimap_resp) = ui.allocate_exact_size(
            Vec2::new(ui.available_width(), state.minimap_height),
            Sense::click_and_drag(),
        );
        let m_painter = ui.painter_at(minimap_rect);
        m_painter.rect_filled(
            minimap_rect,
            theme.ui_rounding(),
            theme.c32_alpha(theme.bg_canvas, 0.94),
        );
        m_painter.line_segment(
            [
                Pos2::new(minimap_rect.min.x, minimap_rect.max.y),
                Pos2::new(minimap_rect.max.x, minimap_rect.max.y),
            ],
            Stroke::new(1.0_f32, theme.grid_line_bar_c32().linear_multiply(0.72)),
        );

        let animation_policy = crate::gui::animation::AnimationPolicy {
            enabled: state.ui_animations_enabled,
            reduce_motion: state.ui_reduce_motion,
            speed: state.ui_animation_speed,
            frame_time_ema_ms: if state.animation_quality_reduced {
                crate::gui::animation::HIGH_FRAME_TIME_MS
            } else {
                16.0
            },
        };
        let animation_time = ui.input(|input| input.time) as f32;
        let pulse = if animation_policy.enabled {
            0.5 + 0.5
                * animation_policy.pulse_amplitude()
                * (animation_time * std::f32::consts::TAU * 0.9 * animation_policy.visual_speed())
                    .sin()
        } else {
            0.0
        };

        // Corner label
        m_painter.text(
            Pos2::new(minimap_rect.min.x + 6.0, minimap_rect.center().y),
            egui::Align2::LEFT_CENTER,
            "RADAR",
            egui::FontId::monospace(8.0),
            theme.text_muted_c32(),
        );

        let map_span_x = (minimap_rect.width() - keyboard_width).max(10.0);
        let map_origin_x = minimap_rect.min.x + keyboard_width;

        // Draw miniature notes in radar
        for n in notes.iter() {
            let n_x_norm = (n.position_ms / total_canvas_ms).clamp(0.0, 1.0) as f32;
            let n_w_norm = ((n.duration_ms / total_canvas_ms).clamp(0.001, 1.0)) as f32;
            let nx = map_origin_x + n_x_norm * map_span_x;
            let nw = (n_w_norm * map_span_x).max(2.0);

            let k_norm = (state.max_midi.saturating_sub(n.midi_key()) as f32
                / (state.max_midi - state.min_midi).max(1) as f32)
                .clamp(0.0, 1.0);
            let ny = minimap_rect.min.y + 2.0 + k_norm * (state.minimap_height - 6.0);

            m_painter.rect_filled(
                Rect::from_min_size(Pos2::new(nx, ny), Vec2::new(nw, 2.0)),
                Rounding::same(0.5),
                theme.c32_alpha(theme.note_fill, 0.75),
            );
        }

        // Viewport window overlay
        let visible_w = (ui.available_width() - keyboard_width).max(10.0);
        // Keep the state in the same range as the ScrollArea. Without this,
        // dragging against the physical window edge can repeatedly alternate
        // between the radar's unconstrained target and ScrollArea's clamped
        // value, producing a visible shake.
        let max_scroll_x = (total_canvas_ms * state.px_per_ms as f64) as f32 - visible_w;
        let max_scroll_x = max_scroll_x.max(0.0);
        let vp_start_norm =
            ((timeline_scroll_x / state.px_per_ms) as f64 / total_canvas_ms).clamp(0.0, 1.0) as f32;
        let vp_dur_ms = (visible_w / state.px_per_ms) as f64;
        let vp_w_norm = ((vp_dur_ms / total_canvas_ms).clamp(0.01, 1.0)) as f32;

        let vp_rect = Rect::from_min_max(
            Pos2::new(
                map_origin_x + vp_start_norm * map_span_x,
                minimap_rect.min.y + 1.0,
            ),
            Pos2::new(
                map_origin_x + (vp_start_norm + vp_w_norm) * map_span_x,
                minimap_rect.max.y - 1.0,
            ),
        );
        let is_vp_hovered = minimap_resp
            .hover_pos()
            .is_some_and(|pos| vp_rect.contains(pos));

        if is_vp_hovered {
            if minimap_resp.dragged() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
            } else {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
            }
        }

        let vp_fill = if is_vp_hovered || minimap_resp.dragged() {
            theme.c32_alpha(theme.accent_color, 0.35)
        } else {
            theme.focus_glow_c32()
        };
        let vp_stroke = if is_vp_hovered || minimap_resp.dragged() {
            Stroke::new(1.8_f32, theme.accent_c32())
        } else {
            Stroke::new(1.0_f32, theme.accent_c32())
        };

        m_painter.rect_filled(vp_rect, theme.ui_rounding(), vp_fill);
        m_painter.rect_stroke(vp_rect, theme.ui_rounding(), vp_stroke);

        // Playhead in minimap: a strong line, a small tracker head, and a
        // restrained halo make the shared transport clock easy to follow.
        let playhead_norm = (state.playhead_ms / total_canvas_ms).clamp(0.0, 1.0) as f32;
        let playhead_map_x = map_origin_x + playhead_norm * map_span_x;
        if state.is_playing && animation_policy.enabled {
            if let Some(interval) = animation_policy.interval() {
                ui.ctx().request_repaint_after(interval);
            }
        }
        m_painter.line_segment(
            [
                Pos2::new(playhead_map_x, minimap_rect.min.y),
                Pos2::new(playhead_map_x, minimap_rect.max.y),
            ],
            Stroke::new(
                3.0 + pulse * 2.0,
                theme.playhead_c32().linear_multiply(0.16 + pulse * 0.12),
            ),
        );
        m_painter.line_segment(
            [
                Pos2::new(playhead_map_x, minimap_rect.min.y),
                Pos2::new(playhead_map_x, minimap_rect.max.y),
            ],
            Stroke::new(1.2 + pulse * 0.8, theme.playhead_c32()),
        );
        m_painter.add(egui::Shape::convex_polygon(
            vec![
                Pos2::new(playhead_map_x - 4.0, minimap_rect.min.y),
                Pos2::new(playhead_map_x + 4.0, minimap_rect.min.y),
                Pos2::new(playhead_map_x, minimap_rect.min.y + 6.0),
            ],
            theme.playhead_c32(),
            Stroke::NONE,
        ));

        // A drag that starts inside the viewport keeps the existing viewport
        // navigation. A simple click anywhere on the timeline scrubs the
        // shared playhead instead.
        if minimap_resp.drag_started() {
            state.minimap_drag_scroll_origin = Some(state.horizontal_scroll_offset);
            state.minimap_navigation_active = true;
            state.horizontal_follow_user_override = true;
            state.auto_scroll_mode = crate::gui::types::AutoScrollMode::Off;
        }
        if minimap_resp.dragged() && state.minimap_drag_scroll_origin.is_some() {
            state.minimap_navigation_active = true;
            state.horizontal_follow_user_override = true;
            state.auto_scroll_mode = crate::gui::types::AutoScrollMode::Off;
            // `drag_delta` belongs to the response that claimed the pointer;
            // unlike the current pointer position, it remains valid after the
            // pointer crosses the radar's edge. Requiring the pointer to stay
            // inside the thin radar strip was what made the viewport appear
            // locked as soon as a trackpad drag escaped that strip.
            let origin = state
                .minimap_drag_scroll_origin
                .unwrap_or(timeline_scroll_x);
            let delta_ms = (minimap_resp.drag_delta().x / map_span_x) as f64 * total_canvas_ms;
            state.horizontal_scroll_offset =
                (origin + (delta_ms * state.px_per_ms as f64) as f32).clamp(0.0, max_scroll_x);
            ui.ctx().request_repaint();
        } else if minimap_resp.clicked() {
            state.minimap_navigation_active = true;
            state.horizontal_follow_user_override = true;
            state.auto_scroll_mode = crate::gui::types::AutoScrollMode::Off;
            if let Some(mpos) = minimap_resp.interact_pointer_pos() {
                if mpos.x >= map_origin_x && mpos.x <= map_origin_x + map_span_x {
                    let click_norm = ((mpos.x - map_origin_x) / map_span_x).clamp(0.0, 1.0) as f64;
                    let scrubbed_t = click_norm * total_canvas_ms;
                    state.playhead_ms = scrubbed_t;
                    on_playhead_scrubbed(scrubbed_t);
                }
            }
        } else if !ui.input(|input| input.pointer.primary_down()) {
            // The release may happen outside the radar. Use the global button
            // state so a completed drag always drops its saved origin.
            state.minimap_drag_scroll_origin = None;
        }
    }

    // Keep the render/playhead bar visible even when idle or the minimap is
    // hidden, so its location and current timeline position stay discoverable.
    let status_animation_policy = crate::gui::animation::AnimationPolicy {
        enabled: state.ui_animations_enabled,
        reduce_motion: state.ui_reduce_motion,
        speed: state.ui_animation_speed,
        frame_time_ema_ms: if state.animation_quality_reduced {
            crate::gui::animation::HIGH_FRAME_TIME_MS
        } else {
            16.0
        },
    };
    draw_playback_bar(ui, state, theme, playback, status_animation_policy, 0.0);
}

fn draw_playback_bar(
    ui: &mut egui::Ui,
    state: &PianoRollState,
    theme: &ThemeConfig,
    playback: TimelinePlaybackStatus<'_>,
    animation_policy: crate::gui::animation::AnimationPolicy,
    pulse: f32,
) {
    if playback.is_rendering && animation_policy.enabled && !animation_policy.reduce_motion {
        if let Some(interval) = animation_policy.interval() {
            ui.ctx().request_repaint_after(interval);
        }
    }
    let height = 34.0;
    let (bar_rect, _) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::hover());
    let painter = ui.painter_at(bar_rect);
    let red = ui.visuals().error_fg_color;
    let accent = theme.accent_c32();
    let muted = theme.text_muted_c32();
    let text_color = theme.text_primary_c32();
    painter.rect_filled(bar_rect, theme.ui_rounding(), theme.card_bg_c32());
    painter.rect_stroke(
        bar_rect,
        theme.ui_rounding(),
        if playback.has_error {
            Stroke::new(1.25_f32, red)
        } else {
            theme.card_stroke()
        },
    );

    let event = playback.render_event;
    let stage = event.map(|event| event.stage);
    let (label, stage_detail) =
        if playback.has_error || stage == Some(crate::renderer::RenderStage::Failed) {
            (playback.language.tr("ERRO", "ERROR"), playback.message)
        } else if stage == Some(crate::renderer::RenderStage::Cancelled) {
            (
                playback.language.tr("CANCELADO", "CANCELLED"),
                playback.message,
            )
        } else if stage == Some(crate::renderer::RenderStage::Complete) {
            (
                playback.language.tr("CONCLUÍDO", "COMPLETE"),
                playback.message,
            )
        } else if playback.is_rendering {
            let label = if playback.is_exporting {
                playback.language.tr("EXPORTANDO", "EXPORTING")
            } else {
                playback.language.tr("PRÉVIA", "PREVIEW")
            };
            let detail = match stage {
                Some(crate::renderer::RenderStage::Rendering) => {
                    playback.language.tr("Sintetizando", "Synthesizing")
                }
                Some(crate::renderer::RenderStage::Mixing) => {
                    playback.language.tr("Mixando faixas", "Mixing tracks")
                }
                Some(crate::renderer::RenderStage::Finalizing) => {
                    playback.language.tr("Finalizando", "Finalizing")
                }
                _ => playback.language.tr("Preparando áudio", "Preparing audio"),
            };
            (label, detail)
        } else {
            (
                playback.language.tr("PRONTO", "READY"),
                playback.language.tr("Renderização pronta", "Render ready"),
            )
        };
    let label_color = if playback.has_error || stage == Some(crate::renderer::RenderStage::Failed) {
        red
    } else if stage == Some(crate::renderer::RenderStage::Cancelled) {
        muted
    } else {
        accent
    };

    let badge_rect = Rect::from_min_size(
        Pos2::new(bar_rect.min.x + 7.0, bar_rect.min.y + 6.0),
        Vec2::new(78.0, 21.0),
    );
    painter.rect_filled(badge_rect, Rounding::same(5.0), theme.bg_panel_c32());
    painter.text(
        badge_rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(9.0),
        label_color,
    );

    let narrow = bar_rect.width() < 560.0;
    let bar_left = bar_rect.min.x + if narrow { 106.0 } else { 174.0 };
    let bar_right = (bar_rect.max.x - 88.0).max(bar_left + 28.0);
    let bar_height = 7.0;
    let progress_rect = Rect::from_min_max(
        Pos2::new(bar_left, bar_rect.max.y - 11.0),
        Pos2::new(bar_right, bar_rect.max.y - 4.0),
    );
    painter.rect_filled(progress_rect, Rounding::same(4.0), theme.bg_canvas_c32());
    let progress = if playback.is_playing {
        Some(if playback.total_duration_ms > 0.0 {
            (state.playhead_ms / playback.total_duration_ms).clamp(0.0, 1.0) as f32
        } else {
            0.0
        })
    } else if playback.is_rendering {
        event
            .and_then(|event| event.fraction)
            .or_else(|| (stage == Some(crate::renderer::RenderStage::Complete)).then_some(1.0))
    } else {
        Some(if playback.total_duration_ms > 0.0 {
            (state.playhead_ms / playback.total_duration_ms).clamp(0.0, 1.0) as f32
        } else {
            0.0
        })
    };
    if playback.is_rendering && progress.is_none() {
        if animation_policy.enabled && !animation_policy.reduce_motion {
            if let Some(interval) = animation_policy.interval() {
                ui.ctx().request_repaint_after(interval);
            }
            let width = progress_rect.width() * 0.24;
            let phase =
                (ui.input(|input| input.time) as f32 * animation_policy.visual_speed() * 0.65)
                    .rem_euclid(1.0);
            let left = progress_rect.left() + (progress_rect.width() - width) * phase;
            painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(left, progress_rect.top()),
                    Vec2::new(width, bar_height),
                ),
                Rounding::same(4.0),
                accent.linear_multiply(0.9 + pulse * 0.1),
            );
        } else {
            painter.circle_filled(progress_rect.center(), 2.5, accent);
        }
    } else if let Some(progress) = progress {
        let fill_width = progress_rect.width() * progress.clamp(0.0, 1.0);
        if fill_width > 0.0 {
            painter.rect_filled(
                Rect::from_min_size(progress_rect.min, Vec2::new(fill_width, bar_height)),
                Rounding::same(4.0),
                if playback.has_error {
                    red
                } else {
                    accent.linear_multiply(0.9 + pulse * 0.1)
                },
            );
        }
    }

    let percent = progress
        .map(|value| format!("{:.0}%", value * 100.0))
        .unwrap_or_else(|| playback.language.tr("···", "···").to_string());
    painter.text(
        Pos2::new(bar_rect.max.x - 8.0, bar_rect.center().y - 4.0),
        egui::Align2::RIGHT_CENTER,
        percent,
        egui::FontId::monospace(9.5),
        label_color,
    );

    let count_detail = event
        .and_then(|event| event.completed.zip(event.total))
        .map(|(completed, total)| format!("{completed}/{total}"));
    let detail = if playback.is_playing && !playback.is_rendering {
        playback.message.to_string()
    } else if let Some(count) = count_detail {
        format!(
            "{stage_detail} · {count} {}",
            playback.language.tr("faixas", "tracks")
        )
    } else {
        stage_detail.to_string()
    };
    painter.text(
        Pos2::new(bar_left, bar_rect.min.y + 4.0),
        egui::Align2::LEFT_TOP,
        detail,
        egui::FontId::proportional(8.5),
        text_color,
    );
    if !narrow && !playback.engine.is_empty() {
        painter.text(
            Pos2::new(bar_rect.max.x - 8.0, bar_rect.max.y - 3.0),
            egui::Align2::RIGHT_BOTTOM,
            format!(
                "{} · {:02}:{:02}",
                playback.engine,
                playback.elapsed_seconds as u64 / 60,
                playback.elapsed_seconds as u64 % 60
            ),
            egui::FontId::proportional(8.0),
            muted,
        );
    }
}
