use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::drivers::{
    NativeResamplerDriver, NativeWavtoolDriver, ResamplerArgs, ResamplerDriver, WavtoolArgs,
    WavtoolDriver,
};
use crate::dsp::midi_to_freq;
use crate::oto::Voicebank;
use crate::project::model::UNote;
use crate::renderer::RenderOptions;

pub struct TrackRenderer;

impl TrackRenderer {
    fn mix_note_with_crossfade(
        track_buffer: &mut [f32],
        note_samples: &[f32],
        start_sample: usize,
        previous_end_sample: usize,
    ) -> usize {
        if note_samples.is_empty() || start_sample >= track_buffer.len() {
            return previous_end_sample;
        }

        let available = (track_buffer.len() - start_sample).min(note_samples.len());
        let overlap_len = previous_end_sample
            .saturating_sub(start_sample)
            .min(available);

        for (index, &sample) in note_samples.iter().take(available).enumerate() {
            let track_index = start_sample + index;
            if index < overlap_len && overlap_len > 1 {
                let t = index as f32 / (overlap_len - 1) as f32;
                // Complementary smoothstep gains keep correlated vowels from
                // doubling in amplitude while avoiding sharp slope changes.
                let new_gain = t * t * (3.0 - 2.0 * t);
                let old_gain = 1.0 - new_gain;
                track_buffer[track_index] =
                    track_buffer[track_index] * old_gain + sample * new_gain;
            } else {
                track_buffer[track_index] += sample;
            }
        }

        previous_end_sample.max(start_sample + available)
    }

    /// Helper to read a WAV file from disk into f32 mono samples
    pub fn load_wav_samples<P: AsRef<Path>>(path: P) -> Result<(Vec<f32>, u32), String> {
        let path = path.as_ref();
        let mut reader = hound::WavReader::open(path)
            .map_err(|e| format!("Failed to open WAV file {:?}: {}", path, e))?;

        let spec = reader.spec();
        let sample_rate = spec.sample_rate;
        if spec.channels == 0 {
            return Err(format!("WAV {:?} declares zero channels", path));
        }

        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Int => {
                if !(1..=32).contains(&spec.bits_per_sample) {
                    return Err(format!(
                        "Unsupported PCM depth {} in {:?}",
                        spec.bits_per_sample, path
                    ));
                }
                let max_val = 2.0f32.powi(i32::from(spec.bits_per_sample) - 1);
                reader
                    .samples::<i32>()
                    .map(|sample| {
                        sample
                            .map(|value| value as f32 / max_val)
                            .map_err(|error| error.to_string())
                    })
                    .collect::<Result<Vec<_>, _>>()?
            }
            hound::SampleFormat::Float => reader
                .samples::<f32>()
                .map(|sample| sample.map_err(|error| error.to_string()))
                .collect::<Result<Vec<_>, _>>()?,
        };

