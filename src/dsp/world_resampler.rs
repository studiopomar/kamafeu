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

#[derive(Debug, Clone, Copy, Default)]
struct TainahFlags {
    gender: f64,
    tone_cents: f64,
    breathiness: f64,
    high_cut: f64,
}

impl TainahFlags {
    fn parse(flags: &str, trace: &mut WorldRenderTrace) -> Self {
        let mut parsed = Self::default();
        for token in crate::drivers::resampler_driver::parse_utau_flags(flags) {
            let value = token.num_val.unwrap_or(0.0);
            match token.key.as_str() {
                "g" => parsed.gender = value.clamp(-100.0, 100.0),
                "t" => parsed.tone_cents = value.clamp(-100.0, 100.0) * 10.0,
                "B" => parsed.breathiness = value.clamp(0.0, 100.0),
                "H" => parsed.high_cut = value.clamp(0.0, 99.0),
                _ => trace.adjustment(
                    &format!("Tainah ignored unsupported flag {}", token.key),
                    "kamafeu.world-rs.flags",
                ),
            }
        }
        parsed
    }
}

fn apply_spectral_flags(matrix: &mut [Vec<f64>], sample_rate: u32, flags: &TainahFlags) {
    if matrix.is_empty() || matrix[0].is_empty() {
        return;
    }
    let bins = matrix[0].len();
    let nyquist = sample_rate as f64 * 0.5;
    let ratio = 2.0f64.powf(flags.gender / 200.0);
    let high_cut_hz = if flags.high_cut <= 0.0 || nyquist <= 1_000.0 {
        nyquist
    } else {
        nyquist * (1_000.0 / nyquist).powf(flags.high_cut / 99.0)
    };
    for row in matrix {
        let original = row.clone();
        for (bin, value) in row.iter_mut().enumerate() {
            let frequency = bin as f64 * nyquist / (bins - 1).max(1) as f64;
            if frequency > high_cut_hz {
                *value = 0.0;
                continue;
            }
            if flags.gender.abs() > f64::EPSILON {
                let source_bin = (bin as f64 / ratio).clamp(0.0, (bins - 1) as f64);
                let lower = source_bin.floor() as usize;
                let upper = (lower + 1).min(bins - 1);
                let fraction = source_bin - lower as f64;
                *value = original[lower] * (1.0 - fraction) + original[upper] * fraction;
            }
        }
    }
}

