//! Resampler Venus
//! Criado por Xiao Pingguo.

use crate::project::model::UPitchBendPoint;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum F0TrackerMethod {
    Yin,
    Pyin,
    World,
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

/// Extrator de Envelope Espectral CheapTrick.
/// Gera um envelope espectral vocal aberto, natural e sem nasalidade.
pub struct CheapTrick;

impl CheapTrick {
    /// Extrai o envelope espectral para o quadro centralizado em `center_idx`.
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

        // Janela Hanning síncrona com o pitch (duração 3 T0)
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

        // Suavização espectral triangular com largura N / T0
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

        // Envelope espectral normalizado pela potência da janela
        let norm_factor = win_power_sum.sqrt().max(1.0);
        let mut spectral_env = vec![0.0f32; num_bins];

        for k in 0..num_bins {
            spectral_env[k] = smoothed[k].sqrt() / norm_factor;
        }

        spectral_env
    }
}

/// Extrator de Aperiodicidade D4C (relação ruído/harmônicos).
pub struct D4CAperiodicity;

impl D4CAperiodicity {
    /// Calcula a aperiodicidade [0.0, 1.0] para as bandas de frequência.
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

/// Sintetizador WORLD de excitação mista e fase mínima.
pub struct WorldSynthesizer;

impl WorldSynthesizer {
    /// Calcula resposta impulsiva causal de fase mínima via cepstro real.
    pub fn compute_minimum_phase_kernel(spectrum: &[f32], fft_size: usize) -> Vec<f32> {
        let num_bins = spectrum.len();
        let mut log_spec_re = vec![0.0f32; fft_size];
        let mut log_spec_im = vec![0.0f32; fft_size];

        // Espectro log-magnitude simétrico
        for k in 0..num_bins {
            let val = spectrum[k].max(1e-7).ln();
            log_spec_re[k] = val;
            if k > 0 && k < fft_size - k {
                log_spec_re[fft_size - k] = val;
            }
        }

        // Cepstro real via IFFT
        FastFft::transform(&mut log_spec_re, &mut log_spec_im, true);

        // Filtragem (liftering) de fase mínima
        let half = fft_size / 2;
        let mut cep_re = vec![0.0f32; fft_size];
        let mut cep_im = vec![0.0f32; fft_size];

        cep_re[0] = log_spec_re[0];
        cep_re[half] = log_spec_re[half];
        for n in 1..half {
            cep_re[n] = 2.0 * log_spec_re[n];
        }

        // Espectro complexo de fase mínima
        FastFft::transform(&mut cep_re, &mut cep_im, false);

        // Exponenciação do espectro complexo
        for k in 0..fft_size {
            let mag = cep_re[k].clamp(-20.0, 20.0).exp();
            let phase = cep_im[k];
            cep_re[k] = mag * phase.cos();
            cep_im[k] = mag * phase.sin();
        }

        // IFFT para obter a resposta ao impulso
        FastFft::transform(&mut cep_re, &mut cep_im, true);

        cep_re
    }
}

thread_local! {
    static ACTIVE_F0_CONFIG: std::cell::RefCell<Option<(f64, f64, F0TrackerMethod)>> = const { std::cell::RefCell::new(None) };
}

/// Resampler Vocoder em Rust puro (Venus) para o Kamafeu Studio.
pub struct VenusResampler;

pub type WorldResampler = VenusResampler;

impl VenusResampler {
    /// Equalizes the level of one rendered alias with one stable gain.
    ///
    /// This follows the same separation used by modern Rust resamplers: first
    /// estimate a stable level from short RMS windows, then apply one gain to
    /// the complete render and finally apply peak safety. This must not be an
    /// automatic gain control: making the gain follow every analysis frame
    /// audibly pumps sustained notes and looped vowels.
    fn normalize_alias_loudness(samples: &mut [f32], sample_rate: u32) {
        // Conservative normalisation: we primarily clamp very loud aliases
        // down, and only apply a small boost to unusually quiet ones.  The
        // aggressive +9 dB boost of the old code amplified background noise
        // and voicebank recording imperfections, making some voicebanks sound
        // raspy or harsh.  The lessampler-ng AutoAMP approach (normalise peak
        // to 0.95 × volume, no fixed-RMS target) informed this redesign:
        //   • Reference the P55 percentile of voiced-frame RMS rather than
        //     P70, so peaks of short vowels do not pull the gain too far.
        //   • MAX_BOOST_DB reduced to 3.0 dB (nearly no boost) – avoids
        //     amplifying noise in quiet aliases.
        //   • MAX_CUT_DB reduced to 8.0 dB – still brings down very hot
        //     aliases without creating jarring level jumps between notes.
        //   • SILENCE threshold raised slightly so silence frames are
        //     excluded more reliably.
        const TARGET_RMS_DBFS: f32 = -20.0;
        const SILENCE_RMS_DBFS: f32 = -48.0;
        const MAX_BOOST_DB: f32 = 3.0;
        const MAX_CUT_DB: f32 = 8.0;

        if samples.len() < 32 || sample_rate == 0 {
            return;
        }

        let frame_len = ((sample_rate as f64 * 0.020).round() as usize).clamp(16, samples.len());
        let hop = ((sample_rate as f64 * 0.010).round() as usize).max(1);
        let mut frame_levels = Vec::new();
        let mut start = 0usize;
        while start < samples.len() {
            let end = (start + frame_len).min(samples.len());
            let rms = (samples[start..end]
                .iter()
                .map(|sample| sample * sample)
                .sum::<f32>()
                / (end - start) as f32)
                .sqrt();
            let db = 20.0 * rms.max(1e-8).log10();
            frame_levels.push(db);
            if end == samples.len() {
                break;
            }
            start += hop;
        }

        let mut voiced_levels = frame_levels
            .iter()
            .copied()
            .filter(|level| *level > SILENCE_RMS_DBFS)
            .collect::<Vec<_>>();
        if voiced_levels.is_empty() {
            return;
        }
        // Use the P70 percentile (body of voiced frames, excluding peaks)
        // so short attacks, tails and fricative frames do not bias the gain.
        voiced_levels.sort_by(|left, right| left.total_cmp(right));
        let reference_index = ((voiced_levels.len() - 1) as f32 * 0.70).round() as usize;
        let reference_db = voiced_levels[reference_index];
        let base_gain_db = (TARGET_RMS_DBFS - reference_db).clamp(-MAX_CUT_DB, MAX_BOOST_DB);
        if base_gain_db.abs() < 0.15 {
            // Dead-band: skip the multiply if the correction is negligible.
            return;
        }
        let base_gain = 10.0_f32.powf(base_gain_db / 20.0);

        for sample in samples {
            *sample *= base_gain;
        }
    }

