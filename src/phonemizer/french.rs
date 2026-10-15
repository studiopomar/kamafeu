//! French syllable-based alias topology.
//!
//! The OpenUtau French CVVC/VCCV plug-ins consume the same `cmudict_fr`
//! syllables but differ in how the consonant transition is serialized. This
//! module keeps that distinction explicit instead of routing both modes
//! through the generic rule-G2P fallback.

use super::{JapanesePhonemizer, PhonemizerMode, RenderPhone};
use crate::oto::Voicebank;
use crate::project::model::UNote;

fn normalize(symbol: &str) -> String {
    match symbol.trim() {
        "aa" => "A",
        "ai" => "E",
        "ei" | "ee" => "e",
        "eu" => "2",
        "oe" => "9",
        "ii" => "i",
        "au" => "o",
        "oo" => "O",
        "ou" => "u",
        "uu" => "y",
        "an" => "a",
        "in" | "un" => "U",
        "on" => "0",
        "uy" => "H",
        "bb" => "b",
        "dd" => "d",
        "ff" => "f",
        "gg" => "g",
        "jj" => "Z",
        "kk" => "k",
        "ll" => "l",
        "mm" => "m",
        "nn" => "n",
        "pp" => "p",
        "rr" => "R",
        "ss" => "s",
        "ch" => "S",
        "tt" => "t",
        "vv" => "v",
        "ww" => "w",
        "yy" => "j",
        "zz" => "z",
        "gn" => "J",
        other => other,
    }
    .to_string()
}

fn is_vowel(symbol: &str) -> bool {
    symbol.chars().next().is_some_and(|c| {
        matches!(
            c,
            'A' | 'E' | 'e' | '2' | '9' | 'i' | 'o' | 'O' | 'u' | 'y' | 'a' | 'U' | '0'
        )
    })
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
    }
}

fn tokens(note: &UNote, mode: PhonemizerMode) -> Vec<String> {
    note.phonetic_hint
        .as_deref()
        .map(|hint| hint.split_whitespace().map(normalize).collect())
        .or_else(|| {
            JapanesePhonemizer::openutau_dictionary_tokens(note.lyric.trim(), mode)
                .map(|items| items.into_iter().map(|item| normalize(&item)).collect())
        })
        .unwrap_or_default()
}

fn push_candidates(
    output: &mut Vec<RenderPhone>,
    note: &UNote,
    note_index: usize,
    candidates: Vec<String>,
) {
    if candidates.is_empty() {
        return;
    }
    let duration = note.duration_ms / candidates.len() as f64;
    for (index, alias) in candidates.into_iter().enumerate() {
        output.push(phone(
            note,
            note_index,
            alias,
            note.position_ms + index as f64 * duration,
            duration,
        ));
    }
}

pub fn apply(notes: &[UNote], vb: &Voicebank, mode: PhonemizerMode) -> Vec<RenderPhone> {
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
        let symbols = tokens(note, mode);
        let Some(vowel_index) = symbols.iter().position(|symbol| is_vowel(symbol)) else {
            push_candidates(
                &mut output,
                note,
                note_index,
                symbols
                    .into_iter()
                    .map(|symbol| resolve(vb, symbol, note))
                    .collect(),
            );
            previous_end = Some(note.position_ms + note.duration_ms);
            continue;
        };
        let onset = &symbols[..vowel_index];
        let vowel = symbols[vowel_index].clone();
        let coda = &symbols[vowel_index + 1..];
        let connected = previous_end.is_some_and(|end| note.position_ms <= end + 0.001);
        let phrase_end = notes.get(note_index + 1).is_none_or(|next| {
            next.lyric.trim().is_empty()
                || next.lyric.eq_ignore_ascii_case("r")
                || next.position_ms > note.position_ms + note.duration_ms + 0.001
        });
        let mut candidates = Vec::new();

        match mode {
            PhonemizerMode::FrenchVCCV => {
                if onset.is_empty() {
                    candidates.push(if connected {
                        format!("{} {}", previous_vowel.as_deref().unwrap_or("-"), vowel)
                    } else {
                        format!("- {vowel}")
                    });
                } else {
                    if connected {
                        if let Some(previous) = previous_vowel.as_deref() {
                            candidates.push(format!("{previous} {}", onset[0]));
                        }
                    } else {
                        candidates.push(format!("- {}{}", onset.join(""), vowel));
                    }
                    if connected {
                        if onset.len() > 1 {
                            for pair in onset.windows(2) {
                                candidates.push(format!("{} {}", pair[0], pair[1]));
                            }
                        }
                        candidates.push(format!("{}{}", onset.last().unwrap(), vowel));
                    }
                }
                if phrase_end {
                    if coda.is_empty() {
                        candidates.push(format!("{vowel} -"));
                    } else {
                        candidates.push(format!("{vowel} {}", coda[0]));
                        candidates.push(format!("{} -", coda.last().unwrap()));
                    }
                }
            }
            PhonemizerMode::FrenchCVVC => {
                if onset.is_empty() {
                    candidates.push(if connected {
                        format!("{} {}", previous_vowel.as_deref().unwrap_or("-"), vowel)
                    } else {
                        vowel.clone()
                    });
                } else {
                    if connected {
                        if let Some(previous) = previous_vowel.as_deref() {
                            candidates.push(format!("{previous} {}", onset[0]));
                        }
                    }
                    candidates.push(format!("{}{}", onset.last().unwrap(), vowel));
                }
                if phrase_end {
                    if coda.is_empty() {
                        candidates.push(format!("{vowel} -"));
                    } else {
                        candidates.push(format!("{vowel}{}", coda[0]));
                        candidates.push(format!("{} -", coda.last().unwrap()));
                    }
                }
            }
            _ => unreachable!("French module only handles French CVVC/VCCV"),
        }

        let resolved = candidates
            .into_iter()
            .map(|candidate| resolve(vb, candidate, note))
            .collect();
        push_candidates(&mut output, note, note_index, resolved);
        previous_vowel = Some(vowel);
        previous_end = Some(note.position_ms + note.duration_ms);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::apply;
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
    fn french_vccv_emits_upstream_phrase_transition_and_tail() {
        let (_directory, voicebank) = voicebank_with_aliases(&["- ba", "a m", "ma", "a -"]);
        let mut first = UNote::new("ignored", "C4", 0.0, 300.0);
        first.phonetic_hint = Some("b a".to_string());
        let mut second = UNote::new("ignored", "C4", 300.0, 300.0);
        second.phonetic_hint = Some("m a".to_string());
        let phones = apply(&[first, second], &voicebank, PhonemizerMode::FrenchVCCV);
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/openutau_compatibility/french_vccv.json"
        ))
        .unwrap();
        assert_eq!(fixture["upstream"]["class"], "FrenchVCCVPhonemizer");
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
    fn french_cvvc_emits_vc_then_cv_and_tail() {
        let (_directory, voicebank) = voicebank_with_aliases(&["ba", "a m", "ma", "a -"]);
        let mut first = UNote::new("ignored", "C4", 0.0, 300.0);
        first.phonetic_hint = Some("b a".to_string());
        let mut second = UNote::new("ignored", "C4", 300.0, 300.0);
        second.phonetic_hint = Some("m a".to_string());
        let phones = apply(&[first, second], &voicebank, PhonemizerMode::FrenchCVVC);
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/openutau_compatibility/french_cvvc.json"
        ))
        .unwrap();
        assert_eq!(fixture["upstream"]["class"], "FrenchCVVCPhonemizer");
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
