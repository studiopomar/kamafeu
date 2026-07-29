use std::path::Path;

use crate::drivers::{NativeResamplerDriver, NativeWavtoolDriver, ResamplerArgs, ResamplerDriver, WavtoolArgs, WavtoolDriver};
use crate::dsp::midi_to_freq;
use crate::oto::Voicebank;
use crate::phonemizer::JapanesePhonemizer;
use crate::project::model::UNote;

pub struct TrackRenderer;

impl TrackRenderer {
    /// Helper to read a WAV file from disk into f32 mono samples
    pub fn load_wav_samples<P: AsRef<Path>>(path: P) -> Result<(Vec<f32>, u32), String> {
        let path = path.as_ref();
        let mut reader = hound::WavReader::open(path)
            .map_err(|e| format!("Failed to open WAV file {:?}: {}", path, e))?;

        let spec = reader.spec();
        let sample_rate = spec.sample_rate;

        let samples: Vec<f32> = match spec.sample_format {
            hound::SampleFormat::Int => {
                let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
                reader
                    .samples::<i32>()
                    .filter_map(Result::ok)
                    .map(|s| s as f32 / max_val)
                    .collect()
            }
            hound::SampleFormat::Float => reader.samples::<f32>().filter_map(Result::ok).collect(),
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
    pub fn save_wav_samples<P: AsRef<Path>>(path: P, samples: &[f32], sample_rate: u32) -> Result<(), String> {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(path.as_ref(), spec)
            .map_err(|e| format!("Failed to create temp WAV {:?}: {}", path.as_ref(), e))?;
        for &s in samples {
            let sample_i16 = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
            writer.write_sample(sample_i16).map_err(|e| format!("Write sample error: {}", e))?;
        }
        writer.finalize().map_err(|e| format!("Finalize WAV error: {}", e))?;
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
        vocal_mode: Option<&crate::gui::left_panel::VocalModeParams>,
    ) -> Vec<f32> {
        if notes.is_empty() {
            return Vec::new();
        }

        let (loudness_db, gender_offset, breathiness_offset, tone_shift, crossfade_ms) = if let Some(vm) = vocal_mode {
            (vm.loudness, vm.gender, vm.breathiness, vm.tone_shift, vm.crossfade_ms)
        } else {
            (0.0, 0.0, 0.0, 0.0, 45.0)
        };

        let max_end_ms = notes
            .iter()
            .map(|n| n.position_ms + n.duration_ms + n.envelope.p5)
            .fold(0.0f64, f64::max);

        let total_samples = ((max_end_ms / 1000.0) * sample_rate as f64) as usize + 44100;
        let mut track_buffer = vec![0.0f32; total_samples];
        eprintln!("[Render] Rendering {} notes, max_end={:.0}ms, buffer_len={}", notes.len(), max_end_ms, total_samples);

        let mut prev_vowel: Option<String> = None;

        for note in notes {
            let target_alias = JapanesePhonemizer::resolve_alias(
                voicebank,
                prev_vowel.as_deref(),
                &note.lyric,
                &note.pitch,
            );

            prev_vowel = JapanesePhonemizer::extract_vowel(&note.lyric).map(|s| s.to_string());

            let oto_entry = voicebank
                .find_entry(&target_alias, &note.pitch)
                .or_else(|| voicebank.find_entry(&note.lyric, &note.pitch));

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
                    let default_filename = format!("{}.wav", note.lyric);
                    (default_filename, 0.0, 50.0, 0.0, 0.0, 0.0)
                };

            let wav_full_path = voicebank.root_path.join(&wav_rel_path);
            eprintln!("[Render] Note '{}' pitch={} pos={:.0}ms dur={:.0}ms wav={:?} oto_found={}",
                note.lyric, note.pitch, note.position_ms, note.duration_ms, wav_full_path, oto_entry.is_some());

            let (raw_samples, src_sample_rate) = match Self::load_wav_samples(&wav_full_path) {
                Ok(res) => {
                    eprintln!("[Render]   WAV loaded: {} samples at {}Hz", res.0.len(), res.1);
                    res
                },
                Err(e) => {
                    eprintln!("[Render]   WAV load FAILED: {} — generating sine fallback", e);
                    let duration_sec = note.duration_ms / 1000.0;
                    let num_s = (sample_rate as f64 * duration_sec.max(0.1)) as usize;
                    let base_midi = note.midi_key() as f64 + tone_shift + (note.expressions.pitch_delta / 100.0);
                    let freq = midi_to_freq(base_midi);
                    let syn: Vec<f32> = (0..num_s)
                        .map(|i| (i as f64 * 2.0 * std::f64::consts::PI * freq / sample_rate as f64).sin() as f32 * 0.5)
                        .collect();
                    (syn, sample_rate)
                }
            };

            let base_midi = note.midi_key() as f64 + tone_shift + (note.expressions.pitch_delta / 100.0);
            let target_freq = midi_to_freq(base_midi);

            let pitch_bend_encoded = crate::dsp::pitch_encoder::encode_utau_base64_pitch(&note.pitch_bend.points, note.duration_ms);

            let total_gender = note.expressions.gender + gender_offset;
            let total_breathiness = note.expressions.breathiness + breathiness_offset;

            let mut flags = String::new();
            if total_gender != 0.0 {
                flags.push_str(&format!("g{:.0}", total_gender));
            }
            if total_breathiness != 0.0 {
                flags.push_str(&format!("B{:.0}", total_breathiness.abs()));
            }

            let velocity = if note.expressions.velocity != 0.0 { note.expressions.velocity } else { 100.0 };
            let active_overlap = if overlap_ms > 0.0 { overlap_ms } else { crossfade_ms };

            let stretch_ratio = 2.0f64.powf(1.0 - velocity * 0.01);
            let skip_over = (preutterance_ms * stretch_ratio - active_overlap).max(0.0);
            let dur_required = (note.duration_ms + skip_over).max(consonant_ms);
            let dur_required = ((dur_required / 50.0 + 0.5).ceil() * 50.0).max(50.0);

            let res_args = ResamplerArgs {
                input_wav: wav_full_path.clone(),
                output_wav: std::env::temp_dir().join(format!("kamafeu_render_{}.wav", note.lyric)),
                pitch_name: note.pitch.clone(),
                pitch_freq: target_freq,
                velocity,
                flags,
                offset_ms,
                duration_ms: dur_required,
                consonant_ms,
                cutoff_ms,
                volume: 100.0,
                modulation: 100.0,
                tempo: tempo_bpm,
                pitch_bend_str: pitch_bend_encoded,
                pitch_points: note.pitch_bend.points.clone(),
            };

            let mut note_rendered = resampler_driver
                .render_sample(&raw_samples, src_sample_rate, &res_args)
                .unwrap_or_else(|e| {
                    eprintln!("[Render]   Resampler FAILED: {} — using raw samples", e);
                    raw_samples.clone()
                });

            let rendered_max = note_rendered.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
            eprintln!("[Render]   After resampler: {} samples, max_amp={:.4}", note_rendered.len(), rendered_max);

            let active_overlap = if overlap_ms > 0.0 { overlap_ms } else { crossfade_ms };

            let wav_args = WavtoolArgs {
                output_wav: std::env::temp_dir().join("kamafeu_out.wav"),
                input_rendered_wav: res_args.output_wav.clone(),
                offset_ms,
                duration_ms: note.duration_ms,
                envelope: note.envelope.clone(),
                overlap_ms: active_overlap,
            };

            wavtool_driver.process_note(&mut note_rendered, sample_rate, &wav_args);
            let post_wavtool_max = note_rendered.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
            eprintln!("[Render]   After wavtool: {} samples, max_amp={:.4}", note_rendered.len(), post_wavtool_max);

            let total_dyn = note.expressions.dynamics + loudness_db;
            if total_dyn != 0.0 {
                let dyn_gain = 10.0f32.powf((total_dyn / 20.0) as f32);
                for s in note_rendered.iter_mut() {
                    *s *= dyn_gain;
                }
            }

            let actual_start_ms = (note.position_ms - preutterance_ms).max(0.0);
            let start_sample_idx = ((actual_start_ms / 1000.0) * sample_rate as f64) as usize;
            eprintln!("[Render]   Placing at start_sample_idx={} (actual_start={:.0}ms)", start_sample_idx, actual_start_ms);

            for (i, sample) in note_rendered.iter().enumerate() {
                let track_idx = start_sample_idx + i;
                if track_idx < track_buffer.len() {
                    track_buffer[track_idx] += sample;
                }
            }
        }

        let buffer_max = track_buffer.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
        eprintln!("[Render] Final track_buffer: {} samples, max_amp={:.4}", track_buffer.len(), buffer_max);

        if let Some(last_nonzero) = track_buffer.iter().rposition(|&s| s.abs() > 1e-4) {
            track_buffer.truncate(last_nonzero + 1);
            eprintln!("[Render] Truncated to {} non-silent samples", track_buffer.len());
        } else {
            eprintln!("[Render] WARNING: Entire track buffer is silent!");
        }

        track_buffer
    }

    /// Render track using default native Rust drivers and tempo 120.0
    pub fn render_track(
        notes: &[UNote],
        voicebank: &Voicebank,
        sample_rate: u32,
    ) -> Vec<f32> {
        let native_resampler = NativeResamplerDriver;
        let native_wavtool = NativeWavtoolDriver;
        Self::render_track_with_drivers(notes, voicebank, sample_rate, 120.0, &native_resampler, &native_wavtool, None)
    }
}
