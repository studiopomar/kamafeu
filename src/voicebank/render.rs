use crate::drivers::{NativeWorldResamplerDriver, ResamplerArgs, ResamplerDriver};
use crate::dsp::sola::SolaStretchMode;
use crate::renderer::TrackRenderer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use super::native::{NativeBank, Unit};
use super::planner::{PhrasePlan, PlannedUnit};

const MAX_PHRASE_SECONDS: f64 = 300.0;
const MAX_PHRASE_UNITS: usize = 10_000;
const MAX_PHRASE_SAMPLES: usize = 64 * 1024 * 1024;
const MAX_CACHED_SOURCE_SAMPLES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderedUnitDiagnostic {
    pub unit_id: String,
    pub asset_path: String,
    pub requested_engine: String,
    pub effective_engine: String,
    pub duration_ms: f64,
    pub peak: f32,
    pub rms: f32,
    #[serde(default)]
    pub stages: Vec<String>,
    #[serde(default)]
    pub fallback_reasons: Vec<String>,
    #[serde(default)]
    pub adjustments: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreviewBackend {
    Sola,
    World,
}

impl PreviewBackend {
    fn engine_id(self) -> &'static str {
        match self {
            Self::Sola => "kamafeu.sola",
            Self::World => "kamafeu.lessampler-tainah",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RenderedPhrase {
    pub bank_id: String,
    pub bank_version: String,
    pub package_revision: u32,
    pub engine_id: String,
    pub sample_rate_hz: u32,
    /// Musical time represented by `samples[0]`; may be negative for pickup.
    pub start_time_ms: f64,
    pub samples: Vec<f32>,
    pub units: Vec<RenderedUnitDiagnostic>,
}

#[derive(Debug)]
struct Fragment {
    start_sample: i64,
    samples: Vec<f32>,
    weights: Vec<f32>,
}

/// Render a previously validated plan with the explicit SOLA prototype.
/// This does not use WORLD and never switches engines on failure. SOLA is a
/// provisional export backend; the paired engineering spike and controlled
/// listening on reviewed, authorized recordings remain pending.
pub fn render_phrase_sola(
    bank: &NativeBank,
    plan: &PhrasePlan,
    output_sample_rate: u32,
) -> Result<RenderedPhrase, String> {
    render_phrase_sola_cancellable(bank, plan, output_sample_rate, None)
}

pub fn render_phrase_sola_cancellable(
    bank: &NativeBank,
    plan: &PhrasePlan,
    output_sample_rate: u32,
    cancel: Option<&AtomicBool>,
) -> Result<RenderedPhrase, String> {
    render_phrase_preview_cancellable(bank, plan, output_sample_rate, cancel, PreviewBackend::Sola)
}

/// Explicit WORLD authoring preview for controlled backend comparisons. It is
/// never selected by export and any internal fallback is returned per unit.
pub fn render_phrase_world(
    bank: &NativeBank,
    plan: &PhrasePlan,
    output_sample_rate: u32,
) -> Result<RenderedPhrase, String> {
    render_phrase_world_cancellable(bank, plan, output_sample_rate, None)
}

pub fn render_phrase_world_cancellable(
    bank: &NativeBank,
    plan: &PhrasePlan,
    output_sample_rate: u32,
    cancel: Option<&AtomicBool>,
) -> Result<RenderedPhrase, String> {
    render_phrase_preview_cancellable(
        bank,
        plan,
        output_sample_rate,
        cancel,
        PreviewBackend::World,
    )
}

fn render_phrase_preview_cancellable(
    bank: &NativeBank,
    plan: &PhrasePlan,
    output_sample_rate: u32,
    cancel: Option<&AtomicBool>,
    backend: PreviewBackend,
) -> Result<RenderedPhrase, String> {
    if cancel.is_some_and(|token| token.load(Ordering::Relaxed)) {
        return Err("native phrase render cancelled".into());
    }
    if output_sample_rate == 0 {
        return Err("output sample rate must be greater than zero".into());
    }
    if plan.bank_id != bank.manifest.bank_id
        || plan.bank_version != bank.manifest.bank_version
        || plan.package_revision != bank.manifest.package_revision
        || plan.engine_id != bank.manifest.engine.id
        || plan.engine_api_major != bank.manifest.engine.api_major
    {
        return Err("PhrasePlan identity does not match the loaded KFV 2 bank".into());
    }
    if plan.units.len() > MAX_PHRASE_UNITS {
        return Err("PhrasePlan contains too many units".into());
    }
    if !bank
        .manifest
        .styles
        .iter()
        .any(|style| style.id == plan.style_id)
    {
        return Err(format!(
            "unknown KFV 2 style in PhrasePlan: {}",
            plan.style_id
        ));
    }
    if plan.units.is_empty() {
        return Ok(RenderedPhrase {
            bank_id: plan.bank_id.clone(),
            bank_version: plan.bank_version.clone(),
            package_revision: plan.package_revision,
            engine_id: backend.engine_id().into(),
            sample_rate_hz: output_sample_rate,
            start_time_ms: 0.0,
            samples: Vec::new(),
            units: Vec::new(),
        });
    }

    let mut estimated_fragment_samples = 0usize;
    let mut estimated_start = 0.0f64;
    let mut estimated_end = 0.0f64;
    for (index, unit) in plan.units.iter().enumerate() {
        if cancel.is_some_and(|token| token.load(Ordering::Relaxed)) {
            return Err("native phrase render cancelled".into());
        }
        if !unit.output_start_ms.is_finite()
            || !unit.target_duration_ms.is_finite()
            || unit.target_duration_ms <= 0.0
            || unit.target_duration_ms > MAX_PHRASE_SECONDS * 1_000.0
        {
            return Err(format!(
                "invalid or excessive render duration at unit {index}"
            ));
        }
        let fragment_samples = (unit.target_duration_ms * output_sample_rate as f64 / 1_000.0)
            .round()
            .max(0.0) as usize;
        estimated_fragment_samples = estimated_fragment_samples
            .checked_add(fragment_samples)
            .ok_or_else(|| "phrase sample estimate overflow".to_string())?;
        let end = unit.output_start_ms + unit.target_duration_ms;
        estimated_start = estimated_start.min(unit.output_start_ms);
        estimated_end = estimated_end.max(end);
        if (estimated_end - estimated_start) * output_sample_rate as f64 / 1_000.0
            > MAX_PHRASE_SAMPLES as f64
        {
            return Err("rendered phrase exceeds the sample memory limit".into());
        }
    }
    if estimated_fragment_samples > MAX_PHRASE_SAMPLES {
        return Err("render fragments exceed the sample memory limit".into());
    }

    let root =
        std::fs::canonicalize(&bank.root).map_err(|e| format!("cannot resolve KFV 2 root: {e}"))?;
    let mut loaded_assets: HashMap<String, (Vec<f32>, u32)> = HashMap::new();
    let mut cached_source_samples = 0usize;
    let mut fragments = Vec::with_capacity(plan.units.len());
    let mut diagnostics = Vec::with_capacity(plan.units.len());
    for (index, planned) in plan.units.iter().enumerate() {
        if cancel.is_some_and(|token| token.load(Ordering::Relaxed)) {
            return Err("native phrase render cancelled".into());
        }
        let (layer, unit_file) = bank
            .layers
            .iter()
            .find(|(layer, _)| layer.id == planned.layer_id)
            .ok_or_else(|| {
                format!(
                    "PhrasePlan unit {index} references missing layer '{}'",
                    planned.layer_id
                )
            })?;
        if layer.style_id != plan.style_id || planned.event_index != index {
            return Err(format!(
                "PhrasePlan layer or event order mismatch at unit {index}"
            ));
        }
        let unit = unit_file
            .units
            .iter()
            .find(|unit| unit.id == planned.unit_id)
            .ok_or_else(|| format!("PhrasePlan references missing unit '{}'", planned.unit_id))?;
        verify_planned_unit(planned, unit)?;
        let asset = unit_file
            .assets
            .iter()
            .find(|asset| asset.id == unit.asset_id)
            .ok_or_else(|| format!("unit '{}' references missing audio asset", unit.id))?;
        if asset.path != planned.asset_path {
            return Err(format!(
                "PhrasePlan asset path mismatch for unit '{}'",
                unit.id
            ));
        }
        let asset_path = bank.manifest.resolve(&root, &asset.path)?;
        let canonical_asset = std::fs::canonicalize(&asset_path)
            .map_err(|e| format!("cannot resolve audio asset '{}': {e}", asset.path))?;
        if !canonical_asset.starts_with(&root) || !canonical_asset.is_file() {
            return Err(format!(
                "audio asset escapes the voicebank root: {}",
                asset.path
            ));
        }
        if !loaded_assets.contains_key(&asset.path) {
            if asset.frame_count
                > MAX_CACHED_SOURCE_SAMPLES.saturating_sub(cached_source_samples) as u64
            {
                return Err("selected source assets exceed the sample memory limit".into());
            }
            let reader = hound::WavReader::open(&canonical_asset)
                .map_err(|e| format!("cannot inspect audio asset '{}': {e}", asset.path))?;
            if reader.spec().channels != asset.channels
                || reader.spec().sample_rate != asset.sample_rate_hz
                || reader.duration() as u64 != asset.frame_count
            {
                return Err(format!(
                    "audio asset changed after validation: {}",
                    asset.path
                ));
            }
            let (samples, sample_rate) = TrackRenderer::load_wav_samples(&canonical_asset)?;
            cached_source_samples = cached_source_samples
                .checked_add(samples.len())
                .ok_or_else(|| "source sample cache estimate overflow".to_string())?;
            if cached_source_samples > MAX_CACHED_SOURCE_SAMPLES {
                return Err("selected source assets exceed the sample memory limit".into());
            }
            if sample_rate != asset.sample_rate_hz || samples.len() as u64 != asset.frame_count {
                return Err(format!(
                    "audio asset changed after validation: {}",
                    asset.path
                ));
            }
            if samples.iter().any(|sample| !sample.is_finite()) {
                return Err(format!(
                    "audio asset contains non-finite samples: {}",
                    asset.path
                ));
            }
            loaded_assets.insert(asset.path.clone(), (samples, sample_rate));
        }
        let (source, source_rate) = loaded_assets
            .get(&asset.path)
            .ok_or_else(|| format!("audio asset cache miss: {}", asset.path))?;
        if planned.selected_midi != layer.recorded_midi {
            return Err(format!(
                "PhrasePlan pitch layer mismatch for unit '{}'",
                unit.id
            ));
        }
        let (rendered, execution) =
            render_unit(source, *source_rate, unit, planned, cancel, backend)?;
        if rendered.is_empty() || rendered.iter().any(|sample| !sample.is_finite()) {
            return Err(format!(
                "{} produced invalid audio for unit '{}'",
                backend.engine_id(),
                unit.id
            ));
        }
        let output =
            TrackRenderer::convert_sample_rate(&rendered, *source_rate, output_sample_rate);
        let peak = output
            .iter()
            .fold(0.0f32, |peak, sample| peak.max(sample.abs()));
        let rms = (output
            .iter()
            .map(|sample| f64::from(*sample).powi(2))
            .sum::<f64>()
            / output.len().max(1) as f64)
            .sqrt() as f32;
        diagnostics.push(RenderedUnitDiagnostic {
            unit_id: unit.id.clone(),
            asset_path: asset.path.clone(),
            requested_engine: backend.engine_id().into(),
            effective_engine: execution.effective_backend_id,
            duration_ms: output.len() as f64 * 1_000.0 / output_sample_rate as f64,
            peak,
            rms,
            stages: execution.stages,
            fallback_reasons: execution.fallback_reasons,
            adjustments: execution.adjustments,
        });
        let start_sample = (planned.output_start_ms * output_sample_rate as f64 / 1_000.0).round();
        if !start_sample.is_finite()
            || start_sample < i64::MIN as f64
            || start_sample > i64::MAX as f64
        {
            return Err(format!("invalid output timeline for unit '{}'", unit.id));
        }
        fragments.push(Fragment {
            start_sample: start_sample as i64,
            weights: vec![1.0; output.len()],
            samples: output,
        });
    }

    let origin = fragments
        .iter()
        .map(|fragment| fragment.start_sample)
        .min()
        .unwrap_or(0)
        .min(0);
    let output_end = fragments
        .iter()
        .map(|fragment| {
            fragment
                .start_sample
                .saturating_add(fragment.samples.len() as i64)
        })
        .max()
        .unwrap_or(origin);
    let total_samples_i64 = output_end.saturating_sub(origin).max(0);
    let max_samples = (MAX_PHRASE_SECONDS * output_sample_rate as f64).round() as i64;
    if total_samples_i64 > max_samples {
        return Err("rendered phrase exceeds the 300 second safety limit".into());
    }
    let total_samples = total_samples_i64 as usize;

    // Each adjacent pair transfers ownership across the authored join. The
    // linear weights sum to one, avoiding a level bump when recordings happen
    // to be phase-correlated. If no join is authored, use an even linear mix
    // over the actual overlap rather than summing two full-scale takes.
    for index in 0..fragments.len().saturating_sub(1) {
        if cancel.is_some_and(|token| token.load(Ordering::Relaxed)) {
            return Err("native phrase render cancelled".into());
        }
        let (left, right) = fragments.split_at_mut(index + 1);
        let outgoing = &mut left[index];
        let incoming = &mut right[0];
        let overlap_start = outgoing.start_sample.max(incoming.start_sample);
        let overlap_end = (outgoing.start_sample + outgoing.samples.len() as i64)
            .min(incoming.start_sample + incoming.samples.len() as i64);
        if overlap_start >= overlap_end {
            continue;
        }
        let authored_join = plan.units[index].preferred_join_to_next_ms.max(0.0);
        let join_samples = (authored_join * output_sample_rate as f64 / 1_000.0).round() as i64;
        let actual_overlap = overlap_end - overlap_start;
        let fade_samples = if join_samples > 0 {
            join_samples.min(actual_overlap)
        } else {
            actual_overlap
        };
        for position in overlap_start..overlap_end {
            let left_index = (position - outgoing.start_sample) as usize;
            let right_index = (position - incoming.start_sample) as usize;
            let phase = ((position - overlap_start + 1) as f64 / fade_samples.max(1) as f64)
                .clamp(0.0, 1.0) as f32;
            if position - overlap_start < fade_samples {
                outgoing.weights[left_index] *= 1.0 - phase;
                incoming.weights[right_index] *= phase;
            } else {
                outgoing.weights[left_index] = 0.0;
            }
        }
    }

    let mut sums = vec![0.0f64; total_samples];
    let mut weights = vec![0.0f64; total_samples];
    for fragment in fragments {
        if cancel.is_some_and(|token| token.load(Ordering::Relaxed)) {
            return Err("native phrase render cancelled".into());
        }
        let start = (fragment.start_sample - origin) as usize;
        for (index, (sample, weight)) in fragment.samples.iter().zip(fragment.weights).enumerate() {
            let output_index = start + index;
            sums[output_index] += f64::from(*sample) * f64::from(weight);
            weights[output_index] += f64::from(weight);
        }
    }
    let samples = sums
        .into_iter()
        .zip(weights)
        .map(|(sum, weight)| {
            if weight > 1e-9 {
                (sum / weight) as f32
            } else {
                0.0
            }
        })
        .collect();
    Ok(RenderedPhrase {
        bank_id: plan.bank_id.clone(),
        bank_version: plan.bank_version.clone(),
        package_revision: plan.package_revision,
        engine_id: backend.engine_id().into(),
        sample_rate_hz: output_sample_rate,
        start_time_ms: origin as f64 * 1_000.0 / output_sample_rate as f64,
        samples,
        units: diagnostics,
    })
}

fn verify_planned_unit(planned: &PlannedUnit, unit: &Unit) -> Result<(), String> {
    if planned.phones != unit.phones
        || planned.kind != unit.kind
        || planned.source_start_frame != unit.source.start_frame
        || planned.source_end_frame != unit.source.end_frame
        || planned.anchor_frame != unit.source.anchor_frame
        || planned.fixed_end_frame != unit.source.fixed_end_frame
        || planned.sustain
            != unit
                .source
                .sustain
                .as_ref()
                .map(|r| (r.start_frame, r.end_frame))
        || planned.release_start_frame != unit.source.release_start_frame
        || planned.sustain_mode != unit.render.sustain_mode
        || planned.gain_db != unit.render.gain_db
        || planned.formant_shift_cents != unit.render.formant_shift_cents
        || !planned.output_start_ms.is_finite()
        || !planned.target_duration_ms.is_finite()
        || planned.target_duration_ms <= 0.0
        || !planned.preferred_join_to_next_ms.is_finite()
        || planned.preferred_join_to_next_ms < 0.0
        || !planned.gain_db.is_finite()
        || !planned.formant_shift_cents.is_finite()
    {
        return Err(format!("PhrasePlan values do not match unit '{}'", unit.id));
    }
    Ok(())
}

fn render_unit(
    samples: &[f32],
    sample_rate: u32,
    unit: &Unit,
    planned: &PlannedUnit,
    cancel: Option<&AtomicBool>,
    backend: PreviewBackend,
) -> Result<(Vec<f32>, crate::drivers::ResamplerExecutionInfo), String> {
    if unit.render.formant_shift_cents != 0.0 {
        return Err(format!(
            "formant_shift_cents is not supported by the {} preview for unit '{}'",
            backend.engine_id(),
            unit.id
        ));
    }
    let source = &unit.source;
    let ms = |frame: u64| frame as f64 * 1_000.0 / sample_rate as f64;
    let total_frames = samples.len() as u64;
    let cutoff_ms = if unit.render.sustain_mode == "finite" {
        -((source.end_frame - source.start_frame) as f64) * 1_000.0 / sample_rate as f64
    } else {
        ms(total_frames - source.end_frame)
    };
    let source_consonant_ms = ms(source.fixed_end_frame - source.start_frame);
    let target_len = (planned.target_duration_ms * sample_rate as f64 / 1_000.0).round() as usize;
    if target_len == 0 || target_len > (MAX_PHRASE_SECONDS * sample_rate as f64) as usize {
        return Err(format!("invalid target duration for unit '{}'", unit.id));
    }
    let frequency = 440.0 * 2.0f64.powf((f64::from(planned.requested_midi) - 69.0) / 12.0);
    let anchor_offset_ms =
        (source.anchor_frame - source.start_frame) as f64 * 1_000.0 / sample_rate as f64;
    let pitch_points: Vec<_> = planned
        .pitch_points
        .iter()
        .cloned()
        .map(|mut point| {
            point.time_offset_ms += anchor_offset_ms;
            point
        })
        .collect();
    let mode = match unit.render.sustain_mode.as_str() {
        "loop" => SolaStretchMode::Loop,
        "finite" | "stretch" => SolaStretchMode::Stretch,
        _ => return Err(format!("unsupported sustain mode for unit '{}'", unit.id)),
    };
    let source_sustain = source
        .sustain
        .as_ref()
        .map(|r| (r.start_frame, r.end_frame));
    let args = ResamplerArgs {
        input_wav: PathBuf::new(),
        output_wav: PathBuf::new(),
        pitch_name: format!("MIDI{}", planned.requested_midi),
        pitch_freq: frequency,
        velocity: 100.0,
        flags: String::new(),
        offset_ms: ms(source.start_frame),
        duration_ms: planned.target_duration_ms,
        source_consonant_ms,
        consonant_ms: source_consonant_ms,
        cutoff_ms,
        volume: 100.0,
        modulation: 0.0,
        normalization: None,
        tempo: 120.0,
        pitch_bend_str: String::new(),
        pitch_points: pitch_points.clone(),
        // Tainah receives absolute positions in the complete recording and
        // subtracts the source offset itself, matching Tainah's region API.
        loop_start_ms: source_sustain.map(|r| ms(r.0)),
        loop_end_ms: source_sustain.map(|r| ms(r.1)),
        tail_start_ms: source.release_start_frame.map(ms),
    };
    let (output, execution) = match backend {
        PreviewBackend::Sola => {
            let rendered = crate::dsp::SolaResampler::render_sample_with_mode_cancellable(
                samples,
                sample_rate,
                ms(source.start_frame),
                source_consonant_ms,
                source_consonant_ms,
                cutoff_ms,
                planned.target_duration_ms,
                frequency,
                &pitch_points,
                source.sustain.as_ref().map(|r| ms(r.start_frame)),
                source.sustain.as_ref().map(|r| ms(r.end_frame)),
                source.release_start_frame.map(ms),
                mode,
                cancel,
            )?;
            (
                rendered,
                crate::drivers::ResamplerExecutionInfo {
                    effective_backend_id: "kamafeu.sola".into(),
                    stages: vec!["kamafeu.sola".into()],
                    ..Default::default()
                },
            )
        }
        PreviewBackend::World => {
            if !matches!(
                unit.render.sustain_mode.as_str(),
                "loop" | "finite" | "stretch"
            ) {
                return Err(format!("unsupported sustain mode for unit '{}'", unit.id));
            }
            let result = NativeWorldResamplerDriver.render_sample_with_execution(
                samples,
                sample_rate,
                &args,
                cancel,
            )?;
            (result.samples, result.execution)
        }
    };
    if output.len() != target_len {
        return Err(format!(
            "{} output duration mismatch for unit '{}'",
            backend.engine_id(),
            unit.id
        ));
    }
    let gain = 10.0f32.powf((unit.render.gain_db / 20.0) as f32);
    Ok((
        output.into_iter().map(|sample| sample * gain).collect(),
        execution,
    ))
}
