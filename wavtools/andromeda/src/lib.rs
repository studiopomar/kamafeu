//! Reusable cosine-envelope and additive-mixing functions for a wavtool.
//! This crate has no dependency on Kamafeu or on a particular synthesizer.

use std::f64::consts::PI;

/// One time/gain point. Times are milliseconds relative to the segment's
/// sample-time origin; gain is a linear multiplier, not a percentage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EnvelopePoint {
    pub time_ms: f64,
    pub gain: f64,
}

impl EnvelopePoint {
    pub const fn new(time_ms: f64, gain: f64) -> Self {
        Self { time_ms, gain }
    }
}

/// Applies piecewise cosine-eased gain through five ordered envelope points.
pub fn apply_cosine_envelope(
    samples: &mut [f32],
    sample_rate: u32,
    sample_time_zero_ms: f64,
    points: &[EnvelopePoint; 5],
) {
    if sample_rate == 0 {
        samples.fill(0.0);
        return;
    }
    for (index, sample) in samples.iter_mut().enumerate() {
        let time_ms = sample_time_zero_ms + index as f64 * 1000.0 / sample_rate as f64;
        *sample *= gain_at(time_ms, points) as f32;
    }
}

/// Returns the cosine-eased gain at `time_ms`.
pub fn gain_at(time_ms: f64, points: &[EnvelopePoint; 5]) -> f64 {
    if time_ms <= points[0].time_ms {
        return points[0].gain;
    }
    for pair in points.windows(2) {
        if time_ms <= pair[1].time_ms {
            let width = (pair[1].time_ms - pair[0].time_ms).max(0.001);
            let position = ((time_ms - pair[0].time_ms) / width).clamp(0.0, 1.0);
            let eased = 0.5 * (1.0 - (PI * position).cos());
            return pair[0].gain + (pair[1].gain - pair[0].gain) * eased;
        }
    }
    points[4].gain
}

/// Applies the envelope to `incoming`, then additively mixes it into `phrase`
/// at the requested sample position. Returns the last written sample index.
pub fn mix_segment(
    phrase: &mut [f32],
    incoming: &mut [f32],
    start_sample: usize,
    sample_rate: u32,
    sample_time_zero_ms: f64,
    points: &[EnvelopePoint; 5],
) -> usize {
    apply_cosine_envelope(incoming, sample_rate, sample_time_zero_ms, points);
    if start_sample >= phrase.len() {
        return start_sample;
    }
    let count = (phrase.len() - start_sample).min(incoming.len());
    for index in 0..count {
        phrase[start_sample + index] += incoming[index];
    }
    start_sample + count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cosine_envelope_eases_between_points() {
        let points = [
            EnvelopePoint::new(0.0, 0.0),
            EnvelopePoint::new(10.0, 1.0),
            EnvelopePoint::new(20.0, 1.0),
            EnvelopePoint::new(90.0, 1.0),
            EnvelopePoint::new(100.0, 0.0),
        ];
        assert_eq!(gain_at(0.0, &points), 0.0);
        assert!((gain_at(2.5, &points) - 0.1464466094).abs() < 1e-8);
        assert_eq!(gain_at(10.0, &points), 1.0);
        assert_eq!(gain_at(100.0, &points), 0.0);
    }

    #[test]
    fn segment_mix_adds_samples_at_the_requested_position() {
        let points = [
            EnvelopePoint::new(-10.0, 1.0),
            EnvelopePoint::new(0.0, 1.0),
            EnvelopePoint::new(10.0, 1.0),
            EnvelopePoint::new(20.0, 1.0),
            EnvelopePoint::new(30.0, 1.0),
        ];
        let mut phrase = vec![0.25; 8];
        let mut incoming = vec![0.5; 4];
        assert_eq!(
            mix_segment(&mut phrase, &mut incoming, 2, 1_000, 0.0, &points),
            6
        );
        assert_eq!(phrase, vec![0.25, 0.25, 0.75, 0.75, 0.75, 0.75, 0.25, 0.25]);
    }

    #[test]
    fn zero_rate_clears_invalid_input_instead_of_dividing_by_zero() {
        let points = [EnvelopePoint::new(0.0, 1.0); 5];
        let mut samples = vec![1.0; 4];
        apply_cosine_envelope(&mut samples, 0, 0.0, &points);
        assert_eq!(samples, vec![0.0; 4]);
    }
}
