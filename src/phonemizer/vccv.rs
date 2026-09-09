//! Case-sensitive English VCCV phonetic input. Alias alternatives are explicit;
//! consonants occupy OTO-sized transitions, while the nucleus owns the sustain.
use super::{consonant_velocity_time_scale, RenderPhone};
use crate::{oto::Voicebank, project::model::UNote};

const VOWELS: &[&str] = &[
    "a", "@", "u", "0", "8", "I", "e", "3", "A", "i", "E", "O", "Q", "6", "o", "1ng", "9", "&",
    "x", "1", "Y", "L", "W", "8n", "Ang", "9l",
];
const CONSONANTS: &[&str] = &[
    "b", "ch", "d", "dh", "f", "g", "h", "j", "k", "l", "m", "n", "ng", "p", "r", "s", "sh", "t",
    "th", "v", "w", "y", "z", "zh", "dd", "hh", "sp", "st",
];

fn select(vb: &Voicebank, pitch: &str, candidates: Vec<String>) -> String {
    candidates
        .iter()
        .find_map(|a| vb.find_mapped_entry(a, pitch).map(|e| e.alias.clone()))
        .unwrap_or_else(|| candidates[0].clone())
}

fn tokens(text: &str) -> Vec<String> {
    let mut result = Vec::new();
    for word in text.split_whitespace() {
        let mut remaining = word;
        while !remaining.is_empty() {
            let symbol = VOWELS
                .iter()
                .chain(CONSONANTS)
                .copied()
                .filter(|s| remaining.starts_with(s))
                .max_by_key(|s| s.len());
            if let Some(symbol) = symbol {
                result.push(symbol.to_string());
                remaining = &remaining[symbol.len()..];
            } else {
                // Keep unknown input intact so a missing-alias error can explain it.
                result.push(remaining.to_string());
                break;
            }
        }
    }
    result
}

fn phone(note: &UNote, index: usize, alias: String, start: f64, duration: f64) -> RenderPhone {
    phone_with_pitch(note, index, alias, start, duration, &note.pitch)
}

fn phone_with_pitch(
    note: &UNote,
    index: usize,
    alias: String,
    start: f64,
    duration: f64,
    pitch: &str,
) -> RenderPhone {
    RenderPhone {
        note_index: index,
        lyric: alias,
        pitch: pitch.to_string(),
        position_ms: start,
        duration_ms: duration,
        envelope: note.envelope.clone(),
        expressions: note.expressions.clone(),
        pitch_bend: note.pitch_bend.clone(),
        vibrato: note.vibrato.clone(),
        flags: note.flags.clone(),
    }
}

fn length(vb: &Voicebank, alias: &str, note: &UNote) -> f64 {
    vb.find_mapped_entry(alias, &note.pitch)
        .map(|oto| (oto.preutterance - oto.overlap.min(0.0)).max(5.0))
        .unwrap_or(60.0)
        * consonant_velocity_time_scale(note.expressions.consonant_velocity)
}

