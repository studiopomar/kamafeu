use crate::drivers::{ResamplerArgs, ResamplerDriver};
use std::collections::{HashMap, VecDeque};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, OnceLock, RwLock};
use std::time::Duration;

// Timing/oto.ini semantics changed in v4 (negative overlap, UST overrides and
// canonical manual VC placement). Older rendered fragments are incompatible.
const CACHE_SCHEMA: u32 = 4;
const MAX_CACHE_ENTRIES: usize = 2048;
const MAX_CACHE_SAMPLES: usize = 128 * 1024 * 1024;

#[derive(Debug, Clone, Copy)]
struct CacheLimits {
    max_entries: usize,
    max_samples: usize,
    max_disk_bytes: u64,
}

impl Default for CacheLimits {
    fn default() -> Self {
        Self {
            max_entries: MAX_CACHE_ENTRIES,
            max_samples: MAX_CACHE_SAMPLES,
            max_disk_bytes: 4 * 1024 * 1024 * 1024,
        }
    }
}

enum CacheEntry {
    Rendering,
    Ready(Arc<[f32]>),
}

#[derive(Default)]
struct CacheState {
    entries: HashMap<u64, CacheEntry>,
    insertion_order: VecDeque<u64>,
    total_samples: usize,
}

fn cache() -> &'static (Mutex<CacheState>, Condvar) {
    static CACHE: OnceLock<(Mutex<CacheState>, Condvar)> = OnceLock::new();
    CACHE.get_or_init(|| (Mutex::new(CacheState::default()), Condvar::new()))
}

fn cache_directory_override() -> &'static RwLock<Option<PathBuf>> {
    static CACHE_DIRECTORY_OVERRIDE: OnceLock<RwLock<Option<PathBuf>>> = OnceLock::new();
    CACHE_DIRECTORY_OVERRIDE.get_or_init(|| RwLock::new(None))
}

fn cache_limits() -> &'static RwLock<CacheLimits> {
    static CACHE_LIMITS: OnceLock<RwLock<CacheLimits>> = OnceLock::new();
    CACHE_LIMITS.get_or_init(|| RwLock::new(CacheLimits::default()))
}

pub fn set_cache_limits(max_ram_cache_mb: usize, max_disk_cache_mb: usize) {
    let max_samples = max_ram_cache_mb
        .max(1)
        .saturating_mul(1024 * 1024)
        .saturating_div(std::mem::size_of::<f32>());
    let limits = CacheLimits {
        max_entries: MAX_CACHE_ENTRIES.min((max_samples / 1024).max(16)),
        max_samples,
        max_disk_bytes: max_disk_cache_mb.max(1).saturating_mul(1024 * 1024) as u64,
    };
    if let Ok(mut current) = cache_limits().write() {
        *current = limits;
    }
}

/// Remove arquivos persistentes que não são acessados há mais de `days` dias.
/// A limpeza é limitada à pasta de cache atual e nunca atravessa subpastas.
pub fn cleanup_older_than(days: u32) -> Result<usize, std::io::Error> {
    let directory = persistent_cache_dir();
    if !directory.is_dir() {
        return Ok(0);
    }
    let cutoff = std::time::SystemTime::now()
        .checked_sub(Duration::from_secs(u64::from(days.max(1)) * 86_400))
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
    let mut deleted = 0;
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if metadata.is_file()
            && metadata
                .modified()
                .map(|modified| modified < cutoff)
                .unwrap_or(false)
            && std::fs::remove_file(entry.path()).is_ok()
        {
            deleted += 1;
        }
    }
    Ok(deleted)
}

/// Seleciona a pasta persistente de cache. `None` restaura o local padrão do
/// sistema. A alteração é usada pelos próximos acessos sem invalidar entradas
/// já presentes em memória.
pub fn set_persistent_cache_dir_override(path: Option<PathBuf>) {
    if let Ok(mut override_path) = cache_directory_override().write() {
        *override_path = path;
    }
}

pub fn persistent_cache_dir() -> std::path::PathBuf {
    if let Ok(override_path) = cache_directory_override().read() {
        if let Some(path) = override_path.as_ref() {
            return path.clone();
        }
    }
    let base = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    #[cfg(target_os = "macos")]
    let base = base.join("Library").join("Caches");
    #[cfg(not(target_os = "macos"))]
    let base = base.join(".cache");
    base.join("kamafeu")
        .join(format!("resampler-v{CACHE_SCHEMA}"))
}

pub fn get_disk_cache_stats() -> (usize, u64) {
    let dir = persistent_cache_dir();
    if !dir.is_dir() {
        return (0, 0);
    }
    let mut count = 0;
    let mut total_bytes = 0;
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                if meta.is_file() {
                    count += 1;
                    total_bytes += meta.len();
                }
            }
        }
    }
    (count, total_bytes)
}

