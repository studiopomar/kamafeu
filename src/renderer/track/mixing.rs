use super::TrackRenderer;
use std::sync::{OnceLock, RwLock};

fn crossfade_curve() -> &'static RwLock<String> {
    static CURVE: OnceLock<RwLock<String>> = OnceLock::new();
    CURVE.get_or_init(|| RwLock::new("Equal Power S-Curve".to_string()))
}

impl TrackRenderer {
    pub(crate) fn set_crossfade_curve_impl(curve: &str) {
        let normalized = match curve {
            "Linear" => "Linear",
            "Logarítmica" | "Logarithmic" => "Logarithmic",
            _ => "Equal Power S-Curve",
        };
        if let Ok(mut value) = crossfade_curve().write() {
            *value = normalized.to_string();
        }
    }

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
        Self::mix_phase_aligned_impl(
            track_buffer,
            note_samples,
            start_sample,
            previous_end_sample,
            crossfade_samples,
            pitch_freq,
            sample_rate,
            false,
        )
    }

    /// Mixes a rendered phone while matching its level to the already-rendered
    /// tail. The resampler and wavtool provide the envelope shape, but aliases
    /// from different recordings can still have noticeably different loudness.
    /// Matching RMS here removes that audible jump without adding another fade.
    pub(super) fn mix_level_matched(
        track_buffer: &mut [f32],
        note_samples: &[f32],
        start_sample: usize,
        previous_end_sample: usize,
        crossfade_samples: usize,
        pitch_freq: f64,
        sample_rate: u32,
    ) -> usize {
        Self::mix_phase_aligned_impl(
            track_buffer,
            note_samples,
            start_sample,
            previous_end_sample,
            crossfade_samples,
            pitch_freq,
            sample_rate,
            true,
        )
    }

    fn mix_phase_aligned_impl(
        track_buffer: &mut [f32],
        note_samples: &[f32],
        start_sample: usize,
        previous_end_sample: usize,
        crossfade_samples: usize,
        pitch_freq: f64,
        sample_rate: u32,
        level_match: bool,
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

        // Use the stable middle of the actual envelope overlap. The edges can
        // contain consonant attacks or release tails and are poor loudness
        // references. Do not match when either side is effectively silent.
        let level_gain = if level_match {
            let level_overlap = previous_end_sample
                .saturating_sub(start_sample)
                .min(available);
            if level_overlap >= 32 {
                let trim = level_overlap / 10;
                let level_start = trim;
                let level_end = level_overlap.saturating_sub(trim);
                let previous_rms =
                    rms(&track_buffer[start_sample + level_start..start_sample + level_end]);
                let incoming_rms = rms(&note_samples[level_start..level_end]);
                match_gain(previous_rms, incoming_rms)
            } else if start_sample > 0 && available >= 32 {
                // Some oto entries meet exactly at the boundary. In that
                // case compare a short tail of the preceding rendered audio
                // with the beginning of the incoming alias instead of giving
                // up on level matching entirely.
                let window = 882.min(start_sample).min(available);
                let previous_rms = rms(&track_buffer[start_sample - window..start_sample]);
                let incoming_rms = rms(&note_samples[..window]);
                match_gain(previous_rms, incoming_rms)
            } else {
                1.0
            }
        } else {
            1.0
        };

        for (index, &sample) in note_samples.iter().take(available).enumerate() {
            let track_index = start_sample + index;
            // Native and external wavtools have already applied the oto.ini
            // envelope. Applying another fade here creates a hole at every
            // VC/VCV join (and two holes in a VCCV sequence).
            // The resampler/wavtool already applied the oto.ini envelope.
            // Never apply a second fade here: doing so creates an energy dip
            // at every VC/VCV boundary. The configured crossfade curve is
            // reserved for engines that explicitly request an un-enveloped
            // segment; native and classic UTAU paths both provide envelopes.
            track_buffer[track_index] += sample * level_gain;
        }

        previous_end_sample.max(start_sample + available)
    }
}

fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let energy = samples.iter().map(|sample| sample * sample).sum::<f32>();
    (energy / samples.len() as f32).sqrt()
}

fn match_gain(previous_rms: f32, incoming_rms: f32) -> f32 {
    if previous_rms > 1e-5 && incoming_rms > 1e-5 {
        (previous_rms / incoming_rms).clamp(0.65, 1.5)
    } else {
        1.0
    }
}

pub(crate) fn set_crossfade_curve(curve: &str) {
    TrackRenderer::set_crossfade_curve_impl(curve);
}
