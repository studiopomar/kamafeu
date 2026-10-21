use crate::project::model::{UTrack, UWavePart};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use web_time::Instant;

#[derive(Debug)]
pub(super) struct WaveformPreview {
    pub(super) bucket_ms: f64,
    pub(super) duration_ms: f64,
    pub(super) peaks: Vec<(f32, f32)>,
}

struct CachedWaveform {
    stamp: u128,
    preview: Arc<WaveformPreview>,
    checked_at: Instant,
}

type WaveformCache = HashMap<String, CachedWaveform>;

pub(super) fn audio_part_level(wave: &UWavePart, track: &UTrack, playhead_ms: f64) -> Option<f32> {
    let duration = wave.duration_ms.max(0.0);
    if duration <= 0.0
        || playhead_ms < wave.position_ms
        || playhead_ms >= wave.position_ms + duration
    {
        return None;
    }
    let preview = cached_waveform(&wave.file_path)?;
    let source_ms =
        (wave.file_offset_ms + playhead_ms - wave.position_ms).clamp(0.0, preview.duration_ms);
    let bucket = (source_ms / preview.bucket_ms).floor() as usize;
    let (low, high) = *preview
        .peaks
        .get(bucket.min(preview.peaks.len().saturating_sub(1)))?;
    let linear_gain = 10.0f32.powf(((wave.volume_db + track.volume_db) as f32) / 20.0);
    Some((low.abs().max(high.abs()) * linear_gain).clamp(0.0, 1.0))
}

pub(super) fn cached_waveform(path: &str) -> Option<Arc<WaveformPreview>> {
    static CACHE: OnceLock<Mutex<WaveformCache>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let now = Instant::now();

    // A waveform is painted every frame. Avoid a filesystem metadata request
    // for every audio part at 60 FPS; checking once per second still refreshes
    // an externally replaced file promptly without stalling the UI.
    if let Ok(cache) = cache.lock() {
        if let Some(cached) = cache.get(path) {
            if now.duration_since(cached.checked_at) < Duration::from_secs(1) {
                return Some(Arc::clone(&cached.preview));
            }
        }
    }

    let stamp = std::fs::metadata(path)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_nanos();

    if let Ok(mut cache) = cache.lock() {
        if let Some(cached) = cache.get_mut(path) {
            cached.checked_at = now;
            if cached.stamp == stamp {
                return Some(Arc::clone(&cached.preview));
            }
        }
    }

    let audio = crate::audio::load_audio_file(path).ok()?;
    let channels = usize::from(audio.channels.max(1));
    let frames = audio.samples.len() / channels;
    if frames == 0 {
        return None;
    }
    // Store one signed peak envelope per millisecond. The painter aggregates
    // these buckets to the current pixel width, preserving transients when
    // zoomed in and preventing aliased bars when zoomed out.
    let bucket_ms = 1.0f64;
    let frames_per_bucket = ((audio.sample_rate as f64 * bucket_ms) / 1000.0)
        .round()
        .max(1.0) as usize;
    let bucket_count = frames.div_ceil(frames_per_bucket).max(1);
    let mut peaks = Vec::with_capacity(bucket_count);
    for bucket in 0..bucket_count {
        let start = bucket * frames_per_bucket;
        let end = (start + frames_per_bucket).min(frames);
        let (mut min, mut max) = (1.0f32, -1.0f32);
        for frame in start..end {
            let sample = (0..channels)
                .map(|channel| audio.samples[frame * channels + channel])
                .sum::<f32>()
                / channels as f32;
            min = min.min(sample);
            max = max.max(sample);
        }
        peaks.push((min, max));
    }
    if let Ok(mut guard) = cache.lock() {
        let preview = Arc::new(WaveformPreview {
            bucket_ms,
            duration_ms: audio.duration_ms,
            peaks,
        });
        guard.insert(
            path.to_owned(),
            CachedWaveform {
                stamp,
                preview: Arc::clone(&preview),
                checked_at: now,
            },
        );
        return Some(preview);
    }
    None
}
