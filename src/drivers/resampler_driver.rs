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
    Catalina,
    MacRes,
    Organum,
    StraycatRs,
    HifisamplerRs,
    World4Utau,
    Tips,
    Moresampler,
}

static CACHE: std::sync::LazyLock<
    std::sync::Mutex<std::collections::HashMap<KnownResampler, Option<PathBuf>>>,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

impl KnownResampler {
    pub const ALL: [Self; 8] = [
        Self::Catalina,
        Self::MacRes,
        Self::Organum,
        Self::StraycatRs,
        Self::HifisamplerRs,
        Self::World4Utau,
        Self::Tips,
        Self::Moresampler,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Catalina => "Catalina (NSF HiFi-GAN)",
            Self::MacRes => "macres (titinko/macres)",
            Self::Organum => "Organum (KakouLabs/Organum)",
            Self::StraycatRs => "straycat-rs (UtaUtaUtau)",
            Self::HifisamplerRs => "Hifisampler (Slidingwall/hifisampler-rs)",
            Self::World4Utau => "World4UTAU (xrdavies/world4utau)",
            Self::Tips => "TIPS (TIPS.exe)",
            Self::Moresampler => "moresampler (moresampler.exe)",
        }
    }

    pub const fn executable_names(self) -> &'static [&'static str] {
        match self {
            Self::Catalina => &[
                "catalina",
                "catalina.exe",
                "hifisampler",
                "hifisampler.exe",
                "hifiserver-rust",
                "hifiserver-rust.exe",
                "hifisampler-rs",
                "hifisampler-rs.exe",
            ],
            Self::MacRes => &["macres", "macres.exe"],
            Self::Organum => &["organum-resampler", "organum-resampler.exe"],
            Self::StraycatRs => &["straycat-rs", "straycat-rs.exe"],
            Self::HifisamplerRs => &[
                "hifisampler",
                "hifisampler.exe",
                "hifiserver-rust",
                "hifiserver-rust.exe",
                "hifisampler-rs",
                "hifisampler-rs.exe",
            ],
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

    pub fn clear_cache() {
        if let Ok(mut guard) = CACHE.lock() {
            guard.clear();
        }
    }

    pub fn find_executable(self) -> Option<PathBuf> {
        if let Ok(guard) = CACHE.lock() {
            if let Some(Some(cached)) = guard.get(&self) {
                // A resampler can be installed while the application is
                // running. Never keep a stale path alive after it disappears.
                if cached.is_file() {
                    return Some(cached.clone());
                }
            }
        }

        // Do not short-circuit a cached `None`: a newly copied/downloaded
        // resampler must become selectable without restarting Kamafeu.
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
        // External engines are not required to honor the host's floating
        // point headroom. Normalize at the driver boundary so Catalina,
        // Venus-compatible binaries, and legacy resamplers all enter the
        // mixer with finite, bounded PCM.
        crate::renderer::track::TrackRenderer::apply_soft_limiter(&mut samples, 0.95);
        Ok(samples)
    }
}

pub struct NativeResamplerDriver;

impl ResamplerDriver for NativeResamplerDriver {
    fn name(&self) -> &str {
        "WORLD (Nativo)"
    }

    fn cache_identity(&self) -> String {
        format!(
            "{}:world-rs-v9-full-tail-coverage-watchdog:{}",
            self.name(),
            crate::dsp::world_resampler::cache_identity()
        )
    }

    fn render_sample(
        &self,
        raw_samples: &[f32],
        sample_rate: u32,
        args: &ResamplerArgs,
        _cancel: Option<&AtomicBool>,
    ) -> Result<Vec<f32>, String> {
        let rendered = crate::dsp::SolaResampler::render_sample(
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
        );
        Ok(rendered)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct NativeWorldResamplerDriver;

impl ResamplerDriver for NativeWorldResamplerDriver {
    fn name(&self) -> &str {
        "WORLD (Nativo)"
    }
    fn cache_identity(&self) -> String {
        format!(
            "WORLD:native-world-rs-v9-full-tail-coverage-watchdog:{}",
            crate::dsp::world_resampler::cache_identity()
        )
    }
    fn render_sample(
        &self,
        raw_samples: &[f32],
        sample_rate: u32,
        args: &ResamplerArgs,
        cancel: Option<&AtomicBool>,
    ) -> Result<Vec<f32>, String> {
        if cancel.is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Relaxed)) {
            return Err("Renderização cancelada".to_string());
        }
        let world = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            crate::dsp::world_resampler::WorldResampler::render_sample_with_oto(
                raw_samples,
                sample_rate,
                args.offset_ms,
                args.source_consonant_ms,
                args.consonant_ms,
                args.duration_ms,
                args.pitch_freq,
                &args.pitch_points,
                args.cutoff_ms,
                args.loop_start_ms,
                args.loop_end_ms,
                args.tail_start_ms,
            )
        }));
        let expected_len =
            ((args.duration_ms.max(0.0) / 1_000.0) * sample_rate as f64).round() as usize;
        let rendered = match world {
            Ok(samples) if has_render_coverage(&samples, expected_len, sample_rate) => samples,
            _ => crate::dsp::SolaResampler::render_sample(
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
            ),
        };
        if cancel.is_some_and(|flag| flag.load(std::sync::atomic::Ordering::Relaxed)) {
            return Err("Renderização cancelada".to_string());
        }
        Ok(rendered)
    }
}

