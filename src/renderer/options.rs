use crate::phonemizer::PhonemizerMode;

/// Rendering parameters that apply to a singer independently of the GUI.
#[derive(Debug, Clone)]
pub struct RenderOptions {
    pub loudness: f64,
    pub tension: f64,
    pub breathiness: f64,
    pub gender: f64,
    pub tone_shift: f64,
    pub crossfade_ms: f64,
    pub legato_glide_ms: f64,
    pub phonemizer_mode: PhonemizerMode,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            loudness: 0.0,
            tension: 20.0,
            breathiness: 15.0,
            gender: 0.0,
            tone_shift: 0.0,
            crossfade_ms: 45.0,
            legato_glide_ms: 85.0,
            phonemizer_mode: PhonemizerMode::BasicCV,
        }
    }
}
