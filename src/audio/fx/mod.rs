//! Stereo effects rack: configuration, filters, and stateful processing.

mod biquad;
mod equalizer;
mod processor;
mod settings;

pub use processor::FxRackProcessor;
pub use settings::{
    AutoPanSettings, ChorusSettings, CompressorSettings, DelaySettings, EqSettings,
    FlangerSettings, FxRackConfig, GateSettings, LimiterSettings, PhaserSettings, ReverbSettings,
    SaturationSettings, StereoWidenerSettings, ISO_31_BAND_FREQS, ISO_31_BAND_LABELS,
};

#[cfg(test)]
mod tests;
