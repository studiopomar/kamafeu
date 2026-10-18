use super::*;
use eframe::{HardwareAcceleration, NativeOptions, Renderer};

pub(super) fn run_gui() -> Result<(), eframe::Error> {
    kamafeu::terminal_log::banner(kamafeu::APP_VERSION, kamafeu::APP_CODENAME);
    kamafeu::drivers::process::prewarm_wine_in_background();
    let icon_data = kamafeu::gui::window_icon::load_window_icon().ok();
    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_title(format!(
            "Kamafeu Studio v{} (Calafate)",
            kamafeu::APP_VERSION
        ))
        .with_inner_size([1280.0, 750.0])
        .with_min_inner_size([800.0, 500.0])
        .with_maximized(true);

    if let Some(icon) = icon_data {
        viewport = viewport.with_icon(icon);
    }

    let renderer = requested_renderer();
    let fallback_viewport = viewport.clone();
    kamafeu::terminal_log::info(
        "Graphics",
        format!("backend={renderer:?} platform={}", std::env::consts::OS),
    );
    let options = NativeOptions {
        viewport,
        hardware_acceleration: HardwareAcceleration::Preferred,
        vsync: true,
        multisampling: 0,
        renderer,
        ..Default::default()
    };

    let result = eframe::run_native(
        &format!("Kamafeu Studio v{} (Calafate)", kamafeu::APP_VERSION),
        options,
        Box::new(|cc| Ok(Box::new(kamafeu::gui::KamafeuStudioApp::new(cc)))),
    );
    if let Err(error) = result {
        if renderer == Renderer::Wgpu {
            kamafeu::terminal_log::warn(
                "Graphics",
                format!("WGPU não pôde iniciar ({error}); tentando fallback Glow/OpenGL"),
            );
            let fallback_options = NativeOptions {
                viewport: fallback_viewport,
                hardware_acceleration: HardwareAcceleration::Preferred,
                vsync: true,
                multisampling: 0,
                renderer: Renderer::Glow,
                ..Default::default()
            };
            eframe::run_native(
                &format!("Kamafeu Studio v{} (Calafate)", kamafeu::APP_VERSION),
                fallback_options,
                Box::new(|cc| Ok(Box::new(kamafeu::gui::KamafeuStudioApp::new(cc)))),
            )?;
        } else {
            return Err(error);
        }
    }

    Ok(())
}

pub fn run() {
    if let Err(error) = real_main() {
        #[cfg(target_os = "windows")]
        {
            let _ = rfd::MessageDialog::new()
                .set_title("Kamafeu Studio")
                .set_description(format!("O Kamafeu não pôde iniciar:\n\n{error}"))
                .show();
        }
        kamafeu::terminal_log::error("Startup", format!("{error}"));
        std::process::exit(1);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn requested_renderer() -> Renderer {
    match std::env::var("KAMAFEU_RENDERER")
        .unwrap_or_else(|_| "auto".to_string())
        .to_ascii_lowercase()
        .as_str()
    {
        "glow" | "opengl" => {
            kamafeu::terminal_log::info("Graphics", "Glow / OpenGL selected");
            Renderer::Glow
        }
        "wgpu" => {
            kamafeu::terminal_log::info("Graphics", "WGPU selected");
            Renderer::Wgpu
        }
        "auto" => {
            kamafeu::terminal_log::info("Graphics", "automatic backend selection");
            Renderer::Wgpu
        }
        other => {
            kamafeu::terminal_log::warn(
                "Graphics",
                format!("backend desconhecido '{other}'; usando modo automático WGPU"),
            );
            Renderer::Wgpu
        }
    }
}
