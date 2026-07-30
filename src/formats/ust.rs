use std::fs;
use std::path::Path;
use encoding_rs::SHIFT_JIS;

use crate::dsp::pitch::midi_to_note_name;
use crate::project::model::{UNote, UPitchBend, UPitchBendPoint, UProject, UVoicePart};

pub struct UstFormat;

impl UstFormat {
    pub fn parse_bytes(bytes: &[u8]) -> Result<UProject, Box<dyn std::error::Error>> {
        let (text, _, _) = SHIFT_JIS.decode(bytes);
        Self::parse_str(&text)
    }

    pub fn parse_str(content: &str) -> Result<UProject, Box<dyn std::error::Error>> {
        let mut project = UProject::default();
        let mut notes: Vec<UNote> = Vec::new();

        let mut current_lyric = String::new();
        let mut current_note_num: u8 = 60;
        let mut current_length_ticks: f64 = 480.0; // 480 ticks = 1 beat (500ms at 120BPM)
        let mut current_bpm: f64 = 120.0;
        let mut current_pb_start_ms: f64 = 0.0;
        let mut current_pb_start_cents: f64 = 0.0;
        let mut current_pbw: Vec<f64> = Vec::new();
        let mut current_pby: Vec<f64> = Vec::new();
        let mut current_pbm: Vec<String> = Vec::new();

        let mut current_time_ms = 0.0f64;
        let mut in_note_section = false;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') {
                continue;
            }

            if line.starts_with("[#") && line.ends_with(']') {
                let section_name = &line[2..line.len() - 1];
                if section_name != "VERSION" && section_name != "SETTING" && section_name != "TRACKEND" {
                    if in_note_section && !current_lyric.is_empty() && current_lyric != "R" && current_lyric != "r" {
                        let duration_ms = (current_length_ticks / 480.0) * (60000.0 / current_bpm);
                        let mut note = UNote::new(
                            &current_lyric,
                            midi_to_note_name(current_note_num),
                            current_time_ms,
                            duration_ms,
                        );

                        // Build pitch bend points
                        if !current_pbw.is_empty() {
                            let mut pb_points = Vec::new();
                            pb_points.push(UPitchBendPoint {
                                time_offset_ms: current_pb_start_ms,
                                pitch_offset_cents: current_pb_start_cents,
                                shape: String::new(),
                            });

                            let mut cum_ms = current_pb_start_ms;
                            for i in 0..current_pbw.len() {
                                cum_ms += current_pbw[i];
                                let y_decacents = if i < current_pby.len() { current_pby[i] } else { 0.0 };
                                let cents = y_decacents * 10.0;
                                let shape = if i < current_pbm.len() { current_pbm[i].clone() } else { String::new() };
                                pb_points.push(UPitchBendPoint {
                                    time_offset_ms: cum_ms,
                                    pitch_offset_cents: cents,
                                    shape,
                                });
                            }

                            note.pitch_bend = UPitchBend { points: pb_points };
                        }

                        notes.push(note);
                        current_time_ms += duration_ms;
                    } else if in_note_section {
                        let duration_ms = (current_length_ticks / 480.0) * (60000.0 / current_bpm);
                        current_time_ms += duration_ms;
                    }

                    in_note_section = true;
                    current_lyric.clear();
                    current_pbw.clear();
                    current_pby.clear();
                    current_pbm.clear();
                    current_pb_start_ms = 0.0;
                    current_pb_start_cents = 0.0;
                }
                continue;
            }

