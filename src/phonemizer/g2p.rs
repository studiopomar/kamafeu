use super::*;
use std::collections::HashMap;
use std::sync::OnceLock;

static OPENUTAU_G2P_DICTIONARIES: OnceLock<HashMap<PhonemizerMode, HashMap<String, Vec<String>>>> =
    OnceLock::new();
static CANTONESE_DICTIONARY: OnceLock<HashMap<String, Vec<String>>> = OnceLock::new();

pub(super) fn apply_rule_g2p(
    notes: &[UNote],
    vb: &Voicebank,
    mode: PhonemizerMode,
) -> Vec<RenderPhone> {
    let arabic = mode == PhonemizerMode::ArabicG2P;
    let icelandic = mode == PhonemizerMode::IcelandicG2P;
    let chinese_like = matches!(
        mode,
        PhonemizerMode::ChineseG2P | PhonemizerMode::CantoneseG2P
    );
    let mapping: std::collections::HashMap<String, Vec<String>> = serde_json::from_str(if arabic {
        include_str!("data/arabic-mapping.json")
    } else {
        include_str!("data/icelandic-mapping.json")
    })
    .unwrap_or_default();
    let mut phones: Vec<RenderPhone> = Vec::new();
    let cvvc = matches!(
        mode,
        PhonemizerMode::TurkishCVVC
            | PhonemizerMode::ChineseCVVC
            | PhonemizerMode::CantoneseCVVC
            | PhonemizerMode::FrenchCVVC
            | PhonemizerMode::FrenchVCCV
            | PhonemizerMode::GermanVCCV
            | PhonemizerMode::GermanDiphone
            | PhonemizerMode::RussianVCCV
            | PhonemizerMode::SpanishVCCV
    );
    let mut previous_vowel: Option<String> = None;
    for (note_index, note) in notes.iter().enumerate() {
        let word = note.lyric.trim();
        if word.is_empty() || word.eq_ignore_ascii_case("r") {
            previous_vowel = None;
            continue;
        }
        let tokens = if arabic {
            arabic_tokens(word)
        } else if icelandic {
            icelandic_tokens(word)
        } else if chinese_like {
            latin_language_tokens(word, mode)
        } else {
            openutau_dictionary_tokens(word, mode).unwrap_or_else(|| {
                    crate::terminal_log::debug(
                        "Phonemizer",
                        format!("mode={mode:?} word={word:?} fallback=passthrough reason=missing_dictionary_entry"),
                    );
                    // An unknown word is not converted by a Kamafeu-specific
                    // grapheme table. Keep the user's token intact until an
                    // official OpenUtau G2P dictionary/provider resolves it.
                    EnglishPhonemizer::phonetic_tokens(word)
                })
        };
        if tokens.is_empty() {
            continue;
        }
        let dur = note.duration_ms / tokens.len() as f64;
        for (index, token) in tokens.iter().enumerate() {
            if cvvc && index == 0 {
                if let Some(vowel) = previous_vowel.as_deref() {
                    if !is_rule_vowel(token) {
                        let vc = format!("{} {}", vowel, token);
                        if let Some(entry) = vb.find_mapped_entry(&vc, &note.pitch) {
                            let transition_duration = dur.min((note.duration_ms * 0.35).max(20.0));
                            phones.push(RenderPhone {
                                note_index,
                                lyric: entry.alias.clone(),
                                pitch: note.pitch.clone(),
                                position_ms: note.position_ms - transition_duration,
                                duration_ms: transition_duration,
                                envelope: note.envelope.clone(),
                                expressions: note.expressions.clone(),
                                pitch_bend: note.pitch_bend.clone(),
                                vibrato: note.vibrato.clone(),
                                flags: note.flags.clone(),
                                resolved_alias: None,
                                resolved_timbre: None,
                            });
                        }
                    }
                }
            }
            let aliases = if arabic || icelandic {
                mapping
                    .get(token)
                    .cloned()
                    .unwrap_or_else(|| vec![token.clone()])
            } else {
                vec![token.clone()]
            };
            let mut alias = aliases.first().cloned().unwrap_or_else(|| token.clone());
            if let Some(entry) = vb.find_mapped_entry(&alias, &note.pitch) {
                alias = entry.alias.clone();
            }
            phones.push(RenderPhone {
                note_index,
                lyric: alias,
                pitch: note.pitch.clone(),
                position_ms: note.position_ms + index as f64 * dur,
                duration_ms: dur,
                envelope: note.envelope.clone(),
                expressions: note.expressions.clone(),
                pitch_bend: note.pitch_bend.clone(),
                vibrato: note.vibrato.clone(),
                flags: note.flags.clone(),
                resolved_alias: None,
                resolved_timbre: None,
            });
        }
        if cvvc {
            previous_vowel = tokens
                .iter()
                .rev()
                .find(|token| is_rule_vowel(token))
                .cloned();
        }
    }
    phones
}

