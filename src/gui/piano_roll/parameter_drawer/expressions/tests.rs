use super::*;

fn frame(
    ctx: &egui::Context,
    state: &mut PianoRollState,
    events: Vec<egui::Event>,
    time: f64,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1000.0, 700.0))),
            events,
            time: Some(time),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                draw(
                    ui,
                    &mut [UNote::new("ka", "C4", 1000.0, 1000.0)],
                    state,
                    65.0,
                    0.0,
                    &mut || {},
                    &ThemeConfig::default(),
                    Rect::from_min_size(Pos2::new(8.0, 8.0), Vec2::new(984.0, 28.0)),
                    120.0,
                    crate::config::AppLanguage::PtBr,
                );
            });
        },
    )
}

fn visible_tabs(output: &egui::FullOutput) -> Vec<(String, Rect)> {
    output
        .shapes
        .iter()
        .filter_map(|clipped| {
            if let egui::epaint::Shape::Text(text) = &clipped.shape {
                let label = text.galley.text();
                let rect = Rect::from_min_size(text.pos, text.galley.size());
                if label.contains('(') && clipped.clip_rect.contains_rect(rect) {
                    return Some((label.to_owned(), rect));
                }
            }
            None
        })
        .collect()
}

fn graph_rect(output: &egui::FullOutput) -> Rect {
    output
        .shapes
        .iter()
        .find_map(|clipped| {
            if let egui::epaint::Shape::Rect(rect) = &clipped.shape {
                if rect.fill == Color32::from_rgb(18, 18, 22) {
                    return Some(rect.rect.intersect(clipped.clip_rect));
                }
            }
            None
        })
        .expect("parameter graph is painted")
}

#[test]
fn taller_drawer_reveals_more_tabs_and_keeps_graph_visible() {
    let mut counts = Vec::new();
    for height in [100.0, 400.0] {
        let ctx = egui::Context::default();
        let mut state = PianoRollState {
            drawer_height: height,
            show_parameters_drawer: true,
            ..Default::default()
        };
        frame(&ctx, &mut state, vec![], 0.0);
        let output = frame(&ctx, &mut state, vec![], 0.1);
        counts.push(visible_tabs(&output).len());
        let graph = graph_rect(&output);
        assert!(graph.width() > 600.0, "graph is offscreen: {graph:?}");
        assert!(
            graph.height() > height - 20.0,
            "graph did not grow: {graph:?}"
        );
    }
    assert!(counts[0] < counts[1], "visible tabs: {counts:?}");
    assert_eq!(counts[1], 12);
}

#[test]
fn dragging_resize_handle_expands_list_and_persists_height() {
    let ctx = egui::Context::default();
    let mut state = PianoRollState {
        drawer_height: 100.0,
        show_parameters_drawer: true,
        ..Default::default()
    };
    frame(&ctx, &mut state, vec![], 0.0);
    let initial = frame(&ctx, &mut state, vec![], 0.1);
    let count = visible_tabs(&initial).len();
    let start = Pos2::new(500.0, 692.0 - state.drawer_height);
    frame(
        &ctx,
        &mut state,
        vec![
            egui::Event::PointerMoved(start),
            egui::Event::PointerButton {
                pos: start,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            },
        ],
        0.2,
    );
    let end = start - Vec2::new(0.0, 300.0);
    frame(&ctx, &mut state, vec![egui::Event::PointerMoved(end)], 0.3);
    frame(&ctx, &mut state, vec![], 0.4);
    frame(
        &ctx,
        &mut state,
        vec![egui::Event::PointerButton {
            pos: end,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::default(),
        }],
        0.5,
    );
    let output = frame(&ctx, &mut state, vec![], 0.6);
    assert!(
        state.drawer_height >= 390.0,
        "height: {}",
        state.drawer_height
    );
    assert!(visible_tabs(&output).len() > count);
    assert_eq!(visible_tabs(&output).len(), 12);
    assert!(graph_rect(&output).height() > 380.0);
}

#[test]
fn clicking_each_parameter_selects_it_without_hiding_graph() {
    let ctx = egui::Context::default();
    let mut state = PianoRollState {
        drawer_height: 400.0,
        show_parameters_drawer: true,
        ..Default::default()
    };
    frame(&ctx, &mut state, vec![], 0.0);
    let output = frame(&ctx, &mut state, vec![], 0.1);
    let tabs = visible_tabs(&output);
    assert_eq!(tabs.len(), 12);
    let expected = [
        ParameterTab::Dynamics,
        ParameterTab::PitchDelta,
        ParameterTab::Gender,
        ParameterTab::Velocity,
        ParameterTab::Breathiness,
        ParameterTab::Modulation,
        ParameterTab::Volume,
        ParameterTab::Attack,
        ParameterTab::Decay,
        ParameterTab::VibratoLength,
        ParameterTab::VibratoDepth,
        ParameterTab::VibratoPeriod,
    ];
    for (index, ((label, rect), expected)) in tabs.iter().zip(expected).enumerate() {
        let pos = rect.center();
        for pressed in [true, false] {
            let output = frame(
                &ctx,
                &mut state,
                vec![
                    egui::Event::PointerMoved(pos),
                    egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::default(),
                    },
                ],
                0.2 + index as f64 * 0.2 + if pressed { 0.0 } else { 0.05 },
            );
            assert!(graph_rect(&output).width() > 600.0);
        }
        assert_eq!(state.selected_parameter, expected, "{label}");
    }
}
