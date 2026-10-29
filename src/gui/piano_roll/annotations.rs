use crate::gui::piano_roll::state::PianoRollState;
use crate::gui::types::AnnotationSubTool;
use crate::project::model::{AnnotationKind, UPianoRollAnnotation};
use eframe::egui::{self, Color32, Painter, Pos2, Rect, Rounding, Stroke, Vec2};

pub fn world_to_screen(
    time_ms: f64,
    midi_pitch: f64,
    canvas_origin: Pos2,
    keyboard_width: f32,
    timeline_scroll_x: f32,
    px_per_ms: f32,
    grid_start_y: f32,
    max_midi: u8,
    row_height: f32,
) -> Pos2 {
    let x =
        canvas_origin.x + keyboard_width + (time_ms * px_per_ms as f64) as f32 - timeline_scroll_x;
    let y = grid_start_y + ((max_midi as f64 - midi_pitch) as f32 * row_height);
    Pos2::new(x, y)
}

pub fn draw_annotations(
    painter: &Painter,
    clip_rect: Rect,
    annotations: &[UPianoRollAnnotation],
    state: &PianoRollState,
    canvas_origin: Pos2,
    keyboard_width: f32,
    timeline_scroll_x: f32,
    px_per_ms: f32,
    grid_start_y: f32,
    max_midi: u8,
    row_height: f32,
) {
    if !state.show_annotations {
        return;
    }

    let default_pen_color = Color32::from_rgba_unmultiplied(
        state.active_annotation_color[0],
        state.active_annotation_color[1],
        state.active_annotation_color[2],
        state.active_annotation_color[3],
    );

    // 1. Desenha anotações persistidas do projeto
    for ann in annotations {
        match &ann.kind {
            AnnotationKind::Drawing {
                points,
                color,
                width,
                is_highlighter,
            } => {
                if points.len() < 2 {
                    continue;
                }
                let mut screen_pts = Vec::with_capacity(points.len());
                for pt in points {
                    screen_pts.push(world_to_screen(
                        pt.0,
                        pt.1,
                        canvas_origin,
                        keyboard_width,
                        timeline_scroll_x,
                        px_per_ms,
                        grid_start_y,
                        max_midi,
                        row_height,
                    ));
                }

                if *is_highlighter {
                    let highlight_color = Color32::from_rgba_unmultiplied(
                        color[0],
                        color[1],
                        color[2],
                        ((color[3] as f32) * 0.35) as u8,
                    );
                    let stroke_width = (width * 3.5).clamp(8.0, 24.0);
                    for window in screen_pts.windows(2) {
                        painter.line_segment(
                            [window[0], window[1]],
                            Stroke::new(stroke_width, highlight_color),
                        );
                    }
                } else {
                    let pen_color =
                        Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
                    for window in screen_pts.windows(2) {
                        painter.line_segment(
                            [window[0], window[1]],
                            Stroke::new(width.max(1.0), pen_color),
                        );
                    }
                }
            }
            AnnotationKind::TextNote {
                time_ms,
                midi_pitch,
                text,
                color,
            } => {
                let anchor = world_to_screen(
                    *time_ms,
                    *midi_pitch,
                    canvas_origin,
                    keyboard_width,
                    timeline_scroll_x,
                    px_per_ms,
                    grid_start_y,
                    max_midi,
                    row_height,
                );

                if !clip_rect.expand(100.0).contains(anchor) {
                    continue;
                }

                let card_padding = Vec2::new(8.0, 6.0);
                let galley = painter.layout_no_wrap(
                    text.clone(),
                    egui::FontId::proportional(11.0),
                    Color32::from_gray(240),
                );
                let card_size = galley.size() + card_padding * 2.0;
                let card_rect = Rect::from_min_size(anchor, card_size.max(Vec2::new(60.0, 26.0)));

                // Sombra suave
                painter.rect_filled(
                    card_rect.translate(Vec2::new(1.5, 1.5)),
                    Rounding::same(5.0),
                    Color32::from_black_alpha(70),
                );
                // Fundo do Post-it / Sticky Note
                let card_bg = Color32::from_rgba_unmultiplied(
                    color[0].saturating_sub(40),
                    color[1].saturating_sub(40),
                    color[2].saturating_sub(40),
                    225,
                );
                painter.rect_filled(card_rect, Rounding::same(5.0), card_bg);
                painter.rect_stroke(
                    card_rect,
                    Rounding::same(5.0),
                    Stroke::new(1.0_f32, Color32::from_white_alpha(50)),
                );

                // Texto da anotação
                painter.galley(card_rect.min + card_padding, galley, Color32::from_gray(245));
            }
        }
    }

    // 2. Desenha o traço ativo do mouse em tempo real enquanto o usuário desenha
    if let Some(ref stroke_pts) = state.active_drawing_stroke {
        if stroke_pts.len() >= 2 {
            let color = match state.active_annotation_tool {
                AnnotationSubTool::Highlighter => Color32::from_rgba_unmultiplied(
                    state.active_annotation_color[0],
                    state.active_annotation_color[1],
                    state.active_annotation_color[2],
                    ((state.active_annotation_color[3] as f32) * 0.40) as u8,
                ),
                _ => default_pen_color,
            };
            let stroke_width = match state.active_annotation_tool {
                AnnotationSubTool::Highlighter => {
                    (state.active_annotation_width * 3.5).clamp(8.0, 24.0)
                }
                _ => state.active_annotation_width.max(1.0),
            };

            let mut screen_pts = Vec::with_capacity(stroke_pts.len());
            for pt in stroke_pts {
                screen_pts.push(world_to_screen(
                    pt.0,
                    pt.1,
                    canvas_origin,
                    keyboard_width,
                    timeline_scroll_x,
                    px_per_ms,
                    grid_start_y,
                    max_midi,
                    row_height,
                ));
            }

            for window in screen_pts.windows(2) {
                painter.line_segment([window[0], window[1]], Stroke::new(stroke_width, color));
            }
        }
    }
}
