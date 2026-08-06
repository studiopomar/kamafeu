use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
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
        }
    }
}

impl UtauEnvelope {
    /// Five-point phoneme envelope used by OpenUtau's classic renderer. Times
    /// are relative to the musical phoneme start, so the first point may be in
    /// the preutterance region.
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
        let volume = volume.clamp(0.0, 200.0) / 100.0;
        let attack = attack.clamp(0.0, 200.0) / 100.0;
        let decay = decay.clamp(0.0, 100.0) / 100.0;
        let p0 = -preutter_ms.max(0.0);
        let fade_in = if overlap_ms > 0.0 {
            overlap_ms
        } else {
            self.p2.max(5.0)
        };
        let p1 = (p0 + fade_in).max(p0 + 5.0);
        let p2 = 0.0f64.max(p1);
        let p4 = duration_ms - tail_intrude_ms + tail_overlap_ms;
        let fade_out = if tail_overlap_ms > 0.0 {
            tail_overlap_ms
        } else {
            self.p5.max(35.0)
        };
        let p3 = p2.max(p4 - fade_out);
        [
            (p0, 0.0),
            (p1, attack * volume * self.v2.clamp(0.0, 200.0) / 100.0),
            (p2, volume * self.v3.clamp(0.0, 200.0) / 100.0),
            (
                p3,
                volume * (1.0 - decay) * self.v4.clamp(0.0, 200.0) / 100.0,
            ),
            (p4.max(p3), 0.0),
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
        if time_ms <= points[0].0 || time_ms >= points[4].0 {
            0.0
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
}
