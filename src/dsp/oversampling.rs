//! Streaming 4x oversampling for memoryless nonlinearities.
//!
//! Waveshapers such as `tanh` create harmonics far above the Nyquist
//! frequency. At the base rate those harmonics fold back as inharmonic
//! aliasing, which is clearly audible on sibilants and high voices. Running
//! the nonlinearity at four times the sample rate between two linear-phase
//! half-band stages removes that aliasing.
//!
//! Both stages are Kaiser-windowed half-band FIR filters (every other tap is
//! zero), evaluated in polyphase form so only the non-zero taps are computed.
//! The first stage (55 non-zero taps per phase at 2x) is flat to ~20 kHz and
//! rejects ~80 dB from ~24 kHz at a 44.1 kHz base rate; the second stage only
//! needs to separate images that are far apart, so it is much shorter.
//!
//! The combined latency is an integer number of base-rate frames
//! ([`Oversampler4x::LATENCY`]), so dry signals can be time-aligned with a
//! plain delay line.

use super::kaiser::{kaiser, sinc};
use std::sync::{Arc, OnceLock};

/// Half-length of the first (2x) stage: 2 * M odd taps, length 4M - 1.
const STAGE1_HALF: usize = 28;
/// Half-length of the second (4x) stage.
const STAGE2_HALF: usize = 6;
/// Kaiser beta for roughly 80 dB of stopband rejection.
const HALFBAND_BETA: f64 = 7.86;

/// Odd (non-zero, non-centre) taps of a half-band low-pass filter of length
/// `4 * half - 1`, ordered from offset `-(2 * half - 1)` to `2 * half - 1`.
/// The centre tap is always 0.5 and the taps sum to 0.5, giving unity DC gain.
fn design_halfband(half: usize, beta: f64) -> Arc<[f32]> {
    let span = (2 * half) as f64;
    let mut taps: Vec<f64> = (0..2 * half)
        .map(|index| {
            let offset = 2.0 * index as f64 - (2 * half - 1) as f64;
            0.5 * sinc(offset * 0.5) * kaiser(offset / span, beta)
        })
        .collect();
    let sum: f64 = taps.iter().sum();
    for tap in &mut taps {
        *tap *= 0.5 / sum;
    }
    taps.into_iter().map(|tap| tap as f32).collect()
}

fn stage_taps(half: usize) -> Arc<[f32]> {
    static STAGE1: OnceLock<Arc<[f32]>> = OnceLock::new();
    static STAGE2: OnceLock<Arc<[f32]>> = OnceLock::new();
    let cell = if half == STAGE1_HALF {
        &STAGE1
    } else {
        &STAGE2
    };
    cell.get_or_init(|| design_halfband(half, HALFBAND_BETA))
        .clone()
}

#[inline]
fn dot(a: &[f32], b: &[f32]) -> f32 {
    // Four independent accumulators let the compiler vectorise the loop.
    let mut sums = [0.0f32; 4];
    let chunks = a.len() / 4 * 4;
    for (left, right) in a[..chunks].chunks_exact(4).zip(b[..chunks].chunks_exact(4)) {
        for lane in 0..4 {
            sums[lane] += left[lane] * right[lane];
        }
    }
    let mut total = (sums[0] + sums[1]) + (sums[2] + sums[3]);
    for (left, right) in a[chunks..].iter().zip(&b[chunks..]) {
        total += left * right;
    }
    total
}

/// History buffer stored twice so the newest `len` samples are always a
/// contiguous slice (oldest first).
#[derive(Clone)]
struct MirroredHistory {
    data: Vec<f32>,
    position: usize,
    len: usize,
}

impl MirroredHistory {
    fn new(len: usize) -> Self {
        Self {
            data: vec![0.0; 2 * len],
            position: 0,
            len,
        }
    }

    #[inline]
    fn push(&mut self, value: f32) -> &[f32] {
        self.data[self.position] = value;
        self.data[self.position + self.len] = value;
        self.position = (self.position + 1) % self.len;
        &self.data[self.position..self.position + self.len]
    }
}

/// 2x half-band interpolator. Produces the even/odd output pair for each
/// input sample with a latency of `half` input samples.
#[derive(Clone)]
struct HalfbandUp {
    taps: Arc<[f32]>,
    history: MirroredHistory,
    half: usize,
}

