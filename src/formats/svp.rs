use crate::dsp::pitch::{midi_to_note_name, note_name_to_midi};
use crate::project::model::{UNote, UProject, UTrack, UVoicePart};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub const SV_BLICKS_PER_BEAT: f64 = 705_600_000.0;

pub struct SvpFormat;

impl SvpFormat {
    pub fn load_file<P: AsRef<Path>>(path: P) -> Result<UProject, Box<dyn std::error::Error>> {
        let bytes = fs::read(path)?;
        Self::parse_bytes(&bytes)
    }

    pub fn parse_bytes(bytes: &[u8]) -> Result<UProject, Box<dyn std::error::Error>> {
        // Synthesizer V (.svp) files typically contain JSON text followed by a null byte '\0'
        // and binary metadata/cache. We must strip everything after the first null byte.
        let json_bytes = match bytes.iter().position(|&b| b == 0) {
            Some(idx) => &bytes[..idx],
            None => bytes,
        };

        // Strip UTF-8 BOM if present (EF BB BF)
        let clean_bytes = if json_bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
            &json_bytes[3..]
        } else {
            json_bytes
        };

        let content = String::from_utf8_lossy(clean_bytes);
        Self::parse_str(&content)
    }

    pub fn parse_str(content: &str) -> Result<UProject, Box<dyn std::error::Error>> {
        let trimmed = content.trim_start_matches('\u{feff}').trim();
        if trimmed.is_empty() {
            return Err("Arquivo SVP está vazio".into());
        }

        // Locate start of JSON object or array
        let start_pos = trimmed
            .find('{')
            .ok_or("Arquivo SVP não contém JSON válido")?;
        let json_slice = &trimmed[start_pos..];

        // Parse with serde_json::Value for maximum tolerance against schema variations
        let root: Value = serde_json::from_str(json_slice).or_else(|_| {
            // If there are trailing characters after the JSON object, extract balanced JSON object
            extract_balanced_json(json_slice)
                .and_then(|s| serde_json::from_str(&s).ok())
                .ok_or_else(|| {
                    std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        "Não foi possível interpretar o JSON do arquivo SVP",
                    )
                })
        })?;

        Self::parse_json_value(&root)
    }

    pub fn parse_json_value(root: &Value) -> Result<UProject, Box<dyn std::error::Error>> {
        // 1. Extract BPM from time.tempo array or root
        let mut bpm = 120.0;
        if let Some(tempos) = root
            .get("time")
            .and_then(|t| t.get("tempo"))
            .and_then(|t| t.as_array())
        {
            if let Some(first) = tempos.first() {
                if let Some(b) = first
                    .get("bpm")
                    .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
                {
                    if b.is_finite() && b > 0.0 {
                        bpm = b;
                    }
                }
            }
        } else if let Some(b) = root
            .get("bpm")
            .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|i| i as f64)))
        {
            if b.is_finite() && b > 0.0 {
                bpm = b;
            }
        }

        let ms_per_beat = 60000.0 / bpm;
        let ms_per_blick = ms_per_beat / SV_BLICKS_PER_BEAT;

        // 2. Index groups in the top-level "library" array by UUID and by name
        let mut library_groups: HashMap<String, &Value> = HashMap::new();
        if let Some(lib_arr) = root.get("library").and_then(|l| l.as_array()) {
            for group in lib_arr {
                if let Some(uuid) = group.get("uuid").and_then(|u| u.as_str()) {
                    library_groups.insert(uuid.to_string(), group);
                }
                if let Some(name) = group.get("name").and_then(|n| n.as_str()) {
                    library_groups.insert(name.to_string(), group);
                }
            }
        }

        let mut tracks = Vec::new();
        let mut parts = Vec::new();

        // 3. Extract tracks
        let raw_tracks = root
            .get("tracks")
            .and_then(|t| t.as_array())
            .cloned()
            .unwrap_or_default();

        for (track_idx, trk_val) in raw_tracks.iter().enumerate() {
            let track_name = trk_val
                .get("name")
                .or_else(|| trk_val.get("dispName"))
                .or_else(|| trk_val.get("title"))
                .and_then(|n| n.as_str())
                .unwrap_or(if track_idx == 0 { "Track 1" } else { "Track" })
                .to_string();

            tracks.push(UTrack {
                name: track_name.clone(),
                singer: "Default Singer".to_string(),
                volume_db: 0.0,
                pan: 0.0,
                mute: false,
                solo: false,
                ..UTrack::default()
            });

            let mut notes = Vec::new();

            // A. Check mainGroup / mainTrack
            let main_group = trk_val
                .get("mainGroup")
                .or_else(|| trk_val.get("mainTrack"));
            if let Some(mg) = main_group {
                collect_notes_from_group(mg, 0, ms_per_blick, &mut notes);
            }

            // B. Check groups array
            if let Some(groups_arr) = trk_val.get("groups").and_then(|g| g.as_array()) {
                for group_ref in groups_arr {
                    let offset_blicks = group_ref
                        .get("offset")
                        .or_else(|| group_ref.get("pos"))
                        .or_else(|| group_ref.get("position"))
                        .and_then(|o| o.as_i64().or_else(|| o.as_f64().map(|f| f as i64)))
                        .unwrap_or(0);

                    // group field can be a UUID string (referencing library) or an embedded group object
                    if let Some(group_uuid) =
                        group_ref.get("group").and_then(|g| g.as_str()).or_else(|| {
                            group_ref
                                .get("target")
                                .or_else(|| group_ref.get("uuid"))
                                .and_then(|u| u.as_str())
                        })
                    {
                        if let Some(lib_grp) = library_groups.get(group_uuid) {
                            collect_notes_from_group(
                                lib_grp,
                                offset_blicks,
                                ms_per_blick,
                                &mut notes,
                            );
                        }
                    } else if let Some(group_obj) =
                        group_ref.get("group").and_then(|g| g.as_object())
                    {
                        let grp_val = Value::Object(group_obj.clone());
                        collect_notes_from_group(
                            &grp_val,
                            offset_blicks,
                            ms_per_blick,
                            &mut notes,
                        );
                    }
                }
            }

            // C. Direct notes in track (if any)
            if let Some(direct_notes) = trk_val.get("notes") {
                let dummy_grp = serde_json::json!({ "notes": direct_notes });
                collect_notes_from_group(&dummy_grp, 0, ms_per_blick, &mut notes);
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

        // If no tracks found, attempt to check library groups directly or root notes
        if tracks.is_empty() {
            let mut notes = Vec::new();
            for grp in library_groups.values() {
                collect_notes_from_group(grp, 0, ms_per_blick, &mut notes);
            }
            if notes.is_empty() {
                collect_notes_from_group(root, 0, ms_per_blick, &mut notes);
            }
            notes.sort_by(|a, b| {
                a.position_ms
                    .partial_cmp(&b.position_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            tracks.push(UTrack {
                name: "Track 1".to_string(),
                singer: "Default Singer".to_string(),
                ..UTrack::default()
            });
            parts.push(UVoicePart {
                name: "Part 1".to_string(),
                track_index: 0,
                position_ms: 0.0,
                notes,
            });
        }

        let mut project = UProject {
            name: root
                .get("name")
                .or_else(|| root.get("title"))
                .and_then(|s| s.as_str())
                .unwrap_or("Synthesizer V Project")
                .to_string(),
            bpm,
            voicebank: None,
            voicebank_path: None,
            phonemizer: None,
            tracks,
            parts,
            ..UProject::default()
        };

        project.normalize();
        Ok(project)
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
                    svp_notes.push(serde_json::json!({
                        "onset": onset,
                        "duration": duration,
                        "pitch": pitch,
                        "lyrics": lyrics,
                        "phonemes": ""
                    }));
                }
            }
            svp_tracks.push(serde_json::json!({
                "name": track.name.clone(),
                "mainGroup": {
                    "notes": svp_notes
                },
                "groups": []
            }));
        }

        let svp_json = serde_json::json!({
            "version": 100,
            "time": {
                "meter": [{ "pos": 0, "num": 4, "den": 4 }],
                "tempo": [{ "pos": 0, "bpm": bpm }]
            },
            "library": [],
            "tracks": svp_tracks
        });

        let json_str = serde_json::to_string_pretty(&svp_json)?;
        fs::write(path, json_str)?;
        Ok(())
    }
}

