use serde::{Deserialize, Serialize};

/// Convert MIDI note number to frequency in Hertz
pub fn midi_to_freq(midi_note: f64) -> f64 {
    440.0 * 2.0f64.powf((midi_note - 69.0) / 12.0)
}

/// Convert musical note name (e.g., "C4", "C#4", "Db4", "A4") to MIDI key number
pub fn note_name_to_midi(name: &str) -> Option<u8> {
    let name = name.trim();
    if name.is_empty() {
        return None;
    }

    let (pitch_part, octave_part) = if name.len() >= 2 && matches!(name.as_bytes()[1], b'#' | b'b')
    {
        (&name[0..2], &name[2..])
    } else {
        (&name[0..1], &name[1..])
    };

    let base_note: i32 = match pitch_part.to_uppercase().as_str() {
        "C" => 0,
        "C#" | "DB" => 1,
        "D" => 2,
        "D#" | "EB" => 3,
        "E" => 4,
        "F" => 5,
        "F#" | "GB" => 6,
        "G" => 7,
        "G#" | "AB" => 8,
        "A" => 9,
        "A#" | "BB" => 10,
        "B" => 11,
        _ => return None,
    };

    let octave: i32 = octave_part.parse().ok()?;
    let midi = (octave + 1) * 12 + base_note;
    if (0..=127).contains(&midi) {
        Some(midi as u8)
    } else {
        None
    }
}

/// Convert MIDI note number to note name (e.g. 60 -> "C4", 69 -> "A4")
pub fn midi_to_note_name(midi: u8) -> String {
    let names = [
        "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
    ];
    let octave = (midi as i32 / 12) - 1;
    let note_idx = (midi % 12) as usize;
    format!("{}{}", names[note_idx], octave)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PitchBendPoint {
    /// Time offset in ms from note start
    pub time_ms: f64,
    /// Pitch offset in semitones (cents / 100)
    pub semitones: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VibratoParam {
    pub length_pct: f64,  // percentage of note duration (e.g. 65%)
    pub period_ms: f64,   // vibrato period in ms (e.g. 175ms)
    pub depth_cents: f64, // vibrato depth in cents (e.g. 50 cents)
    pub fade_in_ms: f64,  // fade in time in ms
}

impl VibratoParam {
    pub fn pitch_offset_cents_at(&self, time_ms: f64, note_duration_ms: f64) -> f64 {
        let vib_start = note_duration_ms * (1.0 - self.length_pct / 100.0);
        if time_ms < vib_start || self.period_ms <= 0.0 {
            return 0.0;
        }

        let vib_time = time_ms - vib_start;
        let fade = if self.fade_in_ms > 0.0 {
            (vib_time / self.fade_in_ms).min(1.0)
        } else {
            1.0
        };

        let phase = (vib_time / self.period_ms) * 2.0 * std::f64::consts::PI;
        phase.sin() * self.depth_cents * fade
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_note_conversion() {
        assert_eq!(note_name_to_midi("C4"), Some(60));
        assert_eq!(note_name_to_midi("A4"), Some(69));
        assert_eq!(midi_to_note_name(60), "C4");
        assert_eq!(midi_to_note_name(69), "A4");
        assert!((midi_to_freq(69.0) - 440.0).abs() < 1e-3);
    }
}
