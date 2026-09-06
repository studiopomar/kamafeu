use crate::oto::Voicebank;
use crate::phonemizer::{consonant_velocity_time_scale, romaji, PhonemizerMode, RenderPhone};
use crate::project::model::UNote;
use std::collections::HashMap;
use std::sync::LazyLock;

static PLAIN_VOWELS: &[&str] = &[
    "あ", "い", "う", "え", "お", "を", "ん", "ン", "a", "i", "u", "e", "o", "n", "N",
];

static NON_VOWELS: &[&str] = &[
    "息", "吸", "R", "-", "k", "ky", "g", "gy", "s", "sh", "z", "j", "t", "ch", "ty", "ts", "d",
    "dy", "n", "ny", "h", "hy", "f", "b", "by", "p", "py", "m", "my", "y", "r", "4", "ry", "w",
    "v", "ng", "l", "・", "B", "H",
];

static VOWEL_TABLE: &[(&str, &[&str])] = &[
    (
        "a",
        &[
            "ぁ", "あ", "か", "が", "さ", "ざ", "た", "だ", "な", "は", "ば", "ぱ", "ま", "ゃ",
            "や", "ら", "わ", "ァ", "ア", "カ", "ガ", "サ", "ザ", "タ", "ダ", "ナ", "ハ", "バ",
            "パ", "マ", "ャ", "ヤ", "ラ", "ワ", "fa", "ka", "ga", "sa", "za", "ta", "da", "na",
            "ha", "ba", "pa", "ma", "ya", "ra", "wa", "kya", "sha", "cha", "tya", "nya", "hya",
            "mya", "rya", "gya", "ja", "jya", "bya", "pya", "dya", "tsa", "va", "a",
        ],
    ),
    (
        "e",
        &[
            "ぇ", "え", "け", "げ", "せ", "ぜ", "て", "で", "ね", "へ", "べ", "ぺ", "め", "れ",
            "ゑ", "ェ", "エ", "ケ", "ゲ", "セ", "ゼ", "テ", "デ", "ネ", "ヘ", "ベ", "ペ", "メ",
            "レ", "ヱ", "fe", "ke", "ge", "se", "ze", "te", "de", "ne", "he", "be", "pe", "me",
            "re", "we", "kye", "she", "che", "tye", "nye", "hye", "mye", "rye", "gye", "je", "jye",
            "bye", "pye", "dye", "tse", "ve", "e",
        ],
    ),
    (
        "i",
        &[
            "ぃ", "い", "き", "ぎ", "し", "じ", "ち", "ぢ", "に", "ひ", "び", "ぴ", "み", "り",
            "ゐ", "ィ", "イ", "キ", "ギ", "シ", "ジ", "チ", "ヂ", "ニ", "ヒ", "ビ", "ピ", "ミ",
            "リ", "ヰ", "fi", "ki", "gi", "shi", "si", "ji", "zi", "chi", "ti", "ni", "hi", "bi",
            "pi", "mi", "ri", "wi", "kyi", "kii", "nyi", "hyi", "myi", "ryi", "gyi", "jyi", "byi",
            "pyi", "dyi", "tsi", "vi", "i",
        ],
    ),
    (
        "o",
        &[
            "ぉ", "お", "こ", "ご", "そ", "ぞ", "と", "ど", "の", "ほ", "ぼ", "ぽ", "も", "ょ",
            "よ", "ろ", "を", "ォ", "オ", "コ", "ゴ", "ソ", "ゾ", "ト", "ド", "ノ", "ホ", "ボ",
            "ポ", "モ", "ョ", "ヨ", "ロ", "ヲ", "fo", "ko", "go", "so", "zo", "to", "do", "no",
            "ho", "bo", "po", "mo", "yo", "ro", "wo", "kyo", "sho", "cho", "tyo", "nyo", "hyo",
            "myo", "ryo", "gyo", "jo", "jyo", "byo", "pyo", "dyo", "tso", "vo", "o",
        ],
    ),
    (
        "u",
        &[
            "ぅ", "う", "く", "ぐ", "す", "ず", "つ", "づ", "ぬ", "ふ", "ぶ", "ぷ", "む", "ゅ",
            "ゆ", "る", "ゥ", "ウ", "ク", "グ", "ス", "ズ", "ツ", "ヅ", "ヌ", "フ", "ブ", "プ",
            "ム", "ュ", "ユ", "ル", "ヴ", "fu", "hu", "ku", "gu", "su", "zu", "tsu", "tu", "du",
            "dzu", "nu", "bu", "pu", "mu", "yu", "ru", "wu", "kyu", "shu", "chu", "tyu", "nyu",
            "hyu", "myu", "ryu", "gyu", "ju", "jyu", "byu", "pyu", "dyu", "vu", "u",
        ],
    ),
    ("n", &["ん", "ン", "n", "N"]),
    (
        "・",
        &[
            "・", "・あ", "・い", "・う", "・え", "・お", "・ん", "・を", "・ン",
        ],
    ),
];