fn collect_notes_from_group(
    group_val: &Value,
    offset_blicks: i64,
    ms_per_blick: f64,
    notes_out: &mut Vec<UNote>,
) {
    let raw_notes = group_val
        .get("notes")
        .or_else(|| group_val.get("note"))
        .and_then(|n| n.as_array());

    if let Some(notes_arr) = raw_notes {
        for n_val in notes_arr {
            let onset_blicks = n_val
                .get("onset")
                .or_else(|| n_val.get("pos"))
                .or_else(|| n_val.get("position"))
                .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
                .unwrap_or(0);

            let dur_blicks = n_val
                .get("duration")
                .or_else(|| n_val.get("dur"))
                .or_else(|| n_val.get("length"))
                .and_then(|v| v.as_i64().or_else(|| v.as_f64().map(|f| f as i64)))
                .unwrap_or(SV_BLICKS_PER_BEAT as i64);

            let pitch_val = n_val
                .get("pitch")
                .or_else(|| n_val.get("key"))
                .or_else(|| n_val.get("pitchName"));

            let midi_key = parse_pitch_value(pitch_val).unwrap_or(60);
            let pitch_name = midi_to_note_name(midi_key);

            let lyric = n_val
                .get("lyrics")
                .or_else(|| n_val.get("lyric"))
                .or_else(|| n_val.get("text"))
                .or_else(|| n_val.get("word"))
                .and_then(|s| s.as_str())
                .unwrap_or("la")
                .to_string();

            let lyric_clean = if lyric.trim().is_empty() {
                "la".to_string()
            } else {
                lyric
            };

            let total_onset = onset_blicks + offset_blicks;
            let pos_ms = (total_onset as f64 * ms_per_blick).max(0.0);
            let dur_ms = (dur_blicks as f64 * ms_per_blick).max(10.0);

            notes_out.push(UNote::new(lyric_clean, pitch_name, pos_ms, dur_ms));
        }
    }
}

