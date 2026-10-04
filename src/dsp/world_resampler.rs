//! WORLD-based resampler core.
//!
//! This module is deliberately separate from the legacy Venus implementation.
//! It follows the reference WORLD pipeline: DIO, StoneMask, CheapTrick and
//! WORLD synthesis. Timing/alias slicing remains outside this module so the
//! renderer can continue to own UTAU oto.ini semantics.

use crate::project::model::UPitchBendPoint;
use std::sync::{OnceLock, RwLock};

#[derive(Debug, Clone)]
pub struct WorldRuntimeConfig {
    pub f0_floor_hz: f64,
    pub f0_ceil_hz: f64,
    pub voiced_aperiodicity: f64,
    pub formant_preservation_mode: String,
    pub f0_detection_method: String,
    pub frame_period_ms: f64,
}

impl Default for WorldRuntimeConfig {
    fn default() -> Self {
        Self {
            // Defaults aligned with straycat-rs' stable WORLD profile. The
            // range remains configurable for bass/child voicebanks, but the
            // conservative default rejects low-frequency noise and avoids
            // high-frequency octave decisions in ordinary singing voices.
            f0_floor_hz: 71.0,
            f0_ceil_hz: 1760.0,
            voiced_aperiodicity: 0.18,
            formant_preservation_mode: "LPC Spectral Envelope".to_string(),
            f0_detection_method: "Harvest/DIO (Espectral)".to_string(),
            frame_period_ms: 5.0,
        }
    }
}

fn runtime_config() -> &'static RwLock<WorldRuntimeConfig> {
    static CONFIG: OnceLock<RwLock<WorldRuntimeConfig>> = OnceLock::new();
    CONFIG.get_or_init(|| RwLock::new(WorldRuntimeConfig::default()))
}

pub fn set_runtime_config(config: WorldRuntimeConfig) {
    if let Ok(mut current) = runtime_config().write() {
        *current = config;
    }
}

pub fn cache_identity() -> String {
    let config = runtime_config()
        .read()
        .map(|value| value.clone())
        .unwrap_or_default();
    format!(
        "world-rs:f0-{:.3}-{:.3}:ap-{:.4}:formant-{}:detector-{}:frame-{:.3}",
        config.f0_floor_hz,
        config.f0_ceil_hz,
        config.voiced_aperiodicity,
        config.formant_preservation_mode,
        config.f0_detection_method,
        config.frame_period_ms
    )
}

pub struct WorldResampler;

