use super::*;

pub(super) fn draw(
    ui: &mut egui::Ui,
    notes: &mut [UNote],
    state: &mut PianoRollState,
    lang: crate::config::AppLanguage,
    on_note_changed: &mut dyn FnMut(),
) {
    if let Some(prop_idx) = state.properties_window_for_note {
        if prop_idx < notes.len() {
            let mut close_window = false;
            let mut modal_note_changed = false;
            let note = &mut notes[prop_idx];
            ui.ctx().show_viewport_immediate(
                egui::ViewportId::from_hash_of("note_properties_native_viewport"),
                egui::ViewportBuilder::default()
                    .with_title(format!(
                        "{} [{}] - Kamafeu Studio",
                        lang.tr("Propriedades da Nota", "Note Properties"),
                        note.pitch
                    ))
                    .with_inner_size([340.0, 430.0])
                    .with_min_inner_size([300.0, 350.0]),
                |ctx, _class| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.heading(
                            egui::RichText::new(format!(
                                "{}: {} ({})",
                                lang.tr("Nota", "Note"),
                                note.lyric,
                                note.pitch
                            ))
                            .size(14.0)
                            .color(MelodyneTheme::TEXT_GOLD_LABEL),
                        );
                        ui.separator();

                        let mut prop_changed = false;
                        egui::Grid::new("prop_grid")
                            .num_columns(2)
                            .spacing([12.0, 8.0])
                            .show(ui, |ui| {
                                ui.label(lang.tr("Letra (Lyric):", "Lyric:"));
                                if ui.text_edit_singleline(&mut note.lyric).changed() {
                                    prop_changed = true;
                                }
                                ui.end_row();

                                ui.label(lang.tr("Duração (ms):", "Duration (ms):"));
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut note.duration_ms)
                                            .speed(1.0)
                                            .range(20.0..=10000.0)
                                            .suffix(" ms"),
                                    )
                                    .changed()
                                {
                                    prop_changed = true;
                                }
                                ui.end_row();

                                ui.label(lang.tr("Vel. da Consoante:", "Consonant Velocity:"));
                                if ui
                                    .add(
                                        egui::DragValue::new(
                                            &mut note.expressions.consonant_velocity,
                                        )
                                        .speed(1.0)
                                        .range(-100.0..=200.0)
                                        .suffix(" %"),
                                    )
                                    .changed()
                                {
                                    prop_changed = true;
                                }
                                ui.end_row();

                                ui.label(lang.tr("Modulação (MOD):", "Modulation (MOD):"));
                                if ui
                                    .add(
                                        egui::DragValue::new(&mut note.expressions.modulation)
                                            .speed(1.0)
                                            .range(0.0..=200.0)
                                            .suffix(" %"),
                                    )
                                    .changed()
                                {
                                    prop_changed = true;
                                }
                                ui.end_row();
                            });

                        ui.add_space(8.0);
                        ui.label(
                            egui::RichText::new(
                                lang.tr("Parâmetros de Vibrato", "Vibrato Parameters"),
                            )
                            .strong()
                            .color(Color32::from_rgb(0, 240, 255)),
                        );

                        egui::Grid::new("vibrato_grid")
                            .num_columns(2)
                            .spacing([12.0, 8.0])
                            .show(ui, |ui| {
                                ui.label(lang.tr("Comprimento (%):", "Length (%):"));
                                if ui
                                    .add(
                                        egui::Slider::new(
                                            &mut note.vibrato.length_pct,
                                            0.0..=100.0,
                                        )
                                        .suffix(" %"),
                                    )
                                    .changed()
                                {
                                    prop_changed = true;
                                }
                                ui.end_row();

                                ui.label(lang.tr("Profundidade (cents):", "Depth (cents):"));
                                if ui
                                    .add(
                                        egui::Slider::new(
                                            &mut note.vibrato.depth_cents,
                                            0.0..=200.0,
                                        )
                                        .suffix(" c"),
                                    )
                                    .changed()
                                {
                                    prop_changed = true;
                                }
                                ui.end_row();

                                ui.label(lang.tr("Período (ms):", "Period (ms):"));
                                if ui
                                    .add(
                                        egui::Slider::new(
                                            &mut note.vibrato.period_ms,
                                            50.0..=450.0,
                                        )
                                        .suffix(" ms"),
                                    )
                                    .changed()
                                {
                                    prop_changed = true;
                                }
                                ui.end_row();

                                ui.label(lang.tr("Fade In (%):", "Fade In (%):"));
                                if ui
                                    .add(
                                        egui::Slider::new(
                                            &mut note.vibrato.fade_in_pct,
                                            0.0..=100.0,
                                        )
                                        .suffix(" %"),
                                    )
                                    .changed()
                                {
                                    prop_changed = true;
                                }
                                ui.end_row();

                                ui.label(lang.tr("Fade Out (%):", "Fade Out (%):"));
                                if ui
                                    .add(
                                        egui::Slider::new(
                                            &mut note.vibrato.fade_out_pct,
                                            0.0..=100.0,
                                        )
                                        .suffix(" %"),
                                    )
                                    .changed()
                                {
                                    prop_changed = true;
                                }
                                ui.end_row();
                            });

                        if prop_changed {
                            modal_note_changed = true;
                        }

                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            if ui.button("OK").clicked() {
                                close_window = true;
                            }
                        });
                    });
                    if ctx.input(|i| i.viewport().close_requested()) {
                        close_window = true;
                    }
                },
            );

            if modal_note_changed {
                state.continuous_edit_dirty = true;
                on_note_changed();
            }

            if close_window {
                state.properties_window_for_note = None;
            }
        } else {
            state.properties_window_for_note = None;
        }
    }
}
