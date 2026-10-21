//! Offline look-ahead peak limiter used on rendered tracks and mixes.
//!
//! The former render "limiter" normalised the entire buffer by its loudest
//! sample and then soft-clipped everything above 92 % of the ceiling. A single
//! transient therefore lowered the level of the whole song, each progressive
//! preview chunk was scaled by a different factor, and the tanh stage added
//! harmonic distortion even to material that never exceeded the ceiling.
//!
//! [`PeakLimiter`] computes a smooth, channel-linked gain envelope that acts
//! only around the overs:
//!
//! * a 4x oversampled detector estimates inter-sample (true) peaks;
//! * a forward-looking sliding minimum provides look-ahead and hold;
//! * an exponential release lets the gain recover without pumping;
//! * two cascaded moving averages turn each gain change into a smooth
//!   S-curve while still guaranteeing that the ceiling is never exceeded.
//!
//! Material that already stays below the ceiling is returned bit-identical,
//! and the processing is local: audio far away from an over is not touched,
//! so chunked renders and full renders agree outside the boundary context.

use super::kaiser::{kaiser, sinc};
use std::collections::VecDeque;
use std::sync::OnceLock;

/// Number of taps of each fractional-delay phase of the true-peak detector.
const TRUE_PEAK_TAPS: usize = 12;
/// Index of the tap aligned with the current sample (taps cover `-5..=6`).
const TRUE_PEAK_CENTER: usize = 5;
/// Fractional phases evaluated between two samples (0.25, 0.5 and 0.75).
const TRUE_PEAK_PHASES: usize = 3;
/// Frames grouped when looking for regions that may contain overs.
const PEAK_BLOCK: usize = 32;

struct TruePeakKernel {
    phases: [[f32; TRUE_PEAK_TAPS]; TRUE_PEAK_PHASES],
    /// Largest L1 norm of a phase. Any interpolated value is bounded by
    /// `l1_bound * max(|x|)` over the taps, which lets the detector skip
    /// quiet regions without evaluating the FIR.
    l1_bound: f32,
}

fn true_peak_kernel() -> &'static TruePeakKernel {
    static KERNEL: OnceLock<TruePeakKernel> = OnceLock::new();
    KERNEL.get_or_init(|| {
        // Kaiser-windowed sinc interpolator, comparable to the 48-tap /
        // 4-phase reference filter of ITU-R BS.1770 true-peak meters.
        let beta = 5.0;
        let half_width = 6.5;
        let mut phases = [[0.0f32; TRUE_PEAK_TAPS]; TRUE_PEAK_PHASES];
        let mut l1_bound = 1.0f32;
        for (phase_index, phase) in phases.iter_mut().enumerate() {
            let fraction = (phase_index + 1) as f64 / (TRUE_PEAK_PHASES + 1) as f64;
            let mut taps = [0.0f64; TRUE_PEAK_TAPS];
            for (tap_index, tap) in taps.iter_mut().enumerate() {
                let distance = fraction - (tap_index as f64 - TRUE_PEAK_CENTER as f64);
                *tap = sinc(distance) * kaiser(distance / half_width, beta);
            }
            let dc: f64 = taps.iter().sum();
            let mut l1 = 0.0f64;
            for (destination, tap) in phase.iter_mut().zip(taps) {
                let normalized = tap / dc;
                l1 += normalized.abs();
                *destination = normalized as f32;
            }
            l1_bound = l1_bound.max(l1 as f32);
        }
        TruePeakKernel { phases, l1_bound }
    })
}

/// Summary of the gain reduction applied by [`PeakLimiter`].
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LimiterReport {
    /// Smallest linear gain applied to any frame (1.0 when untouched).
    pub min_gain: f32,
    /// Number of frames whose gain was reduced.
    pub limited_frames: usize,
    /// Number of non-finite samples replaced by silence.
    pub sanitized_samples: usize,
}

impl LimiterReport {
    pub fn max_reduction_db(&self) -> f32 {
        if self.min_gain > 0.0 {
            -20.0 * self.min_gain.log10()
        } else {
            f32::INFINITY
        }
    }
}

/// Channel-linked look-ahead brickwall limiter for interleaved buffers.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeakLimiter {
    /// Linear ceiling. Output samples never exceed it in magnitude.
    pub ceiling: f32,
    /// Duration of the S-shaped gain ramp that precedes an over.
    pub attack_ms: f32,
    /// Time constant of the exponential gain recovery.
    pub release_ms: f32,
    /// Detect inter-sample peaks with a 4x oversampled interpolator.
    pub true_peak: bool,
}

impl PeakLimiter {
    pub fn new(ceiling: f32) -> Self {
        Self {
            ceiling,
            attack_ms: 2.0,
            release_ms: 80.0,
            true_peak: true,
        }
    }

