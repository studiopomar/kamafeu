pub mod brapa;
pub mod english;
pub mod japanese;
pub mod portuguese;
pub mod romaji;
pub mod vccv;

pub use brapa::{BrapaCVCPhonemizer, VccvBrapaPhonemizer};
pub use english::EnglishPhonemizer;
pub use japanese::JapanesePhonemizer as CoreJapanesePhonemizer;
pub use portuguese::PortuguesePhonemizer;

use crate::oto::Voicebank;
use crate::project::model::UNote;
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

static CUSTOM_RULES: OnceLock<RwLock<HashMap<String, String>>> = OnceLock::new();

pub fn set_custom_rules(rules: HashMap<String, String>) {
    let store = CUSTOM_RULES.get_or_init(|| RwLock::new(HashMap::new()));
    if let Ok(mut current) = store.write() {
        *current = rules;
    }
}

fn apply_custom_rules(notes: &mut [UNote], mode: PhonemizerMode) {
    let Some(store) = CUSTOM_RULES.get() else {
        return;
    };
    let Ok(rules) = store.read() else { return };
    let key = format!("{mode:?}");
    let Some(script) = rules.get(&key) else {
        return;
    };
    for line in script.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((from, to)) = line.split_once("=>") else {
            continue;
        };
        let from = from.trim();
        let to = to.trim();
        if from.is_empty() {
            continue;
        }
        for note in notes.iter_mut() {
            if note.lyric.trim() == from {
                note.lyric = to.to_string();
            }
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default,
)]
pub enum PhonemizerMode {
    #[default]
    None, // Sem Fonemizador (Raw / Manual / Direto)
    BasicCV,             // JA: Japanese CV (Básico - Fonética/Romaji/Kana)
    VCV,                 // JA: Japanese VCV (Completo - Fonética/Romaji/Kana)
    CVVC,                // JA: Japanese CVVC (Fonética/Romaji/Kana)
    EnglishArpasing,     // EN: English Arpasing (Fonética Direta)
    EnglishVCCV,         // EN: English VCCV (Fonética Direta)
    EnglishG2P,          // EN: English G2P (Palavras / Texto -> ARPABET)
    PortugueseBrapaVCCV, // PT: VCCV BRAPA (xiao / PT-BR 3.7)
    PortugueseBrapaCVC,  // PT: BRAPA CVC (Fonética Direta / Tokens BRAPA)
    PortugueseCVVC,      // PT: Portuguese CVVC (Fonética Direta)
    PortugueseVCV,       // PT: Portuguese VCV (Fonética Direta)
    PortugueseG2P,       // PT: Português G2P (Palavras / Texto em Português -> Fonemas)
}

impl PhonemizerMode {
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .iter()
            .copied()
            .find(|mode| format!("{mode:?}") == name)
    }

    pub const ALL: [Self; 10] = [
        Self::BasicCV,
        Self::VCV,
        Self::CVVC,
        Self::EnglishArpasing,
        Self::EnglishVCCV,
        Self::PortugueseBrapaVCCV,
        Self::PortugueseBrapaCVC,
        Self::PortugueseCVVC,
        Self::PortugueseVCV,
        Self::PortugueseG2P,
    ];
}

pub fn consonant_velocity_time_scale(velocity: f64) -> f64 {
    let velocity = if velocity.is_finite() {
        velocity
    } else {
        100.0
    };
    // Negative values are useful for deliberately lengthening consonants.
    2.0f64.powf(1.0 - velocity.clamp(-100.0, 200.0) / 100.0)
}

pub struct RenderPhone {
    pub note_index: usize,
    pub lyric: String,
    pub pitch: String,
    pub position_ms: f64,
    pub duration_ms: f64,
    pub envelope: crate::dsp::envelope::UtauEnvelope,
    pub expressions: crate::project::model::UExpressions,
    pub pitch_bend: crate::project::model::UPitchBend,
    pub vibrato: crate::dsp::pitch::VibratoParam,
    pub flags: String,
}

impl RenderPhone {
    pub fn midi_key(&self) -> u8 {
        crate::dsp::pitch::note_name_to_midi(&self.pitch).unwrap_or(60)
    }
}

