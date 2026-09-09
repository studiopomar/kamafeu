use serde::{Deserialize, Serialize};

pub const ISO_31_BAND_FREQS: [f32; 31] = [
    20.0, 25.0, 31.5, 40.0, 50.0, 63.0, 80.0, 100.0, 125.0, 160.0, 200.0, 250.0, 315.0, 400.0,
    500.0, 630.0, 800.0, 1000.0, 1250.0, 1600.0, 2000.0, 2500.0, 3150.0, 4000.0, 5000.0, 6300.0,
    8000.0, 10000.0, 12500.0, 16000.0, 20000.0,
];

pub const ISO_31_BAND_LABELS: [&str; 31] = [
    "20", "25", "31.5", "40", "50", "63", "80", "100", "125", "160", "200", "250", "315", "400",
    "500", "630", "800", "1k", "1.2k", "1.6k", "2k", "2.5k", "3.1k", "4k", "5k", "6.3k", "8k",
    "10k", "12.5k", "16k", "20k",
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EqSettings {
    pub enabled: bool,
    #[serde(default = "default_gains")]
    pub gains: [f32; 31], // -12.0 dB .. +12.0 dB per band
}

fn default_gains() -> [f32; 31] {
    [0.0; 31]
}

impl Default for EqSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            gains: [0.0; 31],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CompressorSettings {
    pub enabled: bool,
    pub threshold_db: f32,   // -40.0 .. 0.0
    pub ratio: f32,          // 1.0 .. 20.0
    pub attack_ms: f32,      // 0.5 .. 100.0
    pub release_ms: f32,     // 10.0 .. 1000.0
    pub makeup_gain_db: f32, // 0.0 .. 24.0
}

impl Default for CompressorSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            threshold_db: -18.0,
            ratio: 4.0,
            attack_ms: 15.0,
            release_ms: 120.0,
            makeup_gain_db: 3.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DelaySettings {
    pub enabled: bool,
    pub time_ms: f32,   // 10.0 .. 1000.0
    pub feedback: f32,  // 0.0 .. 0.95
    pub wet_level: f32, // 0.0 .. 1.0
    pub ping_pong: bool,
}

impl Default for DelaySettings {
    fn default() -> Self {
        Self {
            enabled: false,
            time_ms: 250.0,
            feedback: 0.35,
            wet_level: 0.25,
            ping_pong: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReverbSettings {
    pub enabled: bool,
    pub room_size: f32, // 0.0 .. 1.0
    pub damping: f32,   // 0.0 .. 1.0
    pub width: f32,     // 0.0 .. 1.0
    pub wet_level: f32, // 0.0 .. 1.0
    pub dry_level: f32, // 0.0 .. 1.0
}

impl Default for ReverbSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            room_size: 0.6,
            damping: 0.4,
            width: 1.0,
            wet_level: 0.20,
            dry_level: 1.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct FxRackConfig {
    pub master_enabled: bool,
    pub eq: EqSettings,
    pub compressor: CompressorSettings,
    pub delay: DelaySettings,
    pub reverb: ReverbSettings,
}
