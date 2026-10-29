//! Brazilian Portuguese CVC topology from OpenUtau's BRAPA phonemizer.

use super::{JapanesePhonemizer, PhonemizerMode, RenderPhone};
use crate::oto::Voicebank;
use crate::project::model::UNote;

fn vowel(token: &str) -> Option<String> {
    let normalized = match token {
        "E" => "eh",
        "O" => "oh",
        "a~" => "an",
        "e~" => "en",
        "i~" => "in",
        "o~" => "on",
        "u~" => "un",
        _ => token,
    };
    normalized
        .chars()
        .next()
        .filter(|c| matches!(c, 'a' | 'e' | 'i' | 'o' | 'u'))
        .map(|_| normalized.to_owned())
}

fn normalize(token: &str) -> String {
    match token {
        "tS" => "ch",
        "dZ" => "dj",
        "J" => "nh",
        "L" => "lh",
        "X" | "R" => "h",
        "Z" => "j",
        "S" => "sh",
        "w~" => "w",
        "j~" => "y",
        _ => token,
    }
    .to_owned()
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

pub fn apply(notes: &[UNote], vb: &Voicebank) -> Vec<RenderPhone> {
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
        let raw_tokens = note
            .phonetic_hint
            .as_deref()
            .map(|hint| {
                hint.split_whitespace()
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .or_else(|| {
                JapanesePhonemizer::openutau_dictionary_tokens(
                    lyric,
                    PhonemizerMode::PortugueseBrazilianCVC,
                )
            })
            .unwrap_or_else(|| super::PortuguesePhonemizer::word_to_phonemes(lyric));
        let symbols = raw_tokens
            .iter()
            .map(|token| normalize(token))
            .collect::<Vec<_>>();
        let Some(vowel_index) = symbols.iter().position(|symbol| vowel(symbol).is_some()) else {
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
        let current_vowel = vowel(&symbols[vowel_index]).unwrap();
        let coda = &symbols[vowel_index + 1..];
        let connected = previous_end.is_some_and(|end| note.position_ms <= end + 0.001);
        let mut aliases = Vec::new();

        if onset.is_empty() {
            if connected {
                aliases.push(
                    previous_vowel
                        .as_deref()
                        .map(|previous| format!("{previous} {current_vowel}"))
                        .unwrap_or_else(|| format!("- {current_vowel}")),
                );
            } else {
                aliases.push(format!("- {current_vowel}"));
            }
        } else if !connected || previous_vowel.is_none() {
            for consonant in &onset[..onset.len().saturating_sub(1)] {
                aliases.push(format!("- {consonant}"));
            }
            aliases.push(format!("-{} {}", onset.last().unwrap(), current_vowel));
        } else {
            if let Some(previous) = previous_vowel.as_deref() {
                aliases.push(format!("{previous} {}", onset[0]));
            }
            for (index, consonant) in onset.iter().enumerate().skip(1) {
                let candidate = format!("{} -", onset[index - 1]);
                aliases.push(if vb.find_mapped_entry(&candidate, &note.pitch).is_some() {
                    candidate
                } else {
                    consonant.to_owned()
                });
            }
            aliases.push(format!("{} {}", onset.last().unwrap(), current_vowel));
        }

        if !coda.is_empty() {
            aliases.push(format!("{} {}-", current_vowel, coda[0]));
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
            aliases.push(format!("{} -", current_vowel));
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
        previous_vowel = Some(current_vowel);
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
    fn brazilian_cvc_emits_cv_vcv_and_phrase_tail() {
        let (_directory, voicebank) = voicebank_with_aliases(&["-b a", "a m", "m a", "a -"]);
        let mut first = UNote::new("ignored", "C4", 0.0, 300.0);
        first.phonetic_hint = Some("b a".to_string());
        let mut second = UNote::new("ignored", "C4", 300.0, 300.0);
        second.phonetic_hint = Some("m a".to_string());

        let phones = apply(&[first, second], &voicebank);
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/openutau_compatibility/pt_br_cvc.json"
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
