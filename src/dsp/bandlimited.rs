//! Band-limited sample-rate conversion.
//!
//! Voicebank recordings, external resampler outputs and imported audio are
//! frequently stored at a rate different from the project rate (48 kHz banks
//! in a 44.1 kHz project are the common case). The previous converter used a
//! 6-tap Lanczos kernel whose cutoff stayed at the *input* Nyquist frequency:
//! when downsampling, everything between the two Nyquist frequencies folded
//! back as aliasing, and the short kernel attenuated the top octave by several
//! dB. Imported audio was converted with plain linear interpolation.
//!
//! This module implements a classic band-limited interpolator (Smith & Gossett)
//! with a Kaiser-windowed sinc prototype stored in a finely sampled table:
//!
//! * the cutoff follows `min(input, output)` Nyquist, so downsampling is
//!   properly anti-aliased and upsampling does not create images;
//! * 32 zero crossings with beta = 9 give a ~90 dB stopband. The transition
//!   band (about +/-9 % around the cutoff) is placed so that any residual
//!   aliasing lands above 20 kHz for 44.1/48 kHz outputs, while the passband
//!   stays flat to ~20 kHz;
//! * every output frame is computed from its absolute position
//!   `frame * input_rate / output_rate` using exact integer arithmetic, so a
//!   sub-range conversion is bit-identical to the same frames of a full
//!   conversion. Chunked renders can therefore convert only what they need.

use super::kaiser::{kaiser, sinc};
use std::sync::OnceLock;

const ZERO_CROSSINGS: usize = 32;
const TABLE_RESOLUTION: usize = 512;
const KAISER_BETA: f64 = 9.0;
const TABLE_LIMIT: usize = ZERO_CROSSINGS * TABLE_RESOLUTION;

struct SincTable {
    values: Vec<f32>,
    deltas: Vec<f32>,
}

fn sinc_table() -> &'static SincTable {
    static TABLE: OnceLock<SincTable> = OnceLock::new();
    TABLE.get_or_init(|| {
        let values: Vec<f32> = (0..=TABLE_LIMIT + 1)
            .map(|index| {
                let distance = index as f64 / TABLE_RESOLUTION as f64;
                if distance >= ZERO_CROSSINGS as f64 {
                    0.0
                } else {
                    (sinc(distance) * kaiser(distance / ZERO_CROSSINGS as f64, KAISER_BETA)) as f32
                }
            })
            .collect();
        let deltas = values
            .windows(2)
            .map(|pair| pair[1] - pair[0])
            .chain(std::iter::once(0.0))
            .collect();
        SincTable { values, deltas }
    })
}

/// Number of output frames covering `input_frames` after conversion.
pub fn converted_frame_count(input_frames: usize, input_rate: u32, output_rate: u32) -> usize {
    if input_rate == 0 || output_rate == 0 {
        return input_frames;
    }
    ((input_frames as f64) * f64::from(output_rate) / f64::from(input_rate)).round() as usize
}

/// Convert a mono buffer to `output_len` samples at `output_rate`.
pub fn resample(input: &[f32], output_len: usize, input_rate: u32, output_rate: u32) -> Vec<f32> {
    resample_interleaved_range(input, 1, 1, input_rate, output_rate, 0, output_len)
}

