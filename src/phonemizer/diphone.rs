//! Shared Latin diphone topology used by OpenUtau's dictionary phonemizers.

use super::{JapanesePhonemizer, PhonemizerMode, RenderPhone};
use crate::oto::Voicebank;
use crate::project::model::UNote;

fn resolve(vb: &Voicebank, previous: &str, symbol: &str, note: &UNote) -> String {
    let candidate = format!("{previous} {symbol}");
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

/// Port of `PhonemeBasedPhonemizer` + `LatinDiphonePhonemizer` without a
/// language-specific vowel fallback table. French CMUSphinx uses exactly this
/// base class in OpenUtau.
pub fn apply(notes: &[UNote], vb: &Voicebank, mode: PhonemizerMode) -> Vec<RenderPhone> {
    let mut output = Vec::new();
    let mut previous_symbol: Option<String> = None;

    for (note_index, note) in notes.iter().enumerate() {
        let lyric = note.lyric.trim();
        if lyric.is_empty() || lyric.eq_ignore_ascii_case("r") {
            previous_symbol = None;
            continue;
        }
        let phrase_start = note_index == 0
            || notes
                .get(note_index.saturating_sub(1))
                .is_none_or(|previous| {
                    previous.position_ms + previous.duration_ms < note.position_ms - 0.001
                });
        if phrase_start {
            previous_symbol = None;
        }
        let symbols = note
            .phonetic_hint
            .as_deref()
            .map(|hint| {
                hint.split_whitespace()
                    .map(str::to_owned)
                    .collect::<Vec<_>>()
            })
            .or_else(|| JapanesePhonemizer::openutau_dictionary_tokens(lyric, mode))
            .unwrap_or_else(|| vec![lyric.to_owned()]);
        if symbols.is_empty() {
            continue;
        }

        let phrase_end = notes.get(note_index + 1).is_none_or(|next| {
            next.lyric.trim().is_empty()
                || next.lyric.eq_ignore_ascii_case("r")
                || next.position_ms > note.position_ms + note.duration_ms + 0.001
        });
        let symbol_count = symbols.len() + usize::from(phrase_end);
        let duration = note.duration_ms / symbol_count as f64;
        for (index, symbol) in symbols.iter().enumerate() {
            let previous = if index == 0 {
                previous_symbol.as_deref().unwrap_or("-")
            } else {
                symbols[index - 1].as_str()
            };
            let alias = resolve(vb, previous, symbol, note);
            output.push(phone(
                note,
                note_index,
                alias,
                note.position_ms + index as f64 * duration,
                duration,
            ));
        }
        if phrase_end {
            let previous = symbols.last().map(String::as_str).unwrap_or("-");
            output.push(phone(
                note,
                note_index,
                resolve(vb, previous, "-", note),
                note.position_ms + symbols.len() as f64 * duration,
                duration,
            ));
        }
        previous_symbol = symbols.last().cloned();
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
    fn french_sphinx_uses_latin_diphone_aliases_and_tail() {
        let (_directory, voicebank) = voicebank_with_aliases(&["- s", "s a", "a n", "n -"]);
        let mut first = UNote::new("ignored", "C4", 0.0, 300.0);
        first.phonetic_hint = Some("s a n".to_string());
        let phones = apply(&[first], &voicebank, PhonemizerMode::FrenchCMUSphinx);
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/openutau_compatibility/french_sphinx.json"
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
