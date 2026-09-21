use super::*;

fn row_at(rect: Rect, count: usize, y: f32) -> usize {
    (((rect.bottom() - y) / rect.height() * count as f32).floor() as usize)
        .min(count.saturating_sub(1))
}

pub(super) fn draw(
    ui: &mut egui::Ui,
    notes: &mut [UNote],
    state: &mut PianoRollState,
    rect: Rect,
    response: &egui::Response,
    origin_x: f32,
    voicebank: Option<&Voicebank>,
    theme: &ThemeConfig,
    lang: crate::config::AppLanguage,
    on_before_change: &mut dyn FnMut(),
    on_note_changed: &mut dyn FnMut(),
) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, Rounding::same(3.0), theme.bg_canvas_c32());
    let mut options = vec![None];
    if let Some(vb) = voicebank {
        options.extend(vb.prefix_map.colors().map(|name| Some(name.to_string())));
    }
    // Retain authored values even when the current bank lacks that timbre.
    for note in notes.iter() {
        if !options.contains(&note.timbre) {
            options.push(note.timbre.clone());
        }
    }
    let row_h = rect.height() / options.len() as f32;
    let (pointer, primary, secondary, pressed) = ui.input(|i| {
        (
            i.pointer.interact_pos(),
            i.pointer.primary_down(),
            i.pointer.secondary_down(),
            i.pointer.primary_pressed() || i.pointer.secondary_pressed(),
        )
    });
    if pressed && (response.is_pointer_button_down_on() || response.contains_pointer()) {
        on_before_change();
        state.timbre_drag = pointer;
        state.timbre_changed = false;
    }
    if let (Some(previous), Some(pos)) = (state.timbre_drag, pointer) {
        if (primary || secondary) && rect.contains(pos) {
            let value = if secondary {
                None
            } else {
                options[row_at(rect, options.len(), pos.y)].clone()
            };
            let left = previous.x.min(pos.x);
            let right = previous.x.max(pos.x);
            let has_selection =
                !state.selected_note_indices.is_empty() || state.selected_note_index.is_some();
            for (idx, note) in notes.iter_mut().enumerate() {
                let start = origin_x + note.position_ms as f32 * state.px_per_ms;
                let end = start + note.duration_ms as f32 * state.px_per_ms;
                let selected = state.selected_note_indices.contains(&idx)
                    || state.selected_note_index == Some(idx);
                if end >= left
                    && start <= right
                    && (!has_selection || selected)
                    && note.timbre != value
                {
                    note.timbre = value.clone();
                    state.timbre_changed = true;
                    state.continuous_edit_dirty = true;
                }
            }
            state.timbre_drag = Some(pos);
        }
    }
    if !primary && !secondary && state.timbre_drag.take().is_some() {
        if state.timbre_changed {
            on_note_changed();
        }
        state.timbre_changed = false;
    }

    for (row, option) in options.iter().enumerate() {
        let y = rect.bottom() - (row as f32 + 0.5) * row_h;
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            theme.card_stroke(),
        );
        for (idx, note) in notes.iter().enumerate() {
            let x = origin_x + note.position_ms as f32 * state.px_per_ms + 5.0;
            if x < rect.left() || x > rect.right() {
                continue;
            }
            let point = Pos2::new(x, y);
            let selected = state.selected_note_indices.contains(&idx)
                || state.selected_note_index == Some(idx);
            let active = &note.timbre == option;
            let color = if selected {
                theme.accent_c32()
            } else {
                theme.text_primary_c32()
            };
            painter.circle_stroke(
                point,
                4.5,
                Stroke::new(
                    if active { 2.0 } else { 1.0 },
                    if active {
                        color
                    } else {
                        theme.text_muted_c32()
                    },
                ),
            );
            if active && option.is_some() {
                painter.circle_filled(point, 2.5, color);
            }
        }
        let label = match option.as_deref() {
            None => lang.tr("Padrão da faixa", "Track default"),
            Some("") => lang.tr("Neutro", "Neutral"),
            Some(name) => name,
        };
        let galley = painter.layout_no_wrap(
            label.to_string(),
            egui::FontId::proportional(11.0),
            theme.text_primary_c32(),
        );
        let label_pos = Pos2::new(
            rect.right() - galley.size().x - 8.0,
            y - galley.size().y * 0.5,
        );
        painter.rect_filled(
            Rect::from_min_size(label_pos, galley.size()).expand(2.0),
            Rounding::same(2.0),
            theme.bg_canvas_c32(),
        );
        painter.galley(label_pos, galley, theme.text_primary_c32());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drag_respects_selection_and_finishes_outside_panel() {
        let ctx = egui::Context::default();
        let dir = tempfile::tempdir().unwrap();
        let mut bank = Voicebank::new(dir.path()).unwrap();
        bank.prefix_map = crate::oto::PrefixMap::parse_yaml_str(
            "subbanks:\n  - color: ''\n    suffix: ''\n  - color: Soft\n    suffix: _soft\n",
        );
        let mut notes = vec![
            UNote::new("a", "C4", 100.0, 100.0),
            UNote::new("a", "C4", 300.0, 100.0),
        ];
        let mut state = PianoRollState::default();
        state.px_per_ms = 1.0;
        state.selected_note_indices.insert(0);
        let mut begins = 0;
        let mut commits = 0;
        let mut frame = |events| {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 300.0))),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        let (rect, response) = ui
                            .allocate_exact_size(Vec2::new(600.0, 120.0), Sense::click_and_drag());
                        draw(
                            ui,
                            &mut notes,
                            &mut state,
                            rect,
                            &response,
                            rect.left(),
                            Some(&bank),
                            &ThemeConfig::default(),
                            crate::config::AppLanguage::PtBr,
                            &mut || begins += 1,
                            &mut || commits += 1,
                        );
                    });
                },
            );
        };
        let button = |pos, pressed, button| egui::Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: egui::Modifiers::default(),
        };
        let start = Pos2::new(130.0, 28.0);
        frame(vec![]);
        frame(vec![egui::Event::PointerMoved(start)]);
        frame(vec![button(start, true, egui::PointerButton::Primary)]);
        frame(vec![egui::Event::PointerMoved(Pos2::new(330.0, 28.0))]);
        let outside = Pos2::new(700.0, 220.0);
        frame(vec![
            egui::Event::PointerMoved(outside),
            button(outside, false, egui::PointerButton::Primary),
        ]);
        drop(frame);
        assert_eq!((begins, commits), (1, 1));
        assert_eq!(notes[0].timbre.as_deref(), Some("Soft"));
        assert_eq!(notes[1].timbre, None);
        assert_eq!((begins, commits), (1, 1));
        assert!(state.timbre_drag.is_none());
    }
}
