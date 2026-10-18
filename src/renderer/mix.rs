use super::project::ProjectRenderer;
use std::sync::{OnceLock, RwLock};

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline]
pub(super) fn accumulate_stereo_scaled(
    destination: &mut [f32],
    source: &[f32],
    left_gain: f32,
    right_gain: f32,
) {
    if std::is_x86_feature_detected!("sse2") {
        // SAFETY: SSE2 is checked immediately above and the slices are valid
        // for the unaligned four-float loads/stores below.
        unsafe { accumulate_stereo_scaled_sse2(destination, source, left_gain, right_gain) };
        return;
    }

    accumulate_stereo_scaled_scalar(destination, source, left_gain, right_gain);
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse2")]
unsafe fn accumulate_stereo_scaled_sse2(
    destination: &mut [f32],
    source: &[f32],
    left_gain: f32,
    right_gain: f32,
) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::{_mm_add_ps, _mm_loadu_ps, _mm_mul_ps, _mm_set_ps, _mm_storeu_ps};
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{_mm_add_ps, _mm_loadu_ps, _mm_mul_ps, _mm_set_ps, _mm_storeu_ps};

    let gains = _mm_set_ps(right_gain, left_gain, right_gain, left_gain);
    let vector_len = destination.len().min(source.len()) / 4 * 4;
    for index in (0..vector_len).step_by(4) {
        let mixed = _mm_mul_ps(_mm_loadu_ps(source.as_ptr().add(index)), gains);
        let existing = _mm_loadu_ps(destination.as_ptr().add(index));
        _mm_storeu_ps(
            destination.as_mut_ptr().add(index),
            _mm_add_ps(existing, mixed),
        );
    }

    accumulate_stereo_scaled_scalar(
        &mut destination[vector_len..],
        &source[vector_len..],
        left_gain,
        right_gain,
    );
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
#[inline]
pub(super) fn accumulate_stereo_scaled(
    destination: &mut [f32],
    source: &[f32],
    left_gain: f32,
    right_gain: f32,
) {
    accumulate_stereo_scaled_scalar(destination, source, left_gain, right_gain);
}

#[inline]
fn accumulate_stereo_scaled_scalar(
    destination: &mut [f32],
    source: &[f32],
    left_gain: f32,
    right_gain: f32,
) {
    let frames = destination.len().min(source.len()) / 2;
    for frame in 0..frames {
        let source = &source[frame * 2..frame * 2 + 2];
        let destination = &mut destination[frame * 2..frame * 2 + 2];
        destination[0] += source[0] * left_gain;
        destination[1] += source[1] * right_gain;
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[inline]
pub(super) fn accumulate_mono_to_stereo(
    destination: &mut [f32],
    source: &[f32],
    left_gain: f32,
    right_gain: f32,
) {
    if std::is_x86_feature_detected!("sse2") {
        // SAFETY: SSE2 is checked immediately above and the destination has
        // two samples per source frame.
        unsafe { accumulate_mono_to_stereo_sse2(destination, source, left_gain, right_gain) };
        return;
    }

    accumulate_mono_to_stereo_scalar(destination, source, left_gain, right_gain);
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse2")]
unsafe fn accumulate_mono_to_stereo_sse2(
    destination: &mut [f32],
    source: &[f32],
    left_gain: f32,
    right_gain: f32,
) {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::{
        _mm_add_ps, _mm_loadu_ps, _mm_mul_ps, _mm_set1_ps, _mm_storeu_ps, _mm_unpackhi_ps,
        _mm_unpacklo_ps,
    };
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::{
        _mm_add_ps, _mm_loadu_ps, _mm_mul_ps, _mm_set1_ps, _mm_storeu_ps, _mm_unpackhi_ps,
        _mm_unpacklo_ps,
    };

    let left_factor = _mm_set1_ps(left_gain);
    let right_factor = _mm_set1_ps(right_gain);
    let source_len = source.len() / 4 * 4;
    for index in (0..source_len).step_by(4) {
        let samples = _mm_loadu_ps(source.as_ptr().add(index));
        let left = _mm_mul_ps(samples, left_factor);
        let right = _mm_mul_ps(samples, right_factor);
        let low = _mm_unpacklo_ps(left, right);
        let high = _mm_unpackhi_ps(left, right);
        let destination_index = index * 2;
        let existing_low = _mm_loadu_ps(destination.as_ptr().add(destination_index));
        let existing_high = _mm_loadu_ps(destination.as_ptr().add(destination_index + 4));
        _mm_storeu_ps(
            destination.as_mut_ptr().add(destination_index),
            _mm_add_ps(existing_low, low),
        );
        _mm_storeu_ps(
            destination.as_mut_ptr().add(destination_index + 4),
            _mm_add_ps(existing_high, high),
        );
    }

    accumulate_mono_to_stereo_scalar(
        &mut destination[source_len * 2..],
        &source[source_len..],
        left_gain,
        right_gain,
    );
}

#[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
#[inline]
pub(super) fn accumulate_mono_to_stereo(
    destination: &mut [f32],
    source: &[f32],
    left_gain: f32,
    right_gain: f32,
) {
    accumulate_mono_to_stereo_scalar(destination, source, left_gain, right_gain);
}

#[inline]
fn accumulate_mono_to_stereo_scalar(
    destination: &mut [f32],
    source: &[f32],
    left_gain: f32,
    right_gain: f32,
) {
    for (frame, &sample) in source.iter().enumerate() {
        destination[frame * 2] += sample * left_gain;
        destination[frame * 2 + 1] += sample * right_gain;
    }
}

#[derive(Clone, Copy)]
struct RenderLimiterConfig {
    enabled: bool,
    peak_db: f32,
}

fn render_limiter_config() -> &'static RwLock<RenderLimiterConfig> {
    static CONFIG: OnceLock<RwLock<RenderLimiterConfig>> = OnceLock::new();
    CONFIG.get_or_init(|| {
        RwLock::new(RenderLimiterConfig {
            enabled: true,
            peak_db: -1.0,
        })
    })
}

pub(super) fn configured_render_limiter_peak() -> Option<f32> {
    let config = render_limiter_config().read().ok()?;
    config.enabled.then(|| 10.0_f32.powf(config.peak_db / 20.0))
}

pub(super) fn set_render_limiter(enabled: bool, peak_db: f32) {
    if let Ok(mut config) = render_limiter_config().write() {
        config.enabled = enabled;
        config.peak_db = peak_db.clamp(-24.0, 0.0);
    }
}

impl ProjectRenderer {
    pub(super) fn mix_track_into(
        stereo: &mut Vec<f32>,
        mono: &mut [f32],
        volume_db: f64,
        pan: f64,
        fx_rack: Option<&crate::audio::FxRackConfig>,
        sample_rate: u32,
    ) {
        let required_len = mono.len().saturating_mul(2);
        if stereo.len() < required_len {
            stereo.resize(required_len, 0.0);
        }

        let gain = 10.0f32.powf((volume_db / 20.0) as f32);
        let pan = pan.clamp(-1.0, 1.0) as f32;
        let left_gain = gain * (1.0 - pan.max(0.0));
        let right_gain = gain * (1.0 + pan.min(0.0));

        if let Some(fx) = fx_rack {
            if fx.master_enabled {
                // Time-based effects like Reverb and Delay require a decay tail ringout
                // so the ambient reflections do not abruptly truncate at the note boundary.
                let tail_frames = (sample_rate as usize * 25) / 10; // 2.5 seconds tail
                let tail_samples = tail_frames * 2;
                let mut track_stereo = vec![0.0; required_len + tail_samples];
                accumulate_mono_to_stereo(
                    &mut track_stereo[..required_len],
                    mono,
                    left_gain,
                    right_gain,
                );

                let mut processor = crate::audio::FxRackProcessor::new(fx.clone(), sample_rate);
                processor.process_interleaved(&mut track_stereo, sample_rate);

                if stereo.len() < track_stereo.len() {
                    stereo.resize(track_stereo.len(), 0.0);
                }

                accumulate_stereo_scaled(
                    &mut stereo[..track_stereo.len()],
                    &track_stereo,
                    1.0,
                    1.0,
                );
                return;
            }
        }

        accumulate_mono_to_stereo(&mut stereo[..required_len], mono, left_gain, right_gain);
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn mix_wave_part_into(
        stereo: &mut Vec<f32>,
        decoded: &crate::audio::DecodedAudio,
        sample_rate: u32,
        wave_position_ms: f64,
        wave_file_offset_ms: f64,
        start_ms: f64,
        volume_db: f64,
        pan: f64,
    ) {
        if decoded.samples.is_empty() || decoded.sample_rate == 0 || decoded.channels == 0 {
            return;
        }

        let decoded_stereo = decoded.to_stereo_at_sample_rate(sample_rate);
        let src_frames = decoded_stereo.len() / 2;
        if src_frames == 0 {
            return;
        }

        let gain = 10.0f32.powf((volume_db / 20.0) as f32);
        let pan = pan.clamp(-1.0, 1.0) as f32;
        let left_gain = gain * (1.0 - pan.max(0.0));
        let right_gain = gain * (1.0 + pan.min(0.0));

        let offset_ms = wave_position_ms - start_ms;
        let file_offset_frames =
            ((wave_file_offset_ms / 1000.0) * sample_rate as f64).round() as isize;

        let dest_start_frame = if offset_ms >= 0.0 {
            ((offset_ms / 1000.0) * sample_rate as f64).round() as usize
        } else {
            0
        };

        let src_start_frame = if offset_ms >= 0.0 {
            file_offset_frames.max(0) as usize
        } else {
            (file_offset_frames + ((-offset_ms / 1000.0) * sample_rate as f64).round() as isize)
                .max(0) as usize
        };

        if src_start_frame >= src_frames {
            return;
        }

        let frames_to_mix = src_frames - src_start_frame;
        let required_len = (dest_start_frame + frames_to_mix) * 2;
        if stereo.len() < required_len {
            stereo.resize(required_len, 0.0);
        }

        let source_start = src_start_frame * 2;
        let source_end = source_start + frames_to_mix * 2;
        let destination_start = dest_start_frame * 2;
        let destination_end = destination_start + frames_to_mix * 2;
        accumulate_stereo_scaled(
            &mut stereo[destination_start..destination_end],
            &decoded_stereo[source_start..source_end],
            left_gain,
            right_gain,
        );
    }
}
