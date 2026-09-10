use crate::gui::KamafeuStudioApp;
use eframe::egui;
use std::time::Instant;

impl eframe::App for KamafeuStudioApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let now = Instant::now();
        let frame_ms = now.duration_since(self.last_frame_instant).as_secs_f32() * 1_000.0;
        self.last_frame_instant = now;
        if frame_ms.is_finite() && frame_ms < 1_000.0 {
            self.frame_time_ema_ms = self.frame_time_ema_ms * 0.9 + frame_ms * 0.1;
        }
        let active_scale = self.config.ui_scale_factor.clamp(0.7, 2.0);
        if (ctx.zoom_factor() - active_scale).abs() > 0.001 {
            ctx.set_zoom_factor(active_scale);
        }

        let project_name = if let Some(ref path) = self.current_project_path {
            path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("project.aps")
                .to_string()
        } else {
            let stem = self.project.name.trim();
            if stem.is_empty() {
                "Novo Projeto".to_string()
            } else {
                stem.to_string()
            }
        };
        let window_title = if self.is_dirty {
            format!("* {} - Kamafeu Studio v1.0.0-A (Âmbar)", project_name)
        } else {
            format!("{} - Kamafeu Studio v1.0.0-A (Âmbar)", project_name)
        };
        if window_title != self.last_window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(window_title.clone()));
            self.last_window_title = window_title;
        }

        // Keep the editor invariant: every active track owns an editable part.
        let _ = self.current_notes_mut();
        self.refresh_voicebank_oto();

        self.update_background_tasks(ctx);

        // Process dropped files (Drag & Drop)
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped.is_empty() {
            for dropped_file in dropped {
                if let Some(path) = dropped_file.path {
                    self.open_project_from_path(&path);
                }
            }
        }

        self.update_keyboard_shortcuts(ctx);

        self.update_editor_panels(ctx);

        self.update_editor_canvas(ctx);

        self.draw_mini_log_window(ctx);

        self.render_dialogs(ctx);

        self.draw_export_notification_toast(ctx);

        self.update_activity();
    }
}
