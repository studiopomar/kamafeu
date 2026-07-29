use crate::dsp::pitch_bend::PitchBendSolver;
use crate::project::model::UPitchBendPoint;

pub struct Resampler;

impl Resampler {
    /// Estimate fundamental pitch period (in samples) using auto-correlation.
    pub fn estimate_pitch_period(slice: &[f32], sample_rate: u32) -> usize {
        let min_period = (sample_rate as f64 / 1000.0) as usize; // Max ~1000 Hz
        let max_period = (sample_rate as f64 / 50.0) as usize;   // Min ~50 Hz

        let mut best_period = min_period;
        let mut max_corr = -1.0f32;

        for period in min_period..=max_period.min(slice.len() / 2) {
            let mut corr = 0.0f32;
            let mut energy = 0.0f32;

            for i in 0..(slice.len() - period) {
                corr += slice[i] * slice[i + period];
                energy += slice[i] * slice[i];
            }

            if energy > 1e-6 {
                let norm_corr = corr / energy;
                if norm_corr > max_corr {
                    max_corr = norm_corr;
                    best_period = period;
                }
            }
        }

        best_period
    }

    /// Pitch-shift and time-stretch an audio slice using TD-PSOLA.
    pub fn render_sample(
        input_samples: &[f32],
        sample_rate: u32,
        offset_ms: f64,
        consonant_ms: f64,
        cutoff_ms: f64,
        target_duration_ms: f64,
        target_pitch_freq: f64,
    ) -> Vec<f32> {
        Self::render_sample_with_pitch_bend(
            input_samples,
            sample_rate,
            offset_ms,
            consonant_ms,
            cutoff_ms,
            target_duration_ms,
            target_pitch_freq,
            &[],
        )
    }

