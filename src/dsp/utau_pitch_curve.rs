//! Decoder for the compact signed 12-bit pitch values passed to classic UTAU
//! resamplers. This is an independent implementation based on static analysis;
//! it does not import UTAU code or change Kamafeu's project format.

use crate::project::model::UPitchBendPoint;
use std::fmt;

const BASE64_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// One compact UTAU pitch value, in cents relative to the note pitch.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UtauPitchValue {
    pub cents: i16,
    /// Frequency multiplier `2^(cents / 1200)`; zero cents therefore maps to 1.
    pub frequency_ratio: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UtauPitchCurveError {
    InvalidBase64 { byte: u8, index: usize },
    MissingValueBeforeRepeat { index: usize },
    InvalidRepeatCount { index: usize },
    RepeatCountOverflow { index: usize },
    MissingRepeatTerminator { index: usize },
    IncompleteValuePair { index: usize },
    InvalidFrameStep,
}

impl fmt::Display for UtauPitchCurveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBase64 { byte, index } => {
                write!(f, "invalid Base64 byte {byte:#04x} at offset {index}")
            }
            Self::MissingValueBeforeRepeat { index } => {
                write!(f, "repeat marker at offset {index} has no preceding value")
            }
            Self::InvalidRepeatCount { index } => {
                write!(f, "invalid repeat count at offset {index}")
            }
            Self::RepeatCountOverflow { index } => {
                write!(f, "repeat count overflows at offset {index}")
            }
            Self::MissingRepeatTerminator { index } => {
                write!(f, "repeat marker at offset {index} has no closing '#' ")
            }
            Self::IncompleteValuePair { index } => {
                write!(f, "incomplete 12-bit value pair at offset {index}")
            }
            Self::InvalidFrameStep => write!(f, "frame step must be finite and positive"),
        }
    }
}

impl std::error::Error for UtauPitchCurveError {}

fn base64_value(byte: u8, index: usize) -> Result<u16, UtauPitchCurveError> {
    BASE64_ALPHABET
        .iter()
        .position(|candidate| *candidate == byte)
        .map(|value| value as u16)
        .ok_or(UtauPitchCurveError::InvalidBase64 { byte, index })
}

fn value_from_pair(high: u8, low: u8, index: usize) -> Result<i16, UtauPitchCurveError> {
    // Confirmed by the analyzed parser and by Kamafeu's existing encoder:
    // the first character carries the high six bits, the second the low six.
    let raw = (base64_value(high, index)? << 6) | base64_value(low, index + 1)?;
    let signed = if raw & 0x0800 != 0 {
        raw as i32 - 0x1000
    } else {
        raw as i32
    };
    Ok(signed as i16)
}

/// Decode compact Base64 pairs and `#count#` repeats into signed 12-bit cents.
///
/// `max_values` bounds allocation and output length. A repeat count is the
/// number of additional copies of the last decoded value, matching the
/// existing Kamafeu encoder. Confirmed from the static pseudocode: alphabet,
/// pair order, signed range, and repeat behavior. The meaning of frame spacing
/// is intentionally not inferred here; callers supply it to the adapter.
pub fn decode_utau_pitch_values(
    encoded: &str,
    max_values: usize,
) -> Result<Vec<UtauPitchValue>, UtauPitchCurveError> {
    let bytes = encoded.as_bytes();
    let mut values = Vec::with_capacity(bytes.len().min(max_values));
    let mut index = 0;
    let mut previous = None;

    while index < bytes.len() && values.len() < max_values {
        if bytes[index] == b'#' {
            let marker_index = index;
            let last = previous.ok_or(UtauPitchCurveError::MissingValueBeforeRepeat {
                index: marker_index,
            })?;
            index += 1;
            let count_start = index;
            let mut count = 0usize;
            while index < bytes.len() && bytes[index].is_ascii_digit() {
                count = count
                    .checked_mul(10)
                    .and_then(|count| count.checked_add((bytes[index] - b'0') as usize))
                    .ok_or(UtauPitchCurveError::RepeatCountOverflow { index: count_start })?;
                index += 1;
            }
            if index == count_start {
                return Err(UtauPitchCurveError::InvalidRepeatCount { index: count_start });
            }
            if index >= bytes.len() || bytes[index] != b'#' {
                return Err(UtauPitchCurveError::MissingRepeatTerminator {
                    index: marker_index,
                });
            }
            index += 1;
            let copies = count.min(max_values - values.len());
            values.extend(std::iter::repeat_n(last, copies));
            continue;
        }

        if index + 1 >= bytes.len() {
            return Err(UtauPitchCurveError::IncompleteValuePair { index });
        }
        let cents = value_from_pair(bytes[index], bytes[index + 1], index)?;
        let value = UtauPitchValue {
            cents,
            frequency_ratio: 2.0f64.powf(f64::from(cents) / 1200.0),
        };
        values.push(value);
        previous = Some(value);
        index += 2;
    }

    Ok(values)
}

