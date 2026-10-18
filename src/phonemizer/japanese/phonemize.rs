use super::*;

impl JapanesePhonemizer {
    pub fn extract_vowel(lyric: &str) -> Option<&'static str> {
        let trimmed = lyric.trim();
        if trimmed.is_empty() {
            return None;
        }

        // `・` is OpenUtau's glottal-stop marker, never a vowel nucleus.
        if trimmed == "・" {
            return None;
        }

        // The sokuon (small tsu) is a consonant-closure/pause marker.  It
        // has no vowel of its own.  Letting it fall through to the generic
        // kana/romaji lookup can make it inherit `u` from `つ` in some
        // converted aliases, which then corrupts VCV/CVVC transitions.
        if trimmed == "っ" || trimmed == "ッ" {
            return None;
        }

        // A long-vowel mark carries the vowel of the preceding mora.  This
        // is also the basis for Presamp's LONG_V alias expansion.
        if let Some(stem) = trimmed.strip_suffix('ー') {
            if !stem.is_empty() {
                return Self::extract_vowel(stem);
            }
        }

        if let Some(&v) = VOWEL_LOOKUP.get(trimmed) {
            return Some(v);
        }

        let last_char = trimmed.chars().last()?.to_string();
        if let Some(&v) = VOWEL_LOOKUP.get(&last_char) {
            return Some(v);
        }

