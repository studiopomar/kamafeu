use super::options::CompatibilityMode;
use crate::phonemizer::consonant_velocity_time_scale;
use serde::{Deserialize, Serialize};

/// Timing resolved for one rendered phoneme.  This mirrors the invariants used
/// by OpenUtau's `UPhoneme.ValidateOverlap`, expressed in milliseconds because
/// Kamafeu's project model is time based.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct PhonemeTiming {
    /// oto.ini preutterance after velocity scaling, before overlap validation.
    /// Resampler pitch data starts here even when the audible lead is clamped.
    pub pitch_leading_ms: f64,
    pub adjacent: bool,
    pub preutter_ms: f64,
    pub overlap_ms: f64,
    pub leading_ms: f64,
    /// Initial rendered audio that must be discarded so the pitch timeline and
    /// validated audible lead remain aligned.
    pub skip_over_ms: f64,
    pub tail_intrude_ms: f64,
    pub tail_overlap_ms: f64,
    /// Velocity-scaled oto.ini values after neighbour validation and before
    /// user deltas. These are the OpenUtau `autoPreutter`/`autoOverlap`.
    pub auto_preutter_ms: f64,
    pub auto_overlap_ms: f64,
}

/// One fully resolved OpenUtau phoneme envelope. The points are in milliseconds
/// relative to the phoneme musical position and gains (0.0..=2.0), matching the
/// values consumed by the native wavtool path and the Piano Roll canvas.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedPhonemeEnvelope {
    pub points: [(f64, f64); 5],
    pub preutter_ms: f64,
    pub overlap_ms: f64,
    pub auto_preutter_ms: f64,
    pub auto_overlap_ms: f64,
    pub velocity: f64,
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
    /// User-authored deltas are applied after automatic oto.ini validation, as in OpenUtau.
    pub preutter_delta_ms: f64,
    pub overlap_delta_ms: f64,
}

/// Stable, serializable inspection record for diagnostics and render
/// provenance. It deliberately contains both source timing and resolved
/// timing, so users can distinguish an oto.ini value from a safety clamp or
/// manual override.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PhonemeTimingDiagnostic {
    pub index: usize,
    pub position_ms: f64,
    pub duration_ms: f64,
    pub oto_preutter_ms: f64,
    pub oto_overlap_ms: f64,
    pub preutter_delta_ms: f64,
    pub overlap_delta_ms: f64,
    pub adjacent: bool,
    pub preutter_ms: f64,
    pub overlap_ms: f64,
    pub auto_preutter_ms: f64,
    pub auto_overlap_ms: f64,
    pub leading_ms: f64,
    pub skip_over_ms: f64,
    pub tail_intrude_ms: f64,
    pub tail_overlap_ms: f64,
}

pub fn diagnostics(
    inputs: &[PhonemeTimingInput],
    timings: &[PhonemeTiming],
) -> Vec<PhonemeTimingDiagnostic> {
    inputs
        .iter()
        .zip(timings.iter())
        .enumerate()
        .map(|(index, (input, timing))| PhonemeTimingDiagnostic {
            index,
            position_ms: input.position_ms,
            duration_ms: input.duration_ms,
            oto_preutter_ms: input.oto_preutter_ms,
            oto_overlap_ms: input.oto_overlap_ms,
            preutter_delta_ms: input.preutter_delta_ms,
            overlap_delta_ms: input.overlap_delta_ms,
            adjacent: timing.adjacent,
            preutter_ms: timing.preutter_ms,
            overlap_ms: timing.overlap_ms,
            auto_preutter_ms: timing.auto_preutter_ms,
            auto_overlap_ms: timing.auto_overlap_ms,
            leading_ms: timing.leading_ms,
            skip_over_ms: timing.skip_over_ms,
            tail_intrude_ms: timing.tail_intrude_ms,
            tail_overlap_ms: timing.tail_overlap_ms,
        })
        .collect()
}

/// A modest handoff keeps automatically expanded phonemes (G2P, CVVC, VCCV,
/// BRAPA and adjacent notes) from becoming a hard cut when an otherwise valid
/// voicebank has no overlap recorded for the aliases. Explicit `oto.ini`
/// overlap and user overrides remain authoritative.
const SAFETY_CROSSFADE_MS: f64 = 12.0;

