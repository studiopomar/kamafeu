use super::{ResamplerArgs, ResamplerDriver};
use std::sync::atomic::AtomicBool;

pub struct NativeResamplerDriver;

impl ResamplerDriver for NativeResamplerDriver {
    fn name(&self) -> &str {
        "SOLA (Nativo)"
    }
    fn engine_id(&self) -> &'static str {
        "kamafeu.sola"
    }

    fn cache_identity(&self) -> String {
        // v2: sub-sample pitch marks and band-limited grain interpolation.
        "kamafeu.sola:classic-oto-v2".to_string()
    }

    fn render_sample(
        &self,
        raw_samples: &[f32],
        sample_rate: u32,
        args: &ResamplerArgs,
        _cancel: Option<&AtomicBool>,
    ) -> Result<Vec<f32>, String> {
        let rendered = crate::dsp::SolaResampler::render_sample(
            raw_samples,
            sample_rate,
            args.offset_ms,
            args.source_consonant_ms,
            args.consonant_ms,
            args.cutoff_ms,
            args.duration_ms,
            args.pitch_freq,
            &args.pitch_points,
            args.loop_start_ms,
            args.loop_end_ms,
            args.tail_start_ms,
        );
        Ok(rendered)
    }
}
