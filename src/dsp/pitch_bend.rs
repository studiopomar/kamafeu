use crate::project::model::UPitchBendPoint;

pub struct PitchBendSolver;

impl PitchBendSolver {
    pub fn get_pitch_offset_cents(time_ms: f64, points: &[UPitchBendPoint]) -> f64 {
        if points.is_empty() {
            return 0.0;
        }

        // Pitch points are kept sorted by every editor/import path. Avoid a
        // clone+sort for every pixel drawn in the piano roll. Malformed project
        // files still take the safe sorting fallback.
        if points
            .windows(2)
            .all(|pair| pair[0].time_offset_ms <= pair[1].time_offset_ms)
        {
            return Self::get_pitch_offset_cents_sorted(time_ms, points);
        }
        let mut sorted = points.to_vec();
        sorted.sort_by(|a, b| {
            a.time_offset_ms
                .partial_cmp(&b.time_offset_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        Self::get_pitch_offset_cents_sorted(time_ms, &sorted)
    }

    pub fn get_pitch_offset_cents_sorted(time_ms: f64, sorted: &[UPitchBendPoint]) -> f64 {
        if sorted.is_empty() {
            return 0.0;
        }

        let p_first = &sorted[0];
        let p_last = sorted.last().unwrap();

        if time_ms < p_first.time_offset_ms {
            return p_first.pitch_offset_cents;
        }

        if time_ms >= p_last.time_offset_ms {
            return p_last.pitch_offset_cents;
        }

        // Binary search reduces long hand-drawn curves from O(n) per sample to
        // O(log n), which is especially noticeable while scrolling or zooming.
        let right = sorted.partition_point(|point| point.time_offset_ms < time_ms);
        let left = right.saturating_sub(1).min(sorted.len() - 2);
        let p0_opt = if left > 0 {
            Some(&sorted[left - 1])
        } else {
            None
        };
        let p1 = &sorted[left];
        let p2 = &sorted[left + 1];
        let p3_opt = if left + 2 < sorted.len() {
            Some(&sorted[left + 2])
        } else {
            None
        };

        if time_ms >= p1.time_offset_ms && time_ms <= p2.time_offset_ms {
            let shape = p1.shape.to_lowercase();
            match shape.as_str() {
                "h" | "hermite" | "c" | "catmull" | "catmull-rom" | "cubic" => {
                    return Self::hermite_interpolate(time_ms, p1, p2, p0_opt, p3_opt);
                }
                _ => {
                    let duration = (p2.time_offset_ms - p1.time_offset_ms).max(1e-3);
                    let norm_t = ((time_ms - p1.time_offset_ms) / duration).clamp(0.0, 1.0);

                    let factor = match shape.as_str() {
                        // OpenUtau names followed by their UTAU Mode 2 aliases.
                        "l" | "linear" => norm_t,
                        "s" | "io" | "s-curve" | "smooth" => {
                            0.5 - 0.5 * (norm_t * std::f64::consts::PI).cos()
                        }
                        "i" | "j" | "easein" | "ease-in" | "exponential" => {
                            1.0 - (norm_t * std::f64::consts::FRAC_PI_2).cos()
                        }
                        "o" | "r" | "easeout" | "ease-out" | "logarithmic" => {
                            (norm_t * std::f64::consts::FRAC_PI_2).sin()
                        }
                        "" => 0.5 - 0.5 * (norm_t * std::f64::consts::PI).cos(),
                        _ => 0.5 - 0.5 * (norm_t * std::f64::consts::PI).cos(),
                    };

                    return p1.pitch_offset_cents
                        + factor * (p2.pitch_offset_cents - p1.pitch_offset_cents);
                }
            }
        }

        0.0
    }

    /// Continuous C^1 Hermite / Non-Uniform Catmull-Rom cubic spline interpolation.
    /// Eliminates pitch slope discontinuities and provides organic vocal transitions.
    pub fn hermite_interpolate(
        time_ms: f64,
        p1: &UPitchBendPoint,
        p2: &UPitchBendPoint,
        p0: Option<&UPitchBendPoint>,
        p3: Option<&UPitchBendPoint>,
    ) -> f64 {
        let dt12 = (p2.time_offset_ms - p1.time_offset_ms).max(1e-3);
        let u = ((time_ms - p1.time_offset_ms) / dt12).clamp(0.0, 1.0);
        let dy = p2.pitch_offset_cents - p1.pitch_offset_cents;
        let s1 = dy / dt12;

        let mut m1 = match p0 {
            Some(prev) => {
                let dt01 = (p1.time_offset_ms - prev.time_offset_ms).max(1e-3);
                let s0 = (p1.pitch_offset_cents - prev.pitch_offset_cents) / dt01;
                let d1 = (dt12 * s0 + dt01 * s1) / (dt01 + dt12);
                d1 * dt12
            }
            None => dy,
        };

        let mut m2 = match p3 {
            Some(next) => {
                let dt23 = (next.time_offset_ms - p2.time_offset_ms).max(1e-3);
                let s2 = (next.pitch_offset_cents - p2.pitch_offset_cents) / dt23;
                let d2 = (dt23 * s1 + dt12 * s2) / (dt12 + dt23);
                d2 * dt12
            }
            None => dy,
        };

        // Monotonicity / overshoot damping (Fritsch-Carlson style)
        if dy.abs() < 1e-4 {
            m1 = 0.0;
            m2 = 0.0;
        } else {
            if m1 * dy < 0.0 {
                m1 = 0.0;
            }
            if m2 * dy < 0.0 {
                m2 = 0.0;
            }
            let max_m = 3.0 * dy.abs();
            if m1.abs() > max_m {
                m1 = max_m * m1.signum();
            }
            if m2.abs() > max_m {
                m2 = max_m * m2.signum();
            }
        }

        let u2 = u * u;
        let u3 = u2 * u;
        let h00 = 2.0 * u3 - 3.0 * u2 + 1.0;
        let h10 = u3 - 2.0 * u2 + u;
        let h01 = -2.0 * u3 + 3.0 * u2;
        let h11 = u3 - u2;

        h00 * p1.pitch_offset_cents + h10 * m1 + h01 * p2.pitch_offset_cents + h11 * m2
    }

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

    pub fn simplify_pitch_points(
        points: &[UPitchBendPoint],
        epsilon_cents: f64,
    ) -> Vec<UPitchBendPoint> {
        if points.len() <= 2 {
            return points.to_vec();
        }

        let mut max_dist = 0.0f64;
        let mut max_index = 0usize;

        let p_first = &points[0];
        let p_last = points.last().unwrap();

        for (i, p) in points.iter().enumerate().take(points.len() - 1).skip(1) {
            let dist = Self::pitch_deviation_from_chord(p, p_first, p_last);
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

    pub fn pitch_deviation_from_chord(
        p: &UPitchBendPoint,
        line_start: &UPitchBendPoint,
        line_end: &UPitchBendPoint,
    ) -> f64 {
        let dt = (line_end.time_offset_ms - line_start.time_offset_ms).max(1e-3);
        let norm_t = ((p.time_offset_ms - line_start.time_offset_ms) / dt).clamp(0.0, 1.0);
        let expected_cents = line_start.pitch_offset_cents
            + norm_t * (line_end.pitch_offset_cents - line_start.pitch_offset_cents);
        (p.pitch_offset_cents - expected_cents).abs()
    }

    pub fn next_pitch_point_shape(current: &str) -> &'static str {
        match current.to_lowercase().as_str() {
            "s" | "io" | "smooth" => "l",
            "l" | "linear" => "j",
            "j" | "i" | "easein" => "r",
            "r" | "o" | "easeout" => "h",
            "h" | "hermite" | "c" | "catmull" => "s",
            _ => "s",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pitch_bend_solver() {
        let points = vec![UPitchBendPoint {
            time_offset_ms: 100.0,
            pitch_offset_cents: 200.0,
            shape: "s".to_string(),
        }];

        let p_start = PitchBendSolver::get_pitch_offset_cents(0.0, &points);
        assert_eq!(p_start, 200.0);

        let p_mid = PitchBendSolver::get_pitch_offset_cents(100.0, &points);
        assert_eq!(p_mid, 200.0);

        let p_after = PitchBendSolver::get_pitch_offset_cents(500.0, &points);
        assert_eq!(p_after, 200.0);
    }

    #[test]
    fn test_pitch_bend_two_points() {
        let points = vec![
            UPitchBendPoint {
                time_offset_ms: 0.0,
                pitch_offset_cents: 200.0,
                shape: "s".to_string(),
            },
            UPitchBendPoint {
                time_offset_ms: 200.0,
                pitch_offset_cents: 0.0,
                shape: "s".to_string(),
            },
        ];

        let p0 = PitchBendSolver::get_pitch_offset_cents(0.0, &points);
        assert_eq!(p0, 200.0);

        let p_mid = PitchBendSolver::get_pitch_offset_cents(100.0, &points);
        assert!(
            (p_mid - 100.0).abs() < 5.0,
            "Expected ~100 cents, got {}",
            p_mid
        );

        let p_end = PitchBendSolver::get_pitch_offset_cents(200.0, &points);
        assert_eq!(p_end, 0.0);

        let p_after = PitchBendSolver::get_pitch_offset_cents(500.0, &points);
        assert_eq!(p_after, 0.0);
    }

    #[test]
    fn test_hermite_cubic_spline_continuity() {
        let points = vec![
            UPitchBendPoint {
                time_offset_ms: 0.0,
                pitch_offset_cents: 0.0,
                shape: "h".to_string(),
            },
            UPitchBendPoint {
                time_offset_ms: 100.0,
                pitch_offset_cents: 100.0,
                shape: "h".to_string(),
            },
            UPitchBendPoint {
                time_offset_ms: 200.0,
                pitch_offset_cents: 200.0,
                shape: "h".to_string(),
            },
        ];

        let p0 = PitchBendSolver::get_pitch_offset_cents(0.0, &points);
        assert_eq!(p0, 0.0);
        let p50 = PitchBendSolver::get_pitch_offset_cents(50.0, &points);
        assert!((p50 - 50.0).abs() < 5.0);
        let p100 = PitchBendSolver::get_pitch_offset_cents(100.0, &points);
        assert_eq!(p100, 100.0);
        let p150 = PitchBendSolver::get_pitch_offset_cents(150.0, &points);
        assert!((p150 - 150.0).abs() < 5.0);
        let p200 = PitchBendSolver::get_pitch_offset_cents(200.0, &points);
        assert_eq!(p200, 200.0);
    }

    #[test]
    fn test_legato_transition() {
        let offset_start = PitchBendSolver::get_legato_transition_offset_cents(-40.0, 60, 62, 80.0);
        assert!((offset_start - (-200.0)).abs() < 1.0);
    }
}
