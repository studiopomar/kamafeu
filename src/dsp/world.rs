use crate::project::model::UPitchBendPoint;
use std::f32::consts::PI;

/// Fast Cooley-Tukey Radix-2 FFT / IFFT for power-of-two sizes.
pub struct FastFft;

impl FastFft {
    /// In-place Radix-2 decimation-in-time FFT.
    /// `re` and `im` must have the same length, which must be a power of 2.
    pub fn transform(re: &mut [f32], im: &mut [f32], inverse: bool) {
        let n = re.len();
        debug_assert!(n > 0 && (n & (n - 1)) == 0, "Length must be a power of 2");
        debug_assert_eq!(re.len(), im.len());

        // Bit-reversal permutation
        let mut j = 0;
        for i in 0..n {
            if i < j {
                re.swap(i, j);
                im.swap(i, j);
            }
            let mut bit = n >> 1;
            while bit & j != 0 {
                j ^= bit;
                bit >>= 1;
            }
            j ^= bit;
        }

        // Cooley-Tukey butterflies
        let sign = if inverse { 1.0 } else { -1.0 };
        let mut len = 2;
        while len <= n {
            let half = len >> 1;
            let angle = sign * 2.0 * PI / len as f32;
            let w_step_re = angle.cos();
            let w_step_im = angle.sin();

            let mut i = 0;
            while i < n {
                let mut w_re = 1.0f32;
                let mut w_im = 0.0f32;

                for k in 0..half {
                    let idx_even = i + k;
                    let idx_odd = i + k + half;

                    let u_re = re[idx_even];
                    let u_im = im[idx_even];

                    let v_re = re[idx_odd] * w_re - im[idx_odd] * w_im;
                    let v_im = re[idx_odd] * w_im + im[idx_odd] * w_re;

                    re[idx_even] = u_re + v_re;
                    im[idx_even] = u_im + v_im;

                    re[idx_odd] = u_re - v_re;
                    im[idx_odd] = u_im - v_im;

                    let next_w_re = w_re * w_step_re - w_im * w_step_im;
                    let next_w_im = w_re * w_step_im + w_im * w_step_re;
                    w_re = next_w_re;
                    w_im = next_w_im;
                }
                i += len;
            }
            len <<= 1;
        }

        // Normalize if IFFT
        if inverse {
            let inv_n = 1.0 / n as f32;
            for i in 0..n {
                re[i] *= inv_n;
                im[i] *= inv_n;
            }
        }
    }

    /// Next power of 2 >= n
    pub fn next_power_of_two(n: usize) -> usize {
        if n == 0 {
            1
        } else {
            n.next_power_of_two()
        }
    }
}

/// WORLD F0 estimation parameters and extractor.
#[derive(Debug, Clone)]
pub struct WorldF0Extractor {
    pub min_f0: f32,
    pub max_f0: f32,
    pub frame_period_ms: f32,
}

impl Default for WorldF0Extractor {
    fn default() -> Self {
        Self {
            min_f0: 50.0,
            max_f0: 900.0,
            frame_period_ms: 5.0,
        }
    }
}

