//! Kaiser window helpers shared by the interpolation and resampling kernels.

/// Zeroth-order modified Bessel function of the first kind.
pub(crate) fn bessel_i0(x: f64) -> f64 {
    let mut sum = 1.0;
    let mut term = 1.0;
    let half = x * 0.5;
    for k in 1..128 {
        let factor = half / k as f64;
        term *= factor * factor;
        sum += term;
        if term < sum * 1e-17 {
            break;
        }
    }
    sum
}

/// Kaiser window evaluated at `ratio` in `[-1, 1]` (zero outside).
pub(crate) fn kaiser(ratio: f64, beta: f64) -> f64 {
    if !(-1.0..=1.0).contains(&ratio) {
        return 0.0;
    }
    bessel_i0(beta * (1.0 - ratio * ratio).sqrt()) / bessel_i0(beta)
}

/// Normalised sinc, `sin(pi x) / (pi x)`.
pub(crate) fn sinc(x: f64) -> f64 {
    if x.abs() < 1e-12 {
        1.0
    } else {
        let pi_x = std::f64::consts::PI * x;
        pi_x.sin() / pi_x
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bessel_matches_reference_values() {
        // Abramowitz & Stegun table 9.8.
        assert!((bessel_i0(0.0) - 1.0).abs() < 1e-15);
        assert!((bessel_i0(1.0) - 1.266_065_877_752_008).abs() < 1e-12);
        assert!((bessel_i0(5.0) - 27.239_871_823_604_45).abs() < 1e-9);
    }

    #[test]
    fn kaiser_is_unity_at_the_centre_and_zero_outside() {
        assert!((kaiser(0.0, 8.0) - 1.0).abs() < 1e-15);
        assert_eq!(kaiser(1.5, 8.0), 0.0);
        assert!(kaiser(0.99, 8.0) < 0.01);
    }
}