pub fn clear_disk_cache() -> Result<usize, std::io::Error> {
    clear_memory_cache();
    let dir = persistent_cache_dir();
    if !dir.is_dir() {
        return Ok(0);
    }
    let mut deleted = 0;
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            if std::fs::remove_file(path).is_ok() {
                deleted += 1;
            }
        }
    }
    Ok(deleted)
}

pub fn clear_memory_cache() {
    let (lock, cvar) = cache();
    let mut state = lock.lock().unwrap();
    state.entries.clear();
    state.insertion_order.clear();
    state.total_samples = 0;
    cvar.notify_all();
}

pub fn get_memory_cache_stats() -> (usize, usize) {
    let (lock, _) = cache();
    let state = lock.lock().unwrap();
    (state.entries.len(), state.total_samples)
}

fn persistent_cache_path(key: u64) -> std::path::PathBuf {
    persistent_cache_dir().join(format!("res-{key:016x}.wav"))
}

fn load_persistent(key: u64, sample_rate: u32) -> Option<Vec<f32>> {
    let path = persistent_cache_path(key);
    let loaded = crate::renderer::TrackRenderer::load_wav_samples(&path);
    match loaded {
        Ok((samples, cached_rate)) if !samples.is_empty() && cached_rate == sample_rate => {
            Some(samples)
        }
        Ok(_) | Err(_) => {
            if path.is_file() {
                let _ = std::fs::remove_file(path);
            }
            None
        }
    }
}

fn store_persistent(key: u64, samples: &[f32], sample_rate: u32) {
    if samples.is_empty() {
        return;
    }
    let directory = persistent_cache_dir();
    if std::fs::create_dir_all(&directory).is_err() {
        return;
    }
    let destination = persistent_cache_path(key);
    let Ok(temporary) = tempfile::Builder::new()
        .prefix("res-")
        .suffix(".wav")
        .tempfile_in(directory)
    else {
        return;
    };
    let temporary = temporary.into_temp_path();
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let stored = hound::WavWriter::create(&temporary, spec).and_then(|mut writer| {
        for &sample in samples {
            writer.write_sample(sample)?;
        }
        writer.finalize()
    });
    if stored.is_ok() {
        let _ = temporary.persist(destination);
        prune_persistent_cache();
    }
}

fn prune_persistent_cache() {
    let limit = cache_limits()
        .read()
        .map(|limits| limits.max_disk_bytes)
        .unwrap_or_else(|_| CacheLimits::default().max_disk_bytes);
    let directory = persistent_cache_dir();
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    let mut files: Vec<(std::path::PathBuf, u64, std::time::SystemTime)> = entries
        .flatten()
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            metadata.is_file().then(|| {
                (
                    entry.path(),
                    metadata.len(),
                    metadata
                        .modified()
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH),
                )
            })
        })
        .collect();
    let mut total: u64 = files.iter().map(|(_, bytes, _)| *bytes).sum();
    files.sort_by_key(|(_, _, modified)| *modified);
    for (path, bytes, _) in files {
        if total <= limit {
            break;
        }
        if std::fs::remove_file(path).is_ok() {
            total = total.saturating_sub(bytes);
        }
    }
}

fn hash_f64(value: f64, hasher: &mut impl Hasher) {
    value.to_bits().hash(hasher);
}

fn hash_optional_f64(value: Option<f64>, hasher: &mut impl Hasher) {
    value.map(f64::to_bits).hash(hasher);
}

fn file_fingerprint(path: &std::path::Path, hasher: &mut impl Hasher) {
    path.hash(hasher);
    if let Ok(metadata) = std::fs::metadata(path) {
        metadata.len().hash(hasher);
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                duration.as_nanos().hash(hasher);
            }
        }
    }
}

fn source_fingerprint(args: &ResamplerArgs, hasher: &mut impl Hasher) {
    let input = &args.input_wav;
    file_fingerprint(input, hasher);

    let extension = input
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let stem = input
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("");
    let without_extension = input.with_extension("");
    let mut sidecars = vec![
        input.with_file_name(format!("{stem}_{extension}.frq")),
        input.with_extension(format!("{extension}.llsm")),
        input.with_extension(format!("{extension}.uspec")),
        input.with_extension(format!("{extension}.dio")),
        input.with_extension(format!("{extension}.star")),
        input.with_extension(format!("{extension}.platinum")),
        input.with_extension(format!("{extension}.frc")),
        input.with_extension(format!("{extension}.pmk")),
        input.with_extension(format!("{extension}.vs4ufrq")),
        without_extension.with_extension("rudb"),
        without_extension.with_extension("sc.npz"),
        without_extension.with_extension("sc"),
        without_extension.with_extension("hifi.npz"),
    ];
    if let Some(parent) = input.parent() {
        sidecars.push(parent.join("desc.mrq"));
    }
    for sidecar in sidecars.into_iter().filter(|path| path.is_file()) {
        file_fingerprint(&sidecar, hasher);
    }
}

