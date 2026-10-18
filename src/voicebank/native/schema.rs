use serde::{Deserialize, Serialize};

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
