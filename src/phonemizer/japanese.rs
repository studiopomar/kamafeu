use crate::oto::Voicebank;
use crate::phonemizer::{consonant_velocity_time_scale, romaji, PhonemizerMode, RenderPhone};
use crate::project::model::UNote;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::LazyLock;

static PLAIN_VOWELS: &[&str] = &[
    "あ", "い", "う", "え", "お", "を", "ん", "ン", "a", "i", "u", "e", "o", "n", "N",
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
    // OpenUtau deliberately keeps hiragana `ん` and katakana `ン` as
    // different transition vowels.  CVVC banks commonly use `n` for the
    // former and `N` for the latter.
    ("n", &["ん", "n"]),
    ("N", &["ン", "N"]),
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
            "・", "・あ", "・い", "・う", "・え", "・お", "・ん", "・を", "・ン",
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

/// Reads the exact-match replacement section used by Presamp.  OpenUtau
/// applies these replacements to the lyric before VCV/CVVC classification,
/// but never applies them to an explicit phonetic hint.
fn parse_presamp_replacements(contents: &str) -> HashMap<String, String> {
    let mut replacements = HashMap::new();
    let mut in_replace = false;
    for raw_line in contents.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            in_replace = line.eq_ignore_ascii_case("[REPLACE]");
            continue;
        }
        if !in_replace || line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        let Some((from, to)) = line.split_once('=') else {
            continue;
        };
        let from = from.trim();
        let to = to.trim();
        if !from.is_empty() {
            replacements.insert(from.to_string(), to.to_string());
        }
    }
    replacements
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PresampAliasConfig {
    vcpad: String,
    vcvpad: String,
    vcv: String,
    beginning_cv: String,
    cross_cv: String,
    vc: String,
    cv: String,
    c: String,
    long_v: String,
    ending1: String,
    ending2: String,
}

#[derive(Debug, Clone)]
struct PresampInventory {
    vowels: HashSet<String>,
    /// Every Presamp vowel symbol mapped to its canonical lower-case vowel.
    vowel_by_phoneme: HashMap<String, String>,
    consonants: HashSet<String>,
    /// Every Presamp consonant symbol mapped to its canonical consonant.
    consonant_by_phoneme: HashMap<String, String>,
    alias_nums: HashSet<String>,
    alias_appends: Vec<String>,
    alias_pitches: Vec<String>,
    priority: HashSet<String>,
    no_crossfade: HashSet<String>,
    /// Presamp `[VCLENGTH]`: 0 derives VC timing from the following CV oto;
    /// 1 uses the fixed legacy duration.
    vc_length_from_cv: bool,
    /// Presamp `[MUSTVC]`: force a VC even when the following alias already
    /// contains its own transition or the note is too short.
    must_vc: bool,
    /// Presamp `[ENDFLAG]`: 0 disables ending conversion, 1 adds/uses the
    /// ending alias, and 2 converts the final note when supported.
    add_ending: u8,
    /// Presamp `[SPLIT]` controls comma/semicolon lyric expansion.
    split: bool,
}

