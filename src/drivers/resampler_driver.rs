use crate::dsp::resampler::Resampler;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    /// Fixed consonant boundary authored in oto.ini.
    pub source_consonant_ms: f64,
    /// Rendered consonant duration after applying consonant velocity.
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
    fn render_sample(
        &self,
        raw_samples: &[f32],
        sample_rate: u32,
        args: &ResamplerArgs,
    ) -> Result<Vec<f32>, String>;
}

/// Open-source resamplers that implement the classic UTAU command-line contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnownResampler {
    MacRes,
    Organum,
    StraycatRs,
    World4Utau,
}

impl KnownResampler {
    pub const ALL: [Self; 4] = [
        Self::MacRes,
        Self::Organum,
        Self::StraycatRs,
        Self::World4Utau,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::MacRes => "macres (titinko/macres)",
            Self::Organum => "Organum (KakouLabs/Organum)",
            Self::StraycatRs => "straycat-rs (UtaUtaUtau)",
            Self::World4Utau => "World4UTAU (xrdavies/world4utau)",
        }
    }

    pub const fn executable_names(self) -> &'static [&'static str] {
        match self {
            Self::MacRes => &["macres", "macres.exe"],
            Self::Organum => &["organum-resampler", "organum-resampler.exe"],
            Self::StraycatRs => &["straycat-rs", "straycat-rs.exe"],
            Self::World4Utau => &["world4utau", "world4utau.exe"],
        }
    }

    pub fn from_label(label: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|profile| profile.label() == label)
    }

    pub fn default_path(self) -> PathBuf {
        PathBuf::from("./resamplers").join(self.executable_names()[0])
    }

    /// Searches the Kamafeu folder, PATH, Downloads and OpenUtau's resampler folder.
    pub fn find_executable(self) -> Option<PathBuf> {
        let mut roots = vec![
            PathBuf::from("./resamplers"),
            PathBuf::from("."),
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/usr/bin"),
        ];

        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            roots.push(home.join("Downloads"));
            roots.push(home.join(".local/bin"));
            roots.push(home.join("Library/Application Support/OpenUTAU/Resamplers"));
            roots.push(home.join("Library/Application Support/OpenUtau/Resamplers"));
        }

        for root in roots {
            for executable in self.executable_names() {
                let candidate = root.join(executable);
                if candidate.is_file() {
                    return Some(candidate);
                }
            }
        }

        for executable in self.executable_names() {
            if let Ok(output) = Command::new("which").arg(executable).output() {
                if output.status.success() {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_owned();
                    let candidate = PathBuf::from(path);
                    if candidate.is_file() {
                        return Some(candidate);
                    }
                }
            }
        }
        None
    }
}

fn actual_input_wav(
    raw_samples: &[f32],
    sample_rate: u32,
    args: &ResamplerArgs,
    temp_dir: &mut Option<tempfile::TempDir>,
) -> Result<PathBuf, String> {
    if args.input_wav.is_file() {
        return Ok(args.input_wav.clone());
    }

    let directory = tempfile::Builder::new()
        .prefix("kamafeu-resampler-")
        .tempdir()
        .map_err(|error| format!("Falha ao criar diretório temporário: {error}"))?;
    let path = directory.path().join("input.wav");
    crate::renderer::TrackRenderer::save_wav_samples(&path, raw_samples, sample_rate)?;
    *temp_dir = Some(directory);
    Ok(path)
}

