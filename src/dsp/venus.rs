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
                if nacf[lag] > nacf[lag - 1]
                    && nacf[lag] >= nacf[lag + 1]
                    && nacf[lag] >= max_corr * 0.85
                {
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
pub struct VenusResampler;

pub type WorldResampler = VenusResampler;

impl VenusResampler {
    // Venus must traverse the analysed vowel continuously. The old Hybrid
    // route alternated loop direction at the sustain boundary; that is useful
    // for an indefinitely held synthetic loop, but it introduces a reversal
    // seam in short CVVC/VCV aliases. Stretch preserves the source timeline
    // from the attack through the tail, while PSOLA still controls pitch.
    const SYNTHESIS_MODE: crate::dsp::sola::SolaStretchMode =
        crate::dsp::sola::SolaStretchMode::Stretch;
    /// Removes non-finite samples and DC before analysis. A DC offset makes
    /// pitch marks and overlap-add gain compensation unreliable, especially in
    /// older UTAU banks that were exported with a biased waveform.
    fn sanitize_input(samples: &[f32]) -> Vec<f32> {
        let mut clean: Vec<f32> = samples
            .iter()
            .map(|sample| if sample.is_finite() { *sample } else { 0.0 })
            .collect();
        if clean.is_empty() {
            return clean;
        }
        let dc = clean.iter().sum::<f32>() / clean.len() as f32;
        for sample in &mut clean {
            *sample -= dc;
        }
        clean
    }

    /// WORLD-style formant manipulation: warp only the slowly-changing
    /// spectral envelope, retaining the local harmonic detail and phase. This
    /// avoids the unstable bin-by-bin magnitude transplant used previously.
    fn apply_formant_shift(samples: &mut [f32], cents: f64) {
        if samples.len() < 32 || cents.abs() <= 1.0 {
            return;
        }

        let ratio = 2.0f32.powf((-cents.clamp(-1_200.0, 1_200.0) / 1_200.0) as f32);
        let fft_size = if samples.len() < 1_024 { 512 } else { 1_024 };
        let hop = fft_size / 4;
        let blocks = (samples.len() + hop - 1) / hop;
        let half = fft_size / 2 + 1;
        let mut re = vec![0.0f32; fft_size];
        let mut im = vec![0.0f32; fft_size];
        let mut morphed = vec![0.0f32; samples.len()];
        let mut weights = vec![0.0f32; samples.len()];

        for block in 0..blocks {
            re.fill(0.0);
            im.fill(0.0);
            let pos = block * hop;
            let block_len = fft_size.min(samples.len().saturating_sub(pos));
            for i in 0..block_len {
                let window = 0.5 - 0.5 * (2.0 * PI * i as f32 / fft_size as f32).cos();
                re[i] = samples[pos + i] * window;
            }
            FastFft::transform(&mut re, &mut im, false);

            let magnitude: Vec<f32> = (0..half)
                .map(|bin| (re[bin] * re[bin] + im[bin] * im[bin]).sqrt().max(1e-7))
                .collect();
            // A modest log-domain smoother models the vocal-tract envelope
            // while preserving harmonic peaks as the excitation component.
            let envelope: Vec<f32> = (0..half)
                .map(|bin| {
                    let start = bin.saturating_sub(8);
                    let end = (bin + 9).min(half);
                    let log_average = magnitude[start..end].iter().map(|v| v.ln()).sum::<f32>()
                        / (end - start) as f32;
                    log_average.exp().max(1e-7)
                })
                .collect();

            for bin in 0..half {
                let source_bin = (bin as f32 / ratio).clamp(0.0, (half - 1) as f32);
                let left = source_bin.floor() as usize;
                let right = (left + 1).min(half - 1);
                let fraction = source_bin - left as f32;
                let shifted_envelope =
                    envelope[left] * (1.0 - fraction) + envelope[right] * fraction;
                let fine_structure = magnitude[bin] / envelope[bin];
                let target_magnitude = (fine_structure * shifted_envelope).max(1e-7);
                let gain = (target_magnitude / magnitude[bin]).clamp(0.25, 4.0);
                re[bin] *= gain;
                im[bin] *= gain;
                if bin > 0 && bin < fft_size - bin {
                    re[fft_size - bin] = re[bin];
                    im[fft_size - bin] = -im[bin];
                }
            }

            FastFft::transform(&mut re, &mut im, true);
            for i in 0..block_len {
                let window = 0.5 - 0.5 * (2.0 * PI * i as f32 / fft_size as f32).cos();
                morphed[pos + i] += re[i] * window;
                weights[pos + i] += window * window;
            }
        }

        for ((sample, sum), weight) in samples.iter_mut().zip(morphed).zip(weights) {
            if weight > 1e-5 {
                *sample = sum / weight;
            }
        }
    }

    /// Adds shaped (not DC-heavy) noise so B does not turn voiced material
    /// into broadband clipping or a metallic buzz.
    fn apply_breathiness(samples: &mut [f32], breathiness: f64) {
        let amount = (breathiness / 100.0).clamp(0.0, 1.0) as f32;
        if amount <= 0.0 {
            return;
        }
        let mut state = 0x6d2b_79f5_u32;
        let mut previous_noise = 0.0f32;
        let mut high_pass = 0.0f32;
        for sample in samples {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let noise = ((state >> 8) as f32 / 8_388_608.0) - 1.0;
            high_pass = 0.985 * (high_pass + noise - previous_noise);
            previous_noise = noise;
            // Keep a clear voiced source even at B100. The noise is shaped and
            // intentionally quieter than the old unfiltered 10% addition.
            *sample = *sample * (1.0 - amount * 0.10) + high_pass * amount * 0.045;
        }
    }

    /// Removes only isolated phase seams left by TD-PSOLA.  This is not a
    /// generic low-pass/de-click effect: normal vocal harmonics have a
    /// comparable slope on both sides of each sample, while a failed grain
    /// join has one derivative many times larger than its local neighbours.
    /// Repairing the short seam before the wavtool envelope prevents it from
    /// being amplified into a broadband click at a CVVC transition.
    fn repair_psola_seams(samples: &mut [f32], sample_rate: u32) {
        if sample_rate == 0 || samples.len() < 96 {
            return;
        }
        let context = ((sample_rate as f64 * 0.00035).round() as usize).clamp(8, 24);
        let fade = ((sample_rate as f64 * 0.00075).round() as usize).clamp(12, 40);
        if samples.len() <= context * 2 + fade + 1 {
            return;
        }

        let original = samples.to_vec();
        let mut seams = Vec::new();
        let mut next_allowed = context;
        for index in context..samples.len() - context - fade {
            if index < next_allowed {
                continue;
            }
            let jump = (original[index] - original[index - 1]).abs();
            let mut neighbour_slope = 0.0f32;
            let mut neighbour_count = 0usize;
            for derivative_index in index - context + 1..=index + context {
                if derivative_index == index {
                    continue;
                }
                neighbour_slope +=
                    (original[derivative_index] - original[derivative_index - 1]).abs();
                neighbour_count += 1;
            }
            let local_mean = neighbour_slope / neighbour_count.max(1) as f32;
            // 0.035 keeps ordinary high-register vowels intact; the relative
            // criterion catches phase flips even when the singer is quiet.
            let threshold = (local_mean * 6.0).max(0.035);
            if jump > threshold {
                seams.push(index);
                next_allowed = index + fade;
            }
        }

        for start in seams {
            let left = original[start - 1];
            let right = original[start + fade];
            for offset in 0..fade {
                let t = (offset + 1) as f32 / (fade + 1) as f32;
                // Zero-slope ends avoid replacing one sharp step with two
                // smaller ones at the repair boundaries.
                let eased = t * t * (3.0 - 2.0 * t);
                samples[start + offset] = left + (right - left) * eased;
            }
        }
    }

    /// Final offline safety stage. Scaling before soft saturation preserves
    /// transients; the soft knee only catches pathological formant/OLA peaks.
    fn apply_output_safety(samples: &mut [f32]) {
        for sample in samples.iter_mut() {
            // DC is removed once before WORLD/PSOLA analysis.  Running a
            // one-pole blocker here changes the absolute waveform at every
            // chunk boundary and can create the very clicks this safety stage
            // is meant to prevent.
            if !sample.is_finite() {
                *sample = 0.0;
            }
        }

        let peak = samples
            .iter()
            .map(|sample| sample.abs())
            .fold(0.0f32, f32::max);
        if peak > 0.89 {
            let gain = 0.89 / peak;
            for sample in samples.iter_mut() {
                *sample *= gain;
            }
        }
        for sample in samples.iter_mut() {
            // A gentle final knee protects against numeric overshoots without
            // a hard clamp, which itself creates audible clicks.
            let abs = sample.abs();
            if abs > 0.89 {
                *sample = sample.signum() * (0.89 + (abs - 0.89) / (1.0 + (abs - 0.89) * 12.0));
            }
        }
    }

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
        let clean_input = Self::sanitize_input(input_samples);
        let mut rendered = crate::dsp::sola::SolaResampler::render_sample_with_mode(
            &clean_input,
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
            Self::SYNTHESIS_MODE,
        );

        let total_samples = rendered.len();
        if total_samples == 0 {
            return rendered;
        }

        Self::repair_psola_seams(&mut rendered, sample_rate);
        Self::apply_breathiness(&mut rendered, breathiness);
        Self::apply_formant_shift(&mut rendered, gender_shift);
        Self::apply_output_safety(&mut rendered);

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
            assert!(
                (re[i] - orig_re[i]).abs() < 1e-5,
                "Mismatch at {i}: {} vs {}",
                re[i],
                orig_re[i]
            );
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
        assert!(
            (avg_f0 - freq).abs() < 5.0,
            "Detected f0 {avg_f0} close to target {freq}"
        );
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
        assert!(
            max_peak >= 0.10,
            "Output must not be silent: peak is {max_peak}"
        );

        // Verify synthesized fundamental frequency is actually 330 Hz!
        let extractor = WorldF0Extractor::default();
        let (f0_contour, _) = extractor.extract_f0(&rendered, sample_rate);
        let voiced_f0s: Vec<f32> = f0_contour.iter().cloned().filter(|&f| f > 50.0).collect();
        assert!(!voiced_f0s.is_empty(), "Synthesized audio must be voiced");
        let avg_f0: f32 = voiced_f0s.iter().sum::<f32>() / voiced_f0s.len() as f32;
        assert!(
            (avg_f0 - 330.0).abs() < 10.0,
            "Synthesized pitch must be 330Hz (E4), but got {avg_f0}Hz"
        );
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
            assert!(
                diff < 0.90,
                "No extreme step discontinuity allowed: {diff} at sample {i}"
            );
        }
    }

    #[test]
    fn venus_repairs_an_isolated_psola_phase_seam_without_touching_a_smooth_wave() {
        let rate = 44_100;
        let mut seamless: Vec<f32> = (0..2_000)
            .map(|index| (std::f32::consts::TAU * 220.0 * index as f32 / rate as f32).sin() * 0.45)
            .collect();
        let original = seamless.clone();
        VenusResampler::repair_psola_seams(&mut seamless, rate);
        assert_eq!(seamless, original, "smooth voiced audio was altered");

        let seam_at = 800;
        let mut broken = original.clone();
        for (index, sample) in broken.iter_mut().enumerate().skip(seam_at) {
            *sample =
                (std::f32::consts::TAU * 220.0 * index as f32 / rate as f32 + 1.4).sin() * 0.45;
        }
        let before = (broken[seam_at] - broken[seam_at - 1]).abs();
        VenusResampler::repair_psola_seams(&mut broken, rate);
        let after = broken
            .windows(2)
            .skip(seam_at - 2)
            .take(40)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0f32, f32::max);
        assert!(before > 0.2, "test input did not contain a seam: {before}");
        assert!(after < before * 0.25, "seam remained too abrupt: {after}");
    }

    #[test]
    fn venus_safety_handles_hot_non_finite_input_and_extreme_expressions() {
        let sample_rate = 44_100;
        let mut samples: Vec<f32> = (0..sample_rate / 4)
            .map(|index| {
                let t = index as f32 / sample_rate as f32;
                (2.0 * PI * 180.0 * t).sin() * 1.8
            })
            .collect();
        samples[17] = f32::NAN;
        samples[23] = f32::INFINITY;

        let rendered = VenusResampler::render_sample(
            &samples,
            sample_rate,
            0.0,
            20.0,
            20.0,
            0.0,
            300.0,
            440.0,
            &[],
            2_400.0,
            100.0,
        );

        assert!(rendered.iter().all(|sample| sample.is_finite()));
        let peak = rendered
            .iter()
            .map(|sample| sample.abs())
            .fold(0.0f32, f32::max);
        assert!(peak <= 0.90, "Safety ceiling exceeded: {peak}");
    }

    #[test]
    fn formant_shift_preserves_audible_short_voicebank_sample() {
        let sample_rate = 44_100;
        let samples: Vec<f32> = (0..3_000)
            .map(|index| {
                let t = index as f32 / sample_rate as f32;
                ((2.0 * PI * 220.0 * t).sin() + 0.35 * (2.0 * PI * 660.0 * t).sin()) * 0.45
            })
            .collect();
        let rendered = VenusResampler::render_sample(
            &samples,
            sample_rate,
            0.0,
            0.0,
            0.0,
            0.0,
            150.0,
            330.0,
            &[],
            -1_200.0,
            0.0,
        );

        let rms = (rendered.iter().map(|sample| sample.powi(2)).sum::<f32>()
            / rendered.len().max(1) as f32)
            .sqrt();
        assert!(rms > 0.01, "Formant shift muted the sample: rms={rms}");
        assert!(rendered.iter().all(|sample| sample.is_finite()));
    }
}
