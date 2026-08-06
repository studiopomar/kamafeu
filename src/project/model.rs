use crate::dsp::envelope::UtauEnvelope;
use crate::dsp::pitch::{midi_to_note_name, note_name_to_midi, VibratoParam};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UPitchBendPoint {
    pub time_offset_ms: f64,
    pub pitch_offset_cents: f64,
    pub shape: String, // "", "s" (linear), "j" (exp), "r" (log)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        let mut points = if self.points.is_empty() {
            automatic_portamento()
        } else {
            self.points.clone()
        };
        points.sort_by(|left, right| {
            left.time_offset_ms
                .partial_cmp(&right.time_offset_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // PitchDraw stores only points touched by the user. Preserve the
        // automatic portamento when the first hand-drawn point occurs later;
        // otherwise the previous note would be held until that point.
        if points
            .first()
            .is_some_and(|first| first.time_offset_ms > start + 1e-6)
        {
            points.extend(automatic_portamento());
            points.sort_by(|left, right| {
                left.time_offset_ms
                    .partial_cmp(&right.time_offset_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
        }
        if self.snap_first && adjacent {
            if let (Some(previous), Some(first)) = (previous_midi, points.first_mut()) {
                first.pitch_offset_cents = (f64::from(previous) - f64::from(current_midi)) * 100.0;
            }
        }
        points
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UExpressions {
    pub dynamics: f64,    // DYN (-240 to +120, in 0.1 dB units)
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
            pitch_delta: 0.0,
            gender: 0.0,
            velocity: 100.0,
            breathiness: 0.0,
            consonant_velocity: 100.0,
            modulation: 0.0,
            volume: 100.0,
            attack: 100.0,
            decay: 0.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UNote {
    pub lyric: String,
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
}

impl UNote {
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
            pitch: pitch_str,
            position_ms,
            duration_ms,
            envelope: UtauEnvelope::default(),
            vibrato: VibratoParam::default(),
            pitch_bend: UPitchBend::default(),
            expressions: UExpressions::default(),
            flags: String::new(),
        }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UTrack {
    pub name: String,
    pub singer: String,
    pub volume_db: f64,
    pub pan: f64,
    pub mute: bool,
    pub solo: bool,
}

impl Default for UTrack {
    fn default() -> Self {
        Self {
            name: "Track 1".to_string(),
            singer: "Default Singer".to_string(),
            volume_db: 0.0,
            pan: 0.0,
            mute: false,
            solo: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UProject {
    pub name: String,
    pub bpm: f64,
    pub tracks: Vec<UTrack>,
    pub parts: Vec<UVoicePart>,
}

impl Default for UProject {
    fn default() -> Self {
        Self {
            name: "Novo Projeto".to_string(),
            bpm: 120.0,
            tracks: vec![UTrack::default()],
            parts: vec![UVoicePart::new("Parte Vocal 1", 0)],
        }
    }
}

impl UProject {
    /// Change the project tempo while keeping every event on the same musical beat.
    ///
    /// Timeline values are stored in milliseconds, so changing only `bpm` would
    /// leave playback speed unchanged. Rescaling positions and durations by the
    /// inverse tempo ratio preserves their beat positions and makes the tempo
    /// change audible.
    pub fn set_bpm_preserving_beats(&mut self, new_bpm: f64) -> Option<f64> {
        if !new_bpm.is_finite() || new_bpm <= 0.0 {
            return None;
        }

        let old_bpm = if self.bpm.is_finite() && self.bpm > 0.0 {
            self.bpm
        } else {
            120.0
        };
        let new_bpm = new_bpm.clamp(20.0, 999.0);
        let time_scale = old_bpm / new_bpm;

        if (old_bpm - new_bpm).abs() <= f64::EPSILON {
            self.bpm = new_bpm;
            return Some(1.0);
        }

        for part in &mut self.parts {
            part.position_ms *= time_scale;
            for note in &mut part.notes {
                note.position_ms *= time_scale;
                note.duration_ms *= time_scale;
                for point in &mut note.pitch_bend.points {
                    point.time_offset_ms *= time_scale;
                }
            }
        }

        self.bpm = new_bpm;
        Some(time_scale)
    }

    /// Restore invariants after loading permissive third-party project files.
    pub fn normalize(&mut self) {
        if !self.bpm.is_finite() || self.bpm <= 0.0 {
            self.bpm = 120.0;
        }
        self.bpm = self.bpm.clamp(20.0, 999.0);

        let required_tracks = self
            .parts
            .iter()
            .map(|part| part.track_index.saturating_add(1))
            .max()
            .unwrap_or(1)
            .max(1);
        while self.tracks.len() < required_tracks {
            self.tracks.push(UTrack {
                name: format!("Track {}", self.tracks.len() + 1),
                ..UTrack::default()
            });
        }
        if self.parts.is_empty() {
            self.parts.push(UVoicePart::new("Parte Vocal 1", 0));
        }

        for track in &mut self.tracks {
            if !track.volume_db.is_finite() {
                track.volume_db = 0.0;
            }
            if !track.pan.is_finite() {
                track.pan = 0.0;
            }
            track.volume_db = track.volume_db.clamp(-60.0, 12.0);
            track.pan = track.pan.clamp(-1.0, 1.0);
        }

        for part in &mut self.parts {
            if !part.position_ms.is_finite() {
                part.position_ms = 0.0;
            }
            part.position_ms = part.position_ms.max(0.0);
            for note in &mut part.notes {
                if !note.position_ms.is_finite() {
                    note.position_ms = 0.0;
                }
                if !note.duration_ms.is_finite() {
                    note.duration_ms = 1.0;
                }
                note.position_ms = note.position_ms.max(0.0);
                note.duration_ms = note.duration_ms.max(1.0);
                if !note.expressions.consonant_velocity.is_finite() {
                    note.expressions.consonant_velocity = 100.0;
                }
                note.expressions.consonant_velocity =
                    note.expressions.consonant_velocity.clamp(0.0, 200.0);
                if !note.expressions.velocity.is_finite() {
                    note.expressions.velocity = 100.0;
                }
                if !note.expressions.dynamics.is_finite() {
                    note.expressions.dynamics = 0.0;
                }
                if !note.expressions.pitch_delta.is_finite() {
                    note.expressions.pitch_delta = 0.0;
                }
                if !note.expressions.gender.is_finite() {
                    note.expressions.gender = 0.0;
                }
                if !note.expressions.breathiness.is_finite() {
                    note.expressions.breathiness = 0.0;
                }
                if !note.expressions.modulation.is_finite() {
                    note.expressions.modulation = 0.0;
                }
                if !note.expressions.volume.is_finite() {
                    note.expressions.volume = 100.0;
                }
                if !note.expressions.attack.is_finite() {
                    note.expressions.attack = 100.0;
                }
                if !note.expressions.decay.is_finite() {
                    note.expressions.decay = 0.0;
                }
                note.expressions.velocity = note.expressions.velocity.clamp(0.0, 200.0);
                note.expressions.dynamics = note.expressions.dynamics.clamp(-240.0, 120.0);
                note.expressions.pitch_delta = note.expressions.pitch_delta.clamp(-1200.0, 1200.0);
                note.expressions.gender = note.expressions.gender.clamp(-100.0, 100.0);
                note.expressions.breathiness = note.expressions.breathiness.clamp(0.0, 100.0);
                note.expressions.modulation = note.expressions.modulation.clamp(0.0, 100.0);
                note.expressions.volume = note.expressions.volume.clamp(0.0, 200.0);
                note.expressions.attack = note.expressions.attack.clamp(0.0, 200.0);
                note.expressions.decay = note.expressions.decay.clamp(0.0, 100.0);
                note.vibrato.normalize();
                if !note.pitch_bend.portamento_start_ms.is_finite() {
                    note.pitch_bend.portamento_start_ms = -40.0;
                }
                if !note.pitch_bend.portamento_length_ms.is_finite() {
                    note.pitch_bend.portamento_length_ms = 80.0;
                }
                note.pitch_bend.portamento_start_ms =
                    note.pitch_bend.portamento_start_ms.clamp(-2000.0, 2000.0);
                note.pitch_bend.portamento_length_ms =
                    note.pitch_bend.portamento_length_ms.clamp(1.0, 2000.0);
                if note.pitch_bend.portamento_shape.is_empty() {
                    note.pitch_bend.portamento_shape = "io".to_string();
                }
                note.pitch_bend.points.retain(|point| {
                    point.time_offset_ms.is_finite() && point.pitch_offset_cents.is_finite()
                });
                note.pitch_bend.points.sort_by(|left, right| {
                    left.time_offset_ms
                        .partial_cmp(&right.time_offset_ms)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }
    }
}

pub fn create_astro_boy_1980_project() -> UProject {
    let mut project = UProject {
        name: "Astro Boy (1980) - Tetsuwan Atom Theme".to_string(),
        bpm: 140.0,
        tracks: vec![UTrack::default()],
        parts: vec![UVoicePart::new("Astro Boy Opening Track", 0)],
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
        };
        project.normalize();

        assert_eq!(project.bpm, 120.0);
        assert_eq!(project.tracks.len(), 3);
        assert_eq!(project.parts[0].position_ms, 0.0);
        assert_eq!(project.parts[0].notes[0].duration_ms, 1.0);
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
}
