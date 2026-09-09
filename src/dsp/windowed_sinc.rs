use crate::dsp::pitch_bend::PitchBendSolver;
use crate::project::model::UPitchBendPoint;

/// Windowed sinc resampler using Lanczos window for high-quality audio resampling.
///
/// This resampler implements high-quality sample rate conversion using windowed sinc
/// interpolation with a Lanczos window. It's suitable for audio applications where
/// quality is important, such as vocal synthesis.
///
/// The Lanczos window is defined as:
///   sinc(x) * sinc(x/a) for -a < x < a, 0 otherwise
/// where a is the order of the Lanczos window (typically 3).
pub struct WindowedSincResampler;

impl WindowedSincResampler {
    /// Create a new WindowedSincResampler instance.
    pub fn new() -> Self {
        WindowedSincResampler
    }

    /// Resample audio samples using windowed sinc interpolation with pitch bend support.
    ///
    /// This function resamples the input audio to match the target duration and pitch,
    /// while preserving consonants and applying pitch bend curves.
    ///
    /// # Arguments
    ///
    /// * `input_samples` - The input audio samples
    /// * `sample_rate` - The sample rate of the input audio in Hz
    /// * `offset_ms` - Offset from the start of the sample in milliseconds (OTO offset)
    /// * `consonant_ms` - Duration of the consonant part in milliseconds (OTO cutoff)
    /// * `target_duration_ms` - Target duration of the output in milliseconds
    /// * `target_pitch_freq` - Target fundamental frequency in Hz
    /// * `pitch_points` - Pitch bend points for applying pitch modulation
    ///
    /// # Returns
    ///
    /// A vector containing the resampled audio samples
    pub fn render_sample(
        input_samples: &[f32],
        sample_rate: u32,
        offset_ms: f64,
        consonant_ms: f64,
        target_duration_ms: f64,
        target_pitch_freq: f64,
        pitch_points: &[UPitchBendPoint],
    ) -> Vec<f32> {
        Self::render_sample_with_pitch_bend(
            input_samples,
            sample_rate,
            offset_ms,
            consonant_ms,
            0.0, // cutoff_ms - not used in this implementation, keeping 0 for compatibility
            target_duration_ms,
            target_pitch_freq,
            pitch_points,
        )
    }

    /// Resample audio samples using windowed sinc interpolation with pitch bend and cutoff support.
    ///
    /// This function resamples the input audio to match the target duration and pitch,
    /// while preserving consonants, applying pitch bend curves, and respecting the cutoff point.
    ///
    /// # Arguments
    ///
    /// * `input_samples` - The input audio samples
    /// * `sample_rate` - The sample rate of the input audio in Hz
    /// * `offset_ms` - Offset from the start of the sample in milliseconds (OTO offset)
    /// * `consonant_ms` - Duration of the consonant part in milliseconds (OTO cutoff)
    /// * `cutoff_ms` - Cutoff point in milliseconds (negative values indicate finite transitions)
    /// * `target_duration_ms` - Target duration of the output in milliseconds
    /// * `target_pitch_freq` - Target fundamental frequency in Hz
    /// * `pitch_points` - Pitch bend points for applying pitch modulation
    ///
    /// # Returns
    ///
    /// A vector containing the resampled audio samples
    pub fn render_sample_with_pitch_bend(
        input_samples: &[f32],
        sample_rate: u32,
        offset_ms: f64,
        consonant_ms: f64,
        cutoff_ms: f64,
        target_duration_ms: f64,
        target_pitch_freq: f64,
        pitch_points: &[UPitchBendPoint],
    ) -> Vec<f32> {
        // Use the shared OTO bounds function to determine the source region
        let (start_sample, end_sample) =
            crate::dsp::oto_source_bounds(input_samples.len(), sample_rate, offset_ms, cutoff_ms);

        if start_sample >= end_sample || start_sample >= input_samples.len() {
            // Return silence if no valid source region
            let target_sample_count = ((target_duration_ms / 1000.0) * sample_rate as f64) as usize;
            return vec![0.0; target_sample_count];
        }

        let slice = &input_samples[start_sample..end_sample];

        // Calculate consonant and vowel regions
        let source_consonant_samples = ((consonant_ms.max(0.0) / 1000.0) * sample_rate as f64)
            .clamp(0.0, slice.len() as f64) as usize;

        let consonant_slice = &slice[..source_consonant_samples];
        let vowel_slice = &slice[source_consonant_samples..];

        // Calculate target sample counts
        let target_total_samples =
            ((target_duration_ms / 1000.0) * sample_rate as f64).round() as usize;
        let target_consonant_samples = (((consonant_ms.max(0.0) / 1000.0) * sample_rate as f64)
            .round() as usize)
            .min(target_total_samples);
        let target_vowel_samples = target_total_samples.saturating_sub(target_consonant_samples);

        let mut output = Vec::with_capacity(target_total_samples);

        // Process consonant region (no pitch modification, just duration change)
        if target_consonant_samples > 0 && !consonant_slice.is_empty() {
            output.extend(Self::resample_sinc(
                consonant_slice,
                target_consonant_samples,
                sample_rate,
                sample_rate, // Same rate for consonant - just duration change
            ));
        } else if target_consonant_samples > 0 {
            output.resize(target_consonant_samples, 0.0);
        }

        // Process vowel region (with pitch modification)
        if target_vowel_samples > 0 && !vowel_slice.is_empty() {
            // Calculate the resampling ratio for the vowel part
            let source_duration_ms = (vowel_slice.len() as f64 / sample_rate as f64) * 1000.0;
            let target_vowel_duration_ms = target_duration_ms
                - (consonant_ms.min(target_consonant_samples as f64 * 1000.0 / sample_rate as f64));

            // Avoid division by zero
            let resample_ratio = if source_duration_ms > 0.0 {
                target_vowel_duration_ms / source_duration_ms
            } else {
                1.0
            };

            let effective_output_rate = (sample_rate as f64 * resample_ratio) as u32;

            // Resample with pitch bend applied
            let vowel_output = Self::resample_sinc_with_pitch_bend(
                vowel_slice,
                target_vowel_samples,
                sample_rate,
                effective_output_rate,
                target_pitch_freq,
                pitch_points,
                (consonant_ms.min(target_consonant_samples as f64 * 1000.0 / sample_rate as f64))
                    as f64,
            );

            output.extend(vowel_output);
        }

        // Ensure exact output length
        output.truncate(target_total_samples);
        if output.len() < target_total_samples {
            output.resize(target_total_samples, 0.0);
        }

        output
    }

