use crate::project::model::{UNote, UPitchBendPoint};

pub fn smooth_pitch_points(raw_points: &[(f64, f64)]) -> Vec<UPitchBendPoint> {
    if raw_points.is_empty() {
        return Vec::new();
    }

    // Pointer samples are not guaranteed to arrive in chronological order:
    // the user may draw from right to left.  Smoothing an unsorted stroke can
    // create a large backward segment and an apparent loop in the curve.
    let mut normalized = raw_points.to_vec();
    normalized.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    let mut ordered: Vec<(f64, f64)> = Vec::with_capacity(normalized.len());
    for (time, cents) in normalized {
        let cents = cents.clamp(-2400.0, 2400.0);
        if let Some(last) = ordered.last_mut() {
            if (last.0 - time).abs() < 0.001 {
                last.1 = (last.1 + cents) * 0.5;
                continue;
            }
        }
        ordered.push((time, cents));
    }

    if ordered.is_empty() {
        return Vec::new();
    }
    if ordered.len() <= 2 {
        return ordered
            .iter()
            .map(|&(t, c)| UPitchBendPoint {
                time_offset_ms: t,
                pitch_offset_cents: c,
                shape: "s".to_string(),
            })
            .collect();
    }

    let n = ordered.len();
    let mut smoothed = Vec::with_capacity(n);
    let kernel = [0.06136, 0.24477, 0.38774, 0.24477, 0.06136]; // 5-tap Gaussian kernel

    for i in 0..n {
        let t_center = ordered[i].0;
        let mut weighted_cents = 0.0;
        let mut weight_sum = 0.0;

        for (k_idx, &weight) in kernel.iter().enumerate() {
            let offset = k_idx as isize - 2;
            let sample_idx = (i as isize + offset).clamp(0, n as isize - 1) as usize;
            weighted_cents += ordered[sample_idx].1 * weight;
            weight_sum += weight;
        }

        let smoothed_cents = if weight_sum > 0.0 {
            weighted_cents / weight_sum
        } else {
            raw_points[i].1
        };

        smoothed.push((t_center, smoothed_cents));
    }

    let mut relaxed = Vec::with_capacity(n);
    for i in 0..n {
        let prev = smoothed[i.saturating_sub(1)].1;
        let curr = smoothed[i].1;
        let next = smoothed[(i + 1).min(n - 1)].1;
        relaxed.push((smoothed[i].0, (prev + curr * 2.0 + next) * 0.25));
    }

    let mut result: Vec<UPitchBendPoint> = Vec::new();
    let min_spacing_ms = 18.0;

    for (t, c) in relaxed {
        if let Some(last) = result.last() {
            if (t - last.time_offset_ms).abs() < min_spacing_ms {
                continue;
            }
        }
        result.push(UPitchBendPoint {
            time_offset_ms: t,
            pitch_offset_cents: c,
            shape: "s".to_string(),
        });
    }

    result
}

/// Canonical pitch point shapes used by the OpenUtau-style point editor.
/// Keep the persisted representation as strings so existing Kamafeu projects
/// and exporters remain compatible.
pub fn canonical_pitch_shape(shape: &str) -> &'static str {
    match shape.to_ascii_lowercase().as_str() {
        "l" | "linear" => "l",
        "i" | "j" | "easein" | "ease-in" => "i",
        "o" | "r" | "easeout" | "ease-out" => "o",
        "sp" | "spline" | "h" | "hermite" | "cubic" => "sp",
        _ => "io",
    }
}

/// Insert one explicit point and return its sorted index.
///
/// OpenUtau edits the curve as a sequence of explicit points instead of
/// storing a sampled brush stroke. This helper deliberately does no smoothing
/// or simplification: the point the user created is the point we persist.
pub fn insert_pitch_point(note: &mut UNote, time_offset_ms: f64, pitch_offset_cents: f64) -> usize {
    note.pitch_bend.points.push(UPitchBendPoint {
        time_offset_ms,
        pitch_offset_cents: pitch_offset_cents.clamp(-2400.0, 2400.0),
        shape: "io".to_string(),
    });
    note.pitch_bend.points.sort_by(|left, right| {
        left.time_offset_ms
            .partial_cmp(&right.time_offset_ms)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    note.pitch_bend
        .points
        .iter()
        .position(|point| {
            (point.time_offset_ms - time_offset_ms).abs() < 1e-6
                && (point.pitch_offset_cents - pitch_offset_cents).abs() < 1e-6
        })
        .unwrap_or_else(|| note.pitch_bend.points.len().saturating_sub(1))
}