        let romaji_str = romaji::hiragana_to_romaji(trimmed);
        if let Some(&v) = VOWEL_LOOKUP.get(&romaji_str) {
            return Some(v);
        }
        let romaji_last = romaji_str.chars().last()?.to_string();
        VOWEL_LOOKUP.get(&romaji_last).copied()
    }

    pub fn extract_consonant(lyric: &str) -> Option<&'static str> {
        let trimmed = lyric.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Keep sokuon out of vowel-to-consonant transition inference.  A
        // voicebank may still provide an explicit `っ`/closure alias, but
        // the marker itself must never be interpreted as the vowel `u`.
        if trimmed == "っ" || trimmed == "ッ" {
            return Some("cl");
        }

        if let Some(&c) = CONSONANT_LOOKUP.get(trimmed) {
            return Some(c);
        }

        let chars: Vec<char> = trimmed.chars().collect();
        if chars.len() >= 2 {
            let prefix2: String = chars[0..2].iter().collect();
            if let Some(&c) = CONSONANT_LOOKUP.get(&prefix2) {
                return Some(c);
            }
        }
        if !chars.is_empty() {
            let prefix1: String = chars[0..1].iter().collect();
            if let Some(&c) = CONSONANT_LOOKUP.get(&prefix1) {
                return Some(c);
            }
        }

        let romaji_str = romaji::hiragana_to_romaji(trimmed);
        if let Some(&c) = CONSONANT_LOOKUP.get(&romaji_str) {
            return Some(c);
        }
        let r_chars: Vec<char> = romaji_str.chars().collect();
        if r_chars.len() >= 2 {
            let r_prefix2: String = r_chars[0..2].iter().collect();
            if let Some(&c) = CONSONANT_LOOKUP.get(&r_prefix2) {
                return Some(c);
            }
        }
        if !r_chars.is_empty() {
            let r_prefix1: String = r_chars[0..1].iter().collect();
            if let Some(&c) = CONSONANT_LOOKUP.get(&r_prefix1) {
                return Some(c);
            }
        }

        None
    }

    fn find_oto_candidate(vb: &Voicebank, candidates: &[String], pitch: &str) -> Option<String> {
        for cand in candidates {
            for spelling in romaji::lyric_candidates(cand) {
                if let Some(entry) = vb.find_mapped_entry(&spelling, pitch) {
                    return Some(entry.alias.clone());
                }
            }
        }
        None
    }

    /// Mirrors OpenUtau's CheckOtoUntilHit: a phoneme-scoped alternate is
    /// tried before the ordinary alias, while preserving candidate order.
    fn find_oto_candidate_for_note(
        vb: &Voicebank,
        candidates: &[String],
        note: &UNote,
        phoneme_index: usize,
    ) -> Option<String> {
        let alternate = note
            .phoneme_overrides
            .iter()
            .find(|override_| override_.index == phoneme_index)
            .and_then(|override_| override_.alternate);
        let mut ordered = Vec::with_capacity(candidates.len() * 2);
        for candidate in candidates {
            if let Some(alternate) = alternate {
                ordered.push(format!("{candidate}{alternate}"));
            }
            ordered.push(candidate.clone());
        }
        Self::find_oto_candidate(vb, &ordered, &note.pitch)
    }

    pub fn apply_japanese(
        notes: &[UNote],
        vb: &Voicebank,
        mode: PhonemizerMode,
    ) -> Vec<RenderPhone> {
        let mut phones: Vec<RenderPhone> = Vec::new();
        let mut prev_vowel: Option<String> = None;
        let mut prev_note_end_ms: Option<f64> = None;

        struct NoteRef<'a> {
            note_index: usize,
            lyric: String,
            note: &'a UNote,
            position_ms: f64,
            duration_ms: f64,
        }

        let mut expanded: Vec<NoteRef> = Vec::new();
        let (presamp_replacements, has_presamp_replace_section) =
            load_presamp_replacements(&vb.root_path);
        let has_presamp_ini = vb.root_path.join("presamp.ini").is_file();
        let presamp_alias = load_presamp_alias_config(&vb.root_path);
        let presamp_inventory = load_presamp_inventory(&vb.root_path);
        for (note_index, note) in notes.iter().enumerate() {
            // OpenUtau resolves a phonetic hint before applying VCV/CVVC
            // rules.  The visible lyric remains untouched in the project,
            // while alias lookup uses the explicit hint verbatim.
            let raw_lyric = note
                .phonetic_hint
                .as_deref()
                .unwrap_or(note.lyric.as_str())
                .nfc()
                .collect::<String>();
            let raw_lyric = raw_lyric.trim();
            let raw_lyric = if note.phonetic_hint.is_none() {
                presamp_replacements
                    .get(raw_lyric)
                    .map(String::as_str)
                    .or_else(|| {
                        if !has_presamp_replace_section {
                            default_presamp_replacement(raw_lyric)
                        } else {
                            None
                        }
                    })
                    .unwrap_or(raw_lyric)
            } else {
                raw_lyric
            };
            if presamp_inventory.split && (raw_lyric.contains(';') || raw_lyric.contains(',')) {
                let parts: Vec<&str> = raw_lyric
                    .split([';', ','])
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .collect();
                if !parts.is_empty() {
                    let durations = note.resolved_phoneme_durations(parts.len());
                    let mut sub_pos = note.position_ms;
                    for (i, part) in parts.into_iter().enumerate() {
                        let sub_dur = durations[i];
                        expanded.push(NoteRef {
                            note_index,
                            lyric: part.to_string(),
                            note,
                            position_ms: sub_pos,
                            duration_ms: sub_dur,
                        });
                        sub_pos += sub_dur;
                    }
                    continue;
                }
            }

            expanded.push(NoteRef {
                note_index,
                lyric: raw_lyric.to_string(),
                note,
                position_ms: note.position_ms,
                duration_ms: note.duration_ms,
            });
        }

        let len = expanded.len();
        let total_phrase_duration_ms: f64 = expanded.iter().map(|note| note.duration_ms).sum();
        for idx in 0..len {
            let exp = &expanded[idx];
            let note = exp.note;
            let note_index = exp.note_index;

            if exp.lyric.is_empty() || exp.lyric == "R" {
                prev_vowel = None;
                prev_note_end_ms = None;
                continue;
            }

            let cur_vowel = extract_vowel_for_mode(&exp.lyric, &presamp_inventory, mode);
            let _cur_consonant = extract_presamp_consonant(&exp.lyric, &presamp_inventory);
            let current_core = presamp_core_lyric(&exp.lyric, &presamp_inventory);

            let is_phrase_start = match prev_note_end_ms {
                Some(end_ms) => exp.position_ms > end_ms + 0.001,
                None => true,
            };

            if is_phrase_start {
                prev_vowel = None;
            }

            // OpenUtau treats `+`, `+n` and `+~` forms as extender notes.
            // They prolong the preceding phoneme instead of becoming a new
            // alias in the phonemizer.
            if exp.lyric.starts_with('+') && !is_phrase_start {
                if let Some(last) = phones.last_mut() {
                    last.duration_ms += exp.duration_ms;
                    prev_note_end_ms = Some(exp.position_ms + exp.duration_ms);
                    continue;
                }
            }

            // OpenUtau's Japanese presamp phonemizer treats an isolated
            // sokuon as a VC/closure phoneme, not as an ordinary CV note.
            // Prefer the explicit closure aliases and then the following
            // consonant (for example `a っ`, `a cl`, `a k`, `っ`).  This is
            // what creates the short stop before the following syllable and
            // prevents `っ` from being rendered as `u`.
            if (mode == PhonemizerMode::CVVC || has_presamp_ini)
                && (exp.lyric == "っ" || exp.lyric == "ッ")
            {
                let mut sokuon_candidates = Vec::new();
                if let Some(vowel) = prev_vowel.as_deref() {
                    if mode == PhonemizerMode::VCV {
                        sokuon_candidates.push(presamp_join(vowel, &presamp_alias.vcvpad, "っ"));
                    }
                    sokuon_candidates.push(presamp_join(vowel, &presamp_alias.vcpad, "っ"));
                    if mode == PhonemizerMode::VCV {
                        sokuon_candidates.push(presamp_join(vowel, &presamp_alias.vcvpad, "cl"));
                    }
                    sokuon_candidates.push(presamp_join(vowel, &presamp_alias.vcpad, "cl"));
                    if let Some(next_exp) = expanded.get(idx + 1) {
                        if let Some(consonant) =
                            extract_presamp_consonant(&next_exp.lyric, &presamp_inventory)
                        {
                            if consonant != "cl" {
                                sokuon_candidates.push(presamp_join(
                                    vowel,
                                    &presamp_alias.vcpad,
                                    &consonant,
                                ));
                            }
                        }
                    }
                }
                sokuon_candidates.push("っ".to_string());
                sokuon_candidates.push("cl".to_string());

                if let Some(alias) =
                    Self::find_oto_candidate_for_note(vb, &sokuon_candidates, note, 0)
                {
                    phones.push(RenderPhone {
                        note_index,
                        lyric: alias,
                        pitch: note.pitch.clone(),
                        position_ms: exp.position_ms,
                        duration_ms: exp.duration_ms,
                        envelope: note.envelope.clone(),
                        expressions: note.expressions.clone(),
                        pitch_bend: note.pitch_bend.clone(),
                        vibrato: note.vibrato.clone(),
                        flags: note.flags.clone(),
                        resolved_alias: None,
                        resolved_timbre: None,
                    });
                    prev_vowel = None;
                    prev_note_end_ms = Some(exp.position_ms + exp.duration_ms);
                    continue;
                }
            }

            // The legacy Japanese VCV/CV phonemizers treat an explicit
            // phonetic hint as an already-resolved alias. Do not prepend a
            // phrase vowel or replace it with a beginning-CV candidate.
            if note.phonetic_hint.is_some()
                && matches!(mode, PhonemizerMode::BasicCV | PhonemizerMode::VCV)
            {
                let alias = Self::find_oto_candidate_for_note(
                    vb,
                    std::slice::from_ref(&exp.lyric),
                    note,
                    0,
                )
                .unwrap_or_else(|| exp.lyric.to_string());
                phones.push(RenderPhone {
                    note_index,
                    lyric: alias,
                    pitch: note.pitch.clone(),
                    position_ms: exp.position_ms,
                    duration_ms: exp.duration_ms,
                    envelope: note.envelope.clone(),
                    expressions: note.expressions.clone(),
                    pitch_bend: note.pitch_bend.clone(),
                    vibrato: note.vibrato.clone(),
                    flags: note.flags.clone(),
                    resolved_alias: None,
                    resolved_timbre: None,
                });
                prev_vowel = cur_vowel;
                prev_note_end_ms = Some(exp.position_ms + exp.duration_ms);
                continue;
            }

            match mode {
                PhonemizerMode::BasicCV => {
                    // Match OpenUtau's legacy Japanese VCV search contract.
                    // The plain CV is deliberately a fallback: when a bank
                    // contains a VCV recording, selecting it first avoids a
                    // hard consonant reset between adjacent notes.
                    let mut candidates = Vec::new();
                    if is_phrase_start {
                        candidates.extend([format!("- {}", exp.lyric), exp.lyric.to_string()]);
                    } else if let Some(pv) = prev_vowel.as_deref() {
                        candidates.extend([
                            presamp_join(pv, &presamp_alias.vcvpad, &exp.lyric),
                            presamp_join("*", &presamp_alias.vcvpad, &exp.lyric),
                            exp.lyric.to_string(),
                            format!("- {}", exp.lyric),
                        ]);
                    } else {
                        candidates.extend([exp.lyric.to_string(), format!("- {}", exp.lyric)]);
                    }

                    // Preserve Presamp-specific aliases after the canonical
                    // OpenUtau candidates. This supports banks whose aliases
                    // use a configured pad or long-vowel convention without
                    // changing the standard VCV precedence above.
                    let mut alias_candidates = expand_presamp_candidates(
                        if is_phrase_start {
                            &presamp_alias.beginning_cv
                        } else {
                            &presamp_alias.vcv
                        },
                        prev_vowel.as_deref().unwrap_or(""),
                        extract_presamp_consonant(&exp.lyric, &presamp_inventory)
                            .as_deref()
                            .unwrap_or(""),
                        &exp.lyric,
                        &exp.lyric,
                        &presamp_alias,
                    );
                    if let Some(long_alias) = long_vowel_candidate(&exp.lyric, &presamp_alias) {
                        alias_candidates.push(long_alias);
                    }
                    candidates.extend(alias_candidates);

                    let alias = Self::find_oto_candidate_for_note(vb, &candidates, note, 0)
                        .unwrap_or_else(|| exp.lyric.to_string());

                    phones.push(RenderPhone {
                        note_index,
                        lyric: alias,
                        pitch: note.pitch.clone(),
                        position_ms: exp.position_ms,
                        duration_ms: exp.duration_ms,
                        envelope: note.envelope.clone(),
                        expressions: note.expressions.clone(),
                        pitch_bend: note.pitch_bend.clone(),
                        vibrato: note.vibrato.clone(),
                        flags: note.flags.clone(),
                        resolved_alias: None,
                        resolved_timbre: None,
                    });
                }

                PhonemizerMode::VCV => {
                    let mut alias = exp.lyric.to_string();
                    if !is_phrase_start {
                        if let Some(pv) = prev_vowel.as_deref() {
                            let mut vcv_group = expand_presamp_candidates(
                                &presamp_alias.vcv,
                                pv,
                                extract_presamp_consonant(&exp.lyric, &presamp_inventory)
                                    .as_deref()
                                    .unwrap_or(""),
                                &exp.lyric,
                                &exp.lyric,
                                &presamp_alias,
                            );
                            if let Some(long_alias) =
                                long_vowel_candidate(&exp.lyric, &presamp_alias)
                            {
                                vcv_group.insert(0, long_alias);
                            }
                            let cross_group = expand_presamp_candidates(
                                &presamp_alias.cross_cv,
                                pv,
                                extract_presamp_consonant(&exp.lyric, &presamp_inventory)
                                    .as_deref()
                                    .unwrap_or(""),
                                &exp.lyric,
                                &exp.lyric,
                                &presamp_alias,
                            );
                            let cv_group = vec![
                                format!("{}_{}", pv, exp.lyric),
                                format!("{}{}", pv, exp.lyric),
                                presamp_join("*", &presamp_alias.vcvpad, &exp.lyric),
                                exp.lyric.to_string(),
                            ];
                            let vcv_cands = order_presamp_candidate_groups(
                                presamp_priority_for_lyric(&exp.lyric, &presamp_inventory),
                                vec![
                                    ("VCV", vcv_group),
                                    ("CROSS_CV", cross_group),
                                    ("CV", cv_group),
                                ],
                            );
                            let mut vcv_cands = vcv_cands;
                            add_presamp_suffix_variants(
                                &mut vcv_cands,
                                &exp.lyric,
                                &presamp_inventory,
                            );
                            if let Some(found) =
                                Self::find_oto_candidate_for_note(vb, &vcv_cands, note, 0)
                            {
                                alias = found;
                            }
                        }
                    }

                    if alias == exp.lyric {
                        let mut head_cands = expand_presamp_candidates(
                            &presamp_alias.beginning_cv,
                            "",
                            extract_presamp_consonant(&exp.lyric, &presamp_inventory)
                                .as_deref()
                                .unwrap_or(""),
                            &exp.lyric,
                            &exp.lyric,
                            &presamp_alias,
                        );
                        head_cands.extend([format!("- {}", exp.lyric), exp.lyric.to_string()]);
                        if let Some(found) =
                            Self::find_oto_candidate_for_note(vb, &head_cands, note, 0)
                        {
                            alias = found;
                        }
                    }

                    phones.push(RenderPhone {
                        note_index,
                        lyric: alias,
                        pitch: note.pitch.clone(),
                        position_ms: exp.position_ms,
                        duration_ms: exp.duration_ms,
                        envelope: note.envelope.clone(),
                        expressions: note.expressions.clone(),
                        pitch_bend: note.pitch_bend.clone(),
                        vibrato: note.vibrato.clone(),
                        flags: note.flags.clone(),
                        resolved_alias: None,
                        resolved_timbre: None,
                    });
                }

                PhonemizerMode::CVVC => {
                    // CVVC distinguishes vowel nuclei from consonant onsets.
                    // Treating every known consonant as a "plain vowel" made
                    // the renderer search for V-V aliases (for example
                    // `a k`) instead of the required CV alias (`ka`).
                    // JapaneseCVVCPhonemizer's `plainVowels` contains only
                    // standalone vowel lyrics. Presamp's [VOWEL] table also
                    // lists CV syllables such as `か` and `さ`; using that
                    // inventory as a plain-vowel test misclassifies almost
                    // every consonant note and skips the CVVC path.
                    let is_plain_vowel = PLAIN_VOWELS.contains(&current_core.as_str());
                    let is_priority = presamp_inventory.priority.contains(&current_core);
                    let mut current_lyric = exp.lyric.to_string();

                    if is_phrase_start {
                        let mut initial_cands = expand_presamp_candidates(
                            &presamp_alias.beginning_cv,
                            "",
                            extract_presamp_consonant(&exp.lyric, &presamp_inventory)
                                .as_deref()
                                .unwrap_or(""),
                            &exp.lyric,
                            &exp.lyric,
                            &presamp_alias,
                        );
                        if let Some(long_alias) = long_vowel_candidate(&exp.lyric, &presamp_alias) {
                            initial_cands.insert(0, long_alias);
                        }
                        initial_cands.extend([format!("- {}", exp.lyric), exp.lyric.to_string()]);
                        add_presamp_suffix_variants(
                            &mut initial_cands,
                            &exp.lyric,
                            &presamp_inventory,
                        );
                        if let Some(found) =
                            Self::find_oto_candidate_for_note(vb, &initial_cands, note, 0)
                        {
                            current_lyric = found;
                        }
                    } else if current_core.contains('・') {
                        let mut glottal_cands = Vec::new();
                        if let Some(pv) = prev_vowel.as_deref() {
                            glottal_cands.push(presamp_join(pv, &presamp_alias.vcvpad, &exp.lyric));
                            glottal_cands.push(presamp_join(pv, &presamp_alias.vcpad, &exp.lyric));
                        }
                        if let Some(vowel) = exp
                            .lyric
                            .chars()
                            .find(|character| "あいうえおんン".contains(*character))
                        {
                            // Same fallback order as OpenUtau's
                            // `glottalCVtests`: banks often record the
                            // glottal onset using the isolated vowel rather
                            // than the full `あ・` lyric.
                            glottal_cands.extend([
                                format!("・{}{}", presamp_alias.vcpad, vowel),
                                format!("・{}", vowel),
                                format!("{}・", vowel),
                                format!("-{}{}・", presamp_alias.vcvpad, vowel),
                                format!("-{}{}", presamp_alias.vcvpad, vowel),
                            ]);
                        }
                        glottal_cands.extend([
                            format!("・{}{}", presamp_alias.vcpad, exp.lyric),
                            format!("・{}{}", presamp_alias.vcvpad, exp.lyric),
                            exp.lyric.to_string(),
                        ]);
                        if let Some(found) =
                            Self::find_oto_candidate_for_note(vb, &glottal_cands, note, 0)
                        {
                            current_lyric = found;
                        }
                    } else if is_priority {
                        let mut priority_cands = vec![exp.lyric.to_string()];
                        priority_cands.extend(expand_presamp_candidates(
                            &presamp_alias.beginning_cv,
                            "",
                            extract_presamp_consonant(&exp.lyric, &presamp_inventory)
                                .as_deref()
                                .unwrap_or(""),
                            &exp.lyric,
                            &exp.lyric,
                            &presamp_alias,
                        ));
                        if let Some(found) =
                            Self::find_oto_candidate_for_note(vb, &priority_cands, note, 0)
                        {
                            current_lyric = found;
                        }
                    } else if is_plain_vowel {
                        if let Some(pv) = prev_vowel.as_deref() {
                            let mut vcv_group = expand_presamp_candidates(
                                &presamp_alias.vcv,
                                pv,
                                extract_presamp_consonant(&exp.lyric, &presamp_inventory)
                                    .as_deref()
                                    .unwrap_or(""),
                                &exp.lyric,
                                &exp.lyric,
                                &presamp_alias,
                            );
                            if let Some(long_alias) =
                                long_vowel_candidate(&exp.lyric, &presamp_alias)
                            {
                                vcv_group.insert(0, long_alias);
                            }
                            let cv_group = vec![
                                presamp_join("*", &presamp_alias.vcvpad, &exp.lyric),
                                format!("_{}", exp.lyric),
                                exp.lyric.to_string(),
                            ];
                            let mut vow_cands = order_presamp_candidate_groups(
                                presamp_priority_for_lyric(&exp.lyric, &presamp_inventory),
                                vec![("VCV", vcv_group), ("CV", cv_group)],
                            );
                            add_presamp_suffix_variants(
                                &mut vow_cands,
                                &exp.lyric,
                                &presamp_inventory,
                            );
                            if let Some(found) =
                                Self::find_oto_candidate_for_note(vb, &vow_cands, note, 0)
                            {
                                current_lyric = found;
                            }
                        }
                    } else {
                        // A CVVC phrase already receives its V-C transition at
                        // the end of the preceding note. Selecting a VCV alias
                        // here repeats that same transition (VC + VCV), which
                        // sounds like a stuck or doubled consonant in hybrid
                        // banks. Match OpenUtau's Japanese CVVC contract: use
                        // the continuous-form CV alias first, then plain CV.
                        // JapaneseCVVCPhonemizer (legacy) has a fixed
                        // precedence here: `* CV`, then plain `CV`.  The
                        // Presamp alias-priority tables must not reorder
                        // this branch, or a custom ini can replace the
                        // continuous CV attack with an incompatible alias.
                        let mut cv_cands = expand_presamp_candidates(
                            &presamp_alias.cross_cv,
                            prev_vowel.as_deref().unwrap_or(""),
                            extract_presamp_consonant(&exp.lyric, &presamp_inventory)
                                .as_deref()
                                .unwrap_or(""),
                            &exp.lyric,
                            &exp.lyric,
                            &presamp_alias,
                        );
                        cv_cands.extend([
                            presamp_join("*", &presamp_alias.vcvpad, &exp.lyric),
                            exp.lyric.to_string(),
                        ]);
                        add_presamp_suffix_variants(&mut cv_cands, &exp.lyric, &presamp_inventory);
                        if let Some(found) =
                            Self::find_oto_candidate_for_note(vb, &cv_cands, note, 0)
                        {
                            current_lyric = found;
                        }
                    }

                    let mut pre_c_to_insert: Option<(String, f64)> = None;
                    if is_phrase_start
                        && !current_lyric.contains(&presamp_alias.vcvpad)
                        && !is_priority
                    {
                        if let Some(consonant) =
                            extract_presamp_consonant(&exp.lyric, &presamp_inventory)
                        {
                            let pre_c_candidates = vec![
                                presamp_join("-", &presamp_alias.vcvpad, &consonant),
                                format!("- {}", consonant),
                            ];
                            if let Some(alias) =
                                Self::find_oto_candidate_for_note(vb, &pre_c_candidates, note, 2)
                            {
                                let authored_len = vb
                                    .find_mapped_entry(&current_lyric, &note.pitch)
                                    .map(|oto| {
                                        if oto.overlap < 0.0 {
                                            oto.preutterance - oto.overlap
                                        } else {
                                            oto.preutterance
                                        }
                                    })
                                    .unwrap_or(30.0);
                                let pre_c_len = authored_len
                                    .max(30.0)
                                    .min((exp.duration_ms * 0.5).max(10.0));
                                pre_c_to_insert = Some((alias, pre_c_len));
                            }
                        }
                    }

                    let next_note_ref = expanded.get(idx + 1);
                    let mut vc_to_insert: Option<(String, f64, bool)> = None;

                    if let Some(next_exp) = next_note_ref {
                        let next_gap = next_exp.position_ms - (exp.position_ms + exp.duration_ms);
                        let is_next_adjacent = next_gap <= 0.001;

                        let next_already_has_transition =
                            next_exp.lyric.contains(&presamp_alias.vcvpad)
                                || next_exp.lyric.contains(&presamp_alias.vcpad);
                        let next_core = presamp_core_lyric(&next_exp.lyric, &presamp_inventory);
                        let next_is_plain_vowel = PLAIN_VOWELS.contains(&next_core.as_str());
                        let short_note_without_forced_vc =
                            total_phrase_duration_ms < 100.0 && !presamp_inventory.must_vc;

                        // OpenUtau's JapaneseCVVC phonemizer leaves a
                        // phonetic-hint note untouched: the author supplied
                        // its alias explicitly, so the preceding note must
                        // not synthesize a second VC for it.
                        let next_has_phonetic_hint = next_exp.note.phonetic_hint.is_some();
                        // OpenUtau/Presamp first probes the next note itself.
                        // If that note already resolves to a VCV or VC alias,
                        // its transition owns the hand-off and the current
                        // note must not receive a second VC fragment. Looking
                        // only at the authored lyric misses banks where the
                        // transition is supplied by a fallback alias.
                        let next_has_resolved_transition =
                            if has_presamp_ini && !next_is_plain_vowel && !next_has_phonetic_hint {
                                if let Some(vow) = cur_vowel.as_deref() {
                                    let next_transition_candidates = vec![
                                        presamp_join(vow, &presamp_alias.vcvpad, &next_exp.lyric),
                                        presamp_join(vow, &presamp_alias.vcpad, &next_exp.lyric),
                                        next_exp.lyric.to_string(),
                                    ];
                                    Self::find_oto_candidate(
                                        vb,
                                        &next_transition_candidates,
                                        &next_exp.note.pitch,
                                    )
                                    .is_some_and(|alias| {
                                        alias.contains(&presamp_alias.vcvpad)
                                            || alias.contains(&presamp_alias.vcpad)
                                    })
                                } else {
                                    false
                                }
                            } else {
                                false
                            };

                        if is_next_adjacent
                            && !short_note_without_forced_vc
                            && !next_already_has_transition
                            && !next_has_resolved_transition
                            && !next_is_plain_vowel
                            && !next_has_phonetic_hint
                        {
                            if let (Some(vow), Some(con)) = (
                                cur_vowel.as_deref(),
                                extract_presamp_consonant(&next_exp.lyric, &presamp_inventory),
                            ) {
                                let mut vc_candidates = expand_presamp_candidates(
                                    &presamp_alias.vc,
                                    vow,
                                    &con,
                                    &next_exp.lyric,
                                    &next_exp.lyric,
                                    &presamp_alias,
                                );
                                vc_candidates.extend([
                                    format!("{}{}", vow, con),
                                    format!("{}_{}", vow, con),
                                ]);

                                if let Some(&sub_con) = SUBSTITUTE_LOOKUP.get(con.as_str()) {
                                    vc_candidates.extend(expand_presamp_candidates(
                                        &presamp_alias.vc,
                                        vow,
                                        sub_con,
                                        &next_exp.lyric,
                                        &next_exp.lyric,
                                        &presamp_alias,
                                    ));
                                    vc_candidates.push(format!("{}{}", vow, sub_con));
                                }

                                add_presamp_suffix_variants(
                                    &mut vc_candidates,
                                    &next_exp.lyric,
                                    &presamp_inventory,
                                );

                                if let Some(vc_alias) =
                                    Self::find_oto_candidate_for_note(vb, &vc_candidates, note, 1)
                                {
                                    let mut vc_length_ms = 80.0;
                                    // The transition length is derived from the
                                    // ordinary next CV oto, never its phrase-head
                                    // `- CV` recording. Head aliases commonly
                                    // have a much larger preutterance and used to
                                    // consume half of the preceding vowel here.
                                    let next_cands = vec![next_exp.lyric.to_string()];
                                    let oto_entry = Self::find_oto_candidate(
                                        vb,
                                        &next_cands,
                                        &next_exp.note.pitch,
                                    )
                                    .and_then(|alias| {
                                        vb.find_mapped_entry(&alias, &next_exp.note.pitch)
                                    });

                                    if let Some(oto) = oto_entry {
                                        if oto.overlap < 0.0 {
                                            vc_length_ms =
                                                (oto.preutterance - oto.overlap).max(10.0);
                                        } else {
                                            vc_length_ms = oto.preutterance.max(10.0);
                                        }
                                    }
                                    // OpenUtau's [VCLENGTH] switch is global
                                    // in Presamp: 0 derives VC length from the
                                    // next CV oto; 1 keeps the classic fixed
                                    // VC duration. It never uses the physical
                                    // WAV duration as timing.
                                    if !presamp_inventory.vc_length_from_cv {
                                        vc_length_ms = 120.0;
                                    }
                                    let vel_scale = consonant_velocity_time_scale(
                                        next_exp.note.expressions.consonant_velocity,
                                    );
                                    let vc_len = (vc_length_ms * vel_scale)
                                        .min(exp.duration_ms * 0.5)
                                        .min((exp.duration_ms - 20.0).max(10.0));
                                    vc_to_insert = Some((
                                        vc_alias,
                                        vc_len,
                                        presamp_inventory.no_crossfade.contains(&con),
                                    ));
                                }
                            }
                        }
                    }

                    if let Some((vc_alias, vc_len, vc_no_crossfade)) = vc_to_insert {
                        let requested_pre_c_len =
                            pre_c_to_insert.as_ref().map(|(_, len)| *len).unwrap_or(0.0);
                        let pre_c_len =
                            requested_pre_c_len.min((exp.duration_ms - vc_len - 10.0).max(0.0));
                        let main_duration = (exp.duration_ms - vc_len - pre_c_len).max(10.0);
                        if let Some((pre_c_alias, _)) = pre_c_to_insert {
                            if pre_c_len > 0.0 {
                                let mut pre_c_expressions = note.expressions.clone();
                                if presamp_inventory.cflags.trim() == "p0" {
                                    pre_c_expressions.presamp_normalization = Some(0.0);
                                }
                                phones.push(RenderPhone {
                                    note_index,
                                    lyric: pre_c_alias,
                                    pitch: note.pitch.clone(),
                                    position_ms: exp.position_ms,
                                    duration_ms: pre_c_len,
                                    envelope: note.envelope.clone(),
                                    expressions: pre_c_expressions,
                                    pitch_bend: note.pitch_bend.clone(),
                                    vibrato: note.vibrato.clone(),
                                    flags: note.flags.clone(),
                                    resolved_alias: None,
                                    resolved_timbre: None,
                                });
                            }
                        }
                        phones.push(RenderPhone {
                            note_index,
                            lyric: current_lyric,
                            pitch: note.pitch.clone(),
                            position_ms: exp.position_ms + pre_c_len,
                            duration_ms: main_duration,
                            envelope: note.envelope.clone(),
                            expressions: note.expressions.clone(),
                            pitch_bend: note.pitch_bend.clone(),
                            vibrato: note.vibrato.clone(),
                            flags: note.flags.clone(),
                            resolved_alias: None,
                            resolved_timbre: None,
                        });

                        let vc_envelope = note.envelope.clone();
                        let mut vc_expressions = note.expressions.clone();
                        vc_expressions.presamp_no_crossfade = vc_no_crossfade;

                        phones.push(RenderPhone {
                            note_index,
                            lyric: vc_alias,
                            pitch: note.pitch.clone(),
                            position_ms: exp.position_ms + pre_c_len + main_duration,
                            duration_ms: vc_len,
                            envelope: vc_envelope,
                            expressions: vc_expressions,
                            pitch_bend: note.pitch_bend.clone(),
                            vibrato: note.vibrato.clone(),
                            flags: note.flags.clone(),
                            resolved_alias: None,
                            resolved_timbre: None,
                        });
                    } else {
                        let requested_pre_c_len =
                            pre_c_to_insert.as_ref().map(|(_, len)| *len).unwrap_or(0.0);
                        let pre_c_len = requested_pre_c_len.min((exp.duration_ms - 10.0).max(0.0));
                        let main_duration = (exp.duration_ms - pre_c_len).max(10.0);
                        if let Some((pre_c_alias, _)) = pre_c_to_insert {
                            if pre_c_len > 0.0 {
                                let mut pre_c_expressions = note.expressions.clone();
                                if presamp_inventory.cflags.trim() == "p0" {
                                    pre_c_expressions.presamp_normalization = Some(0.0);
                                }
                                phones.push(RenderPhone {
                                    note_index,
                                    lyric: pre_c_alias,
                                    pitch: note.pitch.clone(),
                                    position_ms: exp.position_ms,
                                    duration_ms: pre_c_len,
                                    envelope: note.envelope.clone(),
                                    expressions: pre_c_expressions,
                                    pitch_bend: note.pitch_bend.clone(),
                                    vibrato: note.vibrato.clone(),
                                    flags: note.flags.clone(),
                                    resolved_alias: None,
                                    resolved_timbre: None,
                                });
                            }
                        }
                        phones.push(RenderPhone {
                            note_index,
                            lyric: current_lyric,
                            pitch: note.pitch.clone(),
                            position_ms: exp.position_ms + pre_c_len,
                            duration_ms: main_duration,
                            envelope: note.envelope.clone(),
                            expressions: note.expressions.clone(),
                            pitch_bend: note.pitch_bend.clone(),
                            vibrato: note.vibrato.clone(),
                            flags: note.flags.clone(),
                            resolved_alias: None,
                            resolved_timbre: None,
                        });
                    }
                }

                _ => {}
            }

            prev_vowel = cur_vowel;
            prev_note_end_ms = Some(exp.position_ms + exp.duration_ms);
        }

        // Presamp's ending aliases are optional. ENDFLAG=1 adds an ending
        // when the phrase is followed by a rest; ENDFLAG=2 converts the last
        // sounding note even without a following rest. This distinction is
        // part of Presamp's public format and is also how OpenUtau exposes
        // AddEnding.
        if matches!(
            mode,
            PhonemizerMode::BasicCV | PhonemizerMode::VCV | PhonemizerMode::CVVC
        ) && presamp_inventory.add_ending != 0
        {
            for (idx, exp) in expanded.iter().enumerate() {
                let has_ending_rest = expanded.get(idx + 1).is_some_and(|next| {
                    next.lyric == "R"
                        && (next.position_ms - (exp.position_ms + exp.duration_ms)).abs() <= 0.001
                });
                let is_last_sounding_note = presamp_inventory.add_ending == 2
                    && expanded[idx + 1..].iter().all(|future| future.lyric == "R");
                if (presamp_inventory.add_ending == 1 && !has_ending_rest)
                    || (presamp_inventory.add_ending == 2 && !is_last_sounding_note)
                {
                    continue;
                }
                let Some(vowel) = extract_presamp_vowel(&exp.lyric, &presamp_inventory) else {
                    continue;
                };
                let mut candidates = expand_presamp_candidates(
                    &presamp_alias.ending1,
                    &vowel,
                    "",
                    "R",
                    &exp.lyric,
                    &presamp_alias,
                );
                candidates.extend(expand_presamp_candidates(
                    &presamp_alias.ending2,
                    &vowel,
                    "",
                    "R",
                    &exp.lyric,
                    &presamp_alias,
                ));
                add_presamp_suffix_variants(&mut candidates, &exp.lyric, &presamp_inventory);
                let Some(phone_index) = phones
                    .iter()
                    .enumerate()
                    .filter(|(_, phone)| phone.note_index == exp.note_index)
                    .max_by(|(_, left), (_, right)| left.position_ms.total_cmp(&right.position_ms))
                    .map(|(phone_index, _)| phone_index)
                else {
                    continue;
                };
                if let Some(alias) = Self::find_oto_candidate_for_note(vb, &candidates, exp.note, 0)
                {
                    phones[phone_index].lyric = alias;
                }
            }
        }

        phones
    }

    /// Entry point for OpenUtau's `JapanesePresampPhonemizer` contract.
    ///
    /// Presamp selects VCV/CVVC behavior from the voicebank's `presamp.ini`
    /// and per-note phoneme colors.  The shared resolver therefore receives
    /// the VCV-compatible base mode while the presence of `presamp.ini`
    /// enables the Presamp-specific inventory and alias expansion above.
    pub fn apply_presamp(notes: &[UNote], vb: &Voicebank) -> Vec<RenderPhone> {
        Self::apply_japanese(notes, vb, PhonemizerMode::VCV)
    }
}