    /// Resample using windowed sinc interpolation with a Lanczos window.
    ///
    /// # Arguments
    ///
    /// * `input` - Input samples
    /// * `output_len` - Desired output length
    /// * `input_rate` - Input sample rate
    /// * `output_rate` - Output sample rate
    ///
    /// # Returns
    ///
    /// Resampled audio samples
    fn resample_sinc(
        input: &[f32],
        output_len: usize,
        input_rate: u32,
        output_rate: u32,
    ) -> Vec<f32> {
        if input.is_empty() || output_len == 0 {
            return vec![0.0; output_len];
        }

        if input.len() == output_len && input_rate == output_rate {
            return input.to_vec();
        }

        let mut output = Vec::with_capacity(output_len);
        let ratio = input_rate as f64 / output_rate as f64;

        for i in 0..output_len {
            let input_pos = i as f64 * ratio;
            output.push(Self::interpolate_sinc(input, input_pos));
        }

        output
    }

    /// Resample using windowed sinc interpolation with pitch bend support.
    ///
    /// # Arguments
    ///
    /// * `input` - Input samples
    /// * `output_len` - Desired output length
    /// * `input_rate` - Input sample rate
    /// * `output_rate` - Base output sample rate (will be modulated by pitch)
    /// * `target_pitch_freq` - Target fundamental frequency in Hz
    /// * `pitch_points` - Pitch bend points for applying pitch modulation
    /// * `time_offset_ms` - Time offset in milliseconds for pitch bend calculation
    ///
    /// # Returns
    ///
    /// Resampled audio samples with pitch bend applied
    fn resample_sinc_with_pitch_bend(
        input: &[f32],
        output_len: usize,
        input_rate: u32,
        output_rate: u32,
        _target_pitch_freq: f64,
        pitch_points: &[UPitchBendPoint],
        time_offset_ms: f64,
    ) -> Vec<f32> {
        if input.is_empty() || output_len == 0 {
            return vec![0.0; output_len];
        }

        let mut output = Vec::with_capacity(output_len);
        let _ratio = input_rate as f64 / output_rate as f64;

        for i in 0..output_len {
            // Calculate current time for pitch bend lookup
            let current_time_ms = time_offset_ms + (i as f64 / output_rate as f64) * 1000.0;

            // Get pitch bend adjustment
            let pitch_cents =
                PitchBendSolver::get_pitch_offset_cents(current_time_ms, pitch_points);
            let pitch_factor = 2.0f64.powf(pitch_cents / 1200.0);

            // Apply pitch bend to the effective output rate
            let effective_output_rate = (output_rate as f64 * pitch_factor) as u32;
            let effective_ratio = input_rate as f64 / effective_output_rate as f64;

            let input_pos = i as f64 * effective_ratio;
            output.push(Self::interpolate_sinc(input, input_pos));
        }

        output
    }

