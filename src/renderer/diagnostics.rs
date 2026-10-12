//! Métricas objetivas para inspecionar áudio renderizado antes da exportação.
//!
//! A análise é deliberadamente independente de GUI e de engine: ela serve para
//! comparar VENUS, resamplers externos, wavtools e a mixagem final sob o mesmo
//! critério, sem interpretar um render que apenas não clipa como um render bom.

/// Resumo de integridade e nível de um buffer PCM intercalado.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioDiagnostics {
    pub frames: usize,
    pub channels: u16,
    pub peak: f32,
    pub rms: f32,
    pub clipped_samples: usize,
    pub non_finite_samples: usize,
    pub max_step: f32,
    pub max_step_frame: usize,
}

impl AudioDiagnostics {
    /// Analisa PCM intercalado. `channels == 0` é tratado como mono para que o
    /// relatório permaneça seguro mesmo diante de dados incompletos.
    pub fn analyze(samples: &[f32], channels: u16) -> Self {
        let channels = channels.max(1);
        let channel_count = usize::from(channels);
        let mut peak = 0.0_f32;
        let mut energy = 0.0_f64;
        let mut finite_count = 0_usize;
        let mut clipped_samples = 0_usize;
        let mut non_finite_samples = 0_usize;
        let mut max_step = 0.0_f32;
        let mut max_step_frame = 0_usize;

        for (index, &sample) in samples.iter().enumerate() {
            if !sample.is_finite() {
                non_finite_samples += 1;
                continue;
            }
            let magnitude = sample.abs();
            peak = peak.max(magnitude);
            if magnitude >= 1.0 {
                clipped_samples += 1;
            }
            energy += f64::from(sample) * f64::from(sample);
            finite_count += 1;

            if index >= channel_count {
                let previous = samples[index - channel_count];
                if previous.is_finite() {
                    let step = (sample - previous).abs();
                    if step > max_step {
                        max_step = step;
                        max_step_frame = index / channel_count;
                    }
                }
            }
        }

        Self {
            frames: samples.len() / channel_count,
            channels,
            peak,
            rms: if finite_count == 0 {
                0.0
            } else {
                (energy / finite_count as f64).sqrt() as f32
            },
            clipped_samples,
            non_finite_samples,
            max_step,
            max_step_frame,
        }
    }

    pub fn is_safe(&self) -> bool {
        self.non_finite_samples == 0 && self.clipped_samples == 0
    }

    /// Linha compacta apropriada para o console de renderização.
    pub fn render_log_line(&self, stage: &str) -> String {
        let status = if self.is_safe() { "OK" } else { "AVISO" };
        format!(
            "[Audio Diagnostics {status}] {stage}: {} frames, {} ch, peak {:.4}, RMS {:.4}, clip {}, nonfinite {}, max-step {:.4} @ frame {}",
            self.frames,
            self.channels,
            self.peak,
            self.rms,
            self.clipped_samples,
            self.non_finite_samples,
            self.max_step,
            self.max_step_frame,
        )
    }
}

/// RMS de uma janela de amostras. A função é usada para comparar a energia
/// nos limites de fonemas sem alterar o buffer renderizado.
pub fn window_rms(samples: &[f32], window_samples: usize, from_end: bool) -> f32 {
    let window_samples = window_samples.max(1).min(samples.len());
    if window_samples == 0 {
        return 0.0;
    }
    let start = if from_end {
        samples.len() - window_samples
    } else {
        0
    };
    let window = &samples[start..start + window_samples];
    let sum = sum_squares(window);
    (sum / window_samples as f64).sqrt() as f32
}

