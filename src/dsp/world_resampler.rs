//! Runtime WORLD settings shared by the desktop UI and the tainah-engine adapter.
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

pub fn get_runtime_config() -> WorldRuntimeConfig {
    runtime_config()
        .read()
        .map(|value| value.clone())
        .unwrap_or_default()
}

pub fn set_runtime_config(config: WorldRuntimeConfig) {
    if let Ok(mut current) = runtime_config().write() {
        *current = config;
    }
}

pub fn cache_identity() -> String {
    let config = get_runtime_config();
    format!(
        "tainah-cpp-config-v2:f0-{:.3}-{:.3}:ap-{:.4}:formant-{}:detector-{}:frame-{:.3}",
        config.f0_floor_hz,
        config.f0_ceil_hz,
        config.voiced_aperiodicity,
        config.formant_preservation_mode,
        config.f0_detection_method,
        config.frame_period_ms
    )
}