pub(super) fn is_rule_vowel(token: &str) -> bool {
    token.chars().next().is_some_and(|c| {
        matches!(
            c,
            'a' | 'e' | 'i' | 'o' | 'u' | 'y' | 'A' | 'E' | 'I' | 'O' | 'U' | 'Y'
        )
    })
}

pub(super) fn arabic_tokens(word: &str) -> Vec<String> {
    let clean: String = word
        .trim()
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace() && !c.is_ascii_punctuation())
        .collect();
    let chars: Vec<char> = clean.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let token = {
            let c = chars[i];
            i += 1;
            match c {
                'ا' | 'أ' | 'إ' | 'آ' => "a:",
                'ب' | 'b' => "b",
                'ت' | 't' => "t",
                'ث' => "th",
                'ج' | 'j' => "j",
                'ح' | '7' => "H",
                'خ' => "kh",
                'د' | 'd' => "d",
                'ذ' => "dh",
                'ر' | 'r' => "r",
                'ز' | 'z' => "z",
                'س' | 's' => "s",
                'ش' => "sh",
                'ص' | '9' => "s.",
                'ض' => "d.",
                'ط' => "t.",
                'ظ' => "dh.",
                'ع' | '3' => "3",
                'غ' => "gh",
                'ف' | 'f' => "f",
                'ق' | 'q' => "q",
                'ك' | 'k' => "k",
                'ل' | 'l' => "l",
                'م' | 'm' => "m",
                'ن' | 'n' => "n",
                'ه' | 'h' => "h",
                'و' | 'w' => "w",
                'ي' | 'ى' | 'y' => "y",
                'ة' => "a",
                'ء' | '2' => "?",
                _ => continue,
            }
        };
        out.push(token.to_string());
    }
    out
}

pub(super) fn icelandic_tokens(word: &str) -> Vec<String> {
    let lower = word.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        let token = match (c, next) {
            ('a', Some('u')) => {
                i += 2;
                "au"
            }
            ('e', Some('i')) => {
                i += 2;
                "ei"
            }
            ('o', Some('u')) => {
                i += 2;
                "ou"
            }
            ('p', Some('h')) => {
                i += 2;
                "p_h"
            }
            ('t', Some('h')) => {
                i += 2;
                "t_h"
            }
            ('k', Some('h')) => {
                i += 2;
                "k_h"
            }
            ('þ', _) => {
                i += 1;
                "T"
            }
            ('ð', _) => {
                i += 1;
                "D"
            }
            ('æ', _) => {
                i += 1;
                "E"
            }
            ('ö', _) => {
                i += 1;
                "9"
            }
            ('á', _) => {
                i += 1;
                "ai"
            }
            ('é', _) => {
                i += 1;
                "ei"
            }
            ('í', _) => {
                i += 1;
                "i"
            }
            ('ó', _) => {
                i += 1;
                "ou"
            }
            ('ú', _) => {
                i += 1;
                "u"
            }
            _ => {
                i += 1;
                match c {
                    'a' | 'e' | 'i' | 'o' | 'u' => &lower[i - 1..i],
                    _ => match c {
                        'p' | 'b' => "p",
                        't' | 'd' => "t",
                        'k' | 'g' | 'c' => "k",
                        'f' | 'v' => "f",
                        's' => "s",
                        'j' => "j",
                        'h' => "h",
                        'm' => "m",
                        'n' => "n",
                        'l' => "l",
                        'r' => "r",
                        _ => continue,
                    },
                }
            }
        };
        out.push(token.to_string());
    }
    out
}