fn has_render_coverage(samples: &[f32], expected_len: usize, sample_rate: u32) -> bool {
    if expected_len == 0
        || samples.len() < expected_len.saturating_mul(95) / 100
        || samples.iter().any(|sample| !sample.is_finite())
    {
        return false;
    }
    let peak = samples
        .iter()
        .map(|sample| sample.abs())
        .fold(0.0_f32, f32::max);
    if peak <= 1e-5 {
        return false;
    }
    let window = (sample_rate as usize / 20).max(64); // 50 ms
    let threshold = (peak * 0.002).max(1e-5);
    let inspected = &samples[..expected_len.min(samples.len())];
    let mut windows = 0usize;
    let mut audible = 0usize;
    let mut last_window_audible = false;
    for chunk in inspected.chunks(window) {
        if chunk.len() < window / 2 {
            continue;
        }
        windows += 1;
        last_window_audible = chunk.iter().any(|sample| sample.abs() >= threshold);
        if last_window_audible {
            audible += 1;
        }
    }
    // A resampler phoneme must provide material through its requested tail;
    // the wavtool/envelope owns the release. Accepting a loud attack followed
    // by silence creates the characteristic CVVC "hiccup" at every VC -> CV
    // handoff. Permit a small amount of internal quiet material, but require
    // at least 75% coverage and a live final 50 ms window.
    windows > 0 && audible * 4 >= windows * 3 && last_window_audible
}

/// Tokeniza flags UTAU sem confundir flags de um e dois caracteres.
#[derive(Debug, Clone, PartialEq)]
pub struct UtauFlagToken {
    pub key: String,
    pub num_val: Option<f64>,
}

pub fn parse_utau_flags(flags: &str) -> Vec<UtauFlagToken> {
    let chars: Vec<char> = flags.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() || chars[i] == '/' {
            i += 1;
            continue;
        }
        if !chars[i].is_alphabetic() {
            i += 1;
            continue;
        }
        let start = i;
        i += 1;
        if i < chars.len() && chars[i].is_ascii_lowercase() {
            let candidate: String = chars[start..=i].iter().collect();
            if matches!(
                candidate.as_str(),
                "Hb" | "Mb" | "Mo" | "Me" | "Mt" | "Ms" | "Nh" | "Te"
            ) {
                i += 1;
            }
        }
        let key: String = chars[start..i].iter().collect();
        let number_start = i;
        if i < chars.len() && matches!(chars[i], '+' | '-') {
            i += 1;
        }
        while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
            i += 1;
        }
        let num_val = if i > number_start {
            chars[number_start..i]
                .iter()
                .collect::<String>()
                .parse()
                .ok()
        } else {
            None
        };
        tokens.push(UtauFlagToken { key, num_val });
    }
    tokens
}

pub fn parse_flag_numeric(flags: &str, target_flag: &str) -> Option<f64> {
    let tokens = parse_utau_flags(flags);
    for token in tokens.into_iter().rev() {
        if token.key == target_flag {
            return token.num_val;
        }
    }
    None
}

