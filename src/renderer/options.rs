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
    pub phonemizer_mode: PhonemizerMode,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            loudness: 0.0,
            tension: 20.0,
            // Zero means "preserve the resampler's natural balance". Each
            // engine adapter translates positive values to its own flag scale.
            breathiness: 0.0,
            gender: 0.0,
            tone_shift: 0.0,
            crossfade_ms: 45.0,
            phonemizer_mode: PhonemizerMode::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::RenderOptions;

    #[test]
    fn default_breathiness_preserves_the_resampler_native_balance() {
        assert_eq!(RenderOptions::default().breathiness, 0.0);
    }
}