pub(super) fn latin_language_tokens(word: &str, mode: PhonemizerMode) -> Vec<String> {
    let lower = word.to_lowercase();
    if matches!(
        mode,
        PhonemizerMode::ChineseG2P
            | PhonemizerMode::ChineseCVV
            | PhonemizerMode::ChineseCVVPlus
            | PhonemizerMode::ChineseVCV
            | PhonemizerMode::CantoneseG2P
            | PhonemizerMode::CantoneseSyo
            | PhonemizerMode::ChineseCVVC
            | PhonemizerMode::CantoneseCVVC
    ) && lower
        .chars()
        .any(|c| !c.is_ascii() && !c.is_ascii_punctuation())
    {
        let romanized = if matches!(
            mode,
            PhonemizerMode::ChineseG2P
                | PhonemizerMode::ChineseCVV
                | PhonemizerMode::ChineseCVVPlus
                | PhonemizerMode::ChineseVCV
                | PhonemizerMode::ChineseCVVC
        ) {
            pinyin::to_pinyin_vec(&lower, pinyin::Pinyin::with_tone_num_end)
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>()
        } else {
            cantonese_tokens(&lower)
        };
        if !romanized.is_empty() {
            return romanized;
        }
    }
    let mut out = Vec::new();
    let chars: Vec<char> = lower.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() || chars[i].is_ascii_punctuation() {
            i += 1;
            continue;
        }
        let pair = chars
            .get(i..(i + 2).min(chars.len()))
            .unwrap_or(&[])
            .iter()
            .collect::<String>();
        let token = match mode {
            PhonemizerMode::FrenchG2P => match pair.as_str() {
                "ch" => {
                    i += 2;
                    "sh"
                }
                "gn" => {
                    i += 2;
                    "ny"
                }
                "ou" => {
                    i += 2;
                    "u"
                }
                "oi" => {
                    i += 2;
                    "wa"
                }
                _ => latin_char_token(&chars, &mut i),
            },
            PhonemizerMode::GermanG2P => {
                if i + 3 <= chars.len() && chars[i..i + 3].iter().collect::<String>() == "sch" {
                    i += 3;
                    "sh"
                } else {
                    match pair.as_str() {
                        "ch" => {
                            i += 2;
                            "x"
                        }
                        "ei" | "ai" => {
                            i += 2;
                            "ay"
                        }
                        "eu" | "äu" => {
                            i += 2;
                            "oy"
                        }
                        _ => latin_char_token(&chars, &mut i),
                    }
                }
            }
            PhonemizerMode::RussianG2P => cyrillic_token(&chars, &mut i),
            PhonemizerMode::SpanishG2P => match pair.as_str() {
                "ll" => {
                    i += 2;
                    "y"
                }
                "ch" => {
                    i += 2;
                    "ch"
                }
                "rr" => {
                    i += 2;
                    "r"
                }
                _ => latin_char_token(&chars, &mut i),
            },
            PhonemizerMode::TurkishG2P | PhonemizerMode::TurkishCVVC => {
                if matches!(chars[i], 'ş' | 'ç' | 'ğ') {
                    let c = chars[i];
                    i += 1;
                    match c {
                        'ş' => "sh",
                        'ç' => "ch",
                        _ => "g",
                    }
                } else {
                    match pair.as_str() {
                        "ae" | "eu" | "oe" | "ue" => {
                            i += 2;
                            match pair.as_str() {
                                "ae" => "ae",
                                "eu" => "eu",
                                "oe" => "oe",
                                _ => "ue",
                            }
                        }
                        "ch" => {
                            i += 2;
                            "ch"
                        }
                        "sh" => {
                            i += 2;
                            "sh"
                        }
                        "ng" => {
                            i += 2;
                            "ng"
                        }
                        "rr" => {
                            i += 2;
                            "rr"
                        }
                        _ => latin_char_token(&chars, &mut i),
                    }
                }
            }
            PhonemizerMode::ChineseG2P
            | PhonemizerMode::CantoneseG2P
            | PhonemizerMode::ChineseCVVC
            | PhonemizerMode::CantoneseCVVC => {
                // OpenUtau's Chinese/Cantonese phonemizers consume pinyin or
                // jyutping syllables. Preserve an already romanized syllable
                // (including its tone number) as one phoneme unit.
                if chars[i].is_ascii_alphabetic() {
                    let start = i;
                    while i < chars.len()
                        && (chars[i].is_ascii_alphabetic() || chars[i].is_ascii_digit())
                    {
                        i += 1;
                    }
                    let syllable: String = chars[start..i].iter().collect();
                    out.push(syllable);
                    continue;
                }
                i += 1;
                match chars[i - 1] {
                    '你' => out.push(
                        if matches!(
                            mode,
                            PhonemizerMode::ChineseG2P | PhonemizerMode::ChineseCVVC
                        ) {
                            "ni3".to_string()
                        } else {
                            "nei5".to_string()
                        },
                    ),
                    '好' => out.push(
                        if matches!(
                            mode,
                            PhonemizerMode::ChineseG2P | PhonemizerMode::ChineseCVVC
                        ) {
                            "hao3".to_string()
                        } else {
                            "hou2".to_string()
                        },
                    ),
                    '世' => out.push("shi4".to_string()),
                    '界' => out.push(
                        if matches!(
                            mode,
                            PhonemizerMode::ChineseG2P | PhonemizerMode::ChineseCVVC
                        ) {
                            "jie4".to_string()
                        } else {
                            "gaai3".to_string()
                        },
                    ),
                    _ => out.push("a".to_string()),
                }
                continue;
            }
            _ => latin_char_token(&chars, &mut i),
        };
        out.push(token.to_string());
    }
    out
}

