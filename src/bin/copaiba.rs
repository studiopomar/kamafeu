use eframe::{egui, HardwareAcceleration, Renderer};
use kamafeu::audio::AudioPlayer;
use kamafeu::copaiba::gui::{draw_copaiba_toolkit_ui, CopaibaToolkitApp};

fn main() -> eframe::Result<()> {
    let icon_data = kamafeu::gui::window_icon::load_window_icon().ok();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("KAMUSE · Kamafeu Autoria e Modelagem de Unidades Sonoras Editáveis")
        .with_inner_size([1240.0, 780.0])
        .with_min_inner_size([920.0, 620.0]);

    if let Some(icon) = icon_data {
        viewport = viewport.with_icon(icon);
    }

    let options = eframe::NativeOptions {
        viewport,
        hardware_acceleration: HardwareAcceleration::Preferred,
        vsync: true,
        multisampling: 0,
        renderer: match std::env::var("KAMAFEU_RENDERER")
            .unwrap_or_else(|_| "auto".to_string())
            .to_ascii_lowercase()
            .as_str()
        {
            "glow" | "opengl" => Renderer::Glow,
            _ => Renderer::Wgpu,
        },
        ..Default::default()
    };

    eframe::run_native(
        "KAMUSE · Kamafeu Autoria e Modelagem de Unidades Sonoras Editáveis",
        options,
        Box::new(|cc| {
            kamafeu::gui::fonts::setup_custom_fonts(&cc.egui_ctx);
            kamafeu::copaiba::gui::configure_copaiba_theme(&cc.egui_ctx);
            Ok(Box::new(KamuseStandaloneApp::default()))
        }),
    )
}

#[derive(Default)]
pub struct KamuseStandaloneApp {
    app: CopaibaToolkitApp,
    audio_player: AudioPlayer,
}

impl eframe::App for KamuseStandaloneApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            draw_copaiba_toolkit_ui(&mut self.app, &mut self.audio_player, ui);
        });
    }
}
