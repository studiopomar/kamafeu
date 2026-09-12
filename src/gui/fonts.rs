use eframe::egui::{self, FontData, FontDefinitions, FontFamily};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

pub fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();

    let mut primary_candidates = Vec::new();
    let mut cjk_candidates = Vec::new();
    let mut search_dirs = Vec::new();

    // 1. Local App directories
    let local_dirs = [
        PathBuf::from("./fonts"),
        PathBuf::from("./assets/fonts"),
        PathBuf::from("../fonts"),
        PathBuf::from("./res/fonts"),
    ];
    for dir in &local_dirs {
        if dir.is_dir() {
            search_dirs.push(dir.clone());
        }
    }

    // 2. Windows Font directories
    let mut win_font_dirs = Vec::new();
    if let Some(windir) = std::env::var_os("WINDIR").or_else(|| std::env::var_os("SYSTEMROOT")) {
        win_font_dirs.push(PathBuf::from(windir).join("Fonts"));
    }
    win_font_dirs.push(PathBuf::from(r"C:\Windows\Fonts"));
    if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
        win_font_dirs.push(PathBuf::from(&local_app_data).join(r"Microsoft\Windows\Fonts"));
        win_font_dirs.push(PathBuf::from(&local_app_data).join(r"OpenUtau\Fonts"));
    }
    if let Some(app_data) = std::env::var_os("APPDATA") {
        win_font_dirs.push(PathBuf::from(&app_data).join(r"OpenUtau\Fonts"));
    }
    if let Some(user_profile) = std::env::var_os("USERPROFILE") {
        win_font_dirs.push(PathBuf::from(&user_profile).join(r".fonts"));
    }

    for dir in &win_font_dirs {
        if dir.is_dir() {
            search_dirs.push(dir.clone());
        }

        // Windows Primary Western / Cyrillic Fonts
        primary_candidates.push(dir.join("segoeui.ttf"));
        primary_candidates.push(dir.join("arial.ttf"));
        primary_candidates.push(dir.join("tahoma.ttf"));
        primary_candidates.push(dir.join("calibri.ttf"));

        // Windows CJK (Japanese / Chinese / Korean) Fonts & Fallbacks
        cjk_candidates.push(dir.join("msgothic.ttc"));
        cjk_candidates.push(dir.join("meiryo.ttc"));
        cjk_candidates.push(dir.join("meiryob.ttc"));
        cjk_candidates.push(dir.join("YuGothM.ttc"));
        cjk_candidates.push(dir.join("YuGothR.ttc"));
        cjk_candidates.push(dir.join("YuGothB.ttc"));
        cjk_candidates.push(dir.join("YuGothL.ttc"));
        cjk_candidates.push(dir.join("yugoth.ttc"));
        cjk_candidates.push(dir.join("yugothr.ttc"));
        cjk_candidates.push(dir.join("yugothm.ttc"));
        cjk_candidates.push(dir.join("yugothb.ttc"));
        cjk_candidates.push(dir.join("msmincho.ttc"));
        cjk_candidates.push(dir.join("msyh.ttc"));
        cjk_candidates.push(dir.join("msyhl.ttc"));
        cjk_candidates.push(dir.join("msyhbd.ttc"));
        cjk_candidates.push(dir.join("simsun.ttc"));
        cjk_candidates.push(dir.join("simsunb.ttf"));
        cjk_candidates.push(dir.join("simhei.ttf"));
        cjk_candidates.push(dir.join("malgun.ttf"));
        cjk_candidates.push(dir.join("malgunbd.ttf"));
        cjk_candidates.push(dir.join("mingliu.ttc"));
        cjk_candidates.push(dir.join("mingliub.ttc"));
        cjk_candidates.push(dir.join("kaiu.ttf"));
        cjk_candidates.push(dir.join("batang.ttc"));
        cjk_candidates.push(dir.join("gulim.ttc"));
        cjk_candidates.push(dir.join("seguisym.ttf"));
        cjk_candidates.push(dir.join("seguiemj.ttf"));
        cjk_candidates.push(dir.join("arialuni.ttf"));
        cjk_candidates.push(dir.join("Deng.ttf"));
        cjk_candidates.push(dir.join("Dengb.ttf"));
        cjk_candidates.push(dir.join("Dengl.ttf"));
    }

    // Android
    primary_candidates.push(PathBuf::from("/system/fonts/Roboto-Regular.ttf"));
    primary_candidates.push(PathBuf::from("/system/fonts/Roboto.ttf"));
    cjk_candidates.push(PathBuf::from("/system/fonts/NotoSansCJK-Regular.ttc"));
    cjk_candidates.push(PathBuf::from("/system/fonts/NotoSansJP-Regular.otf"));
    cjk_candidates.push(PathBuf::from("/system/fonts/NotoSansSC-Regular.otf"));
    cjk_candidates.push(PathBuf::from("/system/fonts/NotoSansKR-Regular.otf"));
    cjk_candidates.push(PathBuf::from("/system/fonts/DroidSansFallback.ttf"));

    // macOS
    primary_candidates.push(PathBuf::from("/System/Library/Fonts/SFPro-Regular.otf"));
    primary_candidates.push(PathBuf::from("/System/Library/Fonts/Helvetica.ttc"));
    cjk_candidates.push(PathBuf::from("/System/Library/Fonts/Hiragino Sans W3.ttc"));
    cjk_candidates.push(PathBuf::from("/System/Library/Fonts/PingFang.ttc"));
    cjk_candidates.push(PathBuf::from("/System/Library/Fonts/STHeiti Light.ttc"));
    cjk_candidates.push(PathBuf::from(
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
    ));
    cjk_candidates.push(PathBuf::from("/Library/Fonts/Arial Unicode.ttf"));

    // Linux
    primary_candidates.push(PathBuf::from(
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ));
    primary_candidates.push(PathBuf::from(
        "/usr/share/fonts/truetype/ubuntu/Ubuntu-R.ttf",
    ));
    primary_candidates.push(PathBuf::from(
        "/usr/share/fonts/truetype/freefont/FreeSans.ttf",
    ));
    cjk_candidates.push(PathBuf::from(
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
    ));
    cjk_candidates.push(PathBuf::from(
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
    ));
    cjk_candidates.push(PathBuf::from(
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
    ));
    cjk_candidates.push(PathBuf::from(
        "/usr/share/fonts/truetype/droid/DroidSansFallbackFull.ttf",
    ));
    cjk_candidates.push(PathBuf::from(
        "/usr/share/fonts/truetype/fonts-japanese-gothic.ttf",
    ));
    cjk_candidates.push(PathBuf::from(
        "/usr/share/fonts/truetype/takao-gothic/TakaoPGothic.ttf",
    ));
    cjk_candidates.push(PathBuf::from(
        "/usr/share/fonts/truetype/vlgothic/VL-PGothic-Regular.ttf",
    ));
    cjk_candidates.push(PathBuf::from(
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
    ));
    cjk_candidates.push(PathBuf::from("/usr/share/fonts/truetype/ipa/ipag.ttf"));
    cjk_candidates.push(PathBuf::from(
        "/usr/share/fonts/truetype/sazanami/sazanami-gothic.ttf",
    ));

    // Scan search directories for any additional CJK / Asian / Unicode font files
    let mut scanned_cjk = Vec::new();
    let cjk_keywords = [
        "cjk", "jp", "japanese", "goth", "meiryo", "mincho", "yahei", "noto", "han", "simsun",
        "simhei", "malgun", "mingliu", "batang", "gulim", "kaiu", "deng", "wqy", "takao",
        "vlgothic", "ipa", "sazanami", "arialuni", "hiragino", "pingfang", "heiti", "kyokasho",
        "round", "morisawa", "biz",
    ];

    for dir in &search_dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                    let ext_lower = ext.to_ascii_lowercase();
                    if ext_lower == "ttf" || ext_lower == "ttc" || ext_lower == "otf" {
                        if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
                            let lower_name = file_name.to_ascii_lowercase();
                            if cjk_keywords.iter().any(|&kw| lower_name.contains(kw)) {
                                scanned_cjk.push(path);
                            }
                        }
                    }
                }
            }
        }
    }

    cjk_candidates.extend(scanned_cjk);

    let mut loaded_primary = false;
    for path in primary_candidates {
        if path.exists() && path.is_file() {
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
    let mut seen_paths = HashSet::new();

    for path in cjk_candidates {
        if !seen_paths.insert(path.clone()) {
            continue;
        }

        if path.exists() && path.is_file() {
            if let Ok(bytes) = fs::read(&path) {
                let stem = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("cjk_font")
                    .to_string();

                // TTF / OTF / TTC font loading. epaint uses ab_glyph / ttf_parser internally.
                // We use index 0 which is universally supported and safe across all TTCs.
                let font_key = stem.clone();
                if !fonts.font_data.contains_key(&font_key) {
                    let font_data = FontData::from_owned(bytes);
                    fonts.font_data.insert(font_key.clone(), font_data);

                    // Append CJK font face to fallback chain
                    fonts
                        .families
                        .entry(FontFamily::Proportional)
                        .or_default()
                        .push(font_key.clone());
                    fonts
                        .families
                        .entry(FontFamily::Monospace)
                        .or_default()
                        .push(font_key);
                    loaded_any_cjk = true;
                }
            }
        }
    }

    if loaded_primary || loaded_any_cjk {
        ctx.set_fonts(fonts);
    }
}
