use crate::dsp::resampler::Resampler;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::time::Duration;

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
    pub cutoff_ms: f64,
    pub volume: f64,
    pub modulation: f64,
    pub tempo: f64,
    pub pitch_bend_str: String,
    pub pitch_points: Vec<crate::project::model::UPitchBendPoint>,
    pub loop_start_ms: Option<f64>,
    pub loop_end_ms: Option<f64>,
    pub tail_start_ms: Option<f64>,
}

pub trait ResamplerDriver: Send + Sync {
    fn name(&self) -> &str;
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
}

fn prepare_classic_flags(base_flags: &str, gender: f64, breathiness: f64) -> String {
    let mut flags = base_flags.to_string();
    if gender != 0.0 {
        flags.push_str(&format!("g{gender:.0}"));
    }
    if breathiness != 0.0 {
        flags.push_str(&format!("B{:.0}", breathiness.abs()));
    }
    flags
}

fn executable_cache_identity(name: &str, path: &Path) -> String {
    let mut identity = format!("{name}:{}", path.display());
    if let Ok(metadata) = std::fs::metadata(path) {
        identity.push_str(&format!(":{}", metadata.len()));
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                identity.push_str(&format!(":{}", duration.as_nanos()));
            }
        }
    }
    identity
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KnownResampler {
    MacRes,
    Organum,
    StraycatRs,
    World4Utau,
    Tips,
    Moresampler,
}

