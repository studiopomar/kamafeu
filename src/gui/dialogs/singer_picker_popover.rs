use crate::gui::theme::ThemeConfig;
use crate::oto::SingerInfo;
use eframe::egui::{self, Color32, Rect, Rounding, Stroke, Vec2};

pub struct SingerPickerPopover;

impl SingerPickerPopover {
    /// Renders a fast, user-friendly singer picker list with search and avatars.
    /// Returns Some(selected_singer_name) if a singer was picked.
    pub fn show(
        ui: &mut egui::Ui,
        theme: &ThemeConfig,
        lang: crate::config::AppLanguage,
        search_query: &mut String,
        singers: &[SingerInfo],
        current_singer: &str,
        on_preview_singer: &mut dyn FnMut(&SingerInfo),
    ) -> Option<String> {
        let mut selected = None;

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(lang.tr("Buscar cantor:", "Search singer:"))
                    .size(11.0)
                    .color(theme.text_muted_c32()),
            );
            ui.add(
                egui::TextEdit::singleline(search_query)
                    .hint_text(lang.tr("Digite o nome ou autor...", "Type name or author..."))
                    .desired_width(ui.available_width()),
            );
        });

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        let query = search_query.to_lowercase();
        let filtered: Vec<&SingerInfo> = singers
            .iter()
            .filter(|s| {
                query.is_empty()
                    || s.name.to_lowercase().contains(&query)
                    || s.author.to_lowercase().contains(&query)
                    || s.voice_type.to_lowercase().contains(&query)
            })
            .collect();

        egui::ScrollArea::vertical()
            .id_salt("fast_singer_picker_scroll")
            .max_height(260.0)
            .show(ui, |ui| {
                if filtered.is_empty() {
                    ui.vertical_centered(|ui| {
                        ui.add_space(16.0);
                        ui.label(
                            egui::RichText::new(lang.tr(
                                "Nenhum cantor encontrado para esta busca.",
                                "No singer found for this search.",
                            ))
                            .size(11.0)
                            .color(theme.text_muted_c32()),
                        );
                    });
                } else {
                    for singer in filtered {
                        let is_current = singer.name == current_singer;
                        let item_h = 38.0;
                        let (rect, resp) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), item_h),
                            egui::Sense::click(),
                        );

                        let bg = if is_current {
                            theme.c32_alpha(theme.accent_color, 0.20)
                        } else if resp.hovered() {
                            theme.c32_alpha(theme.text_primary, 0.08)
                        } else {
                            Color32::TRANSPARENT
                        };
                        ui.painter().rect_filled(rect, Rounding::same(4.0), bg);

                        // Draw avatar thumbnail
                        let avatar_size = 28.0;
                        let avatar_center = egui::pos2(rect.left() + 18.0, rect.center().y);
                        let avatar_rect =
                            Rect::from_center_size(avatar_center, Vec2::splat(avatar_size));

                        let mut painted_img = false;
                        if let Some(img_path) = singer.image_path.as_ref() {
                            if let Some(texture) =
                                crate::gui::image_cache::texture_for_path(ui.ctx(), img_path)
                            {
                                ui.painter().rect_filled(
                                    avatar_rect,
                                    Rounding::same(avatar_size * 0.5),
                                    Color32::BLACK,
                                );
                                ui.painter().image(
                                    texture.id(),
                                    avatar_rect,
                                    Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                                    Color32::WHITE,
                                );
                                ui.painter().rect_stroke(
                                    avatar_rect,
                                    Rounding::same(avatar_size * 0.5),
                                    Stroke::new(1.0_f32, theme.accent_c32()),
                                );
                                painted_img = true;
                            }
                        }

                        if !painted_img {
                            ui.painter().circle_filled(
                                avatar_center,
                                avatar_size * 0.5,
                                theme.c32_alpha(theme.accent_color, 0.35),
                            );
                            let initial = singer
                                .name
                                .chars()
                                .next()
                                .unwrap_or('?')
                                .to_uppercase()
                                .to_string();
                            ui.painter().text(
                                avatar_center,
                                egui::Align2::CENTER_CENTER,
                                initial,
                                egui::FontId::proportional(12.0),
                                Color32::WHITE,
                            );
                        }

                        // Labels: Name, Type, Author
                        let text_left = rect.left() + 38.0;
                        let name_color = if is_current {
                            theme.accent_c32()
                        } else {
                            theme.text_primary_c32()
                        };

                        ui.painter().text(
                            egui::pos2(text_left, rect.top() + 6.0),
                            egui::Align2::LEFT_TOP,
                            &singer.name,
                            egui::FontId::proportional(11.5),
                            name_color,
                        );

                        let info_text = format!(
                            "{} • {}",
                            if singer.voice_type.is_empty() {
                                "UTAU"
                            } else {
                                &singer.voice_type
                            },
                            if singer.author.is_empty() {
                                "Autor desconhecido"
                            } else {
                                &singer.author
                            }
                        );
                        ui.painter().text(
                            egui::pos2(text_left, rect.top() + 20.0),
                            egui::Align2::LEFT_TOP,
                            info_text,
                            egui::FontId::proportional(9.0),
                            theme.text_muted_c32(),
                        );

                        // Preview button on right
                        let preview_rect = Rect::from_min_max(
                            egui::pos2(rect.right() - 28.0, rect.top() + 8.0),
                            egui::pos2(rect.right() - 6.0, rect.bottom() - 8.0),
                        );
                        let preview_resp = ui.allocate_rect(preview_rect, egui::Sense::click());
                        if preview_resp.hovered() {
                            ui.painter().rect_filled(
                                preview_rect,
                                Rounding::same(3.0),
                                theme.c32_alpha(theme.accent_color, 0.25),
                            );
                        }
                        ui.painter().text(
                            preview_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "▶",
                            egui::FontId::proportional(9.0),
                            theme.text_primary_c32(),
                        );
                        if preview_resp.clicked() {
                            on_preview_singer(singer);
                        }

                        if resp.clicked() && !preview_resp.clicked() {
                            selected = Some(singer.name.clone());
                        }
                    }
                }
            });

        selected
    }
}
