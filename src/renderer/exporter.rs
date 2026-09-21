use flacenc::error::Verify;
use std::fs::File;
use std::io::BufWriter;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AudioExportFormat {
    #[default]
    Wav16,
    Wav24,
    Wav32Float,
    Flac16,
    Flac24,
    RawF32,
}

impl AudioExportFormat {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Wav16 => "WAV (16-bit PCM CD Quality)",
            Self::Wav24 => "WAV (24-bit Studio Master)",
            Self::Wav32Float => "WAV (32-bit Float DAW)",
            Self::Flac16 => "FLAC (16-bit Lossless)",
            Self::Flac24 => "FLAC (24-bit Hi-Res Lossless)",
            Self::RawF32 => "RAW (32-bit Float PCM)",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            Self::Wav16 | Self::Wav24 | Self::Wav32Float => "wav",
            Self::Flac16 | Self::Flac24 => "flac",
            Self::RawF32 => "raw",
        }
    }

    pub fn from_extension(ext: &str) -> Self {
        match ext.to_ascii_lowercase().as_str() {
            "flac" => Self::Flac16,
            "raw" | "pcm" => Self::RawF32,
            _ => Self::Wav16,
        }
    }

    fn integer_bit_depth(self) -> Option<u8> {
        match self {
            Self::Wav16 | Self::Flac16 => Some(16),
            Self::Wav24 | Self::Flac24 => Some(24),
            Self::Wav32Float | Self::RawF32 => None,
        }
    }
}

/// Dither applied immediately before an integer PCM export.
///
/// The TPDF implementation is deterministic so exporting the same mix twice
/// produces the same file, while still decorrelating quantization error from
/// the source signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DitherMode {
    #[default]
    None,
    Tpdf,
}

impl DitherMode {
    pub fn from_config_label(label: &str) -> Self {
        if label.to_ascii_uppercase().contains("TPDF") {
            Self::Tpdf
        } else {
            Self::None
        }
    }
}

pub struct AudioExporter;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MasteringOptions {
    pub target_sample_rate: u32,
    pub normalize_peak_db: Option<f32>,
    pub head_silence_ms: f32,
    pub tail_silence_ms: f32,
}

impl MasteringOptions {
    pub fn prepare(&self, samples: &mut Vec<f32>, source_rate: u32, channels: u16) -> u32 {
        let channels = usize::from(channels.max(1));
        let target_rate = self.target_sample_rate.clamp(8_000, 192_000);
        if source_rate != target_rate && !samples.is_empty() {
            *samples = resample_interleaved(samples, source_rate, target_rate, channels);
        }

        if let Some(peak_db) = self.normalize_peak_db {
            let peak = samples
                .iter()
                .filter(|sample| sample.is_finite())
                .map(|sample| sample.abs())
                .fold(0.0_f32, f32::max);
            let target_peak = 10.0_f32.powf(peak_db.clamp(-60.0, 0.0) / 20.0);
            if peak > 1e-8 {
                let gain = target_peak / peak;
                for sample in samples.iter_mut() {
                    *sample *= gain;
                }
            }
        }

        let head_frames =
            ((self.head_silence_ms.max(0.0) / 1_000.0) * target_rate as f32).round() as usize;
        let tail_frames =
            ((self.tail_silence_ms.max(0.0) / 1_000.0) * target_rate as f32).round() as usize;
        if head_frames > 0 {
            let mut with_head = vec![0.0; head_frames * channels];
            with_head.extend_from_slice(samples);
            *samples = with_head;
        }
        if tail_frames > 0 {
            samples.resize(samples.len() + tail_frames * channels, 0.0);
        }
        target_rate
    }
}

fn resample_interleaved(
    samples: &[f32],
    source_rate: u32,
    target_rate: u32,
    channels: usize,
) -> Vec<f32> {
    let input_frames = samples.len() / channels;
    if input_frames == 0 || source_rate == 0 || target_rate == 0 {
        return samples.to_vec();
    }
    let output_frames = ((input_frames as f64 * f64::from(target_rate) / f64::from(source_rate))
        .round() as usize)
        .max(1);
    let ratio = f64::from(source_rate) / f64::from(target_rate);
    let mut output = Vec::with_capacity(output_frames * channels);
    for frame in 0..output_frames {
        let source_position = frame as f64 * ratio;
        let left = source_position.floor() as usize;
        let right = (left + 1).min(input_frames - 1);
        let fraction = (source_position - left as f64) as f32;
        for channel in 0..channels {
            output.push(
                samples[left * channels + channel] * (1.0 - fraction)
                    + samples[right * channels + channel] * fraction,
            );
        }
    }
    output
}

