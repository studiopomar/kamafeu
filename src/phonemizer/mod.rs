use crate::oto::Voicebank;

pub struct JapanesePhonemizer;

impl JapanesePhonemizer {
    /// Extract the ending vowel of a lyric string (e.g. "ka" -> "a", "shi" -> "i", "tsu" -> "u", "a" -> "a")
    pub fn extract_vowel(lyric: &str) -> Option<&'static str> {
        let lyric_clean = lyric.trim();
        if lyric_clean.is_empty() {
            return None;
        }

        // Japanese Romaji & Kana mapping helpers
        match lyric_clean {
            "a" | "か" | "さ" | "た" | "な" | "は" | "ま" | "や" | "ら" | "わ" | "が" | "ざ" | "だ" | "ば" | "ぱ" | "ファ" => Some("a"),
            "i" | "き" | "し" | "ち" | "に" | "ひ" | "み" | "り" | "ぎ" | "じ" | "ぢ" | "び" | "ぴ" | "フィ" => Some("i"),
            "u" | "く" | "す" | "つ" | "ぬ" | "ふ" | "む" | "ゆ" | "る" | "ん" | "ぐ" | "ず" | "づ" | "ぶ" | "ぷ" => Some("u"),
            "e" | "け" | "せ" | "て" | "ね" | "へ" | "め" | "れ" | "げ" | "ぜ" | "で" | "べ" | "ぺ" | "フェ" => Some("e"),
            "o" | "こ" | "そ" | "と" | "の" | "ほ" | "も" | "よ" | "ろ" | "を" | "ご" | "ぞ" | "ど" | "ぼ" | "ぽ" | "フォ" => Some("o"),
            _ => {
                let last_char = lyric_clean.chars().last().unwrap_or(' ');
                match last_char {
                    'a' | 'A' => Some("a"),
                    'i' | 'I' => Some("i"),
                    'u' | 'U' => Some("u"),
                    'e' | 'E' => Some("e"),
                    'o' | 'O' => Some("o"),
                    'n' | 'N' => Some("n"),
                    _ => None,
                }
            }
        }
    }

    /// Resolve effective phoneme alias for a target note, attempting VCV matching first if previous vowel is known.
    pub fn resolve_alias(
        vb: &Voicebank,
        prev_vowel: Option<&str>,
        lyric: &str,
        pitch_name: &str,
    ) -> String {
        // Try VCV format (e.g. "a ka", "- ka")
        if let Some(pv) = prev_vowel {
            let vcv_alias = format!("{} {}", pv, lyric);
            if vb.find_entry(&vcv_alias, pitch_name).is_some() {
                return vcv_alias;
            }
        } else {
            let vcv_head = format!("- {}", lyric);
            if vb.find_entry(&vcv_head, pitch_name).is_some() {
                return vcv_head;
            }
        }

        // Fallback to plain lyric / CV entry
        lyric.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vowel_extraction() {
        assert_eq!(JapanesePhonemizer::extract_vowel("ka"), Some("a"));
        assert_eq!(JapanesePhonemizer::extract_vowel("shi"), Some("i"));
        assert_eq!(JapanesePhonemizer::extract_vowel("か"), Some("a"));
        assert_eq!(JapanesePhonemizer::extract_vowel("く"), Some("u"));
    }
}
