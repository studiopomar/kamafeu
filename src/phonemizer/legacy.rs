use super::*;

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
            let is_plus = lyric_trimmed.starts_with('+');

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
        OpenUtauPhonemizer::apply(notes, vb, mode)
    }

    pub(super) fn apply_legacy_dispatch(
        notes: &[UNote],
        vb: &Voicebank,
        mode: PhonemizerMode,
    ) -> Vec<RenderPhone> {
        Self::apply_legacy_dispatch_with_cvvc_port(notes, vb, mode, true)
    }

    fn apply_legacy_dispatch_with_cvvc_port(
        notes: &[UNote],
        vb: &Voicebank,
        mode: PhonemizerMode,
        allow_cvvc_port: bool,
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
            let mut phones = Self::apply_legacy_dispatch_with_cvvc_port(
                &lookup_notes,
                vb,
                mode,
                allow_cvvc_port,
            );
            for phone in &mut phones {
                if let Some(note) = notes.get(phone.note_index) {
                    phone.pitch = note.pitch.clone();
                }
            }
            return phones;
        }
        if notes.iter().any(|note| note.timbre.is_some()) {
            // Build the phoneme topology once, without a selected PrefixMap
            // color. Resolving the complete phrase once prevents each color
            // pass from producing a subtly different VCV/CVVC structure.
            let mut neutral = notes.to_vec();
            for note in &mut neutral {
                note.timbre = None;
            }
            let mut topology_bank = vb.clone();
            // Use one deterministic neutral color only to choose the
            // phoneme topology. The selected suffix is removed below before
            // resolving each phone against its effective color bank.
            let topology_color = topology_bank
                .prefix_map
                .colors()
                .find(|color| color.is_empty())
                .or_else(|| {
                    topology_bank
                        .prefix_map
                        .colors()
                        .find(|color| !color.is_empty())
                })
                .map(str::to_owned)
                .unwrap_or_default();
            topology_bank.prefix_map.select_color(&topology_color);
            let topology =
                Self::apply_legacy_dispatch_with_cvvc_port(&neutral, &topology_bank, mode, false);

            let mut colors = std::collections::BTreeSet::new();
            colors.extend(notes.iter().map(|note| note.timbre.clone()));
            let mut mixed = Vec::new();
            for color in colors {
                let mut bank = vb.clone();
                if let Some(color) = &color {
                    bank.prefix_map.select_color(color);
                }
                for mut phone in topology.iter().cloned() {
                    let source_note = notes.get(phone.note_index);
                    // Japanese CVVC places the outgoing VC fragment at the
                    // end of the current note while it actually belongs to
                    // the following note's transition. Resolve that fragment
                    // with the destination color, just like OpenUtau.
                    let transition_target = source_note.and_then(|note| {
                        let note_end = note.position_ms + note.duration_ms;
                        let phone_end = phone.position_ms + phone.duration_ms;
                        if phone.duration_ms + 0.001 < note.duration_ms
                            && (phone_end - note_end).abs() <= 0.01
                        {
                            notes.get(phone.note_index + 1)
                        } else {
                            None
                        }
                    });
                    let effective_note = transition_target.or(source_note);
                    if effective_note.and_then(|note| note.timbre.as_ref()) != color.as_ref() {
                        continue;
                    }
                    // Freeze the mapped alias, including manual phonemizers,
                    // before the renderer sees the shared default voicebank.
                    let lookup_lyric = topology_bank
                        .prefix_map
                        .get_prefix_suffix(&phone.pitch)
                        .map(|(prefix, suffix)| {
                            phone
                                .lyric
                                .strip_prefix(prefix)
                                .unwrap_or(&phone.lyric)
                                .strip_suffix(suffix)
                                .unwrap_or_else(|| {
                                    phone
                                        .lyric
                                        .strip_prefix(prefix.trim())
                                        .unwrap_or(&phone.lyric)
                                })
                                .trim()
                                .to_string()
                        })
                        .unwrap_or_else(|| phone.lyric.clone());
                    if let Some(entry) = bank
                        .find_mapped_entry(&lookup_lyric, &phone.pitch)
                        .or_else(|| bank.find_exact_entry(&phone.lyric))
                    {
                        phone.lyric = entry.alias.clone();
                        phone.resolved_alias = Some(entry.alias.clone());
                        phone.resolved_timbre = transition_target
                            .and_then(|note| note.timbre.clone())
                            .or_else(|| color.clone());
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
                for mut phone in Self::apply_legacy_dispatch(&context, vb, note_mode)
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
            let is_plus = lyric_trimmed.starts_with('+');

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

        let temp_notes: Vec<UNote> = normalized_notes.iter().map(|(_, n)| n.clone()).collect();
        let orig_indices: Vec<usize> = normalized_notes.iter().map(|(idx, _)| *idx).collect();

        let mut phones = match mode {
            PhonemizerMode::None => Self::apply_raw_passthrough(&temp_notes, vb),
            PhonemizerMode::PortugueseBrapaVCCV | PhonemizerMode::PortugueseBrapaCVC => {
                // BRAPA is an explicit phonetic family, not a word G2P. The
                // existing port implements its transition cascade and keeps
                // authored aliases when a bank does not contain a candidate.
                brapa::VccvBrapaPhonemizer::apply_phonemizer(&temp_notes, vb)
            }
            PhonemizerMode::JapanesePresamp => {
                CoreJapanesePhonemizer::apply_presamp(&temp_notes, vb)
            }
            PhonemizerMode::EnglishArpasingPlus => {
                english::apply(&temp_notes, vb, PhonemizerMode::EnglishArpasingPlus)
            }
            PhonemizerMode::EnglishXSampa | PhonemizerMode::EnglishCpV => {
                // These families are explicit symbol sets. Preserve a USTX
                // phonetic hint verbatim instead of sending it through the
                // English word fallback, while retaining the shared
                // phrase/VC timing contract.
                english::apply(&temp_notes, vb, mode)
            }
            PhonemizerMode::KoreanCV
            | PhonemizerMode::KoreanCVC
            | PhonemizerMode::KoreanVCV
            | PhonemizerMode::KoreanCVVC
            | PhonemizerMode::KoreanCVCCV
            | PhonemizerMode::KoreanCBNN
            | PhonemizerMode::KoreanCVVCStandard => korean::apply(&temp_notes, vb, mode),
            PhonemizerMode::EnglishVCCV => {
                let mut expanded = temp_notes.clone();
                for note in &mut expanded {
                    let lyric = note.lyric.trim();
                    if lyric.is_empty()
                        || lyric == "+"
                        || lyric.eq_ignore_ascii_case("r")
                        || lyric.chars().any(char::is_whitespace)
                        || vccv::is_explicit_symbol(lyric)
                        || note.phonetic_hint.is_some()
                    {
                        continue;
                    }
                    if let Some(tokens) =
                        Self::openutau_dictionary_tokens(lyric, PhonemizerMode::EnglishG2P)
                    {
                        note.lyric = tokens.join(" ");
                    }
                }
                vccv::apply(&expanded, vb)
            }
            PhonemizerMode::EnglishArpasing => english::apply_arpasing(&temp_notes, vb),
            PhonemizerMode::EnglishG2P => english::apply(&temp_notes, vb, mode),
            PhonemizerMode::PortugueseCVVC | PhonemizerMode::PortugueseVCV => {
                portuguese::apply(&temp_notes, vb, mode)
            }
            PhonemizerMode::PortugueseBrazilianCVC => portuguese_cvc::apply(&temp_notes, vb),
            PhonemizerMode::RussianCVC => russian::apply(&temp_notes, vb),
            PhonemizerMode::RussianVCCV => russian::apply_vccv(&temp_notes, vb),
            PhonemizerMode::SpanishSyllable => {
                spanish::apply(&temp_notes, vb, PhonemizerMode::SpanishSyllable)
            }
            PhonemizerMode::SpanishMakkusan | PhonemizerMode::SpanishVCCV => {
                spanish::apply(&temp_notes, vb, mode)
            }
            PhonemizerMode::FrenchCMUSphinx => diphone::apply(&temp_notes, vb, mode),
            PhonemizerMode::FrenchCVVC | PhonemizerMode::FrenchVCCV => {
                french::apply(&temp_notes, vb, mode)
            }
            PhonemizerMode::ChineseCVV
            | PhonemizerMode::ChineseCVVC
            | PhonemizerMode::ChineseVCV
            | PhonemizerMode::CantoneseCVVC => chinese::apply(&temp_notes, vb, mode),
            PhonemizerMode::CantoneseSyo => chinese::apply_cantonese_syo(&temp_notes, vb),
            PhonemizerMode::ChineseCVVPlus => chinese::apply_cvv_plus(&temp_notes, vb),
            PhonemizerMode::PortugueseG2P => Self::apply_rule_g2p(&temp_notes, vb, mode),
            PhonemizerMode::FrenchG2P
            | PhonemizerMode::GermanG2P
            | PhonemizerMode::GermanVCCV
            | PhonemizerMode::GermanDiphone
            | PhonemizerMode::RussianG2P
            | PhonemizerMode::SpanishG2P
            | PhonemizerMode::TurkishG2P
            | PhonemizerMode::ChineseG2P
            | PhonemizerMode::CantoneseG2P
            | PhonemizerMode::TurkishCVVC
            | PhonemizerMode::ArabicG2P
            | PhonemizerMode::IcelandicG2P => Self::apply_rule_g2p(&temp_notes, vb, mode),
            PhonemizerMode::BasicCV => japanese::openutau::apply_cv(&temp_notes, vb),
            PhonemizerMode::VCV => japanese::openutau::apply_vcv(&temp_notes, vb),
            PhonemizerMode::CVVC if allow_cvvc_port => {
                japanese::openutau::apply_cvvc(&temp_notes, vb)
            }
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
                    if let Some(alternate) = phoneme_override.alternate {
                        let alternate_alias = format!("{}{}", phone.lyric, alternate);
                        if let Some(entry) = vb.find_mapped_entry(&alternate_alias, &phone.pitch) {
                            phone.lyric = entry.alias.clone();
                        }
                    }
                    if let Some(flags) = phoneme_override
                        .flags
                        .as_deref()
                        .filter(|flags| !flags.trim().is_empty())
                    {
                        phone.flags = crate::drivers::resampler_driver::merge_utau_flags(&[
                            phone.flags.as_str(),
                            flags,
                        ]);
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
                    if let Some(delta) = phoneme_override
                        .attack_time_delta_ms
                        .filter(|value| value.is_finite())
                    {
                        phone.expressions.attack_time_delta_ms += delta;
                    }
                    if let Some(delta) = phoneme_override
                        .release_time_delta_ms
                        .filter(|value| value.is_finite())
                    {
                        phone.expressions.release_time_delta_ms += delta;
                    }
                    if let Some(value) = phoneme_override
                        .consonant_timing_offset_ms
                        .filter(|value| value.is_finite())
                    {
                        phone.expressions.consonant_timing_offset_ms = value;
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
                    if let Some(envelope) = &phoneme_override.envelope {
                        phone.envelope = envelope.clone();
                    }
                    if let Some(vibrato) = &phoneme_override.vibrato {
                        phone.vibrato = vibrato.clone();
                    }
                    if let Some(pitch_bend) = &phoneme_override.pitch_bend {
                        phone.pitch_bend = pitch_bend.clone();
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
                            resolved_alias: None,
                            resolved_timbre: None,
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
                resolved_alias: None,
                resolved_timbre: None,
            });
        }
        phones
    }

    fn apply_rule_g2p(notes: &[UNote], vb: &Voicebank, mode: PhonemizerMode) -> Vec<RenderPhone> {
        g2p::apply_rule_g2p(notes, vb, mode)
    }

    pub(super) fn latin_language_tokens(word: &str, mode: PhonemizerMode) -> Vec<String> {
        g2p::latin_language_tokens(word, mode)
    }

    pub(super) fn openutau_dictionary_tokens(
        word: &str,
        mode: PhonemizerMode,
    ) -> Option<Vec<String>> {
        g2p::openutau_dictionary_tokens(word, mode)
    }

    pub(super) fn apply_japanese(
        notes: &[UNote],
        vb: &Voicebank,
        mode: PhonemizerMode,
    ) -> Vec<RenderPhone> {
        CoreJapanesePhonemizer::apply_japanese(notes, vb, mode)
    }
}
