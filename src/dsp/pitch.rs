use serde::{Deserialize, Serialize};

/// Convert MIDI note number to frequency in Hertz
pub fn midi_to_freq(midi_note: f64) -> f64 {
    440.0 * 2.0f64.powf((midi_note - 69.0) / 12.0)
}

/// Convert musical note name (e.g., "C4", "C#4", "Db4", "A4") to MIDI key number
pub fn note_name_to_midi(name: &str) -> Option<u8> {
    let name = name.trim();
    if name.is_empty() || !name.is_ascii() {
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

#[cfg(test)]
mod note_name_tests {
    use super::note_name_to_midi;

    #[test]
    fn rejects_unicode_without_panicking() {
        assert_eq!(note_name_to_midi("é"), None);
        assert_eq!(note_name_to_midi("♯4"), None);
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VibratoParam {
    pub length_pct: f64,
    pub period_ms: f64,
    pub depth_cents: f64,
    /// Legacy Kamafeu field. Used only when `fade_in_pct` is zero.
    pub fade_in_ms: f64,
    pub fade_in_pct: f64,
    pub fade_out_pct: f64,
    pub shift_pct: f64,
    pub drift_pct: f64,
    pub volume_link_pct: f64,
}

impl Default for VibratoParam {
    fn default() -> Self {
        Self {
            length_pct: 0.0,
            period_ms: 175.0,
            depth_cents: 35.0,
            fade_in_ms: 0.0,
            fade_in_pct: 20.0,
            fade_out_pct: 20.0,
            shift_pct: 0.0,
            drift_pct: 0.0,
            volume_link_pct: 0.0,
        }
    }
}

impl VibratoParam {
    pub fn normalize(&mut self) {
        let (length, period, depth, fade_in, fade_out, shift, drift, link) = self.values();
        self.length_pct = length;
        self.period_ms = period;
        self.depth_cents = depth;
        self.fade_in_pct = fade_in;
        self.fade_out_pct = fade_out;
        self.shift_pct = shift;
        self.drift_pct = drift;
        self.volume_link_pct = link;
        if !self.fade_in_ms.is_finite() || self.fade_in_ms < 0.0 {
            self.fade_in_ms = 0.0;
        }
    }

    fn values(&self) -> (f64, f64, f64, f64, f64, f64, f64, f64) {
        let finite = |value: f64, fallback: f64| if value.is_finite() { value } else { fallback };
        let length = finite(self.length_pct, 0.0).clamp(0.0, 100.0);
        let period = if self.period_ms.is_finite() {
            self.period_ms.clamp(5.0, 500.0)
        } else {
            175.0
        };
        let depth = if self.depth_cents.is_finite() {
            self.depth_cents.clamp(0.0, 200.0)
        } else {
            0.0
        };
        let mut fade_in = finite(self.fade_in_pct, 20.0).clamp(0.0, 100.0);
        let fade_out = finite(self.fade_out_pct, 20.0).clamp(0.0, 100.0 - fade_in);
        fade_in = fade_in.min(100.0 - fade_out);
        (
            length,
            period,
            depth,
            fade_in,
            fade_out,
            finite(self.shift_pct, 0.0).clamp(0.0, 100.0),
            finite(self.drift_pct, 0.0).clamp(-100.0, 100.0),
            finite(self.volume_link_pct, 0.0).clamp(-100.0, 100.0),
        )
    }

    fn fade_gain_at(&self, time_ms: f64, note_duration_ms: f64) -> f64 {
        let (length, _, _, fade_in_pct, fade_out_pct, _, _, _) = self.values();
        if length <= 0.0 || note_duration_ms <= 0.0 {
            return 0.0;
        }
        let vibrato_duration = note_duration_ms * length / 100.0;
        let start = note_duration_ms - vibrato_duration;
        if time_ms < start || time_ms > note_duration_ms {
            return 0.0;
        }
        let elapsed = time_ms - start;
        let fade_in_ms = if self.fade_in_pct == 0.0 && self.fade_in_ms > 0.0 {
            self.fade_in_ms
        } else {
            vibrato_duration * fade_in_pct / 100.0
        };
        let fade_out_ms = vibrato_duration * fade_out_pct / 100.0;
        let mut gain: f64 = 1.0;
        if fade_in_ms > 0.0 && elapsed < fade_in_ms {
            gain = gain.min(elapsed / fade_in_ms);
        }
        let remaining = note_duration_ms - time_ms;
        if fade_out_ms > 0.0 && remaining < fade_out_ms {
            gain = gain.min(remaining / fade_out_ms);
        }
        gain.clamp(0.0, 1.0)
    }

    pub fn pitch_offset_cents_at(&self, time_ms: f64, note_duration_ms: f64) -> f64 {
        let (length, period, depth, _, _, shift, drift, _) = self.values();
        let vib_start = note_duration_ms * (1.0 - length / 100.0);
        let fade = self.fade_gain_at(time_ms, note_duration_ms);
        if fade <= 0.0 || period <= 0.0 {
            return 0.0;
        }
        let vib_time = time_ms - vib_start;
        let phase = (vib_time / period + shift / 100.0) * 2.0 * std::f64::consts::PI;
        (phase.sin() * depth + depth / 100.0 * drift) * fade
    }

    /// Amplitude modulation linked to vibrato, compatible with OpenUtau's
    /// volume-link behavior. A negative value reverses its phase.
    pub fn volume_multiplier_at(&self, time_ms: f64, note_duration_ms: f64) -> f64 {
        let (length, period, _, _, _, mut shift, _, mut link) = self.values();
        let fade = self.fade_gain_at(time_ms, note_duration_ms);
        if fade <= 0.0 || link == 0.0 {
            return 1.0;
        }
        if link < 0.0 {
            shift = (shift + 50.0) % 100.0;
            link = -link;
        }
        let start = note_duration_ms * (1.0 - length / 100.0);
        let phase = ((time_ms - start) / period + shift / 100.0) * 2.0 * std::f64::consts::PI;
        let reduction = (-phase.sin() / 2.0 + 0.3) * link / 100.0 * fade;
        (1.0 - reduction).max(0.0)
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

    #[test]
    fn vibrato_has_fade_out_shift_drift_and_volume_link() {
        let vibrato = VibratoParam {
            length_pct: 100.0,
            period_ms: 200.0,
            depth_cents: 100.0,
            fade_in_pct: 20.0,
            fade_out_pct: 20.0,
            shift_pct: 25.0,
            drift_pct: 10.0,
            volume_link_pct: 50.0,
            ..Default::default()
        };
        assert!(vibrato.pitch_offset_cents_at(100.0, 1000.0).abs() > 40.0);
        assert_eq!(vibrato.pitch_offset_cents_at(1000.0, 1000.0), 0.0);
        assert_ne!(vibrato.volume_multiplier_at(500.0, 1000.0), 1.0);
    }
}
