use crate::dsp::pitch_bend::PitchBendSolver;
use crate::project::model::UPitchBendPoint;

pub struct SolaResampler;

#[derive(Debug, Clone, Copy)]
struct PitchEstimate {
    period: f64,
    periodicity: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolaStretchMode {
    Stretch,
    Loop,
    Spline,
    Hybrid,
}

impl SolaResampler {
    fn estimate_pitch(slice: &[f32], sample_rate: u32) -> Option<PitchEstimate> {
        if slice.len() < 32 || sample_rate == 0 {
            return None;
        }

        // YIN's cumulative mean normalized difference is considerably more
        // resistant to octave errors than choosing the largest autocorrelation
        // peak. The range covers bass through soprano while keeping the hot
        // loop bounded for one-shot voicebank samples.
        let min_period = ((sample_rate as f64 / 1_100.0).floor() as usize).max(8);
        let max_period =
            ((sample_rate as f64 / 55.0).ceil() as usize).min(slice.len().saturating_sub(2) / 2);
        if max_period <= min_period {
            return None;
        }

        let window_len = (max_period * 2)
            .min(2_048)
            .min(slice.len().saturating_sub(max_period));
        if window_len < max_period {
            return None;
        }

        let available = slice.len().saturating_sub(window_len + max_period);
        let frame_start = available / 2;
        let frame = &slice[frame_start..];
        let mut cmnd = vec![1.0f64; max_period + 1];
        let mut cumulative = 0.0f64;

        for tau in 1..=max_period {
            let mut difference = 0.0f64;
            for index in 0..window_len {
                let delta = f64::from(frame[index]) - f64::from(frame[index + tau]);
                difference += delta * delta;
            }
            cumulative += difference;
            cmnd[tau] = if cumulative > 1e-12 {
                difference * tau as f64 / cumulative
            } else {
                1.0
            };
        }

        let mut selected = None;
        let threshold = 0.18;
        for tau in min_period..=max_period {
            if cmnd[tau] < threshold {
                let mut local = tau;
                while local < max_period && cmnd[local + 1] < cmnd[local] {
                    local += 1;
                }
                selected = Some(local);
                break;
            }
        }
        let best = selected.unwrap_or_else(|| {
            (min_period..=max_period)
                .min_by(|&a, &b| cmnd[a].total_cmp(&cmnd[b]))
                .unwrap_or(min_period)
        });

        let mut period = best as f64;
        if best > min_period && best < max_period {
            let left = cmnd[best - 1];
            let center = cmnd[best];
            let right = cmnd[best + 1];
            let denominator = left - 2.0 * center + right;
            if denominator > 1e-12 {
                period += ((left - right) / (2.0 * denominator)).clamp(-0.5, 0.5);
            }
        }

        Some(PitchEstimate {
            period,
            periodicity: (1.0 - cmnd[best]).clamp(0.0, 1.0) as f32,
        })
    }

    /// Estimate the fundamental period using YIN with sub-sample refinement.
    pub fn estimate_pitch_period(slice: &[f32], sample_rate: u32) -> usize {
        Self::estimate_pitch(slice, sample_rate)
            .map(|estimate| estimate.period.round() as usize)
            .unwrap_or_else(|| ((sample_rate as f64 / 220.0).round() as usize).max(8))
    }

