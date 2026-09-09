use super::TrackRenderer;

impl TrackRenderer {
    pub(super) fn mix_phase_aligned(
        track_buffer: &mut [f32],
        note_samples: &[f32],
        start_sample: usize,
        previous_end_sample: usize,
        crossfade_samples: usize,
        pitch_freq: f64,
        sample_rate: u32,
        is_transition: bool,
    ) -> usize {
        if note_samples.is_empty() || start_sample >= track_buffer.len() {
            return previous_end_sample;
        }

        let shift = if is_transition {
            0
        } else {
            let overlap = crossfade_samples
                .min(previous_end_sample.saturating_sub(start_sample))
                .min(track_buffer.len() - start_sample)
                .min(note_samples.len());
            crate::renderer::phase::correction(
                &track_buffer[start_sample..start_sample + overlap],
                &note_samples[..overlap],
                pitch_freq,
                sample_rate,
            )
        };
        let start_sample = start_sample
            .saturating_add_signed(shift)
            .min(track_buffer.len());
        let available = (track_buffer.len() - start_sample).min(note_samples.len());
        for (index, &sample) in note_samples.iter().take(available).enumerate() {
            let track_index = start_sample + index;
            track_buffer[track_index] += sample;
        }

        previous_end_sample.max(start_sample + available)
    }
}
