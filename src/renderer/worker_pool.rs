//! Reusable Rayon pools for repeated preview and export jobs.
//!
//! Pool creation is relatively expensive on short previews. Keep one pool per
//! configured width so changing the thread preference still takes effect.

#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn get(
    threads: usize,
) -> Result<std::sync::Arc<rayon::ThreadPool>, rayon::ThreadPoolBuildError> {
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex, OnceLock};

    static POOLS: OnceLock<Mutex<HashMap<usize, Arc<rayon::ThreadPool>>>> = OnceLock::new();
    let pools = POOLS.get_or_init(|| Mutex::new(HashMap::new()));
    let threads = threads.max(1);

    // Benchmark-only control for comparing the previous per-render pool
    // lifecycle against the reusable pool without changing render settings.
    if std::env::var_os("KAMAFEU_BENCH_DISABLE_POOL_REUSE").is_some() {
        return rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|index| format!("kamafeu-render-{index}"))
            .build()
            .map(Arc::new);
    }

    let mut pools = pools.lock().unwrap_or_else(|error| error.into_inner());
    if let Some(pool) = pools.get(&threads) {
        return Ok(Arc::clone(pool));
    }

    let pool = Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|index| format!("kamafeu-render-{index}"))
            .build()?,
    );
    pools.insert(threads, Arc::clone(&pool));
    Ok(pool)
}
