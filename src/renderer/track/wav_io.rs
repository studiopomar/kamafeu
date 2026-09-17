use super::TrackRenderer;
use std::path::Path;

impl TrackRenderer {
    /// Helper to read a WAV file from disk into f32 mono samples
    pub fn load_wav_samples<P: AsRef<Path>>(path: P) -> Result<(Vec<f32>, u32), String> {
        let path = path.as_ref();
        let mut reader = hound::WavReader::open(path)
            .map_err(|e| format!("Failed to open WAV file {:?}: {}", path, e))?;

        let spec = reader.spec();
        let sample_rate = spec.sample_rate;
        if spec.channels == 0 {
            return Err(format!("WAV {:?} declares zero channels", path));
        }

        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Int => {
                if !(1..=32).contains(&spec.bits_per_sample) {
                    return Err(format!(
                        "Unsupported PCM depth {} in {:?}",
                        spec.bits_per_sample, path
                    ));
                }
                let max_val = 2.0f32.powi(i32::from(spec.bits_per_sample) - 1);
                reader
                    .samples::<i32>()
                    .map(|sample| {
                        sample
                            .map(|value| value as f32 / max_val)
                            .map_err(|error| error.to_string())
                    })
                    .collect::<Result<Vec<_>, _>>()?
            }
            hound::SampleFormat::Float => reader
                .samples::<f32>()
                .map(|sample| sample.map_err(|error| error.to_string()))
                .collect::<Result<Vec<_>, _>>()?,
        };

        if spec.channels > 1 {
            let mono: Vec<f32> = samples
                .chunks_exact(spec.channels as usize)
                .map(|chunk| chunk.iter().sum::<f32>() / spec.channels as f32)
                .collect();
            Ok((mono, sample_rate))
        } else {
            Ok((samples, sample_rate))
        }
    }

    /// Helper to write f32 mono samples into a WAV file on disk
    pub fn save_wav_samples<P: AsRef<Path>>(
        path: P,
        samples: &[f32],
        sample_rate: u32,
    ) -> Result<(), String> {
        Self::save_wav_samples_with_channels(path, samples, sample_rate, 1)
    }

    pub fn save_wav_samples_with_channels<P: AsRef<Path>>(
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
            .map_err(|e| format!("Failed to create temp WAV {:?}: {}", path.as_ref(), e))?;
        for &s in samples {
            let sample_i16 = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
            writer
                .write_sample(sample_i16)
                .map_err(|e| format!("Write sample error: {}", e))?;
        }
        writer
            .finalize()
            .map_err(|e| format!("Finalize WAV error: {}", e))?;
        Ok(())
    }
}
