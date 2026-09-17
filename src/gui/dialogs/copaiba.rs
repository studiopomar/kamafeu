use crate::gui::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(super) fn show_copaiba(&mut self, ctx: &egui::Context) {
        if self.copaiba_window_open {
            let mut is_open = self.copaiba_window_open;
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("copaiba_toolkit_native_viewport"),
                egui::ViewportBuilder::default()
                    .with_title("Copaiba Voicebank Toolkit - Kamafeu Studio")
                    .with_inner_size([1000.0, 600.0])
                    .with_min_inner_size([700.0, 450.0]),
                |ctx, _class| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        crate::copaiba::gui::draw_copaiba_toolkit_ui(
                            &mut self.copaiba_app,
                            &mut self.audio_player,
                            ui,
                        );
                    });
                    if ctx.input(|i| i.viewport().close_requested()) {
                        is_open = false;
                    }
                },
            );
            self.copaiba_window_open = is_open;
        }
    }
}
