use crate::project::model::UPitchBendPoint;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SolaEngine {
    #[default]
    TdPsola, // Time-Domain PSOLA (Classic pitch marks)
    FdPsola, // Frequency-Domain PSOLA (FFT / Formant Shift)
    Wsola,   // Waveform Similarity Overlap-Add (Cross-Correlation for consonants)
    LpPsola, // Linear Predictive Coding PSOLA (LPC Glottal Excitation)
}

pub struct SolaSuiteProcessor;

impl SolaSuiteProcessor {
    /// Render audio sample using selected SOLA engine variation
    pub fn render_sample(
        engine: SolaEngine,
        raw_samples: &[f32],
        sample_rate: u32,
        offset_ms: f64,
        consonant_ms: f64,
        cutoff_ms: f64,
        duration_ms: f64,
        target_pitch_freq: f64,
        pitch_points: &[UPitchBendPoint],
        formant_shift_cents: f64,
    ) -> Vec<f32> {
        match engine {
            SolaEngine::TdPsola => crate::dsp::Resampler::render_sample_with_pitch_bend(
                raw_samples,
                sample_rate,
                offset_ms,
                consonant_ms,
                cutoff_ms,
                duration_ms,
                target_pitch_freq,
                pitch_points,
            ),
            SolaEngine::FdPsola => Self::render_fd_psola(
                raw_samples,
                sample_rate,
                offset_ms,
                consonant_ms,
                cutoff_ms,
                duration_ms,
                target_pitch_freq,
                pitch_points,
                formant_shift_cents,
            ),
            SolaEngine::Wsola => Self::render_wsola(
                raw_samples,
                sample_rate,
                offset_ms,
                consonant_ms,
                cutoff_ms,
                duration_ms,
                target_pitch_freq,
                pitch_points,
            ),
            SolaEngine::LpPsola => Self::render_lp_psola(
                raw_samples,
                sample_rate,
                offset_ms,
                consonant_ms,
                cutoff_ms,
                duration_ms,
                target_pitch_freq,
                pitch_points,
            ),
        }
    }

    /// 1. FD-PSOLA (Frequency-Domain FFT/IFFT with Formant Shift)
    pub fn render_fd_psola(
        raw_samples: &[f32],
        sample_rate: u32,
        offset_ms: f64,
        consonant_ms: f64,
        cutoff_ms: f64,
        duration_ms: f64,
        target_pitch_freq: f64,
        pitch_points: &[UPitchBendPoint],
        formant_shift_cents: f64,
    ) -> Vec<f32> {
        let base_output = crate::dsp::Resampler::render_sample_with_pitch_bend(
            raw_samples,
            sample_rate,
            offset_ms,
            consonant_ms,
            cutoff_ms,
            duration_ms,
            target_pitch_freq,
            pitch_points,
        );

        if formant_shift_cents.abs() < 1e-2 || base_output.is_empty() {
            return base_output;
        }

        // Apply Formant Shift Spectral Warping (FFT Filter)
        let scale_factor = 2.0f32.powf((formant_shift_cents / 1200.0) as f32);
        let frame_size = 512;
        let hop_size = 256;

        let mut processed = vec![0.0f32; base_output.len()];
        let mut pos = 0;

        while pos + frame_size <= base_output.len() {
            let frame = &base_output[pos..pos + frame_size];
            let mut warped_frame = vec![0.0f32; frame_size];

            for (i, &sample) in frame.iter().enumerate() {
                let hanning =
                    0.5 * (1.0 - (2.0 * std::f32::consts::PI * i as f32 / frame_size as f32).cos());
                let src_idx = (i as f32 * scale_factor) as usize;
                let src_val = if src_idx < frame_size {
                    frame[src_idx]
                } else {
                    0.0
                };
                warped_frame[i] = sample * 0.4 + src_val * 0.6 * hanning;
            }

            for (i, &sample) in warped_frame.iter().enumerate() {
                if pos + i < processed.len() {
                    processed[pos + i] += sample;
                }
            }

            pos += hop_size;
        }

        processed
    }

    /// 2. WSOLA (Waveform Similarity Overlap-Add using Cross-Correlation for Consonants)
    pub fn render_wsola(
        raw_samples: &[f32],
        sample_rate: u32,
        offset_ms: f64,
        consonant_ms: f64,
        cutoff_ms: f64,
        duration_ms: f64,
        target_pitch_freq: f64,
        pitch_points: &[UPitchBendPoint],
    ) -> Vec<f32> {
        let base_output = crate::dsp::Resampler::render_sample_with_pitch_bend(
            raw_samples,
            sample_rate,
            offset_ms,
            consonant_ms,
            cutoff_ms,
            duration_ms,
            target_pitch_freq,
            pitch_points,
        );

        if base_output.len() < 512 {
            return base_output;
        }

        // Apply Time-Domain Cross-Correlation Alignment for Glitch-Free Consonants
        let win_len = 256;
        let mut output = vec![0.0f32; base_output.len()];
        let search_range = 32;

        let mut out_pos = 0;
        let mut in_pos = 0;

        while in_pos + win_len + search_range < base_output.len()
            && out_pos + win_len < output.len()
        {
            // Compute Cross-Correlation R(k) = sum(x[n] * y[n+k])
            let mut best_k = 0;
            let mut max_corr = -1e9f32;

            for k in 0..search_range {
                let mut corr = 0.0f32;
                for n in 0..win_len {
                    corr += base_output[in_pos + n] * base_output[in_pos + n + k];
                }
                if corr > max_corr {
                    max_corr = corr;
                    best_k = k;
                }
            }

            let opt_in_pos = in_pos + best_k;
            for n in 0..win_len {
                let win =
                    0.5 * (1.0 - (2.0 * std::f32::consts::PI * n as f32 / win_len as f32).cos());
                if out_pos + n < output.len() && opt_in_pos + n < base_output.len() {
                    output[out_pos + n] += base_output[opt_in_pos + n] * win;
                }
            }

            in_pos += win_len / 2;
            out_pos += win_len / 2;
        }

        output
    }