    /// Pitch-shift and time-stretch with time-varying pitch bend curve evaluation frame-by-frame.
    pub fn render_sample_with_pitch_bend(
        input_samples: &[f32],
        sample_rate: u32,
        offset_ms: f64,
        consonant_ms: f64,
        cutoff_ms: f64,
        target_duration_ms: f64,
        target_pitch_freq: f64,
        pitch_points: &[UPitchBendPoint],
    ) -> Vec<f32> {
        let start_sample = ((offset_ms / 1000.0) * sample_rate as f64)
            .clamp(0.0, input_samples.len() as f64) as usize;

        let end_sample = if cutoff_ms < 0.0 {
            // Negative cutoff: distance in ms from the END of the WAV file
            let cut_samples = ((-cutoff_ms / 1000.0) * sample_rate as f64) as usize;
            input_samples.len().saturating_sub(cut_samples).max(start_sample)
        } else if cutoff_ms > 0.0 {
            // Positive cutoff: fixed length in ms from start_sample (offset)
            let len_samples = ((cutoff_ms / 1000.0) * sample_rate as f64) as usize;
            (start_sample + len_samples).min(input_samples.len())
        } else {
            input_samples.len()
        };

        if start_sample >= end_sample || start_sample >= input_samples.len() {
            let target_sample_count = ((target_duration_ms / 1000.0) * sample_rate as f64) as usize;
            return vec![0.0; target_sample_count];
        }

        let slice = &input_samples[start_sample..end_sample];

        let consonant_samples = ((consonant_ms / 1000.0) * sample_rate as f64)
            .clamp(0.0, slice.len() as f64) as usize;

        let vowel_slice = &slice[consonant_samples..];
        
        let target_total_samples = ((target_duration_ms / 1000.0) * sample_rate as f64) as usize;
        let target_vowel_samples = target_total_samples.saturating_sub(consonant_samples);

        let mut output = Vec::with_capacity(target_total_samples);

        if consonant_samples > 0 {
            let copy_len = consonant_samples.min(slice.len());
            output.extend_from_slice(&slice[..copy_len]);
        }

        if target_vowel_samples > 0 && !vowel_slice.is_empty() {
            let src_t0 = Self::estimate_pitch_period(vowel_slice, sample_rate);
            let win_size = (2 * src_t0).max(64);
            let mut synth_vowel = vec![0.0f32; target_vowel_samples];
            let mut synth_weights = vec![0.0f32; target_vowel_samples];

            let vowel_len = vowel_slice.len();
            let loop_start = (vowel_len / 4).min(vowel_len.saturating_sub(win_size));
            let loop_end = (vowel_len * 3 / 4).max(loop_start + src_t0);
            let loop_len = (loop_end.saturating_sub(loop_start)).max(src_t0);

            let mut out_pos = 0usize;
            let mut in_pos = 0usize;

            while out_pos < target_vowel_samples {
                // Time relative to the start of the NOTE (0..target_duration_ms),
                // matching the time_offset_ms reference used in pitch_bend points
                // drawn by the piano roll. consonant_ms accounts for the consonant
                // part that was already copied before this loop.
                let rel_t_ms = (consonant_ms) + (out_pos as f64 / sample_rate as f64) * 1000.0;
                let pitch_cents = PitchBendSolver::get_pitch_offset_cents(rel_t_ms, pitch_points);
                let frame_freq = (target_pitch_freq * 2.0f64.powf(pitch_cents / 1200.0)).max(20.0);

                let dst_t0 = (sample_rate as f64 / frame_freq).round() as usize;
                let dst_t0 = dst_t0.clamp(16, sample_rate as usize / 4);

                let src_center = if in_pos >= loop_end && loop_len > 0 {
                    loop_start + ((in_pos - loop_start) % loop_len)
                } else {
                    in_pos % vowel_len
                };

                let src_start = src_center.saturating_sub(win_size / 2);
                let src_end = (src_start + win_size).min(vowel_len);

                for i in src_start..src_end {
                    let relative_i = i - src_start;
                    let out_idx = out_pos + relative_i;
                    if out_idx < target_vowel_samples {
                        let w = 0.5 * (1.0 - (2.0 * std::f64::consts::PI * relative_i as f64 / win_size as f64).cos()) as f32;
                        synth_vowel[out_idx] += vowel_slice[i] * w;
                        synth_weights[out_idx] += w;
                    }
                }

                out_pos += dst_t0;
                in_pos += src_t0;
            }

            for i in 0..target_vowel_samples {
                if synth_weights[i] > 1e-4 {
                    synth_vowel[i] /= synth_weights[i];
                }
            }

            output.extend(synth_vowel);
        }

        output.truncate(target_total_samples);
        if output.len() < target_total_samples {
            output.resize(target_total_samples, 0.0);
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample_output_length() {
        let sample_rate = 44100;
        let dummy_samples = vec![0.0f32; sample_rate as usize]; // 1 sec
        let rendered = Resampler::render_sample(&dummy_samples, sample_rate, 0.0, 100.0, 0.0, 500.0, 440.0);
        let expected_len = (sample_rate as f64 * 0.5) as usize;
        assert_eq!(rendered.len(), expected_len);
    }

    #[test]
    fn test_resample_with_negative_cutoff() {
        let sample_rate = 44100;
        // 2 sec sine wave
        let dummy_samples: Vec<f32> = (0..sample_rate as usize * 2)
            .map(|i| (i as f32 * 0.1).sin())
            .collect();
        // offset 100ms, consonant 50ms, cutoff -100ms, target duration 400ms
        let rendered = Resampler::render_sample_with_pitch_bend(
            &dummy_samples, sample_rate, 100.0, 50.0, -100.0, 400.0, 440.0, &[]
        );
        let expected_len = (sample_rate as f64 * 0.4) as usize;
        assert_eq!(rendered.len(), expected_len);
        let max_amp = rendered.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(max_amp > 1e-3, "Expected non-silent output, got max_amp {}", max_amp);
    }
}
