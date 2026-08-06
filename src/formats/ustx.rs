use crate::dsp::pitch::midi_to_note_name;
use crate::project::model::{UNote, UPitchBend, UPitchBendPoint, UProject, UTrack, UVoicePart};
use std::fs;
use std::path::Path;

pub struct UstxFormat;

impl UstxFormat {
    pub fn load_file<P: AsRef<Path>>(path: P) -> Result<UProject, Box<dyn std::error::Error>> {
        let bytes = fs::read(path)?;
        let content = String::from_utf8_lossy(&bytes);
        Self::parse_str(&content)
    }

    pub fn parse_str(content: &str) -> Result<UProject, Box<dyn std::error::Error>> {
        let clean_content = content.trim_start_matches('\u{feff}').trim();

        // Kamafeu's native serialized project uses the same extension but has a
        // strongly typed schema. Parse it before the permissive OpenUTAU reader.
        if let Ok(project) = yaml_serde::from_str::<UProject>(clean_content)
            .or_else(|_| serde_json::from_str::<UProject>(clean_content))
        {
            return Ok(project);
        }
        if let Ok(notes) = serde_json::from_str::<Vec<UNote>>(clean_content) {
            let mut project = UProject::default();
            project.parts[0].notes = notes;
            return Ok(project);
        }

        // Parse the public OpenUTAU schema.
        if let Ok(val) = yaml_serde::from_str::<yaml_serde::Value>(clean_content) {
            if let Some(proj) = Self::parse_from_yaml_value(&val) {
                return Ok(proj);
            }
        }

        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "arquivo USTX não contém um projeto reconhecível",
        )
        .into())
    }

    fn parse_from_yaml_value(val: &yaml_serde::Value) -> Option<UProject> {
        let root = val.get("project").unwrap_or(val);

        let mut bpm = 120.0;
        if let Some(tempos) = root.get("tempos").and_then(|v| v.as_sequence()) {
            if let Some(first_tempo) = tempos.first() {
                bpm = first_tempo
                    .get("bpm")
                    .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
                    .unwrap_or(120.0);
            }
        } else {
            bpm = root
                .get("bpm")
                .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
                .unwrap_or(120.0);
        }

        let name = root
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("OpenUTAU Project")
            .to_string();

        let ms_per_beat = 60000.0 / bpm;
        let ms_per_tick = ms_per_beat / 480.0;

        let mut tracks = root
            .get("tracks")
            .and_then(|value| value.as_sequence())
            .map(|items| {
                items
                    .iter()
                    .enumerate()
                    .map(|(index, value)| UTrack {
                        name: value
                            .get("track_name")
                            .or_else(|| value.get("name"))
                            .and_then(|item| item.as_str())
                            .unwrap_or(if index == 0 { "Track 1" } else { "Track" })
                            .to_string(),
                        singer: value
                            .get("singer")
                            .and_then(|item| item.as_str())
                            .unwrap_or("Default Singer")
                            .to_string(),
                        volume_db: value
                            .get("volume_db")
                            .or_else(|| value.get("volume"))
                            .and_then(|item| item.as_f64())
                            .unwrap_or(0.0),
                        pan: value
                            .get("pan")
                            .and_then(|item| item.as_f64())
                            .unwrap_or(0.0),
                        mute: value
                            .get("mute")
                            .and_then(|item| item.as_bool())
                            .unwrap_or(false),
                        solo: value
                            .get("solo")
                            .and_then(|item| item.as_bool())
                            .unwrap_or(false),
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        let mut parts = Vec::new();

        let parts_arr = root
            .get("voice_parts")
            .or_else(|| root.get("parts"))
            .and_then(|v| v.as_sequence());

        if let Some(parts_seq) = parts_arr {
            for (part_index, part_val) in parts_seq.iter().enumerate() {
                let part_pos_ticks = part_val
                    .get("position")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let part_pos_ms = part_pos_ticks as f64 * ms_per_tick;
                let track_index = part_val
                    .get("track_no")
                    .or_else(|| part_val.get("track_index"))
                    .and_then(|value| value.as_i64())
                    .unwrap_or(0)
                    .max(0) as usize;
                while tracks.len() <= track_index {
                    tracks.push(UTrack {
                        name: format!("Track {}", tracks.len() + 1),
                        ..UTrack::default()
                    });
                }
                let part_name = part_val
                    .get("name")
                    .and_then(|value| value.as_str())
                    .map(str::to_owned)
                    .unwrap_or_else(|| format!("Voice Part {}", part_index + 1));
                let mut part = UVoicePart::new(part_name, track_index);
                part.position_ms = part_pos_ms;

                if let Some(notes_seq) = part_val.get("notes").and_then(|v| v.as_sequence()) {
                    for note_val in notes_seq {
                        let lyric = note_val.get("lyric").and_then(|v| v.as_str()).unwrap_or("");

                        if lyric.is_empty() || lyric == "R" || lyric == "r" {
                            continue;
                        }

                        let tone =
                            note_val.get("tone").and_then(|v| v.as_i64()).unwrap_or(60) as u8;

                        let pos_ticks = note_val
                            .get("position")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);

                        let dur_ticks = note_val
                            .get("duration")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(480);

                        let pitch_str = midi_to_note_name(tone.clamp(0, 127));
                        let pos_ms = pos_ticks as f64 * ms_per_tick;
                        let dur_ms = (dur_ticks as f64 * ms_per_tick).max(20.0);

                        let mut u_note = UNote::new(lyric, pitch_str, pos_ms, dur_ms);

                        // Parse OpenUTAU expressions if present (velocity, dynamics, breathiness, gender, pitch_delta)
                        if let Some(expr_seq) =
                            note_val.get("expressions").and_then(|v| v.as_sequence())
                        {
                            for expr in expr_seq {
                                if let (Some(abbr), Some(val_num)) = (
                                    expr.get("abbr").and_then(|v| v.as_str()),
                                    expr.get("value").and_then(|v| {
                                        v.as_f64().or_else(|| v.as_i64().map(|i| i as f64))
                                    }),
                                ) {
                                    match abbr {
                                        "vel" => {
                                            u_note.expressions.velocity = val_num;
                                            u_note.expressions.consonant_velocity = val_num;
                                        }
                                        "con_vel" | "consonant_velocity" => {
                                            u_note.expressions.consonant_velocity = val_num;
                                        }
                                        "dyn" => u_note.expressions.dynamics = val_num,
                                        "bre" | "bsh" => u_note.expressions.breathiness = val_num,
                                        "gen" | "g" => u_note.expressions.gender = val_num,
                                        "pitd" => u_note.expressions.pitch_delta = val_num,
                                        _ => {}
                                    }
                                }
                            }
                        }

                        // Parse pitch bend points if present
                        if let Some(pitch_data) = note_val
                            .get("pitch")
                            .and_then(|p| p.get("data"))
                            .and_then(|d| d.as_sequence())
                        {
                            let mut pts = Vec::new();
                            for pt_val in pitch_data {
                                let px = pt_val
                                    .get("x")
                                    .and_then(|v| {
                                        v.as_f64().or_else(|| v.as_i64().map(|i| i as f64))
                                    })
                                    .unwrap_or(0.0);
                                let py = pt_val
                                    .get("y")
                                    .and_then(|v| {
                                        v.as_f64().or_else(|| v.as_i64().map(|i| i as f64))
                                    })
                                    .unwrap_or(0.0);
                                let pshape = pt_val
                                    .get("shape")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("s")
                                    .to_string();

                                pts.push(UPitchBendPoint {
                                    time_offset_ms: px * ms_per_tick,
                                    pitch_offset_cents: py * 10.0,
                                    shape: pshape,
                                });
                            }
                            if !pts.is_empty() {
                                u_note.pitch_bend = UPitchBend { points: pts };
                            }
                        }

                        part.notes.push(u_note);
                    }
                }
                parts.push(part);
            }
        }

        if tracks.is_empty() && parts.is_empty() {
            return None;
        }
        if tracks.is_empty() {
            tracks.push(UTrack::default());
        }
        if parts.is_empty() {
            parts.push(UVoicePart::new("Voice Part 1", 0));
        }

        Some(UProject {
            name,
            bpm,
            tracks,
            parts,
        })
    }

    pub fn save_file<P: AsRef<Path>>(
        project: &UProject,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let ticks_per_ms = 480.0 * project.bpm.max(1.0) / 60_000.0;
        let tracks = project
            .tracks
            .iter()
            .map(|track| {
                serde_json::json!({
                    "track_name": track.name,
                    "singer": track.singer,
                    "volume_db": track.volume_db,
                    "pan": track.pan,
                    "mute": track.mute,
                    "solo": track.solo,
                })
            })
            .collect::<Vec<_>>();
        let parts = project
            .parts
            .iter()
            .map(|part| {
                let notes = part
                    .notes
                    .iter()
                    .map(|note| {
                        let pitch_data = note
                            .pitch_bend
                            .points
                            .iter()
                            .map(|point| {
                                serde_json::json!({
                                    "x": (point.time_offset_ms * ticks_per_ms).round() as i64,
                                    "y": point.pitch_offset_cents / 10.0,
                                    "shape": point.shape,
                                })
                            })
                            .collect::<Vec<_>>();
                        serde_json::json!({
                            "position": (note.position_ms * ticks_per_ms).round() as i64,
                            "duration": (note.duration_ms.max(1.0) * ticks_per_ms).round() as i64,
                            "tone": note.midi_key(),
                            "lyric": note.lyric,
                            "pitch": { "data": pitch_data },
                            "expressions": [
                                { "abbr": "vel", "value": note.expressions.consonant_velocity },
                                { "abbr": "dyn", "value": note.expressions.dynamics },
                                { "abbr": "bre", "value": note.expressions.breathiness },
                                { "abbr": "gen", "value": note.expressions.gender },
                                { "abbr": "pitd", "value": note.expressions.pitch_delta },
                            ],
                        })
                    })
                    .collect::<Vec<_>>();
                serde_json::json!({
                    "name": part.name,
                    "track_no": part.track_index,
                    "position": (part.position_ms * ticks_per_ms).round() as i64,
                    "notes": notes,
                })
            })
            .collect::<Vec<_>>();
        let document = serde_json::json!({
            "ustx_version": "0.6",
            "name": project.name,
            "tempos": [{ "position": 0, "bpm": project.bpm }],
            "tracks": tracks,
            "voice_parts": parts,
        });
        let yaml_str = yaml_serde::to_string(&document)?;
        fs::write(path, yaml_str)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ustx_roundtrip() {
        let default_proj = UProject::default();
        let yaml = yaml_serde::to_string(&default_proj).unwrap();
        let parsed = UstxFormat::parse_str(&yaml).unwrap();
        assert_eq!(parsed.name, default_proj.name);
    }

    #[test]
    fn test_openutau_ustx_parse() {
        let openutau_yaml = r#"
name: Test OpenUTAU USTX
bpm: 140
tracks:
  - name: Track 1
    singer: UTAU Singer
voice_parts:
  - name: Voice Part 1
    track_no: 0
    position: 0
    notes:
      - position: 0
        duration: 480
        tone: 60
        lyric: ka
        expressions:
          - abbr: vel
            value: 160
      - position: 480
        duration: 480
        tone: 62
        lyric: ma
"#;
        let proj = UstxFormat::parse_str(openutau_yaml).expect("Failed to parse OpenUTAU USTX");
        assert_eq!(proj.name, "Test OpenUTAU USTX");
        assert_eq!(proj.bpm, 140.0);
        assert_eq!(proj.parts[0].notes.len(), 2);
        assert_eq!(proj.parts[0].notes[0].lyric, "ka");
        assert_eq!(proj.parts[0].notes[0].expressions.consonant_velocity, 160.0);
        assert_eq!(proj.parts[0].notes[1].lyric, "ma");
    }

    #[test]
    fn test_openutau_multitrack_roundtrip() {
        let mut project = UProject {
            name: "Multitrack".to_string(),
            bpm: 120.0,
            tracks: vec![
                UTrack::default(),
                UTrack {
                    name: "Harmony".to_string(),
                    ..UTrack::default()
                },
            ],
            parts: vec![UVoicePart::new("Lead", 0), UVoicePart::new("Harmony", 1)],
        };
        project.parts[0]
            .notes
            .push(UNote::new("ka", "C4", 0.0, 500.0));
        project.parts[1].position_ms = 250.0;
        project.parts[1]
            .notes
            .push(UNote::new("a", "E4", 125.0, 750.0));

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("roundtrip.ustx");
        UstxFormat::save_file(&project, &path).unwrap();
        let parsed = UstxFormat::load_file(path).unwrap();

        assert_eq!(parsed.tracks.len(), 2);
        assert_eq!(parsed.parts.len(), 2);
        assert_eq!(parsed.parts[1].track_index, 1);
        assert!((parsed.parts[1].position_ms - 250.0).abs() < 0.01);
        assert!((parsed.parts[1].notes[0].position_ms - 125.0).abs() < 0.01);
        assert_eq!(parsed.parts[1].notes[0].midi_key(), 64);
    }

    #[test]
    fn test_parse_real_astro_ustx() {
        if std::path::Path::new("/Users/victor/Downloads/astro.ustx").exists() {
            let proj = UstxFormat::load_file("/Users/victor/Downloads/astro.ustx")
                .expect("Failed to parse astro.ustx");
            println!(
                "Loaded astro.ustx: name={}, bpm={}, notes_count={}",
                proj.name,
                proj.bpm,
                proj.parts[0].notes.len()
            );
            assert!(!proj.parts[0].notes.is_empty());
        }
    }
}