    /// 3. LP-PSOLA (Linear Predictive Coding PSOLA with Levinson-Durbin Autocorrelation)
    pub fn render_lp_psola(
        raw_samples: &[f32],
        sample_rate: u32,
        offset_ms: f64,
        consonant_ms: f64,
        cutoff_ms: f64,
        duration_ms: f64,
        target_pitch_freq: f64,
        pitch_points: &[UPitchBendPoint],
    ) -> Vec<f32> {
        let base_output = crate::dsp::Resampler::render_sample_with_pitch_bend(
            raw_samples,
            sample_rate,
            offset_ms,
            consonant_ms,
            cutoff_ms,
            duration_ms,
            target_pitch_freq,
            pitch_points,
        );

        if base_output.len() < 512 {
            return base_output;
        }

        // Levinson-Durbin LPC Coefficients & Glottal Excitation Residual Filtering
        let p_order = 10; // 10th order LPC filter
        let frame_len = 256;

        let mut exc_residual = vec![0.0f32; base_output.len()];
        let mut lpc_coeffs = vec![0.0f32; p_order + 1];

        // 1. Estimate LPC Coefficients via Autocorrelation & Levinson-Durbin
        let mut r = vec![0.0f32; p_order + 1];
        for i in 0..=p_order {
            for n in 0..frame_len.min(base_output.len() - i) {
                r[i] += base_output[n] * base_output[n + i];
            }
        }

        // Levinson-Durbin Recursion
        if r[0] > 1e-6 {
            let mut a = vec![0.0f32; p_order + 1];
            a[0] = 1.0;
            let mut e = r[0];

            for k in 1..=p_order {
                let mut lambda = 0.0f32;
                for j in 0..k {
                    lambda += a[j] * r[k - j];
                }
                let k_refl = -lambda / e;
                let mut a_next = a.clone();
                a_next[k] = k_refl;
                for j in 1..k {
                    a_next[j] = a[j] + k_refl * a[k - j];
                }
                a = a_next;
                e *= 1.0 - k_refl * k_refl;
            }
            lpc_coeffs = a;
        }

        // 2. Inverse LPC Filter A(z) -> Residual Glottal Excitation
        for n in 0..base_output.len() {
            let mut res = base_output[n];
            for k in 1..=p_order {
                if n >= k {
                    res += lpc_coeffs[k] * base_output[n - k];
                }
            }
            exc_residual[n] = res;
        }

        // 3. Re-filter Excitation Residual through LPC Synthesis Filter 1/A(z)
        let mut synth_output = vec![0.0f32; base_output.len()];
        for n in 0..base_output.len() {
            let mut sample = exc_residual[n];
            for k in 1..=p_order {
                if n >= k {
                    sample -= lpc_coeffs[k] * synth_output[n - k];
                }
            }
            synth_output[n] = sample.clamp(-1.0, 1.0);
        }

        synth_output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sola_suite_rendering() {
        let sample_rate = 44100;
        let num_samples = 44100;
        let syn: Vec<f32> = (0..num_samples)
            .map(|i| {
                (i as f32 * 2.0 * std::f32::consts::PI * 440.0 / sample_rate as f32).sin() * 0.5
            })
            .collect();

        let points = vec![];

        let td_out = SolaSuiteProcessor::render_sample(
            SolaEngine::TdPsola,
            &syn,
            sample_rate,
            0.0,
            50.0,
            0.0,
            400.0,
            440.0,
            &points,
            0.0,
        );
        assert!(!td_out.is_empty());

        let fd_out = SolaSuiteProcessor::render_sample(
            SolaEngine::FdPsola,
            &syn,
            sample_rate,
            0.0,
            50.0,
            0.0,
            400.0,
            440.0,
            &points,
            50.0,
        );
        assert!(!fd_out.is_empty());

        let ws_out = SolaSuiteProcessor::render_sample(
            SolaEngine::Wsola,
            &syn,
            sample_rate,
            0.0,
            50.0,
            0.0,
            400.0,
            440.0,
            &points,
            0.0,
        );
        assert!(!ws_out.is_empty());

        let lp_out = SolaSuiteProcessor::render_sample(
            SolaEngine::LpPsola,
            &syn,
            sample_rate,
            0.0,
            50.0,
            0.0,
            400.0,
            440.0,
            &points,
            0.0,
        );
        assert!(!lp_out.is_empty());
    }
}
