/// Align only confidently periodic material. The caller has already applied
/// complementary envelopes; no additional crossfade or gain is introduced.
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
}
