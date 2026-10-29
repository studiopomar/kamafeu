use crate::drivers::tainah_ffi::{self, FormantAnalysis};
use crate::gui::theme::ThemeConfig;
use crate::oto::Voicebank;
use eframe::egui::{self, Color32, FontId, Frame, Pos2, Rect, RichText, Rounding, Sense, Stroke, Vec2};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

/// Classification of reference vowels in the BRAPA + English rhotic vowel space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VowelCategory {
    Oral,
    CentralSchwa,
    Nasal,
    Rhotic,
}

/// Reference vowel in the BRAPA + Schwa + Nasal + Rhotic vowel space.
#[derive(Debug, Clone, Copy)]
pub struct VowelTarget {
    pub symbol: &'static str,
    pub display_label: &'static str,
    pub hint_pt: &'static str,
    pub hint_en: &'static str,
    pub f1_hz: f64,
    pub f2_hz: f64,
    pub f3_ratio: f64,
    pub nasality: f64,
    pub category: VowelCategory,
}

/// Canonical BRAPA oral vowels, centrals/schwa (`ə`, `ax`), BRAPA nasals (`in`, `en`, `an`, `on`, `un`),
/// and the American English retroflex/rhotic R (`ɚ` / `er`).
pub const BRAPA_VOWEL_TARGETS: &[VowelTarget] = &[
    // --- Orais BRAPA ---
    VowelTarget {
        symbol: "i",
        display_label: "i",
        hint_pt: "i fechado (vi)",
        hint_en: "close front (see)",
        f1_hz: 280.0,
        f2_hz: 2300.0,
        f3_ratio: 1.0,
        nasality: 0.0,
        category: VowelCategory::Oral,
    },
    VowelTarget {
        symbol: "e",
        display_label: "e",
        hint_pt: "e fechado (vê)",
        hint_en: "close-mid front",
        f1_hz: 400.0,
        f2_hz: 2050.0,
        f3_ratio: 1.0,
        nasality: 0.0,
        category: VowelCategory::Oral,
    },
    VowelTarget {
        symbol: "eh",
        display_label: "eh",
        hint_pt: "é aberto (pé)",
        hint_en: "open-mid front (eh)",
        f1_hz: 560.0,
        f2_hz: 1850.0,
        f3_ratio: 1.0,
        nasality: 0.0,
        category: VowelCategory::Oral,
    },
    VowelTarget {
        symbol: "a",
        display_label: "a",
        hint_pt: "a aberto (pá)",
        hint_en: "open central (ah)",
        f1_hz: 780.0,
        f2_hz: 1320.0,
        f3_ratio: 1.0,
        nasality: 0.0,
        category: VowelCategory::Oral,
    },
    VowelTarget {
        symbol: "oh",
        display_label: "oh",
        hint_pt: "ó aberto (só)",
        hint_en: "open-mid back (aw)",
        f1_hz: 580.0,
        f2_hz: 980.0,
        f3_ratio: 1.0,
        nasality: 0.0,
        category: VowelCategory::Oral,
    },
    VowelTarget {
        symbol: "o",
        display_label: "o",
        hint_pt: "ô fechado (avô)",
        hint_en: "close-mid back (oh)",
        f1_hz: 420.0,
        f2_hz: 840.0,
        f3_ratio: 1.0,
        nasality: 0.0,
        category: VowelCategory::Oral,
    },
    VowelTarget {
        symbol: "u",
        display_label: "u",
        hint_pt: "u fechado (tu)",
        hint_en: "close back (boo)",
        f1_hz: 300.0,
        f2_hz: 750.0,
        f3_ratio: 1.0,
        nasality: 0.0,
        category: VowelCategory::Oral,
    },
    // --- Centrais / Schwa ---
    VowelTarget {
        symbol: "ə",
        display_label: "ə",
        hint_pt: "schwa central [ə] (inglês 'kah' / 'uh')",
        hint_en: "central schwa ('uh'/'kah')",
        f1_hz: 500.0,
        f2_hz: 1500.0,
        f3_ratio: 1.0,
        nasality: 0.0,
        category: VowelCategory::CentralSchwa,
    },
    VowelTarget {
        symbol: "ax",
        display_label: "ax",
        hint_pt: "a reduzido BRAPA [ɐ] (cama)",
        hint_en: "near-open central [ɐ]",
        f1_hz: 600.0,
        f2_hz: 1420.0,
        f3_ratio: 1.0,
        nasality: 0.0,
        category: VowelCategory::CentralSchwa,
    },
    // --- Nasais BRAPA ---
    VowelTarget {
        symbol: "in",
        display_label: "iñ",
        hint_pt: "i nasal BRAPA [ĩ] (sim, vim)",
        hint_en: "close front nasal [ĩ]",
        f1_hz: 310.0,
        f2_hz: 2180.0,
        f3_ratio: 1.0,
        nasality: 0.85,
        category: VowelCategory::Nasal,
    },
    VowelTarget {
        symbol: "en",
        display_label: "eñ",
        hint_pt: "e nasal BRAPA [ẽ] (vento, tempo)",
        hint_en: "close-mid front nasal [ẽ]",
        f1_hz: 440.0,
        f2_hz: 1920.0,
        f3_ratio: 1.0,
        nasality: 0.85,
        category: VowelCategory::Nasal,
    },
    VowelTarget {
        symbol: "an",
        display_label: "añ",
        hint_pt: "a nasal BRAPA [ɐ̃] (lã, canto, maçã)",
        hint_en: "open-mid central nasal [ɐ̃]",
        f1_hz: 540.0,
        f2_hz: 1380.0,
        f3_ratio: 1.0,
        nasality: 0.85,
        category: VowelCategory::Nasal,
    },
    VowelTarget {
        symbol: "on",
        display_label: "oñ",
        hint_pt: "o nasal BRAPA [õ] (som, bom)",
        hint_en: "close-mid back nasal [õ]",
        f1_hz: 450.0,
        f2_hz: 900.0,
        f3_ratio: 1.0,
        nasality: 0.85,
        category: VowelCategory::Nasal,
    },
    VowelTarget {
        symbol: "un",
        display_label: "uñ",
        hint_pt: "u nasal BRAPA [ũ] (um, atum)",
        hint_en: "close back nasal [ũ]",
        f1_hz: 330.0,
        f2_hz: 780.0,
        f3_ratio: 1.0,
        nasality: 0.85,
        category: VowelCategory::Nasal,
    },
    // --- R do Inglês / Rótico ---
    VowelTarget {
        symbol: "ɚ",
        display_label: "ɚ (R en)",
        hint_pt: "R enrolado/rótico do inglês [ɚ] ('car', 'bird', 'girl' ou R caipira 'porta')",
        hint_en: "rhotic / retroflex R [ɚ] ('bird', 'her', 'car')",
        f1_hz: 470.0,
        f2_hz: 1350.0,
        f3_ratio: 0.68,
        nasality: 0.0,
        category: VowelCategory::Rhotic,
    },
];

