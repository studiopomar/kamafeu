pub mod romaji;

use crate::oto::Voicebank;
use crate::project::model::UNote;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PhonemizerMode {
    BasicCV,
    VCV,
    CVVC,
}

impl Default for PhonemizerMode {
    fn default() -> Self {
        PhonemizerMode::BasicCV
    }
}

pub struct RenderPhone {
    pub lyric: String,
    pub pitch: String,
    pub position_ms: f64,
    pub duration_ms: f64,
    pub envelope: crate::dsp::envelope::UtauEnvelope,
    pub expressions: crate::project::model::UExpressions,
    pub pitch_bend: crate::project::model::UPitchBend,
    pub flags: String,
}

impl RenderPhone {
    pub fn midi_key(&self) -> u8 {
        crate::dsp::pitch::note_name_to_midi(&self.pitch).unwrap_or(60)
    }
}

pub struct JapanesePhonemizer;

impl JapanesePhonemizer {
    pub fn extract_vowel(lyric: &str) -> Option<&'static str> {
        let lyric_clean = lyric.trim();
        if lyric_clean.is_empty() { return None; }
        let h_lyric = romaji::romaji_to_hiragana(lyric_clean);
        match h_lyric.as_str() {
            "あ" | "か" | "さ" | "た" | "な" | "は" | "ま" | "や" | "ら" | "わ" | "が" | "ざ" | "だ" | "ば" | "ぱ" | "ファ" => Some("a"),
            "い" | "き" | "し" | "ち" | "に" | "ひ" | "み" | "り" | "ぎ" | "じ" | "ぢ" | "び" | "ぴ" | "フィ" => Some("i"),
            "う" | "く" | "す" | "つ" | "ぬ" | "ふ" | "む" | "ゆ" | "る" | "ん" | "ぐ" | "ず" | "づ" | "ぶ" | "ぷ" => Some("u"),
            "え" | "け" | "せ" | "て" | "ね" | "へ" | "め" | "れ" | "げ" | "ぜ" | "で" | "べ" | "ぺ" | "フェ" => Some("e"),
            "お" | "こ" | "そ" | "と" | "の" | "ほ" | "も" | "よ" | "ろ" | "を" | "ご" | "ぞ" | "ど" | "ぼ" | "ぽ" | "フォ" => Some("o"),
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

    pub fn extract_consonant(lyric: &str) -> Option<&'static str> {
        let lyric_clean = lyric.trim();
        if lyric_clean.is_empty() { return None; }
        let h_lyric = romaji::romaji_to_hiragana(lyric_clean);
        match h_lyric.as_str() {
            "か" | "き" | "く" | "け" | "こ" | "きゃ" | "きゅ" | "きょ" => Some("k"),
            "さ" | "し" | "す" | "せ" | "そ" | "しゃ" | "しゅ" | "しょ" => Some("s"),
            "た" | "ち" | "つ" | "て" | "と" | "ちゃ" | "ちゅ" | "ちょ" => Some("t"),
            "な" | "に" | "ぬ" | "ね" | "の" | "にゃ" | "にゅ" | "にょ" => Some("n"),
            "は" | "ひ" | "ふ" | "へ" | "ほ" | "ひゃ" | "ひゅ" | "ひょ" => Some("h"),
            "ま" | "み" | "む" | "め" | "も" | "みゃ" | "みゅ" | "みょ" => Some("m"),
            "や" | "ゆ" | "よ" => Some("y"),
            "ら" | "り" | "る" | "れ" | "ろ" | "りゃ" | "りゅ" | "りょ" => Some("r"),
            "わ" | "を" => Some("w"),
            "が" | "ぎ" | "ぐ" | "げ" | "ご" | "ぎゃ" | "ぎゅ" | "ぎょ" => Some("g"),
            "ざ" | "じ" | "ず" | "ぜ" | "ぞ" | "じゃ" | "じゅ" | "じょ" => Some("z"),
            "だ" | "ぢ" | "づ" | "で" | "ど" => Some("d"),
            "ば" | "び" | "ぶ" | "べ" | "ぼ" | "びゃ" | "びゅ" | "びょ" => Some("b"),
            "ぱ" | "ぴ" | "ぷ" | "ぺ" | "ぽ" | "ぴゃ" | "ぴゅ" | "ぴょ" => Some("p"),
            _ => None,
        }
    }