fn cache_key(driver: &dyn ResamplerDriver, sample_rate: u32, args: &ResamplerArgs) -> u64 {
    let mut hasher = DefaultHasher::new();
    CACHE_SCHEMA.hash(&mut hasher);
    driver.cache_identity().hash(&mut hasher);
    sample_rate.hash(&mut hasher);
    source_fingerprint(args, &mut hasher);
    args.pitch_name.hash(&mut hasher);
    hash_f64(args.pitch_freq, &mut hasher);
    hash_f64(args.velocity, &mut hasher);
    args.flags.hash(&mut hasher);
    hash_f64(args.offset_ms, &mut hasher);
    hash_f64(args.duration_ms, &mut hasher);
    hash_f64(args.source_consonant_ms, &mut hasher);
    hash_f64(args.consonant_ms, &mut hasher);
    hash_f64(args.cutoff_ms, &mut hasher);
    hash_f64(args.volume, &mut hasher);
    hash_f64(args.modulation, &mut hasher);
    hash_f64(args.tempo, &mut hasher);
    args.pitch_bend_str.hash(&mut hasher);
    for point in &args.pitch_points {
        hash_f64(point.time_offset_ms, &mut hasher);
        hash_f64(point.pitch_offset_cents, &mut hasher);
        point.shape.hash(&mut hasher);
    }
    hash_optional_f64(args.loop_start_ms, &mut hasher);
    hash_optional_f64(args.loop_end_ms, &mut hasher);
    hash_optional_f64(args.tail_start_ms, &mut hasher);
    hasher.finish()
}

fn insert_ready(state: &mut CacheState, key: u64, samples: Vec<f32>) {
    let limits = cache_limits()
        .read()
        .map(|limits| *limits)
        .unwrap_or_default();
    if samples.len() > limits.max_samples {
        state.entries.remove(&key);
        return;
    }
    while state.entries.len() >= limits.max_entries
        || state.total_samples.saturating_add(samples.len()) > limits.max_samples
    {
        let Some(oldest) = state.insertion_order.pop_front() else {
            break;
        };
        if let Some(CacheEntry::Ready(removed)) = state.entries.remove(&oldest) {
            state.total_samples = state.total_samples.saturating_sub(removed.len());
        }
    }
    state.total_samples = state.total_samples.saturating_add(samples.len());
    state
        .entries
        .insert(key, CacheEntry::Ready(Arc::from(samples)));
    state.insertion_order.push_back(key);
}

