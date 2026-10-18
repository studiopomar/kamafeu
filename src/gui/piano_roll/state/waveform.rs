use super::PianoRollState;
use std::sync::{OnceLock, RwLock};

fn waveform_bucket_index(time_ms: f64) -> i64 {
    let bucket_ms = f64::from(waveform_cache_resolution());
    (time_ms / bucket_ms).floor() as i64
}

fn waveform_bucket_time(bucket: i64) -> f32 {
    (bucket as f64 * f64::from(waveform_cache_resolution())) as f32
}

/// Builds peaks on the global timeline.  The start of a progressive chunk is
/// only an offset into that timeline; it is never the origin of a new mip
/// grid.  This is also used by complete previews so both paths have identical
/// bucket boundaries.
fn waveform_buckets(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
    start_offset_ms: f64,
) -> Vec<(f32, f32, f32)> {
    if samples.is_empty() || sample_rate == 0 || channels == 0 || !start_offset_ms.is_finite() {
        return Vec::new();
    }

    let ch = usize::from(channels);
    let total_frames = samples.len() / ch;
    if total_frames == 0 {
        return Vec::new();
    }

    let ms_per_frame = 1000.0 / sample_rate as f64;
    let mut buckets = std::collections::BTreeMap::<i64, (f32, f32)>::new();
    for (frame_index, frame) in samples.chunks(ch).take(total_frames).enumerate() {
        let time_ms = start_offset_ms + frame_index as f64 * ms_per_frame;
        let bucket = waveform_bucket_index(time_ms);
        for &sample in frame {
            if !sample.is_finite() {
                continue;
            }
            let entry = buckets.entry(bucket).or_insert((sample, sample));
            entry.0 = entry.0.min(sample);
            entry.1 = entry.1.max(sample);
        }
    }

    buckets
        .into_iter()
        .map(|(bucket, (min, max))| (waveform_bucket_time(bucket), min, max))
        .collect()
}

impl PianoRollState {
    pub fn update_rendered_waveform(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
        channels: u16,
        start_offset_ms: f64,
    ) {
        self.rendered_waveform_peaks =
            waveform_buckets(samples, sample_rate, channels, start_offset_ms);
        self.rendered_waveform_peaks.shrink_to_fit();
    }

    pub fn merge_rendered_waveform(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
        channels: u16,
        start_offset_ms: f64,
    ) {
        let chunk_peaks = waveform_buckets(samples, sample_rate, channels, start_offset_ms);
        if chunk_peaks.is_empty() {
            return;
        }

        // Chunks may start and end in the middle of a bucket. Merge by the
        // absolute bucket timestamp instead of replacing a timestamp range.
        // This keeps one stable grid across page boundaries and preserves the
        // strongest peak when a bucket is delivered in multiple chunks.
        let mut merged = std::collections::BTreeMap::<i64, (f32, f32)>::new();
        for &(time, min, max) in &self.rendered_waveform_peaks {
            let bucket = waveform_bucket_index(time as f64);
            merged.insert(bucket, (min, max));
        }
        for &(time, min, max) in &chunk_peaks {
            let bucket = waveform_bucket_index(time as f64);
            let entry = merged.entry(bucket).or_insert((min, max));
            entry.0 = entry.0.min(min);
            entry.1 = entry.1.max(max);
        }
        self.rendered_waveform_peaks = merged
            .into_iter()
            .map(|(bucket, (min, max))| (waveform_bucket_time(bucket), min, max))
            .collect();
    }

    pub fn append_rendered_waveform(
        &mut self,
        samples: &[f32],
        sample_rate: u32,
        channels: u16,
        start_offset_ms: f64,
    ) {
        self.merge_rendered_waveform(samples, sample_rate, channels, start_offset_ms);
    }

    pub fn waveform_min_max_at(&self, time_ms: f32) -> Option<(f32, f32)> {
        let first = self.rendered_waveform_peaks.first()?.0;
        let last = self.rendered_waveform_peaks.last()?.0;
        if time_ms < first || time_ms > last {
            return None;
        }
        let index = match self
            .rendered_waveform_peaks
            .binary_search_by(|(time, _, _)| {
                time.partial_cmp(&time_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }) {
            Ok(index) => index,
            Err(index) => index,
        };
        if index == 0 {
            let p = self.rendered_waveform_peaks[0];
            return Some((p.1, p.2));
        }
        if index >= self.rendered_waveform_peaks.len() {
            return self
                .rendered_waveform_peaks
                .last()
                .map(|(_, min_v, max_v)| (*min_v, *max_v));
        }
        let (t0, min0, max0) = self.rendered_waveform_peaks[index - 1];
        let (t1, min1, max1) = self.rendered_waveform_peaks[index];
        if (t1 - t0).abs() < f32::EPSILON {
            return Some((min0, max0));
        }
        let frac = ((time_ms - t0) / (t1 - t0)).clamp(0.0, 1.0);
        let min_v = min0 + frac * (min1 - min0);
        let max_v = max0 + frac * (max1 - max0);
        Some((min_v, max_v))
    }

    /// Peak envelope covering the complete time interval represented by one
    /// screen pixel. Sampling only the center point loses transients whenever
    /// the timeline is zoomed out.
    pub fn waveform_min_max_between(&self, start_ms: f32, end_ms: f32) -> Option<(f32, f32)> {
        if self.rendered_waveform_peaks.is_empty() || end_ms < start_ms {
            return None;
        }
        let waveform_start = self.rendered_waveform_peaks.first()?.0;
        let waveform_end = self.rendered_waveform_peaks.last()?.0;
        if end_ms < waveform_start || start_ms > waveform_end {
            return None;
        }
        let first = self
            .rendered_waveform_peaks
            .partition_point(|(time, _, _)| *time < start_ms)
            .saturating_sub(1);
        let end = self
            .rendered_waveform_peaks
            .partition_point(|(time, _, _)| *time <= end_ms)
            .max(first + 1)
            .min(self.rendered_waveform_peaks.len());
        let (mut min_value, mut max_value) = (1.0f32, -1.0f32);
        for &(_, min, max) in &self.rendered_waveform_peaks[first..end] {
            min_value = min_value.min(min);
            max_value = max_value.max(max);
        }
        (max_value >= min_value).then_some((min_value, max_value))
    }

    pub fn waveform_display_peak(&self) -> f32 {
        self.rendered_waveform_peaks
            .iter()
            .map(|(_, min, max)| min.abs().max(max.abs()))
            .fold(0.0f32, f32::max)
    }

    pub fn waveform_amplitude_at(&self, time_ms: f32) -> Option<f32> {
        self.waveform_min_max_at(time_ms)
            .map(|(min_v, max_v)| min_v.abs().max(max_v.abs()))
    }
}

fn waveform_cache_resolution_cell() -> &'static RwLock<u32> {
    static RESOLUTION: OnceLock<RwLock<u32>> = OnceLock::new();
    RESOLUTION.get_or_init(|| RwLock::new(2))
}

fn waveform_cache_resolution() -> u32 {
    waveform_cache_resolution_cell()
        .read()
        .map(|value| (*value).clamp(1, 8))
        .unwrap_or(2)
}

pub fn set_waveform_cache_resolution(resolution: u32) {
    if let Ok(mut value) = waveform_cache_resolution_cell().write() {
        *value = resolution.clamp(1, 8);
    }
}
