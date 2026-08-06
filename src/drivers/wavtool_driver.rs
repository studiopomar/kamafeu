use crate::dsp::envelope::UtauEnvelope;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct WavtoolArgs {
    pub output_wav: PathBuf,
    pub input_rendered_wav: PathBuf,
    pub offset_ms: f64,
    pub duration_ms: f64,
    pub envelope: UtauEnvelope,
    pub overlap_ms: f64,
}

pub trait WavtoolDriver: Send + Sync {
    fn name(&self) -> &str;
    fn process_note(&self, note_samples: &mut [f32], sample_rate: u32, args: &WavtoolArgs);
}

/// Native Rust Wavtool Driver (5-point UTAU envelope).
///
/// Crossfading must happen while notes are mixed, where both sides of the
/// transition are available. Applying independent fades here caused gaps and
/// double-fades for VCV/CVVC voicebanks.
pub struct NativeWavtoolDriver;

impl WavtoolDriver for NativeWavtoolDriver {
    fn name(&self) -> &str {
        "Native Rust (Crossfader)"
    }

    fn process_note(&self, note_samples: &mut [f32], sample_rate: u32, args: &WavtoolArgs) {
        if note_samples.is_empty() {
            return;
        }

        // Apply the per-note amplitude envelope. The track mixer performs the
        // complementary crossfade against the preceding phone.
        args.envelope
            .apply(note_samples, sample_rate, args.duration_ms);
    }
}

/// WavtoolYawu Driver (https://github.com/m13253/wavtool-yawu)
pub struct WavtoolYawuDriver {
    pub executable_path: PathBuf,
}

impl WavtoolYawuDriver {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let p = path.as_ref();
        let final_path = if p.exists() {
            p.to_path_buf()
        } else {
            Self::find_executable().unwrap_or_else(|| p.to_path_buf())
        };
        Self {
            executable_path: final_path,
        }
    }

    /// Auto-detect wavtool-yawu executable in system PATH or local folders
    pub fn find_executable() -> Option<PathBuf> {
        let candidates = [
            PathBuf::from("wavtool-yawu"),
            PathBuf::from("./wavtools/wavtool-yawu"),
            PathBuf::from("./wavtool-yawu"),
            PathBuf::from("/opt/homebrew/bin/wavtool-yawu"),
            PathBuf::from("/usr/local/bin/wavtool-yawu"),
        ];

        candidates.into_iter().find(|candidate| candidate.exists())
    }
}

impl WavtoolDriver for WavtoolYawuDriver {
    fn name(&self) -> &str {
        "wavtool-yawu (m13253/wavtool-yawu)"
    }

    fn process_note(&self, note_samples: &mut [f32], sample_rate: u32, args: &WavtoolArgs) {
        if self.executable_path.exists() {
            if !args.input_rendered_wav.exists() {
                let _ = crate::renderer::TrackRenderer::save_wav_samples(
                    &args.input_rendered_wav,
                    note_samples,
                    sample_rate,
                );
            }

            let mut cmd = Command::new(&self.executable_path);
            cmd.arg(&args.output_wav)
                .arg(&args.input_rendered_wav)
                .arg(format!("{:.1}", args.offset_ms))
                .arg(format!("{:.1}", args.duration_ms))
                .arg(format!("{:.1}", args.envelope.p1))
                .arg(format!("{:.1}", args.envelope.p2))
                .arg(format!("{:.1}", args.envelope.p3))
                .arg(format!("{:.1}", args.envelope.p4))
                .arg(format!("{:.1}", args.envelope.p5))
                .arg(format!("{:.0}", args.envelope.v1))
                .arg(format!("{:.0}", args.envelope.v2))
                .arg(format!("{:.0}", args.envelope.v3))
                .arg(format!("{:.0}", args.envelope.v4))
                .arg(format!("{:.0}", args.envelope.v5))
                .arg(format!("{:.1}", args.overlap_ms));

            let _ = cmd.output();
        }

        // Apply native envelope processing for seamless audio output
        args.envelope
            .apply(note_samples, sample_rate, args.duration_ms);
    }
}

/// Generic External Wavtool Driver
pub struct ExternalWavtoolDriver {
    pub executable_path: PathBuf,
}

impl ExternalWavtoolDriver {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            executable_path: path.as_ref().to_path_buf(),
        }
    }
}

impl WavtoolDriver for ExternalWavtoolDriver {
    fn name(&self) -> &str {
        self.executable_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("External Wavtool")
    }

    fn process_note(&self, note_samples: &mut [f32], sample_rate: u32, args: &WavtoolArgs) {
        if self.executable_path.exists() {
            let mut cmd = Command::new(&self.executable_path);
            cmd.arg(&args.output_wav)
                .arg(&args.input_rendered_wav)
                .arg(format!("{:.1}", args.offset_ms))
                .arg(format!("{:.1}", args.duration_ms))
                .arg(format!("{:.1}", args.envelope.p1))
                .arg(format!("{:.1}", args.envelope.p2))
                .arg(format!("{:.1}", args.envelope.p3))
                .arg(format!("{:.1}", args.envelope.p4))
                .arg(format!("{:.1}", args.envelope.p5))
                .arg(format!("{:.0}", args.envelope.v1))
                .arg(format!("{:.0}", args.envelope.v2))
                .arg(format!("{:.0}", args.envelope.v3))
                .arg(format!("{:.0}", args.envelope.v4))
                .arg(format!("{:.0}", args.envelope.v5))
                .arg(format!("{:.1}", args.overlap_ms));

            let _ = cmd.output();
        }

        args.envelope
            .apply(note_samples, sample_rate, args.duration_ms);
    }
}
