use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::fs;
use std::path::PathBuf;

pub fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let mut primary_candidates = Vec::new();
    let mut cjk_candidates = Vec::new();

    // Windows Font directories
    let mut win_font_dirs = Vec::new();
    if let Some(windir) = std::env::var_os("WINDIR").or_else(|| std::env::var_os("SYSTEMROOT")) {
        win_font_dirs.push(PathBuf::from(windir).join("Fonts"));
    }
    win_font_dirs.push(PathBuf::from(r"C:\Windows\Fonts"));
    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
        win_font_dirs.push(PathBuf::from(local_app_data).join(r"Microsoft\Windows\Fonts"));
    }

    for dir in &win_font_dirs {
        // Windows Primary Western Fonts
        primary_candidates.push(dir.join("segoeui.ttf"));
        primary_candidates.push(dir.join("arial.ttf"));
        primary_candidates.push(dir.join("tahoma.ttf"));
        primary_candidates.push(dir.join("calibri.ttf"));

        // Windows CJK (Japanese / Chinese / Korean) Fonts
        cjk_candidates.push(dir.join("msgothic.ttc"));
        cjk_candidates.push(dir.join("meiryo.ttc"));
        cjk_candidates.push(dir.join("YuGothM.ttc"));
        cjk_candidates.push(dir.join("YuGothR.ttc"));
        cjk_candidates.push(dir.join("msmincho.ttc"));
        cjk_candidates.push(dir.join("msyh.ttc"));
        cjk_candidates.push(dir.join("msyhl.ttc"));
        cjk_candidates.push(dir.join("simsun.ttc"));
        cjk_candidates.push(dir.join("malgun.ttf"));
    }

    // Android
    primary_candidates.push(PathBuf::from("/system/fonts/Roboto-Regular.ttf"));
    primary_candidates.push(PathBuf::from("/system/fonts/Roboto.ttf"));
    cjk_candidates.push(PathBuf::from("/system/fonts/NotoSansCJK-Regular.ttc"));
    cjk_candidates.push(PathBuf::from("/system/fonts/NotoSansJP-Regular.otf"));
    cjk_candidates.push(PathBuf::from("/system/fonts/NotoSansSC-Regular.otf"));
    cjk_candidates.push(PathBuf::from("/system/fonts/DroidSansFallback.ttf"));

    // macOS
    primary_candidates.push(PathBuf::from("/System/Library/Fonts/SFPro-Regular.otf"));
    primary_candidates.push(PathBuf::from("/System/Library/Fonts/Helvetica.ttc"));
    cjk_candidates.push(PathBuf::from("/System/Library/Fonts/Hiragino Sans W3.ttc"));
    cjk_candidates.push(PathBuf::from("/System/Library/Fonts/PingFang.ttc"));
    cjk_candidates.push(PathBuf::from("/System/Library/Fonts/STHeiti Light.ttc"));
    cjk_candidates.push(PathBuf::from("/System/Library/Fonts/Supplemental/Arial Unicode.ttf"));
    cjk_candidates.push(PathBuf::from("/Library/Fonts/Arial Unicode.ttf"));

    // Linux
    primary_candidates.push(PathBuf::from("/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf"));
    primary_candidates.push(PathBuf::from("/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf"));
    primary_candidates.push(PathBuf::from("/usr/share/fonts/truetype/freefont/FreeSans.ttf"));
    cjk_candidates.push(PathBuf::from("/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc"));
    cjk_candidates.push(PathBuf::from("/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc"));
    cjk_candidates.push(PathBuf::from("/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc"));
    cjk_candidates.push(PathBuf::from("/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf"));
    cjk_candidates.push(PathBuf::from("/usr/share/fonts/truetype/fonts-japanese-gothic.ttf"));
    cjk_candidates.push(PathBuf::from("/usr/share/fonts/truetype/takao-gothic/TakaoPGothic.ttf"));
    cjk_candidates.push(PathBuf::from("/usr/share/fonts/truetype/vlgothic/VL-PGothic-Regular.ttf"));
    cjk_candidates.push(PathBuf::from("/usr/share/fonts/truetype/wqy/wqy-microhei.ttc"));

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

    let mut loaded_any_cjk = false;
    for path in cjk_candidates {
        if path.exists() {
            if let Ok(bytes) = fs::read(&path) {
                let name = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("cjk_font")
                    .to_string();

                // Check if already inserted
                if !fonts.font_data.contains_key(&name) {
                    fonts
                        .font_data
                        .insert(name.clone(), FontData::from_owned(bytes));
                    // Append CJK font to fallback chain
                    fonts
                        .families
                        .entry(FontFamily::Proportional)
                        .or_default()
                        .push(name.clone());
                    fonts
                        .families
                        .entry(FontFamily::Monospace)
                        .or_default()
                        .push(name);
                    loaded_any_cjk = true;
                }
            }
        }
    }

    if loaded_primary || loaded_any_cjk {
        ctx.set_fonts(fonts);
    }
}