    pub fn apply_phonemizer(
        notes: &[UNote],
        vb: &Voicebank,
        mode: PhonemizerMode,
    ) -> Vec<RenderPhone> {
        let mut phones = Vec::new();
        let mut prev_vowel: Option<&'static str> = None;
        let mut prev_note_end_ms: Option<f64> = None;
        let vc_length_ms = 60.0; // Standard CVVC transition overlap

        for note in notes.iter() {
            let cur_vowel = Self::extract_vowel(&note.lyric);
            let cur_consonant = Self::extract_consonant(&note.lyric);

            // Detect phrase start (first note or gap > 60ms after previous note)
            let is_phrase_start = match prev_note_end_ms {
                Some(end_ms) => note.position_ms > end_ms + 60.0,
                None => true,
            };

            if is_phrase_start {
                prev_vowel = None;
            }

            match mode {
                PhonemizerMode::BasicCV => {
                    let mut alias = note.lyric.clone();
                    if is_phrase_start {
                        let head_alias = format!("- {}", note.lyric);
                        if let Some(entry) = vb.find_entry(&head_alias, &note.pitch) {
                            alias = entry.alias.clone();
                        }
                    }

                    phones.push(RenderPhone {
                        lyric: alias,
                        pitch: note.pitch.clone(),
                        position_ms: note.position_ms,
                        duration_ms: note.duration_ms,
                        envelope: note.envelope.clone(),
                        expressions: note.expressions.clone(),
                        pitch_bend: note.pitch_bend.clone(),
                        flags: note.flags.clone(),
                    });
                }
                PhonemizerMode::VCV => {
                    let mut alias = note.lyric.clone();
                    if !is_phrase_start {
                        if let Some(pv) = prev_vowel {
                            let vcv_alias = format!("{} {}", pv, note.lyric);
                            if let Some(entry) = vb.find_entry(&vcv_alias, &note.pitch) {
                                alias = entry.alias.clone();
                            }
                        }
                    }
                    if alias == note.lyric {
                        let head_alias = format!("- {}", note.lyric);
                        if let Some(entry) = vb.find_entry(&head_alias, &note.pitch) {
                            alias = entry.alias.clone();
                        }
                    }

                    phones.push(RenderPhone {
                        lyric: alias,
                        pitch: note.pitch.clone(),
                        position_ms: note.position_ms,
                        duration_ms: note.duration_ms,
                        envelope: note.envelope.clone(),
                        expressions: note.expressions.clone(),
                        pitch_bend: note.pitch_bend.clone(),
                        flags: note.flags.clone(),
                    });
                }
                PhonemizerMode::CVVC => {
                    let mut cv_alias = note.lyric.clone();

                    if !is_phrase_start {
                        // Inject VC note if possible
                        if let (Some(pv), Some(cc)) = (prev_vowel, cur_consonant) {
                            let vc_alias = format!("{} {}", pv, cc);
                            if let Some(entry) = vb.find_entry(&vc_alias, &note.pitch) {
                                // Cut duration from previous CV note
                                if let Some(last_phone) = phones.last_mut() {
                                    if last_phone.duration_ms > vc_length_ms {
                                        last_phone.duration_ms -= vc_length_ms;
                                    }
                                }
                                // Inject VC transition
                                phones.push(RenderPhone {
                                    lyric: entry.alias.clone(),
                                    pitch: note.pitch.clone(),
                                    position_ms: note.position_ms - vc_length_ms,
                                    duration_ms: vc_length_ms,
                                    envelope: crate::dsp::envelope::UtauEnvelope::default(),
                                    expressions: note.expressions.clone(),
                                    pitch_bend: crate::project::model::UPitchBend::default(),
                                    flags: note.flags.clone(),
                                });
                            }
                        }
                    } else {
                        // Phrase start: check for "- CV" or "- V" head note
                        let head_alias = format!("- {}", note.lyric);
                        if let Some(entry) = vb.find_entry(&head_alias, &note.pitch) {
                            cv_alias = entry.alias.clone();
                        }
                    }

                    // Push the main CV / head note
                    phones.push(RenderPhone {
                        lyric: cv_alias,
                        pitch: note.pitch.clone(),
                        position_ms: note.position_ms,
                        duration_ms: note.duration_ms,
                        envelope: note.envelope.clone(),
                        expressions: note.expressions.clone(),
                        pitch_bend: note.pitch_bend.clone(),
                        flags: note.flags.clone(),
                    });
                }
            }

            prev_vowel = cur_vowel;
            prev_note_end_ms = Some(note.position_ms + note.duration_ms);
        }

        phones
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_phrase_start_head_alias() {
        let mut entries = HashMap::new();
        entries.insert(
            "- か".to_string(),
            crate::oto::OtoEntry::new("ka.wav".to_string(), "- か".to_string(), 0.0, 50.0, 0.0, 0.0, 0.0),
        );
        entries.insert(
            "a き".to_string(),
            crate::oto::OtoEntry::new("ki.wav".to_string(), "a き".to_string(), 0.0, 50.0, 0.0, 0.0, 0.0),
        );

        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            prefix_map: crate::oto::PrefixMap::default(),
        };

        // Note 1 (phrase start): "ka" -> should match "- か"
        // Note 2 (continuation): "ki" at 500ms -> should match VCV "a き"
        // Note 3 (after rest at 2000ms): "ka" -> should match "- か" again
        let notes = vec![
            UNote::new("ka", "C4", 0.0, 400.0),
            UNote::new("ki", "C4", 400.0, 400.0),
            UNote::new("ka", "C4", 2000.0, 400.0),
        ];

        let phones = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::VCV);
        assert_eq!(phones.len(), 3);
        assert_eq!(phones[0].lyric, "- か");
        assert_eq!(phones[1].lyric, "a き");
        assert_eq!(phones[2].lyric, "- か");
    }
}
