use crate::renderer::ProgressiveChunk;
use std::collections::BTreeMap;

const EPSILON_MS: f64 = 0.25;

#[derive(Debug, Default)]
pub(crate) struct PlaybackChunkBuffer {
    expected_start_ms: Option<f64>,
    pending: BTreeMap<usize, ProgressiveChunk>,
    seen_indices: std::collections::BTreeSet<usize>,
    sample_rate: Option<u32>,
    channels: Option<u16>,
    pub duplicates: usize,
    pub gaps: usize,
    pub out_of_order: usize,
    pub format_mismatches: usize,
    pub timing_mismatches: usize,
}

impl PlaybackChunkBuffer {
    pub fn reset(&mut self, start_ms: f64) {
        self.expected_start_ms = Some(start_ms.max(0.0));
        self.pending.clear();
        self.seen_indices.clear();
        self.sample_rate = None;
        self.channels = None;
        self.duplicates = 0;
        self.gaps = 0;
        self.out_of_order = 0;
        self.format_mismatches = 0;
        self.timing_mismatches = 0;
    }

    pub fn push(&mut self, chunk: ProgressiveChunk) -> Vec<ProgressiveChunk> {
        let expected_duration_ms =
            chunk.audio.frame_count() as f64 * 1000.0 / chunk.audio.sample_rate.max(1) as f64;
        if chunk.audio.sample_rate == 0
            || chunk.audio.channels == 0
            || !chunk.chunk_start_ms.is_finite()
            || !chunk.end_ms().is_finite()
            || expected_duration_ms <= 0.0
        {
            self.timing_mismatches += 1;
            return Vec::new();
        }
        let expected = self.expected_start_ms.get_or_insert(chunk.chunk_start_ms);
        if self.seen_indices.contains(&chunk.chunk_index)
            || self.pending.contains_key(&chunk.chunk_index)
        {
            self.duplicates += 1;
            return Vec::new();
        }
        if chunk.chunk_start_ms + EPSILON_MS < *expected {
            self.out_of_order += 1;
            return Vec::new();
        }
        if let Some(rate) = self.sample_rate {
            if rate != chunk.audio.sample_rate || self.channels != Some(chunk.audio.channels) {
                self.gaps += 1;
                self.format_mismatches += 1;
                return Vec::new();
            }
        } else {
            self.sample_rate = Some(chunk.audio.sample_rate);
            self.channels = Some(chunk.audio.channels);
        }
        self.seen_indices.insert(chunk.chunk_index);
        self.pending.insert(chunk.chunk_index, chunk);

        let mut ready = Vec::new();
        loop {
            let Some((&index, candidate)) = self.pending.iter().next() else {
                break;
            };
            let expected = self.expected_start_ms.unwrap_or(0.0);
            if (candidate.chunk_start_ms - expected).abs() > EPSILON_MS {
                if candidate.chunk_start_ms > expected {
                    self.gaps += 1;
                }
                break;
            }
            let chunk = self.pending.remove(&index).expect("chunk disappeared");
            self.expected_start_ms = Some(chunk.end_ms());
            ready.push(chunk);
        }
        ready
    }

    pub fn pending_len(&self) -> usize {
        self.pending.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::RenderedAudio;

    fn chunk(index: usize, start: f64, duration: f64) -> ProgressiveChunk {
        ProgressiveChunk {
            audio: RenderedAudio {
                samples: vec![0.0; duration as usize],
                sample_rate: 1000,
                channels: 1,
                error: None,
            },
            chunk_start_ms: start,
            chunk_index: index,
            is_final: false,
        }
    }

    #[test]
    fn holds_out_of_order_chunks_until_the_timeline_is_contiguous() {
        let mut buffer = PlaybackChunkBuffer::default();
        buffer.reset(0.0);
        assert!(buffer.push(chunk(1, 100.0, 100.0)).is_empty());
        let ready = buffer.push(chunk(0, 0.0, 100.0));
        assert_eq!(
            ready.iter().map(|c| c.chunk_index).collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(buffer.pending_len(), 0);
    }

    #[test]
    fn rejects_duplicates_and_reports_gaps() {
        let mut buffer = PlaybackChunkBuffer::default();
        buffer.reset(0.0);
        assert_eq!(buffer.push(chunk(0, 0.0, 100.0)).len(), 1);
        assert!(buffer.push(chunk(0, 0.0, 100.0)).is_empty());
        assert!(buffer.push(chunk(2, 300.0, 100.0)).is_empty());
        assert_eq!(buffer.duplicates, 1);
        assert!(buffer.gaps > 0);
    }

    #[test]
    fn rejects_channel_changes_inside_one_timeline() {
        let mut buffer = PlaybackChunkBuffer::default();
        buffer.reset(0.0);
        assert_eq!(buffer.push(chunk(0, 0.0, 100.0)).len(), 1);
        let mut stereo = chunk(1, 100.0, 100.0);
        stereo.audio.channels = 2;
        stereo.audio.samples = vec![0.0; 200];
        assert!(buffer.push(stereo).is_empty());
        assert_eq!(buffer.format_mismatches, 1);
    }

    #[test]
    fn rejects_invalid_timing_before_touching_waveform_order() {
        let mut buffer = PlaybackChunkBuffer::default();
        buffer.reset(0.0);
        let mut invalid = chunk(0, 0.0, 100.0);
        invalid.audio.sample_rate = 0;
        assert!(buffer.push(invalid).is_empty());
        assert_eq!(buffer.timing_mismatches, 1);
        assert_eq!(buffer.pending_len(), 0);
    }
}
