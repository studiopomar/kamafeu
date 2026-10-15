use crate::dsp::pitch::midi_to_note_name;
use crate::project::model::{UNote, UProject, UTrack, UVoicePart};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
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
    pub values: Vec<Option<f64>>,
    pub is_absolute: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UfdataConversionReport {
    pub warnings: Vec<String>,
    pub dropped_features: Vec<String>,
    pub exported_pitch: bool,
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
        if let Ok(ufdata) = serde_json::from_str::<UFData>(clean) {
            return Ok(Self::to_uproject(&ufdata));
        }
        if let Ok(project) = serde_json::from_str::<UFProject>(clean) {
            let ufdata = UFData {
                format_version: 1,
                project,
            };
            return Ok(Self::to_uproject(&ufdata));
        }
        let ufdata: UFData = serde_json::from_str(clean)?;
        Ok(Self::to_uproject(&ufdata))
    }

    pub fn save_file<P: AsRef<Path>>(
        project: &UProject,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        Self::save_file_with_report(project, path).map(|_| ())
    }

    pub fn save_file_with_report<P: AsRef<Path>>(
        project: &UProject,
        path: P,
    ) -> Result<UfdataConversionReport, Box<dyn std::error::Error>> {
        let ufdata = Self::from_uproject(project);
        let json_str = serde_json::to_string_pretty(&ufdata)?;
        fs::write(path, json_str)?;
        Ok(Self::conversion_report(project, &ufdata))
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
            let mut pitch_by_tick = BTreeMap::<i64, f64>::new();

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

                    // UFData stores pitch at track level as absolute semitones.
                    // Keep authored points only; automatic portamento remains a
                    // native APS concern and is not silently invented in the
                    // interchange file.
                    for point in &note.pitch_bend.points {
                        let absolute_ms =
                            part.position_ms + note.position_ms + point.time_offset_ms;
                        let tick = (absolute_ms / ms_per_tick).round() as i64;
                        let value = note.midi_key() as f64
                            + (point.pitch_offset_cents + note.expressions.pitch_delta) / 100.0;
                        if value.is_finite() {
                            pitch_by_tick.insert(tick, value);
                        }
                    }
                }
            }

            uf_notes.sort_by_key(|n| n.tick_on);

            let pitch = if pitch_by_tick.is_empty() {
                None
            } else {
                Some(UFPitch {
                    ticks: pitch_by_tick.keys().copied().collect(),
                    values: pitch_by_tick.values().copied().map(Some).collect(),
                    is_absolute: true,
                })
            };

            uf_tracks.push(UFTrack {
                name: if track.name.is_empty() {
                    format!("Track {}", track_idx + 1)
                } else {
                    track.name.clone()
                },
                notes: uf_notes,
                pitch,
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

    pub fn conversion_report(project: &UProject, ufdata: &UFData) -> UfdataConversionReport {
        let exported_pitch = ufdata.project.tracks.iter().any(|track| {
            track
                .pitch
                .as_ref()
                .is_some_and(|pitch| !pitch.ticks.is_empty())
        });
        let mut report = UfdataConversionReport {
            exported_pitch,
            ..Default::default()
        };

        if project.tracks.iter().any(|track| track.fx_rack.is_some()) {
            report
                .dropped_features
                .push("FX rack por track".to_string());
        }
        if !project.wave_parts.is_empty() {
            report.dropped_features.push("partes de áudio".to_string());
        }
        if !project.markers.is_empty() {
            report.dropped_features.push("marcadores".to_string());
        }
        if !project.sections.is_empty() {
            report.dropped_features.push("seções".to_string());
        }
        if !project.extensions.is_empty() {
            report.dropped_features.push("extensões".to_string());
        }
        if project.render_state.is_some() {
            report
                .dropped_features
                .push("estado de renderização".to_string());
        }
        if project
            .tracks
            .iter()
            .any(|track| track.voicebank_path.is_some() || track.phonemizer.is_some())
            || project.voicebank.is_some()
            || project.phonemizer.is_some()
        {
            report
                .dropped_features
                .push("voicebank e phonemizer".to_string());
        }
        if project
            .parts
            .iter()
            .flat_map(|part| &part.notes)
            .any(|note| {
                note.vibrato.length_pct > 0.0
                    || !note.phoneme_overrides.is_empty()
                    || note.envelope != Default::default()
            })
        {
            report
                .dropped_features
                .push("vibrato, envelopes e overrides por fonema".to_string());
        }

        report.warnings = report
            .dropped_features
            .iter()
            .map(|feature| {
                format!("UFData não representa {feature}; o recurso permanece apenas no APS")
            })
            .collect();
        report
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

            if let Some(pitch) = &uf_track.pitch {
                let point_count = pitch.ticks.len().min(pitch.values.len());
                for point_idx in 0..point_count {
                    let Some(value) = pitch.values[point_idx] else {
                        continue;
                    };
                    let tick = pitch.ticks[point_idx];
                    let note_idx = uf_track
                        .notes
                        .iter()
                        .position(|uf_note| tick >= uf_note.tick_on && tick <= uf_note.tick_off);
                    let Some(note_idx) = note_idx else {
                        continue;
                    };
                    let uf_note = &uf_track.notes[note_idx];
                    let note = &mut notes[note_idx];
                    let value_cents = if pitch.is_absolute {
                        (value - note.midi_key() as f64) * 100.0
                    } else {
                        value * 100.0
                    };
                    let absolute_ms = tick as f64 * ms_per_tick;
                    let note_start_ms = uf_note.tick_on as f64 * ms_per_tick;
                    note.pitch_bend
                        .points
                        .push(crate::project::model::UPitchBendPoint {
                            time_offset_ms: absolute_ms - note_start_ms,
                            pitch_offset_cents: value_cents,
                            shape: "io".to_string(),
                        });
                    note.pitch_bend.snap_first = false;
                }
                for note in &mut notes {
                    note.pitch_bend.points.sort_by(|left, right| {
                        left.time_offset_ms
                            .partial_cmp(&right.time_offset_ms)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                }
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

    #[test]
    fn ufdata_roundtrip_exports_absolute_pitch_points() {
        let mut project = UProject::default();
        let mut note = UNote::new("la", "C4", 0.0, 500.0);
        note.pitch_bend
            .points
            .push(crate::project::model::UPitchBendPoint {
                time_offset_ms: 250.0,
                pitch_offset_cents: 50.0,
                shape: "io".to_string(),
            });
        project.parts[0].notes.push(note);

        let ufdata = UfdataFormat::from_uproject(&project);
        let pitch = ufdata.project.tracks[0]
            .pitch
            .as_ref()
            .expect("pitch is exported");
        assert!(pitch.is_absolute);
        assert_eq!(pitch.ticks, vec![240]);
        assert_eq!(pitch.values, vec![Some(60.5)]);

        let restored = UfdataFormat::to_uproject(&ufdata);
        let point = &restored.parts[0].notes[0].pitch_bend.points[0];
        assert!((point.time_offset_ms - 250.0).abs() < 0.01);
        assert!((point.pitch_offset_cents - 50.0).abs() < 0.01);
    }

    #[test]
    fn ufdata_report_identifies_native_features_that_cannot_be_interchanged() {
        let mut project = UProject::default();
        project.tracks[0].fx_rack = Some(crate::audio::FxRackConfig::default());
        project
            .wave_parts
            .push(crate::project::model::UWavePart::new(
                "Backing",
                "backing.wav",
                0,
            ));

        let ufdata = UfdataFormat::from_uproject(&project);
        let report = UfdataFormat::conversion_report(&project, &ufdata);
        assert!(!report.exported_pitch);
        assert!(report
            .dropped_features
            .iter()
            .any(|feature| feature == "FX rack por track"));
        assert!(report
            .dropped_features
            .iter()
            .any(|feature| feature == "partes de áudio"));
        assert_eq!(report.warnings.len(), report.dropped_features.len());
    }
}
