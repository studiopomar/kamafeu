use crate::gui::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(super) fn show_batch_lyrics(&mut self, ctx: &egui::Context) {
        if self.batch_lyrics_open {
            let mut is_open = self.batch_lyrics_open;
            let mut apply_lyrics = false;
            let mut close_modal = false;
            let lang = self.config.language;

            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("batch_lyrics_native_viewport"),
                egui::ViewportBuilder::default()
                    .with_title(format!(
                        "{} - Kamafeu Studio",
                        lang.tr("Inserir Letras em Lote (Batch Lyrics)", "Insert Batch Lyrics")
                    ))
                    .with_inner_size([480.0, 260.0])
                    .with_min_inner_size([380.0, 200.0]),
                |ctx, _class| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.heading(
                            egui::RichText::new(lang.tr("Inserir Letras em Lote", "Insert Batch Lyrics"))
                                .strong()
                                .color(egui::Color32::from_rgb(0, 255, 180)),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            lang.tr(
                                "Cole ou digite o texto com palavras ou sílabas separadas por espaço:",
                                "Paste or type text with words or syllables separated by spaces:"
                            ),
                        );
                        ui.add_space(4.0);
                        ui.add(
                            egui::TextEdit::multiline(&mut self.batch_lyrics_buffer)
                                .desired_rows(6)
                                .desired_width(f32::INFINITY)
                                .hint_text(lang.tr("ex: quem te viu quem te ve", "e.g.: do re mi fa sol")),
                        );
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button(lang.tr("Distribuir Letras pelas Notas", "Distribute Lyrics to Notes")).clicked() {
                                apply_lyrics = true;
                            }
                            if ui.button(lang.tr("Cancelar", "Cancel")).clicked() {
                                close_modal = true;
                            }
                        });
                    });
                    if ctx.input(|i| i.viewport().close_requested()) {
                        close_modal = true;
                    }
                },
            );

            if apply_lyrics {
                let syllables: Vec<String> = self
                    .batch_lyrics_buffer
                    .split_whitespace()
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();

                if !syllables.is_empty() {
                    let undo_before = self.project.clone();
                    self.undo_manager.push_state(undo_before);

                    let sel = self.piano_roll_state.selected_note_indices.clone();
                    let notes = self.current_notes_mut();
                    let target_indices: Vec<usize> = if !sel.is_empty() {
                        let mut v: Vec<usize> = sel.into_iter().collect();
                        v.sort_by(|&a, &b| {
                            notes
                                .get(a)
                                .map(|n| n.position_ms)
                                .unwrap_or(0.0)
                                .partial_cmp(&notes.get(b).map(|n| n.position_ms).unwrap_or(0.0))
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                        v
                    } else {
                        (0..notes.len()).collect()
                    };

                    for (i, &idx) in target_indices.iter().enumerate() {
                        if let Some(syl) = syllables.get(i) {
                            if let Some(note) = notes.get_mut(idx) {
                                note.lyric = syl.clone();
                            }
                        }
                    }
                    self.piano_roll_state.phoneme_cache.clear();
                    self.is_dirty = true;
                    self.transport_state.status_message = format!(
                        "{} {} {}",
                        lang.tr("Letras distribuídas por", "Lyrics distributed across"),
                        syllables.len().min(target_indices.len()),
                        lang.tr("notas", "notes")
                    );
                    close_modal = true;
                }
            }

            if close_modal {
                is_open = false;
            }
            self.batch_lyrics_open = is_open;
        }
    }
}
