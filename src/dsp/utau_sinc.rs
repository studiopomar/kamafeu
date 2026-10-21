//! Isolated implementation of the bounded sinc sample interpolator observed
//! in the classic UTAU resampler. It is not used by Kamafeu's active renderer.

use std::fmt;

/// Invalid arguments to [`utau_sinc_interpolate_i16`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UtauSincError {
    NonFinitePosition,
    InvalidScale,
    ZeroStride,
}

impl fmt::Display for UtauSincError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinitePosition => write!(f, "sample position must be finite"),
            Self::InvalidScale => write!(f, "sinc scale must be finite and positive"),
            Self::ZeroStride => write!(f, "sample stride must be positive"),
        }
    }
}

impl std::error::Error for UtauSincError {}

/// Evaluate the truncated `sin(z) / z` series used by the analyzed code.
///
/// The polynomial is mathematically confirmed through the `z^24` term. This
/// independent f64 implementation may differ slightly from x87 extended
/// precision used by the original binary.
#[inline]
fn sinc_series(z: f64) -> f64 {
    if z == 0.0 {
        return 1.0;
    }

    let z_squared = z * z;
    let mut term = 1.0;
    let mut sum = term;
    for n in 1..=12 {
        let n = n as f64;
        term *= -z_squared / ((2.0 * n) * (2.0 * n + 1.0));
        sum += term;
    }
    sum
}

/// Interpolate signed 16-bit PCM at a fractional source-sample position.
///
/// `scale` defines both the kernel width (`position ± 2 * scale`) and final
/// normalization. `stride` is measured in `input` samples: use 1 for mono or
/// select a channel from interleaved PCM with its channel count as the stride.
/// The position is relative to the start of `input`. Samples below zero and
/// samples beyond the slice are ignored, matching the analyzed lower-bound
/// check while adding an explicit safe upper bound.
///
/// Confirmed from static pseudocode: sinc argument, radius, stride, and divide
/// by scale. Approximate: endpoint conversion uses nearest-even rounding to
/// model the x87 helper; the exact original floating-point environment and
/// x87 accumulation precision are not reproduced. No window or weight-sum
/// normalization is applied.
pub fn utau_sinc_interpolate_i16(
    input: &[i16],
    position: f64,
    scale: f64,
    stride: usize,
) -> Result<f64, UtauSincError> {
    if !position.is_finite() {
        return Err(UtauSincError::NonFinitePosition);
    }
    if !scale.is_finite() || scale <= 0.0 {
        return Err(UtauSincError::InvalidScale);
    }
    if stride == 0 {
        return Err(UtauSincError::ZeroStride);
    }

    let Some(last_index) = input.len().checked_sub(1) else {
        return Ok(0.0);
    };
    let last_sample_index = last_index / stride;
    let lower = position - 2.0 * scale;
    let upper = position + 2.0 * scale;
    if !lower.is_finite() || !upper.is_finite() {
        return Err(UtauSincError::InvalidScale);
    }

    let rounded_lower = lower.round_ties_even();
    let rounded_upper = upper.round_ties_even();
    if rounded_upper < 0.0 || rounded_lower > last_sample_index as f64 {
        return Ok(0.0);
    }
    let start = rounded_lower.max(0.0) as usize;
    let end = rounded_upper.min(last_sample_index as f64) as usize;
    if start > end {
        return Ok(0.0);
    }

    let mut sum = 0.0f64;
    let pi_over_scale = std::f64::consts::PI / scale;
    for sample_index in start..=end {
        let source_index = sample_index * stride;
        let z = (sample_index as f64 - position) * pi_over_scale;
        sum += f64::from(input[source_index]) * sinc_series(z);
    }
    Ok(sum / scale)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_integer_sample_for_unit_scale() {
        let input = [0, 100, 0];
        let value = utau_sinc_interpolate_i16(&input, 1.0, 1.0, 1).unwrap();
        assert!((value - 100.0).abs() < 1e-3);
    }

    #[test]
    fn stride_selects_one_interleaved_channel() {
        let interleaved = [10, -10, 20, -20, 30, -30];
        let value = utau_sinc_interpolate_i16(&interleaved, 1.0, 1.0, 2).unwrap();
        assert!((value - 20.0).abs() < 1e-3);
    }

    #[test]
    fn clips_kernel_to_slice_bounds_without_reading_outside() {
        let input = [75];
        let value = utau_sinc_interpolate_i16(&input, 0.0, 1.0, 1).unwrap();
        assert!((value - 75.0).abs() < 1e-3);
        assert_eq!(utau_sinc_interpolate_i16(&input, 20.0, 1.0, 1), Ok(0.0));
    }

    #[test]
    fn rejects_invalid_scale_stride_and_position() {
        assert_eq!(
            utau_sinc_interpolate_i16(&[1], f64::NAN, 1.0, 1),
            Err(UtauSincError::NonFinitePosition)
        );
        assert_eq!(
            utau_sinc_interpolate_i16(&[1], 0.0, 0.0, 1),
            Err(UtauSincError::InvalidScale)
        );
        assert_eq!(
            utau_sinc_interpolate_i16(&[1], 0.0, 1.0, 0),
            Err(UtauSincError::ZeroStride)
        );
    }
}