const F1_MIN_HZ: f64 = 200.0;
const F1_MAX_HZ: f64 = 1000.0;
const F2_MIN_HZ: f64 = 550.0;
const F2_MAX_HZ: f64 = 2700.0;

/// State communicated from the piano roll / transport during audio playback.
#[derive(Debug, Clone, Copy, Default)]
pub struct PlaybackFormantState {
    pub playhead_ms: f64,
    pub is_playing: bool,
    pub note_pos_ms: f64,
    pub note_dur_ms: f64,
    pub consonant_ms: f64,
}

/// Converts frequency in Hz to the perceptual Bark scale (Traunmüller 1990).
pub fn hz_to_bark(hz: f64) -> f64 {
    let h = hz.max(20.0);
    26.81 * h / (1960.0 + h) - 0.53
}

/// Inverse Bark-to-Hz conversion.
pub fn bark_to_hz(bark: f64) -> f64 {
    let b = (bark + 0.53).clamp(0.5, 25.5);
    1960.0 * b / (26.81 - b)
}

/// Maps `(f1_hz, f2_hz)` into `rect` using Praat conventions:
/// - Horizontal (X): F2 inverted (high F2 on left, low F2 on right)
/// - Vertical (Y): F1 downwards (low F1 at top, high F1 at bottom)
pub fn formant_to_pos(rect: Rect, f1_hz: f64, f2_hz: f64) -> Pos2 {
    let b1_min = hz_to_bark(F1_MIN_HZ);
    let b1_max = hz_to_bark(F1_MAX_HZ);
    let b2_min = hz_to_bark(F2_MIN_HZ);
    let b2_max = hz_to_bark(F2_MAX_HZ);

    let b1 = hz_to_bark(f1_hz.clamp(F1_MIN_HZ, F1_MAX_HZ));
    let b2 = hz_to_bark(f2_hz.clamp(F2_MIN_HZ, F2_MAX_HZ));

    let tx = ((b2_max - b2) / (b2_max - b2_min)).clamp(0.0, 1.0) as f32;
    let ty = ((b1 - b1_min) / (b1_max - b1_min)).clamp(0.0, 1.0) as f32;

    let pad_x = 18.0;
    let pad_y = 16.0;
    let inner = rect.shrink2(Vec2::new(pad_x, pad_y));
    Pos2::new(
        inner.left() + tx * inner.width(),
        inner.top() + ty * inner.height(),
    )
}

/// Maps a screen position inside `rect` back to `(f1_hz, f2_hz)`.
pub fn pos_to_formant(rect: Rect, pos: Pos2) -> (f64, f64) {
    let pad_x = 18.0;
    let pad_y = 16.0;
    let inner = rect.shrink2(Vec2::new(pad_x, pad_y));
    let tx = ((pos.x - inner.left()) / inner.width().max(1.0)).clamp(0.0, 1.0) as f64;
    let ty = ((pos.y - inner.top()) / inner.height().max(1.0)).clamp(0.0, 1.0) as f64;

    let b1_min = hz_to_bark(F1_MIN_HZ);
    let b1_max = hz_to_bark(F1_MAX_HZ);
    let b2_min = hz_to_bark(F2_MIN_HZ);
    let b2_max = hz_to_bark(F2_MAX_HZ);

    let b2 = b2_max - tx * (b2_max - b2_min);
    let b1 = b1_min + ty * (b1_max - b1_min);

    (
        bark_to_hz(b1).clamp(F1_MIN_HZ, F1_MAX_HZ),
        bark_to_hz(b2).clamp(F2_MIN_HZ, F2_MAX_HZ),
    )
}

/// Infers the canonical BRAPA vowel symbol from a note lyric or phoneme alias.
pub fn infer_base_vowel_symbol(lyric: &str) -> &'static str {
    let token = lyric
        .trim()
        .trim_start_matches('-')
        .trim()
        .to_lowercase();
    if token.is_empty() {
        return "a";
    }
    if token.ends_with("in") || token.ends_with('ĩ') || token.ends_with("im") {
        return "in";
    }
    if token.ends_with("en") || token.ends_with('ẽ') || token.ends_with("em") {
        return "en";
    }
    if token.ends_with("an") || token.ends_with('ã') || token.ends_with("am") {
        return "an";
    }
    if token.ends_with("on") || token.ends_with('õ') || token.ends_with("om") {
        return "on";
    }
    if token.ends_with("un") || token.ends_with('ũ') || token.ends_with("um") {
        return "un";
    }
    if token.ends_with("er") || token.ends_with("ɚ") || token.ends_with("ɝ") {
        return "ɚ";
    }
    if token.ends_with("eh") || token.ends_with('é') || token.ends_with('ê') {
        return "eh";
    }
    if token.ends_with("oh") || token.ends_with('ó') {
        return "oh";
    }
    if token.ends_with("ax") {
        return "ax";
    }
    if let Some(ch) = token.chars().last() {
        match ch {
            'i' | 'í' | 'y' | 'い' | 'ぃ' | 'き' | 'ぎ' | 'し' | 'じ' | 'ち' | 'ぢ' | 'に'
            | 'ひ' | 'び' | 'ぴ' | 'み' | 'り' | 'イ' | 'キ' | 'シ' | 'チ' | 'ニ' | 'ヒ'
            | 'ミ' | 'リ' => return "i",
            'u' | 'ú' | 'w' | 'う' | 'ぅ' | 'く' | 'ぐ' | 'す' | 'ず' | 'つ' | 'づ' | 'ぬ'
            | 'ふ' | 'ぶ' | 'ぷ' | 'む' | 'ゆ' | 'ゅ' | 'る' | 'ウ' | 'ク' | 'ス' | 'ツ'
            | 'ヌ' | 'フ' | 'ム' | 'ユ' | 'ル' => return "u",
            'e' | 'え' | 'ぇ' | 'け' | 'げ' | 'せ' | 'ぜ' | 'て' | 'で' | 'ね' | 'へ' | 'べ'
            | 'ぺ' | 'め' | 'れ' | 'エ' | 'ケ' | 'セ' | 'テ' | 'ネ' | 'ヘ' | 'メ' | 'レ' => {
                return "e"
            }
            'o' | 'ô' | 'お' | 'ぉ' | 'こ' | 'ご' | 'そ' | 'ぞ' | 'と' | 'ど' | 'の' | 'ほ'
            | 'ぼ' | 'ぽ' | 'も' | 'よ' | 'ょ' | 'ろ' | 'を' | 'オ' | 'コ' | 'ソ' | 'ト'
            | 'ノ' | 'ホ' | 'モ' | 'ヨ' | 'ロ' => return "o",
            'a' | 'á' | 'à' | 'â' | 'あ' | 'ぁ' | 'か' | 'が' | 'さ' | 'ざ' | 'た'
            | 'だ' | 'な' | 'は' | 'ば' | 'ぱ' | 'ま' | 'や' | 'ゃ' | 'ら' | 'わ' | 'ア'
            | 'カ' | 'サ' | 'タ' | 'ナ' | 'ハ' | 'マ' | 'ヤ' | 'ラ' | 'ワ' => return "a",
            _ => {}
        }
    }
    for ch in token.chars().rev() {
        match ch {
            'i' | 'í' => return "i",
            'u' | 'ú' => return "u",
            'e' | 'é' | 'ê' => return "e",
            'o' | 'ó' | 'ô' => return "o",
            'a' | 'á' | 'â' => return "a",
            _ => {}
        }
    }
    "a"
}

