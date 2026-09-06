pub mod brapa;
pub mod english;
pub mod japanese;
pub mod portuguese;
pub mod romaji;

pub use brapa::{BrapaCVCPhonemizer, VccvBrapaPhonemizer};
pub use english::EnglishPhonemizer;
pub use japanese::JapanesePhonemizer as CoreJapanesePhonemizer;
pub use portuguese::PortuguesePhonemizer;

use crate::oto::Voicebank;
use crate::project::model::UNote;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default,
)]
pub enum PhonemizerMode {
    #[default]
    None, // Sem Fonemizador (Raw / Manual / Direto)
    BasicCV,             // JA: Japanese CV (Básico - Fonética/Romaji/Kana)
    VCV,                 // JA: Japanese VCV (Completo - Fonética/Romaji/Kana)
    CVVC,                // JA: Japanese CVVC (Fonética/Romaji/Kana)
    EnglishArpasing,     // EN: English Arpasing (Fonética Direta)
    EnglishVCCV,         // EN: English VCCV (Fonética Direta)
    EnglishG2P,          // EN: English G2P (Palavras / Texto -> ARPABET)
    PortugueseBrapaVCCV, // PT: VCCV BRAPA (xiao / PT-BR 3.7)
    PortugueseBrapaCVC,  // PT: BRAPA CVC (Fonética Direta / Tokens BRAPA)
    PortugueseCVVC,      // PT: Portuguese CVVC (Fonética Direta)
    PortugueseVCV,       // PT: Portuguese VCV (Fonética Direta)
    PortugueseG2P,       // PT: Português G2P (Palavras / Texto em Português -> Fonemas)
}

pub fn consonant_velocity_time_scale(velocity: f64) -> f64 {
    let velocity = if velocity.is_finite() {
        velocity
    } else {
        100.0
    };
    2.0f64.powf(1.0 - velocity.clamp(0.0, 200.0) / 100.0)
}

