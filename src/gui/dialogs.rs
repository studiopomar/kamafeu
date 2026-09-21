mod autopitch;
mod batch_lyrics;
mod copaiba;
mod export_dialog;
mod export_options_dialog;
mod folder_picker;
mod fx_rack_dialog;
mod humanize_dialog;
mod lyrics_dialog;
mod project;
mod shortcuts_guide;
mod singers_gallery;
mod templates;
mod theme_customizer;
mod theme_editor_dialog;
mod voicebank;

use super::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(crate) fn render_dialogs(&mut self, ctx: &egui::Context) {
        self.render_theme_customizer_dialog(ctx);
        self.render_preferences_dialog(ctx);
        self.render_project_properties_dialog(ctx);
        self.render_templates_dialog(ctx);
        self.render_voicebank_diagnostic_dialog(ctx);

        self.show_theme_editor_dialog(ctx);
        self.show_fx_rack_dialog(ctx);
        self.show_lyrics_dialog(ctx);
        self.show_humanize_dialog(ctx);
        self.show_copaiba(ctx);
        self.show_packages_window(ctx);
        self.show_shortcuts_guide(ctx);
        self.show_batch_lyrics(ctx);
        self.show_singers_gallery(ctx);
        self.show_autopitch(ctx);
        self.show_export_options_dialog(ctx);
        self.show_export_dialog(ctx);
        self.show_folder_picker(ctx);
    }
}

pub fn reveal_in_file_manager_label() -> &'static str {
    reveal_in_file_manager_label_for(crate::config::AppLanguage::PtBr)
}

pub fn reveal_in_file_manager_label_for(lang: crate::config::AppLanguage) -> &'static str {
    #[cfg(target_os = "macos")]
    {
        lang.tr("Revelar no Finder", "Reveal in Finder")
    }
    #[cfg(target_os = "windows")]
    {
        lang.tr("Revelar no Explorador", "Reveal in Explorer")
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        lang.tr("Revelar no Gerenciador", "Reveal in File Manager")
    }
}

pub fn open_file_in_folder<P: AsRef<std::path::Path>>(path: P) {
    let _p = path.as_ref();
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg("-R").arg(_p).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        let path_str = _p.to_string_lossy().to_string();
        let _ = std::process::Command::new("explorer")
            .arg(format!("/select,\"{path_str}\""))
            .spawn();
    }
    #[cfg(all(
        not(target_os = "macos"),
        not(target_os = "windows"),
        not(target_os = "android")
    ))]
    {
        if let Some(parent) = _p.parent() {
            let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
        } else {
            let _ = std::process::Command::new("xdg-open").arg(_p).spawn();
        }
    }
}