    /// Limit `buffer` (interleaved, `channels` per frame) in place.
    pub fn process_interleaved(
        &self,
        buffer: &mut [f32],
        channels: usize,
        sample_rate: u32,
    ) -> LimiterReport {
        let mut report = LimiterReport {
            min_gain: 1.0,
            ..LimiterReport::default()
        };
        let channels = channels.max(1);
        let frames = buffer.len() / channels;
        if frames == 0 {
            return report;
        }
        let ceiling = if self.ceiling.is_finite() {
            self.ceiling.clamp(1e-6, 1.0e6)
        } else {
            1.0
        };

        let kernel = true_peak_kernel();
        let bound = if self.true_peak { kernel.l1_bound } else { 1.0 };

        // Sanitize and collect per-block peaks in a single pass. Quiet blocks
        // are skipped by the detector, and a fully quiet buffer returns here
        // without modifying a single bit.
        let block_count = frames.div_ceil(PEAK_BLOCK);
        let mut block_peaks = vec![0.0f32; block_count];
        let mut global_peak = 0.0f32;
        for (frame_index, frame) in buffer[..frames * channels]
            .chunks_exact_mut(channels)
            .enumerate()
        {
            let mut peak = 0.0f32;
            for sample in frame {
                if !sample.is_finite() {
                    *sample = 0.0;
                    report.sanitized_samples += 1;
                }
                peak = peak.max(sample.abs());
            }
            let block = &mut block_peaks[frame_index / PEAK_BLOCK];
            *block = block.max(peak);
            global_peak = global_peak.max(peak);
        }
        let safe_level = ceiling / bound;
        if global_peak <= safe_level {
            return report;
        }

        let sample_rate = sample_rate.max(1) as f64;
        // Two cascaded boxcars of `ramp` frames form a triangular smoothing
        // kernel spanning `window` frames; the sliding minimum must look
        // exactly that far ahead for the ceiling guarantee to hold. The ramp
        // is never shorter than the detector, which must only read frames
        // that have not been written yet.
        let ramp = ((f64::from(self.attack_ms.max(0.0)) * 0.5e-3 * sample_rate).round() as usize)
            .max(TRUE_PEAK_TAPS / 2 + 1);
        let window = 2 * ramp - 1;
        let lookahead = window - 1;
        let release_alpha = if self.release_ms > 0.0 {
            1.0 - (-1.0 / (f64::from(self.release_ms) * 1e-3 * sample_rate)).exp()
        } else {
            1.0
        };

        let required_gain = |frame: usize, buffer: &[f32]| -> f32 {
            let block_first = frame.saturating_sub(TRUE_PEAK_CENTER) / PEAK_BLOCK;
            let block_last =
                (frame + TRUE_PEAK_TAPS - TRUE_PEAK_CENTER).min(frames - 1) / PEAK_BLOCK;
            let neighbourhood = block_peaks[block_first..=block_last]
                .iter()
                .copied()
                .fold(0.0f32, f32::max);
            if neighbourhood <= safe_level {
                return 1.0;
            }
            let base = frame * channels;
            let mut peak = 0.0f32;
            for channel in 0..channels {
                peak = peak.max(buffer[base + channel].abs());
            }
            if self.true_peak {
                for phase in &kernel.phases {
                    for channel in 0..channels {
                        let mut value = 0.0f32;
                        for (tap_index, tap) in phase.iter().enumerate() {
                            let source =
                                frame as isize + tap_index as isize - TRUE_PEAK_CENTER as isize;
                            if source >= 0 && (source as usize) < frames {
                                value += buffer[source as usize * channels + channel] * tap;
                            }
                        }
                        peak = peak.max(value.abs());
                    }
                }
            }
            if peak > ceiling {
                ceiling / peak
            } else {
                1.0
            }
        };

        // Streaming formulation: the gain of frame `k` is known once the
        // detector has reached frame `k + lookahead`. Stream index `s` maps
        // to detector frame `s - lookahead` and output frame
        // `s - 2 * lookahead`, so unity gain before the buffer start is
        // represented without special cases. The detector reads at most five
        // frames behind its own position, which is still ahead of the frame
        // being written because `lookahead > TRUE_PEAK_TAPS / 2`.
        let mut minimum: VecDeque<(usize, f32)> = VecDeque::with_capacity(window + 1);
        let mut boxcar_a = vec![1.0f64; ramp];
        let mut boxcar_b = vec![1.0f64; ramp];
        let mut sum_a = ramp as f64;
        let mut sum_b = ramp as f64;
        let mut cursor = 0usize;
        let mut released = 1.0f64;
        let inverse_ramp = 1.0 / ramp as f64;

        for stream_index in 0..frames + 2 * lookahead {
            let detect_frame = stream_index as isize - lookahead as isize;
            let gain = if detect_frame >= 0 && (detect_frame as usize) < frames {
                required_gain(detect_frame as usize, buffer)
            } else {
                1.0
            };
            while minimum.back().is_some_and(|&(_, value)| value >= gain) {
                minimum.pop_back();
            }
            minimum.push_back((stream_index, gain));
            if stream_index < lookahead {
                continue;
            }
            let window_start = stream_index - lookahead;
            while minimum
                .front()
                .is_some_and(|&(index, _)| index < window_start)
            {
                minimum.pop_front();
            }
            let lookahead_min = minimum.front().map_or(1.0, |&(_, value)| f64::from(value));

            released = lookahead_min.min(released + (1.0 - released) * release_alpha);
            if released > 1.0 - 1e-9 {
                // Let a fully recovered gain return to exact unity, so
                // material after an over is passed through bit-identically.
                released = 1.0;
            }
            sum_a += released - boxcar_a[cursor];
            boxcar_a[cursor] = released;
            let smoothed_a = sum_a * inverse_ramp;
            sum_b += smoothed_a - boxcar_b[cursor];
            boxcar_b[cursor] = smoothed_a;
            cursor = (cursor + 1) % ramp;
            let smoothed = ((sum_b * inverse_ramp) as f32).min(1.0);

            let output_frame = window_start as isize - lookahead as isize;
            if output_frame < 0 || output_frame as usize >= frames {
                continue;
            }
            let base = output_frame as usize * channels;
            if smoothed < 1.0 {
                report.limited_frames += 1;
                report.min_gain = report.min_gain.min(smoothed);
                for sample in &mut buffer[base..base + channels] {
                    *sample = (*sample * smoothed).clamp(-ceiling, ceiling);
                }
            } else {
                // Rounding can leave a sample a few ULPs above the ceiling
                // when the required reduction is below f32 resolution.
                for sample in &mut buffer[base..base + channels] {
                    *sample = sample.clamp(-ceiling, ceiling);
                }
            }
        }
        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(frequency: f32, sample_rate: u32, frames: usize, amplitude: f32) -> Vec<f32> {
        (0..frames)
            .map(|index| {
                (std::f32::consts::TAU * frequency * index as f32 / sample_rate as f32).sin()
                    * amplitude
            })
            .collect()
    }

    fn noise(frames: usize, amplitude: f32, seed: u32) -> Vec<f32> {
        let mut state = seed;
        (0..frames)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                ((state >> 8) as f32 / 0x00ff_ffff as f32 * 2.0 - 1.0) * amplitude
            })
            .collect()
    }

    #[test]
    fn material_below_the_ceiling_is_bit_identical() {
        let mut buffer = sine(440.0, 44_100, 44_100, 0.5);
        let original = buffer.clone();
        let report = PeakLimiter::new(0.89).process_interleaved(&mut buffer, 1, 44_100);
        assert_eq!(buffer, original);
        assert_eq!(report.limited_frames, 0);
        assert_eq!(report.min_gain, 1.0);
    }

    #[test]
    fn output_never_exceeds_the_ceiling() {
        for channels in [1usize, 2] {
            let mut buffer = noise(44_100 * channels, 2.5, 0x5eed_1234);
            let ceiling = 10.0f32.powf(-1.0 / 20.0);
            let report =
                PeakLimiter::new(ceiling).process_interleaved(&mut buffer, channels, 44_100);
            let peak = buffer
                .iter()
                .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
            assert!(peak <= ceiling, "channels={channels} peak={peak}");
            assert!(report.limited_frames > 0);
            assert!(report.max_reduction_db() > 6.0);
        }
    }

    #[test]
    fn a_single_transient_does_not_change_the_level_of_the_whole_buffer() {
        let sample_rate = 44_100;
        let mut buffer = sine(220.0, sample_rate, sample_rate as usize * 2, 0.5);
        let original = buffer.clone();
        buffer[sample_rate as usize] = 1.8;
        PeakLimiter::new(0.89).process_interleaved(&mut buffer, 1, sample_rate);

        // The old normaliser scaled every sample by 0.89 / 1.8. Audio before
        // the look-ahead window and after the release must stay untouched.
        let attack_region = sample_rate as usize - 200;
        assert_eq!(&buffer[..attack_region], &original[..attack_region]);
        let settled = sample_rate as usize + sample_rate as usize * 3 / 4;
        let max_tail_error = buffer[settled..]
            .iter()
            .zip(&original[settled..])
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(max_tail_error < 1e-4, "tail error {max_tail_error}");
        assert!(buffer[sample_rate as usize] <= 0.89);
    }

    #[test]
    fn stereo_channels_share_the_same_gain() {
        let sample_rate = 48_000;
        let frames = 9_600;
        let left = sine(300.0, sample_rate, frames, 1.6);
        let right = sine(300.0, sample_rate, frames, 0.4);
        let mut buffer: Vec<f32> = left
            .iter()
            .zip(&right)
            .flat_map(|(l, r)| [*l, *r])
            .collect();
        PeakLimiter::new(0.9).process_interleaved(&mut buffer, 2, sample_rate);
        for (index, (frame, (l, r))) in buffer
            .chunks_exact(2)
            .zip(left.iter().zip(&right))
            .enumerate()
        {
            if r.abs() > 0.05 {
                let gain_left = frame[0] / l;
                let gain_right = frame[1] / r;
                assert!(
                    (gain_left - gain_right).abs() < 1e-4,
                    "image shifted at frame {index}: {gain_left} vs {gain_right}"
                );
            }
        }
    }

    #[test]
    fn gain_changes_are_smooth() {
        let sample_rate = 44_100;
        let mut buffer = vec![0.25f32; 8_192];
        for sample in &mut buffer[4_000..4_100] {
            *sample = 2.0;
        }
        PeakLimiter::new(0.5).process_interleaved(&mut buffer, 1, sample_rate);
        let gains: Vec<f32> = buffer
            .iter()
            .map(|sample| {
                if *sample > 0.3 {
                    sample / 2.0
                } else {
                    sample / 0.25
                }
            })
            .collect();
        let largest_step = gains
            .windows(2)
            .enumerate()
            .filter(|(index, _)| !(3_999..=4_099).contains(index))
            .map(|(_, pair)| (pair[1] - pair[0]).abs())
            .fold(0.0f32, f32::max);
        // A 2 ms S-curve moving the gain from 1.0 to 0.25 never jumps by
        // more than a few percent per sample.
        assert!(largest_step < 0.03, "largest gain step {largest_step}");
        assert!(buffer.iter().all(|sample| sample.abs() <= 0.5));
    }

    #[test]
    fn inter_sample_peaks_are_detected() {
        // A tone at fs/4 with a 45 degree phase offset has sample peaks at
        // 0.707 of its true amplitude.
        let sample_rate = 48_000;
        let mut buffer: Vec<f32> = (0..4_800)
            .map(|index| {
                (std::f32::consts::FRAC_PI_2 * index as f32 + std::f32::consts::FRAC_PI_4).sin()
            })
            .collect();
        let sample_peak = buffer.iter().fold(0.0f32, |peak, s| peak.max(s.abs()));
        assert!(sample_peak < 0.71);
        let report = PeakLimiter::new(0.8).process_interleaved(&mut buffer, 1, sample_rate);
        assert!(
            report.limited_frames > 0,
            "true peak above ceiling was missed"
        );
        let middle = &buffer[1_000..3_800];
        let limited_peak = middle.iter().fold(0.0f32, |peak, s| peak.max(s.abs()));
        assert!(
            (limited_peak - 0.8 * std::f32::consts::FRAC_1_SQRT_2).abs() < 0.03,
            "sample peak after true-peak limiting was {limited_peak}"
        );
    }

    #[test]
    fn non_finite_samples_are_silenced() {
        let mut buffer = vec![0.1, f32::NAN, f32::INFINITY, -0.2];
        let report = PeakLimiter::new(0.9).process_interleaved(&mut buffer, 1, 44_100);
        assert_eq!(report.sanitized_samples, 2);
        assert!(buffer.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn chunked_processing_matches_the_full_render_inside_the_chunk() {
        let sample_rate = 44_100;
        let frames = sample_rate as usize * 6;
        let mut full = sine(180.0, sample_rate, frames, 0.6);
        for (index, sample) in full.iter_mut().enumerate() {
            // Overs at 1 s and 4.5 s only.
            if (44_100..44_400).contains(&index) || (198_450..198_800).contains(&index) {
                *sample *= 2.4;
            }
        }
        let source = full.clone();
        PeakLimiter::new(0.89).process_interleaved(&mut full, 1, sample_rate);

        // Preview chunk 2 s..4 s rendered with 1 s of leading context.
        let context_start = sample_rate as usize;
        let chunk_start = 2 * sample_rate as usize;
        let chunk_end = 4 * sample_rate as usize;
        let mut chunk = source[context_start..chunk_end].to_vec();
        PeakLimiter::new(0.89).process_interleaved(&mut chunk, 1, sample_rate);
        let chunk = &chunk[chunk_start - context_start..];
        let max_error = chunk
            .iter()
            .zip(&full[chunk_start..chunk_end])
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(
            max_error < 1e-5,
            "chunk differs from full render by {max_error}"
        );
    }
}
