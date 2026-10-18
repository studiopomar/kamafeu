use crate::oto::Voicebank;
use crate::phonemizer::{consonant_velocity_time_scale, romaji, PhonemizerMode, RenderPhone};
use crate::project::model::UNote;
#[cfg(test)]
use std::collections::{HashMap, HashSet};
use std::fs;
use unicode_normalization::UnicodeNormalization;

static PLAIN_VOWELS: &[&str] = &[
    "あ", "い", "う", "え", "お", "を", "ん", "ン", "a", "i", "u", "e", "o", "n", "N",
];

pub(crate) mod openutau;

mod tables;
use tables::{CONSONANT_LOOKUP, SUBSTITUTE_LOOKUP, VOWEL_LOOKUP};

pub struct JapanesePhonemizer;

mod presamp;
use presamp::*;

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

mod phonemize;

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
    fn presamp_empty_replace_section_disables_defaults() {
        let (replacements, has_section) =
            parse_presamp_replacements_with_presence("[REPLACE]\n[ALIAS]\nVC=a k\n");
        assert!(has_section);
        assert!(replacements.is_empty());

        let (_, absent_section) = parse_presamp_replacements_with_presence("[ALIAS]\nVC=a k\n");
        assert!(!absent_section);
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
            expand_presamp_alias("%v%%CVPAD%%CV%", "a", "k", "か", "あ", &config),
            "a-か"
        );
        assert_eq!(
            expand_presamp_alias(&config.vc, "a", "k", "か", "あ", &config),
            "a-k,k-k"
        );

        let ending = parse_presamp_alias_config("[ENDTYPE]\na R\n[ENDTYPE2]\n-R\n");
        assert_eq!(ending.ending1, "a R");
        assert_eq!(ending.ending2, "-R");

        let standalone_pad = parse_presamp_alias_config("[VCPAD]\n-\n");
        assert_eq!(standalone_pad.vcpad, "-");
    }

    #[test]
    fn presamp_inventory_reads_vowels_consonants_and_priority() {
        let defaults = parse_presamp_inventory("");
        assert!(defaults.priority.contains("k"));
        assert!(defaults.no_crossfade.contains("k"));
        assert!(defaults.alias_pitches.contains(&"C4".to_string()));
        assert!(defaults.alias_pitches.contains(&"Eb7".to_string()));
        assert!(defaults.alias_pitches.contains(&"high".to_string()));

        let inventory = parse_presamp_inventory(
            "[VOWEL]\na=あ=か,が=100\nn=ん=ん=100\n[CONSONANT]\nk=か,く,け,こ=1\ns=さ,す,せ,そ=0\n[PRIORITY]\nk,ky\nっ\n",
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
        assert_eq!(presamp_core_lyric("a か", &inventory), "か");
        assert_eq!(presamp_core_lyric("a s", &inventory), "s");
        assert_eq!(presamp_core_lyric("a-か", &inventory), "か");
        assert_eq!(presamp_core_lyric("a_か", &inventory), "か");
        assert!(inventory.priority.contains("k"));
        assert!(inventory.priority.contains("ky"));
        assert!(inventory.priority.contains("っ"));
        assert!(inventory.no_crossfade.contains("k"));
        assert!(inventory.vc_length_from_cv);
        assert_eq!(inventory.cflags, "p0");
        assert_eq!(
            inventory.alias_priority_default,
            ["VCV", "CVVC", "CROSS_CV", "CV", "BEGINING_CV"]
                .into_iter()
                .map(str::to_string)
                .collect::<Vec<_>>()
        );

        let fixed = parse_presamp_inventory("[VCLENGTH]\n1\n");
        assert!(!fixed.vc_length_from_cv);

        let custom_suffixes = parse_presamp_inventory(
            "[NUM]\n@NUM@\nX\n[APPEND]\n@APPEND@\n強\n[PITCH]\n@PITCH@\nC4\n",
        );
        assert_eq!(custom_suffixes.alias_nums, HashSet::from(["X".to_string()]));
        assert_eq!(custom_suffixes.alias_appends, vec!["強"]);
        assert_eq!(custom_suffixes.alias_pitches, vec!["C4"]);

        let forced = parse_presamp_inventory("[MUSTVC]\n1\n");
        assert!(forced.must_vc);

        let disabled = parse_presamp_inventory("[ENDFLAG]\n0\n");
        assert_eq!(disabled.add_ending, 0);

        let convert_last = parse_presamp_inventory("[ENDFLAG]\n2\n");
        assert_eq!(convert_last.add_ending, 2);

        let no_split = parse_presamp_inventory("[SPLIT]\n0\n");
        assert!(!no_split.split);

        let compound_completion = parse_presamp_inventory("[VOWEL]\na=あ=ゃ\n");
        assert_eq!(
            extract_presamp_vowel("きゃ", &compound_completion),
            Some("a".to_string())
        );

        let custom_flags = parse_presamp_inventory("[CFLAGS]\nY99\n");
        assert_eq!(custom_flags.cflags, "Y99");

        let custom_priorities = parse_presamp_inventory(
            "[ALIAS_PRIORITY]\nCV\nVCV\n[ALIAS_PRIORITY_DIFAPPEND]\nCVVC\n[ALIAS_PRIORITY_DIFPITCH]\nCROSS_CV\n",
        );
        assert_eq!(custom_priorities.alias_priority_default, ["CV", "VCV"]);
        assert_eq!(custom_priorities.alias_priority_diff_append, ["CVVC"]);
        assert_eq!(custom_priorities.alias_priority_diff_pitch, ["CROSS_CV"]);

        let custom_suffix_rules =
            parse_presamp_inventory("[PRE]\n_normal\n_normal\n_soft\n[SU]\n%append%%num%%pitch%\n");
        assert_eq!(custom_suffix_rules.prefixes, ["_normal", "_soft"]);
        assert_eq!(
            custom_suffix_rules.suffix_order,
            ["%append%", "%num%", "%pitch%"]
        );
        assert!(presamp_suffix_variants("か強1C4", &custom_suffix_rules).is_empty());

        let reordered = parse_presamp_inventory(
            "[APPEND]\n強\n[NUM]\n1\n[PITCH]\nC4\n[SU]\n%num%%append%%pitch%\n",
        );
        assert_eq!(presamp_suffix_variants("か強1C4", &reordered), ["か1強C4"]);

        let overlapping = parse_presamp_inventory("[APPEND]\n強\n強い\n");
        assert_eq!(presamp_core_lyric("か強い", &overlapping), "か");

        let ordered = order_presamp_candidate_groups(
            &["CV".to_string(), "VCV".to_string()],
            vec![
                ("VCV", vec!["vcv".to_string()]),
                ("CV", vec!["cv".to_string()]),
            ],
        );
        assert_eq!(ordered, ["cv", "vcv"]);
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
    fn legacy_vcv_maps_ng_to_katakana_nasal_without_changing_cvvc() {
        let inventory = PresampInventory::default();
        assert_eq!(
            extract_vowel_for_mode("ng", &inventory, PhonemizerMode::VCV),
            Some("N".to_string())
        );
        assert_eq!(
            extract_vowel_for_mode("ng", &inventory, PhonemizerMode::CVVC),
            None
        );
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

    #[test]
    fn japanese_basic_cv_preserves_a_phonetic_hint_alias() {
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

        let phones = JapanesePhonemizer::apply_japanese(&[note], &vb, PhonemizerMode::BasicCV);
        assert_eq!(phones[0].lyric, "a か");
    }

    #[test]
    fn japanese_alternate_is_preferred_during_alias_lookup() {
        let mut vb = build_test_voicebank();
        vb.entries.insert(
            "- か2".to_string(),
            OtoEntry::new(
                "ka2.wav".to_string(),
                "- か2".to_string(),
                0.0,
                50.0,
                10.0,
                0.0,
                0.0,
            ),
        );
        let mut note = UNote::new("か", "C4", 0.0, 300.0);
        note.phoneme_overrides
            .push(crate::project::model::UPhonemeOverride {
                index: 0,
                alternate: Some(2),
                ..Default::default()
            });

        let phones = JapanesePhonemizer::apply_japanese(&[note], &vb, PhonemizerMode::VCV);
        assert_eq!(phones[0].lyric, "- か2");
    }

    #[test]
    fn default_presamp_replaces_romaji_capital_n_with_katakana_nasal() {
        let mut vb = build_test_voicebank();
        vb.entries.insert(
            "- ン".to_string(),
            OtoEntry::new(
                "N.wav".to_string(),
                "- ン".to_string(),
                0.0,
                50.0,
                10.0,
                0.0,
                0.0,
            ),
        );
        let note = UNote::new("N", "C4", 0.0, 300.0);

        let phones = JapanesePhonemizer::apply_japanese(&[note], &vb, PhonemizerMode::VCV);
        assert_eq!(phones[0].lyric, "- ン");
    }

    #[test]
    fn default_presamp_replacement_covers_irregular_romaji() {
        assert_eq!(default_presamp_replacement("dha"), Some("でゃ"));
        assert_eq!(default_presamp_replacement("fyu"), Some("ふゅ"));
        assert_eq!(default_presamp_replacement("wo"), Some("うぉ"));
        assert_eq!(default_presamp_replacement("ordinary"), None);
    }

    #[test]
    fn japanese_lyrics_are_nfc_normalized_before_phoneme_classification() {
        let mut vb = build_test_voicebank();
        vb.entries.insert(
            "- が".to_string(),
            OtoEntry::new(
                "ga.wav".to_string(),
                "- が".to_string(),
                0.0,
                50.0,
                10.0,
                0.0,
                0.0,
            ),
        );
        let note = UNote::new("か\u{3099}", "C4", 0.0, 300.0);

        let phones = JapanesePhonemizer::apply_japanese(&[note], &vb, PhonemizerMode::VCV);
        assert_eq!(phones[0].lyric, "- が");
    }

    #[test]
    fn japanese_extender_variants_prolong_the_previous_phoneme() {
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
        let notes = vec![
            UNote::new("あ", "C4", 0.0, 300.0),
            UNote::new("+~", "C4", 300.0, 200.0),
            UNote::new("か", "C4", 500.0, 300.0),
        ];

        let phones = JapanesePhonemizer::apply_japanese(&notes, &vb, PhonemizerMode::VCV);
        assert_eq!(phones[0].lyric, "- あ");
        assert_eq!(phones[0].duration_ms, 500.0);
        assert_eq!(phones[1].lyric, "a か");
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
    fn japanese_cvvc_keeps_cv_syllables_consonantal_with_presamp_vowels() {
        let mut vb = build_test_voicebank();
        let root = std::env::temp_dir().join(format!(
            "kamafeu-presamp-cvvc-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("presamp.ini"),
            "[VOWEL]\na=あ=か,さ\n[CONSONANT]\ns=さ\n",
        )
        .unwrap();
        vb.root_path = root.clone();

        let notes = vec![
            UNote::new("か", "C4", 0.0, 400.0),
            UNote::new("さ", "C4", 400.0, 400.0),
        ];
        let phones = JapanesePhonemizer::apply_japanese(&notes, &vb, PhonemizerMode::CVVC);

        assert_eq!(phones.len(), 3);
        assert_eq!(phones[1].lyric, "a s");
        assert_eq!(phones[2].lyric, "さ");
        let _ = std::fs::remove_dir_all(root);
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
        let root = std::env::temp_dir().join(format!(
            "kamafeu-presamp-cflags-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("presamp.ini"), "[CFLAGS]\np0\n").unwrap();
        vb.root_path = root.clone();
        let phones = JapanesePhonemizer::apply_japanese(
            &[UNote::new("さ", "C4", 0.0, 400.0)],
            &vb,
            PhonemizerMode::CVVC,
        );
        assert_eq!(phones.len(), 2);
        assert_eq!(phones[0].lyric, "- s");
        assert_eq!(phones[1].lyric, "さ");
        assert_eq!(phones[0].expressions.presamp_normalization, Some(0.0));
        assert!(phones[1].position_ms > phones[0].position_ms);
        let _ = std::fs::remove_dir_all(root);
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
    fn presamp_vcv_renders_sokuon_as_a_closure_instead_of_a_vowel() {
        let mut vb = build_test_voicebank();
        vb.entries.insert(
            "a っ".to_string(),
            OtoEntry::new(
                "a-tsu-small.wav".to_string(),
                "a っ".to_string(),
                0.0,
                30.0,
                5.0,
                0.0,
                0.0,
            ),
        );
        let root = std::env::temp_dir().join(format!(
            "kamafeu-presamp-sokuon-vcv-{}-{}",
            std::process::id(),
            std::thread::current().name().unwrap_or("test")
        ));
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(
            root.join("presamp.ini"),
            "[VOWEL]\na=あ\n[CONSONANT]\nk=か\n",
        )
        .unwrap();
        vb.root_path = root.clone();
        let notes = vec![
            UNote::new("か", "C4", 0.0, 300.0),
            UNote::new("っ", "C4", 300.0, 100.0),
            UNote::new("か", "C4", 400.0, 300.0),
        ];

        let phones = JapanesePhonemizer::apply_japanese(&notes, &vb, PhonemizerMode::VCV);
        assert!(phones.iter().any(|phone| phone.lyric == "a っ"));
        assert!(!phones.iter().any(|phone| phone.lyric == "u"));
        let _ = std::fs::remove_dir_all(root);
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
    fn japanese_cvvc_does_not_insert_vc_before_phonetic_hint() {
        let vb = build_test_voicebank();
        let mut hinted = UNote::new("さ", "C4", 400.0, 400.0);
        hinted.phonetic_hint = Some("さ".to_string());
        let notes = vec![UNote::new("か", "C4", 0.0, 400.0), hinted];
        let phones = JapanesePhonemizer::apply_japanese(&notes, &vb, PhonemizerMode::CVVC);

        assert_eq!(phones.len(), 2);
        assert_eq!(phones[0].lyric, "- か");
        assert_eq!(phones[1].lyric, "さ");
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
