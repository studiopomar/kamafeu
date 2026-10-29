use crate::dsp::midi_to_freq;
mod mixing;
mod phone_result;
mod phrase_pitch;
mod slots;
mod wav_io;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::Instant;

use rayon::prelude::*;

use self::phone_result::PhoneResult;
use self::slots::ResamplerSlots;
use crate::drivers::{
    NativeResamplerDriver, NativeWavtoolDriver, ResamplerArgs, ResamplerDriver, WavtoolArgs,
    WavtoolDriver, WavtoolMixStrategy,
};
use crate::oto::Voicebank;
use crate::phonemizer::RenderPhone;
use crate::project::model::UNote;
use crate::renderer::timing::resolve_phoneme_timings;
use crate::renderer::RenderOptions;

pub struct TrackRenderer;

static LAZY_WAV_LOADING: AtomicBool = AtomicBool::new(false);
static NEXT_RENDER_SESSION_ID: AtomicU64 = AtomicU64::new(1);

fn io_thread_count() -> &'static RwLock<usize> {
    static THREADS: OnceLock<RwLock<usize>> = OnceLock::new();
    THREADS.get_or_init(|| RwLock::new(4))
}

fn segment_placement(
    phone_position_ms: f64,
    preutter_ms: f64,
    skip_over_ms: f64,
    wavtool_consumed_skip: bool,
) -> (f64, f64) {
    let mut source_skip_ms = if wavtool_consumed_skip {
        0.0
    } else {
        skip_over_ms
    };
    let unclamped_start_ms = phone_position_ms - preutter_ms;
    let mut actual_start_ms = unclamped_start_ms.max(0.0);
    if unclamped_start_ms < 0.0 {
        source_skip_ms += -unclamped_start_ms;
    }
    if source_skip_ms < 0.0 {
        actual_start_ms += -source_skip_ms;
        source_skip_ms = 0.0;
    }
    (actual_start_ms, source_skip_ms)
}

fn quantized_render_duration_ms(target_render_ms: f64) -> f64 {
    ((target_render_ms.max(1.0) / 50.0).ceil() * 50.0).max(50.0)
}

impl TrackRenderer {
    pub fn set_io_thread_concurrency(threads: u32) {
        if let Ok(mut value) = io_thread_count().write() {
            *value = threads.clamp(1, 16) as usize;
        }
    }

    pub fn set_preload_strategy(strategy: &str) {
        LAZY_WAV_LOADING.store(strategy == "Lazy On-Demand", Ordering::Relaxed);
    }

    pub fn set_anti_aliasing_filter(enabled: bool) {
        wav_io::set_anti_aliasing_filter(enabled);
    }

    pub fn set_crossfade_curve(curve: &str) {
        mixing::set_crossfade_curve(curve);
    }

