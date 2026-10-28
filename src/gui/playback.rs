#[cfg(not(target_arch = "wasm32"))]
use crate::audio::ProgressivePlaybackHandle;
use crate::gui::piano_roll::PianoRollState;
use crate::gui::KamafeuStudioApp;
use crate::oto::Voicebank;
#[cfg(not(target_arch = "wasm32"))]
use crate::renderer::ProgressiveChunk;
use crate::renderer::ProjectRenderer;
use eframe::egui;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use web_time::Instant;

fn progressive_chunk_ms(lookahead_ms: f32, render_chunk_bars: u32, bpm: f64) -> f64 {
    let bpm = bpm.clamp(20.0, 600.0);
    let bars_ms = f64::from(render_chunk_bars.clamp(1, 16)) * 4.0 * 60_000.0 / bpm;
    // A short chunk is very easy to underrun while egui is laying out and
    // painting a large piano roll during a scroll.  Once the audio sink has
    // started, continuity is more important than shaving a few seconds from
    // the first audible block, so keep at least two seconds of lookahead.
    f64::from(lookahead_ms.clamp(2_000.0, 10_000.0))
        .min(bars_ms)
        // At very high BPM one bar can be shorter than two seconds; do not
        // stretch a chunk past the configured musical boundary in that case.
        .clamp(500.0, 10_000.0)
}

const PROGRESSIVE_STARTUP_BUFFER_MS: f64 = 3_000.0;

pub(super) fn progressive_startup_ready(buffered_ms: f64, current_is_final: bool) -> bool {
    // A single page can be consumed before the next page finishes, especially
    // when external resamplers are busy. Prime three seconds of contiguous
    // audio before opening the sink to avoid starting playback on a thin runway.
    buffered_ms >= PROGRESSIVE_STARTUP_BUFFER_MS || current_is_final
}

#[cfg(not(target_arch = "wasm32"))]
fn spawn_progressive_audio_bridge(
    audio_rx: std::sync::mpsc::Receiver<ProgressiveChunk>,
    ui_tx: std::sync::mpsc::Sender<ProgressiveChunk>,
    playback: Option<ProgressivePlaybackHandle>,
    metronome_enabled: bool,
    bpm: f64,
) {
    std::thread::Builder::new()
        .name("kamafeu-progressive-audio-bridge".to_string())
        .spawn(move || {
            let mut startup = Vec::new();
            let mut started = false;
            let mut final_chunk_received = false;
            for mut chunk in audio_rx {
                let _ = ui_tx.send(chunk.clone());
                if chunk.audio.error.is_some() {
                    // A failed render leaves a hole in the indexed timeline.
                    // Stop this generation so a later chunk cannot be sent as
                    // Start or Append with no accepted initial format.
                    break;
                }
                let final_chunk = chunk.is_final;
                final_chunk_received |= final_chunk;
                let chunk_end_ms = chunk.end_ms();
                let mut samples = std::mem::take(&mut chunk.audio.samples);
                if metronome_enabled {
                    crate::audio::apply_metronome_clicks(
                        &mut samples,
                        chunk.audio.sample_rate,
                        chunk.audio.channels,
                        chunk.chunk_start_ms,
                        bpm,
                    );
                }
                if !started {
                    startup.push((
                        chunk.chunk_index,
                        chunk.chunk_start_ms,
                        chunk_end_ms,
                        samples,
                        chunk.audio.sample_rate,
                        chunk.audio.channels,
                        final_chunk,
                    ));
                    let buffered_ms = startup
                        .iter()
                        .map(|(_, start_ms, end_ms, _, _, _, _)| (end_ms - start_ms).max(0.0))
                        .sum();
                    if !progressive_startup_ready(buffered_ms, final_chunk) {
                        continue;
                    }
                    let mut queued = startup.drain(..);
                    if let Some((index, start_ms, end_ms, first, rate, channels, is_final)) =
                        queued.next()
                    {
                        if let Some(playback) = &playback {
                            playback
                                .start(index, start_ms, end_ms, first, rate, channels, is_final);
                        }
                    }
                    for (index, start_ms, end_ms, samples, rate, channels, is_final) in queued {
                        if let Some(playback) = &playback {
                            playback
                                .append(index, start_ms, end_ms, samples, rate, channels, is_final);
                        }
                    }
                    started = true;
                } else {
                    if let Some(playback) = &playback {
                        playback.append(
                            chunk.chunk_index,
                            chunk.chunk_start_ms,
                            chunk_end_ms,
                            samples,
                            chunk.audio.sample_rate,
                            chunk.audio.channels,
                            final_chunk,
                        );
                    }
                }
            }
            if let Some(playback) = &playback {
                playback.producer_closed(final_chunk_received);
            }
        })
        .ok();
}

