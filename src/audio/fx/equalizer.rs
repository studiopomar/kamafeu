use super::biquad::BiquadState;
use super::ISO_31_BAND_FREQS;

/// Caches coefficients and active band indices without allocating in the audio loop.
pub(super) struct Equalizer {
    filters: [BiquadState; 31],
    gains: [f32; 31],
    active: [usize; 31],
    active_count: usize,
    sample_rate: f32,
}

impl Equalizer {
    pub(super) fn new(gains: [f32; 31], sample_rate: f32) -> Self {
        let mut equalizer = Self {
            filters: [BiquadState::default(); 31],
            gains: [f32::NAN; 31],
            active: [0; 31],
            active_count: 0,
            sample_rate,
        };
        equalizer.update(&gains);
        equalizer
    }

    fn update(&mut self, gains: &[f32; 31]) {
        if self.gains == *gains {
            return;
        }
        self.active_count = 0;
        for (index, &gain) in gains.iter().enumerate() {
            if self.gains[index] != gain {
                self.filters[index].set_peaking(
                    ISO_31_BAND_FREQS[index],
                    gain,
                    4.318,
                    self.sample_rate,
                );
            }
            if gain.abs() >= 0.05 {
                self.active[self.active_count] = index;
                self.active_count += 1;
            }
        }
        self.gains = *gains;
    }

    pub(super) fn process(&mut self, samples: &mut [f32], gains: &[f32; 31]) {
        self.update(gains);
        if self.active_count == 0 {
            return;
        }
        for frame in samples.chunks_exact_mut(2) {
            let (mut left, mut right) = (frame[0], frame[1]);
            for &index in &self.active[..self.active_count] {
                (left, right) = self.filters[index].process(left, right);
            }
            frame[0] = left;
            frame[1] = right;
        }
    }
}
