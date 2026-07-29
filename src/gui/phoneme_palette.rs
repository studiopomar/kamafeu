use eframe::egui::{self, Color32, Rect, RichText, Rounding, Stroke, Vec2};
use crate::gui::theme::MelodyneTheme;
use crate::oto::Voicebank;

pub struct PhonemePaletteState {
    pub search_query: String,
    pub selected_folder: String,
    pub dragged_phoneme: Option<String>,
}

impl Default for PhonemePaletteState {
    fn default() -> Self {
        Self {
            search_query: String::new(),
            selected_folder: "All Folders".to_string(),
            dragged_phoneme: None,
        }
    }
}

pub fn draw_phoneme_palette(
    ui: &mut egui::Ui,
    voicebank: Option<&Voicebank>,
    state: &mut PhonemePaletteState,
    on_preview_phoneme: &mut dyn FnMut(&str),
    on_insert_phoneme: &mut dyn FnMut(&str),
) {
    ui.vertical(|ui| {
        ui.add_space(4.0);
        ui.heading(RichText::new("🔤 Phonemes (oto.ini)").strong().size(14.0).color(MelodyneTheme::TEXT_GOLD_LABEL));
        ui.add_space(6.0);

        if let Some(vb) = voicebank {
            // Search Bar Widget
            ui.horizontal(|ui| {
                ui.label(RichText::new("🔍").size(12.0));
                ui.add(
                    egui::TextEdit::singleline(&mut state.search_query)
                        .hint_text("Search alias (e.g. ka, a ka, CV)...")
                        .desired_width(ui.available_width() - 30.0),
                );
            });

            ui.add_space(4.0);

            // Subfolder Filter Dropdown
            let subfolders = vb.get_subfolders();
            ui.horizontal(|ui| {
                ui.label(RichText::new("Folder:").size(11.0).color(MelodyneTheme::TEXT_MUTED));
                egui::ComboBox::from_id_salt("phoneme_folder_combo")
                    .selected_text(&state.selected_folder)
                    .show_ui(ui, |ui| {
                        for folder in &subfolders {
                            ui.selectable_value(&mut state.selected_folder, folder.clone(), folder);
                        }
                    });
            });

            ui.add_space(6.0);
            ui.separator();
            ui.add_space(4.0);

            let mut matches = vb.search_entries(&state.search_query, &state.selected_folder);
            matches.sort_by(|a, b| a.0.cmp(b.0));

            ui.label(RichText::new(format!("Available Phonemes ({})", matches.len())).size(11.0).color(MelodyneTheme::TEXT_MUTED));
            ui.add_space(4.0);

            // Phoneme Cards Grid (Melodyne Gold Buttons with Drag & Drop)
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);

                    for (alias, entry) in matches {
                        let (btn_rect, response) = ui.allocate_exact_size(Vec2::new(62.0, 32.0), egui::Sense::click_and_drag());

                        let is_hovered = response.hovered();
                        let is_being_dragged = response.dragged() || state.dragged_phoneme.as_deref() == Some(alias);

                        let bg_color = if is_being_dragged {
                            MelodyneTheme::NOTE_SELECTED_GOLD
                        } else if is_hovered {
                            MelodyneTheme::NOTE_GOLD_HOVER
                        } else {
                            MelodyneTheme::BG_PANEL
                        };

                        ui.painter().rect_filled(btn_rect, Rounding::same(4.0), bg_color);
                        ui.painter().rect_stroke(
                            btn_rect,
                            Rounding::same(4.0),
                            Stroke::new(1.0, if is_hovered || is_being_dragged { MelodyneTheme::NOTE_SELECTED_GOLD } else { MelodyneTheme::GRID_LINE_BAR }),
                        );

                        // Phoneme Alias Name Label
                        ui.painter().text(
                            btn_rect.min + Vec2::new(6.0, 4.0),
                            egui::Align2::LEFT_TOP,
                            alias,
                            egui::FontId::proportional(12.0),
                            if is_hovered || is_being_dragged { MelodyneTheme::TEXT_NOTE_TAG } else { MelodyneTheme::TEXT_GOLD_LABEL },
                        );

                        // Preutterance Badge
                        ui.painter().text(
                            btn_rect.max - Vec2::new(6.0, 4.0),
                            egui::Align2::RIGHT_BOTTOM,
                            format!("{:.0}ms", entry.preutterance),
                            egui::FontId::proportional(9.0),
                            MelodyneTheme::ACCENT_GOLD,
                        );

                        if response.drag_started() {
                            state.dragged_phoneme = Some(alias.to_string());
                        }

                        if response.double_clicked() {
                            on_insert_phoneme(alias);
                        } else if response.clicked() && !response.dragged() {
                            on_preview_phoneme(alias);
                        }
                    }
                });
            });

            // Render Floating Drag Badge Following Mouse Cursor
            if let Some(ref dragged_alias) = state.dragged_phoneme {
                if ui.input(|i| i.pointer.primary_down()) {
                    if let Some(mpos) = ui.ctx().pointer_latest_pos() {
                        let badge_rect = Rect::from_min_size(mpos + Vec2::new(12.0, 12.0), Vec2::new(70.0, 26.0));
                        let painter = ui.painter();
                        painter.rect_filled(badge_rect, Rounding::same(4.0), MelodyneTheme::NOTE_SELECTED_GOLD);
                        painter.rect_stroke(badge_rect, Rounding::same(4.0), Stroke::new(1.5, Color32::WHITE));
                        painter.text(
                            badge_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("🔤 {}", dragged_alias),
                            egui::FontId::proportional(12.0),
                            MelodyneTheme::TEXT_NOTE_TAG,
                        );
                    }
                } else {
                    // Mouse button released -> clear drag state
                    state.dragged_phoneme = None;
                }
            }
        } else {
            ui.label(RichText::new("No voicebank loaded. Load a voicebank to view phonemes.").italics().color(MelodyneTheme::TEXT_MUTED));
        }
    });
}
