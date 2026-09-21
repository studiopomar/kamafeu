use crate::dsp::pitch::midi_to_note_name;
use crate::dsp::pitch_bend::PitchBendSolver;
use crate::project::model::{UNote, UPitchBend, UPitchBendPoint, UProject, UTrack, UVoicePart};
use std::fs;
use std::path::Path;

pub const VSQX_DEFAULT_TICKS_PER_BEAT: f64 = 480.0;

pub struct VsqxFormat;

#[derive(Debug, Clone)]
struct VsqxCc {
    pos_tick: i64,
    attr_id: String,
    value: i64,
}

impl VsqxFormat {
    pub fn load_file<P: AsRef<Path>>(path: P) -> Result<UProject, Box<dyn std::error::Error>> {
        let bytes = fs::read(path)?;
        let content = String::from_utf8_lossy(&bytes);
        Self::parse_str(&content)
    }

    pub fn parse_str(content: &str) -> Result<UProject, Box<dyn std::error::Error>> {
        let clean = content.trim_start_matches('\u{feff}').trim();
        if !clean.contains("<vsq3") && !clean.contains("<vsq4") && !clean.contains("<vsq") {
            return Err("Not a recognized VSQX document".into());
        }

        let mut bpm = 120.0;
        if let Some(bpm_pos) = clean.find("<bpm>") {
            if let Some(bpm_end) = clean[bpm_pos..].find("</bpm>") {
                let bpm_str = &clean[bpm_pos + 5..bpm_pos + bpm_end];
                if let Ok(val) = bpm_str.trim().parse::<f64>() {
                    if val > 1000.0 {
                        bpm = val / 100.0;
                    } else if val > 0.0 {
                        bpm = val;
                    }
                }
            }
        }
        if !bpm.is_finite() || !(20.0..=999.0).contains(&bpm) {
            bpm = 120.0;
        }

        let ms_per_beat = 60000.0 / bpm;
        let ms_per_tick = ms_per_beat / VSQX_DEFAULT_TICKS_PER_BEAT;

        let mut tracks = Vec::new();
        let mut parts = Vec::new();

        let mut track_start_search = 0;
        let mut track_counter = 0;

        while let Some(track_start) = clean[track_start_search..].find("<vsTrack>") {
            let actual_start = track_start_search + track_start;
            let actual_end = if let Some(track_end) = clean[actual_start..].find("</vsTrack>") {
                actual_start + track_end + "</vsTrack>".len()
            } else {
                clean.len()
            };

            let track_block = &clean[actual_start..actual_end];
            track_start_search = actual_end;
            track_counter += 1;

            let track_name = extract_tag_value(track_block, "trackName")
                .unwrap_or_else(|| format!("Track {}", track_counter));

            let mut notes = Vec::new();

            let mut part_search = 0;
            while let Some(part_pos) = track_block[part_search..].find("<musicalPart>") {
                let p_start = part_search + part_pos;
                let p_end = if let Some(p_close) = track_block[p_start..].find("</musicalPart>") {
                    p_start + p_close + "</musicalPart>".len()
                } else {
                    track_block.len()
                };
                let part_block = &track_block[p_start..p_end];
                part_search = p_end;

                let part_pos_tick: i64 = extract_tag_value(part_block, "posTick")
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(0);

                let cc_events = extract_cc_events(part_block);

                let mut pit_events: Vec<(i64, i64)> = Vec::new();
                let mut pbs_value: f64 = 2.0; // Default PBS = 2 semitones

                for cc in &cc_events {
                    match cc.attr_id.as_str() {
                        "PBS" | "pbs" => {
                            pbs_value = (cc.value as f64).clamp(1.0, 24.0);
                        }
                        "PIT" | "pit" => {
                            pit_events.push((cc.pos_tick, cc.value));
                        }
                        _ => {}
                    }
                }
                pit_events.sort_by_key(|(tick, _)| *tick);

                let mut note_search = 0;
                while let Some(note_pos) = part_block[note_search..].find("<note>") {
                    let n_start = note_search + note_pos;
                    let n_end = if let Some(n_close) = part_block[n_start..].find("</note>") {
                        n_start + n_close + "</note>".len()
                    } else {
                        part_block.len()
                    };
                    let note_block = &part_block[n_start..n_end];
                    note_search = n_end;

                    let pos_tick: i64 = extract_tag_value(note_block, "posTick")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(0);
                    let dur_tick: i64 = extract_tag_value(note_block, "durTick")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(480);
                    let note_num: u8 = extract_tag_value(note_block, "noteNum")
                        .and_then(|s| s.parse().ok())
                        .unwrap_or(60);
                    let lyric =
                        extract_tag_value(note_block, "lyric").unwrap_or_else(|| "a".to_string());

                    let total_tick = part_pos_tick + pos_tick;
                    let pos_ms = (total_tick as f64 * ms_per_tick).max(0.0);
                    let dur_ms = (dur_tick as f64 * ms_per_tick).max(10.0);
                    let pitch_name = midi_to_note_name(note_num);

                    let mut u_note = UNote::new(lyric, pitch_name, pos_ms, dur_ms);

                    if !pit_events.is_empty() {
                        let note_start_tick = total_tick;
                        let note_end_tick = total_tick + dur_tick;

                        let mut note_pit_points: Vec<UPitchBendPoint> = Vec::new();

                        for &(pit_tick, pit_val) in &pit_events {
                            if pit_tick >= note_start_tick - 120 && pit_tick <= note_end_tick {
                                let rel_ms = (pit_tick - note_start_tick) as f64 * ms_per_tick;
                                let cents = (pit_val as f64 / 8192.0) * pbs_value * 100.0;
                                note_pit_points.push(UPitchBendPoint {
                                    time_offset_ms: rel_ms,
                                    pitch_offset_cents: cents,
                                    shape: "s".to_string(),
                                });
                            }
                        }

                        if note_pit_points.len() >= 2 {
                            let simplified =
                                PitchBendSolver::simplify_pitch_points(&note_pit_points, 2.0);
                            let portamento_start_ms = simplified
                                .first()
                                .map(|p| p.time_offset_ms)
                                .unwrap_or(-40.0);
                            let portamento_length_ms = simplified
                                .get(1)
                                .map(|second| second.time_offset_ms - portamento_start_ms)
                                .unwrap_or(80.0)
                                .max(1.0);
                            u_note.pitch_bend = UPitchBend {
                                points: simplified,
                                snap_first: false,
                                portamento_start_ms,
                                portamento_length_ms,
                                portamento_shape: "s".to_string(),
                            };
                        }
                    }

                    notes.push(u_note);
                }
            }

            notes.sort_by(|a, b| {
                a.position_ms
                    .partial_cmp(&b.position_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            if !notes.is_empty() || tracks.is_empty() {
                let track_idx = tracks.len();
                tracks.push(UTrack {
                    name: track_name,
                    singer: "Default Singer".to_string(),
                    voicebank_path: None,
                    phonemizer: None,
                    volume_db: 0.0,
                    pan: 0.0,
                    mute: false,
                    solo: false,
                    ..UTrack::default()
                });

                parts.push(UVoicePart {
                    name: format!("Part {}", track_idx + 1),
                    track_index: track_idx,
                    position_ms: 0.0,
                    notes,
                });
            }
        }

        let mut project = UProject {
            name: "Vocaloid Project".to_string(),
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
        Ok(project)
    }
}

fn extract_cc_events(part_block: &str) -> Vec<VsqxCc> {
    let mut events = Vec::new();
    let mut search = 0;

    while let Some(cc_pos) = part_block[search..].find("<cc>") {
        let cc_start = search + cc_pos;
        let cc_end = if let Some(cc_close) = part_block[cc_start..].find("</cc>") {
            cc_start + cc_close + "</cc>".len()
        } else {
            break;
        };
        let cc_block = &part_block[cc_start..cc_end];
        search = cc_end;

        let pos_tick: i64 = extract_tag_value(cc_block, "posTick")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        if let Some(attr_start) = cc_block.find("<attr") {
            let attr_block = &cc_block[attr_start..];
            if let Some(id_start) = attr_block.find("id=\"") {
                let id_content_start = id_start + 4;
                if let Some(id_end) = attr_block[id_content_start..].find('"') {
                    let attr_id = &attr_block[id_content_start..id_content_start + id_end];
                    if let Some(val_start) = attr_block.find('>') {
                        let after_tag = &attr_block[val_start + 1..];
                        if let Some(val_end) = after_tag.find("</attr>") {
                            let val_str = after_tag[..val_end].trim();
                            if let Ok(value) = val_str.parse::<i64>() {
                                events.push(VsqxCc {
                                    pos_tick,
                                    attr_id: attr_id.to_string(),
                                    value,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    events
}

fn extract_tag_value(xml: &str, tag: &str) -> Option<String> {
    let open_tag = format!("<{}>", tag);
    let close_tag = format!("</{}>", tag);

    if let Some(start) = xml.find(&open_tag) {
        let content_start = start + open_tag.len();
        if let Some(end) = xml[content_start..].find(&close_tag) {
            let mut val = xml[content_start..content_start + end].trim();
            if val.starts_with("<![CDATA[") && val.ends_with("]]>") {
                val = &val[9..val.len() - 3];
            }
            return Some(val.to_string());
        }
    }
    None
}

impl VsqxFormat {
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
        let ms_per_tick = ms_per_beat / VSQX_DEFAULT_TICKS_PER_BEAT;
        let vsq_bpm = (bpm * 100.0).round() as i64;

        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"no\"?>\n");
        xml.push_str("<vsq4 xmlns=\"http://www.yamaha.co.jp/vocaloid/schema/vsq4/\">\n");
        xml.push_str("  <masterTrack>\n");
        xml.push_str("    <seqName><![CDATA[");
        xml.push_str(&project.name);
        xml.push_str("]]></seqName>\n");
        xml.push_str(&format!(
            "    <tempo><posTick>0</posTick><bpm>{}</bpm></tempo>\n",
            vsq_bpm
        ));
        xml.push_str("  </masterTrack>\n");

        for (track_idx, track) in project.tracks.iter().enumerate() {
            xml.push_str("  <vsTrack>\n");
            xml.push_str(&format!("    <tNo>{}</tNo>\n", track_idx));
            xml.push_str("    <name><![CDATA[");
            xml.push_str(&track.name);
            xml.push_str("]]></name>\n");

            xml.push_str("    <musicalPart>\n");
            xml.push_str("      <posTick>0</posTick>\n");
            xml.push_str("      <playTime>768000</playTime>\n");
            xml.push_str("      <partName><![CDATA[Part]]></partName>\n");

            let mut all_pit_events: Vec<(i64, i64)> = Vec::new();
            let mut max_pbs_semitones: f64 = 2.0;

            for part in project.parts.iter().filter(|p| p.track_index == track_idx) {
                for (note_index, note) in part.notes.iter().enumerate() {
                    let note_start_ms = part.position_ms + note.position_ms;
                    let tick_pos = (note_start_ms / ms_per_tick).round() as i64;
                    let dur_tick =
                        (note.duration_ms.max(1.0) / ms_per_tick).round().max(1.0) as i64;
                    let note_num = note.midi_key();
                    let lyric = if note.lyric.trim().is_empty() {
                        "a"
                    } else {
                        note.lyric.trim()
                    };

                    xml.push_str("      <note>\n");
                    xml.push_str(&format!("        <posTick>{}</posTick>\n", tick_pos));
                    xml.push_str(&format!("        <durTick>{}</durTick>\n", dur_tick));
                    xml.push_str(&format!("        <noteNum>{}</noteNum>\n", note_num));
                    xml.push_str("        <velocity>64</velocity>\n");
                    xml.push_str(&format!("        <lyric><![CDATA[{}]]></lyric>\n", lyric));
                    xml.push_str("      </note>\n");

                    let previous = note_index
                        .checked_sub(1)
                        .and_then(|index| part.notes.get(index));
                    let adjacent = previous.is_some_and(|previous| {
                        (previous.position_ms + previous.duration_ms - note.position_ms).abs()
                            <= 1.0
                    });
                    let pitch_points = note.pitch_bend.effective_points(
                        previous.map(|n| n.midi_key()),
                        note.midi_key(),
                        adjacent,
                    );

                    for pt in &pitch_points {
                        let abs_cents = pt.pitch_offset_cents.abs();
                        let needed_pbs = (abs_cents / 100.0).ceil().max(1.0);
                        if needed_pbs > max_pbs_semitones {
                            max_pbs_semitones = needed_pbs;
                        }
                    }

                    for pt in &pitch_points {
                        let pt_tick = tick_pos + (pt.time_offset_ms / ms_per_tick).round() as i64;
                        all_pit_events.push((pt_tick, pt.pitch_offset_cents as i64));
                    }
                }
            }

            let pbs_int = max_pbs_semitones.ceil().clamp(1.0, 24.0) as i64;
            if !all_pit_events.is_empty() {
                xml.push_str(&format!(
                    "      <cc><posTick>0</posTick><attr id=\"PBS\">{}</attr></cc>\n",
                    pbs_int
                ));

                all_pit_events.sort_by_key(|(tick, _)| *tick);
                for (pit_tick, cents_i) in &all_pit_events {
                    let pit_val = if pbs_int > 0 {
                        ((*cents_i as f64 / (pbs_int as f64 * 100.0)) * 8192.0).round() as i64
                    } else {
                        0
                    };
                    let pit_clamped = pit_val.clamp(-8192, 8191);
                    xml.push_str(&format!(
                        "      <cc><posTick>{}</posTick><attr id=\"PIT\">{}</attr></cc>\n",
                        pit_tick, pit_clamped
                    ));
                }
            }

            xml.push_str("    </musicalPart>\n");
            xml.push_str("  </vsTrack>\n");
        }

        xml.push_str("</vsq4>\n");
        fs::write(path, xml)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vsqx_parsing() {
        let sample_vsqx = r#"<?xml version="1.0" encoding="UTF-8"?>
<vsq4 xmlns="http://www.yamaha.co.jp/vocaloid/schema/vsq4/">
  <masterTrack>
    <tempo><posTick>0</posTick><bpm>14000</bpm></tempo>
  </masterTrack>
  <vsTrack>
    <trackName><![CDATA[Vocal 1]]></trackName>
    <musicalPart>
      <posTick>0</posTick>
      <note>
        <posTick>0</posTick>
        <durTick>480</durTick>
        <noteNum>60</noteNum>
        <lyric><![CDATA[ka]]></lyric>
      </note>
      <note>
        <posTick>480</posTick>
        <durTick>480</durTick>
        <noteNum>62</noteNum>
        <lyric><![CDATA[ma]]></lyric>
      </note>
    </musicalPart>
  </vsTrack>
</vsq4>"#;

        let proj = VsqxFormat::parse_str(sample_vsqx).unwrap();
        assert_eq!(proj.bpm, 140.0);
        assert_eq!(proj.tracks[0].name, "Vocal 1");
        assert_eq!(proj.parts[0].notes.len(), 2);
        assert_eq!(proj.parts[0].notes[0].lyric, "ka");
        assert_eq!(proj.parts[0].notes[0].pitch, "C4");
        assert_eq!(proj.parts[0].notes[1].lyric, "ma");
        assert_eq!(proj.parts[0].notes[1].pitch, "D4");
    }

    #[test]
    fn test_vsqx_pitchbend_import() {
        let vsqx_with_pitch = r#"<?xml version="1.0" encoding="UTF-8"?>
<vsq4 xmlns="http://www.yamaha.co.jp/vocaloid/schema/vsq4/">
  <masterTrack>
    <tempo><posTick>0</posTick><bpm>12000</bpm></tempo>
  </masterTrack>
  <vsTrack>
    <trackName><![CDATA[Track 1]]></trackName>
    <musicalPart>
      <posTick>0</posTick>
      <note>
        <posTick>0</posTick>
        <durTick>960</durTick>
        <noteNum>60</noteNum>
        <lyric><![CDATA[a]]></lyric>
      </note>
      <cc><posTick>0</posTick><attr id="PBS">12</attr></cc>
      <cc><posTick>0</posTick><attr id="PIT">0</attr></cc>
      <cc><posTick>120</posTick><attr id="PIT">4096</attr></cc>
      <cc><posTick>240</posTick><attr id="PIT">0</attr></cc>
    </musicalPart>
  </vsTrack>
</vsq4>"#;

        let proj = VsqxFormat::parse_str(vsqx_with_pitch).unwrap();
        assert_eq!(proj.bpm, 120.0);
        assert_eq!(proj.parts[0].notes.len(), 1);

        let note = &proj.parts[0].notes[0];
        assert_eq!(note.pitch, "C4");

        assert!(
            !note.pitch_bend.points.is_empty(),
            "Expected pitchbend points from PIT data, got none"
        );

        let has_600_cents = note
            .pitch_bend
            .points
            .iter()
            .any(|p| (p.pitch_offset_cents - 600.0).abs() < 10.0);
        assert!(
            has_600_cents,
            "Expected a point near 600 cents, points: {:?}",
            note.pitch_bend.points
        );
    }

    #[test]
    fn test_vsqx_pitchbend_roundtrip() {
        let mut project = UProject::default();
        let mut note = UNote::new("a", "C4", 0.0, 1000.0);
        note.pitch_bend.points = vec![
            UPitchBendPoint {
                time_offset_ms: -40.0,
                pitch_offset_cents: 0.0,
                shape: "s".to_string(),
            },
            UPitchBendPoint {
                time_offset_ms: 200.0,
                pitch_offset_cents: 300.0,
                shape: "s".to_string(),
            },
            UPitchBendPoint {
                time_offset_ms: 500.0,
                pitch_offset_cents: 0.0,
                shape: "s".to_string(),
            },
        ];
        project.parts[0].notes.push(note);

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("pitch_test.vsqx");
        VsqxFormat::save_file(&project, &path).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("PBS"), "Export should contain PBS CC");
        assert!(content.contains("PIT"), "Export should contain PIT CC");

        let parsed = VsqxFormat::load_file(&path).unwrap();
        assert!(!parsed.parts[0].notes[0].pitch_bend.points.is_empty());
    }
}
