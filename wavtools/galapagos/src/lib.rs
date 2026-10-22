//! Reusable envelope and overlap-mixing functions for a Galapagos-style
//! wavtool. The API has no dependency on Kamafeu or on a particular synth.

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

/// Applies piecewise-linear gain through five ordered envelope points.
pub fn apply_linear_envelope(
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

/// Returns the linearly interpolated gain at `time_ms`.
pub fn gain_at(time_ms: f64, points: &[EnvelopePoint; 5]) -> f64 {
    if time_ms <= points[0].time_ms {
        return points[0].gain;
    }
    for pair in points.windows(2) {
        if time_ms <= pair[1].time_ms {
            let width = (pair[1].time_ms - pair[0].time_ms).max(0.001);
            let position = ((time_ms - pair[0].time_ms) / width).clamp(0.0, 1.0);
            return pair[0].gain + (pair[1].gain - pair[0].gain) * position;
        }
    }
    points[4].gain
}

/// Chooses a bounded shift that maximizes squared energy in an existing
/// overlap. `incoming` is expected to have its envelope applied already.
/// Returns zero when the pitch or overlap is unusable.
#[allow(clippy::needless_range_loop)]
pub fn optimize_overlap(
    previous_overlap: &[f32],
    incoming: &[f32],
    sample_rate: u32,
    fundamental_hz: f64,
) -> isize {
    if sample_rate == 0 || !fundamental_hz.is_finite() || fundamental_hz <= 0.0 {
        return 0;
    }
    let common_len = previous_overlap.len().min(incoming.len());
    let period = (sample_rate as f64 / fundamental_hz).round() as isize;
    if common_len < 8 || period < 2 {
        return 0;
    }
    let max_shift = (period / 2)
        .min((sample_rate as isize * 5 / 1000).max(1))
        .min((common_len as isize - 1) / 4);
    if max_shift <= 0 {
        return 0;
    }

    let range_start = max_shift as usize;
    let range_end = common_len.saturating_sub(max_shift as usize);
    if range_end <= range_start {
        return 0;
    }

    let mut best_shift = 0isize;
    let mut best_energy = f64::NEG_INFINITY;
    for shift in -max_shift..=max_shift {
        let mut energy = 0.0f64;
        for destination_index in range_start..range_end {
            let source_index = (destination_index as isize - shift) as usize;
            let mixed =
                f64::from(previous_overlap[destination_index]) + f64::from(incoming[source_index]);
            energy += mixed * mixed;
        }
        if energy > best_energy + 1e-12
            || ((energy - best_energy).abs() <= 1e-12
                && shift.unsigned_abs() < best_shift.unsigned_abs())
        {
            best_energy = energy;
            best_shift = shift;
        }
    }
    if best_energy.is_finite() && best_energy > 1e-12 {
        best_shift
    } else {
        0
    }
}
/// Applies the envelope, optimizes only an existing overlap, then additively
/// mixes the segment. Returns the placement adjustment in samples.
#[allow(clippy::too_many_arguments)]
pub fn mix_with_optimized_overlap(
    phrase: &mut [f32],
    incoming: &mut [f32],
    nominal_start_sample: usize,
    previous_end_sample: usize,
    sample_rate: u32,
    sample_time_zero_ms: f64,
    points: &[EnvelopePoint; 5],
    fundamental_hz: Option<f64>,
) -> isize {
    apply_linear_envelope(incoming, sample_rate, sample_time_zero_ms, points);
    if sample_rate == 0 || nominal_start_sample >= phrase.len() {
        return 0;
    }
    let overlap_len = previous_end_sample
        .min(phrase.len())
        .saturating_sub(nominal_start_sample)
        .min(incoming.len());
    let correction = fundamental_hz.map_or(0, |frequency| {
        optimize_overlap(
            &phrase[nominal_start_sample..nominal_start_sample + overlap_len],
            &incoming[..overlap_len],
            sample_rate,
            frequency,
        )
    });

    let corrected_start = nominal_start_sample as i128 + correction as i128;
    let (destination_start, source_start) = if corrected_start < 0 {
        (0, (-corrected_start) as usize)
    } else {
        (corrected_start.min(usize::MAX as i128) as usize, 0)
    };
    if destination_start >= phrase.len() || source_start >= incoming.len() {
        return correction;
    }
    let count = (phrase.len() - destination_start).min(incoming.len() - source_start);
    for index in 0..count {
        phrase[destination_start + index] += incoming[source_start + index];
    }
    correction
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_point_envelope_interpolates_linearly() {
        let points = [
            EnvelopePoint::new(0.0, 0.0),
            EnvelopePoint::new(10.0, 1.0),
            EnvelopePoint::new(20.0, 1.0),
            EnvelopePoint::new(90.0, 1.0),
            EnvelopePoint::new(100.0, 0.0),
        ];
        assert_eq!(gain_at(0.0, &points), 0.0);
        assert_eq!(gain_at(5.0, &points), 0.5);
        assert_eq!(gain_at(50.0, &points), 1.0);
        assert_eq!(gain_at(100.0, &points), 0.0);
    }

    #[test]
    fn overlap_search_reduces_periodic_cancellation() {
        let previous = (0..400)
            .map(|i| (std::f64::consts::TAU * i as f64 / 20.0).sin() as f32)
            .collect::<Vec<_>>();
        let incoming = (0..400)
            .map(|i| (std::f64::consts::TAU * (i as f64 + 3.0) / 20.0).sin() as f32)
            .collect::<Vec<_>>();
        let shift = optimize_overlap(&previous, &incoming, 1_000, 50.0);
        let before = (5..395)
            .map(|i| {
                let sum = previous[i] + incoming[i];
                f64::from(sum * sum)
            })
            .sum::<f64>();
        let after = (5..395)
            .map(|i| {
                let sum = previous[i] + incoming[(i as isize - shift) as usize];
                f64::from(sum * sum)
            })
            .sum::<f64>();
        assert_ne!(shift, 0);
        assert!(after > before * 1.2);
        assert!(shift.unsigned_abs() <= 5);
    }

    #[test]
    fn overlap_search_leaves_silence_unshifted() {
        assert_eq!(optimize_overlap(&[0.0; 100], &[0.0; 100], 1_000, 100.0), 0);
        assert_eq!(optimize_overlap(&[0.5; 100], &[0.5; 100], 0, 100.0), 0);
    }

    #[test]
    fn phrase_mixer_uses_only_the_declared_previous_segment_overlap() {
        let points = [EnvelopePoint::new(-10.0, 1.0); 5];
        let start = 100usize;
        let previous_end = 300usize;
        let mut phrase = vec![0.0; 500];
        let mut incoming = (0..240)
            .map(|i| (std::f64::consts::TAU * (i as f64 + 3.0) / 20.0).sin() as f32)
            .collect::<Vec<_>>();
        for (i, sample) in phrase[start..previous_end].iter_mut().enumerate() {
            *sample = (std::f64::consts::TAU * i as f64 / 20.0).sin() as f32;
        }
        let before = (5..195)
            .map(|i| {
                let sum = phrase[start + i] + incoming[i];
                f64::from(sum * sum)
            })
            .sum::<f64>();

        let shift = mix_with_optimized_overlap(
            &mut phrase,
            &mut incoming,
            start,
            previous_end,
            1_000,
            0.0,
            &points,
            Some(50.0),
        );
        let after = (5..195)
            .map(|i| f64::from(phrase[start + i] * phrase[start + i]))
            .sum::<f64>();

        assert_ne!(shift, 0);
        assert!(after > before * 1.2);
        assert!(phrase[start + incoming.len() + 5..]
            .iter()
            .all(|sample| sample.abs() < 1e-6));
    }
}
