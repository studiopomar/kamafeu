const ROMAJI_HIRAGANA_TABLE: &[(&str, &str)] = &[
    ("tsya", "ちゃ"),
    ("tsyu", "ちゅ"),
    ("tsyo", "ちょ"),
    ("cshi", "っし"),
    ("cchi", "っち"),
    ("ttsu", "っつ"),
    ("kya", "きゃ"),
    ("kyi", "きぃ"),
    ("kyu", "きゅ"),
    ("kye", "きぇ"),
    ("kyo", "きょ"),
    ("qya", "くゃ"),
    ("qyu", "くゅ"),
    ("qyo", "くょ"),
    ("sha", "しゃ"),
    ("shi", "し"),
    ("shu", "しゅ"),
    ("she", "しぇ"),
    ("sho", "しょ"),
    ("sya", "しゃ"),
    ("syi", "し"),
    ("syu", "しゅ"),
    ("sye", "しぇ"),
    ("syo", "しょ"),
    ("cha", "ちゃ"),
    ("chi", "ち"),
    ("chu", "ちゅ"),
    ("che", "ちぇ"),
    ("cho", "ちょ"),
    ("cya", "ちゃ"),
    ("cyi", "ちぃ"),
    ("cyu", "ちゅ"),
    ("cye", "ちぇ"),
    ("cyo", "ちょ"),
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
    ("lya", "りゃ"),
    ("lyi", "りぃ"),
    ("lyu", "りゅ"),
    ("lye", "りぇ"),
    ("lyo", "りょ"),
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
    ("zya", "じゃ"),
    ("zyi", "じぃ"),
    ("zyu", "じゅ"),
    ("zye", "じぇ"),
    ("zyo", "じょ"),
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
    ("fya", "ふゃ"),
    ("fyu", "ふゅ"),
    ("fyo", "ふょ"),
    ("va", "ゔぁ"),
    ("vi", "ゔぃ"),
    ("vu", "ゔ"),
    ("ve", "ゔぇ"),
    ("vo", "ゔぉ"),
    ("vya", "ゔゃ"),
    ("vyu", "ゔゅ"),
    ("vyo", "ゔょ"),
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
    ("ka", "か"),
    ("ki", "き"),
    ("ku", "く"),
    ("ke", "け"),
    ("ko", "こ"),
    ("sa", "さ"),
    ("si", "し"),
    ("su", "す"),
    ("se", "せ"),
    ("so", "そ"),
    ("ta", "た"),
    ("ti", "ち"),
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
    ("yi", "い"),
    ("yu", "ゆ"),
    ("ye", "いぇ"),
    ("yo", "よ"),
    ("ra", "ら"),
    ("ri", "り"),
    ("ru", "る"),
    ("re", "れ"),
    ("ro", "ろ"),
    ("la", "ら"),
    ("li", "り"),
    ("lu", "る"),
    ("le", "れ"),
    ("lo", "ろ"),
    ("wa", "わ"),
    ("wi", "ゐ"),
    ("wu", "う"),
    ("we", "ゑ"),
    ("wo", "を"),
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
    ("pa", "ぱ"),
    ("pi", "ぴ"),
    ("pu", "ぷ"),
    ("pe", "ぺ"),
    ("po", "ぽ"),
    ("a", "あ"),
    ("i", "い"),
    ("u", "う"),
    ("e", "え"),
    ("o", "お"),
    ("n", "ん"),
    ("nn", "ん"),
    ("xa", "ぁ"),
    ("xi", "ぃ"),
    ("xu", "ぅ"),
    ("xe", "ぇ"),
    ("xo", "ぉ"),
    ("la", "ぁ"),
    ("li", "ぃ"),
    ("lu", "ぅ"),
    ("le", "ぇ"),
    ("lo", "ぉ"),
    ("xya", "ゃ"),
    ("xyu", "ゅ"),
    ("xyo", "ょ"),
    ("xtu", "っ"),
    ("xtsu", "っ"),
    ("ltu", "っ"),
    ("ltsu", "っ"),
];

pub fn katakana_to_hiragana(input: &str) -> String {
    input
        .chars()
        .map(|c| {
            if ('\u{30A1}'..='\u{30F6}').contains(&c) {
                std::char::from_u32(c as u32 - 0x60).unwrap_or(c)
            } else {
                c
            }
        })
        .collect()
}

