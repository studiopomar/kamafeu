pub mod autopitch;
pub mod envelope;
pub mod lpc;
pub mod pitch;
pub mod pitch_bend;
pub mod pitch_encoder;
pub mod pyin;
pub mod resampler;
pub mod sola;

/// Change the duration of a segment without changing its playback rate.
///
/// OTO consonant regions often contain part of the voiced transition. A plain
/// linear resize plays that transition faster or slower and therefore changes
/// its pitch. This compact WSOLA implementation instead skips or repeats
/// overlapping waveform grains while keeping every grain at its original
/// sampling rate.
pub(crate) fn resize_preserving_pitch(
    samples: &[f32],
    target_len: usize,
    sample_rate: u32,
) -> Vec<f32> {
    if target_len == 0 || samples.is_empty() {
        return Vec::new();
    }
    if samples.len() == target_len {
        return samples.to_vec();
    }

    let window_len = ((sample_rate as usize * 20) / 1000)
        .clamp(32, 1024)
        .min(samples.len())
        .min(target_len);
    if window_len < 32 {
        // There is not enough signal for a stable overlap correlation. Copy a
        // representative span instead of changing its playback rate.
        return (0..target_len)
            .map(|index| samples[index.min(samples.len() - 1)])
            .collect();
    }

    let synthesis_hop = (window_len / 2).max(1);
    let frame_count = if target_len <= window_len {
        1
    } else {
        (target_len - window_len).div_ceil(synthesis_hop) + 1
    };
    let analysis_span = samples.len().saturating_sub(window_len);
    let analysis_hop = if frame_count > 1 {
        analysis_span as f64 / (frame_count - 1) as f64
    } else {
        0.0
    };
    let search_radius = (window_len / 4).min(256);
    let mut output = vec![0.0f32; target_len];
    let mut weights = vec![0.0f32; target_len];

    for frame in 0..frame_count {
        let output_start = (frame * synthesis_hop).min(target_len.saturating_sub(window_len));
        let nominal = (frame as f64 * analysis_hop).round() as usize;
        let mut input_start = nominal.min(analysis_span);

        if frame > 0 {
            let search_start = input_start.saturating_sub(search_radius);
            let search_end = (input_start + search_radius).min(analysis_span);
            let overlap_len = window_len.min(output_start + window_len).min(synthesis_hop);
            let mut best_score = f64::NEG_INFINITY;
            for candidate in search_start..=search_end {
                let mut dot = 0.0f64;
                let mut old_energy = 0.0f64;
                let mut new_energy = 0.0f64;
                for index in 0..overlap_len {
                    let out_index = output_start + index;
                    if weights[out_index] <= 1e-6 {
                        continue;
                    }
                    let old = output[out_index] as f64 / weights[out_index] as f64;
                    let new = samples[candidate + index] as f64;
                    dot += old * new;
                    old_energy += old * old;
                    new_energy += new * new;
                }
                let score = if old_energy > 1e-9 && new_energy > 1e-9 {
                    dot / (old_energy * new_energy).sqrt()
                } else {
                    f64::NEG_INFINITY
                };
                if score > best_score {
                    best_score = score;
                    input_start = candidate;
                }
            }
        }

        for index in 0..window_len {
            let out_index = output_start + index;
            if out_index >= target_len {
                break;
            }
            let phase = std::f32::consts::TAU * index as f32 / window_len as f32;
            let weight = 0.5 - 0.5 * phase.cos();
            output[out_index] += samples[input_start + index] * weight;
            weights[out_index] += weight;
        }
    }

    for (sample, weight) in output.iter_mut().zip(weights) {
        if weight > 1e-6 {
            *sample /= weight;
        }
    }
    output[0] = samples[0];
    output[target_len - 1] = samples[samples.len() - 1];
    output
}

/// Resolve UTAU oto.ini offset/cutoff semantics to a half-open sample range.
/// Positive cutoff trims from the end; negative cutoff is the absolute length
/// measured from the offset; zero keeps the rest of the WAV.
pub(crate) fn oto_source_bounds(
    sample_count: usize,
    sample_rate: u32,
    offset_ms: f64,
    cutoff_ms: f64,
) -> (usize, usize) {
    let samples_per_ms = sample_rate as f64 / 1000.0;
    let start = (offset_ms.max(0.0) * samples_per_ms).clamp(0.0, sample_count as f64) as usize;
    let end = if cutoff_ms > 0.0 {
        sample_count.saturating_sub((cutoff_ms * samples_per_ms) as usize)
    } else if cutoff_ms < 0.0 {
        start.saturating_add((-cutoff_ms * samples_per_ms) as usize)
    } else {
        sample_count
    }
    .clamp(start, sample_count);
    (start, end)
}

#[cfg(test)]
mod oto_bounds_tests {
    use super::{oto_source_bounds, resize_preserving_pitch};

    #[test]
    fn follows_utau_cutoff_semantics_for_long_wavs() {
        assert_eq!(
            oto_source_bounds(10_000, 1_000, 1_000.0, 2_000.0),
            (1_000, 8_000)
        );
        assert_eq!(
            oto_source_bounds(10_000, 1_000, 1_000.0, -2_000.0),
            (1_000, 3_000)
        );
        assert_eq!(
            oto_source_bounds(10_000, 1_000, 1_000.0, 0.0),
            (1_000, 10_000)
        );
    }

    #[test]
    fn shortening_consonant_region_does_not_raise_pitch() {
        let sample_rate = 44_100;
        let source_frequency = 230.0;
        let source_len = (sample_rate as f64 * 0.204) as usize;
        let source = (0..source_len)
            .map(|index| {
                (std::f64::consts::TAU * source_frequency * index as f64 / sample_rate as f64).sin()
                    as f32
            })
            .collect::<Vec<_>>();

        // Mirrors KYE Bb3/18.wav: a 204 ms OTO consonant region has to fit in
        // roughly 96 ms. Linear resampling used to raise it above 480 Hz.
        let resized =
            resize_preserving_pitch(&source, (sample_rate as f64 * 0.096) as usize, sample_rate);
        let middle = &resized[resized.len() / 8..resized.len() * 7 / 8];
        let positive_crossings = middle
            .windows(2)
            .filter(|pair| pair[0] <= 0.0 && pair[1] > 0.0)
            .count();
        let measured_frequency =
            positive_crossings as f64 * sample_rate as f64 / middle.len() as f64;
        assert!(
            (measured_frequency - source_frequency).abs() < 15.0,
            "pitch changed from {source_frequency:.1} Hz to {measured_frequency:.1} Hz"
        );
    }
}

pub use autopitch::{AutoPitchEngine, AutoPitchOptions, AutoPitchPreset, AutoPitchScope};
pub use envelope::UtauEnvelope;
pub use lpc::LpcExtractor;
pub use pitch::{midi_to_freq, midi_to_note_name, note_name_to_midi, PitchBendPoint, VibratoParam};
pub use pitch_bend::PitchBendSolver;
pub use pitch_encoder::encode_pitch_bend_string;
pub use pyin::{PitchExtractor, PitchResult};
pub use resampler::Resampler;
pub use sola::{SolaResampler, SolaStretchMode};
