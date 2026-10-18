#[path = "wavtool/external.rs"]
mod external;
#[path = "wavtool/io.rs"]
mod io;
#[path = "wavtool/native.rs"]
mod native;
#[path = "wavtool/yawu.rs"]
mod yawu;

pub use super::wavtool_catalog::KnownWavtool;
pub use external::{concatenate_external, ExternalWavtoolDriver};
use io::load_wavtool_output;
pub use native::{GalapagosWavtoolDriver, NativeWavtoolDriver, OpenUtauWavtoolDriver};
pub use yawu::WavtoolYawuDriver;

use crate::dsp::envelope::UtauEnvelope;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

/// How the track renderer should place segments produced by a wavtool.
///
/// `OpenUtauConvergence` deliberately names the compatibility contract rather
/// than a generic phase-align flag: it measures phase on the raw resampler
/// output and shifts the complete, already-enveloped segment during the
/// ordered merge, matching OpenUtau's SharpWavtool pipeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WavtoolMixStrategy {
    Plain,
    LegacyPhaseAligned,
    OpenUtauConvergence,
}

#[derive(Debug, Clone)]
pub struct WavtoolArgs {
    pub output_wav: PathBuf,
    pub input_rendered_wav: PathBuf,
    pub skip_over_ms: f64,
    pub duration_ms: f64,
    pub envelope: UtauEnvelope,
    pub overlap_ms: f64,
    pub phoneme_envelope: [(f64, f64); 5],
    pub sample_time_zero_ms: f64,
}

pub fn describe_wavtool_command(driver: &str, args: &WavtoolArgs) -> String {
    format!(
        "{driver} --input-rendered-wav \"{}\" --output-wav \"{}\" --skip-over-ms {:.3} --duration-ms {:.3} --overlap-ms {:.3} --sample-time-zero-ms {:.3}",
        args.input_rendered_wav.to_string_lossy().replace('"', "\\\""),
        args.output_wav.to_string_lossy().replace('"', "\\\""),
        args.skip_over_ms,
        args.duration_ms,
        args.overlap_ms,
        args.sample_time_zero_ms,
    )
}

pub trait WavtoolDriver: Send + Sync {
    fn name(&self) -> &str;
    /// Stable implementation identity, separate from the localized label.
    fn engine_id(&self) -> &'static str {
        "external.wavtool"
    }
    /// Whether this concatenator intentionally performs convergence-style
    /// phase alignment. Simple and external wavtools own their placement and
    /// must never be shifted again by the track mixer.
    fn phase_alignment(&self) -> bool {
        false
    }
    /// Additive replacement for the historical boolean capability. Existing
    /// drivers that only override `phase_alignment` keep the legacy behavior.
    fn mix_strategy(&self) -> WavtoolMixStrategy {
        if self.phase_alignment() {
            WavtoolMixStrategy::LegacyPhaseAligned
        } else {
            WavtoolMixStrategy::Plain
        }
    }
    fn envelope_curve_name(&self) -> &'static str {
        "driver-defined"
    }
    fn phrase_executable(&self) -> Option<PathBuf> {
        None
    }
    fn consumes_skip_over(&self) -> bool {
        false
    }
    fn process_note(
        &self,
        note_samples: &mut [f32],
        sample_rate: u32,
        args: &WavtoolArgs,
        cancel: Option<&AtomicBool>,
    ) -> Result<(), String>;
}

impl WavtoolDriver for NativeWavtoolDriver {
    fn name(&self) -> &str {
        "Andromeda (Alternativo)"
    }

