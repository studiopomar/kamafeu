use super::remove_flag;
use super::{
    actual_input_wav, classic_arguments, executable_cache_identity, load_resampler_output,
    prepare_classic_flags, KnownResampler, ResamplerArgs, ResamplerDriver,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

pub struct ExternalResamplerDriver {
    pub executable_path: PathBuf,
    display_name: String,
    profile: Option<KnownResampler>,
    empty_flags: &'static str,
    duration_excludes_consonant: bool,
}

impl ExternalResamplerDriver {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref().to_path_buf();
        let display_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Resampler externo")
            .to_string();
        Self {
            executable_path: path,
            display_name,
            profile: None,
            empty_flags: "g0",
            duration_excludes_consonant: false,
        }
    }

    pub fn for_known(profile: KnownResampler, configured_path: Option<PathBuf>) -> Self {
        let executable_path = configured_path
            .filter(|path| path.is_file())
            .or_else(|| profile.find_executable())
            .unwrap_or_else(|| profile.default_path());
        Self {
            executable_path,
            display_name: profile.label().to_string(),
            profile: Some(profile),
            empty_flags: match profile {
                KnownResampler::Organum => "-",
                KnownResampler::Tips => "",
                _ => "g0",
            },
            // straycat-rs treats `length` as vowel/stretch length and adds the
            // rendered consonant to it, unlike the other classic engines.
            duration_excludes_consonant: profile == KnownResampler::StraycatRs,
        }
    }

    pub(super) fn requested_duration_ms(&self, args: &ResamplerArgs) -> f64 {
        if self.duration_excludes_consonant {
            (args.duration_ms - args.consonant_ms).max(1.0)
        } else {
            args.duration_ms
        }
    }
}

fn prepare_straycat_flags(base_flags: &str, gender: f64, breathiness: f64) -> String {
    let mut flags = base_flags.to_string();
    if gender != 0.0 {
        flags.push_str(&format!("g{gender:.0}"));
    }

    // In straycat-rs, B50 is neutral. Kamafeu exposes breathiness as an
    // additive 0..100 expression where 0 means "do not alter the singer".
    // Sending the raw value caused B1..B49 to amplify the harmonic component,
    // making low breathiness settings sound more metallic instead of airier.
    if breathiness > 0.0 {
        flags = remove_flag(&flags, "B");
        let straycat_breathiness = (50.0 + breathiness.clamp(0.0, 100.0) * 0.5).round() as i32;
        flags.push_str(&format!("B{straycat_breathiness}"));
    }

    flags
}

fn prepare_hifisampler_flags(base_flags: &str, gender: f64, breathiness: f64) -> String {
    let mut flags = base_flags.to_string();
    if gender != 0.0 {
        flags.push_str(&format!("g{gender:.0}"));
    }
    if breathiness != 0.0 {
        // Hifisampler uses Hb (100 is default/neutral 100%, 0..500)
        let val = (100.0 + breathiness * 2.0).clamp(0.0, 500.0);
        flags.push_str(&format!("Hb{val:.0}"));
    }
    flags
}

fn uses_nsf_hifigan(profile: Option<KnownResampler>) -> bool {
    matches!(
        profile,
        Some(KnownResampler::Catalina | KnownResampler::HifisamplerRs)
    )
}

static HIFISERVER_PROCESS: std::sync::LazyLock<std::sync::Mutex<Option<std::process::Child>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

pub fn shutdown_hifisampler_server() {
    if let Ok(mut guard) = HIFISERVER_PROCESS.lock() {
        if let Some(mut child) = guard.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

pub fn ensure_hifisampler_server_running(exe_dir: &Path) {
    let server_candidates = [
        "hifiserver-rust",
        "hifiserver-rust.exe",
        "hifiserver",
        "hifiserver.exe",
        "hifisampler-server",
        "hifisampler-server.exe",
    ];
    let mut server_exe = None;
    for cand in server_candidates {
        let p = exe_dir.join(cand);
        if p.is_file() {
            server_exe = Some(p);
            break;
        }
    }

    let Some(server_exe) = server_exe else {
        return;
    };

    if let Ok(mut guard) = HIFISERVER_PROCESS.lock() {
        if let Some(child) = guard.as_mut() {
            if child.try_wait().ok().flatten().is_none() {
                return; // Já está em execução
            }
        }

        if let Ok(mut cmd) = crate::drivers::process::prepare_command(&server_exe) {
            cmd.current_dir(exe_dir);
            if let Ok(child) = cmd.spawn() {
                *guard = Some(child);
            }
        }
    }
}

pub fn ensure_hifisampler_environment(exe_path: &Path) {
    let Some(dir) = exe_path.parent() else {
        return;
    };

    let config_path = dir.join("hificonfig.ini");
    if !config_path.is_file() {
        let default_config = "\
vocoder_path = ./model/pc_nsf_hifigan_44.1k_hop512_128bin_2025.02.onnx
hnsep_path = ./model/hnsep_model.onnx

wave_norm = true
trim_silence = true
silence_threshold = -52.0
loop_mode = true
peak_limit = 1.0
fill = 6

max_workers = 4
";
        let _ = std::fs::write(&config_path, default_config);
    }

    ensure_hifisampler_server_running(dir);
}

pub fn ensure_hifisampler_ready() -> Result<String, String> {
    let exe = KnownResampler::Catalina
        .find_executable()
        .or_else(|| KnownResampler::HifisamplerRs.find_executable());

    let Some(exe_path) = exe else {
        return Err("Executável do Hifisampler não encontrado em ./resamplers".to_string());
    };

    let dir = exe_path.parent().unwrap_or_else(|| Path::new("."));
    ensure_hifisampler_environment(&exe_path);

    let model_dir = dir.join("model");
    let model_found = model_dir
        .join("pc_nsf_hifigan_44.1k_hop512_128bin_2025.02.onnx")
        .is_file()
        || model_dir.join("pc-nsf-hifigan.onnx").is_file()
        || model_dir.join("model.onnx").is_file();

    if model_found {
        Ok(format!(
            "Hifisampler pronto (Modelos ONNX carregados em {})",
            model_dir.display()
        ))
    } else {
        Ok("Hifisampler configurado (Aguardando modelo ONNX em ./resamplers/model)".to_string())
    }
}

static MORESAMPLER_CONFIGURED: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashSet<PathBuf>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashSet::new()));

