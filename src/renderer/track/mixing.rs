use super::TrackRenderer;
use std::sync::{OnceLock, RwLock};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline]
fn accumulate_samples(destination: &mut [f32], source: &[f32]) {
    if std::is_x86_feature_detected!("sse2") {
        // SAFETY: the feature is checked immediately above and both slices
        // are valid for the unaligned four-float loads/stores used below.
        unsafe { accumulate_samples_sse2(destination, source) };
        return;
    }

    accumulate_samples_scalar(destination, source);
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse2")]
unsafe fn accumulate_samples_sse2(destination: &mut [f32], source: &[f32]) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::{_mm_add_ps, _mm_loadu_ps, _mm_storeu_ps};
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{_mm_add_ps, _mm_loadu_ps, _mm_storeu_ps};

    let vector_len = destination.len().min(source.len()) / 4 * 4;
    for index in (0..vector_len).step_by(4) {
        let dst = _mm_loadu_ps(destination.as_ptr().add(index));
        let src = _mm_loadu_ps(source.as_ptr().add(index));
        _mm_storeu_ps(destination.as_mut_ptr().add(index), _mm_add_ps(dst, src));
    }

    accumulate_samples_scalar(&mut destination[vector_len..], &source[vector_len..]);
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
#[inline]
fn accumulate_samples(destination: &mut [f32], source: &[f32]) {
    accumulate_samples_scalar(destination, source);
}

#[inline]
fn accumulate_samples_scalar(destination: &mut [f32], source: &[f32]) {
    for (dst, &src) in destination.iter_mut().zip(source) {
        *dst += src;
    }
}

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

        let destination = &mut track_buffer[start_sample..start_sample + available];
        let source = &note_samples[..available];
        accumulate_samples(destination, source);

        previous_end_sample.max(start_sample + available)
    }
}

pub(crate) fn set_crossfade_curve(curve: &str) {
    TrackRenderer::set_crossfade_curve_impl(curve);
}
