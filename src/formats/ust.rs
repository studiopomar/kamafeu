use encoding_rs::SHIFT_JIS;
use std::fs;
use std::path::Path;

use crate::dsp::envelope::UtauEnvelope;
use crate::dsp::pitch::midi_to_note_name;
use crate::dsp::pitch::VibratoParam;
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
        let mut current_consonant_velocity: f64 = 100.0;
        let mut current_pb_start_ms: f64 = 0.0;
        let mut current_pb_start_cents: f64 = 0.0;
        let mut current_pbw: Vec<f64> = Vec::new();
        let mut current_pby: Vec<f64> = Vec::new();
        let mut current_pbm: Vec<String> = Vec::new();
        let mut current_vibrato = VibratoParam::default();
        let mut current_envelope = UtauEnvelope::default();
        let mut current_volume = 100.0f64;
        let mut current_modulation = 0.0f64;
        let mut current_flags = String::new();

        let mut current_time_ms = 0.0f64;
        let mut in_note_section = false;

        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') {
                continue;
            }

            if line.starts_with("[#") && line.ends_with(']') {
                let section_name = &line[2..line.len() - 1];
                if section_name != "VERSION"
                    && section_name != "SETTING"
                    && section_name != "TRACKEND"
                {
                    if in_note_section
                        && !current_lyric.is_empty()
                        && current_lyric != "R"
                        && current_lyric != "r"
                    {
                        let duration_ms = (current_length_ticks / 480.0) * (60000.0 / current_bpm);
                        let mut note = UNote::new(
                            &current_lyric,
                            midi_to_note_name(current_note_num),
                            current_time_ms,
                            duration_ms,
                        );
                        note.expressions.consonant_velocity = current_consonant_velocity;
                        note.expressions.velocity = current_consonant_velocity;
                        note.expressions.volume = current_volume;
                        note.expressions.modulation = current_modulation;
                        note.vibrato = current_vibrato.clone();
                        note.envelope = current_envelope.clone();
                        note.flags = current_flags.clone();

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
                                let y_decacents = if i < current_pby.len() {
                                    current_pby[i]
                                } else {
                                    0.0
                                };
                                let cents = y_decacents * 10.0;
                                let shape = if i < current_pbm.len() {
                                    current_pbm[i].clone()
                                } else {
                                    String::new()
                                };
                                pb_points.push(UPitchBendPoint {
                                    time_offset_ms: cum_ms,
                                    pitch_offset_cents: cents,
                                    shape,
                                });
                            }

                            note.pitch_bend = UPitchBend {
                                points: pb_points,
                                snap_first: false,
                                ..UPitchBend::default()
                            };
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
                    current_consonant_velocity = 100.0;
                    current_vibrato = VibratoParam::default();
                    current_envelope = UtauEnvelope::default();
                    current_volume = 100.0;
                    current_modulation = 0.0;
                    current_flags.clear();
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
                    "Velocity" => {
                        current_consonant_velocity =
                            val.parse::<f64>().unwrap_or(100.0).clamp(0.0, 200.0);
                    }
                    "Intensity" => current_volume = val.parse().unwrap_or(100.0),
                    "Modulation" => current_modulation = val.parse().unwrap_or(0.0),
                    "Flags" => current_flags = val.to_string(),
                    "VBR" => {
                        let values = val
                            .split(',')
                            .map(|item| item.trim().replace(',', ".").parse::<f64>().unwrap_or(0.0))
                            .collect::<Vec<_>>();
                        if values.len() >= 3 {
                            current_vibrato.length_pct = values[0];
                            current_vibrato.period_ms = values[1];
                            current_vibrato.depth_cents = values[2];
                            current_vibrato.fade_in_pct = *values.get(3).unwrap_or(&0.0);
                            current_vibrato.fade_out_pct = *values.get(4).unwrap_or(&0.0);
                            current_vibrato.shift_pct = *values.get(5).unwrap_or(&0.0);
                            current_vibrato.drift_pct = *values.get(6).unwrap_or(&0.0);
                        }
                    }
                    "Envelope" => {
                        let values = val
                            .split(',')
                            .filter_map(|item| item.trim().parse::<f64>().ok())
                            .collect::<Vec<_>>();
                        if values.len() >= 7 {
                            current_envelope.p1 = values[0];
                            current_envelope.p2 = values[1];
                            current_envelope.p5 = values[2];
                            current_envelope.v1 = values[3];
                            current_envelope.v2 = values[4];
                            current_envelope.v4 = values[5];
                            current_envelope.v5 = values[6];
                            if values.len() >= 11 {
                                current_envelope.p4 = values[8];
                                current_envelope.p3 = values[9];
                                current_envelope.v3 = values[10];
                            }
                        }
                    }
                    "PBS" => {
                        let pbs_parts: Vec<&str> = val.split(';').collect();
                        if !pbs_parts.is_empty() {
                            current_pb_start_ms = pbs_parts[0].parse().unwrap_or(0.0);
                        }
                        if pbs_parts.len() >= 2 {
                            current_pb_start_cents =
                                pbs_parts[1].parse::<f64>().unwrap_or(0.0) * 10.0;
                            // decacents to cents
                        }
                    }
                    "PBW" => {
                        current_pbw = val
                            .split(',')
                            .filter_map(|s| s.parse::<f64>().ok())
                            .collect();
                    }
                    "PBY" => {
                        current_pby = val
                            .split(',')
                            .filter_map(|s| s.parse::<f64>().ok())
                            .collect();
                    }
                    "PBM" => {
                        current_pbm = val.split(',').map(|s| s.to_string()).collect();
                    }
                    _ => {}
                }
            }
        }

        // Push final note if present
        if in_note_section
            && !current_lyric.is_empty()
            && current_lyric != "R"
            && current_lyric != "r"
        {
            let duration_ms = (current_length_ticks / 480.0) * (60000.0 / current_bpm);
            let mut note = UNote::new(
                &current_lyric,
                midi_to_note_name(current_note_num),
                current_time_ms,
                duration_ms,
            );
            note.expressions.consonant_velocity = current_consonant_velocity;
            note.expressions.velocity = current_consonant_velocity;
            note.expressions.volume = current_volume;
            note.expressions.modulation = current_modulation;
            note.vibrato = current_vibrato;
            note.envelope = current_envelope;
            note.flags = current_flags;
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

    pub fn save_file<P: AsRef<Path>>(
        project: &UProject,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
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
                out.push_str(&format!(
                    "[#{:04}]\nLength={}\nLyric=R\nNoteNum=60\n",
                    idx * 2,
                    rest_ticks
                ));
            }

            let ticks = ((note.duration_ms / ms_per_beat) * 480.0).round() as u64;
            out.push_str(&format!("[#{:04}]\n", idx * 2 + 1));
            out.push_str(&format!("Length={}\n", ticks.max(15)));
            out.push_str(&format!("Lyric={}\n", note.lyric));
            out.push_str(&format!("NoteNum={}\n", note.midi_key()));
            out.push_str(&format!(
                "Velocity={:.0}\n",
                note.expressions.consonant_velocity.clamp(0.0, 200.0)
            ));
            out.push_str(&format!(
                "Intensity={:.0}\n",
                note.expressions.volume.clamp(0.0, 200.0)
            ));
            out.push_str(&format!(
                "Modulation={:.0}\n",
                note.expressions.modulation.clamp(0.0, 100.0)
            ));
            if !note.flags.is_empty() {
                out.push_str(&format!("Flags={}\n", note.flags));
            }
            if note.vibrato.length_pct > 0.0 {
                out.push_str(&format!(
                    "VBR={:.1},{:.1},{:.1},{:.1},{:.1},{:.1},{:.1}\n",
                    note.vibrato.length_pct,
                    note.vibrato.period_ms,
                    note.vibrato.depth_cents,
                    note.vibrato.fade_in_pct,
                    note.vibrato.fade_out_pct,
                    note.vibrato.shift_pct,
                    note.vibrato.drift_pct,
                ));
            }
            let env = &note.envelope;
            out.push_str(&format!(
                "Envelope={:.1},{:.1},{:.1},{:.0},{:.0},{:.0},{:.0},0,{:.1},{:.1},{:.0}\n",
                env.p1, env.p2, env.p5, env.v1, env.v2, env.v4, env.v5, env.p4, env.p3, env.v3,
            ));

            let previous = idx.checked_sub(1).and_then(|index| notes.get(index));
            let adjacent = previous.is_some_and(|previous| {
                (previous.position_ms + previous.duration_ms - note.position_ms).abs() <= 1.0
            });
            let pitch_points = note.pitch_bend.effective_points(
                previous.map(UNote::midi_key),
                note.midi_key(),
                adjacent,
            );
            if !pitch_points.is_empty() {
                let pbs_time = pitch_points[0].time_offset_ms;
                let pbs_cents = pitch_points[0].pitch_offset_cents / 10.0;
                out.push_str(&format!("PBS={:.1};{:.1}\n", pbs_time, pbs_cents));

                let mut pbw = Vec::new();
                let mut pby = Vec::new();
                let mut pbm = Vec::new();

                for w in pitch_points.windows(2) {
                    pbw.push(format!(
                        "{:.1}",
                        (w[1].time_offset_ms - w[0].time_offset_ms).max(0.0)
                    ));
                    pby.push(format!("{:.1}", w[1].pitch_offset_cents / 10.0));
                    pbm.push(match w[0].shape.as_str() {
                        "l" | "s" => "s".to_string(),
                        "i" | "j" => "j".to_string(),
                        "o" | "r" => "r".to_string(),
                        _ => String::new(),
                    });
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
Velocity=150
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
        assert_eq!(proj.parts[0].notes[0].expressions.consonant_velocity, 150.0);
        assert_eq!(proj.parts[0].notes[1].lyric, "ki");

        let exported = UstFormat::to_ust_string(&proj);
        assert!(exported.contains("Lyric=ka"));
        assert!(exported.contains("Lyric=ki"));
        assert!(exported.contains("Velocity=150"));
    }

    #[test]
    fn vibrato_envelope_and_volume_roundtrip() {
        let source = r#"[#SETTING]
Tempo=120
[#0000]
Length=480
Lyric=a
NoteNum=69
Velocity=130
Intensity=82
Modulation=7
Flags=g-4
VBR=65,180,40,20,25,10,-5
Envelope=0,8,40,0,90,75,0,15,5,30,85
[#TRACKEND]
"#;
        let project = UstFormat::parse_str(source).unwrap();
        let note = &project.parts[0].notes[0];
        assert_eq!(note.vibrato.length_pct, 65.0);
        assert_eq!(note.vibrato.fade_out_pct, 25.0);
        assert_eq!(note.expressions.volume, 82.0);
        assert_eq!(note.expressions.modulation, 7.0);
        assert_eq!(note.flags, "g-4");
        assert_eq!(note.envelope.v3, 85.0);
        let exported = UstFormat::to_ust_string(&project);
        assert!(exported.contains("VBR=65.0,180.0,40.0,20.0,25.0,10.0,-5.0"));
        assert!(exported.contains("Intensity=82"));
        assert!(exported.contains("Envelope="));
    }
}
