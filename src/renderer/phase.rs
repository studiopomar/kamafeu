const OPENUTAU_REFERENCE_RATE: f64 = 44_100.0;
const OPENUTAU_HALF_WINDOW_SAMPLES: f64 = 440.0;
const OPENUTAU_WINDOW_SAMPLES: f64 = 880.0;

#[derive(Debug, Clone, Copy)]
pub(crate) struct PhaseSignature {
    pub(crate) phase_radians: f64,
    pub(crate) frequency: f64,
    pub(crate) window_start: usize,
    pub(crate) window_len: usize,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SegmentPhaseMetadata {
    pub(crate) head: Option<PhaseSignature>,
    pub(crate) tail: Option<PhaseSignature>,
}

fn envelope_point_sample(time_ms: f64, sample_time_zero_ms: f64, sample_rate: u32) -> f64 {
    (time_ms - sample_time_zero_ms) * sample_rate as f64 / 1000.0
}

fn openutau_window(
    samples_len: usize,
    center_sample: f64,
    sample_rate: u32,
) -> Option<(usize, usize)> {
    if samples_len < 4 || sample_rate == 0 || !center_sample.is_finite() {
        return None;
    }
    let scale = sample_rate as f64 / OPENUTAU_REFERENCE_RATE;
    let half = (OPENUTAU_HALF_WINDOW_SAMPLES * scale).round().max(1.0) as isize;
    let requested_len = (OPENUTAU_WINDOW_SAMPLES * scale).round().max(4.0) as usize;
    let start = ((center_sample.round() as isize) - half).max(0) as usize;
    if start >= samples_len {
        return None;
    }
    let len = requested_len.min(samples_len - start);
    (len >= 4).then_some((start, len))
}

/// A small RBJ band-pass followed forwards and backwards. OpenUtau's
/// SharpWavtool uses a zero-phase IIR peak filter before looking for the two
/// peaks surrounding the window centre; this keeps harmonics and differing
/// formants from dominating the phase estimate.
fn zero_phase_peak_filter(samples: &[f32], sample_rate: u32, frequency: f64) -> Option<Vec<f64>> {
    if sample_rate == 0
        || samples.len() < 4
        || !frequency.is_finite()
        || !(20.0..sample_rate as f64 * 0.45).contains(&frequency)
    {
        return None;
    }

    let omega = std::f64::consts::TAU * frequency / sample_rate as f64;
    let alpha = omega.sin() / (2.0 * 5.0); // Q = 5, as in SharpWavtool.
    let a0 = 1.0 + alpha;
    let b0 = alpha / a0;
    let b1 = 0.0;
    let b2 = -alpha / a0;
    let a1 = -2.0 * omega.cos() / a0;
    let a2 = (1.0 - alpha) / a0;

    let filter_once = |input: &[f64]| {
        let mut output = Vec::with_capacity(input.len());
        let (mut x1, mut x2, mut y1, mut y2) = (0.0, 0.0, 0.0, 0.0);
        for &x0 in input {
            let y0 = b0 * x0 + b1 * x1 + b2 * x2 - a1 * y1 - a2 * y2;
            output.push(y0);
            x2 = x1;
            x1 = x0;
            y2 = y1;
            y1 = y0;
        }
        output
    };

    let input: Vec<_> = samples.iter().map(|&sample| f64::from(sample)).collect();
    let mut filtered = filter_once(&input);
    filtered.reverse();
    filtered = filter_once(&filtered);
    filtered.reverse();
    Some(filtered)
}

fn normalized_correlation(left: &[f64], right: &[f64]) -> f64 {
    let (mut dot, mut left_energy, mut right_energy) = (0.0, 0.0, 0.0);
    for (&a, &b) in left.iter().zip(right) {
        dot += a * b;
        left_energy += a * a;
        right_energy += b * b;
    }
    if left_energy <= 1e-12 || right_energy <= 1e-12 {
        0.0
    } else {
        dot / (left_energy * right_energy).sqrt()
    }
}

fn analyze_window(
    samples: &[f32],
    window_start: usize,
    window_len: usize,
    nominal_origin_sample: isize,
    sample_rate: u32,
    frequency: f64,
) -> Option<PhaseSignature> {
    let window = samples.get(window_start..window_start + window_len)?;
    if sample_rate == 0 || !frequency.is_finite() || frequency <= 0.0 {
        return None;
    }
    let expected_period = (sample_rate as f64 / frequency).round() as usize;
    if expected_period == 0 || expected_period >= window.len() {
        return None;
    }
    let raw: Vec<_> = window.iter().map(|&sample| f64::from(sample)).collect();
    if normalized_correlation(&raw[..raw.len() - expected_period], &raw[expected_period..]) < 0.2 {
        return None;
    }
    let filtered = zero_phase_peak_filter(window, sample_rate, frequency)?;
    if filtered.iter().any(|sample| !sample.is_finite())
        || filtered.iter().copied().fold(f64::NEG_INFINITY, f64::max) > 10.0
    {
        return None;
    }

    let middle = filtered.len() / 2;
    let left = (1..middle).rev().find(|&index| {
        filtered[index] >= filtered[index - 1] && filtered[index] >= filtered[index + 1]
    })?;
    let right = (middle..filtered.len().saturating_sub(1)).find(|&index| {
        filtered[index] >= filtered[index - 1] && filtered[index] >= filtered[index + 1]
    })?;
    if left >= right {
        return None;
    }

    let period = right - left;
    let measured_frequency = sample_rate as f64 / period as f64;
    if (frequency - measured_frequency).abs() > frequency * 0.25 {
        return None;
    }
    if period >= filtered.len()
        || normalized_correlation(&filtered[..filtered.len() - period], &filtered[period..]) < 0.35
    {
        return None;
    }

    let peak_center = (left + right) as f64 * 0.5;
    let absolute_peak_sample = nominal_origin_sample as f64 + window_start as f64 + peak_center;
    let cycles = absolute_peak_sample / sample_rate as f64 * frequency;
    let phase_radians = std::f64::consts::TAU * (cycles.round() - cycles);
    Some(PhaseSignature {
        phase_radians,
        frequency,
        window_start,
        window_len,
    })
}

/// Extracts only the two short raw windows needed by the convergence stage.
/// The caller must invoke this before the wavtool applies the amplitude
/// envelope. `nominal_origin_sample` maps raw sample zero to the track before
/// any phase correction (start minus skip-over), like OpenUtau's `posSamples -
/// skipSamples`.
pub(crate) fn analyze_openutau_segment(
    samples: &[f32],
    sample_rate: u32,
    sample_time_zero_ms: f64,
    envelope: &[(f64, f64); 5],
    nominal_origin_sample: isize,
    head_frequency: f64,
    tail_frequency: f64,
) -> SegmentPhaseMetadata {
    let head_center_ms = (envelope[0].0 + envelope[1].0) * 0.5;
    let tail_center_ms = (envelope[3].0 + envelope[4].0) * 0.5;
    let head_center = envelope_point_sample(head_center_ms, sample_time_zero_ms, sample_rate);
    let tail_center = envelope_point_sample(tail_center_ms, sample_time_zero_ms, sample_rate);
    let head = openutau_window(samples.len(), head_center, sample_rate).and_then(|(start, len)| {
        analyze_window(
            samples,
            start,
            len,
            nominal_origin_sample,
            sample_rate,
            head_frequency,
        )
    });
    let tail = openutau_window(samples.len(), tail_center, sample_rate).and_then(|(start, len)| {
        analyze_window(
            samples,
            start,
            len,
            nominal_origin_sample,
            sample_rate,
            tail_frequency,
        )
    });
    SegmentPhaseMetadata { head, tail }
}

/// Reproduces SharpWavtool's nearest-cycle correction. The prior segment's
/// placement correction contributes to its phase before the next segment is
/// solved, preventing correction drift across a VCV/CVVC phrase.
pub(crate) fn openutau_convergence_correction(
    previous_tail: Option<PhaseSignature>,
    current_head: Option<PhaseSignature>,
    previous_correction_samples: isize,
    sample_rate: u32,
) -> isize {
    let (Some(previous_tail), Some(current_head)) = (previous_tail, current_head) else {
        return 0;
    };
    if sample_rate == 0
        || !current_head.frequency.is_finite()
        || current_head.frequency <= 0.0
        || !previous_tail.phase_radians.is_finite()
        || !current_head.phase_radians.is_finite()
    {
        return 0;
    }

    let last_correction_angle = previous_correction_samples as f64 * std::f64::consts::TAU
        / sample_rate as f64
        * current_head.frequency;
    let mut difference =
        current_head.phase_radians - (previous_tail.phase_radians - last_correction_angle);
    difference = difference.rem_euclid(std::f64::consts::TAU);
    if difference > std::f64::consts::PI {
        difference -= std::f64::consts::TAU;
    }
    let correction = (difference / std::f64::consts::TAU * sample_rate as f64
        / current_head.frequency)
        .trunc() as isize;
    let half_period = (sample_rate as f64 / current_head.frequency * 0.5).ceil() as isize;
    correction.clamp(-half_period, half_period)
}

/// Align only confidently periodic material. The caller has already applied
/// complementary envelopes; no additional crossfade or gain is introduced.
///
/// Correlating the complete waveform is not sufficient for a CVVC/VCV join:
/// the two aliases can have noticeably different formants even though their
/// shared vowel has the same fundamental.  In that case a waveform
/// correlation can reject a perfectly usable overlap and leave a phase jump
/// at the end of the oto.ini fade. Measure the phase of the fundamental first,
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
    if previous_coherence > 0.35 && incoming_coherence > 0.35 {
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

    fn sine(rate: u32, frequency: f64, count: usize, phase: f64) -> Vec<f32> {
        (0..count)
            .map(|index| {
                (std::f64::consts::TAU * frequency * index as f64 / rate as f64 + phase).sin()
                    as f32
            })
            .collect()
    }

    #[test]
    fn openutau_windows_are_880_samples_at_44100_and_scale_with_rate() {
        let envelope = [
            (10.0, 0.0),
            (20.0, 1.0),
            (30.0, 1.0),
            (80.0, 1.0),
            (90.0, 0.0),
        ];
        let at_44100 = analyze_openutau_segment(
            &sine(44_100, 220.5, 5_000, 0.0),
            44_100,
            0.0,
            &envelope,
            0,
            220.5,
            220.5,
        );
        assert_eq!(at_44100.head.unwrap().window_len, 880);
        assert_eq!(at_44100.tail.unwrap().window_len, 880);

        let at_48000 = analyze_openutau_segment(
            &sine(48_000, 240.0, 5_500, 0.0),
            48_000,
            0.0,
            &envelope,
            0,
            240.0,
            240.0,
        );
        assert_eq!(at_48000.head.unwrap().window_len, 958);
    }

    #[test]
    fn openutau_phase_rejects_silence_and_aperiodic_noise() {
        let envelope = [
            (10.0, 0.0),
            (20.0, 1.0),
            (30.0, 1.0),
            (80.0, 1.0),
            (90.0, 0.0),
        ];
        let silent =
            analyze_openutau_segment(&vec![0.0; 5_000], 44_100, 0.0, &envelope, 0, 220.5, 220.5);
        assert!(silent.head.is_none() && silent.tail.is_none());

        let mut state = 0x1234_5678u32;
        let noise: Vec<_> = (0..5_000)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (state as f64 / u32::MAX as f64 * 2.0 - 1.0) as f32
            })
            .collect();
        let noisy = analyze_openutau_segment(&noise, 44_100, 0.0, &envelope, 0, 220.5, 220.5);
        assert!(noisy.head.is_none() && noisy.tail.is_none());
    }

    #[test]
    fn convergence_uses_previous_correction_and_stays_within_half_period() {
        let signature = |phase_radians| PhaseSignature {
            phase_radians,
            frequency: 441.0,
            window_start: 0,
            window_len: 880,
        };
        assert_eq!(
            openutau_convergence_correction(Some(signature(0.0)), Some(signature(0.0)), 0, 44_100,),
            0
        );
        assert_eq!(
            openutau_convergence_correction(Some(signature(0.0)), Some(signature(0.0)), 50, 44_100,),
            50
        );
        let opposite = openutau_convergence_correction(
            Some(signature(0.0)),
            Some(signature(std::f64::consts::PI)),
            0,
            44_100,
        );
        assert!(opposite.abs() <= 50);
    }

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
