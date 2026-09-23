use super::PianoRollState;
use crate::gui::types::AutoScrollMode;
use crate::project::model::UNote;
use eframe::egui;

pub(crate) struct NavigationFrame {
    pub is_mod_zoom: bool,
    pub is_pinch_zoom: bool,
    pub is_middle_panning: bool,
}

/// Handle viewport-scoped panning and zooming. Keeping this frame-global input
/// logic outside the note canvas makes it harder for note interactions to steal
/// events from the inspector or playback transport.
pub(crate) fn update(
    ui: &mut egui::Ui,
    state: &mut PianoRollState,
    ruler_height: f32,
    keyboard_width: f32,
) -> NavigationFrame {
    let (ctrl_pressed, alt_pressed, cmd_pressed) = ui.input(|i| {
        (
            i.modifiers.ctrl,
            i.modifiers.alt,
            i.modifiers.command || i.modifiers.mac_cmd,
        )
    });
    let is_mod_zoom = ctrl_pressed || alt_pressed || cmd_pressed;
    let (is_middle_down, mouse_delta) = ui.input(|i| {
        (
            i.pointer.middle_down() || i.pointer.button_down(egui::PointerButton::Middle),
            i.pointer.delta(),
        )
    });
    let available_viewport = ui.available_rect_before_wrap();
    let is_hovering_piano_roll = ui.input(|i| {
        i.pointer
            .hover_pos()
            .or(i.pointer.latest_pos())
            .map(|pos| available_viewport.contains(pos))
            .unwrap_or(false)
    });
    let is_pinch_zoom = ui.input(|i| {
        i.zoom_delta() != 1.0
            || i.events
                .iter()
                .any(|event| matches!(event, egui::Event::Zoom(_)))
    });

    if state.is_playing && is_hovering_piano_roll {
        if is_middle_down {
            state.horizontal_follow_user_override = true;
            state.vertical_follow_user_override = true;
        } else if !is_mod_zoom && !is_pinch_zoom {
            let (delta, shift) = ui.input(|input| {
                let delta = if input.smooth_scroll_delta.length_sq() > 1e-6 {
                    input.smooth_scroll_delta
                } else {
                    input.raw_scroll_delta
                };
                (delta, input.modifiers.shift)
            });
            // Permite rolar livremente pelo trackpad sem desativar o modo de auto scroll ou cursor estacionário.
            if !shift && delta.y.abs() > 1e-3 {
                state.vertical_follow_user_override = true;
                state.vertical_pitch_follow = false;
                state.pitch_follow_smoothed_midi = None;
            }
        }
    }

    let mut is_middle_panning = false;
    if is_middle_down && (is_hovering_piano_roll || state.is_middle_panning) {
        state.is_middle_panning = true;
        is_middle_panning = true;
        if mouse_delta.length_sq() > 0.0 {
            state.horizontal_scroll_offset =
                (state.horizontal_scroll_offset - mouse_delta.x).max(0.0);
            state.vertical_scroll_offset = (state.vertical_scroll_offset - mouse_delta.y).max(0.0);
        }
        ui.output_mut(|o| o.cursor_icon = egui::CursorIcon::Grabbing);
    } else {
        state.is_middle_panning = false;
    }

    if (is_mod_zoom || is_pinch_zoom) && is_hovering_piano_roll {
        let wheel_delta = ui.input(|i| {
            let mut dy = 0.0f32;
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
            if dy.abs() < 1e-3 {
                dy = (i.zoom_delta() - 1.0) * 80.0;
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
            } else if ctrl_pressed || cmd_pressed || is_pinch_zoom {
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

    NavigationFrame {
        is_mod_zoom,
        is_pinch_zoom,
        is_middle_panning,
    }
}

pub(crate) fn update_playhead_scroll(
    ui: &egui::Ui,
    state: &mut PianoRollState,
    keyboard_width: f32,
) {
    if !state.is_playing {
        return;
    }
    if state.horizontal_follow_user_override {
        return;
    }
    let playhead_x = (state.playhead_ms * state.px_per_ms as f64) as f32;
    let visible_width = (ui.available_width() - keyboard_width).max(100.0);
    match state.auto_scroll_mode {
        AutoScrollMode::StationaryCursor => {
            state.horizontal_scroll_offset = (playhead_x - visible_width * 0.30).max(0.0);
        }
        AutoScrollMode::PageScroll => {
            let current = state.horizontal_scroll_offset;
            if playhead_x > current + visible_width * 0.92 || playhead_x < current {
                state.horizontal_scroll_offset = (playhead_x - visible_width * 0.08).max(0.0);
            }
        }
        AutoScrollMode::Off => {}
    }
}

fn pitch_follow_target_midi(notes: &[UNote], playhead_ms: f64, bpm: f64) -> Option<f32> {
    let previous = notes
        .iter()
        .filter(|note| note.position_ms <= playhead_ms)
        .max_by(|a, b| a.position_ms.total_cmp(&b.position_ms));
    let next = notes
        .iter()
        .filter(|note| note.position_ms > playhead_ms)
        .min_by(|a, b| a.position_ms.total_cmp(&b.position_ms));

    match (previous, next) {
        (Some(previous), Some(next)) => {
            let from = previous.midi_key() as f32;
            let to = next.midi_key() as f32;
            if (to - from).abs() <= 2.0 {
                return Some(from);
            }
            let beat_ms = 60_000.0 / bpm.max(10.0);
            let transition_ms = beat_ms.min((next.position_ms - previous.position_ms).max(120.0));
            let transition_start = (next.position_ms - transition_ms).max(previous.position_ms);
            if playhead_ms <= transition_start {
                return Some(from);
            }
            let progress = ((playhead_ms - transition_start)
                / (next.position_ms - transition_start).max(1.0))
            .clamp(0.0, 1.0);
            let eased = 0.5 - 0.5 * (std::f64::consts::PI * progress).cos();
            Some(from + (to - from) * eased as f32)
        }
        (Some(previous), None) => Some(previous.midi_key() as f32),
        (None, Some(next)) => Some(next.midi_key() as f32),
        (None, None) => None,
    }
}

pub(crate) fn update_pitch_follow(
    ui: &egui::Ui,
    state: &mut PianoRollState,
    notes: &[UNote],
    visible_height: f32,
    bpm: f64,
) {
    if !state.is_playing || !state.vertical_pitch_follow {
        state.pitch_follow_smoothed_midi = None;
        return;
    }
    if state.vertical_follow_user_override {
        return;
    }
    let Some(target_midi) = pitch_follow_target_midi(notes, state.playhead_ms, bpm) else {
        return;
    };

    // Lunai-style frame smoothing: approximately 80 ms time constant.
    let dt = ui.input(|input| input.stable_dt).clamp(0.0005, 0.05);
    let alpha = 1.0 - (-dt / 0.080).exp();
    let smoothed = state
        .pitch_follow_smoothed_midi
        .map_or(target_midi, |current| {
            current + (target_midi - current) * alpha
        });
    state.pitch_follow_smoothed_midi = Some(smoothed);

    let row_from_top = state.max_midi as f32 - smoothed;
    let target_offset =
        row_from_top * state.row_height + state.row_height * 0.5 - visible_height * 0.5;
    let grid_height = (state.max_midi - state.min_midi + 1) as f32 * state.row_height;
    state.vertical_scroll_offset =
        target_offset.clamp(0.0, (grid_height - visible_height).max(0.0));
    ui.ctx().request_repaint();
}

#[cfg(test)]
mod pitch_follow_tests {
    use super::pitch_follow_target_midi;
    use crate::dsp::pitch::midi_to_note_name;
    use crate::project::model::UNote;

    fn note(position_ms: f64, midi: u8) -> UNote {
        UNote::new("a", midi_to_note_name(midi), position_ms, 500.0)
    }

    #[test]
    fn pitch_follow_holds_small_intervals_and_leads_large_jumps() {
        let close = vec![note(0.0, 60), note(1_000.0, 62)];
        assert_eq!(pitch_follow_target_midi(&close, 900.0, 120.0), Some(60.0));

        let jump = vec![note(0.0, 60), note(1_000.0, 72)];
        let during_transition = pitch_follow_target_midi(&jump, 750.0, 120.0).unwrap();
        assert!(during_transition > 60.0 && during_transition < 72.0);
        assert_eq!(pitch_follow_target_midi(&jump, 1_000.0, 120.0), Some(72.0));
    }
}
