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
        "Kamafeu Studio - sintetizador de voz",
        options,
        Box::new(|cc| Ok(Box::new(gui::KamafeuStudioApp::new(cc)))),
    ) {
        eprintln!("Falha ao iniciar Kamafeu Studio no Android: {error}");
    }
}
