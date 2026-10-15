//! Spanish syllable-based alias topology.
//!
//! This is the common, reviewable part shared by OpenUtau's Spanish
//! syllable-based families.  Makkusan keeps its own no-space alias convention
//! and is intentionally not routed through this module.

use super::{PhonemizerMode, RenderPhone};
use crate::oto::Voicebank;
use crate::project::model::UNote;

const VOWELS: [&str; 5] = ["a", "e", "i", "o", "u"];

fn is_vowel(token: &str) -> bool {
    VOWELS.contains(&token)
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

pub fn apply(notes: &[UNote], vb: &Voicebank, mode: PhonemizerMode) -> Vec<RenderPhone> {
    let mut output = Vec::new();
    let mut previous_vowel: Option<String> = None;
    let mut previous_end: Option<f64> = None;

    for (note_index, note) in notes.iter().enumerate() {
        let lyric = note.lyric.trim();
        if lyric.is_empty() || lyric.eq_ignore_ascii_case("r") {
            previous_vowel = None;
            previous_end = None;
            continue;
        }

        let symbols = note
            .phonetic_hint
            .as_deref()
            .map(|hint| {
                hint.split_whitespace()
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .or_else(|| super::JapanesePhonemizer::openutau_dictionary_tokens(lyric, mode))
            .unwrap_or_else(|| vec![lyric.to_owned()]);
        let Some(vowel_index) = symbols.iter().position(|symbol| is_vowel(symbol)) else {
            output.push(phone(
                note,
                note_index,
                resolve(vb, lyric.to_owned(), note),
                note.position_ms,
                note.duration_ms,
            ));
            previous_vowel = None;
            previous_end = Some(note.position_ms + note.duration_ms);
            continue;
        };

        let onset = &symbols[..vowel_index];
        let vowel = symbols[vowel_index].clone();
        let coda = &symbols[vowel_index + 1..];
        let connected = previous_end.is_some_and(|end| note.position_ms <= end + 0.001);
        let no_space = mode == PhonemizerMode::SpanishMakkusan;
        let mut aliases = Vec::new();

        if onset.is_empty() {
            let candidate = if connected {
                previous_vowel
                    .as_deref()
                    .map(|previous| format!("{previous} {vowel}"))
                    .unwrap_or_else(|| format!("- {vowel}"))
            } else {
                format!("- {vowel}")
            };
            aliases.push(candidate);
        } else {
            let consonants = onset.join("");
            let cv = if no_space {
                format!("{consonants}{vowel}")
            } else {
                format!("{consonants}{vowel}")
            };
            let start_cv = if no_space {
                format!("-{cv}")
            } else {
                format!("- {cv}")
            };

            if connected {
                if let Some(previous) = previous_vowel.as_deref() {
                    aliases.push(format!("{previous} {cv}"));
                    if onset.len() > 1 {
                        aliases.push(format!("{previous} {}", onset[0]));
                    }
                } else {
                    aliases.push(start_cv);
                }
            } else {
                aliases.push(start_cv);
            }
        }

        if !coda.is_empty() {
            let coda_alias = if no_space {
                format!("{}{}-", vowel, coda[0])
            } else {
                format!("{} {}", vowel, coda[0])
            };
            aliases.push(coda_alias);
        }

        let duration = note.duration_ms / aliases.len().max(1) as f64;
        for (index, alias) in aliases.into_iter().enumerate() {
            output.push(phone(
                note,
                note_index,
                resolve(vb, alias, note),
                note.position_ms + index as f64 * duration,
                duration,
            ));
        }

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
    fn spanish_syllable_uses_phrase_head_and_vcv_aliases() {
        let (_directory, voicebank) = voicebank_with_aliases(&["- ba", "a ma", "ma", "a m"]);
        let mut first = UNote::new("ignored", "C4", 0.0, 300.0);
        first.phonetic_hint = Some("b a".to_string());
        let mut second = UNote::new("ignored", "C4", 300.0, 300.0);
        second.phonetic_hint = Some("m a".to_string());

        let phones = apply(
            &[first, second],
            &voicebank,
            PhonemizerMode::SpanishSyllable,
        );
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/openutau_compatibility/spanish_syllable.json"
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
    fn spanish_makkusan_keeps_the_no_space_phrase_head_alias() {
        let (_directory, voicebank) = voicebank_with_aliases(&["-ba", "a ma"]);
        let mut first = UNote::new("ignored", "C4", 0.0, 300.0);
        first.phonetic_hint = Some("b a".to_string());
        let mut second = UNote::new("ignored", "C4", 300.0, 300.0);
        second.phonetic_hint = Some("m a".to_string());

        let phones = apply(
            &[first, second],
            &voicebank,
            PhonemizerMode::SpanishMakkusan,
        );
        assert_eq!(
            phones
                .iter()
                .map(|phone| phone.lyric.as_str())
                .collect::<Vec<_>>(),
            ["-ba", "a ma"]
        );
    }
}
