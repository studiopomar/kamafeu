use crate::dsp::pitch::midi_to_note_name;
use crate::project::model::{UNote, UProject, UTrack, UVoicePart};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

pub const SV_BLICKS_PER_BEAT: f64 = 705_600_000.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvpProject {
    #[serde(default)]
    pub version: Option<i32>,
    #[serde(default)]
    pub time: Option<SvpTime>,
    #[serde(default)]
    pub tracks: Vec<SvpTrack>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvpTime {
    #[serde(default)]
    pub tempo: Vec<SvpTempo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvpTempo {
    #[serde(default)]
    pub position: i64,
    #[serde(default = "default_bpm")]
    pub bpm: f64,
}

fn default_bpm() -> f64 {
    120.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvpTrack {
    #[serde(default)]
    pub name: String,
    #[serde(rename = "mainGroup", default)]
    pub main_group: Option<SvpGroup>,
    #[serde(default)]
    pub groups: Vec<SvpGroupRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvpGroupRef {
    #[serde(default)]
    pub group: Option<SvpGroup>,
    #[serde(default)]
    pub offset: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvpGroup {
    #[serde(default)]
    pub notes: Vec<SvpNote>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SvpNote {
    #[serde(default)]
    pub onset: i64,
    #[serde(default)]
    pub duration: i64,
    #[serde(default = "default_pitch")]
    pub pitch: u8,
    #[serde(default)]
    pub lyrics: String,
    #[serde(default)]
    pub phonemes: String,
}

fn default_pitch() -> u8 {
    60
}

pub struct SvpFormat;

impl SvpFormat {
    pub fn load_file<P: AsRef<Path>>(path: P) -> Result<UProject, Box<dyn std::error::Error>> {
        let bytes = fs::read(path)?;
        let content = String::from_utf8_lossy(&bytes);
        Self::parse_str(&content)
    }

    pub fn parse_str(content: &str) -> Result<UProject, Box<dyn std::error::Error>> {
        let clean = content.trim_start_matches('\u{feff}').trim();
        let svp: SvpProject = serde_json::from_str(clean)?;
        Ok(Self::to_uproject(&svp))
    }

    pub fn to_uproject(svp: &SvpProject) -> UProject {
        let bpm = svp
            .time
            .as_ref()
            .and_then(|t| t.tempo.first())
            .map(|t| t.bpm)
            .unwrap_or(120.0)
            .clamp(20.0, 999.0);

        let ms_per_beat = 60000.0 / bpm;
        let ms_per_blick = ms_per_beat / SV_BLICKS_PER_BEAT;

        let mut tracks = Vec::new();
        let mut parts = Vec::new();

        for (track_idx, sv_track) in svp.tracks.iter().enumerate() {
            let track_name = if sv_track.name.is_empty() {
                format!("Track {}", track_idx + 1)
            } else {
                sv_track.name.clone()
            };

            tracks.push(UTrack {
                name: track_name.clone(),
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

            if let Some(main_group) = &sv_track.main_group {
                for sv_note in &main_group.notes {
                    let pos_ms = (sv_note.onset as f64 * ms_per_blick).max(0.0);
                    let dur_ms = (sv_note.duration as f64 * ms_per_blick).max(10.0);
                    let pitch_name = midi_to_note_name(sv_note.pitch);
                    let lyric = if sv_note.lyrics.trim().is_empty() {
                        "la".to_string()
                    } else {
                        sv_note.lyrics.clone()
                    };
                    notes.push(UNote::new(lyric, pitch_name, pos_ms, dur_ms));
                }
            }

            for group_ref in &sv_track.groups {
                if let Some(group) = &group_ref.group {
                    for sv_note in &group.notes {
                        let total_onset = sv_note.onset + group_ref.offset;
                        let pos_ms = (total_onset as f64 * ms_per_blick).max(0.0);
                        let dur_ms = (sv_note.duration as f64 * ms_per_blick).max(10.0);
                        let pitch_name = midi_to_note_name(sv_note.pitch);
                        let lyric = if sv_note.lyrics.trim().is_empty() {
                            "la".to_string()
                        } else {
                            sv_note.lyrics.clone()
                        };
                        notes.push(UNote::new(lyric, pitch_name, pos_ms, dur_ms));
                    }
                }
            }

            notes.sort_by(|a, b| {
                a.position_ms
                    .partial_cmp(&b.position_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            parts.push(UVoicePart {
                name: format!("Part {}", track_idx + 1),
                track_index: track_idx,
                position_ms: 0.0,
                notes,
            });
        }

        let mut project = UProject {
            name: "Synthesizer V Project".to_string(),
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

    pub fn save_file<P: AsRef<Path>>(
        project: &UProject,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let bpm = if project.bpm.is_finite() && project.bpm > 0.0 {
            project.bpm
        } else {
            120.0
        };
        let ms_per_beat = 60000.0 / bpm;
        let ms_per_blick = ms_per_beat / SV_BLICKS_PER_BEAT;

        let mut svp_tracks = Vec::new();

        for (track_idx, track) in project.tracks.iter().enumerate() {
            let mut svp_notes = Vec::new();
            for part in project.parts.iter().filter(|p| p.track_index == track_idx) {
                for note in &part.notes {
                    let note_start_ms = part.position_ms + note.position_ms;
                    let onset = (note_start_ms / ms_per_blick).round() as i64;
                    let duration =
                        (note.duration_ms.max(1.0) / ms_per_blick).round().max(1.0) as i64;
                    let pitch = note.midi_key();
                    let lyrics = if note.lyric.trim().is_empty() {
                        "la".to_string()
                    } else {
                        note.lyric.clone()
                    };
                    svp_notes.push(SvpNote {
                        onset,
                        duration,
                        pitch,
                        lyrics,
                        phonemes: String::new(),
                    });
                }
            }
            svp_tracks.push(SvpTrack {
                name: track.name.clone(),
                main_group: Some(SvpGroup { notes: svp_notes }),
                groups: Vec::new(),
            });
        }

        let svp = SvpProject {
            version: Some(1),
            time: Some(SvpTime {
                tempo: vec![SvpTempo { position: 0, bpm }],
            }),
            tracks: svp_tracks,
        };

        let json_str = serde_json::to_string_pretty(&svp)?;
        fs::write(path, json_str)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svp_roundtrip() {
        let mut proj = UProject::default();
        proj.name = "Test Svp".to_string();
        proj.bpm = 128.0;
        let note1 = UNote::new("ka", "C4", 0.0, 500.0);
        let note2 = UNote::new("ma", "D4", 500.0, 500.0);
        proj.parts[0].notes.push(note1);
        proj.parts[0].notes.push(note2);

        let temp = tempfile::NamedTempFile::new().unwrap();
        SvpFormat::save_file(&proj, temp.path()).unwrap();

        let loaded = SvpFormat::load_file(temp.path()).unwrap();
        assert_eq!(loaded.bpm, 128.0);
        assert_eq!(loaded.parts[0].notes.len(), 2);
        assert_eq!(loaded.parts[0].notes[0].lyric, "ka");
        assert_eq!(loaded.parts[0].notes[0].pitch, "C4");
        assert_eq!(loaded.parts[0].notes[1].lyric, "ma");
        assert_eq!(loaded.parts[0].notes[1].pitch, "D4");
    }
}