pub struct JapanesePhonemizer;

impl JapanesePhonemizer {
    pub fn extract_vowel(lyric: &str) -> Option<&'static str> {
        CoreJapanesePhonemizer::extract_vowel(lyric)
    }

    pub fn extract_consonant(lyric: &str) -> Option<&'static str> {
        CoreJapanesePhonemizer::extract_consonant(lyric)
    }

    pub fn preprocess_plus_notes(notes: &[UNote]) -> Vec<UNote> {
        let mut result: Vec<UNote> = Vec::new();
        for note in notes {
            let lyric_trimmed = note.lyric.trim();
            let is_plus = lyric_trimmed == "+" || lyric_trimmed.starts_with("+ ");

            if is_plus {
                if let Some(last) = result.last_mut() {
                    let last_end = last.position_ms + last.duration_ms;
                    if (note.position_ms - last_end).abs() <= 2.0 {
                        last.duration_ms += note.duration_ms;
                        continue;
                    } else {
                        // Unconnected plus note: borrow lyric from preceding note
                        let mut fallback_note = note.clone();
                        fallback_note.lyric = last.lyric.clone();
                        result.push(fallback_note);
                        continue;
                    }
                }
            }

            result.push(note.clone());
        }
        result
    }

    pub fn apply_phonemizer(
        notes: &[UNote],
        vb: &Voicebank,
        mode: PhonemizerMode,
    ) -> Vec<RenderPhone> {
        if notes.iter().any(|note| note.voicebank_pitch.is_some()) {
            let lookup_notes: Vec<UNote> = notes
                .iter()
                .map(|note| {
                    let mut lookup = note.clone();
                    if let Some(pitch) = &note.voicebank_pitch {
                        lookup.pitch = pitch.clone();
                    }
                    lookup.voicebank_pitch = None;
                    lookup
                })
                .collect();
            let mut phones = Self::apply_phonemizer(&lookup_notes, vb, mode);
            for phone in &mut phones {
                if let Some(note) = notes.get(phone.note_index) {
                    phone.pitch = note.pitch.clone();
                }
            }
            return phones;
        }
        if notes.iter().any(|note| note.timbre.is_some()) {
            // Resolve each distinct bank with the complete phrase so VCV/VV
            // retain their preceding vowel context across timbre changes.
            let mut neutral = notes.to_vec();
            for note in &mut neutral {
                note.timbre = None;
            }
            let mut colors = std::collections::BTreeSet::new();
            for note in notes {
                colors.insert(note.timbre.clone());
            }
            let mut mixed = Vec::new();
            for color in colors {
                let mut bank = vb.clone();
                if let Some(color) = &color {
                    bank.prefix_map.select_color(color);
                }
                for mut phone in Self::apply_phonemizer(&neutral, &bank, mode) {
                    if notes[phone.note_index].timbre != color {
                        continue;
                    }
                    // Freeze the mapped alias, including manual phonemizers,
                    // before the renderer sees the shared default voicebank.
                    if let Some(entry) = bank.find_mapped_entry(&phone.lyric, &phone.pitch) {
                        phone.lyric = entry.alias.clone();
                    }
                    mixed.push(phone);
                }
            }
            mixed.sort_by(|a, b| a.position_ms.total_cmp(&b.position_ms));
            return mixed;
        }
        if notes.iter().any(|note| note.phonemizer_override.is_some()) {
            let mut mixed = Vec::new();
            for (index, note) in notes.iter().enumerate() {
                let note_mode = note
                    .phonemizer_override
                    .as_deref()
                    .and_then(PhonemizerMode::from_name)
                    .unwrap_or(mode);
                // A phonemizer override changes the algorithm for this note, not
                // its musical neighbourhood. Keep the adjacent notes in the
                // request so VCCV/CVVC phonemizers can still see the outgoing
                // vowel, next consonant, pitch and phrase boundary.
                let window_start = index.saturating_sub(1);
                let window_end = (index + 2).min(notes.len());
                let center = index - window_start;
                let mut context = notes[window_start..window_end].to_vec();
                for context_note in &mut context {
                    context_note.phonemizer_override = None;
                }
                for mut phone in Self::apply_phonemizer(&context, vb, note_mode)
                    .into_iter()
                    .filter(|phone| phone.note_index == center)
                {
                    phone.note_index = index;
                    mixed.push(phone);
                }
            }
            if !mixed.is_empty() {
                return mixed;
            }
        }
        let mut normalized_notes: Vec<(usize, UNote)> = Vec::new();
        for (orig_idx, note) in notes.iter().enumerate() {
            let lyric_trimmed = note.lyric.trim();
            let is_plus = lyric_trimmed == "+" || lyric_trimmed.starts_with("+ ");

            if is_plus {
                if let Some((_last_orig_idx, last_note)) = normalized_notes.last_mut() {
                    let last_end = last_note.position_ms + last_note.duration_ms;
                    if (note.position_ms - last_end).abs() <= 2.0 {
                        last_note.duration_ms += note.duration_ms;
                        continue;
                    } else {
                        let mut fallback_note = note.clone();
                        fallback_note.lyric = last_note.lyric.clone();
                        normalized_notes.push((orig_idx, fallback_note));
                        continue;
                    }
                }
            }

            normalized_notes.push((orig_idx, note.clone()));
        }

        let mut temp_notes: Vec<UNote> = normalized_notes.iter().map(|(_, n)| n.clone()).collect();
        apply_custom_rules(&mut temp_notes, mode);
        let orig_indices: Vec<usize> = normalized_notes.iter().map(|(idx, _)| *idx).collect();

        let mut phones = match mode {
            PhonemizerMode::None => Self::apply_raw_passthrough(&temp_notes, vb),
            PhonemizerMode::PortugueseBrapaVCCV | PhonemizerMode::PortugueseBrapaCVC => {
                VccvBrapaPhonemizer::apply_phonemizer(&temp_notes, vb)
            }
            PhonemizerMode::EnglishVCCV => vccv::apply(&temp_notes, vb),
            PhonemizerMode::EnglishArpasing | PhonemizerMode::EnglishG2P => {
                Self::apply_english(&temp_notes, vb, mode)
            }
            PhonemizerMode::PortugueseCVVC
            | PhonemizerMode::PortugueseVCV
            | PhonemizerMode::PortugueseG2P => Self::apply_portuguese(&temp_notes, vb, mode),
            _ => Self::apply_japanese(&temp_notes, vb, mode),
        };

        for p in &mut phones {
            if let Some(&real_idx) = orig_indices.get(p.note_index) {
                p.note_index = real_idx;
            }
        }

        for (orig_idx, note) in notes.iter().enumerate() {
            if !note.phoneme_durations_ms.is_empty() {
                let note_phone_indices: Vec<usize> = phones
                    .iter()
                    .enumerate()
                    .filter(|(_, p)| p.note_index == orig_idx)
                    .map(|(idx, _)| idx)
                    .collect();

                if note_phone_indices.len() == note.phoneme_durations_ms.len() {
                    let initial_offset = phones[note_phone_indices[0]].position_ms;
                    let mut cur_pos = initial_offset;
                    for (i, &phone_idx) in note_phone_indices.iter().enumerate() {
                        let dur = note.phoneme_durations_ms[i];
                        phones[phone_idx].position_ms = cur_pos;
                        phones[phone_idx].duration_ms = dur;
                        cur_pos += dur;
                    }
                }
            }

            if !note.phoneme_overrides.is_empty() {
                let note_phone_indices: Vec<usize> = phones
                    .iter()
                    .enumerate()
                    .filter(|(_, phone)| phone.note_index == orig_idx)
                    .map(|(phone_idx, _)| phone_idx)
                    .collect();
                for phoneme_override in &note.phoneme_overrides {
                    let Some(&phone_idx) = note_phone_indices.get(phoneme_override.index) else {
                        continue;
                    };
                    let phone = &mut phones[phone_idx];
                    if let Some(alias) = phoneme_override
                        .phoneme
                        .as_ref()
                        .filter(|alias| !alias.trim().is_empty())
                    {
                        phone.lyric = alias.clone();
                    }
                    if let Some(offset) = phoneme_override.offset_ms.filter(|v| v.is_finite()) {
                        phone.position_ms += offset;
                    }
                    if let Some(delta) = phoneme_override
                        .preutter_delta_ms
                        .filter(|value| value.is_finite())
                    {
                        phone.expressions.preutter_offset_ms += delta;
                    }
                    if let Some(delta) = phoneme_override
                        .overlap_delta_ms
                        .filter(|value| value.is_finite())
                    {
                        phone.expressions.overlap_offset_ms += delta;
                    }
                    if let Some(value) = phoneme_override.velocity {
                        phone.expressions.velocity = value;
                        phone.expressions.consonant_velocity = value;
                    }
                    if let Some(value) = phoneme_override.volume {
                        phone.expressions.volume = value;
                    }
                    if let Some(value) = phoneme_override.attack {
                        phone.expressions.attack = value;
                    }
                    if let Some(value) = phoneme_override.decay {
                        phone.expressions.decay = value;
                    }
                    if let Some(value) = phoneme_override.breathiness {
                        phone.expressions.breathiness = value;
                    }
                    if let Some(value) = phoneme_override.gender {
                        phone.expressions.gender = value;
                    }
                    if let Some(value) = phoneme_override.modulation {
                        phone.expressions.modulation = value;
                    }
                    if let Some(value) = phoneme_override.pitch_delta {
                        phone.expressions.pitch_delta = value;
                    }
                    if let Some(value) = phoneme_override.dynamics {
                        phone.expressions.dynamics = value;
                    }
                }
            }
        }

        if notes.iter().any(|note| !note.phoneme_overrides.is_empty()) {
            phones.sort_by(|left, right| left.position_ms.total_cmp(&right.position_ms));
            for index in 0..phones.len().saturating_sub(1) {
                let current_note = phones[index].note_index;
                let next_note = phones[index + 1].note_index;
                let same_phrase = current_note == next_note
                    || notes
                        .get(current_note)
                        .zip(notes.get(next_note))
                        .is_some_and(|(current, next)| {
                            (current.position_ms + current.duration_ms - next.position_ms).abs()
                                <= 0.001
                        });
                let duration = phones[index + 1].position_ms - phones[index].position_ms;
                if same_phrase && duration > 0.0 {
                    phones[index].duration_ms = duration;
                }
            }
        }

        phones
    }

    fn apply_raw_passthrough(notes: &[UNote], vb: &Voicebank) -> Vec<RenderPhone> {
        let mut phones: Vec<RenderPhone> = Vec::new();
        for (i, note) in notes.iter().enumerate() {
            let raw_lyric = note.lyric.trim();
            if raw_lyric.is_empty() || raw_lyric == "R" || raw_lyric == "r" || raw_lyric == "+" {
                continue;
            }

            let is_multi = raw_lyric.contains(['.', ';', ',', '|', '/']);

            if is_multi {
                let parts: Vec<&str> = raw_lyric
                    .split(['.', ';', ',', '|', '/'])
                    .map(str::trim)
                    .filter(|s| !s.is_empty() && *s != "+" && *s != "R" && *s != "r")
                    .collect();

                if !parts.is_empty() {
                    let num_parts = parts.len();
                    let durations = if note.phoneme_durations_ms.len() == num_parts {
                        note.resolved_phoneme_durations(num_parts)
                    } else {
                        // Manual aliases describe phoneme positions, not equal slices of a
                        // musical note. Resolve every following alias from its oto.ini
                        // preutterance, backwards from the note end. This is the same
                        // invariant used by OpenUtau phonemizers and prevents a short VC
                        // bridge from receiving half of a long sustained note.
                        let mut resolved = vec![0.0; num_parts];
                        let mut remaining = note.duration_ms.max(0.0);
                        for part_idx in (1..num_parts).rev() {
                            let alias = parts[part_idx];
                            let stretch =
                                consonant_velocity_time_scale(note.expressions.consonant_velocity);
                            let requested = vb
                                .find_mapped_entry(alias, &note.pitch)
                                .map(|oto| {
                                    let preutter = oto.preutterance.max(0.0) * stretch;
                                    let overlap = oto.overlap * stretch;
                                    if overlap < 0.0 {
                                        preutter - overlap
                                    } else {
                                        preutter
                                    }
                                })
                                .unwrap_or(60.0);
                            let duration = requested.max(5.0).min((remaining * 0.5).max(5.0));
                            resolved[part_idx] = duration;
                            remaining = (remaining - duration).max(0.0);
                        }
                        resolved[0] = remaining;
                        resolved
                    };
                    let mut sub_position_ms = note.position_ms;
                    for (part_idx, part) in parts.into_iter().enumerate() {
                        let sub_duration_ms = durations[part_idx];
                        let mut sub_envelope = note.envelope.clone();
                        // Smooth crossfade transitions between intra-note phonemes:
                        // Preserve user-authored attack on the first phone and release on the last phone,
                        // while ensuring interior junctions have seamless crossfade boundaries.
                        if part_idx > 0 {
                            sub_envelope.p1 = 0.0;
                            sub_envelope.p2 = 5.0;
                            sub_envelope.v1 = 0.0;
                            sub_envelope.v2 = 100.0;
                        }
                        if part_idx < num_parts - 1 {
                            sub_envelope.p4 = 0.0;
                            sub_envelope.p5 = 35.0;
                            sub_envelope.v4 = 100.0;
                            sub_envelope.v5 = 0.0;
                        }
                        phones.push(RenderPhone {
                            note_index: i,
                            lyric: part.to_string(),
                            pitch: note.pitch.clone(),
                            position_ms: sub_position_ms,
                            duration_ms: sub_duration_ms,
                            envelope: sub_envelope,
                            expressions: note.expressions.clone(),
                            pitch_bend: note.pitch_bend.clone(),
                            vibrato: note.vibrato.clone(),
                            flags: note.flags.clone(),
                        });
                        sub_position_ms += sub_duration_ms;
                    }
                    continue;
                }
            }

            phones.push(RenderPhone {
                note_index: i,
                lyric: raw_lyric.to_string(),
                pitch: note.pitch.clone(),
                position_ms: note.position_ms,
                duration_ms: note.duration_ms,
                envelope: note.envelope.clone(),
                expressions: note.expressions.clone(),
                pitch_bend: note.pitch_bend.clone(),
                vibrato: note.vibrato.clone(),
                flags: note.flags.clone(),
            });
        }
        phones
    }

    fn apply_japanese(notes: &[UNote], vb: &Voicebank, mode: PhonemizerMode) -> Vec<RenderPhone> {
        CoreJapanesePhonemizer::apply_japanese(notes, vb, mode)
    }

    fn apply_english(notes: &[UNote], vb: &Voicebank, mode: PhonemizerMode) -> Vec<RenderPhone> {
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

            let word_phones = match mode {
                PhonemizerMode::EnglishG2P => EnglishPhonemizer::word_to_arpabet(&note.lyric),
                _ => EnglishPhonemizer::phonetic_tokens(&note.lyric),
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

                if EnglishPhonemizer::is_vowel(p) {
                    prev_vowel = Some(p.clone());
                } else {
                    prev_vowel = None;
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
                });
            }

            prev_note_end_ms = Some(note.position_ms + note.duration_ms);
        }

        phones
    }

    fn apply_portuguese(notes: &[UNote], vb: &Voicebank, mode: PhonemizerMode) -> Vec<RenderPhone> {
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
                PhonemizerMode::PortugueseG2P => {
                    PortuguesePhonemizer::word_to_phonemes(&note.lyric)
                }
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
                });
            }

            prev_note_end_ms = Some(note.position_ms + note.duration_ms);
        }

        phones
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn per_note_timbre_preserves_vcv_context_and_default() {
        let dir = tempfile::tempdir().unwrap();
        let mut vb = Voicebank::new(dir.path()).unwrap();
        vb.prefix_map = crate::oto::PrefixMap::parse_yaml_str("subbanks:\n  - color: ''\n    suffix: _normal\n    tone_ranges: [C4-C5]\n  - color: Soft\n    suffix: _soft\n    tone_ranges: [C4-C5]\n");
        for alias in ["- あ_normal", "a か_soft", "a か_normal"] {
            vb.entries.insert(
                alias.to_string(),
                crate::oto::OtoEntry::new(
                    format!("{alias}.wav"),
                    alias.to_string(),
                    0.0,
                    50.0,
                    10.0,
                    0.0,
                    0.0,
                ),
            );
        }
        let mut notes = vec![
            UNote::new("あ", "C4", 0.0, 400.0),
            UNote::new("か", "C4", 400.0, 400.0),
            UNote::new("か", "C4", 800.0, 400.0),
        ];
        notes[1].timbre = Some("Soft".to_string());
        let phones = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::VCV);
        assert_eq!(
            phones.iter().map(|p| p.lyric.as_str()).collect::<Vec<_>>(),
            ["- あ_normal", "a か_soft", "a か_normal"]
        );
        assert_eq!(
            phones.iter().map(|p| p.note_index).collect::<Vec<_>>(),
            [0, 1, 2]
        );
        assert_eq!(phones[1].position_ms, 400.0);
        assert_eq!(vb.prefix_map.selected_color(), "");
        // Existing projects without the new field still inherit the default.
        let legacy: UNote =
            serde_json::from_str(r#"{"lyric":"a","pitch":"C4","position_ms":0,"duration_ms":400}"#)
                .unwrap();
        assert_eq!(legacy.timbre, None);
    }

    #[test]
    fn test_phrase_start_head_alias() {
        let mut entries = HashMap::new();
        entries.insert(
            "- か".to_string(),
            crate::oto::OtoEntry::new(
                "ka.wav".to_string(),
                "- か".to_string(),
                0.0,
                50.0,
                0.0,
                0.0,
                0.0,
            ),
        );
        entries.insert(
            "a き".to_string(),
            crate::oto::OtoEntry::new(
                "ki.wav".to_string(),
                "a き".to_string(),
                0.0,
                50.0,
                0.0,
                0.0,
                0.0,
            ),
        );

        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let notes = vec![
            UNote::new("ka", "C4", 0.0, 400.0),
            UNote::new("ki", "C4", 400.0, 400.0),
            UNote::new("ka", "C4", 2000.0, 400.0),
        ];

        let phones = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::VCV);
        assert_eq!(phones.len(), 3);
        assert_eq!(phones[0].lyric, "- か");
        assert_eq!(phones[1].lyric, "a き");
        assert_eq!(phones[2].lyric, "- か");
    }

    #[test]
    fn test_english_phonemizer() {
        let phones = EnglishPhonemizer::word_to_arpabet("hello");
        assert_eq!(phones, vec!["hh", "ah", "l", "ow"]);

        let sing = EnglishPhonemizer::word_to_arpabet("sing");
        assert_eq!(sing, vec!["s", "ih", "ng"]);
    }

    #[test]
    fn test_portuguese_phonemizer() {
        let phones = PortuguesePhonemizer::word_to_phonemes("canto");
        assert_eq!(phones, vec!["ka", "n", "to"]);

        let amor = PortuguesePhonemizer::word_to_phonemes("amor");
        assert_eq!(amor, vec!["a", "mo", "r"]);
    }

    #[test]
    fn test_none_phonemizer_passthrough() {
        let entries = std::collections::HashMap::new();
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let notes = vec![
            UNote::new("k_a", "C4", 0.0, 400.0),
            UNote::new("R", "C4", 400.0, 200.0),
            UNote::new("- sa", "D4", 600.0, 400.0),
        ];

        let phones = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::None);
        assert_eq!(phones.len(), 2);
        assert_eq!(phones[0].lyric, "k_a");
        assert_eq!(phones[0].position_ms, 0.0);
        assert_eq!(phones[1].lyric, "- sa");
        assert_eq!(phones[1].position_ms, 600.0);
    }

    #[test]
    fn test_universal_plus_continuation_in_any_mode() {
        let entries = std::collections::HashMap::new();
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let notes = vec![
            UNote::new("k aa", "C4", 0.0, 500.0),
            UNote::new("+", "D4", 500.0, 250.0),
            UNote::new("+", "E4", 750.0, 250.0),
        ];

        let phones_none = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::None);
        assert_eq!(phones_none.len(), 1);
        assert_eq!(phones_none[0].lyric, "k aa");
        assert_eq!(phones_none[0].duration_ms, 1000.0); // 500 + 250 + 250

        let notes_en = vec![
            UNote::new("sing", "C4", 0.0, 500.0),
            UNote::new("+", "D4", 500.0, 250.0),
            UNote::new("+", "E4", 750.0, 250.0),
        ];
        let phones_en =
            JapanesePhonemizer::apply_phonemizer(&notes_en, &vb, PhonemizerMode::EnglishArpasing);
        let total_dur: f64 = phones_en.iter().map(|p| p.duration_ms).sum();
        assert_eq!(total_dur, 1000.0);
    }

    #[test]
    fn test_manual_mode_semicolon_and_comma_sub_phonemes() {
        let entries = std::collections::HashMap::new();
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let notes = vec![UNote::new("-k;k ae;ae n", "C4", 0.0, 300.0)];

        let phones = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::None);
        assert_eq!(phones.len(), 3);
        assert_eq!(phones[0].lyric, "-k");
        assert_eq!(phones[0].duration_ms, 180.0);
        assert_eq!(phones[1].lyric, "k ae");
        assert_eq!(phones[1].duration_ms, 60.0);
        assert_eq!(phones[2].lyric, "ae n");
        assert_eq!(phones[2].duration_ms, 60.0);

        let notes_comma = vec![UNote::new("k ae, ae n", "C4", 0.0, 300.0)];
        let phones_comma =
            JapanesePhonemizer::apply_phonemizer(&notes_comma, &vb, PhonemizerMode::None);
        assert_eq!(phones_comma.len(), 2);
        assert_eq!(phones_comma[0].lyric, "k ae");
        assert_eq!(phones_comma[0].duration_ms, 240.0);
        assert_eq!(phones_comma[1].lyric, "ae n");
        assert_eq!(phones_comma[1].duration_ms, 60.0);

        let mut resized_note = UNote::new("k ae.ae n.", "B3", 0.0, 480.0);
        resized_note.set_phoneme_boundary(2, 1, 360.0);
        let resized =
            JapanesePhonemizer::apply_phonemizer(&[resized_note], &vb, PhonemizerMode::None);
        assert_eq!(resized.len(), 2);
        assert_eq!(resized[0].lyric, "k ae");
        assert_eq!(resized[0].position_ms, 0.0);
        assert_eq!(resized[0].duration_ms, 360.0);
        assert_eq!(resized[1].lyric, "ae n");
        assert_eq!(resized[1].position_ms, 360.0);
        assert_eq!(resized[1].duration_ms, 120.0);

        let notes_dot = vec![UNote::new("m an. an d. d eh. eh l. l a.", "C4", 0.0, 500.0)];
        let phones_dot =
            JapanesePhonemizer::apply_phonemizer(&notes_dot, &vb, PhonemizerMode::None);
        assert_eq!(phones_dot.len(), 5);
        assert_eq!(phones_dot[0].lyric, "m an");
        assert_eq!(phones_dot[0].duration_ms, 260.0);
        assert_eq!(phones_dot[1].lyric, "an d");
        assert_eq!(phones_dot[1].duration_ms, 60.0);
        assert_eq!(phones_dot[2].lyric, "d eh");
        assert_eq!(phones_dot[2].duration_ms, 60.0);
        assert_eq!(phones_dot[3].lyric, "eh l");
        assert_eq!(phones_dot[3].duration_ms, 60.0);
        assert_eq!(phones_dot[4].lyric, "l a");
        assert_eq!(phones_dot[4].duration_ms, 60.0);
    }

    #[test]
    fn manual_composite_uses_oto_preutterance_instead_of_equal_slices() {
        let mut entries = HashMap::new();
        entries.insert(
            "a rh-".to_string(),
            crate::oto::OtoEntry::new(
                "a-rh.wav".to_string(),
                "a rh-".to_string(),
                0.0,
                268.0,
                -147.0,
                122.0,
                90.0,
            ),
        );
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };
        let phones = JapanesePhonemizer::apply_phonemizer(
            &[UNote::new("t a.a rh-", "B3", 0.0, 1200.0)],
            &vb,
            PhonemizerMode::None,
        );
        assert_eq!(phones.len(), 2);
        assert!((phones[0].duration_ms - 1078.0).abs() < 1e-6);
        assert!((phones[1].position_ms - 1078.0).abs() < 1e-6);
        assert!((phones[1].duration_ms - 122.0).abs() < 1e-6);
    }

    #[test]
    fn test_g2p_vs_phonetic_mode() {
        let entries = std::collections::HashMap::new();
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let notes_word = vec![UNote::new("can", "C4", 0.0, 300.0)];
        let phones_g2p =
            JapanesePhonemizer::apply_phonemizer(&notes_word, &vb, PhonemizerMode::EnglishG2P);
        assert_eq!(phones_g2p.len(), 3);
        assert_eq!(phones_g2p[0].lyric, "k");
        assert_eq!(phones_g2p[1].lyric, "ae");
        assert_eq!(phones_g2p[2].lyric, "n");

        let phones_direct =
            JapanesePhonemizer::apply_phonemizer(&notes_word, &vb, PhonemizerMode::EnglishArpasing);
        assert_eq!(phones_direct.len(), 1);
        assert_eq!(phones_direct[0].lyric, "can");

        let notes_pt = vec![UNote::new("sol", "C4", 0.0, 300.0)];
        let phones_pt_g2p =
            JapanesePhonemizer::apply_phonemizer(&notes_pt, &vb, PhonemizerMode::PortugueseG2P);
        assert_eq!(phones_pt_g2p.len(), 2);
        assert_eq!(phones_pt_g2p[0].lyric, "so");
        assert_eq!(phones_pt_g2p[1].lyric, "w");

        let notes_brapa = vec![UNote::new("canto", "C4", 0.0, 400.0)];
        let phones_brapa = JapanesePhonemizer::apply_phonemizer(
            &notes_brapa,
            &vb,
            PhonemizerMode::PortugueseBrapaVCCV,
        );
        assert!(!phones_brapa.is_empty());
    }

    #[test]
    fn test_vccv_brapa_phonemizer_full_cascade() {
        let mut entries = std::collections::HashMap::new();
        entries.insert(
            "- ka".to_string(),
            crate::oto::OtoEntry::new(
                "ka.wav".to_string(),
                "- ka".to_string(),
                0.0,
                50.0,
                0.0,
                0.0,
                0.0,
            ),
        );
        entries.insert(
            "a s".to_string(),
            crate::oto::OtoEntry::new(
                "as.wav".to_string(),
                "a s".to_string(),
                0.0,
                50.0,
                0.0,
                0.0,
                0.0,
            ),
        );
        entries.insert(
            "s a".to_string(),
            crate::oto::OtoEntry::new(
                "sa.wav".to_string(),
                "s a".to_string(),
                0.0,
                50.0,
                0.0,
                0.0,
                0.0,
            ),
        );

        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "BRAPA VB".to_string(),
            author: "Xiao".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let notes = vec![
            UNote::new("ka", "C4", 0.0, 400.0),
            UNote::new("sa", "C4", 400.0, 400.0),
        ];

        let phones =
            JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::PortugueseBrapaVCCV);
        assert_eq!(phones[0].lyric, "- ka");
        assert_eq!(phones[1].lyric, "a s");
        assert!(
            phones[1].position_ms < 400.0,
            "VC transition 'a s' must be before the note start line (400ms)"
        );
        assert_eq!(phones[2].lyric, "s a");
        assert_eq!(
            phones[2].position_ms, 400.0,
            "CV note 's a' must start exactly at the note start line (400ms)"
        );
    }

    #[test]
    fn test_custom_phoneme_durations_across_all_phonemizers() {
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries: HashMap::new(),
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let mut note = UNote::new("can", "C4", 100.0, 600.0);
        note.phoneme_durations_ms = vec![100.0, 350.0, 150.0];

        let phones = JapanesePhonemizer::apply_phonemizer(&[note], &vb, PhonemizerMode::EnglishG2P);
        assert_eq!(phones.len(), 3);
        assert_eq!(phones[0].duration_ms, 100.0);
        assert_eq!(phones[0].position_ms, 100.0);
        assert_eq!(phones[1].duration_ms, 350.0);
        assert_eq!(phones[1].position_ms, 200.0);
        assert_eq!(phones[2].duration_ms, 150.0);
        assert_eq!(phones[2].position_ms, 550.0);
    }
}
