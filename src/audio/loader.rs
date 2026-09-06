use rodio::{Decoder, Source};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct AudioFileInfo {
    pub duration_ms: f64,
    pub sample_rate: u32,
    pub channels: u16,
}

#[derive(Debug, Clone)]
pub struct DecodedAudio {
    /// Interleaved PCM samples in [-1.0, 1.0]
    pub samples: Vec<f32>,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_ms: f64,
}

impl DecodedAudio {
    /// Resample and convert to stereo at target sample rate.
    pub fn to_stereo_at_sample_rate(&self, target_sample_rate: u32) -> Vec<f32> {
        if self.samples.is_empty() || self.channels == 0 || self.sample_rate == 0 {
            return Vec::new();
        }

        let num_frames = self.samples.len() / usize::from(self.channels.max(1));
        let target_frames = ((num_frames as f64) * (target_sample_rate as f64)
            / (self.sample_rate as f64))
            .round() as usize;
        let mut out = Vec::with_capacity(target_frames * 2);

        let ratio = (self.sample_rate as f64) / (target_sample_rate as f64);
        let ch = usize::from(self.channels);

        for i in 0..target_frames {
            let src_pos = (i as f64) * ratio;
            let idx0 = src_pos.floor() as usize;
            let idx1 = (idx0 + 1).min(num_frames.saturating_sub(1));
            let frac = (src_pos - idx0 as f64) as f32;

            let (l0, r0) = if ch == 1 {
                let s = self.samples[idx0];
                (s, s)
            } else {
                (self.samples[idx0 * ch], self.samples[idx0 * ch + 1])
            };

            let (l1, r1) = if ch == 1 {
                let s = self.samples[idx1];
                (s, s)
            } else {
                (self.samples[idx1 * ch], self.samples[idx1 * ch + 1])
            };

            let l = l0 + (l1 - l0) * frac;
            let r = r0 + (r1 - r0) * frac;
            out.push(l);
            out.push(r);
        }

        out
    }
}

/// Probes an audio file (.wav, .mp3, .ogg, .flac) to retrieve its duration, sample rate, and channels.
pub fn probe_audio_file<P: AsRef<Path>>(path: P) -> Option<AudioFileInfo> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    let decoder = Decoder::new(reader).ok()?;
    let sample_rate = decoder.sample_rate();
    let channels = decoder.channels();
    let duration_ms = if let Some(d) = decoder.total_duration() {
        d.as_secs_f64() * 1000.0
    } else {
        let samples_count = decoder.convert_samples::<f32>().count();
        (samples_count as f64 / f64::from(channels.max(1)) / f64::from(sample_rate.max(1))) * 1000.0
    };

    Some(AudioFileInfo {
        duration_ms,
        sample_rate,
        channels,
    })
}

/// Loads and decodes an audio file (.wav, .mp3, .ogg, .flac) into raw float samples.
pub fn load_audio_file<P: AsRef<Path>>(
    path: P,
) -> Result<DecodedAudio, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let decoder = Decoder::new(reader)?;
    let sample_rate = decoder.sample_rate();
    let channels = decoder.channels();
    let duration_from_header = decoder.total_duration().map(|d| d.as_secs_f64() * 1000.0);
    let samples: Vec<f32> = decoder.convert_samples().collect();
    let duration_ms = duration_from_header.unwrap_or_else(|| {
        (samples.len() as f64 / f64::from(channels.max(1)) / f64::from(sample_rate.max(1))) * 1000.0
    });

    Ok(DecodedAudio {
        samples,
        sample_rate,
        channels,
        duration_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decoded_audio_resampling() {
        let audio = DecodedAudio {
            samples: vec![0.5, 0.5, -0.5, -0.5], // 4 mono frames
            sample_rate: 22050,
            channels: 1,
            duration_ms: 4.0 / 22050.0 * 1000.0,
        };
        let stereo = audio.to_stereo_at_sample_rate(44100);
        // 4 mono frames at 22050 Hz -> 8 stereo frames at 44100 Hz -> 16 interleaved samples
        assert_eq!(stereo.len(), 16);
    }
}
