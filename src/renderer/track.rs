use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use rayon::prelude::*;

use crate::drivers::{
    NativeResamplerDriver, NativeWavtoolDriver, ResamplerArgs, ResamplerDriver, WavtoolArgs,
    WavtoolDriver,
};
use crate::dsp::midi_to_freq;
use crate::oto::Voicebank;
use crate::project::model::UNote;
use crate::renderer::timing::{resolve_phoneme_timings, PhonemeTimingInput};
use crate::renderer::RenderOptions;

pub struct TrackRenderer;

struct PhrasePitchNote {
    position_ms: f64,
    duration_ms: f64,
    midi: u8,
    curve_start_ms: f64,
    points: Vec<crate::project::model::UPitchBendPoint>,
    vibrato: crate::dsp::pitch::VibratoParam,
    pitch_delta: f64,
}

impl TrackRenderer {
    fn phrase_pitch_notes(notes: &[UNote]) -> Vec<PhrasePitchNote> {
        let mut result = notes
            .iter()
            .enumerate()
            .map(|(index, note)| {
                let previous = index.checked_sub(1).and_then(|index| notes.get(index));
                let adjacent = previous.is_some_and(|previous| {
                    (note.position_ms - (previous.position_ms + previous.duration_ms)).abs() <= 60.0
                });
                let is_plus = note.lyric.trim() == "+" || note.lyric.trim().starts_with("+ ");
                let points = if is_plus && adjacent {
                    // Nota '+' com legato contínuo herda / faz transição contínua
                    note.pitch_bend.effective_points(
                        previous.map(UNote::midi_key),
                        note.midi_key(),
                        true,
                    )
                } else {
                    note.pitch_bend.effective_points(
                        previous.map(UNote::midi_key),
                        note.midi_key(),
                        adjacent,
                    )
                };
                PhrasePitchNote {
                    position_ms: note.position_ms,
                    duration_ms: note.duration_ms,
                    midi: note.midi_key(),
                    // A note owns its base pitch from its musical start. Only
                    // a negative first point may begin that ownership earlier
                    // to form a portamento from the previous note.
                    curve_start_ms: note.position_ms
                        + points
                            .first()
                            .map(|point| point.time_offset_ms.min(0.0))
                            .unwrap_or(0.0),
                    points,
                    vibrato: note.vibrato.clone(),
                    pitch_delta: note.expressions.pitch_delta,
                }
            })
            .collect::<Vec<_>>();
        result.sort_by(|left, right| {
            left.curve_start_ms
                .partial_cmp(&right.curve_start_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        result
    }

    fn phrase_pitch_cents_at(
        pitch_notes: &[PhrasePitchNote],
        absolute_time_ms: f64,
        tone_shift: f64,
    ) -> f64 {
        use crate::dsp::pitch_bend::PitchBendSolver;
        if pitch_notes.is_empty() {
            return (60.0 + tone_shift) * 100.0;
        }
        let upper = pitch_notes.partition_point(|note| note.curve_start_ms <= absolute_time_ms);
        let index = upper.saturating_sub(1).min(pitch_notes.len() - 1);
        let note = &pitch_notes[index];
        let relative_time_ms = absolute_time_ms - note.position_ms;
        let bend = PitchBendSolver::get_pitch_offset_cents_sorted(relative_time_ms, &note.points);
        let vibrato = note
            .vibrato
            .pitch_offset_cents_at(relative_time_ms, note.duration_ms);
        (f64::from(note.midi) + tone_shift) * 100.0 + note.pitch_delta + bend + vibrato
    }

    fn combined_pitch_points(
        phone: &crate::phonemizer::RenderPhone,
        pitch_notes: &[PhrasePitchNote],
        segment_start_ms: f64,
        duration_ms: f64,
        tone_shift: f64,
    ) -> Vec<crate::project::model::UPitchBendPoint> {
        use crate::dsp::pitch_bend::PitchBendSolver;
        use crate::project::model::UPitchBendPoint;

        let step_ms = 5.0;
        let count = (duration_ms.max(1.0) / step_ms).ceil() as usize + 1;
        let mut points = Vec::with_capacity(count);
        for index in 0..count {
            let time_ms = (index as f64 * step_ms).min(duration_ms);
            let absolute_pitch =
                Self::phrase_pitch_cents_at(pitch_notes, segment_start_ms + time_ms, tone_shift);
            let cents = absolute_pitch - f64::from(phone.midi_key()) * 100.0;
            points.push(UPitchBendPoint {
                time_offset_ms: time_ms,
                pitch_offset_cents: cents,
                shape: "l".to_string(),
            });
        }
        PitchBendSolver::simplify_pitch_points(&points, 0.25)
    }

    fn mix_phase_aligned(
        track_buffer: &mut [f32],
        note_samples: &[f32],
        mut start_sample: usize,
        previous_end_sample: usize,
        crossfade_samples: usize,
        pitch_freq: f64,
        sample_rate: u32,
    ) -> usize {
        if note_samples.is_empty() || start_sample >= track_buffer.len() {
            return previous_end_sample;
        }

        // Align the phase at boundary using normalized cross-correlation search.
        // This ensures overlapping pitch periods match in crests and troughs,
        // preventing destructive phase interference that causes harsh, hollow or comb-filtered sounds.
        let nominal_overlap = previous_end_sample.saturating_sub(start_sample);
        if nominal_overlap >= 64 && crossfade_samples >= 32 {
            let period = (f64::from(sample_rate) / pitch_freq.max(20.0))
                .round()
                .clamp(16.0, 1024.0) as isize;
            let search = period;
            let nominal = start_sample as isize;
            let mut best_start = start_sample;
            let mut best_score = -2.0f64;
            let window_len = ((f64::from(sample_rate) * 0.02).round() as usize)
                .clamp(64, 2048)
                .min(note_samples.len());
            let window_center = (crossfade_samples / 2).min(note_samples.len());
            let new_window_start = window_center
                .saturating_sub(window_len / 2)
                .min(note_samples.len().saturating_sub(window_len));
            for lag in -search..=search {
                let candidate_signed = nominal + lag;
                if candidate_signed < 0 {
                    continue;
                }
                let candidate = candidate_signed as usize;
                let old_window_start = candidate.saturating_add(new_window_start);
                let overlap = window_len
                    .min(previous_end_sample.saturating_sub(old_window_start))
                    .min(track_buffer.len().saturating_sub(old_window_start));
                if overlap < 32 {
                    continue;
                }
                let mut dot = 0.0f64;
                let mut old_energy = 0.0f64;
                let mut new_energy = 0.0f64;
                for i in 0..overlap {
                    let old = track_buffer[old_window_start + i] as f64;
                    let new = note_samples[new_window_start + i] as f64;
                    dot += old * new;
                    old_energy += old * old;
                    new_energy += new * new;
                }
                if old_energy > 1e-9 && new_energy > 1e-9 {
                    let score =
                        dot / (old_energy * new_energy).sqrt() - (lag.unsigned_abs() as f64 * 1e-7);
                    if score > best_score {
                        best_score = score;
                        best_start = candidate;
                    }
                }
            }
            if best_score > 0.1 {
                start_sample = best_start;
            }
        }

        let available = (track_buffer.len() - start_sample).min(note_samples.len());
        for (index, &sample) in note_samples.iter().take(available).enumerate() {
            let track_index = start_sample + index;
            track_buffer[track_index] += sample;
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
        on_progress: Option<&(dyn Fn(f32, &str) + Send + Sync)>,
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
        on_progress: Option<&(dyn Fn(f32, &str) + Send + Sync)>,
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

        let (loudness_db, gender_offset, breathiness_offset, tone_shift, _crossfade_ms) =
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
        let pitch_notes = Self::phrase_pitch_notes(notes);

        let timing_inputs = phones
            .iter()
            .map(|phone| {
                let oto = voicebank.find_entry(&phone.lyric, &phone.pitch);
                let raw_preutter = oto.map(|entry| entry.preutterance).unwrap_or(0.0);
                let raw_overlap = oto.map(|entry| entry.overlap).unwrap_or(0.0);
                let oto_preutter_ms = raw_preutter.max(0.0);
                let oto_overlap_ms = if phone.envelope.crossfade_ms > 0.0 {
                    phone.envelope.crossfade_ms
                } else {
                    raw_overlap
                };
                PhonemeTimingInput {
                    position_ms: phone.position_ms,
                    duration_ms: phone.duration_ms,
                    oto_preutter_ms,
                    oto_overlap_ms,
                    velocity: phone.expressions.consonant_velocity,
                    preutter_delta_ms: phone.expressions.preutter_offset_ms,
                    overlap_delta_ms: phone.expressions.overlap_offset_ms,
                }
            })
            .collect::<Vec<_>>();
        let timings = resolve_phoneme_timings(&timing_inputs);

        let total_phones = phones.len().max(1);

        // ------------------------------------------------------------------
        // Phase 0: Pre-load all unique WAV files into a shared in-memory
        // cache.  This eliminates repeated disk reads for voicebanks where
        // many notes share the same sample file (e.g. all 'あ' notes use the
        // same あ.wav).  The load itself is also parallelised with rayon.
        // ------------------------------------------------------------------
        let unique_wav_paths: std::collections::HashSet<std::path::PathBuf> = phones
            .iter()
            .map(|phone| {
                let rel = voicebank
                    .find_entry(&phone.lyric, &phone.pitch)
                    .map(|e| e.wav_filename.clone())
                    .unwrap_or_else(|| format!("{}.wav", phone.lyric));
                voicebank.root_path.join(rel)
            })
            .collect();

        // Collect paths into a Vec so rayon can index them.
        let wav_paths_vec: Vec<std::path::PathBuf> = unique_wav_paths.into_iter().collect();
        let wav_cache: HashMap<std::path::PathBuf, Arc<(Vec<f32>, u32)>> = wav_paths_vec
            .into_par_iter()
            .filter_map(|path| {
                Self::load_wav_samples(&path)
                    .ok()
                    .map(|(samples, rate)| (path, Arc::new((samples, rate))))
            })
            .collect();

        // ------------------------------------------------------------------
        // Phase 1: Render each phone in parallel.
        //
        // Each phone is self-contained: it reads from the shared `wav_cache`,
        // computes pitch curves and calls the resampler + wavtool.  The
        // results are collected in an unsorted Vec and then sorted by `idx`
        // before the sequential merge below.
        // ------------------------------------------------------------------

        // Holds all the data needed for the sequential merge phase.
        struct PhoneResult {
            idx: usize,
            note_rendered: Vec<f32>,
            actual_start_ms: f64,
            source_skip_ms: f64,
            crossfade_ms: f64,
            pitch_freq: f64,
            logs: Vec<(f32, String)>,
        }

        let temp_dir_path = temp_dir.path().to_path_buf();
        let pitch_notes_ref = &pitch_notes;
        let completed_phones = std::sync::atomic::AtomicUsize::new(0);
        let completed_ref = &completed_phones;

        let mut phone_results: Vec<PhoneResult> = phones
            .into_par_iter()
            .enumerate()
            .filter_map(|(idx, phone)| {
                if cancel.is_some_and(|t| t.load(Ordering::Relaxed)) {
                    return None;
                }

                let mut logs: Vec<(f32, String)> = Vec::new();
                let progress = idx as f32 / total_phones as f32;
                let timing = timings[idx];

                let oto_entry = voicebank.find_entry(&phone.lyric, &phone.pitch);
                let (
                    wav_rel_path,
                    offset_ms,
                    consonant_ms,
                    cutoff_ms,
                    loop_start_ms,
                    loop_end_ms,
                    tail_start_ms,
                ) = if let Some(entry) = oto_entry {
                    (
                        entry.wav_filename.clone(),
                        entry.offset,
                        entry.consonant,
                        entry.cutoff,
                        entry.loop_start,
                        entry.loop_end,
                        entry.tail_start,
                    )
                } else {
                    (
                        format!("{}.wav", phone.lyric),
                        0.0,
                        50.0,
                        0.0,
                        None,
                        None,
                        None,
                    )
                };

                let wav_full_path = voicebank.root_path.join(&wav_rel_path);
                logs.push((
                    progress,
                    format!(
                    "[Render] Phone '{}' ({}/{}) pitch={} pos={:.0}ms dur={:.0}ms wav={:?} oto={}",
                    phone.lyric, idx + 1, total_phones, phone.pitch,
                    phone.position_ms, phone.duration_ms, wav_full_path, oto_entry.is_some()
                ),
                ));

                // Retrieve samples from the in-memory cache (zero disk I/O).
                let cached = wav_cache.get(&wav_full_path);
                let (raw_samples, src_sample_rate) = if let Some(arc) = cached {
                    logs.push((
                        progress,
                        format!("  [WAV] Cache hit: {} samples @ {}Hz", arc.0.len(), arc.1),
                    ));
                    (arc.0.as_slice(), arc.1)
                } else {
                    logs.push((
                        progress,
                        format!("  [WAV] Load FAILED: {:?} — note skipped", wav_full_path),
                    ));
                    return None;
                };

                let base_midi = phone.midi_key() as f64;
                let target_freq = midi_to_freq(base_midi);

                let consonant_velocity = if phone.expressions.consonant_velocity.is_finite() {
                    phone.expressions.consonant_velocity.clamp(0.0, 200.0)
                } else {
                    100.0
                };
                let consonant_time_scale =
                    crate::phonemizer::consonant_velocity_time_scale(consonant_velocity);
                let raw_scaled_consonant = consonant_ms.max(0.0) * consonant_time_scale;
                let active_consonant_ms = (raw_scaled_consonant
                    + phone.expressions.consonant_timing_offset_ms)
                    .clamp(0.0, phone.duration_ms.max(10.0) * 0.95);

                let duration_correction_ms =
                    timing.preutter_ms - timing.tail_intrude_ms + timing.tail_overlap_ms;
                let target_render_ms =
                    (phone.duration_ms + duration_correction_ms + timing.skip_over_ms)
                        .max(active_consonant_ms)
                        .max(1.0);
                let dur_required = ((target_render_ms / 50.0 + 0.5).ceil() * 50.0).max(50.0);
                logs.push((
                    progress,
                    format!(
                        "  [Timing] consonant velocity={:.0}%, offset={:.1}ms: {:.1}ms -> {:.1}ms",
                        consonant_velocity,
                        phone.expressions.consonant_timing_offset_ms,
                        consonant_ms,
                        active_consonant_ms
                    ),
                ));

                let combined_pitch = Self::combined_pitch_points(
                    &phone,
                    pitch_notes_ref,
                    phone.position_ms - timing.pitch_leading_ms,
                    target_render_ms,
                    tone_shift,
                );
                let pitch_bend_encoded = crate::dsp::pitch_encoder::encode_utau_base64_pitch(
                    &combined_pitch,
                    target_render_ms,
                    tempo_bpm,
                );

                let total_gender = phone.expressions.gender + gender_offset;
                let total_breathiness = phone.expressions.breathiness + breathiness_offset;
                let flags =
                    resampler_driver.prepare_flags(&phone.flags, total_gender, total_breathiness);

                let safe_lyric = phone.lyric.replace(['/', '\\', ' ', ':'], "_");
                let res_args = ResamplerArgs {
                    input_wav: wav_full_path.clone(),
                    output_wav: temp_dir_path.join(format!("render_{idx}_{safe_lyric}.wav")),
                    pitch_name: phone.pitch.clone(),
                    pitch_freq: target_freq,
                    velocity: consonant_velocity,
                    flags,
                    offset_ms,
                    duration_ms: dur_required,
                    source_consonant_ms: consonant_ms.max(0.0),
                    consonant_ms: active_consonant_ms,
                    cutoff_ms,
                    volume: phone.expressions.volume,
                    modulation: phone.expressions.modulation,
                    tempo: tempo_bpm,
                    pitch_bend_str: pitch_bend_encoded,
                    pitch_points: combined_pitch,
                    loop_start_ms,
                    loop_end_ms,
                    tail_start_ms,
                };

                logs.push((
                    progress,
                    format!("  [Resampler] Motor: '{}'", resampler_driver.name()),
                ));

                let mut note_rendered = crate::renderer::resampler_cache::render_with_cache(
                    resampler_driver,
                    raw_samples,
                    src_sample_rate,
                    &res_args,
                    cancel,
                )
                .map(|(samples, cache_hit)| {
                    logs.push((
                        progress,
                        if cache_hit {
                            "  [Resampler Cache] hit".to_string()
                        } else {
                            "  [Resampler Cache] miss".to_string()
                        },
                    ));
                    samples
                })
                .unwrap_or_else(|e| {
                    logs.push((
                        progress,
                        format!("  [Resampler] FAILED: {} — using sliced raw samples", e),
                    ));
                    let (start, end) = crate::dsp::oto_source_bounds(
                        raw_samples.len(),
                        src_sample_rate,
                        offset_ms,
                        cutoff_ms,
                    );
                    raw_samples.get(start..end).unwrap_or(raw_samples).to_vec()
                });

                if src_sample_rate != sample_rate {
                    note_rendered =
                        Self::convert_sample_rate(&note_rendered, src_sample_rate, sample_rate);
                    logs.push((
                        progress,
                        format!("  [Sample Rate] Converted {src_sample_rate}Hz -> {sample_rate}Hz"),
                    ));
                }

                // `dur_required` is intentionally longer than the musical
                // duration: classic UTAU resamplers need that head/tail so
                // the wavtool can discard `skip_over` and apply its envelope.
                // Truncating here cuts synthesized grains and creates a
                // crackling/rough tail, especially after a pitch change.
                // OpenUtau preserves the resampler WAV intact until mixing.

                let rendered_max = note_rendered.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
                logs.push((
                    progress,
                    format!(
                        "  [Resampler] {} samples, max_amp={:.4}",
                        note_rendered.len(),
                        rendered_max
                    ),
                ));

                let active_overlap = if phone.envelope.crossfade_ms > 0.0 {
                    phone.envelope.crossfade_ms
                } else {
                    timing.overlap_ms
                };
                let envelope_duration_ms = phone.duration_ms.max(1.0);
                let phoneme_envelope = phone.envelope.phoneme_points(
                    timing.preutter_ms,
                    phone.duration_ms,
                    timing.tail_intrude_ms,
                    timing.tail_overlap_ms,
                    active_overlap,
                    phone.expressions.volume,
                    phone.expressions.attack,
                    phone.expressions.decay,
                );

                let p2_diff = (phoneme_envelope[1].0 - phoneme_envelope[0].0).max(0.0);
                let p3_diff = (phoneme_envelope[4].0 - phoneme_envelope[3].0).max(0.0);
                let p5_diff = (phoneme_envelope[2].0 - phoneme_envelope[1].0).max(0.0);
                let mut wavtool_env = phone.envelope.clone();
                wavtool_env.p1 = 0.0;
                wavtool_env.p2 = p2_diff;
                wavtool_env.p3 = p3_diff;
                wavtool_env.v1 = (phoneme_envelope[0].1 * 100.0).clamp(0.0, 200.0);
                wavtool_env.v2 = (phoneme_envelope[1].1 * 100.0).clamp(0.0, 200.0);
                wavtool_env.v3 = (phoneme_envelope[3].1 * 100.0).clamp(0.0, 200.0);
                wavtool_env.v4 = (phoneme_envelope[4].1 * 100.0).clamp(0.0, 200.0);
                wavtool_env.p4 = 0.0;
                wavtool_env.p5 = p5_diff;
                wavtool_env.v5 = (phoneme_envelope[2].1 * 100.0).clamp(0.0, 200.0);

                let wav_args = WavtoolArgs {
                    output_wav: temp_dir_path.join(format!("wavtool_{idx}.wav")),
                    input_rendered_wav: res_args.output_wav.clone(),
                    skip_over_ms: timing.skip_over_ms,
                    duration_ms: envelope_duration_ms,
                    envelope: wavtool_env,
                    overlap_ms: active_overlap,
                    phoneme_envelope,
                    sample_time_zero_ms: -timing.pitch_leading_ms,
                };

                if let Err(error) =
                    wavtool_driver.process_note(&mut note_rendered, sample_rate, &wav_args, cancel)
                {
                    logs.push((
                        progress,
                        format!("  [Wavtool] {error}; usando processamento nativo"),
                    ));
                    let _ = NativeWavtoolDriver.process_note(
                        &mut note_rendered,
                        sample_rate,
                        &wav_args,
                        cancel,
                    );
                }
                let post_wavtool_max = note_rendered.iter().map(|s| s.abs()).fold(0.0f32, f32::max);
                logs.push((
                    progress,
                    format!(
                        "  [Wavtool] {} samples, max_amp={:.4}",
                        note_rendered.len(),
                        post_wavtool_max
                    ),
                ));

                let total_dyn_db = phone.expressions.dynamics * 0.1 + loudness_db;
                for (sample_index, sample) in note_rendered.iter_mut().enumerate() {
                    let time_ms =
                        sample_index as f64 * 1000.0 / sample_rate as f64 - timing.pitch_leading_ms;
                    let dyn_gain = 10.0f64.powf(total_dyn_db / 20.0);
                    let vibrato_volume = phone
                        .vibrato
                        .volume_multiplier_at(time_ms, phone.duration_ms);
                    *sample *= (dyn_gain * vibrato_volume) as f32;
                }

                let mut source_skip_ms = timing.skip_over_ms;
                let unclamped_start_ms = phone.position_ms - timing.preutter_ms;
                let actual_start_ms = unclamped_start_ms.max(0.0);
                if unclamped_start_ms < 0.0 {
                    source_skip_ms += -unclamped_start_ms;
                }

                let done = completed_ref.fetch_add(1, Ordering::Relaxed) + 1;
                let cur_progress = (done as f32 / total_phones as f32).min(0.99);
                log(
                    cur_progress,
                    &format!(
                        "⚡ [{}] Fonema '{}' ({}/{})",
                        resampler_driver.name(),
                        phone.lyric,
                        done,
                        total_phones
                    ),
                );

                Some(PhoneResult {
                    idx,
                    note_rendered,
                    actual_start_ms,
                    source_skip_ms,
                    crossfade_ms: if timing.overlap_ms > 0.0 {
                        active_overlap
                    } else {
                        0.0
                    },
                    pitch_freq: target_freq,
                    logs,
                })
            })
            .collect();

        // ------------------------------------------------------------------
        // Phase 2: Sequential merge.
        //
        // `mix_phase_aligned` is order-dependent (uses `previous_phone_end`
        // for phase-locked crossfade), so it must run in idx order.
        // ------------------------------------------------------------------
        if cancel.is_some_and(|t| t.load(Ordering::Relaxed)) {
            log(1.0, "[Render] Cancelled");
            return Vec::new();
        }

        phone_results.sort_unstable_by_key(|r| r.idx);

        for result in phone_results {
            // Replay per-phone log messages in-order now that we are sequential.
            for (progress, msg) in result.logs {
                log(progress, &msg);
            }

            let source_skip_samples =
                ((result.source_skip_ms / 1000.0) * sample_rate as f64).round() as usize;
            let audible_samples = result
                .note_rendered
                .get(source_skip_samples.min(result.note_rendered.len())..)
                .unwrap_or(&[]);
            let start_sample_idx =
                ((result.actual_start_ms / 1000.0) * sample_rate as f64).round() as usize;
            previous_phone_end_sample = Self::mix_phase_aligned(
                &mut track_buffer,
                audible_samples,
                start_sample_idx,
                previous_phone_end_sample,
                ((result.crossfade_ms / 1000.0) * sample_rate as f64).round() as usize,
                result.pitch_freq,
                sample_rate,
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
    use crate::drivers::{NativeResamplerDriver, NativeWavtoolDriver};
    use crate::oto::Voicebank;
    use crate::phonemizer::PhonemizerMode;
    use crate::project::model::UNote;
    use crate::renderer::RenderOptions;

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
    fn mixer_adds_pre_enveloped_segments_without_a_second_fade() {
        let mut track = vec![0.0; 10];
        for (index, sample) in track[5..10].iter_mut().enumerate() {
            *sample = 1.0 - index as f32 / 4.0;
        }
        track.resize(15, 0.0);
        let mut next = vec![1.0; 10];
        for (index, sample) in next[..5].iter_mut().enumerate() {
            *sample = index as f32 / 4.0;
        }

        let end = TrackRenderer::mix_phase_aligned(&mut track, &next, 5, 10, 0, 261.63, 44_100);

        assert_eq!(end, 15);
        assert!((track[5] - 1.0).abs() < 1e-6);
        assert!((track[9] - 1.0).abs() < 1e-6);
        assert!(track[5..10].iter().all(|sample| *sample <= 1.000_001));
        assert!(track[5..10].windows(2).all(|pair| {
            let jump = (pair[1] - pair[0]).abs();
            jump < 1e-5
        }));
    }

    #[test]
    fn mixer_phase_aligns_the_voiced_crossfade_window() {
        let sample_rate = 1_000u32;
        let pitch = 100.0;
        let mut track = vec![0.0f32; 160];
        for (index, sample) in track[..100].iter_mut().enumerate() {
            let carrier = (index as f32 * std::f32::consts::TAU / 10.0).sin();
            let gain = if index < 50 {
                1.0
            } else {
                (100 - index) as f32 / 50.0
            };
            *sample = carrier * gain;
        }
        let mut next = vec![0.0f32; 100];
        for (index, sample) in next.iter_mut().enumerate() {
            let carrier = ((index as f32 + 5.0) * std::f32::consts::TAU / 10.0).sin();
            let gain = (index as f32 / 50.0).min(1.0);
            *sample = carrier * gain;
        }

        TrackRenderer::mix_phase_aligned(&mut track, &next, 50, 100, 50, pitch, sample_rate);

        let rms = (track[55..95]
            .iter()
            .map(|sample| f64::from(*sample).powi(2))
            .sum::<f64>()
            / 40.0)
            .sqrt();
        // Without a phase correction these opposite carriers cancel almost
        // completely. The windowed search must recover audible energy.
        assert!(rms > 0.25, "phase-aligned crossfade RMS was {rms}");
    }

    #[test]
    fn real_vcv_fixture_has_no_silent_transition_hole() {
        if !std::path::Path::new("demo_vb/ka.wav").exists() {
            return;
        }
        let directory = tempfile::tempdir().unwrap();
        for name in ["ka.wav", "ki.wav"] {
            std::fs::copy(format!("demo_vb/{name}"), directory.path().join(name)).unwrap();
        }
        std::fs::write(
            directory.path().join("oto.ini"),
            "ka.wav=- ka,10,300,-100,300,100\nki.wav=a ki,10,300,-100,300,100\n",
        )
        .unwrap();
        let voicebank = Voicebank::new(directory.path()).unwrap();
        let notes = vec![
            UNote::new("ka", "C4", 0.0, 500.0),
            UNote::new("ki", "D4", 500.0, 500.0),
        ];
        let options = RenderOptions {
            phonemizer_mode: PhonemizerMode::VCV,
            ..RenderOptions::default()
        };
        let audio = TrackRenderer::render_track_with_drivers(
            &notes,
            &voicebank,
            44_100,
            120.0,
            &NativeResamplerDriver,
            &NativeWavtoolDriver,
            Some(&options),
        );

        // With 300 ms preutterance and 100 ms overlap the VCV transition is
        // 200..300 ms. The previous bug positioned the next segment at 400 ms,
        // leaving this interval silent or ending it with a hard onset.
        for center_ms in [220.0, 250.0, 280.0] {
            let center = (center_ms * 44.1) as usize;
            let radius = 220usize;
            let window = &audio[center - radius..center + radius];
            let rms = (window
                .iter()
                .map(|sample| f64::from(*sample) * f64::from(*sample))
                .sum::<f64>()
                / window.len() as f64)
                .sqrt();
            assert!(
                rms > 0.005,
                "silent VCV transition at {center_ms} ms: {rms}"
            );
        }
    }

    #[test]
    fn phrase_portamento_is_shared_across_adjacent_phonemes() {
        let notes = vec![
            UNote::new("ka", "C4", 0.0, 500.0),
            UNote::new("ki", "D4", 500.0, 500.0),
        ];
        let curve = TrackRenderer::phrase_pitch_notes(&notes);
        let at = |time| TrackRenderer::phrase_pitch_cents_at(&curve, time, 0.0);
        assert!((at(450.0) - 6000.0).abs() < 1e-6);
        assert!((at(460.0) - 6000.0).abs() < 1e-6);
        assert!((at(500.0) - 6100.0).abs() < 1e-6);
        assert!((at(540.0) - 6200.0).abs() < 1e-6);
    }
}
