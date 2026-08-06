use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::Path;

use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

use crate::dsp::pitch::midi_to_note_name;
use crate::project::model::{UNote, UProject, UTrack, UVoicePart};

pub struct MidiFormat;

#[derive(Debug, Clone, Copy)]
struct TempoPoint {
    tick: u64,
    microseconds_per_beat: u32,
}

impl MidiFormat {
    pub fn load_file<P: AsRef<Path>>(path: P) -> Result<UProject, Box<dyn std::error::Error>> {
        Self::parse_bytes(&fs::read(path)?)
    }

    pub fn parse_bytes(bytes: &[u8]) -> Result<UProject, Box<dyn std::error::Error>> {
        let smf = Smf::parse(bytes)?;
        let ticks_per_beat = match smf.header.timing {
            Timing::Metrical(ticks) => u64::from(ticks.as_int()),
            Timing::Timecode(_, _) => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "MIDI com divisão SMPTE ainda não é suportado",
                )
                .into())
            }
        };

        let tempo_map = Self::tempo_map(&smf);
        let initial_bpm = 60_000_000.0 / f64::from(tempo_map[0].microseconds_per_beat);
        let mut project = UProject {
            name: "MIDI Import".to_string(),
            bpm: initial_bpm,
            tracks: Vec::new(),
            parts: Vec::new(),
        };

        for (source_track_index, events) in smf.tracks.iter().enumerate() {
            let mut absolute_tick = 0u64;
            let mut track_name = format!("MIDI Track {}", source_track_index + 1);
            let mut active: HashMap<(u8, u8), VecDeque<(u64, u8)>> = HashMap::new();
            let mut notes = Vec::new();

            for event in events {
                absolute_tick += u64::from(event.delta.as_int());
                match event.kind {
                    TrackEventKind::Meta(MetaMessage::TrackName(name)) => {
                        track_name = String::from_utf8_lossy(name).into_owned();
                    }
                    TrackEventKind::Midi { channel, message } => match message {
                        MidiMessage::NoteOn { key, vel } if vel.as_int() > 0 => {
                            active
                                .entry((channel.as_int(), key.as_int()))
                                .or_default()
                                .push_back((absolute_tick, vel.as_int()));
                        }
                        MidiMessage::NoteOff { key, .. } | MidiMessage::NoteOn { key, vel: _ } => {
                            let identity = (channel.as_int(), key.as_int());
                            if let Some(queue) = active.get_mut(&identity) {
                                if let Some((start_tick, velocity)) = queue.pop_front() {
                                    let start_ms =
                                        Self::tick_to_ms(start_tick, ticks_per_beat, &tempo_map);
                                    let end_ms =
                                        Self::tick_to_ms(absolute_tick, ticks_per_beat, &tempo_map);
                                    let mut note = UNote::new(
                                        "ka",
                                        midi_to_note_name(key.as_int()),
                                        start_ms,
                                        (end_ms - start_ms).max(1.0),
                                    );
                                    note.expressions.velocity = f64::from(velocity) / 127.0 * 100.0;
                                    notes.push(note);
                                }
                                if queue.is_empty() {
                                    active.remove(&identity);
                                }
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }

            if notes.is_empty() {
                continue;
            }

            notes.sort_by(|left, right| left.position_ms.total_cmp(&right.position_ms));
            let target_track_index = project.tracks.len();
            project.tracks.push(UTrack {
                name: track_name.clone(),
                ..UTrack::default()
            });
            let mut part = UVoicePart::new(track_name, target_track_index);
            part.notes = notes;
            project.parts.push(part);
        }

        if project.tracks.is_empty() {
            project.tracks.push(UTrack::default());
            project.parts.push(UVoicePart::new("MIDI Import", 0));
        }

        Ok(project)
    }

    pub fn save_file<P: AsRef<Path>>(
        project: &UProject,
        path: P,
    ) -> Result<(), Box<dyn std::error::Error>> {
        fs::write(path, Self::to_midi_bytes(project)?)?;
        Ok(())
    }

    pub fn to_midi_bytes(project: &UProject) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let ticks_per_beat = 480u16;
        let bpm = project.bpm.clamp(20.0, 999.0);
        let microseconds_per_beat = (60_000_000.0 / bpm).round() as u32;

        let tempo_track = vec![
            TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Meta(MetaMessage::Tempo(microseconds_per_beat.into())),
            },
            TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
            },
        ];

        let mut midi_tracks = vec![tempo_track];
        for track_index in 0..project.tracks.len() {
            let mut absolute_events = Vec::new();
            for part in project
                .parts
                .iter()
                .filter(|part| part.track_index == track_index)
            {
                for note in &part.notes {
                    let start_ms = part.position_ms + note.position_ms;
                    let end_ms = start_ms + note.duration_ms.max(1.0);
                    let start_tick = Self::ms_to_tick(start_ms, bpm, ticks_per_beat);
                    let end_tick =
                        Self::ms_to_tick(end_ms, bpm, ticks_per_beat).max(start_tick + 1);
                    let key = note.midi_key().clamp(0, 127);
                    let velocity = ((note.expressions.velocity.clamp(0.0, 100.0) / 100.0) * 127.0)
                        .round()
                        .clamp(1.0, 127.0) as u8;

                    absolute_events.push((start_tick, 1u8, true, key, velocity));
                    absolute_events.push((end_tick, 0u8, false, key, 0));
                }
            }

            absolute_events.sort_by_key(|(tick, priority, _, key, _)| (*tick, *priority, *key));
            let mut events = Vec::with_capacity(absolute_events.len() + 1);
            let mut previous_tick = 0u64;
            for (tick, _, is_note_on, key, velocity) in absolute_events {
                let delta = tick.saturating_sub(previous_tick).min(0x0fff_ffff) as u32;
                let message = if is_note_on {
                    MidiMessage::NoteOn {
                        key: key.into(),
                        vel: velocity.into(),
                    }
                } else {
                    MidiMessage::NoteOff {
                        key: key.into(),
                        vel: 0.into(),
                    }
                };
                events.push(TrackEvent {
                    delta: delta.into(),
                    kind: TrackEventKind::Midi {
                        channel: 0.into(),
                        message,
                    },
                });
                previous_tick = tick;
            }
            events.push(TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
            });
            midi_tracks.push(events);
        }

        let smf = Smf {
            header: Header::new(Format::Parallel, Timing::Metrical(ticks_per_beat.into())),
            tracks: midi_tracks,
        };
        let mut bytes = Vec::new();
        smf.write(&mut bytes)?;
        Ok(bytes)
    }

    fn tempo_map(smf: &Smf<'_>) -> Vec<TempoPoint> {
        let mut points = vec![TempoPoint {
            tick: 0,
            microseconds_per_beat: 500_000,
        }];
        for track in &smf.tracks {
            let mut absolute_tick = 0u64;
            for event in track {
                absolute_tick += u64::from(event.delta.as_int());
                if let TrackEventKind::Meta(MetaMessage::Tempo(tempo)) = event.kind {
                    points.push(TempoPoint {
                        tick: absolute_tick,
                        microseconds_per_beat: tempo.as_int(),
                    });
                }
            }
        }
        points.sort_by_key(|point| point.tick);
        points.dedup_by(|left, right| {
            if left.tick == right.tick {
                left.microseconds_per_beat = right.microseconds_per_beat;
                true
            } else {
                false
            }
        });
        points
    }

    fn tick_to_ms(tick: u64, ticks_per_beat: u64, tempo_map: &[TempoPoint]) -> f64 {
        let mut elapsed_microseconds = 0.0;
        let mut segment_start = 0u64;
        let mut tempo = 500_000u32;

        for point in tempo_map {
            if point.tick > tick {
                break;
            }
            elapsed_microseconds += (point.tick.saturating_sub(segment_start)) as f64
                * f64::from(tempo)
                / ticks_per_beat as f64;
            segment_start = point.tick;
            tempo = point.microseconds_per_beat;
        }
        elapsed_microseconds +=
            (tick.saturating_sub(segment_start)) as f64 * f64::from(tempo) / ticks_per_beat as f64;
        elapsed_microseconds / 1000.0
    }

    fn ms_to_tick(milliseconds: f64, bpm: f64, ticks_per_beat: u16) -> u64 {
        ((milliseconds.max(0.0) / 1000.0) * (bpm / 60.0) * f64::from(ticks_per_beat)).round() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn midi_roundtrip_preserves_overlapping_notes() {
        let mut project = UProject {
            bpm: 120.0,
            ..UProject::default()
        };
        let mut part = UVoicePart::new("Test", 0);
        part.notes.push(UNote::new("ka", "C4", 0.0, 1000.0));
        part.notes.push(UNote::new("ki", "E4", 500.0, 1000.0));
        project.parts = vec![part];

        let parsed =
            MidiFormat::parse_bytes(&MidiFormat::to_midi_bytes(&project).unwrap()).unwrap();
        assert_eq!(parsed.parts[0].notes.len(), 2);
        assert!((parsed.parts[0].notes[0].position_ms - 0.0).abs() < 1.0);
        assert!((parsed.parts[0].notes[1].position_ms - 500.0).abs() < 1.0);
        assert!((parsed.parts[0].notes[0].duration_ms - 1000.0).abs() < 1.0);
    }

    #[test]
    fn tempo_map_integrates_tempo_changes() {
        let map = [
            TempoPoint {
                tick: 0,
                microseconds_per_beat: 500_000,
            },
            TempoPoint {
                tick: 480,
                microseconds_per_beat: 1_000_000,
            },
        ];
        assert!((MidiFormat::tick_to_ms(960, 480, &map) - 1500.0).abs() < 0.001);
    }

    #[test]
    fn import_applies_tempo_changes_from_separate_track() {
        let tempo_track = vec![
            TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Meta(MetaMessage::Tempo(500_000.into())),
            },
            TrackEvent {
                delta: 480.into(),
                kind: TrackEventKind::Meta(MetaMessage::Tempo(1_000_000.into())),
            },
        ];
        let note_track = vec![
            TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Midi {
                    channel: 0.into(),
                    message: MidiMessage::NoteOn {
                        key: 60.into(),
                        vel: 100.into(),
                    },
                },
            },
            TrackEvent {
                delta: 960.into(),
                kind: TrackEventKind::Midi {
                    channel: 0.into(),
                    message: MidiMessage::NoteOff {
                        key: 60.into(),
                        vel: 0.into(),
                    },
                },
            },
        ];
        let smf = Smf {
            header: Header::new(Format::Parallel, Timing::Metrical(480.into())),
            tracks: vec![tempo_track, note_track],
        };
        let mut bytes = Vec::new();
        smf.write(&mut bytes).unwrap();

        let project = MidiFormat::parse_bytes(&bytes).unwrap();
        assert!((project.parts[0].notes[0].duration_ms - 1500.0).abs() < 0.001);
    }
}