impl AudioExporter {
    /// Export f32 samples to file using the specified format.
    pub fn export_audio<P: AsRef<Path>>(
        path: P,
        samples: &[f32],
        sample_rate: u32,
        channels: u16,
        format: AudioExportFormat,
    ) -> Result<(), String> {
        if let Some(index) = samples.iter().position(|sample| !sample.is_finite()) {
            return Err(format!(
                "Recusada exportação de áudio inválido: amostra não finita no índice {index}"
            ));
        }
        let p = path.as_ref();
        match format {
            AudioExportFormat::Wav16 => Self::export_to_wav_16(p, samples, sample_rate, channels),
            AudioExportFormat::Wav24 => Self::export_to_wav_24(p, samples, sample_rate, channels),
            AudioExportFormat::Wav32Float => {
                Self::export_to_wav_f32(p, samples, sample_rate, channels)
            }
            AudioExportFormat::Flac16 => {
                Self::export_to_flac(p, samples, sample_rate, channels, 16)
            }
            AudioExportFormat::Flac24 => {
                Self::export_to_flac(p, samples, sample_rate, channels, 24)
            }
            AudioExportFormat::RawF32 => Self::export_to_raw(p, samples),
        }
    }

    /// Applies optional dither only when the selected container quantizes to
    /// integer PCM. Float WAV and RAW remain bit-exact.
    pub fn export_audio_with_dither<P: AsRef<Path>>(
        path: P,
        samples: &[f32],
        sample_rate: u32,
        channels: u16,
        format: AudioExportFormat,
        dither: DitherMode,
    ) -> Result<(), String> {
        let dithered;
        let samples = match (dither, format.integer_bit_depth()) {
            (DitherMode::Tpdf, Some(bits)) => {
                dithered = apply_tpdf_dither(samples, bits);
                &dithered
            }
            _ => samples,
        };
        Self::export_audio(path, samples, sample_rate, channels, format)
    }

    /// Export f32 PCM audio buffer to 16-bit WAV file
    pub fn export_to_wav_16<P: AsRef<Path>>(
        path: P,
        samples: &[f32],
        sample_rate: u32,
        channels: u16,
    ) -> Result<(), String> {
        let spec = hound::WavSpec {
            channels: channels.max(1),
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path.as_ref(), spec)
            .map_err(|e| format!("Failed to create WAV file: {e}"))?;
        for &s in samples {
            let sample_i16 = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
            writer
                .write_sample(sample_i16)
                .map_err(|e| format!("Write sample error: {e}"))?;
        }
        writer
            .finalize()
            .map_err(|e| format!("Finalize WAV error: {e}"))?;
        Ok(())
    }

    /// Export f32 PCM audio buffer to 24-bit WAV file
    pub fn export_to_wav_24<P: AsRef<Path>>(
        path: P,
        samples: &[f32],
        sample_rate: u32,
        channels: u16,
    ) -> Result<(), String> {
        let spec = hound::WavSpec {
            channels: channels.max(1),
            sample_rate,
            bits_per_sample: 24,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path.as_ref(), spec)
            .map_err(|e| format!("Failed to create WAV 24-bit file: {e}"))?;
        for &s in samples {
            let sample_i32 = (s.clamp(-1.0, 1.0) * 8388607.0) as i32;
            writer
                .write_sample(sample_i32)
                .map_err(|e| format!("Write sample error: {e}"))?;
        }
        writer
            .finalize()
            .map_err(|e| format!("Finalize WAV error: {e}"))?;
        Ok(())
    }

