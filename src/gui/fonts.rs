use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::fs;
use std::path::PathBuf;

pub fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    // macOS CJK Japanese system font candidates
    let font_candidates = [
        PathBuf::from("/System/Library/Fonts/Supplemental/Arial Unicode.ttf"),
        PathBuf::from("/System/Library/Fonts/Hiragino Sans W3.ttc"),
        PathBuf::from("/System/Library/Fonts/STHeiti Light.ttc"),
        PathBuf::from("/System/Library/Fonts/PingFang.ttc"),
        PathBuf::from("/Library/Fonts/Arial Unicode.ttf"),
        PathBuf::from("/System/Library/Fonts/Helvetica.ttc"),
    ];

    let mut loaded = false;

    for font_path in font_candidates {
        if font_path.exists() {
            if let Ok(font_bytes) = fs::read(&font_path) {
                let font_name = font_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("cjk_japanese_font")
                    .to_string();

                fonts.font_data.insert(
                    font_name.clone(),
                    FontData::from_owned(font_bytes),
                );

                fonts
                    .families
                    .entry(FontFamily::Proportional)
                    .or_default()
                    .push(font_name.clone());

                fonts
                    .families
                    .entry(FontFamily::Monospace)
                    .or_default()
                    .push(font_name);

                loaded = true;
                break;
            }
        }
    }

    if loaded {
        ctx.set_fonts(fonts);
    }
}
