use std::fs;
use std::path::Path;
use crate::dsp::pitch::midi_to_note_name;
use crate::project::model::{UNote, UPitchBend, UPitchBendPoint, UProject, UTrack, UVoicePart};

pub struct UstxFormat;

impl UstxFormat {
    pub fn load_file<P: AsRef<Path>>(path: P) -> Result<UProject, Box<dyn std::error::Error>> {
        let bytes = fs::read(path)?;
        let content = String::from_utf8_lossy(&bytes);
        Self::parse_str(&content)
    }

    pub fn parse_str(content: &str) -> Result<UProject, Box<dyn std::error::Error>> {
        let clean_content = content.trim_start_matches('\u{feff}').trim();
        // 1. Try robust YAML Value parsing (handles any OpenUTAU .ustx schema)
        if let Ok(val) = serde_yaml::from_str::<serde_yaml::Value>(clean_content) {
            if let Some(proj) = Self::parse_from_yaml_value(&val) {
                return Ok(proj);
            }
        }

        // 2. Try parsing as Kamafeu UProject (YAML or JSON)
        let proj: UProject = serde_yaml::from_str(clean_content)
            .or_else(|_| serde_json::from_str(clean_content))?;
        Ok(proj)
    }

    fn parse_from_yaml_value(val: &serde_yaml::Value) -> Option<UProject> {
        let bpm = val.get("bpm")
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
            .unwrap_or(120.0);

        let name = val.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("OpenUTAU Project")
            .to_string();

        let ms_per_beat = 60000.0 / bpm;
        let ms_per_tick = ms_per_beat / 480.0;

        let mut all_notes = Vec::new();

        let parts_arr = val.get("voice_parts")
            .or_else(|| val.get("parts"))
            .and_then(|v| v.as_sequence());

        if let Some(parts_seq) = parts_arr {
            for part_val in parts_seq {
                let part_pos_ticks = part_val.get("position")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0);
                let part_pos_ms = part_pos_ticks as f64 * ms_per_tick;

                if let Some(notes_seq) = part_val.get("notes").and_then(|v| v.as_sequence()) {
                    for note_val in notes_seq {
                        let lyric = note_val.get("lyric")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        if lyric.is_empty() || lyric == "R" || lyric == "r" {
                            continue;
                        }

                        let tone = note_val.get("tone")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(60) as u8;

                        let pos_ticks = note_val.get("position")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);

                        let dur_ticks = note_val.get("duration")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(480);

                        let pitch_str = midi_to_note_name(tone.clamp(0, 127));
                        let pos_ms = (pos_ticks as f64 * ms_per_tick) + part_pos_ms;
                        let dur_ms = (dur_ticks as f64 * ms_per_tick).max(20.0);

                        let mut u_note = UNote::new(lyric, pitch_str, pos_ms, dur_ms);

                        // Parse OpenUTAU expressions if present (velocity, dynamics, breathiness, gender, pitch_delta)
                        if let Some(expr_seq) = note_val.get("expressions").and_then(|v| v.as_sequence()) {
                            for expr in expr_seq {
                                if let (Some(abbr), Some(val_num)) = (
                                    expr.get("abbr").and_then(|v| v.as_str()),
                                    expr.get("value").and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
                                ) {
                                    match abbr {
                                        "vel" => u_note.expressions.velocity = val_num,
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
                        if let Some(pitch_data) = note_val.get("pitch").and_then(|p| p.get("data")).and_then(|d| d.as_sequence()) {
                            let mut pts = Vec::new();
                            for pt_val in pitch_data {
                                let px = pt_val.get("x").and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64))).unwrap_or(0.0);
                                let py = pt_val.get("y").and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64))).unwrap_or(0.0);
                                let pshape = pt_val.get("shape").and_then(|v| v.as_str()).unwrap_or("s").to_string();

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

                        all_notes.push(u_note);
                    }
                }
            }
        }

        if all_notes.is_empty() {
            return None;
        }

        let mut master_part = UVoicePart::new("Voice Part 1", 0);
        master_part.notes = all_notes;

        Some(UProject {
            name,
            bpm,
            tracks: vec![UTrack::default()],
            parts: vec![master_part],
        })
    }

    pub fn save_file<P: AsRef<Path>>(project: &UProject, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let yaml_str = serde_yaml::to_string(project)?;
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
        let yaml = serde_yaml::to_string(&default_proj).unwrap();
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
        assert_eq!(proj.parts[0].notes[1].lyric, "ma");
    }

    #[test]
    fn test_parse_real_astro_ustx() {
        if std::path::Path::new("/Users/victor/Downloads/astro.ustx").exists() {
            let proj = UstxFormat::load_file("/Users/victor/Downloads/astro.ustx").expect("Failed to parse astro.ustx");
            println!("Loaded astro.ustx: name={}, bpm={}, notes_count={}", proj.name, proj.bpm, proj.parts[0].notes.len());
            assert!(!proj.parts[0].notes.is_empty());
        }
    }
}