impl KamafeuStudioApp {
    pub(crate) fn transport_is_active(&self) -> bool {
        self.audio_player.is_playing()
            || self.render_rx.is_some()
            || self.direct_playback_rx.is_some()
    }

    pub(crate) fn retry_failed_chunk(&mut self) {
        #[cfg(target_arch = "wasm32")]
        {
            self.transport_state.status_message =
                "Retry de chunks não está disponível no alvo WASM".to_string();
            return;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            if self.render_rx.is_some() || self.direct_playback_rx.is_some() {
                self.transport_state.status_message =
                    "A prévia já está sendo renderizada".to_string();
                return;
            }
            let Some((chunk_index, start_ms, end_ms, is_final)) = self.failed_chunk else {
                return;
            };
            let (Some(project), Some(voicebank), Some(options)) = (
                self.retry_project.clone(),
                self.retry_voicebank.clone(),
                self.retry_options.clone(),
            ) else {
                self.transport_state.status_message =
                    "Contexto de retry indisponível; inicie uma nova prévia".to_string();
                return;
            };

            let (log_tx, log_rx) = std::sync::mpsc::channel();
            let (progress_tx, progress_rx) = std::sync::mpsc::channel();
            let (audio_tx, audio_rx) = std::sync::mpsc::sync_channel(16);
            let (ui_tx, ui_rx) = std::sync::mpsc::channel();
            if let Some(cancel) = self.preview_waveform_cancel.take() {
                cancel.store(true, Ordering::Relaxed);
            }
            self.preview_waveform_rx = None;
            let cancel = Arc::new(AtomicBool::new(false));
            self.render_log_channel_rx = Some(log_rx);
            self.render_progress_rx = Some(progress_rx);
            self.render_progress_event = Some(crate::renderer::RenderProgressEvent::stage(
                crate::renderer::RenderStage::Preparing,
            ));
            self.render_task_kind = Some(super::RenderTaskKind::Preview);
            self.render_progress_started_at = Some(Instant::now());
            self.render_progress_finished_at = None;
            self.render_rx = Some(ui_rx);
            self.render_cancel = Some(cancel.clone());
            self.render_progress = 0.0;
            self.render_status_title = format!("Retry chunk #{chunk_index}");
            self.failed_chunk = None;
            self.progressive_startup_audio.clear();
            self.playback_chunk_buffer.reset(start_ms);

            // Retry must use the same renderer -> bridge -> scheduler path as
            // normal playback.  Feeding the UI receiver directly used to
            // make retry depend on frame frequency and could recreate the
            // exact underrun that the retry was meant to fix.
            self.audio_player.begin_progressive_generation();
            spawn_progressive_audio_bridge(
                audio_rx,
                ui_tx,
                self.audio_player.progressive_handle(),
                self.transport_state.metronome_enabled,
                self.project.bpm,
            );

            let resampler_driver = self.create_resampler_driver();
            let wavtool_driver = self.create_wavtool_driver();
            let sample_rate = self.sample_rate;
            std::thread::spawn(move || {
                let report_progress = move |progress, message: &str| {
                    let _ = log_tx.send((progress, message.to_string()));
                };
                let report_render_progress = move |event| {
                    let _ = progress_tx.send(event);
                };
                let chunk = ProjectRenderer::retry_progressive_chunk_with_progress_events(
                    &project,
                    &voicebank,
                    sample_rate,
                    chunk_index,
                    start_ms,
                    end_ms,
                    is_final,
                    resampler_driver.as_ref(),
                    wavtool_driver.as_ref(),
                    &options,
                    Some(&report_progress),
                    Some(&report_render_progress),
                    Some(cancel.as_ref()),
                );
                let _ = audio_tx.send(chunk);
            });
        }
    }

