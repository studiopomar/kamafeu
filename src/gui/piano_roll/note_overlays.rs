use super::vibrato_button;
use super::ThemeConfig;
use crate::project::model::UNote;
use eframe::egui::{self, Color32, Painter, Pos2, Rect, Rounding, Stroke, Vec2};

pub(super) struct Context<'a> {
    pub(super) ui: &'a mut egui::Ui,
    pub(super) painter: &'a Painter,
    pub(super) theme: &'a ThemeConfig,
    pub(super) notes: &'a mut [UNote],
    pub(super) lang: crate::config::AppLanguage,
    pub(super) on_before_change: &'a mut dyn FnMut(),
    pub(super) on_note_changed: &'a mut dyn FnMut(),
}

pub(super) struct Pending {
    pub(super) lyric_tags: Vec<(Rect, Color32, String, Color32, bool, Rect)>,
    pub(super) phoneme_badges: Vec<(Rect, String, bool, Rect, bool)>,
    pub(super) mode_badges: Vec<(Rect, String)>,
    pub(super) vibrato_buttons: Vec<(usize, Rect)>,
}

pub(super) fn draw(context: Context<'_>, pending: Pending) {
    let Context {
        ui,
        painter,
        theme,
        notes,
        lang,
        on_before_change,
        on_note_changed,
    } = context;
    let Pending {
        lyric_tags: pending_lyric_tags,
        phoneme_badges: pending_phoneme_badges,
        mode_badges: pending_mode_badges,
        vibrato_buttons,
    } = pending;
    for (_pill_rect, pill_bg, lyric, text_color, is_selected, note_rect) in pending_lyric_tags {
        if note_rect.width() < 10.0 {
            continue;
        }

        let clipped = painter.with_clip_rect(note_rect);
        let pad_x = 5.0;

        // Tipografia limpa, nítida e legível em notas de qualquer tamanho
        let font_size = if note_rect.height() < 16.0 {
            10.5
        } else if note_rect.height() < 22.0 {
            11.5
        } else {
            12.5
        };
        let font = egui::FontId::proportional(font_size);

        // Calcula o tamanho do texto para criar uma cápsula/badge suave de fundo
        let text_galley = painter.layout_no_wrap(lyric.clone(), font.clone(), text_color);
        let text_size = text_galley.size();

        let badge_h = (text_size.y + 2.0).min(note_rect.height() - 4.0).max(12.0);
        let badge_w = (text_size.x + 8.0)
            .min(note_rect.width() - pad_x * 2.0)
            .max(14.0);
        let badge_y = note_rect.center().y - badge_h * 0.5;
        let badge_rect = Rect::from_min_size(
            Pos2::new(note_rect.min.x + pad_x, badge_y),
            Vec2::new(badge_w, badge_h),
        );

        // Superfície translúcida baseada no tema, sem caixa preta rígida.
        let badge_bg = pill_bg;
        let badge_stroke = if is_selected {
            Stroke::new(1.0_f32, theme.note_stroke_c32().linear_multiply(0.72))
        } else {
            Stroke::new(
                0.8_f32,
                theme.note_stroke(false).color.linear_multiply(0.55),
            )
        };

        clipped.rect_filled(badge_rect, Rounding::same(3.5), badge_bg);
        clipped.rect_stroke(badge_rect, Rounding::same(3.5), badge_stroke);

        let text_pos = Pos2::new(badge_rect.min.x + 4.0, badge_rect.center().y);

        let final_text_color = if is_selected {
            theme.text_note_tag_c32().linear_multiply(0.82)
        } else {
            text_color
        };

        // Texto com renderização precisa dentro da badge
        clipped.text(
            text_pos,
            egui::Align2::LEFT_CENTER,
            &lyric,
            font,
            final_text_color,
        );
    }

    for (badge_rect, phoneme, integrated, note_rect, _is_selected) in pending_phoneme_badges {
        if integrated {
            // The resolved phoneme lives above the note, SynthV-style,
            // so selecting a note cannot make it collide with the lyric.
            let badge_bg = theme.c32_alpha(theme.bg_panel, 0.82);
            painter.rect_filled(badge_rect, Rounding::same(3.0), badge_bg);
            painter.rect_stroke(
                badge_rect,
                Rounding::same(3.0),
                Stroke::new(0.7_f32, theme.note_stroke_c32().linear_multiply(0.55)),
            );
            painter.with_clip_rect(badge_rect).text(
                Pos2::new(badge_rect.left() + 4.0, badge_rect.center().y),
                egui::Align2::LEFT_CENTER,
                phoneme,
                egui::FontId::proportional(9.0),
                theme.text_note_tag_c32().linear_multiply(0.82),
            );
        } else {
            painter.circle_filled(
                badge_rect.center(),
                2.5,
                theme.note_stroke_c32().linear_multiply(0.72),
            );
            ui.interact(
                badge_rect,
                ui.make_persistent_id((
                    "phoneme_marker",
                    note_rect.min.x.to_bits(),
                    note_rect.min.y.to_bits(),
                )),
                egui::Sense::hover(),
            )
            .on_hover_text(format!("Fonema: {phoneme}"));
        }
    }

    for (badge_rect, mode) in pending_mode_badges {
        painter.rect_filled(
            badge_rect,
            Rounding::same(6.0),
            theme.c32_alpha(theme.accent_color, 170.0 / 255.0),
        );
        painter.text(
            badge_rect.center(),
            egui::Align2::CENTER_CENTER,
            "ƒ",
            egui::FontId::proportional(10.0),
            theme.text_primary_c32(),
        );
        ui.interact(
            badge_rect,
            ui.make_persistent_id(("phonemizer_mode_badge", &mode)),
            egui::Sense::hover(),
        )
        .on_hover_text(format!("Fonemizador: {mode}"));
    }

    for (index, button_rect) in vibrato_buttons {
        if let Some(note) = notes.get_mut(index) {
            vibrato_button::draw(
                ui,
                note,
                index,
                button_rect,
                theme,
                lang,
                on_before_change,
                on_note_changed,
            );
        }
    }
}