    /// Export f32 PCM audio buffer to 32-bit Float WAV file
    pub fn export_to_wav_f32<P: AsRef<Path>>(
        path: P,
        samples: &[f32],
        sample_rate: u32,
        channels: u16,
    ) -> Result<(), String> {
        let spec = hound::WavSpec {
            channels: channels.max(1),
            sample_rate,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(path.as_ref(), spec)
            .map_err(|e| format!("Failed to create WAV 32-bit Float file: {e}"))?;
        for &s in samples {
            writer
                .write_sample(s)
                .map_err(|e| format!("Write sample error: {e}"))?;
        }
        writer
            .finalize()
            .map_err(|e| format!("Finalize WAV error: {e}"))?;
        Ok(())
    }

    /// Export f32 PCM audio buffer to Lossless FLAC file
    pub fn export_to_flac<P: AsRef<Path>>(
        path: P,
        samples: &[f32],
        sample_rate: u32,
        channels: u16,
        bits_per_sample: usize,
    ) -> Result<(), String> {
        let bits = bits_per_sample.clamp(16, 24);
        let ch = usize::from(channels.max(1));
        let mut i32_samples = Vec::with_capacity(samples.len());
        let max_val = if bits == 24 { 8388607.0 } else { 32767.0 };
        for &s in samples {
            let clamped = (s.clamp(-1.0, 1.0) * max_val) as i32;
            i32_samples.push(clamped);
        }

        let config = flacenc::config::Encoder::default()
            .into_verified()
            .map_err(|e| format!("FLAC config error: {e:?}"))?;
        let block_size = config.block_size;
        let source =
            flacenc::source::MemSource::from_samples(&i32_samples, ch, bits, sample_rate as usize);
        let flac_stream = flacenc::encode_with_fixed_block_size(&config, source, block_size)
            .map_err(|e| format!("FLAC encoding error: {e:?}"))?;

        let mut sink = flacenc::bitsink::ByteSink::new();
        flacenc::component::BitRepr::write(&flac_stream, &mut sink)
            .map_err(|e| format!("FLAC write error: {e:?}"))?;
        std::fs::write(path.as_ref(), sink.as_slice())
            .map_err(|e| format!("Failed to write FLAC file: {e}"))?;
        Ok(())
    }

    /// Export raw 32-bit float samples (interleaved)
    pub fn export_to_raw<P: AsRef<Path>>(path: P, samples: &[f32]) -> Result<(), String> {
        use std::io::Write;
        let file =
            File::create(path.as_ref()).map_err(|e| format!("Failed to create RAW file: {e}"))?;
        let mut writer = BufWriter::new(file);
        for &s in samples {
            writer
                .write_all(&s.to_le_bytes())
                .map_err(|e| format!("RAW write error: {e}"))?;
        }
        writer.flush().map_err(|e| format!("Flush error: {e}"))?;
        Ok(())
    }
}

fn apply_tpdf_dither(samples: &[f32], bits_per_sample: u8) -> Vec<f32> {
    let scale = ((1_u64 << (bits_per_sample - 1)) - 1) as f32;
    let lsb = 1.0 / scale;
    let mut state = 0x6B_61_6D_61_66_65_75_u64;
    samples
        .iter()
        .map(|&sample| {
            let first = next_dither_unit(&mut state);
            let second = next_dither_unit(&mut state);
            (sample + (first - second) * lsb).clamp(-1.0, 1.0)
        })
        .collect()
}

fn next_dither_unit(state: &mut u64) -> f32 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    ((*state >> 40) as f32) / ((1_u32 << 24) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_export_wav_16_and_24_and_f32() {
        let dir = tempfile::tempdir().unwrap();
        let samples = vec![0.0f32, 0.5f32, -0.5f32, 1.0f32];

        let wav16_path = dir.path().join("test_16.wav");
        assert!(AudioExporter::export_to_wav_16(&wav16_path, &samples, 44100, 2).is_ok());
        assert!(wav16_path.exists());

        let wav24_path = dir.path().join("test_24.wav");
        assert!(AudioExporter::export_to_wav_24(&wav24_path, &samples, 44100, 2).is_ok());
        assert!(wav24_path.exists());

        let wav_f32_path = dir.path().join("test_f32.wav");
        assert!(AudioExporter::export_to_wav_f32(&wav_f32_path, &samples, 44100, 2).is_ok());
        assert!(wav_f32_path.exists());
    }

    #[test]
    fn test_export_flac() {
        let dir = tempfile::tempdir().unwrap();
        let samples = vec![0.0f32; 1024];

        let flac_path = dir.path().join("test.flac");
        assert!(AudioExporter::export_to_flac(&flac_path, &samples, 44100, 2, 16).is_ok());
        assert!(flac_path.exists());
    }

    #[test]
    fn refuses_nonfinite_audio_before_creating_an_export() {
        let temporary = tempfile::NamedTempFile::new().expect("temporary output path");
        let result = AudioExporter::export_audio(
            temporary.path(),
            &[0.0, f32::NAN],
            44_100,
            1,
            AudioExportFormat::Wav16,
        );
        assert!(result.is_err());
        assert!(result
            .expect_err("must reject invalid PCM")
            .contains("não finita"));
    }

    #[test]
    fn mastering_resamples_normalizes_and_adds_silence_per_frame() {
        let mut samples = vec![0.5_f32, -0.5, 1.0, -1.0];
        let rate = MasteringOptions {
            target_sample_rate: 16_000,
            normalize_peak_db: Some(-6.0206),
            head_silence_ms: 0.125,
            tail_silence_ms: 0.125,
        }
        .prepare(&mut samples, 8_000, 2);
        assert_eq!(rate, 16_000);
        assert_eq!(samples.len(), 16); // 4 resampled frames + 2 head + 2 tail.
        assert_eq!(&samples[..4], &[0.0; 4]);
        assert_eq!(&samples[12..], &[0.0; 4]);
        let peak = samples.iter().copied().map(f32::abs).fold(0.0, f32::max);
        assert!((peak - 0.5).abs() < 0.001);
    }

    #[test]
    fn tpdf_dither_is_deterministic_and_only_selected_from_tpdf_labels() {
        let source = vec![0.0_f32; 16];
        let first = apply_tpdf_dither(&source, 16);
        assert_eq!(first, apply_tpdf_dither(&source, 16));
        assert!(first.iter().any(|sample| *sample != 0.0));
        assert_eq!(
            DitherMode::from_config_label("TPDF (Triangular)"),
            DitherMode::Tpdf
        );
        assert_eq!(DitherMode::from_config_label("Nenhum"), DitherMode::None);
    }
}
