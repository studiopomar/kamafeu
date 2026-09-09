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
        // exactly at the oto.ini consonant boundary can cut a glottal cycle in half
        // and turn an otherwise correct resynthesis into an audible click.
        let join_samples = ((sample_rate as f64 * 0.005).round() as usize)
            .min(source_consonant_samples)
            .min(slice.len().saturating_sub(source_consonant_samples));
        let vowel_start = source_consonant_samples.saturating_sub(join_samples);
        let vowel_slice = &slice[vowel_start..];

        // If cutoff < 0 (VC transitions, consonant clusters, endings) or the slice after
        // consonant is very short (less than 25ms, impossible to extract periodic vowel pitch),
        // treat this as a finite transition slice. Never loop unvoiced consonant tail with PSOLA!
        let min_vowel_samples = (sample_rate as f64 * 0.025).round() as usize;
        let is_finite_transition = cutoff_ms < 0.0 || vowel_slice.len() < min_vowel_samples;

        let target_consonant_samples =
            (((target_consonant_ms.max(0.0) / 1000.0) * sample_rate as f64).round() as usize)
                .min(target_total_samples);
        let target_vowel_samples = target_total_samples.saturating_sub(target_consonant_samples);
        let target_join_samples = join_samples
            .min(target_consonant_samples)
            .min(target_vowel_samples);

        let mut output = Vec::with_capacity(target_total_samples);

        // Keep consonant timing independent while retuning any voiced portion
        // of the transition to the note pitch.
        if target_consonant_samples > 0 && !consonant_slice.is_empty() {
            output.extend(Self::render_consonant(
                consonant_slice,
                sample_rate,
                target_consonant_samples,
                target_pitch_freq,
                pitch_points,
            ));
        } else if target_consonant_samples > 0 {
            output.resize(target_consonant_samples, 0.0);
        }

        // 2. Process vowel via adaptive TD-PSOLA (or resize_preserving_pitch for finite transitions).
        // Render the shared boundary once more so it can be equal-power crossfaded into the consonant.
        if target_vowel_samples > 0 && !vowel_slice.is_empty() {
            let vowel_out = if vowel_slice.len() < 32 {
                // Truly too short for any periodic analysis; nothing to retune.
                let mut out = crate::dsp::resize_preserving_pitch(
                    vowel_slice,
                    target_vowel_samples + target_join_samples,
                    sample_rate,
                );
                if out.len() < target_vowel_samples + target_join_samples {
                    out.resize(target_vowel_samples + target_join_samples, 0.0);
                }
                out
            } else {
                // A negative cutoff_ms (fixed-length vowel region) or a short vowel
                // segment only means "don't loop this" — it does NOT mean the
                // segment is unvoiced. Route it through PSOLA too (forcing
                // Stretch mode so no loop is attempted) so it still gets retuned
                // to the piano-roll pitch. render_vowel_psola still falls back to
                // resize_preserving_pitch internally for genuinely unvoiced /
                // low-periodicity material.
                let effective_mode = if is_finite_transition {
                    SolaStretchMode::Stretch
                } else {
                    mode
                };
                Self::render_vowel_psola(
                    vowel_slice,
                    sample_rate,
                    target_vowel_samples + target_join_samples,
                    target_pitch_freq,
                    pitch_points,
                    (target_consonant_ms
                        - target_join_samples as f64 * 1_000.0 / sample_rate as f64)
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
                    effective_mode,
                )
            };

            if target_join_samples > 0 && output.len() >= target_join_samples {
                let output_start = output.len() - target_join_samples;
                for index in 0..target_join_samples {
                    let phase = (index as f32 + 0.5) / target_join_samples as f32;
                    let w_consonant = 1.0 - phase;
                    let w_vowel = phase;
                    output[output_start + index] =
                        output[output_start + index] * w_consonant + vowel_out[index] * w_vowel;
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

        let max_peak = output.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        if max_peak > 0.99 {
            let scale = 0.98 / max_peak;
            for s in &mut output {
                *s *= scale;
            }
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
                if target_middle_len <= 1 || loop_len <= 1 {
                    0.0
                } else {
                    middle_position.min(target_middle_len - 1) as f64 * (loop_len - 1) as f64
                        / (target_middle_len - 1) as f64
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

    fn voiced_transition_start(samples: &[f32], sample_rate: u32) -> Option<usize> {
        if samples.len() < 32 || sample_rate == 0 {
            return None;
        }
        let window = ((sample_rate as f64 * 0.07).round() as usize)
            .clamp(64, 4_096)
            .min(samples.len());
        if window == samples.len() {
            return Self::estimate_pitch(samples, sample_rate)
                .filter(|estimate| estimate.periodicity >= 0.32)
                .map(|_| 0);
        }

        let hop = ((sample_rate as f64 * 0.015).round() as usize).max(1);
        let mut candidate = None;
        let mut consecutive = 0;
        let last_start = samples.len() - window;
        for start in (0..=last_start).step_by(hop) {
            let voiced = Self::estimate_pitch(&samples[start..start + window], sample_rate)
                .is_some_and(|estimate| estimate.periodicity >= 0.32);
            if voiced {
                candidate.get_or_insert(start);
                consecutive += 1;
                if consecutive >= 2 {
                    return candidate;
                }
            } else {
                candidate = None;
                consecutive = 0;
            }
        }
        None
    }

    fn render_consonant(
        consonant: &[f32],
        sample_rate: u32,
        target_samples: usize,
        target_pitch_freq: f64,
        pitch_points: &[UPitchBendPoint],
    ) -> Vec<f32> {
        let preserved = crate::dsp::resize_preserving_pitch(consonant, target_samples, sample_rate);
        let Some(voiced_start) = Self::voiced_transition_start(consonant, sample_rate) else {
            return preserved;
        };
        let pitched = Self::render_vowel_psola(
            consonant,
            sample_rate,
            target_samples,
            target_pitch_freq,
            pitch_points,
            0.0,
            None,
            None,
            None,
            SolaStretchMode::Stretch,
        );
        let target_voiced_start =
            voiced_start as f64 * target_samples as f64 / consonant.len().max(1) as f64;
        let crossfade_samples = (sample_rate as f64 * 0.012).round().max(1.0);

        preserved
            .into_iter()
            .zip(pitched)
            .enumerate()
            .map(|(index, (preserved, pitched))| {
                let phase = ((index as f64 - target_voiced_start) / crossfade_samples)
                    .clamp(0.0, 1.0) as f32;
                preserved * (1.0 - phase) + pitched * phase
            })
            .collect()
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
        let fallback_sustain = || {
            let start = v_len / 4;
            let end = (v_len * 3 / 4).max(start + 16).min(v_len);
            (start, end)
        };
        let explicit_loop = match (loop_start_ms, loop_end_ms) {
            (Some(ls), Some(le)) => {
                let s = ((ls / 1000.0) * sample_rate as f64).round() as usize;
                let e = ((le / 1000.0) * sample_rate as f64).round() as usize;
                if s + 16 < e && e <= v_len {
                    Some((s, e))
                } else {
                    None
                }
            }
            _ => None,
        };
        let (analysis_start, analysis_end) = explicit_loop.unwrap_or_else(fallback_sustain);
        let (loop_start_samp, loop_end_samp) = explicit_loop.unwrap_or_else(|| {
            if mode == SolaStretchMode::Stretch {
                (0, v_len)
            } else {
                fallback_sustain()
            }
        });

        let analysis_slice = &vowel[analysis_start..analysis_end];
        let Some(estimate) = Self::estimate_pitch(analysis_slice, sample_rate)
            .or_else(|| Self::estimate_pitch(vowel, sample_rate))
        else {
            return crate::dsp::resize_preserving_pitch(vowel, target_samples, sample_rate);
        };

        // PSOLA makes noise periodic. Route genuinely unvoiced/breathy regions
        // through WSOLA, which preserves their stochastic texture and timing.
        if estimate.periodicity < 0.15 {
            return crate::dsp::resize_preserving_pitch(vowel, target_samples, sample_rate);
        }

        let base_period = estimate.period.round().max(8.0) as usize;
        let pitch_marks = Self::find_pitch_marks(vowel, base_period);
        if pitch_marks.len() < 2 {
            return crate::dsp::resize_preserving_pitch(vowel, target_samples, sample_rate);
        }

        let tail_start_samp = tail_start_ms
            .map(|ms| ((ms / 1000.0) * sample_rate as f64).round() as usize)
            .filter(|&sample| sample < v_len && sample > loop_start_samp)
            .or_else(|| explicit_loop.and_then(|(_, end)| (end < v_len).then_some(end)));

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
            // Formant preservation: the extracted grain keeps the ORIGINAL source
            // period as its window/content size, unscaled. Pitch is shifted purely
            // by how far apart successive grains are placed in the output
            // (`output_center += target_period`, below), never by resampling the
            // waveform inside the grain itself — resampling grain content ties
            // formants to pitch and causes the classic PSOLA "chipmunk" artifact.
            let grain_radius = source_period;
            let first = (output_center - grain_radius).ceil() as isize;
            let last = (output_center + grain_radius).floor() as isize;

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
                    Self::cubic_sample(vowel, source_sample_position) * window;
                weights[output_index] += window;
            }

            output_center += target_period;
        }

        let mut fallback = None;
        for index in 0..target_samples {
            if weights[index] > 1.0 {
                output[index] /= weights[index];
            } else if weights[index] <= 1e-5 {
                let fb = fallback.get_or_insert_with(|| {
                    crate::dsp::resize_preserving_pitch(vowel, target_samples, sample_rate)
                });
                output[index] = fb[index];
            }
        }

        // A restrained RMS correction compensates truncated edge grains and
        // real-world irregular pitch marks without volume pumping.
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
            let level_gain = (source_rms / output_rms).clamp(0.7, 1.4) as f32;
            for sample in &mut output {
                *sample *= level_gain;
            }
        }

        // Peak limiter: prevent any overshoot from clipping
        let source_peak = vowel
            .iter()
            .map(|s| s.abs())
            .fold(0.0f32, f32::max)
            .clamp(0.05, 1.0);
        let output_peak = output.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        if output_peak > source_peak && output_peak > 1e-4 {
            let target_peak = (source_peak * 0.999).min(0.98);
            let scale = target_peak / output_peak;
            for sample in &mut output {
                *sample *= scale;
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
    fn stretch_maps_the_source_forward_without_wrapping() {
        let positions = (0..800)
            .step_by(10)
            .map(|position| {
                SolaResampler::source_position(
                    position,
                    800,
                    400,
                    0,
                    400,
                    None,
                    SolaStretchMode::Stretch,
                )
            })
            .collect::<Vec<_>>();

        assert!(positions.windows(2).all(|pair| pair[1] >= pair[0]));
        assert!(positions.last().copied().unwrap_or_default() > 390.0);
    }

    #[test]
    fn voiced_consonant_transition_is_retuned_to_the_note() {
        let sample_rate = 16_000;
        let input = voice_like_tone(sample_rate, 220.0, 0.4);
        let output = SolaResampler::render_sample(
            &input,
            sample_rate,
            0.0,
            220.0,
            220.0,
            0.0,
            400.0,
            440.0,
            &[],
            None,
            None,
            None,
        );
        let transition =
            &output[sample_rate as usize * 40 / 1_000..sample_rate as usize * 170 / 1_000];
        let estimate = SolaResampler::estimate_pitch(transition, sample_rate).unwrap();
        let measured_frequency = sample_rate as f64 / estimate.period;

        assert!(
            (measured_frequency - 440.0).abs() < 20.0,
            "voiced transition measured {measured_frequency:.1} Hz"
        );
    }

    #[test]
    fn unvoiced_consonant_attack_is_preserved_before_retuning() {
        let sample_rate = 16_000;
        let attack_len = sample_rate as usize * 120 / 1_000;
        let mut state = 0x4d59_5df4u32;
        let mut input = (0..attack_len)
            .map(|_| {
                state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                ((state >> 8) as f32 / 0x00ff_ffff as f32 - 0.5) * 0.5
            })
            .collect::<Vec<_>>();
        input.extend(voice_like_tone(sample_rate, 220.0, 0.18));

        let output = SolaResampler::render_consonant(&input, sample_rate, input.len(), 440.0, &[]);
        let comparison_len = sample_rate as usize * 50 / 1_000;
        let mean_squared_error = input[..comparison_len]
            .iter()
            .zip(&output[..comparison_len])
            .map(|(source, rendered)| f64::from(source - rendered).powi(2))
            .sum::<f64>()
            / comparison_len as f64;

        assert!(
            mean_squared_error < 1e-12,
            "unvoiced attack changed with MSE {mean_squared_error}"
        );
        let voiced_tail = &output[output.len() - sample_rate as usize * 90 / 1_000..];
        let measured_frequency = sample_rate as f64
            / SolaResampler::estimate_pitch(voiced_tail, sample_rate)
                .unwrap()
                .period;
        assert!(
            (measured_frequency - 440.0).abs() < 22.0,
            "voiced consonant tail measured {measured_frequency:.1} Hz"
        );
    }

    #[test]
    fn psola_follows_a_portamento_curve() {
        let sample_rate = 16_000;
        let input = voice_like_tone(sample_rate, 220.0, 0.5);
        let pitch_points = [
            UPitchBendPoint {
                time_offset_ms: 0.0,
                pitch_offset_cents: 0.0,
                shape: "l".to_string(),
            },
            UPitchBendPoint {
                time_offset_ms: 180.0,
                pitch_offset_cents: 0.0,
                shape: "l".to_string(),
            },
            UPitchBendPoint {
                time_offset_ms: 380.0,
                pitch_offset_cents: 1_200.0,
                shape: "l".to_string(),
            },
            UPitchBendPoint {
                time_offset_ms: 600.0,
                pitch_offset_cents: 1_200.0,
                shape: "l".to_string(),
            },
        ];
        let output = SolaResampler::render_sample(
            &input,
            sample_rate,
            0.0,
            0.0,
            0.0,
            0.0,
            600.0,
            220.0,
            &pitch_points,
            None,
            None,
            None,
        );
        let early = &output[sample_rate as usize * 50 / 1_000..sample_rate as usize * 150 / 1_000];
        let late = &output[sample_rate as usize * 440 / 1_000..sample_rate as usize * 560 / 1_000];
        let early_frequency = sample_rate as f64
            / SolaResampler::estimate_pitch(early, sample_rate)
                .unwrap()
                .period;
        let late_frequency = sample_rate as f64
            / SolaResampler::estimate_pitch(late, sample_rate)
                .unwrap()
                .period;

        assert!((early_frequency - 220.0).abs() < 12.0);
        assert!((late_frequency - 440.0).abs() < 22.0);
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

    #[test]
    fn sola_never_clips_hot_inputs_across_large_pitch_shifts() {
        let sample_rate = 44_100;
        let source_hz = 220.0;
        let input_samples: Vec<f32> = (0..sample_rate as usize)
            .map(|i| {
                let t = i as f64 / sample_rate as f64;
                let phase = std::f64::consts::TAU * source_hz * t;
                (0.45 * phase.sin() + 0.35 * (phase * 2.0).sin() + 0.15 * (phase * 3.0).sin())
                    as f32
            })
            .collect();

        for target_pitch in [55.0, 110.0, 165.0, 220.0, 330.0, 440.0, 660.0, 880.0] {
            let rendered = SolaResampler::render_sample(
                &input_samples,
                sample_rate,
                0.0,
                30.0,
                30.0,
                0.0,
                600.0,
                target_pitch,
                &[],
                Some(100.0),
                Some(500.0),
                None,
            );

            let max_amp = rendered.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
            assert!(
                max_amp <= 1.0,
                "Clipping detected at target pitch {target_pitch} Hz: peak amplitude was {max_amp}"
            );
            assert!(
                max_amp >= 0.25,
                "Signal dropped out at target pitch {target_pitch} Hz: peak amplitude was {max_amp}"
            );
        }
    }

    #[test]
    fn consonant_vowel_junction_never_clips() {
        let sample_rate = 44_100;
        let input_samples = vec![0.95f32; sample_rate as usize / 2];
        let rendered = SolaResampler::render_sample(
            &input_samples,
            sample_rate,
            0.0,
            100.0,
            100.0,
            0.0,
            400.0,
            220.0,
            &[],
            None,
            None,
            None,
        );

        let max_amp = rendered.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        assert!(
            max_amp <= 1.0,
            "Boundary crossfade caused clipping: peak was {max_amp}"
        );
    }
}
