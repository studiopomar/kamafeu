use crate::phonemizer::consonant_velocity_time_scale;

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

/// A modest handoff keeps automatically expanded phonemes (G2P, CVVC, VCCV
/// and BRAPA clusters) from becoming a sequence of hard cuts when an otherwise
/// valid voicebank has no overlap recorded for those internal aliases.  It is
/// deliberately only a fallback for phones generated *inside the same musical
/// note*: inter-note `oto.ini` geometry remains completely authoritative.
const INTRA_NOTE_SAFETY_CROSSFADE_MS: f64 = 12.0;

pub fn plan_inputs(
    phones: &[crate::phonemizer::RenderPhone],
    voicebank: &crate::oto::Voicebank,
    crossfade_ms: f64,
) -> Vec<PhonemeTimingInput> {
    phones
        .iter()
        .enumerate()
        .map(|(index, phone)| {
            let oto = voicebank.find_mapped_entry(&phone.lyric, &phone.pitch);
            let raw_preutter = oto.map(|entry| entry.preutterance).unwrap_or(0.0);
            let raw_overlap = oto.map(|entry| entry.overlap).unwrap_or(0.0);
            let oto_preutter_ms = raw_preutter.max(0.0);
            let same_note_handoff = index > 0
                && phones[index - 1].note_index == phone.note_index
                && (phone.position_ms
                    - (phones[index - 1].position_ms + phones[index - 1].duration_ms))
                    .abs()
                    <= 0.001;
            let manual_overlap = if phone.envelope.crossfade_ms > 0.0 {
                Some(phone.envelope.crossfade_ms)
            } else if raw_overlap.abs() <= f64::EPSILON && crossfade_ms > 0.0 {
                Some(crossfade_ms)
            } else if raw_overlap.abs() <= f64::EPSILON
                && same_note_handoff
                && phone.expressions.overlap_override_ms.is_none()
                && phone.expressions.overlap_offset_ms.abs() <= f64::EPSILON
            {
                Some(INTRA_NOTE_SAFETY_CROSSFADE_MS)
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
        };

        // Even when global crossfade_ms is 15.0, the calibrated 100ms overlap must NOT be overwritten.
        let inputs = plan_inputs(&[vcv_phone], &vb, 15.0);
        assert_eq!(inputs[0].oto_overlap_ms, 100.0);
        assert!((inputs[0].overlap_delta_ms).abs() < 1e-6);

        // But for an entry with 0 overlap, fallback to the global crossfade.
        let cv_phone = crate::phonemizer::RenderPhone {
            note_index: 1,
            lyric: "ka".to_string(),
            pitch: "C4".to_string(),
            position_ms: 1000.0,
            duration_ms: 500.0,
            envelope: crate::dsp::envelope::UtauEnvelope::default(),
            expressions: crate::project::model::UExpressions::default(),
            pitch_bend: crate::project::model::UPitchBend::default(),
            vibrato: crate::dsp::pitch::VibratoParam::default(),
            flags: String::new(),
        };
        let cv_inputs = plan_inputs(&[cv_phone], &vb, 15.0);
        assert!((cv_inputs[0].overlap_delta_ms - 15.0).abs() < 1e-6);
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
            },
        ];
        let dir = tempfile::tempdir().unwrap();
        let vb = crate::oto::Voicebank::new(dir.path()).unwrap();
        let timings = resolve_phoneme_timings(&plan_inputs(&phones, &vb, 0.0));

        assert_eq!(timings[1].overlap_ms, INTRA_NOTE_SAFETY_CROSSFADE_MS);
        assert_eq!(timings[0].tail_overlap_ms, INTRA_NOTE_SAFETY_CROSSFADE_MS);
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
}