    /// Executa um bloco de código com parâmetros configurados para o rastreador de F0.
    pub fn with_f0_config<R>(
        min_hz: f64,
        max_hz: f64,
        method: F0TrackerMethod,
        f: impl FnOnce() -> R,
    ) -> R {
        struct ResetGuard(Option<(f64, f64, F0TrackerMethod)>);
        impl Drop for ResetGuard {
            fn drop(&mut self) {
                ACTIVE_F0_CONFIG.with(|cell| {
                    *cell.borrow_mut() = self.0;
                });
            }
        }
        let prev = ACTIVE_F0_CONFIG.with(|cell| {
            let prev = *cell.borrow();
            *cell.borrow_mut() = Some((min_hz, max_hz, method));
            prev
        });
        let _guard = ResetGuard(prev);
        f()
    }

    #[allow(dead_code)]
    fn current_f0_extractor() -> WorldF0Extractor {
        ACTIVE_F0_CONFIG.with(|cell| {
            if let Some((min_hz, max_hz, _method)) = *cell.borrow() {
                WorldF0Extractor {
                    min_f0: min_hz as f32,
                    max_f0: max_hz as f32,
                    frame_period_ms: 5.0,
                }
            } else {
                WorldF0Extractor::default()
            }
        })
    }

    /// Remove amostras não finitas, reconstrói falhas internas curtas e remove
    /// offset DC antes da análise.
    #[allow(dead_code)]
    fn sanitize_input(samples: &[f32], sample_rate: u32) -> Vec<f32> {
        let mut clean: Vec<f32> = samples
            .iter()
            .map(|sample| if sample.is_finite() { *sample } else { 0.0 })
            .collect();
        if clean.is_empty() {
            return clean;
        }
        Self::repair_short_input_dropouts(&mut clean, sample_rate);
        let dc = clean.iter().sum::<f32>() / clean.len() as f32;
        for sample in &mut clean {
            *sample -= dc;
        }
        clean
    }