impl KnownResampler {
    pub const ALL: [Self; 6] = [
        Self::MacRes,
        Self::Organum,
        Self::StraycatRs,
        Self::World4Utau,
        Self::Tips,
        Self::Moresampler,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::MacRes => "macres (titinko/macres)",
            Self::Organum => "Organum (KakouLabs/Organum)",
            Self::StraycatRs => "straycat-rs (UtaUtaUtau)",
            Self::World4Utau => "World4UTAU (xrdavies/world4utau)",
            Self::Tips => "TIPS (TIPS.exe)",
            Self::Moresampler => "moresampler (moresampler.exe)",
        }
    }

    pub const fn executable_names(self) -> &'static [&'static str] {
        match self {
            Self::MacRes => &["macres", "macres.exe"],
            Self::Organum => &["organum-resampler", "organum-resampler.exe"],
            Self::StraycatRs => &["straycat-rs", "straycat-rs.exe"],
            Self::World4Utau => &["world4utau", "world4utau.exe"],
            Self::Tips => &["TIPS.exe", "tips.exe", "TIPS", "tips"],
            Self::Moresampler => &["moresampler.exe", "moresampler"],
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

    pub fn find_executable(self) -> Option<PathBuf> {
        static CACHE: std::sync::LazyLock<
            std::sync::Mutex<std::collections::HashMap<KnownResampler, Option<PathBuf>>>,
        > = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

        if let Ok(guard) = CACHE.lock() {
            if let Some(cached) = guard.get(&self) {
                return cached.clone();
            }
        }

        let result = self.search_executable_uncached();

        if let Ok(mut guard) = CACHE.lock() {
            guard.insert(self, result.clone());
        }

        result
    }

    fn search_executable_uncached(self) -> Option<PathBuf> {
        let mut roots = vec![
            PathBuf::from("./resamplers"),
            PathBuf::from("."),
            PathBuf::from("/opt/homebrew/bin"),
            PathBuf::from("/usr/local/bin"),
            PathBuf::from("/usr/bin"),
        ];

        if let Ok(current_exe) = std::env::current_exe() {
            if let Some(exe_dir) = current_exe.parent() {
                roots.push(exe_dir.join("resamplers"));
                roots.push(exe_dir.to_path_buf());
                if let Some(p1) = exe_dir.parent() {
                    roots.push(p1.join("resamplers"));
                    if let Some(p2) = p1.parent() {
                        roots.push(p2.join("resamplers"));
                        if let Some(p3) = p2.parent() {
                            roots.push(p3.join("resamplers"));
                        }
                    }
                }
            }
        }

        if let Some(home) = std::env::var_os("HOME") {
            let home = PathBuf::from(home);
            roots.push(home.join("Documents/kamafeu/resamplers"));
            roots.push(home.join("Documents/kamafeu"));
            roots.push(home.join("Downloads/resamplers"));
            roots.push(home.join("Downloads"));
            roots.push(home.join(".local/bin"));
            roots.push(home.join("Library/Application Support/OpenUTAU/Resamplers"));
            roots.push(home.join("Library/Application Support/OpenUtau/Resamplers"));
            roots.push(home.join(".wine/drive_c/Program Files (x86)/UTAU/resamplers"));
            roots.push(home.join(".wine/drive_c/Program Files (x86)/UTAU"));
            roots.push(home.join(".wine/drive_c/Program Files/UTAU/resamplers"));
            roots.push(home.join(".wine/drive_c/Program Files/UTAU"));
            roots.push(home.join(".wine/drive_c/UTAU/resamplers"));
            roots.push(home.join(".wine/drive_c/UTAU"));
        }

        if let Some(path_env) = std::env::var_os("PATH") {
            for p in std::env::split_paths(&path_env) {
                if !roots.contains(&p) {
                    roots.push(p);
                }
            }
        }

        for root in roots {
            for executable in self.executable_names() {
                let candidate = root.join(executable);
                if candidate.is_file() {
                    return Some(candidate);
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

pub struct NativeSolaResamplerDriver {
    pub mode: crate::dsp::SolaStretchMode,
}

impl Default for NativeSolaResamplerDriver {
    fn default() -> Self {
        Self {
            mode: crate::dsp::SolaStretchMode::Stretch,
        }
    }
}

impl ResamplerDriver for NativeSolaResamplerDriver {
    fn name(&self) -> &str {
        match self.mode {
            crate::dsp::SolaStretchMode::Stretch => "Nativo (SOLA Stretch)",
            crate::dsp::SolaStretchMode::Loop => "Nativo (SOLA Loop)",
            crate::dsp::SolaStretchMode::Spline => "Nativo (SOLA Spline)",
            crate::dsp::SolaStretchMode::Hybrid => "Nativo (SOLA Híbrido)",
        }
    }

    fn render_sample(
        &self,
        raw_samples: &[f32],
        sample_rate: u32,
        args: &ResamplerArgs,
        _cancel: Option<&AtomicBool>,
    ) -> Result<Vec<f32>, String> {
        let rendered = crate::dsp::SolaResampler::render_sample_with_mode(
            raw_samples,
            sample_rate,
            args.offset_ms,
            args.source_consonant_ms,
            args.consonant_ms,
            args.cutoff_ms,
            args.duration_ms,
            args.pitch_freq,
            &args.pitch_points,
            args.loop_start_ms,
            args.loop_end_ms,
            args.tail_start_ms,
            self.mode,
        );
        Ok(rendered)
    }
}

pub struct NativeResamplerDriver;

impl ResamplerDriver for NativeResamplerDriver {
    fn name(&self) -> &str {
        "Nativo (TD-PSOLA)"
    }

    fn render_sample(
        &self,
        raw_samples: &[f32],
        sample_rate: u32,
        args: &ResamplerArgs,
        _cancel: Option<&AtomicBool>,
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
        if !self.executable_path.exists() {
            eprintln!(
                "[macres] Binary not found at {:?}, falling back to Native TD-PSOLA",
                self.executable_path
            );
            let native = NativeResamplerDriver;
            return native.render_sample(raw_samples, sample_rate, args, cancel);
        }

        let mut temp_input_dir = None;
        let actual_input_wav =
            actual_input_wav(raw_samples, sample_rate, args, &mut temp_input_dir)?;
        if args.output_wav.is_file() {
            let _ = std::fs::remove_file(&args.output_wav);
        }
        let mut cmd = match crate::drivers::process::prepare_command(&self.executable_path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("[macres] Falha ao preparar comando: {e}; usando Native TD-PSOLA");
                return NativeResamplerDriver.render_sample(raw_samples, sample_rate, args, cancel);
            }
        };
        let is_exe = self
            .executable_path
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("exe"))
            .unwrap_or(false);

        cmd.args(classic_arguments(
            &actual_input_wav,
            args,
            "g0",
            args.duration_ms,
            is_exe,
        ));

        let output = match crate::drivers::process::run_with_timeout(
            &mut cmd,
            Duration::from_secs(15),
            cancel,
        ) {
            Ok(o) => o,
            Err(e) => {
                eprintln!("[macres] Falha ao executar: {e}; usando Native TD-PSOLA");
                return NativeResamplerDriver.render_sample(raw_samples, sample_rate, args, cancel);
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!(
                "[macres] execution failed: {}, falling back to Native TD-PSOLA",
                stderr
            );
            let native = NativeResamplerDriver;
            return native.render_sample(raw_samples, sample_rate, args, cancel);
        }

        match load_resampler_output(args, sample_rate) {
            Ok(samples) => Ok(samples),
            Err(error) => {
                eprintln!("[macres] {error}; usando Native TD-PSOLA");
                NativeResamplerDriver.render_sample(raw_samples, sample_rate, args, cancel)
            }
        }
    }
}

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

    fn requested_duration_ms(&self, args: &ResamplerArgs) -> f64 {
        if self.duration_excludes_consonant {
            (args.duration_ms - args.consonant_ms).max(1.0)
        } else {
            args.duration_ms
        }
    }
}

fn remove_numeric_flag(flags: &str, flag: char) -> String {
    let mut result = String::with_capacity(flags.len());
    let mut chars = flags.chars().peekable();

    while let Some(current) = chars.next() {
        if current != flag {
            result.push(current);
            continue;
        }

        if matches!(chars.peek().copied(), Some('+' | '-')) {
            chars.next();
        }
        while chars.peek().is_some_and(|next| next.is_ascii_digit()) {
            chars.next();
        }
    }

    result
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
        flags = remove_numeric_flag(&flags, 'B');
        let straycat_breathiness = (50.0 + breathiness.clamp(0.0, 100.0) * 0.5).round() as i32;
        flags.push_str(&format!("B{straycat_breathiness}"));
    }

    flags
}

impl ResamplerDriver for ExternalResamplerDriver {
    fn name(&self) -> &str {
        &self.display_name
    }

    fn prepare_flags(&self, base_flags: &str, gender: f64, breathiness: f64) -> String {
        if self.profile == Some(KnownResampler::StraycatRs) {
            prepare_straycat_flags(base_flags, gender, breathiness)
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

        let final_exe = match resolved_exe {
            Some(p) => p,
            None => {
                eprintln!(
                    "[{}] AVISO: Executável não encontrado em {:?}; usando fallback Native TD-PSOLA",
                    self.display_name, self.executable_path
                );
                return NativeResamplerDriver.render_sample(raw_samples, sample_rate, args, cancel);
            }
        };

        let is_exe = final_exe
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.eq_ignore_ascii_case("exe"))
            .unwrap_or(false);

        if is_exe {
            let stem = final_exe.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if stem.eq_ignore_ascii_case("moresampler") {
                if let Some(parent) = final_exe.parent() {
                    let moreconfig = parent.join("moreconfig.txt");
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
                }
            }
        }

        let mut temp_input_dir = None;
        let actual_input_wav =
            actual_input_wav(raw_samples, sample_rate, args, &mut temp_input_dir)?;
        if args.output_wav.is_file() {
            let _ = std::fs::remove_file(&args.output_wav);
        }
        let mut cmd = match crate::drivers::process::prepare_command(&final_exe) {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "[{}] Falha ao preparar comando: {e}; usando Native TD-PSOLA",
                    self.display_name
                );
                return NativeResamplerDriver.render_sample(raw_samples, sample_rate, args, cancel);
            }
        };
        let requested_duration_ms = self.requested_duration_ms(args);
        cmd.args(classic_arguments(
            &actual_input_wav,
            args,
            self.empty_flags,
            requested_duration_ms,
            is_exe,
        ));

        let output = match crate::drivers::process::run_with_timeout(
            &mut cmd,
            Duration::from_secs(15),
            cancel,
        ) {
            Ok(o) => o,
            Err(e) => {
                eprintln!(
                    "[{}] Falha ao executar: {e}; usando Native TD-PSOLA",
                    self.display_name
                );
                return NativeResamplerDriver.render_sample(raw_samples, sample_rate, args, cancel);
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!(
                "[{}] execução falhou: {}; usando Native TD-PSOLA",
                self.display_name,
                stderr.trim()
            );
            return NativeResamplerDriver.render_sample(raw_samples, sample_rate, args, cancel);
        }

        match load_resampler_output(args, sample_rate) {
            Ok(samples) => Ok(samples),
            Err(error) => {
                eprintln!("[{}] {error}; usando Native TD-PSOLA", self.display_name);
                NativeResamplerDriver.render_sample(raw_samples, sample_rate, args, cancel)
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
            loop_start_ms: None,
            loop_end_ms: None,
            tail_start_ms: None,
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
        let args = classic_arguments(Path::new("source.wav"), &sample_args(), "-", 500.0, false);
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
    fn external_output_is_normalized_to_the_expected_sample_rate() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("render.wav");
        crate::renderer::TrackRenderer::save_wav_samples(&output, &vec![0.25; 4_410], 44_100)
            .unwrap();
        let mut args = sample_args();
        args.output_wav = output.clone();

        let samples = load_resampler_output(&args, 48_000).unwrap();
        assert_eq!(samples.len(), 4_800);
        // The wavtool consumes this exact file after the resampler returns.
        // Its TempDir owner removes it when the phone render is finished.
        assert!(output.exists());
    }
}
