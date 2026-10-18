use std::collections::{HashMap, HashSet};

use super::*;

/// Reads the exact-match replacement section used by Presamp.  OpenUtau
/// applies these replacements to the lyric before VCV/CVVC classification,
/// but never applies them to an explicit phonetic hint.
#[cfg(test)]
pub(super) fn parse_presamp_replacements(contents: &str) -> HashMap<String, String> {
    parse_presamp_replacements_with_presence(contents).0
}

pub(super) fn parse_presamp_replacements_with_presence(
    contents: &str,
) -> (HashMap<String, String>, bool) {
    let mut replacements = HashMap::new();
    let mut in_replace = false;
    let mut has_replace_section = false;
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_replace = line.eq_ignore_ascii_case("[REPLACE]");
            has_replace_section |= in_replace;
            continue;
        }
        if !in_replace || line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        let Some((from, to)) = line.split_once('=') else {
            continue;
        };
        let from = from.trim();
        let to = to.trim();
        if !from.is_empty() {
            replacements.insert(from.to_string(), to.to_string());
        }
    }
    (replacements, has_replace_section)
}

pub(super) fn default_presamp_replacement(lyric: &str) -> Option<&'static str> {
    // Entries that are intentionally not inferred by the generic romaji
    // converter. These mirror OpenUtau's built-in Presamp defReplace table.
    Some(match lyric {
        "N" => "ン",
        "dha" => "でゃ",
        "dhe" => "でぇ",
        "dhi" => "でぃ",
        "dho" => "でょ",
        "dhu" => "でゅ",
        "fyu" => "ふゅ",
        "hyi" => "ひ",
        "kyi" => "き",
        "myi" => "み",
        "nyi" => "に",
        "pye" => "ぴぇ",
        "pyi" => "ぴ",
        "rye" => "りぇ",
        "ryi" => "り",
        "tyi" => "てぃ",
        "tye" => "てぇ",
        "vyu" => "ヴゅ",
        "wo" => "うぉ",
        "wu" => "うぅ",
        "zya" => "じゃ",
        "zye" => "じぇ",
        "zyi" => "じ",
        "zyo" => "じょ",
        "zyu" => "じゅ",
        "を" => "お",
        "ぢ" => "じ",
        "づ" => "ず",
        _ => return None,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PresampAliasConfig {
    pub(super) vcpad: String,
    pub(super) vcvpad: String,
    pub(super) vcv: String,
    pub(super) beginning_cv: String,
    pub(super) cross_cv: String,
    pub(super) vc: String,
    pub(super) cv: String,
    pub(super) c: String,
    pub(super) long_v: String,
    pub(super) ending1: String,
    pub(super) ending2: String,
}

#[derive(Debug, Clone)]
pub(super) struct PresampInventory {
    pub(super) vowels: HashSet<String>,
    /// Every Presamp vowel symbol mapped to its canonical lower-case vowel.
    pub(super) vowel_by_phoneme: HashMap<String, String>,
    pub(super) consonants: HashSet<String>,
    /// Every Presamp consonant symbol mapped to its canonical consonant.
    pub(super) consonant_by_phoneme: HashMap<String, String>,
    pub(super) alias_nums: HashSet<String>,
    pub(super) alias_appends: Vec<String>,
    pub(super) alias_pitches: Vec<String>,
    /// Presamp `[ALIAS_PRIORITY]` and its suffix-specific variants. The
    /// strings are kept in file order, matching OpenUtau's Presamp parser.
    pub(super) alias_priority_default: Vec<String>,
    pub(super) alias_priority_diff_append: Vec<String>,
    pub(super) alias_priority_diff_pitch: Vec<String>,
    pub(super) prefixes: Vec<String>,
    pub(super) suffix_order: Vec<String>,
    pub(super) priority: HashSet<String>,
    pub(super) no_crossfade: HashSet<String>,
    /// Presamp `[VCLENGTH]`: 0 derives VC timing from the following CV oto;
    /// 1 uses the fixed legacy duration.
    pub(super) vc_length_from_cv: bool,
    /// Presamp `[MUSTVC]`: force a VC even when the following alias already
    /// contains its own transition or the note is too short.
    pub(super) must_vc: bool,
    /// Presamp `[ENDFLAG]`: 0 disables ending conversion, 1 adds/uses the
    /// ending alias, and 2 converts the final note when supported.
    pub(super) add_ending: u8,
    /// Presamp `[SPLIT]` controls comma/semicolon lyric expansion.
    pub(super) split: bool,
    /// Presamp `[CFLAGS]` applied to generated phrase-head consonants.
    pub(super) cflags: String,
}

impl Default for PresampInventory {
    fn default() -> Self {
        let mut alias_pitches = Vec::new();
        for octave in 1..=7 {
            for pitch in [
                "C", "C#", "Db", "D", "D#", "Eb", "E", "F", "F#", "Gb", "G", "G#", "Ab", "A", "A#",
                "Bb", "B",
            ] {
                alias_pitches.push(format!("{pitch}{octave}"));
            }
        }
        alias_pitches.extend(
            ["↑", "↓", "→", "←", "high", "low", "mid"]
                .into_iter()
                .map(str::to_string),
        );
        let priority = [
            "k", "ky", "g", "gy", "t", "ty", "d", "dy", "ch", "ts", "b", "by", "p", "py", "r", "ry",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        let no_crossfade = [
            "ch", "gy", "ts", "ty", "py", "ry", "r", "dy", "by", "b", "d", "g", "k", "p", "t", "ky",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        Self {
            vowels: HashSet::new(),
            vowel_by_phoneme: HashMap::new(),
            consonants: HashSet::new(),
            consonant_by_phoneme: HashMap::new(),
            alias_nums: (0..=9).map(|n| n.to_string()).collect(),
            alias_appends: [
                "強", "弱", "暗", "明", "囁", "叫", "超", "幼", "甘", "笑", "瞭", "憂", "優", "感",
                "張", "抗", "泣叫", "癖", "元気", "艶",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            alias_pitches,
            alias_priority_default: vec![
                "VCV".to_string(),
                "CVVC".to_string(),
                "CROSS_CV".to_string(),
                "CV".to_string(),
                "BEGINING_CV".to_string(),
            ],
            alias_priority_diff_append: vec![
                "CVVC".to_string(),
                "VCV".to_string(),
                "CROSS_CV".to_string(),
                "CV".to_string(),
                "BEGINING_CV".to_string(),
            ],
            alias_priority_diff_pitch: vec![
                "CVVC".to_string(),
                "VCV".to_string(),
                "CROSS_CV".to_string(),
                "CV".to_string(),
                "BEGINING_CV".to_string(),
            ],
            prefixes: Vec::new(),
            suffix_order: vec![
                "%num%".to_string(),
                "%append%".to_string(),
                "%pitch%".to_string(),
            ],
            priority,
            no_crossfade,
            vc_length_from_cv: true,
            must_vc: false,
            add_ending: 1,
            split: true,
            cflags: "p0".to_string(),
        }
    }
}

pub(super) fn parse_presamp_inventory(contents: &str) -> PresampInventory {
    let mut inventory = PresampInventory::default();
    let mut section = "";
    for raw_line in contents.lines() {
        let line = raw_line.trim_end_matches('\r').trim();
        if line.starts_with('[') && line.ends_with(']') {
            section = line;
            match line.to_ascii_uppercase().as_str() {
                "[VOWEL]" => {
                    inventory.vowels.clear();
                    inventory.vowel_by_phoneme.clear();
                }
                "[CONSONANT]" => {
                    inventory.consonants.clear();
                    inventory.consonant_by_phoneme.clear();
                    inventory.no_crossfade.clear();
                }
                "[PRIORITY]" => inventory.priority.clear(),
                "[NUM]" => inventory.alias_nums.clear(),
                "[APPEND]" => inventory.alias_appends.clear(),
                "[PITCH]" => inventory.alias_pitches.clear(),
                "[ALIAS_PRIORITY]" => inventory.alias_priority_default.clear(),
                "[ALIAS_PRIORITY_DIFAPPEND]" => inventory.alias_priority_diff_append.clear(),
                "[ALIAS_PRIORITY_DIFPITCH]" => inventory.alias_priority_diff_pitch.clear(),
                "[PRE]" => inventory.prefixes.clear(),
                "[SU]" => inventory.suffix_order.clear(),
                "[CFLAGS]" => inventory.cflags.clear(),
                _ => {}
            }
            continue;
        }
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('=').map(str::trim).collect();
        let Some(first) = fields.first().filter(|value| !value.is_empty()) else {
            continue;
        };
        let mut values = vec![*first];
        if let Some(list) = fields.get(1) {
            values.extend(
                list.split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty()),
            );
        }
        match section.to_ascii_uppercase().as_str() {
            "[VOWEL]" => {
                // Presamp format: vowel=representative=phoneme1,phoneme2=volume.
                // The representative and every listed phoneme are valid
                // lyric symbols; the final volume field is intentionally
                // ignored by the phonemizer.
                inventory.vowels.insert((*first).to_string());
                inventory
                    .vowel_by_phoneme
                    .insert((*first).to_string(), (*first).to_string());
                if let Some(representative) = fields.get(1).filter(|v| !v.is_empty()) {
                    inventory.vowels.insert((*representative).to_string());
                    inventory
                        .vowel_by_phoneme
                        .insert((*representative).to_string(), (*first).to_string());
                }
                if let Some(list) = fields.get(2) {
                    for value in list.split(',').map(str::trim).filter(|v| !v.is_empty()) {
                        inventory.vowels.insert(value.to_string());
                        inventory
                            .vowel_by_phoneme
                            .insert(value.to_string(), (*first).to_string());
                    }
                }
            }
            "[CONSONANT]" => {
                inventory
                    .consonants
                    .extend(values.into_iter().map(str::to_owned));
                inventory
                    .consonant_by_phoneme
                    .insert((*first).to_string(), (*first).to_string());
                if let Some(list) = fields.get(1) {
                    for value in list.split(',').map(str::trim).filter(|v| !v.is_empty()) {
                        inventory
                            .consonant_by_phoneme
                            .insert(value.to_string(), (*first).to_string());
                    }
                }
                if fields.get(2).is_some_and(|flag| *flag == "1") {
                    // Presamp's third field is the no-crossfade flag, not
                    // priority. Priority is configured by [PRIORITY].
                    inventory.no_crossfade.insert((*first).to_string());
                }
            }
            "[PRIORITY]" => {
                inventory.priority.extend(
                    line.split(',')
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(str::to_owned),
                );
            }
            "[VCLENGTH]" => {
                inventory.vc_length_from_cv = *first != "1";
            }
            "[MUSTVC]" => {
                inventory.must_vc = *first == "1";
            }
            "[ENDFLAG]" => {
                if let Ok(flag) = first.parse::<u8>() {
                    inventory.add_ending = flag.min(2);
                }
            }
            "[SPLIT]" => {
                inventory.split = *first != "0";
            }
            "[CFLAGS]" => {
                inventory.cflags = (*first).to_string();
            }
            "[NUM]" => {
                if !first.starts_with('@') || !first.ends_with('@') {
                    inventory.alias_nums.insert((*first).to_string());
                }
            }
            "[APPEND]" => {
                if !first.starts_with('@') || !first.ends_with('@') {
                    inventory.alias_appends.push((*first).to_string());
                }
            }
            "[PITCH]" => {
                if !first.starts_with('@') || !first.ends_with('@') {
                    inventory.alias_pitches.push((*first).to_string());
                }
            }
            "[ALIAS_PRIORITY]" => inventory.alias_priority_default.push((*first).to_string()),
            "[ALIAS_PRIORITY_DIFAPPEND]" => inventory
                .alias_priority_diff_append
                .push((*first).to_string()),
            "[ALIAS_PRIORITY_DIFPITCH]" => inventory
                .alias_priority_diff_pitch
                .push((*first).to_string()),
            "[PRE]" => {
                if !inventory.prefixes.iter().any(|prefix| prefix == line) {
                    inventory.prefixes.push(line.to_string());
                }
            }
            "[SU]" => {
                let mut remainder = line;
                let mut order = Vec::new();
                while !remainder.is_empty() {
                    let Some(token) = ["%num%", "%append%", "%pitch%"]
                        .into_iter()
                        .find(|token| remainder.starts_with(token))
                    else {
                        order.clear();
                        break;
                    };
                    order.push(token.to_string());
                    remainder = &remainder[token.len()..];
                }
                if !remainder.is_empty() || order.len() != 3 {
                    order.clear();
                }
                if !order.is_empty() {
                    inventory.suffix_order = order;
                }
            }
            _ => {}
        }
    }
    inventory
}

pub(super) fn presamp_core_lyric(lyric: &str, inventory: &PresampInventory) -> String {
    let mut core = lyric.trim().to_string();
    // Presamp's default VCPAD/VCVPAD is a space. ParseAlias therefore
    // classifies the phoneme after the pad (`a か`, `a s`, `- か`) rather
    // than the complete transition alias. This is also required for direct
    // VCV/CVVC specifications entered as a phonetic lyric.
    if core.chars().any(char::is_whitespace) {
        if let Some(right) = core.split_whitespace().last() {
            core = right.to_string();
        }
    } else if let Some((_, right)) = core.rsplit_once('-') {
        // Custom Presamp files commonly set VCPAD to `-`.
        if !right.is_empty() {
            core = right.to_string();
        }
    }
    if let Some((left, right)) = core.rsplit_once('_') {
        if looks_like_pitch_suffix(right) {
            core = left.to_string();
        } else if !right.is_empty()
            && (inventory.vowel_by_phoneme.contains_key(left)
                || JapanesePhonemizer::extract_vowel(left).is_some())
        {
            // `_` is also a configurable VCV pad. In `a_か`, keep the
            // phoneme after the pad; in `か_C4`, keep the left side.
            core = right.to_string();
        }
    }
    if !core.contains('_') {
        let chars: Vec<char> = core.chars().collect();
        for suffix_len in [3usize, 2usize] {
            if chars.len() > suffix_len {
                let suffix: String = chars[chars.len() - suffix_len..].iter().collect();
                if looks_like_pitch_suffix(&suffix) {
                    core.truncate(core.len() - suffix.len());
                    break;
                }
            }
        }
    }
    // Presamp.ParseAlias removes numeric suffixes, then appends, then pitch
    // suffixes. HashSet iteration is deliberately unordered, so sort each
    // category by descending length to avoid stripping a short token from a
    // longer custom token first (for example `強` before `強い`).
    let mut nums = inventory.alias_nums.iter().collect::<Vec<_>>();
    nums.sort_by_key(|suffix| std::cmp::Reverse(suffix.len()));
    let mut suffixes = nums;
    let mut appends = inventory.alias_appends.iter().collect::<Vec<_>>();
    appends.sort_by_key(|suffix| std::cmp::Reverse(suffix.len()));
    suffixes.extend(appends);
    let mut pitches = inventory.alias_pitches.iter().collect::<Vec<_>>();
    pitches.sort_by_key(|suffix| std::cmp::Reverse(suffix.len()));
    suffixes.extend(pitches);
    for suffix in suffixes {
        if let Some(base) = core.strip_suffix(suffix) {
            core = base.to_string();
        }
    }
    core
}

fn looks_like_pitch_suffix(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some('A'..='G' | 'a'..='g'))
        && chars.all(|ch| ch.is_ascii_digit() || matches!(ch, '#' | 'b'))
}

pub(super) fn extract_presamp_vowel(lyric: &str, inventory: &PresampInventory) -> Option<String> {
    let core = presamp_core_lyric(lyric, inventory);
    let trimmed = core.trim();
    if let Some(vowel) = inventory.vowel_by_phoneme.get(trimmed) {
        return Some(vowel.clone());
    }
    // Presamp/OpenUtau completes an unmapped compound kana from its final
    // mora (for example きゃ -> ゃ). This matters for custom [VOWEL] tables
    // that list the small kana but omit every compound spelling.
    if let Some(last) = trimmed.chars().last() {
        let last = last.to_string();
        if let Some(vowel) = inventory.vowel_by_phoneme.get(&last) {
            return Some(vowel.clone());
        }
    }
    JapanesePhonemizer::extract_vowel(trimmed).map(str::to_owned)
}

pub(super) fn extract_presamp_consonant(
    lyric: &str,
    inventory: &PresampInventory,
) -> Option<String> {
    let core = presamp_core_lyric(lyric, inventory);
    let trimmed = core.trim();
    if let Some(consonant) = inventory.consonant_by_phoneme.get(trimmed) {
        return Some(consonant.clone());
    }
    if let Some(last) = trimmed.chars().last() {
        let last = last.to_string();
        if let Some(consonant) = inventory.consonant_by_phoneme.get(&last) {
            return Some(consonant.clone());
        }
    }
    JapanesePhonemizer::extract_consonant(trimmed).map(str::to_owned)
}

pub(super) fn extract_vowel_for_mode(
    lyric: &str,
    inventory: &PresampInventory,
    mode: PhonemizerMode,
) -> Option<String> {
    // JapaneseVCVPhonemizer's legacy table maps `ng` to the katakana nasal
    // vowel N. JapaneseCVVCPhonemizer deliberately keeps `ng` as a
    // consonant, so this exception must be mode-specific.
    if mode == PhonemizerMode::VCV && lyric.trim() == "ng" {
        return Some("N".to_string());
    }
    extract_presamp_vowel(lyric, inventory)
}

impl Default for PresampAliasConfig {
    fn default() -> Self {
        // These are the defaults documented by Presamp/OpenUtau.
        Self {
            vcpad: " ".to_string(),
            vcvpad: " ".to_string(),
            vcv: "%v%%VCVPAD%%CV%".to_string(),
            beginning_cv: "-%VCVPAD%%CV%".to_string(),
            cross_cv: "*%VCVPAD%%CV%".to_string(),
            vc: "%v%%vcpad%%c%,%c%%vcpad%%c%".to_string(),
            cv: "%CV%,%c%%V%".to_string(),
            c: "%c%".to_string(),
            long_v: "%V%ー".to_string(),
            ending1: "%v%%VCPAD%R".to_string(),
            ending2: "-".to_string(),
        }
    }
}

pub(super) fn parse_presamp_alias_config(contents: &str) -> PresampAliasConfig {
    let mut config = PresampAliasConfig::default();
    let mut section = "";
    for raw_line in contents.lines() {
        let line = raw_line.trim_end_matches('\r').trim();
        if line.starts_with('[') && line.ends_with(']') {
            section = line;
            continue;
        }
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if section.eq_ignore_ascii_case("[ENDTYPE]") || section.eq_ignore_ascii_case("[ENDTYPE1]") {
            config.ending1 = line.to_string();
            continue;
        }
        if section.eq_ignore_ascii_case("[ENDTYPE2]") {
            config.ending2 = line.to_string();
            continue;
        }
        if section.eq_ignore_ascii_case("[VCPAD]") {
            config.vcpad = line.to_string();
            continue;
        }
        if !section.eq_ignore_ascii_case("[ALIAS]") {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim().to_ascii_uppercase().as_str() {
            "VCPAD" => config.vcpad = value.to_string(),
            "VCVPAD" => config.vcvpad = value.to_string(),
            "VCV" => config.vcv = value.to_string(),
            "BEGINING_CV" | "BEGINNING_CV" => config.beginning_cv = value.to_string(),
            "CROSS_CV" => config.cross_cv = value.to_string(),
            "VC" => config.vc = value.to_string(),
            "CV" => config.cv = value.to_string(),
            "C" => config.c = value.to_string(),
            "LONG_V" => config.long_v = value.to_string(),
            "ENDING1" => config.ending1 = value.to_string(),
            "ENDING2" => config.ending2 = value.to_string(),
            _ => {}
        }
    }
    config
}

pub(super) fn load_presamp_replacements(root: &std::path::Path) -> (HashMap<String, String>, bool) {
    let path = root.join("presamp.ini");
    let Ok(bytes) = fs::read(path) else {
        return (HashMap::new(), false);
    };
    let text = String::from_utf8(bytes.clone()).unwrap_or_else(|_| {
        let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
        decoded.into_owned()
    });
    parse_presamp_replacements_with_presence(&text)
}

pub(super) fn load_presamp_alias_config(root: &std::path::Path) -> PresampAliasConfig {
    let path = root.join("presamp.ini");
    let Ok(bytes) = fs::read(path) else {
        return PresampAliasConfig::default();
    };
    let text = String::from_utf8(bytes.clone()).unwrap_or_else(|_| {
        let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
        decoded.into_owned()
    });
    parse_presamp_alias_config(&text)
}

pub(super) fn load_presamp_inventory(root: &std::path::Path) -> PresampInventory {
    let path = root.join("presamp.ini");
    let Ok(bytes) = fs::read(path) else {
        return PresampInventory::default();
    };
    let text = String::from_utf8(bytes.clone()).unwrap_or_else(|_| {
        let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
        decoded.into_owned()
    });
    parse_presamp_inventory(&text)
}

pub(super) fn presamp_join(left: &str, pad: &str, right: &str) -> String {
    format!("{left}{pad}{right}")
}

pub(super) fn expand_presamp_alias(
    template: &str,
    vowel: &str,
    consonant: &str,
    cv: &str,
    representative_vowel: &str,
    config: &PresampAliasConfig,
) -> String {
    template
        .replace("%CVPAD%", &config.vcpad)
        .replace("%VCVPAD%", &config.vcvpad)
        .replace("%VCPAD%", &config.vcpad)
        .replace("%vcpad%", &config.vcpad)
        .replace("%v%", vowel)
        .replace("%V%", representative_vowel)
        .replace("%c%", consonant)
        .replace("%CV%", cv)
}

pub(super) fn expand_presamp_candidates(
    template: &str,
    vowel: &str,
    consonant: &str,
    cv: &str,
    representative_vowel: &str,
    config: &PresampAliasConfig,
) -> Vec<String> {
    template
        .split(',')
        .map(|part| expand_presamp_alias(part, vowel, consonant, cv, representative_vowel, config))
        .filter(|candidate| !candidate.is_empty())
        .collect()
}

pub(super) fn presamp_priority_for_lyric<'a>(
    lyric: &str,
    inventory: &'a PresampInventory,
) -> &'a [String] {
    let has_pitch = inventory
        .alias_pitches
        .iter()
        .any(|suffix| !suffix.is_empty() && lyric.contains(suffix));
    let has_append = inventory
        .alias_appends
        .iter()
        .any(|suffix| !suffix.is_empty() && lyric.contains(suffix));
    if has_pitch {
        &inventory.alias_priority_diff_pitch
    } else if has_append {
        &inventory.alias_priority_diff_append
    } else {
        &inventory.alias_priority_default
    }
}

/// Flattens Presamp alias families in the configured order. Unknown or
/// omitted families are appended after the configured ones, preserving the
/// fallback behavior of OpenUtau for incomplete custom ini files.
pub(super) fn order_presamp_candidate_groups(
    priority: &[String],
    groups: Vec<(&str, Vec<String>)>,
) -> Vec<String> {
    let mut ordered = Vec::new();
    let mut used = vec![false; groups.len()];
    for family in priority {
        for (index, (name, candidates)) in groups.iter().enumerate() {
            if !used[index] && family.eq_ignore_ascii_case(name) {
                ordered.extend(candidates.iter().cloned());
                used[index] = true;
            }
        }
    }
    for (index, (_, candidates)) in groups.into_iter().enumerate() {
        if !used[index] {
            ordered.extend(candidates);
        }
    }
    ordered
}

pub(super) fn presamp_suffix_variants(lyric: &str, inventory: &PresampInventory) -> Vec<String> {
    let core = presamp_core_lyric(lyric, inventory);
    let Some(tail) = lyric.strip_prefix(&core) else {
        return Vec::new();
    };
    if tail.is_empty() || inventory.suffix_order.is_empty() {
        return Vec::new();
    }

    let mut parts: Vec<(String, &'static str)> = Vec::new();
    let mut remainder = tail;
    while !remainder.is_empty() {
        let mut found = None;
        for suffix in &inventory.alias_nums {
            if remainder.starts_with(suffix) {
                found = Some((suffix.clone(), "%num%"));
                break;
            }
        }
        if found.is_none() {
            for suffix in &inventory.alias_appends {
                if remainder.starts_with(suffix) {
                    found = Some((suffix.clone(), "%append%"));
                    break;
                }
            }
        }
        if found.is_none() {
            for suffix in &inventory.alias_pitches {
                if remainder.starts_with(suffix) {
                    found = Some((suffix.clone(), "%pitch%"));
                    break;
                }
            }
        }
        let Some((value, kind)) = found else {
            return Vec::new();
        };
        remainder = &remainder[value.len()..];
        parts.push((value, kind));
    }

    let reordered: String = inventory
        .suffix_order
        .iter()
        .flat_map(|kind| {
            parts
                .iter()
                .filter(move |(_, part_kind)| part_kind == kind)
                .map(|(value, _)| value.as_str())
        })
        .collect();
    let original: String = parts.iter().map(|(value, _)| value.as_str()).collect();
    if reordered == original {
        Vec::new()
    } else {
        vec![format!("{core}{reordered}")]
    }
}

pub(super) fn add_presamp_suffix_variants(
    candidates: &mut Vec<String>,
    lyric: &str,
    inventory: &PresampInventory,
) {
    for variant in presamp_suffix_variants(lyric, inventory) {
        let snapshot = candidates.clone();
        for candidate in snapshot {
            if candidate.contains(lyric) {
                candidates.push(candidate.replace(lyric, &variant));
            }
        }
    }
}

pub(super) fn long_vowel_candidate(lyric: &str, config: &PresampAliasConfig) -> Option<String> {
    let stem = lyric.strip_suffix('ー')?.trim();
    if stem.is_empty() {
        return None;
    }
    Some(expand_presamp_alias(
        &config.long_v,
        "",
        "",
        lyric,
        stem,
        config,
    ))
}
