pub mod brapa;
pub mod chinese;
pub mod diphone;
pub mod english;
pub mod french;
pub mod japanese;
pub mod korean;
pub mod portuguese;
pub mod portuguese_cvc;
pub mod romaji;
pub mod russian;
pub mod spanish;
pub mod vccv;

/// Upstream OpenUtau revision used by the compatibility corpus and ports.
pub const OPENUTAU_COMPATIBILITY_COMMIT: &str = "13184085f2cee51c174de9ee99163a76ee970e8c";

pub mod dispatcher;
mod g2p;
pub mod mode;
pub use brapa::{BrapaCVCPhonemizer, VccvBrapaPhonemizer};
pub use dispatcher::OpenUtauPhonemizer;
pub use english::EnglishPhonemizer;
pub use japanese::JapanesePhonemizer as CoreJapanesePhonemizer;
pub use mode::{PhonemizerImplementation, PhonemizerInfo, PhonemizerMode};
pub use portuguese::PortuguesePhonemizer;

use crate::oto::Voicebank;
use crate::project::model::UNote;

/// Retained as a source-compatible no-op. OpenUtau phonemizers own their
/// normalization and custom text substitutions; applying a second Kamafeu
/// rule layer changes the input before the official algorithm sees it.
pub fn set_custom_rules(_rules: std::collections::HashMap<String, String>) {}

pub fn consonant_velocity_time_scale(velocity: f64) -> f64 {
    let velocity = if velocity.is_finite() {
        velocity
    } else {
        100.0
    };
    // Negative values are useful for deliberately lengthening consonants.
    2.0f64.powf(1.0 - velocity.clamp(-100.0, 200.0) / 100.0)
}

#[derive(Debug, Clone)]
pub struct RenderPhone {
    pub note_index: usize,
    pub lyric: String,
    pub pitch: String,
    pub position_ms: f64,
    pub duration_ms: f64,
    pub envelope: crate::dsp::envelope::UtauEnvelope,
    pub expressions: crate::project::model::UExpressions,
    pub pitch_bend: crate::project::model::UPitchBend,
    pub vibrato: crate::dsp::pitch::VibratoParam,
    pub flags: String,
    /// Alias resolved against the exact subbank selected for this phone.
    /// When absent, the renderer performs the normal voicebank lookup.
    pub resolved_alias: Option<String>,
    /// Timbre/subbank used to resolve `resolved_alias`, for diagnostics.
    pub resolved_timbre: Option<String>,
}

impl RenderPhone {
    pub fn midi_key(&self) -> u8 {
        crate::dsp::pitch::note_name_to_midi(&self.pitch).unwrap_or(60)
    }

    pub fn find_entry<'a>(
        &'a self,
        voicebank: &'a crate::oto::Voicebank,
    ) -> Option<&'a crate::oto::OtoEntry> {
        if let Some(alias) = self.resolved_alias.as_deref() {
            // A frozen alias is an authoritative phonemizer decision. Never
            // fall back to PrefixMap here: doing so can apply a second
            // prefix/suffix and silently switch subbanks.
            return voicebank.find_exact_entry(alias);
        }
        voicebank.find_mapped_entry(&self.lyric, &self.pitch)
    }
}

mod legacy;
pub use legacy::JapanesePhonemizer;

#[cfg(test)]
mod tests {
    use super::*;

    fn test_voicebank_with_aliases(aliases: &[&str]) -> (tempfile::TempDir, Voicebank) {
        let directory = tempfile::tempdir().unwrap();
        let mut voicebank = Voicebank::new(directory.path()).unwrap();
        for alias in aliases {
            voicebank.entries.insert(
                (*alias).to_string(),
                crate::oto::OtoEntry::new(
                    format!("{alias}.wav"),
                    (*alias).to_string(),
                    0.0,
                    80.0,
                    10.0,
                    0.0,
                    0.0,
                ),
            );
        }
        (directory, voicebank)
    }

