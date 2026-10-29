//! Russian syllable-based phonemizer primitives.
//!
//! The alias topology here follows OpenUtau's `RussianCVCPhonemizer`: a
//! phrase head uses `-CV`, a connected syllable emits the previous-vowel
//! transition and a CV main phone, and coda consonants are emitted as `VC-`
//!/`C-` tails.  The G2P table remains the same OpenUtau-derived table used by
//! the dispatcher.

use super::{PhonemizerMode, RenderPhone};
use crate::oto::Voicebank;
use crate::project::model::UNote;

const VOWELS: [&str; 8] = ["a", "e", "o", "u", "y", "i", "M", "N"];

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
        coda_truncation: false,
    }
}

fn tokens(note: &UNote) -> Vec<String> {
    note.phonetic_hint
        .as_deref()
        .map(|hint| hint.split_whitespace().map(str::to_owned).collect())
        .unwrap_or_default()
}

/// Apply the CVC topology from OpenUtau's Russian phonemizer.
pub fn apply(notes: &[UNote], vb: &Voicebank) -> Vec<RenderPhone> {
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

        let mut symbols = tokens(note);
        if symbols.is_empty() {
            symbols = super::JapanesePhonemizer::openutau_dictionary_tokens(
                lyric,
                PhonemizerMode::RussianCVC,
            )
            .unwrap_or_else(|| vec![lyric.to_owned()]);
        }
        let Some(vowel_index) = symbols.iter().position(|token| is_vowel(token)) else {
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
        let mut aliases = Vec::new();

        if onset.is_empty() {
            aliases.push(vowel.clone());
        } else if !connected || previous_vowel.is_none() {
            for consonant in &onset[..onset.len().saturating_sub(1)] {
                aliases.push(format!("-{consonant}"));
            }
            aliases.push(format!("-{}{}", onset.last().unwrap(), vowel));
        } else {
            if let Some(previous) = previous_vowel.as_deref() {
                aliases.push(format!("{previous} {}", onset[0]));
            }
            for (index, consonant) in onset.iter().enumerate().skip(1) {
                let candidate = format!("{}-", onset[index - 1]);
                aliases.push(if vb.find_mapped_entry(&candidate, &note.pitch).is_some() {
                    candidate
                } else {
                    consonant.to_string()
                });
            }
            aliases.push(format!("{}{}", onset.last().unwrap(), vowel));
        }

        if !coda.is_empty() {
            aliases.push(format!("{}{}-", vowel, coda[0]));
            for consonant in coda.iter().skip(1) {
                let candidate = format!("{consonant}-");
                aliases.push(if vb.find_mapped_entry(&candidate, &note.pitch).is_some() {
                    candidate
                } else {
                    consonant.to_string()
                });
            }
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

/// The VCCV sibling uses space-separated phrase and cluster transitions.
pub fn apply_vccv(notes: &[UNote], vb: &Voicebank) -> Vec<RenderPhone> {
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
        let symbols = tokens(note).into_iter().collect::<Vec<_>>();
        let symbols = if symbols.is_empty() {
            super::JapanesePhonemizer::openutau_dictionary_tokens(
                lyric,
                PhonemizerMode::RussianVCCV,
            )
            .unwrap_or_else(|| vec![lyric.to_owned()])
        } else {
            symbols
        };
        let Some(vowel_index) = symbols.iter().position(|token| is_vowel(token)) else {
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
        let mut aliases = Vec::new();
        if onset.is_empty() {
            aliases.push(if connected {
                previous_vowel
                    .as_deref()
                    .map(|previous| format!("{previous} {vowel}"))
                    .unwrap_or_else(|| format!("- {vowel}"))
            } else {
                format!("- {vowel}")
            });
        } else if !connected || previous_vowel.is_none() {
            let base = format!("{}{}", onset.last().unwrap(), vowel);
            if vb
                .find_mapped_entry(&format!("- {base}"), &note.pitch)
                .is_some()
            {
                aliases.push(format!("- {base}"));
            } else {
                if !matches!(
                    onset[0].as_str(),
                    "t" | "t'" | "k" | "k'" | "p" | "p'" | "ch" | "ts"
                ) {
                    aliases.push(format!("- {}", onset[0]));
                }
                aliases.push(base);
            }
        } else {
            if let Some(previous) = previous_vowel.as_deref() {
                aliases.push(format!("{previous} {}", onset[0]));
            }
            for pair in onset.windows(2) {
                let candidate = format!("{} {}", pair[0], pair[1]);
                if vb.find_mapped_entry(&candidate, &note.pitch).is_some() {
                    aliases.push(candidate);
                }
            }
            aliases.push(format!("{}{}", onset.last().unwrap(), vowel));
        }
        if !coda.is_empty() {
            aliases.push(format!("{}{} -", vowel, coda[0]));
            for consonant in coda.iter().skip(1) {
                let candidate = format!("{consonant} -");
                aliases.push(if vb.find_mapped_entry(&candidate, &note.pitch).is_some() {
                    candidate
                } else {
                    consonant.to_owned()
                });
            }
        } else if notes.get(note_index + 1).is_none_or(|next| {
            next.lyric.trim().is_empty()
                || next.lyric.eq_ignore_ascii_case("r")
                || next.position_ms > note.position_ms + note.duration_ms + 0.001
        }) {
            aliases.push(format!("{vowel} -"));
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
    fn russian_cvc_keeps_head_transition_and_coda_topology() {
        let (_directory, voicebank) =
            voicebank_with_aliases(&["-m", "-ma", "a m", "ma", "an-", "n-"]);
        let mut first = UNote::new("ignored", "C4", 0.0, 300.0);
        first.phonetic_hint = Some("m a".to_string());
        let mut second = UNote::new("ignored", "C4", 300.0, 300.0);
        second.phonetic_hint = Some("n a n".to_string());

        let phones = apply(&[first, second], &voicebank);
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/openutau_compatibility/russian_cvc.json"
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
    fn russian_vccv_uses_space_separated_transition_aliases() {
        let (_directory, voicebank) = voicebank_with_aliases(&["- ma", "a n", "na", "a -"]);
        let mut first = UNote::new("ignored", "C4", 0.0, 300.0);
        first.phonetic_hint = Some("m a".to_string());
        let mut second = UNote::new("ignored", "C4", 300.0, 300.0);
        second.phonetic_hint = Some("n a".to_string());

        let phones = super::apply_vccv(&[first, second], &voicebank);
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/openutau_compatibility/russian_vccv.json"
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
}