fn apply_breathiness(matrix: &mut [Vec<f64>], amount: f64) {
    if amount <= 0.0 {
        return;
    }
    let amount = amount.clamp(0.0, 100.0) / 100.0;
    for row in matrix {
        for value in row {
            *value = (*value + (0.95 - *value) * amount).clamp(0.0, 1.0);
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorldRenderTrace {
    /// Distinct DSP paths that contributed to the returned samples.
    pub stages: Vec<String>,
    pub fallback_reasons: Vec<String>,
    pub adjustments: Vec<String>,
}

impl WorldRenderTrace {
    fn stage(&mut self, stage: &str) {
        if !self.stages.iter().any(|existing| existing == stage) {
            self.stages.push(stage.to_string());
        }
    }

    fn fallback(&mut self, reason: &str, stage: &str) {
        self.stage(stage);
        self.fallback_reasons.push(reason.to_string());
    }

    fn adjustment(&mut self, reason: &str, stage: &str) {
        self.stage(stage);
        self.adjustments.push(reason.to_string());
    }

    pub fn effective_backend_id(&self) -> String {
        if self.stages.is_empty() {
            "kamafeu.world-rs".into()
        } else {
            self.stages.join("+")
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldRenderResult {
    pub samples: Vec<f32>,
    pub trace: WorldRenderTrace,
}

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
        Self::render_sample_with_oto_traced(
            input_samples,
            sample_rate,
            offset_ms,
            source_consonant_ms,
            target_consonant_ms,
            target_duration_ms,
            target_pitch_freq,
            pitch_points,
            cutoff_ms,
            loop_start_ms,
            loop_end_ms,
            tail_start_ms,
        )
        .samples
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_sample_with_oto_traced(
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
    ) -> WorldRenderResult {
        Self::render_sample_with_oto_traced_flags(
            input_samples,
            sample_rate,
            offset_ms,
            source_consonant_ms,
            target_consonant_ms,
            target_duration_ms,
            target_pitch_freq,
            pitch_points,
            cutoff_ms,
            loop_start_ms,
            loop_end_ms,
            tail_start_ms,
            "",
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_sample_with_oto_traced_flags(
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
        flags: &str,
    ) -> WorldRenderResult {
        let mut trace = WorldRenderTrace::default();
        let samples = Self::render_sample_with_oto_inner(
            input_samples,
            sample_rate,
            offset_ms,
            source_consonant_ms,
            target_consonant_ms,
            target_duration_ms,
            target_pitch_freq,
            pitch_points,
            cutoff_ms,
            loop_start_ms,
            loop_end_ms,
            tail_start_ms,
            flags,
            &mut trace,
        );
        WorldRenderResult { samples, trace }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_sample_with_oto_inner(
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
        flags: &str,
        trace: &mut WorldRenderTrace,
    ) -> Vec<f32> {
        let target_len =
            ((target_duration_ms.max(0.0) / 1000.0) * sample_rate as f64).round() as usize;
        if target_len == 0 || input_samples.is_empty() || sample_rate == 0 {
            trace.fallback("invalid or empty WORLD input", "kamafeu.silence");
            return vec![0.0; target_len];
        }

        let (start, end) =
            crate::dsp::oto_source_bounds(input_samples.len(), sample_rate, offset_ms, cutoff_ms);
        if start >= end || end - start < 64 {
            trace.fallback("empty or too short WORLD source region", "kamafeu.silence");
            return vec![0.0; target_len];
        }
        if input_samples[start..end]
            .iter()
            .any(|sample| !sample.is_finite())
        {
            trace.adjustment(
                "non-finite source samples were replaced with zero before WORLD analysis",
                "kamafeu.input-sanitize",
            );
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
            let vowel = Self::render_sample_with_oto_inner(
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
                    .map(|value| value - offset_ms - source_consonant_ms)
                    .filter(|value| *value >= 0.0),
                loop_end_ms
                    .map(|value| value - offset_ms - source_consonant_ms)
                    .filter(|value| *value >= 0.0),
                tail_start_ms
                    .map(|value| value - offset_ms - source_consonant_ms)
                    .filter(|value| *value >= 0.0),
                flags,
                trace,
            );
            trace.stage("kamafeu.resize-preserving-pitch");
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
        let parsed_flags = TainahFlags::parse(flags, trace);
        let effective_pitch_freq =
            target_pitch_freq.max(1.0) * 2.0f64.powf(parsed_flags.tone_cents / 1_200.0);
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
            return fallback_render_traced(
                input_samples,
                sample_rate,
                offset_ms,
                source_consonant_ms,
                target_consonant_ms,
                cutoff_ms,
                target_duration_ms,
                effective_pitch_freq,
                pitch_points,
                target_len,
                loop_start_ms,
                loop_end_ms,
                tail_start_ms,
                "F0 analysis unavailable",
                trace,
            );
        };
        if raw_f0.is_empty() {
            return fallback_render_traced(
                input_samples,
                sample_rate,
                offset_ms,
                source_consonant_ms,
                target_consonant_ms,
                cutoff_ms,
                target_duration_ms,
                effective_pitch_freq,
                pitch_points,
                target_len,
                loop_start_ms,
                loop_end_ms,
                tail_start_ms,
                "F0 analysis returned no frames",
                trace,
            );
        }
        // world-rs follows WORLD and returns seconds for DIO/Harvest. The
        // local pYIN/YIN extractors above construct their positions in ms.
        let temporal_positions_ms = if use_pyin || use_custom_f0 {
            temporal_positions.clone()
        } else {
            temporal_positions
                .iter()
                .map(|time_seconds| time_seconds * 1_000.0)
                .collect::<Vec<_>>()
        };

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
                return fallback_render_traced(
                    input_samples,
                    sample_rate,
                    offset_ms,
                    source_consonant_ms,
                    target_consonant_ms,
                    cutoff_ms,
                    target_duration_ms,
                    effective_pitch_freq,
                    pitch_points,
                    target_len,
                    loop_start_ms,
                    loop_end_ms,
                    tail_start_ms,
                    "StoneMask refinement failed",
                    trace,
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
            return fallback_render_traced(
                input_samples,
                sample_rate,
                offset_ms,
                source_consonant_ms,
                target_consonant_ms,
                cutoff_ms,
                target_duration_ms,
                effective_pitch_freq,
                pitch_points,
                target_len,
                loop_start_ms,
                loop_end_ms,
                tail_start_ms,
                "CheapTrick spectral analysis failed",
                trace,
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
        let source_duration_ms = input_f32.len() as f64 * 1_000.0 / sample_rate as f64;
        let target_frames =
            ((target_duration_ms.max(1.0) / dio_options.frame_period).ceil() as usize + 1).max(2);
        let source_positions = stretch_source_positions(
            target_frames,
            target_duration_ms,
            source_duration_ms,
            loop_start_ms.map(|value| value - offset_ms),
            loop_end_ms.map(|value| value - offset_ms),
            tail_start_ms.map(|value| value - offset_ms),
        );
        if has_valid_oto_stretch_markers(
            source_duration_ms,
            loop_start_ms.map(|value| value - offset_ms),
            loop_end_ms.map(|value| value - offset_ms),
            tail_start_ms.map(|value| value - offset_ms),
        ) {
            trace.adjustment(
                "Tainah stretch preserved the OTO attack and tail while stretching the voiced sustain",
                "kamafeu.world-rs",
            );
        }
        let mut render_spectrogram = resample_feature_matrix_at_positions(
            &spectrogram,
            &temporal_positions_ms,
            &source_positions,
            bins,
        );
        apply_spectral_flags(&mut render_spectrogram, sample_rate, &parsed_flags);
        if parsed_flags.gender.abs() > f64::EPSILON || parsed_flags.high_cut > 0.0 {
            trace.adjustment(
                &format!(
                    "Tainah spectral flags g{} H{}",
                    parsed_flags.gender as i32, parsed_flags.high_cut as i32
                ),
                "kamafeu.world-rs.flags",
            );
        }
        let render_aperiodicity = resample_feature_matrix_at_positions(
            &aperiodicity,
            &temporal_positions_ms,
            &source_positions,
            bins,
        );
        let mut render_aperiodicity = render_aperiodicity;
        if parsed_flags.breathiness > 0.0 {
            apply_breathiness(&mut render_aperiodicity, parsed_flags.breathiness);
            trace.adjustment(
                format!("Tainah B{} breathiness", parsed_flags.breathiness as i32).as_str(),
                "kamafeu.world-rs.flags",
            );
        }
        if parsed_flags.tone_cents != 0.0 {
            trace.adjustment(
                &format!(
                    "Tainah t{} pitch offset in cents",
                    (parsed_flags.tone_cents / 10.0) as i32
                ),
                "kamafeu.world-rs.flags",
            );
        }
        let voiced_frames = f0
            .iter()
            .filter(|value| **value > dio_options.f0_floor)
            .count();
        let force_voiced_sustain = target_duration_ms > source_duration_ms * 1.25
            && voiced_frames >= 3
            && voiced_frames * 5 >= f0.len();
        let sustain_start_ms = loop_start_ms
            .map(|value| (value - offset_ms).max(0.0))
            .unwrap_or(source_consonant_ms.max(0.0));
        let sustain_end_ms = loop_end_ms
            .map(|value| (value - offset_ms).max(0.0))
            .or_else(|| tail_start_ms.map(|value| (value - offset_ms).max(0.0)))
            .unwrap_or(source_duration_ms);
        let target_f0 = (0..target_frames)
            .map(|frame| {
                let source_time_ms = source_positions[frame.min(source_positions.len() - 1)];
                let source_f0 = sample_scalar_at_time(&f0, &temporal_positions_ms, source_time_ms);
                let voiced_sustain = force_voiced_sustain
                    && source_time_ms >= sustain_start_ms
                    && source_time_ms <= sustain_end_ms;
                if source_f0 <= 0.0 && !voiced_sustain {
                    return 0.0;
                }
                let time_ms = frame as f64 * dio_options.frame_period;
                let cents = crate::dsp::pitch_bend::PitchBendSolver::get_pitch_offset_cents(
                    time_ms,
                    pitch_points,
                );
                (effective_pitch_freq * 2.0f64.powf(cents / 1_200.0))
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
            trace.fallback(
                "WORLD synthesis failed; used pitch-preserving resize",
                "kamafeu.resize-preserving-pitch",
            );
            return crate::dsp::resize_preserving_pitch(&input_f32, target_len, sample_rate);
        };
        let rendered: Vec<f32> = rendered.into_iter().map(|sample| sample as f32).collect();
        if !rendered
            .iter()
            .any(|sample| sample.is_finite() && sample.abs() > 1e-5)
        {
            return fallback_render_traced(
                input_samples,
                sample_rate,
                offset_ms,
                source_consonant_ms,
                target_consonant_ms,
                cutoff_ms,
                target_duration_ms,
                effective_pitch_freq,
                pitch_points,
                target_len,
                loop_start_ms,
                loop_end_ms,
                tail_start_ms,
                "WORLD synthesis returned no audible samples",
                trace,
            );
        }
        let needs_pitch_correction = if effective_pitch_freq.is_finite()
            && effective_pitch_freq > 0.0
        {
            let begin = rendered.len() / 8;
            let end = rendered.len().saturating_mul(7) / 8;
            if end.saturating_sub(begin) >= (sample_rate as usize / 10).max(64) {
                let measured = crate::dsp::Resampler::estimate_pitch_period(
                    &rendered[begin..end],
                    sample_rate,
                );
                measured > 0
                    && ((measured as f64 * effective_pitch_freq / sample_rate as f64) - 1.0).abs()
                        > 0.10
            } else {
                false
            }
        } else {
            false
        };
        let output = if needs_pitch_correction {
            // OTO attack, sustain, and tail coordinates have already shaped
            // WORLD's spectral-frame time map. A second time-stretcher here
            // would stretch an already stretched render and misapply the OTO
            // coordinates to output time instead of source time.
            trace.stage("kamafeu.world-rs");
            trace.adjustment("SOLA applied for pitch correction", "kamafeu.sola");
            crate::dsp::SolaResampler::render_sample(
                &rendered,
                sample_rate,
                0.0,
                0.0,
                0.0,
                0.0,
                target_duration_ms,
                effective_pitch_freq,
                pitch_points,
                None,
                None,
                None,
            )
        } else {
            rendered
        };
        if output
            .iter()
            .any(|sample| sample.is_finite() && sample.abs() > 1e-5)
        {
            if !needs_pitch_correction {
                trace.stage("kamafeu.world-rs");
            }
            output
        } else {
            fallback_render_traced(
                input_samples,
                sample_rate,
                offset_ms,
                source_consonant_ms,
                target_consonant_ms,
                cutoff_ms,
                target_duration_ms,
                effective_pitch_freq,
                pitch_points,
                target_len,
                loop_start_ms,
                loop_end_ms,
                tail_start_ms,
                "WORLD and its guarded SOLA stage returned no audible samples",
                trace,
            )
        }
    }
}

/// Maps the source forward in time, preserving the recorded attack and tail
/// while stretching the sustain interval between OTO loop markers.
fn stretch_source_positions(
    target_frames: usize,
    target_duration_ms: f64,
    source_duration_ms: f64,
    loop_start_ms: Option<f64>,
    loop_end_ms: Option<f64>,
    tail_start_ms: Option<f64>,
) -> Vec<f64> {
    if target_frames == 0 || source_duration_ms <= 0.0 {
        return vec![0.0; target_frames];
    }
    let source_duration_ms = source_duration_ms.max(0.0);
    let target_duration_ms = target_duration_ms.max(0.0);
    let valid_markers = match (loop_start_ms, loop_end_ms) {
        (Some(start), Some(end)) => {
            start.is_finite()
                && end.is_finite()
                && start >= 0.0
                && end > start
                && end <= source_duration_ms
        }
        _ => false,
    };
    let tail_mark = tail_start_ms.filter(|tail| {
        tail.is_finite()
            && valid_markers
            && *tail >= loop_end_ms.unwrap_or(f64::INFINITY)
            && *tail <= source_duration_ms
    });

    (0..target_frames)
        .map(|frame| {
            let target_time_ms = if target_frames > 1 {
                frame as f64 * target_duration_ms / (target_frames - 1) as f64
            } else {
                0.0
            };
            if !valid_markers {
                return target_time_ms * source_duration_ms / target_duration_ms.max(f64::EPSILON);
            }

            let loop_start = loop_start_ms.unwrap().clamp(0.0, source_duration_ms);
            let loop_end = loop_end_ms.unwrap().clamp(loop_start, source_duration_ms);
            let tail_start = tail_mark.unwrap_or(source_duration_ms);
            let attack_len = loop_start;
            let tail_len = source_duration_ms - tail_start;
            let preserved_len = attack_len + tail_len;
            let preserve_scale = if target_duration_ms < preserved_len && preserved_len > 0.0 {
                target_duration_ms / preserved_len
            } else {
                1.0
            };
            let target_attack_len = attack_len * preserve_scale;
            let target_tail_len = tail_len * preserve_scale;
            let target_tail_start = target_duration_ms - target_tail_len;

            if target_time_ms < target_attack_len {
                target_time_ms / preserve_scale.max(f64::EPSILON)
            } else if target_time_ms >= target_tail_start {
                tail_start + (target_time_ms - target_tail_start) / preserve_scale.max(f64::EPSILON)
            } else {
                let target_sustain_len = (target_tail_start - target_attack_len).max(0.0);
                if target_sustain_len <= f64::EPSILON {
                    loop_start
                } else {
                    loop_start
                        + (target_time_ms - target_attack_len).max(0.0) / target_sustain_len
                            * (loop_end - loop_start)
                }
            }
            .clamp(0.0, source_duration_ms)
        })
        .collect()
}

fn has_valid_oto_stretch_markers(
    source_duration_ms: f64,
    loop_start_ms: Option<f64>,
    loop_end_ms: Option<f64>,
    tail_start_ms: Option<f64>,
) -> bool {
    let (Some(start), Some(end)) = (loop_start_ms, loop_end_ms) else {
        return false;
    };
    start.is_finite()
        && end.is_finite()
        && start >= 0.0
        && end > start
        && end <= source_duration_ms
        && tail_start_ms
            .is_none_or(|tail| tail.is_finite() && tail >= end && tail <= source_duration_ms)
}

fn sample_scalar_at_time(values: &[f64], positions_ms: &[f64], time_ms: f64) -> f64 {
    let count = values.len().min(positions_ms.len());
    if values.is_empty() {
        return 0.0;
    }
    if count < 2 {
        return values[0];
    }
    let upper = positions_ms[..count].partition_point(|position| *position < time_ms);
    if upper == 0 {
        return values[0];
    }
    if upper >= count {
        return values[count - 1];
    }
    let left = upper - 1;
    let right = upper.min(values.len() - 1);
    let span = positions_ms[upper] - positions_ms[left];
    let t = if span.is_finite() && span > f64::EPSILON {
        ((time_ms - positions_ms[left]) / span).clamp(0.0, 1.0)
    } else {
        0.0
    };
    values[left] * (1.0 - t) + values[right] * t
}

fn resample_feature_matrix_at_positions(
    source: &[Vec<f64>],
    source_positions_ms: &[f64],
    target_positions_ms: &[f64],
    bins: usize,
) -> Vec<Vec<f64>> {
    if source.is_empty() || target_positions_ms.is_empty() {
        return Vec::new();
    }
    if source.len() == 1 {
        let row = source[0].clone();
        return vec![row; target_positions_ms.len()];
    }
    target_positions_ms
        .iter()
        .map(|&time_ms| {
            let upper = source_positions_ms.partition_point(|position| *position < time_ms);
            let left = upper.saturating_sub(1).min(source.len() - 1);
            let right = upper.min(source.len() - 1);
            let span = source_positions_ms
                .get(right)
                .zip(source_positions_ms.get(left))
                .map(|(right, left)| right - left)
                .unwrap_or(0.0);
            let fraction = if span.is_finite() && span > f64::EPSILON {
                ((time_ms - source_positions_ms[left]) / span).clamp(0.0, 1.0)
            } else {
                0.0
            };
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
    trace: &mut WorldRenderTrace,
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
            trace.adjustment(
                "SOLA output resized to the requested duration",
                "kamafeu.resize-preserving-pitch",
            );
            crate::dsp::resize_preserving_pitch(&output, target_len, sample_rate)
        }
    } else {
        trace.fallback(
            "SOLA returned no audible samples; source region was resized",
            "kamafeu.resize-preserving-pitch",
        );
        let (start, end) =
            crate::dsp::oto_source_bounds(input.len(), sample_rate, offset_ms, cutoff_ms);
        crate::dsp::resize_preserving_pitch(
            &input[start.min(input.len())..end.min(input.len())],
            target_len,
            sample_rate,
        )
    }
}

#[allow(clippy::too_many_arguments)]
fn fallback_render_traced(
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
    reason: &str,
    trace: &mut WorldRenderTrace,
) -> Vec<f32> {
    trace.fallback(reason, "kamafeu.sola");
    fallback_render(
        input,
        sample_rate,
        offset_ms,
        source_consonant_ms,
        target_consonant_ms,
        cutoff_ms,
        duration_ms,
        pitch_hz,
        points,
        target_len,
        loop_start_ms,
        loop_end_ms,
        tail_start_ms,
        trace,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        apply_breathiness, apply_spectral_flags, has_valid_oto_stretch_markers, stabilize_world_f0,
        stretch_source_positions, TainahFlags, WorldRenderTrace, WorldResampler,
    };

    #[test]
    fn tainah_flags_are_case_sensitive_and_clamped_to_documented_ranges() {
        let mut trace = WorldRenderTrace::default();
        let flags = TainahFlags::parse("g140t-120B130H120h20", &mut trace);
        assert_eq!(flags.gender, 100.0);
        assert_eq!(flags.tone_cents, -1_000.0);
        assert_eq!(flags.breathiness, 100.0);
        assert_eq!(flags.high_cut, 99.0);
        assert!(trace.adjustments.iter().any(|item| item.contains("h")));
    }

    #[test]
    fn spectral_gender_shift_and_high_cut_change_only_the_spectral_envelope() {
        let source = vec![vec![0.0, 0.2, 0.4, 0.6, 0.8]];
        let mut shifted = source.clone();
        apply_spectral_flags(
            &mut shifted,
            8_000,
            &TainahFlags {
                gender: 100.0,
                ..Default::default()
            },
        );
        assert_ne!(shifted, source);

        let mut filtered = source.clone();
        apply_spectral_flags(
            &mut filtered,
            8_000,
            &TainahFlags {
                high_cut: 99.0,
                ..Default::default()
            },
        );
        assert_eq!(filtered[0][0], source[0][0]);
        assert_eq!(filtered[0][4], 0.0);
    }

    #[test]
    fn breathiness_interpolates_aperiodicity_to_the_documented_target() {
        let mut ap = vec![vec![0.2, 0.6]];
        apply_breathiness(&mut ap, 50.0);
        assert!((ap[0][0] - 0.575).abs() < 1e-12);
        assert!((ap[0][1] - 0.775).abs() < 1e-12);
        apply_breathiness(&mut ap, 100.0);
        assert_eq!(ap[0], vec![0.95, 0.95]);
    }

    #[test]
    fn tainah_stretch_keeps_attack_and_release_anchors() {
        let positions =
            stretch_source_positions(21, 200.0, 100.0, Some(20.0), Some(80.0), Some(90.0));
        assert!((positions[2] - 20.0).abs() < 1e-9);
        assert!((positions[19] - 90.0).abs() < 1e-9);
        assert!((positions[20] - 100.0).abs() < 1e-9);
        assert!(positions.windows(2).all(|pair| pair[1] >= pair[0]));
        assert!(has_valid_oto_stretch_markers(
            100.0,
            Some(20.0),
            Some(80.0),
            Some(90.0)
        ));
        assert!(!has_valid_oto_stretch_markers(
            100.0,
            Some(20.0),
            Some(80.0),
            Some(70.0)
        ));
    }

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
    fn neutral_flags_are_pcm_identical_and_t_flag_shifts_the_rendered_pitch() {
        let rate = 16_000;
        let input = (0..rate)
            .map(|index| {
                (std::f64::consts::TAU * 220.0 * index as f64 / rate as f64).sin() as f32 * 0.25
            })
            .collect::<Vec<_>>();
        let common = (
            &input,
            rate,
            0.0,
            0.0,
            0.0,
            260.0,
            220.0,
            &[][..],
            0.0,
            None,
            None,
            None,
        );
        let baseline = WorldResampler::render_sample_with_oto_traced(
            common.0, common.1, common.2, common.3, common.4, common.5, common.6, common.7,
            common.8, common.9, common.10, common.11,
        );
        let neutral = WorldResampler::render_sample_with_oto_traced_flags(
            common.0, common.1, common.2, common.3, common.4, common.5, common.6, common.7,
            common.8, common.9, common.10, common.11, "g0t0B0H0",
        );
        assert_eq!(baseline.samples, neutral.samples);

        let shifted = WorldResampler::render_sample_with_oto_traced_flags(
            common.0, common.1, common.2, common.3, common.4, common.5, common.6, common.7,
            common.8, common.9, common.10, common.11, "t12",
        );
        assert_ne!(baseline.samples, shifted.samples);
        assert!(shifted
            .trace
            .adjustments
            .iter()
            .any(|item| item.contains("t12")));
    }

    #[test]
    fn traced_world_render_identifies_recovery_stages() {
        let rate = 16_000;
        let input = vec![0.0f32; rate as usize];
        let output = WorldResampler::render_sample_with_oto_traced(
            &input,
            rate,
            0.0,
            0.0,
            0.0,
            300.0,
            220.0,
            &[],
            0.0,
            None,
            None,
            None,
        );
        assert_eq!(output.samples.len(), 4_800);
        assert!(output
            .trace
            .fallback_reasons
            .iter()
            .any(|reason| !reason.is_empty()));
        assert!(output
            .trace
            .stages
            .iter()
            .any(|stage| stage == "kamafeu.sola"));
        assert_eq!(
            output.trace.effective_backend_id(),
            output.trace.stages.join("+")
        );
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
