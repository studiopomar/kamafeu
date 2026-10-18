use super::super::{consonant_velocity_time_scale, PhonemizerMode, RenderPhone};
use super::EnglishPhonemizer;
use crate::oto::Voicebank;
use crate::project::model::UNote;

pub(crate) fn apply(notes: &[UNote], vb: &Voicebank, mode: PhonemizerMode) -> Vec<RenderPhone> {
    let mut phones: Vec<RenderPhone> = Vec::new();
    let mut prev_vowel: Option<String> = None;
    let mut prev_note_end_ms: Option<f64> = None;

    for (note_index, note) in notes.iter().enumerate() {
        if note.lyric.trim().is_empty() || note.lyric.trim() == "R" {
            prev_vowel = None;
            prev_note_end_ms = None;
            continue;
        }
        let is_phrase_start = match prev_note_end_ms {
            Some(end_ms) => note.position_ms > end_ms + 0.001,
            None => true,
        };

        if is_phrase_start {
            prev_vowel = None;
        }

        let word_phones = if let Some(hint) = note.phonetic_hint.as_deref() {
            hint.split([',', ' '])
                .filter(|token| !token.is_empty())
                .map(|token| {
                    if mode == PhonemizerMode::EnglishXSampa {
                        token.to_string()
                    } else {
                        token.to_lowercase()
                    }
                })
                .collect::<Vec<_>>()
        } else {
            match mode {
            PhonemizerMode::EnglishArpasing | PhonemizerMode::EnglishArpasingPlus => {
                if let Some(tokens) = super::super::g2p::openutau_arpasing_tokens(&note.lyric, vb) {
                    tokens
                } else {
                    // Arpasing is an explicit-phone mode when the singer
                    // does not ship its dictionary. Do not silently turn
                    // an English word into Kamafeu-specific graphemes.
                    crate::terminal_log::debug(
                        "Phonemizer",
                        format!("mode=EnglishArpasing word={:?} fallback=passthrough reason=missing_arpasing_yaml", note.lyric),
                    );
                    EnglishPhonemizer::phonetic_tokens(&note.lyric)
                }
            }
            PhonemizerMode::EnglishG2P => super::super::g2p::openutau_dictionary_tokens(&note.lyric, mode)
                .unwrap_or_else(|| {
                    crate::terminal_log::debug(
                        "Phonemizer",
                        format!("mode=EnglishG2P word={:?} fallback=passthrough reason=missing_dictionary_entry", note.lyric),
                    );
                    EnglishPhonemizer::phonetic_tokens(&note.lyric)
                }),
            _ => EnglishPhonemizer::phonetic_tokens(&note.lyric),
            }
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
                if let Some(entry) = vb.find_mapped_entry(&head_try, &note.pitch) {
                    alias = entry.alias.clone();
                }
            } else if idx == 0 && !is_phrase_start {
                if let Some(ref pv) = prev_vowel {
                    let vc_try = format!("{} {}", pv, p);
                    if let Some(entry) = vb.find_mapped_entry(&vc_try, &note.pitch) {
                        let stretch =
                            consonant_velocity_time_scale(note.expressions.consonant_velocity);
                        let authored = if entry.overlap < 0.0 {
                            entry.preutterance - entry.overlap
                        } else {
                            entry.preutterance
                        } * stretch;
                        let vc_dur = authored.max(5.0).min((note.duration_ms * 0.5).max(5.0));
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
                                    resolved_alias: None,
                                    resolved_timbre: None,
                                });
                            }
                        }
                    }
                }
            } else if let Some(ref pv) = prev_vowel {
                let vc_try = format!("{} {}", pv, p);
                if let Some(entry) = vb.find_mapped_entry(&vc_try, &note.pitch) {
                    alias = entry.alias.clone();
                }
            }

            if EnglishPhonemizer::is_vowel(p)
                || (mode == PhonemizerMode::EnglishXSampa
                    && matches!(
                        p.as_str(),
                        "a" | "A"
                            | "@"
                            | "{"
                            | "V"
                            | "O"
                            | "E"
                            | "3"
                            | "eI"
                            | "I"
                            | "i"
                            | "oU"
                            | "OI"
                            | "U"
                            | "u"
                            | "Q"
                            | "Ol"
                            | "Q@"
                            | "2"
                            | "6"
                            | "7"
                            | "8"
                            | "9"
                            | "&"
                            | "}"
                            | "1"
                            | "e"
                            | "o"
                            | "i:"
                            | "u:"
                            | "O:"
                    ))
            {
                prev_vowel = Some(p.clone());
            } else {
                prev_vowel = None;
            }

            let resolved_entry = if matches!(
                mode,
                PhonemizerMode::EnglishArpasing | PhonemizerMode::EnglishArpasingPlus
            ) {
                std::iter::once(alias.as_str())
                    .chain(
                        super::super::g2p::arpasing_vowel_fallbacks(p)
                            .iter()
                            .copied(),
                    )
                    .find_map(|candidate| vb.find_mapped_entry(candidate, &note.pitch))
            } else {
                vb.find_mapped_entry(&alias, &note.pitch)
            };
            if let Some(entry) = resolved_entry {
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
                resolved_alias: None,
                resolved_timbre: None,
            });
        }

        prev_note_end_ms = Some(note.position_ms + note.duration_ms);
    }

    phones
}

