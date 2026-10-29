//! Chinese and Cantonese syllable primitives.
//!
//! OpenUtau romanizes Hanzi before its built-in Chinese phonemizers run.  The
//! Rust side keeps that conversion in the existing pinyin/Jyutping data path
//! and implements the alias topology here so Chinese modes do not fall back
//! to treating a whole Hanzi string as one opaque alias.

use super::{JapanesePhonemizer, PhonemizerMode, RenderPhone};
use crate::oto::Voicebank;
use crate::project::model::UNote;
use std::collections::{HashMap, HashSet};

const PINYIN_INITIALS: [&str; 23] = [
    "zh", "ch", "sh", "b", "p", "m", "f", "d", "t", "n", "l", "g", "k", "h", "j", "q", "x", "z",
    "c", "s", "r", "y", "w",
];

fn strip_tone(token: &str) -> String {
    token
        .trim()
        .trim_end_matches(|c: char| c.is_ascii_digit())
        .to_owned()
}

fn split_syllable(token: &str) -> (&str, &str) {
    let token = token.trim();
    for initial in PINYIN_INITIALS {
        if let Some(rime) = token.strip_prefix(initial) {
            return (initial, rime);
        }
    }
    ("", token)
}

fn tail(token: &str) -> Option<&'static str> {
    let pure = strip_tone(token);
    let (initial, mut rime) = split_syllable(&pure);
    // ChineseCVVG2p applies the j/q/x/y ü notation before looking up the
    // rime tail. This is observable for `xuan`, `jun`, and `yan`.
    if matches!(initial, "j" | "q" | "x" | "y") {
        if rime == "un" || rime == "uan" {
            rime = if rime == "un" { "vn" } else { "van" };
        } else if initial == "y" && rime == "an" {
            rime = "ian";
        }
    }
    match rime {
        "ai" | "uai" => Some("_ai"),
        "an" | "uan" => Some("_an"),
        "ian" | "van" => Some("_en2"),
        "ang" | "iang" | "uang" => Some("_ang"),
        "ao" => Some("_ao"),
        "ei" => Some("_ei"),
        "en" => Some("_en"),
        "eng" => Some("_eng"),
        "ong" => Some("_ong"),
        "ou" | "iu" => Some("_ou"),
        "un" | "uen" => Some("_un"),
        "vn" => Some("_vn"),
        "in" => Some("_in"),
        "ing" => Some("_ing"),
        _ => None,
    }
}

fn resolve(vb: &Voicebank, candidate: String, note: &UNote) -> String {
    vb.find_mapped_entry(&candidate, &note.pitch)
        .map(|entry| entry.alias.clone())
        .unwrap_or(candidate)
}

fn phone(
    note: &UNote,
    note_index: usize,
    alias: String,
    position_ms: f64,
    duration_ms: f64,
) -> RenderPhone {
    RenderPhone {
        note_index,
        lyric: alias,
        pitch: note.pitch.clone(),
        position_ms,
        duration_ms,
        envelope: note.envelope.clone(),
        expressions: note.expressions.clone(),
        pitch_bend: note.pitch_bend.clone(),
        vibrato: note.vibrato.clone(),
        flags: note.flags.clone(),
        resolved_alias: None,
        resolved_timbre: note.timbre.clone(),
        coda_truncation: false,
    }
}

