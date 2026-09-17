use super::*;

fn frame(
    ctx: &egui::Context,
    state: &mut PianoRollState,
    notes: &mut Vec<UNote>,
    time: f64,
    events: Vec<egui::Event>,
) -> (usize, usize) {
    let mut before = 0;
    let mut changed = 0;
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0))),
            time: Some(time),
            events,
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                draw_piano_roll(
                    ui,
                    notes,
                    state,
                    &ThemeConfig::default(),
                    None,
                    &mut PhonemePaletteState::default(),
                    GridSnapOption::Freeform,
                    120.0,
                    crate::phonemizer::PhonemizerMode::default(),
                    crate::config::AppLanguage::PtBr,
                    &mut |_| {},
                    &mut || before += 1,
                    &mut || changed += 1,
                    &mut |_| {},
                    &mut |_, _| {},
                );
            });
        },
    );
    (before, changed)
}

fn button(pos: Pos2, pressed: bool) -> Vec<egui::Event> {
    vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::default(),
        },
    ]
}

#[test]
fn first_pencil_drag_keeps_duration_and_commits_outside_canvas() {
    let ctx = egui::Context::default();
    let mut state = PianoRollState {
        active_tool: EditTool::Pencil,
        ..Default::default()
    };
    let mut notes = Vec::new();
    frame(&ctx, &mut state, &mut notes, 0.0, vec![]);
    frame(&ctx, &mut state, &mut notes, 0.02, vec![]);
    let start = Pos2::new(200.0, 220.0);
    assert_eq!(
        frame(&ctx, &mut state, &mut notes, 0.1, button(start, true)),
        (1, 0)
    );
    assert_eq!(notes.len(), 1);
    let offset = state.vertical_scroll_offset;
    let end = Pos2::new(400.0, 230.0);
    frame(
        &ctx,
        &mut state,
        &mut notes,
        0.2,
        vec![egui::Event::PointerMoved(end)],
    );
    assert!((notes[0].duration_ms - 800.0).abs() < 0.01);
    assert_eq!(state.vertical_scroll_offset, offset);
    // A release over the ruler still completes the same transaction.
    let outside = Pos2::new(400.0, 0.0);
    assert_eq!(
        frame(&ctx, &mut state, &mut notes, 0.3, button(outside, false)),
        (0, 1)
    );
    assert_eq!(state.creating_note_idx, None);
    assert_eq!(state.drag_start_pos, None);
    assert!((notes[0].duration_ms - 800.0).abs() < 0.01);
    assert_eq!(frame(&ctx, &mut state, &mut notes, 0.4, vec![]), (0, 0));
}

#[test]
fn entering_canvas_with_button_held_does_not_create_a_note() {
    let ctx = egui::Context::default();
    let mut state = PianoRollState {
        active_tool: EditTool::Pencil,
        ..Default::default()
    };
    let mut notes = Vec::new();
    frame(&ctx, &mut state, &mut notes, 0.0, vec![]);
    frame(
        &ctx,
        &mut state,
        &mut notes,
        0.1,
        button(Pos2::new(200.0, 0.0), true),
    );
    frame(
        &ctx,
        &mut state,
        &mut notes,
        0.2,
        vec![egui::Event::PointerMoved(Pos2::new(250.0, 220.0))],
    );
    frame(
        &ctx,
        &mut state,
        &mut notes,
        0.3,
        button(Pos2::new(250.0, 220.0), false),
    );
    assert!(notes.is_empty());
}

#[test]
fn selecting_note_keeps_scroll_and_double_click_edits_lyric() {
    let ctx = egui::Context::default();
    let mut state = PianoRollState {
        active_tool: EditTool::Pencil,
        ..Default::default()
    };
    let mut notes = Vec::new();
    frame(&ctx, &mut state, &mut notes, 0.0, vec![]);
    frame(&ctx, &mut state, &mut notes, 0.02, vec![]);
    let start = Pos2::new(200.0, 220.0);
    frame(&ctx, &mut state, &mut notes, 0.1, button(start, true));
    frame(
        &ctx,
        &mut state,
        &mut notes,
        0.2,
        button(Pos2::new(400.0, 220.0), false),
    );
    state.active_tool = EditTool::Pointer;
    state.selected_note_index = None;
    state.selected_note_indices.clear();
    frame(&ctx, &mut state, &mut notes, 0.8, vec![]);
    let offset = (state.horizontal_scroll_offset, state.vertical_scroll_offset);
    let point = Pos2::new(300.0, 220.0);
    for (time, pressed) in [(1.0, true), (1.05, false), (1.15, true), (1.20, false)] {
        frame(&ctx, &mut state, &mut notes, time, button(point, pressed));
        assert_eq!(
            (state.horizontal_scroll_offset, state.vertical_scroll_offset),
            offset
        );
    }
    assert_eq!(state.selected_note_index, Some(0));
    assert_eq!(state.editing_lyric_index, Some(0));
    // Focus is requested when the inline lyric editor is drawn on the next frame.
    for time in [1.22, 1.24] {
        frame(&ctx, &mut state, &mut notes, time, vec![]);
        assert_eq!(
            (state.horizontal_scroll_offset, state.vertical_scroll_offset),
            offset
        );
    }
}
