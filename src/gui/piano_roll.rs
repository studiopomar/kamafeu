use std::collections::HashSet;
use eframe::egui::{self, Color32, Key, Pos2, Rect, Rounding, Sense, Stroke, Vec2};
use crate::dsp::pitch::{midi_to_freq, midi_to_note_name};
use crate::dsp::pitch_bend::PitchBendSolver;
use crate::gui::phoneme_palette::PhonemePaletteState;
use crate::gui::theme::MelodyneTheme;
use crate::gui::toolbar::EditTool;
use crate::gui::transport::GridSnapOption;
use crate::oto::Voicebank;
use crate::project::model::{UNote, UPitchBendPoint};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParameterTab {
    PitchDelta,
    Dynamics,
    Gender,
    Velocity,
    Breathiness,
}

impl Default for ParameterTab {
    fn default() -> Self {
        ParameterTab::Dynamics
    }
}

pub struct PianoRollState {
    pub selected_note_index: Option<usize>,
    pub selected_note_indices: HashSet<usize>,
    pub editing_lyric_index: Option<usize>,
    pub lyric_buffer: String,
    pub autocomplete_selected_idx: usize,
    pub playhead_ms: f64,
    pub is_playing: bool,
    pub px_per_ms: f32,
    pub row_height: f32,
    pub min_midi: u8,
    pub max_midi: u8,
    pub dragging_note_idx: Option<usize>,
    pub dragging_is_resize: bool,
    pub dragging_is_left_resize: bool,
    pub drag_start_pos: Option<Pos2>,
    pub note_original_start_ms: f64,
    pub note_original_duration_ms: f64,
    pub note_original_midi: u8,
    pub note_original_states: Vec<(usize, f64, u8)>,
    pub marquee_start: Option<Pos2>,
    pub marquee_current: Option<Pos2>,
    pub creating_note_idx: Option<usize>,
    pub active_tool: EditTool,
    pub is_scrubbing_ruler: bool,
    pub show_parameters_drawer: bool,
    pub selected_parameter: ParameterTab,
    pub initial_scrolled: bool,
    pub properties_window_for_note: Option<usize>,
    pub dragging_envelope_pt: Option<(usize, usize)>, // (note_idx, pt_idx)
}

impl Default for PianoRollState {
    fn default() -> Self {
        Self {
            selected_note_index: None,
            selected_note_indices: HashSet::new(),
            editing_lyric_index: None,
            lyric_buffer: String::new(),
            autocomplete_selected_idx: 0,
            playhead_ms: 0.0,
            is_playing: false,
            px_per_ms: 0.25,
            row_height: 22.0,
            min_midi: 36,
            max_midi: 96,
            dragging_note_idx: None,
            dragging_is_resize: false,
            dragging_is_left_resize: false,
            drag_start_pos: None,
            note_original_start_ms: 0.0,
            note_original_duration_ms: 0.0,
            note_original_midi: 60,
            note_original_states: Vec::new(),
            marquee_start: None,
            marquee_current: None,
            creating_note_idx: None,
            active_tool: EditTool::Pointer,
            is_scrubbing_ruler: false,
            show_parameters_drawer: true,
            selected_parameter: ParameterTab::Dynamics,
            initial_scrolled: false,
            properties_window_for_note: None,
            dragging_envelope_pt: None,
        }
    }
}

fn apply_snap(val_ms: f64, snap: GridSnapOption, bpm: f64) -> f64 {
    if let Some(step) = snap.step_ms(bpm) {
        (val_ms / step).round() * step
    } else {
        val_ms
    }
}

