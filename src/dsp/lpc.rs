pub struct LpcExtractor;

impl LpcExtractor {
    pub fn extract_lpc_coefficients(frame: &[f32], order: usize) -> Vec<f32> {
        let n = frame.len();
        if n == 0 || order == 0 {
            return vec![1.0];
        }

        let mut r = vec![0.0f32; order + 1];

        for i in 0..=order {
            let mut sum = 0.0;
            for j in 0..(n - i) {
                sum += frame[j] * frame[j + i];
            }
            r[i] = sum;
        }

        let mut a = vec![0.0f32; order + 1];
        a[0] = 1.0;

        if r[0] <= 1e-7 {
            return a;
        }

        let mut e = r[0];

        for k in 1..=order {
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

        a
    }

    pub fn extract_residual(signal: &[f32], lpc_coeffs: &[f32]) -> Vec<f32> {
        let p = lpc_coeffs.len() - 1;
        let mut residual = vec![0.0f32; signal.len()];

        for n in 0..signal.len() {
            let mut res = signal[n];
            for k in 1..=p {
                if n >= k {
                    res += lpc_coeffs[k] * signal[n - k];
                }
            }
            residual[n] = res;
        }

        residual
    }

    pub fn synthesize(excitation: &[f32], lpc_coeffs: &[f32]) -> Vec<f32> {
        let p = lpc_coeffs.len() - 1;
        let mut synthesized = vec![0.0f32; excitation.len()];

        for n in 0..excitation.len() {
            let mut val = excitation[n];
            for k in 1..=p {
                if n >= k {
                    val -= lpc_coeffs[k] * synthesized[n - k];
                }
            }
            // Clamp to ±1.0 to prevent unstable filter divergence from causing
            // hearing-damaging audio spikes. The previous limit of ±2.0 was
            // incorrect and allowed twice the maximum sample amplitude.
            synthesized[n] = val.clamp(-1.0, 1.0);
        }

        // If the filter was near-unstable, the peak may still be very high.
        // Normalize the output so it never exceeds the input peak amplitude,
        // preserving the relative dynamics without introducing DC offset.
        let output_peak = synthesized.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        let input_peak = excitation.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        if output_peak > 1e-6 && input_peak > 1e-6 && output_peak > input_peak {
            let scale = input_peak / output_peak;
            for s in &mut synthesized {
                *s *= scale;
            }
        }

        synthesized
    }
}
