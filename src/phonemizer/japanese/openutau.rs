use super::JapanesePhonemizer;
use crate::oto::Voicebank;
use crate::phonemizer::{consonant_velocity_time_scale, RenderPhone};
use crate::project::model::UNote;

/// Port of OpenUtau's legacy `JapaneseVCVPhonemizer.Process` candidate
/// order.  This intentionally does not use the larger Kamafeu presamp
/// resolver: presamp is a different OpenUtau phonemizer and must not
/// silently change the legacy VCV contract.
pub(crate) fn apply_vcv(notes: &[UNote], vb: &Voicebank) -> Vec<RenderPhone> {
    let mut phones: Vec<RenderPhone> = Vec::new();
    let mut previous_vowel: Option<String> = None;
    let mut previous_end_ms: Option<f64> = None;

    for (note_index, note) in notes.iter().enumerate() {
        let lyric = note
            .phonetic_hint
            .as_deref()
            .unwrap_or(note.lyric.as_str())
            .trim();
        if lyric.is_empty() || lyric == "R" {
            previous_vowel = None;
            previous_end_ms = None;
            continue;
        }
        if lyric.starts_with('+') {
            if let Some(last) = phones.last_mut() {
                last.duration_ms += note.duration_ms;
            }
            previous_end_ms = Some(note.position_ms + note.duration_ms);
            continue;
        }

        let phrase_start = previous_end_ms
            .map(|end| note.position_ms > end + 0.001)
            .unwrap_or(true);
        let mut candidates = vec![format!("- {lyric}"), lyric.to_string()];
        if !phrase_start {
            if let Some(vowel) = previous_vowel.as_deref() {
                candidates = vec![
                    format!("{vowel} {lyric}"),
                    format!("{vowel}_{lyric}"),
                    format!("* {lyric}"),
                    lyric.to_string(),
                    format!("- {lyric}"),
                ];
            }
        }

        let entry = candidates
            .iter()
            .find_map(|candidate| vb.find_mapped_entry(candidate, &note.pitch));
        let alias = entry
            .map(|entry| entry.alias.clone())
            .unwrap_or_else(|| lyric.to_string());
        let current_vowel = JapanesePhonemizer::extract_vowel(lyric).map(str::to_owned);
        phones.push(RenderPhone {
            note_index,
            lyric: alias.clone(),
            pitch: note.pitch.clone(),
            position_ms: note.position_ms,
            duration_ms: note.duration_ms,
            envelope: note.envelope.clone(),
            expressions: note.expressions.clone(),
            pitch_bend: note.pitch_bend.clone(),
            vibrato: note.vibrato.clone(),
            flags: note.flags.clone(),
            resolved_alias: entry.map(|entry| entry.alias.clone()),
            resolved_timbre: note.timbre.clone(),
        });
        previous_vowel = current_vowel;
        previous_end_ms = Some(note.position_ms + note.duration_ms);
    }
    phones
}

