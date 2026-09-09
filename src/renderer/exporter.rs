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
}

pub struct AudioExporter;

impl AudioExporter {
    /// Export f32 samples to file using the specified format.
    pub fn export_audio<P: AsRef<Path>>(
        path: P,
        samples: &[f32],
        sample_rate: u32,
        channels: u16,
        format: AudioExportFormat,
    ) -> Result<(), String> {
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
}
