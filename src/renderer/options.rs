use crate::phonemizer::PhonemizerMode;

/// Estilos globais de renderização. Eles não alteram o fonemizador nem as
/// expressões já desenhadas por nota; apenas definem o ponto de partida do
/// motor para toda a voz.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VocalRenderPreset {
    Organic,
    Pop,
    Robotic,
}

/// Rendering parameters that apply to a singer independently of the GUI.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct RenderOptions {
    pub loudness: f64,
    pub tension: f64,
    pub breathiness: f64,
    pub gender: f64,
    pub tone_shift: f64,
    pub crossfade_ms: f64,
    /// Number of resampler jobs that may synthesize at the same time.
    /// `0` is treated as one safe instance.
    pub resampler_instances: u32,
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
            crossfade_ms: 0.0,
            resampler_instances: 2,
            phonemizer_mode: PhonemizerMode::None,
        }
    }
}

impl RenderOptions {
    pub fn apply_preset(&mut self, preset: VocalRenderPreset) {
        let (loudness, tension, breathiness, gender, crossfade_ms) = match preset {
            VocalRenderPreset::Organic => (0.0, 40.0, 10.0, 0.0, 50.0),
            VocalRenderPreset::Pop => (1.5, 70.0, 0.0, 0.0, 35.0),
            VocalRenderPreset::Robotic => (0.0, 95.0, 0.0, 0.0, 5.0),
        };
        self.loudness = loudness;
        self.tension = tension;
        self.breathiness = breathiness;
        self.gender = gender;
        self.crossfade_ms = crossfade_ms;
    }
}

#[cfg(test)]
mod tests {
    use super::{RenderOptions, VocalRenderPreset};

    #[test]
    fn default_breathiness_preserves_the_resampler_native_balance() {
        assert_eq!(RenderOptions::default().breathiness, 0.0);
    }

    #[test]
    fn default_uses_two_resampler_instances() {
        assert_eq!(RenderOptions::default().resampler_instances, 2);
    }

    #[test]
    fn render_options_roundtrip_through_config_serialization() {
        let options = RenderOptions {
            loudness: 2.5,
            tension: 68.0,
            breathiness: 11.0,
            gender: -7.0,
            tone_shift: 1.0,
            crossfade_ms: 42.0,
            resampler_instances: 3,
            phonemizer_mode: crate::phonemizer::PhonemizerMode::PortugueseG2P,
        };
        let serialized = serde_json::to_string(&options).expect("serialize render profile");
        assert_eq!(
            serde_json::from_str::<RenderOptions>(&serialized).expect("deserialize render profile"),
            options
        );
    }

    #[test]
    fn presets_are_consistent_and_preserve_the_selected_phonemizer() {
        let mut options = RenderOptions {
            phonemizer_mode: crate::phonemizer::PhonemizerMode::CVVC,
            tone_shift: 3.0,
            ..RenderOptions::default()
        };
        options.apply_preset(VocalRenderPreset::Organic);
        assert_eq!(
            (
                options.loudness,
                options.tension,
                options.breathiness,
                options.crossfade_ms
            ),
            (0.0, 40.0, 10.0, 50.0)
        );
        options.apply_preset(VocalRenderPreset::Pop);
        assert_eq!(
            (
                options.loudness,
                options.tension,
                options.breathiness,
                options.crossfade_ms
            ),
            (1.5, 70.0, 0.0, 35.0)
        );
        options.apply_preset(VocalRenderPreset::Robotic);
        assert_eq!(
            (
                options.loudness,
                options.tension,
                options.breathiness,
                options.crossfade_ms
            ),
            (0.0, 95.0, 0.0, 5.0)
        );
        assert_eq!(
            options.phonemizer_mode,
            crate::phonemizer::PhonemizerMode::CVVC
        );
        assert_eq!(options.tone_shift, 3.0);
    }
}