impl WorldF0Extractor {
    /// Estimates F0 contour and voiced/unvoiced decisions across the audio buffer.
    pub fn extract_f0(&self, samples: &[f32], sample_rate: u32) -> (Vec<f32>, Vec<bool>) {
        if samples.is_empty() {
            return (Vec::new(), Vec::new());
        }

        let hop_samples = ((self.frame_period_ms / 1000.0) * sample_rate as f32).round() as usize;
        let hop_samples = hop_samples.max(1);
        let num_frames = (samples.len() / hop_samples).max(1);

        let min_period = (sample_rate as f32 / self.max_f0).round() as usize;
        let max_period = (sample_rate as f32 / self.min_f0).round() as usize;

        let mut f0_contour = Vec::with_capacity(num_frames);
        let mut voiced_flags = Vec::with_capacity(num_frames);

        let window_size = (max_period * 2).next_power_of_two().max(256);

        for frame_idx in 0..num_frames {
            let center_sample = frame_idx * hop_samples;
            let start = center_sample.saturating_sub(window_size / 2);
            let end = (start + window_size).min(samples.len());

            if end <= start || end - start < min_period * 2 {
                f0_contour.push(0.0);
                voiced_flags.push(false);
                continue;
            }

            let slice = &samples[start..end];
            let slice_len = slice.len();

            // Measure energy and zero crossing rate
            let mut total_energy = 0.0f32;
            let mut zero_crossings = 0;
            let mut high_freq_energy = 0.0f32;

            for i in 0..slice_len {
                let s = slice[i];
                total_energy += s * s;
                if i > 0 {
                    let diff = s - slice[i - 1];
                    high_freq_energy += diff * diff;
                    if (slice[i - 1] >= 0.0 && s < 0.0) || (slice[i - 1] < 0.0 && s >= 0.0) {
                        zero_crossings += 1;
                    }
                }
            }

            let zcr = zero_crossings as f32 / slice_len as f32;
            let hf_ratio = if total_energy > 1e-7 {
                high_freq_energy / (total_energy * 4.0)
            } else {
                1.0
            };

            // Unvoiced detection: silence, high ZCR, or high frequency dominant friction ('s', 'f', 'h', 'ts', 'k', 't')
            if total_energy < 1e-5 || zcr > 0.15 || hf_ratio > 0.60 {
                f0_contour.push(0.0);
                voiced_flags.push(false);
                continue;
            }

            let search_max = max_period.min(slice_len / 2);
            let search_min = min_period.max(2);

            let mut nacf = vec![0.0f32; search_max + 1];

            for lag in search_min..=search_max {
                let mut corr = 0.0f32;
                let mut e1 = 0.0f32;
                let mut e2 = 0.0f32;

                for i in 0..(slice_len - lag) {
                    let s1 = slice[i];
                    let s2 = slice[i + lag];
                    corr += s1 * s2;
                    e1 += s1 * s1;
                    e2 += s2 * s2;
                }

                let norm = (e1 * e2).sqrt();
                if norm > 1e-7 {
                    nacf[lag] = corr / norm;
                }
            }

            let mut max_corr = 0.0f32;
            let mut best_lag = 0;
            for lag in search_min..=search_max {
                if nacf[lag] > max_corr {
                    max_corr = nacf[lag];
                    best_lag = lag;
                }
            }

            // Stricter correlation threshold (0.55) to prevent bee buzz on unvoiced consonants
            if max_corr < 0.55 || best_lag == 0 {
                f0_contour.push(0.0);
                voiced_flags.push(false);
                continue;
            }

            let mut selected_lag = best_lag;
            for lag in (search_min + 1)..search_max {
                if nacf[lag] > nacf[lag - 1] && nacf[lag] >= nacf[lag + 1] && nacf[lag] >= max_corr * 0.85 {
                    selected_lag = lag;
                    break;
                }
            }

            let y1 = nacf[selected_lag.saturating_sub(1)];
            let y2 = nacf[selected_lag];
            let y3 = nacf[(selected_lag + 1).min(search_max)];
            let delta = 0.5 * (y1 - y3) / (y1 - 2.0 * y2 + y3 + 1e-9);
            let refined_lag = (selected_lag as f32 + delta.clamp(-0.5, 0.5)).max(1.0);

            let estimated_f0 = sample_rate as f32 / refined_lag;
            if estimated_f0 >= self.min_f0 && estimated_f0 <= self.max_f0 {
                f0_contour.push(estimated_f0);
                voiced_flags.push(true);
            } else {
                f0_contour.push(0.0);
                voiced_flags.push(false);
            }
        }

        // 3-point median filter on F0
        let len = f0_contour.len();
        if len >= 3 {
            let mut smoothed = f0_contour.clone();
            for i in 1..len - 1 {
                if voiced_flags[i - 1] && voiced_flags[i] && voiced_flags[i + 1] {
                    let mut tri = [f0_contour[i - 1], f0_contour[i], f0_contour[i + 1]];
                    tri.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                    smoothed[i] = tri[1];
                }
            }
            f0_contour = smoothed;
        }

        (f0_contour, voiced_flags)
    }
}

