use crate::drivers::{ResamplerArgs, ResamplerDriver, ResamplerExecutionInfo};
use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, OnceLock, RwLock};
use std::time::Duration;

#[path = "resampler_cache/key.rs"]
mod key;
use key::cache_key;
#[path = "resampler_cache/persistent.rs"]
mod persistent;
use persistent::{load_persistent, store_persistent};
#[cfg(test)]
use persistent::{
    load_persistent_in, persistent_cache_path_in, persistent_execution_path_in, store_persistent_in,
};

// v5 stores effective backend/fallback metadata beside each persistent render.
const CACHE_SCHEMA: u32 = 5;
const MAX_CACHE_ENTRIES: usize = 2048;
const MAX_CACHE_SAMPLES: usize = 128 * 1024 * 1024;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CacheStats {
    pub hits: u64,
    pub misses: u64,
    pub renders: u64,
}

static CACHE_HITS: AtomicU64 = AtomicU64::new(0);
static CACHE_MISSES: AtomicU64 = AtomicU64::new(0);
static CACHE_RENDERS: AtomicU64 = AtomicU64::new(0);

pub fn cache_stats() -> CacheStats {
    CacheStats {
        hits: CACHE_HITS.load(Ordering::Relaxed),
        misses: CACHE_MISSES.load(Ordering::Relaxed),
        renders: CACHE_RENDERS.load(Ordering::Relaxed),
    }
}

pub fn reset_cache_stats() {
    CACHE_HITS.store(0, Ordering::Relaxed);
    CACHE_MISSES.store(0, Ordering::Relaxed);
    CACHE_RENDERS.store(0, Ordering::Relaxed);
}

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
    Ready(CachedRender),
}

