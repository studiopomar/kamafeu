use super::{NativeVoicebankReference, UNote};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UTrack {
    pub name: String,
    pub singer: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voicebank_path: Option<String>,
    /// Stable KFV 2 identity for this track. The legacy path above remains a
    /// location hint so older projects and imports remain readable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_voicebank: Option<NativeVoicebankReference>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phonemizer: Option<crate::phonemizer::PhonemizerMode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resampler: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wavtool: Option<String>,
    /// Flags inherited by notes on this track. Unknown UTAU flags are kept
    /// verbatim and merged by `EffectiveFlags` at render time.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub flags: String,
    pub volume_db: f64,
    pub pan: f64,
    pub mute: bool,
    pub solo: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fx_rack: Option<crate::audio::FxRackConfig>,
}

impl Default for UTrack {
    fn default() -> Self {
        Self {
            name: "Track 1".to_string(),
            singer: "Default Singer".to_string(),
            voicebank_path: None,
            native_voicebank: None,
            phonemizer: None,
            resampler: None,
            wavtool: None,
            flags: String::new(),
            volume_db: 0.0,
            pan: 0.0,
            mute: false,
            solo: false,
            fx_rack: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UVoicePart {
    pub name: String,
    pub track_index: usize,
    pub position_ms: f64,
    pub notes: Vec<UNote>,
}

impl UVoicePart {
    pub fn new(name: impl Into<String>, track_index: usize) -> Self {
        Self {
            name: name.into(),
            track_index,
            position_ms: 0.0,
            notes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UWavePart {
    pub name: String,
    pub file_path: String,
    pub track_index: usize,
    pub position_ms: f64,
    #[serde(default)]
    pub file_offset_ms: f64,
    #[serde(default)]
    pub volume_db: f64,
    #[serde(default)]
    pub duration_ms: f64,
}

impl UWavePart {
    pub fn new(name: impl Into<String>, file_path: impl Into<String>, track_index: usize) -> Self {
        let file_path_str = file_path.into();
        let duration_ms = crate::audio::probe_audio_file(&file_path_str)
            .map(|info| info.duration_ms)
            .unwrap_or(0.0);

        Self {
            name: name.into(),
            file_path: file_path_str,
            track_index,
            position_ms: 0.0,
            file_offset_ms: 0.0,
            volume_db: 0.0,
            duration_ms,
        }
    }
}