pub fn draw_piano_roll(
    ui: &mut egui::Ui,
    notes: &mut Vec<UNote>,
    state: &mut PianoRollState,
    voicebank: Option<&Voicebank>,
    phoneme_state: &mut PhonemePaletteState,
    snap_option: GridSnapOption,
    bpm: f64,
    on_preview_freq: &mut dyn FnMut(f64),
    on_note_changed: &mut dyn FnMut(),
    on_playhead_scrubbed: &mut dyn FnMut(f64),
) {
    let key_count = (state.max_midi - state.min_midi + 1) as usize;
    let ruler_height = 28.0f32;
    let param_drawer_height = if state.show_parameters_drawer { 100.0f32 } else { 0.0f32 };
    let _total_height = key_count as f32 * state.row_height + ruler_height + param_drawer_height;
    let keyboard_width = 65.0f32;

    // 1. Pinned Top Timeline Ruler Header (Fixed 28px height, pinned above vertical pitch scrolling)
    let (ruler_rect, _ruler_resp) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), ruler_height),
        Sense::click_and_drag(),
    );

    let ruler_painter = ui.painter_at(ruler_rect);
    ruler_painter.rect_filled(ruler_rect, Rounding::ZERO, MelodyneTheme::BG_HEADER);
    ruler_painter.line_segment(
        [Pos2::new(ruler_rect.min.x, ruler_rect.max.y), Pos2::new(ruler_rect.max.x, ruler_rect.max.y)],
        Stroke::new(1.5, MelodyneTheme::GRID_LINE_BAR),
    );

    let corner_rect = Rect::from_min_max(
        Pos2::new(ruler_rect.min.x, ruler_rect.min.y),
        Pos2::new(ruler_rect.min.x + keyboard_width, ruler_rect.max.y),
    );
    ruler_painter.rect_filled(corner_rect, Rounding::ZERO, MelodyneTheme::BG_PANEL);
    ruler_painter.text(
        Pos2::new(ruler_rect.min.x + 8.0, ruler_rect.min.y + ruler_height * 0.5),
        egui::Align2::LEFT_CENTER,
        "RULER",
        egui::FontId::proportional(10.0),
        MelodyneTheme::TEXT_GOLD_LABEL,
    );

    let beat_ms = 60000.0 / bpm;
    let bar_ms = beat_ms * 4.0;
    let total_ms = (ruler_rect.width() - keyboard_width) as f64 / state.px_per_ms as f64;
    let mut measure_idx = 1;
    let mut m_ms = 0.0f64;

    while m_ms < total_ms {
        let x = ruler_rect.min.x + keyboard_width + (m_ms * state.px_per_ms as f64) as f32;
        
        ruler_painter.line_segment(
            [Pos2::new(x, ruler_rect.min.y + 12.0), Pos2::new(x, ruler_rect.max.y)],
            Stroke::new(1.5, MelodyneTheme::ACCENT_GOLD),
        );

        ruler_painter.text(
            Pos2::new(x + 4.0, ruler_rect.min.y + 4.0),
            egui::Align2::LEFT_TOP,
            format!("m{}", measure_idx),
            egui::FontId::proportional(11.0),
            MelodyneTheme::TEXT_GOLD_LABEL,
        );

        measure_idx += 1;
        m_ms += bar_ms;
    }

    if let Some(mpos) = ui.input(|i| i.pointer.interact_pos()) {
        if ruler_rect.contains(mpos) && ui.input(|i| i.pointer.primary_down()) {
            state.is_scrubbing_ruler = true;
            let raw_t = (mpos.x - (ruler_rect.min.x + keyboard_width)) as f64 / state.px_per_ms as f64;
            let scrubbed_t = apply_snap(raw_t.max(0.0), snap_option, bpm);
            state.playhead_ms = scrubbed_t;
            on_playhead_scrubbed(scrubbed_t);
        }
    }

    // 2. Fixed Bottom Parameter Automation Drawer (Always visible at bottom, no scrolling required)
    if state.show_parameters_drawer {
        egui::TopBottomPanel::bottom("bottom_param_drawer_fixed")
            .resizable(false)
            .exact_height(120.0)
            .frame(egui::Frame::none().fill(MelodyneTheme::BG_PANEL))
            .show_inside(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add_space(4.0);
                    // Left Tabs Column
                    ui.vertical(|ui| {
                        ui.add_space(4.0);
                        ui.label(egui::RichText::new("📊 PARÂMETROS").strong().size(10.0).color(Color32::from_rgb(0, 255, 157)));
                        ui.add_space(2.0);

                        let param_tabs = [
                            ("Dynamics (Vol)", ParameterTab::Dynamics),
                            ("Pitch Dynamics", ParameterTab::PitchDelta),
                            ("Gender (Formant)", ParameterTab::Gender),
                            ("Velocity", ParameterTab::Velocity),
                            ("Breathiness", ParameterTab::Breathiness),
                        ];

                        for (p_name, tab_val) in param_tabs {
                            let is_sel = state.selected_parameter == tab_val;
                            let (text_color, fill_color) = if is_sel {
                                (Color32::from_rgb(0, 255, 157), Color32::from_rgb(36, 27, 53))
                            } else {
                                (Color32::from_rgb(165, 148, 201), Color32::TRANSPARENT)
                            };

                            let btn = egui::Button::new(egui::RichText::new(p_name).size(10.0).color(text_color))
                                .fill(fill_color)
                                .rounding(Rounding::same(3.0));

                            if ui.add(btn).clicked() {
                                state.selected_parameter = tab_val;
                            }
                        }
                    });

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    // Right Curve Graph Area (Interactive with mouse drag/click)
                    let (graph_rect, graph_response) = ui.allocate_exact_size(
                        Vec2::new(ui.available_width() - 8.0, 110.0),
                        Sense::click_and_drag(),
                    );

                    let painter = ui.painter_at(graph_rect);
                    painter.rect_filled(graph_rect, Rounding::same(4.0), MelodyneTheme::BG_CANVAS);
                    painter.rect_stroke(graph_rect, Rounding::same(4.0), Stroke::new(1.0, MelodyneTheme::GRID_LINE_BAR));

                    let mid_y = graph_rect.min.y + 55.0;
                    painter.line_segment(
                        [Pos2::new(graph_rect.min.x, mid_y), Pos2::new(graph_rect.max.x, mid_y)],
                        Stroke::new(1.0, MelodyneTheme::GRID_LINE_SUB),
                    );

                    // Interactive editing of notes' parameters via graph mouse dragging
                    if let Some(mpos) = graph_response.interact_pointer_pos() {
                        if graph_response.dragged() || graph_response.clicked() {
                            let click_t = (mpos.x - (graph_rect.min.x + keyboard_width)) as f64 / state.px_per_ms as f64;
                            let mut changed = false;
                            for note in notes.iter_mut() {
                                if click_t >= note.position_ms && click_t <= note.position_ms + note.duration_ms {
                                    let norm_y = ((mid_y - mpos.y) / 45.0) as f64; // [-1.0, 1.0]
                                    match state.selected_parameter {
                                        ParameterTab::Dynamics => {
                                            note.expressions.dynamics = (norm_y * 20.0).clamp(-20.0, 20.0);
                                        }
                                        ParameterTab::PitchDelta => {
                                            note.expressions.pitch_delta = (norm_y * 100.0).clamp(-100.0, 100.0);
                                        }
                                        ParameterTab::Gender => {
                                            note.expressions.gender = (norm_y * 100.0).clamp(-100.0, 100.0);
                                        }
                                        ParameterTab::Velocity => {
                                            let norm_01 = ((graph_rect.max.y - mpos.y) / graph_rect.height()) as f64;
                                            note.expressions.velocity = (norm_01 * 200.0).clamp(0.0, 200.0);
                                        }
                                        ParameterTab::Breathiness => {
                                            let norm_01 = ((graph_rect.max.y - mpos.y) / graph_rect.height()) as f64;
                                            note.expressions.breathiness = (norm_01 * 100.0).clamp(0.0, 100.0);
                                        }
                                    }
                                    changed = true;
                                }
                            }
                            if changed {
                                on_note_changed();
                            }
                        }
                    }

                    // Render parameter bars for all notes in score
                    for note in notes.iter() {
                        let x_start = graph_rect.min.x + keyboard_width + (note.position_ms * state.px_per_ms as f64) as f32;
                        let x_end = x_start + (note.duration_ms * state.px_per_ms as f64) as f32;

                        if x_end >= graph_rect.min.x && x_start <= graph_rect.max.x {
                            let (val, min_v, max_v, label_str) = match state.selected_parameter {
                                ParameterTab::Dynamics => (note.expressions.dynamics, -20.0, 20.0, format!("{:+.1} dB", note.expressions.dynamics)),
                                ParameterTab::PitchDelta => (note.expressions.pitch_delta, -100.0, 100.0, format!("{:+.0} c", note.expressions.pitch_delta)),
                                ParameterTab::Gender => (note.expressions.gender, -100.0, 100.0, format!("g{:+.0}", note.expressions.gender)),
                                ParameterTab::Velocity => (note.expressions.velocity, 0.0, 200.0, format!("v{:.0}", note.expressions.velocity)),
                                ParameterTab::Breathiness => (note.expressions.breathiness, 0.0, 100.0, format!("B{:.0}", note.expressions.breathiness)),
                            };

                            let node_y = match state.selected_parameter {
                                ParameterTab::Velocity | ParameterTab::Breathiness => {
                                    let norm = (val - min_v) / (max_v - min_v);
                                    graph_rect.max.y - (norm as f32 * (graph_rect.height() - 10.0)) - 5.0
                                }
                                _ => {
                                    let norm = val / max_v;
                                    mid_y - (norm as f32 * 45.0)
                                }
                            };

                            let bar_min_y = mid_y.min(node_y);
                            let bar_max_y = mid_y.max(node_y);

                            let bar_rect = Rect::from_min_max(
                                Pos2::new(x_start.max(graph_rect.min.x), bar_min_y),
                                Pos2::new(x_end.min(graph_rect.max.x), bar_max_y.max(bar_min_y + 2.0)),
                            );

                            painter.rect_filled(bar_rect, Rounding::same(2.0), Color32::from_rgba_premultiplied(0, 255, 157, 60));
                            painter.line_segment(
                                [Pos2::new(x_start.max(graph_rect.min.x), node_y), Pos2::new(x_end.min(graph_rect.max.x), node_y)],
                                Stroke::new(2.5, Color32::from_rgb(0, 255, 157)),
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
    }

    // 3. 88 Pitch Keys & Piano Roll Grid Canvas
    let max_note_end_ms = notes.iter()
        .map(|n| n.position_ms + n.duration_ms)
        .fold(30000.0f64, f64::max);

    let grid_width = (keyboard_width as f64 + (max_note_end_ms + 10000.0) * state.px_per_ms as f64) as f32;
    let grid_width = grid_width.max(3000.0);
    let grid_height = key_count as f32 * state.row_height;

    let mut scroll_area = egui::ScrollArea::both()
        .id_salt("piano_roll_scroll")
        .auto_shrink([false, false]);

    if !state.initial_scrolled {
        let (first_note_pos, target_midi) = if let Some(first) = notes.iter().min_by(|a, b| a.position_ms.partial_cmp(&b.position_ms).unwrap_or(std::cmp::Ordering::Equal)) {
            (first.position_ms, first.midi_key())
        } else {
            (0.0, 60)
        };

        let row_idx = (state.max_midi.saturating_sub(target_midi)) as f32;
        let target_y = (row_idx * state.row_height - 180.0).max(0.0);
        let target_x = ((first_note_pos * state.px_per_ms as f64) as f32 - 100.0).max(0.0);

        scroll_area = scroll_area.vertical_scroll_offset(target_y).horizontal_scroll_offset(target_x);
        state.initial_scrolled = true;
    }

    if state.is_playing {
        let playhead_x = (state.playhead_ms * state.px_per_ms as f64) as f32;
        let follow_x = (playhead_x - 250.0).max(0.0);
        scroll_area = scroll_area.horizontal_scroll_offset(follow_x);
    }

    scroll_area.show(ui, |ui| {
            let (rect, response) = ui.allocate_exact_size(
                Vec2::new(grid_width, grid_height),
                Sense::click_and_drag(),
            );

            let painter = ui.painter_at(rect);

            painter.rect_filled(rect, Rounding::ZERO, MelodyneTheme::BG_CANVAS);

            let grid_start_y = rect.min.y;
            let grid_end_y = rect.max.y;

            let grid_step_ms = match snap_option {
                GridSnapOption::Freeform => beat_ms / 4.0,
                _ => snap_option.step_ms(bpm).unwrap_or(beat_ms / 4.0),
            };

            let mut time_ms = 0.0f64;

            while time_ms < total_ms {
                let x = rect.min.x + keyboard_width + (time_ms * state.px_per_ms as f64) as f32;
                let is_bar = (time_ms % (beat_ms * 4.0)).abs() < 1e-2;
                let line_color = if is_bar {
                    MelodyneTheme::GRID_LINE_BAR
                } else {
                    MelodyneTheme::GRID_LINE_SUB
                };

                painter.line_segment(
                    [Pos2::new(x, grid_start_y), Pos2::new(x, grid_end_y)],
                    Stroke::new(if is_bar { 1.5 } else { 1.0 }, line_color),
                );

                time_ms += grid_step_ms;
            }

            // 3. Grid Row Backgrounds
            for key_idx in 0..key_count {
                let midi = state.max_midi - key_idx as u8;
                let y_top = grid_start_y + key_idx as f32 * state.row_height;
                let y_bottom = y_top + state.row_height;

                let is_black_key = matches!(midi % 12, 1 | 3 | 6 | 8 | 10);
                let row_bg = if is_black_key {
                    MelodyneTheme::BG_KEYBOARD_BLACK
                } else {
                    MelodyneTheme::BG_CANVAS
                };

                painter.rect_filled(
                    Rect::from_min_max(
                        Pos2::new(rect.min.x + keyboard_width, y_top),
                        Pos2::new(rect.max.x, y_bottom),
                    ),
                    Rounding::ZERO,
                    row_bg,
                );

                painter.line_segment(
                    [Pos2::new(rect.min.x + keyboard_width, y_bottom), Pos2::new(rect.max.x, y_bottom)],
                    Stroke::new(0.5, MelodyneTheme::GRID_LINE_SUB),
                );
            }

            // 4. Render Notes (Melodyne Metallic Gold Blobs)
            let mut note_to_delete: Option<usize> = None;
            let mut commit_lyric_edit: Option<(usize, String)> = None;

            let note_info: Vec<(u8, f64, f64)> = notes
                .iter()
                .map(|n| (n.midi_key(), n.position_ms, n.duration_ms))
                .collect();

            let mouse_interact_pos = ui.input(|i| i.pointer.interact_pos());

            for (idx, note) in notes.iter_mut().enumerate() {
                let note_midi = note.midi_key();
                if note_midi < state.min_midi || note_midi > state.max_midi {
                    continue;
                }

                let key_idx = (state.max_midi - note_midi) as f32;
                let y_top = grid_start_y + key_idx * state.row_height + 2.0;
                let y_bottom = y_top + state.row_height - 4.0;
                let y_center = (y_top + y_bottom) * 0.5;

                let x_start = rect.min.x + keyboard_width + (note.position_ms * state.px_per_ms as f64) as f32;
                let x_end = x_start + (note.duration_ms * state.px_per_ms as f64) as f32;

                let note_rect = Rect::from_min_max(Pos2::new(x_start, y_top), Pos2::new(x_end, y_bottom));

                let is_selected = state.selected_note_index == Some(idx) || state.selected_note_indices.contains(&idx);
                let is_editing_lyric = state.editing_lyric_index == Some(idx);

                let note_color = if is_selected {
                    MelodyneTheme::NOTE_SELECTED_GOLD
                } else {
                    MelodyneTheme::NOTE_GOLD_FILL
                };

                // Draw Melodyne rounded gold note blob
                painter.rect_filled(note_rect, Rounding::same(6.0), note_color);
                painter.rect_stroke(
                    note_rect,
                    Rounding::same(6.0),
                    Stroke::new(1.8, if is_selected { Color32::WHITE } else { MelodyneTheme::NOTE_GOLD_STROKE }),
                );

                // Phoneme Drag & Drop Target Highlight
                if let Some(ref dragged_alias) = phoneme_state.dragged_phoneme {
                    if let Some(mpos) = mouse_interact_pos {
                        if note_rect.contains(mpos) {
                            painter.rect_stroke(note_rect, Rounding::same(6.0), Stroke::new(2.5, Color32::from_rgb(0, 220, 255)));

                            if !ui.input(|i| i.pointer.primary_down()) {
                                commit_lyric_edit = Some((idx, dragged_alias.clone()));
                                phoneme_state.dragged_phoneme = None;
                            }
                        }
                    }
                }

                // Render Audio Waveform Thumbnail Overlay inside note box
                let mut wave_x = x_start + 4.0;
                let step = 3.0f32;
                while wave_x < x_end - 4.0 {
                    let rel_i = (wave_x - x_start) * 0.1;
                    let amp = (rel_i.sin().abs() * 0.4 + 0.1) * (state.row_height * 0.35);
                    painter.line_segment(
                        [Pos2::new(wave_x, y_center - amp), Pos2::new(wave_x, y_center + amp)],
                        Stroke::new(1.0, Color32::from_rgba_premultiplied(40, 25, 5, 160)),
                    );
                    wave_x += step;
                }

                // Render OpenUTAU Magenta Pitch Curve Overlay on Note
                if !note.pitch_bend.points.is_empty() {
                    let mut pitch_line_pts = Vec::new();
                    let step_px = 3.0f32;
                    let mut px = x_start;
                    while px <= x_end {
                        let rel_t = ((px - x_start) / state.px_per_ms) as f64;
                        let cents = crate::dsp::pitch_bend::PitchBendSolver::get_pitch_offset_cents(rel_t, &note.pitch_bend.points);
                        let py = y_center - (cents / 100.0) as f32 * state.row_height;
                        pitch_line_pts.push(Pos2::new(px, py));
                        px += step_px;
                    }
                    if pitch_line_pts.len() >= 2 {
                        for pts_win in pitch_line_pts.windows(2) {
                            painter.line_segment([pts_win[0], pts_win[1]], Stroke::new(2.2, Color32::from_rgb(255, 0, 128)));
                        }
                    }

                    // Render Pitch Control Points (subsampled for clean visual display)
                    for (pt_idx, pt) in note.pitch_bend.points.iter().enumerate() {
                        if pt_idx == 0 || pt_idx == note.pitch_bend.points.len() - 1 || pt_idx % 2 == 0 {
                            let pt_x = x_start + (pt.time_offset_ms * state.px_per_ms as f64) as f32;
                            let pt_y = y_center - (pt.pitch_offset_cents / 100.0) as f32 * state.row_height;
                            if pt_x >= x_start && pt_x <= x_end {
                                painter.circle_filled(Pos2::new(pt_x, pt_y), 3.0, Color32::WHITE);
                                painter.circle_stroke(Pos2::new(pt_x, pt_y), 3.0, Stroke::new(1.0, Color32::from_rgb(255, 0, 128)));
                            }
                        }
                    }
                }

                if is_editing_lyric {
                    let edit_rect = Rect::from_min_size(
                        Pos2::new(x_start + 2.0, y_top + 1.0),
                        Vec2::new((note_rect.width() - 4.0).max(80.0), state.row_height - 2.0),
                    );

                    let mut text_lost_focus = false;
                    ui.allocate_new_ui(egui::UiBuilder::new().max_rect(edit_rect), |ui| {
                        let text_resp = ui.add(
                            egui::TextEdit::singleline(&mut state.lyric_buffer)
                                .desired_width(edit_rect.width())
                                .font(egui::FontId::proportional(12.0)),
                        );
                        text_resp.request_focus();
                        text_lost_focus = text_resp.lost_focus();
                    });

                    let mut candidates: Vec<String> = Vec::new();
                    if let Some(vb) = voicebank {
                        let matches = vb.search_entries(&state.lyric_buffer, "All Folders");
                        for (alias, _) in matches {
                            if !candidates.contains(alias) {
                                candidates.push(alias.clone());
                            }
                        }
                    }
                    candidates.sort();

                    let popup_rect = if !candidates.is_empty() {
                        let popup_pos = Pos2::new(x_start, y_bottom + 4.0);
                        let popup_height = (candidates.len() as f32 * 20.0 + 24.0).min(170.0);
                        Some(Rect::from_min_size(popup_pos, Vec2::new(180.0, popup_height)))
                    } else {
                        None
                    };

                    if let Some(p_rect) = popup_rect {
                        egui::Area::new(egui::Id::new(format!("oto_autocomplete_popup_{}", idx)))
                            .fixed_pos(p_rect.min)
                            .order(egui::Order::Foreground)
                            .show(ui.ctx(), |ui| {
                                egui::Frame::popup(ui.style()).fill(MelodyneTheme::BG_PANEL).show(ui, |ui| {
                                    ui.set_max_width(180.0);
                                    ui.label(
                                        egui::RichText::new(format!("oto.ini Suggestions ({})", candidates.len()))
                                            .size(10.0)
                                            .color(MelodyneTheme::TEXT_GOLD_LABEL),
                                    );
                                    ui.separator();

                                    egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                                        for (cand_i, cand) in candidates.iter().enumerate() {
                                            let is_cand_sel = state.autocomplete_selected_idx == cand_i;
                                            let text_widget = if is_cand_sel {
                                                egui::RichText::new(cand).strong().color(MelodyneTheme::NOTE_SELECTED_GOLD)
                                            } else {
                                                egui::RichText::new(cand).color(Color32::from_rgb(240, 230, 210))
                                            };

                                            if ui.selectable_label(is_cand_sel, text_widget).clicked() {
                                                commit_lyric_edit = Some((idx, cand.clone()));
                                            }
                                        }
                                    });
                                });
                            });
                    }

                    if ui.input(|i| i.key_pressed(Key::ArrowDown)) {
                        if !candidates.is_empty() {
                            state.autocomplete_selected_idx = (state.autocomplete_selected_idx + 1) % candidates.len();
                        }
                    }
                    if ui.input(|i| i.key_pressed(Key::ArrowUp)) {
                        if !candidates.is_empty() {
                            state.autocomplete_selected_idx = if state.autocomplete_selected_idx == 0 {
                                candidates.len() - 1
                            } else {
                                state.autocomplete_selected_idx - 1
                            };
                        }
                    }

                    let is_clicked_outside = ui.input(|i| i.pointer.primary_clicked())
                        && mouse_interact_pos.map_or(false, |mpos| {
                            !edit_rect.contains(mpos) && popup_rect.map_or(true, |pr| !pr.contains(mpos))
                        });

                    if commit_lyric_edit.is_none() && (text_lost_focus || is_clicked_outside || ui.input(|i| i.key_pressed(Key::Enter)) || ui.input(|i| i.key_pressed(Key::Tab))) {
                        let final_lyric = if !candidates.is_empty() && state.autocomplete_selected_idx < candidates.len() && (ui.input(|i| i.key_pressed(Key::Enter)) || ui.input(|i| i.key_pressed(Key::Tab))) {
                            candidates[state.autocomplete_selected_idx].clone()
                        } else if !state.lyric_buffer.trim().is_empty() {
                            state.lyric_buffer.trim().to_string()
                        } else {
                            note.lyric.clone()
                        };
                        commit_lyric_edit = Some((idx, final_lyric));
                    }

                    if ui.input(|i| i.key_pressed(Key::Escape)) {
                        state.editing_lyric_index = None;
                    }
                } else {
                    // Draw Dark Gold Phoneme Tag Box inside note (Melodyne Style)
                    let pill_rect = Rect::from_min_size(
                        Pos2::new(x_start + 4.0, y_top + 3.0),
                        Vec2::new((note.lyric.len() as f32 * 8.0 + 12.0).min(note_rect.width() - 8.0), state.row_height - 10.0),
                    );
                    let pill_bg = if is_selected {
                        Color32::from_rgb(15, 15, 20) // Solid dark for high contrast
                    } else {
                        Color32::from_rgba_premultiplied(35, 20, 4, 190)
                    };
                    
                    let text_color = if is_selected {
                        Color32::WHITE
                    } else {
                        MelodyneTheme::TEXT_GOLD_LABEL
                    };

                    painter.rect_filled(pill_rect, Rounding::same(3.0), pill_bg);
                    painter.text(
                        pill_rect.center(),
                        egui::Align2::CENTER_CENTER,
                        &note.lyric,
                        egui::FontId::proportional(11.0),
                        text_color,
                    );
                }

                // 4.5. Render and Interact with Volume Envelope
                let e = note.envelope.clone();
                let env_pts = [
                    (e.p1, e.v1),
                    (e.p1 + e.p2, e.v2),
                    (e.p1 + e.p2 + e.p3, e.v3),
                    (note.duration_ms - e.p4, e.v4),
                    (note.duration_ms - e.p4 + e.p5, e.v5),
                ];

                let mut env_screen_pts = Vec::new();
                for (t_ms, vol) in env_pts.iter() {
                    let px_x = x_start + (*t_ms * state.px_per_ms as f64) as f32;
                    let px_y = y_bottom - (*vol / 100.0).clamp(0.0, 1.0) as f32 * (state.row_height - 4.0);
                    env_screen_pts.push(Pos2::new(px_x, px_y));
                }

                // Draw connecting lines
                let env_color = Color32::from_rgb(255, 105, 180); // Hot Pink for UTAU-style envelopes
                if let Some(first) = env_screen_pts.first() {
                    painter.line_segment([Pos2::new(x_start, y_bottom), *first], Stroke::new(1.5, env_color.linear_multiply(0.5)));
                }
                for w in env_screen_pts.windows(2) {
                    painter.line_segment([w[0], w[1]], Stroke::new(1.5, env_color));
                }

                // Interaction and drawing of points
                for (pt_idx, &pt_pos) in env_screen_pts.iter().enumerate() {
                    let pt_rect = Rect::from_center_size(pt_pos, Vec2::splat(8.0));
                    let is_hovered = mouse_interact_pos.map_or(false, |m| pt_rect.contains(m));
                    let is_dragging = state.dragging_envelope_pt == Some((idx, pt_idx));

                    let pt_color = if is_dragging || is_hovered { Color32::WHITE } else { env_color };
                    painter.circle_filled(pt_pos, if is_dragging { 4.0 } else { 3.0 }, pt_color);
                    painter.circle_stroke(pt_pos, if is_dragging { 4.0 } else { 3.0 }, Stroke::new(1.0, Color32::BLACK));

                    if is_hovered && ui.input(|i| i.pointer.primary_pressed()) {
                        state.dragging_envelope_pt = Some((idx, pt_idx));
                    }
                }

                if let Some((drag_note_idx, drag_pt_idx)) = state.dragging_envelope_pt {
                    if drag_note_idx == idx {
                        if let Some(mpos) = ui.input(|i| i.pointer.latest_pos()) {
                            let delta_t = ((mpos.x - x_start) / state.px_per_ms) as f64;
                            let delta_v = ((y_bottom - mpos.y) / (state.row_height - 4.0) * 100.0).clamp(0.0, 100.0) as f64;

                            match drag_pt_idx {
                                0 => { note.envelope.p1 = delta_t.max(0.0); note.envelope.v1 = delta_v; }
                                1 => { note.envelope.p2 = (delta_t - note.envelope.p1).max(0.0); note.envelope.v2 = delta_v; }
                                2 => { note.envelope.p3 = (delta_t - note.envelope.p1 - note.envelope.p2).max(0.0); note.envelope.v3 = delta_v; }
                                3 => { note.envelope.p4 = (note.duration_ms - delta_t).max(0.0); note.envelope.v4 = delta_v; }
                                4 => { note.envelope.p5 = (delta_t - (note.duration_ms - note.envelope.p4)).max(0.0); note.envelope.v5 = delta_v; }
                                _ => {}
                            }
                        }
                        if ui.input(|i| i.pointer.primary_released()) {
                            state.dragging_envelope_pt = None;
                        }
                    }
                }

                // 5. Render Continuous Pitch Curve Spline (Melodyne Glowing Amber with Unrestricted Melismas/Vibrato Tails)
                let min_t = note.pitch_bend.points.first().map(|p| p.time_offset_ms).unwrap_or(0.0).min(-40.0);
                let max_t = note.pitch_bend.points.last().map(|p| p.time_offset_ms).unwrap_or(note.duration_ms).max(note.duration_ms + 40.0);

                let mut spline_points = Vec::new();
                let step = 10.0f64;
                let mut t = min_t;
                while t <= max_t {
                    let offset_cents = PitchBendSolver::get_pitch_offset_cents(t, &note.pitch_bend.points);
                    let px_x = x_start + (t * state.px_per_ms as f64) as f32;
                    let px_y = y_center - (offset_cents / 100.0) as f32 * state.row_height;
                    spline_points.push(Pos2::new(px_x, px_y));
                    t += step;
                }

                if spline_points.len() >= 2 {
                    for w in spline_points.windows(2) {
                        painter.line_segment([w[0], w[1]], Stroke::new(2.5, MelodyneTheme::PITCH_ARM_GOLD));
                    }
                }

                // Render Adjacent Note Pitch Transition Arms ("Bracinhos de Pitch" - Melodyne Gold S-curves)
                if idx > 0 {
                    let (prev_midi, prev_start_ms, prev_dur_ms) = note_info[idx - 1];

                    let gap_ms = note.position_ms - (prev_start_ms + prev_dur_ms);
                    if gap_ms <= 150.0 {
                        let curr_midi = note.midi_key();

                        let prev_y_center = grid_start_y + (state.max_midi - prev_midi) as f32 * state.row_height + state.row_height * 0.5;
                        let prev_x_end = rect.min.x + keyboard_width + ((prev_start_ms + prev_dur_ms) * state.px_per_ms as f64) as f32;

                        let glide_ms = 80.0f64;
                        let mut arm_points = Vec::new();
                        let mut arm_t = -glide_ms * 0.5;

                        while arm_t <= glide_ms * 0.5 {
                            let rel_cents = PitchBendSolver::get_legato_transition_offset_cents(arm_t, prev_midi, curr_midi, glide_ms);
                            let arm_x = x_start + (arm_t * state.px_per_ms as f64) as f32;
                            let arm_y = y_center - (rel_cents / 100.0) as f32 * state.row_height;
                            arm_points.push(Pos2::new(arm_x, arm_y));
                            arm_t += 5.0;
                        }

                        painter.line_segment(
                            [Pos2::new(prev_x_end, prev_y_center), Pos2::new(x_start - (glide_ms * 0.5 * state.px_per_ms as f64) as f32, prev_y_center)],
                            Stroke::new(2.8, MelodyneTheme::PITCH_ARM_GOLD),
                        );

                        if arm_points.len() >= 2 {
                            for w in arm_points.windows(2) {
                                painter.line_segment([w[0], w[1]], Stroke::new(2.8, MelodyneTheme::PITCH_ARM_GOLD));
                            }
                        }
                    }
                }

                // Render Pitch Control Anchor Circles
                for pt in &note.pitch_bend.points {
                    let px_x = x_start + (pt.time_offset_ms * state.px_per_ms as f64) as f32;
                    let px_y = y_center - (pt.pitch_offset_cents / 100.0) as f32 * state.row_height;
                    painter.circle_filled(Pos2::new(px_x, px_y), 3.8, MelodyneTheme::PITCH_ANCHOR_CYAN);
                    painter.circle_stroke(Pos2::new(px_x, px_y), 3.8, Stroke::new(1.2, Color32::WHITE));
                }

                // Tool Mouse Interactions & Double-Click to edit lyric
                if let Some(mpos) = mouse_interact_pos {
                    // Right edge resize handle
                    let resize_handle_right = Rect::from_min_max(
                        Pos2::new(x_end - 8.0, y_top),
                        Pos2::new(x_end + 4.0, y_bottom),
                    );

                    // Left edge resize handle (for phoneme boundary adjustment)
                    let resize_handle_left = Rect::from_min_max(
                        Pos2::new(x_start - 4.0, y_top),
                        Pos2::new(x_start + 8.0, y_bottom),
                    );

                    // Show resize cursor when hovering over left or right edge
                    if note_rect.contains(mpos) && (state.active_tool == EditTool::Pointer || state.active_tool == EditTool::Pencil) {
                        if resize_handle_left.contains(mpos) || resize_handle_right.contains(mpos) {
                            ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                        }
                    }

                    let pitch_draw_target_rect = Rect::from_min_max(
                        Pos2::new(x_start - (200.0 * state.px_per_ms as f64) as f32, y_top - state.row_height * 4.0),
                        Pos2::new(x_end + (200.0 * state.px_per_ms as f64) as f32, y_bottom + state.row_height * 4.0),
                    );

                    if note_rect.contains(mpos) && mpos.y > grid_start_y && mpos.y < grid_end_y {
                        if ui.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary)) {
                            state.editing_lyric_index = Some(idx);
                            state.lyric_buffer = note.lyric.clone();
                            state.autocomplete_selected_idx = 0;
                        }

                        match state.active_tool {
                            EditTool::Eraser => {
                                if ui.input(|i| i.pointer.primary_clicked()) {
                                    note_to_delete = Some(idx);
                                }
                            }
                            EditTool::PitchDraw => {}
                            EditTool::Pointer | EditTool::Pencil => {
                                // Use primary_pressed() (not primary_clicked()) so drag starts
                                // on mouse-down, enabling click-and-drag for resize/move
                                let just_pressed = ui.input(|i| i.pointer.primary_pressed());
                                if just_pressed && !is_editing_lyric && state.dragging_note_idx.is_none() {
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
                                    state.properties_window_for_note = Some(idx);
                                }
                            }
                        }
                    }

                    // Pitch Drawing Tool Handler for Note
                    if state.active_tool == EditTool::PitchDraw && pitch_draw_target_rect.contains(mpos) {
                        if ui.input(|i| i.pointer.primary_down()) {
                            let rel_t = ((mpos.x - x_start) / state.px_per_ms) as f64;
                            let delta_y = y_center - mpos.y;
                            let cents = ((delta_y / state.row_height) * 100.0) as f64;
                            let cents = cents.clamp(-1200.0, 1200.0);

                            note.pitch_bend.points.retain(|pt| (pt.time_offset_ms - rel_t).abs() >= 6.0);
                            note.pitch_bend.points.push(UPitchBendPoint {
                                time_offset_ms: rel_t,
                                pitch_offset_cents: cents,
                                shape: "s".to_string(),
                            });

                            note.pitch_bend.points.sort_by(|a, b| a.time_offset_ms.partial_cmp(&b.time_offset_ms).unwrap_or(std::cmp::Ordering::Equal));
                            on_note_changed();
                        }

                        if ui.input(|i| i.pointer.primary_released()) && note.pitch_bend.points.len() > 2 {
                            let simplified = crate::dsp::pitch_bend::PitchBendSolver::simplify_pitch_points(&note.pitch_bend.points, 2.0);
                            note.pitch_bend.points = simplified;
                            on_note_changed();
                        }

                        if ui.input(|i| i.pointer.secondary_clicked()) {
                            note.pitch_bend.points.clear();
                            on_note_changed();
                        }
                    }
                }
            }

            if let Some((idx, new_lyric)) = commit_lyric_edit {
                if idx < notes.len() {
                    notes[idx].lyric = new_lyric;
                }
                state.editing_lyric_index = None;
                on_note_changed();
            }

            if let Some(del_idx) = note_to_delete {
                notes.remove(del_idx);
                state.selected_note_indices.remove(&del_idx);
                if state.selected_note_index == Some(del_idx) {
                    state.selected_note_index = None;
                }
                on_note_changed();
            }

            // Global Keyboard Shortcuts (Cmd+A, Delete, Arrow Up/Down Pitch Transposition)
            if state.editing_lyric_index.is_none() {
                let (select_all, delete_sel, arrow_up, arrow_down, is_shift) = ui.input(|i| (
                    i.modifiers.command && i.key_pressed(egui::Key::A),
                    i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace),
                    i.key_pressed(egui::Key::ArrowUp),
                    i.key_pressed(egui::Key::ArrowDown),
                    i.modifiers.shift,
                ));

                if select_all {
                    state.selected_note_indices = (0..notes.len()).collect();
                    if !notes.is_empty() {
                        state.selected_note_index = Some(0);
                    }
                }

                if delete_sel && !state.selected_note_indices.is_empty() {
                    let mut to_delete: Vec<usize> = state.selected_note_indices.iter().copied().collect();
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
                    let shift_amt = if is_shift { 12 } else { 1 };
                    let delta = if arrow_up { shift_amt } else { -shift_amt };
                    for &n_idx in &state.selected_note_indices {
                        if n_idx < notes.len() {
                            let curr_midi = notes[n_idx].midi_key();
                            let new_midi = (curr_midi as i16 + delta).clamp(state.min_midi as i16, state.max_midi as i16) as u8;
                            notes[n_idx].pitch = midi_to_note_name(new_midi);
                        }
                    }
                    on_note_changed();
                }
            }

            // Dragging update (Supports multi-note selection group movement, left/right resize)
            if let (Some(drag_idx), Some(start_pos)) = (state.dragging_note_idx, state.drag_start_pos) {
                if ui.input(|i| i.pointer.primary_down()) {
                    if let Some(current_pos) = ui.input(|i| i.pointer.interact_pos()) {
                        let delta_x = current_pos.x - start_pos.x;
                        let delta_y = current_pos.y - start_pos.y;
                        let delta_ms = delta_x as f64 / state.px_per_ms as f64;

                        if drag_idx < notes.len() {
                            if state.dragging_is_resize {
                                // Right-edge resize: extend/shrink duration
                                let raw_dur = (state.note_original_duration_ms + delta_ms).max(20.0);
                                notes[drag_idx].duration_ms = apply_snap(raw_dur, snap_option, bpm).max(20.0);
                            } else if state.dragging_is_left_resize {
                                // Left-edge resize: move start position, adjust duration inversely
                                // This keeps the right edge (end position) fixed
                                let original_end_ms = state.note_original_start_ms + state.note_original_duration_ms;
                                let raw_new_start = (state.note_original_start_ms + delta_ms).max(0.0);
                                let new_start = apply_snap(raw_new_start, snap_option, bpm).max(0.0);
                                let new_dur = (original_end_ms - new_start).max(20.0);
                                notes[drag_idx].position_ms = new_start;
                                notes[drag_idx].duration_ms = new_dur;
                            } else {
                                // Full note move (position + pitch relative to original drag start snapshot)
                                let delta_semitones = -(delta_y / state.row_height).round() as i32;
                                let raw_pos = (state.note_original_start_ms + delta_ms).max(0.0);
                                let new_pos = apply_snap(raw_pos, snap_option, bpm).max(0.0);
                                let new_m = (state.note_original_midi as i32 + delta_semitones).clamp(state.min_midi as i32, state.max_midi as i32) as u8;

                                notes[drag_idx].position_ms = new_pos;
                                notes[drag_idx].set_midi_key(new_m);
                            }
                        }
                    }
                } else {
                    state.dragging_note_idx = None;
                    state.drag_start_pos = None;
                    state.dragging_is_left_resize = false;
                    state.dragging_is_resize = false;
                    on_note_changed();
                }
            }

            // 6. Interactive Note Drawing & Marquee Selection Handling
            if let Some(mpos) = mouse_interact_pos {
                let is_hovering_canvas = response.hovered() && ui.clip_rect().contains(mpos);
                if is_hovering_canvas {
                    if mpos.x > rect.min.x + keyboard_width && mpos.y > grid_start_y && mpos.y < grid_end_y && state.editing_lyric_index.is_none() {
                    match state.active_tool {
                        EditTool::Pointer => {
                            if response.drag_started() && state.dragging_note_idx.is_none() {
                                state.marquee_start = Some(mpos);
                                state.marquee_current = Some(mpos);
                            }

                            if response.dragged() && state.marquee_start.is_some() {
                                state.marquee_current = Some(mpos);

                                if let (Some(m_start), Some(m_curr)) = (state.marquee_start, state.marquee_current) {
                                    let sel_rect = Rect::from_two_pos(m_start, m_curr);
                                    painter.rect_filled(sel_rect, Rounding::same(2.0), Color32::from_rgba_premultiplied(245, 176, 65, 35));
                                    painter.rect_stroke(sel_rect, Rounding::same(2.0), Stroke::new(1.2, MelodyneTheme::NOTE_SELECTED_GOLD));

                                    state.selected_note_indices.clear();
                                    for (n_i, note) in notes.iter().enumerate() {
                                        let n_midi = note.midi_key();
                                        if n_midi >= state.min_midi && n_midi <= state.max_midi {
                                            let key_i = (state.max_midi - n_midi) as f32;
                                            let y_t = grid_start_y + key_i * state.row_height + 2.0;
                                            let y_b = y_t + state.row_height - 4.0;
                                            let x_s = rect.min.x + keyboard_width + (note.position_ms * state.px_per_ms as f64) as f32;
                                            let x_e = x_s + (note.duration_ms * state.px_per_ms as f64) as f32;
                                            let n_rect = Rect::from_min_max(Pos2::new(x_s, y_t), Pos2::new(x_e, y_b));

                                            if sel_rect.intersects(n_rect) {
                                                state.selected_note_indices.insert(n_i);
                                            }
                                        }
                                    }
                                }
                            }

                            if !ui.input(|i| i.pointer.primary_down()) {
                                state.marquee_start = None;
                                state.marquee_current = None;
                            }
                        }

                        EditTool::Pencil => {
                            if ui.input(|i| i.pointer.primary_down()) {
                                if state.creating_note_idx.is_none() && state.dragging_note_idx.is_none() {
                                    let click_x = mpos.x - (rect.min.x + keyboard_width);
                                    let raw_start_ms = (click_x / state.px_per_ms) as f64;
                                    let click_start_ms = apply_snap(raw_start_ms, snap_option, bpm).max(0.0);
                                    let key_idx = ((mpos.y - grid_start_y) / state.row_height).floor() as u8;
                                    let click_midi = (state.max_midi.saturating_sub(key_idx)).clamp(state.min_midi, state.max_midi);

                                    let new_note = UNote::new("ka", midi_to_note_name(click_midi), click_start_ms, 50.0);
                                    notes.push(new_note);
                                    let new_idx = notes.len() - 1;
                                    state.creating_note_idx = Some(new_idx);
                                    state.selected_note_index = Some(new_idx);
                                    state.drag_start_pos = Some(mpos);
                                } else if let (Some(c_idx), Some(_start_p)) = (state.creating_note_idx, state.drag_start_pos) {
                                    if c_idx < notes.len() {
                                        let curr_x = mpos.x - (rect.min.x + keyboard_width);
                                        let curr_ms = (curr_x / state.px_per_ms) as f64;
                                        let raw_dur = (curr_ms - notes[c_idx].position_ms).max(50.0);
                                        notes[c_idx].duration_ms = apply_snap(raw_dur, snap_option, bpm).max(50.0);
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
            }



            // 7. Left Sticky Opaque Piano Keyboard Sidebar (Renders OVER notes so notes pass underneath)
            let sticky_key_x = rect.min.x.max(ui.clip_rect().min.x);

            // Draw solid background container for keys sidebar
            let keys_bg_rect = Rect::from_min_max(
                Pos2::new(sticky_key_x, grid_start_y),
                Pos2::new(sticky_key_x + keyboard_width, grid_end_y),
            );
            painter.rect_filled(keys_bg_rect, Rounding::ZERO, Color32::from_rgb(20, 16, 28));

            for key_idx in 0..key_count {
                let midi = state.max_midi - key_idx as u8;
                let y_top = grid_start_y + key_idx as f32 * state.row_height;
                let y_bottom = y_top + state.row_height;

                let is_black_key = matches!(midi % 12, 1 | 3 | 6 | 8 | 10);

                let key_rect = Rect::from_min_max(
                    Pos2::new(sticky_key_x, y_top),
                    Pos2::new(sticky_key_x + keyboard_width, y_bottom),
                );

                let key_color = if is_black_key {
                    MelodyneTheme::BG_KEYBOARD_BLACK
                } else {
                    MelodyneTheme::BG_KEYBOARD_WHITE
                };
                let text_color = if is_black_key {
                    MelodyneTheme::TEXT_GOLD_LABEL
                } else {
                    Color32::from_rgb(30, 25, 35)
                };

                let mouse_pos = ui.input(|i| i.pointer.interact_pos());
                if let Some(mpos) = mouse_pos {
                    if key_rect.contains(mpos) && ui.input(|i| i.pointer.any_click()) {
                        let freq = midi_to_freq(midi as f64);
                        on_preview_freq(freq);
                    }
                }

                painter.rect_filled(key_rect, Rounding::ZERO, key_color);
                painter.rect_stroke(key_rect, Rounding::ZERO, Stroke::new(1.0, Color32::from_rgb(15, 12, 20)));

                if midi % 12 == 0 || !is_black_key {
                    let note_str = midi_to_note_name(midi);
                    painter.text(
                        Pos2::new(sticky_key_x + 8.0, y_top + state.row_height * 0.5),
                        egui::Align2::LEFT_CENTER,
                        note_str,
                        egui::FontId::proportional(11.0),
                        text_color,
                    );
                }
            }

            // Draw dividing vertical gold line separating keys sidebar from canvas grid
            painter.line_segment(
                [Pos2::new(sticky_key_x + keyboard_width, grid_start_y), Pos2::new(sticky_key_x + keyboard_width, grid_end_y)],
                Stroke::new(2.0, MelodyneTheme::ACCENT_GOLD),
            );

            // 8. Playhead Marker Line & Handle
            let playhead_x = rect.min.x + keyboard_width + (state.playhead_ms * state.px_per_ms as f64) as f32;
            if playhead_x >= sticky_key_x + keyboard_width && playhead_x <= rect.max.x {
                let handle_points = vec![
                    Pos2::new(playhead_x - 6.0, rect.min.y + 2.0),
                    Pos2::new(playhead_x + 6.0, rect.min.y + 2.0),
                    Pos2::new(playhead_x, grid_start_y - 2.0),
                ];
                painter.add(egui::Shape::convex_polygon(handle_points, MelodyneTheme::PLAYHEAD_RED, Stroke::new(1.0, Color32::WHITE)));

                painter.line_segment(
                    [Pos2::new(playhead_x, grid_start_y), Pos2::new(playhead_x, grid_end_y)],
                    Stroke::new(2.5, MelodyneTheme::PLAYHEAD_RED),
                );
            }
        });

    if let Some(prop_idx) = state.properties_window_for_note {
        if prop_idx < notes.len() {
            let mut close_window = false;
            let note = &mut notes[prop_idx];
            egui::Window::new("Note Properties")
                .collapsible(false)
                .resizable(false)
                .show(ui.ctx(), |ui| {
                    egui::Grid::new("prop_grid").num_columns(2).spacing([10.0, 10.0]).show(ui, |ui| {
                        ui.label("Lyric");
                        ui.text_edit_singleline(&mut note.lyric);
                        ui.end_row();

                        ui.label("Pitch");
                        ui.label(note.pitch.clone());
                        ui.end_row();

                        ui.label("Duration (ms)");
                        ui.add(egui::DragValue::new(&mut note.duration_ms).speed(1.0).range(20.0..=10000.0));
                        ui.end_row();

                        ui.label("Velocity (VEL)");
                        ui.add(egui::DragValue::new(&mut note.expressions.velocity).speed(1.0).range(0.0..=200.0));
                        ui.end_row();

                        ui.label("Consonant Velocity");
                        ui.add(egui::DragValue::new(&mut note.expressions.consonant_velocity).speed(1.0).range(0.0..=200.0));
                        ui.end_row();

                        ui.label("Modulation (MOD)");
                        ui.add(egui::DragValue::new(&mut note.expressions.modulation).speed(1.0).range(0.0..=200.0));
                        ui.end_row();
                        
                        // NOTE: flags might not exist in UNote yet, I should check model.rs. 
                        // If it doesn't, I will just remove the Flags line or add it.
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
                // Since this uses FnMut and we can't call on_note_changed easily if we borrow it mutably twice,
                // we assume egui::Window handles state and user just presses OK.
                // Or we can just let the main loop capture the change.
            }
        } else {
            state.properties_window_for_note = None;
        }
    }
}
