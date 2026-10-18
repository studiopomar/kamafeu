use super::{WavtoolArgs, WavtoolDriver, WavtoolMixStrategy};
use crate::dsp::envelope::UtauEnvelope;
use std::sync::atomic::AtomicBool;

pub struct NativeWavtoolDriver;

pub struct GalapagosWavtoolDriver;
pub type OpenUtauWavtoolDriver = GalapagosWavtoolDriver;

impl WavtoolDriver for GalapagosWavtoolDriver {
    fn name(&self) -> &str {
        "Galapagos Convergence (Nativo)"
    }

    fn engine_id(&self) -> &'static str {
        "kamafeu.galapagos"
    }

    fn phase_alignment(&self) -> bool {
        true
    }

    fn mix_strategy(&self) -> WavtoolMixStrategy {
        WavtoolMixStrategy::OpenUtauConvergence
    }

    fn envelope_curve_name(&self) -> &'static str {
        "linear-openutau"
    }

    fn process_note(
        &self,
        note_samples: &mut [f32],
        sample_rate: u32,
        args: &WavtoolArgs,
        _cancel: Option<&AtomicBool>,
    ) -> Result<(), String> {
        if note_samples.is_empty() {
            return Ok(());
        }

        // OpenUtau SharpWavtool applies piecewise-linear gain between the five
        // resolved envelope points. Phase analysis is performed by the track
        // renderer before this method mutates the samples.
        UtauEnvelope::apply_points(
            note_samples,
            sample_rate,
            args.sample_time_zero_ms,
            &args.phoneme_envelope,
        );
        Ok(())
    }
}