pub(super) fn cantonese_tokens(word: &str) -> Vec<String> {
    let dictionary = CANTONESE_DICTIONARY.get_or_init(|| {
        let mut map = HashMap::new();
        for line in include_str!("data/cccedict-canto-readings.txt").lines() {
            if line.starts_with('#') {
                continue;
            }
            let Some((head, reading)) = line.split_once('{') else {
                continue;
            };
            let Some(key) = head.split_whitespace().next() else {
                continue;
            };
            let Some(reading) = reading.split('}').next() else {
                continue;
            };
            let syllables: Vec<String> = reading.split_whitespace().map(str::to_string).collect();
            if !syllables.is_empty() {
                map.entry(key.to_string()).or_insert(syllables);
            }
        }
        map
    });
    if let Some(syllables) = dictionary.get(word) {
        return syllables.clone();
    }
    let mut out = Vec::new();
    for c in word.chars() {
        if let Some(syllables) = dictionary.get(&c.to_string()) {
            out.extend(syllables.iter().cloned());
            continue;
        }
        let token = match c {
            '你' => "nei5",
            '好' => "hou2",
            '世' => "sai3",
            '界' => "gaai3",
            '我' => "ngo5",
            '係' => "hai6",
            '佢' => "keoi5",
            '人' => "jan4",
            '中' => "zung1",
            '國' | '国' => "gwok3",
            '學' | '学' => "hok6",
            '生' => "saang1",
            '廣' | '广' => "gwong2",
            '東' | '东' => "dung1",
            _ if c.is_ascii_alphabetic() => continue,
            _ => "a1",
        };
        out.push(token.to_string());
    }
    out
}