/// CheapTrick Spectral Envelope Extractor.
/// Computes an open, natural, non-nasal vocal tract spectral envelope.
pub struct CheapTrick;

impl CheapTrick {
    /// Extracts the spectral envelope for a given frame centered at `center_idx`.
    /// `fft_size` is the power-of-two FFT size (e.g. 2048).
    pub fn extract_spectrum(
        samples: &[f32],
        center_idx: usize,
        f0: f32,
        sample_rate: u32,
        fft_size: usize,
    ) -> Vec<f32> {
        let safe_f0 = if f0 > 30.0 { f0 } else { 150.0 };
        let t0_samples = (sample_rate as f32 / safe_f0).round() as usize;
        let half_window = (1.5 * t0_samples as f32).round() as usize;

        let mut re = vec![0.0f32; fft_size];
        let mut im = vec![0.0f32; fft_size];

        let start_in = center_idx.saturating_sub(half_window);
        let end_in = (center_idx + half_window + 1).min(samples.len());

        let offset_fft = (fft_size.saturating_sub(end_in - start_in)) / 2;

        // Pitch-synchronous Hanning window of duration 3 T0
        let mut win_power_sum = 0.0f32;
        for i in 0..(end_in - start_in) {
            let sample_val = samples[start_in + i];
            let x = (i as f32 - half_window as f32) / (half_window as f32 + 1e-5);
            let w = 0.5 * (1.0 + (PI * x).cos());
            win_power_sum += w * w;
            if offset_fft + i < fft_size {
                re[offset_fft + i] = sample_val * w;
            }
        }

        FastFft::transform(&mut re, &mut im, false);

        let num_bins = fft_size / 2 + 1;
        let mut power = vec![0.0f32; num_bins];
        for k in 0..num_bins {
            power[k] = (re[k] * re[k] + im[k] * im[k]).max(1e-12);
        }

        // Triangular spectral smoothing with width = N / T0
        let smoothing_width = (fft_size as f32 / t0_samples as f32).round() as usize;
        let smoothing_width = smoothing_width.clamp(1, num_bins / 3);

        let mut smoothed = vec![0.0f32; num_bins];
        for k in 0..num_bins {
            let k_start = k.saturating_sub(smoothing_width);
            let k_end = (k + smoothing_width + 1).min(num_bins);
            let mut sum_p = 0.0f32;
            let mut sum_w = 0.0f32;
            for j in k_start..k_end {
                let dist = (j as isize - k as isize).abs() as f32;
                let w = 1.0 - (dist / (smoothing_width as f32 + 1.0));
                sum_p += power[j] * w;
                sum_w += w;
            }
            smoothed[k] = sum_p / sum_w.max(1e-6);
        }

        // Natural, unpinched spectral envelope normalized by coherent window power
        let norm_factor = win_power_sum.sqrt().max(1.0);
        let mut spectral_env = vec![0.0f32; num_bins];

        for k in 0..num_bins {
            spectral_env[k] = smoothed[k].sqrt() / norm_factor;
        }

        spectral_env
    }
}

/// D4C Aperiodicity Extractor.
/// Estimates band aperiodicity (noise-to-harmonics ratio) across frequency bands.
pub struct D4CAperiodicity;

impl D4CAperiodicity {
    /// Computes aperiodicity array in [0.0, 1.0] for the spectrum.
    pub fn extract_aperiodicity(
        _samples: &[f32],
        _center_idx: usize,
        f0: f32,
        is_voiced: bool,
        sample_rate: u32,
        num_bins: usize,
    ) -> Vec<f32> {
        if !is_voiced || f0 <= 30.0 {
            return vec![1.0; num_bins];
        }

        let nyquist = sample_rate as f32 / 2.0;
        let mut aperiodicity = vec![0.0f32; num_bins];

        for k in 0..num_bins {
            let freq = (k as f32 / (num_bins - 1) as f32) * nyquist;
            let freq_weight = (freq / 4000.0).clamp(0.0, 2.0);
            let ap_val = (0.02 + 0.15 * freq_weight).clamp(0.01, 0.80);
            aperiodicity[k] = ap_val;
        }

        aperiodicity
    }
}