/// Decode the curve and adapt its frame values to Kamafeu's existing pitch
/// point type. `frame_step_ms` is explicit because UTAU's P/Q flags select the
/// sampling grid; no default grid is guessed here. Frame-to-point conversion is
/// an adapter, not evidence that both renderers interpolate frames identically.
/// The `l` point shape is a practical linear approximation; UTAU's interpolation
/// between decoded frames remains unconfirmed by the static analysis.
pub fn decode_utau_pitch_curve(
    encoded: &str,
    max_values: usize,
    frame_step_ms: f64,
) -> Result<Vec<UPitchBendPoint>, UtauPitchCurveError> {
    if !frame_step_ms.is_finite() || frame_step_ms <= 0.0 {
        return Err(UtauPitchCurveError::InvalidFrameStep);
    }
    let values = decode_utau_pitch_values(encoded, max_values)?;
    Ok(values
        .into_iter()
        .enumerate()
        .map(|(frame, value)| UPitchBendPoint {
            time_offset_ms: frame as f64 * frame_step_ms,
            pitch_offset_cents: f64::from(value.cents),
            shape: "l".to_string(),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_signed_values_with_encoder_pair_order() {
        let values = decode_utau_pitch_values("AAf/gA", 3).unwrap();
        assert_eq!(
            values.iter().map(|value| value.cents).collect::<Vec<_>>(),
            [0, 2047, -2048]
        );
        assert_eq!(values[0].frequency_ratio, 1.0);
        assert!((values[1].frequency_ratio - 2.0f64.powf(2047.0 / 1200.0)).abs() < 1e-12);
    }

    #[test]
    fn expands_repeat_count_as_additional_copies_and_respects_limit() {
        let values = decode_utau_pitch_values("AB#3#CD", 3).unwrap();
        assert_eq!(values.len(), 3);
        assert!(values.iter().all(|value| value.cents == 1));
        assert_eq!(decode_utau_pitch_values("AB#2#CD", 8).unwrap().len(), 4);
    }

    #[test]
    fn adapts_cents_to_existing_project_points_with_explicit_frame_spacing() {
        let points = decode_utau_pitch_curve("ABAC", 4, 5.25).unwrap();
        assert_eq!(points[0].time_offset_ms, 0.0);
        assert_eq!(points[1].time_offset_ms, 5.25);
        assert_eq!(points[0].pitch_offset_cents, 1.0);
        assert_eq!(points[1].pitch_offset_cents, 2.0);
    }

    #[test]
    fn reports_malformed_pairs_and_repeats() {
        assert!(matches!(
            decode_utau_pitch_values("A", 4),
            Err(UtauPitchCurveError::IncompleteValuePair { .. })
        ));
        assert!(matches!(
            decode_utau_pitch_values("#2#", 4),
            Err(UtauPitchCurveError::MissingValueBeforeRepeat { .. })
        ));
        assert!(matches!(
            decode_utau_pitch_values("AB#x#", 4),
            Err(UtauPitchCurveError::InvalidRepeatCount { .. })
        ));
        assert_eq!(
            decode_utau_pitch_curve("AB", 4, f64::NAN),
            Err(UtauPitchCurveError::InvalidFrameStep)
        );
    }
}
