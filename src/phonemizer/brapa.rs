use crate::oto::Voicebank;
use crate::phonemizer::RenderPhone;
use crate::project::model::UNote;

pub struct VccvBrapaPhonemizer;

impl VccvBrapaPhonemizer {
    pub const VOWELS: &'static [&'static str] = &[
        "a", "e", "i", "o", "u", "eh", "oh", "an", "en", "in", "on", "un",
    ];

    pub const CONSONANTS: &'static [&'static str] = &[
        "b", "ch", "d", "dj", "f", "g", "h", "k", "l", "lh", "m", "n", "nh", "p", "r", "s", "sh",
        "t", "v", "z", "j", "rh", "rw", "-", "w", "y",
    ];

    pub const BURST_CONSONANTS: &'static [&'static str] =
        &["b", "ch", "d", "dj", "g", "k", "p", "t"];

    pub fn is_vowel(p: &str) -> bool {
        Self::VOWELS.contains(&p)
    }

    pub fn is_consonant(p: &str) -> bool {
        Self::CONSONANTS.contains(&p)
    }

    pub fn get_dictionary_replacements() -> &'static [(&'static str, &'static str)] {
        &[
            ("@", "an"),
            ("A", "an"),
            ("6", "an"),
            ("am", "an"),
            ("6~", "an"),
            ("ã", "an"),
            ("a~", "an"),
            ("a'", "an"),
            ("7", "en"),
            ("e~", "en"),
            ("em", "en"),
            ("ẽ", "en"),
            ("1", "in"),
            ("im", "in"),
            ("i~", "in"),
            ("I", "in"),
            ("ĩ", "in"),
            ("0", "on"),
            ("om", "on"),
            ("o~", "on"),
            ("õ", "on"),
            ("Q", "un"),
            ("um", "un"),
            ("u~", "un"),
            ("U", "un"),
            ("ũ", "un"),
            ("3", "eh"),
            ("X", "eh"),
            ("é", "eh"),
            ("E", "eh"),
            ("e'", "eh"),
            ("9", "oh"),
            ("O", "oh"),
            ("ó", "oh"),
            ("'o", "oh"),
            ("o'", "oh"),
            ("qu", "k"),
            ("q", "k"),
            ("c", "k"),
            ("rr", "h"),
            ("x", "h"),
            ("tch", "ch"),
            ("tS", "ch"),
            ("T", "ch"),
            ("zh", "j"),
            ("jh", "j"),
            ("Z", "j"),
            ("Ch", "sh"),
            ("S", "sh"),
            ("ñ", "nh"),
            ("J", "nh"),
            ("4", "r"),
            ("ç", "s"),
            ("ss", "s"),
        ]
    }

    pub fn query_phonemes(text: &str) -> Vec<String> {
        let clean = text.trim();
        if clean.is_empty() {
            return Vec::new();
        }

        if clean.contains(' ') {
            return clean
                .split_whitespace()
                .map(|s| Self::replace_single_symbol(s))
                .collect();
        }

        let replacements = Self::get_dictionary_replacements();
        let mut symbols: Vec<String> = Self::VOWELS
            .iter()
            .chain(Self::CONSONANTS.iter())
            .map(|s| s.to_string())
            .collect();
        for (k, _) in replacements {
            symbols.push(k.to_string());
        }
        symbols.sort_by_key(|s| std::cmp::Reverse(s.len()));

        let mut list = Vec::new();
        let mut remaining = clean.to_string();

        while !remaining.is_empty() {
            let mut matched = false;
            for sym in &symbols {
                if remaining.starts_with(sym) {
                    let canon = Self::replace_single_symbol(sym);
                    list.push(canon);
                    remaining = remaining[sym.len()..].to_string();
                    matched = true;
                    break;
                }
            }
            if !matched {
                let first_char = remaining.chars().next().unwrap();
                let char_len = first_char.len_utf8();
                let mapped = match first_char.to_ascii_lowercase() {
                    'á' | 'â' | 'a' => "a",
                    'é' | 'è' => "eh",
                    'ê' | 'e' => "e",
                    'í' | 'i' => "i",
                    'ó' | 'ò' => "oh",
                    'ô' | 'o' => "o",
                    'ú' | 'u' => "u",
                    'b' => "b",
                    'd' => "d",
                    'f' => "f",
                    'g' => "g",
                    'j' => "j",
                    'k' => "k",
                    'l' => "l",
                    'm' => "m",
                    'n' => "n",
                    'p' => "p",
                    'r' => "r",
                    's' => "s",
                    't' => "t",
                    'v' => "v",
                    'w' => "w",
                    'y' => "y",
                    'z' => "z",
                    _ => "",
                };
                if !mapped.is_empty() {
                    list.push(mapped.to_string());
                }
                remaining = remaining[char_len..].to_string();
            }
        }

        list
    }

    fn replace_single_symbol(sym: &str) -> String {
        for (from, to) in Self::get_dictionary_replacements() {
            if sym == *from {
                return to.to_string();
            }
        }
        sym.to_string()
    }

    pub fn get_transition_basic_length_ms(alias: &str) -> f64 {
        let bursts = ["b", "t", "s", "p", "d", "dj", "f", "k", "g", "z"];
        for b in bursts {
            if alias.contains(&format!(" {b}")) || alias.contains(&format!("{b}-")) {
                return 135.0;
            }
        }

        let clusters = [
            "spr", "str", "skr", "sfr", "spl", "skl", "ks", "zb", "zd", "zg", "zv", "zm", "zn",
            "zl", "zbr", "zdr", "zgr", "ps", "pn", "bj", "bs", "gn", "dv", "tm", "ct", "pt", "mn",
            "ft", "ts", "dz", "kl", "kr",
        ];
        for c in clusters {
            if alias.contains(c) {
                return 150.0;
            }
        }

        if (alias.starts_with("w ")
            || alias.starts_with("y ")
            || alias.starts_with("u ")
            || alias.starts_with("i "))
            && !alias.contains(" w")
            && !alias.contains(" y")
            && !alias.contains("-")
            && !alias.ends_with(" r")
        {
            return 150.0;
        }

        let parts: Vec<&str> = alias.split(' ').collect();
        if parts.len() == 2 && Self::is_consonant(parts[0]) && Self::is_consonant(parts[1]) {
            return 120.0 * 1.15;
        }

        if alias.ends_with(" r") || alias.ends_with(" r-") {
            return 120.0 * 0.23;
        }

        if alias.contains("l-") || alias.ends_with(" l") {
            return 120.0;
        }

        120.0
    }

    pub fn validate_alias(alias: &str) -> String {
        let mut res = alias.to_string();
        for v in Self::VOWELS {
            let target = format!("rh {v}");
            let replacement = format!("r {v}");
            if res.contains(&target) {
                res = res.replace(&target, &replacement);
            }
        }
        res
    }

    pub fn process_syllable(
        prev_v: Option<&str>,
        cc: &[String],
        v: &str,
        is_starting: bool,
        vb: &Voicebank,
        pitch: &str,
    ) -> Vec<String> {
        let mut list = Vec::new();
        let main_cv: Option<String>;

        if is_starting && cc.is_empty() {
            let alias = if v == "w" {
                "- w".to_string()
            } else if v == "y" {
                "- y".to_string()
            } else {
                format!("- {v}")
            };
            main_cv = Some(alias);
        } else if is_starting && cc.len() == 1 {
            let mut text = format!("- {}{v}", cc[0]);
            if vb.find_entry(&text, pitch).is_none() {
                text = format!("-{} {v}", cc[0]);
                if vb.find_entry(&text, pitch).is_none() {
                    text = format!("{} {v}", cc[0]);
                    if vb.find_entry(&text, pitch).is_none() {
                        if v == "w" {
                            let text2 = format!("{} u", cc[0]);
                            if vb.find_entry(&text2, pitch).is_some() {
                                text = text2;
                            }
                        } else if v == "y" {
                            let text3 = format!("{} i", cc[0]);
                            if vb.find_entry(&text3, pitch).is_some() {
                                text = text3;
                            }
                        }
                    }
                }
            }
            main_cv = Some(text);
        } else if is_starting && cc.len() > 1 {
            let mut num = 0;
            while num < cc.len() - 1 {
                if num + 2 < cc.len() {
                    let text4 = format!("{} {} {}", cc[num], cc[num + 1], cc[num + 2]);
                    if vb.find_entry(&text4, pitch).is_some() {
                        list.push(text4);
                        num += 2;
                        continue;
                    }
                }
                let text5 = format!("{} {}", cc[num], cc[num + 1]);
                if vb.find_entry(&text5, pitch).is_some() {
                    list.push(text5);
                } else if cc[num] == "y"
                    && vb
                        .find_entry(&format!("i {}", cc[num + 1]), pitch)
                        .is_some()
                {
                    list.push(format!("i {}", cc[num + 1]));
                } else if cc[num] == "y"
                    && vb
                        .find_entry(&format!("i {}-", cc[num + 1]), pitch)
                        .is_some()
                {
                    list.push(format!("i {}-", cc[num + 1]));
                } else {
                    list.push(text5);
                }
                num += 1;
            }

            let last_c = cc.last().map(|s| s.as_str()).unwrap_or("");
            let mut text = format!("_{last_c} {v}");
            if vb.find_entry(&text, pitch).is_none() {
                text = format!("{last_c} {v}");
                if vb.find_entry(&text, pitch).is_none() {
                    if v == "w" {
                        let text6 = format!("{last_c} u");
                        if vb.find_entry(&text6, pitch).is_some() {
                            text = text6;
                        }
                    } else if v == "y" {
                        let text7 = format!("{last_c} i");
                        if vb.find_entry(&text7, pitch).is_some() {
                            text = text7;
                        }
                    }
                }
            }
            main_cv = Some(text);
        } else if !is_starting && cc.is_empty() {
            if let Some(pv) = prev_v {
                let mut text = format!("{pv} {v}");
                if vb.find_entry(&text, pitch).is_none() {
                    text = format!("_{v}");
                    if vb.find_entry(&text, pitch).is_none() {
                        text = v.to_string();
                    }
                }
                main_cv = Some(text);
            } else {
                main_cv = Some(format!("- {v}"));
            }
        } else {
            let pv = prev_v.unwrap_or("a");
            let first_c = cc[0].as_str();
            let text8 = if first_c == "-" {
                format!("{pv} -")
            } else if first_c == "r" && cc.len() > 1 {
                let rh_try = format!("{pv} rh-");
                if vb.find_entry(&rh_try, pitch).is_some() {
                    rh_try
                } else {
                    format!("{pv} {first_c}")
                }
            } else {
                let try1 = format!("{pv} {first_c}");
                if vb.find_entry(&try1, pitch).is_some() {
                    try1
                } else {
                    let try2 = format!("{pv}{first_c}");
                    if vb.find_entry(&try2, pitch).is_some() {
                        try2
                    } else {
                        format!("{pv} {first_c}-")
                    }
                }
            };
            list.push(text8);

            for i in 0..cc.len().saturating_sub(1) {
                let text10 = cc[i].as_str();
                let text11 = cc[i + 1].as_str();
                if text10 != "-" && text11 != "-" {
                    if text10 != "r"
                        || vb.find_entry(&format!("{pv} {text10}"), pitch).is_some()
                        || vb.find_entry(&format!("{pv} rh-"), pitch).is_none()
                    {
                        let text12 = format!("{text10} {text11}");
                        if vb.find_entry(&text12, pitch).is_some() {
                            list.push(text12);
                        } else if text10 == "w"
                            && vb.find_entry(&format!("u {text11}"), pitch).is_some()
                        {
                            list.push(format!("u {text11}"));
                        } else if text10 == "w"
                            && vb.find_entry(&format!("u {text11}-"), pitch).is_some()
                        {
                            list.push(format!("u {text11}-"));
                        } else if text10 == "y"
                            && vb.find_entry(&format!("i {text11}"), pitch).is_some()
                        {
                            list.push(format!("i {text11}"));
                        } else if text10 == "y"
                            && vb.find_entry(&format!("i {text11}-"), pitch).is_some()
                        {
                            list.push(format!("i {text11}-"));
                        } else {
                            list.push(text12);
                        }
                    }
                }
            }

            if cc.last().map(|s| s.as_str()) == Some("-") {
                main_cv = None;
            } else {
                let mut last_c = cc.last().map(|s| s.as_str()).unwrap_or("");
                if last_c == "w" {
                    last_c = "u";
                } else if last_c == "y" {
                    last_c = "i";
                }

                if cc.len() == 1 || last_c == "`" {
                    main_cv = Some(format!("{last_c} {v}"));
                } else {
                    let try_under = format!("_{last_c} {v}");
                    if vb.find_entry(&try_under, pitch).is_some() {
                        main_cv = Some(try_under);
                    } else {
                        main_cv = Some(format!("{last_c} {v}"));
                    }
                }
            }
        }

        if let Some(cv) = main_cv {
            list.push(cv);
        }

        list.into_iter().map(|a| Self::validate_alias(&a)).collect()
    }

    pub fn process_ending(prev_v: &str, cc: &[String], vb: &Voicebank, pitch: &str) -> Vec<String> {
        if cc.is_empty() {
            return Vec::new();
        }
        let mut list = Vec::new();
        let text = cc[0].as_str();
        let text2 = if text == "-" {
            format!("{prev_v} -")
        } else if text == "r" {
            let try_rh = format!("{prev_v} rh-");
            if vb.find_entry(&try_rh, pitch).is_some() {
                try_rh
            } else {
                format!("{prev_v} r-")
            }
        } else if prev_v == "w" {
            let try_u = format!("u {text}-");
            if vb.find_entry(&try_u, pitch).is_some() {
                try_u
            } else {
                format!("u {text}")
            }
        } else if prev_v == "y" {
            let try_i = format!("i {text}-");
            if vb.find_entry(&try_i, pitch).is_some() {
                try_i
            } else {
                format!("i {text}")
            }
        } else {
            let try_norm = format!("{prev_v} {text}-");
            if vb.find_entry(&try_norm, pitch).is_some() {
                try_norm
            } else {
                format!("{prev_v} {text}")
            }
        };
        list.push(text2);

        for i in 0..cc.len().saturating_sub(1) {
            let text3 = cc[i].as_str();
            let text4 = cc[i + 1].as_str();
            if text3 != "r" || vb.find_entry(&format!("{prev_v} rh-"), pitch).is_none() {
                let text5 = format!("{text3} {text4}-");
                if vb.find_entry(&text5, pitch).is_some() {
                    list.push(text5);
                } else if text3 == "y" && vb.find_entry(&format!("i {text4}-"), pitch).is_some() {
                    list.push(format!("i {text4}-"));
                } else if text3 == "y" && vb.find_entry(&format!("i {text4}"), pitch).is_some() {
                    list.push(format!("i {text4}"));
                } else {
                    list.push(format!("{text3} {text4}"));
                }
            }
        }

        if !cc.is_empty()
            && cc.last().map(|s| s.as_str()) != Some("r")
            && cc.last().map(|s| s.as_str()) != Some("-")
        {
            let last_c = cc.last().unwrap();
            let last_dash = format!("{last_c} -");
            if vb.find_entry(&last_dash, pitch).is_some() {
                list.push(last_dash);
            }
        }

        list.into_iter().map(|a| Self::validate_alias(&a)).collect()
    }

    pub fn apply_phonemizer(notes: &[UNote], vb: &Voicebank) -> Vec<RenderPhone> {
        let mut phones: Vec<RenderPhone> = Vec::new();
        let mut prev_vowel: Option<String> = None;
        let mut prev_note_end_ms: Option<f64> = None;

        for (note_index, note) in notes.iter().enumerate() {
            let lyric_trimmed = note.lyric.trim();
            if lyric_trimmed.is_empty() || lyric_trimmed == "R" || lyric_trimmed == "r" {
                prev_vowel = None;
                prev_note_end_ms = Some(note.position_ms + note.duration_ms);
                continue;
            }

            let is_phrase_start = match prev_note_end_ms {
                Some(end_ms) => note.position_ms > end_ms + 60.0,
                None => true,
            };

            if is_phrase_start {
                prev_vowel = None;
            }

            if (lyric_trimmed == "+" || lyric_trimmed.starts_with("+ ")) && !is_phrase_start {
                if let Some(last_phone) = phones.last_mut() {
                    last_phone.duration_ms += note.duration_ms;
                    prev_note_end_ms = Some(note.position_ms + note.duration_ms);
                    continue;
                }
            }

            let brapa_tokens = Self::query_phonemes(lyric_trimmed);
            if brapa_tokens.is_empty() {
                continue;
            }

            let mut syllables: Vec<(Vec<String>, String)> = Vec::new();
            let mut current_cc = Vec::new();
            let mut i = 0;

            while i < brapa_tokens.len() {
                let token = &brapa_tokens[i];
                if Self::is_vowel(token) {
                    syllables.push((current_cc.clone(), token.clone()));
                    current_cc.clear();
                } else {
                    current_cc.push(token.clone());
                }
                i += 1;
            }

            let trailing_coda = current_cc;

            if syllables.is_empty() {
                syllables.push((trailing_coda.clone(), "a".to_string()));
            }

            let num_syl = syllables.len();
            let syl_dur = note.duration_ms / num_syl as f64;

            let mut note_phones: Vec<RenderPhone> = Vec::new();

            for (syl_idx, (cc, v)) in syllables.iter().enumerate() {
                let syl_is_start = is_phrase_start && syl_idx == 0;
                let syl_start_pos = note.position_ms + (syl_idx as f64 * syl_dur);

                let aliases = Self::process_syllable(
                    prev_vowel.as_deref(),
                    cc,
                    v,
                    syl_is_start,
                    vb,
                    &note.pitch,
                );

                let num_aliases = aliases.len().max(1);
                if num_aliases == 1 {
                    let alias = aliases[0].clone();
                    let final_alias = vb
                        .find_entry(&alias, &note.pitch)
                        .map(|e| e.alias.clone())
                        .unwrap_or(alias);

                    note_phones.push(RenderPhone {
                        note_index,
                        lyric: final_alias,
                        pitch: note.pitch.clone(),
                        position_ms: syl_start_pos,
                        duration_ms: syl_dur,
                        envelope: note.envelope.clone(),
                        expressions: note.expressions.clone(),
                        pitch_bend: note.pitch_bend.clone(),
                        vibrato: note.vibrato.clone(),
                        flags: note.flags.clone(),
                    });
                } else {
                    let mut total_trans_dur = 0.0;
                    let mut trans_durs = Vec::new();

                    for alias in &aliases[..aliases.len() - 1] {
                        let base_len = Self::get_transition_basic_length_ms(alias);
                        let dur = base_len.clamp(25.0, (syl_dur * 0.45).max(30.0));
                        trans_durs.push(dur);
                        total_trans_dur += dur;
                    }

                    let max_trans_allowed = (syl_dur * 0.50).max(30.0);
                    if total_trans_dur > max_trans_allowed && total_trans_dur > 0.0 {
                        let scale = max_trans_allowed / total_trans_dur;
                        for d in &mut trans_durs {
                            *d *= scale;
                        }
                        total_trans_dur = max_trans_allowed;
                    }

                    if syl_idx == 0 {
                        if let Some(last_p) = phones.last_mut() {
                            let borrow = total_trans_dur.min((last_p.duration_ms - 20.0).max(0.0));
                            last_p.duration_ms -= borrow;
                        }

                        let mut trans_pos = syl_start_pos - total_trans_dur;
                        for (alias_idx, alias) in aliases[..aliases.len() - 1].iter().enumerate() {
                            let dur = trans_durs[alias_idx];
                            let final_alias = vb
                                .find_entry(alias, &note.pitch)
                                .map(|e| e.alias.clone())
                                .unwrap_or_else(|| alias.clone());

                            let mut trans_env = crate::dsp::envelope::UtauEnvelope::default();
                            trans_env.p4 = 0.0;
                            trans_env.p5 = 0.0;
                            trans_env.v4 = 100.0;
                            trans_env.v5 = 100.0;

                            note_phones.push(RenderPhone {
                                note_index,
                                lyric: final_alias,
                                pitch: note.pitch.clone(),
                                position_ms: trans_pos,
                                duration_ms: dur,
                                envelope: trans_env,
                                expressions: note.expressions.clone(),
                                pitch_bend: crate::project::model::UPitchBend::default(),
                                vibrato: crate::dsp::pitch::VibratoParam::default(),
                                flags: note.flags.clone(),
                            });
                            trans_pos += dur;
                        }

                        let main_alias = &aliases[aliases.len() - 1];
                        let final_main = vb
                            .find_entry(main_alias, &note.pitch)
                            .map(|e| e.alias.clone())
                            .unwrap_or_else(|| main_alias.clone());

                        let mut main_env = note.envelope.clone();
                        main_env.p1 = 0.0;
                        main_env.p2 = 0.0;
                        main_env.v1 = 100.0;
                        main_env.v2 = 100.0;

                        note_phones.push(RenderPhone {
                            note_index,
                            lyric: final_main,
                            pitch: note.pitch.clone(),
                            position_ms: syl_start_pos,
                            duration_ms: syl_dur,
                            envelope: main_env,
                            expressions: note.expressions.clone(),
                            pitch_bend: note.pitch_bend.clone(),
                            vibrato: note.vibrato.clone(),
                            flags: note.flags.clone(),
                        });
                    } else {
                        if let Some(last_p) = note_phones.last_mut() {
                            let borrow = total_trans_dur.min((last_p.duration_ms - 20.0).max(0.0));
                            last_p.duration_ms -= borrow;
                        }

                        let mut trans_pos = syl_start_pos - total_trans_dur;
                        for (alias_idx, alias) in aliases[..aliases.len() - 1].iter().enumerate() {
                            let dur = trans_durs[alias_idx];
                            let final_alias = vb
                                .find_entry(alias, &note.pitch)
                                .map(|e| e.alias.clone())
                                .unwrap_or_else(|| alias.clone());

                            let mut trans_env = crate::dsp::envelope::UtauEnvelope::default();
                            trans_env.p4 = 0.0;
                            trans_env.p5 = 0.0;
                            trans_env.v4 = 100.0;
                            trans_env.v5 = 100.0;

                            note_phones.push(RenderPhone {
                                note_index,
                                lyric: final_alias,
                                pitch: note.pitch.clone(),
                                position_ms: trans_pos,
                                duration_ms: dur,
                                envelope: trans_env,
                                expressions: note.expressions.clone(),
                                pitch_bend: crate::project::model::UPitchBend::default(),
                                vibrato: crate::dsp::pitch::VibratoParam::default(),
                                flags: note.flags.clone(),
                            });
                            trans_pos += dur;
                        }

                        let main_alias = &aliases[aliases.len() - 1];
                        let final_main = vb
                            .find_entry(main_alias, &note.pitch)
                            .map(|e| e.alias.clone())
                            .unwrap_or_else(|| main_alias.clone());

                        let mut main_env = note.envelope.clone();
                        main_env.p1 = 0.0;
                        main_env.p2 = 0.0;
                        main_env.v1 = 100.0;
                        main_env.v2 = 100.0;

                        note_phones.push(RenderPhone {
                            note_index,
                            lyric: final_main,
                            pitch: note.pitch.clone(),
                            position_ms: syl_start_pos,
                            duration_ms: syl_dur,
                            envelope: main_env,
                            expressions: note.expressions.clone(),
                            pitch_bend: note.pitch_bend.clone(),
                            vibrato: note.vibrato.clone(),
                            flags: note.flags.clone(),
                        });
                    }
                }

                prev_vowel = Some(v.clone());
            }

            if !trailing_coda.is_empty() {
                if let Some(pv) = prev_vowel.as_deref() {
                    let end_aliases = Self::process_ending(pv, &trailing_coda, vb, &note.pitch);
                    for alias in end_aliases {
                        let final_alias = vb
                            .find_entry(&alias, &note.pitch)
                            .map(|e| e.alias.clone())
                            .unwrap_or(alias);

                        let coda_dur =
                            Self::get_transition_basic_length_ms(&final_alias).clamp(35.0, 135.0);
                        if let Some(last) = note_phones.last_mut() {
                            if last.duration_ms > coda_dur + 20.0 {
                                last.duration_ms -= coda_dur;
                                note_phones.push(RenderPhone {
                                    note_index,
                                    lyric: final_alias,
                                    pitch: note.pitch.clone(),
                                    position_ms: note.position_ms + note.duration_ms - coda_dur,
                                    duration_ms: coda_dur,
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

            if !note.phoneme_durations_ms.is_empty() && !note_phones.is_empty() {
                let custom_durs = note.resolved_phoneme_durations(note_phones.len());
                let initial_offset = note_phones[0].position_ms;
                let mut cur_pos = initial_offset;
                for (phone, &dur) in note_phones.iter_mut().zip(custom_durs.iter()) {
                    phone.position_ms = cur_pos;
                    phone.duration_ms = dur;
                    cur_pos += dur;
                }
            }

            phones.extend(note_phones);

            prev_note_end_ms = Some(note.position_ms + note.duration_ms);
        }

        phones
    }
}

pub type BrapaCVCPhonemizer = VccvBrapaPhonemizer;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vccv_brapa_replacements() {
        assert_eq!(
            VccvBrapaPhonemizer::query_phonemes("canto"),
            vec!["k", "an", "t", "o"]
        );
        assert_eq!(
            VccvBrapaPhonemizer::query_phonemes("tempo"),
            vec!["t", "en", "p", "o"]
        );
        assert_eq!(
            VccvBrapaPhonemizer::query_phonemes("cão"),
            vec!["k", "an", "o"]
        );
        assert_eq!(VccvBrapaPhonemizer::query_phonemes("pé"), vec!["p", "eh"]);
        assert_eq!(VccvBrapaPhonemizer::query_phonemes("nó"), vec!["n", "oh"]);
        assert_eq!(
            VccvBrapaPhonemizer::query_phonemes("chuva"),
            vec!["ch", "u", "v", "a"]
        );
    }

    #[test]
    fn test_vccv_brapa_validate_alias() {
        assert_eq!(VccvBrapaPhonemizer::validate_alias("rh a"), "r a");
        assert_eq!(VccvBrapaPhonemizer::validate_alias("a rh-"), "a rh-");
    }
}
