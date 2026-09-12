/// Align only confidently periodic material. The caller has already applied
/// complementary envelopes; no additional crossfade or gain is introduced.
///
/// Correlating the complete waveform is not sufficient for a CVVC/VCV join:
/// the two aliases can have noticeably different formants even though their
/// shared vowel has the same fundamental.  In that case a waveform
/// correlation can reject a perfectly usable overlap and leave a phase jump
/// at the end of the OTO fade.  Measure the phase of the fundamental first,
/// then retain the correlation search as a conservative fallback.
pub fn correction(previous: &[f32], incoming: &[f32], frequency: f64, rate: u32) -> isize {
    if !(60.0..=1500.0).contains(&frequency) || rate == 0 {
        return 0;
    }
    let period = (rate as f64 / frequency).round() as usize;
    let count = previous.len().min(incoming.len());
    if period < 3 || count < period * 3 {
        return 0;
    }
    let lag_limit = (period / 2).min((rate as usize * 5) / 1000).min(count / 8);
    if lag_limit == 0 {
        return 0;
    }
    let correlation = |a: &[f32], b: &[f32]| {
        let (mut dot, mut aa, mut bb) = (0.0f64, 0.0f64, 0.0f64);
        for (&x, &y) in a.iter().zip(b) {
            dot += x as f64 * y as f64;
            aa += x as f64 * x as f64;
            bb += y as f64 * y as f64;
        }
        if aa < 1e-8 || bb < 1e-8 {
            0.0
        } else {
            dot / (aa * bb).sqrt()
        }
    };
    let a = &previous[..count];
    let b = &incoming[..count];
    if correlation(&a[..count - period], &a[period..]) < 0.7
        || correlation(&b[..count - period], &b[period..]) < 0.7
    {
        return 0;
    }
    let reference = &a[lag_limit..count - lag_limit];

    // Project both overlap windows onto the expected fundamental. This is a
    // small, allocation-free equivalent of the phase measurement used by
    // convergence-style wavtools. It deliberately ignores harmonics, whose
    // balance differs between neighbouring CVVC aliases.
    let fundamental_phase = |samples: &[f32]| {
        let omega = std::f64::consts::TAU * frequency / rate as f64;
        let (mut sin_sum, mut cos_sum, mut energy) = (0.0f64, 0.0f64, 0.0f64);
        for (index, &sample) in samples.iter().enumerate() {
            let angle = omega * index as f64;
            let value = sample as f64;
            sin_sum += value * angle.sin();
            cos_sum += value * angle.cos();
            energy += value * value;
        }
        // Random/noisy material also has a phase, but its projection is very
        // small compared with its total energy. Reject it before it can move a
        // fricative or breath segment.
        let coherence = (sin_sum * sin_sum + cos_sum * cos_sum).sqrt()
            / (energy.sqrt() * (samples.len().max(1) as f64).sqrt()).max(1e-12);
        (cos_sum.atan2(sin_sum), coherence)
    };
    let (previous_phase, previous_coherence) = fundamental_phase(reference);
    let (incoming_phase, incoming_coherence) = fundamental_phase(&b[lag_limit..count - lag_limit]);
    if previous_coherence > 0.09 && incoming_coherence > 0.09 {
        let mut phase_delta = incoming_phase - previous_phase;
        while phase_delta > std::f64::consts::PI {
            phase_delta -= std::f64::consts::TAU;
        }
        while phase_delta <= -std::f64::consts::PI {
            phase_delta += std::f64::consts::TAU;
        }
        let phase_shift = (phase_delta / std::f64::consts::TAU * period as f64).round() as isize;
        if phase_shift.unsigned_abs() <= lag_limit {
            return phase_shift;
        }
    }

    let base = correlation(reference, &b[lag_limit..count - lag_limit]);
    let (mut best, mut shift) = (base, 0isize);
    for lag in -(lag_limit as isize)..=lag_limit as isize {
        let start = (lag_limit as isize - lag) as usize;
        let score = correlation(reference, &b[start..start + reference.len()]);
        if score > best {
            best = score;
            shift = lag;
        }
    }
    if best >= 0.7 && best - base > 0.15 {
        shift
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn opposite_phase_is_corrected_with_a_bounded_shift() {
        let a: Vec<_> = (0..2000)
            .map(|i| (i as f32 * std::f32::consts::TAU / 200.0).sin())
            .collect();
        let b: Vec<_> = a.iter().map(|x| -x).collect();
        assert_eq!(correction(&a, &a, 220.5, 44100), 0);
        assert_eq!(correction(&a, &b, 220.5, 44100).abs(), 100);
        assert_eq!(correction(&a[..100], &b[..100], 220.5, 44100), 0);
        assert_eq!(correction(&vec![0.0; 2000], &b, 220.5, 44100), 0);
    }

    #[test]
    fn fundamental_phase_aligns_aliases_with_different_harmonics() {
        let rate = 44_100;
        let frequency = 220.5;
        let period = (rate as f64 / frequency).round() as isize;
        let previous: Vec<_> = (0..4_000)
            .map(|index| {
                let phase = std::f64::consts::TAU * frequency * index as f64 / rate as f64;
                (0.45 * phase.sin() + 0.25 * (phase * 2.0).sin()) as f32
            })
            .collect();
        let incoming: Vec<_> = (0..4_000)
            .map(|index| {
                let phase = std::f64::consts::TAU * frequency * index as f64 / rate as f64
                    + std::f64::consts::PI * 0.25;
                (0.45 * phase.sin() + 0.40 * (phase * 3.0).sin()) as f32
            })
            .collect();

        let shift = correction(&previous, &incoming, frequency, rate);
        assert!(
            (shift - period / 8).abs() <= 1,
            "expected about {} samples, got {shift}",
            period / 8
        );
    }
}
