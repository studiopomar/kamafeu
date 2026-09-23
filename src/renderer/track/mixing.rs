use super::TrackRenderer;

impl TrackRenderer {
    /// Transparent soft-knee limiter that scales peaks above `target_peak` and smooths extreme transients.
    pub fn apply_soft_limiter(buffer: &mut [f32], target_peak: f32) {
        if buffer.is_empty() {
            return;
        }
        let max_peak = buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        if max_peak > target_peak && max_peak > 0.0 {
            let scale = target_peak / max_peak;
            for s in buffer.iter_mut() {
                *s *= scale;
            }
        }
        // Transparent soft-knee saturation curve to protect against any localized harmonic bursts.
        let knee_threshold = target_peak * 0.92;
        let head = (1.0f32 - knee_threshold).max(1e-4);
        for s in buffer.iter_mut() {
            if !s.is_finite() {
                *s = 0.0;
                continue;
            }
            let abs_val = s.abs();
            if abs_val > knee_threshold {
                let sign = s.signum();
                let excess = abs_val - knee_threshold;
                let compressed = knee_threshold + head * (excess / head).tanh();
                *s = sign * compressed.min(target_peak);
            }
        }
    }

    pub(super) fn mix_phase_aligned(
        track_buffer: &mut [f32],
        note_samples: &[f32],
        start_sample: usize,
        previous_end_sample: usize,
        crossfade_samples: usize,
        pitch_freq: f64,
        sample_rate: u32,
    ) -> usize {
        if note_samples.is_empty() || start_sample >= track_buffer.len() {
            return previous_end_sample;
        }

        // VC and VCV aliases often start with the outgoing vowel. Let the
        // correlation guard decide whether that lead is periodic enough for a
        // tiny phase correction; it safely returns zero for noisy consonants.
        let overlap = crossfade_samples
            .min(previous_end_sample.saturating_sub(start_sample))
            .min(track_buffer.len() - start_sample)
            .min(note_samples.len());
        // Envelope/oto.ini timing is authoritative. Phase alignment is only a
        // refinement inside an actual crossfade; applying it to every
        // adjacent phone shifts the precomputed envelope in time and can cut
        // or tear CV/VCV transitions across otherwise valid voicebanks.
        let shift = if crossfade_samples >= 8 && overlap >= 8 {
            let raw_shift = crate::renderer::phase::correction(
                &track_buffer[start_sample..start_sample + overlap],
                &note_samples[..overlap],
                pitch_freq,
                sample_rate,
            );
            raw_shift.clamp(
                -(crossfade_samples as isize / 4),
                crossfade_samples as isize / 4,
            )
        } else {
            0
        };
        let start_sample = start_sample
            .saturating_add_signed(shift)
            .min(track_buffer.len());
        let available = (track_buffer.len() - start_sample).min(note_samples.len());

        for (index, &sample) in note_samples.iter().take(available).enumerate() {
            let track_index = start_sample + index;
            // Native and external wavtools have already applied the oto.ini
            // envelope. Applying another fade here creates a hole at every
            // VC/VCV join (and two holes in a VCCV sequence).
            track_buffer[track_index] += sample;
        }

        previous_end_sample.max(start_sample + available)
    }
}