/// OpenUtau's basic Japanese CV-VV lookup: explicit phonetic hints are
/// authoritative; consonant notes use CV aliases, while two adjacent
/// vowel notes may use a VV alias.  This is deliberately not VCV: a
/// consonant note must never inherit the preceding vowel here.
pub(crate) fn apply_cv(notes: &[UNote], vb: &Voicebank) -> Vec<RenderPhone> {
    let mut phones: Vec<RenderPhone> = Vec::new();
    let mut previous_vowel: Option<String> = None;
    let mut previous_end_ms: Option<f64> = None;
    for (note_index, note) in notes.iter().enumerate() {
        let lyric = note
            .phonetic_hint
            .as_deref()
            .unwrap_or(note.lyric.as_str())
            .trim();
        if lyric.is_empty() || lyric == "R" {
            previous_vowel = None;
            previous_end_ms = None;
            continue;
        }
        if lyric.starts_with('+') {
            if let Some(last) = phones.last_mut() {
                last.duration_ms += note.duration_ms;
            }
            previous_end_ms = Some(note.position_ms + note.duration_ms);
            continue;
        }

        let phrase_start = previous_end_ms
            .map(|end| note.position_ms > end + 0.001)
            .unwrap_or(true);
        // `extract_vowel("か")` is `a`, but `か` is still a CV syllable.
        // VV is valid only for an isolated vowel lyric, never for a CV
        // note whose vowel is merely its nucleus.
        let current_vowel = JapanesePhonemizer::extract_vowel(lyric)
            .filter(|_| JapanesePhonemizer::extract_consonant(lyric).is_none())
            .map(str::to_owned);
        let candidates = if note.phonetic_hint.is_some() {
            vec![lyric.to_string()]
        } else if !phrase_start {
            if let (Some(previous_vowel), Some(_current_vowel)) =
                (previous_vowel.as_deref(), current_vowel.as_deref())
            {
                // Japanese CV-VV banks conventionally spell the vowel
                // handoff as `previous_vowel current_vowel`.  Keep the
                // plain CV fallback for banks without that recording.
                vec![
                    format!("{previous_vowel} {lyric}"),
                    format!("{previous_vowel}_{lyric}"),
                    lyric.to_string(),
                ]
            } else {
                vec![format!("- {lyric}"), lyric.to_string()]
            }
        } else {
            vec![format!("- {lyric}"), lyric.to_string()]
        };
        let entry = candidates
            .iter()
            .find_map(|candidate| vb.find_mapped_entry(candidate, &note.pitch));
        let alias = entry
            .map(|entry| entry.alias.clone())
            .unwrap_or_else(|| lyric.to_string());
        phones.push(RenderPhone {
            note_index,
            lyric: alias.clone(),
            pitch: note.pitch.clone(),
            position_ms: note.position_ms,
            duration_ms: note.duration_ms,
            envelope: note.envelope.clone(),
            expressions: note.expressions.clone(),
            pitch_bend: note.pitch_bend.clone(),
            vibrato: note.vibrato.clone(),
            flags: note.flags.clone(),
            resolved_alias: entry.map(|entry| entry.alias.clone()),
            resolved_timbre: note.timbre.clone(),
        });
        previous_vowel = current_vowel;
        previous_end_ms = Some(note.position_ms + note.duration_ms);
    }
    phones
}

