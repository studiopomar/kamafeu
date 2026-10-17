use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    pub format: String,
    pub schema_version: String,
    pub voice_id: String,
    pub bank_id: String,
    pub bank_version: String,
    pub package_revision: u32,
    pub display_name: String,
    pub publisher: String,
    #[serde(default)]
    pub description: String,
    pub languages: Vec<String>,
    pub phonetic_profiles: Vec<PhoneticProfileRef>,
    pub engine: EngineRequirement,
    pub styles: Vec<Style>,
    pub layers: Vec<Layer>,
    pub defaults: Defaults,
    #[serde(default)]
    pub presentation: Option<Presentation>,
    #[serde(default)]
    pub license: Option<LicenseRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhoneticProfileRef {
    pub id: String,
    pub language: String,
    pub path: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EngineRequirement {
    pub id: String,
    pub api_major: u32,
    #[serde(default)]
    pub required_features: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Style {
    pub id: String,
    pub display_name: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Layer {
    pub id: String,
    pub style_id: String,
    pub recorded_midi: u8,
    pub units_path: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Defaults {
    pub phonetic_profile_id: String,
    pub style_id: String,
    pub layer_policy: String,
    pub missing_unit_policy: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Presentation {
    pub avatar: Option<String>,
    pub portrait: Option<String>,
    pub accent_color: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LicenseRef {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhoneticProfile {
    pub format: String,
    pub schema_version: String,
    pub id: String,
    pub language: String,
    pub symbols: Vec<PhoneSymbol>,
    #[serde(default)]
    pub lexicon: std::collections::BTreeMap<String, Vec<Pronunciation>>,
    pub unknown_word_policy: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhoneSymbol {
    pub id: String,
    pub ipa: String,
    pub class: String,
    pub voiced: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Pronunciation {
    pub phones: Vec<String>,
    #[serde(default)]
    pub stress_nuclei: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnitFile {
    pub format: String,
    pub schema_version: String,
    pub layer_id: String,
    pub assets: Vec<Asset>,
    pub units: Vec<Unit>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Asset {
    pub id: String,
    pub path: String,
    pub sample_rate_hz: u32,
    pub channels: u16,
    pub frame_count: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Unit {
    pub id: String,
    pub asset_id: String,
    pub kind: String,
    pub phones: Vec<String>,
    pub left_context: Context,
    pub right_context: Context,
    #[serde(default)]
    pub legacy_aliases: Vec<String>,
    pub source: SourceRegion,
    pub timing: UnitTiming,
    #[serde(default)]
    pub render: RenderTuning,
    #[serde(default)]
    pub quality: Quality,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Context {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub phone: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SourceRegion {
    pub start_frame: u64,
    pub end_frame: u64,
    pub anchor_frame: u64,
    pub fixed_end_frame: u64,
    #[serde(default)]
    pub sustain: Option<FrameRange>,
    #[serde(default)]
    pub release_start_frame: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FrameRange {
    pub start_frame: u64,
    pub end_frame: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UnitTiming {
    pub min_target_ms: f64,
    pub preferred_crossfade_ms: f64,
    pub max_crossfade_ms: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RenderTuning {
    #[serde(default)]
    pub sustain_mode: String,
    #[serde(default)]
    pub gain_db: f64,
    #[serde(default)]
    pub formant_shift_cents: f64,
}
impl Default for RenderTuning {
    fn default() -> Self {
        Self {
            sustain_mode: "finite".into(),
            gain_db: 0.0,
            formant_shift_cents: 0.0,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Quality {
    #[serde(default)]
    pub reviewed: bool,
    #[serde(default)]
    pub priority: i32,
}

#[derive(Debug, Clone)]
pub struct NativeBank {
    pub root: PathBuf,
    pub manifest: Manifest,
    pub profiles: Vec<PhoneticProfile>,
    pub layers: Vec<(Layer, UnitFile)>,
    /// Keeps extracted package data alive for as long as the bank is used.
    pub temp_dir: Option<Arc<tempfile::TempDir>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeCoverageSummary {
    pub profile_id: String,
    pub style_id: String,
    pub covered_phone_symbols: usize,
    pub total_phone_symbols: usize,
    pub unit_count: usize,
    pub asset_count: usize,
    pub reviewed_unit_count: usize,
}

impl NativeBank {
    /// Load either an authoring folder or a packaged `.kfv` snapshot.
    pub fn load_path(path: &Path) -> Result<Self, String> {
        if path.is_dir() {
            return Self::load_dir(path);
        }
        if !path.is_file()
            || !path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("kfv"))
        {
            return Err(format!(
                "KFV 2 path is not a folder or .kfv package: {}",
                path.display()
            ));
        }
        let temp_dir = tempfile::Builder::new()
            .prefix("kamafeu_kfv2_")
            .tempdir()
            .map_err(|e| format!("failed to create package workspace: {e}"))?;
        crate::oto::voicebank::extract_kfv(path, temp_dir.path())
            .map_err(|e| format!("failed to extract KFV package: {e}"))?;
        let mut bank = Self::load_dir(temp_dir.path())?;
        bank.temp_dir = Some(Arc::new(temp_dir));
        Ok(bank)
    }

    /// Load and cross-validate a KFV 2 folder. Audio is inspected by header;
    /// the sample payload is not decoded or cached at this stage.
    pub fn load_dir(root: &Path) -> Result<Self, String> {
        let root = std::fs::canonicalize(root)
            .map_err(|error| format!("cannot resolve voicebank folder: {error}"))?;
        if !root.is_dir() {
            return Err("KFV 2 voicebank root is not a directory".into());
        }
        let manifest_path = root.join("kamafeu_voicebank.json");
        let manifest_text = std::fs::read_to_string(&manifest_path)
            .map_err(|e| format!("{}: {e}", manifest_path.display()))?;
        let manifest = Manifest::parse_json(&manifest_text)?;
        let mut profiles = Vec::new();
        for reference in &manifest.phonetic_profiles {
            let path = manifest.resolve_existing(&root, &reference.path)?;
            let text =
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let profile: PhoneticProfile =
                serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
            profile.validate()?;
            if profile.id != reference.id || profile.language != reference.language {
                return Err(format!(
                    "phonetic profile identity mismatch: {}",
                    reference.path
                ));
            }
            profiles.push(profile);
        }
        let mut layers = Vec::new();
        for layer in &manifest.layers {
            let path = manifest.resolve_existing(&root, &layer.units_path)?;
            let text =
                std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            let units: UnitFile =
                serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
            let profile = profiles
                .iter()
                .find(|p| p.id == manifest.defaults.phonetic_profile_id)
                .ok_or_else(|| "default phonetic profile is missing".to_string())?;
            units.validate(&layer.id, profile)?;
            for asset in &units.assets {
                let asset_path = manifest.resolve_existing(&root, &asset.path)?;
                let reader = hound::WavReader::open(&asset_path)
                    .map_err(|e| format!("{}: WAV inválido: {e}", asset_path.display()))?;
                let spec = reader.spec();
                let frames = reader.duration() as u64;
                if spec.channels != asset.channels
                    || spec.sample_rate != asset.sample_rate_hz
                    || frames != asset.frame_count
                {
                    return Err(format!(
                        "{}: metadados WAV divergem do manifesto",
                        asset.path
                    ));
                }
            }
            layers.push((layer.clone(), units));
        }
        if let Some(presentation) = &manifest.presentation {
            for path in [
                presentation.avatar.as_deref(),
                presentation.portrait.as_deref(),
            ]
            .into_iter()
            .flatten()
            {
                manifest.resolve_existing(&root, path)?;
            }
        }
        if let Some(license) = &manifest.license {
            manifest.resolve_existing(&root, &license.path)?;
        }
        Ok(Self {
            root,
            manifest,
            profiles,
            layers,
            temp_dir: None,
        })
    }

    /// Hash the normalized manifest and referenced package content. This is
    /// intentionally an explicit operation: callers should compute it when
    /// binding/revalidating a bank, never on a GUI redraw or audio callback.
    pub fn content_sha256(&self) -> Result<String, String> {
        self.content_sha256_cancellable(None)
    }

    pub fn content_sha256_cancellable(
        &self,
        cancel: Option<&AtomicBool>,
    ) -> Result<String, String> {
        use sha2::{Digest, Sha256};
        use std::io::Read;

        let check_cancelled = || {
            if cancel.is_some_and(|token| token.load(Ordering::Relaxed)) {
                Err("KFV 2 content hash cancelled".to_string())
            } else {
                Ok(())
            }
        };
        check_cancelled()?;

        let root = std::fs::canonicalize(&self.root)
            .map_err(|error| format!("cannot resolve voicebank root: {error}"))?;
        let mut hasher = Sha256::new();
        let manifest = serde_json::to_vec(&self.manifest)
            .map_err(|error| format!("cannot normalize voicebank manifest: {error}"))?;
        hash_record(&mut hasher, "kamafeu_voicebank.json", &manifest);

        for reference in &self.manifest.phonetic_profiles {
            check_cancelled()?;
            let profile = self
                .profiles
                .iter()
                .find(|profile| profile.id == reference.id)
                .ok_or_else(|| format!("missing loaded phonetic profile: {}", reference.id))?;
            let bytes = serde_json::to_vec(profile)
                .map_err(|error| format!("cannot normalize phonetic profile: {error}"))?;
            hash_record(&mut hasher, &reference.path, &bytes);
        }
        for (layer, units) in &self.layers {
            check_cancelled()?;
            let bytes = serde_json::to_vec(units)
                .map_err(|error| format!("cannot normalize layer units: {error}"))?;
            hash_record(&mut hasher, &layer.units_path, &bytes);
            for asset in &units.assets {
                let path = self.manifest.resolve(&root, &asset.path)?;
                let canonical = std::fs::canonicalize(&path)
                    .map_err(|error| format!("cannot resolve asset '{}': {error}", asset.path))?;
                if !canonical.starts_with(&root) || !canonical.is_file() {
                    return Err(format!("asset escapes the voicebank root: {}", asset.path));
                }
                let mut file = std::fs::File::open(&canonical)
                    .map_err(|error| format!("cannot open asset '{}': {error}", asset.path))?;
                let length = file
                    .metadata()
                    .map_err(|error| format!("cannot inspect asset '{}': {error}", asset.path))?
                    .len();
                hasher.update((asset.path.len() as u64).to_be_bytes());
                hasher.update(asset.path.as_bytes());
                hasher.update(length.to_be_bytes());
                let mut buffer = [0u8; 64 * 1024];
                loop {
                    check_cancelled()?;
                    let read = file
                        .read(&mut buffer)
                        .map_err(|error| format!("cannot hash asset '{}': {error}", asset.path))?;
                    if read == 0 {
                        break;
                    }
                    hasher.update(&buffer[..read]);
                }
            }
        }
        let mut optional_files = std::collections::BTreeMap::new();
        if let Some(license) = &self.manifest.license {
            optional_files.insert(license.path.clone(), "license");
        }
        if let Some(presentation) = &self.manifest.presentation {
            for relative in [presentation.avatar.as_ref(), presentation.portrait.as_ref()]
                .into_iter()
                .flatten()
            {
                optional_files.insert(relative.clone(), "presentation asset");
            }
        }
        for (relative, name) in optional_files {
            check_cancelled()?;
            let path = self.manifest.resolve(&root, &relative)?;
            let canonical = std::fs::canonicalize(&path)
                .map_err(|error| format!("cannot resolve {name} '{}': {error}", relative))?;
            if !canonical.starts_with(&root) || !canonical.is_file() {
                return Err(format!("{name} escapes the voicebank root: {relative}"));
            }
            let mut file = std::fs::File::open(&canonical)
                .map_err(|error| format!("cannot read {name} '{}': {error}", relative))?;
            let length = file
                .metadata()
                .map_err(|error| format!("cannot inspect {name} '{}': {error}", relative))?
                .len();
            hasher.update((relative.len() as u64).to_be_bytes());
            hasher.update(relative.as_bytes());
            hasher.update(length.to_be_bytes());
            let mut buffer = [0u8; 16 * 1024];
            loop {
                check_cancelled()?;
                let read = file
                    .read(&mut buffer)
                    .map_err(|error| format!("cannot read {name} '{}': {error}", relative))?;
                if read == 0 {
                    break;
                }
                hasher.update(&buffer[..read]);
            }
        }
        check_cancelled()?;
        Ok(format!("sha256:{:x}", hasher.finalize()))
    }

    pub fn project_reference(
        &self,
        source_path_hint: Option<String>,
    ) -> Result<crate::project::NativeVoicebankReference, String> {
        Ok(crate::project::NativeVoicebankReference {
            bank_id: self.manifest.bank_id.clone(),
            bank_version: self.manifest.bank_version.clone(),
            package_revision: self.manifest.package_revision,
            content_sha256: self.content_sha256()?,
            engine_id: self.manifest.engine.id.clone(),
            engine_api_major: self.manifest.engine.api_major,
            planner_version: "kamafeu.planner.v1".into(),
            phonetic_profile_id: self.manifest.defaults.phonetic_profile_id.clone(),
            style_id: self.manifest.defaults.style_id.clone(),
            source_path_hint,
        })
    }

    /// Structural coverage only: counts profile symbols occurring in authored
    /// units, without estimating phrase coverage or acoustic suitability.
    pub fn coverage_summary(
        &self,
        profile_id: &str,
        style_id: &str,
    ) -> Result<NativeCoverageSummary, String> {
        let profile = self
            .profiles
            .iter()
            .find(|profile| profile.id == profile_id)
            .ok_or_else(|| format!("unknown phonetic profile: {profile_id}"))?;
        if !self
            .manifest
            .styles
            .iter()
            .any(|style| style.id == style_id)
        {
            return Err(format!("unknown KFV 2 style: {style_id}"));
        }
        let mut covered_phones = HashSet::new();
        let mut assets = HashSet::new();
        let mut unit_count = 0usize;
        let mut reviewed_unit_count = 0usize;
        for (_, units) in self
            .layers
            .iter()
            .filter(|(layer, _)| layer.style_id == style_id)
        {
            unit_count += units.units.len();
            reviewed_unit_count += units
                .units
                .iter()
                .filter(|unit| unit.quality.reviewed)
                .count();
            assets.extend(units.assets.iter().map(|asset| asset.path.as_str()));
            covered_phones.extend(
                units
                    .units
                    .iter()
                    .flat_map(|unit| unit.phones.iter().map(String::as_str)),
            );
        }
        Ok(NativeCoverageSummary {
            profile_id: profile_id.to_owned(),
            style_id: style_id.to_owned(),
            covered_phone_symbols: profile
                .symbols
                .iter()
                .filter(|symbol| covered_phones.contains(symbol.id.as_str()))
                .count(),
            total_phone_symbols: profile.symbols.len(),
            unit_count,
            asset_count: assets.len(),
            reviewed_unit_count,
        })
    }
}

fn hash_record(hasher: &mut sha2::Sha256, name: &str, bytes: &[u8]) {
    use sha2::Digest;
    hasher.update((name.len() as u64).to_be_bytes());
    hasher.update(name.as_bytes());
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= 200
}
pub fn safe_relative_path(value: &str) -> bool {
    if value.is_empty() || value.contains('\\') || value.contains('\0') {
        return false;
    }
    let path = Path::new(value);
    !path.is_absolute() && path.components().all(|c| matches!(c, Component::Normal(_)))
}
fn version_major(value: &str) -> Option<u64> {
    let parts: Vec<_> = value.split('.').collect();
    if parts.len() != 3
        || parts
            .iter()
            .any(|part| part.is_empty() || !part.bytes().all(|b| b.is_ascii_digit()))
    {
        return None;
    }
    let major = parts[0].parse().ok()?;
    parts[1].parse::<u64>().ok()?;
    parts[2].parse::<u64>().ok()?;
    Some(major)
}
fn valid_language_tag(value: &str) -> bool {
    let mut parts = value.split('-');
    let Some(primary) = parts.next() else {
        return false;
    };
    (2..=8).contains(&primary.len())
        && primary.bytes().all(|b| b.is_ascii_alphabetic())
        && parts.all(|part| {
            (1..=8).contains(&part.len()) && part.bytes().all(|b| b.is_ascii_alphanumeric())
        })
}

impl Manifest {
    pub fn parse_json(content: &str) -> Result<Self, String> {
        let value: serde_json::Value =
            serde_json::from_str(content).map_err(|e| format!("manifest JSON: {e}"))?;
        if value.get("format").and_then(|v| v.as_str()) != Some("kamafeu.voicebank") {
            return Err("manifest format must be kamafeu.voicebank".into());
        }
        let manifest: Self =
            serde_json::from_value(value).map_err(|e| format!("manifest fields: {e}"))?;
        manifest.validate()?;
        Ok(manifest)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.format != "kamafeu.voicebank" || version_major(&self.schema_version) != Some(2) {
            return Err("KFV 2 schema_version 2.x is required".into());
        }
        if [
            self.voice_id.as_str(),
            self.bank_id.as_str(),
            self.bank_version.as_str(),
            self.display_name.as_str(),
            self.publisher.as_str(),
        ]
        .iter()
        .any(|v| !valid_id(v))
        {
            return Err("manifest contains an empty or oversized identity field".into());
        }
        if version_major(&self.bank_version).is_none() {
            return Err("bank_version must use numeric SemVer (major.minor.patch)".into());
        }
        if self.package_revision == 0
            || self.engine.id != "kamafeu.concat"
            || self.engine.api_major != 1
        {
            return Err("unsupported package revision or native engine API".into());
        }
        const FEATURES: &[&str] = &["explicit-anchor", "sustain-region", "finite-units"];
        if self
            .engine
            .required_features
            .iter()
            .any(|feature| !FEATURES.contains(&feature.as_str()))
        {
            return Err("manifest requires an unsupported native engine feature".into());
        }
        if self.languages.is_empty()
            || self.phonetic_profiles.is_empty()
            || self.styles.is_empty()
            || self.layers.is_empty()
        {
            return Err("manifest must declare language, profile, style, and layer".into());
        }
        if self
            .languages
            .iter()
            .any(|language| !valid_language_tag(language))
            || self
                .phonetic_profiles
                .iter()
                .any(|profile| !valid_language_tag(&profile.language))
        {
            return Err("invalid BCP 47 language tag".into());
        }
        let mut ids = HashSet::new();
        for (id, path) in self.phonetic_profiles.iter().map(|x| (&x.id, &x.path)) {
            if !valid_id(id) || !ids.insert(id) || !safe_relative_path(path) {
                return Err(format!(
                    "invalid or duplicate phonetic profile reference: {id}"
                ));
            }
        }
        let language_tags: HashSet<_> = self.languages.iter().map(String::as_str).collect();
        if self
            .phonetic_profiles
            .iter()
            .any(|p| !language_tags.contains(p.language.as_str()))
        {
            return Err("phonetic profile language is not declared by the voicebank".into());
        }
        let profile_ids: HashSet<_> = self
            .phonetic_profiles
            .iter()
            .map(|x| x.id.as_str())
            .collect();
        let mut style_ids = HashSet::new();
        for style in &self.styles {
            if !valid_id(&style.id) || !style_ids.insert(style.id.as_str()) {
                return Err(format!("invalid or duplicate style id: {}", style.id));
            }
        }
        let mut layer_ids = HashSet::new();
        for layer in &self.layers {
            if !valid_id(&layer.id)
                || !layer_ids.insert(layer.id.as_str())
                || !style_ids.contains(layer.style_id.as_str())
                || layer.recorded_midi > 127
                || !safe_relative_path(&layer.units_path)
            {
                return Err(format!("invalid layer: {}", layer.id));
            }
        }
        if !profile_ids.contains(self.defaults.phonetic_profile_id.as_str())
            || !style_ids.contains(self.defaults.style_id.as_str())
        {
            return Err("manifest defaults reference an unknown profile or style".into());
        }
        if self.defaults.layer_policy != "nearest-recorded"
            || self.defaults.missing_unit_policy != "error"
        {
            return Err("unsupported layer or missing-unit policy".into());
        }
        for path in self
            .presentation
            .iter()
            .flat_map(|p| {
                [p.avatar.as_ref(), p.portrait.as_ref()]
                    .into_iter()
                    .flatten()
            })
            .chain(self.license.iter().map(|x| &x.path))
        {
            if !safe_relative_path(path) {
                return Err(format!("unsafe manifest path: {path}"));
            }
        }
        Ok(())
    }
    pub fn resolve(&self, root: &Path, relative: &str) -> Result<PathBuf, String> {
        if !safe_relative_path(relative) {
            return Err(format!("unsafe relative path: {relative}"));
        }
        Ok(root.join(relative))
    }

    /// Resolve an existing referenced file and reject symlinks that leave the
    /// bank root. Lexical path validation alone does not stop `link/file`.
    pub fn resolve_existing(&self, root: &Path, relative: &str) -> Result<PathBuf, String> {
        let root = std::fs::canonicalize(root)
            .map_err(|error| format!("cannot resolve voicebank root: {error}"))?;
        let path = self.resolve(&root, relative)?;
        let canonical = std::fs::canonicalize(&path)
            .map_err(|error| format!("referenced file is missing '{}': {error}", relative))?;
        if !canonical.starts_with(&root) || !canonical.is_file() {
            return Err(format!(
                "referenced file escapes the voicebank root: {relative}"
            ));
        }
        Ok(canonical)
    }
}

impl PhoneticProfile {
    pub fn validate(&self) -> Result<(), String> {
        if self.format != "kamafeu.phonetics"
            || version_major(&self.schema_version) != Some(1)
            || !valid_id(&self.id)
            || !valid_language_tag(&self.language)
        {
            return Err("invalid phonetic profile header".into());
        }
        let mut ids = HashSet::new();
        for symbol in &self.symbols {
            if !valid_id(&symbol.id) || !ids.insert(symbol.id.as_str()) {
                return Err(format!("invalid or duplicate phone symbol: {}", symbol.id));
            }
        }
        if !["ask-for-pronunciation", "error"].contains(&self.unknown_word_policy.as_str()) {
            return Err("unsupported unknown_word_policy".into());
        }
        for (word, pronunciations) in &self.lexicon {
            if word.trim().is_empty() || pronunciations.is_empty() {
                return Err(format!("empty lexicon entry: {word}"));
            }
            for p in pronunciations {
                if p.phones.is_empty()
                    || p.stress_nuclei.iter().any(|i| *i >= p.phones.len())
                    || p.phones.iter().any(|phone| !ids.contains(phone.as_str()))
                {
                    return Err(format!("invalid pronunciation for {word}"));
                }
            }
        }
        Ok(())
    }
}

impl UnitFile {
    pub fn validate(&self, expected_layer: &str, profile: &PhoneticProfile) -> Result<(), String> {
        if self.format != "kamafeu.units"
            || version_major(&self.schema_version) != Some(1)
            || self.layer_id != expected_layer
        {
            return Err("unit file format, version, or layer id mismatch".into());
        }
        let phones: HashSet<_> = profile.symbols.iter().map(|p| p.id.as_str()).collect();
        let mut assets = HashSet::new();
        for a in &self.assets {
            if !valid_id(&a.id)
                || !assets.insert(a.id.as_str())
                || !safe_relative_path(&a.path)
                || a.sample_rate_hz == 0
                || a.channels != 1
                || a.frame_count == 0
            {
                return Err(format!("invalid audio asset: {}", a.id));
            }
        }
        let mut units = HashSet::new();
        for u in &self.units {
            if !valid_id(&u.id)
                || !units.insert(u.id.as_str())
                || !assets.contains(u.asset_id.as_str())
                || u.phones.is_empty()
                || u.phones.iter().any(|p| !phones.contains(p.as_str()))
            {
                return Err(format!("invalid unit reference or phones: {}", u.id));
            }
            let asset = self
                .assets
                .iter()
                .find(|a| a.id == u.asset_id)
                .ok_or_else(|| format!("unit '{}' refers to missing asset", u.id))?;
            if !["CV", "VC", "V", "VV", "CC", "breath", "release"].contains(&u.kind.as_str()) {
                return Err(format!("unsupported unit kind in {}", u.id));
            }
            for context in [&u.left_context, &u.right_context] {
                match context.kind.as_str() {
                    "boundary" | "any" if context.phone.is_none() => {}
                    "phone"
                        if context
                            .phone
                            .as_ref()
                            .is_some_and(|p| phones.contains(p.as_str())) => {}
                    _ => return Err(format!("invalid phonetic context in {}", u.id)),
                }
            }
            let s = &u.source;
            if !(s.start_frame < s.end_frame
                && s.end_frame <= asset.frame_count
                && s.start_frame <= s.anchor_frame
                && s.anchor_frame < s.end_frame
                && s.anchor_frame <= s.fixed_end_frame
                && s.fixed_end_frame <= s.end_frame)
            {
                return Err(format!("invalid frame bounds in unit {}", u.id));
            }
            if let Some(r) = &s.sustain {
                if !(s.fixed_end_frame <= r.start_frame
                    && r.start_frame < r.end_frame
                    && r.end_frame <= s.end_frame)
                {
                    return Err(format!("invalid sustain region in unit {}", u.id));
                }
            }
            if !["finite", "loop", "stretch"].contains(&u.render.sustain_mode.as_str())
                || (u.render.sustain_mode != "finite" && s.sustain.is_none())
            {
                return Err(format!("invalid sustain mode in {}", u.id));
            }
            if let Some(r) = s.release_start_frame {
                if r < s.start_frame
                    || r >= s.end_frame
                    || s.sustain
                        .as_ref()
                        .is_some_and(|sustain| r < sustain.end_frame)
                {
                    return Err(format!("invalid release region in unit {}", u.id));
                }
            }
            if !u.timing.min_target_ms.is_finite()
                || u.timing.min_target_ms <= 0.0
                || !u.timing.preferred_crossfade_ms.is_finite()
                || !u.timing.max_crossfade_ms.is_finite()
                || u.timing.preferred_crossfade_ms < 0.0
                || u.timing.max_crossfade_ms < u.timing.preferred_crossfade_ms
                || !u.render.gain_db.is_finite()
                || !u.render.formant_shift_cents.is_finite()
            {
                return Err(format!("invalid timing or DSP values in unit {}", u.id));
            }
        }
        Ok(())
    }
}

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
        let options = zip::write::FileOptions::default();
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
        assert_eq!(world.engine_id, "kamafeu.lessampler-tainah");
        assert_eq!(world.units[0].requested_engine, "kamafeu.lessampler-tainah");
        assert_eq!(world.units[0].effective_engine, "kamafeu.lessampler-tainah");
        assert!(world.units[0].fallback_reasons.is_empty());
        assert!(!world.units[0].stages.is_empty());
        assert!(!world.samples.is_empty());
        assert!(world.samples.iter().all(|sample| sample.is_finite()));
    }
}