pub(super) fn openutau_dictionary_tokens(word: &str, mode: PhonemizerMode) -> Option<Vec<String>> {
    let clean = word.trim().to_lowercase();
    let dictionaries = OPENUTAU_G2P_DICTIONARIES.get_or_init(|| {
        let mut all = HashMap::new();
        for (modes, dictionary) in [
            (
                &[PhonemizerMode::EnglishG2P][..],
                include_str!("data/openutau/en.dict"),
            ),
            (
                &[
                    PhonemizerMode::FrenchG2P,
                    PhonemizerMode::FrenchCMUSphinx,
                    PhonemizerMode::FrenchCVVC,
                    PhonemizerMode::FrenchVCCV,
                ][..],
                include_str!("data/openutau/fr.dict"),
            ),
            (
                &[
                    PhonemizerMode::GermanG2P,
                    PhonemizerMode::GermanVCCV,
                    PhonemizerMode::GermanDiphone,
                ][..],
                include_str!("data/openutau/de.dict"),
            ),
            (
                &[
                    PhonemizerMode::RussianG2P,
                    PhonemizerMode::RussianCVC,
                    PhonemizerMode::RussianVCCV,
                ][..],
                include_str!("data/openutau/ru.dict"),
            ),
            (
                &[
                    PhonemizerMode::SpanishG2P,
                    PhonemizerMode::SpanishMakkusan,
                    PhonemizerMode::SpanishSyllable,
                    PhonemizerMode::SpanishVCCV,
                ][..],
                include_str!("data/openutau/es.dict"),
            ),
            (
                &[
                    PhonemizerMode::PortugueseG2P,
                    PhonemizerMode::PortugueseBrazilianCVC,
                ][..],
                include_str!("data/openutau/pt.dict"),
            ),
        ] {
            let mut table = HashMap::new();
            for line in dictionary.lines() {
                let mut fields = line.split_whitespace();
                let Some(key) = fields.next() else { continue };
                if key.starts_with(';') || key.starts_with("###") {
                    continue;
                }
                let phones = fields
                    .map(|phone| {
                        phone
                            .trim_end_matches(|c: char| c.is_ascii_digit())
                            .to_string()
                    })
                    .collect::<Vec<_>>();
                if phones.is_empty() {
                    continue;
                }
                table.insert(key.to_lowercase(), phones);
            }
            for mode in modes {
                let normalized = table
                    .iter()
                    .map(|(key, phones)| {
                        let phones = if *mode == PhonemizerMode::EnglishG2P {
                            phones.iter().map(|p| p.to_lowercase()).collect()
                        } else {
                            phones.clone()
                        };
                        (key.clone(), phones)
                    })
                    .collect();
                all.insert(*mode, normalized);
            }
        }
        all
    });
    dictionaries.get(&mode).and_then(|table| {
        table.get(&clean).cloned().or_else(|| {
            table
                .iter()
                .find(|(key, _)| key.starts_with(&format!("{}(", clean)))
                .map(|(_, phones)| phones.clone())
        })
    })
}

