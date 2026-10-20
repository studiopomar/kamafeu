//! KFV 2 authoring state for Copaíba, separate from the legacy oto.ini editor.

mod storage;
use crate::voicebank::native::{
    Defaults, EngineRequirement, Layer, Manifest, NativeBank, PhoneSymbol, PhoneticProfile,
    PhoneticProfileRef, Style, Unit, UnitFile,
};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use storage::write_json_atomic;

mod authoring;
use authoring::UnitAuthoringEdit;

#[derive(Debug, Clone)]
pub struct CopaibaNativeProject {
    pub root: PathBuf,
    pub manifest: Manifest,
    pub profiles: Vec<PhoneticProfile>,
    pub layers: Vec<(Layer, UnitFile)>,
    _workspace: Option<Arc<tempfile::TempDir>>,
    undo: Vec<UnitAuthoringEdit>,
    redo: Vec<UnitAuthoringEdit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayerCoverage {
    pub symbol_count: usize,
    pub covered_symbol_count: usize,
    pub missing_symbols: Vec<String>,
    pub unit_count: usize,
    pub reviewed_unit_count: usize,
    pub asset_count: usize,
}

impl CopaibaNativeProject {
    /// Create a new one-style, one-pitch KFV 2 authoring draft in a new folder
    /// under `parent`. The whole folder appears only after its initial schema
    /// and empty layer have passed the normal loader.
    pub fn create_new_draft(
        parent: &Path,
        voice_id: &str,
        bank_id: &str,
        display_name: &str,
        publisher: &str,
        profile: PhoneticProfile,
    ) -> Result<Self, String> {
        if !parent.is_dir() {
            return Err("choose an existing parent folder for the new voicebank".into());
        }
        profile.validate()?;
        let folder_name: String = bank_id
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() || character == '-' || character == '_' {
                    character
                } else {
                    '-'
                }
            })
            .collect();
        let folder_name = folder_name.trim_matches('-');
        if folder_name.is_empty() {
            return Err("bank id must produce a safe folder name".into());
        }
        let destination = parent.join(folder_name);
        if destination.exists() {
            return Err(format!(
                "destination already exists: {}",
                destination.display()
            ));
        }
        let profile_id = profile.id.clone();
        let language = profile.language.clone();
        let layer = Layer {
            id: "natural-midi-60".into(),
            style_id: "natural".into(),
            recorded_midi: 60,
            units_path: "units/natural-midi-60.json".into(),
        };
        let manifest = Manifest {
            format: "kamafeu.voicebank".into(),
            schema_version: "2.0.0".into(),
            voice_id: voice_id.trim().into(),
            bank_id: bank_id.trim().into(),
            bank_version: "0.1.0".into(),
            package_revision: 1,
            display_name: display_name.trim().into(),
            publisher: publisher.trim().into(),
            description: "New Copaíba KFV 2 authoring draft. Review audio, marks, and license before distribution.".into(),
            languages: vec![language.clone()],
            phonetic_profiles: vec![PhoneticProfileRef {
                id: profile_id.clone(), language,
                path: "phonetics/profile.json".into(),
            }],
            engine: EngineRequirement {
                id: "kamafeu.concat".into(), api_major: 1,
                required_features: vec!["explicit-anchor".into(), "finite-units".into()],
            },
            styles: vec![Style { id: "natural".into(), display_name: "Natural".into() }],
            layers: vec![layer.clone()],
            defaults: Defaults {
                phonetic_profile_id: profile_id, style_id: "natural".into(),
                layer_policy: "nearest-recorded".into(), missing_unit_policy: "error".into(),
            },
            presentation: None, license: None,
        };
        manifest.validate()?;
        let unit_file = UnitFile {
            format: "kamafeu.units".into(),
            schema_version: "1.0.0".into(),
            layer_id: layer.id.clone(),
            assets: Vec::new(),
            units: Vec::new(),
        };
        unit_file.validate(&layer.id, &profile)?;