fn classic_arguments(
    input_wav: &Path,
    args: &ResamplerArgs,
    empty_flags: &str,
    duration_ms: f64,
) -> Vec<OsString> {
    vec![
        input_wav.as_os_str().to_owned(),
        args.output_wav.as_os_str().to_owned(),
        args.pitch_name.clone().into(),
        format!("{:.0}", args.velocity).into(),
        if args.flags.is_empty() {
            empty_flags.into()
        } else {
            args.flags.clone().into()
        },
        format!("{:.1}", args.offset_ms).into(),
        format!("{duration_ms:.1}").into(),
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

fn load_resampler_output(
    args: &ResamplerArgs,
    expected_sample_rate: u32,
) -> Result<Vec<f32>, String> {
    if !args.output_wav.is_file() {
        return Err("o resampler não criou o WAV de saída".to_string());
    }
    let (mut samples, output_sample_rate) =
        crate::renderer::track::TrackRenderer::load_wav_samples(&args.output_wav)?;
    let _ = std::fs::remove_file(&args.output_wav);
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

/// Native Rust Resampler Driver (TD-PSOLA)
pub struct NativeResamplerDriver;

impl ResamplerDriver for NativeResamplerDriver {
    fn name(&self) -> &str {
        "Native Rust (TD-PSOLA)"
    }

    fn render_sample(
        &self,
        raw_samples: &[f32],
        sample_rate: u32,
        args: &ResamplerArgs,
    ) -> Result<Vec<f32>, String> {
        let rendered = Resampler::render_sample_with_pitch_bend_and_consonant_timing(
            raw_samples,
            sample_rate,
            args.offset_ms,
            args.source_consonant_ms,
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
        let final_path = if p.exists() && p.is_file() {
            p.to_path_buf()
        } else {
            Self::find_executable().unwrap_or_else(|| p.to_path_buf())
        };
        Self {
            executable_path: final_path,
        }
    }

    /// Auto-detect macres executable in system PATH, user Downloads, OpenUTAU or local folders
    pub fn find_executable() -> Option<PathBuf> {
        KnownResampler::MacRes.find_executable()
    }
}

impl ResamplerDriver for MacResDriver {
    fn name(&self) -> &str {
        "macres (titinko/macres)"
    }

    fn render_sample(
        &self,
        raw_samples: &[f32],
        sample_rate: u32,
        args: &ResamplerArgs,
    ) -> Result<Vec<f32>, String> {
        if !self.executable_path.exists() {
            eprintln!(
                "[macres] Binary not found at {:?}, falling back to Native TD-PSOLA",
                self.executable_path
            );
            let native = NativeResamplerDriver;
            return native.render_sample(raw_samples, sample_rate, args);
        }

        let mut temp_input_dir = None;
        let actual_input_wav =
            actual_input_wav(raw_samples, sample_rate, args, &mut temp_input_dir)?;
        if args.output_wav.is_file() {
            let _ = std::fs::remove_file(&args.output_wav);
        }
        let mut cmd = Command::new(&self.executable_path);
        cmd.args(classic_arguments(
            &actual_input_wav,
            args,
            "g0",
            args.duration_ms,
        ));

        let output = cmd.output().map_err(|e| {
            format!(
                "Failed to execute macres at {:?}: {}",
                self.executable_path, e
            )
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!(
                "[macres] execution failed: {}, falling back to Native TD-PSOLA",
                stderr
            );
            let native = NativeResamplerDriver;
            return native.render_sample(raw_samples, sample_rate, args);
        }

        match load_resampler_output(args, sample_rate) {
            Ok(samples) => Ok(samples),
            Err(error) => {
                eprintln!("[macres] {error}; usando Native TD-PSOLA");
                NativeResamplerDriver.render_sample(raw_samples, sample_rate, args)
            }
        }
    }
}

/// Generic External Resampler Driver
pub struct ExternalResamplerDriver {
    pub executable_path: PathBuf,
    display_name: String,
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
            empty_flags: if profile == KnownResampler::Organum {
                "-"
            } else {
                "g0"
            },
            // straycat-rs treats `length` as vowel/stretch length and adds the
            // rendered consonant to it, unlike the other classic engines.
            duration_excludes_consonant: profile == KnownResampler::StraycatRs,
        }
    }

    fn requested_duration_ms(&self, args: &ResamplerArgs) -> f64 {
        if self.duration_excludes_consonant {
            (args.duration_ms - args.consonant_ms).max(1.0)
        } else {
            args.duration_ms
        }
    }
}

impl ResamplerDriver for ExternalResamplerDriver {
    fn name(&self) -> &str {
        &self.display_name
    }

    fn render_sample(
        &self,
        raw_samples: &[f32],
        sample_rate: u32,
        args: &ResamplerArgs,
    ) -> Result<Vec<f32>, String> {
        if !self.executable_path.exists() {
            let native = NativeResamplerDriver;
            return native.render_sample(raw_samples, sample_rate, args);
        }

        let mut temp_input_dir = None;
        let actual_input_wav =
            actual_input_wav(raw_samples, sample_rate, args, &mut temp_input_dir)?;
        if args.output_wav.is_file() {
            let _ = std::fs::remove_file(&args.output_wav);
        }
        let mut cmd = Command::new(&self.executable_path);
        let requested_duration_ms = self.requested_duration_ms(args);
        cmd.args(classic_arguments(
            &actual_input_wav,
            args,
            self.empty_flags,
            requested_duration_ms,
        ));

        let output = cmd.output().map_err(|e| {
            format!(
                "Failed to execute external resampler {:?}: {}",
                self.executable_path, e
            )
        })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!(
                "[{}] execução falhou: {}; usando Native TD-PSOLA",
                self.display_name,
                stderr.trim()
            );
            return NativeResamplerDriver.render_sample(raw_samples, sample_rate, args);
        }

        match load_resampler_output(args, sample_rate) {
            Ok(samples) => Ok(samples),
            Err(error) => {
                eprintln!("[{}] {error}; usando Native TD-PSOLA", self.display_name);
                NativeResamplerDriver.render_sample(raw_samples, sample_rate, args)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            cutoff_ms: -20.0,
            volume: 90.0,
            modulation: 5.0,
            tempo: 135.0,
            pitch_bend_str: String::new(),
            pitch_points: Vec::new(),
        }
    }

    #[test]
    fn known_profiles_roundtrip_their_labels() {
        for profile in KnownResampler::ALL {
            assert_eq!(KnownResampler::from_label(profile.label()), Some(profile));
            assert!(!profile.executable_names().is_empty());
        }
    }

    #[test]
    fn classic_arguments_use_utau_tempo_and_pitch_defaults() {
        let args = classic_arguments(Path::new("source.wav"), &sample_args(), "-", 500.0);
        assert_eq!(args[4], OsString::from("-"));
        assert_eq!(args[11], OsString::from("!135.0"));
        assert_eq!(args[12], OsString::from("AA"));
    }

    #[test]
    fn straycat_duration_excludes_the_rendered_consonant() {
        let driver = ExternalResamplerDriver::for_known(KnownResampler::StraycatRs, None);
        assert_eq!(driver.requested_duration_ms(&sample_args()), 436.0);

        let organum = ExternalResamplerDriver::for_known(KnownResampler::Organum, None);
        assert_eq!(organum.requested_duration_ms(&sample_args()), 500.0);
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
        assert!(!output.exists());
    }
}
