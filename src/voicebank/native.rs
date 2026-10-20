use std::path::PathBuf;
use std::sync::Arc;

mod schema;
pub use schema::{
    Asset, Context, Defaults, EngineRequirement, FrameRange, Layer, LicenseRef, Manifest,
    PhoneSymbol, PhoneticProfile, PhoneticProfileRef, Presentation, Pronunciation, Quality,
    RenderTuning, SourceRegion, Style, Unit, UnitFile, UnitTiming,
};
mod validation;
pub use validation::safe_relative_path;

#[derive(Debug, Clone)]
pub struct NativeBank {
    pub root: PathBuf,
    pub manifest: Manifest,
    pub profiles: Vec<PhoneticProfile>,
    pub layers: Vec<(Layer, UnitFile)>,
    /// Keeps extracted package data alive for as long as the bank is used.
    pub temp_dir: Option<Arc<tempfile::TempDir>>,
}

mod coverage;
pub use coverage::NativeCoverageSummary;
mod identity;
mod loader;

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Manifest, PhoneticProfile, UnitFile) {
        let manifest = Manifest {
            format: "kamafeu.voicebank".into(),
            schema_version: "2.0.0".into(),
            voice_id: "com.example.voice".into(),
            bank_id: "com.example.voice.natural".into(),
            bank_version: "1.0.0".into(),
            package_revision: 1,
            display_name: "Voice".into(),
            publisher: "Example".into(),
            description: String::new(),
            languages: vec!["pt-BR".into()],
            phonetic_profiles: vec![PhoneticProfileRef {
                id: "pt-br-v1".into(),
                language: "pt-BR".into(),
                path: "phonetics/pt-BR.json".into(),
            }],
            engine: EngineRequirement {
                id: "kamafeu.concat".into(),
                api_major: 1,
                required_features: vec![],
            },
            styles: vec![Style {
                id: "natural".into(),
                display_name: "Natural".into(),
            }],
            layers: vec![Layer {
                id: "natural-c4".into(),
                style_id: "natural".into(),
                recorded_midi: 60,
                units_path: "units/natural-c4.json".into(),
            }],
            defaults: Defaults {
                phonetic_profile_id: "pt-br-v1".into(),
                style_id: "natural".into(),
                layer_policy: "nearest-recorded".into(),
                missing_unit_policy: "error".into(),
            },
            presentation: None,
            license: None,
        };
        let profile = PhoneticProfile {
            format: "kamafeu.phonetics".into(),
            schema_version: "1.0.0".into(),
            id: "pt-br-v1".into(),
            language: "pt-BR".into(),
            symbols: vec![
                PhoneSymbol {
                    id: "k".into(),
                    ipa: "k".into(),
                    class: "stop".into(),
                    voiced: false,
                },
                PhoneSymbol {
                    id: "a".into(),
                    ipa: "a".into(),
                    class: "vowel".into(),
                    voiced: true,
                },
            ],
            lexicon: Default::default(),
            unknown_word_policy: "ask-for-pronunciation".into(),
        };
        let units = UnitFile {
            format: "kamafeu.units".into(),
            schema_version: "1.0.0".into(),
            layer_id: "natural-c4".into(),
            assets: vec![Asset {
                id: "take1".into(),
                path: "audio/natural/c4/take1.wav".into(),
                sample_rate_hz: 48_000,
                channels: 1,
                frame_count: 48_000,
            }],
            units: vec![Unit {
                id: "ka-1".into(),
                asset_id: "take1".into(),
                kind: "CV".into(),
                phones: vec!["k".into(), "a".into()],
                left_context: Context {
                    kind: "boundary".into(),
                    phone: None,
                },
                right_context: Context {
                    kind: "any".into(),
                    phone: None,
                },
                legacy_aliases: vec![],
                source: SourceRegion {
                    start_frame: 4_800,
                    end_frame: 43_200,
                    anchor_frame: 8_160,
                    fixed_end_frame: 12_000,
                    sustain: Some(FrameRange {
                        start_frame: 14_400,
                        end_frame: 36_000,
                    }),
                    release_start_frame: Some(38_400),
                },
                timing: UnitTiming {
                    min_target_ms: 90.0,
                    preferred_crossfade_ms: 20.0,
                    max_crossfade_ms: 40.0,
                },
                render: Default::default(),
                quality: Default::default(),
            }],
        };
        (manifest, profile, units)
    }

    #[test]
    fn published_minimal_json_examples_match_the_native_loader_contract() {
        let manifest = Manifest::parse_json(include_str!(
            "../../docs/kfv2/examples/minimal/kamafeu_voicebank.json"
        ))
        .unwrap();
        let profile: PhoneticProfile = serde_json::from_str(include_str!(
            "../../docs/kfv2/examples/minimal/phonetics/pt-BR.json"
        ))
        .unwrap();
        let units: UnitFile = serde_json::from_str(include_str!(
            "../../docs/kfv2/examples/minimal/units/natural-c4.json"
        ))
        .unwrap();

        profile.validate().unwrap();
        units.validate("natural-c4", &profile).unwrap();
        assert_eq!(manifest.phonetic_profiles[0].id, profile.id);
        assert_eq!(manifest.phonetic_profiles[0].language, profile.language);
        assert_eq!(manifest.layers[0].id, units.layer_id);
    }

    #[test]
    fn validates_manifest_references_and_unit_coordinates() {
        let (manifest, profile, units) = fixture();
        manifest.validate().unwrap();
        profile.validate().unwrap();
        units.validate("natural-c4", &profile).unwrap();
    }

    #[test]
    fn rejects_unsafe_paths_and_broken_default_references() {
        let (mut manifest, _, _) = fixture();
        manifest.layers[0].units_path = "../outside.json".into();
        assert!(manifest.validate().is_err());
        let (mut manifest, _, _) = fixture();
        manifest.defaults.style_id = "missing".into();
        assert!(manifest.validate().is_err());
        assert!(!safe_relative_path("/absolute/file"));
        assert!(!safe_relative_path("a/../b"));
    }

    #[test]
    fn rejects_out_of_bounds_regions_and_unknown_phones() {
        let (_, profile, mut units) = fixture();
        units.units[0].source.sustain.as_mut().unwrap().end_frame = 50_000;
        assert!(units.validate("natural-c4", &profile).is_err());
        let (_, profile, mut units) = fixture();
        units.units[0].phones[0] = "unknown".into();
        assert!(units.validate("natural-c4", &profile).is_err());
    }

    #[test]
    fn discriminator_rejects_invalid_kfv2_without_oto_fallback() {
        let (manifest, _, _) = fixture();
        let json = serde_json::to_string(&manifest).unwrap();
        assert_eq!(Manifest::parse_json(&json).unwrap(), manifest);
        assert!(
            Manifest::parse_json(r#"{"format":"kamafeu.voicebank","schema_version":"3.0.0"}"#)
                .is_err()
        );
        assert!(Manifest::parse_json(r#"{"format":"classic"}"#).is_err());
    }

    #[test]
    fn loads_metadata_and_checks_the_real_wav_header() {
        let root = tempfile::tempdir().unwrap();
        let (manifest, profile, units) = fixture();
        std::fs::create_dir_all(root.path().join("phonetics")).unwrap();
        std::fs::create_dir_all(root.path().join("units")).unwrap();
        std::fs::create_dir_all(root.path().join("audio/natural/c4")).unwrap();
        std::fs::write(
            root.path().join("kamafeu_voicebank.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        std::fs::write(
            root.path().join("phonetics/pt-BR.json"),
            serde_json::to_vec(&profile).unwrap(),
        )
        .unwrap();
        std::fs::write(
            root.path().join("units/natural-c4.json"),
            serde_json::to_vec(&units).unwrap(),
        )
        .unwrap();
        let wav = root.path().join("audio/natural/c4/take1.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(wav, spec).unwrap();
        for _ in 0..48_000 {
            writer.write_sample(0i16).unwrap();
        }
        writer.finalize().unwrap();
        let bank = NativeBank::load_dir(root.path()).unwrap();
        assert_eq!(bank.manifest.bank_id, manifest.bank_id);
        assert_eq!(bank.layers.len(), 1);
        assert_eq!(bank.layers[0].1.units[0].id, "ka-1");
        let content_hash = bank.content_sha256().unwrap();
        assert!(content_hash.starts_with("sha256:"));
        let reference = bank
            .project_reference(Some(root.path().display().to_string()))
            .unwrap();
        assert_eq!(reference.bank_id, manifest.bank_id);
        assert_eq!(reference.content_sha256, content_hash);

        let wav = root.path().join("audio/natural/c4/take1.wav");
        let mut changed_wav = hound::WavWriter::create(
            &wav,
            hound::WavSpec {
                channels: 1,
                sample_rate: 48_000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for _ in 0..48_000 {
            changed_wav.write_sample(1i16).unwrap();
        }
        changed_wav.finalize().unwrap();
        assert_ne!(bank.content_sha256().unwrap(), content_hash);

        #[cfg(unix)]
        {
            let outside = tempfile::tempdir().unwrap();
            let outside_wav = outside.path().join("outside.wav");
            let mut writer = hound::WavWriter::create(
                &outside_wav,
                hound::WavSpec {
                    channels: 1,
                    sample_rate: 48_000,
                    bits_per_sample: 16,
                    sample_format: hound::SampleFormat::Int,
                },
            )
            .unwrap();
            for _ in 0..48_000 {
                writer.write_sample(0i16).unwrap();
            }
            writer.finalize().unwrap();
            std::os::unix::fs::symlink(
                &outside_wav,
                root.path().join("audio/natural/c4/escape.wav"),
            )
            .unwrap();
            let mut escaped_units = units;
            escaped_units.assets[0].path = "audio/natural/c4/escape.wav".into();
            std::fs::write(
                root.path().join("units/natural-c4.json"),
                serde_json::to_vec(&escaped_units).unwrap(),
            )
            .unwrap();
            assert!(NativeBank::load_dir(root.path())
                .unwrap_err()
                .contains("escapes the voicebank root"));
        }
    }

    #[test]
    fn folder_and_kfv_package_load_the_same_identity_and_units() {
        let root = tempfile::tempdir().unwrap();
        let (manifest, profile, units) = fixture();
        std::fs::create_dir_all(root.path().join("phonetics")).unwrap();
        std::fs::create_dir_all(root.path().join("units")).unwrap();
        std::fs::create_dir_all(root.path().join("audio/natural/c4")).unwrap();
        std::fs::write(
            root.path().join("kamafeu_voicebank.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        std::fs::write(
            root.path().join("phonetics/pt-BR.json"),
            serde_json::to_vec(&profile).unwrap(),
        )
        .unwrap();
        std::fs::write(
            root.path().join("units/natural-c4.json"),
            serde_json::to_vec(&units).unwrap(),
        )
        .unwrap();
        let wav_path = root.path().join("audio/natural/c4/take1.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut wav = hound::WavWriter::create(&wav_path, spec).unwrap();
        for _ in 0..48_000 {
            wav.write_sample(0i16).unwrap();
        }
        wav.finalize().unwrap();

        let folder_bank = NativeBank::load_dir(root.path()).unwrap();
        let package = root.path().join("fixture.kfv");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&package).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        for (path, name) in [
            (
                root.path().join("kamafeu_voicebank.json"),
                "kamafeu_voicebank.json",
            ),
            (
                root.path().join("phonetics/pt-BR.json"),
                "phonetics/pt-BR.json",
            ),
            (
                root.path().join("units/natural-c4.json"),
                "units/natural-c4.json",
            ),
            (wav_path, "audio/natural/c4/take1.wav"),
        ] {
            zip.start_file(name, options).unwrap();
            std::io::copy(&mut std::fs::File::open(path).unwrap(), &mut zip).unwrap();
        }
        zip.finish().unwrap();
        let package_bank = NativeBank::load_path(&package).unwrap();
        assert_eq!(folder_bank.manifest.bank_id, package_bank.manifest.bank_id);
        assert_eq!(folder_bank.layers, package_bank.layers);
        assert_eq!(
            folder_bank.content_sha256().unwrap(),
            package_bank.content_sha256().unwrap()
        );
        let summary = crate::voicebank::package::inspect_package_metadata(&package).unwrap();
        assert_eq!(summary.bank_id, manifest.bank_id);
        assert_eq!(summary.display_name, manifest.display_name);
        assert_eq!(summary.layer_count, 1);
        let installed_path = root.path().join("installed-voice");
        let report =
            crate::voicebank::package::install_package_atomic(&package, &installed_path).unwrap();
        assert_eq!(report.bank_id, manifest.bank_id);
        assert_eq!(report.content_sha256, folder_bank.content_sha256().unwrap());
        assert_eq!(
            NativeBank::load_dir(&installed_path)
                .unwrap()
                .manifest
                .bank_version,
            manifest.bank_version
        );
        assert!(
            crate::voicebank::package::install_package_atomic(&package, &installed_path)
                .unwrap_err()
                .contains("already exists")
        );
        assert!(package_bank.temp_dir.is_some());
        let request = crate::voicebank::planner::PhraseRequest {
            style_id: "natural".into(),
            events: vec![crate::voicebank::planner::PhoneEvent {
                phones: vec!["k".into(), "a".into()],
                kind: "CV".into(),
                position_ms: 250.0,
                duration_ms: 200.0,
                midi: 60,
                pitch_points: Vec::new(),
                unit_override_id: None,
                layer_override_id: None,
            }],
        };
        assert_eq!(
            crate::voicebank::planner::plan_phrase(&folder_bank, &request).unwrap(),
            crate::voicebank::planner::plan_phrase(&package_bank, &request).unwrap()
        );
        assert!(matches!(
            crate::voicebank::VoicebankHandle::open(&package).unwrap(),
            crate::voicebank::VoicebankHandle::NativeV2(_)
        ));
    }

    #[test]
    fn aps_native_track_uses_its_bound_bank_and_persists_selection_decisions() {
        use crate::project::{UNote, UPhonemeOverride, UProject};

        let root = tempfile::tempdir().unwrap();
        let (manifest, mut profile, mut units) = fixture();
        profile.symbols.push(PhoneSymbol {
            id: "s".into(),
            ipa: "s".into(),
            class: "fricative".into(),
            voiced: false,
        });
        let mut vc_unit = units.units[0].clone();
        vc_unit.id = "as-1".into();
        vc_unit.kind = "VC".into();
        vc_unit.phones = vec!["a".into(), "s".into()];
        vc_unit.left_context = Context {
            kind: "phone".into(),
            phone: Some("a".into()),
        };
        units.units.push(vc_unit);
        std::fs::create_dir_all(root.path().join("phonetics")).unwrap();
        std::fs::create_dir_all(root.path().join("units")).unwrap();
        std::fs::create_dir_all(root.path().join("audio/natural/c4")).unwrap();
        std::fs::write(
            root.path().join("kamafeu_voicebank.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        std::fs::write(
            root.path().join("phonetics/pt-BR.json"),
            serde_json::to_vec(&profile).unwrap(),
        )
        .unwrap();
        std::fs::write(
            root.path().join("units/natural-c4.json"),
            serde_json::to_vec(&units).unwrap(),
        )
        .unwrap();
        let wav_path = root.path().join("audio/natural/c4/take1.wav");
        let mut wav = hound::WavWriter::create(
            &wav_path,
            hound::WavSpec {
                channels: 1,
                sample_rate: 48_000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for frame in 0..48_000 {
            let sample =
                ((frame as f64 * std::f64::consts::TAU * 220.0 / 48_000.0).sin() * 12_000.0) as i16;
            wav.write_sample(sample).unwrap();
        }
        wav.finalize().unwrap();
        let source_bank = NativeBank::load_dir(root.path()).unwrap();
        let package_workspace = tempfile::tempdir().unwrap();
        let package_path = package_workspace.path().join("authoring.kfv");
        let mut authoring =
            crate::copaiba::native_project::CopaibaNativeProject::load_dir(root.path()).unwrap();
        authoring.package_kfv2_draft(&package_path).unwrap();
        let installed_path = package_workspace.path().join("installed-voicebank");
        let install_report =
            crate::voicebank::package::install_package_atomic(&package_path, &installed_path)
                .unwrap();
        let bank = NativeBank::load_dir(&installed_path).unwrap();
        assert_eq!(install_report.bank_id, source_bank.manifest.bank_id);
        assert_eq!(
            bank.content_sha256().unwrap(),
            install_report.content_sha256
        );
        assert_eq!(bank.manifest.bank_id, source_bank.manifest.bank_id);
        let coverage = bank.coverage_summary("pt-br-v1", "natural").unwrap();
        assert_eq!(coverage.covered_phone_symbols, 3);
        assert_eq!(coverage.total_phone_symbols, 3);
        assert_eq!(coverage.unit_count, 2);
        assert_eq!(coverage.asset_count, 1);
        assert_eq!(coverage.reviewed_unit_count, 0);
        let reference = bank
            .project_reference(Some(installed_path.display().to_string()))
            .unwrap();
        let mut project = UProject::default();
        project.tracks[0].native_voicebank = Some(reference);
        let mut note = UNote::new("k a s", "C4", 0.0, 240.0);
        note.phoneme_durations_ms = vec![80.0, 80.0, 80.0];
        note.phoneme_overrides.push(UPhonemeOverride {
            index: 0,
            native_unit_id: Some("ka-1".into()),
            native_layer_id: Some("natural-c4".into()),
            ..Default::default()
        });
        note.phoneme_overrides.push(UPhonemeOverride {
            index: 1,
            native_unit_id: Some("as-1".into()),
            native_layer_id: Some("natural-c4".into()),
            ..Default::default()
        });
        project.parts[0].notes.push(note);

        let reopened_project: UProject =
            serde_json::from_slice(&serde_json::to_vec(&project).unwrap()).unwrap();
        assert_eq!(
            reopened_project.tracks[0].native_voicebank,
            project.tracks[0].native_voicebank
        );
        assert_eq!(
            reopened_project.parts[0].notes[0].phoneme_overrides,
            project.parts[0].notes[0].phoneme_overrides
        );
        project = reopened_project;

        let result =
            crate::voicebank::project::render_project_track_sola(&bank, &project, 0, 48_000)
                .unwrap();
        assert_eq!(result.plan.units.len(), 2);
        assert_eq!(result.plan.units[0].unit_id, "ka-1");
        assert_eq!(result.plan.units[0].layer_id, "natural-c4");
        assert_eq!(
            result.plan.units[0].reason,
            crate::voicebank::planner::SelectionReason::Override
        );
        assert_eq!(result.plan.units[1].unit_id, "as-1");
        assert_eq!(result.plan.units[1].phones, vec!["a", "s"]);
        assert_eq!(result.plan.units[1].requested_midi, 60);
        assert_eq!(result.rendered.engine_id, "kamafeu.sola");
        assert!(result
            .rendered
            .samples
            .iter()
            .any(|sample| sample.abs() > 1.0e-4));

        let track_render =
            crate::renderer::ProjectRenderer::render_native_track_sola(&project, &bank, 0, 48_000)
                .unwrap();
        assert_eq!(track_render.audio.channels, 2);
        assert_eq!(track_render.audio.sample_rate, 48_000);
        assert!(track_render.audio.frame_count() > 0);
        assert!(track_render.audio.error.is_none());

        let classic_voicebank = crate::oto::voicebank::Voicebank {
            root_path: PathBuf::from("."),
            name: "Unused classic bank".into(),
            author: String::new(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries: std::collections::HashMap::new(),
            case_insensitive_entries: std::sync::OnceLock::new(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };
        let render_messages = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
        let render_messages_callback = render_messages.clone();
        let report_render_message = move |_progress: f32, message: &str| {
            render_messages_callback
                .lock()
                .unwrap()
                .push(message.to_owned());
        };
        let project_render =
            crate::renderer::ProjectRenderer::render_project_range_with_drivers_cancellable(
                &project,
                &classic_voicebank,
                48_000,
                0.0,
                Some(500.0),
                &crate::drivers::resampler_driver::NativeResamplerDriver,
                &crate::drivers::wavtool_driver::NativeWavtoolDriver,
                &crate::renderer::RenderOptions::default(),
                Some(&report_render_message),
                None,
            );
        assert!(project_render.error.is_none());
        assert_eq!(project_render.frame_count(), 24_000);
        assert!(project_render
            .samples
            .iter()
            .any(|sample| sample.abs() > 1.0e-4));
        assert!(render_messages
            .lock()
            .unwrap()
            .iter()
            .any(|message| message.contains("ka-1") && message.contains("override")));

        let cancelled = std::sync::atomic::AtomicBool::new(true);
        let cancelled_project_render =
            crate::renderer::ProjectRenderer::render_project_range_with_drivers_cancellable(
                &project,
                &classic_voicebank,
                48_000,
                0.0,
                Some(500.0),
                &crate::drivers::resampler_driver::NativeResamplerDriver,
                &crate::drivers::wavtool_driver::NativeWavtoolDriver,
                &crate::renderer::RenderOptions::default(),
                None,
                Some(&cancelled),
            );
        assert!(cancelled_project_render.error.is_none());
        assert!(cancelled_project_render.samples.is_empty());
        assert!(bank
            .content_sha256_cancellable(Some(&cancelled))
            .unwrap_err()
            .contains("cancelled"));
        assert!(
            crate::voicebank::project::render_project_track_sola_cancellable(
                &bank,
                &project,
                0,
                48_000,
                Some(&cancelled),
            )
            .unwrap_err()
            .contains("cancelled")
        );

        let mut stale_project = project;
        stale_project.tracks[0]
            .native_voicebank
            .as_mut()
            .unwrap()
            .bank_version = "9.9.9".into();
        assert!(crate::voicebank::project::render_project_track_sola(
            &bank,
            &stale_project,
            0,
            48_000
        )
        .unwrap_err()
        .contains("does not match"));
    }

    #[test]
    fn facade_keeps_legacy_oto_banks_on_the_classic_loader() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("oto.ini"),
            "sample.wav=ka,0,100,-400,80,30\n",
        )
        .unwrap();
        assert!(matches!(
            crate::voicebank::VoicebankHandle::open(root.path()).unwrap(),
            crate::voicebank::VoicebankHandle::Classic(_)
        ));
    }

    #[test]
    fn facade_does_not_fall_back_when_a_kfv2_manifest_is_invalid() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(
            root.path().join("kamafeu_voicebank.json"),
            r#"{"format":"kamafeu.voicebank","schema_version":"2.0.0"}"#,
        )
        .unwrap();
        std::fs::write(
            root.path().join("oto.ini"),
            "sample.wav=ka,0,100,-400,80,30\n",
        )
        .unwrap();
        let error = crate::voicebank::VoicebankHandle::open(root.path()).unwrap_err();
        assert!(error.contains("manifest fields"));
    }

    #[test]
    fn phrase_plan_is_deterministic_and_preserves_an_explicit_unit_override() {
        let (manifest, profile, units) = fixture();
        let layer = manifest.layers[0].clone();
        let bank = NativeBank {
            root: PathBuf::new(),
            manifest,
            profiles: vec![profile],
            layers: vec![(layer, units)],
            temp_dir: None,
        };
        let request = crate::voicebank::planner::PhraseRequest {
            style_id: "natural".into(),
            events: vec![crate::voicebank::planner::PhoneEvent {
                phones: vec!["k".into(), "a".into()],
                kind: "CV".into(),
                position_ms: 100.0,
                duration_ms: 40.0,
                midi: 60,
                pitch_points: Vec::new(),
                unit_override_id: Some("ka-1".into()),
                layer_override_id: None,
            }],
        };
        let first = crate::voicebank::planner::plan_phrase(&bank, &request).unwrap();
        let second = crate::voicebank::planner::plan_phrase(&bank, &request).unwrap();
        assert_eq!(first, second);
        let serialized = serde_json::to_string(&first).unwrap();
        assert_eq!(
            serde_json::from_str::<crate::voicebank::planner::PhrasePlan>(&serialized).unwrap(),
            first
        );
        assert_eq!(first.bank_id, "com.example.voice.natural");
        assert_eq!(first.units[0].unit_id, "ka-1");
        assert_eq!(
            first.units[0].reason,
            crate::voicebank::planner::SelectionReason::Override
        );
        assert_eq!(first.units[0].output_start_ms, 30.0);
        assert_eq!(first.units[0].target_duration_ms, 110.0);
    }

    #[test]
    fn phrase_plan_reports_missing_coverage_instead_of_silent_fallback() {
        let (manifest, profile, units) = fixture();
        let layer = manifest.layers[0].clone();
        let bank = NativeBank {
            root: PathBuf::new(),
            manifest,
            profiles: vec![profile],
            layers: vec![(layer, units)],
            temp_dir: None,
        };
        let request = crate::voicebank::planner::PhraseRequest {
            style_id: "natural".into(),
            events: vec![crate::voicebank::planner::PhoneEvent {
                phones: vec!["z".into()],
                kind: "V".into(),
                position_ms: 0.0,
                duration_ms: 100.0,
                midi: 60,
                pitch_points: Vec::new(),
                unit_override_id: None,
                layer_override_id: None,
            }],
        };
        let error = crate::voicebank::planner::plan_phrase(&bank, &request).unwrap_err();
        assert!(error.contains("no eligible KFV 2 unit"));
    }

    #[test]
    fn phrase_plan_uses_coverage_before_nearest_pitch_and_priority_before_order() {
        let (manifest, profile, mut units) = fixture();
        let mut preferred = units.units[0].clone();
        preferred.id = "ka-priority".into();
        preferred.quality.priority = 10;
        units.units.insert(0, preferred);
        let base_layer = manifest.layers[0].clone();
        let mut near_layer = base_layer.clone();
        near_layer.id = "near-cs4".into();
        near_layer.recorded_midi = 61;
        let near_units = super::UnitFile {
            layer_id: near_layer.id.clone(),
            units: Vec::new(),
            ..units.clone()
        };
        let bank = NativeBank {
            root: PathBuf::new(),
            manifest,
            profiles: vec![profile],
            layers: vec![(near_layer, near_units), (base_layer, units)],
            temp_dir: None,
        };
        let request = crate::voicebank::planner::PhraseRequest {
            style_id: "natural".into(),
            events: vec![crate::voicebank::planner::PhoneEvent {
                phones: vec!["k".into(), "a".into()],
                kind: "CV".into(),
                position_ms: 100.0,
                duration_ms: 150.0,
                midi: 61,
                pitch_points: Vec::new(),
                unit_override_id: None,
                layer_override_id: None,
            }],
        };
        let plan = crate::voicebank::planner::plan_phrase(&bank, &request).unwrap();
        assert_eq!(plan.units[0].layer_id, "natural-c4");
        assert_eq!(plan.units[0].unit_id, "ka-priority");
        assert_eq!(plan.units[0].selected_midi, 60);
        assert_eq!(plan.units[0].requested_midi, 61);
    }

    #[test]
    fn provisional_sola_renderer_produces_a_timed_audible_phrase() {
        let root = tempfile::tempdir().unwrap();
        let (manifest, profile, units) = fixture();
        std::fs::create_dir_all(root.path().join("phonetics")).unwrap();
        std::fs::create_dir_all(root.path().join("units")).unwrap();
        std::fs::create_dir_all(root.path().join("audio/natural/c4")).unwrap();
        std::fs::write(
            root.path().join("kamafeu_voicebank.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        std::fs::write(
            root.path().join("phonetics/pt-BR.json"),
            serde_json::to_vec(&profile).unwrap(),
        )
        .unwrap();
        std::fs::write(
            root.path().join("units/natural-c4.json"),
            serde_json::to_vec(&units).unwrap(),
        )
        .unwrap();
        let wav = root.path().join("audio/natural/c4/take1.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(wav, spec).unwrap();
        for frame in 0..48_000 {
            let sample = (0.2
                * (f64::from(frame) * std::f64::consts::TAU * 220.0 / 48_000.0).sin()
                * i16::MAX as f64) as i16;
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
        let bank = NativeBank::load_dir(root.path()).unwrap();
        let request = crate::voicebank::planner::PhraseRequest {
            style_id: "natural".into(),
            events: vec![crate::voicebank::planner::PhoneEvent {
                phones: vec!["k".into(), "a".into()],
                kind: "CV".into(),
                position_ms: 100.0,
                duration_ms: 100.0,
                midi: 60,
                pitch_points: Vec::new(),
                unit_override_id: None,
                layer_override_id: None,
            }],
        };
        let plan = crate::voicebank::planner::plan_phrase(&bank, &request).unwrap();
        let rendered = crate::voicebank::render::render_phrase_sola(&bank, &plan, 48_000).unwrap();
        assert_eq!(rendered.engine_id, "kamafeu.sola");
        assert_eq!(rendered.start_time_ms, 0.0);
        assert_eq!(rendered.samples.len(), 9_600);
        assert!(rendered.units[0].peak > 0.01);
        assert!(rendered.samples.iter().all(|sample| sample.is_finite()));

        let world = crate::voicebank::render::render_phrase_world(&bank, &plan, 48_000).unwrap();
        assert_eq!(world.engine_id, "kamafeu.tainah-motor");
        assert_eq!(world.units[0].requested_engine, "kamafeu.tainah-motor");
        assert_eq!(world.units[0].effective_engine, "kamafeu.tainah-motor");
        assert!(world.units[0].fallback_reasons.is_empty());
        assert!(!world.units[0].stages.is_empty());
        assert!(!world.samples.is_empty());
        assert!(world.samples.iter().all(|sample| sample.is_finite()));
    }
}