    fn phase_match_mark(slice: &[f32], reference: usize, predicted: usize, period: f64) -> usize {
        let radius = (period * 0.22).round().max(2.0) as usize;
        let compare_half = (period * 0.42).round().clamp(8.0, 256.0) as usize;
        let search_start = predicted.saturating_sub(radius).max(compare_half + 1);
        let search_end = (predicted + radius).min(slice.len().saturating_sub(compare_half + 2));
        if search_start >= search_end || reference <= compare_half {
            return predicted.min(slice.len().saturating_sub(1));
        }

        let mut best = predicted.clamp(search_start, search_end);
        let mut best_score = f64::NEG_INFINITY;
        for candidate in search_start..=search_end {
            let mut dot = 0.0f64;
            let mut ref_energy = 0.0f64;
            let mut candidate_energy = 0.0f64;
            for delta in 0..compare_half * 2 {
                let ref_sample = f64::from(slice[reference - compare_half + delta]);
                let candidate_sample = f64::from(slice[candidate - compare_half + delta]);
                dot += ref_sample * candidate_sample;
                ref_energy += ref_sample * ref_sample;
                candidate_energy += candidate_sample * candidate_sample;
            }
            let score = if ref_energy > 1e-12 && candidate_energy > 1e-12 {
                dot / (ref_energy * candidate_energy).sqrt()
            } else {
                f64::NEG_INFINITY
            };
            if score > best_score {
                best_score = score;
                best = candidate;
            }
        }
        best
    }

    /// Track phase-coherent pitch marks instead of independently snapping
    /// every mark to the loudest peak, which often alternates between harmonics.
    pub fn find_pitch_marks(slice: &[f32], base_period: usize) -> Vec<usize> {
        if slice.is_empty() || base_period == 0 {
            return Vec::new();
        }

        let period = base_period.max(8) as f64;
        let center = slice.len() / 3;
        let radius = base_period / 2;
        let anchor_start = center.saturating_sub(radius).max(1);
        let anchor_end = (center + radius).min(slice.len().saturating_sub(2));
        let anchor = (anchor_start..=anchor_end)
            .max_by(|&a, &b| {
                let score = |index: usize| {
                    (slice[index] - slice[index - 1]).abs() + slice[index].abs() * 0.2
                };
                score(a).total_cmp(&score(b))
            })
            .unwrap_or(center.min(slice.len() - 1));

        let mut forward = vec![anchor];
        let mut current = anchor;
        while current + base_period / 2 < slice.len().saturating_sub(2) {
            let predicted = (current as f64 + period).round() as usize;
            if predicted >= slice.len().saturating_sub(1) {
                break;
            }
            let next = Self::phase_match_mark(slice, current, predicted, period);
            if next <= current {
                break;
            }
            forward.push(next);
            current = next;
        }

        let mut backward = Vec::new();
        current = anchor;
        while current > base_period / 2 + 2 {
            let predicted = current.saturating_sub(base_period);
            let previous = Self::phase_match_mark(slice, current, predicted, period);
            if previous >= current {
                break;
            }
            backward.push(previous);
            current = previous;
        }
        backward.reverse();
        backward.extend(forward);
        let mut marks = backward;
        marks.dedup();
        marks
    }

