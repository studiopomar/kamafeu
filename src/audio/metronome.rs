use std::f32::consts::PI;

/// Synthesizes and overlays tempo-accurate metronome clicks directly onto an audio buffer.
/// - Downbeat (Beat 1 of measure in 4/4): High pitch (1600 Hz), accented volume.
/// - Secondary beats (Beats 2, 3, 4): Standard pitch (1000 Hz), medium volume.
pub fn apply_metronome_clicks(
    samples: &mut [f32],
    sample_rate: u32,
    channels: u16,
    chunk_start_ms: f64,
    bpm: f64,
) {
    if samples.is_empty() || sample_rate == 0 || bpm <= 0.0 {
        return;
    }

    let ch = (channels as usize).max(1);
    let num_frames = samples.len() / ch;
    let beat_duration_ms = 60000.0 / bpm.max(10.0);
    let sample_rate_f = sample_rate as f64;

    for frame_idx in 0..num_frames {
        let frame_time_ms = chunk_start_ms + (frame_idx as f64 * 1000.0 / sample_rate_f);
        if frame_time_ms < 0.0 {
            continue;
        }

        let beat_num = (frame_time_ms / beat_duration_ms).floor();
        let time_in_beat_ms = frame_time_ms - (beat_num * beat_duration_ms);

        // Click duration is 25 ms
        if time_in_beat_ms < 25.0 {
            let is_downbeat = (beat_num as i64) % 4 == 0;
            let freq = if is_downbeat { 1600.0 } else { 1000.0 };
            let peak_amp = if is_downbeat { 0.50 } else { 0.32 };

            // Exponential decay envelope
            let env = (-time_in_beat_ms / 6.0).exp() as f32 * peak_amp;
            let t_sec = (time_in_beat_ms / 1000.0) as f32;
            let click = (2.0 * PI * freq * t_sec).sin() * env;

            for c in 0..ch {
                let sample_idx = frame_idx * ch + c;
                if sample_idx < samples.len() {
                    samples[sample_idx] = (samples[sample_idx] + click).clamp(-1.0, 1.0);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metronome_click_overlay() {
        let sample_rate = 44100;
        let channels = 2;
        let num_samples = sample_rate as usize * channels as usize; // 1 second
        let mut samples = vec![0.0f32; num_samples];

        apply_metronome_clicks(&mut samples, sample_rate, channels, 0.0, 120.0);

        // At 120 BPM, beats occur at 0ms (sample 0) and 500ms (sample 22050)
        let has_click_at_start = samples[0..100].iter().any(|&s| s.abs() > 0.05);
        assert!(
            has_click_at_start,
            "Downbeat click should be present at 0ms"
        );

        let beat2_idx = (22050 * channels as usize) as usize;
        let has_click_at_beat2 = samples[beat2_idx..beat2_idx + 100]
            .iter()
            .any(|&s| s.abs() > 0.05);
        assert!(
            has_click_at_beat2,
            "Beat 2 click should be present at 500ms"
        );
    }
}
