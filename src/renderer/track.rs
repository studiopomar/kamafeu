use crate::dsp::midi_to_freq;
mod mixing;
mod phrase_pitch;
mod wav_io;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use rayon::prelude::*;

use crate::drivers::{
    NativeResamplerDriver, NativeWavtoolDriver, ResamplerArgs, ResamplerDriver, WavtoolArgs,
    WavtoolDriver,
};
use crate::oto::Voicebank;
use crate::project::model::UNote;
use crate::renderer::timing::resolve_phoneme_timings;
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
        match Self::try_render_track_with_progress_cancellable(
            notes,
            voicebank,
            sample_rate,
            tempo_bpm,
            resampler_driver,
            wavtool_driver,
            vocal_mode,
            on_progress,
            cancel,
        ) {
            Ok(samples) => samples,
            Err(error) => {
                if let Some(cb) = on_progress {
                    cb(1.0, &error);
                }
                eprintln!("{error}");
                Vec::new()
            }
        }
    }

    pub fn try_render_track_with_progress_cancellable(
        notes: &[UNote],
        voicebank: &Voicebank,
        sample_rate: u32,
        tempo_bpm: f64,
        resampler_driver: &dyn ResamplerDriver,
        wavtool_driver: &dyn WavtoolDriver,
        vocal_mode: Option<&RenderOptions>,
        on_progress: Option<&(dyn Fn(f32, &str) + Send + Sync)>,
        cancel: Option<&AtomicBool>,
    ) -> Result<Vec<f32>, String> {
        if notes.is_empty() {
            return Ok(Vec::new());
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
                (0.0, 0.0, 0.0, 0.0, 0.0)
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
                return Err(format!("Falha ao criar diretório temporário: {error}"));
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
        let mut phones =
            crate::phonemizer::JapanesePhonemizer::apply_phonemizer(notes, voicebank, mode);
        let pitch_notes = Self::phrase_pitch_notes(notes);

        // ------------------------------------------------------------------
        // Phase 0: Pre-load all unique WAV files into a shared in-memory
        // cache.  This eliminates repeated disk reads for voicebanks where
        // many notes share the same sample file (e.g. all 'あ' notes use the
        // same あ.wav).  The load itself is also parallelised with rayon.
        // ------------------------------------------------------------------
        let unique_wav_paths: std::collections::HashSet<std::path::PathBuf> = phones
            .iter()
            .filter_map(|phone| {
                voicebank
                    .find_mapped_entry(&phone.lyric, &phone.pitch)
                    .map(|entry| voicebank.root_path.join(&entry.wav_filename))
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
        // Never substitute a missing phoneme with generated audio. Keep its
        // time slot silent and resolve transitions only between available phones.
        phones.retain(|phone| {
            let reason = match voicebank.find_mapped_entry(&phone.lyric, &phone.pitch) {
                None => Some("alias ausente no voicebank".to_string()),
                Some(entry) => {
                    let path = voicebank.root_path.join(&entry.wav_filename);
                    match wav_cache.get(&path) {
                        Some(audio) if !audio.0.is_empty() => None,
                        _ => Some(format!(
                            "amostra WAV ausente, vazia ou ilegível: {}",
                            path.display()
                        )),
                    }
                }
            };
            if let Some(reason) = reason {
                log(
                    0.0,
                    &format!(
                        "[Warning] Fonema '{}' ({}) em {:.0}ms silenciado: {reason}",
                        phone.lyric, phone.pitch, phone.position_ms
                    ),
                );
                false
            } else {
                true
            }
        });
        if phones.is_empty() {
            log(
                1.0,
                "[Render] Nenhum fonema disponível; faixa vocal em silêncio.",
            );
            return Ok(track_buffer);
        }

        let timing_inputs = super::timing::plan_inputs(&phones, voicebank, crossfade_ms);
        let timings = resolve_phoneme_timings(&timing_inputs);

        let total_phones = phones.len().max(1);

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
            wav_args: WavtoolArgs,
            adjacent: bool,
            is_transition: bool,
        }

        let temp_dir_path = temp_dir.path().to_path_buf();
        let pitch_notes_ref = &pitch_notes;
        let completed_phones = std::sync::atomic::AtomicUsize::new(0);
        let completed_ref = &completed_phones;

        let phone_results: Result<Vec<PhoneResult>, String> = phones
            .into_par_iter()
            .enumerate()
            .map(|(idx, phone)| {
                if cancel.is_some_and(|t| t.load(Ordering::Relaxed)) {
                    return Err("Renderização cancelada".to_string());
                }

                let mut logs: Vec<(f32, String)> = Vec::new();
                let progress = idx as f32 / total_phones as f32;
                let timing = timings[idx];

                let entry = voicebank
                    .find_mapped_entry(&phone.lyric, &phone.pitch)
                    .ok_or_else(|| format!("Alias indisponível: {}", phone.lyric))?;
                let offset_ms = entry.offset;
                let consonant_ms = entry.consonant;
                let cutoff_ms = entry.cutoff;
                let loop_start_ms = entry.loop_start;
                let loop_end_ms = entry.loop_end;
                let tail_start_ms = entry.tail_start;
                let wav_full_path = voicebank.root_path.join(&entry.wav_filename);
                logs.push((
                    progress,
                    format!(
                    "[Render] Phone '{}' ({}/{}) pitch={} pos={:.0}ms dur={:.0}ms wav={:?} oto={}",
                    phone.lyric, idx + 1, total_phones, phone.pitch,
                    phone.position_ms, phone.duration_ms, wav_full_path, true
                ),
                ));

                // Retrieve samples from the in-memory cache (zero disk I/O).
                let cached = wav_cache
                    .get(&wav_full_path)
                    .ok_or_else(|| format!("Amostra indisponível: {}", wav_full_path.display()))?;
                let (raw_samples, src_sample_rate) = (cached.0.as_slice(), cached.1);

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
                let active_consonant_ms =
                    (raw_scaled_consonant + phone.expressions.consonant_timing_offset_ms).max(0.0);

                let duration_correction_ms =
                    timing.preutter_ms - timing.tail_intrude_ms + timing.tail_overlap_ms;
                let target_render_ms =
                    (phone.duration_ms + duration_correction_ms + timing.skip_over_ms)
                        .max(consonant_ms.max(0.0))
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
                .map_err(|error| {
                    format!(
                        "Fonema #{} '{}' em {:.1} ms, resampler {}: {error}",
                        idx + 1,
                        phone.lyric,
                        phone.position_ms,
                        resampler_driver.name()
                    )
                })?;

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

                let active_overlap = if timing.adjacent {
                    timing.overlap_ms
                } else {
                    0.0
                };
                let envelope_duration_ms = (phone.duration_ms + duration_correction_ms).max(1.0);
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

                let external_phrase = wavtool_driver.phrase_executable().is_some();
                let wavtool_consumed_skip = if external_phrase {
                    false
                } else {
                    wavtool_driver
                        .process_note(&mut note_rendered, sample_rate, &wav_args, cancel)
                        .map_err(|error| {
                            format!(
                                "Fonema #{} '{}', wavtool {}: {error}",
                                idx + 1,
                                phone.lyric,
                                wavtool_driver.name()
                            )
                        })?;
                    wavtool_driver.consumes_skip_over()
                };
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
                    let time_ms = sample_index as f64 * 1000.0 / sample_rate as f64
                        - timing.pitch_leading_ms
                        + if wavtool_consumed_skip {
                            timing.skip_over_ms
                        } else {
                            0.0
                        };
                    let dyn_gain = 10.0f64.powf(total_dyn_db / 20.0);
                    let vibrato_volume = phone
                        .vibrato
                        .volume_multiplier_at(time_ms, phone.duration_ms);
                    *sample *= (dyn_gain * vibrato_volume) as f32;
                }

                let mut source_skip_ms = if wavtool_consumed_skip {
                    0.0
                } else {
                    timing.skip_over_ms
                };
                let unclamped_start_ms = phone.position_ms - timing.preutter_ms;
                let mut actual_start_ms = unclamped_start_ms.max(0.0);
                if unclamped_start_ms < 0.0 {
                    source_skip_ms += -unclamped_start_ms;
                }

                if source_skip_ms < 0.0 {
                    actual_start_ms += -source_skip_ms;
                    source_skip_ms = 0.0;
                }
                if external_phrase {
                    Self::save_wav_samples(
                        &wav_args.input_rendered_wav,
                        &note_rendered,
                        sample_rate,
                    )?;
                }

                let done = completed_ref.fetch_add(1, Ordering::Relaxed) + 1;
                let cur_progress = (done as f32 / total_phones as f32).min(0.99);
                log(
                    cur_progress,
                    &format!(
                        "[{}] Fonema '{}' ({}/{})",
                        resampler_driver.name(),
                        phone.lyric,
                        done,
                        total_phones
                    ),
                );

                let is_transition = cutoff_ms < 0.0
                    || phone.lyric.contains(' ')
                    || phone.lyric.contains('-')
                    || phone.lyric.starts_with('_');

                Ok(PhoneResult {
                    idx,
                    wav_args,
                    adjacent: timing.adjacent,
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
                    is_transition,
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
            return Err("Renderização cancelada".to_string());
        }

        let mut phone_results = phone_results?;
        phone_results.sort_unstable_by_key(|r| r.idx);

        if let Some(executable) = wavtool_driver.phrase_executable() {
            let mut begin = 0;
            while begin < phone_results.len() {
                let mut end = begin + 1;
                while end < phone_results.len() && phone_results[end].adjacent {
                    end += 1;
                }
                let group = &phone_results[begin..end];
                let args: Vec<_> = group.iter().map(|r| r.wav_args.clone()).collect();
                let output = temp_dir_path.join(format!("phrase-{begin}.wav"));
                let rendered = crate::drivers::wavtool_driver::concatenate_external(
                    &executable,
                    &args,
                    &output,
                    sample_rate,
                    cancel,
                )?;
                let origin =
                    timing_inputs[group[0].idx].position_ms - timings[group[0].idx].preutter_ms;
                let skip = ((-origin).max(0.0) * sample_rate as f64 / 1000.0).round() as usize;
                let start = (origin.max(0.0) * sample_rate as f64 / 1000.0).round() as usize;
                Self::mix_phase_aligned(
                    &mut track_buffer,
                    rendered.get(skip..).unwrap_or(&[]),
                    start,
                    0,
                    0,
                    0.0,
                    sample_rate,
                    false,
                );
                for item in group {
                    for (p, message) in &item.logs {
                        log(*p, message);
                    }
                }
                begin = end;
            }
            return Ok(track_buffer);
        }

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
                result.is_transition,
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

        Ok(track_buffer)
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
mod tests;
