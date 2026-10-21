use crate::dsp::envelope::UtauEnvelope;
use crate::dsp::pitch::{midi_to_note_name, note_name_to_midi, VibratoParam};
use serde::{Deserialize, Serialize};

#[path = "model/expression.rs"]
mod expression;
pub use expression::{UExpressionPoint, UExpressions, UPitchBend, UPitchBendPoint};
#[path = "model/parts.rs"]
mod parts;
pub use parts::{UTrack, UVoicePart, UWavePart};
#[path = "model/operations.rs"]
mod operations;

/// Controls how explicit phonetic data from an imported project is treated.
/// Preserve is intentionally the compatibility default: imported aliases are
/// authored data, not a hint to silently rewrite the whole phrase.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum ImportedPhonemePolicy {
    #[default]
    Preserve,
    Recalculate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct UPhonemeOverride {
    pub index: usize,
    /// Stable KFV 2 selection overrides. These are not aliases and do not
    /// affect classic oto.ini resolution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_unit_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub native_layer_id: Option<String>,
    /// OpenUtau alternate suffix used while resolving the phoneme alias.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alternate: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phoneme: Option<String>,
    /// Literal UTAU flags scoped to this phoneme. Unknown tokens are retained.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flags: Option<String>,
    /// Offset from the position proposed by the phonemizer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset_ms: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preutter_delta_ms: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlap_delta_ms: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack_time_delta_ms: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release_time_delta_ms: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub consonant_timing_offset_ms: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub velocity: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub volume: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub attack: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decay: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub breathiness: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gender: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modulation: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pitch_delta: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dynamics: Option<f64>,
    /// Optional UTAU envelope replacing the note envelope for this phoneme.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub envelope: Option<UtauEnvelope>,
    /// Optional vibrato curve scoped to this phoneme.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vibrato: Option<VibratoParam>,
    /// Optional pitch/portamento curve scoped to this phoneme.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pitch_bend: Option<UPitchBend>,
}

