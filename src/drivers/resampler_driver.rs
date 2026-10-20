#[path = "resampler/classic.rs"]
mod classic;
#[path = "resampler/external.rs"]
mod external;
#[path = "resampler/macres.rs"]
mod macres;
#[path = "resampler/native.rs"]
mod native;
#[path = "resampler/native_world.rs"]
mod native_world;

pub use super::resampler_catalog::KnownResampler;
pub use super::resampler_flags::{
    merge_utau_flags, parse_flag_numeric, parse_utau_flags, remove_flag, set_utau_flag,
    UtauFlagToken,
};
use classic::{
    actual_input_wav, classic_arguments, executable_cache_identity, load_resampler_output,
    prepare_classic_flags,
};
pub use classic::{describe_resampler_command, effective_classic_flags};
pub use external::{
    ensure_hifisampler_environment, ensure_hifisampler_ready, ensure_hifisampler_server_running,
    shutdown_hifisampler_server, ExternalResamplerDriver,
};
pub use macres::MacResDriver;
pub use native::NativeResamplerDriver;
#[cfg(test)]
use native_world::has_render_coverage;
pub use native_world::{
    canonical_resampler_display_label, is_native_world_resampler_label, NativeWorldResamplerDriver,
    TAINAH_RESAMPLER_LABEL,
};

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
#[cfg(test)]
use std::{ffi::OsString, path::Path};