pub struct RenderPhone {
    pub note_index: usize,
    pub lyric: String,
    pub pitch: String,
    pub position_ms: f64,
    pub duration_ms: f64,
    pub envelope: crate::dsp::envelope::UtauEnvelope,
    pub expressions: crate::project::model::UExpressions,
    pub pitch_bend: crate::project::model::UPitchBend,
    pub vibrato: crate::dsp::pitch::VibratoParam,
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
        CoreJapanesePhonemizer::extract_vowel(lyric)
    }

    pub fn extract_consonant(lyric: &str) -> Option<&'static str> {
        CoreJapanesePhonemizer::extract_consonant(lyric)
    }

    pub fn preprocess_plus_notes(notes: &[UNote]) -> Vec<UNote> {
        let mut result: Vec<UNote> = Vec::new();
        for note in notes {
            let lyric_trimmed = note.lyric.trim();
            let is_plus = lyric_trimmed == "+" || lyric_trimmed.starts_with("+ ");

            if is_plus {
                if let Some(last) = result.last_mut() {
                    let last_end = last.position_ms + last.duration_ms;
                    if (note.position_ms - last_end).abs() <= 60.0 {
                        last.duration_ms += note.duration_ms;
                        continue;
                    }
                }
            }

            result.push(note.clone());
        }
        result
    }

    pub fn apply_phonemizer(
        notes: &[UNote],
        vb: &Voicebank,
        mode: PhonemizerMode,
    ) -> Vec<RenderPhone> {
        let mut normalized_notes: Vec<(usize, UNote)> = Vec::new();
        for (orig_idx, note) in notes.iter().enumerate() {
            let lyric_trimmed = note.lyric.trim();
            let is_plus = lyric_trimmed == "+" || lyric_trimmed.starts_with("+ ");

            if is_plus {
                if let Some((_last_orig_idx, last_note)) = normalized_notes.last_mut() {
                    let last_end = last_note.position_ms + last_note.duration_ms;
                    if (note.position_ms - last_end).abs() <= 60.0 {
                        last_note.duration_ms += note.duration_ms;
                        continue;
                    }
                }
            }

            normalized_notes.push((orig_idx, note.clone()));
        }

        let temp_notes: Vec<UNote> = normalized_notes.iter().map(|(_, n)| n.clone()).collect();
        let orig_indices: Vec<usize> = normalized_notes.iter().map(|(idx, _)| *idx).collect();

        let mut phones = match mode {
            PhonemizerMode::None => Self::apply_raw_passthrough(&temp_notes),
            PhonemizerMode::PortugueseBrapaVCCV | PhonemizerMode::PortugueseBrapaCVC => {
                VccvBrapaPhonemizer::apply_phonemizer(&temp_notes, vb)
            }
            PhonemizerMode::EnglishArpasing
            | PhonemizerMode::EnglishVCCV
            | PhonemizerMode::EnglishG2P => Self::apply_english(&temp_notes, vb, mode),
            PhonemizerMode::PortugueseCVVC
            | PhonemizerMode::PortugueseVCV
            | PhonemizerMode::PortugueseG2P => Self::apply_portuguese(&temp_notes, vb, mode),
            _ => Self::apply_japanese(&temp_notes, vb, mode),
        };

        for p in &mut phones {
            if let Some(&real_idx) = orig_indices.get(p.note_index) {
                p.note_index = real_idx;
            }
        }

        for (orig_idx, note) in notes.iter().enumerate() {
            if !note.phoneme_durations_ms.is_empty() {
                let note_phone_indices: Vec<usize> = phones
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.note_index == orig_idx)
                    .map(|(idx, _)| idx)
                    .collect();

                if note_phone_indices.len() == note.phoneme_durations_ms.len() {
                    let initial_offset = phones[note_phone_indices[0]].position_ms;
                    let mut cur_pos = initial_offset;
                    for (i, &phone_idx) in note_phone_indices.iter().enumerate() {
                        let dur = note.phoneme_durations_ms[i];
                        phones[phone_idx].position_ms = cur_pos;
                        phones[phone_idx].duration_ms = dur;
                        cur_pos += dur;
                    }
                }
            }
        }

        phones
    }

    fn apply_raw_passthrough(notes: &[UNote]) -> Vec<RenderPhone> {
        let mut phones: Vec<RenderPhone> = Vec::new();
        for (i, note) in notes.iter().enumerate() {
            let raw_lyric = note.lyric.trim();
            if raw_lyric.is_empty() || raw_lyric == "R" || raw_lyric == "r" || raw_lyric == "+" {
                continue;
            }

            if raw_lyric.contains('.') || raw_lyric.contains(';') || raw_lyric.contains(',') {
                let parts: Vec<&str> = raw_lyric
                    .split(|c| c == '.' || c == ';' || c == ',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty() && *s != "+" && *s != "R" && *s != "r")
                    .collect();

                if !parts.is_empty() {
                    let num_parts = parts.len();
                    let durations = note.resolved_phoneme_durations(num_parts);
                    let mut sub_position_ms = note.position_ms;
                    for (part_idx, part) in parts.into_iter().enumerate() {
                        let sub_duration_ms = durations[part_idx];
                        let mut sub_envelope = note.envelope.clone();
                        if part_idx > 0 {
                            sub_envelope.p1 = 0.0;
                            sub_envelope.p2 = 0.0;
                            sub_envelope.v1 = 100.0;
                            sub_envelope.v2 = 100.0;
                        }
                        if part_idx < num_parts - 1 {
                            sub_envelope.p4 = 0.0;
                            sub_envelope.p5 = 0.0;
                            sub_envelope.v4 = 100.0;
                            sub_envelope.v5 = 100.0;
                        }
                        phones.push(RenderPhone {
                            note_index: i,
                            lyric: part.to_string(),
                            pitch: note.pitch.clone(),
                            position_ms: sub_position_ms,
                            duration_ms: sub_duration_ms,
                            envelope: sub_envelope,
                            expressions: note.expressions.clone(),
                            pitch_bend: note.pitch_bend.clone(),
                            vibrato: note.vibrato.clone(),
                            flags: note.flags.clone(),
                        });
                        sub_position_ms += sub_duration_ms;
                    }
                    continue;
                }
            }

            phones.push(RenderPhone {
                note_index: i,
                lyric: raw_lyric.to_string(),
                pitch: note.pitch.clone(),
                position_ms: note.position_ms,
                duration_ms: note.duration_ms,
                envelope: note.envelope.clone(),
                expressions: note.expressions.clone(),
                pitch_bend: note.pitch_bend.clone(),
                vibrato: note.vibrato.clone(),
                flags: note.flags.clone(),
            });
        }
        phones
    }

    fn apply_japanese(notes: &[UNote], vb: &Voicebank, mode: PhonemizerMode) -> Vec<RenderPhone> {
        CoreJapanesePhonemizer::apply_japanese(notes, vb, mode)
    }

    fn apply_english(notes: &[UNote], vb: &Voicebank, mode: PhonemizerMode) -> Vec<RenderPhone> {
        let mut phones: Vec<RenderPhone> = Vec::new();
        let mut prev_vowel: Option<String> = None;
        let mut prev_note_end_ms: Option<f64> = None;

        for (note_index, note) in notes.iter().enumerate() {
            let is_phrase_start = match prev_note_end_ms {
                Some(end_ms) => note.position_ms > end_ms + 60.0,
                None => true,
            };

            if is_phrase_start {
                prev_vowel = None;
            }

            let word_phones = match mode {
                PhonemizerMode::EnglishG2P => EnglishPhonemizer::word_to_arpabet(&note.lyric),
                _ => EnglishPhonemizer::phonetic_tokens(&note.lyric),
            };
            if word_phones.is_empty() {
                continue;
            }

            let sub_dur = note.duration_ms / word_phones.len() as f64;
            for (idx, p) in word_phones.iter().enumerate() {
                let sub_pos = note.position_ms + (idx as f64 * sub_dur);
                let mut alias = p.clone();

                if is_phrase_start && idx == 0 {
                    let head_try = format!("- {}", p);
                    if let Some(entry) = vb.find_entry(&head_try, &note.pitch) {
                        alias = entry.alias.clone();
                    }
                } else if idx == 0 && !is_phrase_start {
                    if let Some(ref pv) = prev_vowel {
                        let vc_try = format!("{} {}", pv, p);
                        if let Some(entry) = vb.find_entry(&vc_try, &note.pitch) {
                            let authored = ((entry.preutterance - entry.overlap).abs()
                                * consonant_velocity_time_scale(
                                    note.expressions.consonant_velocity,
                                ))
                            .max(10.0);
                            let base_vc = if authored > 25.0 { authored } else { 130.0 };
                            let vc_dur = base_vc.clamp(30.0, (note.duration_ms * 0.45).max(35.0));
                            if let Some(last) = phones.last_mut() {
                                let borrow = vc_dur.min((last.duration_ms - 20.0).max(0.0));
                                if borrow > 0.0 {
                                    last.duration_ms -= borrow;
                                    phones.push(RenderPhone {
                                        note_index,
                                        lyric: entry.alias.clone(),
                                        pitch: note.pitch.clone(),
                                        position_ms: sub_pos - borrow,
                                        duration_ms: borrow,
                                        envelope: crate::dsp::envelope::UtauEnvelope::default(),
                                        expressions: note.expressions.clone(),
                                        pitch_bend: crate::project::model::UPitchBend::default(),
                                        vibrato: crate::dsp::pitch::VibratoParam::default(),
                                        flags: note.flags.clone(),
                                    });
                                }
                            }
                        }
                    }
                } else if let Some(ref pv) = prev_vowel {
                    let vc_try = format!("{} {}", pv, p);
                    if let Some(entry) = vb.find_entry(&vc_try, &note.pitch) {
                        alias = entry.alias.clone();
                    }
                }

                if EnglishPhonemizer::is_vowel(p) {
                    prev_vowel = Some(p.clone());
                } else {
                    prev_vowel = None;
                }

                if let Some(entry) = vb.find_entry(&alias, &note.pitch) {
                    alias = entry.alias.clone();
                }

                phones.push(RenderPhone {
                    note_index,
                    lyric: alias,
                    pitch: note.pitch.clone(),
                    position_ms: sub_pos,
                    duration_ms: sub_dur,
                    envelope: note.envelope.clone(),
                    expressions: note.expressions.clone(),
                    pitch_bend: note.pitch_bend.clone(),
                    vibrato: note.vibrato.clone(),
                    flags: note.flags.clone(),
                });
            }

            prev_note_end_ms = Some(note.position_ms + note.duration_ms);
        }

        phones
    }

    fn apply_portuguese(notes: &[UNote], vb: &Voicebank, mode: PhonemizerMode) -> Vec<RenderPhone> {
        let mut phones: Vec<RenderPhone> = Vec::new();
        let mut prev_vowel: Option<String> = None;
        let mut prev_note_end_ms: Option<f64> = None;

        for (note_index, note) in notes.iter().enumerate() {
            let is_phrase_start = match prev_note_end_ms {
                Some(end_ms) => note.position_ms > end_ms + 60.0,
                None => true,
            };

            if is_phrase_start {
                prev_vowel = None;
            }

            let syllables = match mode {
                PhonemizerMode::PortugueseG2P => {
                    PortuguesePhonemizer::word_to_phonemes(&note.lyric)
                }
                _ => PortuguesePhonemizer::phonetic_tokens(&note.lyric),
            };
            if syllables.is_empty() {
                continue;
            }

            let sub_dur = note.duration_ms / syllables.len() as f64;
            for (idx, syl) in syllables.iter().enumerate() {
                let sub_pos = note.position_ms + (idx as f64 * sub_dur);
                let mut alias = syl.clone();

                if is_phrase_start && idx == 0 {
                    let head_try = format!("- {}", syl);
                    if let Some(entry) = vb.find_entry(&head_try, &note.pitch) {
                        alias = entry.alias.clone();
                    }
                } else if mode == PhonemizerMode::PortugueseVCV {
                    if let Some(ref pv) = prev_vowel {
                        let vcv_try = format!("{} {}", pv, syl);
                        if let Some(entry) = vb.find_entry(&vcv_try, &note.pitch) {
                            alias = entry.alias.clone();
                        }
                    }
                } else if mode == PhonemizerMode::PortugueseCVVC {
                    if let (Some(ref pv), Some(cc)) = (
                        prev_vowel.as_ref(),
                        PortuguesePhonemizer::extract_consonant(syl),
                    ) {
                        let vc_try = format!("{} {}", pv, cc);
                        if let Some(entry) = vb.find_entry(&vc_try, &note.pitch) {
                            let authored = ((entry.preutterance - entry.overlap).abs()
                                * consonant_velocity_time_scale(
                                    note.expressions.consonant_velocity,
                                ))
                            .max(10.0);
                            let base_vc = if authored > 25.0 { authored } else { 135.0 };
                            let vc_dur = base_vc.clamp(30.0, (sub_dur * 0.45).max(35.0));
                            if let Some(last) = phones.last_mut() {
                                let borrow = vc_dur.min((last.duration_ms - 20.0).max(0.0));
                                if borrow > 0.0 {
                                    last.duration_ms -= borrow;
                                    phones.push(RenderPhone {
                                        note_index,
                                        lyric: entry.alias.clone(),
                                        pitch: note.pitch.clone(),
                                        position_ms: sub_pos - borrow,
                                        duration_ms: borrow,
                                        envelope: crate::dsp::envelope::UtauEnvelope::default(),
                                        expressions: note.expressions.clone(),
                                        pitch_bend: crate::project::model::UPitchBend::default(),
                                        vibrato: crate::dsp::pitch::VibratoParam::default(),
                                        flags: note.flags.clone(),
                                    });
                                }
                            }
                        }
                    }
                }

                if let Some(v) = PortuguesePhonemizer::extract_vowel(syl) {
                    prev_vowel = Some(v.to_string());
                }

                if let Some(entry) = vb.find_entry(&alias, &note.pitch) {
                    alias = entry.alias.clone();
                }

                phones.push(RenderPhone {
                    note_index,
                    lyric: alias,
                    pitch: note.pitch.clone(),
                    position_ms: sub_pos,
                    duration_ms: sub_dur,
                    envelope: note.envelope.clone(),
                    expressions: note.expressions.clone(),
                    pitch_bend: note.pitch_bend.clone(),
                    vibrato: note.vibrato.clone(),
                    flags: note.flags.clone(),
                });
            }

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
            crate::oto::OtoEntry::new(
                "ka.wav".to_string(),
                "- か".to_string(),
                0.0,
                50.0,
                0.0,
                0.0,
                0.0,
            ),
        );
        entries.insert(
            "a き".to_string(),
            crate::oto::OtoEntry::new(
                "ki.wav".to_string(),
                "a き".to_string(),
                0.0,
                50.0,
                0.0,
                0.0,
                0.0,
            ),
        );

        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

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

    #[test]
    fn test_english_phonemizer() {
        let phones = EnglishPhonemizer::word_to_arpabet("hello");
        assert_eq!(phones, vec!["hh", "ah", "l", "ow"]);

        let sing = EnglishPhonemizer::word_to_arpabet("sing");
        assert_eq!(sing, vec!["s", "ih", "ng"]);
    }

    #[test]
    fn test_portuguese_phonemizer() {
        let phones = PortuguesePhonemizer::word_to_phonemes("canto");
        assert_eq!(phones, vec!["ka", "n", "to"]);

        let amor = PortuguesePhonemizer::word_to_phonemes("amor");
        assert_eq!(amor, vec!["a", "mo", "r"]);
    }

    #[test]
    fn test_none_phonemizer_passthrough() {
        let entries = std::collections::HashMap::new();
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let notes = vec![
            UNote::new("k_a", "C4", 0.0, 400.0),
            UNote::new("R", "C4", 400.0, 200.0),
            UNote::new("- sa", "D4", 600.0, 400.0),
        ];

        let phones = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::None);
        assert_eq!(phones.len(), 2);
        assert_eq!(phones[0].lyric, "k_a");
        assert_eq!(phones[0].position_ms, 0.0);
        assert_eq!(phones[1].lyric, "- sa");
        assert_eq!(phones[1].position_ms, 600.0);
    }

    #[test]
    fn test_universal_plus_continuation_in_any_mode() {
        let entries = std::collections::HashMap::new();
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let notes = vec![
            UNote::new("k aa", "C4", 0.0, 500.0),
            UNote::new("+", "D4", 500.0, 250.0),
            UNote::new("+", "E4", 750.0, 250.0),
        ];

        let phones_none = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::None);
        assert_eq!(phones_none.len(), 1);
        assert_eq!(phones_none[0].lyric, "k aa");
        assert_eq!(phones_none[0].duration_ms, 1000.0); // 500 + 250 + 250

        let notes_en = vec![
            UNote::new("sing", "C4", 0.0, 500.0),
            UNote::new("+", "D4", 500.0, 250.0),
            UNote::new("+", "E4", 750.0, 250.0),
        ];
        let phones_en =
            JapanesePhonemizer::apply_phonemizer(&notes_en, &vb, PhonemizerMode::EnglishArpasing);
        let total_dur: f64 = phones_en.iter().map(|p| p.duration_ms).sum();
        assert_eq!(total_dur, 1000.0);
    }

    #[test]
    fn test_manual_mode_semicolon_and_comma_sub_phonemes() {
        let entries = std::collections::HashMap::new();
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let notes = vec![UNote::new("-k;k ae;ae n", "C4", 0.0, 300.0)];

        let phones = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::None);
        assert_eq!(phones.len(), 3);
        assert_eq!(phones[0].lyric, "-k");
        assert_eq!(phones[0].duration_ms, 100.0);
        assert_eq!(phones[1].lyric, "k ae");
        assert_eq!(phones[1].duration_ms, 100.0);
        assert_eq!(phones[2].lyric, "ae n");
        assert_eq!(phones[2].duration_ms, 100.0);

        let notes_comma = vec![UNote::new("k ae, ae n", "C4", 0.0, 300.0)];
        let phones_comma =
            JapanesePhonemizer::apply_phonemizer(&notes_comma, &vb, PhonemizerMode::None);
        assert_eq!(phones_comma.len(), 2);
        assert_eq!(phones_comma[0].lyric, "k ae");
        assert_eq!(phones_comma[0].duration_ms, 150.0);
        assert_eq!(phones_comma[1].lyric, "ae n");
        assert_eq!(phones_comma[1].duration_ms, 150.0);

        let mut resized_note = UNote::new("k ae.ae n.", "B3", 0.0, 480.0);
        resized_note.set_phoneme_boundary(2, 1, 360.0);
        let resized =
            JapanesePhonemizer::apply_phonemizer(&[resized_note], &vb, PhonemizerMode::None);
        assert_eq!(resized.len(), 2);
        assert_eq!(resized[0].lyric, "k ae");
        assert_eq!(resized[0].position_ms, 0.0);
        assert_eq!(resized[0].duration_ms, 360.0);
        assert_eq!(resized[1].lyric, "ae n");
        assert_eq!(resized[1].position_ms, 360.0);
        assert_eq!(resized[1].duration_ms, 120.0);

        let notes_dot = vec![UNote::new("m an. an d. d eh. eh l. l a.", "C4", 0.0, 500.0)];
        let phones_dot =
            JapanesePhonemizer::apply_phonemizer(&notes_dot, &vb, PhonemizerMode::None);
        assert_eq!(phones_dot.len(), 5);
        assert_eq!(phones_dot[0].lyric, "m an");
        assert_eq!(phones_dot[0].duration_ms, 100.0);
        assert_eq!(phones_dot[1].lyric, "an d");
        assert_eq!(phones_dot[1].duration_ms, 100.0);
        assert_eq!(phones_dot[2].lyric, "d eh");
        assert_eq!(phones_dot[2].duration_ms, 100.0);
        assert_eq!(phones_dot[3].lyric, "eh l");
        assert_eq!(phones_dot[3].duration_ms, 100.0);
        assert_eq!(phones_dot[4].lyric, "l a");
        assert_eq!(phones_dot[4].duration_ms, 100.0);
    }

    #[test]
    fn test_g2p_vs_phonetic_mode() {
        let entries = std::collections::HashMap::new();
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let notes_word = vec![UNote::new("can", "C4", 0.0, 300.0)];
        let phones_g2p =
            JapanesePhonemizer::apply_phonemizer(&notes_word, &vb, PhonemizerMode::EnglishG2P);
        assert_eq!(phones_g2p.len(), 3);
        assert_eq!(phones_g2p[0].lyric, "k");
        assert_eq!(phones_g2p[1].lyric, "ae");
        assert_eq!(phones_g2p[2].lyric, "n");

        let phones_direct =
            JapanesePhonemizer::apply_phonemizer(&notes_word, &vb, PhonemizerMode::EnglishArpasing);
        assert_eq!(phones_direct.len(), 1);
        assert_eq!(phones_direct[0].lyric, "can");

        let notes_pt = vec![UNote::new("sol", "C4", 0.0, 300.0)];
        let phones_pt_g2p =
            JapanesePhonemizer::apply_phonemizer(&notes_pt, &vb, PhonemizerMode::PortugueseG2P);
        assert_eq!(phones_pt_g2p.len(), 2);
        assert_eq!(phones_pt_g2p[0].lyric, "so");
        assert_eq!(phones_pt_g2p[1].lyric, "w");

        let notes_brapa = vec![UNote::new("canto", "C4", 0.0, 400.0)];
        let phones_brapa = JapanesePhonemizer::apply_phonemizer(
            &notes_brapa,
            &vb,
            PhonemizerMode::PortugueseBrapaVCCV,
        );
        assert!(!phones_brapa.is_empty());
    }

    #[test]
    fn test_vccv_brapa_phonemizer_full_cascade() {
        let mut entries = std::collections::HashMap::new();
        entries.insert(
            "- ka".to_string(),
            crate::oto::OtoEntry::new(
                "ka.wav".to_string(),
                "- ka".to_string(),
                0.0,
                50.0,
                0.0,
                0.0,
                0.0,
            ),
        );
        entries.insert(
            "a s".to_string(),
            crate::oto::OtoEntry::new(
                "as.wav".to_string(),
                "a s".to_string(),
                0.0,
                50.0,
                0.0,
                0.0,
                0.0,
            ),
        );
        entries.insert(
            "s a".to_string(),
            crate::oto::OtoEntry::new(
                "sa.wav".to_string(),
                "s a".to_string(),
                0.0,
                50.0,
                0.0,
                0.0,
                0.0,
            ),
        );

        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "BRAPA VB".to_string(),
            author: "Xiao".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let notes = vec![
            UNote::new("ka", "C4", 0.0, 400.0),
            UNote::new("sa", "C4", 400.0, 400.0),
        ];

        let phones =
            JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::PortugueseBrapaVCCV);
        assert_eq!(phones[0].lyric, "- ka");
        assert_eq!(phones[1].lyric, "a s");
        assert!(
            phones[1].position_ms < 400.0,
            "VC transition 'a s' must be before the note start line (400ms)"
        );
        assert_eq!(phones[2].lyric, "s a");
        assert_eq!(
            phones[2].position_ms, 400.0,
            "CV note 's a' must start exactly at the note start line (400ms)"
        );
    }

    #[test]
    fn test_custom_phoneme_durations_across_all_phonemizers() {
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries: HashMap::new(),
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let mut note = UNote::new("can", "C4", 100.0, 600.0);
        note.phoneme_durations_ms = vec![100.0, 350.0, 150.0];

        let phones = JapanesePhonemizer::apply_phonemizer(&[note], &vb, PhonemizerMode::EnglishG2P);
        assert_eq!(phones.len(), 3);
        assert_eq!(phones[0].duration_ms, 100.0);
        assert_eq!(phones[0].position_ms, 100.0);
        assert_eq!(phones[1].duration_ms, 350.0);
        assert_eq!(phones[1].position_ms, 200.0);
        assert_eq!(phones[2].duration_ms, 150.0);
        assert_eq!(phones[2].position_ms, 550.0);
    }
}
