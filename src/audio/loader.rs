#[cfg(not(target_arch = "wasm32"))]
use rodio::{Decoder, Source};
#[cfg(not(target_arch = "wasm32"))]
use std::fs::File;
#[cfg(not(target_arch = "wasm32"))]
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
    /// Number of frames this audio occupies once converted to
    /// `target_sample_rate`.
    pub fn frames_at_sample_rate(&self, target_sample_rate: u32) -> usize {
        if self.samples.is_empty()
            || self.channels == 0
            || self.sample_rate == 0
            || target_sample_rate == 0
        {
            return 0;
        }
        let num_frames = self.samples.len() / usize::from(self.channels);
        crate::dsp::bandlimited::converted_frame_count(
            num_frames,
            self.sample_rate,
            target_sample_rate,
        )
    }

    /// Resample and convert to stereo at target sample rate.
    pub fn to_stereo_at_sample_rate(&self, target_sample_rate: u32) -> Vec<f32> {
        self.to_stereo_range_at_sample_rate(
            target_sample_rate,
            0,
            self.frames_at_sample_rate(target_sample_rate),
        )
    }

    /// Resample and convert to stereo only the output frames
    /// `first_frame..first_frame + frame_count`.
    ///
    /// The band-limited converter evaluates every output frame from its
    /// absolute position, so the result is identical to the same frames of
    /// [`DecodedAudio::to_stereo_at_sample_rate`]; chunked renders convert only
    /// the window they mix instead of the whole file. Mono sources are copied
    /// to both channels and only the first two channels of multichannel files
    /// are used.
    pub fn to_stereo_range_at_sample_rate(
        &self,
        target_sample_rate: u32,
        first_frame: usize,
        frame_count: usize,
    ) -> Vec<f32> {
        if self.samples.is_empty()
            || self.channels == 0
            || self.sample_rate == 0
            || target_sample_rate == 0
        {
            return Vec::new();
        }
        let channels = usize::from(self.channels);
        let converted = crate::dsp::bandlimited::resample_interleaved_range(
            &self.samples,
            channels,
            channels.min(2),
            self.sample_rate,
            target_sample_rate,
            first_frame,
            frame_count,
        );
        if channels == 1 {
            converted
                .iter()
                .flat_map(|sample| [*sample, *sample])
                .collect()
        } else {
            converted
        }
    }
}

/// Probes an audio file (.wav, .mp3, .ogg, .flac) to retrieve its duration, sample rate, and channels.
#[cfg(not(target_arch = "wasm32"))]
pub fn probe_audio_file<P: AsRef<Path>>(path: P) -> Option<AudioFileInfo> {
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    let decoder = Decoder::new(reader).ok()?;
    let sample_rate = decoder.sample_rate();
    let channels = decoder.channels();
    // MP3 container duration estimates can omit decoded samples (notably
    // encoder delay/padding). Count the decoder output so timing marks are
    // checked against the PCM that synthesis will actually consume.
    let samples_count = decoder.convert_samples::<f32>().count();
    let duration_ms =
        (samples_count as f64 / f64::from(channels.max(1)) / f64::from(sample_rate.max(1)))
            * 1000.0;

    Some(AudioFileInfo {
        duration_ms,
        sample_rate,
        channels,
    })
}

#[cfg(target_arch = "wasm32")]
pub fn probe_audio_file<P: AsRef<Path>>(path: P) -> Option<AudioFileInfo> {
    let reader = hound::WavReader::open(path).ok()?;
    let spec = reader.spec();
    let duration_ms = (reader.duration() as f64 / spec.sample_rate as f64) * 1000.0;
    Some(AudioFileInfo {
        duration_ms,
        sample_rate: spec.sample_rate,
        channels: spec.channels,
    })
}

/// Loads and decodes an audio file into raw float samples.
#[cfg(not(target_arch = "wasm32"))]
pub fn load_audio_file<P: AsRef<Path>>(
    path: P,
) -> Result<DecodedAudio, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let decoder = Decoder::new(reader)?;
    let sample_rate = decoder.sample_rate();
    let channels = decoder.channels();
    let samples: Vec<f32> = decoder.convert_samples().collect();
    let duration_ms =
        (samples.len() as f64 / f64::from(channels.max(1)) / f64::from(sample_rate.max(1)))
            * 1000.0;

    Ok(DecodedAudio {
        samples,
        sample_rate,
        channels,
        duration_ms,
    })
}

#[cfg(target_arch = "wasm32")]
pub fn load_audio_file<P: AsRef<Path>>(
    path: P,
) -> Result<DecodedAudio, Box<dyn std::error::Error>> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    let sample_rate = spec.sample_rate;
    let channels = spec.channels;
    let duration_ms = (reader.duration() as f64 / sample_rate as f64) * 1000.0;
    let samples: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader.samples::<f32>().filter_map(Result::ok).collect(),
        hound::SampleFormat::Int => {
            let max_val = (1 << (spec.bits_per_sample.saturating_sub(1))) as f32;
            reader
                .samples::<i32>()
                .filter_map(Result::ok)
                .map(|s| s as f32 / max_val)
                .collect()
        }
    };

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

    fn two_tone_frame(time: f64) -> [f32; 2] {
        [
            ((std::f64::consts::TAU * 440.0 * time).sin() * 0.5) as f32,
            ((std::f64::consts::TAU * 3_000.0 * time).sin() * 0.25) as f32,
        ]
    }

    fn two_tone_stereo(frames: usize, sample_rate: u32) -> DecodedAudio {
        let samples = (0..frames)
            .flat_map(|index| two_tone_frame(index as f64 / f64::from(sample_rate)))
            .collect();
        DecodedAudio {
            samples,
            sample_rate,
            channels: 2,
            duration_ms: frames as f64 * 1000.0 / f64::from(sample_rate),
        }
    }

    #[test]
    fn range_conversion_matches_the_full_conversion() {
        let audio = two_tone_stereo(9_600, 48_000);
        let full = audio.to_stereo_at_sample_rate(44_100);
        assert_eq!(full.len(), audio.frames_at_sample_rate(44_100) * 2);
        let part = audio.to_stereo_range_at_sample_rate(44_100, 1_234, 3_000);
        assert_eq!(part, full[1_234 * 2..(1_234 + 3_000) * 2]);
    }

    #[test]
    fn imported_audio_conversion_keeps_channels_separate_and_accurate() {
        let audio = two_tone_stereo(48_000, 48_000);
        let converted = audio.to_stereo_at_sample_rate(44_100);
        // Linear interpolation left errors around -45 dB on a 3 kHz tone;
        // the band-limited converter is far below that.
        let mut worst = 0.0f32;
        for (index, frame) in converted.chunks_exact(2).enumerate().skip(500).take(40_000) {
            let [left, right] = two_tone_frame(index as f64 / 44_100.0);
            worst = worst
                .max((frame[0] - left).abs())
                .max((frame[1] - right).abs());
        }
        assert!(worst < 1e-4, "worst conversion error {worst}");
    }
}
