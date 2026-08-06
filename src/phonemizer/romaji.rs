//! Bidirectional Romaji ↔ Hiragana conversion for the UTAU phonemizer.
//!
//! Supports standard Japanese CV syllables, voiced and semi-voiced kana,
//! and combination syllables such as きゃ and しゃ.

/// Full mapping table: (romaji, hiragana)
/// Ordered longest-first for greedy matching.
const ROMAJI_HIRAGANA_TABLE: &[(&str, &str)] = &[
    // Combination syllables (must come before shorter matches)
    ("kya", "きゃ"),
    ("kyi", "きぃ"),
    ("kyu", "きゅ"),
    ("kye", "きぇ"),
    ("kyo", "きょ"),
    ("sha", "しゃ"),
    ("shi", "し"),
    ("shu", "しゅ"),
    ("she", "しぇ"),
    ("sho", "しょ"),
    ("cha", "ちゃ"),
    ("chi", "ち"),
    ("chu", "ちゅ"),
    ("che", "ちぇ"),
    ("cho", "ちょ"),
    ("tya", "ちゃ"),
    ("tyi", "ちぃ"),
    ("tyu", "ちゅ"),
    ("tye", "ちぇ"),
    ("tyo", "ちょ"),
    ("nya", "にゃ"),
    ("nyi", "にぃ"),
    ("nyu", "にゅ"),
    ("nye", "にぇ"),
    ("nyo", "にょ"),
    ("hya", "ひゃ"),
    ("hyi", "ひぃ"),
    ("hyu", "ひゅ"),
    ("hye", "ひぇ"),
    ("hyo", "ひょ"),
    ("mya", "みゃ"),
    ("myi", "みぃ"),
    ("myu", "みゅ"),
    ("mye", "みぇ"),
    ("myo", "みょ"),
    ("rya", "りゃ"),
    ("ryi", "りぃ"),
    ("ryu", "りゅ"),
    ("rye", "りぇ"),
    ("ryo", "りょ"),
    ("gya", "ぎゃ"),
    ("gyi", "ぎぃ"),
    ("gyu", "ぎゅ"),
    ("gye", "ぎぇ"),
    ("gyo", "ぎょ"),
    ("ja", "じゃ"),
    ("ji", "じ"),
    ("ju", "じゅ"),
    ("je", "じぇ"),
    ("jo", "じょ"),
    ("jya", "じゃ"),
    ("jyi", "じぃ"),
    ("jyu", "じゅ"),
    ("jye", "じぇ"),
    ("jyo", "じょ"),
    ("bya", "びゃ"),
    ("byi", "びぃ"),
    ("byu", "びゅ"),
    ("bye", "びぇ"),
    ("byo", "びょ"),
    ("pya", "ぴゃ"),
    ("pyi", "ぴぃ"),
    ("pyu", "ぴゅ"),
    ("pye", "ぴぇ"),
    ("pyo", "ぴょ"),
    ("dya", "ぢゃ"),
    ("dyi", "ぢぃ"),
    ("dyu", "ぢゅ"),
    ("dye", "ぢぇ"),
    ("dyo", "ぢょ"),
    ("tsa", "つぁ"),
    ("tsi", "つぃ"),
    ("tsu", "つ"),
    ("tse", "つぇ"),
    ("tso", "つぉ"),
    ("fa", "ふぁ"),
    ("fi", "ふぃ"),
    ("fu", "ふ"),
    ("fe", "ふぇ"),
    ("fo", "ふぉ"),
    // Double consonants (っ prefix)
    ("kka", "っか"),
    ("kki", "っき"),
    ("kku", "っく"),
    ("kke", "っけ"),
    ("kko", "っこ"),
    ("ssa", "っさ"),
    ("ssi", "っし"),
    ("ssu", "っす"),
    ("sse", "っせ"),
    ("sso", "っそ"),
    ("tta", "った"),
    ("tti", "っち"),
    ("ttu", "っつ"),
    ("tte", "って"),
    ("tto", "っと"),
    ("ppa", "っぱ"),
    ("ppi", "っぴ"),
    ("ppu", "っぷ"),
    ("ppe", "っぺ"),
    ("ppo", "っぽ"),
    // Basic CV syllables
    ("ka", "か"),
    ("ki", "き"),
    ("ku", "く"),
    ("ke", "け"),
    ("ko", "こ"),
    ("sa", "さ"),
    ("su", "す"),
    ("se", "せ"),
    ("so", "そ"),
    ("ta", "た"),
    ("tu", "つ"),
    ("te", "て"),
    ("to", "と"),
    ("na", "な"),
    ("ni", "に"),
    ("nu", "ぬ"),
    ("ne", "ね"),
    ("no", "の"),
    ("ha", "は"),
    ("hi", "ひ"),
    ("hu", "ふ"),
    ("he", "へ"),
    ("ho", "ほ"),
    ("ma", "ま"),
    ("mi", "み"),
    ("mu", "む"),
    ("me", "め"),
    ("mo", "も"),
    ("ya", "や"),
    ("yu", "ゆ"),
    ("yo", "よ"),
    ("ra", "ら"),
    ("ri", "り"),
    ("ru", "る"),
    ("re", "れ"),
    ("ro", "ろ"),
    ("wa", "わ"),
    ("wi", "ゐ"),
    ("we", "ゑ"),
    ("wo", "を"),
    // Voiced (dakuten)
    ("ga", "が"),
    ("gi", "ぎ"),
    ("gu", "ぐ"),
    ("ge", "げ"),
    ("go", "ご"),
    ("za", "ざ"),
    ("zi", "じ"),
    ("zu", "ず"),
    ("ze", "ぜ"),
    ("zo", "ぞ"),
    ("da", "だ"),
    ("di", "ぢ"),
    ("du", "づ"),
    ("de", "で"),
    ("do", "ど"),
    ("ba", "ば"),
    ("bi", "び"),
    ("bu", "ぶ"),
    ("be", "べ"),
    ("bo", "ぼ"),
    // Semi-voiced (handakuten)
    ("pa", "ぱ"),
    ("pi", "ぴ"),
    ("pu", "ぷ"),
    ("pe", "ぺ"),
    ("po", "ぽ"),
    // Pure vowels (must come after longer matches)
    ("a", "あ"),
    ("i", "い"),
    ("u", "う"),
    ("e", "え"),
    ("o", "お"),
    // Special
    ("n", "ん"),
    ("nn", "ん"),
    // Small kana
    ("xa", "ぁ"),
    ("xi", "ぃ"),
    ("xu", "ぅ"),
    ("xe", "ぇ"),
    ("xo", "ぉ"),
    ("xya", "ゃ"),
    ("xyu", "ゅ"),
    ("xyo", "ょ"),
    ("xtu", "っ"),
];