        if spec.channels > 1 {
            let mono: Vec<f32> = samples
                .chunks_exact(spec.channels as usize)
                .map(|chunk| chunk.iter().sum::<f32>() / spec.channels as f32)
                .collect();
            Ok((mono, sample_rate))
        } else {
            Ok((samples, sample_rate))
        }
    }

    /// Helper to write f32 mono samples into a WAV file on disk
    pub fn save_wav_samples<P: AsRef<Path>>(
        path: P,
        samples: &[f32],
        sample_rate: u32,
    ) -> Result<(), String> {
        Self::save_wav_samples_with_channels(path, samples, sample_rate, 1)
    }

    pub fn save_wav_samples_with_channels<P: AsRef<Path>>(
        path: P,
        samples: &[f32],
        sample_rate: u32,
        channels: u16,
    ) -> Result<(), String> {
        let spec = hound::WavSpec {
            channels: channels.max(1),
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path.as_ref(), spec)
            .map_err(|e| format!("Failed to create temp WAV {:?}: {}", path.as_ref(), e))?;
        for &s in samples {
            let sample_i16 = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
            writer
                .write_sample(sample_i16)
                .map_err(|e| format!("Write sample error: {}", e))?;
        }
        writer
            .finalize()
            .map_err(|e| format!("Finalize WAV error: {}", e))?;
        Ok(())
    }

    /// Render a list of UNotes to a single PCM audio buffer using custom resampler & wavtool drivers
    pub fn render_track_with_drivers(
        notes: &[UNote],
        voicebank: &Voicebank,
        sample_rate: u32,
        tempo_bpm: f64,
        resampler_driver: &dyn ResamplerDriver,
        wavtool_driver: &dyn WavtoolDriver,
        vocal_mode: Option<&RenderOptions>,
    ) -> Vec<f32> {
        Self::render_track_with_progress(
            notes,
            voicebank,
            sample_rate,
            tempo_bpm,
            resampler_driver,
            wavtool_driver,
            vocal_mode,
            None,
        )
    }

    /// Render track with optional real-time progress & log reporting callback: on_progress(progress_0_to_1, log_line)
    pub fn render_track_with_progress(
        notes: &[UNote],
        voicebank: &Voicebank,
        sample_rate: u32,
        tempo_bpm: f64,
        resampler_driver: &dyn ResamplerDriver,
        wavtool_driver: &dyn WavtoolDriver,
        vocal_mode: Option<&RenderOptions>,
        on_progress: Option<&dyn Fn(f32, &str)>,
    ) -> Vec<f32> {
        Self::render_track_with_progress_cancellable(
            notes,
            voicebank,
            sample_rate,
            tempo_bpm,
            resampler_driver,
            wavtool_driver,
            vocal_mode,
            on_progress,
            None,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_track_with_progress_cancellable(
        notes: &[UNote],
        voicebank: &Voicebank,
        sample_rate: u32,
        tempo_bpm: f64,
        resampler_driver: &dyn ResamplerDriver,
        wavtool_driver: &dyn WavtoolDriver,
        vocal_mode: Option<&RenderOptions>,
        on_progress: Option<&dyn Fn(f32, &str)>,
        cancel: Option<&AtomicBool>,
    ) -> Vec<f32> {
        if notes.is_empty() {
            return Vec::new();
        }

        let log = |progress: f32, msg: &str| {
            if on_progress.is_none() {
                eprintln!("{}", msg);
            }
            if let Some(ref cb) = on_progress {
                cb(progress, msg);
            }
        };

        let (loudness_db, gender_offset, breathiness_offset, tone_shift, crossfade_ms) =
            if let Some(vm) = vocal_mode {
                (
                    vm.loudness,
                    vm.gender,
                    vm.breathiness,
                    vm.tone_shift,
                    vm.crossfade_ms,
                )
            } else {
                (0.0, 0.0, 0.0, 0.0, 45.0)
            };

        let max_end_ms = notes
            .iter()
            .map(|n| n.position_ms + n.duration_ms + n.envelope.p5)
            .fold(0.0f64, f64::max);

        let total_samples =
            ((max_end_ms / 1000.0) * sample_rate as f64) as usize + sample_rate as usize;
        let mut track_buffer = vec![0.0f32; total_samples];
        let mut previous_phone_end_sample = 0usize;

        let temp_dir = match tempfile::Builder::new().prefix("kamafeu-render-").tempdir() {
            Ok(directory) => directory,
            Err(error) => {
                log(
                    1.0,
                    &format!("[Render] Failed to create temporary directory: {error}"),
                );
                return Vec::new();
            }
        };

        let start_msg = format!(
            "[Render] Rendering {} notes, max_end={:.0}ms, buffer_len={}",
            notes.len(),
            max_end_ms,
            total_samples
        );
        log(0.0, &start_msg);

        let mode = if let Some(vm) = vocal_mode {
            vm.phonemizer_mode
        } else {
            crate::phonemizer::PhonemizerMode::BasicCV
        };
        let phones =
            crate::phonemizer::JapanesePhonemizer::apply_phonemizer(notes, voicebank, mode);

        let total_phones = phones.len().max(1);

        for (idx, phone) in phones.into_iter().enumerate() {
            if cancel.is_some_and(|token| token.load(Ordering::Relaxed)) {
                log(1.0, "[Render] Cancelled");
                return Vec::new();
            }
            let progress = (idx as f32) / (total_phones as f32);

            let oto_entry = voicebank.find_entry(&phone.lyric, &phone.pitch);

            let (wav_rel_path, offset_ms, consonant_ms, cutoff_ms, preutterance_ms, overlap_ms) =
                if let Some(entry) = oto_entry {
                    (
                        entry.wav_filename.clone(),
                        entry.offset,
                        entry.consonant,
                        entry.cutoff,
                        entry.preutterance,
                        entry.overlap,
                    )
                } else {
                    let default_filename = format!("{}.wav", phone.lyric);
                    (default_filename, 0.0, 50.0, 0.0, 0.0, 0.0)
                };

            let wav_full_path = voicebank.root_path.join(&wav_rel_path);
            let phone_msg = format!(
                "[Render] Phone '{}' ({}/{}) pitch={} pos={:.0}ms dur={:.0}ms wav={:?} oto={}",
                phone.lyric,
                idx + 1,
                total_phones,
                phone.pitch,
                phone.position_ms,
                phone.duration_ms,
                wav_full_path,
                oto_entry.is_some()
            );
            log(progress, &phone_msg);

            let (raw_samples, src_sample_rate) = match Self::load_wav_samples(&wav_full_path) {
                Ok(res) => {
                    log(
                        progress,
                        &format!("  [WAV] Loaded {} samples @ {}Hz", res.0.len(), res.1),
                    );
                    res
                }
                Err(e) => {
                    log(
                        progress,
                        &format!("  [WAV] Load FAILED: {e} — note skipped"),
                    );
                    continue;
                }
            };

            let base_midi =
                phone.midi_key() as f64 + tone_shift + (phone.expressions.pitch_delta / 100.0);
            let target_freq = midi_to_freq(base_midi);

            let consonant_velocity = if phone.expressions.consonant_velocity.is_finite() {
                phone.expressions.consonant_velocity.clamp(0.0, 200.0)
            } else {
                100.0
            };
            let consonant_time_scale =
                crate::phonemizer::consonant_velocity_time_scale(consonant_velocity);
            let active_consonant_ms = consonant_ms.max(0.0) * consonant_time_scale;

            let scaled_preutterance_ms = (preutterance_ms.max(0.0) * consonant_time_scale).max(0.0);
            let timing_overlap_ms =
                (overlap_ms.max(0.0) * consonant_time_scale).min(scaled_preutterance_ms);
            let authored_lead_ms = (scaled_preutterance_ms - timing_overlap_ms).max(0.0);
            let target_render_ms = (phone.duration_ms + authored_lead_ms)
                .max(active_consonant_ms)
                .max(1.0);
            let dur_required = ((target_render_ms / 50.0).ceil() * 50.0).max(50.0);
            log(
                progress,
                &format!(
                    "  [Timing] consonant velocity={:.0}%: {:.1}ms -> {:.1}ms",
                    consonant_velocity, consonant_ms, active_consonant_ms
                ),
            );

            let pitch_bend_encoded = crate::dsp::pitch_encoder::encode_utau_base64_pitch(
                &phone.pitch_bend.points,
                dur_required,
            );

            let total_gender = phone.expressions.gender + gender_offset;
            let total_breathiness = phone.expressions.breathiness + breathiness_offset;

            let mut flags = phone.flags.clone();
            if total_gender != 0.0 {
                flags.push_str(&format!("g{:.0}", total_gender));
            }
            if total_breathiness != 0.0 {
                flags.push_str(&format!("B{:.0}", total_breathiness.abs()));
            }

            let safe_lyric = phone.lyric.replace(['/', '\\', ' ', ':'], "_");
            let res_args = ResamplerArgs {
                input_wav: wav_full_path.clone(),
                output_wav: temp_dir
                    .path()
                    .join(format!("render_{idx}_{safe_lyric}.wav")),
                pitch_name: phone.pitch.clone(),
                pitch_freq: target_freq,
                velocity: consonant_velocity,
                flags,
                offset_ms,
                duration_ms: dur_required,
                source_consonant_ms: consonant_ms.max(0.0),
                consonant_ms: active_consonant_ms,
                cutoff_ms,
                volume: 100.0,
                modulation: phone.expressions.modulation,
                tempo: tempo_bpm,
                pitch_bend_str: pitch_bend_encoded,
                pitch_points: phone.pitch_bend.points.clone(),
            };

            let mut note_rendered = resampler_driver
                .render_sample(&raw_samples, src_sample_rate, &res_args)
                .unwrap_or_else(|e| {
                    log(
                        progress,
                        &format!("  [Resampler] FAILED: {} — using raw samples", e),
                    );
                    raw_samples.clone()
                });

            if src_sample_rate != sample_rate {
                note_rendered =
                    Self::convert_sample_rate(&note_rendered, src_sample_rate, sample_rate);
                log(
                    progress,
                    &format!("  [Sample Rate] Converted {src_sample_rate}Hz -> {sample_rate}Hz"),
                );
            }

            // Resamplers commonly round output to 50 ms blocks. Keep the exact
            // phone length so that it ends on the musical note boundary.
            let target_render_samples =
                ((target_render_ms / 1000.0) * sample_rate as f64).round() as usize;
            note_rendered.resize(target_render_samples.max(1), 0.0);

            let rendered_max = note_rendered.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
            log(
                progress,
                &format!(
                    "  [Resampler] {} samples, max_amp={:.4}",
                    note_rendered.len(),
                    rendered_max
                ),
            );

            let active_overlap = timing_overlap_ms.max(crossfade_ms.max(0.0));
            let envelope_duration_ms = (target_render_ms - phone.envelope.p5.max(0.0)).max(1.0);

            let wav_args = WavtoolArgs {
                output_wav: temp_dir.path().join(format!("wavtool_{idx}.wav")),
                input_rendered_wav: res_args.output_wav.clone(),
                offset_ms,
                duration_ms: envelope_duration_ms,
                envelope: phone.envelope.clone(),
                overlap_ms: active_overlap,
            };

            wavtool_driver.process_note(&mut note_rendered, sample_rate, &wav_args);
            let post_wavtool_max = note_rendered.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
            log(
                progress,
                &format!(
                    "  [Wavtool] {} samples, max_amp={:.4}",
                    note_rendered.len(),
                    post_wavtool_max
                ),
            );

            let total_dyn = phone.expressions.dynamics + loudness_db;
            if total_dyn != 0.0 {
                let dyn_gain = 10.0f32.powf((total_dyn / 20.0) as f32);
                for s in note_rendered.iter_mut() {
                    *s *= dyn_gain;
                }
            }

            // Preserve the oto.ini preutterance alignment. The user crossfade
            // setting may shorten the audible lead, but authored overlap values
            // remain a lower bound for VCV/CVVC transition samples.
            let audible_lead_ms = if crossfade_ms <= 0.0 {
                0.0
            } else {
                authored_lead_ms.min(crossfade_ms.max(timing_overlap_ms))
            };
            let mut source_skip_ms = (authored_lead_ms - audible_lead_ms).max(0.0);
            let unclamped_start_ms = phone.position_ms - audible_lead_ms;
            let actual_start_ms = unclamped_start_ms.max(0.0);
            if unclamped_start_ms < 0.0 {
                source_skip_ms += -unclamped_start_ms;
            }

            let source_skip_samples =
                ((source_skip_ms / 1000.0) * sample_rate as f64).round() as usize;
            let audible_samples = note_rendered
                .get(source_skip_samples.min(note_rendered.len())..)
                .unwrap_or(&[]);
            let start_sample_idx =
                ((actual_start_ms / 1000.0) * sample_rate as f64).round() as usize;
            previous_phone_end_sample = Self::mix_note_with_crossfade(
                &mut track_buffer,
                audible_samples,
                start_sample_idx,
                previous_phone_end_sample,
            );
        }

        let buffer_max = track_buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        log(
            0.95,
            &format!(
                "[Render] Final track_buffer: {} samples, max_amp={:.4}",
                track_buffer.len(),
                buffer_max
            ),
        );

        if let Some(last_nonzero) = track_buffer.iter().rposition(|&s| s.abs() > 1e-4) {
            track_buffer.truncate(last_nonzero + 1);
            log(
                1.0,
                &format!(
                    "[Render] Truncated to {} non-silent samples",
                    track_buffer.len()
                ),
            );
        } else {
            log(1.0, "[Render] WARNING: Entire track buffer is silent!");
        }

        track_buffer
    }

    /// Render track using default native Rust drivers and tempo 120.0
    pub fn render_track(notes: &[UNote], voicebank: &Voicebank, sample_rate: u32) -> Vec<f32> {
        let native_resampler = NativeResamplerDriver;
        let native_wavtool = NativeWavtoolDriver;
        Self::render_track_with_drivers(
            notes,
            voicebank,
            sample_rate,
            120.0,
            &native_resampler,
            &native_wavtool,
            None,
        )
    }

    pub(crate) fn convert_sample_rate(
        samples: &[f32],
        source_rate: u32,
        target_rate: u32,
    ) -> Vec<f32> {
        if samples.is_empty() || source_rate == 0 || target_rate == 0 {
            return samples.to_vec();
        }
        if source_rate == target_rate {
            return samples.to_vec();
        }

        let output_len = ((samples.len() as f64 * f64::from(target_rate) / f64::from(source_rate))
            .round() as usize)
            .max(1);
        let ratio = f64::from(source_rate) / f64::from(target_rate);
        let mut output = Vec::with_capacity(output_len);
        for output_index in 0..output_len {
            let source_position = output_index as f64 * ratio;
            let left = source_position.floor() as usize;
            let right = (left + 1).min(samples.len() - 1);
            let fraction = (source_position - left as f64) as f32;
            output.push(samples[left] * (1.0 - fraction) + samples[right] * fraction);
        }
        output
    }
}

#[cfg(test)]
mod wav_tests {
    use super::TrackRenderer;

    #[test]
    fn reads_32_bit_pcm_without_sign_overflow() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("pcm32.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44_100,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        writer.write_sample(i32::MAX).unwrap();
        writer.write_sample(i32::MIN).unwrap();
        writer.finalize().unwrap();

        let (samples, _) = TrackRenderer::load_wav_samples(path).unwrap();
        assert!(samples[0] > 0.99);
        assert!(samples[1] <= -1.0);
    }

    #[test]
    fn converts_sample_rate_and_preserves_duration() {
        let input = vec![0.0; 44_100];
        let output = TrackRenderer::convert_sample_rate(&input, 44_100, 48_000);
        assert_eq!(output.len(), 48_000);
    }

    #[test]
    fn crossfade_uses_complementary_smooth_gains() {
        let mut track = vec![1.0; 10];
        track.resize(15, 0.0);
        let next = vec![1.0; 10];

        let end = TrackRenderer::mix_note_with_crossfade(&mut track, &next, 5, 10);

        assert_eq!(end, 15);
        assert!((track[5] - 1.0).abs() < 1e-6);
        assert!((track[9] - 1.0).abs() < 1e-6);
        assert!(track[5..10].iter().all(|sample| *sample <= 1.000_001));
        assert!(track[5..10].windows(2).all(|pair| {
            let jump = (pair[1] - pair[0]).abs();
            jump < 1e-5
        }));
    }
}
