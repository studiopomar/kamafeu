use eframe::egui::{self, Color32, RichText};

#[derive(Clone, Copy)]
pub(super) enum Art {
    Plugins,
    Vocal,
    Pitch,
    Export,
    Diagnostics,
}

fn svg(art: Art) -> &'static [u8] {
    match art {
        Art::Plugins => include_bytes!("../../../assets/dialogs/plugins.svg"),
        Art::Vocal => include_bytes!("../../../assets/dialogs/vocal.svg"),
        Art::Pitch => include_bytes!("../../../assets/dialogs/pitch.svg"),
        Art::Export => include_bytes!("../../../assets/dialogs/export.svg"),
        Art::Diagnostics => include_bytes!("../../../assets/dialogs/diagnostics.svg"),
    }
}

pub(super) fn header(ui: &mut egui::Ui, art: Art, title: &str, subtitle: &str, accent: Color32) {
    egui::Frame::group(ui.style())
        .fill(ui.visuals().extreme_bg_color)
        .stroke(egui::Stroke::new(
            1.0_f32,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .rounding(egui::Rounding::same(10.0))
        .inner_margin(egui::Margin::same(12.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    egui::Image::from_bytes(
                        format!("bytes://kamafeu-dialog-{}.svg", title.replace(' ', "-")),
                        svg(art),
                    )
                    .fit_to_exact_size(egui::vec2(68.0, 46.0)),
                );
                ui.vertical(|ui| {
                    ui.label(RichText::new(title).strong().size(17.0).color(accent));
                    ui.label(
                        RichText::new(subtitle)
                            .size(11.0)
                            .color(ui.visuals().text_color()),
                    );
                });
            });
        });
}

pub(super) fn card<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::group(ui.style())
        .fill(ui.visuals().panel_fill)
        .stroke(egui::Stroke::new(
            1.0_f32,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .rounding(egui::Rounding::same(8.0))
        .inner_margin(egui::Margin::same(10.0))
        .show(ui, add_contents)
        .inner
}
