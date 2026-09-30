use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UtauEnvelope {
    pub p1: f64, // ms delay before attack
    pub p2: f64, // ms attack duration
    pub p3: f64, // ms decay duration
    pub p4: f64, // ms sustain fadeout start from note end
    pub p5: f64, // ms release duration
    pub v1: f64, // level at p1 (0-200%)
    pub v2: f64, // level at p2 (0-200%)
    pub v3: f64, // level at p3 (0-200%)
    pub v4: f64, // level at p4 (0-200%)
    pub v5: f64, // level at p5 (0-200%)
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
    /// Keeps an envelope valid after loading, numeric editing, or a drag.
    ///
    /// OpenUtau stores the five points as ordered X/Y values.  Kamafeu stores
    /// the same shape using UTAU's duration fields (`p1..p5`), so this is the
    /// single place where invalid/NaN values are converted back to a usable
    /// envelope.
    pub fn normalize(&mut self, duration_ms: f64) {
        let duration = if duration_ms.is_finite() {
            duration_ms.max(1.0)
        } else {
            1.0
        };

        let finite = |value: f64, fallback: f64| {
            if value.is_finite() { value } else { fallback }
        };

        self.p1 = finite(self.p1, 0.0).clamp(0.0, duration);
        self.p2 = finite(self.p2, 5.0).clamp(0.0, 500.0);
        self.p3 = finite(self.p3, 35.0).clamp(0.0, 1_500.0);
        self.p4 = finite(self.p4, 0.0).clamp(0.0, 1_500.0);
        self.p5 = finite(self.p5, 35.0).clamp(0.0, 500.0);
        self.crossfade_ms = finite(self.crossfade_ms, 0.0).clamp(0.0, 600.0);

        self.v1 = finite(self.v1, 0.0).clamp(0.0, 200.0);
        self.v2 = finite(self.v2, 100.0).clamp(0.0, 200.0);
        self.v3 = finite(self.v3, 100.0).clamp(0.0, 200.0);
        self.v4 = finite(self.v4, 100.0).clamp(0.0, 200.0);
        self.v5 = finite(self.v5, 0.0).clamp(0.0, 200.0);
    }

    /// Returns the 5 effective points `(time_ms, volume_percent)` relative to note start (0.0 ms).
    pub fn get_effective_points(&self, duration_ms: f64) -> [(f64, f64); 5] {
        let p1_t = self.p1;
        let p2_t = (p1_t + self.p2).max(p1_t);
        let p3_t = (p2_t + self.p3).max(p2_t);
        let p4_t = (duration_ms - self.p4).max(p3_t);
        let p5_t = (p4_t + self.p5).max(p4_t);

        [
            (p1_t, self.v1),
            (p2_t, self.v2),
            (p3_t, self.v3),
            (p4_t, self.v4),
            (p5_t, self.v5),
        ]
    }

    /// Sets point position and volume from interactive dragging, updating internal parameters.
    pub fn set_point(
        &mut self,
        pt_idx: usize,
        time_ms: f64,
        volume_percent: f64,
        duration_ms: f64,
    ) {
        let dur = duration_ms.max(10.0);
        let vol = if volume_percent.is_finite() {
            volume_percent.clamp(0.0, 200.0)
        } else {
            0.0
        };

        match pt_idx {
            0 => {
                // P1: time is p1, volume is v1
                self.p1 = time_ms.clamp(-300.0, dur * 0.5);
                self.v1 = vol;
            }
            1 => {
                // P2: time is p1 + p2, volume is v2
                let p2_t = time_ms.max(self.p1);
                self.p2 = (p2_t - self.p1).clamp(0.0, dur);
                self.v2 = vol;
            }
            2 => {
                // P3: time is p1 + p2 + p3, volume is v3
                let p2_t = self.p1 + self.p2;
                let p3_t = time_ms.max(p2_t);
                self.p3 = (p3_t - p2_t).clamp(0.0, dur * 1.5);
                self.v3 = vol;
            }
            3 => {
                // P4: time is duration - p4, volume is v4
                let p4_t = time_ms.clamp(self.p1 + self.p2, dur + 200.0);
                self.p4 = (dur - p4_t).max(0.0);
                self.v4 = vol;
            }
            4 => {
                // P5: time is duration - p4 + p5, volume is v5
                let p4_t = dur - self.p4;
                let p5_t = time_ms.max(p4_t);
                self.p5 = (p5_t - p4_t).clamp(0.0, 500.0);
                self.v5 = vol;
            }
            _ => {}
        }

        self.normalize(dur);
    }

    /// Resets an individual point to standard defaults
    pub fn reset_point(&mut self, pt_idx: usize) {
        match pt_idx {
            0 => {
                self.p1 = 0.0;
                self.v1 = 0.0;
            }
            1 => {
                self.p2 = 5.0;
                self.v2 = 100.0;
            }
            2 => {
                self.p3 = 35.0;
                self.v3 = 100.0;
            }
            3 => {
                self.p4 = 0.0;
                self.v4 = 100.0;
            }
            4 => {
                self.p5 = 35.0;
                self.v5 = 0.0;
            }
            _ => {}
        }
    }

    /// ACPT: Auto Crossfade preset matching overlap
    pub fn acpt(&mut self, overlap_ms: f64) {
        self.p1 = 0.0;
        self.p2 = overlap_ms.max(5.0);
        self.v1 = 0.0;
        self.v2 = 100.0;
        self.v3 = 100.0;
        self.v4 = 100.0;
        self.v5 = 0.0;
        self.crossfade_ms = 0.0;
    }

    /// P2P3: Snap attack peak to sustain
    pub fn p2p3(&mut self) {
        self.v2 = self.v3;
        self.p2 = self.p2.max(5.0);
    }

    /// P1P4: Zero margins preset
    pub fn p1p4(&mut self) {
        self.p1 = 0.0;
        self.p4 = 0.0;
        self.v1 = 0.0;
        self.v4 = 100.0;
    }

    /// OPT: Optimize envelope durations proportionally to note length
    pub fn opt(&mut self, duration_ms: f64) {
        let dur = duration_ms.max(10.0);
        if dur < 150.0 {
            let scale = dur / 150.0;
            self.p2 = (self.p2 * scale).max(2.0);
            self.p3 = (self.p3 * scale).max(5.0);
            self.p5 = (self.p5 * scale).max(5.0);
        }
    }

    /// RESET: Full envelope reset to UTAU standard (5/35/0/35)
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Generates high-density cosine-interpolated curve points `(t_ms, vol_pct)` for smooth visual rendering.
    pub fn generate_visual_curve(&self, duration_ms: f64, num_subdivisions: usize) -> Vec<(f64, f64)> {
        let pts = self.get_effective_points(duration_ms);
        let mut curve = Vec::new();
        let subs = num_subdivisions.max(8);

        for i in 0..4 {
            let (x0, y0) = pts[i];
            let (x1, y1) = pts[i + 1];
            let dx = x1 - x0;

            for step in 0..subs {
                let frac = step as f64 / subs as f64;
                let t_ms = x0 + dx * frac;
                // Cosine smooth S-curve interpolation: S(t) = 0.5 * (1 - cos(pi * t))
                let smooth_t = 0.5 * (1.0 - (std::f64::consts::PI * frac).cos());
                let vol = y0 + (y1 - y0) * smooth_t;
                curve.push((t_ms, vol));
            }
        }
        curve.push(pts[4]);
        curve
    }

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
    fn test_effective_points_and_visual_curve() {
        let env = UtauEnvelope::default();
        let pts = env.get_effective_points(500.0);
        assert_eq!(pts.len(), 5);
        assert_eq!(pts[0], (0.0, 0.0));
        assert_eq!(pts[1], (5.0, 100.0));
        assert_eq!(pts[4], (535.0, 0.0));

        let curve = env.generate_visual_curve(500.0, 16);
        assert!(!curve.is_empty());
        assert_eq!(curve.first().unwrap().0, 0.0);
        assert_eq!(curve.last().unwrap().0, 535.0);
    }

    #[test]
    fn test_envelope_presets() {
        let mut env = UtauEnvelope::default();
        env.acpt(45.0);
        assert_eq!(env.p2, 45.0);

        env.p1p4();
        assert_eq!(env.p1, 0.0);
        assert_eq!(env.p4, 0.0);

        env.reset();
        assert_eq!(env.p2, 5.0);
        assert_eq!(env.p5, 35.0);
    }

    #[test]
    fn test_set_point_preserves_order_and_supports_openutau_levels() {
        let mut env = UtauEnvelope::default();
        env.set_point(3, 40.0, 175.0, 200.0);
        env.set_point(2, 180.0, 150.0, 200.0);
        env.set_point(1, 20.0, 125.0, 200.0);

        let points = env.get_effective_points(200.0);
        assert!(points.windows(2).all(|pair| pair[0].0 <= pair[1].0));
        assert_eq!(env.v1, 0.0);
        assert_eq!(env.v2, 125.0);
        assert_eq!(env.v3, 150.0);
        assert_eq!(env.v4, 175.0);
        assert!(points.iter().any(|(_, level)| *level > 100.0));
    }

    #[test]
    fn test_normalize_repairs_invalid_values() {
        let mut env = UtauEnvelope {
            p1: f64::NAN,
            p2: f64::INFINITY,
            p3: -10.0,
            p4: f64::NAN,
            p5: 900.0,
            v1: -20.0,
            v2: f64::NAN,
            v3: 250.0,
            v4: f64::INFINITY,
            v5: -1.0,
            crossfade_ms: f64::INFINITY,
        };
        env.normalize(100.0);
        assert_eq!(env.p1, 0.0);
        assert_eq!(env.p2, 5.0);
        assert_eq!(env.p3, 0.0);
        assert_eq!(env.p5, 500.0);
        assert_eq!(env.v1, 0.0);
        assert_eq!(env.v2, 100.0);
        assert_eq!(env.v3, 200.0);
        assert_eq!(env.v4, 100.0);
        assert_eq!(env.v5, 0.0);
        assert_eq!(env.crossfade_ms, 0.0);
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