/// Convert romaji string to hiragana.
/// Handles multi-character romaji sequences using greedy longest-match.
pub fn romaji_to_hiragana(input: &str) -> String {
    let input_lower = input.to_lowercase();
    let chars: Vec<char> = input_lower.chars().collect();
    let mut result = String::new();
    let mut i = 0;

    while i < chars.len() {
        let remaining = &input_lower[chars[..i].iter().map(|c| c.len_utf8()).sum::<usize>()..];
        let mut matched = false;

        // Try longest matches first
        for &(romaji, hiragana) in ROMAJI_HIRAGANA_TABLE {
            if remaining.starts_with(romaji) {
                // Special case: "n" before a vowel or "y" should not match standalone "n"
                if romaji == "n" && remaining.len() > 1 {
                    let next_char = remaining.chars().nth(1).unwrap_or(' ');
                    if "aiueoy".contains(next_char) {
                        continue;
                    }
                }
                result.push_str(hiragana);
                i += romaji.chars().count();
                matched = true;
                break;
            }
        }

        if !matched {
            // Handle geminate consonant (double consonant → っ)
            if i + 1 < chars.len()
                && chars[i] == chars[i + 1]
                && chars[i].is_ascii_alphabetic()
                && !"aiueo".contains(chars[i])
            {
                result.push('っ');
                i += 1;
            } else {
                result.push(chars[i]);
                i += 1;
            }
        }
    }

    result
}

