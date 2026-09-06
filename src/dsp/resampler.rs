use crate::dsp::pitch_bend::PitchBendSolver;
use crate::project::model::UPitchBendPoint;

pub struct Resampler;

impl Resampler {
    pub fn estimate_pitch_period(slice: &[f32], sample_rate: u32) -> usize {
        let min_period = (sample_rate as f64 / 1000.0) as usize; // Max ~1000 Hz
        let max_period = (sample_rate as f64 / 50.0) as usize; // Min ~50 Hz

        let mut best_period = min_period;
        let mut max_corr = -1.0f32;

        for period in min_period..=max_period.min(slice.len() / 2) {
            let mut corr = 0.0f32;
            let mut energy = 0.0f32;

            for i in 0..(slice.len() - period) {
                corr += slice[i] * slice[i + period];
                energy += slice[i] * slice[i];
            }

            if energy > 1e-6 {
                let norm_corr = corr / energy;
                if norm_corr > max_corr {
                    max_corr = norm_corr;
                    best_period = period;
                }
            }
        }

        best_period
    }

    pub fn render_sample(
        input_samples: &[f32],
        sample_rate: u32,
        offset_ms: f64,
        consonant_ms: f64,
        cutoff_ms: f64,
        target_duration_ms: f64,
        target_pitch_freq: f64,
    ) -> Vec<f32> {
        Self::render_sample_with_pitch_bend(
            input_samples,
            sample_rate,
            offset_ms,
            consonant_ms,
            cutoff_ms,
            target_duration_ms,
            target_pitch_freq,
            &[],
        )
    }