    #[test]
    fn basic_cv_uses_vv_only_between_adjacent_vowels() {
        let (_directory, voicebank) =
            test_voicebank_with_aliases(&["- あ", "a い", "あ", "か", "a か"]);
        let notes = vec![
            UNote::new("あ", "C4", 0.0, 400.0),
            UNote::new("い", "C4", 400.0, 400.0),
        ];
        let phones =
            JapanesePhonemizer::apply_phonemizer(&notes, &voicebank, PhonemizerMode::BasicCV);
        assert_eq!(
            phones
                .iter()
                .map(|phone| phone.lyric.as_str())
                .collect::<Vec<_>>(),
            ["- あ", "a い"]
        );

        let notes = vec![
            UNote::new("あ", "C4", 0.0, 400.0),
            UNote::new("か", "C4", 400.0, 400.0),
        ];
        let phones =
            JapanesePhonemizer::apply_phonemizer(&notes, &voicebank, PhonemizerMode::BasicCV);
        assert_eq!(phones[1].lyric, "か");
    }

    #[test]
    fn cvvc_keeps_cv_main_phone_and_adds_only_outgoing_vc() {
        let (_directory, voicebank) = test_voicebank_with_aliases(&["- あ", "a さ", "a s", "さ"]);
        let notes = vec![
            UNote::new("あ", "C4", 0.0, 400.0),
            UNote::new("さ", "C4", 400.0, 400.0),
        ];
        let phones = JapanesePhonemizer::apply_phonemizer(&notes, &voicebank, PhonemizerMode::CVVC);
        assert_eq!(
            phones
                .iter()
                .map(|phone| phone.lyric.as_str())
                .collect::<Vec<_>>(),
            ["- あ", "a s", "さ"]
        );
    }

    #[test]
    fn dispatcher_reports_supported_openutau_families_explicitly() {
        assert_eq!(
            OpenUtauPhonemizer::implementation(PhonemizerMode::None),
            PhonemizerImplementation::Passthrough
        );
        assert_eq!(
            OpenUtauPhonemizer::implementation(PhonemizerMode::VCV),
            PhonemizerImplementation::OpenUtau
        );
        assert_eq!(PhonemizerMode::VCV.info().source, "OpenUtau (validado)");
        for mode in [
            PhonemizerMode::BasicCV,
            PhonemizerMode::CVVC,
            PhonemizerMode::JapanesePresamp,
        ] {
            assert_eq!(
                OpenUtauPhonemizer::implementation(mode),
                PhonemizerImplementation::OpenUtau
            );
            assert_eq!(mode.info().source, "OpenUtau (validado)");
        }
        assert_eq!(
            OpenUtauPhonemizer::implementation(PhonemizerMode::EnglishVCCV),
            PhonemizerImplementation::OpenUtau
        );
    }