/// Convert hiragana string to romaji.
pub fn hiragana_to_romaji(input: &str) -> String {
    let mut result = String::new();
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let remaining = &input[chars[..i].iter().map(|c| c.len_utf8()).sum::<usize>()..];
        let mut matched = false;

        // Try longest hiragana matches first (combination kana are multi-char)
        for &(romaji, hiragana) in ROMAJI_HIRAGANA_TABLE {
            if remaining.starts_with(hiragana) {
                result.push_str(romaji);
                i += hiragana.chars().count();
                matched = true;
                break;
            }
        }

        if !matched {
            // Pass through any character we don't recognize (spaces, punctuation, etc.)
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

/// Given a lyric, return all candidate alias forms to try in the voicebank.
/// Returns: [original, hiragana_version, romaji_version] (deduplicated).
pub fn lyric_candidates(lyric: &str) -> Vec<String> {
    let trimmed = lyric.trim();
    if trimmed.is_empty() {
        return vec![];
    }

    let mut candidates = Vec::new();
    candidates.push(trimmed.to_string());

    // Check for VCV / Head prefix patterns: "- ka", "a ka", "-ka", "a_ka"
    if let Some(space_idx) = trimmed.find([' ', '_']) {
        let prefix = &trimmed[..space_idx + 1];
        let body = &trimmed[space_idx + 1..];

        if !body.is_empty() {
            let h_body = romaji_to_hiragana(body);
            let r_body = hiragana_to_romaji(body);

            let cand_h = format!("{}{}", prefix, h_body);
            let cand_r = format!("{}{}", prefix, r_body);

            if !candidates.contains(&cand_h) {
                candidates.push(cand_h);
            }
            if !candidates.contains(&cand_r) {
                candidates.push(cand_r);
            }
        }
    }

    // Check if the input looks like romaji (ASCII) or hiragana
    let is_ascii = trimmed.is_ascii();

    if is_ascii {
        // Input is romaji → generate hiragana form
        let hiragana = romaji_to_hiragana(trimmed);
        if !candidates.contains(&hiragana) {
            candidates.push(hiragana);
        }
    } else {
        // Input contains non-ASCII (likely hiragana/katakana) → generate romaji form
        let romaji = hiragana_to_romaji(trimmed);
        if !candidates.contains(&romaji) {
            candidates.push(romaji);
        }
    }

    // Also try full conversions
    let hiragana_form = romaji_to_hiragana(trimmed);
    let romaji_form = hiragana_to_romaji(trimmed);

    if !candidates.contains(&hiragana_form) {
        candidates.push(hiragana_form);
    }
    if !candidates.contains(&romaji_form) {
        candidates.push(romaji_form);
    }

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_romaji_to_hiragana_basic() {
        assert_eq!(romaji_to_hiragana("ka"), "か");
        assert_eq!(romaji_to_hiragana("ki"), "き");
        assert_eq!(romaji_to_hiragana("ku"), "く");
        assert_eq!(romaji_to_hiragana("ke"), "け");
        assert_eq!(romaji_to_hiragana("ko"), "こ");
    }

    #[test]
    fn test_romaji_to_hiragana_vowels() {
        assert_eq!(romaji_to_hiragana("a"), "あ");
        assert_eq!(romaji_to_hiragana("i"), "い");
        assert_eq!(romaji_to_hiragana("u"), "う");
        assert_eq!(romaji_to_hiragana("e"), "え");
        assert_eq!(romaji_to_hiragana("o"), "お");
    }

    #[test]
    fn test_romaji_to_hiragana_special() {
        assert_eq!(romaji_to_hiragana("n"), "ん");
        assert_eq!(romaji_to_hiragana("tsu"), "つ");
        assert_eq!(romaji_to_hiragana("shi"), "し");
        assert_eq!(romaji_to_hiragana("chi"), "ち");
    }

    #[test]
    fn test_romaji_to_hiragana_combinations() {
        assert_eq!(romaji_to_hiragana("sha"), "しゃ");
        assert_eq!(romaji_to_hiragana("kya"), "きゃ");
        assert_eq!(romaji_to_hiragana("ryo"), "りょ");
    }

    #[test]
    fn test_hiragana_to_romaji() {
        assert_eq!(hiragana_to_romaji("か"), "ka");
        assert_eq!(hiragana_to_romaji("き"), "ki");
        assert_eq!(hiragana_to_romaji("ら"), "ra");
        assert_eq!(hiragana_to_romaji("ん"), "n");
    }

    #[test]
    fn test_lyric_candidates() {
        let cands = lyric_candidates("ka");
        assert!(cands.contains(&"ka".to_string()));
        assert!(cands.contains(&"か".to_string()));

        let cands2 = lyric_candidates("か");
        assert!(cands2.contains(&"か".to_string()));
        assert!(cands2.contains(&"ka".to_string()));
    }

    #[test]
    fn test_n_before_vowel() {
        // "na" should be な, not ん + あ
        assert_eq!(romaji_to_hiragana("na"), "な");
        assert_eq!(romaji_to_hiragana("ni"), "に");
    }
}
