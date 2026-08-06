use crate::phonemizer::consonant_velocity_time_scale;

/// Timing resolved for one rendered phoneme.  This mirrors the invariants used
/// by OpenUtau's `UPhoneme.ValidateOverlap`, expressed in milliseconds because
/// Kamafeu's project model is time based.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PhonemeTiming {
    /// OTO preutterance after velocity scaling, before overlap validation.
    /// Resampler pitch data starts here even when the audible lead is clamped.
    pub pitch_leading_ms: f64,
    pub preutter_ms: f64,
    pub overlap_ms: f64,
    pub leading_ms: f64,
    /// Initial rendered audio that must be discarded so the pitch timeline and
    /// validated audible lead remain aligned.
    pub skip_over_ms: f64,
    pub tail_intrude_ms: f64,
    pub tail_overlap_ms: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct PhonemeTimingInput {
    pub position_ms: f64,
    pub duration_ms: f64,
    pub oto_preutter_ms: f64,
    pub oto_overlap_ms: f64,
    pub velocity: f64,
}

pub fn resolve_phoneme_timings(inputs: &[PhonemeTimingInput]) -> Vec<PhonemeTiming> {
    let mut result = vec![PhonemeTiming::default(); inputs.len()];

    for index in 0..inputs.len() {
        let input = inputs[index];
        let stretch = consonant_velocity_time_scale(input.velocity);
        let auto_preutter = input.oto_preutter_ms.max(0.0) * stretch;
        let mut auto_overlap = input.oto_overlap_ms * stretch;
        let mut max_preutter = f64::INFINITY;

        if index > 0 {
            let previous = inputs[index - 1];
            let previous_end = previous.position_ms + previous.duration_ms.max(0.0);
            let gap = input.position_ms - previous_end;
            let adjacent = gap <= 1.0;

            if adjacent {
                let previous_duration = previous.duration_ms.max(0.0);
                if auto_overlap > 0.0 {
                    let non_overlap = auto_preutter - auto_overlap;
                    if non_overlap > previous_duration * 0.5 && non_overlap > 0.0 {
                        max_preutter =
                            max_preutter.min(auto_preutter * previous_duration * 0.5 / non_overlap);
                    }
                } else {
                    max_preutter = max_preutter.min(previous_duration * 0.9);
                }
                max_preutter = max_preutter.min(previous_duration);
                if result[index - 1].preutter_ms < 5.0 {
                    max_preutter = max_preutter
                        .min((previous_duration + result[index - 1].preutter_ms - 5.0).max(0.0));
                }
            } else if gap > 0.0 && gap < auto_preutter {
                max_preutter = max_preutter.min(gap);
            }
        }

        let preutter = auto_preutter.min(max_preutter).max(0.0);
        if auto_preutter > 0.0 && preutter < auto_preutter {
            auto_overlap *= preutter / auto_preutter;
        }

        if index > 0 {
            let previous = inputs[index - 1];
            let previous_end = previous.position_ms + previous.duration_ms.max(0.0);
            let adjacent = input.position_ms - previous_end <= 1.0;
            if adjacent && auto_overlap < 0.0 {
                auto_overlap =
                    auto_overlap.max((35.0 - previous.duration_ms.max(0.0) + preutter).min(0.0));
            }
        }

        result[index].preutter_ms = preutter;
        result[index].overlap_ms = auto_overlap;
        result[index].pitch_leading_ms = auto_preutter;
        // OpenUtau positions the segment at note position minus the complete
        // preutterance. The overlap is the fade duration inside that lead, not
        // an amount to subtract from it.
        result[index].leading_ms = preutter;
        result[index].skip_over_ms = (auto_preutter - preutter).max(0.0);

        if index > 0 {
            let previous = inputs[index - 1];
            let previous_end = previous.position_ms + previous.duration_ms.max(0.0);
            if input.position_ms - previous_end <= 1.0 {
                result[index - 1].tail_intrude_ms = preutter.max(preutter - auto_overlap);
                result[index - 1].tail_overlap_ms = auto_overlap.max(0.0);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn velocity_stretches_oto_timing() {
        let timing = resolve_phoneme_timings(&[PhonemeTimingInput {
            position_ms: 0.0,
            duration_ms: 500.0,
            oto_preutter_ms: 100.0,
            oto_overlap_ms: 40.0,
            velocity: 200.0,
        }]);
        assert!((timing[0].preutter_ms - 50.0).abs() < 1e-6);
        assert!((timing[0].overlap_ms - 20.0).abs() < 1e-6);
        assert!((timing[0].pitch_leading_ms - 50.0).abs() < 1e-6);
        assert!(timing[0].skip_over_ms.abs() < 1e-6);
    }

    #[test]
    fn adjacent_short_note_clamps_preutter_and_preserves_ratio() {
        let timing = resolve_phoneme_timings(&[
            PhonemeTimingInput {
                position_ms: 0.0,
                duration_ms: 80.0,
                oto_preutter_ms: 0.0,
                oto_overlap_ms: 0.0,
                velocity: 100.0,
            },
            PhonemeTimingInput {
                position_ms: 80.0,
                duration_ms: 400.0,
                oto_preutter_ms: 120.0,
                oto_overlap_ms: 20.0,
                velocity: 100.0,
            },
        ]);
        assert!(timing[1].preutter_ms - timing[1].overlap_ms <= 40.0 + 1e-6);
        assert!((timing[1].overlap_ms / timing[1].preutter_ms - 1.0 / 6.0).abs() < 1e-6);
        assert!(timing[0].tail_intrude_ms > 0.0);
        assert!(timing[1].pitch_leading_ms > timing[1].leading_ms);
        assert!((timing[1].skip_over_ms
            - (timing[1].pitch_leading_ms - timing[1].leading_ms))
            .abs()
            < 1e-6);
    }
}
