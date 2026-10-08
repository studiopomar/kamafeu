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
    let sum = samples[start..start + window_samples]
        .iter()
        .filter(|sample| sample.is_finite())
        .map(|sample| f64::from(*sample) * f64::from(*sample))
        .sum::<f64>();
    (sum / window_samples as f64).sqrt() as f32
}

#[cfg(test)]
mod tests {
    use super::{window_rms, AudioDiagnostics};

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
}
