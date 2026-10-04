//! Análise de F0 e FFT usada pelo pipeline WORLD.

use std::f32::consts::PI;

/// FFT / IFFT rápida Cooley-Tukey Radix-2 para tamanhos potências de 2.
pub struct FastFft;

impl FastFft {
    /// FFT Radix-2 in-place por decimação no tempo.
    /// `re` e `im` devem ter o mesmo tamanho (potência de 2).
    pub fn transform(re: &mut [f32], im: &mut [f32], inverse: bool) {
        let n = re.len();
        debug_assert!(n > 0 && (n & (n - 1)) == 0, "Length must be a power of 2");
        debug_assert_eq!(re.len(), im.len());

        // Permutação bit-reversal
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

        // Borboletas Cooley-Tukey
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

        // Normalização na IFFT
        if inverse {
            let inv_n = 1.0 / n as f32;
            for i in 0..n {
                re[i] *= inv_n;
                im[i] *= inv_n;
            }
        }
    }

    /// Próxima potência de 2 >= n.
    pub fn next_power_of_two(n: usize) -> usize {
        if n == 0 {
            1
        } else {
            n.next_power_of_two()
        }
    }
}

/// Extrator e parâmetros de F0 do algoritmo WORLD.
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
    /// Estima contorno de F0 e decisões vozeado/não-vozeado no áudio.
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

            // Mede energia e taxa de cruzamento por zero (ZCR)
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

            // Detecção de não-vozeado: silêncio, ZCR alto ou fricção aguda ('s', 'f', 'h', 'ts', 'k', 't')
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

            // Limiar estrito (0.55) para evitar artefatos em consoantes surdas
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

        // Filtro da mediana de 3 pontos para suavizar F0
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

        // Repair a one-frame voicing dropout only when both neighbours agree.
        // It keeps an analysis region continuous without turning a real
        // unvoiced consonant into voiced audio. The geometric mean is the
        // correct midpoint for pitch ratios and avoids octave bias.
        if len >= 3 {
            for index in 1..len - 1 {
                let left = f0_contour[index - 1];
                let right = f0_contour[index + 1];
                let neighbour_cents = if left > 0.0 && right > 0.0 {
                    1_200.0 * (left / right).log2().abs()
                } else {
                    f32::INFINITY
                };
                if !voiced_flags[index]
                    && voiced_flags[index - 1]
                    && voiced_flags[index + 1]
                    && neighbour_cents < 80.0
                {
                    f0_contour[index] = (left * right).sqrt();
                    voiced_flags[index] = true;
                }
            }
        }

        (f0_contour, voiced_flags)
    }
}