/// Loads the singer-local Arpasing dictionary used by OpenUtau's
/// `ArpasingPhonemizer`.  Arpasing dictionaries are intentionally read
/// from the voicebank, rather than replaced by a built-in English
/// grapheme heuristic.  This keeps the phonemizer authoritative for the
/// singer and makes missing data observable to the caller.
pub(super) fn openutau_arpasing_tokens(word: &str, vb: &Voicebank) -> Option<Vec<String>> {
    let key = word.trim().to_lowercase();
    if let Ok(content) = std::fs::read_to_string(vb.root_path.join("arpasing.yaml")) {
        if let Ok(document) = yaml_serde::from_str::<yaml_serde::Value>(&content) {
            let value = document.get(&key).or_else(|| {
                document.as_mapping().and_then(|mapping| {
                    mapping.iter().find_map(|(candidate, value)| {
                        (candidate.as_str()?.to_lowercase() == key).then_some(value)
                    })
                })
            });
            let parse_tokens = |value: &yaml_serde::Value| -> Option<Vec<String>> {
                let mut tokens = match value {
                    yaml_serde::Value::String(value) => value
                        .split_whitespace()
                        .map(str::to_owned)
                        .collect::<Vec<_>>(),
                    yaml_serde::Value::Sequence(values) => values
                        .iter()
                        .filter_map(|value| value.as_str().map(str::to_owned))
                        .collect::<Vec<_>>(),
                    _ => Vec::new(),
                };
                tokens.retain(|token| !token.is_empty());
                (!tokens.is_empty()).then_some(tokens)
            };
            if let Some(tokens) = value.and_then(parse_tokens) {
                return Some(tokens);
            }
            if let Some(tokens) = document
                .get("entries")
                .and_then(|entries| entries.as_sequence())
                .and_then(|entries| {
                    entries.iter().find_map(|entry| {
                        let grapheme = entry.get("grapheme")?.as_str()?;
                        (grapheme.to_lowercase() == key)
                            .then(|| entry.get("phonemes"))
                            .flatten()
                            .and_then(parse_tokens)
                    })
                })
            {
                return Some(tokens);
            }
        }
    }

    // OpenUtau's Arpasing phonemizer falls back to Arpabet G2P when the
    // singer-local dictionary has no entry. The bundled OpenUtau/CMU
    // dictionary is the deterministic equivalent available to Kamafeu.
    openutau_dictionary_tokens(word, PhonemizerMode::EnglishG2P)
}

/// Vowel substitutions declared by OpenUtau's Arpasing phonemizer.
/// These are alias lookup fallbacks only; the generated phoneme remains
/// the original Arpabet token for timing and diagnostics.
pub(super) fn arpasing_vowel_fallbacks(phone: &str) -> &'static [&'static str] {
    match phone {
        "aa" => &["ah", "ae"],
        "ae" => &["ah", "aa"],
        "ah" => &["aa", "ae"],
        "ao" => &["ow"],
        "ow" => &["ao"],
        "eh" => &["ae"],
        "ih" => &["iy"],
        "iy" => &["ih"],
        "uh" => &["uw"],
        "uw" => &["uh"],
        "aw" => &["ao"],
        _ => &[],
    }
}

pub(super) fn latin_char_token(chars: &[char], i: &mut usize) -> &'static str {
    let c = chars[*i];
    *i += 1;
    match c {
        'a' | 'á' | 'à' | 'â' | 'ä' | 'ã' => "a",
        'e' | 'é' | 'è' | 'ê' | 'ë' => "e",
        'i' | 'í' | 'ì' | 'î' | 'ï' => "i",
        'o' | 'ó' | 'ò' | 'ô' | 'ö' | 'õ' => "o",
        'u' | 'ú' | 'ù' | 'û' | 'ü' => "u",
        'b' => "b",
        'c' => "k",
        'd' => "d",
        'f' => "f",
        'g' => "g",
        'h' => "h",
        'j' => "j",
        'k' => "k",
        'l' => "l",
        'm' => "m",
        'n' | 'ñ' => "n",
        'p' => "p",
        'q' => "k",
        'r' => "r",
        's' => "s",
        't' => "t",
        'v' => "v",
        'w' => "w",
        'x' => "x",
        'y' => "y",
        'z' => "z",
        _ => "a",
    }
}

pub(super) fn cyrillic_token(chars: &[char], i: &mut usize) -> &'static str {
    let c = chars[*i];
    *i += 1;
    match c {
        'а' | 'я' => "a",
        'э' | 'е' | 'ё' => "e",
        'и' | 'й' => "i",
        'о' => "o",
        'у' | 'ю' => "u",
        'б' => "b",
        'в' => "v",
        'г' => "g",
        'д' => "d",
        'ж' => "zh",
        'з' => "z",
        'к' => "k",
        'л' => "l",
        'м' => "m",
        'н' => "n",
        'п' => "p",
        'р' => "r",
        'с' => "s",
        'т' => "t",
        'ф' => "f",
        'х' => "kh",
        'ц' => "ts",
        'ч' => "ch",
        'ш' => "sh",
        'щ' => "sh",
        _ => "a",
    }
}
