use std::collections::HashMap;

pub struct EnglishPhonemizer;

impl EnglishPhonemizer {
    pub const ALL_ARPABET_PHONES: &'static [&'static str] = &[
        "aa", "ae", "ah", "ao", "aw", "ay", "b", "ch", "d", "dh", "eh", "er", "ey", "f", "g", "hh",
        "ih", "iy", "jh", "k", "l", "m", "n", "ng", "ow", "oy", "p", "r", "s", "sh", "t", "th",
        "uh", "uw", "v", "w", "y", "z", "zh",
    ];

    pub fn is_arpabet(token: &str) -> bool {
        let clean = token.trim().to_lowercase();
        Self::ALL_ARPABET_PHONES.contains(&clean.as_str())
    }

    pub fn phonetic_tokens(input: &str) -> Vec<String> {
        let clean = input.trim().to_lowercase();
        if clean.is_empty() {
            return Vec::new();
        }
        if clean.contains(' ') || clean.contains('-') {
            return clean
                .split(|c| c == ' ' || c == '-')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();
        }
        vec![clean]
    }

    pub fn word_to_arpabet(word: &str) -> Vec<String> {
        let clean = word.trim().to_lowercase();
        if clean.is_empty() {
            return Vec::new();
        }

        if clean.contains(' ') || clean.contains('-') {
            return clean
                .split(|c| c == ' ' || c == '-')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();
        }

        let dict: HashMap<&str, &[&str]> = HashMap::from([
            ("a", &["ah"][..]),
            ("the", &["dh", "ah"]),
            ("to", &["t", "uw"]),
            ("and", &["ae", "n", "d"]),
            ("in", &["ih", "n"]),
            ("is", &["ih", "z"]),
            ("you", &["y", "uw"]),
            ("that", &["dh", "ae", "t"]),
            ("it", &["ih", "t"]),
            ("he", &["hh", "iy"]),
            ("was", &["w", "aa", "z"]),
            ("for", &["f", "ao", "r"]),
            ("on", &["aa", "n"]),
            ("are", &["aa", "r"]),
            ("as", &["ae", "z"]),
            ("with", &["w", "ih", "dh"]),
            ("his", &["hh", "ih", "z"]),
            ("they", &["dh", "ey"]),
            ("i", &["ay"]),
            ("at", &["ae", "t"]),
            ("be", &["b", "iy"]),
            ("this", &["dh", "ih", "s"]),
            ("have", &["hh", "ae", "v"]),
            ("from", &["f", "r", "ah", "m"]),
            ("or", &["ao", "r"]),
            ("one", &["w", "ah", "n"]),
            ("had", &["hh", "ae", "d"]),
            ("by", &["b", "ay"]),
            ("word", &["w", "er", "d"]),
            ("but", &["b", "ah", "t"]),
            ("not", &["n", "aa", "t"]),
            ("what", &["w", "ah", "t"]),
            ("all", &["ao", "l"]),
            ("were", &["w", "er"]),
            ("we", &["w", "iy"]),
            ("when", &["w", "eh", "n"]),
            ("your", &["y", "ao", "r"]),
            ("can", &["k", "ae", "n"]),
            ("said", &["s", "eh", "d"]),
            ("there", &["dh", "eh", "r"]),
            ("use", &["y", "uw", "z"]),
            ("an", &["ae", "n"]),
            ("each", &["iy", "ch"]),
            ("which", &["w", "ih", "ch"]),
            ("she", &["sh", "iy"]),
            ("do", &["d", "uw"]),
            ("how", &["hh", "aw"]),
            ("their", &["dh", "eh", "r"]),
            ("if", &["ih", "f"]),
            ("will", &["w", "ih", "l"]),
            ("up", &["ah", "p"]),
            ("other", &["ah", "dh", "er"]),
            ("about", &["ah", "b", "aw", "t"]),
            ("out", &["aw", "t"]),
            ("many", &["m", "eh", "n", "iy"]),
            ("then", &["dh", "eh", "n"]),
            ("them", &["dh", "eh", "m"]),
            ("these", &["dh", "iy", "z"]),
            ("so", &["s", "ow"]),
            ("some", &["s", "ah", "m"]),
            ("her", &["hh", "er"]),
            ("would", &["w", "uh", "d"]),
            ("make", &["m", "ey", "k"]),
            ("like", &["l", "ay", "k"]),
            ("him", &["hh", "ih", "m"]),
            ("into", &["ih", "n", "t", "uw"]),
            ("time", &["t", "ay", "m"]),
            ("has", &["hh", "ae", "z"]),
            ("look", &["l", "uh", "k"]),
            ("two", &["t", "uw"]),
            ("more", &["m", "ao", "r"]),
            ("write", &["r", "ay", "t"]),
            ("go", &["g", "ow"]),
            ("see", &["s", "iy"]),
            ("no", &["n", "ow"]),
            ("way", &["w", "ey"]),
            ("could", &["k", "uh", "d"]),
            ("people", &["p", "iy", "p", "ah", "l"]),
            ("my", &["m", "ay"]),
            ("than", &["dh", "ae", "n"]),
            ("first", &["f", "er", "s", "t"]),
            ("water", &["w", "ao", "t", "er"]),
            ("been", &["b", "ih", "n"]),
            ("call", &["k", "ao", "l"]),
            ("who", &["hh", "uw"]),
            ("oil", &["oy", "l"]),
            ("its", &["ih", "t", "s"]),
            ("now", &["n", "aw"]),
            ("find", &["f", "ay", "n", "d"]),
            ("long", &["l", "ao", "ng"]),
            ("down", &["d", "aw", "n"]),
            ("day", &["d", "ey"]),
            ("did", &["d", "ih", "d"]),
            ("get", &["g", "eh", "t"]),
            ("come", &["k", "ah", "m"]),
            ("made", &["m", "ey", "d"]),
            ("may", &["m", "ey"]),
            ("part", &["p", "aa", "r", "t"]),
            ("love", &["l", "ah", "v"]),
            ("sing", &["s", "ih", "ng"]),
            ("song", &["s", "ao", "ng"]),
            ("voice", &["v", "oy", "s"]),
            ("heart", &["hh", "aa", "r", "t"]),
            ("world", &["w", "er", "l", "d"]),
            ("hello", &["hh", "ah", "l", "ow"]),
            ("night", &["n", "ay", "t"]),
            ("light", &["l", "ay", "t"]),
            ("sky", &["s", "k", "ay"]),
            ("sun", &["s", "ah", "n"]),
            ("dream", &["d", "r", "iy", "m"]),
            ("star", &["s", "t", "aa", "r"]),
            ("eyes", &["ay", "z"]),
            ("life", &["l", "ay", "f"]),
            ("feel", &["f", "iy", "l"]),
            ("fly", &["f", "l", "ay"]),
            ("free", &["f", "r", "iy"]),
        ]);

        if let Some(phones) = dict.get(clean.as_str()) {
            return phones.iter().map(|&s| s.to_string()).collect();
        }

        let mut result = Vec::new();
        let chars: Vec<char> = clean.chars().collect();
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            match c {
                'a' => {
                    if i + 1 < chars.len() && chars[i + 1] == 'y' {
                        result.push("ey".to_string());
                        i += 1;
                    } else if i + 1 < chars.len() && chars[i + 1] == 'w' {
                        result.push("ao".to_string());
                        i += 1;
                    } else {
                        result.push("ae".to_string());
                    }
                }
                'e' => {
                    if i + 1 < chars.len() && chars[i + 1] == 'e' {
                        result.push("iy".to_string());
                        i += 1;
                    } else {
                        result.push("eh".to_string());
                    }
                }
                'i' => result.push("ih".to_string()),
                'o' => {
                    if i + 1 < chars.len() && chars[i + 1] == 'o' {
                        result.push("uw".to_string());
                        i += 1;
                    } else if i + 1 < chars.len() && chars[i + 1] == 'u' {
                        result.push("aw".to_string());
                        i += 1;
                    } else {
                        result.push("aa".to_string());
                    }
                }
                'u' => result.push("ah".to_string()),
                'b' => result.push("b".to_string()),
                'c' => {
                    if i + 1 < chars.len() && chars[i + 1] == 'h' {
                        result.push("ch".to_string());
                        i += 1;
                    } else {
                        result.push("k".to_string());
                    }
                }
                'd' => result.push("d".to_string()),
                'f' => result.push("f".to_string()),
                'g' => result.push("g".to_string()),
                'h' => result.push("hh".to_string()),
                'j' => result.push("jh".to_string()),
                'k' => result.push("k".to_string()),
                'l' => result.push("l".to_string()),
                'm' => result.push("m".to_string()),
                'n' => {
                    if i + 1 < chars.len() && chars[i + 1] == 'g' {
                        result.push("ng".to_string());
                        i += 1;
                    } else {
                        result.push("n".to_string());
                    }
                }
                'p' => {
                    if i + 1 < chars.len() && chars[i + 1] == 'h' {
                        result.push("f".to_string());
                        i += 1;
                    } else {
                        result.push("p".to_string());
                    }
                }
                'r' => result.push("r".to_string()),
                's' => {
                    if i + 1 < chars.len() && chars[i + 1] == 'h' {
                        result.push("sh".to_string());
                        i += 1;
                    } else {
                        result.push("s".to_string());
                    }
                }
                't' => {
                    if i + 1 < chars.len() && chars[i + 1] == 'h' {
                        result.push("th".to_string());
                        i += 1;
                    } else {
                        result.push("t".to_string());
                    }
                }
                'v' => result.push("v".to_string()),
                'w' => result.push("w".to_string()),
                'y' => result.push("y".to_string()),
                'z' => result.push("z".to_string()),
                _ => {}
            }
            i += 1;
        }

        if result.is_empty() {
            vec![clean]
        } else {
            result
        }
    }

    pub fn is_vowel(phone: &str) -> bool {
        matches!(
            phone,
            "aa" | "ae"
                | "ah"
                | "ao"
                | "aw"
                | "ay"
                | "eh"
                | "er"
                | "ey"
                | "ih"
                | "iy"
                | "ow"
                | "oy"
                | "uh"
                | "uw"
                | "a"
                | "e"
                | "i"
                | "o"
                | "u"
        )
    }
}
