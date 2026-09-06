use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::fs;
use std::path::PathBuf;

pub fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let primary_candidates = [
        PathBuf::from("/system/fonts/Roboto-Regular.ttf"),
        PathBuf::from("/system/fonts/Roboto.ttf"),
        PathBuf::from("/System/Library/Fonts/SFPro-Regular.otf"),
        PathBuf::from("/System/Library/Fonts/Helvetica.ttc"),
        PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"),
    ];

    let cjk_candidates = [
        PathBuf::from("/system/fonts/NotoSansCJK-Regular.ttc"),
        PathBuf::from("/system/fonts/NotoSansJP-Regular.otf"),
        PathBuf::from("/system/fonts/NotoSansSC-Regular.otf"),
        PathBuf::from("/system/fonts/DroidSansFallback.ttf"),
        PathBuf::from("/System/Library/Fonts/Hiragino Sans W3.ttc"),
        PathBuf::from("/System/Library/Fonts/PingFang.ttc"),
        PathBuf::from("/System/Library/Fonts/STHeiti Light.ttc"),
        PathBuf::from("/System/Library/Fonts/Supplemental/Arial Unicode.ttf"),
        PathBuf::from("/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc"),
    ];

    let mut loaded_primary = false;
    for path in primary_candidates {
        if path.exists() {
            if let Ok(bytes) = fs::read(&path) {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("primary_font")
                    .to_string();

                fonts
                    .font_data
                    .insert(name.clone(), FontData::from_owned(bytes));
                fonts
                    .families
                    .entry(FontFamily::Proportional)
                    .or_default()
                    .insert(0, name.clone());
                fonts
                    .families
                    .entry(FontFamily::Monospace)
                    .or_default()
                    .insert(0, name);
                loaded_primary = true;
                break;
            }
        }
    }

    let mut loaded_cjk = false;
    for path in cjk_candidates {
        if path.exists() {
            if let Ok(bytes) = fs::read(&path) {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("cjk_font")
                    .to_string();

                fonts
                    .font_data
                    .insert(name.clone(), FontData::from_owned(bytes));
                let insert_idx = if loaded_primary { 1 } else { 0 };
                fonts
                    .families
                    .entry(FontFamily::Proportional)
                    .or_default()
                    .insert(insert_idx, name.clone());
                fonts
                    .families
                    .entry(FontFamily::Monospace)
                    .or_default()
                    .insert(insert_idx, name);
                loaded_cjk = true;
                break;
            }
        }
    }

    if loaded_primary || loaded_cjk {
        ctx.set_fonts(fonts);
    }
}