/// Reuses identical resampler work across notes and preview restarts.
///
/// The `Rendering` state also coalesces identical requests produced by Rayon,
/// preventing several Wine processes from synthesizing the same phone at once.
pub(crate) fn render_with_cache(
    driver: &dyn ResamplerDriver,
    raw_samples: &[f32],
    sample_rate: u32,
    args: &ResamplerArgs,
    cancel: Option<&std::sync::atomic::AtomicBool>,
) -> Result<(Vec<f32>, bool), String> {
    use std::sync::atomic::Ordering;

    if cancel.is_some_and(|token| token.load(Ordering::Relaxed)) {
        return Err("renderização cancelada".to_string());
    }

    let key = cache_key(driver, sample_rate, args);
    let (state_mutex, ready) = cache();

    loop {
        let mut state = state_mutex
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        match state.entries.get(&key) {
            Some(CacheEntry::Ready(samples)) => return Ok((samples.to_vec(), true)),
            Some(CacheEntry::Rendering) => {
                if cancel.is_some_and(|token| token.load(Ordering::Relaxed)) {
                    return Err("renderização cancelada".to_string());
                }
                let (next_state, _) = ready
                    .wait_timeout(state, Duration::from_millis(25))
                    .unwrap_or_else(|error| error.into_inner());
                drop(next_state);
            }
            None => {
                state.entries.insert(key, CacheEntry::Rendering);
                break;
            }
        }
    }

    if driver.supports_persistent_cache() {
        if let Some(samples) = load_persistent(key, sample_rate) {
            let mut state = state_mutex
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            insert_ready(&mut state, key, samples.clone());
            ready.notify_all();
            return Ok((samples, true));
        }
    }

    let rendered = driver
        .render_sample(raw_samples, sample_rate, args, cancel)
        .and_then(|samples| {
            if samples.is_empty() || samples.iter().any(|s| !s.is_finite()) {
                Err("Resampler produziu áudio vazio ou amostras inválidas".to_string())
            } else {
                Ok(samples)
            }
        });
    match rendered {
        Ok(samples) => {
            // External drivers create this file only when the external engine
            // actually succeeds. Do not permanently cache a transient native
            // fallback caused by Wine/process failure.
            let cacheable = !driver.uses_external_process() || args.output_wav.is_file();
            if cacheable && driver.supports_persistent_cache() {
                store_persistent(key, &samples, sample_rate);
            }
            let mut state = state_mutex
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if cacheable {
                insert_ready(&mut state, key, samples.clone());
            } else {
                state.entries.remove(&key);
            }
            ready.notify_all();
            Ok((samples, false))
        }
        Err(error) => {
            let mut state = state_mutex
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            state.entries.remove(&key);
            ready.notify_all();
            Err(error)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::model::UPitchBendPoint;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingDriver {
        id: usize,
        calls: AtomicUsize,
        delay: Duration,
    }

    impl ResamplerDriver for CountingDriver {
        fn name(&self) -> &str {
            "cache-test"
        }

        fn cache_identity(&self) -> String {
            format!("cache-test-{}", self.id)
        }

        fn supports_persistent_cache(&self) -> bool {
            false
        }

        fn render_sample(
            &self,
            raw_samples: &[f32],
            _sample_rate: u32,
            _args: &ResamplerArgs,
            _cancel: Option<&std::sync::atomic::AtomicBool>,
        ) -> Result<Vec<f32>, String> {
            self.calls.fetch_add(1, Ordering::Relaxed);
            std::thread::sleep(self.delay);
            Ok(raw_samples.to_vec())
        }
    }

    fn args() -> ResamplerArgs {
        ResamplerArgs {
            input_wav: PathBuf::from("cache-test-input.wav"),
            output_wav: PathBuf::from("cache-test-output.wav"),
            pitch_name: "C4".to_string(),
            pitch_freq: 261.63,
            velocity: 100.0,
            flags: String::new(),
            offset_ms: 0.0,
            duration_ms: 500.0,
            source_consonant_ms: 100.0,
            consonant_ms: 100.0,
            cutoff_ms: 0.0,
            volume: 100.0,
            modulation: 0.0,
            tempo: 120.0,
            pitch_bend_str: String::new(),
            pitch_points: vec![UPitchBendPoint {
                time_offset_ms: 0.0,
                pitch_offset_cents: 0.0,
                shape: "s".to_string(),
            }],
            loop_start_ms: None,
            loop_end_ms: None,
            tail_start_ms: None,
        }
    }

    #[test]
    fn identical_request_runs_resampler_once() {
        let driver = CountingDriver {
            id: 1,
            calls: AtomicUsize::new(0),
            delay: Duration::ZERO,
        };
        let request = args();
        let first = render_with_cache(&driver, &[0.1, 0.2], 44_100, &request, None).unwrap();
        let second = render_with_cache(&driver, &[0.1, 0.2], 44_100, &request, None).unwrap();

        assert!(!first.1);
        assert!(second.1);
        assert_eq!(driver.calls.load(Ordering::Relaxed), 1);
        assert_eq!(second.0, vec![0.1, 0.2]);
    }

    #[test]
    fn changed_pitch_does_not_reuse_stale_audio() {
        let driver = CountingDriver {
            id: 2,
            calls: AtomicUsize::new(0),
            delay: Duration::ZERO,
        };
        let first_request = args();
        let mut changed_request = args();
        changed_request.pitch_freq = 293.66;

        render_with_cache(&driver, &[0.1], 44_100, &first_request, None).unwrap();
        render_with_cache(&driver, &[0.1], 44_100, &changed_request, None).unwrap();
        assert_eq!(driver.calls.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn concurrent_identical_requests_are_coalesced() {
        let driver = CountingDriver {
            id: 3,
            calls: AtomicUsize::new(0),
            delay: Duration::from_millis(50),
        };
        let request = args();
        std::thread::scope(|scope| {
            let handles = (0..8)
                .map(|_| {
                    scope.spawn(|| {
                        render_with_cache(&driver, &[0.3], 44_100, &request, None).unwrap()
                    })
                })
                .collect::<Vec<_>>();
            for handle in handles {
                assert_eq!(handle.join().unwrap().0, vec![0.3]);
            }
        });
        assert_eq!(driver.calls.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn analysis_sidecar_invalidates_the_cache_key() {
        let directory = tempfile::tempdir().unwrap();
        let input = directory.path().join("sample.wav");
        std::fs::write(&input, b"source").unwrap();
        let driver = CountingDriver {
            id: 4,
            calls: AtomicUsize::new(0),
            delay: Duration::ZERO,
        };
        let mut request = args();
        request.input_wav = input;
        let before = cache_key(&driver, 44_100, &request);

        std::fs::write(directory.path().join("sample_wav.frq"), b"analysis").unwrap();
        let after = cache_key(&driver, 44_100, &request);
        assert_ne!(before, after);
    }
}
