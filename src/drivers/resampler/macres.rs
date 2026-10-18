use super::{
    actual_input_wav, classic_arguments, executable_cache_identity, load_resampler_output,
    KnownResampler, ResamplerArgs, ResamplerDriver,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

pub struct MacResDriver {
    pub executable_path: PathBuf,
}

impl MacResDriver {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let p = path.as_ref();
        let final_path = if p.exists() && p.is_file() {
            p.to_path_buf()
        } else {
            Self::find_executable().unwrap_or_else(|| p.to_path_buf())
        };
        Self {
            executable_path: final_path,
        }
    }

    pub fn find_executable() -> Option<PathBuf> {
        KnownResampler::MacRes.find_executable()
    }
}

impl ResamplerDriver for MacResDriver {
    fn name(&self) -> &str {
        "macres (titinko/macres)"
    }

    fn engine_id(&self) -> &'static str {
        KnownResampler::MacRes.engine_id()
    }

    fn cache_identity(&self) -> String {
        executable_cache_identity(self.name(), &self.executable_path)
    }

    fn uses_external_process(&self) -> bool {
        true
    }

    fn render_sample(
        &self,
        raw_samples: &[f32],
        sample_rate: u32,
        args: &ResamplerArgs,
        cancel: Option<&AtomicBool>,
    ) -> Result<Vec<f32>, String> {
        if !self.executable_path.is_file() {
            return Err(format!(
                "Resampler não encontrado: {}",
                self.executable_path.display()
            ));
        }

        let is_exe = self
            .executable_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("exe"))
            .unwrap_or(false);

        let mut temp_input_dir = None;
        let actual_input_wav =
            actual_input_wav(raw_samples, sample_rate, args, is_exe, &mut temp_input_dir)?;
        if args.output_wav.is_file() {
            let _ = std::fs::remove_file(&args.output_wav);
        }
        let mut cmd = crate::drivers::process::prepare_command(&self.executable_path)?;

        cmd.args(classic_arguments(
            &actual_input_wav,
            args,
            "g0",
            args.duration_ms,
            is_exe,
        ));

        let dynamic_timeout = Duration::from_secs(15 + (args.duration_ms / 1000.0).round() as u64);
        let output = crate::drivers::process::run_with_timeout(&mut cmd, dynamic_timeout, cancel)?;

        if !output.status.success() {
            return Err(format!(
                "Resampler {} falhou: {}",
                self.name(),
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }

        load_resampler_output(args, sample_rate)
    }
}