fn note_tokens(note: &UNote, mode: PhonemizerMode) -> Vec<String> {
    note.phonetic_hint
        .as_deref()
        .map(|hint| {
            hint.split([',', ' '])
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_else(|| JapanesePhonemizer::latin_language_tokens(&note.lyric, mode))
}

fn push_aliases(
    output: &mut Vec<RenderPhone>,
    note: &UNote,
    note_index: usize,
    aliases: Vec<String>,
) {
    let duration = note.duration_ms / aliases.len().max(1) as f64;
    for (index, alias) in aliases.into_iter().enumerate() {
        output.push(phone(
            note,
            note_index,
            alias,
            note.position_ms + index as f64 * duration,
            duration,
        ));
    }
}

#[derive(Debug, Clone)]
struct CvvPlusConfig {
    vowel_tail_prefix: String,
    use_single_nasal_vowel: bool,
    use_single_multiple_vowel: bool,
    use_retan: bool,
    supported_tail_breath: HashSet<String>,
    fast_tail_vowel_timing_tick: f64,
    single_vowels_reference_timing_tick: f64,
    fast_tail_vowel_dict: HashMap<String, String>,
    slow_tail_vowel_dict: HashMap<String, String>,
    nasal_vowels: HashSet<String>,
    multiple_vowels: HashSet<String>,
}

impl Default for CvvPlusConfig {
    fn default() -> Self {
        let fast_tail_vowel_dict = [
            ("ia", "ia"),
            ("ie", "ie"),
            ("ua", "ua"),
            ("uo", "uo"),
            ("ve", "ve"),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
        let slow_tail_vowel_dict = [
            ("ai", "ai"),
            ("ei", "ei"),
            ("ao", "ao"),
            ("ou", "ou"),
            ("an", "an"),
            ("en", "en"),
            ("ang", "ang"),
            ("eng", "eng"),
            ("ong", "ong"),
            ("iao", "ao"),
            ("iu", "ou"),
            ("iou", "ou"),
            ("ian", "ian"),
            ("in", "in"),
            ("iang", "ang"),
            ("ing", "ing"),
            ("iong", "ong"),
            ("uai", "ai"),
            ("ui", "ei"),
            ("uei", "ei"),
            ("uan", "an"),
            ("un", "uen"),
            ("uang", "ang"),
            ("ueng", "eng"),
            ("van", "en"),
            ("vn", "vn"),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect();
        Self {
            vowel_tail_prefix: "_".to_string(),
            use_single_nasal_vowel: false,
            use_single_multiple_vowel: false,
            use_retan: false,
            supported_tail_breath: ["-".to_string()].into_iter().collect(),
            fast_tail_vowel_timing_tick: 100.0,
            single_vowels_reference_timing_tick: 480.0,
            nasal_vowels: [
                "an", "en", "ang", "eng", "ong", "ian", "iang", "ing", "iong", "uan", "uen", "un",
                "uang", "ueng", "van", "vn",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            multiple_vowels: [
                "ai", "ei", "ao", "ou", "ia", "iao", "ie", "iou", "ua", "uo", "uai", "uei", "ui",
                "ve",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            fast_tail_vowel_dict,
            slow_tail_vowel_dict,
        }
    }
}

fn yaml_string_map(value: Option<&yaml_serde::Value>) -> HashMap<String, String> {
    value
        .and_then(yaml_serde::Value::as_mapping)
        .map(|mapping| {
            mapping
                .iter()
                .filter_map(|(key, value)| {
                    Some((key.as_str()?.to_string(), value.as_str()?.to_string()))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn load_cvv_plus_config(vb: &Voicebank) -> CvvPlusConfig {
    let mut config = CvvPlusConfig::default();
    let Ok(contents) = std::fs::read_to_string(vb.root_path.join("zhcvvplus.yaml")) else {
        return config;
    };
    let Ok(value) = yaml_serde::from_str::<yaml_serde::Value>(&contents) else {
        return config;
    };
    if let Some(value) = value.get("VowelTailPrefix").and_then(|v| v.as_str()) {
        config.vowel_tail_prefix = value.to_string();
    }
    if let Some(value) = value.get("UseSingleNasalVowel").and_then(|v| v.as_bool()) {
        config.use_single_nasal_vowel = value;
    }
    if let Some(value) = value
        .get("UseSingleMultipleVowel")
        .and_then(|v| v.as_bool())
    {
        config.use_single_multiple_vowel = value;
    }
    if let Some(value) = value.get("UseRetan").and_then(|v| v.as_bool()) {
        config.use_retan = value;
    }
    if let Some(value) = value
        .get("SupportedTailBreath")
        .and_then(|v| v.as_sequence())
    {
        config.supported_tail_breath = value
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect();
    }
    if let Some(value) = value
        .get("FastTailVowelTimingTick")
        .and_then(|v| v.as_f64())
    {
        config.fast_tail_vowel_timing_tick = value;
    }
    if let Some(value) = value
        .get("SingleVowelsReferenceTimimgTick")
        .and_then(|v| v.as_f64())
    {
        config.single_vowels_reference_timing_tick = value;
    }
    let fast = yaml_string_map(value.get("FastTailVowelDict"));
    if !fast.is_empty() {
        config.fast_tail_vowel_dict = fast;
    }
    let slow = yaml_string_map(value.get("SlowTailVowelDict"));
    if !slow.is_empty() {
        config.slow_tail_vowel_dict = slow;
    }
    if let Some(value) = value.get("NasalVowelDict").and_then(|v| v.as_sequence()) {
        config.nasal_vowels = value
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect();
    }
    if let Some(value) = value.get("MultipleVowelDict").and_then(|v| v.as_sequence()) {
        config.multiple_vowels = value
            .iter()
            .filter_map(|item| item.as_str().map(str::to_string))
            .collect();
    }
    config
}

fn cvv_plus_vowel(lyric: &str) -> String {
    let mut value = lyric.trim().to_string();
    while value
        .chars()
        .next()
        .is_some_and(|c| !c.is_ascii_alphabetic())
    {
        value.remove(0);
    }
    let mut initial = String::new();
    for candidate in PINYIN_INITIALS {
        if value.starts_with(candidate) {
            initial = candidate.to_string();
            value = value[candidate.len()..].to_string();
            break;
        }
    }
    let _ = initial;
    value.replace("yu", "v").replace('y', "i").replace('w', "u")
}

pub fn apply_cvv_plus(notes: &[UNote], vb: &Voicebank) -> Vec<RenderPhone> {
    let config = load_cvv_plus_config(vb);
    let mut output = Vec::new();
    let mut previous_vowel: Option<String> = None;
    let mut previous_end = None;
    for (note_index, note) in notes.iter().enumerate() {
        let lyric = note.lyric.trim();
        if lyric.is_empty() || lyric.eq_ignore_ascii_case("r") {
            previous_vowel = None;
            previous_end = None;
            continue;
        }
        let connected = previous_end.is_some_and(|end| note.position_ms <= end + 0.001);
        if let Some(hint) = note.phonetic_hint.as_deref() {
            let hints = hint
                .split(',')
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .collect::<Vec<_>>();
            if !hints.is_empty() {
                let duration = note.duration_ms / hints.len() as f64;
                for (index, hint) in hints.iter().enumerate() {
                    output.push(phone(
                        note,
                        note_index,
                        resolve(vb, (*hint).to_string(), note),
                        note.position_ms + index as f64 * duration,
                        duration,
                    ));
                }
                previous_vowel = Some(cvv_plus_vowel(hints.last().copied().unwrap_or("")));
                previous_end = Some(note.position_ms + note.duration_ms);
                continue;
            }
        }

        let vowel = cvv_plus_vowel(lyric);
        if config.supported_tail_breath.contains(lyric) && connected {
            let candidate = previous_vowel
                .as_deref()
                .map(|previous| format!("{previous} {lyric}"))
                .unwrap_or_else(|| lyric.to_string());
            output.push(phone(
                note,
                note_index,
                resolve(vb, candidate, note),
                note.position_ms,
                note.duration_ms,
            ));
        } else {
            let mut aliases = Vec::new();
            let base = if config.use_retan && !connected {
                let retan = format!("- {lyric}");
                if vb.find_mapped_entry(&retan, &note.pitch).is_some() {
                    retan
                } else {
                    lyric.to_string()
                }
            } else {
                lyric.to_string()
            };
            aliases.push((base, 0.0));
            let tail_value = config
                .fast_tail_vowel_dict
                .get(&vowel)
                .or_else(|| config.slow_tail_vowel_dict.get(&vowel));
            let is_short_single = note.duration_ms <= config.single_vowels_reference_timing_tick
                && ((config.use_single_nasal_vowel && config.nasal_vowels.contains(&vowel))
                    || (config.use_single_multiple_vowel
                        && config.multiple_vowels.contains(&vowel)));
            let use_tail = tail_value.is_some()
                && ((config.nasal_vowels.contains(&vowel) && !config.use_single_nasal_vowel)
                    || (config.multiple_vowels.contains(&vowel)
                        && !config.use_single_multiple_vowel)
                    || is_short_single);
            if let Some(tail_value) = tail_value.filter(|_| use_tail) {
                let tail_alias = format!("{}{tail_value}", config.vowel_tail_prefix);
                let tail_position = if config.fast_tail_vowel_dict.contains_key(&vowel) {
                    note.duration_ms * (config.fast_tail_vowel_timing_tick / 480.0)
                } else {
                    note.duration_ms * (2.0 / 3.0)
                };
                aliases.push((tail_alias, tail_position));
            }
            for (alias, offset) in aliases {
                output.push(phone(
                    note,
                    note_index,
                    resolve(vb, alias, note),
                    note.position_ms + offset,
                    note.duration_ms - offset,
                ));
            }
        }
        previous_vowel = Some(vowel);
        previous_end = Some(note.position_ms + note.duration_ms);
    }
    output
}

pub fn apply(notes: &[UNote], vb: &Voicebank, mode: PhonemizerMode) -> Vec<RenderPhone> {
    let mut output = Vec::new();
    let mut previous_tail: Option<String> = None;
    let mut previous_end = None;

    for (note_index, note) in notes.iter().enumerate() {
        let lyric = note.lyric.trim();
        if lyric.is_empty() || lyric.eq_ignore_ascii_case("r") {
            previous_tail = None;
            previous_end = None;
            continue;
        }
        let tokens = note_tokens(note, mode);
        if tokens.is_empty() {
            continue;
        }
        let connected = previous_end.is_some_and(|end| note.position_ms <= end + 0.001);
        let mut aliases = Vec::new();

        for token in tokens {
            let pure = strip_tone(&token);
            let (initial, _) = split_syllable(&pure);
            match mode {
                PhonemizerMode::ChineseCVV => {
                    aliases.push(resolve(vb, pure.clone(), note));
                    if let Some(vowel_tail) = tail(&pure) {
                        aliases.push(resolve(vb, vowel_tail.to_owned(), note));
                    }
                }
                PhonemizerMode::ChineseCVVPlus => {
                    aliases.push(resolve(vb, pure.clone(), note));
                    if let Some(vowel_tail) = tail(&pure) {
                        aliases.push(resolve(vb, vowel_tail.to_owned(), note));
                    }
                }
                PhonemizerMode::ChineseVCV => {
                    let base = if connected {
                        previous_tail
                            .as_deref()
                            .map(|previous| format!("{previous} {pure}"))
                            .unwrap_or_else(|| format!("- {pure}"))
                    } else {
                        format!("- {pure}")
                    };
                    aliases.push(resolve(vb, base, note));
                }
                PhonemizerMode::ChineseCVVC | PhonemizerMode::CantoneseCVVC => {
                    let base = resolve(vb, pure.clone(), note);
                    if !initial.is_empty() {
                        let transition = previous_tail
                            .as_deref()
                            .filter(|_| connected)
                            .map(|previous| format!("{previous} {initial}"))
                            .unwrap_or_else(|| format!("- {initial}"));
                        if vb.find_mapped_entry(&transition, &note.pitch).is_some() {
                            aliases.push(resolve(vb, transition, note));
                        }
                    }
                    aliases.push(base);
                }
                PhonemizerMode::CantoneseSyo => unreachable!("Syo has a dedicated topology"),
                _ => aliases.push(resolve(vb, pure.clone(), note)),
            }

            previous_tail = tail(&pure).map(str::to_owned);
        }

        push_aliases(&mut output, note, note_index, aliases);
        previous_end = Some(note.position_ms + note.duration_ms);
    }
    output
}

const CANTONESE_INITIALS: [&str; 19] = [
    "gw", "kw", "ng", "b", "p", "m", "f", "d", "t", "n", "l", "g", "k", "h", "w", "z", "c", "s",
    "j",
];

fn cantonese_syo_split(token: &str) -> (String, String) {
    let token = strip_tone(token);
    for initial in CANTONESE_INITIALS {
        if let Some(final_part) = token.strip_prefix(initial) {
            if initial != "ng" || !final_part.is_empty() {
                return (initial.to_string(), final_part.to_string());
            }
        }
    }
    (String::new(), token)
}

fn cantonese_syo_tail(final_part: &str) -> Option<&'static str> {
    Some(match final_part {
        "aap" => "aa p",
        "aat" => "aa t",
        "aak" => "aa k",
        "aam" => "aa m",
        "aan" => "aa n",
        "aang" => "aa ng",
        "aai" => "aa i",
        "aau" => "aa u",
        "ap" => "a p",
        "at" => "a t",
        "ak" => "a k",
        "am" => "a m",
        "an" => "a n",
        "ang" => "a ng",
        "ai" => "a i",
        "au" => "a u",
        "op" => "o p",
        "ot" => "o t",
        "ok" => "o k",
        "om" => "o m",
        "on" => "o n",
        "ong" => "o ng",
        "oi" => "o i",
        "ou" => "o u",
        "oet" => "oe t",
        "oek" => "oe k",
        "oeng" => "oe ng",
        "oei" => "oe i",
        "eot" => "eo t",
        "eon" => "eo n",
        "eoi" => "eo i",
        "ep" => "e p",
        "et" => "e t",
        "ek" => "e k",
        "em" => "e m",
        "en" => "e n",
        "eng" => "e ng",
        "ei" => "e i",
        "eu" => "e u",
        "up" => "u p",
        "ut" => "u t",
        "um" => "um",
        "un" => "u n",
        "ung" => "uu ng",
        "ui" => "u i",
        "yut" => "yu t",
        "yun" => "yu n",
        "ip" => "i p",
        "it" => "i t",
        "ik" => "ii k",
        "im" => "i m",
        "in" => "i n",
        "ing" => "ii ng",
        "iu" => "i u",
        _ => return None,
    })
}

/// CantoneseSyo is not the Chinese CVV tail algorithm. OpenUtau first keeps
/// the complete Jyutping syllable, then optionally emits its Syo-style split
/// (`jyut` -> `jyu`, `yu t`) and only uses a leading alias at phrase starts.
pub fn apply_cantonese_syo(notes: &[UNote], vb: &Voicebank) -> Vec<RenderPhone> {
    let mut output = Vec::new();
    for (note_index, note) in notes.iter().enumerate() {
        let lyric = note.lyric.trim();
        if lyric.is_empty() || lyric.eq_ignore_ascii_case("r") {
            continue;
        }
        let tokens = note_tokens(note, PhonemizerMode::CantoneseSyo);
        let duration = note.duration_ms / tokens.len().max(1) as f64;
        for (token_index, token) in tokens.iter().enumerate() {
            let pure = strip_tone(token);
            let (initial, final_part) = cantonese_syo_split(&pure);
            let split = cantonese_syo_tail(&final_part);
            let mut aliases = vec![if initial.is_empty() {
                pure.clone()
            } else {
                format!("{initial}{final_part}")
            }];
            if let Some(tail_alias) = split {
                aliases.push(tail_alias.to_string());
            }
            let alias_duration = duration / aliases.len() as f64;
            for (alias_index, candidate) in aliases.into_iter().enumerate() {
                let candidate = if token_index == 0 && alias_index == 0 {
                    let initial_candidate = format!("- {candidate}");
                    if vb
                        .find_mapped_entry(&initial_candidate, &note.pitch)
                        .is_some()
                    {
                        initial_candidate
                    } else {
                        candidate
                    }
                } else {
                    candidate
                };
                output.push(phone(
                    note,
                    note_index,
                    resolve(vb, candidate, note),
                    note.position_ms
                        + token_index as f64 * duration
                        + alias_index as f64 * alias_duration,
                    alias_duration,
                ));
            }
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::{apply, apply_cantonese_syo, apply_cvv_plus};
    use crate::oto::Voicebank;
    use crate::phonemizer::PhonemizerMode;
    use crate::project::model::UNote;

    fn voicebank_with_aliases(aliases: &[&str]) -> (tempfile::TempDir, Voicebank) {
        let directory = tempfile::tempdir().unwrap();
        let mut voicebank = Voicebank::new(directory.path()).unwrap();
        for alias in aliases {
            voicebank.entries.insert(
                (*alias).to_string(),
                crate::oto::OtoEntry::new(
                    format!("{alias}.wav"),
                    (*alias).to_string(),
                    0.0,
                    80.0,
                    10.0,
                    0.0,
                    0.0,
                ),
            );
        }
        (directory, voicebank)
    }

    #[test]
    fn chinese_cvv_splits_the_pinyin_rime_tail() {
        let (_directory, voicebank) = voicebank_with_aliases(&["duang", "_ang"]);
        let mut note = UNote::new("duang", "C4", 0.0, 300.0);
        note.phonetic_hint = Some("duang".to_string());
        let phones = apply(&[note], &voicebank, PhonemizerMode::ChineseCVV);
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/openutau_compatibility/chinese_cvv.json"
        ))
        .unwrap();
        let expected = fixture["cases"][0]["expected_aliases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|alias| alias.as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            phones
                .iter()
                .map(|phone| phone.lyric.as_str())
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn cantonese_syo_uses_jyutping_split_table() {
        let (_directory, voicebank) = voicebank_with_aliases(&["jyut", "jyu", "yu t"]);
        let mut note = UNote::new("jyut", "C4", 0.0, 300.0);
        note.phonetic_hint = Some("jyut".to_string());
        let phones = apply_cantonese_syo(&[note], &voicebank);
        assert_eq!(
            phones
                .iter()
                .map(|phone| phone.lyric.as_str())
                .collect::<Vec<_>>(),
            ["jyut", "yu t"]
        );
    }

    #[test]
    fn chinese_cvv_matches_openutau_special_rime_rules() {
        let (_directory, voicebank) = voicebank_with_aliases(&["dian", "_en2", "xuan", "ma"]);
        let mut dian = UNote::new("ignored", "C4", 0.0, 300.0);
        dian.phonetic_hint = Some("dian".to_string());
        let mut xuan = UNote::new("ignored", "C4", 300.0, 300.0);
        xuan.phonetic_hint = Some("xuan".to_string());
        let mut ma = UNote::new("ignored", "C4", 600.0, 300.0);
        ma.phonetic_hint = Some("ma".to_string());
        let phones = apply(&[dian, xuan, ma], &voicebank, PhonemizerMode::ChineseCVV);
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/openutau_compatibility/chinese_cvv.json"
        ))
        .unwrap();
        let expected = fixture["cases"][1]["expected_aliases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|alias| alias.as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            phones
                .iter()
                .map(|phone| phone.lyric.as_str())
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn chinese_cvv_plus_reads_config_and_preserves_comma_hints() {
        let (directory, mut voicebank) =
            voicebank_with_aliases(&["- ma", "duang", "_ang", "_custom"]);
        std::fs::write(
            directory.path().join("zhcvvplus.yaml"),
            "UseRetan: true\nVowelTailPrefix: _\nSlowTailVowelDict: {uang: ang}\n",
        )
        .unwrap();
        voicebank.root_path = directory.path().to_path_buf();

        let mut first = UNote::new("ma", "C4", 0.0, 300.0);
        first.phonetic_hint = Some("_custom, _custom".to_string());
        let second = UNote::new("duang", "C4", 300.0, 300.0);
        let phones = apply_cvv_plus(&[first, second], &voicebank);
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/openutau_compatibility/chinese_cvv_plus.json"
        ))
        .unwrap();
        assert_eq!(fixture["upstream"]["class"], "ChineseCVVPlusPhonemizer");
        let expected = fixture["cases"][0]["expected_aliases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|alias| alias.as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            phones
                .iter()
                .map(|phone| phone.lyric.as_str())
                .collect::<Vec<_>>(),
            expected
        );
    }
}