    /// Resample a single phoneme sample using advanced Pitch-Synchronous Overlap-Add (TD-PSOLA).
    #[allow(clippy::too_many_arguments)]
    pub fn render_sample(
        input_samples: &[f32],
        sample_rate: u32,
        offset_ms: f64,
        source_consonant_ms: f64,
        target_consonant_ms: f64,
        cutoff_ms: f64,
        target_duration_ms: f64,
        target_pitch_freq: f64,
        pitch_points: &[UPitchBendPoint],
        loop_start_ms: Option<f64>,
        loop_end_ms: Option<f64>,
        tail_start_ms: Option<f64>,
    ) -> Vec<f32> {
        Self::render_sample_with_mode(
            input_samples,
            sample_rate,
            offset_ms,
            source_consonant_ms,
            target_consonant_ms,
            cutoff_ms,
            target_duration_ms,
            target_pitch_freq,
            pitch_points,
            loop_start_ms,
            loop_end_ms,
            tail_start_ms,
            SolaStretchMode::Stretch,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_sample_with_mode(
        input_samples: &[f32],
        sample_rate: u32,
        offset_ms: f64,
        source_consonant_ms: f64,
        target_consonant_ms: f64,
        cutoff_ms: f64,
        target_duration_ms: f64,
        target_pitch_freq: f64,
        pitch_points: &[UPitchBendPoint],
        loop_start_ms: Option<f64>,
        loop_end_ms: Option<f64>,
        tail_start_ms: Option<f64>,
        mode: SolaStretchMode,
    ) -> Vec<f32> {
        let (start_sample, end_sample) =
            crate::dsp::oto_source_bounds(input_samples.len(), sample_rate, offset_ms, cutoff_ms);

        let target_total_samples =
            ((target_duration_ms / 1000.0) * sample_rate as f64).round() as usize;

        if start_sample >= end_sample
            || start_sample >= input_samples.len()
            || target_total_samples == 0
        {
            return vec![0.0; target_total_samples];
        }

        let slice = &input_samples[start_sample..end_sample];

        let source_consonant_samples = ((source_consonant_ms.max(0.0) / 1000.0)
            * sample_rate as f64)
            .clamp(0.0, slice.len() as f64) as usize;

        let consonant_slice = &slice[..source_consonant_samples];

        // Keep a tiny piece of the voiced transition on both sides. Splitting
        // exactly at the OTO consonant boundary can cut a glottal cycle in half
        // and turn an otherwise correct resynthesis into an audible click.
        let join_samples = ((sample_rate as f64 * 0.005).round() as usize)
            .min(source_consonant_samples)
            .min(slice.len().saturating_sub(source_consonant_samples));
        let vowel_start = source_consonant_samples.saturating_sub(join_samples);
        let vowel_slice = &slice[vowel_start..];

        let target_consonant_samples =
            (((target_consonant_ms.max(0.0) / 1000.0) * sample_rate as f64).round() as usize)
                .min(target_total_samples);
        let target_vowel_samples = target_total_samples.saturating_sub(target_consonant_samples);
        let target_join_samples = join_samples
            .min(target_consonant_samples)
            .min(target_vowel_samples);

        let mut output = Vec::with_capacity(target_total_samples);

        // 1. Process Consonant (Preserve timing and formants)
        if target_consonant_samples > 0 && !consonant_slice.is_empty() {
            output.extend(crate::dsp::resize_preserving_pitch(
                consonant_slice,
                target_consonant_samples,
                sample_rate,
            ));
        } else if target_consonant_samples > 0 {
            output.resize(target_consonant_samples, 0.0);
        }

        // 2. Process vowel via adaptive TD-PSOLA. Render the shared boundary
        // once more so it can be equal-power crossfaded into the consonant.
        if target_vowel_samples > 0 && !vowel_slice.is_empty() {
            let vowel_out = Self::render_vowel_psola(
                vowel_slice,
                sample_rate,
                target_vowel_samples + target_join_samples,
                target_pitch_freq,
                pitch_points,
                (target_consonant_ms - target_join_samples as f64 * 1_000.0 / sample_rate as f64)
                    .max(0.0),
                loop_start_ms.map(|ms| {
                    (ms - offset_ms - source_consonant_ms
                        + join_samples as f64 * 1_000.0 / sample_rate as f64)
                        .max(0.0)
                }),
                loop_end_ms.map(|ms| {
                    (ms - offset_ms - source_consonant_ms
                        + join_samples as f64 * 1_000.0 / sample_rate as f64)
                        .max(0.0)
                }),
                tail_start_ms.map(|ms| {
                    (ms - offset_ms - source_consonant_ms
                        + join_samples as f64 * 1_000.0 / sample_rate as f64)
                        .max(0.0)
                }),
                mode,
            );

            if target_join_samples > 0 && output.len() >= target_join_samples {
                let output_start = output.len() - target_join_samples;
                for index in 0..target_join_samples {
                    let phase = (index as f32 + 0.5) / target_join_samples as f32
                        * std::f32::consts::FRAC_PI_2;
                    output[output_start + index] =
                        output[output_start + index] * phase.cos() + vowel_out[index] * phase.sin();
                }
                output.extend_from_slice(&vowel_out[target_join_samples..]);
            } else {
                output.extend(vowel_out);
            }
        }

        output.truncate(target_total_samples);
        if output.len() < target_total_samples {
            output.resize(target_total_samples, 0.0);
        }

        output
    }

    fn cubic_sample(samples: &[f32], position: f64) -> f32 {
        let base = position.floor() as isize;
        let fraction = (position - position.floor()) as f32;
        let at = |index: isize| {
            let clamped = index.clamp(0, samples.len().saturating_sub(1) as isize) as usize;
            samples[clamped]
        };
        let before = at(base - 1);
        let current = at(base);
        let next = at(base + 1);
        let after = at(base + 2);
        let slope = (next - before) * 0.5;
        let delta = current - next;
        let curve = slope + delta;
        let a = curve + delta + (after - current) * 0.5;
        let b = curve + a;
        ((a * fraction - b) * fraction + slope) * fraction + current
    }

    fn nearest_mark_index(marks: &[usize], source_position: f64) -> usize {
        let target = source_position.round().max(0.0) as usize;
        match marks.binary_search(&target) {
            Ok(index) => index,
            Err(0) => 0,
            Err(index) if index >= marks.len() => marks.len() - 1,
            Err(index) => {
                if target - marks[index - 1] <= marks[index] - target {
                    index - 1
                } else {
                    index
                }
            }
        }
    }

    fn local_period(marks: &[usize], index: usize, fallback: f64) -> f64 {
        let previous = index
            .checked_sub(1)
            .map(|previous| (marks[index] - marks[previous]) as f64);
        let next = marks
            .get(index + 1)
            .map(|next| (*next - marks[index]) as f64);
        match (previous, next) {
            (Some(previous), Some(next)) => (previous + next) * 0.5,
            (Some(period), None) | (None, Some(period)) => period,
            (None, None) => fallback,
        }
        .clamp(fallback * 0.55, fallback * 1.8)
    }

    fn source_position(
        output_position: usize,
        target_samples: usize,
        source_samples: usize,
        loop_start: usize,
        loop_end: usize,
        tail_start: Option<usize>,
        mode: SolaStretchMode,
    ) -> f64 {
        let minimum_attack = loop_start.min(target_samples);
        let tail_len = tail_start.map_or(0, |start| {
            source_samples
                .saturating_sub(start)
                .min(target_samples.saturating_sub(minimum_attack))
        });
        let target_tail_start = target_samples.saturating_sub(tail_len);
        let attack_end = loop_start.min(target_tail_start);

        if output_position < attack_end {
            return output_position as f64;
        }
        if let Some(tail_start) = tail_start {
            if output_position >= target_tail_start {
                return (tail_start + output_position - target_tail_start) as f64;
            }
        }

        let loop_len = loop_end.saturating_sub(loop_start).max(1);
        let middle_position = output_position.saturating_sub(attack_end);
        let target_middle_len = target_tail_start.saturating_sub(attack_end).max(1);
        let mapped = match mode {
            SolaStretchMode::Loop => (middle_position % loop_len) as f64,
            SolaStretchMode::Stretch => {
                if target_middle_len <= loop_len {
                    middle_position as f64 * loop_len as f64 / target_middle_len as f64
                } else {
                    (middle_position % loop_len) as f64
                }
            }
            SolaStretchMode::Spline => {
                let phase = (middle_position % loop_len) as f64 / loop_len as f64;
                let smooth = phase * phase * (3.0 - 2.0 * phase);
                smooth * loop_len as f64
            }
            SolaStretchMode::Hybrid => {
                // Hybrid mode traverses alternate cycles backwards, avoiding a
                // hard jump from the end to the beginning of long sustain loops.
                let cycle = middle_position / loop_len;
                let within = (middle_position % loop_len) as f64;
                if cycle % 2 == 0 {
                    within
                } else {
                    loop_len as f64 - within
                }
            }
        };
        loop_start as f64 + mapped.min(loop_len.saturating_sub(1) as f64)
    }

    #[allow(clippy::too_many_arguments)]
    fn render_vowel_psola(
        vowel: &[f32],
        sample_rate: u32,
        target_samples: usize,
        target_pitch_freq: f64,
        pitch_points: &[UPitchBendPoint],
        start_time_offset_ms: f64,
        loop_start_ms: Option<f64>,
        loop_end_ms: Option<f64>,
        tail_start_ms: Option<f64>,
        mode: SolaStretchMode,
    ) -> Vec<f32> {
        let v_len = vowel.len();
        if v_len < 32 {
            return vec![0.0; target_samples];
        }

        // Resolve the stable sustain before analysis so noisy attacks and
        // releases do not dominate F0 detection.
        let fallback_loop = || {
            let start = v_len / 4;
            let end = (v_len * 3 / 4).max(start + 16).min(v_len);
            (start, end)
        };
        let (loop_start_samp, loop_end_samp) = match (loop_start_ms, loop_end_ms) {
            (Some(ls), Some(le)) => {
                let s = ((ls / 1000.0) * sample_rate as f64).round() as usize;
                let e = ((le / 1000.0) * sample_rate as f64).round() as usize;
                if s + 16 < e && e <= v_len {
                    (s, e)
                } else {
                    fallback_loop()
                }
            }
            _ => fallback_loop(),
        };

        let analysis_slice = &vowel[loop_start_samp..loop_end_samp];
        let Some(estimate) = Self::estimate_pitch(analysis_slice, sample_rate)
            .or_else(|| Self::estimate_pitch(vowel, sample_rate))
        else {
            return crate::dsp::resize_preserving_pitch(vowel, target_samples, sample_rate);
        };

        // PSOLA makes noise periodic. Route genuinely unvoiced/breathy regions
        // through WSOLA, which preserves their stochastic texture and timing.
        if estimate.periodicity < 0.42 {
            return crate::dsp::resize_preserving_pitch(vowel, target_samples, sample_rate);
        }

        let base_period = estimate.period.round().max(8.0) as usize;
        let pitch_marks = Self::find_pitch_marks(vowel, base_period);
        if pitch_marks.len() < 2 {
            return crate::dsp::resize_preserving_pitch(vowel, target_samples, sample_rate);
        }

        let tail_start_samp = tail_start_ms
            .map(|ms| ((ms / 1000.0) * sample_rate as f64).round() as usize)
            .filter(|&sample| sample < v_len && sample > loop_start_samp);

        let mut output = vec![0.0f32; target_samples];
        let mut weights = vec![0.0f32; target_samples];
        let mut output_center = 0.0f64;
        let minimum_target_period = (sample_rate as f64 / 1_400.0).max(8.0);
        let maximum_target_period = sample_rate as f64 / 40.0;

        // Fractional synthesis marks eliminate the integer-hop jitter that is
        // especially audible on sustained vowels and vibrato.
        while (output_center as usize) < target_samples {
            let cur_time_ms = start_time_offset_ms + output_center / sample_rate as f64 * 1_000.0;
            let pitch_cents = PitchBendSolver::get_pitch_offset_cents(cur_time_ms, pitch_points);
            let cur_target_freq =
                (target_pitch_freq * 2.0f64.powf(pitch_cents / 1_200.0)).clamp(40.0, 1_400.0);
            let target_period = (sample_rate as f64 / cur_target_freq)
                .clamp(minimum_target_period, maximum_target_period);
            let source_position = Self::source_position(
                output_center.round() as usize,
                target_samples,
                v_len,
                loop_start_samp,
                loop_end_samp,
                tail_start_samp,
                mode,
            );
            let mark_index = Self::nearest_mark_index(&pitch_marks, source_position);
            let source_mark = pitch_marks[mark_index] as f64;
            let source_period = Self::local_period(&pitch_marks, mark_index, estimate.period);
            // For large downward shifts the synthesis marks are farther apart.
            // Widen just enough to maintain coverage without resampling a grain.
            let grain_radius = source_period
                .max(target_period * 0.55)
                .clamp(8.0, sample_rate as f64 / 35.0);
            let first = (output_center - grain_radius).ceil() as isize;
            let last = (output_center + grain_radius).floor() as isize;
            let overlap_gain = (target_period / source_period).clamp(0.25, 4.0) as f32;

            for output_index in first..=last {
                if output_index < 0 || output_index as usize >= target_samples {
                    continue;
                }
                let delta = output_index as f64 - output_center;
                let source_sample_position = source_mark + delta;
                if source_sample_position < 0.0 || source_sample_position >= v_len as f64 {
                    continue;
                }
                let window =
                    (0.5 + 0.5 * (std::f64::consts::PI * delta / grain_radius).cos()) as f32;
                let output_index = output_index as usize;
                output[output_index] +=
                    Self::cubic_sample(vowel, source_sample_position) * window * overlap_gain;
                weights[output_index] += window * overlap_gain;
            }

            output_center += target_period;
        }

        let mut fallback = None;
        for index in 0..target_samples {
            if weights[index] <= 1e-5 {
                let fallback = fallback.get_or_insert_with(|| {
                    crate::dsp::resize_preserving_pitch(vowel, target_samples, sample_rate)
                });
                output[index] = fallback[index];
            }
        }

        // The theoretical OLA gain above keeps identity and pitch ratios
        // correct. A restrained RMS correction compensates truncated edge
        // grains and real-world irregular pitch marks without pumping.
        let source_rms = (analysis_slice
            .iter()
            .map(|sample| f64::from(*sample).powi(2))
            .sum::<f64>()
            / analysis_slice.len().max(1) as f64)
            .sqrt();
        let output_rms = (output
            .iter()
            .map(|sample| f64::from(*sample).powi(2))
            .sum::<f64>()
            / output.len().max(1) as f64)
            .sqrt();
        if source_rms > 1e-6 && output_rms > 1e-6 {
            let level_gain = (source_rms / output_rms).clamp(0.67, 1.5) as f32;
            for sample in &mut output {
                *sample *= level_gain;
            }
        }
        output
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rms(samples: &[f32]) -> f64 {
        (samples
            .iter()
            .map(|sample| f64::from(*sample).powi(2))
            .sum::<f64>()
            / samples.len().max(1) as f64)
            .sqrt()
    }

    fn voice_like_tone(sample_rate: u32, frequency: f64, duration: f64) -> Vec<f32> {
        let sample_count = (sample_rate as f64 * duration).round() as usize;
        (0..sample_count)
            .map(|index| {
                let phase = std::f64::consts::TAU * frequency * index as f64 / sample_rate as f64;
                (0.38 * phase.sin() + 0.48 * (phase * 2.0).sin() + 0.14 * (phase * 3.0).sin())
                    as f32
            })
            .collect()
    }

    #[test]
    fn yin_estimate_uses_the_fundamental_with_strong_harmonics() {
        let sample_rate = 16_000;
        let input = voice_like_tone(sample_rate, 200.0, 0.35);
        let estimate = SolaResampler::estimate_pitch(&input, sample_rate).unwrap();
        assert!(
            (estimate.period - 80.0).abs() < 1.0,
            "estimated period was {:.3}",
            estimate.period
        );
        assert!(estimate.periodicity > 0.85);
    }

    #[test]
    fn adaptive_psola_tracks_pitch_and_keeps_level_across_an_octave() {
        let sample_rate = 16_000;
        let input = voice_like_tone(sample_rate, 220.0, 0.5);

        for target_frequency in [110.0, 440.0] {
            let output = SolaResampler::render_sample(
                &input,
                sample_rate,
                0.0,
                0.0,
                0.0,
                0.0,
                700.0,
                target_frequency,
                &[],
                Some(100.0),
                Some(400.0),
                None,
            );
            let middle = &output[output.len() / 4..output.len() * 3 / 4];
            let estimate = SolaResampler::estimate_pitch(middle, sample_rate).unwrap();
            let measured_frequency = sample_rate as f64 / estimate.period;
            assert!(
                (measured_frequency - target_frequency).abs() < target_frequency * 0.035,
                "target {target_frequency:.1} Hz measured {measured_frequency:.1} Hz"
            );
            let level = rms(middle);
            assert!(
                (0.25..0.85).contains(&level),
                "unstable RMS {level:.3} at {target_frequency:.1} Hz"
            );
        }
    }

    #[test]
    fn sustain_does_not_replace_the_vowel_attack() {
        let sample_rate = 16_000;
        let mut input = voice_like_tone(sample_rate, 200.0, 0.4);
        for sample in &mut input[..(sample_rate as usize * 60 / 1_000)] {
            *sample *= 0.12;
        }
        let output = SolaResampler::render_sample(
            &input,
            sample_rate,
            0.0,
            0.0,
            0.0,
            0.0,
            800.0,
            200.0,
            &[],
            Some(100.0),
            Some(350.0),
            None,
        );

        let early = rms(&output[100..sample_rate as usize * 40 / 1_000]);
        let sustain =
            rms(&output[sample_rate as usize * 180 / 1_000..sample_rate as usize * 260 / 1_000]);
        assert!(
            early < sustain * 0.4,
            "attack RMS {early:.3} should remain below sustain {sustain:.3}"
        );
    }

    #[test]
    fn unvoiced_material_uses_wsola_without_becoming_silent() {
        let sample_rate = 16_000;
        let mut state = 0x1234_5678u32;
        let input = (0..sample_rate as usize / 4)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                ((state >> 8) as f32 / 0x00ff_ffff as f32 - 0.5) * 0.5
            })
            .collect::<Vec<_>>();
        let output = SolaResampler::render_sample(
            &input,
            sample_rate,
            0.0,
            0.0,
            0.0,
            0.0,
            600.0,
            440.0,
            &[],
            None,
            None,
            None,
        );
        assert_eq!(output.len(), 9_600);
        assert!(rms(&output) > 0.03);
    }

    #[test]
    fn test_sola_resampler_pitch_shift_and_loop() {
        let sample_rate = 44100;
        let freq = 220.0; // A3
        let duration_secs = 0.5;
        let total_samples = (sample_rate as f64 * duration_secs) as usize;

        // Generate synthetic sine wave
        let input_samples: Vec<f32> = (0..total_samples)
            .map(|i| {
                let t = i as f64 / sample_rate as f64;
                (2.0 * std::f64::consts::PI * freq * t).sin() as f32
            })
            .collect();

        // Render with SOLA stretching from 500ms to 1200ms with loop and pitch shifting to 440Hz (A4)
        let output = SolaResampler::render_sample(
            &input_samples,
            sample_rate,
            0.0,
            50.0,
            50.0,
            0.0,
            1200.0,
            440.0,
            &[],
            Some(100.0),
            Some(400.0),
            Some(450.0),
        );

        let expected_samples = (1.2 * sample_rate as f64).round() as usize;
        assert_eq!(output.len(), expected_samples);

        // Check that output is not silent and has healthy amplitude
        let max_amp = output.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(max_amp > 0.4, "Output amplitude too low: {}", max_amp);
    }

    #[test]
    fn all_stretch_modes_render_the_requested_duration() {
        let sample_rate = 8_000;
        let input = (0..4_000)
            .map(|index| {
                (std::f64::consts::TAU * 200.0 * index as f64 / sample_rate as f64).sin() as f32
            })
            .collect::<Vec<_>>();
        for mode in [
            SolaStretchMode::Stretch,
            SolaStretchMode::Loop,
            SolaStretchMode::Spline,
            SolaStretchMode::Hybrid,
        ] {
            let output = SolaResampler::render_sample_with_mode(
                &input,
                sample_rate,
                0.0,
                20.0,
                20.0,
                0.0,
                800.0,
                220.0,
                &[],
                None,
                None,
                None,
                mode,
            );
            assert_eq!(output.len(), 6_400, "wrong duration for {mode:?}");
            assert!(
                output.iter().any(|sample| sample.abs() > 0.01),
                "silent output for {mode:?}"
            );
        }
    }
}
