/// Tokeniza flags UTAU sem confundir flags de um e dois caracteres.
#[derive(Debug, Clone, PartialEq)]
pub struct UtauFlagToken {
    pub key: String,
    pub num_val: Option<f64>,
}

pub fn parse_utau_flags(flags: &str) -> Vec<UtauFlagToken> {
    let chars: Vec<char> = flags.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() || chars[i] == '/' {
            i += 1;
            continue;
        }
        if !chars[i].is_alphabetic() {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        if i < chars.len() && chars[i].is_ascii_lowercase() {
            let candidate: String = chars[start..=i].iter().collect();
            if matches!(
                candidate.as_str(),
                "Hb" | "Mb" | "Mo" | "Me" | "Mt" | "Ms" | "Nh" | "Te"
            ) {
                i += 1;
            }
        }
        let key: String = chars[start..i].iter().collect();
        let number_start = i;
        if i < chars.len() && matches!(chars[i], '+' | '-') {
            i += 1;
        }
        while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
            i += 1;
        }
        let num_val = if i > number_start {
            chars[number_start..i]
                .iter()
                .collect::<String>()
                .parse()
                .ok()
        } else {
            None
        };
        tokens.push(UtauFlagToken { key, num_val });
    }
    tokens
}

/// Merges UTAU flags from least to most specific scope. The last value for a
/// known token wins, while tokens unknown to Kamafeu are retained verbatim.
/// This is intentionally independent from a particular resampler so the GUI,
/// preview and final renderer share the same precedence rule.
pub fn merge_utau_flags(scopes: &[&str]) -> String {
    let mut merged: Vec<UtauFlagToken> = Vec::new();
    for scope in scopes {
        for token in parse_utau_flags(scope) {
            if let Some(previous) = merged.iter_mut().find(|item| item.key == token.key) {
                *previous = token;
            } else {
                merged.push(token);
            }
        }
    }
    format_tokens(&merged)
}

/// Sets or removes one structured flag without discarding tokens that this
/// version of Kamafeu does not understand. `None` removes all occurrences of
/// the requested token; the remaining token order is stable.
pub fn set_utau_flag(flags: &str, key: &str, value: Option<f64>) -> String {
    let mut tokens = parse_utau_flags(flags);
    tokens.retain(|token| token.key != key);
    if let Some(value) = value {
        tokens.push(UtauFlagToken {
            key: key.to_string(),
            num_val: Some(value),
        });
    }
    format_tokens(&tokens)
}

fn format_tokens(tokens: &[UtauFlagToken]) -> String {
    tokens
        .iter()
        .map(|token| match token.num_val {
            Some(value) if value.fract() == 0.0 => format!("{}{value:.0}", token.key),
            Some(value) => format!("{}{}", token.key, value),
            None => token.key.clone(),
        })
        .collect()
}

pub fn parse_flag_numeric(flags: &str, target_flag: &str) -> Option<f64> {
    let tokens = parse_utau_flags(flags);
    for token in tokens.into_iter().rev() {
        if token.key == target_flag {
            return token.num_val;
        }
    }
    None
}

pub fn remove_flag(flags: &str, target_flag: &str) -> String {
    let tokens = parse_utau_flags(flags);
    let mut result = String::with_capacity(flags.len());
    for token in tokens {
        if token.key != target_flag {
            result.push_str(&token.key);
            if let Some(val) = token.num_val {
                if val.fract() == 0.0 {
                    result.push_str(&format!("{val:.0}"));
                } else {
                    result.push_str(&format!("{val}"));
                }
            }
        }
    }
    result
}