#[derive(Debug, Clone)]
pub struct ResamplerArgs {
    pub input_wav: PathBuf,
    pub output_wav: PathBuf,
    pub pitch_name: String,
    pub pitch_freq: f64,
    pub velocity: f64,
    pub flags: String,
    pub offset_ms: f64,
    pub duration_ms: f64,
    pub source_consonant_ms: f64,
    pub consonant_ms: f64,
    /// Optional WORLD-only target boundary aligned to the resolved note onset.
    /// Classic resamplers continue to use `consonant_ms` unchanged.
    pub world_target_consonant_ms: Option<f64>,
    pub cutoff_ms: f64,
    pub volume: f64,
    pub modulation: f64,
    /// OpenUtau's per-phoneme NORM expression.
    pub normalization: Option<f64>,
    pub tempo: f64,
    pub pitch_bend_str: String,
    pub pitch_points: Vec<crate::project::model::UPitchBendPoint>,
    pub loop_start_ms: Option<f64>,
    pub loop_end_ms: Option<f64>,
    pub tail_start_ms: Option<f64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResamplerExecutionInfo {
    pub effective_backend_id: String,
    #[serde(default)]
    pub stages: Vec<String>,
    #[serde(default)]
    pub fallback_reasons: Vec<String>,
    #[serde(default)]
    pub adjustments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResamplerRenderOutput {
    pub samples: Vec<f32>,
    pub execution: ResamplerExecutionInfo,
}

pub trait ResamplerDriver: Send + Sync {
    fn name(&self) -> &str;
    /// Stable implementation identity for project provenance and caches.
    /// This is deliberately separate from the localized display name.
    fn engine_id(&self) -> &'static str {
        "external.unknown"
    }
    fn prepare_flags(&self, base_flags: &str, gender: f64, breathiness: f64) -> String {
        prepare_classic_flags(base_flags, gender, breathiness)
    }
    fn cache_identity(&self) -> String {
        self.name().to_string()
    }
    fn uses_external_process(&self) -> bool {
        false
    }
    fn supports_persistent_cache(&self) -> bool {
        true
    }
    fn render_sample(
        &self,
        raw_samples: &[f32],
        sample_rate: u32,
        args: &ResamplerArgs,
        cancel: Option<&AtomicBool>,
    ) -> Result<Vec<f32>, String>;

    /// Render audio and report the implementation path. Drivers with internal
    /// fallbacks override this method; simple drivers inherit their own stable
    /// identity as the effective backend.
    fn render_sample_with_execution(
        &self,
        raw_samples: &[f32],
        sample_rate: u32,
        args: &ResamplerArgs,
        cancel: Option<&AtomicBool>,
    ) -> Result<ResamplerRenderOutput, String> {
        self.render_sample(raw_samples, sample_rate, args, cancel)
            .map(|samples| ResamplerRenderOutput {
                samples,
                execution: ResamplerExecutionInfo {
                    effective_backend_id: self.engine_id().to_string(),
                    stages: vec![self.engine_id().to_string()],
                    ..Default::default()
                },
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tainah_display_name_and_legacy_world_names_keep_the_native_route() {
        for label in [
            "Tainah",
            "kamafeu.world-rs",
            "WORLD (Nativo)",
            "WORLD (Native)",
            "WORLD",
            "Venus Spectral",
        ] {
            assert!(is_native_world_resampler_label(label), "{label}");
        }
        assert!(!is_native_world_resampler_label("World4UTAU"));
        assert_eq!(
            canonical_resampler_display_label("WORLD (Nativo)"),
            "Tainah"
        );
        assert_eq!(
            canonical_resampler_display_label("straycat-rs (UtaUtaUtau)"),
            "straycat-rs (UtaUtaUtau)"
        );
        assert_eq!(NativeWorldResamplerDriver.name(), "Tainah");
        assert_eq!(
            NativeWorldResamplerDriver.engine_id(),
            "kamafeu.tainah-motor"
        );
    }

    #[test]
    fn executable_cache_identity_uses_content_not_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("resampler.bin");
        std::fs::write(&path, b"first").unwrap();
        let first = executable_cache_identity("test", &path);
        std::fs::write(&path, b"second").unwrap();
        let second = executable_cache_identity("test", &path);
        assert_ne!(first, second);
        assert!(first.contains(":content:"));
    }

    fn sample_args() -> ResamplerArgs {
        ResamplerArgs {
            input_wav: PathBuf::from("input.wav"),
            output_wav: PathBuf::from("output.wav"),
            pitch_name: "C4".to_string(),
            pitch_freq: 261.63,
            velocity: 125.0,
            flags: String::new(),
            offset_ms: 10.0,
            duration_ms: 500.0,
            source_consonant_ms: 80.0,
            consonant_ms: 64.0,
            world_target_consonant_ms: None,
            cutoff_ms: -20.0,
            volume: 90.0,
            modulation: 5.0,
            normalization: None,
            tempo: 135.0,
            pitch_bend_str: String::new(),
            pitch_points: Vec::new(),
            loop_start_ms: None,
            loop_end_ms: None,
            tail_start_ms: None,
        }
    }

    #[test]
    fn world_driver_rejects_silent_input_without_fallback() {
        let driver = NativeWorldResamplerDriver;
        let error = driver
            .render_sample_with_execution(&vec![0.0; 44_100], 44_100, &sample_args(), None)
            .expect_err("silent input must not yield a plausible render");
        assert!(error.contains("Tainah WORLD render failed"));
        assert!(!error.to_ascii_lowercase().contains("sola"));
    }

    #[test]
    fn world_driver_rejects_invalid_input_and_zero_coverage_without_fallback() {
        let driver = NativeWorldResamplerDriver;
        let error = driver
            .render_sample_with_execution(&[], 44_100, &sample_args(), None)
            .expect_err("empty input must fail WORLD analysis");
        assert!(error.contains("Tainah WORLD render failed"), "{error}");

        let mut args = sample_args();
        args.duration_ms = 0.0;
        let input = (0..44_100)
            .map(|index| {
                (std::f64::consts::TAU * 220.0 * index as f64 / 44_100.0).sin() as f32 * 0.25
            })
            .collect::<Vec<_>>();
        let error = driver
            .render_sample_with_execution(&input, 44_100, &args, None)
            .expect_err("zero requested duration cannot pass WORLD coverage");
        assert!(error.contains("Tainah WORLD render failed"), "{error}");
        assert!(!error.to_ascii_lowercase().contains("sola"));
    }

    #[test]
    fn world_driver_success_reports_only_world_stages() {
        let driver = NativeWorldResamplerDriver;
        let input = (0..44_100)
            .map(|index| {
                (std::f64::consts::TAU * 220.0 * index as f64 / 44_100.0).sin() as f32 * 0.25
            })
            .collect::<Vec<_>>();
        let output = driver
            .render_sample_with_execution(&input, 44_100, &sample_args(), None)
            .expect("WORLD should synthesize a voiced tone");
        assert_eq!(output.samples.len(), 22_050);
        assert!(output.samples.iter().all(|sample| sample.is_finite()));
        assert!(output.samples.iter().any(|sample| sample.abs() > 1e-5));
        assert!(output
            .execution
            .stages
            .iter()
            .all(|stage| stage.contains("tainah-motor")));
    }

    #[test]
    fn world_driver_accepts_short_voiced_phoneme_input() {
        let driver = NativeWorldResamplerDriver;
        let rate = 16_000_u32;
        let input = (0..rate as usize / 5)
            .map(|index| {
                (std::f64::consts::TAU * 220.0 * index as f64 / rate as f64).sin() as f32 * 0.25
            })
            .collect::<Vec<_>>();
        let mut args = sample_args();
        args.offset_ms = 0.0;
        args.duration_ms = 200.0;
        args.source_consonant_ms = 0.0;
        args.consonant_ms = 0.0;
        args.cutoff_ms = 0.0;
        args.pitch_freq = 220.0;
        let output = driver
            .render_sample_with_execution(&input, rate, &args, None)
            .expect("short voiced phoneme should remain renderable by WORLD");
        assert_eq!(output.samples.len(), 3_200);
        assert!(output.samples.iter().all(|sample| sample.is_finite()));
        assert!(output.samples.iter().any(|sample| sample.abs() > 1e-5));
    }

    #[test]
    fn known_profiles_roundtrip_their_labels() {
        for profile in KnownResampler::ALL {
            assert_eq!(KnownResampler::from_label(profile.label()), Some(profile));
            assert!(!profile.executable_names().is_empty());
            assert_ne!(profile.engine_id(), "external.resampler.unknown");
        }
        let ids = KnownResampler::ALL.map(KnownResampler::engine_id);
        let unique = ids.iter().collect::<std::collections::HashSet<_>>();
        assert_eq!(ids.len(), unique.len());
    }

    #[test]
    fn classic_arguments_use_utau_tempo_and_pitch_defaults() {
        let args = classic_arguments(Path::new("source.wav"), &sample_args(), "-", 500.0, false);
        assert_eq!(args[4], OsString::from("-"));
        assert_eq!(args[6], OsString::from("500"));
        assert_eq!(args[11], OsString::from("!135.0"));
        assert_eq!(args[12], OsString::from("AA"));
    }

    #[test]
    fn classic_arguments_preserve_unknown_flags_and_transmit_norm() {
        let mut args = sample_args();
        args.flags = "P86Unknown".to_string();
        args.normalization = Some(0.0);
        let command = describe_resampler_command("straycat-rs", &args);
        let arguments = classic_arguments(Path::new("source.wav"), &args, "g0", 500.0, false);
        assert_eq!(arguments[4], OsString::from("P86UnknownNORM=0"));
        assert!(command.contains("--flags 'P86UnknownNORM=0'"));
    }

    #[test]
    fn straycat_duration_excludes_the_rendered_consonant() {
        let driver = ExternalResamplerDriver::for_known(KnownResampler::StraycatRs, None);
        assert_eq!(driver.requested_duration_ms(&sample_args()), 436.0);

        let organum = ExternalResamplerDriver::for_known(KnownResampler::Organum, None);
        assert_eq!(organum.requested_duration_ms(&sample_args()), 500.0);
    }

    #[test]
    fn straycat_neutral_breathiness_preserves_its_native_default_and_manual_flags() {
        let driver = ExternalResamplerDriver::for_known(KnownResampler::StraycatRs, None);

        assert_eq!(driver.prepare_flags("P86", 0.0, 0.0), "P86");
        assert_eq!(driver.prepare_flags("B35P86", 0.0, 0.0), "B35P86");
    }

    #[test]
    fn straycat_maps_kamafeu_breathiness_above_its_neutral_b50() {
        let driver = ExternalResamplerDriver::for_known(KnownResampler::StraycatRs, None);

        assert_eq!(driver.prepare_flags("P86", 0.0, 1.0), "P86B51");
        assert_eq!(driver.prepare_flags("P86", 0.0, 50.0), "P86B75");
        assert_eq!(driver.prepare_flags("P86", 0.0, 100.0), "P86B100");
    }

    #[test]
    fn straycat_expression_replaces_authored_breathiness_without_duplicates() {
        let driver = ExternalResamplerDriver::for_known(KnownResampler::StraycatRs, None);

        assert_eq!(
            driver.prepare_flags("B10g-5B90P86", 4.0, 20.0),
            "g-5P86g4B60"
        );
    }

    #[test]
    fn other_resamplers_keep_the_classic_breathiness_mapping() {
        let driver = ExternalResamplerDriver::for_known(KnownResampler::Organum, None);

        assert_eq!(driver.prepare_flags("P86", -4.0, 15.0), "P86g-4B15");
    }

    #[test]
    fn hifisampler_maps_breathiness_to_hb_flag() {
        let driver = ExternalResamplerDriver::for_known(KnownResampler::HifisamplerRs, None);

        assert_eq!(driver.prepare_flags("P86", 0.0, 0.0), "P86");
        assert_eq!(driver.prepare_flags("P86", -2.0, 20.0), "P86g-2Hb140");
    }

    #[test]
    fn external_output_is_normalized_to_the_expected_sample_rate() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("render.wav");
        crate::renderer::TrackRenderer::save_wav_samples(&output, &vec![0.25; 4_410], 44_100)
            .unwrap();
        let mut args = sample_args();
        args.output_wav = output.clone();
        let samples = load_resampler_output(&args, 48_000).unwrap();
        assert_eq!(samples.len(), 4_800);
        assert!(output.exists());
    }

    #[test]
    fn external_output_preserves_resampler_level_without_host_limiting() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("unsafe.wav");
        crate::renderer::TrackRenderer::save_wav_samples(
            &output,
            &[f32::NAN, f32::INFINITY, -2.0, 2.0, 0.25],
            48_000,
        )
        .unwrap();
        let mut args = sample_args();
        args.output_wav = output;
        let samples = load_resampler_output(&args, 48_000).unwrap();
        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(
            samples
                .iter()
                .map(|sample| sample.abs())
                .fold(0.0, f32::max)
                > 0.95
        );
    }

    #[test]
    fn world_watchdog_rejects_a_fragment_that_dies_before_the_note_ends() {
        let sample_rate = 44_100;
        let expected_len = sample_rate * 2;
        let mut fragment = vec![0.0; expected_len];
        fragment[..sample_rate / 4].fill(0.25);

        assert!(!has_render_coverage(
            &fragment,
            expected_len,
            sample_rate as u32
        ));
    }

    #[test]
    fn world_watchdog_accepts_a_sustained_note() {
        let sample_rate = 44_100;
        let expected_len = sample_rate * 2;
        let sustained = vec![0.25; expected_len];

        assert!(has_render_coverage(
            &sustained,
            expected_len,
            sample_rate as u32
        ));
    }

    #[test]
    fn world_watchdog_rejects_a_short_phone_with_a_silent_tail() {
        let sample_rate = 44_100;
        let expected_len = sample_rate / 5;
        let mut truncated = vec![0.0; expected_len];
        truncated[..expected_len * 2 / 3].fill(0.25);

        assert!(!has_render_coverage(
            &truncated,
            expected_len,
            sample_rate as u32
        ));
    }

    #[test]
    fn test_parse_utau_flags_distinguishes_composite_flags() {
        let tokens = parse_utau_flags("g-5Hb140B50Mb20P86t+10");
        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0].key, "g");
        assert_eq!(tokens[0].num_val, Some(-5.0));
        assert_eq!(tokens[1].key, "Hb");
        assert_eq!(tokens[1].num_val, Some(140.0));
        assert_eq!(tokens[2].key, "B");
        assert_eq!(tokens[2].num_val, Some(50.0));
        assert_eq!(tokens[3].key, "Mb");
        assert_eq!(tokens[3].num_val, Some(20.0));
        assert_eq!(tokens[4].key, "P");
        assert_eq!(tokens[4].num_val, Some(86.0));
        assert_eq!(tokens[5].key, "t");
        assert_eq!(tokens[5].num_val, Some(10.0));