fn ensure_moresampler_config(parent_dir: &Path) {
    if let Ok(guard) = MORESAMPLER_CONFIGURED.lock() {
        if guard.contains(parent_dir) {
            return;
        }
    }

    let moreconfig = parent_dir.join("moreconfig.txt");
    let needs_fix = if moreconfig.is_file() {
        std::fs::read_to_string(&moreconfig)
            .map(|content| !content.contains("resampler-compatibility on"))
            .unwrap_or(true)
    } else {
        true
    };
    if needs_fix {
        let _ = std::fs::write(&moreconfig, "resampler-compatibility on\n");
    }

    if let Ok(mut guard) = MORESAMPLER_CONFIGURED.lock() {
        guard.insert(parent_dir.to_path_buf());
    }
}

impl ResamplerDriver for ExternalResamplerDriver {
    fn name(&self) -> &str {
        &self.display_name
    }

    fn engine_id(&self) -> &'static str {
        self.profile
            .map(KnownResampler::engine_id)
            .unwrap_or("external.resampler.unknown")
    }

    fn prepare_flags(&self, base_flags: &str, gender: f64, breathiness: f64) -> String {
        if self.profile == Some(KnownResampler::StraycatRs) {
            prepare_straycat_flags(base_flags, gender, breathiness)
        } else if uses_nsf_hifigan(self.profile) {
            prepare_hifisampler_flags(base_flags, gender, breathiness)
        } else {
            prepare_classic_flags(base_flags, gender, breathiness)
        }
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
        let resolved_exe = if self.executable_path.is_file() {
            Some(self.executable_path.clone())
        } else {
            KnownResampler::from_label(&self.display_name).and_then(|p| p.find_executable())
        };

        let final_exe = resolved_exe.ok_or_else(|| {
            format!(
                "Resampler {} não encontrado: {}",
                self.display_name,
                self.executable_path.display()
            )
        })?;

        let is_exe = final_exe
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("exe"))
            .unwrap_or(false);

        if let Some(parent) = final_exe.parent() {
            if uses_nsf_hifigan(self.profile) {
                ensure_hifisampler_environment(&final_exe);
            }
            if is_exe {
                let stem = final_exe.file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if stem.eq_ignore_ascii_case("moresampler")
                    || stem.eq_ignore_ascii_case("openmoresampler")
                {
                    ensure_moresampler_config(parent);
                }
            }
        }

        let mut temp_input_dir = None;
        let actual_input_wav =
            actual_input_wav(raw_samples, sample_rate, args, is_exe, &mut temp_input_dir)?;
        if args.output_wav.is_file() {
            let _ = std::fs::remove_file(&args.output_wav);
        }
        let mut cmd = crate::drivers::process::prepare_command(&final_exe)?;
        if let Some(parent) = final_exe.parent() {
            cmd.current_dir(parent);
        }
        let requested_duration_ms = self.requested_duration_ms(args);
        cmd.args(classic_arguments(
            &actual_input_wav,
            args,
            self.empty_flags,
            requested_duration_ms,
            is_exe,
        ));

        // Timeout dinâmico: 30 segundos base + 1 segundo por segundo de áudio requisitado
        let dynamic_timeout =
            Duration::from_secs(30 + (requested_duration_ms / 1000.0).round() as u64);
        let output = crate::drivers::process::run_with_timeout(&mut cmd, dynamic_timeout, cancel)?;

        if !output.status.success() {
            let stderr_msg = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let stdout_msg = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let detail = if !stderr_msg.is_empty() {
                stderr_msg
            } else if !stdout_msg.is_empty() {
                stdout_msg
            } else {
                format!("código de saída {}", output.status)
            };
            return Err(format!("Resampler {} falhou: {}", self.name(), detail));
        }

        if !args.output_wav.is_file() {
            let stdout_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let stderr_str = String::from_utf8_lossy(&output.stderr).trim().to_string();
            let msg = if !stderr_str.is_empty() {
                format!("o resampler não criou o WAV de saída: {stderr_str}")
            } else if !stdout_str.is_empty() {
                format!("o resampler não criou o WAV de saída (stdout: {stdout_str})")
            } else {
                "o resampler não criou o WAV de saída".to_string()
            };
            return Err(msg);
        }

        load_resampler_output(args, sample_rate)
    }
}
