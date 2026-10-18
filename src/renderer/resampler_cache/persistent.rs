use super::{cache_limits, CacheLimits, CACHE_SCHEMA};
use crate::drivers::ResamplerExecutionInfo;
use sha2::{Digest, Sha256};
use std::io::Write;

pub(super) fn persistent_cache_path_in(
    directory: &std::path::Path,
    key: u64,
) -> std::path::PathBuf {
    directory.join(format!("res-{key:016x}.wav"))
}

pub(super) fn persistent_execution_path_in(
    directory: &std::path::Path,
    key: u64,
) -> std::path::PathBuf {
    directory.join(format!("res-{key:016x}.json"))
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PersistentExecution {
    schema: u32,
    cache_key: u64,
    sample_rate: u32,
    sample_count: usize,
    audio_sha256: String,
    execution: ResamplerExecutionInfo,
}

fn cached_audio_sha256(samples: &[f32]) -> String {
    let mut hasher = Sha256::new();
    for sample in samples {
        hasher.update(sample.to_le_bytes());
    }
    format!("{:x}", hasher.finalize())
}

pub(super) fn load_persistent(
    key: u64,
    sample_rate: u32,
) -> Option<(Vec<f32>, ResamplerExecutionInfo)> {
    load_persistent_in(&super::persistent_cache_dir(), key, sample_rate)
}

pub(super) fn load_persistent_in(
    directory: &std::path::Path,
    key: u64,
    sample_rate: u32,
) -> Option<(Vec<f32>, ResamplerExecutionInfo)> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (directory, key, sample_rate);
        None
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let path = persistent_cache_path_in(directory, key);
        let execution_path = persistent_execution_path_in(directory, key);
        let execution = std::fs::read(&execution_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<PersistentExecution>(&bytes).ok())
            .filter(|value| {
                value.schema == CACHE_SCHEMA
                    && value.cache_key == key
                    && value.sample_rate == sample_rate
            });
        let loaded = crate::renderer::TrackRenderer::load_wav_samples(&path);
        match loaded {
            Ok((samples, cached_rate))
                if !samples.is_empty()
                    && cached_rate == sample_rate
                    && execution.as_ref().is_some_and(|metadata| {
                        metadata.sample_count == samples.len()
                            && metadata.audio_sha256 == cached_audio_sha256(&samples)
                    }) =>
            {
                Some((samples, execution.unwrap().execution))
            }
            Ok(_) | Err(_) => {
                if path.is_file() {
                    let _ = std::fs::remove_file(path);
                }
                let _ = std::fs::remove_file(execution_path);
                None
            }
        }
    }
}

pub(super) fn store_persistent(
    key: u64,
    samples: &[f32],
    sample_rate: u32,
    execution: &ResamplerExecutionInfo,
) {
    store_persistent_in(
        &super::persistent_cache_dir(),
        key,
        samples,
        sample_rate,
        execution,
    )
}

pub(super) fn store_persistent_in(
    directory: &std::path::Path,
    key: u64,
    samples: &[f32],
    sample_rate: u32,
    execution: &ResamplerExecutionInfo,
) {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (directory, key, samples, sample_rate, execution);
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if samples.is_empty() {
            return;
        }
        if std::fs::create_dir_all(directory).is_err() {
            return;
        }
        let destination = persistent_cache_path_in(directory, key);
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
            let metadata = PersistentExecution {
                schema: CACHE_SCHEMA,
                cache_key: key,
                sample_rate,
                sample_count: samples.len(),
                audio_sha256: cached_audio_sha256(samples),
                execution: execution.clone(),
            };
            if let Ok(bytes) = serde_json::to_vec(&metadata) {
                if let Ok(mut file) = tempfile::Builder::new()
                    .prefix("res-meta-")
                    .suffix(".json")
                    .tempfile_in(directory)
                {
                    if file.write_all(&bytes).is_ok() {
                        let _ = file.persist(persistent_execution_path_in(directory, key));
                    }
                }
            }
            prune_persistent_cache(directory);
        }
    }
}

fn prune_persistent_cache(directory: &std::path::Path) {
    let limit = cache_limits()
        .read()
        .map(|limits| limits.max_disk_bytes)
        .unwrap_or_else(|_| CacheLimits::default().max_disk_bytes);
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
