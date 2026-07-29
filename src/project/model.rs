use serde::{Deserialize, Serialize};
use crate::dsp::envelope::UtauEnvelope;
use crate::dsp::pitch::{midi_to_note_name, note_name_to_midi, VibratoParam};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UPitchBendPoint {
    pub time_offset_ms: f64,
    pub pitch_offset_cents: f64,
    pub shape: String, // "", "s" (linear), "j" (exp), "r" (log)
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UPitchBend {
    pub points: Vec<UPitchBendPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UExpressions {
    pub dynamics: f64,    // DYN (-100 to +100, default 0)
    pub pitch_delta: f64, // PITD (-1200 to +1200 cents, default 0)
    pub gender: f64,      // GEN (-100 to +100, default 0)
    #[serde(default = "default_velocity")]
    pub velocity: f64,    // VEL (0 to 200, default 100)
    pub breathiness: f64, // BRE (-100 to +100, default 0)
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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UNote {
    pub lyric: String,
    pub pitch: String,
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
}

impl UNote {
    pub fn new(lyric: impl Into<String>, pitch: impl Into<String>, position_ms: f64, duration_ms: f64) -> Self {
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
            name: "New Project".to_string(),
            bpm: 120.0,
            tracks: vec![UTrack::default()],
            parts: vec![UVoicePart::new("Voice Part 1", 0)],
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
                UPitchBendPoint { time_offset_ms: -40.0, pitch_offset_cents: 0.0, shape: "".to_string() },
                UPitchBendPoint { time_offset_ms: dur_ms * 0.35, pitch_offset_cents: 35.0, shape: "s".to_string() },
                UPitchBendPoint { time_offset_ms: dur_ms * 0.7, pitch_offset_cents: -25.0, shape: "s".to_string() },
                UPitchBendPoint { time_offset_ms: dur_ms, pitch_offset_cents: 0.0, shape: "s".to_string() },
            ];
        }

        notes.push(note);
    }

    project.parts[0].notes = notes;
    project
}
