pub struct PortuguesePhonemizer;

impl PortuguesePhonemizer {
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

    pub fn word_to_phonemes(word: &str) -> Vec<String> {
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

        let mut tokens = Vec::new();
        let chars: Vec<char> = clean.chars().collect();
        let len = chars.len();
        let mut i = 0;

        while i < len {
            let c = chars[i];
            let next = if i + 1 < len {
                Some(chars[i + 1])
            } else {
                None
            };
            let next2 = if i + 2 < len {
                Some(chars[i + 2])
            } else {
                None
            };

            if c == 'c' && next == Some('h') {
                if let Some(v) = next2.and_then(Self::char_to_vowel) {
                    tokens.push(format!("x{}", v));
                    i += 3;
                    continue;
                } else {
                    tokens.push("x".to_string());
                    i += 2;
                    continue;
                }
            } else if c == 'n' && next == Some('h') {
                if let Some(v) = next2.and_then(Self::char_to_vowel) {
                    tokens.push(format!("nh{}", v));
                    i += 3;
                    continue;
                } else {
                    tokens.push("nh".to_string());
                    i += 2;
                    continue;
                }
            } else if c == 'l' && next == Some('h') {
                if let Some(v) = next2.and_then(Self::char_to_vowel) {
                    tokens.push(format!("lh{}", v));
                    i += 3;
                    continue;
                } else {
                    tokens.push("lh".to_string());
                    i += 2;
                    continue;
                }
            } else if c == 'r' && next == Some('r') {
                if let Some(v) = next2.and_then(Self::char_to_vowel) {
                    tokens.push(format!("rr{}", v));
                    i += 3;
                    continue;
                } else {
                    tokens.push("rr".to_string());
                    i += 2;
                    continue;
                }
            } else if c == 's' && next == Some('s') {
                if let Some(v) = next2.and_then(Self::char_to_vowel) {
                    tokens.push(format!("s{}", v));
                    i += 3;
                    continue;
                } else {
                    tokens.push("s".to_string());
                    i += 2;
                    continue;
                }
            } else if (c == 'q' || c == 'g') && next == Some('u') {
                if let Some(v) = next2.and_then(Self::char_to_vowel) {
                    if c == 'q' {
                        if v == 'e' || v == 'i' {
                            tokens.push(format!("k{}", v));
                        } else {
                            tokens.push(format!("ku{}", v));
                        }
                    } else {
                        if v == 'e' || v == 'i' {
                            tokens.push(format!("g{}", v));
                        } else {
                            tokens.push(format!("gu{}", v));
                        }
                    }
                    i += 3;
                    continue;
                }
            }

            if !Self::is_vowel_char(c) {
                if let Some(v) = next.and_then(Self::char_to_vowel) {
                    let mapped_c = match c {
                        'c' => {
                            if v == 'e' || v == 'i' {
                                "s"
                            } else {
                                "k"
                            }
                        }
                        'ç' => "s",
                        'k' => "k",
                        'g' => {
                            if v == 'e' || v == 'i' {
                                "j"
                            } else {
                                "g"
                            }
                        }
                        'j' => "j",
                        'x' => "x",
                        _ => match c {
                            'b' => "b",
                            'd' => "d",
                            'f' => "f",
                            'h' => "",
                            'l' => "l",
                            'm' => "m",
                            'n' => "n",
                            'p' => "p",
                            'r' => "r",
                            's' => "s",
                            't' => "t",
                            'v' => "v",
                            'z' => "z",
                            _ => "",
                        },
                    };

                    if mapped_c.is_empty() {
                        tokens.push(v.to_string());
                    } else {
                        tokens.push(format!("{}{}", mapped_c, v));
                    }
                    i += 2;
                    continue;
                } else {
                    let coda = match c {
                        's' | 'z' | 'x' => "s",
                        'r' => "r",
                        'l' => "w", // No PT-BR 'l' final vira 'w' (ex: sol -> sow)
                        'm' | 'n' => "n",
                        'p' => "p",
                        't' => "t",
                        'k' | 'c' => "k",
                        _ => "s",
                    };
                    tokens.push(coda.to_string());
                    i += 1;
                    continue;
                }
            }

            if let Some(v) = Self::char_to_vowel(c) {
                tokens.push(v.to_string());
            }
            i += 1;
        }

        if tokens.is_empty() {
            vec![clean]
        } else {
            tokens
        }
    }

    pub fn is_vowel_char(c: char) -> bool {
        matches!(
            c,
            'a' | 'e'
                | 'i'
                | 'o'
                | 'u'
                | 'á'
                | 'é'
                | 'í'
                | 'ó'
                | 'ú'
                | 'â'
                | 'ê'
                | 'î'
                | 'ô'
                | 'û'
                | 'ã'
                | 'õ'
                | 'à'
                | 'è'
                | 'ì'
                | 'ò'
                | 'ù'
        )
    }

    pub fn char_to_vowel(c: char) -> Option<char> {
        match c {
            'a' | 'á' | 'â' | 'à' => Some('a'),
            'e' | 'é' | 'ê' | 'è' => Some('e'),
            'i' | 'í' | 'î' | 'ì' => Some('i'),
            'o' | 'ó' | 'ô' | 'ò' => Some('o'),
            'u' | 'ú' | 'û' | 'ù' => Some('u'),
            'ã' => Some('a'),
            'õ' => Some('o'),
            _ => None,
        }
    }

    pub fn extract_vowel(token: &str) -> Option<&'static str> {
        let t = token.trim();
        if t.ends_with('a') {
            Some("a")
        } else if t.ends_with('e') {
            Some("e")
        } else if t.ends_with('i') {
            Some("i")
        } else if t.ends_with('o') {
            Some("o")
        } else if t.ends_with('u') || t.ends_with('w') {
            Some("u")
        } else {
            None
        }
    }

    pub fn extract_consonant(token: &str) -> Option<&'static str> {
        let t = token.trim();
        if t.starts_with("ch") || t.starts_with("sh") || t.starts_with('x') {
            Some("sh")
        } else if t.starts_with("nh") || t.starts_with("ny") {
            Some("nh")
        } else if t.starts_with("lh") || t.starts_with("ly") {
            Some("lh")
        } else if t.starts_with("rr") {
            Some("rr")
        } else if t.starts_with('k') || t.starts_with('c') {
            Some("k")
        } else if t.starts_with('s') || t.starts_with('z') {
            Some("s")
        } else if t.starts_with('t') {
            Some("t")
        } else if t.starts_with('d') {
            Some("d")
        } else if t.starts_with('p') {
            Some("p")
        } else if t.starts_with('b') {
            Some("b")
        } else if t.starts_with('f') {
            Some("f")
        } else if t.starts_with('v') {
            Some("v")
        } else if t.starts_with('m') {
            Some("m")
        } else if t.starts_with('n') {
            Some("n")
        } else if t.starts_with('l') {
            Some("l")
        } else if t.starts_with('r') {
            Some("r")
        } else if t.starts_with('j') || t.starts_with('g') {
            Some("j")
        } else {
            None
        }
    }
}