/// Removes isolated sample discontinuities produced by a resampler.  A jump
/// this large at 44.1/48 kHz is a click, not a musical transient.  The short
/// interpolation window keeps the repair local and leaves normal consonant
/// attacks untouched.
pub fn repair_discontinuities(samples: &mut [f32], threshold: f32, window: usize) -> usize {
    if samples.len() < 2 || window == 0 {
        return 0;
    }

    let mut repaired = 0;
    let mut index = 1;
    while index < samples.len() {
        if !samples[index - 1].is_finite() || !samples[index].is_finite() {
            index += 1;
            continue;
        }
        if (samples[index] - samples[index - 1]).abs() <= threshold {
            index += 1;
            continue;
        }

        let start = index - 1;
        let end = (index + window).min(samples.len() - 1);
        let left = samples[start];
        let right = samples[end];
        let span = (end - start) as f32;
        for offset in 1..=end - start {
            let amount = offset as f32 / span;
            samples[start + offset] = left + (right - left) * amount;
        }
        repaired += 1;
        index = end + 1;
    }
    repaired
}

/// Attenuates only the high-frequency energy of an already identified
/// sibilant. The gain envelope has a fast attack and gentle release, so a VC
/// sample keeps its voiced vowel while `s`/`sh` noise stops dominating the mix.
/// Returns the largest reduction applied in dB.
pub fn de_ess(samples: &mut [f32], sample_rate: u32, max_reduction_db: f32) -> f32 {
    if samples.is_empty() || sample_rate == 0 || max_reduction_db <= 0.0 {
        return 0.0;
    }

    let hp_decay = 0.90_f32;
    let attack = (-1.0_f32 / (sample_rate as f32 * 0.0015)).exp();
    let release = (-1.0_f32 / (sample_rate as f32 * 0.018)).exp();
    let gain_attack = (-1.0_f32 / (sample_rate as f32 * 0.001)).exp();
    let gain_release = (-1.0_f32 / (sample_rate as f32 * 0.025)).exp();
    let floor = 10.0_f32.powf(-max_reduction_db / 20.0);
    let mut previous = 0.0_f32;
    let mut high_pass = 0.0_f32;
    let mut envelope = 0.0_f32;
    let mut gain = 1.0_f32;
    let mut min_gain = 1.0_f32;

    for sample in samples {
        let input = if sample.is_finite() { *sample } else { 0.0 };
        high_pass = input - previous + hp_decay * high_pass;
        previous = input;
        let magnitude = high_pass.abs();
        let coefficient = if magnitude > envelope {
            attack
        } else {
            release
        };
        envelope = coefficient * envelope + (1.0 - coefficient) * magnitude;

        let target_gain = if envelope > 0.075 {
            (0.075 / envelope).sqrt().clamp(floor, 1.0)
        } else {
            1.0
        };
        let coefficient = if target_gain < gain {
            gain_attack
        } else {
            gain_release
        };
        gain = coefficient * gain + (1.0 - coefficient) * target_gain;
        *sample = input * gain;
        min_gain = min_gain.min(gain);
    }

    -20.0 * min_gain.max(f32::MIN_POSITIVE).log10()
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline]
fn sum_squares(samples: &[f32]) -> f64 {
    if samples.iter().all(|sample| sample.is_finite()) && std::is_x86_feature_detected!("sse2") {
        // SAFETY: SSE2 is checked immediately above and the slice is valid
        // for unaligned loads. The scalar tail handles the remainder.
        return unsafe { sum_squares_sse2(samples) };
    }

    sum_squares_scalar(samples)
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse2")]
unsafe fn sum_squares_sse2(samples: &[f32]) -> f64 {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::{
        _mm_add_pd, _mm_cvtps_pd, _mm_loadu_ps, _mm_movehl_ps, _mm_mul_pd, _mm_setzero_pd,
    };
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{
        _mm_add_pd, _mm_cvtps_pd, _mm_loadu_ps, _mm_movehl_ps, _mm_mul_pd, _mm_setzero_pd,
    };

    let mut sum = _mm_setzero_pd();
    let vector_len = samples.len() / 4 * 4;
    for index in (0..vector_len).step_by(4) {
        let values = _mm_loadu_ps(samples.as_ptr().add(index));
        let low = _mm_cvtps_pd(values);
        let high = _mm_cvtps_pd(_mm_movehl_ps(values, values));
        sum = _mm_add_pd(sum, _mm_mul_pd(low, low));
        sum = _mm_add_pd(sum, _mm_mul_pd(high, high));
    }

    let mut lanes = [0.0f64; 2];
    std::ptr::copy_nonoverlapping(&sum as *const _ as *const f64, lanes.as_mut_ptr(), 1);
    std::ptr::copy_nonoverlapping(
        (&sum as *const _ as *const f64).add(1),
        lanes.as_mut_ptr().add(1),
        1,
    );
    lanes[0] + lanes[1] + sum_squares_scalar(&samples[vector_len..])
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
#[inline]
fn sum_squares(samples: &[f32]) -> f64 {
    sum_squares_scalar(samples)
}

#[inline]
fn sum_squares_scalar(samples: &[f32]) -> f64 {
    samples
        .iter()
        .filter(|sample| sample.is_finite())
        .map(|sample| f64::from(*sample) * f64::from(*sample))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::{de_ess, repair_discontinuities, window_rms, AudioDiagnostics};

    #[test]
    fn de_esser_reduces_sustained_high_frequency_noise() {
        let mut samples: Vec<f32> = (0..4_410)
            .map(|index| if index % 2 == 0 { 0.6 } else { -0.6 })
            .collect();
        let reduction = de_ess(&mut samples, 44_100, 6.0);
        let tail_peak = samples[2_000..]
            .iter()
            .map(|sample| sample.abs())
            .fold(0.0f32, f32::max);
        assert!(reduction > 2.0);
        assert!(tail_peak < 0.5);
    }

    #[test]
    fn repairs_an_isolated_click_without_touching_normal_audio() {
        let mut samples = vec![0.0f32; 128];
        samples[32] = 0.9;
        samples[33] = -0.9;
        let repaired = repair_discontinuities(&mut samples, 0.45, 32);
        assert_eq!(repaired, 1);
        assert!(samples[32].abs() < 0.1);
        assert!(samples[33].abs() < 0.1);
    }

    #[test]
    fn analyzes_interleaved_audio_per_channel() {
        let metrics = AudioDiagnostics::analyze(&[0.0, 0.0, 0.5, -0.5, 0.75, -0.25], 2);
        assert_eq!(metrics.frames, 3);
        assert_eq!(metrics.channels, 2);
        assert_eq!(metrics.peak, 0.75);
        assert_eq!(metrics.clipped_samples, 0);
        assert_eq!(metrics.non_finite_samples, 0);
        assert_eq!(metrics.max_step, 0.5);
        assert!(metrics.is_safe());
    }

    #[test]
    fn detects_clipping_nonfinite_values_and_large_join_steps() {
        let metrics = AudioDiagnostics::analyze(&[0.0, 1.2, -1.3, f32::NAN], 1);
        assert_eq!(metrics.clipped_samples, 2);
        assert_eq!(metrics.non_finite_samples, 1);
        assert_eq!(metrics.max_step, 2.5);
        assert_eq!(metrics.max_step_frame, 2);
        assert!(!metrics.is_safe());
    }

    #[test]
    fn measures_non_destructive_boundary_windows() {
        let samples = [1.0, 1.0, 0.0, 0.0];
        assert_eq!(window_rms(&samples, 2, false), 1.0);
        assert_eq!(window_rms(&samples, 2, true), 0.0);
        assert_eq!(samples, [1.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn rms_simd_path_matches_expected_energy_with_tail() {
        let samples = [1.0, -1.0, 0.5, -0.5, 0.25];
        let expected = (2.5625_f64 / 5.0).sqrt() as f32;
        assert!((window_rms(&samples, samples.len(), false) - expected).abs() < 1e-7);
    }
}
