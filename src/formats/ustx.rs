use crate::dsp::pitch::midi_to_note_name;
use crate::dsp::pitch_bend::PitchBendSolver;
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

        if let Ok(mut project) = yaml_serde::from_str::<UProject>(clean_content)
            .or_else(|_| serde_json::from_str::<UProject>(clean_content))
        {
            project.normalize();
            return Ok(project);
        }
        if let Ok(notes) = serde_json::from_str::<Vec<UNote>>(clean_content) {
            let mut project = UProject::default();
            project.parts[0].notes = notes;
            return Ok(project);
        }

        if let Ok(val) = yaml_serde::from_str::<yaml_serde::Value>(clean_content) {
            if let Some(mut proj) = Self::parse_from_yaml_value(&val) {
                proj.normalize();
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
        if !bpm.is_finite() || !(20.0..=999.0).contains(&bpm) {
            bpm = 120.0;
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
                    .map(|(index, value)| {
                        let phonemizer = value
                            .get("phonemizer")
                            .and_then(|item| item.as_str())
                            .and_then(Self::openutau_name_to_phonemizer_mode);

                        let renderer_settings = value.get("renderer_settings");
                        let resampler = renderer_settings
                            .and_then(|rs| rs.get("resampler"))
                            .and_then(|r| r.as_str())
                            .map(str::to_owned)
                            .or_else(|| {
                                value
                                    .get("resampler")
                                    .and_then(|r| r.as_str())
                                    .map(str::to_owned)
                            });
                        let wavtool = renderer_settings
                            .and_then(|rs| rs.get("wavtool"))
                            .and_then(|w| w.as_str())
                            .map(str::to_owned)
                            .or_else(|| {
                                value
                                    .get("wavtool")
                                    .and_then(|w| w.as_str())
                                    .map(str::to_owned)
                            });

                        UTrack {
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
                            phonemizer,
                            resampler,
                            wavtool,
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
                            ..UTrack::default()
                        }
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

                let mut part_pitd_events: Vec<(i64, f64)> = Vec::new();
                if let Some(curves_seq) = part_val.get("curves").and_then(|v| v.as_sequence()) {
                    for curve_val in curves_seq {
                        let abbr = curve_val.get("abbr").and_then(|v| v.as_str()).unwrap_or("");
                        if abbr == "pitd" || abbr == "pit" || abbr == "pitch" {
                            let xs = curve_val.get("xs").and_then(|v| v.as_sequence());
                            let ys = curve_val.get("ys").and_then(|v| v.as_sequence());
                            if let (Some(xs_arr), Some(ys_arr)) = (xs, ys) {
                                for (x_item, y_item) in xs_arr.iter().zip(ys_arr.iter()) {
                                    let x_tick = x_item
                                        .as_i64()
                                        .or_else(|| x_item.as_f64().map(|f| f as i64));
                                    let y_cent = y_item
                                        .as_f64()
                                        .or_else(|| y_item.as_i64().map(|i| i as f64));
                                    if let (Some(x), Some(y)) = (x_tick, y_cent) {
                                        part_pitd_events.push((x, y));
                                    }
                                }
                            }
                        }
                    }
                }
                part_pitd_events.sort_by_key(|&(x, _)| x);

                if let Some(notes_seq) = part_val.get("notes").and_then(|v| v.as_sequence()) {
                    for note_val in notes_seq {
                        let lyric = note_val.get("lyric").and_then(|v| v.as_str()).unwrap_or("");

                        if lyric.is_empty() || lyric == "R" || lyric == "r" {
                            continue;
                        }

                        let tone = note_val
                            .get("tone")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(60)
                            .clamp(0, 127) as u8;

                        let pos_ticks = note_val
                            .get("position")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);

                        let dur_ticks = note_val
                            .get("duration")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(480);

                        let pitch_str = midi_to_note_name(tone);
                        let pos_ms = pos_ticks as f64 * ms_per_tick;
                        let dur_ms = (dur_ticks as f64 * ms_per_tick).max(20.0);

                        let mut u_note = UNote::new(lyric, pitch_str, pos_ms, dur_ms);

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
                                        "vol" => u_note.expressions.volume = val_num,
                                        "atk" => u_note.expressions.attack = val_num,
                                        "dec" => u_note.expressions.decay = val_num,
                                        "mod" => u_note.expressions.modulation = val_num,
                                        _ => {}
                                    }
                                }
                            }
                        }

                        if let Some(vibrato) = note_val.get("vibrato") {
                            let number = |key: &str| {
                                vibrato.get(key).and_then(|value| {
                                    value.as_f64().or_else(|| value.as_i64().map(|n| n as f64))
                                })
                            };
                            u_note.vibrato.length_pct = number("length").unwrap_or(0.0);
                            u_note.vibrato.period_ms = number("period").unwrap_or(175.0);
                            u_note.vibrato.depth_cents = number("depth").unwrap_or(35.0);
                            u_note.vibrato.fade_in_pct = number("in").unwrap_or(20.0);
                            u_note.vibrato.fade_out_pct = number("out").unwrap_or(20.0);
                            u_note.vibrato.shift_pct = number("shift").unwrap_or(0.0);
                            u_note.vibrato.drift_pct = number("drift").unwrap_or(0.0);
                            u_note.vibrato.volume_link_pct = number("volLink")
                                .or_else(|| number("vol_link"))
                                .unwrap_or(0.0);
                        }

                        let mut note_pitch_points = Vec::new();
                        let mut snap_first = true;

                        if let Some(pitch_obj) = note_val.get("pitch") {
                            if let Some(snap) = pitch_obj
                                .get("snap_first")
                                .or_else(|| pitch_obj.get("snapFirst"))
                                .and_then(|value| value.as_bool())
                            {
                                snap_first = snap;
                            }

                            if let Some(pitch_data) = pitch_obj
                                .get("data")
                                .or_else(|| pitch_obj.get("Data"))
                                .and_then(|d| d.as_sequence())
                            {
                                for pt_val in pitch_data {
                                    let px = pt_val
                                        .get("x")
                                        .or_else(|| pt_val.get("X"))
                                        .and_then(|v| {
                                            v.as_f64().or_else(|| v.as_i64().map(|i| i as f64))
                                        })
                                        .unwrap_or(0.0);
                                    let py = pt_val
                                        .get("y")
                                        .or_else(|| pt_val.get("Y"))
                                        .and_then(|v| {
                                            v.as_f64().or_else(|| v.as_i64().map(|i| i as f64))
                                        })
                                        .unwrap_or(0.0);
                                    let pshape = pt_val
                                        .get("shape")
                                        .or_else(|| pt_val.get("Shape"))
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("s")
                                        .to_string();

                                    note_pitch_points.push(UPitchBendPoint {
                                        time_offset_ms: px,
                                        pitch_offset_cents: py * 10.0,
                                        shape: pshape,
                                    });
                                }
                            }
                        }

                        let note_start_tick = pos_ticks;
                        let note_end_tick = pos_ticks + dur_ticks;
                        let mut curve_points_for_note = Vec::new();
                        for &(x_tick, y_cent) in &part_pitd_events {
                            if x_tick >= note_start_tick - 120 && x_tick <= note_end_tick {
                                let rel_ms = (x_tick - note_start_tick) as f64 * ms_per_tick;
                                curve_points_for_note.push(UPitchBendPoint {
                                    time_offset_ms: rel_ms,
                                    pitch_offset_cents: y_cent,
                                    shape: "s".to_string(),
                                });
                            }
                        }

                        let final_points = if !curve_points_for_note.is_empty() {
                            if !note_pitch_points.is_empty() {
                                let mut combined = Vec::new();
                                for cp in &curve_points_for_note {
                                    let base_pitch = PitchBendSolver::get_pitch_offset_cents(
                                        cp.time_offset_ms,
                                        &note_pitch_points,
                                    );
                                    combined.push(UPitchBendPoint {
                                        time_offset_ms: cp.time_offset_ms,
                                        pitch_offset_cents: base_pitch + cp.pitch_offset_cents,
                                        shape: "s".to_string(),
                                    });
                                }
                                for np in &note_pitch_points {
                                    if np.time_offset_ms
                                        < curve_points_for_note
                                            .first()
                                            .map(|p| p.time_offset_ms)
                                            .unwrap_or(0.0)
                                        || np.time_offset_ms
                                            > curve_points_for_note
                                                .last()
                                                .map(|p| p.time_offset_ms)
                                                .unwrap_or(0.0)
                                    {
                                        combined.push(np.clone());
                                    }
                                }
                                combined.sort_by(|a, b| {
                                    a.time_offset_ms
                                        .partial_cmp(&b.time_offset_ms)
                                        .unwrap_or(std::cmp::Ordering::Equal)
                                });
                                PitchBendSolver::simplify_pitch_points(&combined, 1.5)
                            } else {
                                PitchBendSolver::simplify_pitch_points(&curve_points_for_note, 1.5)
                            }
                        } else {
                            note_pitch_points
                        };

                        if !final_points.is_empty() {
                            let portamento_start_ms = final_points[0].time_offset_ms;
                            let portamento_length_ms = final_points
                                .get(1)
                                .map(|second| {
                                    second.time_offset_ms - final_points[0].time_offset_ms
                                })
                                .unwrap_or(80.0)
                                .max(1.0);
                            let portamento_shape = final_points[0].shape.clone();
                            u_note.pitch_bend = UPitchBend {
                                points: final_points,
                                snap_first,
                                portamento_start_ms,
                                portamento_length_ms,
                                portamento_shape,
                            };
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

        let root_phonemizer = root
            .get("phonemizer")
            .and_then(|item| item.as_str())
            .and_then(Self::openutau_name_to_phonemizer_mode)
            .or_else(|| tracks.first().and_then(|t| t.phonemizer));
        let root_resampler = root
            .get("resampler")
            .and_then(|item| item.as_str())
            .map(str::to_owned)
            .or_else(|| tracks.first().and_then(|t| t.resampler.clone()));
        let root_wavtool = root
            .get("wavtool")
            .and_then(|item| item.as_str())
            .map(str::to_owned)
            .or_else(|| tracks.first().and_then(|t| t.wavtool.clone()));

        Some(UProject {
            name,
            bpm,
            phonemizer: root_phonemizer,
            resampler: root_resampler,
            wavtool: root_wavtool,
            tracks,
            parts,
            ..UProject::default()
        })
    }

    pub fn phonemizer_mode_to_openutau_name(
        mode: crate::phonemizer::PhonemizerMode,
    ) -> &'static str {
        match mode {
            crate::phonemizer::PhonemizerMode::None => "OpenUtau.Core.DefaultPhonemizer",
            crate::phonemizer::PhonemizerMode::BasicCV => {
                "OpenUtau.Plugin.Builtin.JapaneseBasicPhonemizer"
            }
            crate::phonemizer::PhonemizerMode::VCV => {
                "OpenUtau.Plugin.Builtin.JapaneseVCVPhonemizer"
            }
            crate::phonemizer::PhonemizerMode::CVVC => {
                "OpenUtau.Plugin.Builtin.JapaneseCVVCPhonemizer"
            }
            crate::phonemizer::PhonemizerMode::EnglishArpasing => {
                "OpenUtau.Plugin.Builtin.EnArpasingPhonemizer"
            }
            crate::phonemizer::PhonemizerMode::EnglishVCCV => {
                "OpenUtau.Plugin.Builtin.EnVCCVPhonemizer"
            }
            crate::phonemizer::PhonemizerMode::EnglishG2P => {
                "OpenUtau.Plugin.Builtin.EnG2pPhonemizer"
            }
            crate::phonemizer::PhonemizerMode::PortugueseBrapaVCCV => {
                "OpenUtau.Plugin.Builtin.PortugueseBrapaVCCVPhonemizer"
            }
            crate::phonemizer::PhonemizerMode::PortugueseBrapaCVC => {
                "OpenUtau.Plugin.Builtin.PortugueseBrapaCVCPhonemizer"
            }
            crate::phonemizer::PhonemizerMode::PortugueseCVVC => {
                "OpenUtau.Plugin.Builtin.PortugueseCVVCPhonemizer"
            }
            crate::phonemizer::PhonemizerMode::PortugueseVCV => {
                "OpenUtau.Plugin.Builtin.PortugueseVCVPhonemizer"
            }
            crate::phonemizer::PhonemizerMode::PortugueseG2P => {
                "OpenUtau.Plugin.Builtin.PortugueseG2pPhonemizer"
            }
        }
    }

    pub fn openutau_name_to_phonemizer_mode(
        name: &str,
    ) -> Option<crate::phonemizer::PhonemizerMode> {
        let n = name.to_lowercase();
        if n.contains("japanesecvvc")
            || n.contains("ja cvvc")
            || (n.contains("cvvc") && n.contains("ja"))
        {
            Some(crate::phonemizer::PhonemizerMode::CVVC)
        } else if n.contains("japanesevcv")
            || n.contains("ja vcv")
            || (n.contains("vcv") && n.contains("ja"))
        {
            Some(crate::phonemizer::PhonemizerMode::VCV)
        } else if n.contains("japanesebasic")
            || n.contains("ja cv")
            || (n.contains("basic") && n.contains("ja"))
        {
            Some(crate::phonemizer::PhonemizerMode::BasicCV)
        } else if n.contains("portuguesebrapavccv")
            || n.contains("brapa vccv")
            || (n.contains("brapa") && n.contains("vccv"))
        {
            Some(crate::phonemizer::PhonemizerMode::PortugueseBrapaVCCV)
        } else if n.contains("portuguesebrapacvc") || n.contains("brapa cvc") || n.contains("brapa")
        {
            Some(crate::phonemizer::PhonemizerMode::PortugueseBrapaCVC)
        } else if n.contains("portuguesecvvc") || n.contains("pt cvvc") {
            Some(crate::phonemizer::PhonemizerMode::PortugueseCVVC)
        } else if n.contains("portuguesevcv") || n.contains("pt vcv") {
            Some(crate::phonemizer::PhonemizerMode::PortugueseVCV)
        } else if n.contains("portugueseg2p") || n.contains("pt g2p") {
            Some(crate::phonemizer::PhonemizerMode::PortugueseG2P)
        } else if n.contains("enarpasing") || n.contains("arpasing") || n.contains("arpa") {
            Some(crate::phonemizer::PhonemizerMode::EnglishArpasing)
        } else if n.contains("envccv") || n.contains("en vccv") {
            Some(crate::phonemizer::PhonemizerMode::EnglishVCCV)
        } else if n.contains("eng2p") || n.contains("en g2p") {
            Some(crate::phonemizer::PhonemizerMode::EnglishG2P)
        } else if n.contains("default") || n.contains("none") || n.contains("raw") {
            Some(crate::phonemizer::PhonemizerMode::None)
        } else {
            None
        }
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
                let phonemizer_str = track
                    .phonemizer
                    .or(project.phonemizer)
                    .map(Self::phonemizer_mode_to_openutau_name)
                    .unwrap_or("OpenUtau.Core.DefaultPhonemizer");
                let resampler_str = track
                    .resampler
                    .as_deref()
                    .or(project.resampler.as_deref())
                    .unwrap_or("straycat");
                let wavtool_str = track
                    .wavtool
                    .as_deref()
                    .or(project.wavtool.as_deref())
                    .unwrap_or("wavtool-yawu");

                serde_json::json!({
                    "track_name": track.name,
                    "singer": track.singer,
                    "phonemizer": phonemizer_str,
                    "renderer_settings": {
                        "renderer": "CLASSIC",
                        "resampler": resampler_str,
                        "wavtool": wavtool_str,
                    },
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
                    .enumerate()
                    .map(|(note_index, note)| {
                        let previous = note_index
                            .checked_sub(1)
                            .and_then(|index| part.notes.get(index));
                        let adjacent = previous.is_some_and(|previous| {
                            (previous.position_ms + previous.duration_ms - note.position_ms).abs()
                                <= 1.0
                        });
                        let pitch_points = note.pitch_bend.effective_points(
                            previous.map(UNote::midi_key),
                            note.midi_key(),
                            adjacent,
                        );
                        let pitch_data = pitch_points
                            .iter()
                            .map(|point| {
                                serde_json::json!({
                                    "x": point.time_offset_ms,
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
                            "pitch": {
                                "data": pitch_data,
                                "snap_first": note.pitch_bend.snap_first,
                            },
                            "vibrato": {
                                "length": note.vibrato.length_pct,
                                "period": note.vibrato.period_ms,
                                "depth": note.vibrato.depth_cents,
                                "in": note.vibrato.fade_in_pct,
                                "out": note.vibrato.fade_out_pct,
                                "shift": note.vibrato.shift_pct,
                                "drift": note.vibrato.drift_pct,
                                "volLink": note.vibrato.volume_link_pct,
                            },
                            "expressions": [
                                { "abbr": "vel", "value": note.expressions.consonant_velocity },
                                { "abbr": "dyn", "value": note.expressions.dynamics },
                                { "abbr": "bre", "value": note.expressions.breathiness },
                                { "abbr": "gen", "value": note.expressions.gender },
                                { "abbr": "pitd", "value": note.expressions.pitch_delta },
                                { "abbr": "vol", "value": note.expressions.volume },
                                { "abbr": "atk", "value": note.expressions.attack },
                                { "abbr": "dec", "value": note.expressions.decay },
                                { "abbr": "mod", "value": note.expressions.modulation },
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
        let root_phonemizer_str = project
            .phonemizer
            .map(Self::phonemizer_mode_to_openutau_name)
            .unwrap_or("OpenUtau.Core.DefaultPhonemizer");
        let document = serde_json::json!({
            "ustx_version": "0.6",
            "name": project.name,
            "bpm": project.bpm,
            "tempos": [{ "position": 0, "bpm": project.bpm }],
            "phonemizer": root_phonemizer_str,
            "resampler": project.resampler.as_deref().unwrap_or("straycat"),
            "wavtool": project.wavtool.as_deref().unwrap_or("wavtool-yawu"),
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
    fn test_ustx_phonemizer_and_resampler_roundtrip() {
        let proj = UProject {
            name: "Phonemizer Test".to_string(),
            bpm: 130.0,
            phonemizer: Some(crate::phonemizer::PhonemizerMode::CVVC),
            resampler: Some("straycat".to_string()),
            wavtool: Some("wavtool-yawu".to_string()),
            tracks: vec![UTrack {
                name: "Lead".to_string(),
                singer: "Kitsune".to_string(),
                phonemizer: Some(crate::phonemizer::PhonemizerMode::CVVC),
                resampler: Some("straycat".to_string()),
                wavtool: Some("wavtool-yawu".to_string()),
                ..UTrack::default()
            }],
            parts: vec![UVoicePart {
                name: "Part 1".to_string(),
                track_index: 0,
                position_ms: 0.0,
                notes: vec![UNote::new("ka", "C4", 0.0, 480.0)],
            }],
            ..UProject::default()
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join("test.ustx");

        UstxFormat::save_file(&proj, &path).unwrap();
        let loaded = UstxFormat::load_file(&path).unwrap();

        assert_eq!(loaded.name, "Phonemizer Test");
        assert_eq!(loaded.bpm, 130.0);
        assert_eq!(
            loaded.phonemizer,
            Some(crate::phonemizer::PhonemizerMode::CVVC)
        );
        assert_eq!(loaded.resampler.as_deref(), Some("straycat"));
        assert_eq!(loaded.wavtool.as_deref(), Some("wavtool-yawu"));
        assert_eq!(
            loaded.tracks[0].phonemizer,
            Some(crate::phonemizer::PhonemizerMode::CVVC)
        );
        assert_eq!(loaded.tracks[0].resampler.as_deref(), Some("straycat"));
        assert_eq!(loaded.tracks[0].wavtool.as_deref(), Some("wavtool-yawu"));
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
            ..UProject::default()
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
    fn openutau_vibrato_and_amplitude_expressions_roundtrip() {
        let mut project = UProject::default();
        let mut note = UNote::new("a", "A4", 0.0, 1000.0);
        note.vibrato.length_pct = 65.0;
        note.vibrato.fade_out_pct = 30.0;
        note.vibrato.volume_link_pct = -25.0;
        note.expressions.volume = 80.0;
        note.expressions.attack = 60.0;
        note.expressions.decay = 15.0;
        project.parts[0].notes.push(note);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("expressions.ustx");
        UstxFormat::save_file(&project, &path).unwrap();
        let parsed = UstxFormat::load_file(path).unwrap();
        let note = &parsed.parts[0].notes[0];
        assert_eq!(note.vibrato.length_pct, 65.0);
        assert_eq!(note.vibrato.fade_out_pct, 30.0);
        assert_eq!(note.vibrato.volume_link_pct, -25.0);
        assert_eq!(note.expressions.volume, 80.0);
        assert_eq!(note.expressions.attack, 60.0);
        assert_eq!(note.expressions.decay, 15.0);
    }

    #[test]
    fn openutau_pitch_point_x_is_milliseconds() {
        let source = r#"
name: Portamento
bpm: 120
tracks: [{name: Track}]
voice_parts:
  - track_no: 0
    position: 0
    notes:
      - position: 0
        duration: 480
        tone: 60
        lyric: a
        pitch:
          snap_first: true
          data:
            - {x: -25, y: 0, shape: io}
            - {x: 25, y: 0, shape: io}
"#;
        let project = UstxFormat::parse_str(source).unwrap();
        let pitch = &project.parts[0].notes[0].pitch_bend;
        assert_eq!(pitch.points[0].time_offset_ms, -25.0);
        assert_eq!(pitch.portamento_start_ms, -25.0);
        assert_eq!(pitch.portamento_length_ms, 50.0);
        assert!(pitch.snap_first);
    }

    #[test]
    fn openutau_part_level_pitd_curves_parse() {
        let source = r#"
name: Pitd Curves Test
bpm: 120
tracks: [{name: Track 1}]
voice_parts:
  - track_no: 0
    position: 0
    curves:
      - xs:
        - 0
        - 120
        - 240
        - 360
        - 480
        ys:
        - 0
        - 50
        - 100
        - 25
        - 0
        abbr: pitd
    notes:
      - position: 0
        duration: 480
        tone: 60
        lyric: la
"#;
        let project = UstxFormat::parse_str(source).unwrap();
        let note = &project.parts[0].notes[0];
        assert!(
            !note.pitch_bend.points.is_empty(),
            "Pitch bend points should not be empty"
        );
        assert_eq!(note.pitch_bend.points[0].time_offset_ms, 0.0);
        assert_eq!(note.pitch_bend.points[0].pitch_offset_cents, 0.0);
        let has_peak = note
            .pitch_bend
            .points
            .iter()
            .any(|p| (p.pitch_offset_cents - 100.0).abs() < 1.0);
        assert!(
            has_peak,
            "Should contain pitch bend peak point of ~100 cents"
        );
    }

    #[test]
    fn test_parse_real_ustx_files() {
        let files = [
            "/Users/victor/Downloads/m (1).ustx",
            "/Users/victor/Downloads/pururu UTAU_meperdeu_join.ustx",
            "/Users/victor/Downloads/happy mode.ustx",
            "/Users/victor/Downloads/talkloid-autosave.ustx",
            "/Users/victor/Downloads/astro.ustx",
        ];
        for f in files {
            if std::path::Path::new(f).exists() {
                let proj = UstxFormat::load_file(f)
                    .unwrap_or_else(|e| panic!("Failed to parse {}: {}", f, e));
                println!(
                    "Loaded {}: name={}, bpm={}, parts={}",
                    f,
                    proj.name,
                    proj.bpm,
                    proj.parts.len()
                );
                let total_pitch_points: usize = proj
                    .parts
                    .iter()
                    .flat_map(|p| &p.notes)
                    .map(|n| n.pitch_bend.points.len())
                    .sum();
                println!("  Total pitch bend points parsed: {}", total_pitch_points);
            }
        }
    }
}