fn parse_pitch_value(val: Option<&Value>) -> Option<u8> {
    let val = val?;
    if let Some(n) = val.as_i64() {
        return Some(n.clamp(0, 127) as u8);
    }
    if let Some(f) = val.as_f64() {
        return Some(f.round().clamp(0.0, 127.0) as u8);
    }
    if let Some(s) = val.as_str() {
        if let Ok(num) = s.trim().parse::<u8>() {
            return Some(num.clamp(0, 127));
        }
        if let Some(midi) = note_name_to_midi(s) {
            return Some(midi);
        }
    }
    None
}

/// Helper to extract a balanced `{ ... }` JSON string in case of trailing junk/binary after the JSON
fn extract_balanced_json(s: &str) -> Option<String> {
    let start = s.find('{')?;
    let mut depth = 0;
    let mut in_str = false;
    let mut escape = false;

    for (i, c) in s[start..].char_indices() {
        if escape {
            escape = false;
            continue;
        }
        match c {
            '\\' if in_str => escape = true,
            '"' => in_str = !in_str,
            '{' if !in_str => depth += 1,
            '}' if !in_str => {
                depth -= 1;
                if depth == 0 {
                    let end = start + i + 1;
                    return Some(s[start..end].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svp_with_library_groups_and_null_terminator() {
        let svp_json_with_binary = r#"{
            "version": 102,
            "time": {
                "meter": [{ "pos": 0, "num": 4, "den": 4 }],
                "tempo": [{ "pos": 0, "bpm": 130.0 }]
            },
            "library": [
                {
                    "name": "Main Chorus",
                    "uuid": "chorus-uuid-123",
                    "notes": [
                        { "onset": 0, "duration": 705600000, "pitch": 60, "lyrics": "do" },
                        { "onset": 705600000, "duration": 705600000, "pitch": 62, "lyrics": "re" }
                    ]
                }
            ],
            "tracks": [
                {
                    "name": "Lead Vocal",
                    "mainGroup": { "notes": [] },
                    "groups": [
                        { "group": "chorus-uuid-123", "offset": 1411200000 }
                    ]
                }
            ]
        }"#;

        // Simulate Synthesizer V null terminator + trailing binary audio cache
        let mut raw_bytes = svp_json_with_binary.as_bytes().to_vec();
        raw_bytes.push(0x00);
        raw_bytes.extend_from_slice(b"\xDE\xAD\xBE\xEF\x00\x01\x02\x03JUNK_AUDIO_CACHE");

        let project = SvpFormat::parse_bytes(&raw_bytes).unwrap();
        assert_eq!(project.bpm, 130.0);
        assert_eq!(project.tracks.len(), 1);
        assert_eq!(project.tracks[0].name, "Lead Vocal");
        assert_eq!(project.parts[0].notes.len(), 2);
        assert_eq!(project.parts[0].notes[0].lyric, "do");
        assert_eq!(project.parts[0].notes[0].pitch, "C4");
        assert_eq!(project.parts[0].notes[1].lyric, "re");
        assert_eq!(project.parts[0].notes[1].pitch, "D4");
    }

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