pub fn apply(notes: &[UNote], vb: &Voicebank) -> Vec<RenderPhone> {
    let mut out: Vec<RenderPhone> = Vec::new();
    let mut previous_vowel: Option<String> = None;
    let mut previous_pitch: Option<String> = None;
    let mut previous_end = f64::NEG_INFINITY;
    for (index, note) in notes.iter().enumerate() {
        let text = note.lyric.trim();
        if text.is_empty() || text == "R" || text == "+" {
            previous_vowel = None;
            previous_pitch = None;
            previous_end = f64::NEG_INFINITY;
            continue;
        }
        if note.position_ms - previous_end > 0.001 {
            previous_vowel = None;
            previous_pitch = None;
        }
        // Explicit aliases with boundary markers and user-authored diphones
        // are already phonemized. Never split '-' or lowercase them.
        if (text.contains('-') || text.contains(' '))
            && vb.find_mapped_entry(text, &note.pitch).is_some()
        {
            out.push(phone(
                note,
                index,
                select(vb, &note.pitch, vec![text.into()]),
                note.position_ms,
                note.duration_ms,
            ));
            previous_vowel = tokens(text)
                .into_iter()
                .rev()
                .find(|s| VOWELS.contains(&s.as_str()));
            previous_end = note.position_ms + note.duration_ms;
            continue;
        }
        let symbols = tokens(text);
        let nuclei: Vec<_> = symbols
            .iter()
            .enumerate()
            .filter(|(_, s)| VOWELS.contains(&s.as_str()))
            .map(|(i, _)| i)
            .collect();
        if nuclei.is_empty() {
            out.push(phone(
                note,
                index,
                select(vb, &note.pitch, vec![text.into()]),
                note.position_ms,
                note.duration_ms,
            ));
            previous_vowel = None;
            previous_end = note.position_ms + note.duration_ms;
            continue;
        }
        let syllable_duration = note.duration_ms / nuclei.len() as f64;
        let mut consonant_start = 0;
        for (syllable, &nucleus) in nuclei.iter().enumerate() {
            let vowel = &symbols[nucleus];
            let cc = &symbols[consonant_start..nucleus];
            let start = note.position_ms + syllable as f64 * syllable_duration;
            let mut transitions = Vec::new();
            let base = if cc.is_empty() {
                match &previous_vowel {
                    Some(prev) => select(
                        vb,
                        &note.pitch,
                        vec![
                            format!("{prev}{vowel}"),
                            format!("{prev} {vowel}"),
                            vowel.clone(),
                        ],
                    ),
                    None => select(
                        vb,
                        &note.pitch,
                        vec![format!("-{vowel}"), format!("- {vowel}"), vowel.clone()],
                    ),
                }
            } else {
                let cluster = cc.join("");
                let whole = if previous_vowel.is_none() {
                    format!("-{cluster}{vowel}")
                } else {
                    format!("{cluster}{vowel}")
                };
                let trans_pitch = previous_pitch.as_deref().unwrap_or(&note.pitch);
                if vb.find_mapped_entry(&whole, &note.pitch).is_some() {
                    if let Some(prev) = &previous_vowel {
                        transitions.push(select(
                            vb,
                            trans_pitch,
                            vec![format!("{prev} {}", cc[0]), format!("{prev}{}", cc[0])],
                        ));
                    }
                    whole
                } else {
                    if let Some(prev) = &previous_vowel {
                        transitions.push(select(
                            vb,
                            trans_pitch,
                            vec![format!("{prev} {}", cc[0]), format!("{prev}{}", cc[0])],
                        ));
                    }
                    for pair in cc.windows(2) {
                        transitions.push(select(
                            vb,
                            &note.pitch,
                            vec![
                                format!("{} {}", pair[0], pair[1]),
                                format!("{}{}", pair[0], pair[1]),
                                pair[0].clone(),
                            ],
                        ));
                    }
                    let c = cc.last().unwrap();
                    let mut choices = Vec::new();
                    if previous_vowel.is_none() {
                        choices.push(format!("-{c}{vowel}"));
                    }
                    choices.extend([format!("{c}{vowel}"), format!("{c} {vowel}")]);
                    select(vb, &note.pitch, choices)
                }
            };
            let mut lengths: Vec<_> = transitions.iter().map(|a| length(vb, a, note)).collect();
            let total: f64 = lengths.iter().sum();
            let available = out
                .last()
                .filter(|p| (p.position_ms + p.duration_ms - start).abs() <= 0.001)
                .map(|p| (p.duration_ms * 0.5).max(0.0))
                .unwrap_or(syllable_duration * 0.5);
            let budget = total.min(available).min(syllable_duration * 0.5);
            if total > 0.0 {
                for value in &mut lengths {
                    *value *= budget / total;
                }
            }
            if let Some(last) = out
                .last_mut()
                .filter(|p| (p.position_ms + p.duration_ms - start).abs() <= 0.001)
            {
                last.duration_ms -= budget;
            }
            let mut cursor = start - budget;
            let trans_pitch = previous_pitch.as_deref().unwrap_or(&note.pitch);
            for (alias, duration) in transitions.into_iter().zip(lengths) {
                if duration > 0.0 {
                    out.push(phone_with_pitch(
                        note,
                        index,
                        alias,
                        cursor,
                        duration,
                        trans_pitch,
                    ));
                    cursor += duration;
                }
            }
            out.push(phone(note, index, base, start, syllable_duration));
            previous_vowel = Some(vowel.clone());
            consonant_start = nucleus + 1;
        }
        let coda = &symbols[consonant_start..];
        let at_phrase_end = notes.get(index + 1).is_none_or(|next| {
            next.position_ms - (note.position_ms + note.duration_ms) > 0.001
                || next.lyric.trim() == "R"
        });
        let mut endings = Vec::new();
        if !coda.is_empty() {
            let mut prev = previous_vowel.clone().unwrap();
            for c in coda {
                endings.push(select(
                    vb,
                    &note.pitch,
                    vec![format!("{prev} {c}"), format!("{prev}{c}"), c.clone()],
                ));
                prev = c.clone();
            }
            if at_phrase_end {
                let c = coda.last().unwrap();
                if let Some(oto) = vb.find_mapped_entry(&format!("{c}-"), &note.pitch) {
                    endings.push(oto.alias.clone());
                }
            }
            previous_vowel = None;
        } else if at_phrase_end {
            if let Some(v) = &previous_vowel {
                if let Some(oto) = vb.find_mapped_entry(&format!("{v}-"), &note.pitch) {
                    endings.push(oto.alias.clone());
                }
            }
        }
        if !endings.is_empty() {
            let lengths: Vec<_> = endings.iter().map(|a| length(vb, a, note)).collect();
            let total: f64 = lengths.iter().sum();
            let last = out.last_mut().unwrap();
            let budget = total.min(last.duration_ms * 0.5);
            last.duration_ms -= budget;
            let mut cursor = note.position_ms + note.duration_ms - budget;
            for (alias, duration) in endings.into_iter().zip(lengths) {
                let duration = duration * budget / total;
                out.push(phone(note, index, alias, cursor, duration));
                cursor += duration;
            }
        }
        previous_end = note.position_ms + note.duration_ms;
        previous_pitch = Some(note.pitch.clone());
    }
    out
}
