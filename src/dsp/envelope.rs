use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UtauEnvelope {
    pub p1: f64, // ms delay before attack
    pub p2: f64, // ms attack duration
    pub p3: f64, // ms decay duration
    pub p4: f64, // ms sustain fadeout start from note end
    pub p5: f64, // ms release duration
    pub v1: f64, // level at p1 (0-100)
    pub v2: f64, // level at p2 (0-100)
    pub v3: f64, // level at p3 (0-100)
    pub v4: f64, // level at p4 (0-100)
    pub v5: f64, // level at p5 (0-100)
    /// Crossfade individual. Zero mantém o overlap automático do oto.ini/global.
    #[serde(default)]
    pub crossfade_ms: f64,
}

impl Default for UtauEnvelope {
    fn default() -> Self {
        Self {
            p1: 0.0,
            p2: 5.0,
            p3: 35.0,
            p4: 0.0,
            p5: 35.0,
            v1: 0.0,
            v2: 100.0,
            v3: 100.0,
            v4: 100.0,
            v5: 0.0,
            crossfade_ms: 0.0,
        }
    }
}

impl UtauEnvelope {
    /// Five-point phoneme envelope following standard UTAU and OpenUtau geometry.
    /// Times are relative to the musical phoneme start (0.0 ms), so preutterance
    /// points lie at negative offsets.
    pub fn phoneme_points(
        &self,
        preutter_ms: f64,
        duration_ms: f64,
        tail_intrude_ms: f64,
        tail_overlap_ms: f64,
        overlap_ms: f64,
        volume: f64,
        attack: f64,
        decay: f64,
    ) -> [(f64, f64); 5] {
        let vol = (volume.clamp(0.0, 200.0) / 100.0) * (self.v3.clamp(0.0, 200.0) / 100.0);
        let v1_level = (self.v1.clamp(0.0, 200.0) / 100.0) * vol;
        let v2_level =
            (self.v2.clamp(0.0, 200.0) / 100.0) * vol * (attack.clamp(0.0, 200.0) / 100.0);
        let v3_level = vol;
        let v4_level =
            (self.v4.clamp(0.0, 200.0) / 100.0) * vol * (1.0 - decay.clamp(0.0, 100.0) / 100.0);
        let v5_level = (self.v5.clamp(0.0, 200.0) / 100.0) * vol;

        let preutter = preutter_ms.max(0.0);
        let p0 = -preutter + self.p1.max(0.0);

        let fade_in = if overlap_ms > 0.0 {
            overlap_ms
        } else {
            self.p2.max(5.0)
        };

        // Determine fade-out end (p4) and fade-out duration
        let is_adjacent = tail_intrude_ms > 0.0 || tail_overlap_ms > 0.0;
        let (p4_target, fade_out) = if is_adjacent {
            let p4_end = duration_ms - tail_intrude_ms + tail_overlap_ms;
            let fo = if tail_overlap_ms > 0.0 {
                tail_overlap_ms
            } else {
                self.p5.max(5.0)
            };
            (p4_end, fo)
        } else {
            let fo = self.p5.max(5.0);
            (duration_ms + fo, fo)
        };

        let len = (p4_target - p0).max(0.0);
        let total_fade = fade_in + fade_out;
        let (eff_fade_in, eff_fade_out) = if total_fade > len && total_fade > 0.0 {
            let r = len / total_fade;
            (fade_in * r, fade_out * r)
        } else {
            (fade_in, fade_out)
        };

        let p1 = p0 + eff_fade_in;
        let p3 = (p4_target - eff_fade_out - self.p4.max(0.0)).max(p1);
        let p2 = if self.p3 > 0.0 {
            (p1 + self.p3).min(p3)
        } else {
            p1
        };
        let p4 = p4_target.max(p3 + 0.1);

        [
            (p0, v1_level),
            (p1, v2_level),
            (p2, v3_level),
            (p3, v4_level),
            (p4, v5_level),
        ]
    }

