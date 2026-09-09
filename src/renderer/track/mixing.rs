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

        // Equal-power crossfade across the overlap with whatever the previous
        // note already wrote into `track_buffer`. Without this, two
        // near-full-amplitude notes summed in the overlap region can spike
        // well past 1.0, which the exporter then hard-clips (flat-tops).
        let fade_len = crossfade_samples
            .min(previous_end_sample.saturating_sub(start_sample))
            .min(available);

        for (index, &sample) in note_samples.iter().take(available).enumerate() {
            let track_index = start_sample + index;
            if fade_len > 0 && index < fade_len {
                let phase = (index as f32 + 0.5) / fade_len as f32;
                let w_in = (phase * std::f32::consts::FRAC_PI_2).sin();
                let w_out = (phase * std::f32::consts::FRAC_PI_2).cos();
                track_buffer[track_index] = track_buffer[track_index] * w_out + sample * w_in;
            } else {
                track_buffer[track_index] += sample;
            }
        }

        previous_end_sample.max(start_sample + available)
    }
}
