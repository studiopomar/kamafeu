use super::{ResamplerArgs, ResamplerDriver, ResamplerExecutionInfo, ResamplerRenderOutput};
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone, Copy, Default)]
pub struct NativeWorldResamplerDriver;

pub const TAINAH_RESAMPLER_LABEL: &str = "Tainah";

/// Recognizes the current display label and names stored by older projects.
/// Keep this exact enough not to mistake external engines such as World4UTAU
/// for the built-in renderer.
pub fn is_native_world_resampler_label(label: &str) -> bool {
    let normalized = label.trim().to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "tainah"
            | "world"
            | "world-rs"
            | "kamafeu.world-rs"
            | "kamafeu.tainah-motor"
            | "world (nativo)"
            | "world (native)"
    ) || normalized.contains("venus")
}

pub fn canonical_resampler_display_label(label: &str) -> String {
    if is_native_world_resampler_label(label) {
        TAINAH_RESAMPLER_LABEL.to_string()
    } else {
        label.to_string()
    }
}

impl ResamplerDriver for NativeWorldResamplerDriver {
    fn name(&self) -> &str {
        TAINAH_RESAMPLER_LABEL
    }
    fn engine_id(&self) -> &'static str {
        "kamafeu.tainah-motor"
    }
    fn cache_identity(&self) -> String {
        #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
        {
            crate::drivers::tainah_ffi::cache_identity()
        }
        #[cfg(any(target_os = "android", target_arch = "wasm32"))]
        {
            "tainah-motor:unsupported-platform".to_string()
        }
    }
    fn render_sample(
        &self,
        raw_samples: &[f32],
        sample_rate: u32,
        args: &ResamplerArgs,
        cancel: Option<&AtomicBool>,
    ) -> Result<Vec<f32>, String> {
        self.render_sample_with_execution(raw_samples, sample_rate, args, cancel)
            .map(|output| output.samples)
    }
    fn render_sample_with_execution(
        &self,
        raw_samples: &[f32],
        sample_rate: u32,
        args: &ResamplerArgs,
        cancel: Option<&AtomicBool>,
    ) -> Result<ResamplerRenderOutput, String> {
        #[cfg(not(any(target_os = "android", target_arch = "wasm32")))]
        {
            let samples =
                crate::drivers::tainah_ffi::render(raw_samples, sample_rate, args, cancel)
                    .map_err(|reason| format!("Tainah WORLD render failed: {reason}"))?;
            let expected_len =
                ((args.duration_ms.max(0.0) / 1_000.0) * sample_rate as f64).round() as usize;
            if !has_render_coverage(&samples, expected_len, sample_rate) {
                return Err("Tainah WORLD render failed: output failed coverage validation".into());
            }
            Ok(ResamplerRenderOutput {
                samples,
                execution: ResamplerExecutionInfo {
                    effective_backend_id: self.engine_id().to_string(),
                    stages: vec![self.engine_id().to_string()],
                    ..Default::default()
                },
            })
        }
        #[cfg(any(target_os = "android", target_arch = "wasm32"))]
        {
            let _ = (raw_samples, sample_rate, args, cancel);
            Err("Tainah WORLD library is unavailable on this platform".into())
        }
    }
}

pub(super) fn has_render_coverage(samples: &[f32], expected_len: usize, sample_rate: u32) -> bool {
    if expected_len == 0
        || samples.len() < expected_len.saturating_mul(95) / 100
        || samples.iter().any(|sample| !sample.is_finite())
    {
        return false;
    }
    let peak = samples
        .iter()
        .map(|sample| sample.abs())
        .fold(0.0_f32, f32::max);
    if peak <= 1e-5 {
        return false;
    }
    let window = (sample_rate as usize / 20).max(64); // 50 ms
    let threshold = (peak * 0.002).max(1e-5);
    let inspected = &samples[..expected_len.min(samples.len())];
    let mut windows = 0usize;
    let mut audible = 0usize;
    let mut last_window_audible = false;
    for chunk in inspected.chunks(window) {
        if chunk.len() < window / 2 {
            continue;
        }
        windows += 1;
        last_window_audible = chunk.iter().any(|sample| sample.abs() >= threshold);
        if last_window_audible {
            audible += 1;
        }
    }
    // A resampler phoneme must provide material through its requested tail;
    // the wavtool/envelope owns the release. Accepting a loud attack followed
    // by silence creates the characteristic CVVC "hiccup" at every VC -> CV
    // handoff. Permit a small amount of internal quiet material, but require
    // at least 75% coverage and a live final 50 ms window.
    windows > 0 && audible * 4 >= windows * 3 && last_window_audible
}