    pub fn apply_points(
        samples: &mut [f32],
        sample_rate: u32,
        sample_time_zero_ms: f64,
        points: &[(f64, f64); 5],
    ) {
        for (index, sample) in samples.iter_mut().enumerate() {
            let time_ms = sample_time_zero_ms + index as f64 * 1000.0 / sample_rate as f64;
            let gain = Self::gain_at_points(time_ms, points);
            *sample *= gain as f32;
        }
    }

    pub fn gain_at_points(time_ms: f64, points: &[(f64, f64); 5]) -> f64 {
        if time_ms <= points[0].0 {
            points[0].1
        } else if time_ms >= points[4].0 {
            points[4].1
        } else {
            let mut value = points[4].1;
            for pair in points.windows(2) {
                if time_ms <= pair[1].0 {
                    let width = (pair[1].0 - pair[0].0).max(0.001);
                    let t = ((time_ms - pair[0].0) / width).clamp(0.0, 1.0);
                    value = pair[0].1 + (pair[1].1 - pair[0].1) * t;
                    break;
                }
            }
            value
        }
    }

    pub fn gain_at_points_cosine(time_ms: f64, points: &[(f64, f64); 5]) -> f64 {
        if time_ms <= points[0].0 {
            points[0].1
        } else if time_ms >= points[4].0 {
            points[4].1
        } else {
            let mut value = points[4].1;
            for pair in points.windows(2) {
                if time_ms <= pair[1].0 {
                    let width = (pair[1].0 - pair[0].0).max(0.001);
                    let t = ((time_ms - pair[0].0) / width).clamp(0.0, 1.0);
                    let smooth_t = 0.5 * (1.0 - (std::f64::consts::PI * t).cos());
                    value = pair[0].1 + (pair[1].1 - pair[0].1) * smooth_t;
                    break;
                }
            }
            value
        }
    }

    pub fn apply_points_cosine(
        samples: &mut [f32],
        sample_rate: u32,
        sample_time_zero_ms: f64,
        points: &[(f64, f64); 5],
    ) {
        for (index, sample) in samples.iter_mut().enumerate() {
            let time_ms = sample_time_zero_ms + index as f64 * 1000.0 / sample_rate as f64;
            let gain = Self::gain_at_points_cosine(time_ms, points);
            *sample *= gain as f32;
        }
    }

    /// Calculate amplitude multiplier (0.0 to 1.0) at a given point in time (ms) for total note duration (ms).
    pub fn gain_at(&self, time_ms: f64, note_duration_ms: f64) -> f64 {
        if time_ms < 0.0 || time_ms > note_duration_ms + self.p5 {
            return 0.0;
        }

        let t_p1 = self.p1;
        let t_p2 = t_p1 + self.p2;
        let t_p3 = t_p2 + self.p3;

        let release_start = (note_duration_ms - self.p4).max(t_p3);
        let t_p5 = release_start + self.p5;

        let v1 = self.v1 / 100.0;
        let v2 = self.v2 / 100.0;
        let v3 = self.v3 / 100.0;
        let v4 = self.v4 / 100.0;
        let v5 = self.v5 / 100.0;

        if time_ms <= t_p1 {
            if t_p1 == 0.0 {
                v1
            } else {
                v1 * (time_ms / t_p1)
            }
        } else if time_ms <= t_p2 {
            let norm = (time_ms - t_p1) / (t_p2 - t_p1).max(0.001);
            v1 + norm * (v2 - v1)
        } else if time_ms <= t_p3 {
            let norm = (time_ms - t_p2) / (t_p3 - t_p2).max(0.001);
            v2 + norm * (v3 - v2)
        } else if time_ms <= release_start {
            let norm = if release_start > t_p3 {
                (time_ms - t_p3) / (release_start - t_p3)
            } else {
                0.0
            };
            v3 + norm * (v4 - v3)
        } else if time_ms <= t_p5 {
            let norm = (time_ms - release_start) / (t_p5 - release_start).max(0.001);
            v4 + norm * (v5 - v4)
        } else {
            0.0
        }
    }

