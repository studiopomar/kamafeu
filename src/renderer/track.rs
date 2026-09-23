use crate::dsp::midi_to_freq;
mod mixing;
mod phone_result;
mod phrase_pitch;
mod slots;
mod wav_io;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

use self::phone_result::PhoneResult;
use self::slots::ResamplerSlots;
use crate::drivers::{
    NativeResamplerDriver, NativeWavtoolDriver, ResamplerArgs, ResamplerDriver, WavtoolArgs,
    WavtoolDriver,
};
use crate::oto::Voicebank;
use crate::phonemizer::RenderPhone;
use crate::project::model::UNote;
use crate::renderer::timing::resolve_phoneme_timings;
use crate::renderer::RenderOptions;

pub struct TrackRenderer;

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

        let (
            loudness_db,
            gender_offset,
            breathiness_offset,
            tone_shift,
            crossfade_ms,
            resampler_instances,
        ) = if let Some(vm) = vocal_mode {
            (
                vm.loudness,
                vm.gender,
                vm.breathiness,
                vm.tone_shift,
                vm.crossfade_ms,
                vm.resampler_instances,
            )
        } else {
            (0.0, 0.0, 0.0, 0.0, 0.0, 2)
        };

        let max_end_ms = notes
            .iter()
            .map(|n| n.position_ms + n.duration_ms + n.envelope.p5)
            .fold(0.0f64, f64::max);

        let total_samples =
            ((max_end_ms / 1000.0) * sample_rate as f64) as usize + sample_rate as usize;
        let mut track_buffer = vec![0.0f32; total_samples];
        let mut previous_phone_end_sample = 0usize;

        let temp_dir = tempfile::Builder::new()
            .prefix("kamafeu-render-")
            .tempdir()
            .ok();
        let temp_dir_path = temp_dir
            .as_ref()
            .map(|d| d.path().to_path_buf())
            .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));

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
        let phase_alignment = wavtool_driver.phase_alignment();
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
        #[cfg(not(target_arch = "wasm32"))]
        let wav_cache: HashMap<std::path::PathBuf, Arc<(Vec<f32>, u32)>> = wav_paths_vec
            .into_par_iter()
            .filter_map(|path| {
                Self::load_wav_samples(&path)
                    .ok()
                    .map(|(samples, rate)| (path, Arc::new((samples, rate))))
            })
            .collect();
        #[cfg(target_arch = "wasm32")]
        let wav_cache: HashMap<std::path::PathBuf, Arc<(Vec<f32>, u32)>> = wav_paths_vec
            .into_iter()
            .filter_map(|path| {
                Self::load_wav_samples(&path)
                    .ok()
                    .map(|(samples, rate)| (path, Arc::new((samples, rate))))
            })
            .collect();
        // Never substitute a missing phoneme with generated audio. Keep its
        // time slot silent and resolve transitions only between available phones.
        let unavailable = phones
            .iter()
            .map(|phone| {
                voicebank
                    .find_mapped_entry(&phone.lyric, &phone.pitch)
                    .is_none_or(|entry| {
                        let path = voicebank.root_path.join(&entry.wav_filename);
                        wav_cache.get(&path).is_none_or(|audio| audio.0.is_empty())
                    })
            })
            .collect::<Vec<_>>();
        let missing_inside_phrase = unavailable.iter().enumerate().any(|(index, missing)| {
            if !missing {
                return false;
            }
            let phone = &phones[index];
            let touches_previous = index > 0
                && (phones[index - 1].position_ms + phones[index - 1].duration_ms
                    - phone.position_ms)
                    .abs()
                    <= 0.001;
            let touches_next = phones.get(index + 1).is_some_and(|next| {
                (phone.position_ms + phone.duration_ms - next.position_ms).abs() <= 0.001
            });
            touches_previous || touches_next
        });
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
        if missing_inside_phrase {
            // The phonemizer has already shortened the preceding vowel to make
            // room for the missing bridge. Restore continuity up to the next
            // available phone instead of leaving a hole or aborting unrelated
            // phrases. The warning above still exposes the voicebank defect.
            for index in 0..phones.len().saturating_sub(1) {
                let current_note = phones[index].note_index;
                let next_note = phones[index + 1].note_index;
                let same_phrase = current_note == next_note
                    || notes
                        .get(current_note)
                        .zip(notes.get(next_note))
                        .is_some_and(|(current, next)| {
                            (current.position_ms + current.duration_ms - next.position_ms).abs()
                                <= 0.001
                        });
                let next_position = phones[index + 1].position_ms;
                let current_end = phones[index].position_ms + phones[index].duration_ms;
                if same_phrase && next_position > current_end + 0.001 {
                    phones[index].duration_ms = next_position - phones[index].position_ms;
                }
            }
            log(
                0.0,
                "[Warning] A geometria da frase foi recomposta ao redor de aliases ausentes; a vogal anterior foi preservada até o próximo fonema disponível.",
            );
        }
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

        let temp_dir_path = temp_dir_path.clone();
        let pitch_notes_ref = &pitch_notes;
        let completed_phones = std::sync::atomic::AtomicUsize::new(0);
        let completed_ref = &completed_phones;

        // Schedule the earliest audible leads first. A VCV/CVVC alias often
        // begins before its musical note because of preutterance, so this is a
        // better preview order than the source note order.
        let mut render_order: Vec<_> = phones.into_iter().enumerate().collect();
        render_order.sort_unstable_by(|(left_index, left), (right_index, right)| {
            (left.position_ms - timings[*left_index].preutter_ms)
                .total_cmp(&(right.position_ms - timings[*right_index].preutter_ms))
        });
        let resampler_slots = ResamplerSlots::new(resampler_instances);

        let render_phone_fn = |(idx, phone): (usize, RenderPhone)| -> Result<PhoneResult, String> {
            if cancel.is_some_and(|t| t.load(Ordering::Relaxed)) {
                return Err("Renderização cancelada".to_string());
            }

            let mut logs: Vec<(f32, String)> = Vec::new();
            let progress = idx as f32 / total_phones as f32;
            let timing = timings[idx];

            let entry = voicebank
                .find_mapped_entry(&phone.lyric, &phone.pitch)
                .ok_or_else(|| format!("Alias indisponível: {}", phone.lyric))?;
            let offset_ms = entry.offset + phone.expressions.start_point_ms.unwrap_or(0.0).max(0.0);
            let consonant_ms = entry.consonant;
            let cutoff_ms = entry.cutoff;
            let loop_start_ms = entry.loop_start;
            let loop_end_ms = entry.loop_end;
            let tail_start_ms = entry.tail_start;
            let wav_full_path = voicebank.root_path.join(&entry.wav_filename);
            logs.push((
                    progress,
                    format!(
                    "[Render] Phone '{}' ({}/{}) pitch={} pos={:.0}ms dur={:.0}ms wav={:?} oto.ini={}",
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
                phone.expressions.consonant_velocity.clamp(-100.0, 200.0)
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

            let res_args = ResamplerArgs {
                input_wav: wav_full_path.clone(),
                output_wav: temp_dir_path.join(format!("render_{idx}.wav")),
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

            let rendered_or_cached = {
                let _resampler_slot = resampler_slots.acquire(cancel)?;
                crate::renderer::resampler_cache::render_with_cache(
                    resampler_driver,
                    raw_samples,
                    src_sample_rate,
                    &res_args,
                    cancel,
                )
            };
            let mut note_rendered = rendered_or_cached
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

            let dynamics_curve = phone.expressions.dynamics_curve.clone();
            for (sample_index, sample) in note_rendered.iter_mut().enumerate() {
                let time_ms = sample_index as f64 * 1000.0 / sample_rate as f64
                    - timing.pitch_leading_ms
                    + if wavtool_consumed_skip {
                        timing.skip_over_ms
                    } else {
                        0.0
                    };
                let curve_value = if dynamics_curve.is_empty() {
                    phone.expressions.dynamics
                } else {
                    let t = time_ms.max(0.0);
                    dynamics_curve
                        .windows(2)
                        .find(|pair| t <= pair[1].time_offset_ms)
                        .map(|pair| {
                            let span = (pair[1].time_offset_ms - pair[0].time_offset_ms).max(1e-6);
                            let u = ((t - pair[0].time_offset_ms) / span).clamp(0.0, 1.0);
                            pair[0].value + (pair[1].value - pair[0].value) * u
                        })
                        .or_else(|| dynamics_curve.last().map(|point| point.value))
                        .unwrap_or(phone.expressions.dynamics)
                };
                let dyn_gain = 10.0f64.powf((curve_value * 0.1 + loudness_db) / 20.0);
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
                Self::save_wav_samples(&wav_args.input_rendered_wav, &note_rendered, sample_rate)?;
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

            Ok(PhoneResult {
                idx,
                wav_args,
                adjacent: timing.adjacent,
                note_rendered,
                actual_start_ms,
                source_skip_ms,
                crossfade_ms: if phase_alignment && timing.overlap_ms > 0.0 {
                    active_overlap
                } else {
                    0.0
                },
                pitch_freq: target_freq,
                logs,
            })
        };

        #[cfg(not(target_arch = "wasm32"))]
        let phone_results: Result<Vec<PhoneResult>, String> =
            render_order.into_par_iter().map(render_phone_fn).collect();

        #[cfg(target_arch = "wasm32")]
        let phone_results: Result<Vec<PhoneResult>, String> =
            render_order.into_iter().map(render_phone_fn).collect();

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
            );
        }

        Self::apply_soft_limiter(&mut track_buffer, 0.89);

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
}

#[cfg(test)]
mod tests;