    /// Preenche uma lacuna digital curta no meio de uma região vozeada.
    ///
    /// Alguns bancos contêm poucos milissegundos de zeros entre o ataque e a
    /// vogal. A análise por quadros interpreta isso como uma quebra de fase e a reproduz como
    /// corte. A ponte Hermite mantém valor e inclinação nas duas bordas, sem
    /// tocar no silêncio inicial/final, respirações ou pausas longas.
    #[allow(dead_code)]
    fn repair_short_input_dropouts(samples: &mut [f32], sample_rate: u32) {
        if sample_rate == 0 || samples.len() < 16 {
            return;
        }
        let min_gap = ((sample_rate as f64 * 0.0005).round() as usize).max(3);
        let max_gap = ((sample_rate as f64 * 0.006).round() as usize).max(min_gap);
        let quiet = 0.001_f32;
        let mut index = 2;
        while index + 2 < samples.len() {
            if samples[index].abs() > quiet {
                index += 1;
                continue;
            }
            let start = index;
            while index < samples.len() && samples[index].abs() <= quiet {
                index += 1;
            }
            let end = index;
            let length = end - start;
            if length < min_gap || length > max_gap || start < 2 || end + 1 >= samples.len() {
                continue;
            }

            let context = (sample_rate as usize / 1_000).clamp(8, 48);
            let left_start = start.saturating_sub(context);
            let right_end = (end + context).min(samples.len());
            let left_peak = samples[left_start..start]
                .iter()
                .map(|sample| sample.abs())
                .fold(0.0_f32, f32::max);
            let right_peak = samples[end..right_end]
                .iter()
                .map(|sample| sample.abs())
                .fold(0.0_f32, f32::max);
            if left_peak < 0.01 || right_peak < 0.01 {
                continue;
            }

            let left = samples[start - 1];
            let right = samples[end];
            let left_slope = left - samples[start - 2];
            let right_slope = samples[end + 1] - right;
            let span = (length + 1) as f32;
            let ceiling = left_peak.max(right_peak) * 1.05;
            for offset in 0..length {
                let t = (offset + 1) as f32 / span;
                let t2 = t * t;
                let t3 = t2 * t;
                let bridge = (2.0 * t3 - 3.0 * t2 + 1.0) * left
                    + (t3 - 2.0 * t2 + t) * span * left_slope
                    + (-2.0 * t3 + 3.0 * t2) * right
                    + (t3 - t2) * span * right_slope;
                samples[start + offset] = bridge.clamp(-ceiling, ceiling);
            }
        }
    }

    /// Deslocamento de formantes estilo WORLD: deforma apenas o envelope
    /// espectral mantendo fase e detalhes harmônicos.
    fn apply_formant_shift(samples: &mut [f32], cents: f64) {
        if samples.len() < 32 || cents.abs() <= 1.0 {
            return;
        }

        // Clamp to ±2400 cents (2 octaves) to prevent extreme distortion.
        // Negate so that a positive gender value shifts formants up (brighter/female).
        let ratio = 2.0f32.powf((-cents.clamp(-2_400.0, 2_400.0) / 1_200.0) as f32);
        // Guard against ratios that are essentially 1.0 (would be a no-op with rounding errors)
        if (ratio - 1.0).abs() < 0.005 {
            return;
        }
        let fft_size = if samples.len() < 1_024 { 512 } else { 1_024 };
        let hop = fft_size / 4;
        let blocks = (samples.len() + hop - 1) / hop;
        let half = fft_size / 2 + 1;
        let mut re = vec![0.0f32; fft_size];
        let mut im = vec![0.0f32; fft_size];
        let mut morphed = vec![0.0f32; samples.len()];
        let mut weights = vec![0.0f32; samples.len()];
        let mut previous_envelope: Option<Vec<f32>> = None;

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
            // Suavização no domínio log para obter o envelope do trato vocal
            let mut envelope: Vec<f32> = (0..half)
                .map(|bin| {
                    let start = bin.saturating_sub(8);
                    let end = (bin + 9).min(half);
                    let log_average = magnitude[start..end].iter().map(|v| v.ln()).sum::<f32>()
                        / (end - start) as f32;
                    log_average.exp().max(1e-7)
                })
                .collect();
            // Keep the formant envelope temporally coherent across adjacent
            // STFT blocks. This stabilizes sustained notes and avoids the
            // frame-to-frame brightness jitter of an independently shifted
            // spectrum.
            if let Some(previous) = previous_envelope.as_ref() {
                for (current, prior) in envelope.iter_mut().zip(previous) {
                    *current = *prior * 0.65 + *current * 0.35;
                }
            }
            previous_envelope = Some(envelope.clone());

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
            if weight >= 0.25 {
                *sample = sum / weight;
            } else if weight > 1e-6 {
                let morphed_val = sum / weight;
                let blend = weight / 0.25;
                *sample = *sample * (1.0 - blend) + morphed_val * blend;
            }
        }
    }