impl Default for PresampInventory {
    fn default() -> Self {
        let priority = [
            "k", "ky", "g", "gy", "t", "ty", "d", "dy", "ch", "ts", "b", "by", "p", "py", "r", "ry",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        let no_crossfade = [
            "ch", "gy", "ts", "ty", "py", "ry", "r", "dy", "by", "b", "d", "g", "k", "p", "t", "ky",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();
        Self {
            vowels: HashSet::new(),
            vowel_by_phoneme: HashMap::new(),
            consonants: HashSet::new(),
            consonant_by_phoneme: HashMap::new(),
            alias_nums: (0..=9).map(|n| n.to_string()).collect(),
            alias_appends: [
                "強", "弱", "暗", "明", "囁", "叫", "超", "幼", "甘", "笑", "瞭", "憂", "優", "感",
                "張", "抗", "泣叫", "癖", "元気", "艶",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            alias_pitches: Vec::new(),
            priority,
            no_crossfade,
            vc_length_from_cv: true,
            must_vc: false,
            add_ending: 1,
            split: true,
        }
    }
}

fn parse_presamp_inventory(contents: &str) -> PresampInventory {
    let mut inventory = PresampInventory::default();
    let mut section = "";
    for raw_line in contents.lines() {
        let line = raw_line.trim_end_matches('\r').trim();
        if line.starts_with('[') && line.ends_with(']') {
            section = line;
            match line.to_ascii_uppercase().as_str() {
                "[VOWEL]" => {
                    inventory.vowels.clear();
                    inventory.vowel_by_phoneme.clear();
                }
                "[CONSONANT]" => {
                    inventory.consonants.clear();
                    inventory.consonant_by_phoneme.clear();
                    inventory.no_crossfade.clear();
                }
                "[PRIORITY]" => inventory.priority.clear(),
                _ => {}
            }
            continue;
        }
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('=').map(str::trim).collect();
        let Some(first) = fields.first().filter(|value| !value.is_empty()) else {
            continue;
        };
        let mut values = vec![*first];
        if let Some(list) = fields.get(1) {
            values.extend(
                list.split(',')
                    .map(str::trim)
                    .filter(|value| !value.is_empty()),
            );
        }
        match section.to_ascii_uppercase().as_str() {
            "[VOWEL]" => {
                // Presamp format: vowel=representative=phoneme1,phoneme2=volume.
                // The representative and every listed phoneme are valid
                // lyric symbols; the final volume field is intentionally
                // ignored by the phonemizer.
                inventory.vowels.insert((*first).to_string());
                inventory
                    .vowel_by_phoneme
                    .insert((*first).to_string(), (*first).to_string());
                if let Some(representative) = fields.get(1).filter(|v| !v.is_empty()) {
                    inventory.vowels.insert((*representative).to_string());
                    inventory
                        .vowel_by_phoneme
                        .insert((*representative).to_string(), (*first).to_string());
                }
                if let Some(list) = fields.get(2) {
                    for value in list.split(',').map(str::trim).filter(|v| !v.is_empty()) {
                        inventory.vowels.insert(value.to_string());
                        inventory
                            .vowel_by_phoneme
                            .insert(value.to_string(), (*first).to_string());
                    }
                }
            }
            "[CONSONANT]" => {
                inventory
                    .consonants
                    .extend(values.into_iter().map(str::to_owned));
                inventory
                    .consonant_by_phoneme
                    .insert((*first).to_string(), (*first).to_string());
                if let Some(list) = fields.get(1) {
                    for value in list.split(',').map(str::trim).filter(|v| !v.is_empty()) {
                        inventory
                            .consonant_by_phoneme
                            .insert(value.to_string(), (*first).to_string());
                    }
                }
                if fields.get(2).is_some_and(|flag| *flag == "1") {
                    // Presamp's third field is the no-crossfade flag, not
                    // priority. Priority is configured by [PRIORITY].
                    inventory.no_crossfade.insert((*first).to_string());
                }
            }
            "[PRIORITY]" => {
                inventory
                    .priority
                    .extend(values.into_iter().map(str::to_owned));
            }
            "[VCLENGTH]" => {
                inventory.vc_length_from_cv = *first != "1";
            }
            "[MUSTVC]" => {
                inventory.must_vc = *first == "1";
            }
            "[ENDFLAG]" => {
                if let Ok(flag) = first.parse::<u8>() {
                    inventory.add_ending = flag.min(2);
                }
            }
            "[SPLIT]" => {
                inventory.split = *first != "0";
            }
            "[NUM]" => {
                inventory.alias_nums.insert((*first).to_string());
            }
            "[APPEND]" => {
                inventory.alias_appends.push((*first).to_string());
            }
            "[PITCH]" => {
                inventory.alias_pitches.push((*first).to_string());
            }
            _ => {}
        }
    }
    inventory
}

fn presamp_core_lyric(lyric: &str, inventory: &PresampInventory) -> String {
    let mut core = lyric.trim().to_string();
    if let Some((left, right)) = core.rsplit_once('_') {
        if looks_like_pitch_suffix(right) {
            core = left.to_string();
        } else if !right.is_empty()
            && (inventory.vowel_by_phoneme.contains_key(left)
                || JapanesePhonemizer::extract_vowel(left).is_some())
        {
            // `_` is also a configurable VCV pad. In `a_か`, keep the
            // phoneme after the pad; in `か_C4`, keep the left side.
            core = right.to_string();
        }
    }
    if !core.contains('_') {
        let chars: Vec<char> = core.chars().collect();
        for suffix_len in [3usize, 2usize] {
            if chars.len() > suffix_len {
                let suffix: String = chars[chars.len() - suffix_len..].iter().collect();
                if looks_like_pitch_suffix(&suffix) {
                    core.truncate(core.len() - suffix.len());
                    break;
                }
            }
        }
    }
    let mut suffixes = inventory.alias_nums.iter().collect::<Vec<_>>();
    suffixes.extend(inventory.alias_appends.iter());
    suffixes.extend(inventory.alias_pitches.iter());
    for suffix in suffixes {
        if let Some(base) = core.strip_suffix(suffix) {
            core = base.to_string();
        }
    }
    core
}

fn looks_like_pitch_suffix(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some('A'..='G' | 'a'..='g'))
        && chars.all(|ch| ch.is_ascii_digit() || matches!(ch, '#' | 'b'))
}

fn extract_presamp_vowel(lyric: &str, inventory: &PresampInventory) -> Option<String> {
    let core = presamp_core_lyric(lyric, inventory);
    let trimmed = core.trim();
    if let Some(vowel) = inventory.vowel_by_phoneme.get(trimmed) {
        return Some(vowel.clone());
    }
    JapanesePhonemizer::extract_vowel(trimmed).map(str::to_owned)
}

fn extract_presamp_consonant(lyric: &str, inventory: &PresampInventory) -> Option<String> {
    let core = presamp_core_lyric(lyric, inventory);
    let trimmed = core.trim();
    if let Some(consonant) = inventory.consonant_by_phoneme.get(trimmed) {
        return Some(consonant.clone());
    }
    JapanesePhonemizer::extract_consonant(trimmed).map(str::to_owned)
}

impl Default for PresampAliasConfig {
    fn default() -> Self {
        // These are the defaults documented by Presamp/OpenUtau.
        Self {
            vcpad: " ".to_string(),
            vcvpad: " ".to_string(),
            vcv: "%v%%VCVPAD%%CV%".to_string(),
            beginning_cv: "-%VCVPAD%%CV%".to_string(),
            cross_cv: "*%VCVPAD%%CV%".to_string(),
            vc: "%v%%vcpad%%c%,%c%%vcpad%%c%".to_string(),
            cv: "%CV%,%c%%V%".to_string(),
            c: "%c%".to_string(),
            long_v: "%V%ー".to_string(),
            ending1: "%v%%VCPAD%R".to_string(),
            ending2: "-".to_string(),
        }
    }
}

fn parse_presamp_alias_config(contents: &str) -> PresampAliasConfig {
    let mut config = PresampAliasConfig::default();
    let mut section = "";
    for raw_line in contents.lines() {
        let line = raw_line.trim_end_matches('\r').trim();
        if line.starts_with('[') && line.ends_with(']') {
            section = line;
            continue;
        }
        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
            continue;
        }
        if section.eq_ignore_ascii_case("[ENDTYPE]") || section.eq_ignore_ascii_case("[ENDTYPE1]") {
            config.ending1 = line.to_string();
            continue;
        }
        if section.eq_ignore_ascii_case("[ENDTYPE2]") {
            config.ending2 = line.to_string();
            continue;
        }
        if !section.eq_ignore_ascii_case("[ALIAS]") {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim().to_ascii_uppercase().as_str() {
            "VCPAD" => config.vcpad = value.to_string(),
            "VCVPAD" => config.vcvpad = value.to_string(),
            "VCV" => config.vcv = value.to_string(),
            "BEGINING_CV" | "BEGINNING_CV" => config.beginning_cv = value.to_string(),
            "CROSS_CV" => config.cross_cv = value.to_string(),
            "VC" => config.vc = value.to_string(),
            "CV" => config.cv = value.to_string(),
            "C" => config.c = value.to_string(),
            "LONG_V" => config.long_v = value.to_string(),
            "ENDING1" => config.ending1 = value.to_string(),
            "ENDING2" => config.ending2 = value.to_string(),
            _ => {}
        }
    }
    config
}

fn load_presamp_replacements(root: &std::path::Path) -> HashMap<String, String> {
    let path = root.join("presamp.ini");
    let Ok(bytes) = fs::read(path) else {
        return HashMap::new();
    };
    let text = String::from_utf8(bytes.clone()).unwrap_or_else(|_| {
        let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
        decoded.into_owned()
    });
    parse_presamp_replacements(&text)
}

fn load_presamp_alias_config(root: &std::path::Path) -> PresampAliasConfig {
    let path = root.join("presamp.ini");
    let Ok(bytes) = fs::read(path) else {
        return PresampAliasConfig::default();
    };
    let text = String::from_utf8(bytes.clone()).unwrap_or_else(|_| {
        let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
        decoded.into_owned()
    });
    parse_presamp_alias_config(&text)
}

fn load_presamp_inventory(root: &std::path::Path) -> PresampInventory {
    let path = root.join("presamp.ini");
    let Ok(bytes) = fs::read(path) else {
        return PresampInventory::default();
    };
    let text = String::from_utf8(bytes.clone()).unwrap_or_else(|_| {
        let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
        decoded.into_owned()
    });
    parse_presamp_inventory(&text)
}

fn presamp_join(left: &str, pad: &str, right: &str) -> String {
    format!("{left}{pad}{right}")
}

fn expand_presamp_alias(
    template: &str,
    vowel: &str,
    consonant: &str,
    cv: &str,
    representative_vowel: &str,
    config: &PresampAliasConfig,
) -> String {
    template
        .replace("%VCVPAD%", &config.vcvpad)
        .replace("%VCPAD%", &config.vcpad)
        .replace("%vcpad%", &config.vcpad)
        .replace("%v%", vowel)
        .replace("%V%", representative_vowel)
        .replace("%c%", consonant)
        .replace("%CV%", cv)
}

fn expand_presamp_candidates(
    template: &str,
    vowel: &str,
    consonant: &str,
    cv: &str,
    representative_vowel: &str,
    config: &PresampAliasConfig,
) -> Vec<String> {
    template
        .split(',')
        .map(|part| expand_presamp_alias(part, vowel, consonant, cv, representative_vowel, config))
        .filter(|candidate| !candidate.is_empty())
        .collect()
}

fn long_vowel_candidate(lyric: &str, config: &PresampAliasConfig) -> Option<String> {
    let stem = lyric.strip_suffix('ー')?.trim();
    if stem.is_empty() {
        return None;
    }
    Some(expand_presamp_alias(
        &config.long_v,
        "",
        "",
        lyric,
        stem,
        config,
    ))
}

/// Converts a note lyric written as a Japanese VCV alias to its CV lyric.
///
/// VCV aliases are represented by the previous vowel followed by the note
/// lyric (for example `a か` or `- あ`).  Keeping the last token also makes
/// this safe for aliases that contain extra spacing.
pub fn vcv_to_cv_lyric(lyric: &str) -> String {
    lyric.split_whitespace().last().unwrap_or(lyric).to_string()
}

/// Converts a CV lyric to a VCV lyric using the vowel of the preceding note.
/// The caller supplies the preceding lyric from the original, unmodified
/// note list so converting a selection does not depend on iteration order.
pub fn cv_to_vcv_lyric(lyric: &str, previous_lyric: Option<&str>) -> String {
    let cv = vcv_to_cv_lyric(lyric);
    if cv.trim().is_empty() || lyric.split_whitespace().count() > 1 {
        return lyric.to_string();
    }

    let previous_vowel = previous_lyric
        .and_then(JapanesePhonemizer::extract_vowel)
        .unwrap_or("-");
    format!("{previous_vowel} {cv}")
}

/// Converts katakana to hiragana and removes every remaining non-hiragana
/// character, preserving whitespace so VCV separators remain readable.
pub fn remove_non_hiragana(lyric: &str) -> String {
    let normalized = romaji::katakana_to_hiragana(lyric);
    normalized
        .chars()
        .filter(|c| ('\u{3040}'..='\u{309F}').contains(c) || c.is_whitespace())
        .collect()
}

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
        let presamp_replacements = load_presamp_replacements(&vb.root_path);
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
                .trim();
            let raw_lyric = if note.phonetic_hint.is_none() {
                presamp_replacements
                    .get(raw_lyric)
                    .map(String::as_str)
                    .unwrap_or(raw_lyric)
            } else {
                raw_lyric
            };
            if presamp_inventory.split && (raw_lyric.contains(';') || raw_lyric.contains(',')) {
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

            let cur_vowel = extract_presamp_vowel(&exp.lyric, &presamp_inventory);
            let _cur_consonant = extract_presamp_consonant(&exp.lyric, &presamp_inventory);
            let current_core = presamp_core_lyric(&exp.lyric, &presamp_inventory);

            let is_phrase_start = match prev_note_end_ms {
                Some(end_ms) => exp.position_ms > end_ms + 0.001,
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

            // OpenUtau's Japanese presamp phonemizer treats an isolated
            // sokuon as a VC/closure phoneme, not as an ordinary CV note.
            // Prefer the explicit closure aliases and then the following
            // consonant (for example `a っ`, `a cl`, `a k`, `っ`).  This is
            // what creates the short stop before the following syllable and
            // prevents `っ` from being rendered as `u`.
            if mode == PhonemizerMode::CVVC && (exp.lyric == "っ" || exp.lyric == "ッ") {
                let mut sokuon_candidates = Vec::new();
                if let Some(vowel) = prev_vowel.as_deref() {
                    sokuon_candidates.push(presamp_join(vowel, &presamp_alias.vcpad, "っ"));
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

                if let Some(alias) = Self::find_oto_candidate(vb, &sokuon_candidates, &note.pitch) {
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
                    prev_vowel = None;
                    prev_note_end_ms = Some(exp.position_ms + exp.duration_ms);
                    continue;
                }
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

                    let alias = Self::find_oto_candidate(vb, &candidates, &note.pitch)
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
                    });
                }

                PhonemizerMode::VCV => {
                    let mut alias = exp.lyric.to_string();
                    if !is_phrase_start {
                        if let Some(pv) = prev_vowel.as_deref() {
                            let mut vcv_cands = expand_presamp_candidates(
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
                                vcv_cands.insert(0, long_alias);
                            }
                            vcv_cands.extend(expand_presamp_candidates(
                                &presamp_alias.cross_cv,
                                pv,
                                extract_presamp_consonant(&exp.lyric, &presamp_inventory)
                                    .as_deref()
                                    .unwrap_or(""),
                                &exp.lyric,
                                &exp.lyric,
                                &presamp_alias,
                            ));
                            vcv_cands.extend([
                                format!("{}_{}", pv, exp.lyric),
                                format!("{}{}", pv, exp.lyric),
                                presamp_join("*", &presamp_alias.vcvpad, &exp.lyric),
                                exp.lyric.to_string(),
                            ]);
                            if let Some(found) =
                                Self::find_oto_candidate(vb, &vcv_cands, &note.pitch)
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
                    // CVVC distinguishes vowel nuclei from consonant onsets.
                    // Treating every known consonant as a "plain vowel" made
                    // the renderer search for V-V aliases (for example
                    // `a k`) instead of the required CV alias (`ka`).
                    let is_plain_vowel = PLAIN_VOWELS.contains(&current_core.as_str())
                        || presamp_inventory.vowels.contains(&current_core);
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
                        if let Some(found) =
                            Self::find_oto_candidate(vb, &initial_cands, &note.pitch)
                        {
                            current_lyric = found;
                        }
                    } else if current_core.contains('・') {
                        let mut glottal_cands = Vec::new();
                        if let Some(pv) = prev_vowel.as_deref() {
                            glottal_cands.push(presamp_join(pv, &presamp_alias.vcvpad, &exp.lyric));
                            glottal_cands.push(presamp_join(pv, &presamp_alias.vcpad, &exp.lyric));
                        }
                        glottal_cands.extend([
                            format!("・{}{}", presamp_alias.vcpad, exp.lyric),
                            format!("・{}{}", presamp_alias.vcvpad, exp.lyric),
                            exp.lyric.to_string(),
                        ]);
                        if let Some(found) =
                            Self::find_oto_candidate(vb, &glottal_cands, &note.pitch)
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
                            Self::find_oto_candidate(vb, &priority_cands, &note.pitch)
                        {
                            current_lyric = found;
                        }
                    } else if is_plain_vowel {
                        if let Some(pv) = prev_vowel.as_deref() {
                            let mut vow_cands = expand_presamp_candidates(
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
                                vow_cands.insert(0, long_alias);
                            }
                            vow_cands.extend([
                                presamp_join("*", &presamp_alias.vcvpad, &exp.lyric),
                                format!("_{}", exp.lyric),
                                exp.lyric.to_string(),
                            ]);
                            if let Some(found) =
                                Self::find_oto_candidate(vb, &vow_cands, &note.pitch)
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
                        if let Some(found) = Self::find_oto_candidate(vb, &cv_cands, &note.pitch) {
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
                                Self::find_oto_candidate(vb, &pre_c_candidates, &note.pitch)
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
                        let next_is_plain_vowel = PLAIN_VOWELS.contains(&next_core.as_str())
                            || presamp_inventory.vowels.contains(&next_core);
                        let short_note_without_forced_vc =
                            total_phrase_duration_ms < 100.0 && !presamp_inventory.must_vc;

                        if is_next_adjacent
                            && !short_note_without_forced_vc
                            && !next_already_has_transition
                            && !next_is_plain_vowel
                        {
                            if let (Some(vow), Some(con)) = (
                                cur_vowel.as_deref(),
                                extract_presamp_consonant(&next_exp.lyric, &presamp_inventory),
                            ) {
                                let mut vc_candidates = expand_presamp_candidates(
                                    &presamp_alias.vc,
                                    &vow,
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
                                        &vow,
                                        sub_con,
                                        &next_exp.lyric,
                                        &next_exp.lyric,
                                        &presamp_alias,
                                    ));
                                    vc_candidates.push(format!("{}{}", vow, sub_con));
                                }

                                if let Some(vc_alias) =
                                    Self::find_oto_candidate(vb, &vc_candidates, &note.pitch)
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
                                phones.push(RenderPhone {
                                    note_index,
                                    lyric: pre_c_alias,
                                    pitch: note.pitch.clone(),
                                    position_ms: exp.position_ms,
                                    duration_ms: pre_c_len,
                                    envelope: note.envelope.clone(),
                                    expressions: note.expressions.clone(),
                                    pitch_bend: note.pitch_bend.clone(),
                                    vibrato: note.vibrato.clone(),
                                    flags: note.flags.clone(),
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
                        });
                    } else {
                        let requested_pre_c_len =
                            pre_c_to_insert.as_ref().map(|(_, len)| *len).unwrap_or(0.0);
                        let pre_c_len = requested_pre_c_len.min((exp.duration_ms - 10.0).max(0.0));
                        let main_duration = (exp.duration_ms - pre_c_len).max(10.0);
                        if let Some((pre_c_alias, _)) = pre_c_to_insert {
                            if pre_c_len > 0.0 {
                                phones.push(RenderPhone {
                                    note_index,
                                    lyric: pre_c_alias,
                                    pitch: note.pitch.clone(),
                                    position_ms: exp.position_ms,
                                    duration_ms: pre_c_len,
                                    envelope: note.envelope.clone(),
                                    expressions: note.expressions.clone(),
                                    pitch_bend: note.pitch_bend.clone(),
                                    vibrato: note.vibrato.clone(),
                                    flags: note.flags.clone(),
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
                        });
                    }
                }

                _ => {}
            }

            prev_vowel = cur_vowel;
            prev_note_end_ms = Some(exp.position_ms + exp.duration_ms);
        }

        // Presamp's ending aliases are optional: only replace the final
        // rendered phone when the configured ENDING1/ENDING2 alias exists in
        // the voicebank. A missing tail recording must never invalidate an
        // otherwise valid note.
        if matches!(
            mode,
            PhonemizerMode::BasicCV | PhonemizerMode::VCV | PhonemizerMode::CVVC
        ) && presamp_inventory.add_ending != 0
        {
            for (idx, exp) in expanded.iter().enumerate() {
                let Some(next) = expanded.get(idx + 1) else {
                    continue;
                };
                if next.lyric != "R"
                    || (next.position_ms - (exp.position_ms + exp.duration_ms)).abs() > 0.001
                {
                    continue;
                }
                let Some(vowel) = Self::extract_vowel(&exp.lyric) else {
                    continue;
                };
                let mut candidates = expand_presamp_candidates(
                    &presamp_alias.ending1,
                    vowel,
                    "",
                    "R",
                    &exp.lyric,
                    &presamp_alias,
                );
                candidates.extend(expand_presamp_candidates(
                    &presamp_alias.ending2,
                    vowel,
                    "",
                    "R",
                    &exp.lyric,
                    &presamp_alias,
                ));
                let Some(phone_index) = phones
                    .iter()
                    .enumerate()
                    .filter(|(_, phone)| phone.note_index == exp.note_index)
                    .max_by(|(_, left), (_, right)| left.position_ms.total_cmp(&right.position_ms))
                    .map(|(phone_index, _)| phone_index)
                else {
                    continue;
                };
                if let Some(alias) =
                    Self::find_oto_candidate(vb, &candidates, &phones[phone_index].pitch)
                {
                    phones[phone_index].lyric = alias;
                }
            }
        }

        phones
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oto::OtoEntry;
    use std::path::PathBuf;

    #[test]
    fn note_lyric_conversions_handle_vcv_and_hiragana_cleanup() {
        assert_eq!(vcv_to_cv_lyric("a か"), "か");
        assert_eq!(vcv_to_cv_lyric("- あ"), "あ");
        assert_eq!(cv_to_vcv_lyric("さ", Some("か")), "a さ");
        assert_eq!(cv_to_vcv_lyric("あ", None), "- あ");
        assert_eq!(cv_to_vcv_lyric("a さ", Some("か")), "a さ");
        assert_eq!(remove_non_hiragana("KA カ か!"), " か か");
    }

    #[test]
    fn presamp_replace_section_uses_exact_matches_only() {
        let replacements = parse_presamp_replacements(
            "[VOWEL]\na=あ\n[REPLACE]\nka=か\n っ = cl\n; ignored=x\n[ALIAS]\nVC=a k\n",
        );
        assert_eq!(replacements.get("ka"), Some(&"か".to_string()));
        assert_eq!(replacements.get("っ"), Some(&"cl".to_string()));
        assert!(!replacements.contains_key("k"));
    }

    #[test]
    fn presamp_alias_section_overrides_default_padding() {
        let config = parse_presamp_alias_config("[ALIAS]\nVCPAD=-\nVCVPAD=_\n[REPLACE]\nka=か\n");
        assert_eq!(
            config,
            PresampAliasConfig {
                vcpad: "-".to_string(),
                vcvpad: "_".to_string(),
                ..PresampAliasConfig::default()
            }
        );
        assert_eq!(presamp_join("a", &config.vcpad, "k"), "a-k");
        assert_eq!(presamp_join("a", &config.vcvpad, "か"), "a_か");
        assert_eq!(
            expand_presamp_alias(&config.vc, "a", "k", "か", "あ", &config),
            "a-k,k-k"
        );

        let ending = parse_presamp_alias_config("[ENDTYPE]\na R\n[ENDTYPE2]\n-R\n");
        assert_eq!(ending.ending1, "a R");
        assert_eq!(ending.ending2, "-R");
    }

    #[test]
    fn presamp_inventory_reads_vowels_consonants_and_priority() {
        let defaults = parse_presamp_inventory("");
        assert!(defaults.priority.contains("k"));
        assert!(defaults.no_crossfade.contains("k"));

        let inventory = parse_presamp_inventory(
            "[VOWEL]\na=あ=か,が=100\nn=ん=ん=100\n[CONSONANT]\nk=か,く,け,こ=1\ns=さ,す,せ,そ=0\n[PRIORITY]\nk\nっ\n",
        );
        assert!(inventory.vowels.contains("か"));
        assert_eq!(inventory.vowel_by_phoneme.get("が"), Some(&"a".to_string()));
        assert!(inventory.consonants.contains("す"));
        assert_eq!(
            inventory.consonant_by_phoneme.get("か"),
            Some(&"k".to_string())
        );
        assert_eq!(presamp_core_lyric("か2", &inventory), "か");
        assert_eq!(presamp_core_lyric("か強", &inventory), "か");
        assert_eq!(presamp_core_lyric("か_C4", &inventory), "か");
        assert_eq!(presamp_core_lyric("かC4", &inventory), "か");
        assert_eq!(presamp_core_lyric("かC#4", &inventory), "か");
        assert_eq!(presamp_core_lyric("a_か", &inventory), "か");
        assert!(inventory.priority.contains("k"));
        assert!(inventory.priority.contains("っ"));
        assert!(inventory.no_crossfade.contains("k"));
        assert!(inventory.vc_length_from_cv);

        let fixed = parse_presamp_inventory("[VCLENGTH]\n1\n");
        assert!(!fixed.vc_length_from_cv);

        let forced = parse_presamp_inventory("[MUSTVC]\n1\n");
        assert!(forced.must_vc);

        let disabled = parse_presamp_inventory("[ENDFLAG]\n0\n");
        assert_eq!(disabled.add_ending, 0);

        let no_split = parse_presamp_inventory("[SPLIT]\n0\n");
        assert!(!no_split.split);
    }

    #[test]
    fn sokuon_is_a_closure_without_a_vowel() {
        assert_eq!(JapanesePhonemizer::extract_vowel("っ"), None);
        assert_eq!(JapanesePhonemizer::extract_vowel("ッ"), None);
        assert_eq!(JapanesePhonemizer::extract_consonant("っ"), Some("cl"));
        assert_eq!(JapanesePhonemizer::extract_vowel("っか"), Some("a"));
    }

    #[test]
    fn japanese_mora_n_preserves_openutau_hiragana_katakana_aliases() {
        assert_eq!(JapanesePhonemizer::extract_vowel("ん"), Some("n"));
        assert_eq!(JapanesePhonemizer::extract_vowel("n"), Some("n"));
        assert_eq!(JapanesePhonemizer::extract_vowel("ン"), Some("N"));
        assert_eq!(JapanesePhonemizer::extract_vowel("N"), Some("N"));
    }

    #[test]
    fn long_vowels_keep_their_preceding_mora_vowel() {
        assert_eq!(JapanesePhonemizer::extract_vowel("あー"), Some("a"));
        assert_eq!(JapanesePhonemizer::extract_vowel("おー"), Some("o"));
        let config = PresampAliasConfig::default();
        assert_eq!(
            long_vowel_candidate("あー", &config),
            Some("あー".to_string())
        );
    }

    #[test]
    fn ending_alias_is_used_only_when_the_voicebank_contains_it() {
        let mut vb = build_test_voicebank();
        vb.entries.insert(
            "a R".to_string(),
            OtoEntry::new(
                "a-r.wav".to_string(),
                "a R".to_string(),
                0.0,
                30.0,
                5.0,
                0.0,
                0.0,
            ),
        );
        let notes = vec![
            UNote::new("か", "C4", 0.0, 300.0),
            UNote::new("R", "C4", 300.0, 100.0),
        ];
        let phones = JapanesePhonemizer::apply_japanese(&notes, &vb, PhonemizerMode::VCV);
        assert_eq!(phones.last().map(|phone| phone.lyric.as_str()), Some("a R"));
    }

    #[test]
    fn glottal_stop_is_not_used_as_a_vowel_transition() {
        assert_eq!(JapanesePhonemizer::extract_vowel("・"), None);
        assert_eq!(JapanesePhonemizer::extract_consonant("・"), Some("・"));
    }

    #[test]
    fn japanese_phonetic_hint_is_used_before_lyric_conversion() {
        let mut vb = build_test_voicebank();
        vb.entries.insert(
            "a か".to_string(),
            OtoEntry::new(
                "a-ka.wav".to_string(),
                "a か".to_string(),
                0.0,
                50.0,
                10.0,
                0.0,
                0.0,
            ),
        );
        let mut note = UNote::new("か", "C4", 0.0, 300.0);
        note.phonetic_hint = Some("a か".to_string());

        let phones = JapanesePhonemizer::apply_japanese(&[note], &vb, PhonemizerMode::VCV);
        assert_eq!(phones[0].lyric, "a か");
    }

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
    fn japanese_cvvc_inserts_phrase_head_consonant_when_available() {
        let mut vb = build_test_voicebank();
        vb.entries.insert(
            "- s".to_string(),
            OtoEntry::new(
                "-s.wav".to_string(),
                "- s".to_string(),
                0.0,
                35.0,
                5.0,
                0.0,
                0.0,
            ),
        );
        let phones = JapanesePhonemizer::apply_japanese(
            &[UNote::new("さ", "C4", 0.0, 400.0)],
            &vb,
            PhonemizerMode::CVVC,
        );
        assert_eq!(phones.len(), 2);
        assert_eq!(phones[0].lyric, "- s");
        assert_eq!(phones[1].lyric, "さ");
        assert!(phones[1].position_ms > phones[0].position_ms);
    }

    #[test]
    fn japanese_cvvc_renders_sokuon_as_a_closure_before_next_consonant() {
        let mut vb = build_test_voicebank();
        vb.entries.insert(
            "a k".to_string(),
            OtoEntry::new(
                "ak.wav".to_string(),
                "a k".to_string(),
                0.0,
                30.0,
                5.0,
                0.0,
                0.0,
            ),
        );
        let notes = vec![
            UNote::new("か", "C4", 0.0, 300.0),
            UNote::new("っ", "C4", 300.0, 100.0),
            UNote::new("か", "C4", 400.0, 300.0),
        ];

        let phones = JapanesePhonemizer::apply_japanese(&notes, &vb, PhonemizerMode::CVVC);
        assert!(phones.iter().any(|phone| phone.lyric == "a k"));
        assert!(!phones.iter().any(|phone| phone.lyric == "u"));
    }

    #[test]
    fn japanese_cvvc_does_not_repeat_the_transition_with_a_vcv_alias() {
        let mut vb = build_test_voicebank();
        vb.entries.insert(
            "a さ".to_string(),
            OtoEntry::new(
                "asa.wav".to_string(),
                "a さ".to_string(),
                0.0,
                100.0,
                -400.0,
                80.0,
                30.0,
            ),
        );
        let notes = vec![
            UNote::new("か", "C4", 0.0, 400.0),
            UNote::new("さ", "C4", 400.0, 400.0),
        ];

        let phones = JapanesePhonemizer::apply_japanese(&notes, &vb, PhonemizerMode::CVVC);
        assert_eq!(
            phones
                .iter()
                .map(|phone| phone.lyric.as_str())
                .collect::<Vec<_>>(),
            ["- か", "a s", "さ"]
        );
    }

    #[test]
    fn japanese_cvvc_uses_plain_cv_timing_instead_of_phrase_head_timing() {
        let mut vb = build_test_voicebank();
        vb.entries.insert(
            "- さ".to_string(),
            OtoEntry::new(
                "head-sa.wav".to_string(),
                "- さ".to_string(),
                0.0,
                300.0,
                -500.0,
                300.0,
                100.0,
            ),
        );
        let plain = vb.entries.get_mut("さ").unwrap();
        plain.preutterance = 60.0;
        plain.overlap = 20.0;
        let notes = vec![
            UNote::new("か", "C4", 0.0, 400.0),
            UNote::new("さ", "C4", 400.0, 400.0),
        ];

        let phones = JapanesePhonemizer::apply_japanese(&notes, &vb, PhonemizerMode::CVVC);
        assert_eq!(phones[1].lyric, "a s");
        assert!((phones[1].position_ms - 340.0).abs() < 1e-6);
        assert!((phones[1].duration_ms - 60.0).abs() < 1e-6);
    }

    #[test]
    fn japanese_cvvc_resolves_the_vc_with_the_outgoing_note_subbank() {
        let mut vb = build_test_voicebank();
        vb.prefix_map = crate::oto::PrefixMap::parse_yaml_str(
            "subbanks:\n  - suffix: _C4\n    tone_ranges: [C4-C4]\n  - suffix: _D4\n    tone_ranges: [D4-D4]\n",
        );
        for alias in ["- か_C4", "a s_C4", "さ_D4"] {
            vb.entries.insert(
                alias.to_string(),
                OtoEntry::new(
                    format!("{alias}.wav"),
                    alias.to_string(),
                    0.0,
                    80.0,
                    -400.0,
                    60.0,
                    20.0,
                ),
            );
        }
        let notes = vec![
            UNote::new("か", "C4", 0.0, 400.0),
            UNote::new("さ", "D4", 400.0, 400.0),
        ];

        let phones = JapanesePhonemizer::apply_japanese(&notes, &vb, PhonemizerMode::CVVC);
        assert_eq!(phones[1].lyric, "a s_C4");
        assert_eq!(phones[1].pitch, "C4");
        assert_eq!(phones[2].lyric, "さ_D4");
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

    #[test]
    fn test_cvvc_keeps_cv_after_the_inserted_vc_even_when_vcv_exists() {
        let mut vb = build_test_voicebank();
        vb.entries.insert(
            "a さ".to_string(),
            OtoEntry::new(
                "asa.wav".to_string(),
                "a さ".to_string(),
                0.0,
                50.0,
                10.0,
                0.0,
                0.0,
            ),
        );
        let notes = vec![
            UNote::new("あ", "C4", 0.0, 400.0),
            UNote::new("さ", "C4", 400.0, 400.0),
        ];
        let phones = JapanesePhonemizer::apply_japanese(&notes, &vb, PhonemizerMode::CVVC);
        assert_eq!(phones[1].lyric, "a s");
        assert_eq!(phones[2].lyric, "さ");
    }

    #[test]
    fn test_cv_vv_alias_is_preferred_when_available() {
        let mut vb = build_test_voicebank();
        vb.entries.insert(
            "a い".to_string(),
            OtoEntry::new(
                "ai.wav".to_string(),
                "a い".to_string(),
                0.0,
                50.0,
                10.0,
                0.0,
                0.0,
            ),
        );
        let notes = vec![
            UNote::new("あ", "C4", 0.0, 400.0),
            UNote::new("い", "C4", 400.0, 400.0),
        ];
        let phones = JapanesePhonemizer::apply_japanese(&notes, &vb, PhonemizerMode::BasicCV);
        assert_eq!(phones[1].lyric, "a い");
    }

    #[test]
    fn basic_cv_matches_openutau_vcv_precedence() {
        let mut vb = build_test_voicebank();
        vb.entries.insert(
            "a さ".to_string(),
            OtoEntry::new(
                "a-sa.wav".to_string(),
                "a さ".to_string(),
                0.0,
                50.0,
                10.0,
                0.0,
                0.0,
            ),
        );
        let notes = vec![
            UNote::new("あ", "C4", 0.0, 400.0),
            UNote::new("さ", "C4", 400.0, 400.0),
        ];

        let phones = JapanesePhonemizer::apply_japanese(&notes, &vb, PhonemizerMode::BasicCV);
        assert_eq!(phones[0].lyric, "- あ");
        assert_eq!(phones[1].lyric, "a さ");
    }

    #[test]
    fn test_cvvc_romaji_notes_converted_and_matched() {
        let vb = build_test_voicebank();
        // Notes entered in Romaji: "ka", "sa"
        let notes = vec![
            UNote::new("ka", "C4", 0.0, 400.0),
            UNote::new("sa", "C4", 400.0, 400.0),
        ];

        let phones = JapanesePhonemizer::apply_japanese(&notes, &vb, PhonemizerMode::CVVC);
        assert_eq!(phones.len(), 3);
        assert_eq!(phones[0].lyric, "- か", "Romaji 'ka' should match '- か'");
        assert_eq!(phones[1].lyric, "a s", "VC should be 'a s'");
        assert_eq!(phones[2].lyric, "さ", "Romaji 'sa' should match 'さ'");
    }

    #[test]
    fn test_cvvc_hiragana_vc_matching() {
        let mut vb = build_test_voicebank();
        vb.entries.remove("a s");
        vb.entries.insert(
            "あ s".to_string(),
            OtoEntry::new(
                "as.wav".to_string(),
                "あ s".to_string(),
                0.0,
                30.0,
                5.0,
                0.0,
                0.0,
            ),
        );

        let notes = vec![
            UNote::new("ka", "C4", 0.0, 400.0),
            UNote::new("sa", "C4", 400.0, 400.0),
        ];

        let phones = JapanesePhonemizer::apply_japanese(&notes, &vb, PhonemizerMode::CVVC);
        assert_eq!(phones.len(), 3);
        assert_eq!(phones[0].lyric, "- か");
        assert_eq!(
            phones[1].lyric, "あ s",
            "Should find 'あ s' alias in voicebank"
        );
        assert_eq!(phones[2].lyric, "さ");
    }
}