pub fn romaji_to_hiragana(input: &str) -> String {
    let input_normalized = katakana_to_hiragana(input);
    let input_lower = input_normalized.to_lowercase();
    let chars: Vec<char> = input_lower.chars().collect();
    let mut result = String::new();
    let mut i = 0;

    while i < chars.len() {
        let remaining = &input_lower[chars[..i].iter().map(|c| c.len_utf8()).sum::<usize>()..];
        let mut matched = false;

        for &(romaji, hiragana) in ROMAJI_HIRAGANA_TABLE {
            if remaining.starts_with(romaji) {
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

pub fn hiragana_to_romaji(input: &str) -> String {
    let input_normalized = katakana_to_hiragana(input);
    let mut result = String::new();
    let chars: Vec<char> = input_normalized.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        let remaining = &input_normalized[chars[..i].iter().map(|c| c.len_utf8()).sum::<usize>()..];
        let mut matched = false;

        for &(romaji, hiragana) in ROMAJI_HIRAGANA_TABLE {
            if remaining.starts_with(hiragana) {
                result.push_str(romaji);
                i += hiragana.chars().count();
                matched = true;
                break;
            }
        }

        if !matched {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}

pub fn lyric_candidates(lyric: &str) -> Vec<String> {
    let trimmed = lyric.trim();
    if trimmed.is_empty() {
        return vec![];
    }

    let mut candidates = Vec::new();
    candidates.push(trimmed.to_string());

    if let Some(space_idx) = trimmed.find([' ', '_']) {
        let sep = &trimmed[space_idx..=space_idx];
        let prefix = &trimmed[..space_idx];
        let body = &trimmed[space_idx + 1..];

        if !body.is_empty() {
            let h_prefix = romaji_to_hiragana(prefix);
            let r_prefix = hiragana_to_romaji(prefix);
            let h_body = romaji_to_hiragana(body);
            let r_body = hiragana_to_romaji(body);

            let variants = [
                format!("{}{}{}", prefix, sep, h_body),
                format!("{}{}{}", prefix, sep, r_body),
                format!("{}{}{}", h_prefix, sep, body),
                format!("{}{}{}", h_prefix, sep, h_body),
                format!("{}{}{}", h_prefix, sep, r_body),
                format!("{}{}{}", r_prefix, sep, body),
                format!("{}{}{}", r_prefix, sep, h_body),
                format!("{}{}{}", r_prefix, sep, r_body),
                format!("{}{}", h_prefix, h_body),
                format!("{}{}", r_prefix, r_body),
            ];

            for v in variants {
                if !candidates.contains(&v) {
                    candidates.push(v);
                }
            }
        }
    }

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
        assert_eq!(romaji_to_hiragana("si"), "し");
        assert_eq!(romaji_to_hiragana("ti"), "ち");
        assert_eq!(romaji_to_hiragana("tu"), "つ");
        assert_eq!(romaji_to_hiragana("hu"), "ふ");
        assert_eq!(romaji_to_hiragana("zi"), "じ");
        assert_eq!(romaji_to_hiragana("di"), "ぢ");
        assert_eq!(romaji_to_hiragana("du"), "づ");
    }

    #[test]
    fn test_romaji_to_hiragana_combinations() {
        assert_eq!(romaji_to_hiragana("sha"), "しゃ");
        assert_eq!(romaji_to_hiragana("kya"), "きゃ");
        assert_eq!(romaji_to_hiragana("ryo"), "りょ");
        assert_eq!(romaji_to_hiragana("sya"), "しゃ");
        assert_eq!(romaji_to_hiragana("cya"), "ちゃ");
        assert_eq!(romaji_to_hiragana("zya"), "じゃ");
    }

    #[test]
    fn test_katakana_to_hiragana() {
        assert_eq!(katakana_to_hiragana("カ"), "か");
        assert_eq!(katakana_to_hiragana("ラーメン"), "らーめん");
        assert_eq!(romaji_to_hiragana("カ"), "か");
    }

    #[test]
    fn test_hiragana_to_romaji() {
        assert_eq!(hiragana_to_romaji("か"), "ka");
        assert_eq!(hiragana_to_romaji("き"), "ki");
        assert_eq!(hiragana_to_romaji("ら"), "ra");
        assert_eq!(hiragana_to_romaji("ん"), "n");
        assert_eq!(hiragana_to_romaji("カ"), "ka");
    }

    #[test]
    fn test_lyric_candidates() {
        let cands = lyric_candidates("ka");
        assert!(cands.contains(&"ka".to_string()));
        assert!(cands.contains(&"か".to_string()));

        let cands2 = lyric_candidates("か");
        assert!(cands2.contains(&"か".to_string()));
        assert!(cands2.contains(&"ka".to_string()));

        let cands3 = lyric_candidates("a ka");
        assert!(cands3.contains(&"a ka".to_string()));
        assert!(cands3.contains(&"a か".to_string()));
        assert!(cands3.contains(&"あ ka".to_string()));
        assert!(cands3.contains(&"あ か".to_string()));

        let cands4 = lyric_candidates("a k");
        assert!(cands4.contains(&"a k".to_string()));
        assert!(cands4.contains(&"あ k".to_string()));
    }

    #[test]
    fn test_n_before_vowel() {
        assert_eq!(romaji_to_hiragana("na"), "な");
        assert_eq!(romaji_to_hiragana("ni"), "に");
    }
}