        // Test numerical lookup
        assert_eq!(parse_flag_numeric("g-5Hb140B50", "Hb"), Some(140.0));
        assert_eq!(parse_flag_numeric("g-5Hb140B50", "B"), Some(50.0));
        assert_eq!(parse_flag_numeric("g-5Hb140B50", "g"), Some(-5.0));

        // Test flag removal without touching composite prefixes
        assert_eq!(remove_flag("g-5Hb140B50P86", "B"), "g-5Hb140P86");
        assert_eq!(remove_flag("g-5Hb140B50P86", "Hb"), "g-5B50P86");
    }

    #[test]
    fn flag_scopes_use_specific_values_and_preserve_unknown_tokens() {
        assert_eq!(
            merge_utau_flags(&["P86g-5Q5", "g10Hb140", "Q8"]),
            "P86g10Q8Hb140"
        );
    }

    #[test]
    fn structured_flag_updates_keep_other_tokens_and_can_remove_one() {
        assert_eq!(set_utau_flag("P86Q5g-2", "g", Some(12.5)), "P86Q5g12.5");
        assert_eq!(set_utau_flag("P86Q5g-2", "g", None), "P86Q5");
    }

    #[test]
    fn structured_flag_updates_preserve_unknown_and_case_sensitive_tokens() {
        let original = "Q7b-3B40Mt20x99";
        let updated = set_utau_flag(original, "b", Some(8.0));

        assert_eq!(updated, "Q7B40Mt20x99b8");
        assert_eq!(parse_flag_numeric(&updated, "B"), Some(40.0));
        assert_eq!(parse_flag_numeric(&updated, "b"), Some(8.0));
        assert_eq!(parse_flag_numeric(&updated, "Mt"), Some(20.0));
        assert_eq!(parse_flag_numeric(&updated, "x"), Some(99.0));
    }
}
