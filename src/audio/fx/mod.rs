//! Stereo effects rack: configuration, filters, and stateful processing.

mod biquad;
mod equalizer;
mod processor;
mod settings;

pub use processor::FxRackProcessor;
pub use settings::{
    AutoPanSettings, ChorusSettings, CompressorSettings, DelaySettings, EqSettings, FxRackConfig,
    GateSettings, LimiterSettings, ReverbSettings, SaturationSettings, ISO_31_BAND_FREQS,
    ISO_31_BAND_LABELS,
};

#[cfg(test)]
mod tests;
