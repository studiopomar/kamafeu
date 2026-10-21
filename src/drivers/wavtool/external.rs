use super::{load_wavtool_output, KnownWavtool, WavtoolArgs, WavtoolDriver};
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

/// Positional numeric arguments for the classic `wavtool2` command.
///
/// Kept in one place because both phrase concatenation and one-note execution
/// must preserve the UTAU CLI order. `offset_ms` and `overlap_ms` are supplied
/// by the caller because phrase concatenation handles the first segment and
/// negative offsets differently from the per-note path.
fn classic_wavtool_numeric_args(args: &WavtoolArgs, offset_ms: f64, overlap_ms: f64) -> [f64; 13] {
    let envelope = &args.envelope;
    [
        offset_ms,
        args.duration_ms,
        envelope.p1,
        envelope.p2,
        envelope.p3,
        envelope.v1,
        envelope.v2,
        envelope.v3,
        envelope.v4,
        overlap_ms,
        envelope.p4,
        envelope.p5,
        envelope.v5,
    ]
}

fn classic_wavtool_cli_args(
    args: &WavtoolArgs,
    offset_ms: f64,
    overlap_ms: f64,
    phrase_call: bool,
) -> [String; 13] {
    let numeric_args = classic_wavtool_numeric_args(args, offset_ms, overlap_ms);
    std::array::from_fn(|index| {
        let value = numeric_args[index];
        if phrase_call {
            format!("{value:.6}")
        } else if matches!(index, 5..=8 | 12) {
            format!("{value:.0}")
        } else {
            format!("{value:.1}")
        }
    })
}

fn describe_classic_wavtool_cli_args(values: &[String; 13]) -> String {
    format!(
        "offset={} length={} p1={} p2={} p3={} v1={} v2={} v3={} v4={} ovr={} p4={} p5={} v5={}",
        values[0],
        values[1],
        values[2],
        values[3],
        values[4],
        values[5],
        values[6],
        values[7],
        values[8],
        values[9],
        values[10],
        values[11],
        values[12]
    )
}

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
        let mut cmd = crate::drivers::process::prepare_command(executable)?;
        cmd.arg(path_arg(output)).arg(path_arg(&input));
        let overlap_ms = if index == 0 {
            0.0
        } else {
            item.overlap_ms.max(0.0)
        };
        let numeric_args =
            classic_wavtool_cli_args(item, item.skip_over_ms.max(0.0), overlap_ms, true);
        for value in &numeric_args {
            cmd.arg(value);
        }
        let result =
            crate::drivers::process::run_with_timeout(&mut cmd, Duration::from_secs(120), cancel)
                .map_err(|error| {
                format!(
                    "Wavtool {}, segmento {} ({}): {error}",
                    executable.display(),
                    index + 1,
                    describe_classic_wavtool_cli_args(&numeric_args)
                )
            })?;
        if !result.status.success() {
            return Err(format!(
                "Wavtool {}, segmento {} ({}) falhou: {}",
                executable.display(),
                index + 1,
                describe_classic_wavtool_cli_args(&numeric_args),
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
            cmd.arg(out_arg).arg(in_arg);
            let numeric_args =
                classic_wavtool_cli_args(args, args.skip_over_ms, args.overlap_ms, false);
            for value in &numeric_args {
                cmd.arg(value);
            }

            let output = crate::drivers::process::run_with_timeout(
                &mut cmd,
                Duration::from_secs(120),
                cancel,
            )
            .map_err(|error| {
                format!(
                    "wavtool falhou ({}): {error}",
                    describe_classic_wavtool_cli_args(&numeric_args)
                )
            })?;
            if !output.status.success() {
                return Err(format!(
                    "wavtool falhou ({}): {}",
                    describe_classic_wavtool_cli_args(&numeric_args),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::envelope::UtauEnvelope;

    #[test]
    fn classic_wavtool_numeric_arguments_follow_utau_cli_order() {
        let args = WavtoolArgs {
            output_wav: PathBuf::from("out.wav"),
            input_rendered_wav: PathBuf::from("in.wav"),
            skip_over_ms: 1.0,
            duration_ms: 2.0,
            envelope: UtauEnvelope {
                p1: 3.0,
                p2: 4.0,
                p3: 5.0,
                v1: 6.0,
                v2: 7.0,
                v3: 8.0,
                v4: 9.0,
                p4: 11.0,
                p5: 12.0,
                v5: 13.0,
                ..UtauEnvelope::default()
            },
            overlap_ms: 10.0,
            phoneme_envelope: [(0.0, 0.0); 5],
            sample_time_zero_ms: 0.0,
        };

        assert_eq!(
            classic_wavtool_numeric_args(&args, 1.0, 10.0),
            [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0]
        );
        assert_eq!(
            classic_wavtool_cli_args(&args, 1.0, 10.0, false),
            ["1.0", "2.0", "3.0", "4.0", "5.0", "6", "7", "8", "9", "10.0", "11.0", "12.0", "13"]
                .map(str::to_string)
        );
        assert_eq!(
            describe_classic_wavtool_cli_args(&classic_wavtool_cli_args(
                &args, 1.0, 10.0, false
            )),
            "offset=1.0 length=2.0 p1=3.0 p2=4.0 p3=5.0 v1=6 v2=7 v3=8 v4=9 ovr=10.0 p4=11.0 p5=12.0 v5=13"
        );
        assert_eq!(
            classic_wavtool_cli_args(&args, 1.0, 10.0, true),
            [
                "1.000000",
                "2.000000",
                "3.000000",
                "4.000000",
                "5.000000",
                "6.000000",
                "7.000000",
                "8.000000",
                "9.000000",
                "10.000000",
                "11.000000",
                "12.000000",
                "13.000000"
            ]
            .map(str::to_string)
        );
    }
}
