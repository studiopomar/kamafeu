//! Sidecar analysis files used by the VENUS resampler.
//!
//! A `.venus` file belongs to one WAV and is deliberately human-readable so
//! that it can be inspected, versioned and edited by the VENUS Editor.

use super::{
    sola::SolaResampler,
    venus::{FastFft, WorldF0Extractor},
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const VENUS_ANALYSIS_FORMAT: &str = "kamafeu-venus-analysis";
pub const VENUS_ANALYSIS_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VenusAnalysisFrame {
    pub time_ms: f32,
    pub f0_hz: f32,
    pub voiced: bool,
    pub rms: f32,
    #[serde(default)]
    pub spectral_centroid_hz: f32,
    #[serde(default)]
    pub spectral_tilt_db: f32,
    #[serde(default)]
    pub harmonicity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VenusAnalysis {
    pub format: String,
    pub version: u32,
    pub source_hash: String,
    pub sample_rate: u32,
    pub sample_count: usize,
    pub frames: Vec<VenusAnalysisFrame>,
    pub pitch_marks: Vec<usize>,
    /// User correction applied after VENUS synthesis. Zero is neutral.
    #[serde(default)]
    pub gain_db: f32,
    /// User formant correction in cents. Zero is neutral.
    #[serde(default)]
    pub formant_shift_cents: f32,
    /// Extra breathiness added by this alias. Zero is neutral.
    #[serde(default)]
    pub breathiness: f32,
    #[serde(default)]
    pub notes: String,
}

impl VenusAnalysis {
    pub fn is_current_for(&self, samples: &[f32], sample_rate: u32) -> bool {
        self.format == VENUS_ANALYSIS_FORMAT
            && self.version == VENUS_ANALYSIS_VERSION
            && self.sample_rate == sample_rate
            && self.sample_count == samples.len()
            && self.source_hash == source_hash(samples, sample_rate)
    }

    pub fn gain_linear(&self) -> f32 {
        10.0_f32.powf(self.gain_db.clamp(-24.0, 24.0) / 20.0)
    }
}

pub fn sidecar_path(wav_path: &Path) -> PathBuf {
    wav_path.with_extension("venus")
}

pub fn analyze_samples(samples: &[f32], sample_rate: u32) -> VenusAnalysis {
    let extractor = WorldF0Extractor::default();
    let (f0, voiced) = extractor.extract_f0(samples, sample_rate);
    let hop = ((extractor.frame_period_ms / 1_000.0) * sample_rate as f32)
        .round()
        .max(1.0) as usize;
    let frames = f0
        .iter()
        .enumerate()
        .map(|(index, &f0_hz)| {
            let start = index * hop;
            let end = (start + hop).min(samples.len());
            let rms = if end > start {
                (samples[start..end]
                    .iter()
                    .map(|sample| sample * sample)
                    .sum::<f32>()
                    / (end - start) as f32)
                    .sqrt()
            } else {
                0.0
            };
            let (spectral_centroid_hz, spectral_tilt_db, harmonicity) =
                spectral_features(&samples[start..end], sample_rate, f0_hz);
            VenusAnalysisFrame {
                time_ms: index as f32 * extractor.frame_period_ms,
                f0_hz,
                voiced: voiced.get(index).copied().unwrap_or(false),
                rms,
                spectral_centroid_hz,
                spectral_tilt_db,
                harmonicity,
            }
        })
        .collect::<Vec<_>>();

    let voiced_f0 = f0
        .iter()
        .copied()
        .filter(|value| *value > 0.0)
        .collect::<Vec<_>>();
    let pitch_marks = if voiced_f0.is_empty() {
        Vec::new()
    } else {
        let mut sorted = voiced_f0;
        sorted.sort_by(|left, right| left.total_cmp(right));
        let median_f0 = sorted[sorted.len() / 2].max(1.0);
        let period = (sample_rate as f32 / median_f0).round().max(8.0) as usize;
        SolaResampler::find_pitch_marks(samples, period)
    };

    VenusAnalysis {
        format: VENUS_ANALYSIS_FORMAT.to_string(),
        version: VENUS_ANALYSIS_VERSION,
        source_hash: source_hash(samples, sample_rate),
        sample_rate,
        sample_count: samples.len(),
        frames,
        pitch_marks,
        gain_db: 0.0,
        formant_shift_cents: 0.0,
        breathiness: 0.0,
        notes: String::new(),
    }
}

/// Compact per-frame spectral descriptors for visual inspection and stable
/// formant processing. The full spectrum is intentionally not persisted: it
/// would make each sidecar large while these descriptors capture the useful
/// long-term changes in brightness and harmonic content.
fn spectral_features(samples: &[f32], sample_rate: u32, f0_hz: f32) -> (f32, f32, f32) {
    if samples.len() < 8 || sample_rate == 0 {
        return (0.0, 0.0, 0.0);
    }
    let fft_size = samples.len().next_power_of_two().clamp(256, 1024);
    let mut real = vec![0.0f32; fft_size];
    let mut imag = vec![0.0f32; fft_size];
    for (index, value) in samples.iter().take(fft_size).enumerate() {
        let window = 0.5 - 0.5 * (std::f32::consts::TAU * index as f32 / fft_size as f32).cos();
        real[index] = value * window;
    }
    FastFft::transform(&mut real, &mut imag, false);
    let half = fft_size / 2;
    let bin_hz = sample_rate as f32 / fft_size as f32;
    let mut energy = 0.0f32;
    let mut weighted_frequency = 0.0f32;
    let mut low_energy = 0.0f32;
    let mut high_energy = 0.0f32;
    let mut harmonic_energy = 0.0f32;
    for bin in 1..=half {
        let magnitude = (real[bin] * real[bin] + imag[bin] * imag[bin]).sqrt();
        let frequency = bin as f32 * bin_hz;
        energy += magnitude;
        weighted_frequency += magnitude * frequency;
        if frequency < 1_000.0 {
            low_energy += magnitude;
        } else {
            high_energy += magnitude;
        }
        if f0_hz > 30.0 {
            let harmonic = (frequency / f0_hz).round().max(1.0) * f0_hz;
            if (frequency - harmonic).abs() <= bin_hz * 1.5 {
                harmonic_energy += magnitude;
            }
        }
    }
    let centroid = weighted_frequency / energy.max(1e-8);
    let tilt = 20.0 * (high_energy.max(1e-8) / low_energy.max(1e-8)).log10();
    (
        centroid,
        tilt,
        (harmonic_energy / energy.max(1e-8)).clamp(0.0, 1.0),
    )
}

pub fn load(path: &Path) -> Result<VenusAnalysis, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("não foi possível ler {}: {error}", path.display()))?;
    serde_json::from_str(&content)
        .map_err(|error| format!("arquivo VENUS inválido em {}: {error}", path.display()))
}

