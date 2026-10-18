use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UPitchBendPoint {
    pub time_offset_ms: f64,
    pub pitch_offset_cents: f64,
    pub shape: String, // "", "s" (linear), "j" (exp), "r" (log)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UExpressionPoint {
    pub time_offset_ms: f64,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UPitchBend {
    pub points: Vec<UPitchBendPoint>,
    pub snap_first: bool,
    pub portamento_start_ms: f64,
    pub portamento_length_ms: f64,
    pub portamento_shape: String,
}

impl Default for UPitchBend {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            snap_first: true,
            portamento_start_ms: -40.0,
            portamento_length_ms: 80.0,
            portamento_shape: "io".to_string(),
        }
    }
}

impl UPitchBend {
    /// Materializes the default portamento so newly created notes and adjacent
    /// notes behave identically in the editor, renderer and exported formats.
    pub fn ensure_portamento(
        &mut self,
        previous_midi: Option<u8>,
        current_midi: u8,
        adjacent: bool,
    ) {
        if self.points.is_empty() {
            let start = self.portamento_start_ms.clamp(-2000.0, 2000.0);
            let length = self.portamento_length_ms.clamp(1.0, 2000.0);
            let first_offset = if self.snap_first && adjacent {
                previous_midi
                    .map(|previous| (f64::from(previous) - f64::from(current_midi)) * 100.0)
                    .unwrap_or(0.0)
            } else {
                0.0
            };
            self.points = vec![
                UPitchBendPoint {
                    time_offset_ms: start,
                    pitch_offset_cents: first_offset,
                    shape: self.portamento_shape.clone(),
                },
                UPitchBendPoint {
                    time_offset_ms: start + length,
                    pitch_offset_cents: 0.0,
                    shape: self.portamento_shape.clone(),
                },
            ];
        }
    }

    pub fn effective_points(
        &self,
        previous_midi: Option<u8>,
        current_midi: u8,
        adjacent: bool,
    ) -> Vec<UPitchBendPoint> {
        let start = self.portamento_start_ms.clamp(-2000.0, 2000.0);
        let length = self.portamento_length_ms.clamp(1.0, 2000.0);
        let automatic_portamento = || {
            vec![
                UPitchBendPoint {
                    time_offset_ms: start,
                    pitch_offset_cents: 0.0,
                    shape: self.portamento_shape.clone(),
                },
                UPitchBendPoint {
                    time_offset_ms: start + length,
                    pitch_offset_cents: 0.0,
                    shape: self.portamento_shape.clone(),
                },
            ]
        };

        if self.points.is_empty() {
            let mut points = automatic_portamento();
            if self.snap_first && adjacent {
                if let (Some(previous), Some(first)) = (previous_midi, points.first_mut()) {
                    first.pitch_offset_cents =
                        (f64::from(previous) - f64::from(current_midi)) * 100.0;
                }
            }
            points
        } else {
            let mut points = self.points.clone();
            points.sort_by(|left, right| {
                left.time_offset_ms
                    .partial_cmp(&right.time_offset_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            if self.snap_first
                && adjacent
                && points
                    .first()
                    .is_some_and(|first| first.time_offset_ms > 0.0)
            {
                let mut auto = automatic_portamento();
                if let (Some(previous), Some(first)) = (previous_midi, auto.first_mut()) {
                    first.pitch_offset_cents =
                        (f64::from(previous) - f64::from(current_midi)) * 100.0;
                }
                points.extend(auto);
                points.sort_by(|left, right| {
                    left.time_offset_ms
                        .partial_cmp(&right.time_offset_ms)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            } else if self.snap_first && adjacent {
                if let (Some(previous), Some(first)) = (previous_midi, points.first_mut()) {
                    first.pitch_offset_cents =
                        (f64::from(previous) - f64::from(current_midi)) * 100.0;
                }
            }
            points
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UExpressions {
    pub dynamics: f64, // DYN (-240 to +120, in 0.1 dB units)
    #[serde(default)]
    pub dynamics_curve: Vec<UExpressionPoint>,
    pub pitch_delta: f64, // PITD (-1200 to +1200 cents, default 0)
    pub gender: f64,      // GEN (-100 to +100, default 0)
    #[serde(default = "default_velocity")]
    pub velocity: f64, // VEL (0 to 200, default 100)
    pub breathiness: f64, // BRE (-100 to +100, default 0)
    #[serde(default = "default_consonant_velocity")]
    pub consonant_velocity: f64, // (0 to 200, default 100)
    #[serde(default = "default_modulation")]
    pub modulation: f64, // MOD (0 to 200, default 0)
    #[serde(default = "default_expression_percent")]
    pub volume: f64, // VOL (0 to 200, default 100)
    #[serde(default = "default_expression_percent")]
    pub attack: f64, // ATK (0 to 200, default 100)
    #[serde(default)]
    pub decay: f64, // DEC (0 to 100, default 0)
    #[serde(default)]
    pub consonant_timing_offset_ms: f64, // Consoante/Vogal timing shift (-500ms to +500ms)
    #[serde(default)]
    pub preutter_offset_ms: f64, // Pré-emissão / Início de transição timing shift (-500ms to +500ms)
    #[serde(default)]
    pub overlap_offset_ms: f64, // Overlap / Crossfade timing shift (-500ms to +500ms)
    /// OpenUtau per-phoneme attack-handle delta. This is timing, not ATK.
    #[serde(default)]
    pub attack_time_delta_ms: f64,
    /// OpenUtau per-phoneme release-handle delta. This is timing, not DEC.
    #[serde(default)]
    pub release_time_delta_ms: f64,
    /// Presamp marks this consonant as incompatible with VC/CV crossfading.
    /// It is an internal phonemizer decision, not an UTAU resampler flag.
    #[serde(default)]
    pub presamp_no_crossfade: bool,
    /// OpenUtau's per-phoneme NORM expression. `None` preserves the
    /// renderer's native behavior; Presamp may set it for generated pre-C
    /// consonants through `[CFLAGS]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presamp_normalization: Option<f64>,
    /// Absolute legacy UST values. Kept separate from editable deltas so an
    /// imported project reproduces its original timing against any oto.ini entry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preutter_override_ms: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overlap_override_ms: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_point_ms: Option<f64>,
}

fn default_expression_percent() -> f64 {
    100.0
}

fn default_modulation() -> f64 {
    0.0
}
fn default_consonant_velocity() -> f64 {
    100.0
}

fn default_velocity() -> f64 {
    100.0
}

impl Default for UExpressions {
    fn default() -> Self {
        Self {
            dynamics: 0.0,
            dynamics_curve: Vec::new(),
            pitch_delta: 0.0,
            gender: 0.0,
            velocity: 100.0,
            breathiness: 0.0,
            consonant_velocity: 100.0,
            modulation: 0.0,
            volume: 100.0,
            attack: 100.0,
            decay: 0.0,
            consonant_timing_offset_ms: 0.0,
            preutter_offset_ms: 0.0,
            overlap_offset_ms: 0.0,
            attack_time_delta_ms: 0.0,
            release_time_delta_ms: 0.0,
            presamp_no_crossfade: false,
            presamp_normalization: None,
            preutter_override_ms: None,
            overlap_override_ms: None,
            start_point_ms: None,
        }
    }
}
