use std::fs;

use crate::oto::entry::OtoEntry;

use super::{AliasResolutionReport, Voicebank};

impl Voicebank {
    /// Exact phonetic lookup with pitch and candidate mapping. Never changes case or drops a VC/VCV prefix.
    pub fn find_exact_entry(&self, alias: &str) -> Option<&OtoEntry> {
        self.entries.get(alias)
    }

    pub fn find_mapped_entry(&self, lyric: &str, pitch_name: &str) -> Option<&OtoEntry> {
        let lyric_trimmed = lyric.trim();
        if lyric_trimmed.is_empty() {
            return None;
        }
        let (prefix, suffix) = self
            .prefix_map
            .get_prefix_suffix(pitch_name)
            .unwrap_or(("", ""));

        let clean_p = prefix.trim_matches('"').trim_matches('\'');
        let clean_s = suffix.trim_matches('"').trim_matches('\'');

        let lyric_cands = crate::phonemizer::romaji::lyric_candidates(lyric_trimmed);

        let mut candidates = Vec::new();

        for cand_lyric in &lyric_cands {
            let raw_alias = self.prefix_map.get_alias(cand_lyric, pitch_name);
            candidates.push(raw_alias);

            if !clean_p.is_empty() || !clean_s.is_empty() {
                candidates.push(format!("{}{}{}", clean_p, cand_lyric, clean_s));

                if !clean_p.is_empty() {
                    let p_trimmed = clean_p.trim();
                    candidates.push(format!("{} {}{}", p_trimmed, cand_lyric, clean_s));
                    candidates.push(format!("{}_{}{}", p_trimmed, cand_lyric, clean_s));
                    candidates.push(format!("{}{}{}", p_trimmed, cand_lyric, clean_s));
                }

                if !clean_s.is_empty() {
                    let s_trimmed = clean_s.trim();
                    candidates.push(format!("{}{}_{}", clean_p, cand_lyric, s_trimmed));
                    candidates.push(format!("{}{}{}", clean_p, cand_lyric, s_trimmed));
                }
            }

            candidates.push(cand_lyric.clone());

            if !pitch_name.is_empty() {
                candidates.push(format!("{}_{}", cand_lyric, pitch_name));
                candidates.push(format!("{}_{}", pitch_name, cand_lyric));
                candidates.push(format!("{} {}", pitch_name, cand_lyric));
                candidates.push(format!("{} {}", cand_lyric, pitch_name));
                candidates.push(format!("{}{}", pitch_name, cand_lyric));
            }
        }

        for cand in &candidates {
            if let Some(entry) = self.entries.get(cand) {
                return Some(entry);
            }
        }

        // Exact authored aliases remain the final local fallback. This keeps
        // aliases such as "k a" available, while allowing a mapped pitch
        // subbank such as "a s_C4" to win when the current note has one.
        if let Some(entry) = self.entries.get(lyric_trimmed) {
            return Some(entry);
        }

        if let Some(target_midi) = crate::dsp::pitch::note_name_to_midi(pitch_name) {
            let mut alt_pitches: Vec<(&str, i32)> = self
                .prefix_map
                .mapped_pitches()
                .filter(|&p| p != pitch_name)
                .filter_map(|p| {
                    crate::dsp::pitch::note_name_to_midi(p)
                        .map(|m| (p, (m as i32 - target_midi as i32).abs()))
                })
                .collect();

            alt_pitches.sort_by_key(|&(_, dist)| dist);

            for (alt_pitch, _) in alt_pitches {
                if let Some((pfx, sfx)) = self.prefix_map.get_prefix_suffix(alt_pitch) {
                    for cand_lyric in &lyric_cands {
                        let cand = format!("{}{}{}", pfx.trim(), cand_lyric, sfx.trim());
                        if let Some(entry) = self.entries.get(&cand) {
                            return Some(entry);
                        }
                    }
                }
            }
        }

        None
    }