    pub fn play_current_track(&mut self) {
        if let Err(error) = self.audio_player.prepare_for_playback() {
            self.transport_state.status_message = format!("Erro de áudio: {error}");
            self.render_log_messages.push(error);
            return;
        }

        if self.render_rx.is_some() || self.direct_playback_rx.is_some() {
            self.transport_state.status_message = "A prévia já está sendo renderizada".to_string();
            return;
        }

        if self.project.parts.iter().all(|part| part.notes.is_empty())
            && self.project.wave_parts.is_empty()
        {
            self.transport_state.status_message =
                "Nenhum áudio ou nota para reproduzir".to_string();
            return;
        }

        let resampler_driver = self.create_resampler_driver();
        let wavtool_driver = self.create_wavtool_driver();

        let sample_rate = self.sample_rate;

        let dummy_vb = Voicebank {
            root_path: PathBuf::from("."),
            name: "Synthetic Fallback".to_string(),
            author: "System".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries: std::collections::HashMap::new(),
            case_insensitive_entries: Default::default(),
            prefix_map: crate::oto::PrefixMap::default(),
            temp_dir: None,
        };

        let active_vb = self.voicebank.clone().unwrap_or(dummy_vb);
        let mut vocal_mode_params = self.vocal_mode_params.clone();
        vocal_mode_params.resampler_instances = self.config.dsp.resampler_instances.max(1);
        // Keep one logical CPU available for egui/CoreAudio.  Using every
        // core for synthesis makes scrolling the piano roll starve the UI
        // thread long enough to empty the audio queue.
        let cpu_count = std::thread::available_parallelism()
            .map(std::num::NonZeroUsize::get)
            .unwrap_or(2);
        let render_threads =
            (self.render_threads.clamp(1, 16) as usize).min(cpu_count.saturating_sub(1).max(1));
        let mut project = self.project.clone();
        project.bpm = self.transport_state.bpm;
        if self.transport_state.preview_selection_only {
            let selected = &self.piano_roll_state.selected_note_indices;
            if !selected.is_empty() {
                let active_part_idx = self
                    .project
                    .parts
                    .iter()
                    .position(|part| part.track_index == self.active_track_index)
                    .unwrap_or(0);
                for (part_idx, part) in project.parts.iter_mut().enumerate() {
                    if part_idx == active_part_idx {
                        part.notes = part
                            .notes
                            .iter()
                            .enumerate()
                            .filter(|(index, _)| selected.contains(index))
                            .map(|(_, note)| note.clone())
                            .collect();
                    } else {
                        part.notes.clear();
                    }
                }
            }
        }
        let mut max_project_end = project
            .parts
            .iter()
            .flat_map(|part| {
                part.notes
                    .iter()
                    .map(move |note| part.position_ms + note.position_ms + note.duration_ms)
            })
            .fold(0.0f64, f64::max);

        for wave in &project.wave_parts {
            let dur = if wave.duration_ms > 0.0 {
                wave.duration_ms
            } else {
                30_000.0
            };
            max_project_end = max_project_end.max(wave.position_ms + dur);
        }

        if max_project_end <= 0.0 {
            self.transport_state.status_message =
                "Nenhum áudio ou nota para reproduzir".to_string();
            return;
        }
        let mut playhead_ms = self.piano_roll_state.playhead_ms;
        let mut render_end_ms = max_project_end + 250.0;

        if self.transport_state.loop_enabled
            && self.transport_state.loop_end_ms > self.transport_state.loop_start_ms
        {
            if playhead_ms < self.transport_state.loop_start_ms
                || playhead_ms >= self.transport_state.loop_end_ms
            {
                playhead_ms = self.transport_state.loop_start_ms;
                self.piano_roll_state.playhead_ms = self.transport_state.loop_start_ms;
            }
            render_end_ms = self.transport_state.loop_end_ms;
        } else if playhead_ms >= max_project_end {
            playhead_ms = 0.0;
            self.piano_roll_state.playhead_ms = 0.0;
        }

        // A progressive render publishes absolute waveform buckets by
        // merging them into the current cache. When playback starts from the
        // beginning, that cache belongs to a previous render generation and
        // must not remain visible before the first note. Keep the cache for a
        // mid-project resume, where it still represents already rendered
        // audio, but reset it for a fresh timeline.
        if playhead_ms <= f64::EPSILON {
            self.piano_roll_state.rendered_waveform_peaks.clear();
            self.render_log_messages
                .push("[Waveform] cache reset: nova geração iniciada no tempo 0ms".to_string());
        }

        self.retry_project = Some(project.clone());
        self.retry_voicebank = Some(active_vb.clone());
        self.retry_options = Some(vocal_mode_params.clone());
        self.failed_chunk = None;
        self.project.render_state = Some(Default::default());

        self.render_log_window_open = false;
        self.render_progress = 0.0;
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.render_progress_rx = None;
        }
        self.render_progress_event = Some(crate::renderer::RenderProgressEvent::stage(
            crate::renderer::RenderStage::Preparing,
        ));
        self.render_task_kind = Some(super::RenderTaskKind::Preview);
        self.render_progress_started_at = Some(Instant::now());
        self.render_progress_finished_at = None;
        self.render_status_title = format!("{} • {:.0}ms", resampler_driver.name(), playhead_ms);
        self.transport_state.status_message = "Renderizando...".to_string();

