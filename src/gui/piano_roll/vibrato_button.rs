use super::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn popup_applies_preset_after_snapshot_and_can_remove_vibrato() {
        let ctx = egui::Context::default();
        let mut note = UNote::new("ka", "C4", 0.0, 500.0);
        let mut before = 0;
        let mut changed = 0;
        let mut time = 0.0;
        let icon = Rect::from_min_size(Pos2::new(100.0, 100.0), Vec2::new(18.0, 14.0));
        let mut frame = |note: &mut UNote, events: Vec<egui::Event>| {
            time += 0.05;
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0))),
                    time: Some(time),
                    events,
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        draw(
                            ui,
                            note,
                            0,
                            icon,
                            &ThemeConfig::default(),
                            crate::config::AppLanguage::PtBr,
                            &mut || before += 1,
                            &mut || changed += 1,
                        );
                    });
                },
            )
        };
        let click = |pos, pressed| {
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::default(),
                },
            ]
        };
        frame(&mut note, vec![]);
        for label in ["Pop suave", "Remover vibrato"] {
            // Click to open popup
            frame(&mut note, click(icon.center(), true));
            frame(&mut note, click(icon.center(), false));
            let output = frame(&mut note, vec![]);
            let pos = output
                .shapes
                .iter()
                .find_map(|shape| {
                    if let egui::epaint::Shape::Text(text) = &shape.shape {
                        if text.galley.text().contains(label) {
                            return Some(text.pos + text.galley.size() * 0.5);
                        }
                    }
                    None
                })
                .expect("preset menu is visible");
            frame(&mut note, click(pos, true));
            frame(&mut note, click(pos, false));
            assert_eq!(
                note.vibrato.length_pct,
                if label == "Pop suave" { 65.0 } else { 0.0 }
            );
            assert_eq!(note.position_ms, 0.0);
            assert_eq!(note.duration_ms, 500.0);
        }
        assert_eq!(before, 2);
        assert_eq!(changed, 2);
    }
}

pub(super) fn rect(note: &UNote, state: &PianoRollState, origin: Pos2) -> Rect {
    let x = origin.x + ((note.position_ms + note.duration_ms) * state.px_per_ms as f64) as f32;
    let y = origin.y + f32::from(state.max_midi.saturating_sub(note.midi_key())) * state.row_height;
    Rect::from_min_size(Pos2::new(x - 18.0, y - 15.0), Vec2::new(18.0, 14.0))
}