pub fn remove_flag(flags: &str, target_flag: &str) -> String {
    let tokens = parse_utau_flags(flags);
    let mut result = String::with_capacity(flags.len());
    for token in tokens {
        if token.key != target_flag {
            result.push_str(&token.key);
            if let Some(val) = token.num_val {
                if val.fract() == 0.0 {
                    result.push_str(&format!("{val:.0}"));
                } else {
                    result.push_str(&format!("{val}"));
                }
            }
        }
    }
    result
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
                if stem.eq_ignore_ascii_case("moresampler") {
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
        assert_eq!(args[6], OsString::from("500"));
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
    fn hifisampler_maps_breathiness_to_hb_flag() {
        let driver = ExternalResamplerDriver::for_known(KnownResampler::HifisamplerRs, None);

        assert_eq!(driver.prepare_flags("P86", 0.0, 0.0), "P86");
        assert_eq!(driver.prepare_flags("P86", -2.0, 20.0), "P86g-2Hb140");
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
        assert!(output.exists());
    }

    #[test]
    fn external_output_is_finite_and_safe_for_the_mixer() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("unsafe.wav");
        crate::renderer::TrackRenderer::save_wav_samples(
            &output,
            &[f32::NAN, f32::INFINITY, -2.0, 2.0, 0.25],
            48_000,
        )
        .unwrap();
        let mut args = sample_args();
        args.output_wav = output;
        let samples = load_resampler_output(&args, 48_000).unwrap();
        assert!(samples.iter().all(|sample| sample.is_finite()));
        assert!(
            samples
                .iter()
                .map(|sample| sample.abs())
                .fold(0.0, f32::max)
                <= 0.95
        );
    }

    #[test]
    fn world_watchdog_rejects_a_fragment_that_dies_before_the_note_ends() {
        let sample_rate = 44_100;
        let expected_len = sample_rate * 2;
        let mut fragment = vec![0.0; expected_len];
        fragment[..sample_rate / 4].fill(0.25);

        assert!(!has_render_coverage(
            &fragment,
            expected_len,
            sample_rate as u32
        ));
    }

    #[test]
    fn world_watchdog_accepts_a_sustained_note() {
        let sample_rate = 44_100;
        let expected_len = sample_rate * 2;
        let sustained = vec![0.25; expected_len];

        assert!(has_render_coverage(
            &sustained,
            expected_len,
            sample_rate as u32
        ));
    }

    #[test]
    fn world_watchdog_rejects_a_short_phone_with_a_silent_tail() {
        let sample_rate = 44_100;
        let expected_len = sample_rate / 5;
        let mut truncated = vec![0.0; expected_len];
        truncated[..expected_len * 2 / 3].fill(0.25);

        assert!(!has_render_coverage(
            &truncated,
            expected_len,
            sample_rate as u32
        ));
    }

    #[test]
    fn test_parse_utau_flags_distinguishes_composite_flags() {
        let tokens = parse_utau_flags("g-5Hb140B50Mb20P86t+10");
        assert_eq!(tokens.len(), 6);
        assert_eq!(tokens[0].key, "g");
        assert_eq!(tokens[0].num_val, Some(-5.0));
        assert_eq!(tokens[1].key, "Hb");
        assert_eq!(tokens[1].num_val, Some(140.0));
        assert_eq!(tokens[2].key, "B");
        assert_eq!(tokens[2].num_val, Some(50.0));
        assert_eq!(tokens[3].key, "Mb");
        assert_eq!(tokens[3].num_val, Some(20.0));
        assert_eq!(tokens[4].key, "P");
        assert_eq!(tokens[4].num_val, Some(86.0));
        assert_eq!(tokens[5].key, "t");
        assert_eq!(tokens[5].num_val, Some(10.0));

        // Test numerical lookup
        assert_eq!(parse_flag_numeric("g-5Hb140B50", "Hb"), Some(140.0));
        assert_eq!(parse_flag_numeric("g-5Hb140B50", "B"), Some(50.0));
        assert_eq!(parse_flag_numeric("g-5Hb140B50", "g"), Some(-5.0));

        // Test flag removal without touching composite prefixes
        assert_eq!(remove_flag("g-5Hb140B50P86", "B"), "g-5Hb140P86");
        assert_eq!(remove_flag("g-5Hb140B50P86", "Hb"), "g-5B50P86");
    }
}
