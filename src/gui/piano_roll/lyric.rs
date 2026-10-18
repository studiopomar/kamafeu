/// Characters that separate multiple phonemes in a note lyric.
pub const PHONEME_SEPARATORS: [char; 5] = ['.', ',', ';', '|', '/'];

/// Returns the final phoneme fragment from a multi-phoneme lyric.
pub fn active_phoneme_query(lyric: &str) -> &str {
    lyric
        .rsplit_once(PHONEME_SEPARATORS)
        .map_or(lyric, |(_, active)| active)
        .trim()
}

/// Replaces only the final phoneme fragment, preserving its leading spacing.
pub fn replace_active_phoneme(lyric: &str, alias: &str) -> String {
    let Some((separator_index, separator)) = lyric
        .char_indices()
        .rev()
        .find(|(_, character)| PHONEME_SEPARATORS.contains(character))
    else {
        return alias.to_string();
    };

    let active_start = separator_index + separator.len_utf8();
    let active = &lyric[active_start..];
    let whitespace_bytes = active.len() - active.trim_start().len();
    format!(
        "{}{}{}",
        &lyric[..active_start],
        &active[..whitespace_bytes],
        alias
    )
}

#[cfg(test)]
mod tests {
    use super::{active_phoneme_query, replace_active_phoneme, PHONEME_SEPARATORS};

    #[test]
    fn query_and_replacement_target_the_last_phoneme() {
        for separator in PHONEME_SEPARATORS {
            let lyric = format!("- aa{separator} d ih");
            assert_eq!(active_phoneme_query(&lyric), "d ih");
            assert_eq!(
                replace_active_phoneme(&lyric, "d ih D4"),
                format!("- aa{separator} d ih D4")
            );
        }
    }

    #[test]
    fn empty_active_phoneme_keeps_whitespace_after_separator() {
        assert_eq!(active_phoneme_query("- aa."), "");
        assert_eq!(replace_active_phoneme("- aa. ", "d ih"), "- aa. d ih");
    }

    #[test]
    fn lyric_without_separator_is_replaced_entirely() {
        assert_eq!(
            replace_active_phoneme("  old alias  ", "new alias"),
            "new alias"
        );
    }
}