    /// Content identity for render provenance and cache diagnostics. It uses
    /// sorted aliases, oto timing and WAV bytes, so replacing a sample at the
    /// same path cannot preserve the old identity.
    pub fn content_fingerprint(&self) -> String {
        let mut hash = 0xcbf29ce484222325u64;
        let mut entries = self.entries.iter().collect::<Vec<_>>();
        entries.sort_by(|left, right| left.0.cmp(right.0));
        for (alias, entry) in entries {
            for value in [
                alias.as_bytes(),
                entry.wav_filename.as_bytes(),
                format!(
                    "{:.9},{:.9},{:.9},{:.9},{:.9},{:?},{:?},{:?}",
                    entry.offset,
                    entry.consonant,
                    entry.cutoff,
                    entry.preutterance,
                    entry.overlap,
                    entry.loop_start,
                    entry.loop_end,
                    entry.tail_start
                )
                .as_bytes(),
            ] {
                for byte in value {
                    hash ^= u64::from(*byte);
                    hash = hash.wrapping_mul(0x100000001b3);
                }
                hash ^= 0xff;
                hash = hash.wrapping_mul(0x100000001b3);
            }
            if let Ok(bytes) = fs::read(self.root_path.join(&entry.wav_filename)) {
                for byte in bytes {
                    hash ^= u64::from(byte);
                    hash = hash.wrapping_mul(0x100000001b3);
                }
            }
            hash ^= 0xfe;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{hash:016x}")
    }

    /// Resolves an alias through the same prefix/suffix and case rules used by
    /// rendering, while returning a stable non-destructive diagnostic.
    pub fn alias_resolution_report(
        &self,
        lyric: impl Into<String>,
        pitch: impl Into<String>,
    ) -> AliasResolutionReport {
        let requested_alias = lyric.into();
        let pitch = pitch.into();
        match self.find_mapped_entry(&requested_alias, &pitch) {
            Some(entry) => AliasResolutionReport {
                requested_alias,
                pitch,
                resolved: true,
                resolved_alias: Some(entry.alias.clone()),
                wav_filename: Some(entry.wav_filename.clone()),
                offset_ms: Some(entry.offset),
                consonant_ms: Some(entry.consonant),
                preutterance_ms: Some(entry.preutterance),
                overlap_ms: Some(entry.overlap),
                reason: None,
            },
            None => AliasResolutionReport {
                requested_alias,
                pitch,
                resolved: false,
                resolved_alias: None,
                wav_filename: None,
                offset_ms: None,
                consonant_ms: None,
                preutterance_ms: None,
                overlap_ms: None,
                reason: Some("alias ausente ou sem entrada compatível no oto.ini".to_string()),
            },
        }
    }

    pub fn find_entry(&self, lyric: &str, pitch_name: &str) -> Option<&OtoEntry> {
        let lyric_trimmed = lyric.trim();
        if lyric_trimmed.is_empty() {
            return None;
        }
        let (prefix, suffix) = self
            .prefix_map
            .get_prefix_suffix(pitch_name)
            .unwrap_or(("", ""));

        let clean_p = prefix.trim_matches('"').trim_matches('\'');
        let clean_s = suffix.trim_matches('"').trim_matches('\'');

        let lyric_cands = crate::phonemizer::romaji::lyric_candidates(lyric_trimmed);

        let mut candidates = Vec::new();

        for cand_lyric in &lyric_cands {
            let raw_alias = self.prefix_map.get_alias(cand_lyric, pitch_name);
            candidates.push(raw_alias);

            if !clean_p.is_empty() || !clean_s.is_empty() {
                candidates.push(format!("{}{}{}", clean_p, cand_lyric, clean_s));

                if !clean_p.is_empty() {
                    let p_trimmed = clean_p.trim();
                    candidates.push(format!("{} {}{}", p_trimmed, cand_lyric, clean_s));
                    candidates.push(format!("{}_{}{}", p_trimmed, cand_lyric, clean_s));
                    candidates.push(format!("{}{}{}", p_trimmed, cand_lyric, clean_s));
                }

                if !clean_s.is_empty() {
                    let s_trimmed = clean_s.trim();
                    candidates.push(format!("{}{}_{}", clean_p, cand_lyric, s_trimmed));
                    candidates.push(format!("{}{}{}", clean_p, cand_lyric, s_trimmed));
                }
            }

            candidates.push(cand_lyric.clone());

            if !pitch_name.is_empty() {
                candidates.push(format!("{}_{}", cand_lyric, pitch_name));
                candidates.push(format!("{}_{}", pitch_name, cand_lyric));
                candidates.push(format!("{} {}", pitch_name, cand_lyric));
                candidates.push(format!("{} {}", cand_lyric, pitch_name));
                candidates.push(format!("{}{}", pitch_name, cand_lyric));
            }
        }

        for cand in &candidates {
            if let Some(entry) = self.entries.get(cand) {
                return Some(entry);
            }
        }

        if let Some(entry) = self.entries.get(lyric_trimmed) {
            return Some(entry);
        }

        // Try case-insensitive match using a single, lazily-built index. The
        // previous nested scan was O(candidates × aliases) for every note.
        let case_insensitive_entries = self.case_insensitive_entries.get_or_init(|| {
            self.entries
                .keys()
                .map(|alias| (alias.to_lowercase(), alias.clone()))
                .collect()
        });
        for cand in &candidates {
            if let Some(original_alias) = case_insensitive_entries.get(&cand.to_lowercase()) {
                if let Some(entry) = self.entries.get(original_alias) {
                    return Some(entry);
                }
            }
        }

        if let Some(target_midi) = crate::dsp::pitch::note_name_to_midi(pitch_name) {
            let mut alt_pitches: Vec<(&str, i32)> = self
                .prefix_map
                .mapped_pitches()
                .filter(|&p| p != pitch_name)
                .filter_map(|p| {
                    crate::dsp::pitch::note_name_to_midi(p)
                        .map(|m| (p, (m as i32 - target_midi as i32).abs()))
                })
                .collect();

            alt_pitches.sort_by_key(|&(_, dist)| dist);

            for (alt_pitch, _) in alt_pitches {
                if let Some((pfx, sfx)) = self.prefix_map.get_prefix_suffix(alt_pitch) {
                    let cand = format!("{}{}{}", pfx.trim(), lyric_trimmed, sfx.trim());
                    if let Some(entry) = self.entries.get(&cand) {
                        return Some(entry);
                    }
                }
            }
        }

        if let Some(space_idx) = lyric_trimmed.find(' ') {
            let cv_part = lyric_trimmed[space_idx + 1..].trim();
            if !cv_part.is_empty() && cv_part != lyric_trimmed {
                return self.find_entry(cv_part, pitch_name);
            }
        }

        None
    }
}
