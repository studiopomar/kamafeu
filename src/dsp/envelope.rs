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
            if t_p1 == 0.0 { v1 } else { v1 * (time_ms / t_p1) }
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
}
