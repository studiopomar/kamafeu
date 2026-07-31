use eframe::egui;
use kamafeu::copaiba::gui::{CopaibaToolkitApp, draw_copaiba_toolkit_ui};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Copaiba Voicebank Toolkit - Kamafeu Synthesizer")
            .with_inner_size([1100.0, 680.0])
            .with_min_inner_size([850.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Copaiba Voicebank Toolkit",
        options,
        Box::new(|_cc| Ok(Box::new(CopaibaStandaloneApp::default()))),
    )
}

pub struct CopaibaStandaloneApp {
    app: CopaibaToolkitApp,
}

impl Default for CopaibaStandaloneApp {
    fn default() -> Self {
        Self {
            app: CopaibaToolkitApp::default(),
        }
    }
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