    #[test]
    fn japanese_presamp_dispatch_uses_voicebank_presamp_ini() {
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(
            directory.path().join("presamp.ini"),
            "[VOWEL]\na=あ=あ\n[CONSONANT]\nk=k,か\n[ALIAS]\nVC=a k\nVCV=%v% %CV%\nBEGINNINGCV=- %CV%\n",
        )
        .unwrap();
        let mut voicebank = Voicebank::new(directory.path()).unwrap();
        for alias in ["- あ", "a か"] {
            voicebank.entries.insert(
                alias.to_string(),
                crate::oto::OtoEntry::new(
                    format!("{alias}.wav"),
                    alias.to_string(),
                    0.0,
                    80.0,
                    10.0,
                    0.0,
                    0.0,
                ),
            );
        }

        let notes = vec![
            UNote::new("あ", "C4", 0.0, 120.0),
            UNote::new("か", "C4", 120.0, 120.0),
        ];
        let phones = OpenUtauPhonemizer::apply(&notes, &voicebank, PhonemizerMode::JapanesePresamp);
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/openutau_compatibility/japanese_presamp.json"
        ))
        .unwrap();
        assert_eq!(fixture["upstream"]["class"], "JapanesePresampPhonemizer");
        let expected = fixture["cases"][0]["expected_aliases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|alias| alias.as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            phones
                .iter()
                .map(|phone| phone.lyric.as_str())
                .collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn arpasing_uses_the_voicebank_openutau_dictionary() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("arpasing.yaml"),
            "symbols: [{symbol: hh, type: aspirate}]\nentries:\n  - grapheme: hello\n    phonemes: [hh, eh, l, ow]\n  - grapheme: WORLD\n    phonemes: [w, er, l, d]\n",
        )
        .unwrap();
        let vb = Voicebank::new(dir.path()).unwrap();

        assert_eq!(
            g2p::openutau_arpasing_tokens("hello", &vb).unwrap(),
            vec!["hh", "eh", "l", "ow"]
        );
        assert_eq!(
            g2p::openutau_arpasing_tokens("world", &vb).unwrap(),
            vec!["w", "er", "l", "d"]
        );
        assert_eq!(
            g2p::openutau_arpasing_tokens("can", &vb).unwrap(),
            vec!["k", "ae", "n"]
        );
        assert_eq!(g2p::arpasing_vowel_fallbacks("ao"), &["ow"]);
        assert!(g2p::openutau_arpasing_tokens("zzzznotaword", &vb).is_none());
    }

    #[test]
    fn arpasing_uses_the_previous_symbol_for_consonant_clusters() {
        let (_directory, voicebank) =
            test_voicebank_with_aliases(&["- s", "s t", "t ae", "ae n", "n -", "- ao", "- ah"]);
        let mut first = UNote::new("ignored", "C4", 0.0, 100.0);
        first.phonetic_hint = Some("s".to_string());
        let mut second = UNote::new("ignored", "C4", 100.0, 300.0);
        second.phonetic_hint = Some("t ae n".to_string());
        let phones = JapanesePhonemizer::apply_phonemizer(
            &[first, second],
            &voicebank,
            PhonemizerMode::EnglishArpasing,
        );

        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/openutau_compatibility/english_arpasing.json"
        ))
        .unwrap();
        let expected = fixture["cases"][0]["expected_aliases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|alias| alias.as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(
            phones
                .iter()
                .map(|phone| phone.lyric.as_str())
                .collect::<Vec<_>>(),
            expected
        );
    }

    use std::collections::HashMap;

    #[test]
    fn per_note_timbre_preserves_vcv_context_and_default() {
        let dir = tempfile::tempdir().unwrap();
        let mut vb = Voicebank::new(dir.path()).unwrap();
        vb.prefix_map = crate::oto::PrefixMap::parse_yaml_str("subbanks:\n  - color: ''\n    suffix: _normal\n    tone_ranges: [C4-C5]\n  - color: Soft\n    suffix: _soft\n    tone_ranges: [C4-C5]\n");
        for alias in ["- あ_normal", "a か_soft", "a か_normal"] {
            vb.entries.insert(
                alias.to_string(),
                crate::oto::OtoEntry::new(
                    format!("{alias}.wav"),
                    alias.to_string(),
                    0.0,
                    50.0,
                    10.0,
                    0.0,
                    0.0,
                ),
            );
        }
        let mut notes = vec![
            UNote::new("あ", "C4", 0.0, 400.0),
            UNote::new("か", "C4", 400.0, 400.0),
            UNote::new("か", "C4", 800.0, 400.0),
        ];
        notes[1].timbre = Some("Soft".to_string());
        let phones = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::VCV);
        assert_eq!(
            phones.iter().map(|p| p.lyric.as_str()).collect::<Vec<_>>(),
            ["- あ_normal", "a か_soft", "a か_normal"]
        );
        assert_eq!(
            phones.iter().map(|p| p.note_index).collect::<Vec<_>>(),
            [0, 1, 2]
        );
        assert_eq!(phones[1].position_ms, 400.0);
        assert_eq!(
            phones
                .iter()
                .map(|p| p.resolved_alias.as_deref())
                .collect::<Vec<_>>(),
            [Some("- あ_normal"), Some("a か_soft"), Some("a か_normal")]
        );
        assert_eq!(phones[1].resolved_timbre.as_deref(), Some("Soft"));
        assert_eq!(vb.prefix_map.selected_color(), "");

        // Repeated per-note timbre resolution must not leak the temporary
        // color into the shared PrefixMap state.
        notes[0].timbre = Some("Soft".to_string());
        notes[1].timbre = None;
        let swapped = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::VCV);
        assert_eq!(swapped[0].resolved_timbre.as_deref(), Some("Soft"));
        assert_eq!(swapped[1].resolved_timbre, None);
        assert_eq!(vb.prefix_map.selected_color(), "");
        // Existing projects without the new field still inherit the default.
        let legacy: UNote =
            serde_json::from_str(r#"{"lyric":"a","pitch":"C4","position_ms":0,"duration_ms":400}"#)
                .unwrap();
        assert_eq!(legacy.timbre, None);
    }

    #[test]
    fn cvvc_outgoing_transition_uses_destination_timbre() {
        let dir = tempfile::tempdir().unwrap();
        let mut vb = Voicebank::new(dir.path()).unwrap();
        vb.prefix_map = crate::oto::PrefixMap::parse_yaml_str(
            "subbanks:\n  - color: ''\n    suffix: _normal\n    tone_ranges: [C4-C5]\n  - color: Soft\n    suffix: _soft\n    tone_ranges: [C4-C5]\n",
        );
        for alias in [
            "- あ_normal",
            "a k_normal",
            "a k_soft",
            "か_soft",
            "か_normal",
        ] {
            vb.entries.insert(
                alias.to_string(),
                crate::oto::OtoEntry::new(
                    format!("{alias}.wav"),
                    alias.to_string(),
                    0.0,
                    50.0,
                    10.0,
                    0.0,
                    0.0,
                ),
            );
        }
        let mut notes = vec![
            UNote::new("あ", "C4", 0.0, 400.0),
            UNote::new("か", "C4", 400.0, 400.0),
        ];
        notes[1].timbre = Some("Soft".to_string());
        let phones = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::CVVC);
        let transition = phones.iter().find(|phone| phone.lyric == "a k_soft");
        assert!(transition.is_some(), "phones: {:?}", phones);
        assert_eq!(transition.unwrap().resolved_timbre.as_deref(), Some("Soft"));
    }

    #[test]
    fn frozen_alias_does_not_fall_back_to_prefix_map() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("oto.ini"),
            "a.wav=a_normal,0,50,-200,40,0\n",
        )
        .unwrap();
        let vb = Voicebank::new(dir.path()).unwrap();
        let phone = RenderPhone {
            note_index: 0,
            lyric: "a".to_string(),
            pitch: "C4".to_string(),
            position_ms: 0.0,
            duration_ms: 100.0,
            envelope: crate::dsp::envelope::UtauEnvelope::default(),
            expressions: crate::project::model::UExpressions::default(),
            pitch_bend: crate::project::model::UPitchBend::default(),
            vibrato: crate::dsp::pitch::VibratoParam::default(),
            flags: String::new(),
            resolved_alias: Some("a_missing".to_string()),
            resolved_timbre: Some("Soft".to_string()),
        };
        assert!(phone.find_entry(&vb).is_none());
    }

    #[test]
    fn test_phrase_start_head_alias() {
        let mut entries = HashMap::new();
        entries.insert(
            "- か".to_string(),
            crate::oto::OtoEntry::new(
                "ka.wav".to_string(),
                "- か".to_string(),
                0.0,
                50.0,
                0.0,
                0.0,
                0.0,
            ),
        );
        entries.insert(
            "a き".to_string(),
            crate::oto::OtoEntry::new(
                "ki.wav".to_string(),
                "a き".to_string(),
                0.0,
                50.0,
                0.0,
                0.0,
                0.0,
            ),
        );

        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let notes = vec![
            UNote::new("ka", "C4", 0.0, 400.0),
            UNote::new("ki", "C4", 400.0, 400.0),
            UNote::new("ka", "C4", 2000.0, 400.0),
        ];

        let phones = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::VCV);
        assert_eq!(phones.len(), 3);
        assert_eq!(phones[0].lyric, "- か");
        assert_eq!(phones[1].lyric, "a き");
        assert_eq!(phones[2].lyric, "- か");
    }

    #[test]
    fn test_english_phonemizer() {
        let phones = EnglishPhonemizer::word_to_arpabet("hello");
        assert_eq!(phones, vec!["hh", "ah", "l", "ow"]);

        let sing = EnglishPhonemizer::word_to_arpabet("sing");
        assert_eq!(sing, vec!["s", "ih", "ng"]);
    }

    #[test]
    fn test_portuguese_phonemizer() {
        let phones = PortuguesePhonemizer::word_to_phonemes("canto");
        assert_eq!(phones, vec!["ka", "n", "to"]);

        let amor = PortuguesePhonemizer::word_to_phonemes("amor");
        assert_eq!(amor, vec!["a", "mo", "r"]);
    }

    #[test]
    fn test_none_phonemizer_passthrough() {
        let entries = std::collections::HashMap::new();
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let notes = vec![
            UNote::new("k_a", "C4", 0.0, 400.0),
            UNote::new("R", "C4", 400.0, 200.0),
            UNote::new("- sa", "D4", 600.0, 400.0),
        ];

        let phones = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::None);
        assert_eq!(phones.len(), 2);
        assert_eq!(phones[0].lyric, "k_a");
        assert_eq!(phones[0].position_ms, 0.0);
        assert_eq!(phones[1].lyric, "- sa");
        assert_eq!(phones[1].position_ms, 600.0);
    }

    #[test]
    fn phoneme_override_applies_alias_and_advanced_vocal_controls() {
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries: std::collections::HashMap::new(),
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };
        let mut note = UNote::new("original", "C4", 0.0, 400.0);
        note.phoneme_overrides
            .push(crate::project::model::UPhonemeOverride {
                index: 0,
                phoneme: Some("k a".to_string()),
                envelope: Some(crate::dsp::envelope::UtauEnvelope {
                    p2: 19.0,
                    ..Default::default()
                }),
                vibrato: Some(crate::dsp::pitch::VibratoParam {
                    length_pct: 33.0,
                    ..Default::default()
                }),
                pitch_bend: Some(crate::project::model::UPitchBend {
                    portamento_length_ms: 77.0,
                    ..Default::default()
                }),
                consonant_timing_offset_ms: Some(14.0),
                ..Default::default()
            });

        let phones = JapanesePhonemizer::apply_phonemizer(&[note], &vb, PhonemizerMode::None);
        assert_eq!(phones.len(), 1);
        assert_eq!(phones[0].lyric, "k a");
        assert_eq!(phones[0].envelope.p2, 19.0);
        assert_eq!(phones[0].vibrato.length_pct, 33.0);
        assert_eq!(phones[0].pitch_bend.portamento_length_ms, 77.0);
        assert_eq!(phones[0].expressions.consonant_timing_offset_ms, 14.0);
    }

    #[test]
    fn test_universal_plus_continuation_in_any_mode() {
        let entries = std::collections::HashMap::new();
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let notes = vec![
            UNote::new("k aa", "C4", 0.0, 500.0),
            UNote::new("+", "D4", 500.0, 250.0),
            UNote::new("+", "E4", 750.0, 250.0),
        ];

        let phones_none = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::None);
        assert_eq!(phones_none.len(), 1);
        assert_eq!(phones_none[0].lyric, "k aa");
        assert_eq!(phones_none[0].duration_ms, 1000.0); // 500 + 250 + 250

        let notes_en = vec![
            UNote::new("sing", "C4", 0.0, 500.0),
            UNote::new("+", "D4", 500.0, 250.0),
            UNote::new("+", "E4", 750.0, 250.0),
        ];
        let phones_en =
            JapanesePhonemizer::apply_phonemizer(&notes_en, &vb, PhonemizerMode::EnglishArpasing);
        let total_dur: f64 = phones_en.iter().map(|p| p.duration_ms).sum();
        assert_eq!(total_dur, 1000.0);
    }

    #[test]
    fn test_manual_mode_semicolon_and_comma_sub_phonemes() {
        let entries = std::collections::HashMap::new();
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let notes = vec![UNote::new("-k;k ae;ae n", "C4", 0.0, 300.0)];

        let phones = JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::None);
        assert_eq!(phones.len(), 3);
        assert_eq!(phones[0].lyric, "-k");
        assert_eq!(phones[0].duration_ms, 180.0);
        assert_eq!(phones[1].lyric, "k ae");
        assert_eq!(phones[1].duration_ms, 60.0);
        assert_eq!(phones[2].lyric, "ae n");
        assert_eq!(phones[2].duration_ms, 60.0);

        let notes_comma = vec![UNote::new("k ae, ae n", "C4", 0.0, 300.0)];
        let phones_comma =
            JapanesePhonemizer::apply_phonemizer(&notes_comma, &vb, PhonemizerMode::None);
        assert_eq!(phones_comma.len(), 2);
        assert_eq!(phones_comma[0].lyric, "k ae");
        assert_eq!(phones_comma[0].duration_ms, 240.0);
        assert_eq!(phones_comma[1].lyric, "ae n");
        assert_eq!(phones_comma[1].duration_ms, 60.0);

        let mut resized_note = UNote::new("k ae.ae n.", "B3", 0.0, 480.0);
        resized_note.set_phoneme_boundary(2, 1, 360.0);
        let resized =
            JapanesePhonemizer::apply_phonemizer(&[resized_note], &vb, PhonemizerMode::None);
        assert_eq!(resized.len(), 2);
        assert_eq!(resized[0].lyric, "k ae");
        assert_eq!(resized[0].position_ms, 0.0);
        assert_eq!(resized[0].duration_ms, 360.0);
        assert_eq!(resized[1].lyric, "ae n");
        assert_eq!(resized[1].position_ms, 360.0);
        assert_eq!(resized[1].duration_ms, 120.0);

        let notes_dot = vec![UNote::new("m an. an d. d eh. eh l. l a.", "C4", 0.0, 500.0)];
        let phones_dot =
            JapanesePhonemizer::apply_phonemizer(&notes_dot, &vb, PhonemizerMode::None);
        assert_eq!(phones_dot.len(), 5);
        assert_eq!(phones_dot[0].lyric, "m an");
        assert_eq!(phones_dot[0].duration_ms, 260.0);
        assert_eq!(phones_dot[1].lyric, "an d");
        assert_eq!(phones_dot[1].duration_ms, 60.0);
        assert_eq!(phones_dot[2].lyric, "d eh");
        assert_eq!(phones_dot[2].duration_ms, 60.0);
        assert_eq!(phones_dot[3].lyric, "eh l");
        assert_eq!(phones_dot[3].duration_ms, 60.0);
        assert_eq!(phones_dot[4].lyric, "l a");
        assert_eq!(phones_dot[4].duration_ms, 60.0);
    }

    #[test]
    fn manual_composite_uses_oto_preutterance_instead_of_equal_slices() {
        let mut entries = HashMap::new();
        entries.insert(
            "a rh-".to_string(),
            crate::oto::OtoEntry::new(
                "a-rh.wav".to_string(),
                "a rh-".to_string(),
                0.0,
                268.0,
                -147.0,
                122.0,
                90.0,
            ),
        );
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };
        let phones = JapanesePhonemizer::apply_phonemizer(
            &[UNote::new("t a.a rh-", "B3", 0.0, 1200.0)],
            &vb,
            PhonemizerMode::None,
        );
        assert_eq!(phones.len(), 2);
        assert!((phones[0].duration_ms - 1078.0).abs() < 1e-6);
        assert!((phones[1].position_ms - 1078.0).abs() < 1e-6);
        assert!((phones[1].duration_ms - 122.0).abs() < 1e-6);
    }

    #[test]
    fn test_g2p_vs_phonetic_mode() {
        let entries = std::collections::HashMap::new();
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let notes_word = vec![UNote::new("can", "C4", 0.0, 300.0)];
        let phones_g2p =
            JapanesePhonemizer::apply_phonemizer(&notes_word, &vb, PhonemizerMode::EnglishG2P);
        assert_eq!(phones_g2p.len(), 3);
        assert_eq!(phones_g2p[0].lyric, "k");
        assert_eq!(phones_g2p[1].lyric, "ae");
        assert_eq!(phones_g2p[2].lyric, "n");

        let phones_direct =
            JapanesePhonemizer::apply_phonemizer(&notes_word, &vb, PhonemizerMode::EnglishArpasing);
        assert_eq!(phones_direct.len(), 4);
        assert_eq!(
            phones_direct
                .iter()
                .map(|phone| phone.lyric.as_str())
                .collect::<Vec<_>>(),
            ["- k", "k ae", "ae n", "n -"]
        );

        let notes_pt = vec![UNote::new("sol", "C4", 0.0, 300.0)];
        let phones_pt_g2p =
            JapanesePhonemizer::apply_phonemizer(&notes_pt, &vb, PhonemizerMode::PortugueseG2P);
        assert_eq!(phones_pt_g2p.len(), 3);
        assert_eq!(phones_pt_g2p[0].lyric, "s");
        assert_eq!(phones_pt_g2p[1].lyric, "O");
        assert_eq!(phones_pt_g2p[2].lyric, "w");

        let notes_brapa = vec![UNote::new("canto", "C4", 0.0, 400.0)];
        let phones_brapa = JapanesePhonemizer::apply_phonemizer(
            &notes_brapa,
            &vb,
            PhonemizerMode::PortugueseBrapaVCCV,
        );
        assert!(!phones_brapa.is_empty());
    }

    #[test]
    fn test_vccv_brapa_phonemizer_full_cascade() {
        let mut entries = std::collections::HashMap::new();
        entries.insert(
            "- ka".to_string(),
            crate::oto::OtoEntry::new(
                "ka.wav".to_string(),
                "- ka".to_string(),
                0.0,
                50.0,
                0.0,
                0.0,
                0.0,
            ),
        );
        entries.insert(
            "a s".to_string(),
            crate::oto::OtoEntry::new(
                "as.wav".to_string(),
                "a s".to_string(),
                0.0,
                50.0,
                0.0,
                0.0,
                0.0,
            ),
        );
        entries.insert(
            "s a".to_string(),
            crate::oto::OtoEntry::new(
                "sa.wav".to_string(),
                "s a".to_string(),
                0.0,
                50.0,
                0.0,
                0.0,
                0.0,
            ),
        );

        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "BRAPA VB".to_string(),
            author: "Xiao".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let notes = vec![
            UNote::new("ka", "C4", 0.0, 400.0),
            UNote::new("sa", "C4", 400.0, 400.0),
        ];

        let phones =
            JapanesePhonemizer::apply_phonemizer(&notes, &vb, PhonemizerMode::PortugueseBrapaVCCV);
        assert_eq!(
            phones
                .iter()
                .map(|phone| phone.lyric.as_str())
                .collect::<Vec<_>>(),
            ["- ka", "a s", "s a"]
        );
        assert_eq!(phones[0].position_ms, 0.0);
        assert_eq!(phones[1].position_ms, 375.0);
    }

    #[test]
    fn test_custom_phoneme_durations_across_all_phonemizers() {
        let vb = Voicebank {
            root_path: std::path::PathBuf::from("/tmp"),
            name: "Test VB".to_string(),
            author: "Test".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries: HashMap::new(),
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let mut note = UNote::new("can", "C4", 100.0, 600.0);
        note.phoneme_durations_ms = vec![100.0, 350.0, 150.0];

        let phones = JapanesePhonemizer::apply_phonemizer(&[note], &vb, PhonemizerMode::EnglishG2P);
        assert_eq!(phones.len(), 3);
        assert_eq!(phones[0].duration_ms, 100.0);
        assert_eq!(phones[0].position_ms, 100.0);
        assert_eq!(phones[1].duration_ms, 350.0);
        assert_eq!(phones[1].position_ms, 200.0);
        assert_eq!(phones[2].duration_ms, 150.0);
        assert_eq!(phones[2].position_ms, 550.0);
    }

    #[test]
    fn rule_g2p_handles_arabic_and_icelandic_scripts() {
        let arabic = g2p::arabic_tokens("شمس");
        assert_eq!(arabic, vec!["sh", "m", "s"]);
        let icelandic = g2p::icelandic_tokens("þú");
        assert_eq!(icelandic, vec!["T", "u"]);
        assert_eq!(
            JapanesePhonemizer::latin_language_tokens("chanson", PhonemizerMode::FrenchG2P),
            vec!["sh", "a", "n", "s", "o", "n"]
        );
        assert_eq!(
            JapanesePhonemizer::latin_language_tokens("шаг", PhonemizerMode::RussianG2P),
            vec!["sh", "a", "g"]
        );
        assert_eq!(
            JapanesePhonemizer::openutau_dictionary_tokens("-ce", PhonemizerMode::FrenchG2P)
                .unwrap(),
            vec!["ss"]
        );
        assert_eq!(
            JapanesePhonemizer::openutau_dictionary_tokens("can", PhonemizerMode::EnglishG2P)
                .unwrap(),
            vec!["k", "ae", "n"]
        );
        assert_eq!(
            JapanesePhonemizer::latin_language_tokens("ni3 hao3", PhonemizerMode::ChineseG2P),
            vec!["ni3", "hao3"]
        );
        assert_eq!(
            JapanesePhonemizer::latin_language_tokens("你好", PhonemizerMode::ChineseG2P),
            vec!["ni3", "hao3"]
        );
        assert_eq!(
            JapanesePhonemizer::latin_language_tokens("你好", PhonemizerMode::CantoneseG2P),
            vec!["nei5", "hou2"]
        );
        assert_eq!(
            JapanesePhonemizer::latin_language_tokens("廣東", PhonemizerMode::CantoneseG2P),
            vec!["gwong2", "dung1"]
        );
        assert_eq!(
            JapanesePhonemizer::latin_language_tokens("şeu", PhonemizerMode::TurkishCVVC),
            vec!["sh", "eu"]
        );
    }

    #[test]
    fn catalog_contains_requested_languages_and_origins() {
        let languages: std::collections::BTreeSet<_> = PhonemizerMode::ALL
            .iter()
            .map(|mode| mode.info().language)
            .collect();
        for language in [
            "Inglês",
            "Japonês",
            "Francês",
            "Português",
            "Alemão",
            "Russo",
            "Espanhol",
            "Turco",
            "Chinês",
            "Cantonês",
            "Árabe",
            "Islandês",
        ] {
            assert!(languages.contains(language), "missing language {language}");
        }
        assert_eq!(
            PhonemizerMode::ArabicG2P.info().source,
            "Plugin dorayakito/OpenUtau (dados)"
        );
        assert_eq!(
            PhonemizerMode::IcelandicG2P.info().source,
            "Plugin dorayakito/OpenUtau (dados)"
        );
        assert_eq!(
            PhonemizerMode::FrenchG2P.info().source,
            "OpenUtau (portado)"
        );
    }
}
