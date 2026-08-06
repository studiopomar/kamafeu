use crate::dsp::pitch_bend::PitchBendSolver;
use crate::project::model::UPitchBendPoint;

const BASE64_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encode pitch bend value (in 10-cents units) into 2 UTAU Base64 characters
pub fn encode_pitch_value_to_base64(val_10cents: i16) -> (char, char) {
    let clamped = val_10cents.clamp(-2048, 2047);
    let u = (clamped as u16) & 0x0FFF;
    let high = ((u >> 6) & 0x3F) as usize;
    let low = (u & 0x3F) as usize;
    (BASE64_ALPHABET[high] as char, BASE64_ALPHABET[low] as char)
}

/// Encode UPitchBendPoint list into standard UTAU Base64 pitch bend string for macres & UTAU CLI resamplers
pub fn encode_utau_base64_pitch(points: &[UPitchBendPoint], duration_ms: f64) -> String {
    if points.is_empty() {
        return String::new();
    }

    let mut b64_chars = String::new();
    let step_ms = 10.0f64;
    let num_samples = ((duration_ms / step_ms).ceil() as usize).max(1);

    for i in 0..num_samples {
        let t = i as f64 * step_ms;
        let cents = PitchBendSolver::get_pitch_offset_cents(t, points);
        let val_10cents = (cents / 10.0).round() as i16;
        let (c1, c2) = encode_pitch_value_to_base64(val_10cents);
        b64_chars.push(c1);
        b64_chars.push(c2);
    }

    format!("#10#{}", b64_chars)
}

/// Encode UPitchBendPoint list into UTAU Mode 2 pitch bend parameters string (PBS, PBW, PBY, PBM)
pub fn encode_pitch_bend_string(points: &[UPitchBendPoint], _tempo: f64) -> String {
    if points.is_empty() {
        return String::new();
    }

    let pbs_time = points[0].time_offset_ms;
    let pbs_pitch = points[0].pitch_offset_cents / 10.0;

    let mut pbw = Vec::new();
    let mut pby = Vec::new();
    let mut pbm = Vec::new();

    for window in points.windows(2) {
        let p0 = &window[0];
        let p1 = &window[1];
        let width = (p1.time_offset_ms - p0.time_offset_ms).max(0.0);
        let height_delta = (p1.pitch_offset_cents - p0.pitch_offset_cents) / 10.0;

        pbw.push(format!("{:.1}", width));
        pby.push(format!("{:.1}", height_delta));
        pbm.push(if p1.shape.is_empty() {
            "s".to_string()
        } else {
            p1.shape.clone()
        });
    }

    let pbw_str = pbw.join(",");
    let pby_str = pby.join(",");
    let pbm_str = pbm.join(",");

    format!(
        "PBS={:.1},{:.1};PBW={};PBY={};PBM={}",
        pbs_time, pbs_pitch, pbw_str, pby_str, pbm_str
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_pitch_value_to_base64() {
        let (c1, c2) = encode_pitch_value_to_base64(0);
        assert_eq!((c1, c2), ('A', 'A'));
    }

    #[test]
    fn test_encode_utau_base64_pitch() {
        let points = vec![
            UPitchBendPoint {
                time_offset_ms: 0.0,
                pitch_offset_cents: 0.0,
                shape: "".to_string(),
            },
            UPitchBendPoint {
                time_offset_ms: 100.0,
                pitch_offset_cents: 100.0,
                shape: "s".to_string(),
            },
        ];

        let encoded = encode_utau_base64_pitch(&points, 100.0);
        assert!(encoded.starts_with("#10#"));
        assert!(encoded.len() > 10);
    }
}