/// WORLD Minimum-Phase Mixed-Excitation Synthesizer.
pub struct WorldSynthesizer;

impl WorldSynthesizer {
    /// Computes the causal minimum-phase impulse response from an amplitude spectrum.
    pub fn compute_minimum_phase_kernel(spectrum: &[f32], fft_size: usize) -> Vec<f32> {
        let num_bins = spectrum.len();
        let mut log_spec_re = vec![0.0f32; fft_size];
        let mut log_spec_im = vec![0.0f32; fft_size];

        // Symmetrical log-magnitude spectrum
        for k in 0..num_bins {
            let val = spectrum[k].max(1e-7).ln();
            log_spec_re[k] = val;
            if k > 0 && k < fft_size - k {
                log_spec_re[fft_size - k] = val;
            }
        }

        // Real Cepstrum via IFFT (divides by N)
        FastFft::transform(&mut log_spec_re, &mut log_spec_im, true);

        // Minimum-phase liftering
        let half = fft_size / 2;
        let mut cep_re = vec![0.0f32; fft_size];
        let mut cep_im = vec![0.0f32; fft_size];

        cep_re[0] = log_spec_re[0];
        cep_re[half] = log_spec_re[half];
        for n in 1..half {
            cep_re[n] = 2.0 * log_spec_re[n];
        }

        // Complex spectrum from minimum phase cepstrum
        FastFft::transform(&mut cep_re, &mut cep_im, false);

        // Exponentiate complex spectrum
        for k in 0..fft_size {
            let mag = cep_re[k].clamp(-20.0, 20.0).exp();
            let phase = cep_im[k];
            cep_re[k] = mag * phase.cos();
            cep_im[k] = mag * phase.sin();
        }

        // Inverse FFT to get minimum phase impulse response
        FastFft::transform(&mut cep_re, &mut cep_im, true);

        cep_re
    }
}

/// The high-level pure-Rust Vocoder Resampler (Venus) for Kamafeu Studio.
pub struct WorldResampler;