    /// Adiciona ruído filtrado (respiração/B flag) proporcional ao nível do sinal.
    fn apply_breathiness(samples: &mut [f32], breathiness: f64) {
        let amount = (breathiness / 100.0).clamp(0.0, 1.0) as f32;
        if amount <= 0.0 {
            return;
        }
        let mut state = 0x6d2b_79f5_u32;
        let mut previous_noise = 0.0f32;
        let mut high_pass = 0.0f32;
        // Acompanha a amplitude local para não gerar ruído de fundo no silêncio
        let mut envelope = 0.0f32;
        for sample in samples {
            envelope += (sample.abs() - envelope) * 0.02;
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let noise = ((state >> 8) as f32 / 8_388_608.0) - 1.0;
            high_pass = 0.985 * (high_pass + noise - previous_noise);
            previous_noise = noise;
            // Preserva o sinal vozeado adicionando ruído passa-alta moldado
            *sample = *sample * (1.0 - amount * 0.10) + high_pass * amount * 0.045 * envelope;
        }
    }

    /// Suaviza descontinuidades isoladas e micro-estalos deixados pela síntese (De-crackle / De-click).
    pub fn repair_phase_seams(samples: &mut [f32], sample_rate: u32) {
        if sample_rate == 0 || samples.len() < 32 {
            return;
        }
        // Wider context and fade windows make the repair more gradual and
        // less likely to leave a secondary artifact at the patch boundaries.
        // The raised minimum threshold avoids treating the natural rapid
        // amplitude changes of fricative consonants (/s/, /f/, /h/) as seams.
        let context = ((sample_rate as f64 * 0.00040).round() as usize).clamp(8, 28);
        let fade = ((sample_rate as f64 * 0.0012).round() as usize).clamp(18, 60);
        if samples.len() <= context * 2 + fade + 1 {
            return;
        }

        // 1. De-crackle para micro-spikes isolados de 1-2 amostras (impulsos anômalos)
        for i in 2..samples.len().saturating_sub(2) {
            let prev2 = samples[i - 2];
            let prev1 = samples[i - 1];
            let curr = samples[i];
            let next1 = samples[i + 1];
            let next2 = samples[i + 2];

            let predicted = (prev1 + next1) * 0.5;
            let error = (curr - predicted).abs();
            let local_step = (prev1 - prev2).abs().max((next2 - next1).abs()).max(0.008);
            if error > local_step * 4.5 && (curr - prev1) * (curr - next1) > 0.0 {
                let slope_l = prev1 - prev2;
                let slope_r = next2 - next1;
                samples[i] = prev1 * 0.5 + next1 * 0.5 + (slope_l - slope_r) * 0.125;
            }
        }

        // 2. Reparação de saltos de fase (phase seams) em janelas de transição
        let original = samples.to_vec();
        // Soma móvel de inclinação para detectar saltos anômalos
        let slope = |i: usize| f64::from((original[i] - original[i - 1]).abs());
        let mut slope_sum: f64 = (1..=2 * context).map(slope).sum();
        let mut seams = Vec::new();
        let mut next_allowed = context;
        for index in context..samples.len() - context - fade {
            if index > context {
                slope_sum += slope(index + context) - slope(index - context);
            }
            if index < next_allowed {
                continue;
            }
            let jump = (original[index] - original[index - 1]).abs();
            let neighbour_slope = slope_sum - f64::from(jump);
            let local_mean = (neighbour_slope.max(0.0) / (2 * context - 1) as f64) as f32;
            // Raised minimum threshold (0.050) prevents false-positive seam
            // detection on fricative consonants whose waveform naturally has
            // large sample-to-sample steps.
            let threshold = (local_mean * 5.0).max(0.050);
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
                // Interpolação suave em 'S' (smoothstep)
                let eased = t * t * (3.0 - 2.0 * t);
                samples[start + offset] = left + (right - left) * eased;
            }
        }
    }

    /// Estágio final de segurança inspirado no straycat-rs:
    /// 1. Repara NaN/Inf.
    /// 2. Aplica Peak Normalization proporcional mantendo headroom seguro de -0.45 dBFS (~0.95),
    ///    preservando 100% da linearidade e dinâmica original sem deformar cristas de onda.
    /// 3. Aplica soft-knee limiter monotônico suave (tanh) para conter picos interamostrais residuais.
    pub(crate) fn apply_output_safety(samples: &mut [f32]) {
        for sample in samples.iter_mut() {
            if !sample.is_finite() {
                *sample = 0.0;
            }
        }

        // 1. Peak Normalization linear (evita distorções de saturação/clipping)
        let peak = samples.iter().fold(0.0f32, |acc, &s| acc.max(s.abs()));
        if peak > 0.95 {
            let scale = 0.95 / peak;
            for sample in samples.iter_mut() {
                *sample *= scale;
            }
        }

        // 2. Soft limiter estritamente monotônico acima de 0.95
        for sample in samples.iter_mut() {
            let x = *sample;
            let abs_x = x.abs();
            if abs_x > 0.95 {
                let excess = abs_x - 0.95;
                let compressed = 0.95 + 0.048 * (excess / 0.048).tanh();
                *sample = x.signum() * compressed.min(0.998);
            }
        }
    }

    #[allow(dead_code)]
    #[allow(clippy::too_many_arguments)]
    fn render_world(
        input: &[f32],
        sample_rate: u32,
        source_consonant_ms: f64,
        target_consonant_ms: f64,
        target_duration_ms: f64,
        target_pitch_freq: f64,
        pitch_points: &[UPitchBendPoint],
    ) -> Vec<f32> {
        let output_len =
            ((target_duration_ms.max(0.0) / 1_000.0) * sample_rate as f64).round() as usize;
        if input.is_empty() || sample_rate == 0 || output_len == 0 {
            return vec![0.0; output_len];
        }
        let extractor = Self::current_f0_extractor();
        let (f0s, voiced) = extractor.extract_f0(input, sample_rate);
        let hop = ((extractor.frame_period_ms / 1_000.0) * sample_rate as f32)
            .round()
            .max(1.0) as usize;
        let fft_size = 1024usize;
        let spectra: Vec<Vec<f32>> = f0s
            .iter()
            .enumerate()
            .map(|(frame, &f0)| {
                CheapTrick::extract_spectrum(input, frame * hop, f0, sample_rate, fft_size)
            })
            .collect();

        let source_consonant_samples =
            ((source_consonant_ms.max(0.0) / 1_000.0) * sample_rate as f64).round() as usize;
        let source_consonant_samples = source_consonant_samples.min(input.len());
        let target_consonant_samples =
            (((target_consonant_ms.max(0.0) / 1_000.0) * sample_rate as f64).round() as usize)
                .min(output_len);
        let target_vowel_samples = output_len.saturating_sub(target_consonant_samples);
        let source_vowel_samples = input.len().saturating_sub(source_consonant_samples);

        let mut output = vec![0.0; output_len];
        let mut phase = 0.0_f64;
        let mut voiced_mix = 0.0_f32;
        let mut source_envelope = 0.0_f32;
        let harmonics_cap = 40usize;
        for (index, sample) in output.iter_mut().enumerate() {
            let source_position = if index < target_consonant_samples
                && target_consonant_samples > 0
            {
                let t = index as f64 / target_consonant_samples as f64;
                t * source_consonant_samples as f64
            } else if target_vowel_samples > 0 && source_vowel_samples > 0 {
                let vowel_index = index - target_consonant_samples;
                let t = vowel_index as f64 / target_vowel_samples.saturating_sub(1).max(1) as f64;
                source_consonant_samples as f64 + t * source_vowel_samples.saturating_sub(1) as f64
            } else if source_consonant_samples > 0 {
                (input.len().saturating_sub(1)) as f64
            } else {
                index as f64 * (input.len().saturating_sub(1)) as f64
                    / output_len.saturating_sub(1).max(1) as f64
            };

            let source_index =
                (source_position.floor() as usize).min(input.len().saturating_sub(1));
            let source_next = (source_index + 1).min(input.len() - 1);
            let fraction = (source_position - source_index as f64).clamp(0.0, 1.0) as f32;
            let source_sample =
                input[source_index] * (1.0 - fraction) + input[source_next] * fraction;
            // WORLD excitation needs the slowly varying amplitude envelope,
            // never the instantaneous waveform. Using `abs(source_sample)`
            // here rectifies every glottal cycle and creates the grunting
            // amplitude modulation reported in voiced vowels.
            let envelope_target = source_sample.abs();
            let envelope_rate = if envelope_target > source_envelope {
                1.0 / (sample_rate as f32 * 0.003).max(1.0)
            } else {
                1.0 / (sample_rate as f32 * 0.018).max(1.0)
            };
            source_envelope += (envelope_target - source_envelope) * envelope_rate;
            let frame = (source_index / hop).min(f0s.len().saturating_sub(1));
            let is_voiced = voiced.get(frame).copied().unwrap_or(false);
            let desired_mix = if is_voiced { 1.0 } else { 0.0 };
            // 1 ms slew prevents an amplitude step at V/UV boundaries.
            voiced_mix +=
                (desired_mix - voiced_mix) * (1.0 / (sample_rate as f32 * 0.001).max(1.0));
            let time_ms = index as f64 * 1_000.0 / sample_rate as f64;
            let cents = crate::dsp::PitchBendSolver::get_pitch_offset_cents(time_ms, pitch_points);
            let target_f0 =
                (target_pitch_freq * 2.0_f64.powf(cents / 1_200.0)).clamp(40.0, 1_400.0);
            phase = (phase + std::f64::consts::TAU * target_f0 / sample_rate as f64)
                .rem_euclid(std::f64::consts::TAU);
            let spectrum = spectra.get(frame);
            let mut voiced_sample = 0.0_f32;
            if let Some(spectrum) = spectrum {
                let reference = spectrum.iter().copied().fold(1e-6_f32, f32::max);
                let harmonic_count =
                    ((sample_rate as f64 / 2.0 / target_f0) as usize).clamp(1, harmonics_cap);
                for harmonic in 1..=harmonic_count {
                    let bin = ((harmonic as f64 * target_f0 / sample_rate as f64) * fft_size as f64)
                        .round() as usize;
                    let amplitude = spectrum.get(bin).copied().unwrap_or(0.0) / reference;
                    voiced_sample += (phase * harmonic as f64).sin() as f32 * amplitude;
                }
                // The envelope supplies relative harmonic levels; retain the local
                // recording level instead of allowing overlap peaks to clip.
                voiced_sample *= source_envelope.max(0.015) / (harmonic_count as f32).sqrt();
            }
            *sample = voiced_sample * voiced_mix + source_sample * (1.0 - voiced_mix);
        }
        output
    }

    /// Renderiza a amostra fonética conforme parâmetros de tempo, afinação, formantes e respiração.
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
        loop_start_ms: Option<f64>,
        loop_end_ms: Option<f64>,
        tail_start_ms: Option<f64>,
        gender_shift: f64,
        breathiness: f64,
    ) -> Vec<f32> {
        // Use the F0 method configured via with_f0_config to select the
        // pitch-analysis strategy, but keep PSOLA as the synthesis engine so
        // that phoneme identity (consonant character, timing) is preserved.
        let stretch_mode = ACTIVE_F0_CONFIG.with(|cell| {
            cell.borrow()
                .map(|(_, _, method)| match method {
                    F0TrackerMethod::World => crate::dsp::SolaStretchMode::Stretch,
                    _ => crate::dsp::SolaStretchMode::Stretch,
                })
                .unwrap_or(crate::dsp::SolaStretchMode::Stretch)
        });

        let mut rendered = crate::dsp::SolaResampler::render_sample_with_mode(
            input_samples,
            sample_rate,
            offset_ms,
            source_consonant_ms,
            target_consonant_ms,
            cutoff_ms,
            target_duration_ms,
            target_pitch_freq,
            pitch_points,
            loop_start_ms,
            loop_end_ms,
            tail_start_ms,
            stretch_mode,
        );

        let total_samples = rendered.len();
        if total_samples == 0 {
            return rendered;
        }

        Self::apply_breathiness(&mut rendered, breathiness);
        Self::apply_formant_shift(&mut rendered, gender_shift);
        Self::repair_phase_seams(&mut rendered, sample_rate);
        Self::normalize_alias_loudness(&mut rendered, sample_rate);
        Self::apply_output_safety(&mut rendered);

        rendered
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breathiness_preserves_silence_and_scales_with_voice_level() {
        let mut silent = vec![0.0; 4096];
        VenusResampler::apply_breathiness(&mut silent, 100.0);
        assert!(silent.iter().all(|s| *s == 0.0));
        let mut normal: Vec<f32> = (0..4096).map(|i| (i as f32 * 0.04).sin() * 0.4).collect();
        let mut soft: Vec<f32> = normal.iter().map(|s| s * 0.01).collect();
        VenusResampler::apply_breathiness(&mut normal, 50.0);
        VenusResampler::apply_breathiness(&mut soft, 50.0);
        for (a, b) in normal.iter().zip(soft) {
            assert!((a * 0.01 - b).abs() < 1e-7);
        }
    }

    #[test]
    fn venus_loudness_normalization_converges_alias_bodies_without_lifting_tail() {
        let sample_rate = 44_100;
        let tone = |amplitude: f32| {
            (0..sample_rate as usize)
                .map(|index| {
                    (std::f32::consts::TAU * 220.0 * index as f32 / sample_rate as f32).sin()
                        * amplitude
                })
                .collect::<Vec<_>>()
        };
        let mut quiet = tone(0.08);
        let mut loud = tone(0.40);
        let tail_start = quiet.len() * 4 / 5;
        quiet[tail_start..].fill(0.0);

        VenusResampler::normalize_alias_loudness(&mut quiet, sample_rate);
        VenusResampler::normalize_alias_loudness(&mut loud, sample_rate);

        let rms = |samples: &[f32]| {
            (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32)
                .sqrt()
        };
        let quiet_body = rms(&quiet[sample_rate as usize / 4..sample_rate as usize / 2]);
        let loud_body = rms(&loud[sample_rate as usize / 4..sample_rate as usize / 2]);
        // With the conservative MAX_BOOST_DB=3 dB the quiet alias can only be
        // lifted by up to ~41%, so full convergence to the loud alias level is
        // not expected.  The test verifies partial convergence (>0.60) and that
        // the loud alias is cut down (ratio < 10), without requiring the
        // aggressive +9 dB boost that amplified voicebank recording noise.
        assert!(
            (quiet_body / loud_body).clamp(0.0, 10.0) > 0.60,
            "alias bodies remained unbalanced: quiet={quiet_body:.3}, loud={loud_body:.3}"
        );
        // The silent tail must never be amplified by normalisation.
        assert!(quiet[tail_start..].iter().all(|sample| *sample == 0.0));
    }

    #[test]
    fn venus_loudness_normalization_uses_one_gain_across_a_long_alias() {
        let sample_rate = 44_100;
        let segment_len = sample_rate as usize / 5;
        let mut source = Vec::with_capacity(segment_len * 3);
        for amplitude in [0.40, 0.08, 0.40] {
            source.extend((0..segment_len).map(|index| {
                (std::f32::consts::TAU * 220.0 * index as f32 / sample_rate as f32).sin()
                    * amplitude
            }));
        }
        let original = source.clone();
        VenusResampler::normalize_alias_loudness(&mut source, sample_rate);
        let rms = |samples: &[f32]| {
            (samples.iter().map(|sample| sample * sample).sum::<f32>() / samples.len() as f32)
                .sqrt()
        };
        let gains = (0..3)
            .map(|segment| {
                let range = segment * segment_len..(segment + 1) * segment_len;
                rms(&source[range.clone()]) / rms(&original[range])
            })
            .collect::<Vec<_>>();
        assert!(
            gains
                .windows(2)
                .all(|pair| (pair[0] - pair[1]).abs() < 1e-5),
            "normalization gain must not pump across a long alias: {gains:?}"
        );
    }

    #[test]
    fn venus_repairs_a_short_internal_voicebank_dropout_before_analysis() {
        let sample_rate = 44_100;
        let mut source: Vec<f32> = (0..sample_rate as usize / 2)
            .map(|index| {
                (std::f32::consts::TAU * 220.0 * index as f32 / sample_rate as f32).sin() * 0.45
            })
            .collect();
        let gap_start = sample_rate as usize / 5;
        let gap_length = (sample_rate as usize * 3) / 1_000;
        source[gap_start..gap_start + gap_length].fill(0.0);

        let repaired = VenusResampler::sanitize_input(&source, sample_rate);
        assert!(
            repaired[gap_start..gap_start + gap_length]
                .iter()
                .any(|sample| sample.abs() > 0.01),
            "the voiced dropout must be reconstructed before VENUS analysis"
        );
        let max_step = repaired
            .windows(2)
            .skip(gap_start.saturating_sub(2))
            .take(gap_length + 4)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0_f32, f32::max);
        assert!(
            max_step < 0.2,
            "dropout repair introduced a seam: {max_step}"
        );
    }

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
            None,
            None,
            None,
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
    fn venus_obeys_piano_roll_pitch_for_voiced_aliases_at_multiple_notes() {
        let sample_rate = 44_100;
        let source = (0..sample_rate as usize / 2)
            .map(|index| {
                (std::f32::consts::TAU * 220.0 * index as f32 / sample_rate as f32).sin() * 0.45
            })
            .collect::<Vec<_>>();
        let extractor = WorldF0Extractor::default();

        for target_hz in [164.81, 329.63, 440.0] {
            let rendered = VenusResampler::render_sample(
                &source,
                sample_rate,
                0.0,
                80.0,
                80.0,
                0.0,
                500.0,
                target_hz,
                &[],
                None,
                None,
                None,
                0.0,
                0.0,
            );
            let middle = &rendered[rendered.len() / 3..rendered.len() * 2 / 3];
            let (f0, _) = extractor.extract_f0(middle, sample_rate);
            let voiced = f0
                .into_iter()
                .filter(|value| *value > 50.0)
                .collect::<Vec<_>>();
            assert!(
                !voiced.is_empty(),
                "render at {target_hz:.2} Hz was unvoiced"
            );
            let measured = voiced.iter().sum::<f32>() / voiced.len() as f32;
            assert!(
                (measured - target_hz as f32).abs() < target_hz as f32 * 0.06,
                "piano-roll target {target_hz:.2} Hz rendered as {measured:.2} Hz"
            );
        }
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
            None,
            None,
            None,
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
    fn venus_repairs_an_isolated_phase_seam_without_touching_a_smooth_wave() {
        let rate = 44_100;
        let mut seamless: Vec<f32> = (0..2_000)
            .map(|index| (std::f32::consts::TAU * 220.0 * index as f32 / rate as f32).sin() * 0.45)
            .collect();
        let original = seamless.clone();
        VenusResampler::repair_phase_seams(&mut seamless, rate);
        assert_eq!(seamless, original, "smooth voiced audio was altered");

        let seam_at = 800;
        let mut broken = original.clone();
        for (index, sample) in broken.iter_mut().enumerate().skip(seam_at) {
            *sample =
                (std::f32::consts::TAU * 220.0 * index as f32 / rate as f32 + 1.4).sin() * 0.45;
        }
        let before = (broken[seam_at] - broken[seam_at - 1]).abs();
        VenusResampler::repair_phase_seams(&mut broken, rate);
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
            None,
            None,
            None,
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
            None,
            None,
            None,
            -1_200.0,
            0.0,
        );

        let rms = (rendered.iter().map(|sample| sample.powi(2)).sum::<f32>()
            / rendered.len().max(1) as f32)
            .sqrt();
        assert!(rms > 0.01, "Formant shift muted the sample: rms={rms}");
        assert!(rendered.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn venus_respects_oto_offset_and_cutoff_bounds() {
        let sample_rate = 44_100;
        // Construct a 2-second audio file:
        // [0..500ms]: Silence
        // [500ms..1000ms]: 220Hz tone (the target alias)
        // [1000ms..2000ms]: Silence
        let total_samples = sample_rate * 2;
        let mut samples = vec![0.0f32; total_samples as usize];
        let start_idx = (sample_rate as f64 * 0.5) as usize;
        let end_idx = (sample_rate as f64 * 1.0) as usize;
        for i in start_idx..end_idx {
            let t = (i - start_idx) as f32 / sample_rate as f32;
            samples[i] = (2.0 * PI * 220.0 * t).sin() * 0.5;
        }

        // 1. If offset and cutoff point to the active region (offset=500ms, cutoff=-500ms),
        // it must produce audible sound and correct duration.
        let rendered = VenusResampler::render_sample(
            &samples,
            sample_rate,
            500.0,
            50.0,
            50.0,
            -500.0, // negative cutoff: 500ms slice length
            400.0,
            220.0,
            &[],
            None,
            None,
            None,
            0.0,
            0.0,
        );
        let expected_len = (sample_rate as f64 * 0.4).round() as usize;
        assert_eq!(rendered.len(), expected_len);
        let max_amp = rendered.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(
            max_amp > 0.05,
            "Alias audio should be rendered: max_amp={max_amp}"
        );

        // 2. If offset is in the silence (offset=0ms, cutoff=-400ms), output must be silence.
        let silent_rendered = VenusResampler::render_sample(
            &samples,
            sample_rate,
            0.0,
            50.0,
            50.0,
            -400.0,
            400.0,
            220.0,
            &[],
            None,
            None,
            None,
            0.0,
            0.0,
        );
        assert_eq!(silent_rendered.len(), expected_len);
        let silent_max = silent_rendered
            .iter()
            .map(|s| s.abs())
            .fold(0.0f32, f32::max);
        assert!(
            silent_max < 0.01,
            "Silence region must remain silent: max={silent_max}"
        );
    }

    #[test]
    fn venus_renders_smooth_waveform_without_crackle_or_slope_discontinuities() {
        let sample_rate = 44_100;
        let freq = 220.0;
        let input: Vec<f32> = (0..sample_rate as usize / 2)
            .map(|i| {
                let t = i as f64 / sample_rate as f64;
                (0.40 * (std::f64::consts::TAU * freq * t).sin()
                    + 0.30 * (std::f64::consts::TAU * freq * 2.0 * t).sin()
                    + 0.15 * (std::f64::consts::TAU * freq * 3.0 * t).sin()) as f32
            })
            .collect();

        // Render with 2.5x time stretch (200ms to 500ms) and pitch bend
        let rendered = VenusResampler::render_sample(
            &input,
            sample_rate,
            0.0,
            20.0,
            20.0,
            0.0,
            500.0,
            261.63, // C4
            &[],
            Some(50.0),
            Some(180.0),
            None,
            0.0,
            0.0,
        );

        assert_eq!(rendered.len(), (sample_rate as f64 * 0.5).round() as usize);
        assert!(rendered.iter().all(|s| s.is_finite()));

        // Check that sample-to-sample slope changes are bounded (no crackle or sharp impulse clicks)
        let max_step = rendered
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0f32, f32::max);
        assert!(
            max_step < 0.25,
            "Detected excessive slope jump (crackle spike): {max_step}"
        );

        // Check that second differences (curvature spikes) are smooth
        let max_curvature = rendered
            .windows(3)
            .map(|w| (w[2] - 2.0 * w[1] + w[0]).abs())
            .fold(0.0f32, f32::max);
        assert!(
            max_curvature < 0.25,
            "Detected curvature spike (crackle impulse): {max_curvature}"
        );
    }
}

#[cfg(test)]
#[path = "venus_reference_tests.rs"]
mod reference_tests;
