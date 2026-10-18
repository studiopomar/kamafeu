use super::{load_wavtool_output, KnownWavtool, WavtoolArgs, WavtoolDriver};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

/// Classic wavtools own concatenation: every call appends to the same phrase.
pub fn concatenate_external(
    executable: &Path,
    items: &[WavtoolArgs],
    output: &Path,
    sample_rate: u32,
    cancel: Option<&AtomicBool>,
) -> Result<Vec<f32>, String> {
    let is_exe = executable
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("exe"));
    let path_arg = |path: &Path| {
        if is_exe {
            crate::drivers::process::to_wine_windows_path(path)
        } else {
            path.as_os_str().to_owned()
        }
    };
    for (index, item) in items.iter().enumerate() {
        if cancel.is_some_and(|c| c.load(std::sync::atomic::Ordering::Relaxed)) {
            return Err("Renderização cancelada".into());
        }
        let mut input = item.input_rendered_wav.clone();
        if item.skip_over_ms < 0.0 {
            let (samples, rate) = crate::renderer::TrackRenderer::load_wav_samples(&input)?;
            let padding = (-item.skip_over_ms * rate as f64 / 1000.0).round() as usize;
            let mut padded = vec![0.0; padding];
            padded.extend(samples);
            input = output.with_file_name(format!("padded-{index}.wav"));
            crate::renderer::TrackRenderer::save_wav_samples(&input, &padded, rate)?;
        }
        let env = &item.envelope;
        let mut cmd = crate::drivers::process::prepare_command(executable)?;
        cmd.arg(path_arg(output)).arg(path_arg(&input));
        for value in [
            item.skip_over_ms.max(0.0),
            item.duration_ms,
            env.p1,
            env.p2,
            env.p3,
            env.v1,
            env.v2,
            env.v3,
            env.v4,
            if index == 0 {
                0.0
            } else {
                item.overlap_ms.max(0.0)
            },
            env.p4,
            env.p5,
            env.v5,
        ] {
            cmd.arg(format!("{value:.6}"));
        }
        let result =
            crate::drivers::process::run_with_timeout(&mut cmd, Duration::from_secs(120), cancel)?;
        if !result.status.success() {
            return Err(format!(
                "Wavtool {}, segmento {}: {}",
                executable.display(),
                index + 1,
                String::from_utf8_lossy(&result.stderr).trim()
            ));
        }
    }
    let header = PathBuf::from(format!("{}.whd", output.display()));
    let data = PathBuf::from(format!("{}.dat", output.display()));
    if header.exists() || data.exists() {
        let mut bytes = std::fs::read(&header).map_err(|e| format!("Cabeçalho wavtool: {e}"))?;
        bytes.extend(std::fs::read(&data).map_err(|e| format!("Dados wavtool: {e}"))?);
        std::fs::write(output, bytes).map_err(|e| e.to_string())?;
    }
    let (samples, rate) = crate::renderer::TrackRenderer::load_wav_samples(output)?;
    if samples.is_empty() {
        return Err("Wavtool produziu uma frase vazia".into());
    }
    Ok(crate::renderer::TrackRenderer::convert_sample_rate(
        &samples,
        rate,
        sample_rate,
    ))
}

pub struct ExternalWavtoolDriver {
    pub executable_path: PathBuf,
    display_name: String,
}

impl ExternalWavtoolDriver {
    pub fn new<P: AsRef<Path>>(path: P) -> Self {
        let path = path.as_ref().to_path_buf();
        let display_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("External Wavtool")
            .to_string();
        Self {
            executable_path: path,
            display_name,
        }
    }

    pub fn for_known(profile: KnownWavtool, configured_path: Option<PathBuf>) -> Self {
        let executable_path = configured_path
            .filter(|path| path.is_file())
            .or_else(|| profile.find_executable())
            .unwrap_or_else(|| profile.default_path());
        Self {
            executable_path,
            display_name: profile.label().to_string(),
        }
    }
}

impl WavtoolDriver for ExternalWavtoolDriver {
    fn engine_id(&self) -> &'static str {
        "external.wavtool"
    }

    fn phrase_executable(&self) -> Option<PathBuf> {
        Some(self.executable_path.clone())
    }
    fn name(&self) -> &str {
        &self.display_name
    }

    fn consumes_skip_over(&self) -> bool {
        true
    }

    fn process_note(
        &self,
        note_samples: &mut [f32],
        sample_rate: u32,
        args: &WavtoolArgs,
        cancel: Option<&AtomicBool>,
    ) -> Result<(), String> {
        let resolved_exe = if self.executable_path.is_file() {
            Some(self.executable_path.clone())
        } else {
            KnownWavtool::from_label(&self.display_name).and_then(|p| p.find_executable())
        };
        let final_exe = match resolved_exe {
            Some(p) => p,
            None => {
                return Err(format!(
                    "wavtool não encontrado: {}",
                    self.executable_path.display()
                ));
            }
        };
        {
            if !args.input_rendered_wav.exists() {
                crate::renderer::TrackRenderer::save_wav_samples(
                    &args.input_rendered_wav,
                    note_samples,
                    sample_rate,
                )?;
            }

            let is_exe = final_exe
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.eq_ignore_ascii_case("exe"))
                .unwrap_or(false);

            let out_arg = if is_exe {
                crate::drivers::process::to_wine_windows_path(&args.output_wav)
            } else {
                args.output_wav.as_os_str().to_owned()
            };
            let in_arg = if is_exe {
                crate::drivers::process::to_wine_windows_path(&args.input_rendered_wav)
            } else {
                args.input_rendered_wav.as_os_str().to_owned()
            };

            let mut cmd = crate::drivers::process::prepare_command(&final_exe)?;
            cmd.arg(out_arg)
                .arg(in_arg)
                .arg(format!("{:.1}", args.skip_over_ms))
                .arg(format!("{:.1}", args.duration_ms))
                .arg(format!("{:.1}", args.envelope.p1))
                .arg(format!("{:.1}", args.envelope.p2))
                .arg(format!("{:.1}", args.envelope.p3))
                .arg(format!("{:.0}", args.envelope.v1))
                .arg(format!("{:.0}", args.envelope.v2))
                .arg(format!("{:.0}", args.envelope.v3))
                .arg(format!("{:.0}", args.envelope.v4))
                .arg(format!("{:.1}", args.overlap_ms))
                .arg(format!("{:.1}", args.envelope.p4))
                .arg(format!("{:.1}", args.envelope.p5))
                .arg(format!("{:.0}", args.envelope.v5));

            let output = crate::drivers::process::run_with_timeout(
                &mut cmd,
                Duration::from_secs(120),
                cancel,
            )?;
            if !output.status.success() {
                return Err(format!(
                    "wavtool falhou: {}",
                    String::from_utf8_lossy(&output.stderr).trim()
                ));
            }
        }

        // External engines already applied the UTAU envelope.  Their WAV must
        // become the phone audio before the renderer mixes it into the track.
        load_wavtool_output(note_samples, sample_rate, &args.output_wav)?;
        Ok(())
    }
}