pub fn save(path: &Path, analysis: &VenusAnalysis) -> Result<(), String> {
    let content = serde_json::to_string_pretty(analysis)
        .map_err(|error| format!("não foi possível serializar análise VENUS: {error}"))?;
    let temporary = path.with_extension("venus.tmp");
    fs::write(&temporary, content)
        .map_err(|error| format!("não foi possível gravar {}: {error}", temporary.display()))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("não foi possível finalizar {}: {error}", path.display()))
}

/// Loads a valid sidecar or creates one. An out-of-date sidecar is replaced
/// because its user edits refer to a different recording and would be unsafe.
pub fn load_or_analyze(
    wav_path: &Path,
    samples: &[f32],
    sample_rate: u32,
) -> Result<VenusAnalysis, String> {
    let path = sidecar_path(wav_path);
    if let Ok(analysis) = load(&path) {
        if analysis.is_current_for(samples, sample_rate) {
            return Ok(analysis);
        }
    }
    let analysis = analyze_samples(samples, sample_rate);
    save(&path, &analysis)?;
    Ok(analysis)
}

fn source_hash(samples: &[f32], sample_rate: u32) -> String {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in sample_rate
        .to_le_bytes()
        .into_iter()
        .chain((samples.len() as u64).to_le_bytes())
    {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x1000_0000_01b3);
    }
    for sample in samples {
        for byte in sample.to_bits().to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x1000_0000_01b3);
        }
    }
    format!("fnv1a64:{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analysis_round_trip_is_current_and_preserves_user_controls() {
        let samples = (0..1_600)
            .map(|index| (index as f32 * 0.1).sin() * 0.4)
            .collect::<Vec<_>>();
        let mut analysis = analyze_samples(&samples, 16_000);
        analysis.gain_db = 3.0;
        analysis.formant_shift_cents = -120.0;
        assert!(analysis.is_current_for(&samples, 16_000));
        assert!((analysis.gain_linear() - 10.0_f32.powf(3.0 / 20.0)).abs() < 1e-6);

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("a.venus");
        save(&path, &analysis).unwrap();
        assert_eq!(load(&path).unwrap(), analysis);
    }

    #[test]
    fn changed_audio_invalidates_analysis() {
        let analysis = analyze_samples(&[0.0, 0.2, -0.2, 0.0], 44_100);
        assert!(!analysis.is_current_for(&[0.0, 0.1, -0.1, 0.0], 44_100));
    }
}
