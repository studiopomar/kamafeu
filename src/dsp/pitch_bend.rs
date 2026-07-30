use crate::project::model::UPitchBendPoint;

pub struct PitchBendSolver;

impl PitchBendSolver {
    /// Calculate the pitch offset (in cents) at `time_ms` relative to note start using Mode 2 pitch points.
    pub fn get_pitch_offset_cents(time_ms: f64, points: &[UPitchBendPoint]) -> f64 {
        if points.is_empty() {
            return 0.0;
        }

        // Sort points by time_offset_ms safely
        let mut sorted = points.to_vec();
        sorted.sort_by(|a, b| a.time_offset_ms.partial_cmp(&b.time_offset_ms).unwrap_or(std::cmp::Ordering::Equal));

        let p_first = &sorted[0];
        let p_last = sorted.last().unwrap();

        // 1. Before first point: hold first point's pitch value (OpenUTAU behavior)
        if time_ms < p_first.time_offset_ms {
            return p_first.pitch_offset_cents;
        }

        // 2. After last point: hold the last point's pitch value
        if time_ms >= p_last.time_offset_ms {
            return p_last.pitch_offset_cents;
        }

        // 3. Between consecutive points: interpolate using Cosine Sigmoid S-curve
        for window in sorted.windows(2) {
            let p0 = &window[0];
            let p1 = &window[1];

            if time_ms >= p0.time_offset_ms && time_ms <= p1.time_offset_ms {
                let duration = (p1.time_offset_ms - p0.time_offset_ms).max(1e-3);
                let norm_t = ((time_ms - p0.time_offset_ms) / duration).clamp(0.0, 1.0);

                let factor = match p1.shape.to_lowercase().as_str() {
                    // Linear
                    "r" | "l" => norm_t,
                    // Ease-in only (quadratic)
                    "j" => norm_t * norm_t,
                    // Ease-out only (sqrt curve)
                    "o" => norm_t.sqrt(),
                    // io / inout / s: Cubic Hermite Spline (smoothstep)
                    "s" | "i" | "io" | "" => norm_t * norm_t * (3.0 - 2.0 * norm_t),
                    // Fallback: Cubic Hermite
                    _ => norm_t * norm_t * (3.0 - 2.0 * norm_t),
                };

                return p0.pitch_offset_cents + factor * (p1.pitch_offset_cents - p0.pitch_offset_cents);
            }
        }

        0.0
    }

    /// Calculate smooth pitch transition glide ("bracinho de pitch") between Note A and Note B.
    pub fn get_legato_transition_offset_cents(
        rel_t_ms: f64,
        note_a_midi: u8,
        note_b_midi: u8,
        glide_duration_ms: f64,
    ) -> f64 {
        if note_a_midi == note_b_midi || glide_duration_ms <= 1e-3 {
            return 0.0;
        }

        let total_semitones = note_a_midi as f64 - note_b_midi as f64;
        let total_cents = total_semitones * 100.0;

        let norm_t = ((rel_t_ms + glide_duration_ms * 0.5) / glide_duration_ms).clamp(0.0, 1.0);
        let factor = 0.5 * (1.0 - (std::f64::consts::PI * norm_t).cos());

        (1.0 - factor) * total_cents
    }

    /// Ramer-Douglas-Peucker (RDP) point simplification
    pub fn simplify_pitch_points(points: &[UPitchBendPoint], epsilon_cents: f64) -> Vec<UPitchBendPoint> {
        if points.len() <= 2 {
            return points.to_vec();
        }

        let mut max_dist = 0.0f64;
        let mut max_index = 0usize;

        let p_first = &points[0];
        let p_last = points.last().unwrap();

        for i in 1..points.len() - 1 {
            let p = &points[i];
            let dist = Self::perpendicular_distance(p, p_first, p_last);
            if dist > max_dist {
                max_dist = dist;
                max_index = i;
            }
        }

        if max_dist > epsilon_cents {
            let mut left = Self::simplify_pitch_points(&points[..=max_index], epsilon_cents);
            let right = Self::simplify_pitch_points(&points[max_index..], epsilon_cents);
            left.pop();
            left.extend(right);
            left
        } else {
            vec![p_first.clone(), p_last.clone()]
        }
    }

    fn perpendicular_distance(p: &UPitchBendPoint, line_start: &UPitchBendPoint, line_end: &UPitchBendPoint) -> f64 {
        let dx = line_end.time_offset_ms - line_start.time_offset_ms;
        let dy = line_end.pitch_offset_cents - line_start.pitch_offset_cents;

        let len_sq = dx * dx + dy * dy;
        if len_sq < 1e-6 {
            let px = p.time_offset_ms - line_start.time_offset_ms;
            let py = p.pitch_offset_cents - line_start.pitch_offset_cents;
            return (px * px + py * py).sqrt();
        }

        let num = ((p.time_offset_ms - line_start.time_offset_ms) * dy - (p.pitch_offset_cents - line_start.pitch_offset_cents) * dx).abs();
        num / len_sq.sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pitch_bend_solver() {
        let points = vec![
            UPitchBendPoint { time_offset_ms: 100.0, pitch_offset_cents: 200.0, shape: "s".to_string() },
        ];

        // Before first point: holds first point pitch value (OpenUTAU behavior)
        let p_start = PitchBendSolver::get_pitch_offset_cents(0.0, &points);
        assert_eq!(p_start, 200.0);

        // At the point: should be exactly 200 cents
        let p_mid = PitchBendSolver::get_pitch_offset_cents(100.0, &points);
        assert_eq!(p_mid, 200.0);

        // After last point: should HOLD at 200 cents (not decay to 0)
        let p_after = PitchBendSolver::get_pitch_offset_cents(500.0, &points);
        assert_eq!(p_after, 200.0);
    }

    #[test]
    fn test_pitch_bend_two_points() {
        let points = vec![
            UPitchBendPoint { time_offset_ms: 0.0, pitch_offset_cents: 200.0, shape: "s".to_string() },
            UPitchBendPoint { time_offset_ms: 200.0, pitch_offset_cents: 0.0, shape: "s".to_string() },
        ];

        // At t=0: first point value
        let p0 = PitchBendSolver::get_pitch_offset_cents(0.0, &points);
        assert_eq!(p0, 200.0);

        // At midpoint t=100ms: approx 100 cents (S-curve midpoint)
        let p_mid = PitchBendSolver::get_pitch_offset_cents(100.0, &points);
        assert!((p_mid - 100.0).abs() < 5.0, "Expected ~100 cents, got {}", p_mid);

        // At t=200ms: last point value (0 cents)
        let p_end = PitchBendSolver::get_pitch_offset_cents(200.0, &points);
        assert_eq!(p_end, 0.0);

        // After last point: hold at 0
        let p_after = PitchBendSolver::get_pitch_offset_cents(500.0, &points);
        assert_eq!(p_after, 0.0);
    }

    #[test]
    fn test_legato_transition() {
        let offset_start = PitchBendSolver::get_legato_transition_offset_cents(-40.0, 60, 62, 80.0);
        assert!((offset_start - (-200.0)).abs() < 1.0);
    }
}
