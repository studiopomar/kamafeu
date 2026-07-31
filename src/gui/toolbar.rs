use eframe::egui::{self, Color32, RichText, Rounding, Stroke};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditTool {
    Pointer,   // Select / Move / Resize
    Pencil,    // Draw new notes
    PitchDraw, // Draw pitch curve splines
    Eraser,    // Delete notes
}

impl Default for EditTool {
    fn default() -> Self {
        EditTool::Pointer
    }
}

pub fn draw_toolbar(
    ui: &mut egui::Ui,
    current_tool: &mut EditTool,
    px_per_ms: &mut f32,
    row_height: &mut f32,
) {
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        ui.label(RichText::new("FERRAMENTAS").strong().size(10.0).color(Color32::from_rgb(180, 170, 190)));

        let tools = [
            (EditTool::Pointer, "Ponteiro [V]"),
            (EditTool::Pencil, "Lápis [N]"),
            (EditTool::PitchDraw, "Desenhar Pitch [P]"),
            (EditTool::Eraser, "Borracha [E]"),
        ];

        for (tool, label) in tools {
            let is_selected = *current_tool == tool;
            let (bg_color, stroke_color, text_color) = if is_selected {
                (
                    Color32::from_rgb(45, 38, 55),
                    Stroke::new(1.5, Color32::from_rgb(255, 215, 0)),
                    Color32::from_rgb(255, 215, 0),
                )
            } else {
                (
                    Color32::from_rgb(28, 25, 34),
                    Stroke::new(1.0, Color32::from_rgb(45, 40, 56)),
                    Color32::from_rgb(180, 175, 195),
                )
            };

            let button_widget = egui::Button::new(RichText::new(label).size(11.0).color(text_color))
                .fill(bg_color)
                .stroke(stroke_color)
                .rounding(Rounding::same(6.0));

            if ui.add(button_widget).clicked() {
                *current_tool = tool;
            }
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // Horizontal Zoom Controls (Timeline px_per_ms)
        ui.label(RichText::new("Zoom X:").size(10.0).color(Color32::from_rgb(180, 170, 190)));
        if ui.button(RichText::new("-").size(11.0)).clicked() {
            *px_per_ms = (*px_per_ms * 0.8).max(0.05);
        }
        ui.add(egui::Slider::new(px_per_ms, 0.05..=1.0).show_value(false));
        if ui.button(RichText::new("+").size(11.0)).clicked() {
            *px_per_ms = (*px_per_ms * 1.25).min(1.0);
        }

        ui.add_space(8.0);
        ui.separator();
        ui.add_space(8.0);

        // Vertical Zoom Controls (Key Row Height)
        ui.label(RichText::new("Zoom Y:").size(10.0).color(Color32::from_rgb(180, 170, 190)));
        if ui.button(RichText::new("-").size(11.0)).clicked() {
            *row_height = (*row_height * 0.85).max(12.0);
        }
        ui.add(egui::Slider::new(row_height, 12.0..=48.0).show_value(false));
        if ui.button(RichText::new("+").size(11.0)).clicked() {
            *row_height = (*row_height * 1.15).min(48.0);
        }
    });
}
