use crate::dsp::pitch::midi_to_note_name;
use crate::project::model::{UNote, UProject, UTrack, UVoicePart};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const DEFAULT_TICKS_PER_BEAT: f64 = 480.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UFData {
    pub format_version: i32,
    pub project: UFProject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UFProject {
    pub name: String,
    pub tracks: Vec<UFTrack>,
    #[serde(default)]
    pub time_signatures: Vec<UFTimeSignature>,
    #[serde(default)]
    pub tempos: Vec<UFTempo>,
    #[serde(default)]
    pub measure_prefix: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UFTimeSignature {
    pub measure_position: i64,
    pub numerator: i32,
    pub denominator: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UFTempo {
    pub tick_position: i64,
    pub bpm: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UFTrack {
    pub name: String,
    pub notes: Vec<UFNote>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pitch: Option<UFPitch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UFNote {
    pub key: u8,
    pub tick_on: i64,
    pub tick_off: i64,
    pub lyric: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub phoneme: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UFPitch {
    pub ticks: Vec<i64>,
    pub values: Vec<f64>,
    pub is_absolute: bool,
}

pub struct UfdataFormat;

impl UfdataFormat {
    pub fn load_file<P: AsRef<Path>>(path: P) -> Result<UProject, Box<dyn std::error::Error>> {
        let bytes = fs::read(path)?;
        let content = String::from_utf8_lossy(&bytes);
        Self::parse_str(&content)
    }

    pub fn parse_str(content: &str) -> Result<UProject, Box<dyn std::error::Error>> {
        let clean = content.trim_start_matches('\u{feff}').trim();
        let ufdata: UFData = serde_json::from_str(clean)?;
        Ok(Self::to_uproject(&ufdata))
    }

    pub fn save_file<P: AsRef<Path>>(
        project: &UProject,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let ufdata = Self::from_uproject(project);
        let json_str = serde_json::to_string_pretty(&ufdata)?;
        fs::write(path, json_str)?;
        Ok(())
    }

    pub fn from_uproject(project: &UProject) -> UFData {
        let bpm = if project.bpm.is_finite() && project.bpm > 0.0 {
            project.bpm
        } else {
            120.0
        };
        let ms_per_beat = 60000.0 / bpm;
        let ms_per_tick = ms_per_beat / DEFAULT_TICKS_PER_BEAT;

        let mut uf_tracks = Vec::new();

        for (track_idx, track) in project.tracks.iter().enumerate() {
            let mut uf_notes = Vec::new();

            for part in project.parts.iter().filter(|p| p.track_index == track_idx) {
                for note in &part.notes {
                    let note_start_ms = part.position_ms + note.position_ms;
                    let note_end_ms = note_start_ms + note.duration_ms.max(1.0);

                    let tick_on = (note_start_ms / ms_per_tick).round() as i64;
                    let tick_off = (note_end_ms / ms_per_tick)
                        .round()
                        .max((tick_on + 1) as f64) as i64;
                    let key = note.midi_key();

                    uf_notes.push(UFNote {
                        key,
                        tick_on,
                        tick_off,
                        lyric: if note.lyric.is_empty() {
                            "a".to_string()
                        } else {
                            note.lyric.clone()
                        },
                        phoneme: None,
                    });
                }
            }

            uf_notes.sort_by_key(|n| n.tick_on);

            uf_tracks.push(UFTrack {
                name: if track.name.is_empty() {
                    format!("Track {}", track_idx + 1)
                } else {
                    track.name.clone()
                },
                notes: uf_notes,
                pitch: None,
            });
        }

        if uf_tracks.is_empty() {
            uf_tracks.push(UFTrack {
                name: "Track 1".to_string(),
                notes: Vec::new(),
                pitch: None,
            });
        }

        UFData {
            format_version: 1,
            project: UFProject {
                name: if project.name.is_empty() {
                    "Kamafeu Project".to_string()
                } else {
                    project.name.clone()
                },
                tracks: uf_tracks,
                time_signatures: vec![UFTimeSignature {
                    measure_position: 0,
                    numerator: 4,
                    denominator: 4,
                }],
                tempos: vec![UFTempo {
                    tick_position: 0,
                    bpm,
                }],
                measure_prefix: 0,
            },
        }
    }

    pub fn to_uproject(ufdata: &UFData) -> UProject {
        let bpm = ufdata
            .project
            .tempos
            .first()
            .map(|t| t.bpm)
            .unwrap_or(120.0)
            .clamp(20.0, 999.0);

        let ms_per_beat = 60000.0 / bpm;
        let ms_per_tick = ms_per_beat / DEFAULT_TICKS_PER_BEAT;

        let mut tracks = Vec::new();
        let mut parts = Vec::new();

        for (track_idx, uf_track) in ufdata.project.tracks.iter().enumerate() {
            tracks.push(UTrack {
                name: uf_track.name.clone(),
                singer: "Default Singer".to_string(),
                voicebank_path: None,
                phonemizer: None,
                volume_db: 0.0,
                pan: 0.0,
                mute: false,
                solo: false,
                ..UTrack::default()
            });

            let mut notes = Vec::new();
            for uf_note in &uf_track.notes {
                let position_ms = (uf_note.tick_on as f64 * ms_per_tick).max(0.0);
                let duration_ms =
                    ((uf_note.tick_off - uf_note.tick_on) as f64 * ms_per_tick).max(10.0);
                let pitch_name = midi_to_note_name(uf_note.key);

                let lyric = if uf_note.lyric.trim().is_empty() {
                    "a".to_string()
                } else {
                    uf_note.lyric.clone()
                };

                let note = UNote::new(lyric, pitch_name, position_ms, duration_ms);
                notes.push(note);
            }

            parts.push(UVoicePart {
                name: format!("Part {}", track_idx + 1),
                track_index: track_idx,
                position_ms: 0.0,
                notes,
            });
        }

        let mut project = UProject {
            name: ufdata.project.name.clone(),
            bpm,
            voicebank: None,
            voicebank_path: None,
            phonemizer: None,
            tracks: if tracks.is_empty() {
                vec![UTrack::default()]
            } else {
                tracks
            },
            parts: if parts.is_empty() {
                vec![UVoicePart::new("Part 1", 0)]
            } else {
                parts
            },
            ..UProject::default()
        };

        project.normalize();
        project
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ufdata_roundtrip() {
        let mut proj = UProject::default();
        proj.name = "Test Utaformatix".to_string();
        proj.bpm = 130.0;
        let note1 = UNote::new("ka", "C4", 0.0, 480.0);
        let note2 = UNote::new("ma", "D4", 480.0, 480.0);
        proj.parts[0].notes.push(note1);
        proj.parts[0].notes.push(note2);

        let uf = UfdataFormat::from_uproject(&proj);
        assert_eq!(uf.project.tracks[0].notes.len(), 2);
        assert_eq!(uf.project.tracks[0].notes[0].lyric, "ka");
        assert_eq!(uf.project.tracks[0].notes[1].lyric, "ma");

        let converted_back = UfdataFormat::to_uproject(&uf);
        assert_eq!(converted_back.bpm, 130.0);
        assert_eq!(converted_back.parts[0].notes.len(), 2);
        assert_eq!(converted_back.parts[0].notes[0].pitch, "C4");
        assert_eq!(converted_back.parts[0].notes[1].pitch, "D4");
    }
}