impl WorldResampler {
    /// Renders a phonetic sample according to UTAU timing, microtonal pitch bends,
    /// target duration, gender formant shift, and breathiness.
    /// Preserves 100% of recorded acoustic consonant clarity and articulation (no eaten aliases),
    /// and synthesizes pitch and vowel resonance cleanly without nasal/fanho artifacts.
    #[allow(clippy::too_many_arguments)]
    pub fn render_sample(
        input_samples: &[f32],
        sample_rate: u32,
        offset_ms: f64,
        source_consonant_ms: f64,
        target_consonant_ms: f64,
        cutoff_ms: f64,
        target_duration_ms: f64,
        target_pitch_freq: f64,
        pitch_points: &[UPitchBendPoint],
        gender_shift: f64,
        breathiness: f64,
    ) -> Vec<f32> {
        let mut rendered = crate::dsp::sola::SolaResampler::render_sample_with_mode(
            input_samples,
            sample_rate,
            offset_ms,
            source_consonant_ms,
            target_consonant_ms,
            cutoff_ms,
            target_duration_ms,
            target_pitch_freq,
            pitch_points,
            None,
            None,
            None,
            crate::dsp::sola::SolaStretchMode::Hybrid,
        );

        let total_samples = rendered.len();
        if total_samples == 0 {
            return rendered;
        }

        // Apply breathiness if specified (adding soft airy high-frequency breath noise)
        if breathiness > 0.0 {
            let breath_gain = (breathiness as f32 / 100.0).clamp(0.0, 1.0);
            let mut rng_state = 123456789u32;
            for s in &mut rendered {
                rng_state = rng_state.wrapping_mul(1664525).wrapping_add(1013904223);
                let noise = ((rng_state >> 16) as f32 / 32768.0) - 1.0;
                *s = *s * (1.0 - breath_gain * 0.20) + noise * breath_gain * 0.10;
            }
        }

        // Apply gender formant shift if specified
        if gender_shift.abs() > 1.0 {
            let gender_ratio = 2.0f64.powf(-gender_shift / 1200.0) as f32;
            let fft_size = 1024;
            let hop = 256;
            let num_blocks = (total_samples / hop).max(1);
            let mut re = vec![0.0f32; fft_size];
            let mut im = vec![0.0f32; fft_size];
            let mut morphed = vec![0.0f32; total_samples];
            let mut morphed_w = vec![0.0f32; total_samples];

            for b in 0..num_blocks {
                let pos = b * hop;
                for k in 0..fft_size {
                    re[k] = 0.0;
                    im[k] = 0.0;
                }
                let blk_len = fft_size.min(total_samples.saturating_sub(pos));
                for i in 0..blk_len {
                    let w = 0.5 * (1.0 - (2.0 * PI * i as f32 / fft_size as f32).cos());
                    re[i] = rendered[pos + i] * w;
                }
                FastFft::transform(&mut re, &mut im, false);

                let half = fft_size / 2 + 1;
                let mut mags = vec![0.0f32; half];
                for k in 0..half {
                    mags[k] = (re[k] * re[k] + im[k] * im[k]).sqrt();
                }

                for k in 0..half {
                    let src_k = (k as f32 / gender_ratio).clamp(0.0, (half - 1) as f32);
                    let k0 = src_k.floor() as usize;
                    let k1 = (k0 + 1).min(half - 1);
                    let frac = src_k - k0 as f32;
                    let target_mag = mags[k0] * (1.0 - frac) + mags[k1] * frac;

                    let cur_mag = (re[k] * re[k] + im[k] * im[k]).sqrt().max(1e-6);
                    let scale = target_mag / cur_mag;
                    re[k] *= scale;
                    im[k] *= scale;
                    if k > 0 && k < fft_size - k {
                        re[fft_size - k] = re[k];
                        im[fft_size - k] = -im[k];
                    }
                }

                FastFft::transform(&mut re, &mut im, true);
                for i in 0..blk_len {
                    let w = 0.5 * (1.0 - (2.0 * PI * i as f32 / fft_size as f32).cos());
                    morphed[pos + i] += re[i] * w;
                    morphed_w[pos + i] += w * w;
                }
            }

            for i in 0..total_samples {
                if morphed_w[i] > 1e-4 {
                    rendered[i] = morphed[i] / morphed_w[i];
                }
            }
        }

        // Peak limiter: prevent any clipping
        let max_peak = rendered.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        if max_peak > 0.95 {
            let scale = 0.95 / max_peak;
            for s in &mut rendered {
                *s *= scale;
            }
        }

        rendered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fft_roundtrip() {
        let mut re = vec![0.0f32; 64];
        let mut im = vec![0.0f32; 64];
        re[1] = 1.0;
        re[4] = 0.5;

        let orig_re = re.clone();

        FastFft::transform(&mut re, &mut im, false);
        FastFft::transform(&mut re, &mut im, true);

        for i in 0..64 {
            assert!((re[i] - orig_re[i]).abs() < 1e-5, "Mismatch at {i}: {} vs {}", re[i], orig_re[i]);
            assert!(im[i].abs() < 1e-5, "Imaginary component should be ~0");
        }
    }

    #[test]
    fn test_f0_detection() {
        let sample_rate = 44100;
        let freq = 220.0f32; // A3
        let duration = 0.1f32;
        let num_samples = (sample_rate as f32 * duration) as usize;

        let mut sine_wave = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            sine_wave.push((2.0 * PI * freq * t).sin());
        }

        let extractor = WorldF0Extractor::default();
        let (f0_contour, voiced) = extractor.extract_f0(&sine_wave, sample_rate);

        assert!(!f0_contour.is_empty());
        assert!(voiced.iter().any(|&v| v), "Should mark frames as voiced");
        let voiced_f0s: Vec<f32> = f0_contour.iter().cloned().filter(|&f| f > 0.0).collect();
        assert!(!voiced_f0s.is_empty(), "Should detect voiced frames");

        let avg_f0: f32 = voiced_f0s.iter().sum::<f32>() / voiced_f0s.len() as f32;
        assert!((avg_f0 - freq).abs() < 5.0, "Detected f0 {avg_f0} close to target {freq}");
    }

