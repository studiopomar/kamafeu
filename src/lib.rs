#![allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::unnecessary_map_or,
    clippy::collapsible_if,
    dependency_on_unit_never_type_fallback,
    bindings_with_variant_name
)]
#![allow(unknown_lints)]
#![allow(clippy::all)]

pub mod audio;
pub mod config;
pub mod copaiba;
pub mod copaiba_bridge;
pub mod dialogs;
pub mod discord_rpc;
pub mod drivers;
pub mod dsp;
pub mod formats;
pub mod gui;
pub mod oto;
pub mod phonemizer;
pub mod project;
pub mod renderer;

pub use audio::*;
pub use config::*;
pub use copaiba::*;
pub use dialogs::*;
pub use drivers::*;
pub use dsp::*;
pub use formats::*;
pub use gui::*;
pub use oto::*;
pub use phonemizer::*;
pub use project::*;
pub use renderer::*;

/// Public application version used by the native window, Android activity and
/// in-app version badge. Cargo remains the single source of truth.
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
/// Internal lineage codename, intentionally kept out of the primary branding.
pub const APP_CODENAME: &str = "projeto_saturno";

#[cfg(target_os = "android")]
use android_activity::AndroidApp;

#[cfg(target_os = "android")]
#[no_mangle]
fn android_main(app: AndroidApp) {
    use eframe::NativeOptions;
    use winit::platform::android::EventLoopBuilderExtAndroid;

    let mut options = NativeOptions::default();
    options.event_loop_builder = Some(Box::new(move |builder| {
        builder.with_android_app(app);
    }));

    if let Err(error) = eframe::run_native(
        concat!(
            "Kamafeu Studio v",
            env!("CARGO_PKG_VERSION"),
            " (Bariloche)"
        ),
        options,
        Box::new(|cc| Ok(Box::new(gui::KamafeuStudioApp::new(cc)))),
    ) {
        eprintln!("Falha ao iniciar Kamafeu Studio no Android: {error}");
    }
}

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub async fn start() -> Result<(), JsValue> {
    // Redirect log messages to console.log and panic messages to console.error
    console_error_panic_hook::set_once();
    let _ = console_log::init_with_level(log::Level::Info);

    let web_options = eframe::WebOptions::default();

    let document = web_sys::window()
        .and_then(|win| win.document())
        .ok_or_else(|| JsValue::from_str("No document found"))?;
    let canvas = document
        .get_element_by_id("kamafeu_canvas")
        .ok_or_else(|| JsValue::from_str("Canvas 'kamafeu_canvas' not found in HTML"))?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| JsValue::from_str("Element is not a canvas"))?;

    eframe::WebRunner::new()
        .start(
            canvas,
            web_options,
            Box::new(|cc| Ok(Box::new(gui::KamafeuStudioApp::new(cc)))),
        )
        .await
        .map_err(|e| JsValue::from_str(&format!("Failed to start eframe: {e:?}")))?;

    Ok(())
}