    /// Apply envelope gain curve to audio sample array
    pub fn apply(&self, samples: &mut [f32], sample_rate: u32, note_duration_ms: f64) {
        for (i, sample) in samples.iter_mut().enumerate() {
            let time_ms = (i as f64 / sample_rate as f64) * 1000.0;
            let gain = self.gain_at(time_ms, note_duration_ms);
            *sample *= gain as f32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope_gain() {
        let env = UtauEnvelope::default();
        let g_start = env.gain_at(0.0, 500.0);
        let g_peak = env.gain_at(5.0, 500.0);
        assert!((g_start - 0.0).abs() < 1e-3);
        assert!((g_peak - 1.0).abs() < 1e-3);
    }

    #[test]
    fn phoneme_envelope_crossfades_and_applies_expression_levels() {
        let env = UtauEnvelope::default();
        let points = env.phoneme_points(80.0, 500.0, 70.0, 30.0, 30.0, 80.0, 50.0, 25.0);
        assert_eq!(points[0], (-80.0, 0.0));
        assert!((points[1].1 - 0.4).abs() < 1e-6);
        assert!((points[3].1 - 0.6).abs() < 1e-6);
        assert_eq!(points[4].0, 460.0);
    }

    #[test]
    fn vcv_envelopes_are_complementary_on_the_absolute_timeline() {
        let env = UtauEnvelope::default();
        // Next phoneme: note at 500 ms, 300 ms preutter and 100 ms overlap.
        // Therefore both fades must occupy absolute time 200..300 ms.
        let previous = env.phoneme_points(0.0, 500.0, 300.0, 100.0, 0.0, 100.0, 100.0, 0.0);
        let current = env.phoneme_points(300.0, 500.0, 0.0, 0.0, 100.0, 100.0, 100.0, 0.0);
        for absolute_ms in [200.0, 225.0, 250.0, 275.0, 300.0] {
            let old_gain = UtauEnvelope::gain_at_points(absolute_ms, &previous);
            let new_gain = UtauEnvelope::gain_at_points(absolute_ms - 500.0, &current);
            assert!(
                (old_gain + new_gain - 1.0).abs() < 1e-6,
                "transition gain at {absolute_ms} ms was {}",
                old_gain + new_gain
            );
        }
    }

    #[test]
    fn short_notes_scale_envelope_gracefully_without_abrupt_drops() {
        let env = UtauEnvelope::default();
        let pts = env.phoneme_points(0.0, 60.0, 100.0, 40.0, 0.0, 100.0, 100.0, 0.0);
        assert!(pts[0].0 <= pts[1].0);
        assert!(pts[1].0 <= pts[2].0);
        assert!(pts[2].0 <= pts[3].0);
        assert!(pts[3].0 <= pts[4].0);
        for step in 0..=50 {
            let t = pts[0].0 + (pts[4].0 - pts[0].0) * step as f64 / 50.0;
            let g = UtauEnvelope::gain_at_points(t, &pts);
            assert!(g.is_finite());
            assert!((0.0..=1.0).contains(&g));
        }
    }

    #[test]
    fn cosine_envelope_is_smooth_and_within_bounds() {
        let env = UtauEnvelope::default();
        let pts = env.phoneme_points(50.0, 500.0, 50.0, 50.0, 50.0, 100.0, 100.0, 0.0);
        let mut samples = vec![1.0f32; 1000];
        UtauEnvelope::apply_points_cosine(&mut samples, 1000, -50.0, &pts);
        assert_eq!(samples[0], 0.0);
        for s in &samples {
            assert!(s.is_finite());
            assert!(*s >= 0.0 && *s <= 1.05);
        }
    }
}