    fn engine_id(&self) -> &'static str {
        "kamafeu.andromeda"
    }

    fn envelope_curve_name(&self) -> &'static str {
        "cosine"
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

        // Use the same equal-power S-curve as the phase-aligned native
        // wavtool. Linear ramps expose the exact handoff point between
        // phonemes as a sudden change in slope, especially for short VC/CV
        // aliases.
        UtauEnvelope::apply_points_cosine(
            note_samples,
            sample_rate,
            args.sample_time_zero_ms,
            &args.phoneme_envelope,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn yawu_test_args(duration_ms: f64, skip_over_ms: f64) -> WavtoolArgs {
        WavtoolArgs {
            output_wav: PathBuf::from("unused-output.wav"),
            input_rendered_wav: PathBuf::from("unused-input.wav"),
            skip_over_ms,
            duration_ms,
            envelope: UtauEnvelope {
                p1: 10.0,
                p2: 10.0,
                p3: 1.0,
                p4: 0.0,
                p5: 1.0,
                v1: 0.0,
                v2: 100.0,
                v3: 100.0,
                v4: 0.0,
                v5: 100.0,
                crossfade_ms: 0.0,
            },
            overlap_ms: 0.0,
            phoneme_envelope: [
                (-100.0, 0.0),
                (-50.0, 1.0),
                (0.0, 1.0),
                (900.0, 1.0),
                (950.0, 0.0),
            ],
            sample_time_zero_ms: -100.0,
        }
    }

    #[test]
    fn known_wavtools_roundtrip_their_labels() {
        for profile in KnownWavtool::ALL {
            assert_eq!(KnownWavtool::from_label(profile.label()), Some(profile));
            assert!(!profile.executable_names().is_empty());
        }
    }

    #[test]
    fn wavtool_output_replaces_the_phone_audio() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("wavtool.wav");
        crate::renderer::TrackRenderer::save_wav_samples(&output, &[0.25; 4], 44_100).unwrap();
        let mut phone = [0.0; 4];

        load_wavtool_output(&mut phone, 44_100, &output).unwrap();

        assert!(phone.iter().all(|sample| (sample - 0.25).abs() < 1e-3));
    }

    #[test]
    fn integrated_yawu_leaves_skip_to_the_shared_mixer() {
        let mut phone = (0..2_000)
            .map(|index| index as f32 / 2_000.0)
            .collect::<Vec<_>>();
        let args = yawu_test_args(1_000.0, 100.0);

        WavtoolYawuDriver::process_integrated(&mut phone, 1_000, &args);

        assert!((phone[600] - 0.3).abs() < 1e-3);
        assert!(phone[1_050..].iter().all(|sample| sample.abs() < 1e-6));
    }

    #[test]
    fn integrated_yawu_handles_skip_beyond_source() {
        let mut phone = vec![0.75; 100];
        let args = yawu_test_args(100.0, 200.0);

        WavtoolYawuDriver::process_integrated(&mut phone, 1_000, &args);

        assert!(phone
            .get(args.skip_over_ms as usize..)
            .unwrap_or(&[])
            .is_empty());
    }

    #[test]
    fn integrated_yawu_preserves_float_headroom_for_the_mixer() {
        let mut phone = vec![1.25; 1_000];
        let mut args = yawu_test_args(1_000.0, 0.0);
        args.envelope.p1 = 0.0;
        args.envelope.p2 = 0.0;
        args.envelope.v1 = 100.0;

        WavtoolYawuDriver::process_integrated(&mut phone, 1_000, &args);

        assert!(phone[500] > 1.2);
    }

    #[test]
    fn integrated_yawu_keeps_release_after_the_musical_duration() {
        let mut phone = vec![0.75; 1_200];
        let mut args = yawu_test_args(1_000.0, 0.0);
        args.sample_time_zero_ms = 0.0;
        args.phoneme_envelope = [
            (0.0, 0.0),
            (10.0, 1.0),
            (20.0, 1.0),
            (1_050.0, 1.0),
            (1_100.0, 0.0),
        ];

        WavtoolYawuDriver::process_integrated(&mut phone, 1_000, &args);

        assert!(phone[1_025] > 0.7);
        assert!(phone[1_075] > 0.3);
        assert_eq!(phone[1_100], 0.0);
    }

    #[test]
    fn galapagos_driver_applies_openutau_linear_envelope() {
        let driver = GalapagosWavtoolDriver;
        assert_eq!(driver.name(), "Galapagos Convergence (Nativo)");
        assert!(driver.phase_alignment());
        assert_eq!(
            driver.mix_strategy(),
            WavtoolMixStrategy::OpenUtauConvergence
        );
        assert_eq!(driver.envelope_curve_name(), "linear-openutau");
        assert!(!driver.consumes_skip_over());
        assert!(driver.phrase_executable().is_none());

        let mut samples = vec![1.0f32; 1_000];
        let args = yawu_test_args(1_000.0, 0.0);
        driver
            .process_note(&mut samples, 1_000, &args, None)
            .unwrap();

        assert_eq!(samples[0], 0.0);
        assert!((samples[25] - 0.5).abs() < 1e-6);
        assert!((samples[50] - 1.0).abs() < 1e-6);
        assert!(samples[100] > 0.9);
        assert!(samples.iter().all(|s| s.is_finite()));
    }

    #[test]
    fn native_wavtool_uses_a_soft_curve_at_phoneme_attacks() {
        let driver = NativeWavtoolDriver;
        assert_eq!(driver.engine_id(), "kamafeu.andromeda");
        assert_eq!(driver.mix_strategy(), WavtoolMixStrategy::Plain);
        assert_eq!(driver.envelope_curve_name(), "cosine");
        let mut samples = vec![1.0f32; 1_000];
        let args = yawu_test_args(1_000.0, 0.0);

        driver
            .process_note(&mut samples, 1_000, &args, None)
            .unwrap();

        // At -87 ms the attack is one quarter through a -100..-50 ms ramp.
        // A cosine S-curve is intentionally below the linear 0.5 midpoint.
        assert!(samples[12] < 0.35);
        assert!(samples[50] > 0.9);
    }

    #[test]
    fn phase_aligned_wavtool_has_a_distinct_stable_identity() {
        let driver = GalapagosWavtoolDriver;
        assert_eq!(driver.engine_id(), "kamafeu.galapagos");
        assert_eq!(
            driver.mix_strategy(),
            WavtoolMixStrategy::OpenUtauConvergence
        );
    }
}