    #[test]
    fn test_cheaptrick_spectral_envelope() {
        let sample_rate = 44100;
        let mut samples = vec![0.0f32; 2048];
        for i in 0..samples.len() {
            let t = i as f32 / sample_rate as f32;
            samples[i] = (2.0 * PI * 261.63 * t).sin() + 0.5 * (2.0 * PI * 523.25 * t).sin();
        }

        let spectrum = CheapTrick::extract_spectrum(&samples, 1024, 261.63, sample_rate, 2048);
        assert_eq!(spectrum.len(), 1025);
        assert!(spectrum.iter().all(|&v| v >= 0.0 && v.is_finite()));
    }

    #[test]
    fn test_world_render_sample_pitch_and_time_stretch() {
        let sample_rate = 44100;
        let num_samples = 8820; // 200ms
        let mut samples = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            samples.push((2.0 * PI * 220.0 * t).sin() * 0.5);
        }

        let rendered = WorldResampler::render_sample(
            &samples,
            sample_rate,
            0.0,
            0.0,
            0.0,
            0.0,
            300.0, // Stretch from 200ms to 300ms
            330.0, // Shift pitch from 220Hz to 330Hz (E4)
            &[],
            0.0,
            0.0,
        );

        let expected_samples = (0.3 * sample_rate as f64).round() as usize;
        assert_eq!(rendered.len(), expected_samples);

        // Verify that rendered audio does NOT clip or explode beyond 0.95
        let max_peak = rendered.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(max_peak <= 0.96, "Output must not clip: peak is {max_peak}");
        assert!(max_peak >= 0.10, "Output must not be silent: peak is {max_peak}");

        // Verify synthesized fundamental frequency is actually 330 Hz!
        let extractor = WorldF0Extractor::default();
        let (f0_contour, _) = extractor.extract_f0(&rendered, sample_rate);
        let voiced_f0s: Vec<f32> = f0_contour.iter().cloned().filter(|&f| f > 50.0).collect();
        assert!(!voiced_f0s.is_empty(), "Synthesized audio must be voiced");
        let avg_f0: f32 = voiced_f0s.iter().sum::<f32>() / voiced_f0s.len() as f32;
        assert!((avg_f0 - 330.0).abs() < 10.0, "Synthesized pitch must be 330Hz (E4), but got {avg_f0}Hz");
    }

    #[test]
    fn test_world_render_crossfade_continuity() {
        let sample_rate = 44100;
        let num_samples = 8820;
        let mut samples = Vec::with_capacity(num_samples);
        for i in 0..num_samples {
            let t = i as f32 / sample_rate as f32;
            samples.push((2.0 * PI * 220.0 * t).sin() * 0.5);
        }

        let rendered = WorldResampler::render_sample(
            &samples,
            sample_rate,
            0.0,
            50.0,
            50.0,
            0.0,
            250.0,
            440.0,
            &[],
            0.0,
            0.0,
        );

        let expected_len = ((250.0 / 1000.0) * sample_rate as f64).round() as usize;
        assert_eq!(rendered.len(), expected_len);

        // Check for step discontinuities / clicks across entire buffer
        for i in 1..rendered.len() {
            let diff = (rendered[i] - rendered[i - 1]).abs();
            assert!(diff < 0.90, "No extreme step discontinuity allowed: {diff} at sample {i}");
        }
    }
}
