use super::*;

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
    let mut pitches: Vec<String> = voicebank
        .map(|vb| vb.prefix_map.mapped_pitches().map(str::to_string).collect())
        .unwrap_or_default();
    pitches.sort_by_key(|p| crate::dsp::pitch::note_name_to_midi(p).unwrap_or(0));
    pitches.dedup();
    if pitches.is_empty() {
        pitches.push(lang.tr("Automático", "Automatic").to_string());
    }
    let row_h = rect.height() / pitches.len() as f32;
    let pointer = ui.input(|i| i.pointer.interact_pos());
    let primary = ui.input(|i| i.pointer.primary_down());
    let secondary = ui.input(|i| i.pointer.secondary_down());
    if (primary || secondary) && response.hovered() {
        if state.timbre_drag.is_none() {
            on_before_change();
        }
        state.timbre_drag = pointer;
        if let Some(pos) = pointer {
            let row = (((rect.bottom() - pos.y) / row_h).floor() as usize).min(pitches.len() - 1);
            let value = if secondary {
                None
            } else {
                Some(pitches[row].clone())
            };
            let left = pos.x - 2.0;
            for (idx, note) in notes.iter_mut().enumerate() {
                let x = origin_x + note.position_ms as f32 * state.px_per_ms;
                let selected = state.selected_note_indices.contains(&idx)
                    || state.selected_note_index == Some(idx);
                if x >= left
                    && x <= pos.x + 2.0
                    && (!selected
                        || state.selected_note_indices.is_empty()
                            && state.selected_note_index.is_none())
                {
                    note.voicebank_pitch = value.clone();
                    state.timbre_changed = true;
                }
            }
        }
    } else if state.timbre_drag.take().is_some() {
        if state.timbre_changed {
            on_note_changed();
        }
        state.timbre_changed = false;
    }
    for (row, pitch) in pitches.iter().enumerate() {
        let y = rect.bottom() - (row as f32 + 0.5) * row_h;
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            theme.card_stroke(),
        );
        for note in notes.iter() {
            let x = origin_x + note.position_ms as f32 * state.px_per_ms + 5.0;
            if rect.contains(Pos2::new(x, y)) {
                let active = note.voicebank_pitch.as_deref() == Some(pitch);
                painter.circle_stroke(
                    Pos2::new(x, y),
                    4.5,
                    Stroke::new(
                        if active { 2.0 } else { 1.0 },
                        if active {
                            theme.accent_c32()
                        } else {
                            theme.text_muted_c32()
                        },
                    ),
                );
                if active {
                    painter.circle_filled(Pos2::new(x, y), 2.5, theme.accent_c32());
                }
            }
        }
        painter.text(
            Pos2::new(rect.right() - 65.0, y),
            egui::Align2::RIGHT_CENTER,
            pitch,
            egui::FontId::proportional(10.0),
            theme.text_primary_c32(),
        );
    }
}