        let staging = tempfile::Builder::new()
            .prefix(".kamafeu_kfv2_new_")
            .tempdir_in(parent)
            .map_err(|error| format!("cannot create staging folder: {error}"))?;
        fs::create_dir_all(staging.path().join("phonetics"))
            .map_err(|error| format!("cannot create profile folder: {error}"))?;
        fs::create_dir_all(staging.path().join("units"))
            .map_err(|error| format!("cannot create unit folder: {error}"))?;
        write_json_atomic(&staging.path().join("kamafeu_voicebank.json"), &manifest)?;
        write_json_atomic(&staging.path().join("phonetics/profile.json"), &profile)?;
        write_json_atomic(&staging.path().join(&layer.units_path), &unit_file)?;
        NativeBank::load_dir(staging.path())?;
        fs::rename(staging.path(), &destination)
            .map_err(|error| format!("cannot publish new draft folder atomically: {error}"))?;
        Self::load_dir(&destination)
    }

    /// Starter PT-BR symbol inventory only. It deliberately contains no G2P
    /// lexicon; creators must provide direct phonetic sequences or author one.
    pub fn starter_pt_br_profile() -> PhoneticProfile {
        let entries = [
            ("a", "a", "vowel", true),
            ("e", "e", "vowel", true),
            ("eh", "ɛ", "vowel", true),
            ("i", "i", "vowel", true),
            ("o", "o", "vowel", true),
            ("oh", "ɔ", "vowel", true),
            ("u", "u", "vowel", true),
            ("an", "ɐ̃", "vowel", true),
            ("en", "ẽ", "vowel", true),
            ("in", "ĩ", "vowel", true),
            ("on", "õ", "vowel", true),
            ("un", "ũ", "vowel", true),
            ("b", "b", "stop", true),
            ("p", "p", "stop", false),
            ("t", "t", "stop", false),
            ("d", "d", "stop", true),
            ("k", "k", "stop", false),
            ("g", "g", "stop", true),
            ("f", "f", "fricative", false),
            ("v", "v", "fricative", true),
            ("s", "s", "fricative", false),
            ("z", "z", "fricative", true),
            ("sh", "ʃ", "fricative", false),
            ("zh", "ʒ", "fricative", true),
            ("m", "m", "nasal", true),
            ("n", "n", "nasal", true),
            ("nh", "ɲ", "nasal", true),
            ("l", "l", "liquid", true),
            ("lh", "ʎ", "liquid", true),
            ("r", "ɾ", "liquid", true),
            ("rr", "ʁ", "liquid", true),
            ("ch", "tʃ", "affricate", false),
            ("dj", "dʒ", "affricate", true),
            ("y", "j", "glide", true),
            ("w", "w", "glide", true),
        ];
        PhoneticProfile {
            format: "kamafeu.phonetics".into(),
            schema_version: "1.0.0".into(),
            id: "pt-br-kamafeu-basic-v1".into(),
            language: "pt-BR".into(),
            symbols: entries
                .into_iter()
                .map(|(id, ipa, class, voiced)| PhoneSymbol {
                    id: id.into(),
                    ipa: ipa.into(),
                    class: class.into(),
                    voiced,
                })
                .collect(),
            lexicon: Default::default(),
            unknown_word_policy: "ask-for-pronunciation".into(),
        }
    }

    pub fn unit(&self, layer_id: &str, unit_id: &str) -> Option<&Unit> {
        self.layers
            .iter()
            .find(|(layer, _)| layer.id == layer_id)?
            .1
            .units
            .iter()
            .find(|unit| unit.id == unit_id)
    }

    /// Snapshot the validated authoring state for planner/render previews. The
    /// workspace owner remains alive on `self` while the snapshot is in use.
    pub fn native_bank_snapshot(&self) -> NativeBank {
        NativeBank {
            root: self.root.clone(),
            manifest: self.manifest.clone(),
            profiles: self.profiles.clone(),
            layers: self.layers.clone(),
            temp_dir: self._workspace.clone(),
        }
    }

    /// Read one imported/source asset for waveform comparison or playback.
    /// The package metadata is rechecked on every load; this method never
    /// writes back to the recording.
    pub fn load_asset_audio(
        &self,
        layer_id: &str,
        asset_id: &str,
    ) -> Result<(Vec<f32>, u32), String> {
        const MAX_COMPARISON_FRAMES: u64 = 8 * 1024 * 1024;
        let (_, file) = self
            .layers
            .iter()
            .find(|(layer, _)| layer.id == layer_id)
            .ok_or_else(|| format!("unknown KFV 2 layer: {layer_id}"))?;
        let asset = file
            .assets
            .iter()
            .find(|asset| asset.id == asset_id)
            .ok_or_else(|| format!("unknown KFV 2 asset: {asset_id}"))?;
        if asset.frame_count > MAX_COMPARISON_FRAMES {
            return Err("recording is too large to load for A/B comparison".into());
        }
        let path = self.manifest.resolve_existing(&self.root, &asset.path)?;
        let reader = hound::WavReader::open(&path)
            .map_err(|error| format!("cannot inspect KFV 2 recording: {error}"))?;
        let spec = reader.spec();
        if spec.channels != asset.channels
            || spec.sample_rate != asset.sample_rate_hz
            || reader.duration() as u64 != asset.frame_count
        {
            return Err(format!("recording metadata changed: {}", asset.path));
        }
        drop(reader);
        let (samples, sample_rate) = crate::renderer::TrackRenderer::load_wav_samples(&path)?;
        if sample_rate != asset.sample_rate_hz
            || samples.len() as u64 != asset.frame_count
            || samples.iter().any(|sample| !sample.is_finite())
        {
            return Err(format!(
                "recording data changed or is invalid: {}",
                asset.path
            ));
        }
        Ok((samples, sample_rate))
    }

    /// Plan a direct-phone coverage probe against one authored layer. A
    /// multi-phone sequence is represented by adjacent transitions, matching
    /// the KFV 2 project renderer. The fixed probe timing is for selection and
    /// preview only; it is not a musical timing recommendation.
    pub fn plan_direct_phone_phrase(
        &self,
        layer_id: &str,
        direct_phones: &str,
    ) -> Result<crate::voicebank::planner::PhrasePlan, String> {
        let (layer, _) = self
            .layers
            .iter()
            .find(|(layer, _)| layer.id == layer_id)
            .ok_or_else(|| format!("unknown KFV 2 layer: {layer_id}"))?;
        let profile = self
            .profiles
            .iter()
            .find(|profile| profile.id == self.manifest.defaults.phonetic_profile_id)
            .ok_or_else(|| "default phonetic profile is missing".to_string())?;
        let phones: Vec<String> = direct_phones
            .split_whitespace()
            .map(str::to_owned)
            .collect();
        if phones.is_empty() {
            return Err("enter one or more direct phone IDs for the phrase probe".into());
        }
        for phone in &phones {
            if !profile.symbols.iter().any(|symbol| symbol.id == *phone) {
                return Err(format!("unknown phone in phrase probe: {phone}"));
            }
        }
        let segments: Vec<Vec<String>> = if phones.len() == 1 {
            vec![phones]
        } else {
            phones
                .windows(2)
                .map(|pair| vec![pair[0].clone(), pair[1].clone()])
                .collect()
        };
        let mut events = Vec::with_capacity(segments.len());
        for (index, segment) in segments.into_iter().enumerate() {
            let classes: Vec<&str> = segment
                .iter()
                .map(|phone| {
                    profile
                        .symbols
                        .iter()
                        .find(|symbol| symbol.id == *phone)
                        .map(|symbol| symbol.class.as_str())
                        .ok_or_else(|| format!("unknown phone in phrase probe: {phone}"))
                })
                .collect::<Result<_, _>>()?;
            let is_vowel = |class: &str| class == "vowel";
            let kind = match classes.as_slice() {
                [class] if is_vowel(class) => "V",
                [class] if *class == "breath" => "breath",
                [class] if *class == "release" => "release",
                [left, right] if !is_vowel(left) && is_vowel(right) => "CV",
                [left, right] if is_vowel(left) && !is_vowel(right) => "VC",
                [left, right] if is_vowel(left) && is_vowel(right) => "VV",
                [_, _] => "CC",
                _ => return Err("phrase probe supports one or two phones per event".into()),
            };
            events.push(crate::voicebank::planner::PhoneEvent {
                phones: segment,
                kind: kind.into(),
                position_ms: index as f64 * 250.0,
                duration_ms: 250.0,
                midi: layer.recorded_midi,
                pitch_points: Vec::new(),
                unit_override_id: None,
                layer_override_id: Some(layer.id.clone()),
            });
        }
        let bank = self.native_bank_snapshot();
        crate::voicebank::planner::plan_phrase(
            &bank,
            &crate::voicebank::planner::PhraseRequest {
                style_id: layer.style_id.clone(),
                events,
            },
        )
    }

    /// Report inventory presence for one layer. This is structural symbol
    /// coverage, not a prediction of phrase or contextual coverage.
    pub fn layer_coverage(&self, layer_id: &str) -> Result<LayerCoverage, String> {
        let (_, file) = self
            .layers
            .iter()
            .find(|(layer, _)| layer.id == layer_id)
            .ok_or_else(|| format!("unknown layer: {layer_id}"))?;
        let profile = self
            .profiles
            .iter()
            .find(|profile| profile.id == self.manifest.defaults.phonetic_profile_id)
            .ok_or_else(|| "default phonetic profile is missing".to_string())?;
        let missing_symbols = profile
            .symbols
            .iter()
            .filter(|symbol| {
                !file
                    .units
                    .iter()
                    .any(|unit| unit.phones.iter().any(|phone| phone == &symbol.id))
            })
            .map(|symbol| symbol.id.clone())
            .collect::<Vec<_>>();
        Ok(LayerCoverage {
            symbol_count: profile.symbols.len(),
            covered_symbol_count: profile.symbols.len() - missing_symbols.len(),
            missing_symbols,
            unit_count: file.units.len(),
            reviewed_unit_count: file
                .units
                .iter()
                .filter(|unit| unit.quality.reviewed)
                .count(),
            asset_count: file.assets.len(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voicebank::native::{
        Asset, Context, Defaults, EngineRequirement, FrameRange, PhoneSymbol, PhoneticProfileRef,
        Quality, RenderTuning, SourceRegion, Style, UnitTiming,
    };

    fn project(root: &Path) -> CopaibaNativeProject {
        let manifest = Manifest {
            format: "kamafeu.voicebank".into(),
            schema_version: "2.0.0".into(),
            voice_id: "com.example.voice".into(),
            bank_id: "com.example.voice.natural".into(),
            bank_version: "1.0.0".into(),
            package_revision: 1,
            display_name: "Test".into(),
            publisher: "Fixture".into(),
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
                required_features: vec![
                    "explicit-anchor".into(),
                    "sustain-region".into(),
                    "finite-units".into(),
                ],
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
            symbols: vec![PhoneSymbol {
                id: "a".into(),
                ipa: "a".into(),
                class: "vowel".into(),
                voiced: true,
            }],
            lexicon: Default::default(),
            unknown_word_policy: "ask-for-pronunciation".into(),
        };
        let unit_file = UnitFile {
            format: "kamafeu.units".into(),
            schema_version: "1.0.0".into(),
            layer_id: "natural-c4".into(),
            assets: vec![Asset {
                id: "take1".into(),
                path: "audio/take1.wav".into(),
                sample_rate_hz: 48_000,
                channels: 1,
                frame_count: 48_000,
            }],
            units: vec![Unit {
                id: "a-natural-c4".into(),
                asset_id: "take1".into(),
                kind: "V".into(),
                phones: vec!["a".into()],
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
                    start_frame: 0,
                    end_frame: 48_000,
                    anchor_frame: 4_800,
                    fixed_end_frame: 4_800,
                    sustain: Some(FrameRange {
                        start_frame: 9_600,
                        end_frame: 36_000,
                    }),
                    release_start_frame: Some(40_000),
                },
                timing: UnitTiming {
                    min_target_ms: 80.0,
                    preferred_crossfade_ms: 15.0,
                    max_crossfade_ms: 40.0,
                },
                render: RenderTuning {
                    sustain_mode: "loop".into(),
                    gain_db: 0.0,
                    formant_shift_cents: 0.0,
                },
                quality: Quality::default(),
            }],
        };
        CopaibaNativeProject {
            root: root.to_path_buf(),
            manifest,
            profiles: vec![profile],
            layers: vec![(
                Layer {
                    id: "natural-c4".into(),
                    style_id: "natural".into(),
                    recorded_midi: 60,
                    units_path: "units/natural-c4.json".into(),
                },
                unit_file,
            )],
            _workspace: None,
            undo: Vec::new(),
            redo: Vec::new(),
        }
    }

    fn wav(path: &Path, seed: i16) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for frame in 0..48_000 {
            writer
                .write_sample(seed.wrapping_add(frame as i16))
                .unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn creates_a_complete_editable_ptbr_draft_without_hand_authored_bank_json() {
        let parent = tempfile::tempdir().unwrap();
        let profile = CopaibaNativeProject::starter_pt_br_profile();
        profile.validate().unwrap();
        assert!(profile.symbols.len() >= 30);
        assert!(profile.lexicon.is_empty());
        let mut project = CopaibaNativeProject::create_new_draft(
            parent.path(),
            "org.example.voice.ana",
            "org.example.voice.ana.natural",
            "Ana Natural",
            "Example Author",
            profile,
        )
        .unwrap();
        assert_eq!(project.layers[0].1.units.len(), 0);
        assert!(project.layers[0].1.assets.is_empty());
        assert_eq!(project.manifest.styles[0].id, "natural");
        assert_eq!(project.layers[0].0.recorded_midi, 60);

        let source_parent = tempfile::tempdir().unwrap();
        let source = source_parent.path().join("take.wav");
        wav(&source, 300);
        let original = fs::read(&source).unwrap();
        project
            .import_recording("natural-midi-60", &source, "take-one")
            .unwrap();
        project
            .add_unit_from_asset(
                "natural-midi-60",
                "take-one",
                "vowel-a",
                "V",
                vec!["a".into()],
            )
            .unwrap();
        let unit = project.unit("natural-midi-60", "vowel-a").unwrap();
        let mut marks = unit.source.clone();
        marks.anchor_frame = 4_000;
        marks.fixed_end_frame = 8_000;
        project
            .set_source_region("natural-midi-60", "vowel-a", marks)
            .unwrap();
        project
            .set_unit_reviewed("natural-midi-60", "vowel-a", true)
            .unwrap();

        let package = parent.path().join("ana.kfv");
        project.package_kfv2_draft(&package).unwrap();
        assert_eq!(fs::read(&source).unwrap(), original);
        let reopened = NativeBank::load_path(&package).unwrap();
        assert_eq!(reopened.manifest.bank_id, "org.example.voice.ana.natural");
        assert!(reopened.manifest.license.is_none());
        assert_eq!(reopened.layers[0].1.units[0].source.anchor_frame, 4_000);
        assert!(reopened.layers[0].1.units[0].quality.reviewed);
        assert!(CopaibaNativeProject::create_new_draft(
            parent.path(),
            "org.example.voice.ana",
            "org.example.voice.ana.natural",
            "Changed Name",
            "Example Author",
            CopaibaNativeProject::starter_pt_br_profile(),
        )
        .is_err());
    }

    #[test]
    fn mark_edits_are_validated_coalesced_and_undoable() {
        let root = tempfile::tempdir().unwrap();
        let mut project = project(root.path());
        let mut marks = project
            .unit("natural-c4", "a-natural-c4")
            .unwrap()
            .source
            .clone();
        marks.anchor_frame = 5_000;
        marks.fixed_end_frame = 5_000;
        project
            .set_source_region("natural-c4", "a-natural-c4", marks.clone())
            .unwrap();
        marks.anchor_frame = 5_200;
        marks.fixed_end_frame = 5_200;
        project
            .set_source_region("natural-c4", "a-natural-c4", marks)
            .unwrap();
        let preview_bank = project.native_bank_snapshot();
        assert_eq!(preview_bank.layers[0].1.units[0].source.anchor_frame, 5_200);
        assert!(project.undo().unwrap());
        assert_eq!(
            project
                .unit("natural-c4", "a-natural-c4")
                .unwrap()
                .source
                .anchor_frame,
            4_800
        );
        assert!(project.redo().unwrap());
        assert_eq!(
            project
                .unit("natural-c4", "a-natural-c4")
                .unwrap()
                .source
                .anchor_frame,
            5_200
        );
        project
            .set_unit_reviewed("natural-c4", "a-natural-c4", true)
            .unwrap();
        assert!(
            project
                .unit("natural-c4", "a-natural-c4")
                .unwrap()
                .quality
                .reviewed
        );
        assert!(project.undo().unwrap());
        assert!(
            !project
                .unit("natural-c4", "a-natural-c4")
                .unwrap()
                .quality
                .reviewed
        );
        assert!(project.redo().unwrap());
        assert!(
            project
                .unit("natural-c4", "a-natural-c4")
                .unwrap()
                .quality
                .reviewed
        );
        project
            .set_unit_reviewed("natural-c4", "a-natural-c4", false)
            .unwrap();
        assert!(
            !project
                .unit("natural-c4", "a-natural-c4")
                .unwrap()
                .quality
                .reviewed
        );
        assert!(project.undo().unwrap());
        assert_eq!(
            project
                .unit("natural-c4", "a-natural-c4")
                .unwrap()
                .source
                .anchor_frame,
            4_800
        );
        assert!(project.redo().unwrap());
        assert_eq!(
            project
                .unit("natural-c4", "a-natural-c4")
                .unwrap()
                .source
                .anchor_frame,
            5_200
        );
        let mut invalid = project
            .unit("natural-c4", "a-natural-c4")
            .unwrap()
            .source
            .clone();
        invalid.anchor_frame = 99_000;
        assert!(project
            .set_source_region("natural-c4", "a-natural-c4", invalid)
            .is_err());
        assert_eq!(
            project
                .unit("natural-c4", "a-natural-c4")
                .unwrap()
                .source
                .anchor_frame,
            5_200
        );
    }

    #[test]
    fn layer_coverage_reports_structural_symbol_gaps_and_review_count() {
        let root = tempfile::tempdir().unwrap();
        let mut project = project(root.path());
        project.profiles[0]
            .symbols
            .push(crate::voicebank::native::PhoneSymbol {
                id: "k".into(),
                ipa: "k".into(),
                class: "stop".into(),
                voiced: false,
            });
        project.layers[0].1.units[0].quality.reviewed = true;

        let coverage = project.layer_coverage("natural-c4").unwrap();
        assert_eq!(coverage.symbol_count, 2);
        assert_eq!(coverage.covered_symbol_count, 1);
        assert_eq!(coverage.missing_symbols, ["k"]);
        assert_eq!(coverage.unit_count, 1);
        assert_eq!(coverage.reviewed_unit_count, 1);
        assert_eq!(coverage.asset_count, 1);
    }

    #[test]
    fn direct_phone_phrase_probe_reports_planned_and_missing_coverage() {
        let root = tempfile::tempdir().unwrap();
        let project = project(root.path());
        let plan = project.plan_direct_phone_phrase("natural-c4", "a").unwrap();
        assert_eq!(plan.units.len(), 1);
        assert_eq!(plan.units[0].unit_id, "a-natural-c4");
        assert_eq!(plan.units[0].layer_id, "natural-c4");

        let missing = project
            .plan_direct_phone_phrase("natural-c4", "a a")
            .unwrap_err();
        assert!(missing.contains("no eligible KFV 2 unit for event 0: a a"));
        assert!(project
            .plan_direct_phone_phrase("natural-c4", "unknown")
            .unwrap_err()
            .contains("unknown phone"));
    }

    #[test]
    fn direct_phone_phrase_probe_uses_adjacent_contextual_transitions() {
        let root = tempfile::tempdir().unwrap();
        let mut project = project(root.path());
        project.profiles[0].symbols.extend([
            PhoneSymbol {
                id: "k".into(),
                ipa: "k".into(),
                class: "stop".into(),
                voiced: false,
            },
            PhoneSymbol {
                id: "s".into(),
                ipa: "s".into(),
                class: "fricative".into(),
                voiced: false,
            },
        ]);
        let units = &mut project.layers[0].1.units;
        let mut cv = units[0].clone();
        cv.id = "ka-1".into();
        cv.kind = "CV".into();
        cv.phones = vec!["k".into(), "a".into()];
        cv.left_context = Context {
            kind: "boundary".into(),
            phone: None,
        };
        cv.right_context = Context {
            kind: "any".into(),
            phone: None,
        };
        let mut vc = units[0].clone();
        vc.id = "as-1".into();
        vc.kind = "VC".into();
        vc.phones = vec!["a".into(), "s".into()];
        vc.left_context = Context {
            kind: "phone".into(),
            phone: Some("a".into()),
        };
        vc.right_context = Context {
            kind: "boundary".into(),
            phone: None,
        };
        units.clear();
        units.extend([cv, vc]);

        let plan = project
            .plan_direct_phone_phrase("natural-c4", "k a s")
            .unwrap();
        assert_eq!(plan.units.len(), 2);
        assert_eq!(plan.units[0].unit_id, "ka-1");
        assert_eq!(plan.units[1].unit_id, "as-1");
    }

    #[test]
    fn import_copies_source_bytes_and_draft_packaging_uses_referenced_files_only() {
        let root = tempfile::tempdir().unwrap();
        let source_dir = tempfile::tempdir().unwrap();
        let source = source_dir.path().join("take.wav");
        wav(&source, 300);
        let original = fs::read(&source).unwrap();
        let mut project = project(root.path());
        project
            .import_recording("natural-c4", &source, "imported-take")
            .unwrap();
        let (comparison_samples, comparison_rate) = project
            .load_asset_audio("natural-c4", "imported-take")
            .unwrap();
        assert_eq!(comparison_rate, 48_000);
        assert_eq!(comparison_samples.len(), 48_000);
        assert_eq!(fs::read(&source).unwrap(), original);
        assert_eq!(
            fs::read(root.path().join("audio/imports/imported-take.wav")).unwrap(),
            original
        );
        wav(&root.path().join("audio/take1.wav"), 100);
        fs::write(root.path().join("unused-notes.txt"), "exclude me").unwrap();
        project
            .set_unit_reviewed("natural-c4", "a-natural-c4", true)
            .unwrap();
        project.save_to_dir(root.path()).unwrap();
        let package = root.path().join("draft.kfv");
        project.package_kfv2_draft(&package).unwrap();
        let reopened = NativeBank::load_path(&package).unwrap();
        assert!(reopened.layers[0].1.units[0].quality.reviewed);
        let mut archive = zip::ZipArchive::new(fs::File::open(package).unwrap()).unwrap();
        let names = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().unwrap().into_owned())
            .collect::<BTreeSet<_>>();
        assert!(!names.contains("unused-notes.txt"));
        assert!(names.contains("audio/take1.wav"));
        assert!(names.contains("audio/imports/imported-take.wav"));
        assert!(names.contains("kamafeu_voicebank.json"));
    }

    #[test]
    fn package_opens_as_an_editable_workspace_without_mutating_its_source() {
        let root = tempfile::tempdir().unwrap();
        let mut source_project = project(root.path());
        wav(&root.path().join("audio/take1.wav"), 42);
        source_project.save_to_dir(root.path()).unwrap();
        let package = root.path().join("source.kfv");
        source_project.package_kfv2_draft(&package).unwrap();
        let original_package = fs::read(&package).unwrap();

        let mut editing = CopaibaNativeProject::load_path(&package).unwrap();
        assert!(editing._workspace.is_some());
        let workspace = editing.root.clone();
        let mut source = editing
            .unit("natural-c4", "a-natural-c4")
            .unwrap()
            .source
            .clone();
        source.anchor_frame = 5_100;
        source.fixed_end_frame = 5_100;
        editing
            .set_source_region("natural-c4", "a-natural-c4", source)
            .unwrap();
        editing.save_to_dir(&workspace).unwrap();
        assert_eq!(fs::read(&package).unwrap(), original_package);

        let edited_package = root.path().join("edited.kfv");
        editing.package_kfv2_draft(&edited_package).unwrap();
        let reopened = NativeBank::load_path(&edited_package).unwrap();
        assert_eq!(reopened.layers[0].1.units[0].source.anchor_frame, 5_100);
        drop(editing);
        assert!(!workspace.exists());
    }
}
