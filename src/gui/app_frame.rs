use crate::gui::KamafeuStudioApp;
use eframe::egui;
use web_time::Instant;

impl eframe::App for KamafeuStudioApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|input| input.viewport().close_requested())
            && self.is_dirty
            && self.config.workflow.confirm_on_exit_dirty
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.exit_confirmation_open = true;
        }
        // Some macOS window managers ignore `ViewportBuilder::with_maximized`
        // during native window creation. Apply it once after the first frame,
        // when the viewport already exists, without overriding later toggles.
        #[cfg(not(target_arch = "wasm32"))]
        if self.startup_maximize_requested {
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            self.startup_maximize_requested = false;
        }

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

        // Touch-first density on narrow viewports. Keep desktop compact, but
        // make every primary control comfortable for a finger on Android.
        let viewport = ctx.screen_rect();
        let narrow = cfg!(target_os = "android")
            || viewport.width() < 1100.0
            || (viewport.height() < 600.0 && viewport.width() < 1200.0);
        ctx.style_mut(|style| {
            // Give hover/focus states a short, quiet interpolation instead of
            // switching abruptly between high-contrast surfaces.
            style.animation_time = if self.config.workflow.ui_animations_enabled {
                0.16 / self.config.workflow.ui_animation_speed.clamp(0.25, 4.0)
            } else {
                0.0
            };
            if narrow {
                style.spacing.item_spacing = egui::vec2(8.0, 8.0);
                style.spacing.button_padding = egui::vec2(12.0, 8.0);
                style.spacing.interact_size = egui::vec2(44.0, 44.0);
                style.spacing.menu_margin = egui::Margin::same(8.0);
            } else {
                style.spacing.item_spacing = egui::vec2(6.0, 4.0);
                style.spacing.button_padding = egui::vec2(8.0, 5.0);
                style.spacing.interact_size = egui::vec2(24.0, 24.0);
                style.spacing.menu_margin = egui::Margin::same(4.0);
            }
        });

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
            format!(
                "* {} - Kamafeu Studio v{} (Bariloche)",
                project_name,
                crate::APP_VERSION,
            )
        } else {
            format!(
                "{} - Kamafeu Studio v{} (Bariloche)",
                project_name,
                crate::APP_VERSION
            )
        };
        #[cfg(not(target_arch = "wasm32"))]
        if window_title != self.last_window_title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(window_title.clone()));
            self.last_window_title = window_title;
        }

        // Keep the editor invariant: every active track owns an editable part.
        let _ = self.current_notes_mut();
        self.refresh_voicebank_oto();

        self.update_background_tasks(ctx);

        if !self.preferences_state.automatic_check_started {
            self.preferences_state.automatic_check_started = true;
            let frequency = self.config.updates.frequency.as_str();
            if self.config.updates.check_enabled && frequency != "never" {
                let interval = match frequency {
                    "daily" => 86_400,
                    "monthly" => 30 * 86_400,
                    _ => 7 * 86_400,
                };
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_secs() as i64)
                    .unwrap_or_default();
                if self
                    .config
                    .updates
                    .last_check_unix
                    .map(|last| now.saturating_sub(last) >= interval)
                    .unwrap_or(true)
                {
                    self.config.updates.last_check_unix = Some(now);
                    self.preferences_state.request_update_check();
                    self.persist_config();
                }
            }
        }
        self.preferences_state.poll_update_check();
        if matches!(
            self.preferences_state.update_result,
            Some(Ok(ref release)) if crate::updater::is_newer(&release.tag_name, crate::APP_VERSION)
        ) {
            self.preferences_window_open = true;
            self.preferences_tab = 10;
            if self.config.updates.automatic_download
                && !self.preferences_state.update_opened_automatically
            {
                if let Some(Ok(release)) = self.preferences_state.update_result.as_ref() {
                    crate::gui::open_external_url(&release.html_url);
                }
                self.preferences_state.update_opened_automatically = true;
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            let mut pending = Vec::new();
            if let Ok(mut queue) = crate::gui::project_files::web_file_queue().lock() {
                if !queue.is_empty() {
                    pending = std::mem::take(&mut *queue);
                }
            }
            for (name, bytes) in pending {
                self.open_project_from_bytes(&name, &bytes);
            }
        }

        // Process dropped files (Drag & Drop)
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if !dropped.is_empty() {
            for dropped_file in dropped {
                if let Some(bytes) = dropped_file.bytes {
                    self.open_project_from_bytes(&dropped_file.name, &bytes);
                } else if let Some(path) = dropped_file.path {
                    self.open_project_from_path(&path);
                }
            }
        }

        self.update_keyboard_shortcuts(ctx);

        self.update_editor_panels(ctx);

        self.update_editor_canvas(ctx);

        self.draw_mini_log_window(ctx);

        self.render_dialogs(ctx);

        self.show_exit_confirmation(ctx);

        self.draw_export_notification_toast(ctx);
        self.draw_panel_tips_bubble(ctx);
        self.draw_no_phonemizer_warning_bubble(ctx);

        self.update_activity();
    }
}

impl KamafeuStudioApp {
    fn show_exit_confirmation(&mut self, ctx: &egui::Context) {
        if !self.exit_confirmation_open {
            return;
        }

        let lang = self.config.language;
        let mut keep_editing = false;
        let mut discard = false;
        let mut save = false;
        egui::Window::new(lang.tr(
            "Salvar alterações antes de fechar",
            "Save changes before closing",
        ))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
        .show(ctx, |ui| {
            ui.label(lang.tr(
                "Este projeto possui alterações não salvas.",
                "This project has unsaved changes.",
            ));
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(lang.tr("Salvar", "Save")).clicked() {
                    save = true;
                }
                if ui.button(lang.tr("Descartar", "Discard")).clicked() {
                    discard = true;
                }
                if ui.button(lang.tr("Cancelar", "Cancel")).clicked() {
                    keep_editing = true;
                }
            });
        });

        if keep_editing {
            self.exit_confirmation_open = false;
        } else if save {
            self.save_project();
            // A failed or cancelled Save As must return to the project rather
            // than silently dropping edits.
            self.exit_confirmation_open = false;
            if !self.is_dirty {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        } else if discard {
            self.exit_confirmation_open = false;
            self.is_dirty = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
