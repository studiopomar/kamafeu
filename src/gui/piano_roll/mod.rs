use crate::dsp::pitch::{midi_to_freq, midi_to_note_name};
use crate::dsp::pitch_bend::PitchBendSolver;
use crate::gui::phoneme_palette::PhonemePaletteState;
use crate::gui::theme::MelodyneTheme;
use crate::gui::types::{AutoScrollMode, EditTool, GridSnapOption, PitchSubTool};
use crate::oto::Voicebank;
use crate::project::model::{UNote, UPitchBendPoint};
use eframe::egui::{self, Color32, Key, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use std::collections::HashSet;
pub mod grid;
pub mod state;
pub use grid::*;
pub use state::*;

pub fn draw_piano_roll(
    ui: &mut egui::Ui,
    notes: &mut Vec<UNote>,
    state: &mut PianoRollState,
    voicebank: Option<&Voicebank>,
    phoneme_state: &mut PhonemePaletteState,
    snap_option: GridSnapOption,
    bpm: f64,
    phonemizer_mode: crate::phonemizer::PhonemizerMode,
    on_preview_freq: &mut dyn FnMut(f64),
    on_before_change: &mut dyn FnMut(),
    on_note_changed: &mut dyn FnMut(),
    on_playhead_scrubbed: &mut dyn FnMut(f64),
    on_edit_oto_alias: &mut dyn FnMut(&str, &str),
) {
    let was_editing_lyric = state.editing_lyric_index.is_some();
    let key_count = (state.max_midi - state.min_midi + 1) as usize;
    let ruler_height = 28.0f32;
    let param_drawer_height = if state.show_parameters_drawer {
        state.drawer_height
    } else {
        0.0f32
    };
    let _total_height = key_count as f32 * state.row_height + ruler_height + param_drawer_height;
    let keyboard_width = 65.0f32;
    let (ctrl_pressed, alt_pressed, cmd_pressed) = ui.input(|i| {
        (
            i.modifiers.ctrl,
            i.modifiers.alt,
            i.modifiers.command || i.modifiers.mac_cmd,
        )
    });
    let is_mod_zoom = ctrl_pressed || alt_pressed || cmd_pressed;

    if is_mod_zoom {
        let wheel_delta = ui.input(|i| {
            let mut dy = i.smooth_scroll_delta.y;
            if dy.abs() < 1e-3 {
                dy = i.raw_scroll_delta.y;
            }
            if dy.abs() < 1e-3 {
                dy = i.smooth_scroll_delta.x;
                if dy.abs() < 1e-3 {
                    dy = i.raw_scroll_delta.x;
                }
            }
            if dy.abs() < 1e-3 && (i.zoom_delta() - 1.0).abs() > 1e-3 {
                dy = (i.zoom_delta() - 1.0) * 80.0;
            }
            for ev in &i.events {
                if let egui::Event::MouseWheel { delta, .. } = ev {
                    if delta.y.abs() > dy.abs() {
                        dy = delta.y;
                    }
                } else if let egui::Event::Zoom(factor) = ev {
                    let derived = (*factor - 1.0) * 80.0;
                    if derived.abs() > dy.abs() {
                        dy = derived;
                    }
                }
            }
            dy
        });

        if wheel_delta != 0.0 {
            let zoom_factor = 1.05f32.powf(wheel_delta * 0.05);

            let mouse_pos = ui.input(|i| {
                i.pointer
                    .hover_pos()
                    .or(i.pointer.interact_pos())
                    .or(i.pointer.latest_pos())
            });

            if alt_pressed {
                let old_row_h = state.row_height;
                let new_row_h = (old_row_h * zoom_factor).clamp(12.0, 70.0);

                if let Some(mpos) = mouse_pos {
                    let grid_screen_top = ui.available_rect_before_wrap().min.y + ruler_height;
                    let mouse_y_in_viewport = (mpos.y - grid_screen_top).max(0.0);
                    let canvas_y_under_mouse = mouse_y_in_viewport + state.vertical_scroll_offset;
                    let key_ratio_under_mouse = canvas_y_under_mouse / old_row_h;
                    let new_canvas_y = key_ratio_under_mouse * new_row_h;
                    state.vertical_scroll_offset = (new_canvas_y - mouse_y_in_viewport).max(0.0);
                }

                state.row_height = new_row_h;
            } else if ctrl_pressed || cmd_pressed {
                let old_px_per_ms = state.px_per_ms;
                let new_px_per_ms = (old_px_per_ms * zoom_factor).clamp(0.04, 2.5);

                if let Some(mpos) = mouse_pos {
                    let canvas_screen_left = ui.available_rect_before_wrap().min.x + keyboard_width;
                    let mouse_x_in_viewport = (mpos.x - canvas_screen_left).max(0.0);
                    let time_under_mouse = (mouse_x_in_viewport + state.horizontal_scroll_offset)
                        as f64
                        / old_px_per_ms as f64;
                    let new_mouse_x_in_canvas = (time_under_mouse * new_px_per_ms as f64) as f32;
                    state.horizontal_scroll_offset =
                        (new_mouse_x_in_canvas - mouse_x_in_viewport).max(0.0);
                }

                state.px_per_ms = new_px_per_ms;
            }
        }

        ui.ctx().input_mut(|i| {
            i.smooth_scroll_delta = egui::Vec2::ZERO;
            i.raw_scroll_delta = egui::Vec2::ZERO;
        });
    }

    if state.is_playing {
        let playhead_x = (state.playhead_ms * state.px_per_ms as f64) as f32;
        let visible_width = (ui.available_width() - keyboard_width).max(100.0);

        match state.auto_scroll_mode {
            AutoScrollMode::StationaryCursor => {
                let target_x = (playhead_x - visible_width * 0.30).max(0.0);
                state.horizontal_scroll_offset = target_x;
            }
            AutoScrollMode::PageScroll => {
                let curr_offset = state.horizontal_scroll_offset;
                if playhead_x > curr_offset + visible_width * 0.92 || playhead_x < curr_offset {
                    let target_x = (playhead_x - visible_width * 0.08).max(0.0);
                    state.horizontal_scroll_offset = target_x;
                }
            }
            AutoScrollMode::Off => {}
        }
    }

    let timeline_scroll_x = state.horizontal_scroll_offset;
    let max_note_end_ms = notes
        .iter()
        .map(|note| note.position_ms + note.duration_ms)
        .fold(0.0f64, f64::max);
    let total_canvas_ms = (max_note_end_ms + 30_000.0).max(60_000.0);

    {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        notes.len().hash(&mut hasher);
        for n in notes.iter() {
            n.lyric.hash(&mut hasher);
            n.pitch.hash(&mut hasher);
            n.position_ms.to_bits().hash(&mut hasher);
            n.duration_ms.to_bits().hash(&mut hasher);
            n.phoneme_durations_ms.len().hash(&mut hasher);
            for d in &n.phoneme_durations_ms {
                d.to_bits().hash(&mut hasher);
            }
            n.expressions
                .consonant_timing_offset_ms
                .to_bits()
                .hash(&mut hasher);
            n.expressions.preutter_offset_ms.to_bits().hash(&mut hasher);
            n.expressions.overlap_offset_ms.to_bits().hash(&mut hasher);
            n.expressions.consonant_velocity.to_bits().hash(&mut hasher);
        }
        if let Some(vb) = voicebank {
            vb.name.hash(&mut hasher);
        }
        phonemizer_mode.hash(&mut hasher);
        let new_hash = hasher.finish();
        if new_hash != state.phoneme_cache_hash {
            state.phoneme_cache_hash = new_hash;
            state.phoneme_cache = vec![String::new(); notes.len()];
            state.note_phonemes_cache = vec![Vec::new(); notes.len()];
            state.oto_consonant_cache = vec![0.0; notes.len()];
            state.oto_preutter_cache = vec![0.0; notes.len()];
            state.oto_overlap_cache = vec![0.0; notes.len()];
            if let Some(vb) = voicebank {
                let phones = crate::phonemizer::JapanesePhonemizer::apply_phonemizer(
                    notes,
                    vb,
                    phonemizer_mode,
                );
                for p in phones {
                    if p.note_index < state.phoneme_cache.len() {
                        let is_first_phone = state.note_phonemes_cache[p.note_index].is_empty();
                        if is_first_phone {
                            if let Some(entry) = vb.find_entry(&p.lyric, &p.pitch) {
                                state.oto_consonant_cache[p.note_index] = entry.consonant.max(0.0);
                                state.oto_preutter_cache[p.note_index] =
                                    entry.preutterance.max(0.0);
                                state.oto_overlap_cache[p.note_index] = entry.overlap;
                            }
                        }
                        if state.phoneme_cache[p.note_index].is_empty() {
                            state.phoneme_cache[p.note_index] = p.lyric.clone();
                        } else {
                            state.phoneme_cache[p.note_index] =
                                format!("{} [{}]", state.phoneme_cache[p.note_index], p.lyric);
                        }
                        let note_pos = notes[p.note_index].position_ms;
                        let rel_pos = p.position_ms - note_pos;
                        state.note_phonemes_cache[p.note_index].push((
                            p.lyric,
                            rel_pos,
                            p.duration_ms,
                        ));
                    }
                }
            }
        }
    }

    let (ruler_rect, _ruler_resp) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), ruler_height),
        Sense::click_and_drag(),
    );

    let ruler_painter = ui.painter_at(ruler_rect);
    ruler_painter.rect_filled(ruler_rect, Rounding::ZERO, MelodyneTheme::BG_HEADER);
    ruler_painter.line_segment(
        [
            Pos2::new(ruler_rect.min.x, ruler_rect.max.y),
            Pos2::new(ruler_rect.max.x, ruler_rect.max.y),
        ],
        Stroke::new(1.5_f32, MelodyneTheme::GRID_LINE_BAR),
    );

    let corner_rect = Rect::from_min_max(
        Pos2::new(ruler_rect.min.x, ruler_rect.min.y),
        Pos2::new(ruler_rect.min.x + keyboard_width, ruler_rect.max.y),
    );
    ruler_painter.rect_filled(corner_rect, Rounding::ZERO, MelodyneTheme::BG_PANEL);
    ruler_painter.text(
        Pos2::new(
            ruler_rect.min.x + 8.0,
            ruler_rect.min.y + ruler_height * 0.5,
        ),
        egui::Align2::LEFT_CENTER,
        "RULER",
        egui::FontId::proportional(10.0),
        MelodyneTheme::TEXT_GOLD_LABEL,
    );

    let beat_ms = 60000.0 / bpm;
    let bar_ms = beat_ms * 4.0;
    let ruler_visible_width = (ruler_rect.width() - keyboard_width).max(1.0);
    let ruler_start_ms = (timeline_scroll_x / state.px_per_ms).max(0.0) as f64;
    let ruler_end_ms = ruler_start_ms + (ruler_visible_width / state.px_per_ms) as f64 + bar_ms;
    let first_measure = (ruler_start_ms / bar_ms).floor().max(0.0) as usize;
    let mut measure_idx = first_measure + 1;
    let mut m_ms = first_measure as f64 * bar_ms;

    while m_ms < total_canvas_ms.min(ruler_end_ms) {
        let x = ruler_rect.min.x + keyboard_width + (m_ms * state.px_per_ms as f64) as f32
            - timeline_scroll_x;
        if x >= ruler_rect.min.x + keyboard_width && x <= ruler_rect.max.x + 200.0 {
            ruler_painter.line_segment(
                [
                    Pos2::new(x, ruler_rect.min.y + 12.0),
                    Pos2::new(x, ruler_rect.max.y),
                ],
                Stroke::new(1.5_f32, MelodyneTheme::ACCENT_GOLD),
            );

            ruler_painter.text(
                Pos2::new(x + 4.0, ruler_rect.min.y + 4.0),
                egui::Align2::LEFT_TOP,
                format!("m{}", measure_idx),
                egui::FontId::proportional(11.0),
                MelodyneTheme::TEXT_GOLD_LABEL,
            );
        }

        measure_idx += 1;
        m_ms += bar_ms;
    }

    let ruler_playhead_x =
        ruler_rect.min.x + keyboard_width + (state.playhead_ms * state.px_per_ms as f64) as f32
            - timeline_scroll_x;
    if ruler_playhead_x >= ruler_rect.min.x + keyboard_width && ruler_playhead_x <= ruler_rect.max.x
    {
        let tri = vec![
            Pos2::new(ruler_playhead_x - 3.5, ruler_rect.min.y + 1.0),
            Pos2::new(ruler_playhead_x + 3.5, ruler_rect.min.y + 1.0),
            Pos2::new(ruler_playhead_x, ruler_rect.min.y + 11.0),
        ];
        ruler_painter.add(egui::Shape::convex_polygon(
            tri,
            Color32::from_rgb(255, 65, 85),
            Stroke::new(0.8_f32, Color32::WHITE),
        ));
        ruler_painter.line_segment(
            [
                Pos2::new(ruler_playhead_x, ruler_rect.min.y + 10.0),
                Pos2::new(ruler_playhead_x, ruler_rect.max.y),
            ],
            Stroke::new(1.0_f32, Color32::from_rgb(255, 65, 85)),
        );
    }

    let ruler_response = ui.interact(
        ruler_rect,
        ui.make_persistent_id("piano_roll_ruler_interaction"),
        Sense::click_and_drag(),
    );

    if ruler_response.clicked() || ruler_response.dragged() {
        if let Some(mpos) = ruler_response.interact_pointer_pos() {
            state.is_scrubbing_ruler = true;
            let raw_t = (mpos.x - (ruler_rect.min.x + keyboard_width) + timeline_scroll_x) as f64
                / state.px_per_ms as f64;
            let scrubbed_t = apply_snap(raw_t.max(0.0), snap_option, bpm);
            state.playhead_ms = scrubbed_t;
            on_playhead_scrubbed(scrubbed_t);

            let playhead_canvas_x = (scrubbed_t * state.px_per_ms as f64) as f32;
            let visible_w = (ui.available_width() - keyboard_width).max(100.0);
            if playhead_canvas_x < state.horizontal_scroll_offset + 50.0 {
                state.horizontal_scroll_offset = (playhead_canvas_x - 50.0).max(0.0);
            } else if playhead_canvas_x > state.horizontal_scroll_offset + visible_w - 70.0 {
                state.horizontal_scroll_offset = (playhead_canvas_x - visible_w + 70.0).max(0.0);
            }
        }
    }

    if state.is_scrubbing_ruler && !ui.input(|i| i.pointer.primary_down()) {
        state.is_scrubbing_ruler = false;
    }

    crate::gui::phoneme_ruler::draw_phoneme_ruler(
        ui,
        state,
        notes,
        ruler_rect,
        keyboard_width,
        timeline_scroll_x,
        bpm,
        on_before_change,
        on_note_changed,
        on_edit_oto_alias,
    );

    if state.show_envelope_handles {
        let panel_response = egui::TopBottomPanel::bottom("bottom_expanded_envelope_editor")
            .resizable(true)
            .height_range(130.0..=500.0)
            .default_height(state.drawer_height.max(180.0))
            .frame(
                egui::Frame::none()
                    .fill(MelodyneTheme::BG_PANEL)
                    .stroke(Stroke::new(1.5_f32, Color32::from_rgb(255, 90, 195))),
            )
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new("🎚 ENVELOPES DA FRASE — linha do tempo")
                            .strong()
                            .color(Color32::from_rgb(255, 120, 205)),
                    );
                    ui.label(
                        egui::RichText::new(
                            "Arraste horizontalmente para o tempo e verticalmente para o volume",
                        )
                        .size(9.5)
                        .color(MelodyneTheme::TEXT_MUTED),
                    );
                    if ui.button("Fechar").clicked() {
                        state.show_envelope_handles = false;
                        state.dragging_envelope_pt = None;
                    }
                });

                let graph_size = Vec2::new(
                    ui.available_width(),
                    (ui.available_height() - 4.0).max(90.0),
                );
                let (graph_rect, response) =
                    ui.allocate_exact_size(graph_size, Sense::click_and_drag());
                let painter = ui.painter_at(graph_rect);
                painter.rect_filled(graph_rect, Rounding::same(4.0), MelodyneTheme::BG_CANVAS);
                painter.rect_stroke(
                    graph_rect,
                    Rounding::same(4.0),
                    Stroke::new(1.0_f32, MelodyneTheme::GRID_LINE_BAR),
                );
                let inner = graph_rect.shrink2(Vec2::new(6.0, 16.0));
                for level in [0.0f32, 0.25, 0.5, 0.75, 1.0] {
                    let y = inner.max.y - level * inner.height();
                    painter.line_segment(
                        [Pos2::new(inner.min.x, y), Pos2::new(inner.max.x, y)],
                        Stroke::new(0.7_f32, MelodyneTheme::GRID_LINE_SUB),
                    );
                    painter.text(
                        Pos2::new(inner.min.x + 3.0, y),
                        egui::Align2::LEFT_CENTER,
                        format!("{:.0}%", level * 100.0),
                        egui::FontId::proportional(8.5),
                        MelodyneTheme::TEXT_MUTED,
                    );
                }

                let timeline_origin_x = ruler_rect.min.x + keyboard_width - timeline_scroll_x;
                let pointer = response.interact_pointer_pos();
                let mut hovered_handle = None;

                for (note_index, note) in notes.iter().enumerate() {
                    let duration = note.duration_ms.max(1.0);
                    let note_start_x =
                        timeline_origin_x + (note.position_ms * state.px_per_ms as f64) as f32;
                    let note_end_x = note_start_x + (duration * state.px_per_ms as f64) as f32;
                    if note_end_x < graph_rect.min.x || note_start_x > graph_rect.max.x {
                        continue;
                    }
                    let selected = state.selected_note_index == Some(note_index)
                        || state.selected_note_indices.contains(&note_index);
                    let env = &note.envelope;
                    let points = [
                        (env.p1, env.v1),
                        (env.p1 + env.p2, env.v2),
                        (env.p1 + env.p2 + env.p3, env.v3),
                        ((duration - env.p4).max(0.0), env.v4),
                        ((duration - env.p4 + env.p5).max(0.0), env.v5),
                    ];
                    let screen_points = points.map(|(time, volume)| {
                        Pos2::new(
                            note_start_x + (time * state.px_per_ms as f64) as f32,
                            inner.max.y - (volume / 100.0).clamp(0.0, 1.0) as f32 * inner.height(),
                        )
                    });

                    let note_band = Rect::from_min_max(
                        Pos2::new(note_start_x.max(graph_rect.min.x), inner.min.y),
                        Pos2::new(note_end_x.min(graph_rect.max.x), inner.max.y),
                    );
                    painter.rect_filled(
                        note_band,
                        Rounding::ZERO,
                        if selected {
                            Color32::from_rgba_unmultiplied(0, 180, 220, 24)
                        } else {
                            Color32::from_rgba_unmultiplied(40, 80, 130, 14)
                        },
                    );
                    painter.text(
                        Pos2::new(note_start_x + 4.0, inner.min.y + 2.0),
                        egui::Align2::LEFT_TOP,
                        &note.lyric,
                        egui::FontId::proportional(9.0),
                        if selected {
                            Color32::from_rgb(0, 235, 255)
                        } else {
                            MelodyneTheme::TEXT_MUTED
                        },
                    );

                    let adjacent_previous = note_index.checked_sub(1).and_then(|previous_index| {
                        notes.get(previous_index).filter(|previous| {
                            let gap =
                                note.position_ms - (previous.position_ms + previous.duration_ms);
                            gap.abs() <= 200.0
                        })
                    });
                    let automatic_crossfade = state
                        .oto_overlap_cache
                        .get(note_index)
                        .copied()
                        .unwrap_or(0.0)
                        .abs()
                        .max(env.p2.min(45.0));
                    let visual_crossfade_ms = if env.crossfade_ms > 0.0 {
                        env.crossfade_ms
                    } else if adjacent_previous.is_some() {
                        automatic_crossfade
                    } else {
                        0.0
                    };
                    if visual_crossfade_ms > 0.0 {
                        let cross_start =
                            note_start_x - (visual_crossfade_ms * state.px_per_ms as f64) as f32;
                        let left = cross_start.max(graph_rect.min.x);
                        let right = note_start_x.min(graph_rect.max.x);
                        if right > left {
                            painter.rect_filled(
                                Rect::from_min_max(
                                    Pos2::new(left, inner.min.y),
                                    Pos2::new(right, inner.max.y),
                                ),
                                Rounding::ZERO,
                                Color32::from_rgba_unmultiplied(0, 200, 180, 26),
                            );
                            let color = Color32::from_rgb(0, 225, 200);
                            painter.line_segment(
                                [Pos2::new(left, inner.max.y), Pos2::new(right, inner.min.y)],
                                Stroke::new(1.8_f32, color),
                            );
                            painter.line_segment(
                                [Pos2::new(left, inner.min.y), Pos2::new(right, inner.max.y)],
                                Stroke::new(1.8_f32, color),
                            );
                            let handle = Pos2::new(cross_start, inner.center().y);
                            if graph_rect.expand(15.0).contains(handle) {
                                let hovered = pointer
                                    .is_some_and(|position| position.distance(handle) <= 13.0);
                                let dragging = state.dragging_envelope_pt == Some((note_index, 5));
                                painter.circle_filled(
                                    handle,
                                    if hovered || dragging { 8.5 } else { 6.0 },
                                    if dragging {
                                        Color32::WHITE
                                    } else {
                                        Color32::from_rgb(0, 235, 210)
                                    },
                                );
                                painter.circle_stroke(
                                    handle,
                                    if hovered || dragging { 8.5 } else { 6.0 },
                                    Stroke::new(1.2_f32, Color32::WHITE),
                                );
                                if hovered || dragging {
                                    hovered_handle = Some((note_index, 5));
                                    painter.text(
                                        Pos2::new(handle.x, handle.y - 14.0),
                                        egui::Align2::CENTER_BOTTOM,
                                        format!("Crossfade · {:.0}ms", visual_crossfade_ms),
                                        egui::FontId::proportional(9.5),
                                        Color32::WHITE,
                                    );
                                }
                            }
                        }
                    }

                    painter.add(egui::Shape::line(
                        screen_points.to_vec(),
                        Stroke::new(
                            if selected { 2.6_f32 } else { 1.5_f32 },
                            if selected {
                                Color32::from_rgb(0, 220, 250)
                            } else {
                                Color32::from_rgb(65, 145, 185)
                            },
                        ),
                    ));
                    for (point_index, point) in screen_points.iter().enumerate() {
                        if !graph_rect.expand(15.0).contains(*point) {
                            continue;
                        }
                        let hovered =
                            pointer.is_some_and(|position| position.distance(*point) <= 13.0);
                        if hovered {
                            hovered_handle = Some((note_index, point_index));
                        }
                        let dragging =
                            state.dragging_envelope_pt == Some((note_index, point_index));
                        painter.circle_filled(
                            *point,
                            if hovered || dragging { 8.0 } else { 5.5 },
                            if dragging {
                                Color32::WHITE
                            } else if selected {
                                Color32::from_rgb(0, 235, 255)
                            } else {
                                Color32::from_rgb(70, 160, 200)
                            },
                        );
                        painter.circle_stroke(
                            *point,
                            if hovered || dragging { 8.0 } else { 5.5 },
                            Stroke::new(
                                1.0_f32,
                                if dragging {
                                    Color32::from_rgb(0, 220, 255)
                                } else {
                                    Color32::from_rgb(20, 30, 45)
                                },
                            ),
                        );
                        if hovered || dragging {
                            let label_text = format!(
                                "{} · P{} · {:.0}ms · {:.0}%",
                                note.lyric,
                                point_index + 1,
                                points[point_index].0,
                                points[point_index].1
                            );
                            let text_shape = painter.layout_no_wrap(
                                label_text,
                                egui::FontId::proportional(9.5),
                                Color32::WHITE,
                            );
                            let pill_rect = Rect::from_center_size(
                                Pos2::new(point.x, point.y - 14.0),
                                Vec2::new(text_shape.size().x + 10.0, 16.0),
                            );
                            painter.rect_filled(
                                pill_rect,
                                Rounding::same(4.0),
                                Color32::from_rgba_unmultiplied(15, 22, 36, 230),
                            );
                            painter.rect_stroke(
                                pill_rect,
                                Rounding::same(4.0),
                                Stroke::new(1.0_f32, Color32::from_rgb(0, 210, 240)),
                            );
                            painter.galley(
                                Pos2::new(
                                    pill_rect.center().x - text_shape.size().x * 0.5,
                                    pill_rect.center().y - text_shape.size().y * 0.5,
                                ),
                                text_shape,
                                Color32::WHITE,
                            );
                        }
                    }
                }

                if response.drag_started() {
                    if let Some(handle) = hovered_handle {
                        on_before_change();
                        state.dragging_envelope_pt = Some(handle);
                        state.selected_note_index = Some(handle.0);
                        state.selected_note_indices.clear();
                        state.selected_note_indices.insert(handle.0);
                    }
                }
                if let (Some((note_index, point_index)), Some(position)) =
                    (state.dragging_envelope_pt, pointer)
                {
                    if let Some(note) = notes.get_mut(note_index) {
                        let duration = note.duration_ms.max(1.0);
                        let note_start_x =
                            timeline_origin_x + (note.position_ms * state.px_per_ms as f64) as f32;
                        let time = (f64::from(position.x - note_start_x)
                            / f64::from(state.px_per_ms))
                        .max(0.0);
                        let volume = ((inner.max.y - position.y) / inner.height()).clamp(0.0, 1.0)
                            as f64
                            * 100.0;
                        match point_index {
                            0 => {
                                note.envelope.p1 = time.clamp(0.0, duration);
                                note.envelope.v1 = volume.clamp(0.0, 100.0);
                            }
                            1 => {
                                let p2_time = time.max(note.envelope.p1);
                                note.envelope.p2 = (p2_time - note.envelope.p1).max(0.0);
                                note.envelope.v2 = volume.clamp(0.0, 100.0);
                            }
                            2 => {
                                let p3_time = time.max(note.envelope.p1 + note.envelope.p2);
                                note.envelope.p3 =
                                    (p3_time - note.envelope.p1 - note.envelope.p2).max(0.0);
                                note.envelope.v3 = volume.clamp(0.0, 100.0);
                            }
                            3 => {
                                let p4_pos = time.clamp(0.0, duration + 200.0);
                                note.envelope.p4 = (duration - p4_pos).max(0.0);
                                note.envelope.v4 = volume.clamp(0.0, 100.0);
                            }
                            4 => {
                                let p4_pos = (duration - note.envelope.p4).max(0.0);
                                let p5_pos = time.max(p4_pos);
                                note.envelope.p5 = (p5_pos - p4_pos).max(0.0);
                                note.envelope.v5 = volume.clamp(0.0, 100.0);
                            }
                            5 => {
                                note.envelope.crossfade_ms = (f64::from(note_start_x - position.x)
                                    / f64::from(state.px_per_ms))
                                .clamp(0.0, 600.0);
                            }
                            _ => {}
                        }
                        state.continuous_edit_dirty = true;
                    }
                }
                if response.drag_stopped() {
                    state.dragging_envelope_pt = None;
                }
            });
        // `ui.max_rect()` inside the closure still reflects the layout before
        // the resize interaction is finalized. Persist the resulting panel
        // rectangle instead, otherwise the next frame restores the old size.
        state.drawer_height = panel_response.response.rect.height().clamp(130.0, 500.0);
    } else if state.show_parameters_drawer {
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
                let actual_h = ui.max_rect().height().clamp(60.0, 750.0);
                let current_drawer_h = actual_h;
                let available_graph_h = (current_drawer_h - 10.0).max(50.0);

                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.add_space(2.0);
                        ui.label(
                            egui::RichText::new("📊 PARÂMETROS / EXPRESSÕES")
                                .strong()
                                .size(10.0)
                                .color(Color32::from_rgb(0, 255, 157)),
                        );
                        ui.add_space(2.0);

                        egui::ScrollArea::vertical()
                            .id_salt("param_tabs_scroll")
                            .max_width(135.0)
                            .max_height((current_drawer_h - 22.0).max(50.0))
                            .show(ui, |ui| {
                                let param_tabs = [
                                    ("Dynamics (DYN)", ParameterTab::Dynamics),
                                    ("Pitch Offset (PITD)", ParameterTab::PitchDelta),
                                    ("Gender (GEN/g)", ParameterTab::Gender),
                                    ("Vel. Consoante (VEL)", ParameterTab::Velocity),
                                    ("Breathiness (BRE/B)", ParameterTab::Breathiness),
                                    ("Modulação (MOD)", ParameterTab::Modulation),
                                    ("Volume (VOL)", ParameterTab::Volume),
                                    ("Ataque (ATK)", ParameterTab::Attack),
                                    ("Decaimento (DEC)", ParameterTab::Decay),
                                    ("Vibrato Tam (VIBL)", ParameterTab::VibratoLength),
                                    ("Vibrato Prof (VIBD)", ParameterTab::VibratoDepth),
                                    ("Vibrato Per (VIBP)", ParameterTab::VibratoPeriod),
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
                                    .fill(fill_color)
                                    .rounding(Rounding::same(3.0));

                                    if ui.add(btn).clicked() {
                                        state.selected_parameter = tab_val;
                                    }
                                }
                            });
                    });

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    let (graph_rect, graph_response) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width() - 8.0, available_graph_h),
                        Sense::click_and_drag(),
                    );

                    let painter = ui.painter_at(graph_rect);
                    painter.rect_filled(graph_rect, Rounding::same(4.0), MelodyneTheme::BG_CANVAS);
                    painter.rect_stroke(
                        graph_rect,
                        Rounding::same(4.0),
                        Stroke::new(1.0_f32, MelodyneTheme::GRID_LINE_BAR),
                    );

                    let is_bipolar = matches!(
                        state.selected_parameter,
                        ParameterTab::Dynamics | ParameterTab::PitchDelta | ParameterTab::Gender
                    );

                    let mid_y = if is_bipolar {
                        graph_rect.min.y + graph_rect.height() * 0.5
                    } else {
                        graph_rect.max.y - 12.0
                    };

                    let half_span_y = (graph_rect.height() * 0.45).max(10.0);

                    if is_bipolar {
                        painter.line_segment(
                            [
                                Pos2::new(graph_rect.min.x, mid_y),
                                Pos2::new(graph_rect.max.x, mid_y),
                            ],
                            Stroke::new(1.0_f32, MelodyneTheme::GRID_LINE_SUB),
                        );
                    }

                    if let Some(mpos) = graph_response.interact_pointer_pos() {
                        if graph_response.dragged() || graph_response.clicked() {
                            let click_t = (mpos.x - (ruler_rect.min.x + keyboard_width)
                                + timeline_scroll_x)
                                as f64
                                / state.px_per_ms as f64;
                            let mut changed = false;
                            for note in notes.iter_mut() {
                                if click_t >= note.position_ms
                                    && click_t <= note.position_ms + note.duration_ms
                                {
                                    let norm_bipolar =
                                        ((mid_y - mpos.y) / half_span_y).clamp(-1.0, 1.0) as f64; // [-1.0, 1.0]
                                    let norm_unipolar = ((graph_rect.max.y - mpos.y)
                                        / (graph_rect.height() - 15.0).max(1.0))
                                    .clamp(0.0, 1.0)
                                        as f64; // [0.0, 1.0]

                                    match state.selected_parameter {
                                        ParameterTab::Dynamics => {
                                            note.expressions.dynamics =
                                                (norm_bipolar * 180.0 - 60.0).clamp(-240.0, 120.0);
                                        }
                                        ParameterTab::PitchDelta => {
                                            note.expressions.pitch_delta =
                                                (norm_bipolar * 1200.0).clamp(-1200.0, 1200.0);
                                        }
                                        ParameterTab::Gender => {
                                            note.expressions.gender =
                                                (norm_bipolar * 100.0).clamp(-100.0, 100.0);
                                        }
                                        ParameterTab::Velocity => {
                                            note.expressions.consonant_velocity =
                                                (norm_unipolar * 200.0).clamp(0.0, 200.0);
                                        }
                                        ParameterTab::Breathiness => {
                                            note.expressions.breathiness =
                                                (norm_unipolar * 100.0).clamp(0.0, 100.0);
                                        }
                                        ParameterTab::Modulation => {
                                            note.expressions.modulation =
                                                (norm_unipolar * 200.0).clamp(0.0, 200.0);
                                        }
                                        ParameterTab::Volume => {
                                            note.expressions.volume =
                                                (norm_unipolar * 200.0).clamp(0.0, 200.0);
                                        }
                                        ParameterTab::Attack => {
                                            note.expressions.attack =
                                                (norm_unipolar * 200.0).clamp(0.0, 200.0);
                                        }
                                        ParameterTab::Decay => {
                                            note.expressions.decay =
                                                (norm_unipolar * 100.0).clamp(0.0, 100.0);
                                        }
                                        ParameterTab::VibratoLength => {
                                            note.vibrato.length_pct =
                                                (norm_unipolar * 100.0).clamp(0.0, 100.0);
                                        }
                                        ParameterTab::VibratoDepth => {
                                            note.vibrato.depth_cents =
                                                (norm_unipolar * 200.0).clamp(0.0, 200.0);
                                        }
                                        ParameterTab::VibratoPeriod => {
                                            note.vibrato.period_ms =
                                                (norm_unipolar * 400.0 + 50.0).clamp(50.0, 450.0);
                                        }
                                    }
                                    changed = true;
                                }
                            }
                            if changed {
                                state.continuous_edit_dirty = true;
                            }
                        }
                    }

                    for (note_index, note) in notes.iter().enumerate() {
                        let x_start = ruler_rect.min.x
                            + keyboard_width
                            + (note.position_ms * state.px_per_ms as f64) as f32
                            - timeline_scroll_x;
                        let x_end = x_start + (note.duration_ms * state.px_per_ms as f64) as f32;

                        if x_end >= graph_rect.min.x && x_start <= graph_rect.max.x {
                            let (val, min_v, max_v, value_label) = match state.selected_parameter {
                                ParameterTab::Dynamics => (
                                    note.expressions.dynamics,
                                    -240.0,
                                    120.0,
                                    format!("{:+.1} dB", note.expressions.dynamics * 0.1),
                                ),
                                ParameterTab::PitchDelta => (
                                    note.expressions.pitch_delta,
                                    -1200.0,
                                    1200.0,
                                    format!("{:+.0} c", note.expressions.pitch_delta),
                                ),
                                ParameterTab::Gender => (
                                    note.expressions.gender,
                                    -100.0,
                                    100.0,
                                    format!("g{:+.0}", note.expressions.gender),
                                ),
                                ParameterTab::Velocity => (
                                    note.expressions.consonant_velocity,
                                    0.0,
                                    200.0,
                                    format!("VEL {:.0}%", note.expressions.consonant_velocity),
                                ),
                                ParameterTab::Breathiness => (
                                    note.expressions.breathiness,
                                    0.0,
                                    100.0,
                                    format!("B{:.0}", note.expressions.breathiness),
                                ),
                                ParameterTab::Modulation => (
                                    note.expressions.modulation,
                                    0.0,
                                    200.0,
                                    format!("MOD {:.0}%", note.expressions.modulation),
                                ),
                                ParameterTab::Volume => (
                                    note.expressions.volume,
                                    0.0,
                                    200.0,
                                    format!("VOL {:.0}%", note.expressions.volume),
                                ),
                                ParameterTab::Attack => (
                                    note.expressions.attack,
                                    0.0,
                                    200.0,
                                    format!("ATK {:.0}%", note.expressions.attack),
                                ),
                                ParameterTab::Decay => (
                                    note.expressions.decay,
                                    0.0,
                                    100.0,
                                    format!("DEC {:.0}%", note.expressions.decay),
                                ),
                                ParameterTab::VibratoLength => (
                                    note.vibrato.length_pct,
                                    0.0,
                                    100.0,
                                    format!("VIBL {:.0}%", note.vibrato.length_pct),
                                ),
                                ParameterTab::VibratoDepth => (
                                    note.vibrato.depth_cents,
                                    0.0,
                                    200.0,
                                    format!("VIBD {:.0} c", note.vibrato.depth_cents),
                                ),
                                ParameterTab::VibratoPeriod => (
                                    note.vibrato.period_ms,
                                    50.0,
                                    450.0,
                                    format!("VIBP {:.0} ms", note.vibrato.period_ms),
                                ),
                            };
                            let label_str = format!("{} · {}", note.lyric, value_label);
                            let is_selected = state.selected_note_index == Some(note_index)
                                || state.selected_note_indices.contains(&note_index);

                            let node_y = if is_bipolar {
                                let norm = val / max_v;
                                mid_y - (norm as f32 * half_span_y)
                            } else {
                                let norm = ((val - min_v) / (max_v - min_v)).clamp(0.0, 1.0);
                                graph_rect.max.y
                                    - (norm as f32 * (graph_rect.height() - 20.0).max(10.0))
                                    - 10.0
                            };

                            let bar_min_y = mid_y.min(node_y);
                            let bar_max_y = mid_y.max(node_y);

                            let bar_rect = Rect::from_min_max(
                                Pos2::new(x_start.max(graph_rect.min.x), bar_min_y),
                                Pos2::new(
                                    x_end.min(graph_rect.max.x),
                                    bar_max_y.max(bar_min_y + 2.0),
                                ),
                            );

                            painter.line_segment(
                                [
                                    Pos2::new(x_start, graph_rect.min.y),
                                    Pos2::new(x_start, graph_rect.max.y),
                                ],
                                Stroke::new(
                                    if is_selected { 1.5_f32 } else { 0.7_f32 },
                                    if is_selected {
                                        MelodyneTheme::ACCENT_GOLD
                                    } else {
                                        Color32::from_rgba_premultiplied(110, 85, 160, 100)
                                    },
                                ),
                            );

                            painter.rect_filled(
                                bar_rect,
                                Rounding::same(2.0),
                                if is_selected {
                                    Color32::from_rgba_premultiplied(255, 190, 60, 95)
                                } else {
                                    Color32::from_rgba_premultiplied(0, 255, 157, 60)
                                },
                            );
                            painter.line_segment(
                                [
                                    Pos2::new(x_start.max(graph_rect.min.x), node_y),
                                    Pos2::new(x_end.min(graph_rect.max.x), node_y),
                                ],
                                Stroke::new(
                                    2.5_f32,
                                    if is_selected {
                                        MelodyneTheme::ACCENT_GOLD
                                    } else {
                                        Color32::from_rgb(0, 255, 157)
                                    },
                                ),
                            );

                            let text_pos = Pos2::new((x_start + x_end) * 0.5, node_y - 6.0);
                            if text_pos.x >= graph_rect.min.x && text_pos.x <= graph_rect.max.x {
                                painter.text(
                                    text_pos,
                                    egui::Align2::CENTER_BOTTOM,
                                    &label_str,
                                    egui::FontId::proportional(9.0),
                                    Color32::from_rgb(216, 180, 254),
                                );
                            }
                        }
                    }
                });
            });
        state.drawer_height = panel_response.response.rect.height().clamp(60.0, 750.0);
    }

    let grid_width = (keyboard_width as f64 + total_canvas_ms * state.px_per_ms as f64) as f32;
    let grid_width = grid_width.max(3000.0);
    let grid_height = key_count as f32 * state.row_height;

    let mut scroll_area = egui::ScrollArea::both()
        .id_salt("piano_roll_scroll")
        .auto_shrink([false, false])
        .enable_scrolling(!is_mod_zoom);

    if !state.initial_scrolled {
        let (first_note_pos, target_midi) = if let Some(first) = notes.iter().min_by(|a, b| {
            a.position_ms
                .partial_cmp(&b.position_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
        }) {
            (first.position_ms, first.midi_key())
        } else {
            (0.0, 60)
        };

        let row_idx = (state.max_midi.saturating_sub(target_midi)) as f32;
        let target_y = (row_idx * state.row_height - 180.0).max(0.0);
        let target_x = ((first_note_pos * state.px_per_ms as f64) as f32 - 100.0).max(0.0);

        state.vertical_scroll_offset = target_y;
        state.horizontal_scroll_offset = target_x;
        scroll_area = scroll_area
            .vertical_scroll_offset(state.vertical_scroll_offset)
            .horizontal_scroll_offset(state.horizontal_scroll_offset);
        state.initial_scrolled = true;
    }

    if state.is_playing && state.auto_scroll_mode != AutoScrollMode::Off {
        scroll_area = scroll_area.horizontal_scroll_offset(state.horizontal_scroll_offset);
    }

    if is_mod_zoom || state.is_scrubbing_ruler {
        scroll_area = scroll_area
            .horizontal_scroll_offset(state.horizontal_scroll_offset)
            .vertical_scroll_offset(state.vertical_scroll_offset);
    }

    let scroll_output = scroll_area.show(ui, |ui| {
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(grid_width, grid_height), Sense::click_and_drag());

        let painter = ui.painter_at(rect);

        painter.rect_filled(rect, Rounding::ZERO, MelodyneTheme::BG_CANVAS);

        let grid_start_y = rect.min.y;
        let grid_end_y = rect.max.y;

        let visible_clip = ui.clip_rect();
        let first_visible_key = (((visible_clip.min.y - grid_start_y) / state.row_height).floor()
            as isize)
            .max(0) as usize;
        let last_visible_key = (((visible_clip.max.y - grid_start_y) / state.row_height).ceil()
            as isize)
            .clamp(0, key_count as isize) as usize;

        let row_x_min = (rect.min.x + keyboard_width).max(visible_clip.min.x);
        let row_x_max = visible_clip.max.x.min(rect.max.x);

        for key_idx in first_visible_key..last_visible_key {
            let midi = state.max_midi - key_idx as u8;
            let y_top = grid_start_y + key_idx as f32 * state.row_height;
            let y_bottom = y_top + state.row_height;

            let is_black_key = matches!(midi % 12, 1 | 3 | 6 | 8 | 10);
            let row_color = if is_black_key {
                MelodyneTheme::BG_ROW_BLACK_KEY
            } else {
                MelodyneTheme::BG_ROW_WHITE_KEY
            };

            if row_x_max > row_x_min {
                painter.rect_filled(
                    Rect::from_min_max(Pos2::new(row_x_min, y_top), Pos2::new(row_x_max, y_bottom)),
                    Rounding::ZERO,
                    row_color,
                );
                painter.line_segment(
                    [
                        Pos2::new(row_x_min, y_bottom),
                        Pos2::new(row_x_max, y_bottom),
                    ],
                    Stroke::new(0.5_f32, MelodyneTheme::GRID_LINE_SUB),
                );
            }
        }

        grid::draw_timeline_grid(
            &painter,
            state,
            rect,
            visible_clip,
            keyboard_width,
            grid_start_y,
            grid_end_y,
            total_canvas_ms,
            bpm,
            snap_option,
        );

        if state.show_waveform && !notes.is_empty() {
            let max_wave_half_h = (state.row_height * 0.90).max(18.0);
            let lowest_visible_note_y = notes
                .iter()
                .filter_map(|note| {
                    let key_index = state.max_midi.saturating_sub(note.midi_key()) as f32;
                    let note_y = grid_start_y + key_index * state.row_height;
                    (note_y >= visible_clip.min.y - state.row_height
                        && note_y <= visible_clip.max.y)
                        .then_some(note_y)
                })
                .fold(None::<f32>, |lowest, y| {
                    Some(lowest.map_or(y, |current| current.max(y)))
                });
            let preferred_y = lowest_visible_note_y
                .map(|y| y + state.row_height * 2.2)
                .unwrap_or(visible_clip.max.y - max_wave_half_h - 12.0);
            let wave_min_y = visible_clip.min.y + max_wave_half_h + 12.0;
            let wave_max_y = visible_clip.max.y - max_wave_half_h - 12.0;
            let wave_y_center = if wave_min_y <= wave_max_y {
                preferred_y.clamp(wave_min_y, wave_max_y)
            } else {
                visible_clip.center().y
            };

            let visible_x_start = (visible_clip.min.x - 20.0).max(rect.min.x + keyboard_width);
            let visible_x_end = (visible_clip.max.x + 20.0).min(rect.max.x);

            if !state.rendered_waveform_peaks.is_empty() {
                let audio_start_ms = state
                    .rendered_waveform_peaks
                    .first()
                    .map(|p| p.0)
                    .unwrap_or(0.0);
                let audio_end_ms = state
                    .rendered_waveform_peaks
                    .last()
                    .map(|p| p.0)
                    .unwrap_or(0.0);
                let audio_start_x = rect.min.x
                    + keyboard_width
                    + (audio_start_ms as f64 * state.px_per_ms as f64) as f32;
                let audio_end_x = rect.min.x
                    + keyboard_width
                    + (audio_end_ms as f64 * state.px_per_ms as f64) as f32;

                let draw_start_x = audio_start_x.max(visible_x_start);
                let draw_end_x = audio_end_x.min(visible_x_end);

                if draw_start_x < draw_end_x {
                    painter.line_segment(
                        [
                            Pos2::new(draw_start_x, wave_y_center),
                            Pos2::new(draw_end_x, wave_y_center),
                        ],
                        Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(80, 160, 220, 50)),
                    );

                    let step_px = 1.5f32;
                    let mut wave_x = draw_start_x;
                    let mut top_pts = Vec::new();
                    let mut bottom_pts = Vec::new();

                    while wave_x <= draw_end_x {
                        let time_ms = ((wave_x - (rect.min.x + keyboard_width)) as f64
                            / state.px_per_ms as f64) as f32;
                        let amp = state.waveform_amplitude_at(time_ms).unwrap_or(0.0);
                        let h = amp * max_wave_half_h;

                        top_pts.push(Pos2::new(wave_x, wave_y_center - h));
                        bottom_pts.push(Pos2::new(wave_x, wave_y_center + h));

                        wave_x += step_px;
                    }

                    if top_pts.len() >= 2 {
                        let mut mesh = egui::Mesh::default();
                        for i in 0..top_pts.len() - 1 {
                            let p0_top = top_pts[i];
                            let p1_top = top_pts[i + 1];
                            let p0_bot = bottom_pts[i];
                            let p1_bot = bottom_pts[i + 1];

                            let crest_color = Color32::from_rgba_unmultiplied(65, 175, 240, 95);
                            let center_color = Color32::from_rgba_unmultiplied(30, 95, 175, 55);

                            let v0 = mesh.vertices.len() as u32;
                            mesh.vertices.push(egui::epaint::Vertex {
                                pos: p0_top,
                                uv: egui::epaint::WHITE_UV,
                                color: crest_color,
                            });
                            mesh.vertices.push(egui::epaint::Vertex {
                                pos: p1_top,
                                uv: egui::epaint::WHITE_UV,
                                color: crest_color,
                            });
                            mesh.vertices.push(egui::epaint::Vertex {
                                pos: Pos2::new(p1_top.x, wave_y_center),
                                uv: egui::epaint::WHITE_UV,
                                color: center_color,
                            });
                            mesh.vertices.push(egui::epaint::Vertex {
                                pos: Pos2::new(p0_top.x, wave_y_center),
                                uv: egui::epaint::WHITE_UV,
                                color: center_color,
                            });

                            mesh.add_triangle(v0, v0 + 1, v0 + 2);
                            mesh.add_triangle(v0, v0 + 2, v0 + 3);

                            let v1 = mesh.vertices.len() as u32;
                            mesh.vertices.push(egui::epaint::Vertex {
                                pos: Pos2::new(p0_bot.x, wave_y_center),
                                uv: egui::epaint::WHITE_UV,
                                color: center_color,
                            });
                            mesh.vertices.push(egui::epaint::Vertex {
                                pos: Pos2::new(p1_bot.x, wave_y_center),
                                uv: egui::epaint::WHITE_UV,
                                color: center_color,
                            });
                            mesh.vertices.push(egui::epaint::Vertex {
                                pos: p1_bot,
                                uv: egui::epaint::WHITE_UV,
                                color: crest_color,
                            });
                            mesh.vertices.push(egui::epaint::Vertex {
                                pos: p0_bot,
                                uv: egui::epaint::WHITE_UV,
                                color: crest_color,
                            });

                            mesh.add_triangle(v1, v1 + 1, v1 + 2);
                            mesh.add_triangle(v1, v1 + 2, v1 + 3);
                        }
                        painter.add(egui::Shape::mesh(mesh));

                        painter.add(egui::Shape::line(
                            top_pts,
                            Stroke::new(1.3_f32, Color32::from_rgb(120, 215, 255)),
                        ));
                        painter.add(egui::Shape::line(
                            bottom_pts,
                            Stroke::new(1.3_f32, Color32::from_rgb(120, 215, 255)),
                        ));
                    }
                }
            } else {
                for note in notes.iter() {
                    let note_start_x = rect.min.x
                        + keyboard_width
                        + (note.position_ms * state.px_per_ms as f64) as f32;
                    let note_end_x =
                        note_start_x + (note.duration_ms * state.px_per_ms as f64) as f32;

                    if note_end_x < visible_x_start || note_start_x > visible_x_end {
                        continue;
                    }

                    let x_start_clamped = note_start_x.max(visible_x_start);
                    let x_end_clamped = note_end_x.min(visible_x_end);
                    let mut px = x_start_clamped;
                    let step_px = 1.5f32;
                    let mut ph_top_pts = Vec::new();
                    let mut ph_bottom_pts = Vec::new();

                    while px <= x_end_clamped {
                        let rel_t = ((px - note_start_x) / (note_end_x - note_start_x).max(1.0))
                            .clamp(0.0, 1.0);
                        let env = if rel_t < 0.15 {
                            (rel_t / 0.15) * 0.95
                        } else if rel_t > 0.85 {
                            ((1.0 - rel_t) / 0.15) * 0.85
                        } else {
                            0.78 + (rel_t * 12.0).sin().abs() * 0.14
                        };

                        let amp = env.clamp(0.0, 1.0) * max_wave_half_h;
                        ph_top_pts.push(Pos2::new(px, wave_y_center - amp));
                        ph_bottom_pts.push(Pos2::new(px, wave_y_center + amp));
                        px += step_px;
                    }

                    if ph_top_pts.len() >= 2 {
                        let mut mesh = egui::Mesh::default();
                        for i in 0..ph_top_pts.len() - 1 {
                            let p0_top = ph_top_pts[i];
                            let p1_top = ph_top_pts[i + 1];
                            let p0_bot = ph_bottom_pts[i];
                            let p1_bot = ph_bottom_pts[i + 1];

                            let v0 = mesh.vertices.len() as u32;
                            let color = Color32::from_rgba_unmultiplied(80, 140, 200, 35);
                            mesh.vertices.push(egui::epaint::Vertex {
                                pos: p0_top,
                                uv: egui::epaint::WHITE_UV,
                                color,
                            });
                            mesh.vertices.push(egui::epaint::Vertex {
                                pos: p1_top,
                                uv: egui::epaint::WHITE_UV,
                                color,
                            });
                            mesh.vertices.push(egui::epaint::Vertex {
                                pos: p1_bot,
                                uv: egui::epaint::WHITE_UV,
                                color,
                            });
                            mesh.vertices.push(egui::epaint::Vertex {
                                pos: p0_bot,
                                uv: egui::epaint::WHITE_UV,
                                color,
                            });

                            mesh.add_triangle(v0, v0 + 1, v0 + 2);
                            mesh.add_triangle(v0, v0 + 2, v0 + 3);
                        }
                        painter.add(egui::Shape::mesh(mesh));
                        painter.add(egui::Shape::line(
                            ph_top_pts,
                            Stroke::new(
                                1.0_f32,
                                Color32::from_rgba_unmultiplied(100, 170, 230, 70),
                            ),
                        ));
                        painter.add(egui::Shape::line(
                            ph_bottom_pts,
                            Stroke::new(
                                1.0_f32,
                                Color32::from_rgba_unmultiplied(100, 170, 230, 70),
                            ),
                        ));
                    }
                }
            }
        }

        let mut note_to_delete: Option<usize> = None;
        let mut note_to_slice: Option<(usize, f64)> = None;
        let mut commit_lyric_edit: Option<(usize, String, Option<usize>)> = None;
        let mut pending_lyric_edit: Option<(usize, Rect, f32, f32)> = None;

        let note_info: Vec<(u8, f64, f64, f64)> = notes
            .iter()
            .map(|n| {
                let first_t = n
                    .pitch_bend
                    .points
                    .first()
                    .map(|p| p.time_offset_ms)
                    .unwrap_or_else(|| n.pitch_bend.portamento_start_ms.clamp(-2000.0, 2000.0));
                (n.midi_key(), n.position_ms, n.duration_ms, first_t)
            })
            .collect();

        let mouse_interact_pos = ui.input(|i| i.pointer.interact_pos());
        let mut interacted_with_note_or_ui = false;
        let mut pending_lyric_tags: Vec<(Rect, Color32, String, Color32)> = Vec::new();
        let mut pending_phoneme_badges: Vec<(Rect, String)> = Vec::new();

        for (idx, note) in notes.iter_mut().enumerate() {
            let note_midi = note.midi_key();
            if note_midi < state.min_midi || note_midi > state.max_midi {
                continue;
            }

            let key_idx = (state.max_midi - note_midi) as f32;
            let y_top = grid_start_y + key_idx * state.row_height + 1.0;
            let y_bottom = y_top + state.row_height - 2.0;
            let y_center = (y_top + y_bottom) * 0.5;

            let x_start =
                rect.min.x + keyboard_width + (note.position_ms * state.px_per_ms as f64) as f32;
            let x_end = x_start + (note.duration_ms * state.px_per_ms as f64) as f32;

            let note_rect =
                Rect::from_min_max(Pos2::new(x_start, y_top), Pos2::new(x_end, y_bottom));

            let extended_clip = visible_clip.expand2(Vec2::new(300.0, 40.0));
            if !note_rect.intersects(extended_clip) {
                continue;
            }

            let is_selected = state.selected_note_index == Some(idx)
                || state.selected_note_indices.contains(&idx);
            let is_editing_lyric = state.editing_lyric_index == Some(idx);

            let note_color = if is_selected {
                MelodyneTheme::NOTE_SELECTED_GOLD
            } else {
                MelodyneTheme::NOTE_GOLD_FILL
            };

            painter.rect_filled(note_rect, Rounding::same(6.0), note_color);
            painter.rect_stroke(
                note_rect,
                Rounding::same(6.0),
                Stroke::new(
                    1.8_f32,
                    if is_selected {
                        Color32::WHITE
                    } else {
                        MelodyneTheme::NOTE_GOLD_STROKE
                    },
                ),
            );

            if let Some(ref dragged_alias) = phoneme_state.dragged_phoneme {
                if let Some(mpos) = mouse_interact_pos {
                    if note_rect.contains(mpos) {
                        painter.rect_stroke(
                            note_rect,
                            Rounding::same(6.0),
                            Stroke::new(2.5_f32, Color32::from_rgb(0, 220, 255)),
                        );

                        if !ui.input(|i| i.pointer.primary_down()) {
                            commit_lyric_edit = Some((idx, dragged_alias.clone(), None));
                            phoneme_state.dragged_phoneme = None;
                        }
                    }
                }
            }

            let note_wave_start_x = (x_start + 4.0).max(visible_clip.min.x);
            let note_wave_end_x = (x_end - 4.0).min(visible_clip.max.x);
            let max_note_wave_h = state.row_height * 0.36;

            if note_wave_start_x < note_wave_end_x {
                let step = 2.0f32;
                let mut wave_x = note_wave_start_x;
                let mut note_top_pts = Vec::new();
                let mut note_bottom_pts = Vec::new();

                while wave_x <= note_wave_end_x {
                    let time_ms = (note.position_ms
                        + ((wave_x - x_start) / state.px_per_ms as f32) as f64)
                        as f32;
                    let amp = if !state.rendered_waveform_peaks.is_empty() {
                        state.waveform_amplitude_at(time_ms).unwrap_or(0.0)
                    } else {
                        let rel_t =
                            ((wave_x - x_start) / (x_end - x_start).max(1.0)).clamp(0.0, 1.0);
                        if rel_t < 0.2 {
                            rel_t / 0.2 * 0.5
                        } else if rel_t > 0.8 {
                            (1.0 - rel_t) / 0.2 * 0.45
                        } else {
                            0.45 + (rel_t * 16.0).sin().abs() * 0.12
                        }
                    };

                    let h = (amp * max_note_wave_h).max(0.5);
                    note_top_pts.push(Pos2::new(wave_x, y_center - h));
                    note_bottom_pts.push(Pos2::new(wave_x, y_center + h));
                    wave_x += step;
                }

                if note_top_pts.len() >= 2 {
                    let mut note_mesh = egui::Mesh::default();
                    let wave_fill = if is_selected {
                        Color32::from_rgba_unmultiplied(255, 255, 255, 36)
                    } else {
                        Color32::from_rgba_unmultiplied(0, 0, 0, 30)
                    };
                    for i in 0..note_top_pts.len() - 1 {
                        let p0_top = note_top_pts[i];
                        let p1_top = note_top_pts[i + 1];
                        let p0_bot = note_bottom_pts[i];
                        let p1_bot = note_bottom_pts[i + 1];

                        let v0 = note_mesh.vertices.len() as u32;
                        note_mesh.vertices.push(egui::epaint::Vertex {
                            pos: p0_top,
                            uv: egui::epaint::WHITE_UV,
                            color: wave_fill,
                        });
                        note_mesh.vertices.push(egui::epaint::Vertex {
                            pos: p1_top,
                            uv: egui::epaint::WHITE_UV,
                            color: wave_fill,
                        });
                        note_mesh.vertices.push(egui::epaint::Vertex {
                            pos: p1_bot,
                            uv: egui::epaint::WHITE_UV,
                            color: wave_fill,
                        });
                        note_mesh.vertices.push(egui::epaint::Vertex {
                            pos: p0_bot,
                            uv: egui::epaint::WHITE_UV,
                            color: wave_fill,
                        });

                        note_mesh.add_triangle(v0, v0 + 1, v0 + 2);
                        note_mesh.add_triangle(v0, v0 + 2, v0 + 3);
                    }
                    painter.add(egui::Shape::mesh(note_mesh));

                    let crest_stroke = if is_selected {
                        Color32::from_rgba_unmultiplied(255, 255, 255, 55)
                    } else {
                        Color32::from_rgba_unmultiplied(20, 35, 10, 50)
                    };
                    painter.add(egui::Shape::line(
                        note_top_pts,
                        Stroke::new(1.0_f32, crest_stroke),
                    ));
                    painter.add(egui::Shape::line(
                        note_bottom_pts,
                        Stroke::new(1.0_f32, crest_stroke),
                    ));
                }
            }

            if is_editing_lyric {
                let edit_rect = Rect::from_min_size(
                    Pos2::new(x_start + 1.0, y_top + 1.0),
                    Vec2::new((note_rect.width() - 2.0).max(95.0), state.row_height - 2.0),
                );
                pending_lyric_edit = Some((idx, edit_rect, x_start, y_bottom));
            } else {
                let pill_rect = Rect::from_min_size(
                    Pos2::new(x_start + 4.0, y_top + 3.0),
                    Vec2::new(
                        (note.lyric.len() as f32 * 8.0 + 12.0).min(note_rect.width() - 8.0),
                        state.row_height - 10.0,
                    ),
                );
                let pill_bg = if is_selected {
                    Color32::from_rgb(15, 15, 20) // Solid dark for high contrast
                } else {
                    Color32::from_rgb(26, 18, 8) // Opaque dark so pitch line NEVER shows through
                };

                let text_color = if is_selected {
                    Color32::WHITE
                } else {
                    MelodyneTheme::TEXT_GOLD_LABEL
                };

                pending_lyric_tags.push((pill_rect, pill_bg, note.lyric.clone(), text_color));
            }

            let lyric_trimmed = note.lyric.trim();
            if lyric_trimmed != "+" && !lyric_trimmed.starts_with("+ ") {
                if let Some(phoneme) = state.phoneme_cache.get(idx) {
                    if !phoneme.is_empty() && phoneme != lyric_trimmed {
                        let pill_w = (phoneme.len() as f32 * 6.5 + 10.0).max(20.0);
                        let pill_h = 15.0f32;
                        let pill_cx = (x_start + x_end) * 0.5;
                        let pill_rect = Rect::from_center_size(
                            Pos2::new(pill_cx, y_top - pill_h * 0.5 - 1.0),
                            Vec2::new(pill_w, pill_h),
                        );
                        pending_phoneme_badges.push((pill_rect, phoneme.clone()));
                    }
                }
            }

            let e = &note.envelope;
            if e.crossfade_ms > 0.0 {
                let crossfade_end_x = (x_start
                    + (e.crossfade_ms.min(note.duration_ms) * state.px_per_ms as f64) as f32)
                    .min(x_end);
                let crossfade_rect = Rect::from_min_max(
                    Pos2::new(x_start, y_top),
                    Pos2::new(crossfade_end_x, y_bottom),
                );
                painter.rect_filled(
                    crossfade_rect,
                    Rounding::ZERO,
                    Color32::from_rgba_unmultiplied(0, 200, 180, 26),
                );
                let cross_color = Color32::from_rgb(0, 220, 195);
                painter.line_segment(
                    [
                        Pos2::new(x_start, y_bottom),
                        Pos2::new(crossfade_end_x, y_top),
                    ],
                    Stroke::new(1.5_f32, cross_color),
                );
                painter.line_segment(
                    [
                        Pos2::new(x_start, y_top),
                        Pos2::new(crossfade_end_x, y_bottom),
                    ],
                    Stroke::new(1.5_f32, cross_color),
                );
                if is_selected && crossfade_end_x - x_start > 24.0 {
                    painter.text(
                        crossfade_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        format!("X {:.0}ms", e.crossfade_ms),
                        egui::FontId::proportional(9.0),
                        Color32::from_rgb(220, 255, 245),
                    );
                }
            }
            let env_pts = [
                (e.p1, e.v1),
                (e.p1 + e.p2, e.v2),
                (e.p1 + e.p2 + e.p3, e.v3),
                ((note.duration_ms - e.p4).max(0.0), e.v4),
                ((note.duration_ms - e.p4 + e.p5).max(0.0), e.v5),
            ];

            let mut env_screen_pts = Vec::with_capacity(5);
            for (t_ms, vol) in env_pts.iter() {
                let px_x = x_start + (*t_ms * state.px_per_ms as f64) as f32;
                let px_y = y_bottom - (*vol / 100.0).clamp(0.0, 1.0) as f32 * (y_bottom - y_top);
                env_screen_pts.push(Pos2::new(px_x, px_y));
            }

            let env_color = if is_selected {
                Color32::from_rgba_unmultiplied(0, 225, 250, 175)
            } else {
                Color32::from_rgba_unmultiplied(0, 180, 220, 50)
            };

            if let Some(first) = env_screen_pts.first() {
                painter.line_segment(
                    [Pos2::new(x_start, y_bottom), *first],
                    Stroke::new(1.3_f32, env_color),
                );
            }
            for i in 0..env_screen_pts.len().saturating_sub(1) {
                painter.line_segment(
                    [env_screen_pts[i], env_screen_pts[i + 1]],
                    Stroke::new(1.3_f32, env_color),
                );
            }
            if let Some(last) = env_screen_pts.last() {
                painter.line_segment(
                    [*last, Pos2::new(x_end, y_bottom)],
                    Stroke::new(1.3_f32, env_color),
                );
            }

            if is_selected && state.show_envelope_handles {
                for (pt_i, pt) in env_screen_pts.iter().enumerate() {
                    let is_pt_hover = mouse_interact_pos.is_some_and(|m| m.distance(*pt) <= 12.0);
                    let is_pt_drag = state.dragging_envelope_pt == Some((idx, pt_i));
                    painter.circle_filled(
                        *pt,
                        if is_pt_hover || is_pt_drag { 7.5 } else { 5.0 },
                        if is_pt_drag {
                            Color32::WHITE
                        } else {
                            Color32::from_rgb(0, 235, 255)
                        },
                    );
                    painter.circle_stroke(
                        *pt,
                        if is_pt_hover || is_pt_drag { 7.5 } else { 5.0 },
                        Stroke::new(
                            1.0_f32,
                            if is_pt_drag {
                                Color32::from_rgb(0, 210, 240)
                            } else {
                                Color32::from_rgb(15, 25, 40)
                            },
                        ),
                    );
                }
            }

            let has_vibrato = note.vibrato.length_pct > 0.0 && note.vibrato.depth_cents > 0.0;
            let vib_start_t = if has_vibrato {
                note.duration_ms * (1.0 - (note.vibrato.length_pct / 100.0).clamp(0.0, 1.0))
            } else {
                f64::INFINITY
            };

            if has_vibrato && vib_start_t < note.duration_ms {
                let vib_px_start =
                    (x_start + (vib_start_t * state.px_per_ms as f64) as f32).max(x_start);
                let vib_px_end = x_end;
                if vib_px_end > vib_px_start {
                    let vib_rect = Rect::from_min_max(
                        Pos2::new(vib_px_start, y_top),
                        Pos2::new(vib_px_end, y_bottom),
                    );
                    painter.rect_filled(
                        vib_rect,
                        Rounding::same(4.0),
                        Color32::from_rgba_unmultiplied(
                            0,
                            220,
                            255,
                            if is_selected { 35 } else { 18 },
                        ),
                    );
                }
            }

            // resampler. The first point snaps to the previous adjacent note.
            let (previous_midi, is_adjacent) = if idx > 0 {
                let (prev_m, prev_pos, prev_dur, _) = note_info[idx - 1];
                let adj = (prev_pos + prev_dur - note.position_ms).abs() <= 1.0;
                (Some(prev_m), adj)
            } else {
                (None, false)
            };

            let pitch_curve =
                note.pitch_bend
                    .effective_points(previous_midi, note.midi_key(), is_adjacent);

            let min_t = pitch_curve.first().map(|p| p.time_offset_ms).unwrap_or(0.0);
            let max_t = if let Some(&(_next_m, next_pos, _next_dur, next_first_t)) =
                note_info.get(idx + 1)
            {
                let next_adjacent = (note.position_ms + note.duration_ms - next_pos).abs() <= 1.0;
                let next_start = next_pos + next_first_t;
                if next_adjacent {
                    (next_start - note.position_ms).max(min_t)
                } else {
                    note.duration_ms
                }
            } else {
                note.duration_ms
            };

            let visible_note_start = ((visible_clip.min.x - x_start) / state.px_per_ms) as f64;
            let visible_note_end = ((visible_clip.max.x - x_start) / state.px_per_ms) as f64;
            let draw_min_t = min_t.max(visible_note_start);
            let draw_max_t = max_t.min(visible_note_end);
            let step = (2.0 / state.px_per_ms.max(0.001)) as f64;
            let capacity = if draw_max_t > draw_min_t {
                ((draw_max_t - draw_min_t) / step).ceil() as usize + 2
            } else {
                0
            };
            let mut spline_points = Vec::with_capacity(capacity);
            let mut t = draw_min_t;
            while t <= draw_max_t {
                let mut offset_cents =
                    PitchBendSolver::get_pitch_offset_cents_sorted(t, &pitch_curve);

                if has_vibrato && t >= vib_start_t {
                    let vib_t = t - vib_start_t;
                    let vib_dur = (note.duration_ms - vib_start_t).max(1.0);
                    let fade_in_dur = vib_dur * (note.vibrato.fade_in_pct / 100.0).clamp(0.0, 1.0);
                    let fade_in = if fade_in_dur > 0.0 {
                        (vib_t / fade_in_dur).min(1.0)
                    } else {
                        1.0
                    };
                    let fade_out_dur =
                        vib_dur * (note.vibrato.fade_out_pct / 100.0).clamp(0.0, 1.0);
                    let fade_out_start = vib_dur - fade_out_dur;
                    let fade_out = if vib_t >= fade_out_start && fade_out_dur > 0.0 {
                        ((vib_dur - vib_t) / fade_out_dur).clamp(0.0, 1.0)
                    } else {
                        1.0
                    };
                    let period_ms = note.vibrato.period_ms.clamp(40.0, 1000.0);
                    let phase = (vib_t / period_ms) * std::f64::consts::TAU
                        + (note.vibrato.shift_pct / 100.0) * std::f64::consts::TAU;
                    let vib_mod = phase.sin() * note.vibrato.depth_cents * fade_in * fade_out;
                    offset_cents += vib_mod;
                }

                let px_x = x_start + (t * state.px_per_ms as f64) as f32;
                let px_y = y_center - (offset_cents / 100.0) as f32 * state.row_height;
                spline_points.push(Pos2::new(px_x, px_y));
                t += step;
            }

            let in_pitch_mode = state.active_tool == EditTool::PitchDraw;
            let pitch_stroke = if in_pitch_mode {
                Stroke::new(1.6_f32, Color32::from_rgba_unmultiplied(255, 215, 60, 120))
            } else {
                Stroke::new(1.2_f32, Color32::from_rgba_unmultiplied(255, 215, 0, 70))
            };
            if spline_points.len() >= 2 {
                painter.add(egui::Shape::line(spline_points, pitch_stroke));
            }

            if in_pitch_mode {
                let mut last_rendered_x: Option<f32> = None;
                for (pt_idx, pt) in pitch_curve.iter().enumerate() {
                    let px_x = x_start + (pt.time_offset_ms * state.px_per_ms as f64) as f32;
                    let px_y = y_center - (pt.pitch_offset_cents / 100.0) as f32 * state.row_height;
                    let pt_pos = Pos2::new(px_x, px_y);
                    if visible_clip.contains(pt_pos)
                        || (px_x >= visible_clip.min.x && px_x <= visible_clip.max.x)
                    {
                        let is_active_pt = state.dragging_pitch_pt == Some((idx, pt_idx));
                        let is_hovered = mouse_interact_pos
                            .map(|m| m.distance(pt_pos) <= 8.0)
                            .unwrap_or(false);

                        let is_first_or_last = pt_idx == 0 || pt_idx + 1 == pitch_curve.len();
                        let dist_ok =
                            last_rendered_x.map_or(true, |last_x| (px_x - last_x).abs() >= 9.0);

                        if is_active_pt || is_hovered || is_first_or_last || dist_ok {
                            last_rendered_x = Some(px_x);

                            let radius = if is_active_pt {
                                3.8
                            } else if is_hovered {
                                3.4
                            } else {
                                2.3
                            };

                            if is_active_pt {
                                painter.circle_filled(
                                    pt_pos,
                                    radius + 2.5,
                                    Color32::from_rgba_unmultiplied(255, 215, 80, 85),
                                );
                                painter.circle_filled(
                                    pt_pos,
                                    radius,
                                    Color32::from_rgb(255, 240, 160),
                                );
                                painter.circle_stroke(
                                    pt_pos,
                                    radius,
                                    Stroke::new(1.0_f32, Color32::WHITE),
                                );
                            } else if is_hovered {
                                painter.circle_filled(
                                    pt_pos,
                                    radius + 2.5,
                                    Color32::from_rgba_unmultiplied(0, 230, 255, 95),
                                );
                                painter.circle_filled(
                                    pt_pos,
                                    radius,
                                    Color32::from_rgb(180, 245, 255),
                                );
                                painter.circle_stroke(
                                    pt_pos,
                                    radius,
                                    Stroke::new(1.0_f32, Color32::WHITE),
                                );
                            } else {
                                painter.circle_filled(
                                    pt_pos,
                                    radius,
                                    Color32::from_rgb(255, 220, 100),
                                );
                                painter.circle_stroke(
                                    pt_pos,
                                    radius,
                                    Stroke::new(
                                        0.8_f32,
                                        Color32::from_rgba_unmultiplied(20, 15, 28, 220),
                                    ),
                                );
                            }

                            if is_hovered || is_active_pt {
                                let shape_txt = match pt.shape.to_lowercase().as_str() {
                                    "l" => "Linear (l)",
                                    "j" | "i" => "J-Curve (j)",
                                    "r" | "o" => "R-Curve (r)",
                                    _ => "S-Curve (s)",
                                };
                                painter.text(
                                    Pos2::new(pt_pos.x, pt_pos.y - 10.0),
                                    egui::Align2::CENTER_BOTTOM,
                                    shape_txt,
                                    egui::FontId::proportional(8.5),
                                    Color32::from_rgb(220, 240, 255),
                                );
                            }
                        }
                    }
                }
            }

            if in_pitch_mode && !state.pitch_brush_raw_stroke.is_empty() {
                let stroke_pts: Vec<Pos2> = state
                    .pitch_brush_raw_stroke
                    .iter()
                    .filter(|(n_idx, _, _)| *n_idx == idx)
                    .map(|(_, t, c)| {
                        let px_x = x_start + (t * state.px_per_ms as f64) as f32;
                        let px_y = y_center - (c / 100.0) as f32 * state.row_height;
                        Pos2::new(px_x, px_y)
                    })
                    .collect();
                if stroke_pts.len() >= 2 {
                    for pair in stroke_pts.windows(2) {
                        painter.line_segment(
                            [pair[0], pair[1]],
                            Stroke::new(3.5_f32, Color32::from_rgba_unmultiplied(255, 215, 0, 160)),
                        );
                        painter.line_segment(
                            [pair[0], pair[1]],
                            Stroke::new(1.8_f32, Color32::from_rgb(255, 255, 255)),
                        );
                    }
                }
            }

            if is_selected && !state.is_playing {
                let tb_w = 175.0f32;
                let tb_h = 22.0f32;
                let tb_x = x_start;
                let tb_y = if y_top - tb_h - 4.0 >= visible_clip.min.y {
                    y_top - tb_h - 4.0
                } else {
                    y_bottom + 4.0
                };
                let tb_rect = Rect::from_min_size(Pos2::new(tb_x, tb_y), Vec2::new(tb_w, tb_h));

                if mouse_interact_pos
                    .map(|m| tb_rect.contains(m))
                    .unwrap_or(false)
                {
                    interacted_with_note_or_ui = true;
                }

                painter.rect_filled(tb_rect, Rounding::same(5.0), Color32::from_rgb(15, 12, 24));
                painter.rect_stroke(
                    tb_rect,
                    Rounding::same(5.0),
                    Stroke::new(1.0_f32, Color32::from_rgb(255, 215, 0)),
                );

                let btn_vib_rect =
                    Rect::from_min_size(Pos2::new(tb_x + 2.0, tb_y + 2.0), Vec2::new(40.0, 18.0));
                let vib_hover = mouse_interact_pos
                    .map(|m| btn_vib_rect.contains(m))
                    .unwrap_or(false);
                let vib_active = state.vibrato_popover_note_idx == Some(idx);
                painter.rect_filled(
                    btn_vib_rect,
                    Rounding::same(3.0),
                    if vib_active {
                        Color32::from_rgb(0, 180, 220)
                    } else if vib_hover {
                        Color32::from_rgb(35, 30, 50)
                    } else {
                        Color32::TRANSPARENT
                    },
                );
                painter.text(
                    btn_vib_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "〰 Vib",
                    egui::FontId::proportional(10.0),
                    if vib_active {
                        Color32::WHITE
                    } else {
                        Color32::from_rgb(0, 240, 255)
                    },
                );
                if vib_hover && ui.input(|i| i.pointer.primary_clicked()) {
                    state.vibrato_popover_note_idx = if vib_active { None } else { Some(idx) };
                    interacted_with_note_or_ui = true;
                }

                let btn_env_rect =
                    Rect::from_min_size(Pos2::new(tb_x + 44.0, tb_y + 2.0), Vec2::new(40.0, 18.0));
                let env_hover = mouse_interact_pos
                    .map(|m| btn_env_rect.contains(m))
                    .unwrap_or(false);
                let env_active = state.show_envelope_handles;
                painter.rect_filled(
                    btn_env_rect,
                    Rounding::same(3.0),
                    if env_active {
                        Color32::from_rgb(0, 145, 175)
                    } else if env_hover {
                        Color32::from_rgb(35, 30, 50)
                    } else {
                        Color32::TRANSPARENT
                    },
                );
                painter.text(
                    btn_env_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "🎚 Env",
                    egui::FontId::proportional(10.0),
                    if env_active {
                        Color32::WHITE
                    } else {
                        Color32::from_rgb(0, 215, 240)
                    },
                );
                if env_hover && ui.input(|i| i.pointer.primary_clicked()) {
                    state.show_envelope_handles = !state.show_envelope_handles;
                    interacted_with_note_or_ui = true;
                }

                let btn_pit_rect =
                    Rect::from_min_size(Pos2::new(tb_x + 86.0, tb_y + 2.0), Vec2::new(44.0, 18.0));
                let pit_hover = mouse_interact_pos
                    .map(|m| btn_pit_rect.contains(m))
                    .unwrap_or(false);
                let pit_active = state.active_tool == EditTool::PitchDraw;
                painter.rect_filled(
                    btn_pit_rect,
                    Rounding::same(3.0),
                    if pit_active {
                        Color32::from_rgb(200, 160, 20)
                    } else if pit_hover {
                        Color32::from_rgb(35, 30, 50)
                    } else {
                        Color32::TRANSPARENT
                    },
                );
                painter.text(
                    btn_pit_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "📈 Pitch",
                    egui::FontId::proportional(10.0),
                    if pit_active {
                        Color32::WHITE
                    } else {
                        Color32::from_rgb(255, 215, 0)
                    },
                );
                if pit_hover && ui.input(|i| i.pointer.primary_clicked()) {
                    state.active_tool = if pit_active {
                        EditTool::Pointer
                    } else {
                        EditTool::PitchDraw
                    };
                    interacted_with_note_or_ui = true;
                }

                let btn_prop_rect =
                    Rect::from_min_size(Pos2::new(tb_x + 132.0, tb_y + 2.0), Vec2::new(40.0, 18.0));
                let prop_hover = mouse_interact_pos
                    .map(|m| btn_prop_rect.contains(m))
                    .unwrap_or(false);
                painter.rect_filled(
                    btn_prop_rect,
                    Rounding::same(3.0),
                    if prop_hover {
                        Color32::from_rgb(50, 45, 70)
                    } else {
                        Color32::TRANSPARENT
                    },
                );
                painter.text(
                    btn_prop_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "⚙ Prop",
                    egui::FontId::proportional(10.0),
                    Color32::from_rgb(220, 210, 235),
                );
                if prop_hover && ui.input(|i| i.pointer.primary_clicked()) {
                    state.properties_window_for_note = Some(idx);
                    interacted_with_note_or_ui = true;
                }

                if state.vibrato_popover_note_idx == Some(idx) {
                    let pop_w = 200.0f32;
                    let pop_h = 24.0f32;
                    let pop_rect = Rect::from_min_size(
                        Pos2::new(tb_x, tb_y - pop_h - 2.0),
                        Vec2::new(pop_w, pop_h),
                    );
                    if mouse_interact_pos
                        .map(|m| pop_rect.contains(m))
                        .unwrap_or(false)
                    {
                        interacted_with_note_or_ui = true;
                    }
                    painter.rect_filled(
                        pop_rect,
                        Rounding::same(4.0),
                        Color32::from_rgb(18, 14, 28),
                    );
                    painter.rect_stroke(
                        pop_rect,
                        Rounding::same(4.0),
                        Stroke::new(1.0_f32, Color32::from_rgb(0, 220, 255)),
                    );

                    let presets = [
                        ("🌸 Pop", 65.0, 48.0, 175.0),
                        ("🎭 Drama", 75.0, 75.0, 160.0),
                        ("🍃 Lento", 80.0, 50.0, 220.0),
                        ("⚡ Fast", 60.0, 60.0, 140.0),
                        ("🚫 Off", 0.0, 0.0, 175.0),
                    ];
                    let btn_w = (pop_w - 4.0) / presets.len() as f32;
                    for (p_i, &(label, len, dep, per)) in presets.iter().enumerate() {
                        let p_btn_rect = Rect::from_min_size(
                            Pos2::new(tb_x + 2.0 + p_i as f32 * btn_w, tb_y - pop_h),
                            Vec2::new(btn_w - 2.0, pop_h - 4.0),
                        );
                        let p_hov = mouse_interact_pos
                            .map(|m| p_btn_rect.contains(m))
                            .unwrap_or(false);
                        if p_hov {
                            painter.rect_filled(
                                p_btn_rect,
                                Rounding::same(2.0),
                                Color32::from_rgb(40, 35, 60),
                            );
                        }
                        painter.text(
                            p_btn_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            label,
                            egui::FontId::proportional(9.0),
                            Color32::from_rgb(200, 230, 255),
                        );
                        if p_hov && ui.input(|i| i.pointer.primary_clicked()) {
                            note.vibrato.length_pct = len;
                            note.vibrato.depth_cents = dep;
                            note.vibrato.period_ms = per;
                            state.vibrato_popover_note_idx = None;
                            state.continuous_edit_dirty = true;
                            interacted_with_note_or_ui = true;
                        }
                    }
                }
            }

            if in_pitch_mode && state.pitch_sub_tool == PitchSubTool::Line {
                if let Some((line_note_idx, l_start_t, l_start_cents)) = state.pitch_line_start {
                    if line_note_idx == idx {
                        if let Some(mpos) = mouse_interact_pos {
                            let start_px_x = x_start + (l_start_t * state.px_per_ms as f64) as f32;
                            let start_px_y =
                                y_center - (l_start_cents / 100.0) as f32 * state.row_height;
                            let end_px_x = mpos.x;
                            let end_px_y = mpos.y;
                            painter.line_segment(
                                [
                                    Pos2::new(start_px_x, start_px_y),
                                    Pos2::new(end_px_x, end_px_y),
                                ],
                                Stroke::new(1.5_f32, Color32::from_rgb(255, 220, 100)),
                            );
                        }
                    }
                }
            }

            if let Some(mpos) = mouse_interact_pos {
                let resize_handle_right = Rect::from_min_max(
                    Pos2::new(x_end - 8.0, y_top),
                    Pos2::new(x_end + 4.0, y_bottom),
                );

                let resize_handle_left = Rect::from_min_max(
                    Pos2::new(x_start - 4.0, y_top),
                    Pos2::new(x_start + 8.0, y_bottom),
                );

                if note_rect.contains(mpos)
                    && (state.active_tool == EditTool::Pointer
                        || state.active_tool == EditTool::Pencil)
                    && (resize_handle_left.contains(mpos) || resize_handle_right.contains(mpos))
                {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                }

                let pitch_draw_target_rect = Rect::from_min_max(
                    Pos2::new(
                        x_start - (200.0 * state.px_per_ms as f64) as f32,
                        y_top - state.row_height * 4.0,
                    ),
                    Pos2::new(
                        x_end + (200.0 * state.px_per_ms as f64) as f32,
                        y_bottom + state.row_height * 4.0,
                    ),
                );

                if note_rect.contains(mpos) && mpos.y > grid_start_y && mpos.y < grid_end_y {
                    interacted_with_note_or_ui = true;

                    if state.active_tool == EditTool::Pointer
                        || state.active_tool == EditTool::Pencil
                    {
                        let is_double_clicked = ui.input(|i| {
                            i.pointer
                                .button_double_clicked(egui::PointerButton::Primary)
                        });
                        let is_enter_on_note = !was_editing_lyric
                            && ui.input(|i| {
                                i.key_pressed(egui::Key::Enter)
                                    && !i.modifiers.command
                                    && !i.modifiers.alt
                            });
                        if is_double_clicked || is_enter_on_note {
                            state.editing_lyric_index = Some(idx);
                            state.lyric_buffer = note.lyric.clone();
                            state.autocomplete_selected_idx = 0;
                            state.lyric_needs_select_all = true;
                            state.selected_note_index = Some(idx);
                            state.selected_note_indices.clear();
                            state.selected_note_indices.insert(idx);
                        }

                        let hovered_env_pt = if is_selected && state.show_envelope_handles {
                            env_screen_pts
                                .iter()
                                .position(|pt| mpos.distance(*pt) <= 13.0)
                        } else {
                            None
                        };
                        let is_over_envelope_handle = is_selected
                            && state.show_envelope_handles
                            && (state.dragging_envelope_pt.is_some() || hovered_env_pt.is_some());
                        let is_over_pitch_anchor = state.active_tool == EditTool::PitchDraw
                            && (state.dragging_pitch_pt.is_some()
                                || pitch_curve.iter().any(|pt| {
                                    let px_x = x_start
                                        + (pt.time_offset_ms * state.px_per_ms as f64) as f32;
                                    let px_y = y_center
                                        - (pt.pitch_offset_cents / 100.0) as f32 * state.row_height;
                                    mpos.distance(Pos2::new(px_x, px_y)) <= 10.0
                                }));

                        let just_pressed = ui.input(|i| i.pointer.primary_pressed());
                        if just_pressed && hovered_env_pt.is_some() {
                            if let Some(pt_idx) = hovered_env_pt {
                                on_before_change();
                                state.dragging_envelope_pt = Some((idx, pt_idx));
                            }
                        }

                        if let Some((drag_idx, pt_idx)) = state.dragging_envelope_pt {
                            if drag_idx == idx && ui.input(|i| i.pointer.primary_down()) {
                                let duration = note.duration_ms.max(1.0);
                                let time = (f64::from(mpos.x - x_start)
                                    / f64::from(state.px_per_ms))
                                .max(0.0);
                                let volume = ((y_bottom - mpos.y) / (y_bottom - y_top))
                                    .clamp(0.0, 1.0)
                                    as f64
                                    * 100.0;
                                match pt_idx {
                                    0 => {
                                        note.envelope.p1 = time.clamp(0.0, duration);
                                        note.envelope.v1 = volume.clamp(0.0, 100.0);
                                    }
                                    1 => {
                                        let p2_time = time.max(note.envelope.p1);
                                        note.envelope.p2 = (p2_time - note.envelope.p1).max(0.0);
                                        note.envelope.v2 = volume.clamp(0.0, 100.0);
                                    }
                                    2 => {
                                        let p3_time = time.max(note.envelope.p1 + note.envelope.p2);
                                        note.envelope.p3 =
                                            (p3_time - note.envelope.p1 - note.envelope.p2)
                                                .max(0.0);
                                        note.envelope.v3 = volume.clamp(0.0, 100.0);
                                    }
                                    3 => {
                                        let p4_pos = time.clamp(0.0, duration + 200.0);
                                        note.envelope.p4 = (duration - p4_pos).max(0.0);
                                        note.envelope.v4 = volume.clamp(0.0, 100.0);
                                    }
                                    4 => {
                                        let p4_pos = (duration - note.envelope.p4).max(0.0);
                                        let p5_pos = time.max(p4_pos);
                                        note.envelope.p5 = (p5_pos - p4_pos).max(0.0);
                                        note.envelope.v5 = volume.clamp(0.0, 100.0);
                                    }
                                    5 => {
                                        note.envelope.crossfade_ms = (f64::from(x_start - mpos.x)
                                            / f64::from(state.px_per_ms))
                                        .clamp(0.0, 600.0);
                                    }
                                    _ => {}
                                }
                                state.continuous_edit_dirty = true;
                            }
                        }

                        // Use primary_pressed() (not primary_clicked()) so drag starts
                        // on mouse-down, enabling click-and-drag for resize/move
                        if just_pressed
                            && !is_editing_lyric
                            && state.dragging_note_idx.is_none()
                            && !is_over_envelope_handle
                            && !is_over_pitch_anchor
                        {
                            on_before_change();
                            state.selected_note_index = Some(idx);
                            if !state.selected_note_indices.contains(&idx) {
                                state.selected_note_indices.clear();
                                state.selected_note_indices.insert(idx);
                            }
                            state.dragging_note_idx = Some(idx);
                            state.drag_start_pos = Some(mpos);
                            state.note_original_start_ms = note.position_ms;
                            state.note_original_duration_ms = note.duration_ms;
                            state.note_original_midi = note_midi;
                            state.dragging_is_resize = resize_handle_right.contains(mpos);
                            state.dragging_is_left_resize = resize_handle_left.contains(mpos);
                        }

                        if ui.input(|i| i.pointer.secondary_clicked()) {
                            if !state.selected_note_indices.contains(&idx) {
                                state.selected_note_indices.clear();
                                state.selected_note_indices.insert(idx);
                                state.selected_note_index = Some(idx);
                            }
                            state.context_menu_note_idx = Some(idx);
                            state.context_menu_pos = Some(mpos);
                        }
                    } else if state.active_tool == EditTool::Slice {
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                        if ui.input(|i| i.pointer.primary_clicked()) {
                            let click_t =
                                ((mpos.x - (rect.min.x + keyboard_width)) / state.px_per_ms) as f64;
                            let snapped_t = apply_snap(click_t, snap_option, bpm);
                            let slice_t = if (snapped_t - note.position_ms) >= 15.0
                                && (note.position_ms + note.duration_ms - snapped_t) >= 15.0
                            {
                                snapped_t
                            } else {
                                click_t
                            };
                            note_to_slice = Some((idx, slice_t));
                        }
                    } else if state.active_tool == EditTool::Eraser {
                        if ui.input(|i| i.pointer.primary_clicked()) {
                            note_to_delete = Some(idx);
                        }
                    }
                }

                if state.active_tool == EditTool::PitchDraw && pitch_draw_target_rect.contains(mpos)
                {
                    let mut hovered_pitch_pt: Option<usize> = None;
                    for (pt_idx, pt) in pitch_curve.iter().enumerate() {
                        let px_x = x_start + (pt.time_offset_ms * state.px_per_ms as f64) as f32;
                        let px_y =
                            y_center - (pt.pitch_offset_cents / 100.0) as f32 * state.row_height;
                        let pt_pos = Pos2::new(px_x, px_y);
                        if mpos.distance(pt_pos) <= 8.0 {
                            hovered_pitch_pt = Some(pt_idx);
                            ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                            break;
                        }
                    }

                    let rel_t = ((mpos.x - x_start) / state.px_per_ms) as f64;
                    let delta_y = y_center - mpos.y;
                    let cents =
                        (((delta_y / state.row_height) * 100.0) as f64).clamp(-1200.0, 1200.0);
                    let is_alt = ui.input(|i| i.modifiers.alt);
                    let is_shift = ui.input(|i| i.modifiers.shift);

                    if let Some(pt_idx) = hovered_pitch_pt {
                        if ui.input(|i| i.pointer.primary_clicked() && i.modifiers.alt)
                            || ui.input(|i| i.pointer.secondary_clicked())
                        {
                            if note.pitch_bend.points.is_empty() {
                                note.pitch_bend.points = pitch_curve.clone();
                            }
                            if pt_idx < note.pitch_bend.points.len() {
                                note.pitch_bend.points.remove(pt_idx);
                                state.dragging_pitch_pt = None;
                                on_note_changed();
                            }
                        }
                    } else if ui.input(|i| i.pointer.secondary_clicked()) {
                        note.pitch_bend.points.clear();
                        state.dragging_pitch_pt = None;
                        state.pitch_line_start = None;
                        on_note_changed();
                    }

                    if hovered_pitch_pt.is_none()
                        && (ui.input(|i| {
                            i.pointer
                                .button_double_clicked(egui::PointerButton::Primary)
                        }) || (ui.input(|i| i.pointer.primary_clicked()) && is_shift))
                    {
                        if note.pitch_bend.points.is_empty() {
                            note.pitch_bend.points = pitch_curve.clone();
                        }
                        note.pitch_bend.points.push(UPitchBendPoint {
                            time_offset_ms: rel_t,
                            pitch_offset_cents: cents,
                            shape: "s".to_string(),
                        });
                        note.pitch_bend.points.sort_by(|a, b| {
                            a.time_offset_ms
                                .partial_cmp(&b.time_offset_ms)
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                        on_note_changed();
                    }

                    if ui.input(|i| i.pointer.primary_pressed()) && !is_alt && !is_shift {
                        on_before_change();
                        if let Some(pt_idx) = hovered_pitch_pt {
                            if note.pitch_bend.points.is_empty() {
                                note.pitch_bend.points = pitch_curve.clone();
                            }
                            if pt_idx < note.pitch_bend.points.len() {
                                state.dragging_pitch_pt = Some((idx, pt_idx));
                            }
                        } else if state.pitch_sub_tool == PitchSubTool::Line {
                            state.pitch_line_start = Some((idx, rel_t, cents));
                        } else if state.pitch_sub_tool == PitchSubTool::Freehand {
                            state.pitch_brush_raw_stroke.clear();
                            state.pitch_brush_raw_stroke.push((idx, rel_t, cents));
                        }
                    }

                    if ui.input(|i| i.pointer.primary_down()) && !is_alt {
                        if let Some((d_note_idx, d_pt_idx)) = state.dragging_pitch_pt {
                            if d_note_idx == idx {
                                if note.pitch_bend.points.is_empty() {
                                    note.pitch_bend.points = pitch_curve.clone();
                                }
                                if d_pt_idx < note.pitch_bend.points.len() {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                                    note.pitch_bend.points[d_pt_idx].time_offset_ms = rel_t;
                                    note.pitch_bend.points[d_pt_idx].pitch_offset_cents = cents;
                                    state.continuous_edit_dirty = true;
                                }
                            }
                        } else {
                            match state.pitch_sub_tool {
                                PitchSubTool::Freehand => {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                                    let should_add = state.pitch_brush_raw_stroke.last().map_or(
                                        true,
                                        |&(_, last_t, last_c)| {
                                            (last_t - rel_t).abs() >= 4.0
                                                || (last_c - cents).abs() >= 5.0
                                        },
                                    );
                                    if should_add {
                                        state.pitch_brush_raw_stroke.push((idx, rel_t, cents));
                                    }
                                }
                                PitchSubTool::Line => {
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                                }
                                PitchSubTool::Vibrato => {
                                    if note.pitch_bend.points.is_empty() {
                                        note.pitch_bend.points = pitch_curve.clone();
                                    }
                                    let min_dist_ms = 15.0;
                                    let vibrato_freq = 5.5; // ~5.5 Hz
                                    let vibrato_amp =
                                        (delta_y.abs() * 2.5).clamp(25.0, 180.0) as f64;
                                    let phase =
                                        (rel_t / 1000.0) * vibrato_freq * std::f64::consts::TAU;
                                    let vib_cents =
                                        (phase.sin() * vibrato_amp).clamp(-1200.0, 1200.0);

                                    note.pitch_bend.points.retain(|pt| {
                                        (pt.time_offset_ms - rel_t).abs() >= min_dist_ms
                                    });
                                    note.pitch_bend.points.push(UPitchBendPoint {
                                        time_offset_ms: rel_t,
                                        pitch_offset_cents: vib_cents,
                                        shape: "s".to_string(),
                                    });
                                    note.pitch_bend.points.sort_by(|a, b| {
                                        a.time_offset_ms
                                            .partial_cmp(&b.time_offset_ms)
                                            .unwrap_or(std::cmp::Ordering::Equal)
                                    });
                                    state.continuous_edit_dirty = true;
                                }
                                PitchSubTool::Smooth => {
                                    if !note.pitch_bend.points.is_empty() {
                                        let radius_ms = 60.0;
                                        for pt_i in 0..note.pitch_bend.points.len() {
                                            let p_time =
                                                note.pitch_bend.points[pt_i].time_offset_ms;
                                            if (p_time - rel_t).abs() <= radius_ms {
                                                let prev_cents = if pt_i > 0 {
                                                    note.pitch_bend.points[pt_i - 1]
                                                        .pitch_offset_cents
                                                } else {
                                                    note.pitch_bend.points[pt_i].pitch_offset_cents
                                                };
                                                let next_cents = if pt_i + 1
                                                    < note.pitch_bend.points.len()
                                                {
                                                    note.pitch_bend.points[pt_i + 1]
                                                        .pitch_offset_cents
                                                } else {
                                                    note.pitch_bend.points[pt_i].pitch_offset_cents
                                                };
                                                let curr =
                                                    note.pitch_bend.points[pt_i].pitch_offset_cents;
                                                note.pitch_bend.points[pt_i].pitch_offset_cents =
                                                    curr * 0.7 + (prev_cents + next_cents) * 0.15;
                                            }
                                        }
                                        state.continuous_edit_dirty = true;
                                    }
                                }
                            }
                        }
                    }

                    if ui.input(|i| i.pointer.primary_released()) {
                        if state.dragging_pitch_pt.is_some() {
                            state.dragging_pitch_pt = None;
                            note.pitch_bend.points.sort_by(|a, b| {
                                a.time_offset_ms
                                    .partial_cmp(&b.time_offset_ms)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            });
                            state.continuous_edit_dirty = true;
                        } else if state.pitch_sub_tool == PitchSubTool::Freehand
                            && !state.pitch_brush_raw_stroke.is_empty()
                        {
                            let raw_pts: Vec<(f64, f64)> = state
                                .pitch_brush_raw_stroke
                                .iter()
                                .filter(|(n_idx, _, _)| *n_idx == idx)
                                .map(|(_, t, c)| (*t, *c))
                                .collect();

                            if !raw_pts.is_empty() {
                                let smoothed = smooth_pitch_points(&raw_pts);
                                if !smoothed.is_empty() {
                                    if note.pitch_bend.points.is_empty() {
                                        note.pitch_bend.points = pitch_curve.clone();
                                    }
                                    let t_min =
                                        smoothed.first().map(|p| p.time_offset_ms).unwrap_or(0.0);
                                    let t_max =
                                        smoothed.last().map(|p| p.time_offset_ms).unwrap_or(0.0);

                                    note.pitch_bend.points.retain(|pt| {
                                        pt.time_offset_ms < t_min - 8.0
                                            || pt.time_offset_ms > t_max + 8.0
                                    });

                                    note.pitch_bend.points.extend(smoothed);
                                    note.pitch_bend.points.sort_by(|a, b| {
                                        a.time_offset_ms
                                            .partial_cmp(&b.time_offset_ms)
                                            .unwrap_or(std::cmp::Ordering::Equal)
                                    });
                                    state.continuous_edit_dirty = true;
                                    on_note_changed();
                                }
                            }
                            state.pitch_brush_raw_stroke.clear();
                        } else if state.pitch_sub_tool == PitchSubTool::Line {
                            if let Some((l_note_idx, l_start_t, l_start_cents)) =
                                state.pitch_line_start
                            {
                                if l_note_idx == idx {
                                    if note.pitch_bend.points.is_empty() {
                                        note.pitch_bend.points = pitch_curve.clone();
                                    }
                                    let (min_t, max_t) = if l_start_t <= rel_t {
                                        (l_start_t, rel_t)
                                    } else {
                                        (rel_t, l_start_t)
                                    };
                                    note.pitch_bend.points.retain(|pt| {
                                        pt.time_offset_ms < min_t || pt.time_offset_ms > max_t
                                    });

                                    note.pitch_bend.points.push(UPitchBendPoint {
                                        time_offset_ms: l_start_t,
                                        pitch_offset_cents: l_start_cents,
                                        shape: "l".to_string(),
                                    });
                                    note.pitch_bend.points.push(UPitchBendPoint {
                                        time_offset_ms: rel_t,
                                        pitch_offset_cents: cents,
                                        shape: "s".to_string(),
                                    });
                                    note.pitch_bend.points.sort_by(|a, b| {
                                        a.time_offset_ms
                                            .partial_cmp(&b.time_offset_ms)
                                            .unwrap_or(std::cmp::Ordering::Equal)
                                    });
                                    state.continuous_edit_dirty = true;
                                }
                            }
                            state.pitch_line_start = None;
                        } else if note.pitch_bend.points.len() > 2 {
                            let simplify_tol = match state.pitch_sub_tool {
                                PitchSubTool::Vibrato => 2.0,
                                PitchSubTool::Smooth => 3.0,
                                _ => 4.0,
                            };
                            let simplified =
                                crate::dsp::pitch_bend::PitchBendSolver::simplify_pitch_points(
                                    &note.pitch_bend.points,
                                    simplify_tol,
                                );
                            note.pitch_bend.points = simplified;
                            state.continuous_edit_dirty = true;
                        }
                    }
                }
            }
        }

        {
            let mut global_segments: Vec<Vec<Pos2>> = Vec::new();
            let mut current_segment: Vec<Pos2> = Vec::new();

            for (idx, note) in notes.iter().enumerate() {
                let note_midi = note.midi_key();
                if note_midi < state.min_midi || note_midi > state.max_midi {
                    continue;
                }

                let x_start_note = rect.min.x
                    + keyboard_width
                    + (note.position_ms * state.px_per_ms as f64) as f32;
                let x_end_note = x_start_note + (note.duration_ms * state.px_per_ms as f64) as f32;

                // Skip notes completely outside visible area (with margin for
                // portamento that starts before the note)
                if x_end_note < visible_clip.min.x - 200.0
                    || x_start_note > visible_clip.max.x + 200.0
                {
                    if !current_segment.is_empty() {
                        global_segments.push(std::mem::take(&mut current_segment));
                    }
                    continue;
                }

                let (previous_midi, is_adjacent) = if idx > 0 {
                    let (prev_m, prev_pos, prev_dur, _) = note_info[idx - 1];
                    let adj = (prev_pos + prev_dur - note.position_ms).abs() <= 1.0;
                    (Some(prev_m), adj)
                } else {
                    (None, false)
                };

                // Break the segment at gaps between non-adjacent notes
                if !is_adjacent && !current_segment.is_empty() {
                    global_segments.push(std::mem::take(&mut current_segment));
                }

                let pitch_curve =
                    note.pitch_bend
                        .effective_points(previous_midi, note_midi, is_adjacent);

                let min_t = pitch_curve.first().map(|p| p.time_offset_ms).unwrap_or(0.0);
                let max_t = if let Some(&(_next_m, next_pos, _next_dur, next_first_t)) =
                    note_info.get(idx + 1)
                {
                    let next_adjacent =
                        (note.position_ms + note.duration_ms - next_pos).abs() <= 1.0;
                    let next_start = next_pos + next_first_t;
                    if next_adjacent {
                        (next_start - note.position_ms).max(min_t)
                    } else {
                        note.duration_ms
                    }
                } else {
                    note.duration_ms
                };

                let has_vibrato = note.vibrato.length_pct > 0.0 && note.vibrato.depth_cents > 0.0;
                let vib_start_t = if has_vibrato {
                    note.duration_ms * (1.0 - (note.vibrato.length_pct / 100.0).clamp(0.0, 1.0))
                } else {
                    f64::INFINITY
                };

                // Only sample within the visible pixel range for performance
                let visible_note_start =
                    ((visible_clip.min.x - x_start_note) / state.px_per_ms) as f64;
                let visible_note_end =
                    ((visible_clip.max.x - x_start_note) / state.px_per_ms) as f64;
                let draw_min_t = min_t.max(visible_note_start - 20.0);
                let draw_max_t = max_t.min(visible_note_end + 20.0);
                let step = (2.0 / state.px_per_ms.max(0.001)) as f64;

                let mut t = draw_min_t;
                while t <= draw_max_t {
                    let mut offset_cents =
                        PitchBendSolver::get_pitch_offset_cents_sorted(t, &pitch_curve);

                    // Include vibrato modulation
                    if has_vibrato && t >= vib_start_t {
                        let vib_t = t - vib_start_t;
                        let vib_dur = (note.duration_ms - vib_start_t).max(1.0);
                        let fade_in_dur =
                            vib_dur * (note.vibrato.fade_in_pct / 100.0).clamp(0.0, 1.0);
                        let fade_in = if fade_in_dur > 0.0 {
                            (vib_t / fade_in_dur).min(1.0)
                        } else {
                            1.0
                        };
                        let fade_out_dur =
                            vib_dur * (note.vibrato.fade_out_pct / 100.0).clamp(0.0, 1.0);
                        let fade_out_start = vib_dur - fade_out_dur;
                        let fade_out = if vib_t >= fade_out_start && fade_out_dur > 0.0 {
                            ((vib_dur - vib_t) / fade_out_dur).clamp(0.0, 1.0)
                        } else {
                            1.0
                        };
                        let period_ms = note.vibrato.period_ms.clamp(40.0, 1000.0);
                        let phase = (vib_t / period_ms) * std::f64::consts::TAU
                            + (note.vibrato.shift_pct / 100.0) * std::f64::consts::TAU;
                        let vib_mod = phase.sin() * note.vibrato.depth_cents * fade_in * fade_out;
                        offset_cents += vib_mod;
                    }

                    let absolute_midi = note_midi as f64 + offset_cents / 100.0;
                    let px_y = grid_start_y
                        + (state.max_midi as f64 - absolute_midi) as f32 * state.row_height
                        + state.row_height * 0.5;
                    let px_x = x_start_note + (t * state.px_per_ms as f64) as f32;

                    current_segment.push(Pos2::new(px_x, px_y));
                    t += step;
                }
            }

            if !current_segment.is_empty() {
                global_segments.push(current_segment);
            }

            let global_pitch_stroke =
                Stroke::new(1.8_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 175));
            for segment in &global_segments {
                if segment.len() >= 2 {
                    painter.add(egui::Shape::line(segment.clone(), global_pitch_stroke));
                }
            }
        }

        for (pill_rect, pill_bg, lyric, text_color) in pending_lyric_tags {
            painter.rect_filled(pill_rect, Rounding::same(3.0), pill_bg);
            painter.text(
                pill_rect.center(),
                egui::Align2::CENTER_CENTER,
                &lyric,
                egui::FontId::proportional(11.0),
                text_color,
            );
        }

        for (pill_rect, phoneme) in pending_phoneme_badges {
            painter.rect_filled(pill_rect, Rounding::same(4.0), Color32::from_rgb(10, 8, 20));
            painter.rect_stroke(
                pill_rect,
                Rounding::same(4.0),
                Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(0, 220, 255, 140)),
            );
            painter.text(
                pill_rect.center(),
                egui::Align2::CENTER_CENTER,
                phoneme,
                egui::FontId::proportional(10.5),
                Color32::from_rgb(0, 235, 255),
            );
        }

        if let Some((idx, edit_rect, x_start, y_bottom)) = pending_lyric_edit {
            painter.rect_filled(
                edit_rect.expand(2.0),
                Rounding::same(5.0),
                Color32::from_rgb(14, 12, 22),
            );
            painter.rect_stroke(
                edit_rect.expand(2.0),
                Rounding::same(5.0),
                Stroke::new(2.0_f32, Color32::from_rgb(0, 220, 255)),
            );

            let mut text_lost_focus = false;
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(edit_rect), |ui| {
                let text_id = ui.make_persistent_id(format!("note_lyric_text_edit_{}", idx));

                if state.lyric_needs_select_all {
                    let char_count = state.lyric_buffer.chars().count();
                    let mut te_state = egui::text_edit::TextEditState::default();
                    te_state
                        .cursor
                        .set_char_range(Some(egui::text::CCursorRange::two(
                            egui::text::CCursor::new(0),
                            egui::text::CCursor::new(char_count),
                        )));
                    te_state.store(ui.ctx(), text_id);
                    state.lyric_needs_select_all = false;
                }

                let text_resp = ui.add(
                    egui::TextEdit::singleline(&mut state.lyric_buffer)
                        .id(text_id)
                        .text_color(Color32::WHITE)
                        .desired_width(edit_rect.width())
                        .font(egui::FontId::proportional(13.0)),
                );

                let enter_pressed =
                    text_resp.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
                if !enter_pressed
                    && ui.input(|i| !i.key_pressed(Key::Enter) && !i.key_pressed(Key::Escape))
                {
                    text_resp.request_focus();
                }
                text_lost_focus = text_resp.lost_focus();
            });

            if let Some(vb) = voicebank {
                let query = active_phoneme_query(&state.lyric_buffer);
                let has_separator = state
                    .lyric_buffer
                    .chars()
                    .any(|character| PHONEME_SEPARATORS.contains(&character));
                let cache_key = format!(
                    "{}\0{}\0{}\0{}",
                    vb.root_path.display(),
                    vb.entries.len(),
                    has_separator,
                    query
                );
                let query_changed = cache_key != state.autocomplete_cache_key;
                if query_changed {
                    state.autocomplete_cache_key = cache_key;
                    state.autocomplete_candidates.clear();
                }
                if query_changed && (!query.is_empty() || has_separator) {
                    let mut seen = HashSet::new();
                    let matches = vb.search_entries(query, "All Folders");
                    for (alias, _) in matches {
                        if seen.insert(alias.clone()) {
                            state.autocomplete_candidates.push(alias.clone());
                        }
                    }
                    state.autocomplete_candidates.sort();
                    // A janela exibe poucas linhas; limitar o cache evita cópias
                    // enormes ao listar um voicebank inteiro após um separador.
                    state.autocomplete_candidates.truncate(200);
                }
            } else {
                state.autocomplete_cache_key.clear();
                state.autocomplete_candidates.clear();
            }
            let candidates = state.autocomplete_candidates.clone();
            if !candidates.is_empty() {
                state.autocomplete_selected_idx %= candidates.len();
            }

            let popup_rect = if !candidates.is_empty() {
                let popup_pos = Pos2::new(x_start, y_bottom + 4.0);
                let popup_height = (candidates.len() as f32 * 20.0 + 24.0).min(170.0);
                Some(Rect::from_min_size(
                    popup_pos,
                    Vec2::new(180.0, popup_height),
                ))
            } else {
                None
            };

            if let Some(p_rect) = popup_rect {
                egui::Area::new(egui::Id::new(format!("oto_autocomplete_popup_{}", idx)))
                    .fixed_pos(p_rect.min)
                    .order(egui::Order::Foreground)
                    .show(ui.ctx(), |ui| {
                        egui::Frame::popup(ui.style())
                            .fill(MelodyneTheme::BG_PANEL)
                            .show(ui, |ui| {
                                ui.set_max_width(180.0);
                                ui.label(
                                    egui::RichText::new(format!(
                                        "oto.ini Suggestions ({})",
                                        candidates.len()
                                    ))
                                    .size(10.0)
                                    .color(MelodyneTheme::TEXT_GOLD_LABEL),
                                );
                                ui.separator();

                                egui::ScrollArea::vertical()
                                    .max_height(140.0)
                                    .show(ui, |ui| {
                                        for (cand_i, cand) in candidates.iter().enumerate() {
                                            let is_cand_sel =
                                                state.autocomplete_selected_idx == cand_i;
                                            let text_widget = if is_cand_sel {
                                                egui::RichText::new(cand)
                                                    .strong()
                                                    .color(MelodyneTheme::NOTE_SELECTED_GOLD)
                                            } else {
                                                egui::RichText::new(cand)
                                                    .color(Color32::from_rgb(240, 230, 210))
                                            };

                                            if ui
                                                .selectable_label(is_cand_sel, text_widget)
                                                .clicked()
                                            {
                                                commit_lyric_edit = Some((
                                                    idx,
                                                    replace_active_phoneme(
                                                        &state.lyric_buffer,
                                                        cand,
                                                    ),
                                                    None,
                                                ));
                                            }
                                        }
                                    });
                            });
                    });
            }

            if ui.input(|i| i.key_pressed(Key::ArrowDown)) && !candidates.is_empty() {
                state.autocomplete_selected_idx =
                    (state.autocomplete_selected_idx + 1) % candidates.len();
            }
            if ui.input(|i| i.key_pressed(Key::ArrowUp)) && !candidates.is_empty() {
                state.autocomplete_selected_idx = if state.autocomplete_selected_idx == 0 {
                    candidates.len() - 1
                } else {
                    state.autocomplete_selected_idx - 1
                };
            }

            let is_clicked_outside = ui.input(|i| i.pointer.primary_clicked())
                && mouse_interact_pos.is_some_and(|mpos| {
                    !edit_rect.contains(mpos) && popup_rect.is_none_or(|pr| !pr.contains(mpos))
                });

            let is_escape = ui.input(|i| i.key_pressed(Key::Escape));
            let is_enter = ui.input(|i| i.key_pressed(Key::Enter));
            let is_tab = ui.input(|i| i.key_pressed(Key::Tab));
            let is_shift = ui.input(|i| i.modifiers.shift);

            if is_escape {
                state.editing_lyric_index = None;
            } else if commit_lyric_edit.is_none()
                && (text_lost_focus || is_clicked_outside || is_enter || is_tab)
            {
                let final_lyric = if !state.lyric_buffer.trim().is_empty() {
                    state.lyric_buffer.trim().to_string()
                } else {
                    notes[idx].lyric.clone()
                };
                let next_edit_idx = if is_tab {
                    if is_shift {
                        if idx > 0 {
                            Some(idx - 1)
                        } else {
                            None
                        }
                    } else if idx + 1 < notes.len() {
                        Some(idx + 1)
                    } else {
                        None
                    }
                } else {
                    None
                };
                commit_lyric_edit = Some((idx, final_lyric, next_edit_idx));
            }
        }

        if let Some((idx, new_lyric, next_edit_idx)) = commit_lyric_edit {
            if idx < notes.len() && notes[idx].lyric != new_lyric {
                on_before_change();
                notes[idx].lyric = new_lyric;
                on_note_changed();
            }
            if let Some(next_idx) = next_edit_idx {
                if next_idx < notes.len() {
                    state.editing_lyric_index = Some(next_idx);
                    state.lyric_buffer = notes[next_idx].lyric.clone();
                    state.autocomplete_selected_idx = 0;
                    state.lyric_needs_select_all = true;
                    state.selected_note_index = Some(next_idx);
                    state.selected_note_indices.clear();
                    state.selected_note_indices.insert(next_idx);
                } else {
                    state.editing_lyric_index = None;
                }
            } else {
                state.editing_lyric_index = None;
            }
        }

        if ui.input(|i| i.pointer.primary_released()) {
            if state.dragging_envelope_pt.is_some() {
                state.dragging_envelope_pt = None;
            }
            if state.continuous_edit_dirty {
                state.continuous_edit_dirty = false;
                on_note_changed();
            }
        }

        if let Some(del_idx) = note_to_delete {
            on_before_change();
            notes.remove(del_idx);
            state.selected_note_indices.remove(&del_idx);
            if state.selected_note_index == Some(del_idx) {
                state.selected_note_index = None;
            }
            on_note_changed();
        }

        if let Some((slice_idx, slice_time_ms)) = note_to_slice {
            if slice_idx < notes.len() {
                let orig_pos = notes[slice_idx].position_ms;
                let orig_dur = notes[slice_idx].duration_ms;
                let split_offset = slice_time_ms - orig_pos;
                if split_offset >= 15.0 && split_offset <= orig_dur - 15.0 {
                    on_before_change();
                    let mut second_note = notes[slice_idx].clone();
                    notes[slice_idx].duration_ms = split_offset;
                    second_note.position_ms = slice_time_ms;
                    second_note.duration_ms = orig_dur - split_offset;
                    notes.insert(slice_idx + 1, second_note);
                    state.selected_note_index = Some(slice_idx + 1);
                    state.selected_note_indices.clear();
                    state.selected_note_indices.insert(slice_idx + 1);
                    on_note_changed();
                }
            }
        }

        if !was_editing_lyric && state.editing_lyric_index.is_none() {
            let (select_all, delete_sel, arrow_up, arrow_down, is_shift, press_enter) =
                ui.input(|i| {
                    (
                        i.modifiers.command && i.key_pressed(egui::Key::A),
                        i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace),
                        i.key_pressed(egui::Key::ArrowUp),
                        i.key_pressed(egui::Key::ArrowDown),
                        i.modifiers.shift,
                        i.key_pressed(egui::Key::Enter) && !i.modifiers.command && !i.modifiers.alt,
                    )
                });

            if press_enter {
                let target_idx = state
                    .selected_note_index
                    .or_else(|| state.selected_note_indices.iter().next().copied());
                if let Some(sel_idx) = target_idx {
                    if sel_idx < notes.len() {
                        state.editing_lyric_index = Some(sel_idx);
                        state.lyric_buffer = notes[sel_idx].lyric.clone();
                        state.autocomplete_selected_idx = 0;
                        state.lyric_needs_select_all = true;
                    }
                }
            }

            if select_all {
                state.selected_note_indices = (0..notes.len()).collect();
                if !notes.is_empty() {
                    state.selected_note_index = Some(0);
                }
            }

            if delete_sel && !state.selected_note_indices.is_empty() {
                on_before_change();
                let mut to_delete: Vec<usize> =
                    state.selected_note_indices.iter().copied().collect();
                to_delete.sort_by(|a, b| b.cmp(a));
                for d_idx in to_delete {
                    if d_idx < notes.len() {
                        notes.remove(d_idx);
                    }
                }
                state.selected_note_indices.clear();
                state.selected_note_index = None;
                on_note_changed();
            }

            if (arrow_up || arrow_down) && !state.selected_note_indices.is_empty() {
                on_before_change();
                let shift_amt = if is_shift { 12 } else { 1 };
                let delta = if arrow_up { shift_amt } else { -shift_amt };
                for &n_idx in &state.selected_note_indices {
                    if n_idx < notes.len() {
                        let curr_midi = notes[n_idx].midi_key();
                        let new_midi = (curr_midi as i16 + delta)
                            .clamp(state.min_midi as i16, state.max_midi as i16)
                            as u8;
                        notes[n_idx].pitch = midi_to_note_name(new_midi);
                    }
                }
                on_note_changed();
            }
        }

        if state.active_tool == EditTool::Pointer || state.active_tool == EditTool::Pencil {
            if let (Some(drag_idx), Some(start_pos)) =
                (state.dragging_note_idx, state.drag_start_pos)
            {
                if ui.input(|i| i.pointer.primary_down()) {
                    if let Some(current_pos) = ui.input(|i| i.pointer.interact_pos()) {
                        let delta_x = current_pos.x - start_pos.x;
                        let delta_y = current_pos.y - start_pos.y;
                        let delta_ms = delta_x as f64 / state.px_per_ms as f64;

                        if drag_idx < notes.len() {
                            if state.dragging_is_resize {
                                let raw_dur =
                                    (state.note_original_duration_ms + delta_ms).max(20.0);
                                notes[drag_idx].duration_ms =
                                    apply_snap(raw_dur, snap_option, bpm).max(20.0);
                            } else if state.dragging_is_left_resize {
                                let original_end_ms =
                                    state.note_original_start_ms + state.note_original_duration_ms;
                                let raw_new_start =
                                    (state.note_original_start_ms + delta_ms).max(0.0);
                                let new_start =
                                    apply_snap(raw_new_start, snap_option, bpm).max(0.0);
                                let new_dur = (original_end_ms - new_start).max(20.0);
                                notes[drag_idx].position_ms = new_start;
                                notes[drag_idx].duration_ms = new_dur;
                            } else {
                                // Full note move (position + pitch relative to original drag start snapshot)
                                let delta_semitones = -(delta_y / state.row_height).round() as i32;
                                let raw_pos = (state.note_original_start_ms + delta_ms).max(0.0);
                                let new_pos = apply_snap(raw_pos, snap_option, bpm).max(0.0);
                                let new_m = (state.note_original_midi as i32 + delta_semitones)
                                    .clamp(state.min_midi as i32, state.max_midi as i32)
                                    as u8;

                                notes[drag_idx].position_ms = new_pos;
                                notes[drag_idx].set_midi_key(new_m);
                            }
                        }
                    }
                } else {
                    let note_was_changed = notes.get(drag_idx).is_some_and(|note| {
                        (note.position_ms - state.note_original_start_ms).abs() > f64::EPSILON
                            || (note.duration_ms - state.note_original_duration_ms).abs()
                                > f64::EPSILON
                            || note.midi_key() != state.note_original_midi
                    });
                    state.dragging_note_idx = None;
                    state.drag_start_pos = None;
                    state.dragging_is_left_resize = false;
                    state.dragging_is_resize = false;
                    if note_was_changed {
                        on_note_changed();
                    }
                }
            }
        } else {
            state.dragging_note_idx = None;
            state.drag_start_pos = None;
            state.dragging_is_left_resize = false;
            state.dragging_is_resize = false;
        }

        if let Some(mpos) = mouse_interact_pos {
            let is_hovering_canvas = response.hovered() && ui.clip_rect().contains(mpos);
            if is_hovering_canvas
                && mpos.x > rect.min.x + keyboard_width
                && mpos.y > grid_start_y
                && mpos.y < grid_end_y
                && state.editing_lyric_index.is_none()
            {
                match state.active_tool {
                    EditTool::Pointer => {
                        if response.drag_started()
                            && !interacted_with_note_or_ui
                            && state.dragging_note_idx.is_none()
                        {
                            state.marquee_start = Some(mpos);
                            state.marquee_current = Some(mpos);
                        }

                        if response.dragged() && state.marquee_start.is_some() {
                            state.marquee_current = Some(mpos);

                            if let (Some(m_start), Some(m_curr)) =
                                (state.marquee_start, state.marquee_current)
                            {
                                let sel_rect = Rect::from_two_pos(m_start, m_curr);
                                painter.rect_filled(
                                    sel_rect,
                                    Rounding::same(2.0),
                                    Color32::from_rgba_premultiplied(245, 176, 65, 35),
                                );
                                painter.rect_stroke(
                                    sel_rect,
                                    Rounding::same(2.0),
                                    Stroke::new(1.2_f32, MelodyneTheme::NOTE_SELECTED_GOLD),
                                );

                                state.selected_note_indices.clear();
                                for (n_i, note) in notes.iter().enumerate() {
                                    let n_midi = note.midi_key();
                                    if n_midi >= state.min_midi && n_midi <= state.max_midi {
                                        let key_i = (state.max_midi - n_midi) as f32;
                                        let y_t = grid_start_y + key_i * state.row_height + 2.0;
                                        let y_b = y_t + state.row_height - 4.0;
                                        let x_s = rect.min.x
                                            + keyboard_width
                                            + (note.position_ms * state.px_per_ms as f64) as f32;
                                        let x_e = x_s
                                            + (note.duration_ms * state.px_per_ms as f64) as f32;
                                        let n_rect = Rect::from_min_max(
                                            Pos2::new(x_s, y_t),
                                            Pos2::new(x_e, y_b),
                                        );

                                        if sel_rect.intersects(n_rect) {
                                            state.selected_note_indices.insert(n_i);
                                        }
                                    }
                                }
                            }
                        }

                        if response.clicked()
                            && !interacted_with_note_or_ui
                            && state.dragging_note_idx.is_none()
                        {
                            let click_x = mpos.x - (rect.min.x + keyboard_width);
                            let raw_t = (click_x / state.px_per_ms) as f64;
                            let scrubbed_t = apply_snap(raw_t.max(0.0), snap_option, bpm);
                            state.playhead_ms = scrubbed_t;
                            on_playhead_scrubbed(scrubbed_t);
                            state.selected_note_indices.clear();
                            state.selected_note_index = None;
                        }

                        if !ui.input(|i| i.pointer.primary_down()) {
                            state.marquee_start = None;
                            state.marquee_current = None;
                        }
                    }

                    EditTool::Pencil => {
                        if ui.input(|i| i.pointer.primary_down()) {
                            if state.creating_note_idx.is_none()
                                && state.dragging_note_idx.is_none()
                                && !interacted_with_note_or_ui
                            {
                                on_before_change();
                                let click_x = mpos.x - (rect.min.x + keyboard_width);
                                let raw_start_ms = (click_x / state.px_per_ms) as f64;
                                let click_start_ms =
                                    apply_snap(raw_start_ms, snap_option, bpm).max(0.0);
                                let key_idx =
                                    ((mpos.y - grid_start_y) / state.row_height).floor() as u8;
                                let click_midi = (state.max_midi.saturating_sub(key_idx))
                                    .clamp(state.min_midi, state.max_midi);

                                let new_note = UNote::new(
                                    "ka",
                                    midi_to_note_name(click_midi),
                                    click_start_ms,
                                    50.0,
                                );
                                notes.push(new_note);
                                let new_idx = notes.len() - 1;
                                state.creating_note_idx = Some(new_idx);
                                state.selected_note_index = Some(new_idx);
                                state.drag_start_pos = Some(mpos);
                            } else if let (Some(c_idx), Some(_start_p)) =
                                (state.creating_note_idx, state.drag_start_pos)
                            {
                                if c_idx < notes.len() {
                                    let curr_x = mpos.x - (rect.min.x + keyboard_width);
                                    let curr_ms = (curr_x / state.px_per_ms) as f64;
                                    let raw_dur = (curr_ms - notes[c_idx].position_ms).max(50.0);
                                    notes[c_idx].duration_ms =
                                        apply_snap(raw_dur, snap_option, bpm).max(50.0);
                                }
                            }
                        } else if let Some(c_idx) = state.creating_note_idx.take() {
                            if c_idx < notes.len() {
                                let freq = midi_to_freq(notes[c_idx].midi_key() as f64);
                                on_preview_freq(freq);
                            }
                            state.drag_start_pos = None;
                            on_note_changed();
                        }
                    }
                    _ => {}
                }
            }
        }

        if let Some(ref dragged_alias) = phoneme_state.dragged_phoneme.clone() {
            if let Some(mpos) = mouse_interact_pos {
                let is_hovering_canvas = response.hovered() && ui.clip_rect().contains(mpos);
                if is_hovering_canvas
                    && mpos.x > rect.min.x + keyboard_width
                    && mpos.y > grid_start_y
                    && mpos.y < grid_end_y
                {
                    let key_idx = ((mpos.y - grid_start_y) / state.row_height).floor() as u8;
                    let hover_midi = (state.max_midi.saturating_sub(key_idx))
                        .clamp(state.min_midi, state.max_midi);

                    let y_top = grid_start_y
                        + (state.max_midi - hover_midi) as f32 * state.row_height
                        + 2.0;
                    let y_bottom = y_top + state.row_height - 4.0;
                    let hover_rect = Rect::from_min_max(
                        Pos2::new(rect.min.x + keyboard_width, y_top),
                        Pos2::new(rect.max.x, y_bottom),
                    );

                    painter.rect_filled(
                        hover_rect,
                        Rounding::ZERO,
                        Color32::from_rgba_premultiplied(0, 255, 157, 18),
                    );
                    painter.rect_stroke(
                        hover_rect,
                        Rounding::ZERO,
                        Stroke::new(1.0_f32, Color32::from_rgb(0, 255, 157)),
                    );

                    if !ui.input(|i| i.pointer.primary_down()) {
                        let mut dropped_on_note = false;
                        for (idx, note) in notes.iter_mut().enumerate() {
                            let note_midi = note.midi_key();
                            let k_i = (state.max_midi - note_midi) as f32;
                            let n_yt = grid_start_y + k_i * state.row_height + 2.0;
                            let n_yb = n_yt + state.row_height - 4.0;
                            let n_xs = rect.min.x
                                + keyboard_width
                                + (note.position_ms * state.px_per_ms as f64) as f32;
                            let n_xe = n_xs + (note.duration_ms * state.px_per_ms as f64) as f32;
                            let n_rect =
                                Rect::from_min_max(Pos2::new(n_xs, n_yt), Pos2::new(n_xe, n_yb));

                            if n_rect.contains(mpos) {
                                note.lyric = dragged_alias.clone();
                                state.selected_note_index = Some(idx);
                                dropped_on_note = true;
                                break;
                            }
                        }

                        if !dropped_on_note {
                            let click_x = mpos.x - (rect.min.x + keyboard_width);
                            let raw_start_ms = (click_x / state.px_per_ms) as f64;
                            let drop_start_ms = apply_snap(raw_start_ms, snap_option, bpm).max(0.0);
                            let new_note = UNote::new(
                                dragged_alias,
                                midi_to_note_name(hover_midi),
                                drop_start_ms,
                                400.0,
                            );
                            notes.push(new_note);
                            state.selected_note_index = Some(notes.len() - 1);
                        }

                        let freq = midi_to_freq(hover_midi as f64);
                        on_preview_freq(freq);
                        on_note_changed();
                        phoneme_state.dragged_phoneme = None;
                    }
                }
            }

            if !ui.input(|i| i.pointer.primary_down()) {
                phoneme_state.dragged_phoneme = None;
            }
        }

        let sticky_key_x = rect.min.x.max(visible_clip.min.x);
        let keys_y_min = visible_clip.min.y.max(grid_start_y);
        let keys_y_max = visible_clip.max.y.min(grid_end_y);

        grid::draw_piano_keys(
            &painter,
            ui,
            state,
            rect,
            visible_clip,
            keyboard_width,
            grid_start_y,
            grid_end_y,
            first_visible_key,
            last_visible_key,
            on_preview_freq,
        );

        let playhead_x =
            rect.min.x + keyboard_width + (state.playhead_ms * state.px_per_ms as f64) as f32;
        if playhead_x >= sticky_key_x + keyboard_width && playhead_x <= visible_clip.max.x {
            let handle_points = vec![
                Pos2::new(playhead_x - 3.5, keys_y_min + 1.0),
                Pos2::new(playhead_x + 3.5, keys_y_min + 1.0),
                Pos2::new(playhead_x, keys_y_min + 9.0),
            ];
            painter.add(egui::Shape::convex_polygon(
                handle_points,
                Color32::from_rgb(255, 65, 85),
                Stroke::new(0.8_f32, Color32::WHITE),
            ));

            painter.line_segment(
                [
                    Pos2::new(playhead_x, keys_y_min),
                    Pos2::new(playhead_x, keys_y_max),
                ],
                Stroke::new(3.0_f32, Color32::from_rgba_unmultiplied(255, 65, 85, 45)),
            );

            painter.line_segment(
                [
                    Pos2::new(playhead_x, keys_y_min),
                    Pos2::new(playhead_x, keys_y_max),
                ],
                Stroke::new(1.0_f32, Color32::from_rgb(255, 80, 100)),
            );
        }
    });
    if !state.is_playing && !is_mod_zoom && !state.is_scrubbing_ruler {
        state.horizontal_scroll_offset = scroll_output.state.offset.x;
        state.vertical_scroll_offset = scroll_output.state.offset.y;
    }

    if let (Some(menu_idx), Some(menu_pos)) = (state.context_menu_note_idx, state.context_menu_pos)
    {
        let mut close_menu = false;
        let mut trigger_note_changed = false;

        let sel_count = state.selected_note_indices.len().max(1);

        egui::Area::new(egui::Id::new("piano_roll_note_context_menu"))
            .fixed_pos(menu_pos)
            .order(egui::Order::Tooltip)
            .show(ui.ctx(), |ui| {
                egui::Frame::menu(ui.style())
                    .fill(MelodyneTheme::BG_PANEL)
                    .stroke(Stroke::new(1.0_f32, MelodyneTheme::ACCENT_GOLD))
                    .rounding(Rounding::same(6.0))
                    .shadow(egui::epaint::Shadow {
                        offset: Vec2::new(0.0, 4.0),
                        blur: 12.0,
                        spread: 0.0,
                        color: Color32::from_black_alpha(180),
                    })
                    .inner_margin(egui::Margin::same(6.0))
                    .show(ui, |ui| {
                        ui.set_min_width(260.0);

                        ui.label(
                            egui::RichText::new(format!("Menu da Nota ({} selecionada{})", sel_count, if sel_count > 1 { "s" } else { "" }))
                                .size(11.5)
                                .color(MelodyneTheme::TEXT_GOLD_LABEL)
                                .strong(),
                        );
                        ui.separator();

                        ui.label(egui::RichText::new("✨ AutoPitch & Afinação").size(10.5).color(Color32::from_rgb(255, 215, 0)));
                        if ui
                            .button(
                                egui::RichText::new("🌸 AutoPitch Suave / Pop")
                                    .color(Color32::from_rgb(0, 240, 255))
                                    .strong(),
                            )
                            .on_hover_text("Transições limpas e rápidas com vibrato sutil no final da nota")
                            .clicked()
                        {
                            apply_autopitch_to_selection(notes, &state.selected_note_indices, AutoPitchStyle::SmoothPop);
                            trigger_note_changed = true;
                            close_menu = true;
                        }
                        if ui
                            .button(
                                egui::RichText::new("✨ AutoPitch Natural (Humano)")
                                    .color(MelodyneTheme::NOTE_SELECTED_GOLD),
                            )
                            .on_hover_text("Aplica curvas suaves de portamento, overshoot inicial e vibrato expressivo realista")
                            .clicked()
                        {
                            apply_autopitch_to_selection(notes, &state.selected_note_indices, AutoPitchStyle::Natural);
                            trigger_note_changed = true;
                            close_menu = true;
                        }
                        if ui
                            .button(
                                egui::RichText::new("🔥 AutoPitch Dramático / Intenso")
                                    .color(Color32::from_rgb(255, 120, 160)),
                            )
                            .on_hover_text("Curvas de afinação com modulação marcante, vibrato profundo e portamento longo")
                            .clicked()
                        {
                            apply_autopitch_to_selection(notes, &state.selected_note_indices, AutoPitchStyle::Expressive);
                            trigger_note_changed = true;
                            close_menu = true;
                        }

                        ui.separator();

                        ui.label(egui::RichText::new("〰 Presets de Vibrato Vocal").size(10.5).color(Color32::from_rgb(0, 240, 255)));
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("🌸 Pop Suave").on_hover_text("65% comprimento, 48 cents, 5.7 Hz").clicked() {
                                for &idx in &state.selected_note_indices {
                                    if idx < notes.len() {
                                        notes[idx].vibrato.length_pct = 65.0;
                                        notes[idx].vibrato.depth_cents = 48.0;
                                        notes[idx].vibrato.period_ms = 175.0;
                                        notes[idx].vibrato.fade_in_pct = 25.0;
                                        notes[idx].vibrato.fade_out_pct = 15.0;
                                    }
                                }
                                trigger_note_changed = true;
                                close_menu = true;
                            }
                            if ui.button("🎭 Dramático").on_hover_text("75% comprimento, 75 cents, 6.2 Hz").clicked() {
                                for &idx in &state.selected_note_indices {
                                    if idx < notes.len() {
                                        notes[idx].vibrato.length_pct = 75.0;
                                        notes[idx].vibrato.depth_cents = 75.0;
                                        notes[idx].vibrato.period_ms = 160.0;
                                        notes[idx].vibrato.fade_in_pct = 20.0;
                                        notes[idx].vibrato.fade_out_pct = 10.0;
                                    }
                                }
                                trigger_note_changed = true;
                                close_menu = true;
                            }
                            if ui.button("🍃 Balada").on_hover_text("80% comprimento, 50 cents, 4.5 Hz").clicked() {
                                for &idx in &state.selected_note_indices {
                                    if idx < notes.len() {
                                        notes[idx].vibrato.length_pct = 80.0;
                                        notes[idx].vibrato.depth_cents = 50.0;
                                        notes[idx].vibrato.period_ms = 220.0;
                                        notes[idx].vibrato.fade_in_pct = 35.0;
                                        notes[idx].vibrato.fade_out_pct = 15.0;
                                    }
                                }
                                trigger_note_changed = true;
                                close_menu = true;
                            }
                            if ui.button("⚡ Rápido").on_hover_text("60% comprimento, 60 cents, 7.0 Hz").clicked() {
                                for &idx in &state.selected_note_indices {
                                    if idx < notes.len() {
                                        notes[idx].vibrato.length_pct = 60.0;
                                        notes[idx].vibrato.depth_cents = 60.0;
                                        notes[idx].vibrato.period_ms = 140.0;
                                        notes[idx].vibrato.fade_in_pct = 20.0;
                                        notes[idx].vibrato.fade_out_pct = 10.0;
                                    }
                                }
                                trigger_note_changed = true;
                                close_menu = true;
                            }
                            if ui.button("🚫 Desligar").on_hover_text("Zera o vibrato da nota").clicked() {
                                for &idx in &state.selected_note_indices {
                                    if idx < notes.len() {
                                        notes[idx].vibrato.length_pct = 0.0;
                                    }
                                }
                                trigger_note_changed = true;
                                close_menu = true;
                            }
                        });

                        ui.separator();

                        if ui
                            .button(
                                egui::RichText::new("🧹 Limpar Curvas de Pitch e Vibrato")
                                    .color(Color32::from_rgb(200, 190, 210)),
                            )
                            .clicked()
                        {
                            for &idx in &state.selected_note_indices {
                                if idx < notes.len() {
                                    notes[idx].pitch_bend.points.clear();
                                    notes[idx].vibrato.length_pct = 0.0;
                                }
                            }
                            trigger_note_changed = true;
                            close_menu = true;
                        }
                        if ui
                            .button(
                                egui::RichText::new("🧹 Resetar Envelopes de Volume")
                                    .color(Color32::from_rgb(200, 190, 210)),
                            )
                            .clicked()
                        {
                            for &idx in &state.selected_note_indices {
                                if idx < notes.len() {
                                    notes[idx].envelope = crate::dsp::envelope::UtauEnvelope::default();
                                }
                            }
                            trigger_note_changed = true;
                            close_menu = true;
                        }
                        if ui
                            .button(
                                egui::RichText::new("⏱ Resetar Tempos dos Fonemas")
                                    .color(Color32::from_rgb(255, 205, 70))
                                    .strong(),
                            )
                            .on_hover_text("Restaura os tempos e divisões originais calculados pelo fonemizador para esta nota ou todas as notas selecionadas")
                            .clicked()
                        {
                            on_before_change();
                            let target_indices: Vec<usize> = if !state.selected_note_indices.is_empty() {
                                state.selected_note_indices.iter().copied().collect()
                            } else {
                                vec![menu_idx]
                            };
                            for idx in target_indices {
                                if idx < notes.len() {
                                    notes[idx].phoneme_durations_ms.clear();
                                    notes[idx].expressions.consonant_timing_offset_ms = 0.0;
                                    notes[idx].expressions.preutter_offset_ms = 0.0;
                                    notes[idx].expressions.overlap_offset_ms = 0.0;
                                }
                            }
                            state.phoneme_cache_hash = 0;
                            trigger_note_changed = true;
                            close_menu = true;
                        }

                        ui.separator();

                        if ui
                            .button(
                                egui::RichText::new("⚙ Propriedades Detalhadas da Nota...")
                                    .color(Color32::from_rgb(230, 220, 240)),
                            )
                            .clicked()
                        {
                            state.properties_window_for_note = Some(menu_idx);
                            close_menu = true;
                        }
                    });
            });

        if ui.input(|i| i.pointer.primary_clicked() || i.pointer.secondary_clicked()) {
            if let Some(mpos) = ui.input(|i| i.pointer.interact_pos()) {
                let menu_rect = Rect::from_min_size(menu_pos, Vec2::new(260.0, 300.0));
                if !menu_rect.contains(mpos) {
                    close_menu = true;
                }
            }
        }

        if close_menu {
            state.context_menu_note_idx = None;
            state.context_menu_pos = None;
        }

        if trigger_note_changed {
            state.continuous_edit_dirty = true;
            on_note_changed();
        }
    }

    if let Some(prop_idx) = state.properties_window_for_note {
        if prop_idx < notes.len() {
            let mut close_window = false;
            let note = &mut notes[prop_idx];
            egui::Window::new("⚙ Propriedades da Nota & Vibrato")
                .collapsible(false)
                .resizable(false)
                .default_width(320.0)
                .show(ui.ctx(), |ui| {
                    ui.heading(
                        egui::RichText::new(format!("Nota: {} ({})", note.lyric, note.pitch))
                            .size(14.0)
                            .color(MelodyneTheme::TEXT_GOLD_LABEL),
                    );
                    ui.separator();

                    egui::Grid::new("prop_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Letra (Lyric):");
                            ui.text_edit_singleline(&mut note.lyric);
                            ui.end_row();

                            ui.label("Duração (ms):");
                            ui.add(
                                egui::DragValue::new(&mut note.duration_ms)
                                    .speed(1.0)
                                    .range(20.0..=10000.0)
                                    .suffix(" ms"),
                            );
                            ui.end_row();

                            ui.label("Vel. da Consoante:");
                            ui.add(
                                egui::DragValue::new(&mut note.expressions.consonant_velocity)
                                    .speed(1.0)
                                    .range(0.0..=200.0)
                                    .suffix(" %"),
                            );
                            ui.end_row();

                            ui.label("Modulação (MOD):");
                            ui.add(
                                egui::DragValue::new(&mut note.expressions.modulation)
                                    .speed(1.0)
                                    .range(0.0..=200.0)
                                    .suffix(" %"),
                            );
                            ui.end_row();
                        });

                    ui.add_space(8.0);
                    ui.label(
                        egui::RichText::new("〰 Parâmetros de Vibrato")
                            .strong()
                            .color(Color32::from_rgb(0, 240, 255)),
                    );

                    egui::Grid::new("vibrato_grid")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("Comprimento (%):");
                            ui.add(
                                egui::Slider::new(&mut note.vibrato.length_pct, 0.0..=100.0)
                                    .suffix(" %"),
                            );
                            ui.end_row();

                            ui.label("Profundidade (cents):");
                            ui.add(
                                egui::Slider::new(&mut note.vibrato.depth_cents, 0.0..=200.0)
                                    .suffix(" c"),
                            );
                            ui.end_row();

                            ui.label("Período (ms):");
                            ui.add(
                                egui::Slider::new(&mut note.vibrato.period_ms, 50.0..=450.0)
                                    .suffix(" ms"),
                            );
                            ui.end_row();

                            ui.label("Fade In (%):");
                            ui.add(
                                egui::Slider::new(&mut note.vibrato.fade_in_pct, 0.0..=100.0)
                                    .suffix(" %"),
                            );
                            ui.end_row();

                            ui.label("Fade Out (%):");
                            ui.add(
                                egui::Slider::new(&mut note.vibrato.fade_out_pct, 0.0..=100.0)
                                    .suffix(" %"),
                            );
                            ui.end_row();
                        });

                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        if ui.button("OK").clicked() {
                            close_window = true;
                        }
                    });
                });

            if close_window {
                state.properties_window_for_note = None;
            }
        } else {
            state.properties_window_for_note = None;
        }
    }
}