        let (log_tx, log_rx) = std::sync::mpsc::channel();
        self.render_log_channel_rx = Some(log_rx);
        if let Some(cancel) = self.preview_waveform_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.preview_waveform_rx = None;
        self.direct_playback_active = false;
        let cancel = Arc::new(AtomicBool::new(false));
        self.render_cancel = Some(cancel.clone());
        self.playback_start_offset_ms = playhead_ms;
        self.piano_roll_state.horizontal_follow_user_override = false;
        self.piano_roll_state.vertical_follow_user_override = false;
        self.piano_roll_state.is_playing = false;
        self.playback_start_instant = None;
        self.playback_duration_ms = 0.0;
        self.progressive_playback_started = false;
        self.progressive_startup_audio.clear();
        self.playback_chunk_buffer.reset(playhead_ms);
        let log_tx = Arc::new(std::sync::Mutex::new(log_tx));
        let log_tx_cb = log_tx.clone();
        #[cfg(not(target_arch = "wasm32"))]
        {
            let (audio_tx, audio_rx) = std::sync::mpsc::sync_channel(16);
            let (ui_tx, ui_rx) = std::sync::mpsc::channel();
            self.render_rx = Some(ui_rx);
            self.direct_playback_rx = None;
            let chunk_ms = progressive_chunk_ms(
                self.config.dsp.render_lookahead_ms,
                self.config.dsp.render_chunk_bars,
                project.bpm,
            );
            self.audio_player.begin_progressive_generation();
            spawn_progressive_audio_bridge(
                audio_rx,
                ui_tx,
                self.audio_player.progressive_handle(),
                self.transport_state.metronome_enabled,
                project.bpm,
            );
            std::thread::spawn(move || {
                let report_progress = move |progress, message: &str| {
                    if let Ok(guard) = log_tx_cb.lock() {
                        let _ = guard.send((progress, message.to_string()));
                    }
                };
                let render = || {
                    ProjectRenderer::render_project_progressive(
                        &project,
                        &active_vb,
                        sample_rate,
                        playhead_ms,
                        render_end_ms,
                        chunk_ms,
                        resampler_driver.as_ref(),
                        wavtool_driver.as_ref(),
                        &vocal_mode_params,
                        Some(&report_progress),
                        Some(cancel.as_ref()),
                        &audio_tx,
                    );
                };
                let worker =
                    std::panic::AssertUnwindSafe(|| {
                        match crate::renderer::worker_pool::get(render_threads) {
                            Ok(pool) => pool.install(render),
                            Err(_) => render(),
                        }
                    });
                if std::panic::catch_unwind(worker).is_err() {
                    crate::terminal_log::error(
                        "Render",
                        "O worker do Catalina encerrou com panic durante a renderização progressiva",
                    );
                }
            });
        }