impl HalfbandUp {
    fn new(half: usize) -> Self {
        Self {
            taps: stage_taps(half),
            history: MirroredHistory::new(2 * half),
            half,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> (f32, f32) {
        let half = self.half;
        let window = self.history.push(input);
        let even = window[half - 1];
        let odd = 2.0 * dot(window, &self.taps);
        (even, odd)
    }
}

/// 2x half-band decimator. Consumes an even/odd input pair and returns one
/// output sample with a latency of `half - 1` output samples.
#[derive(Clone)]
struct HalfbandDown {
    taps: Arc<[f32]>,
    odd_history: MirroredHistory,
    even_delay: Vec<f32>,
    even_position: usize,
}

impl HalfbandDown {
    fn new(half: usize) -> Self {
        Self {
            taps: stage_taps(half),
            odd_history: MirroredHistory::new(2 * half),
            even_delay: vec![0.0; half],
            even_position: 0,
        }
    }

    #[inline]
    fn process(&mut self, even: f32, odd: f32) -> f32 {
        let length = self.even_delay.len();
        self.even_delay[self.even_position] = even;
        self.even_position = (self.even_position + 1) % length;
        let centre = self.even_delay[self.even_position];
        let window = self.odd_history.push(odd);
        0.5 * centre + dot(window, &self.taps)
    }
}

/// One channel of 4x oversampling around a memoryless nonlinearity.
#[derive(Clone)]
pub struct Oversampler4x {
    up_first: HalfbandUp,
    up_second: HalfbandUp,
    down_second: HalfbandDown,
    down_first: HalfbandDown,
    /// One 2x-rate sample of delay that keeps the total latency integral.
    alignment: f32,
}

impl Default for Oversampler4x {
    fn default() -> Self {
        Self::new()
    }
}

impl Oversampler4x {
    /// Latency in base-rate frames: `2 * M1 - 1` for the first stage plus
    /// `M2` for the second stage (including the alignment sample).
    pub const LATENCY: usize = 2 * STAGE1_HALF - 1 + STAGE2_HALF;

    pub fn new() -> Self {
        Self {
            up_first: HalfbandUp::new(STAGE1_HALF),
            up_second: HalfbandUp::new(STAGE2_HALF),
            down_second: HalfbandDown::new(STAGE2_HALF),
            down_first: HalfbandDown::new(STAGE1_HALF),
            alignment: 0.0,
        }
    }

    /// Process one base-rate sample, applying `shape` to the four
    /// oversampled values. The result is delayed by [`Self::LATENCY`].
    #[inline]
    pub fn process(&mut self, input: f32, mut shape: impl FnMut(f32) -> f32) -> f32 {
        let (a0, a1) = self.up_first.process(input);
        let (b0, b1) = self.up_second.process(a0);
        let (b2, b3) = self.up_second.process(a1);
        let c0 = self.down_second.process(shape(b0), shape(b1));
        let c1 = self.down_second.process(shape(b2), shape(b3));
        let delayed = self.alignment;
        self.alignment = c1;
        self.down_first.process(delayed, c0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(frequency: f64, sample_rate: f64, frames: usize, amplitude: f64) -> Vec<f32> {
        (0..frames)
            .map(|index| {
                ((std::f64::consts::TAU * frequency * index as f64 / sample_rate).sin() * amplitude)
                    as f32
            })
            .collect()
    }

    fn tone_amplitude(samples: &[f32], frequency: f64, sample_rate: f64) -> f64 {
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (index, sample) in samples.iter().enumerate() {
            let phase = std::f64::consts::TAU * frequency * index as f64 / sample_rate;
            re += f64::from(*sample) * phase.cos();
            im += f64::from(*sample) * phase.sin();
        }
        2.0 * (re * re + im * im).sqrt() / samples.len() as f64
    }

    #[test]
    fn halfband_taps_have_unity_dc_gain() {
        for half in [STAGE1_HALF, STAGE2_HALF] {
            let taps = design_halfband(half, HALFBAND_BETA);
            let sum: f32 = taps.iter().sum();
            assert!((sum - 0.5).abs() < 1e-6, "half={half} sum={sum}");
            for (left, right) in taps.iter().zip(taps.iter().rev()) {
                assert!((left - right).abs() < 1e-9, "taps must be symmetric");
            }
        }
    }

    #[test]
    fn linear_path_is_a_pure_delay_in_the_passband() {
        let sample_rate = 44_100.0;
        for frequency in [100.0, 1_000.0, 8_000.0, 18_000.0] {
            let input = tone(frequency, sample_rate, 8_192, 0.5);
            let mut oversampler = Oversampler4x::new();
            let output: Vec<f32> = input
                .iter()
                .map(|sample| oversampler.process(*sample, |value| value))
                .collect();
            let latency = Oversampler4x::LATENCY;
            let worst = output[latency + 512..]
                .iter()
                .zip(&input[512..])
                .map(|(out, reference)| (out - reference).abs())
                .fold(0.0f32, f32::max);
            assert!(worst < 2e-3, "{frequency} Hz deviates by {worst}");
        }
    }

    #[test]
    fn oversampled_clipping_has_far_less_aliasing() {
        // A 5 kHz tone clipped hard: the 7th harmonic (35 kHz) folds to
        // 9.1 kHz when the clipper runs at 44.1 kHz.
        let sample_rate = 44_100.0;
        let input = tone(5_000.0, sample_rate, 22_050, 0.9);
        let clip = |value: f32| (value * 8.0).tanh();
        let naive: Vec<f32> = input.iter().map(|sample| clip(*sample)).collect();
        let mut oversampler = Oversampler4x::new();
        let oversampled: Vec<f32> = input
            .iter()
            .map(|sample| oversampler.process(*sample, clip))
            .collect();
        let alias_naive = tone_amplitude(&naive[1_000..], 9_100.0, sample_rate);
        let alias_oversampled = tone_amplitude(&oversampled[1_000..], 9_100.0, sample_rate);
        let fundamental = tone_amplitude(&oversampled[1_000..], 5_000.0, sample_rate);
        assert!(fundamental > 0.9, "fundamental lost: {fundamental}");
        assert!(
            alias_oversampled < alias_naive * 0.05,
            "alias {alias_oversampled} vs naive {alias_naive}"
        );
    }
}