/// Convert interleaved audio and return output frames
/// `first_frame..first_frame + frame_count`.
///
/// `input_channels` is the frame stride of `input`; only the first
/// `output_channels` channels are converted and written (interleaved) to the
/// result. Positions outside the input are treated as silence.
pub fn resample_interleaved_range(
    input: &[f32],
    input_channels: usize,
    output_channels: usize,
    input_rate: u32,
    output_rate: u32,
    first_frame: usize,
    frame_count: usize,
) -> Vec<f32> {
    let input_channels = input_channels.max(1);
    let output_channels = output_channels.clamp(1, input_channels);
    let input_frames = input.len() / input_channels;
    let mut output = vec![0.0f32; frame_count * output_channels];
    if input_frames == 0 || frame_count == 0 {
        return output;
    }

    if input_rate == output_rate || input_rate == 0 || output_rate == 0 {
        for (offset, destination) in output.chunks_exact_mut(output_channels).enumerate() {
            let frame = first_frame + offset;
            if frame >= input_frames {
                break;
            }
            let source = &input[frame * input_channels..frame * input_channels + output_channels];
            destination.copy_from_slice(source);
        }
        return output;
    }

    let table = sinc_table();
    // Cutoff relative to the input Nyquist frequency.
    let cutoff = (f64::from(output_rate) / f64::from(input_rate)).min(1.0);
    let half_width = ZERO_CROSSINGS as f64 / cutoff;
    let table_scale = cutoff * TABLE_RESOLUTION as f64;
    let gain = cutoff as f32;
    let input_rate = u64::from(input_rate);
    let output_rate = u64::from(output_rate);
    let mut accumulator = vec![0.0f32; output_channels];

    for (offset, destination) in output.chunks_exact_mut(output_channels).enumerate() {
        let numerator = (first_frame + offset) as u64 * input_rate;
        let integer = (numerator / output_rate) as f64;
        let fraction = (numerator % output_rate) as f64 / output_rate as f64;
        let position = integer + fraction;

        let first = (position - half_width).ceil().max(0.0) as usize;
        let last_position = (position + half_width).floor();
        if last_position < 0.0 || first >= input_frames {
            continue;
        }
        let last = (last_position as usize).min(input_frames - 1);
        if first > last {
            continue;
        }

        accumulator.fill(0.0);
        for source_frame in first..=last {
            let distance = (position - source_frame as f64).abs() * table_scale;
            let index = distance as usize;
            if index >= TABLE_LIMIT {
                continue;
            }
            let blend = (distance - index as f64) as f32;
            let weight = table.values[index] + table.deltas[index] * blend;
            let base = source_frame * input_channels;
            for (channel, sum) in accumulator.iter_mut().enumerate() {
                *sum += input[base + channel] * weight;
            }
        }
        for (destination, sum) in destination.iter_mut().zip(&accumulator) {
            *destination = sum * gain;
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tone(frequency: f64, sample_rate: u32, frames: usize) -> Vec<f32> {
        (0..frames)
            .map(|index| {
                (std::f64::consts::TAU * frequency * index as f64 / f64::from(sample_rate)).sin()
                    as f32
            })
            .collect()
    }

    fn rms(samples: &[f32]) -> f64 {
        (samples.iter().map(|s| f64::from(*s).powi(2)).sum::<f64>() / samples.len() as f64).sqrt()
    }

    /// Amplitude of `frequency` in `samples` measured by correlation.
    fn tone_amplitude(samples: &[f32], frequency: f64, sample_rate: u32) -> f64 {
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (index, sample) in samples.iter().enumerate() {
            let phase = std::f64::consts::TAU * frequency * index as f64 / f64::from(sample_rate);
            re += f64::from(*sample) * phase.cos();
            im += f64::from(*sample) * phase.sin();
        }
        2.0 * (re * re + im * im).sqrt() / samples.len() as f64
    }

    #[test]
    fn passband_tones_are_reconstructed_accurately() {
        for (input_rate, output_rate) in [(48_000, 44_100), (44_100, 48_000), (96_000, 44_100)] {
            for frequency in [440.0, 5_000.0, 15_000.0, 19_000.0] {
                let input = tone(frequency, input_rate, input_rate as usize / 2);
                let output_len = converted_frame_count(input.len(), input_rate, output_rate);
                let output = resample(&input, output_len, input_rate, output_rate);
                let expected = tone(frequency, output_rate, output_len);
                let edge = output_rate as usize / 100;
                let error: Vec<f32> = output[edge..output_len - edge]
                    .iter()
                    .zip(&expected[edge..output_len - edge])
                    .map(|(a, b)| a - b)
                    .collect();
                let error_db = 20.0 * (rms(&error) / std::f64::consts::FRAC_1_SQRT_2).log10();
                assert!(
                    error_db < -60.0,
                    "{input_rate}->{output_rate} {frequency} Hz error {error_db:.1} dB"
                );
            }
        }
    }

    #[test]
    fn downsampling_rejects_content_above_the_output_nyquist() {
        // 30 kHz cannot be represented at 44.1 kHz; the old Lanczos kernel
        // folded it back to 14.1 kHz at roughly -10 dB.
        let input_rate = 96_000;
        let output_rate = 44_100;
        let input = tone(30_000.0, input_rate, input_rate as usize / 2);
        let output_len = converted_frame_count(input.len(), input_rate, output_rate);
        let output = resample(&input, output_len, input_rate, output_rate);
        let edge = output_rate as usize / 50;
        let alias = tone_amplitude(&output[edge..output_len - edge], 14_100.0, output_rate);
        let alias_db = 20.0 * alias.log10();
        assert!(alias_db < -80.0, "alias at {alias_db:.1} dB");

        // 24.5 kHz would fold to 19.6 kHz, inside the audible band.
        let input = tone(24_500.0, input_rate, input_rate as usize / 2);
        let output = resample(&input, output_len, input_rate, output_rate);
        let alias = tone_amplitude(&output[edge..output_len - edge], 19_600.0, output_rate);
        let alias_db = 20.0 * alias.log10();
        assert!(alias_db < -80.0, "audible alias at {alias_db:.1} dB");

        // 48 kHz -> 44.1 kHz: the transition band is placed above 20 kHz, so
        // a 23 kHz component folds to 21.1 kHz (inaudible) and is still
        // attenuated. Lanczos-3 passed it almost unchanged.
        let input = tone(23_000.0, 48_000, 24_000);
        let output_len = converted_frame_count(input.len(), 48_000, 44_100);
        let output = resample(&input, output_len, 48_000, 44_100);
        let near_nyquist = tone_amplitude(&output[441..output_len - 441], 21_100.0, 44_100);
        assert!(
            20.0 * near_nyquist.log10() < -20.0,
            "near-Nyquist alias {near_nyquist}"
        );
    }

    #[test]
    fn identical_rates_copy_the_input() {
        let input = tone(1_000.0, 44_100, 1_000);
        assert_eq!(resample(&input, input.len(), 44_100, 44_100), input);
    }

    #[test]
    fn sub_ranges_match_a_full_conversion_exactly() {
        let frames = 12_000;
        let left = tone(700.0, 48_000, frames);
        let right = tone(2_300.0, 48_000, frames);
        let interleaved: Vec<f32> = left
            .iter()
            .zip(&right)
            .flat_map(|(l, r)| [*l, *r, 0.25])
            .collect();
        let total = converted_frame_count(frames, 48_000, 44_100);
        let full = resample_interleaved_range(&interleaved, 3, 2, 48_000, 44_100, 0, total);
        let start = 4_321;
        let count = 2_000;
        let part = resample_interleaved_range(&interleaved, 3, 2, 48_000, 44_100, start, count);
        assert_eq!(part, full[start * 2..(start + count) * 2]);
    }

    #[test]
    fn constant_signals_keep_their_level() {
        let input = vec![0.5f32; 20_000];
        let output_len = converted_frame_count(input.len(), 44_100, 48_000);
        let output = resample(&input, output_len, 44_100, 48_000);
        for sample in &output[200..output_len - 200] {
            assert!((sample - 0.5).abs() < 1e-4, "DC drifted to {sample}");
        }
    }
}