            let parts: Vec<&str> = line.splitn(2, '=').collect();
            if parts.len() == 2 {
                let key = parts[0].trim();
                let val = parts[1].trim();

                match key {
                    "Tempo" => {
                        let val_dot = val.replace(',', ".");
                        if let Ok(bpm) = val_dot.parse::<f64>() {
                            current_bpm = bpm;
                            project.bpm = bpm;
                        }
                    }
                    "Lyric" => current_lyric = val.to_string(),
                    "NoteNum" => current_note_num = val.parse::<u8>().unwrap_or(60),
                    "Length" => current_length_ticks = val.parse::<f64>().unwrap_or(480.0),
                    "PBS" => {
                        let pbs_parts: Vec<&str> = val.split(';').collect();
                        if pbs_parts.len() >= 1 {
                            current_pb_start_ms = pbs_parts[0].parse().unwrap_or(0.0);
                        }
                        if pbs_parts.len() >= 2 {
                            current_pb_start_cents = pbs_parts[1].parse::<f64>().unwrap_or(0.0) * 10.0; // decacents to cents
                        }
                    }
                    "PBW" => {
                        current_pbw = val.split(',').filter_map(|s| s.parse::<f64>().ok()).collect();
                    }
                    "PBY" => {
                        current_pby = val.split(',').filter_map(|s| s.parse::<f64>().ok()).collect();
                    }
                    "PBM" => {
                        current_pbm = val.split(',').map(|s| s.to_string()).collect();
                    }
                    _ => {}
                }
            }
        }

        // Push final note if present
        if in_note_section && !current_lyric.is_empty() && current_lyric != "R" && current_lyric != "r" {
            let duration_ms = (current_length_ticks / 480.0) * (60000.0 / current_bpm);
            let note = UNote::new(
                &current_lyric,
                midi_to_note_name(current_note_num),
                current_time_ms,
                duration_ms,
            );
            notes.push(note);
        }

        if !notes.is_empty() {
            let mut part = UVoicePart::new("UST Import", 0);
            part.notes = notes;
            project.parts = vec![part];
        }

        Ok(project)
    }

    pub fn load_file<P: AsRef<Path>>(path: P) -> Result<UProject, Box<dyn std::error::Error>> {
        let bytes = fs::read(path)?;
        Self::parse_bytes(&bytes)
    }

    pub fn save_file<P: AsRef<Path>>(project: &UProject, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let content = Self::to_ust_string(project);
        let (bytes, _, _) = SHIFT_JIS.encode(&content);
        fs::write(path, bytes)?;
        Ok(())
    }

    pub fn to_ust_string(project: &UProject) -> String {
        let mut out = String::new();
        out.push_str("[#VERSION]\nUST Version1.2\n[#SETTING]\n");
        out.push_str(&format!("Tempo={:.2}\n", project.bpm));
        out.push_str("Tracks=1\n");

        let notes = if !project.parts.is_empty() {
            &project.parts[0].notes
        } else {
            &Vec::new()
        };

        let mut curr_ms = 0.0f64;
        let ms_per_beat = 60000.0 / project.bpm.max(20.0);

        for (idx, note) in notes.iter().enumerate() {
            // Check for gap rest
            if note.position_ms > curr_ms + 1.0 {
                let rest_dur_ms = note.position_ms - curr_ms;
                let rest_ticks = ((rest_dur_ms / ms_per_beat) * 480.0).round() as u64;
                out.push_str(&format!("[#{:04}]\nLength={}\nLyric=R\nNoteNum=60\n", idx * 2, rest_ticks));
            }

            let ticks = ((note.duration_ms / ms_per_beat) * 480.0).round() as u64;
            out.push_str(&format!("[#{:04}]\n", idx * 2 + 1));
            out.push_str(&format!("Length={}\n", ticks.max(15)));
            out.push_str(&format!("Lyric={}\n", note.lyric));
            out.push_str(&format!("NoteNum={}\n", note.midi_key()));

            if !note.pitch_bend.points.is_empty() {
                let pbs_time = note.pitch_bend.points[0].time_offset_ms;
                let pbs_cents = note.pitch_bend.points[0].pitch_offset_cents / 10.0;
                out.push_str(&format!("PBS={:.1};{:.1}\n", pbs_time, pbs_cents));

                let mut pbw = Vec::new();
                let mut pby = Vec::new();
                let mut pbm = Vec::new();

                for w in note.pitch_bend.points.windows(2) {
                    pbw.push(format!("{:.1}", (w[1].time_offset_ms - w[0].time_offset_ms).max(0.0)));
                    pby.push(format!("{:.1}", w[1].pitch_offset_cents / 10.0));
                    pbm.push(if w[1].shape.is_empty() { "s".to_string() } else { w[1].shape.clone() });
                }

                if !pbw.is_empty() {
                    out.push_str(&format!("PBW={}\n", pbw.join(",")));
                    out.push_str(&format!("PBY={}\n", pby.join(",")));
                    out.push_str(&format!("PBM={}\n", pbm.join(",")));
                }
            }

            curr_ms = note.position_ms + note.duration_ms;
        }

        out.push_str("[#TRACKEND]\n");
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ust_string() {
        let sample_ust = r#"
[#SETTING]
Tempo=145.00
[#0000]
Length=480
Lyric=ka
NoteNum=60
PBS=-40;0
PBW=50,50
PBY=0,10
PBM=s,
[#0001]
Length=480
Lyric=ki
NoteNum=62
"#;
        let proj = UstFormat::parse_str(sample_ust).unwrap();
        assert_eq!(proj.bpm, 145.0);
        assert_eq!(proj.parts[0].notes.len(), 2);
        assert_eq!(proj.parts[0].notes[0].lyric, "ka");
        assert_eq!(proj.parts[0].notes[1].lyric, "ki");

        let exported = UstFormat::to_ust_string(&proj);
        assert!(exported.contains("Lyric=ka"));
        assert!(exported.contains("Lyric=ki"));
    }
}
