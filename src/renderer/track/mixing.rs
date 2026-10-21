use super::TrackRenderer;
use std::sync::{OnceLock, RwLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MixResult {
    pub(super) end_sample: usize,
    pub(super) correction_samples: isize,
    pub(super) source_trim_samples: usize,
}

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

    /// Mono compatibility wrapper around [`TrackRenderer::apply_render_limiter`].
    ///
    /// Older callers did not pass the sample rate; the limiter time constants
    /// are then evaluated at 44.1 kHz. Interleaved buffers must use
    /// `apply_render_limiter` so the channels share one gain envelope.
    pub fn apply_soft_limiter(buffer: &mut [f32], target_peak: f32) {
        Self::apply_render_limiter(buffer, 1, 44_100, target_peak);
    }

    /// Transparent look-ahead true-peak limiter for rendered audio.
    ///
    /// Only the neighbourhood of an over is attenuated (2 ms S-shaped attack,
    /// 80 ms release); audio below `target_peak` is returned unchanged and
    /// non-finite samples are replaced by silence. See
    /// [`crate::dsp::PeakLimiter`].
    pub fn apply_render_limiter(
        buffer: &mut [f32],
        channels: usize,
        sample_rate: u32,
        target_peak: f32,
    ) -> crate::dsp::LimiterReport {
        crate::dsp::PeakLimiter::new(target_peak).process_interleaved(buffer, channels, sample_rate)
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

    /// Places a segment using a correction already resolved from raw phase
    /// signatures. No gain, RMS matching, or second crossfade is applied.
    /// When a negative correction crosses the track origin, the unavailable
    /// source prefix is skipped so the remaining audio stays time-aligned.
    pub(super) fn mix_with_resolved_correction(
        track_buffer: &mut [f32],
        note_samples: &[f32],
        start_sample: usize,
        previous_end_sample: usize,
        correction_samples: isize,
    ) -> MixResult {
        if note_samples.is_empty() || track_buffer.is_empty() {
            return MixResult {
                end_sample: previous_end_sample,
                correction_samples,
                source_trim_samples: 0,
            };
        }

        let corrected_start = start_sample as i128 + correction_samples as i128;
        let (destination_start, source_start) = if corrected_start < 0 {
            (0usize, (-corrected_start) as usize)
        } else {
            (corrected_start.min(usize::MAX as i128) as usize, 0usize)
        };
        if destination_start >= track_buffer.len() || source_start >= note_samples.len() {
            return MixResult {
                end_sample: previous_end_sample,
                correction_samples,
                source_trim_samples: source_start,
            };
        }

        let available =
            (track_buffer.len() - destination_start).min(note_samples.len() - source_start);
        accumulate_samples(
            &mut track_buffer[destination_start..destination_start + available],
            &note_samples[source_start..source_start + available],
        );
        MixResult {
            end_sample: previous_end_sample.max(destination_start + available),
            correction_samples,
            source_trim_samples: source_start,
        }
    }
}

pub(crate) fn set_crossfade_curve(curve: &str) {
    TrackRenderer::set_crossfade_curve_impl(curve);
}