        #[cfg(target_arch = "wasm32")]
        {
            let (progress_tx, progress_rx) = std::sync::mpsc::channel();
            self.render_progress_rx = Some(progress_rx);
            let progress_tx_cb = progress_tx.clone();
            let (audio_tx, audio_rx) = std::sync::mpsc::channel();
            self.direct_playback_rx = Some(audio_rx);
            let report_progress = move |progress, message: &str| {
                if let Ok(guard) = log_tx_cb.lock() {
                    let _ = guard.send((progress, message.to_string()));
                }
            };
            let report_render_progress = move |event| {
                let _ = progress_tx_cb.send(event);
            };

            let rendered = ProjectRenderer::render_project_range_with_progress_events_cancellable(
                &project,
                &active_vb,
                sample_rate,
                playhead_ms,
                Some(render_end_ms),
                resampler_driver.as_ref(),
                wavtool_driver.as_ref(),
                &vocal_mode_params,
                Some(&report_progress),
                Some(&report_render_progress),
                Some(cancel.as_ref()),
            );
            let _ = audio_tx.send(rendered);
        }
    }

    pub fn pause_audio(&mut self) {
        let was_rendering = self.direct_playback_rx.is_some() || self.render_rx.is_some();
        if self.progressive_playback_started && !self.direct_playback_active {
            self.piano_roll_state.playhead_ms =
                self.playback_start_offset_ms + self.audio_player.scheduler_position_ms();
        }
        if let Some(cancel) = self.render_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        if was_rendering {
            self.render_progress_event = Some(crate::renderer::RenderProgressEvent::stage(
                crate::renderer::RenderStage::Cancelled,
            ));
            self.render_progress_finished_at = Some(Instant::now());
            self.render_progress_rx = None;
        }
        self.audio_player.stop();
        self.piano_roll_state.is_playing = false;
        self.playback_start_instant = None;
        self.render_rx = None;
        self.direct_playback_rx = None;
        self.direct_playback_active = false;
        self.progressive_playback_started = false;
        self.progressive_startup_audio.clear();
        self.playback_chunk_buffer
            .reset(self.piano_roll_state.playhead_ms);
        self.transport_state.status_message = "Pausado".to_string();
    }

    pub fn stop_audio(&mut self) {
        self.pause_audio();
        self.piano_roll_state.playhead_ms = 0.0;
        self.transport_state.status_message = "Parado".to_string();
    }

    pub fn preview_tone(&mut self, freq: f64) {
        let sample_rate = self.sample_rate.max(22050);
        let raw_samples = crate::dsp::piano::render_piano_preview(freq, sample_rate);
        if !raw_samples.is_empty() {
            self.audio_player.play_samples(raw_samples, sample_rate);
        }
    }

    pub fn compute_preview_waveform_hash(&self) -> u64 {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.active_track_index.hash(&mut hasher);
        self.sample_rate.hash(&mut hasher);
        if let Ok(options) = serde_json::to_vec(&self.vocal_mode_params) {
            options.hash(&mut hasher);
        }
        if let Some(track) = self.project.tracks.get(self.active_track_index) {
            // The piano-roll guide is a dry vocal reference. Mixer-only edits
            // (mute, solo, volume, pan and track FX) cannot change that signal,
            // so they must not invalidate an expensive native render.
            let mut reference_track = track.clone();
            reference_track.mute = false;
            reference_track.solo = false;
            reference_track.volume_db = 0.0;
            reference_track.pan = 0.0;
            reference_track.fx_rack = None;
            if let Ok(track_data) = serde_json::to_vec(&reference_track) {
                track_data.hash(&mut hasher);
            }
        }
        (self.project.bpm * 1000.0)
            .round()
            .to_bits()
            .hash(&mut hasher);
        if let Some(vb) = &self.voicebank {
            vb.name.hash(&mut hasher);
            vb.root_path.hash(&mut hasher);
            vb.prefix_map.selected_color().hash(&mut hasher);
            vb.entries.len().hash(&mut hasher);
        }
        for (p_idx, part) in self
            .project
            .parts
            .iter()
            .filter(|part| part.track_index == self.active_track_index)
            .enumerate()
        {
            p_idx.hash(&mut hasher);
            part.track_index.hash(&mut hasher);
            (part.position_ms * 10.0)
                .round()
                .to_bits()
                .hash(&mut hasher);
            for n in &part.notes {
                // Hash the complete sound-producing note model. The former
                // hand-picked list omitted volume, dynamics, envelopes,
                // vibrato volume-link, flags and per-phoneme overrides.
                if let Ok(note_data) = serde_json::to_vec(n) {
                    note_data.hash(&mut hasher);
                }
                n.lyric.hash(&mut hasher);
                n.timbre.hash(&mut hasher);
                n.voicebank_pitch.hash(&mut hasher);
                n.pitch.hash(&mut hasher);
                (n.position_ms * 10.0).round().to_bits().hash(&mut hasher);
                (n.duration_ms * 10.0).round().to_bits().hash(&mut hasher);
                n.expressions.pitch_delta.to_bits().hash(&mut hasher);
                n.expressions.consonant_velocity.to_bits().hash(&mut hasher);
                n.expressions.preutter_offset_ms.to_bits().hash(&mut hasher);
                n.expressions.overlap_offset_ms.to_bits().hash(&mut hasher);
                for d in &n.phoneme_durations_ms {
                    (d * 10.0).round().to_bits().hash(&mut hasher);
                }
            }
        }
        hasher.finish()
    }

    pub fn trigger_background_waveform_preview(&mut self, ctx: &egui::Context) {
        if self.transport_is_active() || self.export_rx.is_some() {
            return;
        }

        // Hashing the active part serializes its notes. Four checks per second
        // are sufficient for a debounced preview and keep idle editing cheap.
        const HASH_CHECK_INTERVAL: std::time::Duration = std::time::Duration::from_millis(250);
        let now = Instant::now();
        if let Some(last_check) = self.preview_waveform_last_hash_check {
            let remaining = HASH_CHECK_INTERVAL.saturating_sub(last_check.elapsed());
            if !remaining.is_zero() {
                ctx.request_repaint_after(remaining);
                return;
            }
        }
        self.preview_waveform_last_hash_check = Some(now);

        let has_notes = self
            .project
            .parts
            .iter()
            .any(|part| part.track_index == self.active_track_index && !part.notes.is_empty());
        if !has_notes {
            if !self.piano_roll_state.rendered_waveform_peaks.is_empty() {
                self.piano_roll_state.rendered_waveform_peaks.clear();
            }
            self.preview_waveform_cache_hash = 0;
            self.preview_waveform_pending = None;
            self.piano_roll_state.waveform_rendering = false;
            return;
        }

        // Let an active gesture settle before serializing the notes or
        // scheduling another WORLD render. Keep any valid in-flight result;
        // after release the hash decides whether it is still current.
        if ctx.input(|input| input.pointer.primary_down() || input.pointer.secondary_down()) {
            ctx.request_repaint_after(HASH_CHECK_INTERVAL);
            return;
        }

        let current_hash = self.compute_preview_waveform_hash();
        if current_hash == self.preview_waveform_cache_hash {
            self.preview_waveform_pending = None;
            return;
        }

        const PREVIEW_DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(250);
        match self.preview_waveform_pending {
            Some((pending_hash, started)) if pending_hash == current_hash => {
                let remaining = PREVIEW_DEBOUNCE.saturating_sub(started.elapsed());
                if !remaining.is_zero() {
                    ctx.request_repaint_after(remaining);
                    return;
                }
            }
            _ => {
                self.preview_waveform_pending = Some((current_hash, now));
                if let Some(cancel) = self.preview_waveform_cancel.take() {
                    cancel.store(true, Ordering::Relaxed);
                }
                self.preview_waveform_rx = None;
                ctx.request_repaint_after(PREVIEW_DEBOUNCE);
                return;
            }
        }

        self.preview_waveform_pending = None;

        // Keep the last valid waveform visible while the replacement is
        // rendering. Clearing here creates the same apparent "fixed after
        // pause and rerender" symptom as a broken progressive merge.
        self.preview_waveform_cache_hash = current_hash;
        self.piano_roll_state.waveform_rendering = true;
        let cancel = Arc::new(AtomicBool::new(false));
        self.preview_waveform_cancel = Some(cancel.clone());

        let (tx, rx) = std::sync::mpsc::channel();
        self.preview_waveform_rx = Some(rx);

        // The piano-roll waveform is vocal-only. Instrumental/audio clips are
        // owned by the arrangement and must never bleed into the note editor.
        let mut project = self.project.clone();
        project
            .parts
            .retain(|part| part.track_index == self.active_track_index);
        project.wave_parts.clear();
        // Master processing and mixer controls are intentionally absent from
        // this vocal-only reference render.
        project.master_fx = None;
        for (track_index, track) in project.tracks.iter_mut().enumerate() {
            track.mute = track_index != self.active_track_index;
            track.solo = false;
            track.volume_db = 0.0;
            track.pan = 0.0;
            // The lower piano-roll waveform is a stable vocal reference. FX
            // remain active for playback/export, but time-based effects can
            // add tails and dynamics processing can reshape every peak. Using
            // the dry track here prevents the visual guide from changing
            // shape merely because the mix treatment changed.
            if track_index == self.active_track_index {
                track.fx_rack = None;
            }
        }
        let active_vb = self.voicebank.clone().unwrap_or_else(|| Voicebank {
            root_path: PathBuf::from("."),
            name: "Synthetic Fallback".to_string(),
            author: "System".to_string(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries: std::collections::HashMap::new(),
            case_insensitive_entries: std::sync::OnceLock::new(),
            prefix_map: crate::oto::PrefixMap::new(),
            temp_dir: None,
        });

        let sample_rate = self.sample_rate;
        let resampler_driver = self.create_resampler_driver();
        let wavtool_driver = self.create_wavtool_driver();
        let mut vocal_mode_params = self.vocal_mode_params.clone();
        vocal_mode_params.resampler_instances = self.config.dsp.resampler_instances.max(1);
        let render_threads = self.render_threads.max(1) as usize;

        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            let render = || {
                ProjectRenderer::render_project_with_drivers_cancellable(
                    &project,
                    &active_vb,
                    sample_rate,
                    0.0,
                    resampler_driver.as_ref(),
                    wavtool_driver.as_ref(),
                    &vocal_mode_params,
                    None,
                    Some(cancel.as_ref()),
                )
            };

            let rendered_audio = match crate::renderer::worker_pool::get(render_threads) {
                Ok(pool) => pool.install(render),
                Err(_) => render(),
            };

            if cancel.load(Ordering::Relaxed) {
                return;
            }

            if !cancel.load(Ordering::Relaxed) {
                let mut temp_state = PianoRollState::default();
                if !rendered_audio.samples.is_empty() {
                temp_state.update_rendered_waveform(
                    &rendered_audio.samples,
                    rendered_audio.sample_rate,
                    rendered_audio.channels,
                    0.0,
                );
                }
                let _ = tx.send((current_hash, temp_state.rendered_waveform_peaks));
            }
        });

        #[cfg(target_arch = "wasm32")]
        {
            let rendered_audio = ProjectRenderer::render_project_with_drivers_cancellable(
                &project,
                &active_vb,
                sample_rate,
                0.0,
                resampler_driver.as_ref(),
                wavtool_driver.as_ref(),
                &vocal_mode_params,
                None,
                Some(cancel.as_ref()),
            );

            if !cancel.load(Ordering::Relaxed) {
                let mut temp_state = PianoRollState::default();
                if !rendered_audio.samples.is_empty() {
                    temp_state.update_rendered_waveform(
                        &rendered_audio.samples,
                        rendered_audio.sample_rate,
                        rendered_audio.channels,
                        0.0,
                    );
                }
                let _ = tx.send((current_hash, temp_state.rendered_waveform_peaks));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{progressive_chunk_ms, progressive_startup_ready};

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn failed_progressive_chunk_stops_the_generation() {
        use super::spawn_progressive_audio_bridge;
        use crate::renderer::{ProgressiveChunk, RenderedAudio};
        let (audio_tx, audio_rx) = std::sync::mpsc::channel();
        let (ui_tx, ui_rx) = std::sync::mpsc::channel();
        spawn_progressive_audio_bridge(audio_rx, ui_tx, None, false, 120.0);
        audio_tx
            .send(ProgressiveChunk {
                audio: RenderedAudio::failed(48_000, "WORLD failed".into()),
                chunk_start_ms: 0.0,
                chunk_index: 0,
                is_final: true,
            })
            .unwrap();
        let _ = audio_tx.send(ProgressiveChunk {
            audio: RenderedAudio {
                samples: vec![0.1; 96_000],
                sample_rate: 48_000,
                channels: 2,
                error: None,
            },
            chunk_start_ms: 1000.0,
            chunk_index: 1,
            is_final: true,
        });
        drop(audio_tx);
        assert!(ui_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap()
            .audio
            .error
            .is_some());
        assert!(ui_rx
            .recv_timeout(std::time::Duration::from_millis(100))
            .is_err());
    }

    #[test]
    fn progressive_chunk_respects_lookahead_and_musical_bar_limit() {
        assert_eq!(progressive_chunk_ms(2_000.0, 4, 120.0), 2_000.0);
        assert_eq!(progressive_chunk_ms(10_000.0, 1, 120.0), 2_000.0);
        assert_eq!(progressive_chunk_ms(500.0, 16, 120.0), 2_000.0);
    }

    #[test]
    fn progressive_playback_primes_three_seconds_but_starts_short_final_projects() {
        assert!(!progressive_startup_ready(2_000.0, false));
        assert!(progressive_startup_ready(3_000.0, false));
        assert!(progressive_startup_ready(1_000.0, true));
    }
}