    pub fn render_sample_with_pitch_bend(
        input_samples: &[f32],
        sample_rate: u32,
        offset_ms: f64,
        consonant_ms: f64,
        cutoff_ms: f64,
        target_duration_ms: f64,
        target_pitch_freq: f64,
        pitch_points: &[UPitchBendPoint],
    ) -> Vec<f32> {
        Self::render_sample_with_pitch_bend_and_consonant_timing(
            input_samples,
            sample_rate,
            offset_ms,
            consonant_ms,
            consonant_ms,
            cutoff_ms,
            target_duration_ms,
            target_pitch_freq,
            pitch_points,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_sample_with_pitch_bend_and_consonant_timing(
        input_samples: &[f32],
        sample_rate: u32,
        offset_ms: f64,
        source_consonant_ms: f64,
        target_consonant_ms: f64,
        cutoff_ms: f64,
        target_duration_ms: f64,
        target_pitch_freq: f64,
        pitch_points: &[UPitchBendPoint],
    ) -> Vec<f32> {
        let (start_sample, end_sample) =
            crate::dsp::oto_source_bounds(input_samples.len(), sample_rate, offset_ms, cutoff_ms);

        if start_sample >= end_sample || start_sample >= input_samples.len() {
            let target_sample_count = ((target_duration_ms / 1000.0) * sample_rate as f64) as usize;
            return vec![0.0; target_sample_count];
        }

        let slice = &input_samples[start_sample..end_sample];

        let source_consonant_samples = ((source_consonant_ms.max(0.0) / 1000.0)
            * sample_rate as f64)
            .clamp(0.0, slice.len() as f64) as usize;

        let consonant_slice = &slice[..source_consonant_samples];
        let vowel_slice = &slice[source_consonant_samples..];

        let target_total_samples = ((target_duration_ms / 1000.0) * sample_rate as f64) as usize;
        let target_consonant_samples =
            (((target_consonant_ms.max(0.0) / 1000.0) * sample_rate as f64).round() as usize)
                .min(target_total_samples);
        let target_vowel_samples = target_total_samples.saturating_sub(target_consonant_samples);

        let mut output = Vec::with_capacity(target_total_samples);

        if target_consonant_samples > 0 && !consonant_slice.is_empty() {
            output.extend(crate::dsp::resize_preserving_pitch(
                consonant_slice,
                target_consonant_samples,
                sample_rate,
            ));
        } else if target_consonant_samples > 0 {
            output.resize(target_consonant_samples, 0.0);
        }

        if target_vowel_samples > 0 && !vowel_slice.is_empty() {
            let pyin_res =
                crate::dsp::pyin::PitchExtractor::extract_pitch_and_gci(vowel_slice, sample_rate);

            let fallback_t0 = Self::estimate_pitch_period(vowel_slice, sample_rate);
            let mut synth_residual = vec![0.0f32; target_vowel_samples];
            let mut synth_weights = vec![0.0f32; target_vowel_samples];

            let vowel_len = vowel_slice.len();
            let loop_start = vowel_len / 4;
            let loop_end = (vowel_len * 3 / 4).max(loop_start + fallback_t0);
            let loop_len = (loop_end.saturating_sub(loop_start)).max(fallback_t0);

            let mut out_pos = 0usize;
            let mut virt_in_pos = 0f64;

            while out_pos < target_vowel_samples {
                let rel_t_ms = target_consonant_ms + (out_pos as f64 / sample_rate as f64) * 1000.0;
                let pitch_cents = PitchBendSolver::get_pitch_offset_cents(rel_t_ms, pitch_points);
                let frame_freq = (target_pitch_freq * 2.0f64.powf(pitch_cents / 1200.0)).max(20.0);

                let dst_t0 = (sample_rate as f64 / frame_freq).round() as usize;
                let dst_t0 = dst_t0.clamp(16, sample_rate as usize / 4);

                let in_pos_int = virt_in_pos as usize;
                let src_center = if in_pos_int >= loop_end && loop_len > 0 {
                    loop_start + ((in_pos_int - loop_start) % loop_len)
                } else {
                    in_pos_int % vowel_len
                };

                let win_size_max = (2 * fallback_t0).max(64);
                let check_start = src_center.saturating_sub(win_size_max / 2);
                let check_end = (src_center + win_size_max / 2).min(vowel_len);

                let mut zero_crossings = 0;
                let mut energy = 0.0;
                for i in check_start + 1..check_end {
                    if (vowel_slice[i - 1] > 0.0 && vowel_slice[i] <= 0.0)
                        || (vowel_slice[i - 1] < 0.0 && vowel_slice[i] >= 0.0)
                    {
                        zero_crossings += 1;
                    }
                    energy += vowel_slice[i] * vowel_slice[i];
                }

                let check_len = (check_end - check_start).max(1);
                let zcr = zero_crossings as f32 / check_len as f32;

                let is_unvoiced = zcr > 0.25 || (energy < 1e-4 && zcr > 0.1);

                let (src_start, src_end, win_size, src_advance) = if is_unvoiced {
                    let w_size = (sample_rate as f64 * 0.01).round() as usize; // 10ms window
                    let s_start = src_center.saturating_sub(w_size / 2);
                    let s_end = (s_start + w_size).min(vowel_len);
                    (s_start, s_end, w_size, dst_t0 as f64)
                } else {
                    let mut best_gci = src_center;
                    let mut src_t0 = fallback_t0;

                    if !pyin_res.gci_marks.is_empty() {
                        let mut min_dist = usize::MAX;
                        for (i, &gci) in pyin_res.gci_marks.iter().enumerate() {
                            let dist = (gci as isize - src_center as isize).unsigned_abs();
                            if dist < min_dist {
                                min_dist = dist;
                                best_gci = gci;
                                if i < pyin_res.pitch_contour.len() {
                                    src_t0 = pyin_res.pitch_contour[i] as usize;
                                }
                            }
                        }
                    }

                    let w_size = (2 * src_t0).max(64);
                    let s_start = best_gci.saturating_sub(w_size / 2);
                    let s_end = (s_start + w_size).min(vowel_len);
                    (s_start, s_end, w_size, src_t0 as f64)
                };

                let actual_win = (src_end - src_start).min(target_vowel_samples - out_pos);
                let w_step = 2.0 * std::f32::consts::PI / win_size as f32;

                let out_slice_res = &mut synth_residual[out_pos..out_pos + actual_win];
                let out_slice_wei = &mut synth_weights[out_pos..out_pos + actual_win];

                if is_unvoiced {
                    let src_slice = &vowel_slice[src_start..src_start + actual_win];
                    for i in 0..actual_win {
                        let w = 0.5 * (1.0 - (w_step * i as f32).cos());
                        out_slice_res[i] += src_slice[i] * w;
                        out_slice_wei[i] += w;
                    }
                } else {
                    // PSOLA already preserves the spectral envelope by moving
                    // complete waveform grains. Reconstructing an LPC residual
                    // here amplified high-frequency errors whenever the target
                    // period changed, producing the characteristic "chipmunk"
                    // voice on even very small bends.
                    let src_slice = &vowel_slice[src_start..src_start + actual_win];
                    for i in 0..actual_win {
                        let w = 0.5 * (1.0 - (w_step * i as f32).cos());
                        out_slice_res[i] += src_slice[i] * w;
                        out_slice_wei[i] += w;
                    }
                }

                out_pos += dst_t0;
                virt_in_pos += src_advance;
            }

            for i in 0..target_vowel_samples {
                if synth_weights[i] > 1e-4 {
                    synth_residual[i] /= synth_weights[i];
                }
            }

            output.extend(synth_residual);
        }

        output.truncate(target_total_samples);
        if output.len() < target_total_samples {
            output.resize(target_total_samples, 0.0);
        }

        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample_output_length() {
        let sample_rate = 44100;
        let dummy_samples = vec![0.0f32; sample_rate as usize]; // 1 sec
        let rendered =
            Resampler::render_sample(&dummy_samples, sample_rate, 0.0, 100.0, 0.0, 500.0, 440.0);
        let expected_len = (sample_rate as f64 * 0.5) as usize;
        assert_eq!(rendered.len(), expected_len);
    }

    #[test]
    fn test_resample_with_negative_cutoff() {
        let sample_rate = 44100;
        let dummy_samples: Vec<f32> = (0..sample_rate as usize * 2)
            .map(|i| (i as f32 * 0.1).sin())
            .collect();
        let rendered = Resampler::render_sample_with_pitch_bend(
            &dummy_samples,
            sample_rate,
            100.0,
            50.0,
            -100.0,
            400.0,
            440.0,
            &[],
        );
        let expected_len = (sample_rate as f64 * 0.4) as usize;
        assert_eq!(rendered.len(), expected_len);
        let max_amp = rendered.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(
            max_amp > 1e-3,
            "Expected non-silent output, got max_amp {}",
            max_amp
        );
    }

    #[test]
    fn consonant_timing_compresses_the_complete_source_segment() {
        let sample_rate = 1_000;
        let input: Vec<f32> = (0..200).map(|index| index as f32 / 199.0).collect();
        let rendered = Resampler::render_sample_with_pitch_bend_and_consonant_timing(
            &input,
            sample_rate,
            0.0,
            100.0,
            50.0,
            0.0,
            50.0,
            100.0,
            &[],
        );

        assert_eq!(rendered.len(), 50);
        assert!((rendered[0] - input[0]).abs() < 1e-6);
        assert!((rendered[49] - input[99]).abs() < 1e-6);
    }

    #[test]
    fn internal_engines_do_not_read_past_a_negative_cutoff() {
        let sample_rate = 1_000;
        let mut input = vec![0.25; 200];
        input.extend(vec![0.9; 800]);

        let td_psola = Resampler::render_sample_with_pitch_bend_and_consonant_timing(
            &input,
            sample_rate,
            0.0,
            300.0,
            200.0,
            -200.0,
            200.0,
            100.0,
            &[],
        );
        let sola = crate::dsp::SolaResampler::render_sample(
            &input,
            sample_rate,
            0.0,
            300.0,
            200.0,
            -200.0,
            200.0,
            100.0,
            &[],
            None,
            None,
            None,
        );

        assert!(td_psola.iter().all(|sample| (*sample - 0.25).abs() < 1e-4));
        assert!(sola.iter().all(|sample| (*sample - 0.25).abs() < 1e-4));
    }

    #[test]
    fn light_pitch_bend_does_not_jump_to_chipmunk_range() {
        let sample_rate = 8_000;
        let source_hz = 220.0;
        let input: Vec<f32> = (0..sample_rate)
            .map(|index| {
                (2.0 * std::f64::consts::PI * source_hz * index as f64 / sample_rate as f64).sin()
                    as f32
            })
            .collect();
        let bend = [
            crate::project::model::UPitchBendPoint {
                time_offset_ms: 0.0,
                pitch_offset_cents: -20.0,
                shape: "l".to_string(),
            },
            crate::project::model::UPitchBendPoint {
                time_offset_ms: 500.0,
                pitch_offset_cents: 20.0,
                shape: "l".to_string(),
            },
        ];
        let render_td = Resampler::render_sample_with_pitch_bend_and_consonant_timing(
            &input,
            sample_rate,
            0.0,
            0.0,
            0.0,
            0.0,
            500.0,
            source_hz,
            &bend,
        );
        let render_sola = crate::dsp::SolaResampler::render_sample(
            &input,
            sample_rate,
            0.0,
            0.0,
            0.0,
            0.0,
            500.0,
            source_hz,
            &bend,
            None,
            None,
            None,
        );

        fn zero_crossing_hz(samples: &[f32], sample_rate: u32) -> f64 {
            let crossings = samples
                .windows(2)
                .filter(|pair| pair[0] <= 0.0 && pair[1] > 0.0)
                .count();
            crossings as f64 * sample_rate as f64 / samples.len() as f64
        }

        let expected = source_hz;
        for (engine, rendered) in [("SOLA", &render_sola), ("TD-PSOLA", &render_td)] {
            let measured = zero_crossing_hz(rendered, sample_rate);
            assert!(
                (measured - expected).abs() < 45.0,
                "{engine}: pitch leve virou frequência anormal: esperado {expected:.1} Hz, medido {measured:.1} Hz"
            );
        }
    }
}
