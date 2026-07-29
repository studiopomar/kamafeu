use std::fs;
use std::path::Path;
use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

use crate::dsp::pitch::midi_to_note_name;
use crate::project::model::{UNote, UProject, UVoicePart};

pub struct MidiFormat;

impl MidiFormat {
    /// Load and parse a Standard MIDI file (.mid / .midi) into a UProject
    pub fn load_file<P: AsRef<Path>>(path: P) -> Result<UProject, Box<dyn std::error::Error>> {
        let bytes = fs::read(path)?;
        Self::parse_bytes(&bytes)
    }

    /// Parse MIDI bytes into a UProject
    pub fn parse_bytes(bytes: &[u8]) -> Result<UProject, Box<dyn std::error::Error>> {
        let smf = Smf::parse(bytes)?;
        let mut project = UProject::default();
        let mut notes: Vec<UNote> = Vec::new();

        let ticks_per_beat = match smf.header.timing {
            Timing::Metrical(t) => t.as_int() as f64,
            _ => 480.0,
        };

        let mut bpm = 120.0f64;
        let mut active_notes: std::collections::HashMap<u8, (f64, u8)> = std::collections::HashMap::new();

        for track in &smf.tracks {
            let mut current_tick = 0u64;

            for event in track {
                current_tick += event.delta.as_int() as u64;
                let time_ms = (current_tick as f64 / ticks_per_beat) * (60000.0 / bpm);

                match event.kind {
                    TrackEventKind::Meta(MetaMessage::Tempo(tempo_micro)) => {
                        let micro_sec = tempo_micro.as_int() as f64;
                        if micro_sec > 0.0 {
                            bpm = 60_000_000.0 / micro_sec;
                            project.bpm = bpm;
                        }
                    }
                    TrackEventKind::Midi { message, .. } => match message {
                        MidiMessage::NoteOn { key, vel } => {
                            let key_u8 = key.as_int();
                            let vel_u8 = vel.as_int();
                            if vel_u8 > 0 {
                                active_notes.insert(key_u8, (time_ms, vel_u8));
                            } else if let Some((start_ms, _)) = active_notes.remove(&key_u8) {
                                let duration_ms = (time_ms - start_ms).max(50.0);
                                let note = UNote::new(
                                    "ka",
                                    midi_to_note_name(key_u8),
                                    start_ms,
                                    duration_ms,
                                );
                                notes.push(note);
                            }
                        }
                        MidiMessage::NoteOff { key, .. } => {
                            let key_u8 = key.as_int();
                            if let Some((start_ms, _)) = active_notes.remove(&key_u8) {
                                let duration_ms = (time_ms - start_ms).max(50.0);
                                let note = UNote::new(
                                    "ka",
                                    midi_to_note_name(key_u8),
                                    start_ms,
                                    duration_ms,
                                );
                                notes.push(note);
                            }
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }
        }

        notes.sort_by(|a, b| a.position_ms.partial_cmp(&b.position_ms).unwrap());

        if !notes.is_empty() {
            let mut part = UVoicePart::new("MIDI Import", 0);
            part.notes = notes;
            project.parts = vec![part];
        }

        Ok(project)
    }

    /// Export UProject notes to Standard MIDI file (.mid)
    pub fn save_file<P: AsRef<Path>>(project: &UProject, path: P) -> Result<(), Box<dyn std::error::Error>> {
        let midi_bytes = Self::to_midi_bytes(project)?;
        fs::write(path, midi_bytes)?;
        Ok(())
    }

    /// Serialize UProject into raw MIDI file bytes
    pub fn to_midi_bytes(project: &UProject) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let ticks_per_beat = 480u16;
        let bpm = project.bpm.max(20.0);

        let mut header = Header::new(Format::SingleTrack, Timing::Metrical(ticks_per_beat.into()));
        header.format = Format::SingleTrack;

        let mut track_events: Vec<TrackEvent> = Vec::new();

        // 1. Tempo Meta Event
        let micro_sec = (60_000_000.0 / bpm) as u32;

        track_events.push(TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(midly::num::u24::from(micro_sec))),
        });

        // 2. Note Events
        let notes = if !project.parts.is_empty() {
            &project.parts[0].notes
        } else {
            &Vec::new()
        };

        let mut last_tick = 0u64;

        for note in notes {
            let start_tick = ((note.position_ms / 1000.0) * (bpm / 60.0) * ticks_per_beat as f64) as u64;
            let dur_ticks = ((note.duration_ms / 1000.0) * (bpm / 60.0) * ticks_per_beat as f64) as u64;
            let end_tick = start_tick + dur_ticks.max(10);

            let midi_key = note.midi_key();

            let delta_start = (start_tick.saturating_sub(last_tick)) as u32;
            track_events.push(TrackEvent {
                delta: delta_start.into(),
                kind: TrackEventKind::Midi {
                    channel: 0.into(),
                    message: MidiMessage::NoteOn {
                        key: midi_key.into(),
                        vel: 100.into(),
                    },
                },
            });
            last_tick = start_tick;

            let delta_end = (end_tick.saturating_sub(last_tick)) as u32;
            track_events.push(TrackEvent {
                delta: delta_end.into(),
                kind: TrackEventKind::Midi {
                    channel: 0.into(),
                    message: MidiMessage::NoteOff {
                        key: midi_key.into(),
                        vel: 0.into(),
                    },
                },
            });
            last_tick = end_tick;
        }

        // End of track meta event
        track_events.push(TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        });

        let smf = Smf {
            header,
            tracks: vec![track_events],
        };

        let mut buffer = Vec::new();
        smf.write(&mut buffer)?;
        Ok(buffer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_midi_roundtrip() {
        let mut proj = UProject::default();
        proj.bpm = 128.0;
        let mut part = UVoicePart::new("Test", 0);
        part.notes.push(UNote::new("ka", "C4", 0.0, 500.0));
        part.notes.push(UNote::new("ki", "D4", 500.0, 500.0));
        proj.parts = vec![part];

        let midi_bytes = MidiFormat::to_midi_bytes(&proj).unwrap();
        assert!(!midi_bytes.is_empty());

        let parsed = MidiFormat::parse_bytes(&midi_bytes).unwrap();
        assert_eq!(parsed.parts[0].notes.len(), 2);
        assert_eq!(parsed.parts[0].notes[0].midi_key(), 60);
        assert_eq!(parsed.parts[0].notes[1].midi_key(), 62);
    }
}
