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
                    None,
                    state,
                    65.0,
                    0.0,
                    &mut || {},
                    &ThemeConfig::default(),
                    Rect::from_min_size(Pos2::new(8.0, 8.0), Vec2::new(984.0, 28.0)),
                    120.0,
                    4,
                    4,
                    crate::config::AppLanguage::PtBr,
                    None,
                    "",
                    &mut || {},
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
                if rect.fill == egui::Visuals::dark().panel_fill {
                    return Some(rect.rect.intersect(clipped.clip_rect));
                }
            }
            None
        })
        .expect("parameter graph is painted")
}

fn frame_with_note(
    ctx: &egui::Context,
    state: &mut PianoRollState,
    note: &mut UNote,
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
                    std::slice::from_mut(note),
                    None,
                    state,
                    65.0,
                    0.0,
                    &mut || {},
                    &ThemeConfig::default(),
                    Rect::from_min_size(Pos2::new(8.0, 8.0), Vec2::new(984.0, 28.0)),
                    120.0,
                    4,
                    4,
                    crate::config::AppLanguage::PtBr,
                    None,
                    "",
                    &mut || {},
                );
            });
        },
    )
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
    assert_eq!(counts[1], 15);
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
    assert_eq!(visible_tabs(&output).len(), 15);
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
    assert_eq!(tabs.len(), 15);
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
        ParameterTab::EnvelopeFadeIn,
        ParameterTab::EnvelopeFadeOut,
        ParameterTab::EnvelopeCrossfade,
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

#[test]
fn tainah_resampler_exposes_acoustic_flag_tabs() {
    let ctx = egui::Context::default();
    let mut state = PianoRollState {
        drawer_height: 800.0,
        show_parameters_drawer: true,
        ..Default::default()
    };
    let run_frame = |ctx: &egui::Context, state: &mut PianoRollState, time: f64| {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(1400.0, 1000.0))),
                time: Some(time),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    draw(
                        ui,
                        &mut [UNote::new("ka", "C4", 1000.0, 1000.0)],
                        None,
                        state,
                        65.0,
                        0.0,
                        &mut || {},
                        &ThemeConfig::default(),
                        Rect::from_min_size(Pos2::new(8.0, 8.0), Vec2::new(1384.0, 28.0)),
                        120.0,
                        4,
                        4,
                        crate::config::AppLanguage::PtBr,
                        None,
                        "Tainah",
                        &mut || {},
                    );
                });
            },
        )
    };
    run_frame(&ctx, &mut state, 0.0);
    let output = run_frame(&ctx, &mut state, 0.1);

    let all_labels: Vec<String> = output
        .shapes
        .iter()
        .filter_map(|clipped| {
            if let egui::epaint::Shape::Text(text) = &clipped.shape {
                Some(text.galley.text().to_string())
            } else {
                None
            }
        })
        .collect();

    // Deve conter todas as abas das novas flags acústicas do Tainah
    assert!(
        all_labels.iter().any(|t| t.contains("Tensão (T)")),
        "aba de Tensão deve estar presente"
    );
    assert!(
        all_labels.iter().any(|t| t.contains("Arezamento (A)")),
        "aba de Arezamento deve estar presente"
    );
    assert!(
        all_labels.iter().any(|t| t.contains("Calor de Peito (W)")),
        "aba de Calor deve estar presente"
    );
    assert!(
        all_labels.iter().any(|t| t.contains("Punch Consoante (E)")),
        "aba de Edge deve estar presente"
    );
    assert!(
        all_labels.iter().any(|t| t.contains("De-Esser (s)")),
        "aba de De-Esser deve estar presente"
    );
    assert!(
        all_labels
            .iter()
            .any(|t| t.contains("Saturação Glotal (U)")),
        "aba de Saturação deve estar presente"
    );
    assert!(
        all_labels.iter().any(|t| t.contains("Humanização (J)")),
        "aba de Humanize deve estar presente"
    );
    assert!(
        all_labels.iter().any(|t| t.contains("Laringe (K)")),
        "aba de Laringe deve estar presente"
    );
    assert!(
        all_labels.iter().any(|t| t.contains("Nasalidade (N)")),
        "aba de Nasalidade deve estar presente"
    );
    assert!(
        all_labels.iter().any(|t| t.contains("Formante Cantor (S)")),
        "aba de Singer Formant deve estar presente"
    );
    assert!(
        all_labels.iter().any(|t| t.contains("R Rótico (R)")),
        "aba de Rótico deve estar presente"
    );
    assert!(
        all_labels.iter().any(|t| t.contains("Abertura F1 (V)")),
        "aba de Abertura Vogal deve estar presente"
    );
    assert!(
        all_labels
            .iter()
            .any(|t| t.contains("Anterioridade F2 (D)")),
        "aba de Avanço Vogal deve estar presente"
    );

    // Chips rápidos horizontais também devem estar presentes
    assert!(
        all_labels.iter().any(|t| t.contains("A Air")),
        "chip A Air deve estar presente"
    );
    assert!(
        all_labels.iter().any(|t| t.contains("s DeEss")),
        "chip s DeEss deve estar presente"
    );
}

#[test]
fn dynamics_hover_uses_the_cached_phoneme_under_the_cursor() {
    let ctx = egui::Context::default();
    let mut state = PianoRollState {
        drawer_height: 240.0,
        show_parameters_drawer: true,
        ..Default::default()
    };
    state.note_phonemes_cache = vec![vec![
        CachedPhoneme {
            alias: "か".to_string(),
            relative_position_ms: 0.0,
            duration_ms: 400.0,
            ..Default::default()
        },
        CachedPhoneme {
            alias: "e".to_string(),
            relative_position_ms: 400.0,
            duration_ms: 600.0,
            ..Default::default()
        },
    ]];
    let mut note = UNote::new("ka", "C4", 1000.0, 1000.0);

    frame_with_note(&ctx, &mut state, &mut note, vec![], 0.0);
    let first_frame = frame_with_note(&ctx, &mut state, &mut note, vec![], 0.1);
    let zero_label = first_frame
        .shapes
        .iter()
        .find_map(|clipped| match &clipped.shape {
            egui::epaint::Shape::Text(text) if text.galley.text() == "0" => Some(text.pos),
            _ => None,
        })
        .expect("parameter graph has a zero-value axis label");
    let cursor = Pos2::new(zero_label.x + 80.0, zero_label.y);
    let phoneme_time_ms = (cursor.x - 73.0) as f64 / f64::from(state.px_per_ms);
    note.position_ms = (phoneme_time_ms - 500.0).max(0.0);
    let output = frame_with_note(
        &ctx,
        &mut state,
        &mut note,
        vec![egui::Event::PointerMoved(cursor)],
        0.2,
    );

    let labels = output
        .shapes
        .iter()
        .filter_map(|clipped| match &clipped.shape {
            egui::epaint::Shape::Text(text) => Some(text.galley.text().to_owned()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        labels.iter().any(|label| label.contains("ka (e)")),
        "Dynamics hover should identify the cached phone under the pointer; labels: {labels:?}"
    );
}
