use crate::dsp::midi_to_freq;
use crate::project::model::UNote;

/// Inharmonicity coefficient for acoustic piano string stiffness
const INHARMONICITY_B: f64 = 0.00015;

/// Number of partials / harmonics to synthesize per note (limited by Nyquist)
const MAX_PARTIALS: usize = 14;

/// Generates a single piano/keyboard note audio buffer with realistic acoustic modeling:
/// - Inharmonic additive synthesis (stiffness factor)
/// - Dual detuned string pairs for beating and warmth
/// - Frequency-dependent per-harmonic damping
/// - Hammer strike attack transient
/// - Natural note duration and damper release
pub fn render_piano_tone(freq: f64, duration_ms: f64, sample_rate: u32, velocity: f32) -> Vec<f32> {
    if freq <= 10.0 || sample_rate == 0 {
        return Vec::new();
    }

    let sr = sample_rate as f64;
    let nyquist = sr * 0.48;
    let dur_sec = (duration_ms / 1000.0).max(0.04);
    let release_sec = 0.09;
    let total_sec = dur_sec + release_sec;
    let total_samples = (total_sec * sr).ceil() as usize;

    if total_samples == 0 {
        return Vec::new();
    }

    // Base decay time depends on pitch (low notes sustain longer, high notes decay faster)
    let midi_approx = 69.0 + 12.0 * (freq / 440.0).log2();
    let pitch_decay_scale = (1.0 + ((70.0 - midi_approx) / 35.0)).clamp(0.4, 3.2);
    let base_decay_time = 1.6 * pitch_decay_scale;

    let vel = velocity.clamp(0.1, 1.5);
    // Higher velocity brightens the sound and opens higher harmonics
    let brightness = (0.5 + 0.5 * vel as f64).clamp(0.4, 1.6);

    let mut samples = vec![0.0f32; total_samples];

    // Compute partial frequencies, base amplitudes, and decay constants
    let mut partial_params = Vec::with_capacity(MAX_PARTIALS);
    for k in 1..=MAX_PARTIALS {
        let k_f = k as f64;
        // Inharmonicity formula: f_k = k * f0 * sqrt(1 + B * k^2)
        let f_k = k_f * freq * (1.0 + INHARMONICITY_B * k_f * k_f).sqrt();
        if f_k >= nyquist {
            break;
        }

        // Spectral envelope rolloff
        let amp_base = (1.0 / (k_f.powf(1.15))) * (brightness.powf(k_f * 0.25));
        // Higher partials decay significantly faster
        let decay_rate = (1.0 + 0.55 * (k_f - 1.0).powf(1.3)) / base_decay_time;

        // Two detuned string frequencies for acoustic beating
        let detune_cents = 0.6 / (1.0 + 0.1 * k_f);
        let detune_ratio = (detune_cents / 1200.0) * (2.0f64).ln();
        let f_k1 = f_k * (1.0 - detune_ratio);
        let f_k2 = f_k * (1.0 + detune_ratio);

        partial_params.push((f_k1, f_k2, amp_base as f32, decay_rate));
    }

    let attack_time = 0.0035; // 3.5ms attack
    let attack_samples = (attack_time * sr).max(1.0) as usize;

    // Synthesize partials
    for i in 0..total_samples {
        let t = i as f64 / sr;

        // Attack envelope
        let attack_env = if i < attack_samples {
            (i as f32 / attack_samples as f32).powf(0.7)
        } else {
            1.0f32
        };

        // Release / damper envelope
        let release_env = if t > dur_sec {
            let rel_t = (t - dur_sec) / release_sec;
            (1.0 - rel_t).clamp(0.0, 1.0) as f32
        } else {
            1.0f32
        };

        let env_mod = attack_env * release_env;
        if env_mod <= 1e-6 {
            continue;
        }

        let mut sample_sum = 0.0f32;

        for &(f_k1, f_k2, amp_base, decay_rate) in &partial_params {
            let partial_decay = (-(decay_rate * t)).exp() as f32;
            let p1 = (t * 2.0 * std::f64::consts::PI * f_k1).sin() as f32;
            let p2 = (t * 2.0 * std::f64::consts::PI * f_k2).sin() as f32;
            sample_sum += 0.5 * (p1 + p2) * (amp_base * partial_decay);
        }

        // Add subtle hammer strike percussive transient in first 10ms
        if t < 0.010 {
            let hammer_t = t / 0.010;
            let hammer_env = (1.0 - hammer_t).powi(2) as f32 * 0.15 * brightness as f32;
            let click = ((t * 2.0 * std::f64::consts::PI * (freq * 4.0 + 800.0)).sin()
                + (t * 2.0 * std::f64::consts::PI * 1800.0).sin()) as f32
                * 0.5;
            sample_sum += click * hammer_env;
        }

        samples[i] = sample_sum * env_mod * 0.28 * vel;
    }

    samples
}

/// Generates a quick ~400ms preview tone for interactive piano roll feedback
pub fn render_piano_preview(freq: f64, sample_rate: u32) -> Vec<f32> {
    render_piano_tone(freq, 350.0, sample_rate, 1.0)
}

/// Renders a whole sequence of UNotes as piano/keyboard audio into a PCM track buffer.
/// Polyphony and overlapping notes are summed cleanly.
pub fn render_piano_track(notes: &[UNote], sample_rate: u32) -> Vec<f32> {
    if notes.is_empty() || sample_rate == 0 {
        return Vec::new();
    }

    let max_end_ms = notes
        .iter()
        .map(|n| n.position_ms + n.duration_ms + 200.0)
        .fold(0.0f64, f64::max);

    let total_samples =
        ((max_end_ms / 1000.0) * sample_rate as f64).ceil() as usize + sample_rate as usize;
    let mut track_buffer = vec![0.0f32; total_samples];

    for note in notes {
        let freq = midi_to_freq(note.midi_key() as f64);
        let velocity = (note.expressions.dynamics / 100.0).clamp(0.2, 1.8) as f32;
        let note_samples = render_piano_tone(freq, note.duration_ms, sample_rate, velocity);

        let start_sample = ((note.position_ms / 1000.0) * sample_rate as f64).round() as usize;

        for (i, &s) in note_samples.iter().enumerate() {
            let out_idx = start_sample + i;
            if out_idx < track_buffer.len() {
                track_buffer[out_idx] += s;
            }
        }
    }

    // Soft saturation to protect against clipping on dense polyphony
    for sample in &mut track_buffer {
        let x = *sample;
        if x.abs() > 0.8 {
            *sample = (x / 0.8).tanh() * 0.8;
        }
    }

    track_buffer
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_piano_tone_validity() {
        let tone = render_piano_tone(440.0, 300.0, 44100, 1.0);
        assert!(!tone.is_empty());
        for &s in &tone {
            assert!(!s.is_nan());
            assert!(!s.is_infinite());
            assert!(s.abs() <= 1.5);
        }
    }

    #[test]
    fn test_render_piano_track() {
        let notes = vec![
            UNote::new("ka", "C4", 0.0, 500.0),
            UNote::new("ka", "E4", 250.0, 500.0),
            UNote::new("ka", "G4", 500.0, 500.0),
        ];
        let track = render_piano_track(&notes, 44100);
        assert!(!track.is_empty());
        assert!(track.len() > 44100);
    }
}