impl WorldResampler {
    #[allow(clippy::too_many_arguments)]
    pub fn render_sample(
        input_samples: &[f32],
        sample_rate: u32,
        offset_ms: f64,
        source_consonant_ms: f64,
        target_consonant_ms: f64,
        target_duration_ms: f64,
        target_pitch_freq: f64,
        pitch_points: &[UPitchBendPoint],
        cutoff_ms: f64,
    ) -> Vec<f32> {
        Self::render_sample_with_oto(
            input_samples,
            sample_rate,
            offset_ms,
            source_consonant_ms,
            target_consonant_ms,
            target_duration_ms,
            target_pitch_freq,
            pitch_points,
            cutoff_ms,
            None,
            None,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_sample_with_oto(
        input_samples: &[f32],
        sample_rate: u32,
        offset_ms: f64,
        source_consonant_ms: f64,
        target_consonant_ms: f64,
        target_duration_ms: f64,
        target_pitch_freq: f64,
        pitch_points: &[UPitchBendPoint],
        cutoff_ms: f64,
        loop_start_ms: Option<f64>,
        loop_end_ms: Option<f64>,
        tail_start_ms: Option<f64>,
    ) -> Vec<f32> {
        let target_len =
            ((target_duration_ms.max(0.0) / 1000.0) * sample_rate as f64).round() as usize;
        if target_len == 0 || input_samples.is_empty() || sample_rate == 0 {
            return vec![0.0; target_len];
        }

        let (start, end) =
            crate::dsp::oto_source_bounds(input_samples.len(), sample_rate, offset_ms, cutoff_ms);
        if start >= end || end - start < 64 {
            return vec![0.0; target_len];
        }

        let input: Vec<f64> = input_samples[start..end]
            .iter()
            .map(|sample| {
                if sample.is_finite() {
                    f64::from(*sample)
                } else {
                    0.0
                }
            })
            .collect();
        let input_f32: Vec<f32> = input.iter().map(|sample| *sample as f32).collect();
        let source_consonant_samples = ((source_consonant_ms.max(0.0) / 1000.0)
            * sample_rate as f64)
            .round()
            .clamp(0.0, input_f32.len() as f64) as usize;
        let target_consonant_samples = ((target_consonant_ms.max(0.0) / 1000.0)
            * sample_rate as f64)
            .round()
            .clamp(0.0, target_len as f64) as usize;

        // WORLD is a voiced spectral analyser. Do not feed the consonant
        // attack into it: doing so creates unstable F0/AP frames and the
        // characteristic granular edge on CV/VCV aliases. Preserve the
        // consonant separately, then analyse only the vowel region.
        if source_consonant_samples > 0
            && source_consonant_samples < input_f32.len()
            && target_consonant_samples > 0
            && target_consonant_samples < target_len
        {
            let vowel_duration_ms =
                target_duration_ms - target_consonant_samples as f64 * 1000.0 / sample_rate as f64;
            let vowel = Self::render_sample_with_oto(
                &input_f32[source_consonant_samples..],
                sample_rate,
                0.0,
                0.0,
                0.0,
                vowel_duration_ms.max(1.0),
                target_pitch_freq,
                pitch_points,
                0.0,
                loop_start_ms
                    .map(|value| value - source_consonant_ms)
                    .filter(|value| *value >= 0.0),
                loop_end_ms
                    .map(|value| value - source_consonant_ms)
                    .filter(|value| *value >= 0.0),
                tail_start_ms
                    .map(|value| value - source_consonant_ms)
                    .filter(|value| *value >= 0.0),
            );
            let consonant = crate::dsp::resize_preserving_pitch(
                &input_f32[..source_consonant_samples],
                target_consonant_samples,
                sample_rate,
            );
            let join_samples = ((sample_rate as f64 * 0.004).round() as usize)
                .min(consonant.len())
                .min(vowel.len());
            let mut consonant = consonant;
            if join_samples > 0 {
                let consonant_start = consonant.len() - join_samples;
                for index in 0..join_samples {
                    let t = (index as f32 + 0.5) / join_samples as f32;
                    let fade_out = (t * std::f32::consts::FRAC_PI_2).cos();
                    let fade_in = (t * std::f32::consts::FRAC_PI_2).sin();
                    consonant[consonant_start + index] =
                        consonant[consonant_start + index] * fade_out + vowel[index] * fade_in;
                }
            }
            let mut combined = Vec::with_capacity(target_len);
            combined.extend_from_slice(&consonant);
            // Keep the requested duration exact. The overlap is an energy
            // smoothing pass on the consonant tail; the vowel timeline is
            // not shortened.
            combined.extend_from_slice(&vowel);
            combined.truncate(target_len);
            combined.resize(target_len, 0.0);
            return combined;
        }
        let fs = sample_rate as f64;
        let settings = runtime_config()
            .read()
            .map(|value| value.clone())
            .unwrap_or_default();
        let dio_options = world_rs::dio::initialize_dio_option();
        let mut dio_options = dio_options;
        dio_options.frame_period = settings.frame_period_ms.clamp(1.0, 20.0);
        dio_options.f0_floor = settings.f0_floor_hz.clamp(20.0, 2_000.0);
        dio_options.f0_ceil = settings
            .f0_ceil_hz
            .max(dio_options.f0_floor + 1.0)
            .min(4_000.0);
        let use_harvest = settings
            .f0_detection_method
            .to_ascii_lowercase()
            .contains("harvest");
        let method = settings.f0_detection_method.to_ascii_lowercase();
        let use_pyin = method.contains("pyin");
        let use_custom_f0 = use_pyin || method.contains("yin");
        let analysis = if use_pyin {
            let result = crate::dsp::pyin::PitchExtractor::extract_pitch_and_gci_with_range(
                &input_f32,
                sample_rate,
                settings.f0_floor_hz,
                settings.f0_ceil_hz,
            );
            let hop_ms = ((sample_rate as f64 / settings.f0_ceil_hz.max(1.0)) * 0.25 * 1000.0
                / sample_rate as f64)
                .max(1.0);
            let positions = (0..result.pitch_contour.len())
                .map(|index| index as f64 * hop_ms)
                .collect::<Vec<_>>();
            let f0 = result
                .pitch_contour
                .into_iter()
                .map(|period| {
                    if period > 0.0 {
                        sample_rate as f64 / f64::from(period)
                    } else {
                        0.0
                    }
                })
                .collect::<Vec<_>>();
            Some((positions, f0))
        } else if use_custom_f0 {
            let extractor = crate::dsp::world_analysis::WorldF0Extractor {
                min_f0: settings.f0_floor_hz as f32,
                max_f0: settings.f0_ceil_hz as f32,
                frame_period_ms: dio_options.frame_period as f32,
            };
            let (f0, voiced) = extractor.extract_f0(&input_f32, sample_rate);
            let positions = (0..f0.len())
                .map(|index| index as f64 * dio_options.frame_period)
                .collect::<Vec<_>>();
            Some((
                positions,
                f0.into_iter()
                    .zip(voiced)
                    .map(|(value, voiced)| if voiced { f64::from(value) } else { 0.0 })
                    .collect(),
            ))
        } else if use_harvest {
            let mut option = world_rs::harvest::initialize_harvest_option();
            option.f0_floor = dio_options.f0_floor;
            option.f0_ceil = dio_options.f0_ceil;
            option.frame_period = dio_options.frame_period;
            world_rs::harvest::harvest(&input, fs, &option)
                .ok()
                .map(|result| (result.temporal_positions, result.f0))
        } else {
            world_rs::dio::dio(&input, fs, &dio_options)
                .ok()
                .map(|result| (result.temporal_positions, result.f0))
        };
        let Some((temporal_positions, raw_f0)) = analysis else {
            return fallback_render(
                input_samples,
                sample_rate,
                offset_ms,
                source_consonant_ms,
                target_consonant_ms,
                cutoff_ms,
                target_duration_ms,
                target_pitch_freq,
                pitch_points,
                target_len,
                loop_start_ms,
                loop_end_ms,
                tail_start_ms,
            );
        };
        if raw_f0.is_empty() {
            return fallback_render(
                input_samples,
                sample_rate,
                offset_ms,
                source_consonant_ms,
                target_consonant_ms,
                cutoff_ms,
                target_duration_ms,
                target_pitch_freq,
                pitch_points,
                target_len,
                loop_start_ms,
                loop_end_ms,
                tail_start_ms,
            );
        }

        let mut f0 = if use_custom_f0 {
            raw_f0.clone()
        } else {
            let Ok(refined) = world_rs::stonemask::stone_mask(
                &input,
                input.len(),
                fs,
                &temporal_positions,
                &raw_f0,
                raw_f0.len(),
            ) else {
                return fallback_render(
                    input_samples,
                    sample_rate,
                    offset_ms,
                    source_consonant_ms,
                    target_consonant_ms,
                    cutoff_ms,
                    target_duration_ms,
                    target_pitch_freq,
                    pitch_points,
                    target_len,
                    loop_start_ms,
                    loop_end_ms,
                    tail_start_ms,
                );
            };
            refined
        };
        stabilize_world_f0(&mut f0, dio_options.f0_floor, dio_options.f0_ceil);
        // Keep the analysis/synthesis F0 in the source domain. WORLD's
        // spectral envelope is tied to that contour; pitch transposition is
        // applied after WORLD through the same pitch-synchronous stage used
        // by the renderer, which also handles arbitrary piano-roll bends.

        let mut option = world_rs::cheaptrick::initialize_cheaptrick_option(fs);
        option.q1 = match settings.formant_preservation_mode.as_str() {
            "True Envelope" => -0.05,
            "Desativado" | "Disabled" => 0.0,
            _ => -0.15,
        };
        let Ok(spectrogram) =
            world_rs::cheaptrick::cheaptrick(&input, fs, &temporal_positions, &f0, &option)
        else {
            return fallback_render(
                input_samples,
                sample_rate,
                offset_ms,
                source_consonant_ms,
                target_consonant_ms,
                cutoff_ms,
                target_duration_ms,
                target_pitch_freq,
                pitch_points,
                target_len,
                loop_start_ms,
                loop_end_ms,
                tail_start_ms,
            );
        };
        let fft_size = option.fft_size as usize;
        let bins = fft_size / 2 + 1;
        // D4C supplies aperiodicity per frame and frequency band. The user
        // preference is a voiced-noise floor blended into that measurement,
        // rather than a constant AP matrix (which sounds granular and erases
        // the natural difference between vowels and consonants).
        // straycat-rs uses the WORLD reference threshold 0.25 (rather than
        // the crate's generic 0.85 default). The lower threshold keeps the
        // periodic component of voiced material from being over-classified as
        // noise, which is a major source of the rough/granular character.
        let mut d4c_option = world_rs::d4c::initialize_d4c_option();
        d4c_option.threshold = 0.25;
        let measured_aperiodicity = world_rs::d4c::d4c(
            &input,
            fs,
            &temporal_positions,
            &f0,
            option.fft_size,
            &d4c_option,
        )
        .ok();
        let mut aperiodicity = f0
            .iter()
            .enumerate()
            .map(|(frame, value)| {
                let measured = measured_aperiodicity
                    .as_ref()
                    .and_then(|matrix| matrix.get(frame))
                    .filter(|row| row.len() == bins);
                match measured {
                    Some(row) => row
                        .iter()
                        .map(|ap| {
                            let floor = if *value > settings.f0_floor_hz {
                                settings.voiced_aperiodicity.clamp(0.0, 1.0) * 0.35
                            } else {
                                0.05
                            };
                            let measured = if ap.is_finite() { *ap } else { 0.0 };
                            measured.max(floor).clamp(0.0, 1.0)
                        })
                        .collect(),
                    None => {
                        let ap = if *value > settings.f0_floor_hz {
                            settings.voiced_aperiodicity.clamp(0.0, 1.0) * 0.35
                        } else {
                            0.05
                        };
                        vec![ap; bins]
                    }
                }
            })
            .collect::<Vec<_>>();
        // D4C is intentionally sensitive to consonant/noise changes, but a
        // one-frame jump inside a sustained vowel becomes audible as a
        // granular flutter after WORLD synthesis. Match the stable behaviour
        // of the reference resamplers with a conservative three-frame
        // temporal filter, restricted to voiced frames so fricatives keep
        // their natural noise boundary.
        if aperiodicity.len() >= 3 {
            let original = aperiodicity.clone();
            for frame in 1..aperiodicity.len() - 1 {
                if f0[frame] <= settings.f0_floor_hz
                    || f0[frame - 1] <= settings.f0_floor_hz
                    || f0[frame + 1] <= settings.f0_floor_hz
                {
                    continue;
                }
                for bin in 0..bins {
                    aperiodicity[frame][bin] = (original[frame - 1][bin] * 0.25
                        + original[frame][bin] * 0.5
                        + original[frame + 1][bin] * 0.25)
                        .clamp(0.0, 1.0);
                }
            }
        }
        // WORLD synthesis expects feature rows at a fixed frame period. For
        // short OTO regions stretched into long notes, increasing the frame
        // period makes the feature timeline end early and the remaining note
        // becomes silent. Resample the analysed features to the target frame
        // count instead, matching straycat-rs' render-time interpolation.
        let target_frames =
            ((target_duration_ms.max(1.0) / dio_options.frame_period).ceil() as usize + 1).max(2);
        let render_spectrogram = resample_feature_matrix(&spectrogram, target_frames, bins);
        let render_aperiodicity = resample_feature_matrix(&aperiodicity, target_frames, bins);
        let source_duration_ms = input_f32.len() as f64 * 1_000.0 / sample_rate as f64;
        let voiced_frames = f0
            .iter()
            .filter(|value| **value > dio_options.f0_floor)
            .count();
        let force_voiced_sustain = target_duration_ms > source_duration_ms * 1.25
            && voiced_frames >= 3
            && voiced_frames * 5 >= f0.len();
        let target_f0 = (0..target_frames)
            .map(|frame| {
                let source_position = if target_frames > 1 && f0.len() > 1 {
                    frame as f64 * (f0.len() - 1) as f64 / (target_frames - 1) as f64
                } else {
                    0.0
                };
                let source_frame = source_position.round() as usize;
                if !force_voiced_sustain && f0[source_frame.min(f0.len() - 1)] <= 0.0 {
                    return 0.0;
                }
                let time_ms = frame as f64 * dio_options.frame_period;
                let cents = crate::dsp::pitch_bend::PitchBendSolver::get_pitch_offset_cents(
                    time_ms,
                    pitch_points,
                );
                (target_pitch_freq.max(1.0) * 2.0f64.powf(cents / 1_200.0))
                    .clamp(dio_options.f0_floor, dio_options.f0_ceil)
            })
            .collect::<Vec<_>>();
        let output_len = target_len;
        let Ok(rendered) = world_rs::synthesis::synthesis(
            &target_f0,
            target_frames,
            &render_spectrogram,
            &render_aperiodicity,
            fft_size,
            dio_options.frame_period,
            fs,
            output_len,
        ) else {
            return crate::dsp::resize_preserving_pitch(&input_f32, target_len, sample_rate);
        };
        let rendered: Vec<f32> = rendered.into_iter().map(|sample| sample as f32).collect();
        if !rendered
            .iter()
            .any(|sample| sample.is_finite() && sample.abs() > 1e-5)
        {
            return fallback_render(
                input_samples,
                sample_rate,
                offset_ms,
                source_consonant_ms,
                target_consonant_ms,
                cutoff_ms,
                target_duration_ms,
                target_pitch_freq,
                pitch_points,
                target_len,
                loop_start_ms,
                loop_end_ms,
                tail_start_ms,
            );
        }
        let has_explicit_oto_sustain =
            loop_start_ms.is_some() || loop_end_ms.is_some() || tail_start_ms.is_some();
        let needs_pitch_correction = if target_pitch_freq.is_finite() && target_pitch_freq > 0.0 {
            let begin = rendered.len() / 8;
            let end = rendered.len().saturating_mul(7) / 8;
            if end.saturating_sub(begin) >= (sample_rate as usize / 10).max(64) {
                let measured = crate::dsp::Resampler::estimate_pitch_period(
                    &rendered[begin..end],
                    sample_rate,
                );
                measured > 0
                    && ((measured as f64 * target_pitch_freq / sample_rate as f64) - 1.0).abs()
                        > 0.10
            } else {
                false
            }
        } else {
            false
        };
        let output = if needs_pitch_correction || has_explicit_oto_sustain {
            // This is a guarded compatibility path for voices whose WORLD
            // analysis loses the fundamental, and the authoritative OTO
            // sustain path for entries that explicitly define loop/tail
            // coordinates. Notes without either condition remain pure WORLD
            // synthesis and avoid a second time-stretcher.
            crate::dsp::SolaResampler::render_sample(
                &rendered,
                sample_rate,
                0.0,
                0.0,
                0.0,
                0.0,
                target_duration_ms,
                target_pitch_freq,
                pitch_points,
                loop_start_ms,
                loop_end_ms,
                tail_start_ms,
            )
        } else {
            rendered
        };
        if output
            .iter()
            .any(|sample| sample.is_finite() && sample.abs() > 1e-5)
        {
            output
        } else {
            fallback_render(
                input_samples,
                sample_rate,
                offset_ms,
                source_consonant_ms,
                target_consonant_ms,
                cutoff_ms,
                target_duration_ms,
                target_pitch_freq,
                pitch_points,
                target_len,
                loop_start_ms,
                loop_end_ms,
                tail_start_ms,
            )
        }
    }
}

fn resample_feature_matrix(
    source: &[Vec<f64>],
    target_frames: usize,
    bins: usize,
) -> Vec<Vec<f64>> {
    if source.is_empty() || target_frames == 0 {
        return Vec::new();
    }
    if source.len() == 1 {
        let row = source[0].clone();
        return vec![row; target_frames];
    }
    (0..target_frames)
        .map(|frame| {
            let position = if target_frames > 1 {
                frame as f64 * (source.len() - 1) as f64 / (target_frames - 1) as f64
            } else {
                0.0
            };
            let left = position.floor() as usize;
            let right = (left + 1).min(source.len() - 1);
            let fraction = position - left as f64;
            (0..bins)
                .map(|bin| {
                    let a = source[left].get(bin).copied().unwrap_or(0.0);
                    let b = source[right].get(bin).copied().unwrap_or(a);
                    a + (b - a) * fraction
                })
                .collect()
        })
        .collect()
}

/// Remove isolated octave decisions from the WORLD contour.  A single DIO or
/// Harvest octave error is especially damaging because it is consumed by both
/// CheapTrick and synthesis.  Only replace a frame when both neighbours are
/// voiced and agree within a semitone-ish tolerance; continuous pitch bends
/// and vibrato therefore remain untouched.
fn stabilize_world_f0(f0: &mut [f64], floor: f64, ceil: f64) {
    if f0.len() < 3 {
        return;
    }
    let original = f0.to_vec();
    for index in 1..f0.len() - 1 {
        let previous = original[index - 1];
        let current = original[index];
        let next = original[index + 1];
        if previous < floor || current < floor || next < floor {
            continue;
        }
        let neighbour_ratio = (previous / next).max(next / previous);
        if neighbour_ratio > 1.35 {
            continue;
        }
        let current_ratio = (current / previous).max(previous / current);
        if current_ratio > 1.65 {
            f0[index] = ((previous + next) * 0.5).clamp(floor, ceil);
        }
    }
}

fn fallback_render(
    input: &[f32],
    sample_rate: u32,
    offset_ms: f64,
    source_consonant_ms: f64,
    target_consonant_ms: f64,
    cutoff_ms: f64,
    duration_ms: f64,
    pitch_hz: f64,
    points: &[UPitchBendPoint],
    target_len: usize,
    loop_start_ms: Option<f64>,
    loop_end_ms: Option<f64>,
    tail_start_ms: Option<f64>,
) -> Vec<f32> {
    let sanitized;
    let input = if input.iter().all(|sample| sample.is_finite()) {
        input
    } else {
        sanitized = input
            .iter()
            .map(|sample| if sample.is_finite() { *sample } else { 0.0 })
            .collect::<Vec<_>>();
        sanitized.as_slice()
    };
    let output = crate::dsp::SolaResampler::render_sample(
        input,
        sample_rate,
        offset_ms,
        source_consonant_ms,
        target_consonant_ms,
        cutoff_ms,
        duration_ms,
        pitch_hz,
        points,
        loop_start_ms,
        loop_end_ms,
        tail_start_ms,
    );
    let output_has_signal = output
        .iter()
        .any(|sample| sample.is_finite() && sample.abs() > 1e-5);
    if output_has_signal {
        if output.len() == target_len {
            output
        } else {
            crate::dsp::resize_preserving_pitch(&output, target_len, sample_rate)
        }
    } else {
        let (start, end) =
            crate::dsp::oto_source_bounds(input.len(), sample_rate, offset_ms, cutoff_ms);
        crate::dsp::resize_preserving_pitch(
            &input[start.min(input.len())..end.min(input.len())],
            target_len,
            sample_rate,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{stabilize_world_f0, WorldResampler};

    #[test]
    fn world_f0_stabilizer_repairs_an_isolated_octave_without_flattening_bend() {
        let mut contour = vec![220.0, 440.0, 222.0, 224.0, 226.0];
        stabilize_world_f0(&mut contour, 40.0, 1_400.0);
        assert!((contour[1] - 221.0).abs() < 2.0);

        let mut bend = vec![220.0, 230.0, 240.0, 250.0, 260.0];
        stabilize_world_f0(&mut bend, 40.0, 1_400.0);
        assert_eq!(bend, vec![220.0, 230.0, 240.0, 250.0, 260.0]);
    }

    #[test]
    fn world_pipeline_returns_requested_duration_and_finite_samples() {
        let rate = 16_000;
        let input = (0..rate)
            .map(|index| {
                let phase = std::f64::consts::TAU * 220.0 * index as f64 / rate as f64;
                (phase.sin() * 0.25) as f32
            })
            .collect::<Vec<_>>();
        let output =
            WorldResampler::render_sample(&input, rate, 0.0, 0.0, 0.0, 700.0, 220.0, &[], 0.0);
        assert_eq!(output.len(), 11_200);
        assert!(output.iter().all(|sample| sample.is_finite()));
        assert!(output.iter().any(|sample| sample.abs() > 0.01));
    }

    #[test]
    fn world_supports_common_audio_sample_rates() {
        for rate in [44_100_u32, 48_000_u32] {
            let input = (0..rate)
                .map(|index| {
                    (std::f64::consts::TAU * 220.0 * index as f64 / rate as f64).sin() as f32
                })
                .collect::<Vec<_>>();
            let output =
                WorldResampler::render_sample(&input, rate, 0.0, 0.0, 0.0, 600.0, 220.0, &[], 0.0);
            assert_eq!(output.len(), (rate as f64 * 0.6) as usize);
            assert!(output.iter().all(|sample| sample.is_finite()));
            assert!(output.iter().any(|sample| sample.abs() > 0.01));
        }
    }

    #[test]
    fn world_sanitizes_nonfinite_source_samples() {
        let rate = 16_000_u32;
        let mut input = (0..rate as usize)
            .map(|index| (std::f64::consts::TAU * 220.0 * index as f64 / rate as f64).sin() as f32)
            .collect::<Vec<_>>();
        input[rate as usize / 3] = f32::NAN;
        input[rate as usize / 2] = f32::INFINITY;
        let output =
            WorldResampler::render_sample(&input, rate, 0.0, 0.0, 0.0, 500.0, 220.0, &[], 0.0);
        assert!(output.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn world_tracks_requested_octave() {
        let rate = 16_000;
        let input = (0..rate)
            .map(|index| (std::f64::consts::TAU * 220.0 * index as f64 / rate as f64).sin() as f32)
            .collect::<Vec<_>>();
        let low =
            WorldResampler::render_sample(&input, rate, 0.0, 0.0, 0.0, 700.0, 220.0, &[], 0.0);
        let high =
            WorldResampler::render_sample(&input, rate, 0.0, 0.0, 0.0, 700.0, 440.0, &[], 0.0);
        let low_period = crate::dsp::Resampler::estimate_pitch_period(&low[2_000..8_000], rate);
        let high_period = crate::dsp::Resampler::estimate_pitch_period(&high[2_000..8_000], rate);
        assert!(
            (low_period as f64 - rate as f64 / 220.0).abs() < 8.0,
            "low period={low_period}"
        );
        assert!(
            (high_period as f64 - rate as f64 / 440.0).abs() < 8.0,
            "high period={high_period}"
        );
    }

    #[test]
    fn world_respects_oto_offset_and_cutoff() {
        let input = (0..16_000)
            .map(|i| (i as f32 / 16_000.0).sin())
            .collect::<Vec<_>>();
        let output = WorldResampler::render_sample(
            &input,
            16_000,
            100.0,
            20.0,
            20.0,
            300.0,
            220.0,
            &[],
            100.0,
        );
        assert_eq!(output.len(), 4_800);
        assert!(output.iter().all(|sample| sample.is_finite()));
    }

    #[test]
    fn world_preserves_oto_consonant_handoff_without_a_silent_gap() {
        let rate = 16_000;
        let input = (0..rate)
            .map(|index| {
                let time = index as f32 / rate as f32;
                if time < 0.08 {
                    // Synthetic consonant attack: deterministic broadband
                    // burst before the voiced vowel begins.
                    (((index * 37) % 101) as f32 / 50.0 - 1.0) * 0.35
                } else {
                    (std::f32::consts::TAU * 220.0 * time).sin() * 0.25
                }
            })
            .collect::<Vec<_>>();
        let output =
            WorldResampler::render_sample(&input, rate, 20.0, 60.0, 90.0, 500.0, 220.0, &[], 0.0);
        assert_eq!(output.len(), 8_000);
        assert!(output.iter().all(|sample| sample.is_finite()));
        let attack_peak = output[..(rate as usize * 90 / 1_000)]
            .iter()
            .map(|sample| sample.abs())
            .fold(0.0, f32::max);
        assert!(attack_peak > 0.05, "oto consonant attack was lost");
        assert!(output
            .iter()
            .skip(rate as usize * 120 / 1_000)
            .any(|sample| sample.abs() > 0.05));
    }

    #[test]
    fn world_uses_oto_loop_coordinates_for_long_vowels() {
        let rate = 16_000;
        let input = (0..rate)
            .map(|index| (std::f32::consts::TAU * 180.0 * index as f32 / rate as f32).sin() * 0.25)
            .collect::<Vec<_>>();
        let output = WorldResampler::render_sample_with_oto(
            &input,
            rate,
            0.0,
            80.0,
            80.0,
            1_200.0,
            180.0,
            &[],
            0.0,
            Some(260.0),
            Some(600.0),
            Some(760.0),
        );
        assert_eq!(output.len(), 19_200);
        assert!(output.iter().all(|sample| sample.is_finite()));
        let tail_rms = output[rate as usize * 900 / 1_000..]
            .iter()
            .map(|sample| sample * sample)
            .sum::<f32>()
            / (output.len() - rate as usize * 900 / 1_000) as f32;
        assert!(tail_rms.sqrt() > 0.01, "looped vowel tail became silent");
    }

    #[test]
    fn short_vcv_oto_vowel_sustains_through_a_long_note() {
        let rate = 44_100_u32;
        let input = (0..(rate as usize * 400 / 1_000))
            .map(|index| {
                let time = index as f32 / rate as f32;
                if time < 0.25 {
                    (((index * 31) % 97) as f32 / 48.0 - 1.0) * 0.12
                } else {
                    (std::f32::consts::TAU * 196.0 * time).sin() * 0.25
                }
            })
            .collect::<Vec<_>>();
        let output = WorldResampler::render_sample(
            &input,
            rate,
            0.0,
            250.0,
            250.0,
            2_750.0,
            196.0,
            &[],
            -400.0,
        );
        assert_eq!(output.len(), (rate as f64 * 2.75) as usize);
        let late_peak = output[rate as usize * 2..]
            .iter()
            .map(|sample| sample.abs())
            .fold(0.0, f32::max);
        assert!(late_peak > 0.02, "long VCV sustain became silent");
    }
}