/// Port of the legacy OpenUtau Japanese CVVC contract. The main CV is
/// kept on the current note and the optional V-C fragment is placed at
/// the end of that same note, using the next note's oto timing.
pub(crate) fn apply_cvvc(notes: &[UNote], vb: &Voicebank) -> Vec<RenderPhone> {
    fn make_phone(
        note_index: usize,
        note: &UNote,
        alias: String,
        position_ms: f64,
        duration_ms: f64,
    ) -> RenderPhone {
        RenderPhone {
            note_index,
            lyric: alias.clone(),
            pitch: note.pitch.clone(),
            position_ms,
            duration_ms,
            envelope: note.envelope.clone(),
            expressions: note.expressions.clone(),
            pitch_bend: note.pitch_bend.clone(),
            vibrato: note.vibrato.clone(),
            flags: note.flags.clone(),
            resolved_alias: Some(alias),
            resolved_timbre: note.timbre.clone(),
        }
    }

    let mut phones: Vec<RenderPhone> = Vec::new();
    let mut previous_end_ms: Option<f64> = None;

    for (note_index, note) in notes.iter().enumerate() {
        let lyric = note
            .phonetic_hint
            .as_deref()
            .unwrap_or(note.lyric.as_str())
            .trim();
        if lyric.is_empty() || lyric == "R" {
            previous_end_ms = None;
            continue;
        }
        if lyric.starts_with('+') {
            if let Some(last) = phones.last_mut() {
                last.duration_ms += note.duration_ms;
            }
            previous_end_ms = Some(note.position_ms + note.duration_ms);
            continue;
        }

        let phrase_start = previous_end_ms
            .map(|end| note.position_ms > end + 0.001)
            .unwrap_or(true);
        let mut candidates = if note.phonetic_hint.is_some() {
            vec![lyric.to_string()]
        } else if phrase_start {
            vec![format!("- {lyric}"), lyric.to_string()]
        } else {
            // CVVC has a separate outgoing VC phone.  The main phone is
            // therefore always the current CV, never a VCV alias that
            // would repeat the same consonant transition.
            vec![format!("* {lyric}"), lyric.to_string()]
        };
        let main_entry = candidates
            .drain(..)
            .find_map(|candidate| vb.find_mapped_entry(&candidate, &note.pitch));
        let main_alias = main_entry
            .map(|entry| entry.alias.clone())
            .unwrap_or_else(|| lyric.to_string());

        let next = notes.get(note_index + 1).filter(|next| {
            let next_lyric = next
                .phonetic_hint
                .as_deref()
                .unwrap_or(next.lyric.as_str())
                .trim();
            !next_lyric.is_empty()
                && next_lyric != "R"
                && !next_lyric.starts_with('+')
                && (next.position_ms - (note.position_ms + note.duration_ms)).abs() <= 0.001
        });

        let mut vc: Option<(String, f64)> = None;
        if let Some(next) = next {
            let next_lyric = next
                .phonetic_hint
                .as_deref()
                .unwrap_or(next.lyric.as_str())
                .trim();
            // A CV lyric such as `さ` has vowel nucleus `a`, but it is
            // still a consonant onset and therefore needs the outgoing
            // VC fragment from the preceding note.
            let next_is_plain_vowel = JapanesePhonemizer::extract_consonant(next_lyric).is_none();
            if !next_is_plain_vowel {
                let vowel = JapanesePhonemizer::extract_vowel(lyric).unwrap_or("");
                let consonant = JapanesePhonemizer::extract_consonant(next_lyric).unwrap_or("");
                if !vowel.is_empty() && !consonant.is_empty() {
                    let substitute = match consonant {
                        "ch" | "ty" | "ts" => Some("t"),
                        "j" | "dy" => Some("d"),
                        "gy" => Some("g"),
                        "ky" => Some("k"),
                        "py" => Some("p"),
                        "ny" => Some("n"),
                        "ry" => Some("r"),
                        "my" => Some("m"),
                        "hy" | "f" => Some("h"),
                        "by" | "v" => Some("b"),
                        "dz" => Some("z"),
                        "l" => Some("r"),
                        "ly" => Some("l"),
                        _ => None,
                    };
                    let mut vc_candidates = vec![format!("{vowel} {consonant}")];
                    if let Some(substitute) = substitute {
                        vc_candidates.push(format!("{vowel} {substitute}"));
                    }
                    if let Some(entry) = vc_candidates
                        .iter()
                        .find_map(|candidate| vb.find_mapped_entry(candidate, &note.pitch))
                    {
                        let next_entry = vb.find_mapped_entry(next_lyric, &next.pitch);
                        let authored_ms = next_entry
                            .map(|entry| {
                                if entry.overlap < 0.0 {
                                    entry.preutterance - entry.overlap
                                } else {
                                    entry.preutterance
                                }
                            })
                            .unwrap_or(120.0);
                        let scaled = authored_ms
                            * consonant_velocity_time_scale(next.expressions.consonant_velocity);
                        let vc_len = scaled.max(0.0).min((note.duration_ms * 0.5).max(0.0));
                        vc = Some((entry.alias.clone(), vc_len));
                    }
                }
            }
        }

        let main_duration = vc
            .as_ref()
            .map(|(_, length)| (note.duration_ms - *length).max(0.0))
            .unwrap_or(note.duration_ms);
        phones.push(make_phone(
            note_index,
            note,
            main_alias,
            note.position_ms,
            main_duration,
        ));
        if let Some((vc_alias, vc_len)) = vc {
            phones.push(make_phone(
                note_index,
                note,
                vc_alias,
                note.position_ms + note.duration_ms - vc_len,
                vc_len,
            ));
        }
        previous_end_ms = Some(note.position_ms + note.duration_ms);
    }
    phones
}