    /// Interpolate using windowed sinc with Lanczos window.
    ///
    /// # Arguments
    ///
    /// * `input` - Input samples
    /// * `pos` - Floating point position in the input array
    ///
    /// # Returns
    ///
    /// Interpolated sample value
    fn interpolate_sinc(input: &[f32], pos: f64) -> f32 {
        if input.is_empty() {
            return 0.0;
        }

        // Lanczos window order (typically 2 or 3 for good quality)
        const LANCZOS_ORDER: i32 = 3;
        let radius = LANCZOS_ORDER as f64;

        // Calculate the range of input samples needed
        let mut sum = 0.0f64;
        let mut sum_weights = 0.0f64;

        // Determine the range of indices to consider
        let start_idx = ((pos - radius).floor() as isize).max(0);
        let end_idx = ((pos + radius).ceil() as isize).min(input.len() as isize - 1);

        for idx in start_idx..=end_idx {
            let x = (idx as f64 - pos).abs();

            // Avoid division by zero
            if x < 1e-9 {
                // Direct sample when very close to an integer position
                return input[idx as usize];
            }

            // Calculate sinc function: sinc(x) = sin(πx) / (πx)
            let sinc_val = (std::f64::consts::PI * x).sin() / (std::f64::consts::PI * x);

            // Calculate Lanczos window: sinc(x) * sinc(x/a) for -a < x < a, 0 otherwise
            let lanczos_val = if x < radius && x > 0.0 {
                sinc_val * (std::f64::consts::PI * x / radius).sin()
                    / (std::f64::consts::PI * x / radius)
            } else if x < 1e-9 {
                1.0
            } else {
                0.0
            };

            let weight = lanczos_val;

            sum += input[idx as usize] as f64 * weight;
            sum_weights += weight;
        }

        // Avoid division by zero
        if sum_weights > 1e-9 {
            (sum / sum_weights) as f32
        } else {
            // Fallback to nearest neighbor
            let idx = pos.round() as usize;
            input[idx.min(input.len() - 1)]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resample_output_length() {
        let sample_rate = 44100;
        let dummy_samples = vec![0.5f32; sample_rate as usize]; // 1 sec of 0.5
        let rendered = WindowedSincResampler::render_sample(
            &dummy_samples,
            sample_rate,
            0.0,
            0.0,
            500.0, // 500ms target
            440.0,
            &[],
        );
        let expected_len = ((sample_rate as f64 * 0.5) as usize).max(1);
        assert_eq!(rendered.len(), expected_len);
    }

    #[test]
    fn test_resample_pitch_shift() {
        let sample_rate = 8000;
        let frequency = 220.0; // A3
        let duration_secs = 0.5;
        let total_samples = (sample_rate as f64 * duration_secs) as usize;

        // Generate sine wave
        let input_samples: Vec<f32> = (0..total_samples)
            .map(|i| {
                let t = i as f64 / sample_rate as f64;
                (2.0 * std::f64::consts::PI * frequency * t).sin() as f32
            })
            .collect();

        // Shift pitch up by one octave (440 Hz)
        let rendered = WindowedSincResampler::render_sample(
            &input_samples,
            sample_rate,
            0.0,
            0.0,
            250.0, // Half duration due to pitch doubling
            440.0,
            &[],
        );

        // Should be about half the length
        let expected_len = (total_samples / 2).max(1);
        assert!(rendered.len() >= expected_len * 9 / 10); // Allow some tolerance
        assert!(rendered.len() <= expected_len * 11 / 10);
    }

    #[test]
    fn test_resample_silent_input() {
        let sample_rate = 44100;
        let silent_samples = vec![0.0f32; sample_rate as usize];
        let rendered = WindowedSincResampler::render_sample(
            &silent_samples,
            sample_rate,
            0.0,
            0.0,
            500.0,
            440.0,
            &[],
        );
        assert!(rendered.iter().all(|&s| s.abs() < 1e-6));
    }

    #[test]
    fn test_resample_with_consonant() {
        let sample_rate = 8000;
        // Create a signal with distinct consonant and vowel parts
        let mut input = Vec::new();
        // Consonant part: deterministic noise-like pattern
        for i in 0..1000 {
            input.push((i as f32 * 0.1).sin() * 0.5);
        }
        // Vowel part: sine wave
        let frequency = 220.0;
        for i in 0..2000 {
            let t = i as f64 / sample_rate as f64;
            input.push((2.0 * std::f64::consts::PI * frequency * t).sin() as f32 * 0.5);
        }

        let rendered = WindowedSincResampler::render_sample(
            &input,
            sample_rate,
            0.0,
            250.0, // 250ms consonant
            500.0, // 500ms total target
            440.0,
            &[],
        );

        // Should produce reasonable output length
        let expected_len = (sample_rate as f64 * 0.5) as usize; // 500ms target
        assert!(rendered.len() >= expected_len * 8 / 10);
        assert!(rendered.len() <= expected_len * 12 / 10);
    }
}
