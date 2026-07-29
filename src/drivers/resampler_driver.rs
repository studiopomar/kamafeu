use std::path::{Path, PathBuf};
use std::process::Command;
use crate::dsp::resampler::Resampler;

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
    pub consonant_ms: f64,
    pub cutoff_ms: f64,
    pub volume: f64,
    pub modulation: f64,
    pub tempo: f64,
    pub pitch_bend_str: String,
    pub pitch_points: Vec<crate::project::model::UPitchBendPoint>,
}

pub trait ResamplerDriver: Send + Sync {
    fn name(&self) -> &str;
    fn render_sample(&self, raw_samples: &[f32], sample_rate: u32, args: &ResamplerArgs) -> Result<Vec<f32>, String>;
}

/// Native Rust Resampler Driver (TD-PSOLA)
pub struct NativeResamplerDriver;

impl ResamplerDriver for NativeResamplerDriver {
    fn name(&self) -> &str {
        "Native Rust (TD-PSOLA)"
    }

    fn render_sample(&self, raw_samples: &[f32], sample_rate: u32, args: &ResamplerArgs) -> Result<Vec<f32>, String> {
        let rendered = Resampler::render_sample_with_pitch_bend(
            raw_samples,
            sample_rate,
            args.offset_ms,
            args.consonant_ms,
            args.cutoff_ms,
            args.duration_ms,
            args.pitch_freq,
            &args.pitch_points,
        );
        Ok(rendered)
    }
}

/// MacRes Resampler Driver (https://github.com/titinko/macres)
pub struct MacResDriver {
    pub executable_path: PathBuf,
}

impl MacResDriver {
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

    /// Auto-detect macres executable in system PATH or local folders
    pub fn find_executable() -> Option<PathBuf> {
        let candidates = [
            PathBuf::from("macres"),
            PathBuf::from("./resamplers/macres"),
            PathBuf::from("./macres"),
            PathBuf::from("/opt/homebrew/bin/macres"),
            PathBuf::from("/usr/local/bin/macres"),
        ];

        for cand in candidates {
            if cand.exists() {
                return Some(cand);
            }
        }
        None
    }
}

impl ResamplerDriver for MacResDriver {
    fn name(&self) -> &str {
        "macres (titinko/macres)"
    }

    fn render_sample(&self, raw_samples: &[f32], sample_rate: u32, args: &ResamplerArgs) -> Result<Vec<f32>, String> {
        if !self.executable_path.exists() {
            // Fallback to Native TD-PSOLA if macres binary is missing
            let native = NativeResamplerDriver;
            return native.render_sample(raw_samples, sample_rate, args);
        }

        let actual_input_wav = if args.input_wav.exists() {
            args.input_wav.clone()
        } else {
            let temp_input = std::env::temp_dir().join(format!("kamafeu_input_{}.wav", args.pitch_name));
            let _ = crate::renderer::TrackRenderer::save_wav_samples(&temp_input, raw_samples, sample_rate);
            temp_input
        };

        let mut cmd = Command::new(&self.executable_path);
        cmd.arg(&actual_input_wav)
            .arg(&args.output_wav)
            .arg(&args.pitch_name)
            .arg(format!("{:.0}", args.velocity))
            .arg(if args.flags.is_empty() { "g0" } else { &args.flags })
            .arg(format!("{:.1}", args.offset_ms))
            .arg(format!("{:.1}", args.duration_ms))
            .arg(format!("{:.1}", args.consonant_ms))
            .arg(format!("{:.1}", args.cutoff_ms))
            .arg(format!("{:.0}", args.volume))
            .arg(format!("{:.0}", args.modulation))
            .arg(format!("{:.1}", args.tempo))
            .arg(if args.pitch_bend_str.is_empty() { "" } else { &args.pitch_bend_str });

        let output = cmd.output().map_err(|e| format!("Failed to execute macres at {:?}: {}", self.executable_path, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("macres execution failed: {}", stderr));
        }

        if args.output_wav.exists() {
            if let Ok((samples, _)) = crate::renderer::track::TrackRenderer::load_wav_samples(&args.output_wav) {
                if !samples.is_empty() {
                    return Ok(samples);
                }
            }
        }
        
        let native = NativeResamplerDriver;
        native.render_sample(raw_samples, sample_rate, args)
    }
}

/// Generic External Resampler Driver
pub struct ExternalResamplerDriver {
    pub executable_path: PathBuf,
}

impl ExternalResamplerDriver {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        Self {
            executable_path: path.as_ref().to_path_buf(),
        }
    }
}

impl ResamplerDriver for ExternalResamplerDriver {
    fn name(&self) -> &str {
        self.executable_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("External Resampler")
    }

    fn render_sample(&self, raw_samples: &[f32], sample_rate: u32, args: &ResamplerArgs) -> Result<Vec<f32>, String> {
        if !self.executable_path.exists() {
            let native = NativeResamplerDriver;
            return native.render_sample(raw_samples, sample_rate, args);
        }

        let mut cmd = Command::new(&self.executable_path);

        cmd.arg(&args.input_wav)
            .arg(&args.output_wav)
            .arg(&args.pitch_name)
            .arg(format!("{:.0}", args.velocity))
            .arg(if args.flags.is_empty() { "g0" } else { &args.flags })
            .arg(format!("{:.1}", args.offset_ms))
            .arg(format!("{:.1}", args.duration_ms))
            .arg(format!("{:.1}", args.consonant_ms))
            .arg(format!("{:.1}", args.cutoff_ms))
            .arg(format!("{:.0}", args.volume))
            .arg(format!("{:.0}", args.modulation))
            .arg(format!("{:.1}", args.tempo))
            .arg(if args.pitch_bend_str.is_empty() { "" } else { &args.pitch_bend_str });

        let output = cmd.output().map_err(|e| format!("Failed to execute external resampler {:?}: {}", self.executable_path, e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("External resampler failed: {}", stderr));
        }

        if args.output_wav.exists() {
            let (samples, _) = crate::renderer::track::TrackRenderer::load_wav_samples(&args.output_wav)?;
            Ok(samples)
        } else {
            let native = NativeResamplerDriver;
            native.render_sample(raw_samples, sample_rate, args)
        }
    }
}