    /// Applies the advanced pitch sampling controls used by all subsequent
    /// renders. This is intentionally process-wide because rendering happens
    /// on worker threads and the preferences dialog can change while idle.
    pub fn set_pitch_sampling_config(step_ms: f32, interpolation: &str) {
        phrase_pitch::set_pitch_sampling_config(step_ms, interpolation);
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
                crate::terminal_log::error("Renderer", error);
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
        Self::try_render_track_with_callbacks(
            notes,
            voicebank,
            sample_rate,
            tempo_bpm,
            resampler_driver,
            wavtool_driver,
            vocal_mode,
            on_progress,
            None,
            cancel,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn try_render_track_with_progress_events(
        notes: &[UNote],
        voicebank: &Voicebank,
        sample_rate: u32,
        tempo_bpm: f64,
        resampler_driver: &dyn ResamplerDriver,
        wavtool_driver: &dyn WavtoolDriver,
        vocal_mode: Option<&RenderOptions>,
        on_progress: Option<&(dyn Fn(f32, &str) + Send + Sync)>,
        on_progress_event: Option<&(dyn Fn(crate::renderer::RenderProgressEvent) + Send + Sync)>,
        cancel: Option<&AtomicBool>,
    ) -> Result<Vec<f32>, String> {
        Self::try_render_track_with_callbacks(
            notes,
            voicebank,
            sample_rate,
            tempo_bpm,
            resampler_driver,
            wavtool_driver,
            vocal_mode,
            on_progress,
            on_progress_event,
            cancel,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn try_render_track_with_callbacks(
        notes: &[UNote],
        voicebank: &Voicebank,
        sample_rate: u32,
        tempo_bpm: f64,
        resampler_driver: &dyn ResamplerDriver,
        wavtool_driver: &dyn WavtoolDriver,
        vocal_mode: Option<&RenderOptions>,
        on_progress: Option<&(dyn Fn(f32, &str) + Send + Sync)>,
        on_progress_event: Option<&(dyn Fn(crate::renderer::RenderProgressEvent) + Send + Sync)>,
        cancel: Option<&AtomicBool>,
    ) -> Result<Vec<f32>, String> {
        if notes.is_empty() {
            return Ok(Vec::new());
        }

        let render_session_id = NEXT_RENDER_SESSION_ID.fetch_add(1, Ordering::Relaxed);
        let render_started = Instant::now();
        let log = |progress: f32, msg: &str| {
            // The GUI receives progress continuously. The terminal trace is
            // opt-in through KAMAFEU_LOG=verbose so normal renders stay quiet.
            crate::terminal_log::render(
                render_session_id,
                render_started.elapsed().as_millis(),
                progress,
                msg,
            );
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

        let is_default_or_empty_vb = voicebank.entries.is_empty()
            || voicebank.name == "Synthetic Fallback"
            || voicebank.name == "Default Singer"
            || voicebank.name == "Cantor Padrão";

        if is_default_or_empty_vb {
            if !notes.is_empty() {
                log(
                    1.0,
                    "[Render] Cantor Padrão (sem voicebank): sintetizando notas com som de piano/teclado...",
                );
                return Ok(crate::dsp::piano::render_piano_track(notes, sample_rate));
            } else {
                return Ok(Vec::new());
            }
        }

        let mode = if let Some(vm) = vocal_mode {
            vm.phonemizer_mode
        } else {
            crate::phonemizer::PhonemizerMode::BasicCV
        };
        let mix_strategy = wavtool_driver.mix_strategy();
        let preview_skip_failed_phones = vocal_mode
            .is_some_and(|options| options.preview_skip_failed_phones);
        log(
            0.0,
            &format!(
                "[Render][Start] notes={} sample_rate={}Hz tempo={:.3}BPM voicebank='{}' root='{}' resampler='{}' wavtool='{}' phonemizer={:?} flags='{}' crossfade_ms={:.3} resampler_instances={} lazy_wav_loading={} mix_strategy={:?} envelope_curve={}",
                notes.len(),
                sample_rate,
                tempo_bpm,
                voicebank.name,
                voicebank.root_path.display(),
                resampler_driver.name(),
                wavtool_driver.name(),
                mode,
                vocal_mode.map(|value| value.flags.as_str()).unwrap_or(""),
                crossfade_ms,
                resampler_instances,
                LAZY_WAV_LOADING.load(Ordering::Relaxed),
                mix_strategy,
                wavtool_driver.envelope_curve_name(),
            ),
        );
        let mut phones =
            crate::phonemizer::JapanesePhonemizer::apply_phonemizer(notes, voicebank, mode);
        log(
            0.02,
            &format!(
                "[Phonemizer][Input] mode={mode:?} notes_in={} phones_out={}",
                notes.len(),
                phones.len(),
            ),
        );
        for (index, note) in notes.iter().enumerate() {
            log(
                0.02,
                &format!(
                    "[Phonemizer][Note {}/{}] lyric='{}' pitch='{}' position_ms={:.3} duration_ms={:.3} flags='{}' voicebank_pitch={:?} timbre={:?} phonemizer_override={:?} phoneme_durations={:?} overrides={}",
                    index + 1,
                    notes.len(),
                    note.lyric,
                    note.pitch,
                    note.position_ms,
                    note.duration_ms,
                    note.flags,
                    note.voicebank_pitch,
                    note.timbre,
                    note.phonemizer_override,
                    note.phoneme_durations_ms,
                    note.phoneme_overrides.len(),
                ),
            );
        }
        for (index, phone) in phones.iter().enumerate() {
            log(
                0.03,
                &format!(
                    "[Phonemizer][Phone {}/{}] note_index={} alias='{}' resolved_alias={:?} resolved_timbre={:?} pitch='{}' position_ms={:.3} duration_ms={:.3} flags='{}' envelope={:?} expressions={:?}",
                    index + 1,
                    phones.len(),
                    phone.note_index,
                    phone.lyric,
                    phone.resolved_alias,
                    phone.resolved_timbre,
                    phone.pitch,
                    phone.position_ms,
                    phone.duration_ms,
                    phone.flags,
                    phone.envelope,
                    phone.expressions,
                ),
            );
        }
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
                phone
                    .find_entry(voicebank)
                    .map(|entry| voicebank.root_path.join(&entry.wav_filename))
            })
            .collect();

        // Collect paths into a Vec so rayon can index them.
        let wav_paths_vec: Vec<std::path::PathBuf> = if LAZY_WAV_LOADING.load(Ordering::Relaxed) {
            Vec::new()
        } else {
            unique_wav_paths.into_iter().collect()
        };
        #[cfg(not(target_arch = "wasm32"))]
        let io_threads = io_thread_count().read().map(|value| *value).unwrap_or(4);
        #[cfg(target_arch = "wasm32")]
        let io_threads = 1;
        #[cfg(not(target_arch = "wasm32"))]
        let wav_cache: HashMap<std::path::PathBuf, Arc<(Vec<f32>, u32)>> =
            if wav_paths_vec.is_empty() {
                HashMap::new()
            } else {
                let load_wavs = || {
                    wav_paths_vec
                        .into_par_iter()
                        .filter_map(|path| {
                            Self::load_wav_samples(&path)
                                .ok()
                                .map(|(samples, rate)| (path, Arc::new((samples, rate))))
                        })
                        .collect()
                };
                match crate::renderer::worker_pool::get(io_threads) {
                    Ok(pool) => pool.install(load_wavs),
                    Err(_) => load_wavs(),
                }
            };
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
                phone.find_entry(voicebank).is_none_or(|entry| {
                    let path = voicebank.root_path.join(&entry.wav_filename);
                    if LAZY_WAV_LOADING.load(Ordering::Relaxed) {
                        !path.is_file() || crate::audio::loader::probe_audio_file(&path).is_none()
                    } else {
                        wav_cache.get(&path).is_none_or(|audio| audio.0.is_empty())
                    }
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
            let reason = match phone.find_entry(voicebank) {
                None => Some("alias ausente no voicebank".to_string()),
                Some(entry) => {
                    let path = voicebank.root_path.join(&entry.wav_filename);
                    let available = if LAZY_WAV_LOADING.load(Ordering::Relaxed) {
                        path.is_file() && crate::audio::loader::probe_audio_file(&path).is_some()
                    } else {
                        wav_cache
                            .get(&path)
                            .is_some_and(|audio| !audio.0.is_empty())
                    };
                    (!available).then(|| {
                        format!("amostra WAV ausente, vazia ou ilegível: {}", path.display())
                    })
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

        let timing_inputs = super::timing::plan_inputs_with_mode(
            &phones,
            voicebank,
            crossfade_ms,
            vocal_mode
                .map(|options| options.compatibility_mode)
                .unwrap_or_default(),
        );
        let timings = resolve_phoneme_timings(&timing_inputs);
        let resolved_envelopes = super::timing::resolve_openutau_envelopes(&phones, &timings);
        for diagnostic in super::timing::diagnostics(&timing_inputs, &timings) {
            let serialized = serde_json::to_string(&diagnostic)
                .expect("phoneme timing diagnostic is serializable");
            log(0.0, &format!("[Timing] {serialized}"));
        }

        let total_phones = phones.len().max(1);
        if let Some(callback) = on_progress_event {
            callback(crate::renderer::RenderProgressEvent::counted(
                crate::renderer::RenderStage::Rendering,
                0,
                total_phones,
            ));
        }

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
        let resolved_envelopes_ref = &resolved_envelopes;

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
            let resolved_envelope = resolved_envelopes_ref[idx];

            let entry = phone
                .find_entry(voicebank)
                .ok_or_else(|| format!("Alias indisponível: {}", phone.lyric))?;
            let offset_ms = entry.offset + phone.expressions.start_point_ms.unwrap_or(0.0).max(0.0);
            let consonant_ms = entry.consonant;
            let mut cutoff_ms = entry.cutoff;
            if phone.coda_truncation {
                let safe_coda_len = (consonant_ms.max(20.0) + 25.0).max(45.0);
                cutoff_ms = -(offset_ms + safe_coda_len);
            }
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
            let on_demand;
            let cached = if let Some(cached) = wav_cache.get(&wav_full_path) {
                cached
            } else {
                on_demand = Self::load_wav_samples(&wav_full_path)
                    .map(|(samples, rate)| Arc::new((samples, rate)))
                    .map_err(|error| format!("Amostra indisponível: {error}"))?;
                &on_demand
            };
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
            let mut dur_required = quantized_render_duration_ms(target_render_ms);
            if phone.coda_truncation {
                dur_required = dur_required.min(consonant_ms + 40.0).max(30.0);
            }
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
            let inherited_flags = vocal_mode.map(|mode| mode.flags.as_str()).unwrap_or("");
            let effective_flags = crate::drivers::resampler_driver::merge_utau_flags(&[
                inherited_flags,
                &phone.flags,
            ]);
            let flags =
                resampler_driver.prepare_flags(&effective_flags, total_gender, total_breathiness);

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
                normalization: phone.expressions.presamp_normalization,
                tempo: tempo_bpm,
                pitch_bend_str: pitch_bend_encoded,
                pitch_points: combined_pitch,
                loop_start_ms,
                loop_end_ms,
                tail_start_ms,
                vowel_formant: phone.expressions.vowel_formant(),
            };

            logs.push((
                progress,
                format!(
                    "  [Resampler] Motor: '{}' | flags finais: '{}' | pitch: {} | duração: {:.1}ms",
                    resampler_driver.name(),
                    crate::drivers::resampler_driver::effective_classic_flags(&res_args),
                    res_args.pitch_name,
                    res_args.duration_ms
                ),
            ));
            logs.push((
                progress,
                format!(
                    "  [Resampler Command] {}",
                    crate::drivers::resampler_driver::describe_resampler_command(
                        resampler_driver.name(),
                        &res_args,
                    )
                ),
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
            let rendered_with_execution =
                rendered_or_cached.map(|(samples, cache_hit, execution)| {
                    logs.push((
                        progress,
                        format!(
                            "  [Resampler Execution] {}",
                            serde_json::to_string(&execution)
                                .expect("resampler execution metadata is serializable")
                        ),
                    ));
                    logs.push((
                        progress,
                        if cache_hit {
                            "  [Resampler Cache] hit".to_string()
                        } else {
                            "  [Resampler Cache] miss".to_string()
                        },
                    ));
                    (samples, execution)
                });
            let (mut note_rendered, resampler_execution): (
                Vec<f32>,
                crate::drivers::ResamplerExecutionInfo,
            ) = rendered_with_execution.map_err(|error| {
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

            let resampler_diagnostics =
                crate::renderer::diagnostics::AudioDiagnostics::analyze(&note_rendered, 1);
            let resampler_output_sample_rate = if res_args.output_wav.is_file() {
                Self::load_wav_samples(&res_args.output_wav)
                    .map(|(_, rate)| rate)
                    .unwrap_or(sample_rate)
            } else {
                sample_rate
            };
            let sample_rate_converted = resampler_output_sample_rate != sample_rate;
            logs.push((
                progress,
                format!(
                    "  [Resampler Diagnostics] {}",
                    resampler_diagnostics.render_log_line("raw")
                ),
            ));

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
            let phoneme_envelope = resolved_envelope.points;

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

            // SharpWavtool measures phase on the raw resampler output, before
            // applying either the amplitude envelope or post-wavtool dynamics.
            // Only two short windows are analyzed; the raw segment is not
            // cloned into the ordered merge result.
            let phase_metadata = if mix_strategy == WavtoolMixStrategy::OpenUtauConvergence {
                let (nominal_start_ms, nominal_skip_ms) = segment_placement(
                    phone.position_ms,
                    timing.preutter_ms,
                    timing.skip_over_ms,
                    false,
                );
                let nominal_start_samples =
                    (nominal_start_ms * sample_rate as f64 / 1000.0).round() as isize;
                let nominal_skip_samples =
                    (nominal_skip_ms * sample_rate as f64 / 1000.0).round() as isize;
                let head_center_ms = (phoneme_envelope[0].0 + phoneme_envelope[1].0) * 0.5;
                let tail_center_ms = (phoneme_envelope[3].0 + phoneme_envelope[4].0) * 0.5;
                let head_pitch_cents = Self::phrase_pitch_cents_at(
                    pitch_notes_ref,
                    phone.position_ms + head_center_ms,
                    tone_shift,
                );
                let tail_pitch_cents = Self::phrase_pitch_cents_at(
                    pitch_notes_ref,
                    phone.position_ms + tail_center_ms,
                    tone_shift,
                );
                crate::renderer::phase::analyze_openutau_segment(
                    &note_rendered,
                    sample_rate,
                    wav_args.sample_time_zero_ms,
                    &phoneme_envelope,
                    nominal_start_samples - nominal_skip_samples,
                    midi_to_freq(head_pitch_cents / 100.0),
                    midi_to_freq(tail_pitch_cents / 100.0),
                )
            } else {
                crate::renderer::phase::SegmentPhaseMetadata::default()
            };

            let wavtool_command = crate::drivers::wavtool_driver::describe_wavtool_command(
                wavtool_driver.name(),
                &wav_args,
            );

            logs.push((progress, format!("  [Wavtool Command] {}", wavtool_command)));

            let external_phrase = wavtool_driver.phrase_executable().is_some();
            logs.push((
                progress,
                format!(
                    "  [Wavtool] Motor: '{}' | modo: {} | envelope: {}",
                    wavtool_driver.name(),
                    if external_phrase { "externo" } else { "nativo" },
                    wavtool_driver.envelope_curve_name(),
                ),
            ));

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
            let wavtool_diagnostics =
                crate::renderer::diagnostics::AudioDiagnostics::analyze(&note_rendered, 1);
            logs.push((
                progress,
                format!(
                    "  [Wavtool] {} samples, max_amp={:.4} | {}",
                    note_rendered.len(),
                    post_wavtool_max,
                    wavtool_diagnostics.render_log_line("output")
                ),
            ));

            let render_diagnostic = crate::renderer::PhonemeRenderDiagnostic {
                phone_index: idx,
                lyric: phone.lyric.clone(),
                resampler_driver_id: resampler_driver.engine_id().to_string(),
                effective_resampler_engine_id: resampler_execution.effective_backend_id.clone(),
                resampler_stages: resampler_execution.stages.clone(),
                resampler_fallback_reasons: resampler_execution.fallback_reasons.clone(),
                resampler_adjustments: resampler_execution.adjustments.clone(),
                resampler: resampler_driver.name().to_string(),
                wavtool_engine_id: wavtool_driver.engine_id().to_string(),
                wavtool: wavtool_driver.name().to_string(),
                input_wav: res_args.input_wav.display().to_string(),
                output_wav: res_args.output_wav.display().to_string(),
                command: crate::drivers::resampler_driver::describe_resampler_command(
                    resampler_driver.name(),
                    &res_args,
                ),
                pitch: res_args.pitch_name.clone(),
                pitch_frequency: res_args.pitch_freq,
                flags: crate::drivers::resampler_driver::effective_classic_flags(&res_args),
                offset_ms: res_args.offset_ms,
                requested_duration_ms: phone.duration_ms,
                resampler_duration_ms: res_args.duration_ms,
                source_consonant_ms: res_args.source_consonant_ms,
                consonant_ms: res_args.consonant_ms,
                cutoff_ms: res_args.cutoff_ms,
                skip_over_ms: wav_args.skip_over_ms,
                overlap_ms: wav_args.overlap_ms,
                input_sample_rate: src_sample_rate,
                output_sample_rate: sample_rate,
                resampler_output_sample_rate,
                sample_rate_converted,
                resampler_peak: resampler_diagnostics.peak,
                resampler_rms: resampler_diagnostics.rms,
                resampler_clipped_samples: resampler_diagnostics.clipped_samples,
                wavtool_peak: wavtool_diagnostics.peak,
                wavtool_rms: wavtool_diagnostics.rms,
                wavtool_clipped_samples: wavtool_diagnostics.clipped_samples,
                envelope_points: phoneme_envelope,
                crossfade_ms: active_overlap.max(0.0),
            };
            logs.push((
                progress,
                format!(
                    "  [Phoneme Render] {}",
                    serde_json::to_string(&render_diagnostic)
                        .expect("phoneme render diagnostic is serializable")
                ),
            ));

            let dynamics_curve = &phone.expressions.dynamics_curve;
            let dynamics_curve_sorted = dynamics_curve
                .windows(2)
                .all(|pair| pair[0].time_offset_ms <= pair[1].time_offset_ms);
            let mut dynamics_curve_index = 0usize;
            let sample_time_step_ms = 1000.0 / sample_rate as f64;
            let time_base_ms = -timing.pitch_leading_ms
                + if wavtool_consumed_skip {
                    timing.skip_over_ms
                } else {
                    0.0
                };
            for (sample_index, sample) in note_rendered.iter_mut().enumerate() {
                let time_ms = sample_index as f64 * sample_time_step_ms + time_base_ms;
                let t = time_ms.max(0.0);
                let curve_value = if dynamics_curve.is_empty() {
                    phone.expressions.dynamics
                } else if dynamics_curve_sorted {
                    while dynamics_curve_index + 1 < dynamics_curve.len()
                        && t > dynamics_curve[dynamics_curve_index + 1].time_offset_ms
                    {
                        dynamics_curve_index += 1;
                    }

                    if dynamics_curve_index + 1 >= dynamics_curve.len() {
                        dynamics_curve
                            .last()
                            .map(|point| point.value)
                            .unwrap_or(phone.expressions.dynamics)
                    } else {
                        let left = &dynamics_curve[dynamics_curve_index];
                        let right = &dynamics_curve[dynamics_curve_index + 1];
                        let span = (right.time_offset_ms - left.time_offset_ms).max(1e-6);
                        let u = ((t - left.time_offset_ms) / span).clamp(0.0, 1.0);
                        left.value + (right.value - left.value) * u
                    }
                } else {
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

            let expression_diagnostics =
                crate::renderer::diagnostics::AudioDiagnostics::analyze(&note_rendered, 1);
            logs.push((
                progress,
                format!(
                    "  [Phone Diagnostics] phone='{}' {}",
                    phone.lyric,
                    expression_diagnostics.render_log_line("after-expressions")
                ),
            ));

            let boundary_window = ((sample_rate as f64 * 0.010).round() as usize).max(1);
            let head_rms =
                crate::renderer::diagnostics::window_rms(&note_rendered, boundary_window, false);
            let tail_rms =
                crate::renderer::diagnostics::window_rms(&note_rendered, boundary_window, true);
            logs.push((
                progress,
                format!(
                    "  [Timing] boundary-rms phone='{}' head={:.6} tail={:.6} window_ms=10.0",
                    phone.lyric, head_rms, tail_rms
                ),
            ));

            let (actual_start_ms, source_skip_ms) = segment_placement(
                phone.position_ms,
                timing.preutter_ms,
                timing.skip_over_ms,
                wavtool_consumed_skip,
            );
            if external_phrase {
                Self::save_wav_samples(&wav_args.input_rendered_wav, &note_rendered, sample_rate)?;
            }

            let done = completed_ref.fetch_add(1, Ordering::Relaxed) + 1;
            if let Some(callback) = on_progress_event {
                callback(crate::renderer::RenderProgressEvent::counted(
                    crate::renderer::RenderStage::Rendering,
                    done,
                    total_phones,
                ));
            }
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
                lyric: phone.lyric.clone(),
                wav_args,
                adjacent: timing.adjacent,
                note_rendered,
                actual_start_ms,
                source_skip_ms,
                crossfade_ms: if mix_strategy == WavtoolMixStrategy::LegacyPhaseAligned
                    && timing.overlap_ms > 0.0
                {
                    active_overlap
                } else {
                    0.0
                },
                pitch_freq: target_freq,
                logs,
                phase_metadata,
            })
        };

        #[cfg(not(target_arch = "wasm32"))]
        let rendered_phones: Vec<(usize, Result<PhoneResult, String>)> = render_order
            .into_par_iter()
            .map(|(idx, phone)| (idx, render_phone_fn((idx, phone))))
            .collect();

        #[cfg(target_arch = "wasm32")]
        let rendered_phones: Vec<(usize, Result<PhoneResult, String>)> = render_order
            .into_iter()
            .map(|(idx, phone)| (idx, render_phone_fn((idx, phone))))
            .collect();

        // ------------------------------------------------------------------
        // Phase 2: Sequential merge.
        //
        // Convergence is order-dependent: each correction contributes to the
        // phase used by the following phone, so this must run in idx order.
        // ------------------------------------------------------------------
        if cancel.is_some_and(|t| t.load(Ordering::Relaxed)) {
            log(1.0, "[Render] Cancelled");
            return Err("Renderização cancelada".to_string());
        }

        let mut phone_results = Vec::with_capacity(rendered_phones.len());
        let mut failed_phone_indices = HashSet::new();
        for (idx, result) in rendered_phones {
            match result {
                Ok(result) => phone_results.push(result),
                Err(error) if preview_skip_failed_phones => {
                    failed_phone_indices.insert(idx);
                    let done = completed_ref.fetch_add(1, Ordering::Relaxed) + 1;
                    if let Some(callback) = on_progress_event {
                        callback(crate::renderer::RenderProgressEvent::counted(
                            crate::renderer::RenderStage::Rendering,
                            done,
                            total_phones,
                        ));
                    }
                    let progress = (done as f32 / total_phones as f32).min(0.99);
                    log(
                        progress,
                        &format!(
                            "[Preview] Fonema #{} ignorado para manter o playback: {}",
                            idx + 1,
                            error
                        ),
                    );
                }
                Err(error) => return Err(error),
            }
        }
        phone_results.sort_unstable_by_key(|r| r.idx);
        for result in &mut phone_results {
            if result.idx > 0 && failed_phone_indices.contains(&(result.idx - 1)) {
                result.adjacent = false;
            }
        }

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

        let mut previous_phase_tail = None;
        let mut previous_phase_correction = 0isize;
        let mut previous_lyric: Option<String> = None;
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
            // The wavtool envelope is the authority for transition energy.
            // Do not apply RMS matching here: even a synthetic safety overlap
            // can otherwise raise a consonant by up to 1.5x and make it sound
            // like a separate, overly loud phoneme.
            match mix_strategy {
                WavtoolMixStrategy::OpenUtauConvergence => {
                    if !result.adjacent {
                        previous_phase_correction = 0;
                    }
                    let correction = if result.adjacent {
                        crate::renderer::phase::openutau_convergence_correction(
                            previous_phase_tail,
                            result.phase_metadata.head,
                            previous_phase_correction,
                            sample_rate,
                        )
                    } else {
                        0
                    };
                    let mixed = Self::mix_with_resolved_correction(
                        &mut track_buffer,
                        audible_samples,
                        start_sample_idx,
                        previous_phone_end_sample,
                        correction,
                    );
                    previous_phone_end_sample = mixed.end_sample;
                    let head = result.phase_metadata.head;
                    let tail = previous_phase_tail;
                    log(
                        result.idx as f32 / total_phones as f32,
                        &format!(
                            "  [Transition] strategy=OpenUtauConvergence from='{}' to='{}' adjacent={} tail_valid={} head_valid={} head_window={} tail_window={} correction_samples={} correction_ms={:.3} source_trim_samples={} envelope=linear-openutau",
                            previous_lyric.as_deref().unwrap_or("<none>"),
                            result.lyric,
                            result.adjacent,
                            tail.is_some(),
                            head.is_some(),
                            head.map_or_else(|| "none".to_string(), |signature| format!(
                                "{}+{}@{:.3}Hz",
                                signature.window_start, signature.window_len, signature.frequency
                            )),
                            tail.map_or_else(|| "none".to_string(), |signature| format!(
                                "{}+{}@{:.3}Hz",
                                signature.window_start, signature.window_len, signature.frequency
                            )),
                            mixed.correction_samples,
                            mixed.correction_samples as f64 * 1000.0 / sample_rate.max(1) as f64,
                            mixed.source_trim_samples,
                        ),
                    );
                    previous_phase_correction = correction;
                }
                WavtoolMixStrategy::LegacyPhaseAligned => {
                    let crossfade_samples =
                        ((result.crossfade_ms / 1000.0) * sample_rate as f64).round() as usize;
                    previous_phone_end_sample = Self::mix_phase_aligned(
                        &mut track_buffer,
                        audible_samples,
                        start_sample_idx,
                        previous_phone_end_sample,
                        crossfade_samples,
                        result.pitch_freq,
                        sample_rate,
                    );
                }
                WavtoolMixStrategy::Plain => {
                    previous_phone_end_sample = Self::mix_with_resolved_correction(
                        &mut track_buffer,
                        audible_samples,
                        start_sample_idx,
                        previous_phone_end_sample,
                        0,
                    )
                    .end_sample;
                }
            }
            previous_phase_tail = result.phase_metadata.tail;
            previous_lyric = Some(result.lyric);
        }

        if let Some(target_peak) = crate::renderer::project::configured_render_limiter_peak() {
            Self::apply_soft_limiter(&mut track_buffer, target_peak);
        }

        let diagnostics = crate::renderer::diagnostics::AudioDiagnostics::analyze(&track_buffer, 1);
        log(0.95, &diagnostics.render_log_line("Final track_buffer"));

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

        let final_diagnostics =
            crate::renderer::diagnostics::AudioDiagnostics::analyze(&track_buffer, 1);
        log(
            1.0,
            &format!(
                "[Render][Complete] output_samples={} output_duration_ms={:.3} {}",
                track_buffer.len(),
                track_buffer.len() as f64 * 1000.0 / sample_rate.max(1) as f64,
                final_diagnostics.render_log_line("final output"),
            ),
        );

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
