//! Korean phonemizer primitives.
//!
//! This module deliberately keeps the Hangul decomposition independent from
//! the alias strategy.  The OpenUtau Korean families share the same Unicode
//! syllable decomposition but differ in whether they request CV, VCV, CVVC or
//! batchim transitions.  Keeping that boundary explicit makes the remaining
//! family-specific ports testable instead of hiding them behind a generic G2P.

use super::{PhonemizerMode, RenderPhone};
use crate::oto::Voicebank;
use crate::project::model::UNote;

const INITIALS: [&str; 19] = [
    "g", "gg", "n", "d", "dd", "r", "m", "b", "bb", "s", "ss", "", "j", "jj", "ch", "k", "t", "p",
    "h",
];
const MEDIALS: [&str; 21] = [
    "a", "ae", "ya", "yae", "eo", "e", "yeo", "ye", "o", "wa", "wae", "oe", "yo", "u", "weo", "we",
    "wi", "yu", "eu", "eui", "i",
];
const FINALS: [&str; 28] = [
    "", "k", "k", "k", "n", "n", "n", "t", "l", "l", "l", "l", "l", "l", "l", "l", "m", "p", "p",
    "t", "t", "ng", "t", "t", "k", "t", "p", "t",
];

#[derive(Debug, Clone)]
struct Syllable<'a> {
    initial: &'a str,
    cv: String,
    vowel: &'a str,
    final_sound: &'a str,
}

fn decompose(lyric: &str) -> Option<Syllable<'static>> {
    let code = lyric.trim().chars().next()? as u32;
    if !(0xAC00..=0xD7A3).contains(&code) {
        return None;
    }
    let offset = code - 0xAC00;
    let initial = (offset / 588) as usize;
    let medial = ((offset % 588) / 28) as usize;
    let final_index = (offset % 28) as usize;
    Some(Syllable {
        initial: INITIALS[initial],
        cv: format!("{}{}", INITIALS[initial], MEDIALS[medial]),
        vowel: MEDIALS[medial],
        final_sound: FINALS[final_index],
    })
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

fn resolve(vb: &Voicebank, candidate: &str, note: &UNote) -> String {
    vb.find_mapped_entry(candidate, &note.pitch)
        .map(|entry| entry.alias.clone())
        .unwrap_or_else(|| candidate.to_string())
}

fn resolve_candidates(vb: &Voicebank, candidates: &[String], note: &UNote) -> String {
    candidates
        .iter()
        .find_map(|candidate| {
            vb.find_mapped_entry(candidate, &note.pitch)
                .map(|entry| entry.alias.clone())
        })
        .unwrap_or_else(|| candidates.first().cloned().unwrap_or_default())
}

/// Shared first-pass topology for the native Korean UTAU families.
///
/// The family-specific OpenUtau ports still need their individual candidate
/// ordering and pronunciation rules.  This function is intentionally marked
/// as a porting boundary and is not presented as validated compatibility until
/// its golden corpus is complete.
pub fn apply(notes: &[UNote], vb: &Voicebank, mode: PhonemizerMode) -> Vec<RenderPhone> {
    let mut output: Vec<RenderPhone> = Vec::new();
    let mut previous_vowel: Option<&str> = None;
    let mut previous_end = None;

    for (index, note) in notes.iter().enumerate() {
        let lyric = note.phonetic_hint.as_deref().unwrap_or(note.lyric.trim());
        if lyric.is_empty() || matches!(lyric, "R" | "-" | "H" | "B" | "bre") {
            previous_vowel = None;
            previous_end = None;
            continue;
        }
        if lyric.starts_with('+') {
            if let Some(last) = output.last_mut() {
                last.duration_ms += note.duration_ms;
            }
            previous_end = Some(note.position_ms + note.duration_ms);
            continue;
        }

        let Some(syllable) = decompose(lyric) else {
            let alias = resolve(vb, lyric, note);
            output.push(phone(
                note,
                index,
                alias,
                note.position_ms,
                note.duration_ms,
            ));
            previous_vowel = None;
            previous_end = Some(note.position_ms + note.duration_ms);
            continue;
        };

        let connected = previous_end.is_some_and(|end| note.position_ms <= end + 0.001);
        let cv_candidates = if connected {
            match (mode, previous_vowel) {
                (PhonemizerMode::KoreanVCV | PhonemizerMode::KoreanCVCCV, Some(vowel)) => {
                    vec![format!("{vowel} {}", syllable.cv), syllable.cv.clone()]
                }
                _ => vec![syllable.cv.clone()],
            }
        } else {
            match mode {
                PhonemizerMode::KoreanCV => vec![syllable.cv.clone(), format!("- {}", syllable.cv)],
                _ => vec![format!("- {}", syllable.cv), syllable.cv.clone()],
            }
        };
        let cv = resolve_candidates(vb, &cv_candidates, note);

        let split_transition = matches!(
            mode,
            PhonemizerMode::KoreanCVVC
                | PhonemizerMode::KoreanCVCCV
                | PhonemizerMode::KoreanCVVCStandard
        ) && connected
            && !syllable.initial.is_empty();
        let transition_duration = (note.duration_ms * 0.18).clamp(8.0, 90.0);
        if split_transition {
            if let Some(vowel) = previous_vowel {
                let transition = resolve_candidates(
                    vb,
                    &[
                        format!("{vowel} {}", syllable.initial),
                        format!("{vowel}{}", syllable.initial),
                    ],
                    note,
                );
                output.push(phone(
                    note,
                    index,
                    transition,
                    note.position_ms,
                    transition_duration,
                ));
            }
        }
        let cv_duration = if split_transition {
            (note.duration_ms - transition_duration).max(1.0)
        } else {
            note.duration_ms
        };
        output.push(phone(
            note,
            index,
            cv,
            if split_transition {
                note.position_ms + transition_duration
            } else {
                note.position_ms
            },
            cv_duration,
        ));

        if !syllable.final_sound.is_empty()
            && matches!(
                mode,
                PhonemizerMode::KoreanCVC
                    | PhonemizerMode::KoreanCVCCV
                    | PhonemizerMode::KoreanCBNN
            )
        {
            let final_alias = resolve_candidates(
                vb,
                &[
                    format!("{} {}", syllable.vowel, syllable.final_sound),
                    format!("{}{}", syllable.vowel, syllable.final_sound),
                    syllable.final_sound.to_string(),
                ],
                note,
            );
            output.push(phone(
                note,
                index,
                final_alias,
                note.position_ms + note.duration_ms * 0.72,
                note.duration_ms * 0.28,
            ));
        }

        previous_vowel = Some(syllable.vowel);
        previous_end = Some(note.position_ms + note.duration_ms);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::decompose;

    #[test]
    fn decomposes_hangul_into_openutau_components() {
        let syllable = decompose("한").expect("Hangul syllable");
        assert_eq!(syllable.cv, "ha");
        assert_eq!(syllable.initial, "h");
        assert_eq!(syllable.vowel, "a");
        assert_eq!(syllable.final_sound, "n");
    }
}