static CONSONANT_TABLE: &[(&str, &[&str])] = &[
    (
        "ch",
        &[
            "ち", "ちぇ", "ちゃ", "ちゅ", "ちょ", "チ", "チェ", "チャ", "チュ", "チョ", "cha",
            "che", "chu", "cho", "chi",
        ],
    ),
    (
        "gy",
        &[
            "ぎ", "ぎぇ", "ぎゃ", "ぎゅ", "ぎょ", "ギ", "ギェ", "ギャ", "ギュ", "ギョ", "gya",
            "gye", "gyi", "gyu", "gyo",
        ],
    ),
    (
        "ts",
        &[
            "つ", "つぁ", "つぃ", "つぇ", "つぉ", "ツ", "ツァ", "ツィ", "ツェ", "ツォ", "tsa",
            "tsi", "tse", "tso", "tsu",
        ],
    ),
    (
        "ty",
        &[
            "てぃ", "てぇ", "てゃ", "てゅ", "てょ", "ティ", "テェ", "テャ", "テュ", "テョ", "tya",
            "tye", "tyi", "tyu", "tyo",
        ],
    ),
    (
        "py",
        &[
            "ぴ", "ぴぇ", "ぴゃ", "ぴゅ", "ぴょ", "ピ", "ピェ", "ピャ", "ピュ", "ピョ", "pya",
            "pye", "pyi", "pyu", "pyo",
        ],
    ),
    (
        "ry",
        &[
            "り", "りぇ", "りゃ", "りゅ", "りょ", "リ", "リェ", "リャ", "リュ", "リョ", "rya",
            "rye", "ryi", "ryu", "ryo",
        ],
    ),
    (
        "ly",
        &[
            "リ", "リェ", "リャ", "リュ", "リョ", "lya", "lye", "lyi", "lyu", "lyo",
        ],
    ),
    (
        "ny",
        &[
            "に", "にぇ", "にゃ", "にゅ", "にょ", "ニ", "ニェ", "ニャ", "ニュ", "ニョ", "nya",
            "nye", "nyi", "nyu", "nyo",
        ],
    ),
    (
        "r",
        &[
            "ら", "る", "るぃ", "れ", "ろ", "ラ", "ル", "レ", "ロ", "ra", "ru", "re", "ro",
        ],
    ),
    (
        "hy",
        &[
            "ひ", "ひぇ", "ひゃ", "ひゅ", "ひょ", "ヒ", "ヒェ", "ヒャ", "ヒュ", "ヒョ", "hya",
            "hye", "hyi", "hyu", "hyo",
        ],
    ),
    (
        "dy",
        &[
            "でぃ", "でぇ", "でゃ", "でゅ", "でょ", "ディ", "デェ", "デャ", "デュ", "デョ", "dya",
            "dye", "dyi", "dyu", "dyo",
        ],
    ),
    (
        "by",
        &[
            "び", "びぇ", "びゃ", "びゅ", "びょ", "ビ", "ビェ", "ビャ", "ビュ", "ビョ", "bya",
            "bye", "byi", "byu", "byo",
        ],
    ),
    (
        "b",
        &[
            "ば", "ぶ", "ぶぃ", "べ", "ぼ", "バ", "ブ", "ベ", "ボ", "ba", "bu", "be", "bo",
        ],
    ),
    (
        "d",
        &[
            "だ", "で", "ど", "どぃ", "どぅ", "ダ", "デ", "ド", "da", "de", "do",
        ],
    ),
    (
        "g",
        &[
            "が", "ぐ", "ぐぃ", "げ", "ご", "ガ", "グ", "ゲ", "ゴ", "ga", "gu", "ge", "go",
        ],
    ),
    (
        "f",
        &[
            "ふ", "ふぁ", "ふぃ", "ふぇ", "ふぉ", "フ", "ファ", "フィ", "フェ", "フォ", "fa", "fi",
            "fu", "fe", "fo",
        ],
    ),
    (
        "h",
        &[
            "は", "はぃ", "へ", "ほ", "ほぅ", "ハ", "ヘ", "ホ", "ha", "he", "ho", "hi",
        ],
    ),
    (
        "k",
        &[
            "か", "く", "くぃ", "け", "こ", "カ", "ク", "ケ", "コ", "ka", "ku", "ke", "ko",
        ],
    ),
    (
        "j",
        &[
            "じ", "じぇ", "じゃ", "じゅ", "じょ", "ぢ", "ぢぇ", "ぢゃ", "ぢゅ", "ぢょ", "ジ",
            "ジェ", "ジャ", "ジュ", "ジョ", "ja", "je", "ji", "ju", "jo",
        ],
    ),
    (
        "m",
        &[
            "ま", "む", "むぃ", "め", "も", "マ", "ム", "メ", "モ", "ma", "mu", "me", "mo",
        ],
    ),
    (
        "n",
        &[
            "な", "ぬ", "ぬぃ", "ね", "の", "ナ", "ヌ", "ネ", "ノ", "na", "nu", "ne", "no",
        ],
    ),
    (
        "p",
        &[
            "ぱ", "ぷ", "ぷぃ", "ぺ", "ぽ", "パ", "プ", "ペ", "ポ", "pa", "pu", "pe", "po",
        ],
    ),
    (
        "s",
        &[
            "さ", "す", "すぃ", "せ", "そ", "サ", "ス", "セ", "ソ", "sa", "su", "se", "so",
        ],
    ),
    (
        "sh",
        &[
            "し", "しぇ", "しゃ", "しゅ", "しょ", "シ", "シェ", "シャ", "シュ", "ショ", "sha",
            "she", "shi", "shu", "sho", "si",
        ],
    ),
    (
        "t",
        &[
            "た", "て", "と", "とぃ", "とぅ", "タ", "テ", "ト", "ta", "te", "to",
        ],
    ),
    (
        "v",
        &[
            "ヴ", "ヴぁ", "ヴぃ", "ヴぅ", "ヴぇ", "ヴぉ", "va", "vi", "vu", "ve", "vo",
        ],
    ),
    (
        "ky",
        &[
            "き", "きぇ", "きゃ", "きゅ", "きょ", "キ", "キェ", "キャ", "キュ", "キョ", "kya",
            "kye", "kyi", "kyu", "kyo",
        ],
    ),
    (
        "w",
        &[
            "うぃ", "うぅ", "うぇ", "うぉ", "わ", "ゐ", "ゑ", "を", "ヰ", "ヱ", "ワ", "ウィ",
            "ウェ", "ウォ", "wa", "wi", "we", "wo",
        ],
    ),
    (
        "y",
        &[
            "いぃ", "いぇ", "や", "ゆ", "よ", "ヤ", "ユ", "ヨ", "イェ", "ya", "yu", "yo", "ye",
        ],
    ),
    (
        "z",
        &[
            "ざ", "ず", "ずぃ", "ぜ", "ぞ", "ザ", "ズ", "ゼ", "ゾ", "za", "zu", "ze", "zo",
        ],
    ),
    ("dz", &["づ", "づぃ", "ヅ", "dzu"]),
    (
        "my",
        &[
            "み", "みぇ", "みゃ", "みゅ", "みょ", "ミ", "ミェ", "ミャ", "ミュ", "ミョ", "mya",
            "mye", "myi", "myu", "myo",
        ],
    ),
    (
        "ng",
        &[
            "ガ", "ギ", "グ", "ゲ", "ゴ", "ギェ", "ギャ", "ギュ", "ギョ", "nga", "ngi", "ngu",
            "nge", "ngo",
        ],
    ),
    ("l", &["ラ", "ル", "レ", "ロ", "la", "lu", "le", "lo"]),
    (
        "・",
        &[
            "・あ", "・い", "・う", "・え", "・お", "・ん", "・を", "・ン",
        ],
    ),
];