struct CachedRender {
    samples: Arc<[f32]>,
    execution: ResamplerExecutionInfo,
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
    #[cfg(target_arch = "wasm32")]
    {
        let _ = days;
        return Ok(0);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let directory = persistent_cache_dir();
        if !directory.is_dir() {
            return Ok(0);
        }
        let cutoff = web_time::SystemTime::now()
            .checked_sub(Duration::from_secs(u64::from(days.max(1)) * 86_400))
            .unwrap_or(web_time::SystemTime::UNIX_EPOCH);
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
    #[cfg(target_arch = "wasm32")]
    {
        return std::path::PathBuf::from("/tmp/kamafeu_cache");
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
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
}

pub fn get_disk_cache_stats() -> (usize, u64) {
    #[cfg(target_arch = "wasm32")]
    {
        return (0, 0);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
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
        if path.is_file() && std::fs::remove_file(path).is_ok() {
            deleted += 1;
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

fn insert_ready(
    state: &mut CacheState,
    key: u64,
    samples: Vec<f32>,
    execution: ResamplerExecutionInfo,
) {
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
            state.total_samples = state.total_samples.saturating_sub(removed.samples.len());
        }
    }
    state.total_samples = state.total_samples.saturating_add(samples.len());
    state.entries.insert(
        key,
        CacheEntry::Ready(CachedRender {
            samples: Arc::from(samples),
            execution,
        }),
    );
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
) -> Result<(Vec<f32>, bool, ResamplerExecutionInfo), String> {
    use std::sync::atomic::Ordering;

    if cancel.is_some_and(|token| token.load(Ordering::Relaxed)) {
        return Err("renderização cancelada".to_string());
    }

    let key = cache_key(driver, sample_rate, args, raw_samples);
    let (state_mutex, ready) = cache();

    loop {
        let mut state = state_mutex
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        match state.entries.get(&key) {
            Some(CacheEntry::Ready(cached)) => {
                CACHE_HITS.fetch_add(1, Ordering::Relaxed);
                return Ok((cached.samples.to_vec(), true, cached.execution.clone()));
            }
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
                CACHE_MISSES.fetch_add(1, Ordering::Relaxed);
                state.entries.insert(key, CacheEntry::Rendering);
                break;
            }
        }
    }

    if driver.supports_persistent_cache() {
        if let Some((samples, execution)) = load_persistent(key, sample_rate) {
            CACHE_HITS.fetch_add(1, Ordering::Relaxed);
            let mut state = state_mutex
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            insert_ready(&mut state, key, samples.clone(), execution.clone());
            ready.notify_all();
            return Ok((samples, true, execution));
        }
    }

    CACHE_RENDERS.fetch_add(1, Ordering::Relaxed);
    let rendered = driver
        .render_sample_with_execution(raw_samples, sample_rate, args, cancel)
        .and_then(|output| {
            let samples = output.samples;
            if samples.is_empty() || samples.iter().any(|s| !s.is_finite()) {
                Err("Resampler produziu áudio vazio ou amostras inválidas".to_string())
            } else {
                Ok((samples, output.execution))
            }
        });
    match rendered {
        Ok((samples, execution)) => {
            // External drivers create this file only when the external engine
            // actually succeeds. Do not permanently cache a transient native
            // fallback caused by Wine/process failure.
            let cacheable = !driver.uses_external_process() || args.output_wav.is_file();
            if cacheable && driver.supports_persistent_cache() {
                store_persistent(key, &samples, sample_rate, &execution);
            }
            let mut state = state_mutex
                .lock()
                .unwrap_or_else(|error| error.into_inner());
            if cacheable {
                insert_ready(&mut state, key, samples.clone(), execution.clone());
            } else {
                state.entries.remove(&key);
            }
            ready.notify_all();
            Ok((samples, false, execution))
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

        fn engine_id(&self) -> &'static str {
            "test.counting-driver"
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
            normalization: None,
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
    fn effective_flag_string_participates_in_render_cache_identity() {
        let driver = CountingDriver {
            id: 0,
            calls: AtomicUsize::new(0),
            delay: Duration::ZERO,
        };
        let input = vec![0.1f32; 128];
        let base = args();
        let mut changed = base.clone();
        changed.flags = "g12t-4B30H8".into();
        assert_ne!(
            cache_key(&driver, 48_000, &base, &input),
            cache_key(&driver, 48_000, &changed, &input)
        );
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
        assert_eq!(first.2.effective_backend_id, "test.counting-driver");
        assert_eq!(second.2, first.2);
        assert_eq!(driver.calls.load(Ordering::Relaxed), 1);
        assert_eq!(second.0, vec![0.1, 0.2]);
    }

    #[test]
    fn persistent_cache_roundtrips_execution_and_rejects_audio_without_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let execution = ResamplerExecutionInfo {
            effective_backend_id: "kamafeu.sola".into(),
            stages: vec!["kamafeu.sola".into()],
            fallback_reasons: vec!["WORLD coverage guard rejected output".into()],
            adjustments: Vec::new(),
        };
        store_persistent_in(directory.path(), 0x1234, &[0.25, -0.5], 44_100, &execution);

        let loaded = load_persistent_in(directory.path(), 0x1234, 44_100)
            .expect("audio and execution metadata");
        assert_eq!(loaded.0, vec![0.25, -0.5]);
        assert_eq!(loaded.1, execution);

        crate::renderer::TrackRenderer::save_wav_samples(
            persistent_cache_path_in(directory.path(), 0x1234),
            &[0.5, -0.25],
            44_100,
        )
        .unwrap();
        assert!(load_persistent_in(directory.path(), 0x1234, 44_100).is_none());
        assert!(!persistent_cache_path_in(directory.path(), 0x1234).exists());

        store_persistent_in(directory.path(), 0x1235, &[0.25, -0.5], 44_100, &execution);
        std::fs::remove_file(persistent_execution_path_in(directory.path(), 0x1235)).unwrap();
        assert!(load_persistent_in(directory.path(), 0x1235, 44_100).is_none());
        assert!(!persistent_cache_path_in(directory.path(), 0x1235).exists());
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
        let before = cache_key(&driver, 44_100, &request, &[]);

        std::fs::write(directory.path().join("sample_wav.frq"), b"analysis").unwrap();
        let after = cache_key(&driver, 44_100, &request, &[]);
        assert_ne!(before, after);
    }

    #[test]
    fn temporary_source_samples_invalidate_the_cache_key() {
        let driver = CountingDriver {
            id: 5,
            calls: AtomicUsize::new(0),
            delay: Duration::ZERO,
        };
        let request = args();
        let first = cache_key(&driver, 44_100, &request, &[0.1, 0.2]);
        let second = cache_key(&driver, 44_100, &request, &[0.1, 0.3]);
        assert_ne!(first, second);
    }
}
