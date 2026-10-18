use crate::oto::Voicebank;
use crate::phonemizer::{consonant_velocity_time_scale, PhonemizerMode, RenderPhone};
use crate::project::model::UNote;

pub struct PortuguesePhonemizer;

mod g2p;

pub fn apply(notes: &[UNote], vb: &Voicebank, mode: PhonemizerMode) -> Vec<RenderPhone> {
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

        let syllables = match mode {
            PhonemizerMode::PortugueseG2P => PortuguesePhonemizer::word_to_phonemes(&note.lyric),
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
                if let Some(entry) = vb.find_mapped_entry(&head_try, &note.pitch) {
                    alias = entry.alias.clone();
                }
            } else if mode == PhonemizerMode::PortugueseVCV {
                if let Some(ref pv) = prev_vowel {
                    let vcv_try = format!("{} {}", pv, syl);
                    if let Some(entry) = vb.find_mapped_entry(&vcv_try, &note.pitch) {
                        alias = entry.alias.clone();
                    }
                }
            } else if mode == PhonemizerMode::PortugueseCVVC {
                if let (Some(ref pv), Some(cc)) = (
                    prev_vowel.as_ref(),
                    PortuguesePhonemizer::extract_consonant(syl),
                ) {
                    let vc_try = format!("{} {}", pv, cc);
                    if let Some(entry) = vb.find_mapped_entry(&vc_try, &note.pitch) {
                        let stretch =
                            consonant_velocity_time_scale(note.expressions.consonant_velocity);
                        let authored = if entry.overlap < 0.0 {
                            entry.preutterance - entry.overlap
                        } else {
                            entry.preutterance
                        } * stretch;
                        let vc_dur = authored.max(5.0).min((sub_dur * 0.5).max(5.0));
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
            }

            if let Some(v) = PortuguesePhonemizer::extract_vowel(syl) {
                prev_vowel = Some(v.to_string());
            }

            if let Some(entry) = vb.find_mapped_entry(&alias, &note.pitch) {
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
