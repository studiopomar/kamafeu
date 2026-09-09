use super::*;

pub(super) fn draw(
    notes: &[UNote],
    state: &mut PianoRollState,
    voicebank: Option<&Voicebank>,
    phonemizer_mode: crate::phonemizer::PhonemizerMode,
) {
    {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        notes.len().hash(&mut hasher);
        for n in notes.iter() {
            n.lyric.hash(&mut hasher);
            n.pitch.hash(&mut hasher);
            n.position_ms.to_bits().hash(&mut hasher);
            n.duration_ms.to_bits().hash(&mut hasher);
            n.phoneme_durations_ms.len().hash(&mut hasher);
            for d in &n.phoneme_durations_ms {
                d.to_bits().hash(&mut hasher);
            }
            n.expressions
                .consonant_timing_offset_ms
                .to_bits()
                .hash(&mut hasher);
            n.expressions.preutter_offset_ms.to_bits().hash(&mut hasher);
            n.expressions.overlap_offset_ms.to_bits().hash(&mut hasher);
            n.expressions.consonant_velocity.to_bits().hash(&mut hasher);
        }
        if let Some(vb) = voicebank {
            vb.name.hash(&mut hasher);
        }
        phonemizer_mode.hash(&mut hasher);
        let new_hash = hasher.finish();
        if new_hash != state.phoneme_cache_hash {
            state.phoneme_cache_hash = new_hash;
            state.phoneme_cache = vec![String::new(); notes.len()];
            state.note_phonemes_cache = vec![Vec::new(); notes.len()];
            state.oto_consonant_cache = vec![0.0; notes.len()];
            state.oto_preutter_cache = vec![0.0; notes.len()];
            state.oto_overlap_cache = vec![0.0; notes.len()];
            if let Some(vb) = voicebank {
                let phones = crate::phonemizer::JapanesePhonemizer::apply_phonemizer(
                    notes,
                    vb,
                    phonemizer_mode,
                );
                for p in phones {
                    if p.note_index < state.phoneme_cache.len() {
                        let is_first_phone = state.note_phonemes_cache[p.note_index].is_empty();
                        if is_first_phone {
                            if let Some(entry) = vb.find_entry(&p.lyric, &p.pitch) {
                                state.oto_consonant_cache[p.note_index] = entry.consonant.max(0.0);
                                state.oto_preutter_cache[p.note_index] =
                                    entry.preutterance.max(0.0);
                                state.oto_overlap_cache[p.note_index] = entry.overlap;
                            }
                        }
                        if state.phoneme_cache[p.note_index].is_empty() {
                            state.phoneme_cache[p.note_index] = p.lyric.clone();
                        } else {
                            state.phoneme_cache[p.note_index] =
                                format!("{} [{}]", state.phoneme_cache[p.note_index], p.lyric);
                        }
                        let note_pos = notes[p.note_index].position_ms;
                        let rel_pos = p.position_ms - note_pos;
                        state.note_phonemes_cache[p.note_index].push((
                            p.lyric,
                            rel_pos,
                            p.duration_ms,
                        ));
                    }
                }
            }
        }
    }
}