pub fn plan_inputs(
    phones: &[crate::phonemizer::RenderPhone],
    voicebank: &crate::oto::Voicebank,
    crossfade_ms: f64,
) -> Vec<PhonemeTimingInput> {
    plan_inputs_with_mode(
        phones,
        voicebank,
        crossfade_ms,
        CompatibilityMode::KamafeuExtended,
    )
}

pub fn plan_inputs_with_mode(
    phones: &[crate::phonemizer::RenderPhone],
    voicebank: &crate::oto::Voicebank,
    crossfade_ms: f64,
    compatibility_mode: CompatibilityMode,
) -> Vec<PhonemeTimingInput> {
    phones
        .iter()
        .enumerate()
        .map(|(index, phone)| {
            let oto = phone.find_entry(voicebank);
            let raw_preutter = oto.map(|entry| entry.preutterance).unwrap_or(0.0);
            let raw_overlap = if phone.expressions.presamp_no_crossfade {
                0.0
            } else {
                oto.map(|entry| entry.overlap).unwrap_or(0.0)
            };
            let oto_preutter_ms = raw_preutter.max(0.0);
            let adjacent_handoff = index > 0
                && (phone.position_ms
                    - (phones[index - 1].position_ms + phones[index - 1].duration_ms))
                    .abs()
                    <= 0.001;
            let manual_overlap = if phone.expressions.presamp_no_crossfade
                || compatibility_mode == CompatibilityMode::UtauLegacy
            {
                None
            } else if phone.envelope.crossfade_ms > 0.0 {
                Some(phone.envelope.crossfade_ms)
            } else if raw_overlap.abs() <= f64::EPSILON && adjacent_handoff && crossfade_ms > 0.0 {
                Some(crossfade_ms)
            } else if raw_overlap.abs() <= f64::EPSILON
                && adjacent_handoff
                && phone.expressions.overlap_override_ms.is_none()
                && phone.expressions.overlap_offset_ms.abs() <= f64::EPSILON
            {
                (compatibility_mode == CompatibilityMode::KamafeuExtended)
                    .then_some(SAFETY_CROSSFADE_MS)
            } else {
                None
            };
            let scaled_overlap = raw_overlap
                * crate::phonemizer::consonant_velocity_time_scale(
                    phone.expressions.consonant_velocity,
                );
            let scaled_preutter = oto_preutter_ms
                * crate::phonemizer::consonant_velocity_time_scale(
                    phone.expressions.consonant_velocity,
                );
            let oto_overlap_ms = raw_overlap;
            PhonemeTimingInput {
                position_ms: phone.position_ms,
                duration_ms: phone.duration_ms,
                oto_preutter_ms,
                oto_overlap_ms,
                velocity: phone.expressions.consonant_velocity,
                preutter_delta_ms: phone
                    .expressions
                    .preutter_override_ms
                    .map_or(phone.expressions.preutter_offset_ms, |absolute| {
                        absolute - scaled_preutter + phone.expressions.preutter_offset_ms
                    }),
                overlap_delta_ms: phone
                    .expressions
                    .overlap_override_ms
                    .map_or(phone.expressions.overlap_offset_ms, |absolute| {
                        absolute - scaled_overlap + phone.expressions.overlap_offset_ms
                    })
                    + manual_overlap.map_or(0.0, |value| value - scaled_overlap),
            }
        })
        .collect::<Vec<_>>()
}

