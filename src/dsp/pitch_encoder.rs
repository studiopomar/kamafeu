use crate::dsp::pitch_bend::PitchBendSolver;
use crate::project::model::UPitchBendPoint;

const BASE64_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Encode one pitch value in cents into UTAU's signed 12-bit Base64 pair.
pub fn encode_pitch_value_to_base64(val_cents: i16) -> (char, char) {
    let clamped = val_cents.clamp(-2048, 2047);
    let u = (clamped as u16) & 0x0FFF;
    let high = ((u >> 6) & 0x3F) as usize;
    let low = (u & 0x3F) as usize;
    (BASE64_ALPHABET[high] as char, BASE64_ALPHABET[low] as char)
}

fn encode_pitch_values(values_cents: impl IntoIterator<Item = i16>) -> String {
    let mut encoded = String::new();
    let mut previous = None;
    let mut duplicates = 0usize;

    for value in values_cents {
        let pair = encode_pitch_value_to_base64(value);
        if previous == Some(pair) {
            duplicates += 1;
            continue;
        }

        if duplicates > 0 {
            encoded.push('#');
            encoded.push_str(&duplicates.to_string());
            encoded.push('#');
            duplicates = 0;
        }
        encoded.push(pair.0);
        encoded.push(pair.1);
        previous = Some(pair);
    }

    if duplicates > 0 {
        encoded.push('#');
        encoded.push_str(&duplicates.to_string());
        encoded.push('#');
    }
    encoded
}

/// Encode a pitch curve for classic UTAU-compatible resamplers.
///
/// The protocol samples one integer-cent value every five musical ticks. The
/// `#n#` form is run-length compression for repeated pairs; it is not a sample
/// interval marker.
pub fn encode_utau_base64_pitch(
    points: &[UPitchBendPoint],
    duration_ms: f64,
    tempo_bpm: f64,
) -> String {
    if points.is_empty() {
        return String::new();
    }

    let safe_tempo = if tempo_bpm.is_finite() && tempo_bpm > 0.0 {
        tempo_bpm
    } else {
        120.0
    };
    let step_ms = 60_000.0 * 5.0 / (safe_tempo * 480.0);
    let num_samples = ((duration_ms / step_ms).ceil() as usize).max(1);
    encode_pitch_values((0..num_samples).map(|index| {
        let time_ms = index as f64 * step_ms;
        PitchBendSolver::get_pitch_offset_cents(time_ms, points)
            .round()
            .clamp(-2048.0, 2047.0) as i16
    }))
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
    fn utau_run_length_encoding_matches_openutau() {
        assert_eq!(encode_pitch_values([0]), "AA");
        assert_eq!(encode_pitch_values([0, 0]), "AA#1#");
        assert_eq!(encode_pitch_values([0, 0, 1, 1]), "AA#1#AB#1#");
        assert_eq!(encode_pitch_values([-2048, 2047]), "gAf/");
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

        let encoded = encode_utau_base64_pitch(&points, 100.0, 120.0);
        assert!(encoded.starts_with("AA"));
        assert!(!encoded.starts_with("#10#"));
        // 100 ms at 120 BPM contains 20 samples (5.2083 ms apart).
        assert!(encoded.len() >= 20);
    }
}
