use eframe::egui;
use kamafeu::copaiba::gui::{draw_copaiba_toolkit_ui, CopaibaToolkitApp};

fn main() -> eframe::Result<()> {
    let icon_data = kamafeu::gui::window_icon::load_window_icon().ok();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Copaiba Voicebank Toolkit - Kamafeu Synthesizer")
        .with_inner_size([1100.0, 680.0])
        .with_min_inner_size([850.0, 500.0]);

    if let Some(icon) = icon_data {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Copaiba Voicebank Toolkit",
        options,
        Box::new(|_cc| Ok(Box::new(CopaibaStandaloneApp::default()))),
    )
}

#[derive(Default)]
pub struct CopaibaStandaloneApp {
    app: CopaibaToolkitApp,
}

impl eframe::App for CopaibaStandaloneApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = egui::Color32::from_rgb(22, 16, 32);
        visuals.window_fill = egui::Color32::from_rgb(22, 16, 32);
        ctx.set_visuals(visuals);

        egui::CentralPanel::default().show(ctx, |ui| {
            draw_copaiba_toolkit_ui(&mut self.app, ui);
        });
    }
}
