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
            let pyin_res = crate::dsp::pyin::PitchExtractor::extract_pitch_and_gci(vowel_slice, sample_rate);
            
            // LPC Formant Preservation
            let lpc_order = (sample_rate / 1000 + 2) as usize; // e.g., 46 for 44.1kHz
            let lpc_coeffs = crate::dsp::lpc::LpcExtractor::extract_lpc_coefficients(vowel_slice, lpc_order);
            let residual_slice = crate::dsp::lpc::LpcExtractor::extract_residual(vowel_slice, &lpc_coeffs);

            // Fallback period if pyin failed
            let fallback_t0 = Self::estimate_pitch_period(vowel_slice, sample_rate);
            let mut synth_residual = vec![0.0f32; target_vowel_samples];
            let mut synth_weights = vec![0.0f32; target_vowel_samples];

            let vowel_len = vowel_slice.len();
            let loop_start = vowel_len / 4;
            let loop_end = (vowel_len * 3 / 4).max(loop_start + fallback_t0);
            let loop_len = (loop_end.saturating_sub(loop_start)).max(fallback_t0);

            let mut out_pos = 0usize;
            let mut virt_in_pos = 0f64;

            while out_pos < target_vowel_samples {
                let rel_t_ms = (consonant_ms) + (out_pos as f64 / sample_rate as f64) * 1000.0;
                let pitch_cents = PitchBendSolver::get_pitch_offset_cents(rel_t_ms, pitch_points);
                let frame_freq = (target_pitch_freq * 2.0f64.powf(pitch_cents / 1200.0)).max(20.0);

                let dst_t0 = (sample_rate as f64 / frame_freq).round() as usize;
                let dst_t0 = dst_t0.clamp(16, sample_rate as usize / 4);

                let in_pos_int = virt_in_pos as usize;
                let src_center = if in_pos_int >= loop_end && loop_len > 0 {
                    loop_start + ((in_pos_int - loop_start) % loop_len)
                } else {
                    in_pos_int % vowel_len
                };

                let win_size_max = (2 * fallback_t0).max(64);
                let check_start = src_center.saturating_sub(win_size_max / 2);
                let check_end = (src_center + win_size_max / 2).min(vowel_len);
                
                // Voiced/Unvoiced (V/UV) Detection via Zero-Crossing Rate (ZCR)
                let mut zero_crossings = 0;
                let mut energy = 0.0;
                for i in check_start + 1..check_end {
                    if (vowel_slice[i - 1] > 0.0 && vowel_slice[i] <= 0.0) || (vowel_slice[i - 1] < 0.0 && vowel_slice[i] >= 0.0) {
                        zero_crossings += 1;
                    }
                    energy += vowel_slice[i] * vowel_slice[i];
                }
                
                let check_len = (check_end - check_start).max(1);
                let zcr = zero_crossings as f32 / check_len as f32;
                
                // High ZCR and low periodic energy implies Unvoiced (Consonant/Breath)
                let is_unvoiced = zcr > 0.25 || (energy < 1e-4 && zcr > 0.1);

                let (src_start, src_end, win_size, src_advance) = if is_unvoiced {
                    // WSOLA for Unvoiced: Fixed window, no pitch shift
                    let w_size = (sample_rate as f64 * 0.01).round() as usize; // 10ms window
                    let s_start = src_center.saturating_sub(w_size / 2);
                    let s_end = (s_start + w_size).min(vowel_len);
                    (s_start, s_end, w_size, dst_t0 as f64)
                } else {
                    // PSOLA for Voiced: GCI-aligned window
                    let mut best_gci = src_center;
                    let mut src_t0 = fallback_t0;
                    
                    if !pyin_res.gci_marks.is_empty() {
                        let mut min_dist = usize::MAX;
                        for (i, &gci) in pyin_res.gci_marks.iter().enumerate() {
                            let dist = (gci as isize - src_center as isize).abs() as usize;
                            if dist < min_dist {
                                min_dist = dist;
                                best_gci = gci;
                                if i < pyin_res.pitch_contour.len() {
                                    src_t0 = pyin_res.pitch_contour[i] as usize;
                                }
                            }
                        }
                    }

                    let w_size = (2 * src_t0).max(64);
                    let s_start = best_gci.saturating_sub(w_size / 2);
                    let s_end = (s_start + w_size).min(vowel_len);
                    (s_start, s_end, w_size, src_t0 as f64)
                };

                let actual_win = (src_end - src_start).min(target_vowel_samples - out_pos);
                let w_step = 2.0 * std::f32::consts::PI / win_size as f32;
                
                let out_slice_res = &mut synth_residual[out_pos..out_pos + actual_win];
                let out_slice_wei = &mut synth_weights[out_pos..out_pos + actual_win];
                
                if is_unvoiced {
                    let src_slice = &vowel_slice[src_start..src_start + actual_win];
                    for i in 0..actual_win {
                        let w = 0.5 * (1.0 - (w_step * i as f32).cos());
                        out_slice_res[i] += src_slice[i] * w;
                        out_slice_wei[i] += w;
                    }
                } else {
                    let src_slice = &residual_slice[src_start..src_start + actual_win];
                    for i in 0..actual_win {
                        let w = 0.5 * (1.0 - (w_step * i as f32).cos());
                        out_slice_res[i] += src_slice[i] * w;
                        out_slice_wei[i] += w;
                    }
                }

                out_pos += dst_t0;
                virt_in_pos += src_advance;
            }

            for i in 0..target_vowel_samples {
                if synth_weights[i] > 1e-4 {
                    synth_residual[i] /= synth_weights[i];
                }
            }

            // Re-apply LPC Formant Filter
            let synth_vowel = crate::dsp::lpc::LpcExtractor::synthesize(&synth_residual, &lpc_coeffs);
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
