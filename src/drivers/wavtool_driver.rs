use std::path::{Path, PathBuf};
use std::process::Command;
use crate::dsp::envelope::UtauEnvelope;

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

/// Native Rust Wavtool Driver (crossfade & 5-point UTAU envelope)
pub struct NativeWavtoolDriver;

impl WavtoolDriver for NativeWavtoolDriver {
    fn name(&self) -> &str {
        "Native Rust (Crossfader)"
    }

    fn process_note(&self, note_samples: &mut [f32], sample_rate: u32, args: &WavtoolArgs) {
        if note_samples.is_empty() {
            return;
        }

        // 1. Apply 5-point UTAU envelope (amplitude shaping over note duration)
        args.envelope.apply(note_samples, sample_rate, args.duration_ms);

        // 2. Smooth equal-power cosine fade-in (overlap/preutterance region)
        let overlap_ms = if args.overlap_ms > 0.0 { args.overlap_ms } else { 45.0 };
        let fade_in_samples = ((overlap_ms / 1000.0) * sample_rate as f64) as usize;
        let fade_in_len = fade_in_samples.min(note_samples.len() / 2).max(1);

        for i in 0..fade_in_len {
            let t = i as f32 / fade_in_len as f32;
            // Equal-power cosine curve: smooth sine ramp
            let gain = (t * std::f32::consts::FRAC_PI_2).sin();
            note_samples[i] *= gain;
        }

        // 3. Smooth equal-power cosine fade-out (release tail for crossfade with next note)
        let fade_out_ms = overlap_ms.min(35.0); // release tail: shorter than attack for natural decay
        let fade_out_samples = ((fade_out_ms / 1000.0) * sample_rate as f64) as usize;
        let fade_out_len = fade_out_samples.min(note_samples.len() / 2).max(1);
        let total_len = note_samples.len();

        for i in 0..fade_out_len {
            let t = i as f32 / fade_out_len as f32;
            // Equal-power cosine curve: smooth cosine ramp down
            let gain = (t * std::f32::consts::FRAC_PI_2).cos();
            note_samples[total_len - 1 - i] *= gain;
        }
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

        for cand in candidates {
            if cand.exists() {
                return Some(cand);
            }
        }
        None
    }
}

impl WavtoolDriver for WavtoolYawuDriver {
    fn name(&self) -> &str {
        "wavtool-yawu (m13253/wavtool-yawu)"
    }

    fn process_note(&self, note_samples: &mut [f32], sample_rate: u32, args: &WavtoolArgs) {
        if self.executable_path.exists() {
            if !args.input_rendered_wav.exists() {
                let _ = crate::renderer::TrackRenderer::save_wav_samples(&args.input_rendered_wav, note_samples, sample_rate);
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
        args.envelope.apply(note_samples, sample_rate, args.duration_ms);
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

        args.envelope.apply(note_samples, sample_rate, args.duration_ms);
    }
}