pub fn canonical_vowel(symbol: &str) -> VowelTarget {
    BRAPA_VOWEL_TARGETS
        .iter()
        .copied()
        .find(|v| v.symbol == symbol)
        .unwrap_or(BRAPA_VOWEL_TARGETS[3]) // default "a"
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct AnalysisCacheKey {
    wav_path: PathBuf,
    offset_q: i64,
    consonant_q: i64,
    cutoff_q: i64,
}

#[derive(Debug, Clone)]
pub struct CachedVowelAnalysis {
    pub analysis: FormantAnalysis,
    pub wave_cycle: Vec<f32>,
}

#[derive(Debug, Clone)]
enum AnalysisEntry {
    Loading,
    Ready(CachedVowelAnalysis),
    Failed(String),
}

static ANALYSIS_CACHE: LazyLock<Mutex<HashMap<AnalysisCacheKey, AnalysisEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn request_or_get_analysis(
    ctx: &egui::Context,
    wav_path: PathBuf,
    offset_ms: f64,
    consonant_ms: f64,
    cutoff_ms: f64,
) -> Option<Result<CachedVowelAnalysis, String>> {
    let key = AnalysisCacheKey {
        wav_path: wav_path.clone(),
        offset_q: (offset_ms * 10.0).round() as i64,
        consonant_q: (consonant_ms * 10.0).round() as i64,
        cutoff_q: (cutoff_ms * 10.0).round() as i64,
    };
    {
        let mut guard = ANALYSIS_CACHE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match guard.get(&key) {
            Some(AnalysisEntry::Ready(data)) => return Some(Ok(data.clone())),
            Some(AnalysisEntry::Failed(err)) => return Some(Err(err.clone())),
            Some(AnalysisEntry::Loading) => return None,
            None => {
                if guard.len() > 128 {
                    guard.clear();
                }
                guard.insert(key.clone(), AnalysisEntry::Loading);
            }
        }
    }

    let ctx_clone = ctx.clone();
    std::thread::spawn(move || {
        let result = (|| -> Result<CachedVowelAnalysis, String> {
            let (samples, sample_rate) =
                crate::renderer::TrackRenderer::load_wav_samples(&wav_path)?;
            let analysis = tainah_ffi::analyze_formants(
                &samples,
                sample_rate,
                offset_ms,
                consonant_ms,
                cutoff_ms,
            )?;
            let start_ms = (offset_ms.max(0.0) + consonant_ms.max(0.0) + 25.0).max(0.0);
            let start_idx = ((start_ms * sample_rate as f64 / 1000.0) as usize)
                .min(samples.len().saturating_sub(1));
            let period_samples = if analysis.f0_hz > 40.0 {
                (sample_rate as f64 / analysis.f0_hz).round() as usize
            } else {
                (sample_rate as usize / 200).max(64)
            };
            let span = (period_samples * 3).clamp(96, 1200);
            let end_idx = (start_idx + span).min(samples.len());
            let mut wave_cycle = Vec::with_capacity(96);
            if end_idx > start_idx + 8 {
                let slice = &samples[start_idx..end_idx];
                let peak = slice
                    .iter()
                    .map(|v| v.abs())
                    .fold(0.0f32, f32::max)
                    .max(1e-4);
                for i in 0..96 {
                    let pos = i as f32 * (slice.len() - 1) as f32 / 95.0;
                    let idx = pos as usize;
                    let frac = pos - idx as f32;
                    let s0 = slice[idx];
                    let s1 = slice[(idx + 1).min(slice.len() - 1)];
                    wave_cycle.push((s0 + frac * (s1 - s0)) / peak);
                }
            }
            Ok(CachedVowelAnalysis {
                analysis,
                wave_cycle,
            })
        })();

        let mut guard = ANALYSIS_CACHE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        match result {
            Ok(data) => {
                guard.insert(key, AnalysisEntry::Ready(data));
            }
            Err(err) => {
                guard.insert(key, AnalysisEntry::Failed(err));
            }
        }
        ctx_clone.request_repaint();
    });

    None
}

/// Draws the interactive Praat-style F1 × F2 vowel formant editor, harmonic spectrum strip,
/// vowel waveform strip, and advanced formant controls (F3/Rhotic, Nasality, Singer's Formant).
#[allow(clippy::too_many_arguments)]
pub fn draw_vowel_formant_editor(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    lang: crate::config::AppLanguage,
    voicebank: Option<&Voicebank>,
    query_alias: &str,
    pitch_str: &str,
    selected_resampler: &str,
    f1_ratio: &mut f64,
    f2_ratio: &mut f64,
    f3_ratio: &mut f64,
    nasality: &mut f64,
    singers_formant: &mut f64,
    playback: Option<PlaybackFormantState>,
    animate_controls: bool,
) -> bool {
    let mut changed = false;
    let base_symbol = infer_base_vowel_symbol(query_alias);
    let canonical = canonical_vowel(base_symbol);

    let mut analysis_opt: Option<CachedVowelAnalysis> = None;
    let mut analysis_loading = false;
    let mut analysis_error: Option<String> = None;

    if let Some(vb) = voicebank {
        if let Some(entry) = vb.find_mapped_entry(query_alias, pitch_str) {
            let wav_path = vb.root_path.join(&entry.wav_filename);
            if wav_path.is_file() {
                match request_or_get_analysis(
                    ui.ctx(),
                    wav_path,
                    entry.offset,
                    entry.consonant,
                    entry.cutoff,
                ) {
                    Some(Ok(data)) => analysis_opt = Some(data),
                    Some(Err(err)) => analysis_error = Some(err),
                    None => analysis_loading = true,
                }
            }
        }
    }

    let (src_f0, src_f1, src_f2, src_f3) = if let Some(ref cached) = analysis_opt {
        (
            cached.analysis.f0_hz,
            cached.analysis.f1_hz.clamp(F1_MIN_HZ, F1_MAX_HZ),
            cached.analysis.f2_hz.clamp(F2_MIN_HZ, F2_MAX_HZ),
            cached.analysis.f3_hz.max(2200.0),
        )
    } else {
        let pitch_hz = crate::dsp::pitch::note_name_to_midi(pitch_str)
            .map(|m| crate::dsp::pitch::midi_to_freq(m as f64))
            .unwrap_or(220.0);
        (pitch_hz, canonical.f1_hz, canonical.f2_hz, 2850.0)
    };

    let singer_scale_f1 = (src_f1 / canonical.f1_hz).clamp(0.75, 1.30);
    let singer_scale_f2 = (src_f2 / canonical.f2_hz).clamp(0.75, 1.30);

    let is_tainah = selected_resampler.to_ascii_lowercase().contains("tainah")
        || selected_resampler.to_ascii_lowercase().contains("world");

    // Real-time animation physics during playback
    let pb = playback.unwrap_or_default();
    let is_animating = pb.is_playing;
    let live_weight = if is_animating {
        ui.ctx().request_repaint();
        let rel_ms = pb.playhead_ms - pb.note_pos_ms;
        let c_boundary = pb.consonant_ms.max(0.0);
        let early_lead = (c_boundary * 0.65).min(55.0);
        let t_start = (c_boundary - early_lead).max(0.0);
        let t_dur = 85.0_f64;
        if rel_ms < t_start {
            0.0
        } else if rel_ms < t_start + t_dur {
            let u = ((rel_ms - t_start) / t_dur).clamp(0.0, 1.0);
            0.5 - 0.5 * (std::f64::consts::PI * u).cos()
        } else {
            1.0
        }
    } else {
        1.0
    };

    // Live interpolated formants for smooth display
    let live_f1 = (src_f1 * f1_ratio.powf(live_weight)).clamp(F1_MIN_HZ, F1_MAX_HZ);
    let live_f2 = (src_f2 * f2_ratio.powf(live_weight)).clamp(F2_MIN_HZ, F2_MAX_HZ);
    let live_f3 = (src_f3 * f3_ratio.powf(live_weight)).clamp(1350.0, 4500.0);
    let live_nasality = (*nasality * live_weight).clamp(0.0, 1.0);
    let live_singers = (*singers_formant * live_weight).clamp(-1.0, 1.0);

    Frame::none()
        .fill(theme.elevated_surface_c32())
        .rounding(theme.ui_rounding())
        .stroke(theme.card_stroke())
        .inner_margin(egui::Margin::same(8.0))
        .show(ui, |ui| {
            // Header row
            ui.horizontal(|ui| {
                let title = if is_animating {
                    format!("▶ {} (F1 × F2 · BRAPA)", lang.tr("Ao Vivo", "Live"))
                } else {
                    lang.tr("Vogal & Formantes (F1 × F2 · BRAPA)", "Vowel & Formants (F1 × F2 · BRAPA)").to_string()
                };
                ui.label(
                    RichText::new(title)
                        .strong()
                        .size(11.0)
                        .color(if is_animating { Color32::from_rgb(112, 232, 192) } else { theme.accent_c32() }),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let has_mod = (*f1_ratio - 1.0).abs() > 1e-4
                        || (*f2_ratio - 1.0).abs() > 1e-4
                        || (*f3_ratio - 1.0).abs() > 1e-4
                        || *nasality > 1e-4
                        || singers_formant.abs() > 1e-4;
                    if has_mod
                        && ui
                            .small_button(lang.tr("Resetar", "Reset"))
                            .on_hover_text(lang.tr(
                                "Voltar para a vogal e formantes originais (1.0×)",
                                "Reset to original vowel and formants (1.0×)",
                            ))
                            .clicked()
                    {
                        *f1_ratio = 1.0;
                        *f2_ratio = 1.0;
                        *f3_ratio = 1.0;
                        *nasality = 0.0;
                        *singers_formant = 0.0;
                        changed = true;
                    }
                    let status_badge = if is_animating {
                        lang.tr("● 60 FPS Playback", "● 60 FPS Playback")
                    } else if analysis_loading {
                        lang.tr("Analisando Tainah…", "Analyzing Tainah…")
                    } else if analysis_opt.is_some() {
                        lang.tr("Tainah WORLD", "Tainah WORLD")
                    } else {
                        lang.tr("Ref. Canônica", "Canonical Ref.")
                    };
                    ui.label(
                        RichText::new(status_badge)
                            .size(9.0)
                            .monospace()
                            .color(if is_animating { Color32::from_rgb(112, 232, 192) } else { theme.text_muted_c32() }),
                    );
                });
            });

            if !is_tainah {
                ui.label(
                    RichText::new(lang.tr(
                        "Dica: selecione o resampler Tainah para sintetizar os formantes alterados.",
                        "Tip: select the Tainah resampler to synthesize modified formants.",
                    ))
                    .size(9.0)
                    .color(ui.visuals().warn_fg_color),
                );
            }
            if let Some(ref err) = analysis_error {
                ui.label(
                    RichText::new(err)
                        .size(8.5)
                        .color(theme.text_muted_c32()),
                );
            }

            ui.add_space(4.0);

            // Theme colors
            let display_bg = Color32::from_rgb(8, 12, 22);
            let border_teal = Color32::from_rgb(38, 118, 118);
            let corner_mint = Color32::from_rgb(112, 232, 192);
            let grid_col = Color32::from_rgba_unmultiplied(50, 95, 115, 70);
            let triangle_col = Color32::from_rgba_unmultiplied(64, 168, 160, 140);
            let f1_orange = Color32::from_rgb(242, 146, 68);
            let f2_cyan = Color32::from_rgb(86, 198, 245);
            let f3_violet = Color32::from_rgb(196, 128, 248);
            let nasal_lilac = Color32::from_rgb(228, 152, 255);
            let wave_mint = Color32::from_rgb(110, 236, 196);
            let gold_ring = Color32::from_rgb(245, 198, 92);
            let rhotic_cyan = Color32::from_rgb(90, 235, 255);

            let avail_w = ui.available_width().max(180.0);

            // =================================================================
            // 1. TOP PANEL: CARTESIAN PLANE F2 × F1 (Praat Vowel Chart)
            // =================================================================
            let chart_h = (avail_w * 0.62).clamp(142.0, 196.0);
            let (chart_rect, chart_resp) =
                ui.allocate_exact_size(Vec2::new(avail_w, chart_h), Sense::click_and_drag());
            let animation_time = ui.input(|input| input.time);
            if animate_controls {
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(33));
            }

            let mut target_f1 = (src_f1 * *f1_ratio).clamp(F1_MIN_HZ, F1_MAX_HZ);
            let mut target_f2 = (src_f2 * *f2_ratio).clamp(F2_MIN_HZ, F2_MAX_HZ);

            if chart_resp.double_clicked() || chart_resp.clicked_by(egui::PointerButton::Secondary)
            {
                *f1_ratio = 1.0;
                *f2_ratio = 1.0;
                *f3_ratio = 1.0;
                *nasality = 0.0;
                target_f1 = src_f1;
                target_f2 = src_f2;
                changed = true;
            } else if chart_resp.dragged() || chart_resp.clicked() {
                if let Some(pointer_pos) = chart_resp.interact_pointer_pos() {
                    let mut snapped_target: Option<VowelTarget> = None;
                    let shift_held = ui.input(|i| i.modifiers.shift);
                    let alt_held = ui.input(|i| i.modifiers.alt);

                    if chart_resp.clicked() && !chart_resp.dragged() {
                        for v in BRAPA_VOWEL_TARGETS {
                            let vf1 = (v.f1_hz * singer_scale_f1).clamp(F1_MIN_HZ, F1_MAX_HZ);
                            let vf2 = (v.f2_hz * singer_scale_f2).clamp(F2_MIN_HZ, F2_MAX_HZ);
                            let vpos = formant_to_pos(chart_rect, vf1, vf2);
                            if vpos.distance(pointer_pos) <= 13.0 {
                                snapped_target = Some(*v);
                                break;
                            }
                        }
                    }

                    let (mut new_f1, mut new_f2) = if let Some(target) = snapped_target {
                        *f3_ratio = target.f3_ratio;
                        *nasality = target.nasality;
                        (
                            (target.f1_hz * singer_scale_f1).clamp(F1_MIN_HZ, F1_MAX_HZ),
                            (target.f2_hz * singer_scale_f2).clamp(F2_MIN_HZ, F2_MAX_HZ),
                        )
                    } else {
                        pos_to_formant(chart_rect, pointer_pos)
                    };

                    if shift_held {
                        if let Some(best) = BRAPA_VOWEL_TARGETS.iter().min_by(|a, b| {
                            let af1 = a.f1_hz * singer_scale_f1;
                            let af2 = a.f2_hz * singer_scale_f2;
                            let bf1 = b.f1_hz * singer_scale_f1;
                            let bf2 = b.f2_hz * singer_scale_f2;
                            let da = (hz_to_bark(af1) - hz_to_bark(new_f1)).powi(2)
                                + (hz_to_bark(af2) - hz_to_bark(new_f2)).powi(2);
                            let db = (hz_to_bark(bf1) - hz_to_bark(new_f1)).powi(2)
                                + (hz_to_bark(bf2) - hz_to_bark(new_f2)).powi(2);
                            da.total_cmp(&db)
                        }) {
                            new_f1 = (best.f1_hz * singer_scale_f1).clamp(F1_MIN_HZ, F1_MAX_HZ);
                            new_f2 = (best.f2_hz * singer_scale_f2).clamp(F2_MIN_HZ, F2_MAX_HZ);
                            *f3_ratio = best.f3_ratio;
                            *nasality = best.nasality;
                        }
                    } else if alt_held {
                        let d = chart_resp.drag_delta();
                        if d.x.abs() > d.y.abs() {
                            new_f1 = target_f1;
                        } else {
                            new_f2 = target_f2;
                        }
                    }

                    if new_f2 < new_f1 + 220.0 {
                        new_f2 = (new_f1 + 220.0).min(F2_MAX_HZ);
                    }

                    *f1_ratio = (new_f1 / src_f1).clamp(0.5, 2.0);
                    *f2_ratio = (new_f2 / src_f2).clamp(0.5, 2.0);
                    target_f1 = (src_f1 * *f1_ratio).clamp(F1_MIN_HZ, F1_MAX_HZ);
                    target_f2 = (src_f2 * *f2_ratio).clamp(F2_MIN_HZ, F2_MAX_HZ);
                    changed = true;
                }
            }

            let painter = ui.painter_at(chart_rect);
            painter.rect_filled(chart_rect, Rounding::same(4.0), display_bg);
            painter.rect_stroke(
                chart_rect,
                Rounding::same(4.0),
                Stroke::new(1.0_f32, border_teal),
            );

            // Corner brackets
            let cl = 8.0_f32;
            let c_stroke = Stroke::new(1.8_f32, corner_mint);
            for (corner, dx, dy) in [
                (chart_rect.left_top(), cl, cl),
                (chart_rect.right_top(), -cl, cl),
                (chart_rect.left_bottom(), cl, -cl),
                (chart_rect.right_bottom(), -cl, -cl),
            ] {
                painter.line_segment([corner, Pos2::new(corner.x + dx, corner.y)], c_stroke);
                painter.line_segment([corner, Pos2::new(corner.x, corner.y + dy)], c_stroke);
            }

            // Grid
            for step in 1..4 {
                let t = step as f32 / 4.0;
                let y = chart_rect.top() + t * chart_rect.height();
                let x = chart_rect.left() + t * chart_rect.width();
                painter.line_segment(
                    [Pos2::new(chart_rect.left() + 4.0, y), Pos2::new(chart_rect.right() - 4.0, y)],
                    Stroke::new(0.7_f32, grid_col),
                );
                painter.line_segment(
                    [Pos2::new(x, chart_rect.top() + 4.0), Pos2::new(x, chart_rect.bottom() - 4.0)],
                    Stroke::new(0.7_f32, grid_col),
                );
            }

            // Axis labels
            painter.text(
                Pos2::new(chart_rect.center().x, chart_rect.top() + 10.0),
                egui::Align2::CENTER_CENTER,
                "← F2",
                FontId::monospace(10.5),
                f2_cyan,
            );
            painter.text(
                Pos2::new(chart_rect.left() + 14.0, chart_rect.center().y - 6.0),
                egui::Align2::LEFT_CENTER,
                "F1",
                FontId::monospace(10.5),
                f1_orange,
            );
            painter.text(
                Pos2::new(chart_rect.left() + 17.0, chart_rect.center().y + 6.0),
                egui::Align2::LEFT_CENTER,
                "↓",
                FontId::monospace(10.5),
                f1_orange,
            );

            // Vowel Triangle (Oral boundary i -> a -> u -> i)
            let pos_of = |sym: &str| -> Pos2 {
                let v = canonical_vowel(sym);
                formant_to_pos(
                    chart_rect,
                    (v.f1_hz * singer_scale_f1).clamp(F1_MIN_HZ, F1_MAX_HZ),
                    (v.f2_hz * singer_scale_f2).clamp(F2_MIN_HZ, F2_MAX_HZ),
                )
            };
            painter.line_segment([pos_of("i"), pos_of("a")], Stroke::new(1.2_f32, triangle_col));
            painter.line_segment([pos_of("a"), pos_of("u")], Stroke::new(1.2_f32, triangle_col));
            painter.line_segment([pos_of("u"), pos_of("i")], Stroke::new(1.2_f32, triangle_col));

            // Closest vowel target
            let closest_vowel = BRAPA_VOWEL_TARGETS
                .iter()
                .min_by(|a, b| {
                    let af1 = a.f1_hz * singer_scale_f1;
                    let af2 = a.f2_hz * singer_scale_f2;
                    let bf1 = b.f1_hz * singer_scale_f1;
                    let bf2 = b.f2_hz * singer_scale_f2;
                    let da = (hz_to_bark(af1) - hz_to_bark(target_f1)).powi(2)
                        + (hz_to_bark(af2) - hz_to_bark(target_f2)).powi(2);
                    let db = (hz_to_bark(bf1) - hz_to_bark(target_f1)).powi(2)
                        + (hz_to_bark(bf2) - hz_to_bark(target_f2)).powi(2);
                    da.total_cmp(&db)
                })
                .copied()
                .unwrap_or(canonical);

            // Draw all target vowels with distinct category colors
            for (index, v) in BRAPA_VOWEL_TARGETS.iter().enumerate() {
                let vf1 = (v.f1_hz * singer_scale_f1).clamp(F1_MIN_HZ, F1_MAX_HZ);
                let vf2 = (v.f2_hz * singer_scale_f2).clamp(F2_MIN_HZ, F2_MAX_HZ);
                let vpos = formant_to_pos(chart_rect, vf1, vf2);
                let phase = animation_time * 1.35 + index as f64 * 0.91;
                let dance = if animate_controls {
                    Vec2::new(
                        (phase.sin() * 2.8 + (phase * 0.63).cos() * 1.0) as f32,
                        ((phase * 0.82).cos() * 2.4 + (phase * 0.51).sin() * 0.8) as f32,
                    )
                } else {
                    Vec2::ZERO
                };
                let animated_pos = vpos + dance;
                let is_closest = v.symbol == closest_vowel.symbol;

                let (dot_col, text_col) = match v.category {
                    VowelCategory::Oral => {
                        if is_closest {
                            (corner_mint, corner_mint)
                        } else {
                            (Color32::from_rgba_unmultiplied(110, 220, 190, 170), Color32::from_rgb(130, 230, 200))
                        }
                    }
                    VowelCategory::CentralSchwa => {
                        (gold_ring, gold_ring)
                    }
                    VowelCategory::Nasal => {
                        (nasal_lilac, nasal_lilac)
                    }
                    VowelCategory::Rhotic => {
                        (rhotic_cyan, rhotic_cyan)
                    }
                };

                if animate_controls {
                    let breathe = ((phase * 0.72).sin() * 0.5 + 0.5) as f32;
                    painter.circle_stroke(
                        animated_pos,
                        4.0 + breathe * 1.8,
                        Stroke::new(
                            0.8_f32,
                            dot_col.gamma_multiply(0.25 + breathe * 0.25),
                        ),
                    );
                }
                painter.circle_filled(
                    animated_pos,
                    if is_closest { 3.4 } else { 2.2 },
                    dot_col,
                );
                painter.text(
                    Pos2::new(animated_pos.x, animated_pos.y - 8.5),
                    egui::Align2::CENTER_CENTER,
                    v.display_label,
                    FontId::monospace(if is_closest { 11.0 } else { 9.0 }),
                    text_col,
                );
            }

            let src_pos = formant_to_pos(chart_rect, src_f1, src_f2);
            let target_pos = formant_to_pos(chart_rect, target_f1, target_f2);

            if src_pos.distance(target_pos) > 2.0 {
                painter.circle_stroke(
                    src_pos,
                    4.5,
                    Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(255, 255, 255, 130)),
                );
                painter.line_segment(
                    [src_pos, target_pos],
                    Stroke::new(1.4_f32, gold_ring.gamma_multiply(0.75)),
                );
            }

            // Target ring
            painter.circle_stroke(target_pos, 18.0, Stroke::new(1.4_f32, gold_ring));
            painter.circle_filled(
                target_pos,
                6.5,
                Color32::from_rgba_unmultiplied(255, 225, 140, 90),
            );
            painter.circle_filled(target_pos, 4.0, Color32::from_rgb(255, 248, 220));

            // Animated live playback cursor
            if is_animating {
                let live_pos = formant_to_pos(chart_rect, live_f1, live_f2);
                let pulse = (ui.input(|i| i.time) * 6.0).sin().abs() as f32;
                painter.circle_stroke(
                    live_pos,
                    12.0 + pulse * 4.0,
                    Stroke::new(1.6_f32, corner_mint),
                );
                painter.circle_filled(
                    live_pos,
                    5.5,
                    Color32::from_rgb(120, 255, 210),
                );
                painter.circle_filled(
                    live_pos,
                    2.8,
                    Color32::WHITE,
                );
                if live_pos.distance(src_pos) > 3.0 {
                    painter.line_segment(
                        [src_pos, live_pos],
                        Stroke::new(2.2_f32, corner_mint.gamma_multiply(0.85)),
                    );
                }
            }

            let callout_offset = if target_pos.x > chart_rect.center().x {
                Vec2::new(-22.0, 10.0)
            } else {
                Vec2::new(22.0, 10.0)
            };
            painter.text(
                target_pos + callout_offset,
                egui::Align2::CENTER_CENTER,
                closest_vowel.display_label.to_uppercase(),
                FontId::monospace(13.0),
                Color32::WHITE,
            );

            // Status badge on the chart
            let f3_info = if (*f3_ratio - 1.0).abs() > 0.05 {
                format!(" · F3 {:.0}Hz ({:.2}× rótico)", live_f3, *f3_ratio)
            } else {
                String::new()
            };
            let nas_info = if *nasality > 0.05 {
                format!(" · nasal {:.0}%", *nasality * 100.0)
            } else {
                String::new()
            };
            painter.text(
                Pos2::new(chart_rect.left() + 8.0, chart_rect.bottom() - 9.0),
                egui::Align2::LEFT_CENTER,
                format!("F0 {:.0}Hz · base '{}'{f3_info}{nas_info}", src_f0, base_symbol),
                FontId::monospace(8.5),
                Color32::from_rgba_unmultiplied(170, 210, 220, 190),
            );

            ui.add_space(4.0);

            // =================================================================
            // 2. MIDDLE PANEL: HARMONIC SPECTRUM & F1/F2/F3/NASAL PEAKS
            // =================================================================
            let spec_h = 48.0;
            let (spec_rect, _) =
                ui.allocate_exact_size(Vec2::new(avail_w, spec_h), Sense::hover());
            let sp = ui.painter_at(spec_rect);
            sp.rect_filled(spec_rect, Rounding::same(3.0), display_bg);
            sp.rect_stroke(
                spec_rect,
                Rounding::same(3.0),
                Stroke::new(1.0_f32, border_teal),
            );

            let max_spec_hz = 4600.0_f64;
            let num_bars = 40usize;
            let bar_step = (spec_rect.width() - 12.0) / num_bars as f32;
            let z_nasal = (live_f1 * 1.38).clamp(650.0, 1180.0);
            for idx in 0..num_bars {
                let hz = (idx as f64 + 0.6) / num_bars as f64 * max_spec_hz;
                let g1 = (-((hz - live_f1) / 135.0).powi(2)).exp() * (1.0 - live_nasality * 0.35);
                let g2 = (-((hz - live_f2) / 200.0).powi(2)).exp() * 0.85;
                let g3 = (-((hz - live_f3) / 240.0).powi(2)).exp() * 0.55;
                // Nasal acoustics
                let nasal_pole = (-((hz - 270.0) / 95.0).powi(2)).exp() * (live_nasality * 0.7);
                let nasal_notch = (1.0 - (-((hz - z_nasal) / 125.0).powi(2)).exp() * (live_nasality * 0.6)).max(0.1);
                // Singer's formant ring (~3 kHz)
                let sf_boost = (-((hz - 3000.0) / 420.0).powi(2)).exp() * (live_singers * 0.45);
                let tilt = (1.0 - (hz / max_spec_hz) * 0.52).max(0.08);

                let amp = (((g1 + g2 + g3 + nasal_pole + sf_boost).max(0.10) * nasal_notch) * tilt).clamp(0.05, 1.0) as f32;
                let x = spec_rect.left() + 6.0 + idx as f32 * bar_step;
                let bh = amp * (spec_rect.height() - 10.0);

                let bar_col = if live_nasality > 0.3 && (hz - 270.0).abs() < 120.0 {
                    nasal_lilac
                } else if (hz - live_f1).abs() < 170.0 && g1 >= g2 {
                    f1_orange
                } else if (hz - live_f2).abs() < 240.0 {
                    f2_cyan
                } else if (hz - live_f3).abs() < 220.0 {
                    if *f3_ratio < 0.88 { rhotic_cyan } else { f3_violet }
                } else if live_singers.abs() > 0.25 && (hz - 3000.0).abs() < 350.0 {
                    gold_ring
                } else {
                    Color32::from_rgb(42, 132, 126)
                };

                sp.line_segment(
                    [
                        Pos2::new(x, spec_rect.bottom() - 3.0),
                        Pos2::new(x, spec_rect.bottom() - 3.0 - bh),
                    ],
                    Stroke::new(2.0_f32, bar_col),
                );
            }

            // Legend
            sp.text(
                Pos2::new(spec_rect.right() - 88.0, spec_rect.top() + 9.0),
                egui::Align2::RIGHT_CENTER,
                "▪F1",
                FontId::monospace(8.5),
                f1_orange,
            );
            sp.text(
                Pos2::new(spec_rect.right() - 62.0, spec_rect.top() + 9.0),
                egui::Align2::RIGHT_CENTER,
                "▪F2",
                FontId::monospace(8.5),
                f2_cyan,
            );
            sp.text(
                Pos2::new(spec_rect.right() - 36.0, spec_rect.top() + 9.0),
                egui::Align2::RIGHT_CENTER,
                if *f3_ratio < 0.88 { "▪F3(R)" } else { "▪F3" },
                FontId::monospace(8.5),
                if *f3_ratio < 0.88 { rhotic_cyan } else { f3_violet },
            );
            if *nasality > 0.1 {
                sp.text(
                    Pos2::new(spec_rect.right() - 6.0, spec_rect.top() + 9.0),
                    egui::Align2::RIGHT_CENTER,
                    "▪Nas",
                    FontId::monospace(8.5),
                    nasal_lilac,
                );
            }

            ui.add_space(4.0);

            // =================================================================
            // 3. BOTTOM PANEL: VOWEL WAVEFORM STRIP
            // =================================================================
            let wave_h = 36.0;
            let (wave_rect, _) =
                ui.allocate_exact_size(Vec2::new(avail_w, wave_h), Sense::hover());
            let wp = ui.painter_at(wave_rect);
            wp.rect_filled(wave_rect, Rounding::same(3.0), display_bg);
            wp.rect_stroke(
                wave_rect,
                Rounding::same(3.0),
                Stroke::new(1.0_f32, border_teal),
            );
            wp.line_segment(
                [
                    Pos2::new(wave_rect.left() + 2.0, wave_rect.center().y),
                    Pos2::new(wave_rect.right() - 2.0, wave_rect.center().y),
                ],
                Stroke::new(0.6_f32, grid_col),
            );

            let n_pts = 96usize;
            let f1_over_f0 = (live_f1 / src_f0.max(80.0)).clamp(1.2, 8.0);
            let f2_over_f0 = (live_f2 / src_f0.max(80.0)).clamp(2.5, 18.0);
            let f3_over_f0 = (live_f3 / src_f0.max(80.0)).clamp(4.0, 26.0);
            let mut wave_pts = Vec::with_capacity(n_pts);
            for i in 0..n_pts {
                let t = i as f64 / (n_pts - 1) as f64;
                let phase = t * 3.0 * std::f64::consts::TAU;
                let synth_val = (0.50 * (phase * f1_over_f0.round().max(1.0)).sin()
                    + 0.28 * (phase * f2_over_f0.round().max(2.0)).sin()
                    + 0.16 * (phase * f3_over_f0.round().max(3.0)).sin()
                    + 0.20 * phase.sin())
                    * (1.0 - 0.35 * (phase * 0.5).cos().abs());
                let sample = if let Some(ref cached) = analysis_opt {
                    if let Some(&orig) = cached.wave_cycle.get(i) {
                        let warp_mix = (((*f1_ratio - 1.0).abs() + (*f2_ratio - 1.0).abs() + (*f3_ratio - 1.0).abs() + *nasality) * 0.6)
                            .clamp(0.0, 0.85) as f32;
                        orig * (1.0 - warp_mix) + (synth_val as f32) * warp_mix
                    } else {
                        synth_val as f32
                    }
                } else {
                    synth_val as f32
                };
                let x = wave_rect.left() + 4.0 + (i as f32 / (n_pts - 1) as f32) * (wave_rect.width() - 8.0);
                let y = wave_rect.center().y - sample.clamp(-1.0, 1.0) * (wave_rect.height() * 0.38);
                wave_pts.push(Pos2::new(x, y));
            }
            wp.add(egui::Shape::line(wave_pts, Stroke::new(1.5_f32, wave_mint)));

            ui.add_space(5.0);

            // =================================================================
            // 4. PRESET BUTTONS (Orais · Schwa · Nasais · R Inglês)
            // =================================================================
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(3.0, 3.0);
                for v in BRAPA_VOWEL_TARGETS {
                    let vf1 = (v.f1_hz * singer_scale_f1).clamp(F1_MIN_HZ, F1_MAX_HZ);
                    let vf2 = (v.f2_hz * singer_scale_f2).clamp(F2_MIN_HZ, F2_MAX_HZ);
                    let r1 = (vf1 / src_f1).clamp(0.5, 2.0);
                    let r2 = (vf2 / src_f2).clamp(0.5, 2.0);
                    let is_active = (*f1_ratio - r1).abs() < 0.05
                        && (*f2_ratio - r2).abs() < 0.05
                        && (*f3_ratio - v.f3_ratio).abs() < 0.08
                        && (*nasality - v.nasality).abs() < 0.15;

                    let btn_color = match v.category {
                        VowelCategory::Oral => theme.accent_c32(),
                        VowelCategory::CentralSchwa => gold_ring,
                        VowelCategory::Nasal => nasal_lilac,
                        VowelCategory::Rhotic => rhotic_cyan,
                    };

                    let btn = egui::Button::new(
                        RichText::new(v.display_label)
                            .size(9.5)
                            .monospace()
                            .color(if is_active { Color32::BLACK } else { btn_color }),
                    );
                    if ui.add(btn).on_hover_text(lang.tr(v.hint_pt, v.hint_en)).clicked() {
                        *f1_ratio = r1;
                        *f2_ratio = r2;
                        *f3_ratio = v.f3_ratio;
                        *nasality = v.nasality;
                        changed = true;
                    }
                }
            });

            ui.add_space(4.0);

            // =================================================================
            // 5. NUMERIC F1 & F2 CONTROLS
            // =================================================================
            ui.columns(2, |cols| {
                let mut f1_hz_edit = (src_f1 * *f1_ratio).round();
                if cols[0]
                    .add_sized(
                        Vec2::new(cols[0].available_width(), 18.0),
                        egui::DragValue::new(&mut f1_hz_edit)
                            .range(F1_MIN_HZ..=F1_MAX_HZ)
                            .speed(3.0)
                            .prefix("F1: ")
                            .suffix(format!(" Hz ({:.2}×)", *f1_ratio)),
                    )
                    .changed()
                {
                    *f1_ratio = (f1_hz_edit / src_f1).clamp(0.5, 2.0);
                    changed = true;
                }

                let mut f2_hz_edit = (src_f2 * *f2_ratio).round();
                if cols[1]
                    .add_sized(
                        Vec2::new(cols[1].available_width(), 18.0),
                        egui::DragValue::new(&mut f2_hz_edit)
                            .range(F2_MIN_HZ..=F2_MAX_HZ)
                            .speed(6.0)
                            .prefix("F2: ")
                            .suffix(format!(" Hz ({:.2}×)", *f2_ratio)),
                    )
                    .changed()
                {
                    *f2_ratio = (f2_hz_edit / src_f2).clamp(0.5, 2.0);
                    changed = true;
                }
            });

            ui.add_space(4.0);

            // =================================================================
            // 6. EXTENDED SLIDERS: NASALIDADE, F3 / R INGLÊS, CANTOR ~3 kHz
            // =================================================================
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(lang.tr("Nasalidade (˜):", "Nasality (˜):"))
                        .size(9.5)
                        .color(nasal_lilac),
                );
                let nas_s = ui.add_sized(
                    Vec2::new(ui.available_width() - 48.0, 18.0),
                    egui::Slider::new(nasality, 0.0..=1.0)
                        .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
                        .step_by(0.01),
                );
                if nas_s.changed() {
                    changed = true;
                }
                if ui.small_button(if *nasality > 0.05 { "Oral" } else { "85%" }).clicked() {
                    *nasality = if *nasality > 0.05 { 0.0 } else { 0.85 };
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(lang.tr("F3 · R Inglês (ɚ):", "F3 · English R (ɚ):"))
                        .size(9.5)
                        .color(if *f3_ratio < 0.88 { rhotic_cyan } else { f3_violet }),
                );
                let f3_s = ui.add_sized(
                    Vec2::new(ui.available_width() - 56.0, 18.0),
                    egui::Slider::new(f3_ratio, 0.55..=1.40)
                        .suffix("×")
                        .step_by(0.01),
                );
                if f3_s.changed() {
                    changed = true;
                }
                let is_rhotic = (*f3_ratio - 0.68).abs() < 0.05;
                if ui.small_button(RichText::new(if is_rhotic { "1.0×" } else { "ɚ (0.68×)" }).color(rhotic_cyan)).clicked() {
                    *f3_ratio = if is_rhotic { 1.0 } else { 0.68 };
                    changed = true;
                }
            });

            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(lang.tr("Cantor (~3 kHz):", "Singer's (~3 kHz):"))
                        .size(9.5)
                        .color(gold_ring),
                );
                let sf_s = ui.add_sized(
                    Vec2::new(ui.available_width() - 48.0, 18.0),
                    egui::Slider::new(singers_formant, -1.0..=1.0)
                        .custom_formatter(|v, _| format!("{:.0}%", v * 100.0))
                        .step_by(0.02),
                );
                if sf_s.changed() {
                    changed = true;
                }
                if ui.small_button("0%").clicked() {
                    *singers_formant = 0.0;
                    changed = true;
                }
            });
        });

    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bark_round_trip_and_plane_mapping_are_invertible() {
        let rect = Rect::from_min_size(Pos2::new(10.0, 20.0), Vec2::new(240.0, 160.0));
        for &(f1, f2) in &[(280.0, 2300.0), (500.0, 1500.0), (780.0, 1320.0), (300.0, 750.0)] {
            let pos = formant_to_pos(rect, f1, f2);
            let (r1, r2) = pos_to_formant(rect, pos);
            assert!((r1 - f1).abs() < 1.5, "f1={f1} got {r1}");
            assert!((r2 - f2).abs() < 2.5, "f2={f2} got {r2}");
        }
    }

    #[test]
    fn infers_base_vowel_from_japanese_brapa_and_nasal_lyrics() {
        assert_eq!(infer_base_vowel_symbol("ka"), "a");
        assert_eq!(infer_base_vowel_symbol("- か"), "a");
        assert_eq!(infer_base_vowel_symbol("shi"), "i");
        assert_eq!(infer_base_vowel_symbol("tsu"), "u");
        assert_eq!(infer_base_vowel_symbol("p eh"), "eh");
        assert_eq!(infer_base_vowel_symbol("s oh"), "oh");
        assert_eq!(infer_base_vowel_symbol("kan"), "an");
        assert_eq!(infer_base_vowel_symbol("sim"), "in");
        assert_eq!(infer_base_vowel_symbol("her"), "ɚ");
        assert_eq!(infer_base_vowel_symbol("car"), "a");
    }
}