impl UPhonemeOverride {
    /// Repairs values from permissive imports without changing the override's
    /// presence semantics. `None` continues to mean inheritance.
    pub fn normalize(&mut self, note_duration_ms: f64) {
        if let Some(value) = &mut self.consonant_timing_offset_ms {
            *value = if value.is_finite() {
                value.clamp(-500.0, 500.0)
            } else {
                0.0
            };
        }
        for delta in [
            &mut self.preutter_delta_ms,
            &mut self.overlap_delta_ms,
            &mut self.attack_time_delta_ms,
            &mut self.release_time_delta_ms,
        ]
        .into_iter()
        .flatten()
        {
            *delta = if delta.is_finite() {
                delta.clamp(-2_000.0, 2_000.0)
            } else {
                0.0
            };
        }
        if let Some(envelope) = &mut self.envelope {
            envelope.normalize(note_duration_ms);
        }
        if let Some(vibrato) = &mut self.vibrato {
            vibrato.normalize();
        }
        if let Some(pitch_bend) = &mut self.pitch_bend {
            if !pitch_bend.portamento_start_ms.is_finite() {
                pitch_bend.portamento_start_ms = -40.0;
            }
            if !pitch_bend.portamento_length_ms.is_finite() {
                pitch_bend.portamento_length_ms = 80.0;
            }
            pitch_bend.portamento_start_ms = pitch_bend.portamento_start_ms.clamp(-2000.0, 2000.0);
            pitch_bend.portamento_length_ms = pitch_bend.portamento_length_ms.clamp(1.0, 2000.0);
            if pitch_bend.portamento_shape.trim().is_empty() {
                pitch_bend.portamento_shape = "io".to_string();
            }
            pitch_bend.points.retain(|point| {
                point.time_offset_ms.is_finite() && point.pitch_offset_cents.is_finite()
            });
            pitch_bend.points.sort_by(|left, right| {
                left.time_offset_ms
                    .partial_cmp(&right.time_offset_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UNote {
    /// Pitch source used for voicebank lookup; None keeps automatic selection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voicebank_pitch: Option<String>,
    /// None inherits the active voicebank timbre; Some("") selects its neutral bank.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timbre: Option<String>,
    pub lyric: String,
    /// Optional explicit phonetic hint, equivalent to OpenUtau's
    /// `phoneticHint`. When present, phonemizers must not reinterpret the
    /// visible lyric before resolving aliases.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phonetic_hint: Option<String>,
    pub pitch: String,
    #[serde(alias = "start_time_ms")]
    pub position_ms: f64,
    pub duration_ms: f64,
    #[serde(default)]
    pub envelope: UtauEnvelope,
    #[serde(default)]
    pub vibrato: VibratoParam,
    #[serde(default)]
    pub pitch_bend: UPitchBend,
    #[serde(default)]
    pub expressions: UExpressions,
    #[serde(default)]
    pub flags: String,
    /// Durações autoradas dos subfonemas de uma nota composta. Projetos
    /// antigos deixam este vetor vazio e continuam usando divisão igual.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub phoneme_durations_ms: Vec<f64>,
    /// Per-phoneme positions and oto.ini deltas imported from USTX or authored in
    /// Kamafeu. These remain independent from the musical note duration.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub phoneme_overrides: Vec<UPhonemeOverride>,
    /// Optional per-note phonemizer override. `None` uses the track/global mode.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phonemizer_override: Option<String>,
}

impl UNote {
    /// Returns each note's previous and next note by musical position, while
    /// preserving the caller's indices for editing and selection state.
    pub(crate) fn time_neighbors(notes: &[UNote]) -> Vec<(Option<usize>, Option<usize>)> {
        let mut order: Vec<usize> = (0..notes.len()).collect();
        order.sort_by(|&left, &right| {
            notes[left]
                .position_ms
                .total_cmp(&notes[right].position_ms)
                .then_with(|| left.cmp(&right))
        });
        let mut neighbors = vec![(None, None); notes.len()];
        for pair in order.windows(2) {
            neighbors[pair[1]].0 = Some(pair[0]);
            neighbors[pair[0]].1 = Some(pair[1]);
        }
        neighbors
    }

    /// Two notes share a portamento boundary when their ends meet within the
    /// editor's one millisecond timing precision.
    pub(crate) fn are_adjacent(previous_end_ms: f64, next_start_ms: f64) -> bool {
        previous_end_ms.is_finite()
            && next_start_ms.is_finite()
            && (previous_end_ms - next_start_ms).abs() <= 1.0
    }

    pub fn new(
        lyric: impl Into<String>,
        pitch: impl Into<String>,
        position_ms: f64,
        duration_ms: f64,
    ) -> Self {
        let lyric_str = lyric.into();
        let pitch_str = pitch.into();
        Self {
            lyric: lyric_str,
            phonetic_hint: None,
            voicebank_pitch: None,
            timbre: None,
            pitch: pitch_str,
            position_ms,
            duration_ms,
            envelope: UtauEnvelope::default(),
            vibrato: VibratoParam::default(),
            pitch_bend: UPitchBend::default(),
            expressions: UExpressions::default(),
            flags: String::new(),
            phoneme_durations_ms: Vec::new(),
            phoneme_overrides: Vec::new(),
            phonemizer_override: None,
        }
    }

    /// Restaura todos os parâmetros de expressão, envelopes UTAU, vibrato, curvas de afinação,
    /// flags e durações fonéticas da nota para seus valores padrão de fábrica.
    pub fn reset_all_parameters(&mut self) {
        self.voicebank_pitch = None;
        self.timbre = None;
        self.envelope = UtauEnvelope::default();
        self.vibrato = VibratoParam::default();
        self.pitch_bend = UPitchBend::default();
        self.expressions = UExpressions::default();
        self.flags.clear();
        self.phoneme_durations_ms.clear();
        self.phoneme_overrides.clear();
        self.phonemizer_override = None;
    }

    /// Resolve durações internas válidas para `count` fonemas, preservando as
    /// proporções autoradas quando a nota inteira foi redimensionada.
    pub fn resolved_phoneme_durations(&self, count: usize) -> Vec<f64> {
        if count == 0 {
            return Vec::new();
        }
        let total = self.duration_ms.max(0.0);
        let authored_sum: f64 = self.phoneme_durations_ms.iter().sum();
        if self.phoneme_durations_ms.len() == count
            && authored_sum.is_finite()
            && authored_sum > 0.0
            && self
                .phoneme_durations_ms
                .iter()
                .all(|duration| duration.is_finite() && *duration > 0.0)
        {
            let scale = total / authored_sum;
            return self
                .phoneme_durations_ms
                .iter()
                .map(|duration| duration * scale)
                .collect();
        }
        vec![total / count as f64; count]
    }

    /// Move uma fronteira entre dois subfonemas sem alterar a duração total
    /// nem as demais divisões.
    pub fn set_phoneme_boundary(&mut self, count: usize, boundary: usize, position_ms: f64) {
        if count < 2 || boundary == 0 || boundary >= count {
            return;
        }
        let mut durations = if self.phoneme_durations_ms.len() == count {
            self.phoneme_durations_ms.clone()
        } else {
            self.resolved_phoneme_durations(count)
        };
        let pair_start: f64 = durations[..boundary - 1].iter().sum();
        let pair_total = durations[boundary - 1] + durations[boundary];
        let minimum = 10.0f64.min(pair_total / 2.0);
        let left = (position_ms - pair_start).clamp(minimum, pair_total - minimum);
        durations[boundary - 1] = left;
        durations[boundary] = pair_total - left;
        self.phoneme_durations_ms = durations;
    }

    pub fn midi_key(&self) -> u8 {
        note_name_to_midi(&self.pitch)
            .or_else(|| self.pitch.parse::<u8>().ok())
            .unwrap_or(60)
    }

    pub fn set_midi_key(&mut self, key: u8) {
        self.pitch = midi_to_note_name(key);
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UProjectMarker {
    pub name: String,
    pub position_ms: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

impl UProjectMarker {
    pub fn new(name: impl Into<String>, position_ms: f64) -> Self {
        Self {
            name: name.into(),
            position_ms,
            color: None,
        }
    }
}

/// A named span on the arrangement timeline, useful for verses, choruses and
/// review ranges. Sections deliberately remain independent from notes so they
/// survive editing, retiming and track changes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UProjectSection {
    pub name: String,
    pub start_ms: f64,
    pub end_ms: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
}

/// Persisted, non-musical state from the most recent progressive render.
/// Keeping this separate from notes means an interrupted preview can be
/// inspected or retried without changing the composition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UProjectRenderState {
    pub pipeline_fingerprint: String,
    pub chunks: Vec<UProjectRenderChunk>,
    pub scheduler_generation: u64,
    pub scheduler_state: String,
    pub scheduler_played_ms: f64,
    pub scheduler_queued_until_ms: f64,
    pub scheduler_last_event: String,
}

impl Default for UProjectRenderState {
    fn default() -> Self {
        Self {
            pipeline_fingerprint: String::new(),
            chunks: Vec::new(),
            scheduler_generation: 0,
            scheduler_state: "idle".to_string(),
            scheduler_played_ms: 0.0,
            scheduler_queued_until_ms: 0.0,
            scheduler_last_event: String::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UProjectRenderChunk {
    pub index: usize,
    pub start_ms: f64,
    pub end_ms: f64,
    pub is_final: bool,
    pub status: String,
    pub error: Option<String>,
}

impl Default for UProjectRenderChunk {
    fn default() -> Self {
        Self {
            index: 0,
            start_ms: 0.0,
            end_ms: 0.0,
            is_final: false,
            status: "unknown".to_string(),
            error: None,
        }
    }
}

impl UProjectSection {
    pub fn new(name: impl Into<String>, start_ms: f64, end_ms: f64) -> Self {
        Self {
            name: name.into(),
            start_ms,
            end_ms,
            color: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UProject {
    /// Version of the native APS data contract. Missing in legacy projects.
    #[serde(default = "default_project_schema_version")]
    pub schema_version: u32,
    pub name: String,
    pub bpm: f64,
    #[serde(default = "default_time_signature_numerator")]
    pub time_signature_numerator: u8,
    #[serde(default = "default_time_signature_denominator")]
    pub time_signature_denominator: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voicebank: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voicebank_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phonemizer: Option<crate::phonemizer::PhonemizerMode>,
    /// Import policy for explicit aliases/phoneme overrides. Optional in the
    /// serialized format so older projects keep the preserve behavior.
    #[serde(default, skip_serializing_if = "is_default_imported_phoneme_policy")]
    pub imported_phoneme_policy: ImportedPhonemePolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resampler: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wavtool: Option<String>,
    /// Project-level literal UTAU flags. More specific scopes override the
    /// same token while preserving tokens unknown to Kamafeu.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub flags: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_threads: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub render_state: Option<UProjectRenderState>,
    /// Extensions associated with this project. The host may keep entries
    /// visible even when they are currently unavailable or incompatible.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extensions: Vec<UProjectExtension>,
    pub tracks: Vec<UTrack>,
    pub parts: Vec<UVoicePart>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub wave_parts: Vec<UWavePart>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub markers: Vec<UProjectMarker>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sections: Vec<UProjectSection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UProjectExtension {
    pub id: String,
    pub version: String,
    pub origin: String,
    pub manifest_fingerprint: String,
}

/// Reproducibility metadata for a native voicebank bound to this project.
/// Stable IDs and hashes are authoritative; the source path is only a hint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeVoicebankReference {
    pub bank_id: String,
    pub bank_version: String,
    pub package_revision: u32,
    pub content_sha256: String,
    pub engine_id: String,
    pub engine_api_major: u32,
    pub planner_version: String,
    pub phonetic_profile_id: String,
    pub style_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path_hint: Option<String>,
}

fn default_time_signature_numerator() -> u8 {
    4
}

pub const CURRENT_PROJECT_SCHEMA_VERSION: u32 = 4;

fn default_project_schema_version() -> u32 {
    CURRENT_PROJECT_SCHEMA_VERSION
}

fn default_time_signature_denominator() -> u8 {
    4
}

fn is_default_imported_phoneme_policy(policy: &ImportedPhonemePolicy) -> bool {
    *policy == ImportedPhonemePolicy::Preserve
}

impl Default for UProject {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_PROJECT_SCHEMA_VERSION,
            name: "Novo Projeto".to_string(),
            bpm: 120.0,
            time_signature_numerator: 4,
            time_signature_denominator: 4,
            voicebank: None,
            voicebank_path: None,
            phonemizer: None,
            imported_phoneme_policy: ImportedPhonemePolicy::Preserve,
            resampler: None,
            wavtool: None,
            flags: String::new(),
            sample_rate: None,
            render_threads: None,
            render_state: None,
            extensions: Vec::new(),
            tracks: vec![UTrack::default()],
            parts: vec![UVoicePart::new("Parte Vocal 1", 0)],
            wave_parts: Vec::new(),
            markers: Vec::new(),
            sections: Vec::new(),
        }
    }
}

pub fn create_astro_boy_1980_project() -> UProject {
    let mut project = UProject {
        name: "Astro Boy (1980) - Tetsuwan Atom Theme".to_string(),
        bpm: 140.0,
        tracks: vec![UTrack::default()],
        parts: vec![UVoicePart::new("Astro Boy Opening Track", 0)],
        ..UProject::default()
    };

    let ms_per_beat = 60000.0 / 140.0; // ~428.57 ms per quarter beat

    let melody_data = vec![
        ("ra", "C4", 0.0, 1.0),
        ("ra", "D4", 1.0, 1.0),
        ("ra", "E4", 2.0, 1.0),
        ("su", "F4", 3.0, 1.0),
        ("bo", "G4", 4.0, 1.5),
        ("ra", "A4", 5.5, 1.5),
        ("ko", "G4", 7.0, 2.0),
        ("so", "C5", 9.0, 1.0),
        ("ra", "B4", 10.0, 1.0),
        ("wo", "A4", 11.0, 1.0),
        ("ko", "G4", 12.0, 1.5),
        ("e", "F4", 13.5, 1.0),
        ("te", "E4", 14.5, 2.0),
        ("te", "D4", 16.5, 1.0),
        ("tsu", "C4", 17.5, 1.0),
        ("wa", "E4", 18.5, 1.0),
        ("n", "G4", 19.5, 1.0),
        ("a", "C5", 20.5, 2.0),
        ("to", "B4", 22.5, 1.0),
        ("mu", "C5", 23.5, 3.0),
    ];

    let mut notes = Vec::new();
    for (lyric, pitch, beat_offset, beat_dur) in melody_data {
        let pos_ms = beat_offset * ms_per_beat;
        let dur_ms = beat_dur * ms_per_beat;
        let mut note = UNote::new(lyric, pitch, pos_ms, dur_ms);

        if lyric == "a" || lyric == "mu" || lyric == "ko" {
            note.pitch_bend.points = vec![
                UPitchBendPoint {
                    time_offset_ms: -40.0,
                    pitch_offset_cents: 0.0,
                    shape: "".to_string(),
                },
                UPitchBendPoint {
                    time_offset_ms: dur_ms * 0.35,
                    pitch_offset_cents: 35.0,
                    shape: "s".to_string(),
                },
                UPitchBendPoint {
                    time_offset_ms: dur_ms * 0.7,
                    pitch_offset_cents: -25.0,
                    shape: "s".to_string(),
                },
                UPitchBendPoint {
                    time_offset_ms: dur_ms,
                    pitch_offset_cents: 0.0,
                    shape: "s".to_string(),
                },
            ];
        }

        notes.push(note);
    }

    project.parts[0].notes = notes;
    project
}

#[cfg(test)]
mod project_tests {
    use super::*;

    #[test]
    fn normalize_restores_project_invariants() {
        let mut project = UProject {
            name: "Broken".to_string(),
            bpm: f64::NAN,
            tracks: Vec::new(),
            parts: vec![UVoicePart {
                name: "Part".to_string(),
                track_index: 2,
                position_ms: -50.0,
                notes: vec![UNote::new("ka", "C4", -10.0, -20.0)],
            }],
            ..UProject::default()
        };
        project.normalize();

        assert_eq!(project.bpm, 120.0);
        assert_eq!(project.tracks.len(), 3);
        assert_eq!(project.parts[0].position_ms, 0.0);
        assert_eq!(project.parts[0].notes[0].duration_ms, 1.0);
    }

    #[test]
    fn normalize_repairs_per_phoneme_vocal_controls() {
        let mut project = UProject::default();
        let mut note = UNote::new("k a", "C4", 0.0, 400.0);
        note.phoneme_overrides.push(UPhonemeOverride {
            index: 0,
            envelope: Some(UtauEnvelope {
                p2: f64::NAN,
                ..Default::default()
            }),
            vibrato: Some(VibratoParam {
                period_ms: f64::NAN,
                ..Default::default()
            }),
            pitch_bend: Some(UPitchBend {
                portamento_length_ms: f64::INFINITY,
                points: vec![UPitchBendPoint {
                    time_offset_ms: f64::NAN,
                    pitch_offset_cents: 0.0,
                    shape: String::new(),
                }],
                ..Default::default()
            }),
            consonant_timing_offset_ms: Some(f64::INFINITY),
            ..Default::default()
        });
        project.parts[0].notes.push(note);

        project.normalize();
        let override_ = &project.parts[0].notes[0].phoneme_overrides[0];
        assert_eq!(override_.envelope.as_ref().unwrap().p2, 5.0);
        assert_eq!(override_.vibrato.as_ref().unwrap().period_ms, 175.0);
        assert_eq!(
            override_.pitch_bend.as_ref().unwrap().portamento_length_ms,
            80.0
        );
        assert!(override_.pitch_bend.as_ref().unwrap().points.is_empty());
        assert_eq!(override_.consonant_timing_offset_ms, Some(0.0));
    }

    #[test]
    fn normalize_restores_a_safe_time_signature() {
        let mut project = UProject {
            time_signature_numerator: 0,
            time_signature_denominator: 3,
            ..Default::default()
        };
        project.normalize();

        assert_eq!(project.time_signature_numerator, 1);
        assert_eq!(project.time_signature_denominator, 4);
    }

    #[test]
    fn normalize_deduplicates_and_orders_project_extensions() {
        let mut project = UProject {
            extensions: vec![
                UProjectExtension {
                    id: " org.example.b ".into(),
                    version: "1".into(),
                    origin: "b".into(),
                    manifest_fingerprint: "2".into(),
                },
                UProjectExtension {
                    id: "org.example.a".into(),
                    version: "1".into(),
                    origin: "a".into(),
                    manifest_fingerprint: "1".into(),
                },
                UProjectExtension {
                    id: "org.example.b".into(),
                    version: "1".into(),
                    origin: "b".into(),
                    manifest_fingerprint: "2".into(),
                },
            ],
            ..Default::default()
        };
        project.normalize();

        assert_eq!(project.extensions.len(), 2);
        assert_eq!(project.extensions[0].id, "org.example.a");
        assert_eq!(project.extensions[1].id, "org.example.b");
    }

    #[test]
    fn tempo_change_preserves_beat_positions() {
        let mut project = UProject {
            bpm: 120.0,
            ..UProject::default()
        };
        project.parts[0].position_ms = 500.0;
        let mut note = UNote::new("a", "C4", 1_000.0, 250.0);
        note.pitch_bend.points.push(UPitchBendPoint {
            time_offset_ms: 125.0,
            pitch_offset_cents: 50.0,
            shape: "s".to_string(),
        });
        project.parts[0].notes.push(note);

        let scale = project.set_bpm_preserving_beats(60.0).unwrap();

        assert_eq!(scale, 2.0);
        assert_eq!(project.bpm, 60.0);
        assert_eq!(project.parts[0].position_ms, 1_000.0);
        assert_eq!(project.parts[0].notes[0].position_ms, 2_000.0);
        assert_eq!(project.parts[0].notes[0].duration_ms, 500.0);
        assert_eq!(
            project.parts[0].notes[0].pitch_bend.points[0].time_offset_ms,
            250.0
        );
    }

    #[test]
    fn invalid_tempo_is_rejected_without_changing_project() {
        let mut project = UProject::default();
        let original = project.parts[0].position_ms;

        assert_eq!(project.set_bpm_preserving_beats(f64::NAN), None);
        assert_eq!(project.bpm, 120.0);
        assert_eq!(project.parts[0].position_ms, original);
    }

    #[test]
    fn markers_normalize_sort_and_follow_tempo_changes() {
        let mut project = UProject {
            markers: vec![
                UProjectMarker {
                    name: "  Chorus  ".to_string(),
                    position_ms: 2_000.0,
                    color: Some("#00ffaa".to_string()),
                },
                UProjectMarker {
                    name: " ".to_string(),
                    position_ms: f64::NAN,
                    color: Some("not-a-color".to_string()),
                },
            ],
            ..Default::default()
        };
        project.normalize();
        assert_eq!(project.markers[0].name, "Marcador 2");
        assert_eq!(project.markers[0].position_ms, 0.0);
        assert_eq!(project.markers[0].color, None);
        assert_eq!(project.markers[1].name, "Chorus");
        assert_eq!(project.markers[1].color.as_deref(), Some("#00ffaa"));

        project.set_bpm_preserving_beats(240.0);
        assert_eq!(project.markers[1].position_ms, 1_000.0);
    }

    #[test]
    fn sections_normalize_and_follow_tempo_changes() {
        let mut project = UProject {
            sections: vec![
                UProjectSection::new("  Chorus  ", 2_000.0, 1_000.0),
                UProjectSection {
                    name: " ".to_string(),
                    start_ms: f64::NAN,
                    end_ms: f64::NAN,
                    color: Some("bad".to_string()),
                },
            ],
            ..Default::default()
        };
        project.normalize();
        assert_eq!(project.sections[0].name, "Seção 2");
        assert_eq!(project.sections[0].start_ms, 0.0);
        assert_eq!(project.sections[0].end_ms, 0.0);
        assert_eq!(project.sections[0].color, None);
        assert_eq!(project.sections[1].name, "Chorus");
        assert_eq!(project.sections[1].start_ms, 2_000.0);
        assert_eq!(project.sections[1].end_ms, 2_000.0);

        project.set_bpm_preserving_beats(240.0);
        assert_eq!(project.sections[1].start_ms, 1_000.0);
        assert_eq!(project.sections[1].end_ms, 1_000.0);
    }

    #[test]
    fn hand_drawn_pitch_after_onset_keeps_automatic_portamento() {
        let bend = UPitchBend {
            points: vec![UPitchBendPoint {
                time_offset_ms: 30.0,
                pitch_offset_cents: 25.0,
                shape: "l".to_string(),
            }],
            ..UPitchBend::default()
        };

        let points = bend.effective_points(Some(60), 62, true);
        assert_eq!(points[0].time_offset_ms, -40.0);
        assert_eq!(points[0].pitch_offset_cents, -200.0);
        assert!(points.iter().any(|point| {
            (point.time_offset_ms - 30.0).abs() < 1e-6
                && (point.pitch_offset_cents - 25.0).abs() < 1e-6
        }));
    }

    #[test]
    fn automatic_portamento_tracks_current_neighbor_without_becoming_authored_data() {
        let bend = UPitchBend::default();

        let high_note = bend.effective_points(Some(60), 72, true);
        assert_eq!(high_note[0].pitch_offset_cents, -1_200.0);

        let moved_note = bend.effective_points(Some(65), 67, true);
        assert_eq!(moved_note[0].pitch_offset_cents, -200.0);

        let separated_note = bend.effective_points(Some(65), 67, false);
        assert_eq!(separated_note[0].pitch_offset_cents, 0.0);
        assert!(bend.points.is_empty());
    }

    #[test]
    fn note_neighbors_follow_timeline_order_after_a_note_is_moved() {
        let notes = [
            UNote::new("high", "G4", 500.0, 250.0),
            UNote::new("low", "C4", 0.0, 250.0),
            UNote::new("next", "D4", 250.0, 250.0),
        ];
        let neighbors = UNote::time_neighbors(&notes);

        assert_eq!(neighbors[0].0, Some(2));
        assert_eq!(neighbors[0].1, None);
        assert_eq!(neighbors[1].0, None);
        assert_eq!(neighbors[1].1, Some(2));
        assert_eq!(neighbors[2].0, Some(1));
        assert_eq!(neighbors[2].1, Some(0));
        assert!(UNote::are_adjacent(250.0, 250.8));
        assert!(!UNote::are_adjacent(250.0, 251.1));
    }

    #[test]
    fn test_reset_all_parameters_restores_defaults() {
        let mut note = UNote::new("ka", "A4", 480.0, 480.0);
        note.expressions.dynamics = 45.0;
        note.expressions.gender = -20.0;
        note.expressions.breathiness = 30.0;
        note.expressions.velocity = 150.0;
        note.expressions.consonant_velocity = 120.0;
        note.expressions.modulation = 50.0;
        note.expressions.volume = 80.0;
        note.expressions.attack = 110.0;
        note.expressions.decay = 25.0;
        note.expressions.consonant_timing_offset_ms = 15.0;
        note.expressions.preutter_offset_ms = -10.0;
        note.expressions.overlap_offset_ms = 8.0;
        note.flags = "g-5BRE50".to_string();
        note.phoneme_durations_ms = vec![120.0, 360.0];
        note.vibrato.depth_cents = 60.0;
        note.envelope.p1 = 20.0;
        note.pitch_bend.points.push(UPitchBendPoint {
            time_offset_ms: 50.0,
            pitch_offset_cents: 20.0,
            shape: "s".to_string(),
        });

        note.reset_all_parameters();

        assert_eq!(note.lyric, "ka");
        assert_eq!(note.pitch, "A4");
        assert_eq!(note.position_ms, 480.0);
        assert_eq!(note.duration_ms, 480.0);
        assert_eq!(note.expressions.dynamics, 0.0);
        assert_eq!(note.expressions.gender, 0.0);
        assert_eq!(note.expressions.breathiness, 0.0);
        assert_eq!(note.expressions.velocity, 100.0);
        assert_eq!(note.expressions.consonant_velocity, 100.0);
        assert_eq!(note.expressions.modulation, 0.0);
        assert_eq!(note.expressions.volume, 100.0);
        assert_eq!(note.expressions.attack, 100.0);
        assert_eq!(note.expressions.decay, 0.0);
        assert_eq!(note.expressions.consonant_timing_offset_ms, 0.0);
        assert_eq!(note.expressions.preutter_offset_ms, 0.0);
        assert_eq!(note.expressions.overlap_offset_ms, 0.0);
        assert!(note.flags.is_empty());
        assert!(note.phoneme_durations_ms.is_empty());
        assert_eq!(note.vibrato.length_pct, 0.0);
        assert_eq!(note.envelope.p1, 0.0);
        assert_eq!(note.envelope.p2, 5.0);
        assert!(note.pitch_bend.points.is_empty());
    }

    #[test]
    fn test_portamento_and_envelope_isolation() {
        let mut note = UNote::new("ka", "C4", 0.0, 480.0);
        note.envelope.p1 = 15.0;
        note.envelope.p2 = 45.0;
        note.envelope.p3 = 30.0;
        note.envelope.p4 = 10.0;
        note.envelope.p5 = 55.0;
        note.envelope.v1 = 20.0;
        note.envelope.v2 = 90.0;
        note.envelope.crossfade_ms = 25.0;

        let env_snapshot = note.envelope.clone();

        // Mutating portamento / pitch bend must leave the envelope 100% untouched
        note.pitch_bend.portamento_start_ms = -55.0;
        note.pitch_bend.portamento_length_ms = 110.0;
        note.pitch_bend.portamento_shape = "io".to_string();
        note.pitch_bend.ensure_portamento(Some(58), 60, true);

        assert_eq!(note.envelope, env_snapshot);
        assert_eq!(note.pitch_bend.points.len(), 2);
    }

    #[test]
    fn imported_phoneme_policy_defaults_to_preserve_and_recalculate_is_explicit() {
        let mut project = UProject::default();
        project.parts.push(UVoicePart::new("Imported", 0));
        project.parts[0]
            .notes
            .push(UNote::new("ka", "C4", 0.0, 400.0));
        let note = &mut project.parts[0].notes[0];
        note.phonetic_hint = Some("a k".to_string());
        note.phoneme_durations_ms = vec![80.0, 320.0];
        note.phoneme_overrides.push(UPhonemeOverride {
            index: 0,
            phoneme: Some("a".to_string()),
            ..UPhonemeOverride::default()
        });
        assert_eq!(
            project.imported_phoneme_policy,
            ImportedPhonemePolicy::Preserve
        );

        project.recalculate_imported_phonemes();
        let note = &project.parts[0].notes[0];
        assert!(note.phonetic_hint.is_none());
        assert!(note.phoneme_durations_ms.is_empty());
        assert!(note.phoneme_overrides.is_empty());
    }
}