pub fn resolve_phoneme_timings(inputs: &[PhonemeTimingInput]) -> Vec<PhonemeTiming> {
    let mut result = vec![PhonemeTiming::default(); inputs.len()];

    for index in 0..inputs.len() {
        let input = inputs[index];
        let stretch = consonant_velocity_time_scale(input.velocity);
        let max_oto_preutter = input.oto_preutter_ms.max(0.0) * stretch;
        let mut auto_preutter = max_oto_preutter;
        let mut auto_overlap = input.oto_overlap_ms * stretch;
        let mut max_preutter = f64::INFINITY;

        if index > 0 {
            let previous = inputs[index - 1];
            let previous_end = previous.position_ms + previous.duration_ms.max(0.0);
            let gap = input.position_ms - previous_end;
            let adjacent = gap <= 0.001;

            if adjacent {
                let previous_duration = previous.duration_ms.max(0.0);
                if auto_overlap > 0.0 {
                    let non_overlap = auto_preutter - auto_overlap;
                    if non_overlap > previous_duration * 0.5 && non_overlap > 0.0 {
                        max_preutter =
                            max_preutter.min(auto_preutter * previous_duration * 0.5 / non_overlap);
                    }
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

        if auto_preutter > max_preutter {
            let ratio = if auto_preutter > 0.0 {
                max_preutter / auto_preutter
            } else {
                1.0
            };
            auto_preutter = max_preutter;
            auto_overlap *= ratio;
        }

        if index > 0 {
            let previous = inputs[index - 1];
            if auto_overlap < 0.0 {
                auto_overlap = auto_overlap
                    .max((35.0 - previous.duration_ms.max(0.0) + auto_preutter).min(0.0));
            }
        }

        let preutter = (auto_preutter + input.preutter_delta_ms).max(0.0);
        // Negative overlap is meaningful in UTAU voicebanks and must survive
        // timing validation. Clamp only the crossfade duration at its consumer.
        let mut overlap = auto_overlap + input.overlap_delta_ms;
        if index > 0 && inputs[index - 1].duration_ms - preutter < 5.0 {
            overlap = overlap.max(5.0 - (inputs[index - 1].duration_ms - preutter));
        }
        result[index].preutter_ms = preutter;
        result[index].overlap_ms = overlap;
        result[index].auto_preutter_ms = auto_preutter;
        result[index].auto_overlap_ms = auto_overlap;
        result[index].pitch_leading_ms = max_oto_preutter;
        // OpenUtau positions the segment at note position minus the complete
        // preutterance. The overlap is the fade duration inside that lead, not
        // an amount to subtract from it.
        result[index].leading_ms = preutter;
        result[index].skip_over_ms = max_oto_preutter - preutter;

        if index > 0 {
            let previous = inputs[index - 1];
            let previous_end = previous.position_ms + previous.duration_ms.max(0.0);
            let adjacent = input.position_ms - previous_end <= 0.001;
            result[index].adjacent = adjacent;
            if adjacent {
                result[index - 1].tail_intrude_ms = if overlap >= 0.0 {
                    preutter
                } else {
                    preutter - overlap
                };
                result[index - 1].tail_overlap_ms = overlap.max(0.0);
            }
        }
    }
    result
}

/// Mirrors OpenUtau `UPhoneme.ValidateEnvelope`. Do not derive the visible
/// envelope from the legacy P1-P5 fields: in OpenUtau mode the five points are
/// a consequence of oto.ini, VEL and the four editable phoneme timing deltas.
pub fn resolve_openutau_envelopes(
    phones: &[crate::phonemizer::RenderPhone],
    timings: &[PhonemeTiming],
) -> Vec<ResolvedPhonemeEnvelope> {
    phones
        .iter()
        .zip(timings.iter())
        .map(|(phone, timing)| {
            let volume = phone.expressions.volume.clamp(0.0, 200.0) / 100.0;
            let attack = phone.expressions.attack.clamp(0.0, 200.0) / 100.0;
            let decay = phone.expressions.decay.clamp(0.0, 100.0) / 100.0;
            let fade_in = if timing.adjacent && timing.overlap_ms > 0.0 {
                timing.overlap_ms
            } else {
                5.0
            };
            let fade_out = if timing.tail_overlap_ms > 0.0 {
                timing.tail_overlap_ms
            } else {
                35.0
            };
            let p0_x = -timing.preutter_ms;
            let p1_x = (p0_x + 5.0).max(p0_x + fade_in + phone.expressions.attack_time_delta_ms);
            let p2_x = p1_x.max(0.0);
            let p4_x = phone.duration_ms.max(0.0) - timing.tail_intrude_ms + timing.tail_overlap_ms;
            let p3_x = p2_x.max(p4_x - fade_out - phone.expressions.release_time_delta_ms);
            ResolvedPhonemeEnvelope {
                points: [
                    (p0_x, 0.0),
                    (p1_x, volume * attack),
                    (p2_x, volume),
                    (p3_x, volume * (1.0 - decay)),
                    (p4_x.max(p3_x), 0.0),
                ],
                preutter_ms: timing.preutter_ms,
                overlap_ms: timing.overlap_ms,
                auto_preutter_ms: timing.auto_preutter_ms,
                auto_overlap_ms: timing.auto_overlap_ms,
                velocity: phone.expressions.consonant_velocity,
                tail_intrude_ms: timing.tail_intrude_ms,
                tail_overlap_ms: timing.tail_overlap_ms,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn phone(position_ms: f64, duration_ms: f64, velocity: f64) -> crate::phonemizer::RenderPhone {
        let expressions = crate::project::model::UExpressions {
            consonant_velocity: velocity,
            ..Default::default()
        };
        crate::phonemizer::RenderPhone {
            note_index: 0,
            lyric: "a".to_string(),
            pitch: "C4".to_string(),
            position_ms,
            duration_ms,
            envelope: crate::dsp::envelope::UtauEnvelope::default(),
            expressions,
            pitch_bend: crate::project::model::UPitchBend::default(),
            vibrato: crate::dsp::pitch::VibratoParam::default(),
            flags: String::new(),
            resolved_alias: None,
            resolved_timbre: None,
        }
    }

    #[test]
    fn openutau_envelope_moves_preutter_with_velocity() {
        let slow = phone(0.0, 500.0, 50.0);
        let mut fast = slow.clone();
        fast.expressions.consonant_velocity = 200.0;
        let input = |velocity| PhonemeTimingInput {
            position_ms: 0.0,
            duration_ms: 500.0,
            oto_preutter_ms: 100.0,
            oto_overlap_ms: 40.0,
            velocity,
            preutter_delta_ms: 0.0,
            overlap_delta_ms: 0.0,
        };
        let slow_timing = resolve_phoneme_timings(&[input(50.0)]);
        let fast_timing = resolve_phoneme_timings(&[input(200.0)]);
        let slow_envelope = resolve_openutau_envelopes(&[slow], &slow_timing);
        let fast_envelope = resolve_openutau_envelopes(&[fast], &fast_timing);
        assert!((slow_envelope[0].points[0].0 + 141.421356).abs() < 1e-3);
        assert!((fast_envelope[0].points[0].0 + 50.0).abs() < 1e-6);
        assert!(slow_envelope[0].points[1].0 < 0.0);
        assert!(fast_envelope[0].points[1].0 < 0.0);
    }

    #[test]
    fn velocity_stretches_oto_timing() {
        let timing = resolve_phoneme_timings(&[PhonemeTimingInput {
            position_ms: 0.0,
            duration_ms: 500.0,
            oto_preutter_ms: 100.0,
            oto_overlap_ms: 40.0,
            velocity: 200.0,
            preutter_delta_ms: 0.0,
            overlap_delta_ms: 0.0,
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
                preutter_delta_ms: 0.0,
                overlap_delta_ms: 0.0,
            },
            PhonemeTimingInput {
                position_ms: 80.0,
                duration_ms: 400.0,
                oto_preutter_ms: 120.0,
                oto_overlap_ms: 20.0,
                velocity: 100.0,
                preutter_delta_ms: 0.0,
                overlap_delta_ms: 0.0,
            },
        ]);
        assert!(timing[1].preutter_ms - timing[1].overlap_ms <= 40.0 + 1e-6);
        assert!((timing[1].overlap_ms / timing[1].preutter_ms - 1.0 / 6.0).abs() < 1e-6);
        assert!(timing[0].tail_intrude_ms > 0.0);
        assert!(timing[1].pitch_leading_ms > timing[1].leading_ms);
        assert!(
            (timing[1].skip_over_ms - (timing[1].pitch_leading_ms - timing[1].leading_ms)).abs()
                < 1e-6
        );
    }

    #[test]
    fn manual_deltas_are_not_scaled_by_velocity() {
        let timing = resolve_phoneme_timings(&[PhonemeTimingInput {
            position_ms: 0.0,
            duration_ms: 500.0,
            oto_preutter_ms: 100.0,
            oto_overlap_ms: 40.0,
            velocity: 200.0,
            preutter_delta_ms: 20.0,
            overlap_delta_ms: 10.0,
        }]);
        assert!((timing[0].preutter_ms - 70.0).abs() < 1e-6);
        assert!((timing[0].overlap_ms - 30.0).abs() < 1e-6);
    }

    #[test]
    fn calibrated_oto_overlap_is_preserved_when_global_crossfade_is_present() {
        let dir = tempfile::tempdir().unwrap();
        let samples = vec![0.0f32; 44100];
        crate::renderer::TrackRenderer::save_wav_samples(
            dir.path().join("sample.wav"),
            &samples,
            44100,
        )
        .unwrap();
        std::fs::write(
            dir.path().join("oto.ini"),
            "sample.wav=i na,0,100,-600,280,100\nsample.wav=ka,0,100,-600,50,0\n",
        )
        .unwrap();
        let vb = crate::oto::Voicebank::new(dir.path()).unwrap();

        let vcv_phone = crate::phonemizer::RenderPhone {
            note_index: 0,
            lyric: "i na".to_string(),
            pitch: "C4".to_string(),
            position_ms: 500.0,
            duration_ms: 500.0,
            envelope: crate::dsp::envelope::UtauEnvelope::default(),
            expressions: crate::project::model::UExpressions::default(),
            pitch_bend: crate::project::model::UPitchBend::default(),
            vibrato: crate::dsp::pitch::VibratoParam::default(),
            flags: String::new(),
            resolved_alias: None,
            resolved_timbre: None,
        };

        // Even when global crossfade_ms is 15.0, the calibrated 100ms overlap must NOT be overwritten.
        let inputs = plan_inputs(&[vcv_phone], &vb, 15.0);
        assert_eq!(inputs[0].oto_overlap_ms, 100.0);
        assert!((inputs[0].overlap_delta_ms).abs() < 1e-6);

        // But for an entry with 0 overlap, fallback to the global crossfade.
        let cv_phone = crate::phonemizer::RenderPhone {
            note_index: 0,
            lyric: "ka".to_string(),
            pitch: "C4".to_string(),
            position_ms: 1000.0,
            duration_ms: 500.0,
            envelope: crate::dsp::envelope::UtauEnvelope::default(),
            expressions: crate::project::model::UExpressions::default(),
            pitch_bend: crate::project::model::UPitchBend::default(),
            vibrato: crate::dsp::pitch::VibratoParam::default(),
            flags: String::new(),
            resolved_alias: None,
            resolved_timbre: None,
        };
        let cv_inputs = plan_inputs(
            &[
                crate::phonemizer::RenderPhone {
                    position_ms: 0.0,
                    duration_ms: 500.0,
                    ..cv_phone.clone()
                },
                crate::phonemizer::RenderPhone {
                    position_ms: 500.0,
                    ..cv_phone
                },
            ],
            &vb,
            15.0,
        );
        assert!((cv_inputs[1].overlap_delta_ms - 15.0).abs() < 1e-6);
    }

    #[test]
    fn adjacent_notes_without_oto_overlap_receive_a_safety_crossfade() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("oto.ini"),
            "sample.wav=a,0,100,-600,40,0\nsample.wav=ka,0,100,-600,50,0\n",
        )
        .unwrap();
        let vb = crate::oto::Voicebank::new(dir.path()).unwrap();
        let first = crate::phonemizer::RenderPhone {
            note_index: 0,
            lyric: "a".into(),
            pitch: "C4".into(),
            position_ms: 0.0,
            duration_ms: 400.0,
            envelope: crate::dsp::envelope::UtauEnvelope::default(),
            expressions: crate::project::model::UExpressions::default(),
            pitch_bend: crate::project::model::UPitchBend::default(),
            vibrato: crate::dsp::pitch::VibratoParam::default(),
            flags: String::new(),
            resolved_alias: None,
            resolved_timbre: None,
        };
        let mut second = first.clone();
        second.note_index = 1;
        second.lyric = "ka".into();
        second.position_ms = 400.0;
        let inputs = plan_inputs(&[first, second], &vb, 42.0);

        assert_eq!(inputs[1].overlap_delta_ms, 42.0);
    }

    #[test]
    fn internal_phonemes_without_oto_overlap_receive_a_smooth_handoff() {
        let phones = vec![
            crate::phonemizer::RenderPhone {
                note_index: 0,
                lyric: "a".to_string(),
                pitch: "C4".to_string(),
                position_ms: 0.0,
                duration_ms: 150.0,
                envelope: crate::dsp::envelope::UtauEnvelope::default(),
                expressions: crate::project::model::UExpressions::default(),
                pitch_bend: crate::project::model::UPitchBend::default(),
                vibrato: crate::dsp::pitch::VibratoParam::default(),
                flags: String::new(),
                resolved_alias: None,
                resolved_timbre: None,
            },
            crate::phonemizer::RenderPhone {
                note_index: 0,
                lyric: "k a".to_string(),
                pitch: "C4".to_string(),
                position_ms: 150.0,
                duration_ms: 150.0,
                envelope: crate::dsp::envelope::UtauEnvelope::default(),
                expressions: crate::project::model::UExpressions::default(),
                pitch_bend: crate::project::model::UPitchBend::default(),
                vibrato: crate::dsp::pitch::VibratoParam::default(),
                flags: String::new(),
                resolved_alias: None,
                resolved_timbre: None,
            },
        ];
        let dir = tempfile::tempdir().unwrap();
        let vb = crate::oto::Voicebank::new(dir.path()).unwrap();
        let timings = resolve_phoneme_timings(&plan_inputs(&phones, &vb, 0.0));

        assert_eq!(timings[1].overlap_ms, SAFETY_CROSSFADE_MS);
        assert_eq!(timings[0].tail_overlap_ms, SAFETY_CROSSFADE_MS);
    }

    #[test]
    fn safety_handoff_never_replaces_a_recorded_oto_overlap() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("oto.ini"),
            "sample.wav=a,0,100,-600,40,0\nsample.wav=k a,0,100,-600,50,80\n",
        )
        .unwrap();
        let vb = crate::oto::Voicebank::new(dir.path()).unwrap();
        let mut phones = Vec::new();
        for (lyric, position_ms) in [("a", 0.0), ("k a", 150.0)] {
            phones.push(crate::phonemizer::RenderPhone {
                note_index: 0,
                lyric: lyric.to_string(),
                pitch: "C4".to_string(),
                position_ms,
                duration_ms: 150.0,
                envelope: crate::dsp::envelope::UtauEnvelope::default(),
                expressions: crate::project::model::UExpressions::default(),
                pitch_bend: crate::project::model::UPitchBend::default(),
                vibrato: crate::dsp::pitch::VibratoParam::default(),
                flags: String::new(),
                resolved_alias: None,
                resolved_timbre: None,
            });
        }

        let timings = resolve_phoneme_timings(&plan_inputs(&phones, &vb, 0.0));
        assert_eq!(timings[1].overlap_ms, 80.0);
    }

    #[test]
    fn adjacent_tail_intrude_preserves_negative_overlap_geometry() {
        let timing = resolve_phoneme_timings(&[
            PhonemeTimingInput {
                position_ms: 0.0,
                duration_ms: 500.0,
                oto_preutter_ms: 0.0,
                oto_overlap_ms: 0.0,
                velocity: 100.0,
                preutter_delta_ms: 0.0,
                overlap_delta_ms: 0.0,
            },
            PhonemeTimingInput {
                position_ms: 500.0,
                duration_ms: 500.0,
                oto_preutter_ms: 100.0,
                oto_overlap_ms: 40.0,
                velocity: 100.0,
                preutter_delta_ms: 0.0,
                overlap_delta_ms: -200.0,
            },
        ]);
        assert_eq!(timing[1].overlap_ms, -160.0);
        assert_eq!(timing[0].tail_intrude_ms, 260.0);
        assert_eq!(timing[0].tail_overlap_ms, 0.0);
    }

    #[test]
    fn diagnostics_pair_source_and_resolved_geometry_deterministically() {
        let inputs = vec![
            PhonemeTimingInput {
                position_ms: 0.0,
                duration_ms: 100.0,
                oto_preutter_ms: 10.0,
                oto_overlap_ms: 2.0,
                velocity: 100.0,
                preutter_delta_ms: 3.0,
                overlap_delta_ms: -1.0,
            },
            PhonemeTimingInput {
                position_ms: 100.0,
                duration_ms: 200.0,
                oto_preutter_ms: 20.0,
                oto_overlap_ms: 5.0,
                velocity: 100.0,
                preutter_delta_ms: 0.0,
                overlap_delta_ms: 0.0,
            },
        ];
        let timings = resolve_phoneme_timings(&inputs);
        let report = diagnostics(&inputs, &timings);
        assert_eq!(report.len(), 2);
        assert_eq!(report[1].index, 1);
        assert_eq!(report[1].oto_overlap_ms, 5.0);
        assert!(report[1].adjacent);
        assert_eq!(report[0].tail_intrude_ms, report[1].preutter_ms);
        assert_eq!(report[0].tail_overlap_ms, report[1].overlap_ms.max(0.0));
        let json = serde_json::to_value(&report).expect("timing diagnostics JSON");
        assert_eq!(json[1]["oto_overlap_ms"], serde_json::json!(5.0));
        assert_eq!(json[1]["adjacent"], serde_json::json!(true));
        assert!(json[0].get("tail_intrude_ms").is_some());
    }
}