/// OpenUtau's `ArpasingPhonemizer` is a Latin diphone phonemizer: aliases
/// are selected from the previous symbol and the current symbol, with a
/// leading `-` at a phrase boundary.  The old Kamafeu path tracked only
/// the previous vowel, which lost clusters such as `s t ae`.  Keep this
/// implementation separate from the generic G2P path so its alias graph
/// can be compared directly with OpenUtau's `LatinDiphonePhonemizer`.
pub(crate) fn apply_arpasing(notes: &[UNote], vb: &Voicebank) -> Vec<RenderPhone> {
    let mut phones = Vec::new();
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

        let symbols = if let Some(hint) = note.phonetic_hint.as_deref() {
            hint.split_whitespace()
                .filter(|symbol| !symbol.is_empty())
                .map(str::to_lowercase)
                .collect::<Vec<_>>()
        } else {
            super::super::g2p::openutau_arpasing_tokens(lyric, vb).unwrap_or_else(|| {
                crate::terminal_log::debug(
                    "Phonemizer",
                    format!("mode=EnglishArpasing word={lyric:?} fallback=passthrough reason=missing_arpasing_yaml"),
                );
                EnglishPhonemizer::phonetic_tokens(lyric)
            })
        };
        if symbols.is_empty() {
            continue;
        }

        let phrase_end = notes.get(note_index + 1).is_none_or(|next| {
            next.lyric.trim().is_empty()
                || next.lyric.eq_ignore_ascii_case("r")
                || next.position_ms > note.position_ms + note.duration_ms + 0.001
        });
        let symbol_count = (symbols.len() + usize::from(phrase_end)) as f64;
        let duration = note.duration_ms / symbol_count;
        for (symbol_index, symbol) in symbols.iter().enumerate() {
            let previous = if symbol_index == 0 {
                previous_symbol.as_deref().unwrap_or("-")
            } else {
                symbols[symbol_index - 1].as_str()
            };
            let alias = resolve_arpasing_alias(previous, symbol, vb, note);
            phones.push(RenderPhone {
                note_index,
                lyric: alias,
                pitch: note.pitch.clone(),
                position_ms: note.position_ms + symbol_index as f64 * duration,
                duration_ms: duration,
                envelope: note.envelope.clone(),
                expressions: note.expressions.clone(),
                pitch_bend: note.pitch_bend.clone(),
                vibrato: note.vibrato.clone(),
                flags: note.flags.clone(),
                resolved_alias: None,
                resolved_timbre: note.timbre.clone(),
            });
        }

        if phrase_end {
            let previous = symbols.last().map(String::as_str).unwrap_or("-");
            let alias = resolve_arpasing_alias(previous, "-", vb, note);
            phones.push(RenderPhone {
                note_index,
                lyric: alias,
                pitch: note.pitch.clone(),
                position_ms: note.position_ms + symbols.len() as f64 * duration,
                duration_ms: duration,
                envelope: note.envelope.clone(),
                expressions: note.expressions.clone(),
                pitch_bend: note.pitch_bend.clone(),
                vibrato: note.vibrato.clone(),
                flags: note.flags.clone(),
                resolved_alias: None,
                resolved_timbre: note.timbre.clone(),
            });
        }

        previous_symbol = symbols.last().cloned();
    }
    phones
}

fn resolve_arpasing_alias(previous: &str, symbol: &str, vb: &Voicebank, note: &UNote) -> String {
    let candidate = format!("{previous} {symbol}");
    if let Some(entry) = vb.find_mapped_entry(&candidate, &note.pitch) {
        return entry.alias.clone();
    }
    for fallback in super::super::g2p::arpasing_vowel_fallbacks(symbol) {
        let candidate = format!("{previous} {fallback}");
        if let Some(entry) = vb.find_mapped_entry(&candidate, &note.pitch) {
            return entry.alias.clone();
        }
    }
    if let Some(entry) = vb.find_mapped_entry(&format!("- {symbol}"), &note.pitch) {
        return entry.alias.clone();
    }
    format!("{previous} {symbol}")
}