pub(super) fn draw(
    ui: &mut egui::Ui,
    note: &mut UNote,
    index: usize,
    rect: Rect,
    theme: &ThemeConfig,
    lang: crate::config::AppLanguage,
    on_before_change: &mut dyn FnMut(),
    on_note_changed: &mut dyn FnMut(),
) {
    let id = ui.make_persistent_id(("note_vibrato", index));
    let response = ui
        .interact(rect, id, Sense::click())
        .on_hover_text(lang.tr("Presets de vibrato", "Vibrato presets"));
    let popup_id = id.with("presets");
    let active = note.vibrato.length_pct > 0.0 && note.vibrato.depth_cents > 0.0;
    let color = if active || response.hovered() {
        theme.accent_c32()
    } else {
        Color32::from_gray(190)
    };
    ui.painter()
        .rect_filled(rect, Rounding::same(3.0), theme.bg_panel_c32());
    let points: Vec<_> = (0..17)
        .map(|i| {
            let t = i as f32 / 16.0;
            Pos2::new(
                rect.left() + 3.0 + t * 12.0,
                rect.center().y - (t * std::f32::consts::TAU * 1.5).sin() * 3.0,
            )
        })
        .collect();
    ui.painter()
        .add(egui::Shape::line(points, Stroke::new(1.3, color)));

    if response.clicked() {
        ui.memory_mut(|memory| memory.toggle_popup(popup_id));
    }

    egui::popup_below_widget(
        ui,
        popup_id,
        &response,
        egui::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_min_width(170.0);
            ui.set_max_width(220.0);

            // Header styling
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(lang.tr("Vibrato", "Vibrato"))
                        .strong()
                        .size(12.0)
                        .color(theme.accent_c32()),
                );
            });
            ui.add_space(2.0);
            ui.separator();
            ui.add_space(3.0);

            // Capture the project on press, before the click applies a preset on release.
            if ui.input(|input| input.pointer.primary_pressed()) {
                on_before_change();
            }

            let presets = [
                (
                    lang.tr("Pop suave", "Soft Pop"),
                    65.0,
                    48.0,
                    175.0,
                    25.0,
                    15.0,
                    "65% • 48c",
                ),
                (
                    lang.tr("Dramático", "Dramatic"),
                    75.0,
                    75.0,
                    160.0,
                    20.0,
                    10.0,
                    "75% • 75c",
                ),
                (
                    lang.tr("Balada", "Ballad"),
                    80.0,
                    50.0,
                    220.0,
                    35.0,
                    15.0,
                    "80% • 50c",
                ),
                (
                    lang.tr("Rápido", "Fast"),
                    60.0,
                    60.0,
                    140.0,
                    20.0,
                    10.0,
                    "60% • 60c",
                ),
            ];

            for (display_label, length, depth, period, fade_in, fade_out, desc) in presets {
                let is_current = (note.vibrato.length_pct - length).abs() < 0.1
                    && (note.vibrato.depth_cents - depth).abs() < 0.1
                    && (note.vibrato.period_ms - period).abs() < 0.1;

                let text = egui::RichText::new(display_label)
                    .size(11.5)
                    .color(if is_current {
                        theme.accent_c32()
                    } else {
                        theme.text_primary_c32()
                    });

                let btn = egui::Button::new(text)
                    .min_size(Vec2::new(ui.available_width(), 22.0))
                    .wrap_mode(egui::TextWrapMode::Extend);

                let period_label = lang.tr("Período", "Period");
                let btn_resp = ui
                    .add(btn)
                    .on_hover_text(format!("{} ({}: {:.0}ms)", desc, period_label, period));
                if btn_resp.clicked() {
                    let vibrato = crate::dsp::pitch::VibratoParam {
                        length_pct: length,
                        depth_cents: depth,
                        period_ms: period,
                        fade_in_pct: fade_in,
                        fade_out_pct: fade_out,
                        ..Default::default()
                    };
                    if note.vibrato != vibrato {
                        note.vibrato = vibrato;
                        on_note_changed();
                    }
                    ui.memory_mut(|memory| memory.close_popup());
                }
                ui.add_space(1.0);
            }

            ui.add_space(2.0);
            ui.separator();
            ui.add_space(2.0);

            // Remover vibrato
            let is_empty = note.vibrato.length_pct == 0.0 || note.vibrato.depth_cents == 0.0;
            let remove_text = egui::RichText::new(lang.tr("Remover vibrato", "Remove vibrato"))
                .size(11.0)
                .color(if is_empty {
                    theme.text_muted_c32()
                } else {
                    Color32::from_rgb(235, 100, 100)
                });

            let remove_btn = egui::Button::new(remove_text)
                .min_size(Vec2::new(ui.available_width(), 20.0))
                .wrap_mode(egui::TextWrapMode::Extend);

            if ui.add(remove_btn).clicked() {
                let vibrato = crate::dsp::pitch::VibratoParam {
                    length_pct: 0.0,
                    depth_cents: 0.0,
                    period_ms: 175.0,
                    fade_in_pct: 0.0,
                    fade_out_pct: 0.0,
                    ..Default::default()
                };
                if note.vibrato != vibrato {
                    note.vibrato = vibrato;
                    on_note_changed();
                }
                ui.memory_mut(|memory| memory.close_popup());
            }
        },
    );
}