static SUBSTITUTIONS: &[(&[&str], &str)] = &[
    (&["ty", "ch", "ts"], "t"),
    (&["j", "dy"], "d"),
    (&["gy"], "g"),
    (&["ky"], "k"),
    (&["py"], "p"),
    (&["ny"], "n"),
    (&["ry"], "r"),
    (&["my"], "m"),
    (&["hy", "f"], "h"),
    (&["by", "v"], "b"),
    (&["dz"], "z"),
    (&["l"], "r"),
    (&["ly"], "l"),
];

static VOWEL_LOOKUP: LazyLock<HashMap<String, &'static str>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for &(vowel, list) in VOWEL_TABLE {
        for &kana in list {
            map.insert(kana.to_string(), vowel);
        }
    }
    map
});

static CONSONANT_LOOKUP: LazyLock<HashMap<String, &'static str>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for &(consonant, list) in CONSONANT_TABLE {
        for &kana in list {
            map.insert(kana.to_string(), consonant);
        }
    }
    map
});

static SUBSTITUTE_LOOKUP: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut map = HashMap::new();
    for &(srcs, target) in SUBSTITUTIONS {
        for &src in srcs {
            map.insert(src, target);
        }
    }
    map
});

pub struct JapanesePhonemizer;

impl JapanesePhonemizer {
    pub fn extract_vowel(lyric: &str) -> Option<&'static str> {
        let trimmed = lyric.trim();
        if trimmed.is_empty() {
            return None;
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
            if let Some(entry) = vb.find_entry(cand, pitch) {
                return Some(entry.alias.clone());
            }
        }
        None
    }

    pub fn apply_japanese(
        notes: &[UNote],
        vb: &Voicebank,
        mode: PhonemizerMode,
    ) -> Vec<RenderPhone> {
        let mut phones: Vec<RenderPhone> = Vec::new();
        let mut prev_vowel: Option<&'static str> = None;
        let mut prev_note_end_ms: Option<f64> = None;

        struct NoteRef<'a> {
            note_index: usize,
            lyric: &'a str,
            note: &'a UNote,
            position_ms: f64,
            duration_ms: f64,
        }

        let mut expanded: Vec<NoteRef> = Vec::new();
        for (note_index, note) in notes.iter().enumerate() {
            let raw_lyric = note.lyric.trim();
            if raw_lyric.contains(';') || raw_lyric.contains(',') {
                let parts: Vec<&str> = raw_lyric
                    .split(|c| c == ';' || c == ',')
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
                            lyric: part,
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
                lyric: raw_lyric,
                note,
                position_ms: note.position_ms,
                duration_ms: note.duration_ms,
            });
        }

        let len = expanded.len();
        for idx in 0..len {
            let exp = &expanded[idx];
            let note = exp.note;
            let note_index = exp.note_index;

            let cur_vowel = Self::extract_vowel(exp.lyric);
            let _cur_consonant = Self::extract_consonant(exp.lyric);

            let is_phrase_start = match prev_note_end_ms {
                Some(end_ms) => exp.position_ms > end_ms + 60.0,
                None => true,
            };

            if is_phrase_start {
                prev_vowel = None;
            }

            if exp.lyric == "+" && !is_phrase_start {
                if let Some(last) = phones.last_mut() {
                    last.duration_ms += exp.duration_ms;
                    prev_note_end_ms = Some(exp.position_ms + exp.duration_ms);
                    continue;
                }
            }

            match mode {
                PhonemizerMode::BasicCV => {
                    let mut alias = exp.lyric.to_string();
                    if is_phrase_start {
                        let head_cands = vec![
                            format!("- {}", exp.lyric),
                            format!("-{}", exp.lyric),
                            exp.lyric.to_string(),
                        ];
                        if let Some(found) = Self::find_oto_candidate(vb, &head_cands, &note.pitch)
                        {
                            alias = found;
                        }
                    } else if let Some(found) = vb.find_entry(&alias, &note.pitch) {
                        alias = found.alias.clone();
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
                    });
                }

                PhonemizerMode::VCV => {
                    let mut alias = exp.lyric.to_string();
                    if !is_phrase_start {
                        if let Some(pv) = prev_vowel {
                            let vcv_cands = vec![
                                format!("{} {}", pv, exp.lyric),
                                format!("{}_{}", pv, exp.lyric),
                                format!("{}{}", pv, exp.lyric),
                            ];
                            if let Some(found) =
                                Self::find_oto_candidate(vb, &vcv_cands, &note.pitch)
                            {
                                alias = found;
                            }
                        }
                    }

                    if alias == exp.lyric {
                        let head_cands = vec![
                            format!("- {}", exp.lyric),
                            format!("-{}", exp.lyric),
                            exp.lyric.to_string(),
                        ];
                        if let Some(found) = Self::find_oto_candidate(vb, &head_cands, &note.pitch)
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
                    });
                }

                PhonemizerMode::CVVC => {
                    let is_plain_vowel =
                        PLAIN_VOWELS.contains(&exp.lyric) || NON_VOWELS.contains(&exp.lyric);
                    let mut current_lyric = exp.lyric.to_string();

                    if is_phrase_start {
                        let initial_cands = vec![
                            format!("- {}", exp.lyric),
                            format!("-{}", exp.lyric),
                            exp.lyric.to_string(),
                        ];
                        if let Some(found) =
                            Self::find_oto_candidate(vb, &initial_cands, &note.pitch)
                        {
                            current_lyric = found;
                        }
                    } else if is_plain_vowel {
                        if let Some(pv) = prev_vowel {
                            let vow_cands = vec![
                                format!("{} {}", pv, exp.lyric),
                                format!("* {}", exp.lyric),
                                format!("_{}", exp.lyric),
                                exp.lyric.to_string(),
                            ];
                            if let Some(found) =
                                Self::find_oto_candidate(vb, &vow_cands, &note.pitch)
                            {
                                current_lyric = found;
                            }
                        }
                    } else {
                        let cv_cands = vec![format!("* {}", exp.lyric), exp.lyric.to_string()];
                        if let Some(found) = Self::find_oto_candidate(vb, &cv_cands, &note.pitch) {
                            current_lyric = found;
                        }
                    }

                    let next_note_ref = expanded.get(idx + 1);
                    let mut vc_to_insert: Option<(String, f64)> = None;

                    if let Some(next_exp) = next_note_ref {
                        let next_gap = next_exp.position_ms - (exp.position_ms + exp.duration_ms);
                        let is_next_adjacent = next_gap <= 60.0;

                        if is_next_adjacent && !PLAIN_VOWELS.contains(&next_exp.lyric) {
                            if let (Some(vow), Some(con)) =
                                (cur_vowel, Self::extract_consonant(next_exp.lyric))
                            {
                                let mut vc_candidates = vec![
                                    format!("{} {}", vow, con),
                                    format!("{}{}", vow, con),
                                    format!("{}_{}", vow, con),
                                ];

                                if let Some(&sub_con) = SUBSTITUTE_LOOKUP.get(con) {
                                    vc_candidates.push(format!("{} {}", vow, sub_con));
                                    vc_candidates.push(format!("{}{}", vow, sub_con));
                                }

                                if let Some(vc_alias) = Self::find_oto_candidate(
                                    vb,
                                    &vc_candidates,
                                    &next_exp.note.pitch,
                                ) {
                                    let mut vc_length_ms = 80.0;
                                    if let Some(oto) =
                                        vb.find_entry(next_exp.lyric, &next_exp.note.pitch)
                                    {
                                        if oto.overlap < 0.0 {
                                            vc_length_ms =
                                                (oto.preutterance - oto.overlap).max(10.0);
                                        } else {
                                            vc_length_ms = oto.preutterance.max(10.0);
                                        }
                                    }
                                    let vel_scale = consonant_velocity_time_scale(
                                        next_exp.note.expressions.consonant_velocity,
                                    );
                                    let vc_len = (vc_length_ms * vel_scale)
                                        .min(exp.duration_ms * 0.5)
                                        .min((exp.duration_ms - 20.0).max(10.0));
                                    vc_to_insert = Some((vc_alias, vc_len));
                                }
                            }
                        }
                    }

                    if let Some((vc_alias, vc_len)) = vc_to_insert {
                        let main_duration = (exp.duration_ms - vc_len).max(10.0);
                        phones.push(RenderPhone {
                            note_index,
                            lyric: current_lyric,
                            pitch: note.pitch.clone(),
                            position_ms: exp.position_ms,
                            duration_ms: main_duration,
                            envelope: note.envelope.clone(),
                            expressions: note.expressions.clone(),
                            pitch_bend: note.pitch_bend.clone(),
                            vibrato: note.vibrato.clone(),
                            flags: note.flags.clone(),
                        });

                        let vc_envelope = crate::dsp::envelope::UtauEnvelope {
                            p1: 0.0,
                            p2: 5.0,
                            p3: 20.0,
                            p4: 0.0,
                            p5: 10.0,
                            v1: 0.0,
                            v2: 100.0,
                            v3: 100.0,
                            v4: 100.0,
                            v5: 100.0,
                            crossfade_ms: 0.0,
                        };

                        phones.push(RenderPhone {
                            note_index,
                            lyric: vc_alias,
                            pitch: note.pitch.clone(),
                            position_ms: exp.position_ms + main_duration,
                            duration_ms: vc_len,
                            envelope: vc_envelope,
                            expressions: note.expressions.clone(),
                            pitch_bend: crate::project::model::UPitchBend::default(),
                            vibrato: crate::dsp::pitch::VibratoParam::default(),
                            flags: note.flags.clone(),
                        });
                    } else {
                        phones.push(RenderPhone {
                            note_index,
                            lyric: current_lyric,
                            pitch: note.pitch.clone(),
                            position_ms: exp.position_ms,
                            duration_ms: exp.duration_ms,
                            envelope: note.envelope.clone(),
                            expressions: note.expressions.clone(),
                            pitch_bend: note.pitch_bend.clone(),
                            vibrato: note.vibrato.clone(),
                            flags: note.flags.clone(),
                        });
                    }
                }

                _ => {}
            }

            prev_vowel = cur_vowel;
            prev_note_end_ms = Some(exp.position_ms + exp.duration_ms);
        }

        phones
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oto::OtoEntry;
    use std::path::PathBuf;

    fn build_test_voicebank() -> Voicebank {
        let mut entries = HashMap::new();
        let list = [
            ("- か", "ka.wav", 0.0, 50.0, 10.0),
            ("か", "ka.wav", 0.0, 50.0, 10.0),
            ("a s", "as.wav", 0.0, 30.0, 5.0),
            ("a k", "ak.wav", 0.0, 30.0, 5.0),
            ("a t", "at.wav", 0.0, 30.0, 5.0),
            ("さ", "sa.wav", 0.0, 50.0, 10.0),
            ("a あ", "a.wav", 0.0, 20.0, 5.0),
            ("あ", "a.wav", 0.0, 20.0, 5.0),
            ("- あ", "a.wav", 0.0, 20.0, 5.0),
            ("ちゃ", "cha.wav", 0.0, 60.0, 10.0),
        ];

        for &(alias, wav, off, pre, ovl) in &list {
            entries.insert(
                alias.to_string(),
                OtoEntry::new(wav.to_string(), alias.to_string(), off, pre, ovl, 0.0, 0.0),
            );
        }

        Voicebank {
            root_path: PathBuf::from("/tmp"),
            name: "Test Japanese VB".to_string(),
            author: "System".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        }
    }

    #[test]
    fn test_japanese_cvvc_head_and_vc_transition() {
        let vb = build_test_voicebank();
        let notes = vec![
            UNote::new("か", "C4", 0.0, 400.0),
            UNote::new("さ", "C4", 400.0, 400.0),
        ];

        let phones = JapanesePhonemizer::apply_japanese(&notes, &vb, PhonemizerMode::CVVC);
        assert_eq!(phones.len(), 3);
        assert_eq!(
            phones[0].lyric, "- か",
            "Initial note should get - CV head alias"
        );
        assert_eq!(
            phones[1].lyric, "a s",
            "VC transition should be inserted before 'さ'"
        );
        assert!(
            phones[1].position_ms < 400.0,
            "VC transition begins before 400ms"
        );
        assert_eq!(phones[2].lyric, "さ", "Main CV note starts at 400ms");
        assert_eq!(phones[2].position_ms, 400.0);
    }

    #[test]
    fn test_japanese_cvvc_substitute_consonant() {
        let vb = build_test_voicebank();
        let notes = vec![
            UNote::new("か", "C4", 0.0, 400.0),
            UNote::new("ちゃ", "C4", 400.0, 400.0),
        ];

        let phones = JapanesePhonemizer::apply_japanese(&notes, &vb, PhonemizerMode::CVVC);
        assert_eq!(phones.len(), 3);
        assert_eq!(phones[0].lyric, "- か");
        assert_eq!(
            phones[1].lyric, "a t",
            "Should fallback to substitute 'a t' for 'ch'"
        );
        assert_eq!(phones[2].lyric, "ちゃ");
    }

    #[test]
    fn test_japanese_vcv_transition() {
        let mut vb = build_test_voicebank();
        vb.entries.insert(
            "a か".to_string(),
            OtoEntry::new(
                "aka.wav".to_string(),
                "a か".to_string(),
                0.0,
                50.0,
                10.0,
                0.0,
                0.0,
            ),
        );

        let notes = vec![
            UNote::new("あ", "C4", 0.0, 400.0),
            UNote::new("か", "C4", 400.0, 400.0),
        ];

        let phones = JapanesePhonemizer::apply_japanese(&notes, &vb, PhonemizerMode::VCV);
        assert_eq!(phones.len(), 2);
        assert_eq!(phones[0].lyric, "- あ");
        assert_eq!(phones[1].lyric, "a か");
    }
}
