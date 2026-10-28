//! Output-stage DSP. Each playback generation owns its filter history; only
//! settings are shared with the UI. Audio uses try_lock and never waits for it.
use super::{FxRackConfig, FxRackProcessor};
use rodio::{buffer::SamplesBuffer, Source};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub type Settings = Arc<Mutex<FxRackConfig>>;
pub type State = Arc<Mutex<OutputState>>;
pub struct OutputState {
    processor: FxRackProcessor,
    target: FxRackConfig,
    wet: f32,
    rate: u32,
}
impl OutputState {
    pub fn new(rate: u32) -> State {
        Arc::new(Mutex::new(Self {
            processor: FxRackProcessor::new(FxRackConfig::default(), rate),
            target: FxRackConfig::default(),
            wet: 0.0,
            rate,
        }))
    }
    fn process(&mut self, data: &mut [f32], settings: &Settings) {
        if let Ok(config) = settings.try_lock() {
            self.target = config.clone();
        }
        let previous = self.processor.config.eq.gains;
        self.processor.config = self.target.clone();
        self.processor.config.master_enabled = true;
        self.processor.config.eq.enabled = true;
        let alpha = 1.0 - (-(data.len() as f32 / 2.0) / (self.rate as f32 * 0.015)).exp();
        for (i, gain) in self.processor.config.eq.gains.iter_mut().enumerate() {
            let target = if self.target.eq.enabled {
                self.target.eq.gains[i]
            } else {
                0.0
            };
            *gain = previous[i] + (target - previous[i]) * alpha;
        }
        let mut dry = [0.0; 128];
        dry[..data.len()].copy_from_slice(data);
        self.processor.process_interleaved(data, self.rate);
        let target = if self.target.master_enabled { 1.0 } else { 0.0 };
        let step = 1.0 / (self.rate as f32 * 0.01);
        for (frame, original) in data.chunks_exact_mut(2).zip(dry.chunks_exact(2)) {
            self.wet += (target - self.wet).clamp(-step, step);
            for c in 0..2 {
                frame[c] = original[c] + (frame[c] - original[c]) * self.wet;
            }
        }
    }
}

pub struct MasterSource {
    input: SamplesBuffer<f32>,
    settings: Settings,
    state: State,
    buffer: [f32; 128],
    cursor: usize,
    len: usize,
}
impl MasterSource {
    pub fn new(
        samples: Vec<f32>,
        rate: u32,
        channels: u16,
        settings: Settings,
        state: State,
    ) -> Self {
        Self {
            input: SamplesBuffer::new(channels.max(1), rate, samples),
            settings,
            state,
            buffer: [0.0; 128],
            cursor: 0,
            len: 0,
        }
    }
}
impl Iterator for MasterSource {
    type Item = f32;
    fn next(&mut self) -> Option<f32> {
        if self.cursor == self.len {
            self.len = 0;
            self.cursor = 0;
            for frame in self.buffer.chunks_exact_mut(2) {
                let Some(left) = self.input.next() else {
                    break;
                };
                let channels = self.input.channels();
                let right = if channels == 1 {
                    left
                } else {
                    self.input.next().unwrap_or(left)
                };
                for _ in 2..channels {
                    self.input.next();
                }
                frame[0] = left;
                frame[1] = right;
                self.len += 2;
            }
            if self.len == 0 {
                return None;
            }
            if let Ok(mut state) = self.state.try_lock() {
                state.process(&mut self.buffer[..self.len], &self.settings);
            }
        }
        let sample = self.buffer[self.cursor];
        self.cursor += 1;
        Some(sample)
    }
}
impl Source for MasterSource {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        2
    }
    fn sample_rate(&self) -> u32 {
        self.input.sample_rate()
    }
    fn total_duration(&self) -> Option<Duration> {
        self.input.total_duration()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn signal(frames: usize) -> Vec<f32> {
        (0..frames)
            .flat_map(|i| {
                let v = (i as f32 * 1000.0 * std::f32::consts::TAU / 48000.0).sin() * 0.1;
                [v, v]
            })
            .collect()
    }
    fn settings(enabled: bool) -> Settings {
        let mut config = FxRackConfig::default();
        config.master_enabled = enabled;
        config.eq.gains[17] = 9.0;
        Arc::new(Mutex::new(config))
    }
    #[test]
    fn disabled_master_is_sample_exact_and_mono_becomes_stereo() {
        let input = signal(512);
        let output: Vec<_> = MasterSource::new(
            input.clone(),
            48000,
            2,
            settings(false),
            OutputState::new(48000),
        )
        .collect();
        assert_eq!(input, output);
        let mono = vec![0.1, 0.2, -0.2];
        let output: Vec<_> =
            MasterSource::new(mono, 48000, 1, settings(false), OutputState::new(48000)).collect();
        assert_eq!(output, vec![0.1, 0.1, 0.2, 0.2, -0.2, -0.2]);
    }
    #[test]
    fn contiguous_chunks_keep_filter_history() {
        let input = signal(8192);
        let config = settings(true);
        let full: Vec<_> = MasterSource::new(
            input.clone(),
            48000,
            2,
            config.clone(),
            OutputState::new(48000),
        )
        .collect();
        let state = OutputState::new(48000);
        let mut chunked = Vec::new();
        for chunk in input.chunks(2048) {
            chunked.extend(MasterSource::new(
                chunk.to_vec(),
                48000,
                2,
                config.clone(),
                state.clone(),
            ));
        }
        assert_eq!(full, chunked);
        let fresh: Vec<_> = MasterSource::new(
            input.clone(),
            48000,
            2,
            config.clone(),
            OutputState::new(48000),
        )
        .collect();
        assert_eq!(fresh, full);
        assert!(full.iter().zip(input).any(|(a, b)| (a - b).abs() > 0.05));
    }
    #[test]
    fn changes_and_bypass_reach_an_already_running_source() {
        let config = settings(false);
        let mut source = MasterSource::new(
            signal(48000),
            48000,
            2,
            config.clone(),
            OutputState::new(48000),
        );
        let dry: Vec<_> = source.by_ref().take(9600).collect();
        config.lock().unwrap().master_enabled = true;
        let wet: Vec<_> = source.by_ref().take(19200).collect();
        let rms = |v: &[f32]| (v.iter().map(|x| x * x).sum::<f32>() / v.len() as f32).sqrt();
        assert!(rms(&wet[9600..]) > rms(&dry) * 1.5);
        config.lock().unwrap().master_enabled = false;
        let bypass: Vec<_> = source.by_ref().take(19200).collect();
        assert!((rms(&bypass[9600..]) - rms(&dry)).abs() < 0.001);
        assert!(wet.windows(2).all(|w| (w[1] - w[0]).abs() < 0.15));
    }
    #[test]
    fn busy_settings_never_block_audio() {
        let config = settings(false);
        let mut source = MasterSource::new(
            signal(128),
            48000,
            2,
            config.clone(),
            OutputState::new(48000),
        );
        let _guard = config.lock().unwrap();
        assert!(source.next().is_some());
    }
}
