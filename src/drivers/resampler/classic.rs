use super::ResamplerArgs;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Human-readable command description used by diagnostics and render manifests.
/// It is intentionally not an executable shell command: paths are quoted for
/// display and no interpolation is ever passed back to a shell.
pub fn describe_resampler_command(driver: &str, args: &ResamplerArgs) -> String {
    format!(
        "{driver} --input {} --output {} --pitch {} --velocity {:.0} --flags '{}' --offset-ms {:.3} --duration-ms {:.3} --consonant-ms {:.3} --cutoff-ms {:.3} --norm {} --tempo {:.3}",
        quote_path(&args.input_wav),
        quote_path(&args.output_wav),
        args.pitch_name,
        args.velocity,
        effective_classic_flags(args),
        args.offset_ms,
        args.duration_ms,
        args.consonant_ms,
        args.cutoff_ms,
        args.normalization
            .map_or_else(|| "default".to_string(), |value| format!("{value:.3}")),
        args.tempo,
    )
}

/// Build the literal flag field sent through the classic UTAU protocol.
/// `NORM` is deliberately represented in the same field as authored flags so
/// external engines receive the per-phoneme value without a host-side gain
/// or normalization pass.
pub fn effective_classic_flags(args: &ResamplerArgs) -> String {
    let mut flags = args.flags.clone();
    if let Some(normalization) = args.normalization {
        if normalization.is_finite() {
            flags.push_str(&format!("NORM={normalization}"));
        }
    }
    flags
}

fn quote_path(path: &Path) -> String {
    format!("\"{}\"", path.to_string_lossy().replace('"', "\\\""))
}

pub(super) fn prepare_classic_flags(base_flags: &str, gender: f64, breathiness: f64) -> String {
    let mut flags = base_flags.to_string();
    if gender != 0.0 {
        flags.push_str(&format!("g{gender:.0}"));
    }
    if breathiness != 0.0 {
        flags.push_str(&format!("B{:.0}", breathiness.abs()));
    }
    flags
}

pub(super) fn executable_cache_identity(name: &str, path: &Path) -> String {
    let content = std::fs::read(path);
    let fingerprint = content
        .as_deref()
        .map(content_fingerprint)
        .unwrap_or_else(|_| "missing".to_string());
    format!("{name}:{}:content:{fingerprint}", path.display())
}

fn content_fingerprint(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

pub(super) fn actual_input_wav(
    raw_samples: &[f32],
    sample_rate: u32,
    args: &ResamplerArgs,
    is_wine_exe: bool,
    temp_dir: &mut Option<tempfile::TempDir>,
) -> Result<PathBuf, String> {
    // If the path contains non-ASCII characters (e.g. Japanese voicebank filenames like あ.wav)
    // or when running under Wine (which uses ANSI codepage on CLI args), copy or write
    // to a clean ASCII temporary path so external resamplers (TIPS, Moresampler, etc.) can open it.
    let path_str = args.input_wav.to_string_lossy();
    let is_non_ascii = !path_str.is_ascii();

    if args.input_wav.is_file() && !is_non_ascii && !is_wine_exe {
        return Ok(args.input_wav.clone());
    }

    let directory = tempfile::Builder::new()
        .prefix("kamafeu-resampler-")
        .tempdir()
        .map_err(|error| format!("Falha ao criar diretório temporário: {error}"))?;
    let path = directory.path().join("input.wav");

    if args.input_wav.is_file() {
        let _ = std::fs::copy(&args.input_wav, &path);
    } else if !raw_samples.is_empty() {
        crate::renderer::TrackRenderer::save_wav_samples(&path, raw_samples, sample_rate)?;
    } else if args.input_wav.exists() {
        let _ = std::fs::copy(&args.input_wav, &path);
    }

    *temp_dir = Some(directory);
    Ok(path)
}

pub(super) fn classic_arguments(
    input_wav: &Path,
    args: &ResamplerArgs,
    empty_flags: &str,
    duration_ms: f64,
    is_wine_exe: bool,
) -> Vec<OsString> {
    let in_arg = if is_wine_exe {
        crate::drivers::process::to_wine_windows_path(input_wav)
    } else {
        input_wav.as_os_str().to_owned()
    };
    let out_arg = if is_wine_exe {
        crate::drivers::process::to_wine_windows_path(&args.output_wav)
    } else {
        args.output_wav.as_os_str().to_owned()
    };
    vec![
        in_arg,
        out_arg,
        args.pitch_name.clone().into(),
        format!("{:.0}", args.velocity).into(),
        if effective_classic_flags(args).is_empty() {
            empty_flags.into()
        } else {
            effective_classic_flags(args).into()
        },
        format!("{:.1}", args.offset_ms).into(),
        format!("{:.0}", duration_ms.round()).into(),
        format!("{:.1}", args.source_consonant_ms).into(),
        format!("{:.1}", args.cutoff_ms).into(),
        format!("{:.0}", args.volume).into(),
        format!("{:.0}", args.modulation).into(),
        // Classic UTAU resamplers expect the tempo marker with a leading `!`.
        format!("!{:.1}", args.tempo).into(),
        if args.pitch_bend_str.is_empty() {
            "AA".into()
        } else {
            args.pitch_bend_str.clone().into()
        },
    ]
}

pub(super) fn load_resampler_output(
    args: &ResamplerArgs,
    expected_sample_rate: u32,
) -> Result<Vec<f32>, String> {
    if !args.output_wav.is_file() {
        return Err("o resampler não criou o WAV de saída".to_string());
    }
    let (mut samples, output_sample_rate) =
        crate::renderer::track::TrackRenderer::load_wav_samples(&args.output_wav)?;
    if samples.is_empty() {
        Err("o resampler criou um WAV vazio".to_string())
    } else {
        if output_sample_rate != expected_sample_rate {
            samples = crate::renderer::track::TrackRenderer::convert_sample_rate(
                &samples,
                output_sample_rate,
                expected_sample_rate,
            );
        }
        Ok(samples)
    }
}
