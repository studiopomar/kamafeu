use crate::gui::KamafeuStudioApp;
use eframe::egui;
use web_time::Instant;

fn poll_render_chunk(
    rx: &std::sync::mpsc::Receiver<crate::renderer::ProgressiveChunk>,
    completed: bool,
) -> Result<crate::renderer::ProgressiveChunk, std::sync::mpsc::TryRecvError> {
    // The producer normally closes immediately after its final chunk. Once
    // that chunk (or an error) is handled, do not interpret closure as failure.
    if completed {
        return Err(std::sync::mpsc::TryRecvError::Empty);
    }
    rx.try_recv()
}

fn scheduler_may_stop_transport(
    scheduler_finished: bool,
    scheduler_stopped: bool,
    direct_playback_active: bool,
) -> bool {
    (scheduler_finished || scheduler_stopped) && !direct_playback_active
}

fn playback_playhead_ms(
    start_offset_ms: f64,
    rendered_duration_ms: f64,
    player_position_ms: Option<f64>,
    fallback_elapsed_ms: f64,
) -> f64 {
    let elapsed_ms = player_position_ms.unwrap_or(fallback_elapsed_ms).max(0.0);
    let bounded_ms = if rendered_duration_ms > 0.0 {
        elapsed_ms.min(rendered_duration_ms)
    } else {
        elapsed_ms
    };
    start_offset_ms.max(0.0) + bounded_ms
}

fn progressive_playhead_ms(start_offset_ms: f64, scheduler_position_ms: f64) -> f64 {
    start_offset_ms.max(0.0) + scheduler_position_ms.max(0.0)
}

impl KamafeuStudioApp {
    pub(super) fn update_background_tasks(&mut self, ctx: &egui::Context) {
        // Progressive audio still crosses the UI boundary before reaching the
        // dedicated scheduler. Keep egui polling while rendering so an idle or
        // expensive piano-roll frame cannot starve the sink.
        if self.piano_roll_state.is_playing || self.audio_player.scheduler_is_active() {
            ctx.request_repaint();
        } else if self.render_rx.is_some() || self.direct_playback_rx.is_some() {
            if let Some(interval) = self.animation_policy().interval() {
                ctx.request_repaint_after(interval);
            } else {
                // Playback/render polling remains functional when visual
                // animations are disabled, but does not spin continuously.
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        let mut scheduler_finished = false;
        #[cfg(not(target_arch = "wasm32"))]
        let mut scheduler_stopped = false;
        #[cfg(not(target_arch = "wasm32"))]
        for event in self.audio_player.drain_scheduler_events() {
            let event_generation = match &event {
                crate::audio::PlaybackSchedulerEvent::PlaybackStarted { generation, .. }
                | crate::audio::PlaybackSchedulerEvent::ChunkAccepted { generation, .. }
                | crate::audio::PlaybackSchedulerEvent::ChunkRejected { generation, .. }
                | crate::audio::PlaybackSchedulerEvent::WaitingForChunk { generation, .. }
                | crate::audio::PlaybackSchedulerEvent::PlaybackResumed { generation, .. }
                | crate::audio::PlaybackSchedulerEvent::Underrun { generation, .. }
                | crate::audio::PlaybackSchedulerEvent::PlaybackFinished { generation, .. }
                | crate::audio::PlaybackSchedulerEvent::PlaybackStopped { generation, .. }
                | crate::audio::PlaybackSchedulerEvent::ProducerClosed { generation, .. } => {
                    *generation
                }
            };
            if event_generation != self.audio_player.current_playback_generation() {
                // A stop/finish from the previous Play request may still be
                // queued when the next request begins. It must not cancel the
                // new transport or overwrite its diagnostics.
                continue;
            }
            let render_state = self
                .project
                .render_state
                .get_or_insert_with(Default::default);
            let (generation, state, played_ms) = match &event {
                crate::audio::PlaybackSchedulerEvent::PlaybackStarted { generation, .. } => {
                    self.piano_roll_state.is_playing = true;
                    self.progressive_playback_started = true;
                    self.playback_start_instant = Some(Instant::now());
                    (*generation, "running", 0.0)
                }
                crate::audio::PlaybackSchedulerEvent::ChunkAccepted { generation, .. } => (
                    *generation,
                    "running",
                    self.audio_player.scheduler_position_ms(),
                ),
                crate::audio::PlaybackSchedulerEvent::ChunkRejected { generation, .. } => (
                    *generation,
                    "invalid",
                    self.audio_player.scheduler_position_ms(),
                ),
                crate::audio::PlaybackSchedulerEvent::WaitingForChunk {
                    generation,
                    played_ms,
                } => (*generation, "waiting", *played_ms),
                crate::audio::PlaybackSchedulerEvent::PlaybackResumed {
                    generation,
                    played_ms,
                } => (*generation, "running", *played_ms),
                crate::audio::PlaybackSchedulerEvent::Underrun {
                    generation,
                    played_ms,
                } => (*generation, "underrun", *played_ms),
                crate::audio::PlaybackSchedulerEvent::PlaybackFinished {
                    generation,
                    played_ms,
                } => {
                    scheduler_finished = true;
                    (*generation, "finished", *played_ms)
                }
                crate::audio::PlaybackSchedulerEvent::PlaybackStopped {
                    generation,
                    played_ms,
                } => {
                    scheduler_stopped = true;
                    (*generation, "stopped", *played_ms)
                }
                crate::audio::PlaybackSchedulerEvent::ProducerClosed {
                    generation,
                    final_chunk_received,
                    played_ms,
                } => {
                    if !final_chunk_received {
                        scheduler_stopped = true;
                    }
                    (*generation, "producer_closed", *played_ms)
                }
            };
            render_state.scheduler_generation = generation;
            render_state.scheduler_state = state.to_string();
            render_state.scheduler_played_ms = played_ms;
            render_state.scheduler_queued_until_ms = self.audio_player.scheduler_queued_until_ms();
            render_state.scheduler_last_event = format!("{event:?}");
            self.render_log_messages.push(format!(
                "[AudioScheduler] {event:?} played_ms={:.1} queued_until_ms={:.1} runway_ms={:.1}",
                self.audio_player.scheduler_position_ms(),
                self.audio_player.scheduler_queued_until_ms(),
                (self.audio_player.scheduler_queued_until_ms()
                    - self.audio_player.scheduler_position_ms())
                .max(0.0)
            ));
            if !self.direct_playback_active {
                self.transport_state.status_message = match &event {
                    crate::audio::PlaybackSchedulerEvent::PlaybackStarted { start_ms, .. } => {
                        format!("Áudio iniciado em {:.0} ms", start_ms)
                    }
                    crate::audio::PlaybackSchedulerEvent::ChunkAccepted {
                        index,
                        start_ms,
                        end_ms,
                        ..
                    } => format!(
                        "Chunk de áudio aceito #{index}: {:.0}–{:.0} ms",
                        start_ms, end_ms
                    ),
                    crate::audio::PlaybackSchedulerEvent::ChunkRejected { reason, .. } => {
                        format!("Áudio rejeitado pelo dispositivo: {reason}")
                    }
                    crate::audio::PlaybackSchedulerEvent::PlaybackFinished {
                        played_ms, ..
                    } => format!("Áudio finalizado em {:.0} ms", played_ms),
                    crate::audio::PlaybackSchedulerEvent::PlaybackStopped { played_ms, .. } => {
                        format!("Áudio parado em {:.0} ms", played_ms)
                    }
                    crate::audio::PlaybackSchedulerEvent::Underrun { played_ms, .. } => {
                        format!("Dispositivo de áudio sem dados em {:.0} ms", played_ms)
                    }
                    _ => self.transport_state.status_message.clone(),
                };
            }
            match &event {
                crate::audio::PlaybackSchedulerEvent::ChunkRejected { reason, .. } => {
                    self.transport_state.status_message =
                        format!("Áudio rejeitado pelo dispositivo: {reason}");
                }
                crate::audio::PlaybackSchedulerEvent::PlaybackStopped { played_ms, .. }
                    if *played_ms < 50.0 && self.render_rx.is_some() =>
                {
                    self.transport_state.status_message =
                        "O dispositivo de áudio parou o primeiro bloco antes de reproduzi-lo."
                            .to_string();
                }
                _ => {}
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        if scheduler_may_stop_transport(
            scheduler_finished,
            scheduler_stopped,
            self.direct_playback_active,
        ) {
            // The scheduler is the authority for progressive playback. Do
            // not leave the UI transport marked as playing after the sink
            // reached the final timestamp or the producer failed.
            self.piano_roll_state.is_playing = false;
            self.progressive_playback_started = false;
            self.playback_start_instant = None;
        }
        let mut progress_events = Vec::new();
        if let Some(ref rx) = self.render_progress_rx {
            for _ in 0..64 {
                match rx.try_recv() {
                    Ok(event) => progress_events.push(event),
                    Err(_) => break,
                }
            }
        }
        for event in progress_events {
            self.render_progress = event.fraction.unwrap_or(0.0);
            self.transport_state.render_progress = self.render_progress;
            if self.export_in_progress {
                self.export_progress = self.render_progress;
            }
            self.render_progress_event = Some(event);
            self.render_status_title = format!("{:?}", event.stage);
            if matches!(
                event.stage,
                crate::renderer::RenderStage::Complete
                    | crate::renderer::RenderStage::Cancelled
                    | crate::renderer::RenderStage::Failed
            ) {
                self.render_progress_finished_at = Some(Instant::now());
            }
        }
        if self
            .render_progress_finished_at
            .is_some_and(|finished| finished.elapsed() >= std::time::Duration::from_secs(3))
        {
            self.render_progress_event = None;
            self.render_task_kind = None;
            self.render_progress_started_at = None;
            self.render_progress_finished_at = None;
            self.render_progress_rx = None;
        }

        if let Some(ref rx) = self.render_log_channel_rx {
            // Keep UI frames responsive even if a renderer emits a burst of logs.
            for _ in 0..64 {
                let Ok((_prog, msg)) = rx.try_recv() else {
                    break;
                };
                if self.export_in_progress {
                    self.export_status_detail = msg.clone();
                } else if self.render_rx.is_some() {
                    self.transport_state.status_message = msg.clone();
                }
                self.render_log_messages.push(msg);
                if self.render_log_messages.len() > 1000 {
                    let excess = self.render_log_messages.len() - 1000;
                    self.render_log_messages.drain(..excess);
                }
            }
        }

        if let Some(ref rx) = self.export_rx {
            if let Ok(result) = rx.try_recv() {
                self.export_in_progress = false;
                self.export_progress = 1.0;
                let terminal_stage = if result.is_ok() {
                    crate::renderer::RenderStage::Complete
                } else if result
                    .as_ref()
                    .err()
                    .is_some_and(|error| error.to_ascii_lowercase().contains("cancel"))
                {
                    crate::renderer::RenderStage::Cancelled
                } else {
                    crate::renderer::RenderStage::Failed
                };
                self.render_progress_event =
                    Some(crate::renderer::RenderProgressEvent::stage(terminal_stage));
                self.render_progress_finished_at = Some(Instant::now());
                let fmt_name = self.export_audio_format.display_name();
                self.transport_state.status_message = match &result {
                    Ok(()) => format!("Exportação ({fmt_name}) concluída com sucesso!"),
                    Err(error) if error.to_ascii_lowercase().contains("cancel") => {
                        format!(
                            "Exportação ({fmt_name}) cancelada; nenhum resultado foi concluído."
                        )
                    }
                    Err(error) => format!("Erro na exportação ({fmt_name}): {error}"),
                };
                if let Ok(()) = result {
                    if let Some(ref path) = self.export_save_path {
                        let fname = path
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        self.last_exported_notification = Some((
                            path.clone(),
                            format!("Áudio ({fmt_name}) exportado: {fname}"),
                            Instant::now(),
                        ));
                    }
                }
                self.export_result = Some(result);
                self.export_rx = None;
                self.export_cancel = None;
            }
        }

        if let Some(ref rx) = self.direct_playback_rx {
            match rx.try_recv() {
                Ok(mut audio) => {
                    self.direct_playback_rx = None;
                    self.render_cancel = None;
                    self.render_progress = 1.0;
                    self.transport_state.render_progress = 1.0;
                    if let Some(error) = audio.error.take() {
                        self.audio_player.stop();
                        self.piano_roll_state.is_playing = false;
                        self.transport_state.status_message = format!("Erro de render: {error}");
                        self.render_status_title = "Erro de áudio".to_string();
                        self.render_progress_event =
                            Some(crate::renderer::RenderProgressEvent::stage(
                                crate::renderer::RenderStage::Failed,
                            ));
                        self.render_progress_finished_at = Some(Instant::now());
                    } else if audio.samples.is_empty()
                        || audio.sample_rate == 0
                        || audio.channels == 0
                        || audio.samples.len() % audio.channels as usize != 0
                    {
                        self.audio_player.stop();
                        self.piano_roll_state.is_playing = false;
                        self.transport_state.status_message =
                            "Erro de áudio: render vazio ou com formato inválido.".to_string();
                        self.render_status_title = "Erro de áudio".to_string();
                        self.render_progress_event =
                            Some(crate::renderer::RenderProgressEvent::stage(
                                crate::renderer::RenderStage::Failed,
                            ));
                        self.render_progress_finished_at = Some(Instant::now());
                    } else {
                        let waveform_samples = audio.samples.clone();
                        if !self
                            .project
                            .tracks
                            .get(self.active_track_index)
                            .and_then(|track| track.fx_rack.as_ref())
                            .is_some_and(|fx| fx.master_enabled)
                        {
                            self.piano_roll_state.merge_rendered_waveform(
                                &waveform_samples,
                                audio.sample_rate,
                                audio.channels,
                                self.playback_start_offset_ms,
                            );
                        }
                        if self.transport_state.metronome_enabled {
                            crate::audio::apply_metronome_clicks(
                                &mut audio.samples,
                                audio.sample_rate,
                                audio.channels,
                                self.playback_start_offset_ms,
                                self.transport_state.bpm,
                            );
                        }
                        self.playback_duration_ms =
                            audio.frame_count() as f64 * 1000.0 / audio.sample_rate.max(1) as f64;
                        match self.audio_player.try_play_samples_with_channels(
                            audio.samples,
                            audio.sample_rate,
                            audio.channels,
                        ) {
                            Ok(()) => {
                                self.direct_playback_active = true;
                                self.piano_roll_state.is_playing = true;
                                self.playback_start_instant = Some(Instant::now());
                                self.progressive_playback_started = false;
                                self.transport_state.status_message =
                                    "Reproduzindo áudio...".to_string();
                                self.render_status_title = "Reproduzindo...".to_string();
                            }
                            Err(error) => {
                                self.audio_player.stop();
                                self.piano_roll_state.is_playing = false;
                                self.transport_state.status_message =
                                    format!("Erro de áudio: {error}");
                                self.render_status_title = "Dispositivo indisponível".to_string();
                            }
                        }
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.direct_playback_rx = None;
                    self.render_cancel = None;
                    self.audio_player.stop();
                    self.piano_roll_state.is_playing = false;
                    self.transport_state.status_message =
                        "Erro de áudio: o renderizador encerrou sem resultado.".to_string();
                    self.render_status_title = "Erro de áudio".to_string();
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }

        let mut drop_rx = false;
        if let Some(ref rx) = self.render_rx {
            // Drain the complete ready batch.  Limiting this to two chunks let
            // a busy piano-roll frame leave the sink without the next block.
            // Drain the whole ready window in one frame. The producer may
            // already have rendered ahead while egui was busy scrolling.
            for _ in 0..64 {
                match poll_render_chunk(rx, drop_rx) {
                    Ok(chunk) => {
                        #[cfg(not(target_arch = "wasm32"))]
                        if let Some(delay_ms) = std::env::var("KAMAFEU_DEBUG_UI_CHUNK_DELAY_MS")
                            .ok()
                            .and_then(|value| value.parse::<u64>().ok())
                        {
                            std::thread::sleep(std::time::Duration::from_millis(delay_ms));
                        }
                        let ready_chunks = if chunk.audio.error.is_some() {
                            vec![chunk]
                        } else {
                            let before = self.playback_chunk_buffer.pending_len();
                            let before_counters = (
                                self.playback_chunk_buffer.gaps,
                                self.playback_chunk_buffer.out_of_order,
                                self.playback_chunk_buffer.duplicates,
                                self.playback_chunk_buffer.format_mismatches,
                                self.playback_chunk_buffer.timing_mismatches,
                            );
                            let ready = self.playback_chunk_buffer.push(chunk);
                            let counters = (
                                self.playback_chunk_buffer.gaps,
                                self.playback_chunk_buffer.out_of_order,
                                self.playback_chunk_buffer.duplicates,
                                self.playback_chunk_buffer.format_mismatches,
                                self.playback_chunk_buffer.timing_mismatches,
                            );
                            if counters != before_counters
                                || (ready.is_empty()
                                    && self.playback_chunk_buffer.pending_len() > before)
                            {
                                self.render_log_messages.push(format!(
                                    "[PlaybackBuffer] pending={} gaps={} out_of_order={} duplicates={} format_mismatches={} timing_mismatches={}",
                                    self.playback_chunk_buffer.pending_len(),
                                    self.playback_chunk_buffer.gaps,
                                    self.playback_chunk_buffer.out_of_order,
                                    self.playback_chunk_buffer.duplicates,
                                    self.playback_chunk_buffer.format_mismatches,
                                    self.playback_chunk_buffer.timing_mismatches,
                                ));
                            }
                            ready
                        };
                        for mut chunk in ready_chunks {
                            let chunk_frames = chunk.audio.frame_count();
                            self.render_log_messages.push(format!(
                                "[Chunk] #{} start={:.0}ms end={:.0}ms frames={}{}",
                                chunk.chunk_index,
                                chunk.chunk_start_ms,
                                chunk.end_ms(),
                                chunk_frames,
                                if chunk.audio.error.is_some() {
                                    " status=error"
                                } else {
                                    " status=ready"
                                }
                            ));
                            let render_state = self
                                .project
                                .render_state
                                .get_or_insert_with(Default::default);
                            render_state
                                .chunks
                                .retain(|stored| stored.index != chunk.chunk_index);
                            render_state
                                .chunks
                                .push(crate::project::model::UProjectRenderChunk {
                                    index: chunk.chunk_index,
                                    start_ms: chunk.chunk_start_ms,
                                    end_ms: chunk.end_ms(),
                                    is_final: chunk.is_final,
                                    status: if chunk.audio.error.is_some() {
                                        "failed"
                                    } else {
                                        "ready"
                                    }
                                    .to_string(),
                                    error: chunk.audio.error.clone(),
                                });
                            render_state.chunks.sort_by_key(|stored| stored.index);
                            if let Some(error) = chunk.audio.error.take() {
                                self.failed_chunk = Some((
                                    chunk.chunk_index,
                                    chunk.chunk_start_ms,
                                    chunk.end_ms(),
                                    chunk.is_final,
                                ));
                                self.audio_player.stop();
                                self.piano_roll_state.is_playing = false;
                                self.progressive_playback_started = false;
                                self.progressive_startup_audio.clear();
                                self.transport_state.status_message =
                                    format!("Erro no render: {error}");
                                self.render_log_messages
                                    .push(format!("Chunk #{}: {error}", chunk.chunk_index));
                                drop_rx = true;
                                break;
                            }

                            let chunk_is_final = chunk.is_final;
                            // The waveform represents the renderer output,
                            // never transport-only audio such as metronome
                            // clicks. Keep this copy untouched; the mutable
                            // buffer below is exclusively for playback.
                            let waveform_samples = chunk.audio.samples.clone();

                            let active_track_has_fx = self
                                .project
                                .tracks
                                .get(self.active_track_index)
                                .and_then(|track| track.fx_rack.as_ref())
                                .is_some_and(|fx| fx.master_enabled);
                            if !active_track_has_fx {
                                self.piano_roll_state.merge_rendered_waveform(
                                    &waveform_samples,
                                    chunk.audio.sample_rate,
                                    chunk.audio.channels,
                                    chunk.chunk_start_ms,
                                );
                            }

                            // The dedicated audio bridge already sends the
                            // playback copy to the scheduler. The UI only
                            // merges the untouched renderer output into the
                            // waveform and observes scheduler state.
                            if !self.progressive_playback_started
                                && self.audio_player.scheduler_is_active()
                            {
                                self.piano_roll_state.is_playing = true;
                                self.playback_start_instant = Some(Instant::now());
                                self.progressive_playback_started = true;
                                self.transport_state.status_message =
                                    "Reproduzindo áudio...".to_string();
                            }

                            // The scheduler can consume a short first chunk
                            // and move to Waiting/Underrun before this UI
                            // frame observes it. Keep the transport alive for
                            // every accepted generation instead of leaving a
                            // rendered project visually frozen at its old
                            // playhead position.

                            if chunk_is_final {
                                self.transport_state.render_progress = 1.0;
                                self.render_progress = 1.0;
                                drop_rx = true;
                                break;
                            }
                        }
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                        // A disconnected worker is never a successful
                        // completion: even if earlier chunks reached the
                        // sink, the remaining timeline is invalid. Stop the
                        // sink and playhead instead of letting the piano roll
                        // continue past a partial render.
                        self.audio_player.stop();
                        self.piano_roll_state.is_playing = false;
                        self.progressive_playback_started = false;
                        self.progressive_startup_audio.clear();
                        self.playback_start_instant = None;
                        let error =
                            "O worker de renderização terminou antes do chunk final. Verifique o engine e o log de renderização.";
                        self.transport_state.status_message = error.to_string();
                        self.render_log_messages.push(error.to_string());
                        drop_rx = true;
                        break;
                    }
                }
            }
        }
        if drop_rx {
            self.render_rx = None;
            self.render_cancel = None;
        }

        if self.piano_roll_state.is_playing {
            if let Some(_start_t) = self.playback_start_instant {
                let fallback_elapsed_ms =
                    _start_t.elapsed().as_secs_f64() * 1000.0 * self.playback_speed_rate;
                let player_position_ms = if self.direct_playback_active {
                    self.audio_player.playback_position_ms()
                } else {
                    None
                };
                let playhead_ms = if self.direct_playback_active {
                    playback_playhead_ms(
                        self.playback_start_offset_ms,
                        self.playback_duration_ms,
                        player_position_ms,
                        fallback_elapsed_ms,
                    )
                } else if self.progressive_playback_started {
                    progressive_playhead_ms(
                        self.playback_start_offset_ms,
                        self.audio_player.scheduler_position_ms(),
                    )
                } else {
                    self.playback_start_offset_ms + fallback_elapsed_ms
                };
                let elapsed_ms = (playhead_ms - self.playback_start_offset_ms).max(0.0);
                self.piano_roll_state.playhead_ms = playhead_ms;

                // Finding the final note is O(all notes). It is only needed
                // after the audio sink has stopped, not on every live playback
                // frame where the sink is still advancing normally.
                if self.direct_playback_active && !self.audio_player.is_playing() {
                    self.piano_roll_state.playhead_ms =
                        self.playback_start_offset_ms + self.playback_duration_ms;
                    self.direct_playback_active = false;
                    self.piano_roll_state.is_playing = false;
                    self.playback_start_instant = None;
                    self.transport_state.status_message = "Reprodução concluída".to_string();
                } else if elapsed_ms > 1000.0 && !self.audio_player.is_playing() {
                    let max_end_ms = self
                        .project
                        .parts
                        .iter()
                        .flat_map(|part| {
                            part.notes.iter().map(move |note| {
                                part.position_ms + note.position_ms + note.duration_ms
                            })
                        })
                        .fold(0.0f64, f64::max);
                    if self.piano_roll_state.playhead_ms > max_end_ms + 1000.0 {
                        self.pause_audio();
                    }
                }

                if self.transport_state.loop_enabled
                    && self.transport_state.loop_end_ms > self.transport_state.loop_start_ms
                    && self.piano_roll_state.playhead_ms >= self.transport_state.loop_end_ms
                {
                    self.pause_audio();
                    self.piano_roll_state.playhead_ms = self.transport_state.loop_start_ms;
                    self.playback_start_offset_ms = self.transport_state.loop_start_ms;
                    self.play_current_track();
                }
            }

            // Real audio VU meter ballistics calculated from synthesized waveform peaks
            let cur_time = self.piano_roll_state.playhead_ms as f32;
            let amp = self
                .piano_roll_state
                .waveform_amplitude_at(cur_time)
                .unwrap_or(0.0);
            let any_solo = self.project.tracks.iter().any(|track| track.solo);
            let active_track = self.project.tracks.get(self.active_track_index);
            let audible = active_track.is_some_and(|track| {
                !track.mute && (!any_solo || track.solo)
            });
            let (track_gain, track_pan) = active_track.map_or((0.0, 0.0), |track| {
                (10.0f32.powf(track.volume_db as f32 / 20.0), track.pan as f32)
            });
            let metered_amp = if audible { amp * track_gain } else { 0.0 };
            let target_l = (metered_amp * (1.0 - track_pan.clamp(0.0, 1.0))).clamp(0.0, 1.0);
            let target_r = (metered_amp * (1.0 + track_pan.clamp(-1.0, 0.0))).clamp(0.0, 1.0);
            self.piano_roll_state.update_arrangement_meter(
                self.project.tracks.len(),
                self.active_track_index,
                metered_amp,
            );
            self.transport_state.vu_level_l =
                self.transport_state.vu_level_l * 0.65 + target_l * 0.35;
            self.transport_state.vu_level_r =
                self.transport_state.vu_level_r * 0.65 + target_r * 0.35;
            self.transport_state.vu_peak_l = self
                .transport_state
                .vu_peak_l
                .max(self.transport_state.vu_level_l);
            self.transport_state.vu_peak_r = self
                .transport_state
                .vu_peak_r
                .max(self.transport_state.vu_level_r);
            self.transport_state.vu_peak_l =
                (self.transport_state.vu_peak_l - 0.015).max(self.transport_state.vu_level_l);
            self.transport_state.vu_peak_r =
                (self.transport_state.vu_peak_r - 0.015).max(self.transport_state.vu_level_r);

            // Live scope: envelope of the last 1.6 s of rendered audio.
            const SCOPE_POINTS: usize = 64;
            const SCOPE_WINDOW_MS: f32 = 1_600.0;
            let gain = if audible { track_gain } else { 0.0 };
            let step = SCOPE_WINDOW_MS / SCOPE_POINTS as f32;
            let start = cur_time - SCOPE_WINDOW_MS;
            self.transport_state.scope.clear();
            self.transport_state.scope.extend((0..SCOPE_POINTS).map(|i| {
                let t0 = start + i as f32 * step;
                self.piano_roll_state
                    .waveform_min_max_between(t0, t0 + step)
                    .map(|(min, max)| (min * gain, max * gain))
                    .unwrap_or((0.0, 0.0))
            }));

            ctx.request_repaint();
        } else {
            self.transport_state.vu_level_l = (self.transport_state.vu_level_l - 0.08).max(0.0);
            self.transport_state.vu_level_r = (self.transport_state.vu_level_r - 0.08).max(0.0);
            self.transport_state.vu_peak_l = (self.transport_state.vu_peak_l - 0.02).max(0.0);
            self.transport_state.vu_peak_r = (self.transport_state.vu_peak_r - 0.02).max(0.0);
            let mut scope_alive = false;
            for (min, max) in &mut self.transport_state.scope {
                *min *= 0.82;
                *max *= 0.82;
                scope_alive |= min.abs() > 0.002 || max.abs() > 0.002;
            }
            if !scope_alive {
                self.transport_state.scope.clear();
            }
            if scope_alive
                || self.transport_state.vu_level_l > 0.0
                || self.transport_state.vu_peak_l > 0.0
                || self.transport_state.vu_peak_r > 0.0
            {
                // Let the meters fall back smoothly instead of freezing on the
                // last level until the next input event.
                ctx.request_repaint_after(std::time::Duration::from_millis(16));
            }
            self.piano_roll_state
                .decay_arrangement_meters(self.project.tracks.len());

            if self.piano_roll_state.is_scrubbing_ruler {
                ctx.request_repaint();
            }
        }

        if self.render_rx.is_some()
            || self.export_rx.is_some()
            || self.preview_waveform_rx.is_some()
        {
            ctx.request_repaint();
        }

        // Receive pre-rendered background waveform preview
        if let Some(ref rx) = self.preview_waveform_rx {
            if let Ok((hash, peaks)) = rx.try_recv() {
                if hash == self.preview_waveform_cache_hash
                    && !self.piano_roll_state.is_playing
                    && self.render_rx.is_none()
                {
                    self.piano_roll_state.rendered_waveform_peaks = peaks;
                }
                self.preview_waveform_rx = None;
                self.preview_waveform_cancel = None;
                self.piano_roll_state.waveform_rendering = false;
                ctx.request_repaint();
            } else if matches!(rx.try_recv(), Err(std::sync::mpsc::TryRecvError::Disconnected)) {
                // A canceled or failed background render must be retryable. The
                // sender intentionally drops without a payload on render errors.
                self.preview_waveform_rx = None;
                self.preview_waveform_cancel = None;
                self.piano_roll_state.waveform_rendering = false;
                self.preview_waveform_cache_hash = 0;
                self.preview_waveform_pending = None;
                ctx.request_repaint();
            }
        }

        // Keep the waveform fresh while editing only when the user opted in.
        // The preview method is hash-gated, so this inexpensive call does not
        // start a new render on every UI frame.
        #[cfg(not(target_arch = "wasm32"))]
        if self.config.dsp.background_prerender {
            self.trigger_background_waveform_preview(ctx);
        } else {
            if let Some(cancel) = self.preview_waveform_cancel.take() {
                cancel.store(true, std::sync::atomic::Ordering::Relaxed);
                self.preview_waveform_rx = None;
            }
            self.piano_roll_state.waveform_rendering = false;
            self.preview_waveform_pending = None;
        }

        let cur_ms = self.piano_roll_state.playhead_ms.max(0.0);
        let total_sec = (cur_ms / 1000.0) as u32;
        let mins = total_sec / 60;
        let secs = total_sec % 60;
        let ms_rem = (cur_ms % 1000.0) as u32;
        self.transport_state.playhead_time_str = format!("{:02}:{:02}.{:03}", mins, secs, ms_rem);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        playback_playhead_ms, poll_render_chunk, progressive_playhead_ms,
        scheduler_may_stop_transport,
    };

    #[test]
    fn completed_render_does_not_turn_producer_closure_into_a_stop() {
        use crate::renderer::{ProgressiveChunk, RenderedAudio};
        use std::sync::mpsc::{channel, TryRecvError};

        for final_chunk in [false, true] {
            let (tx, rx) = channel();
            tx.send(ProgressiveChunk {
                audio: RenderedAudio {
                    samples: vec![0.25; 4410],
                    sample_rate: 44100,
                    channels: 1,
                    error: None,
                },
                chunk_start_ms: 0.0,
                chunk_index: 0,
                is_final: final_chunk,
            })
            .unwrap();
            drop(tx);

            let chunk = poll_render_chunk(&rx, false).unwrap();
            let result = poll_render_chunk(&rx, chunk.is_final);
            if final_chunk {
                assert!(matches!(result, Err(TryRecvError::Empty)));
            } else {
                // A producer that really failed before the final chunk must
                // still reach the transport's disconnected-worker handler.
                assert!(matches!(result, Err(TryRecvError::Disconnected)));
            }
        }
    }

    #[test]
    fn stale_scheduler_stop_never_freezes_direct_playback() {
        assert!(!scheduler_may_stop_transport(false, true, true));
        assert!(!scheduler_may_stop_transport(true, false, true));
        assert!(scheduler_may_stop_transport(false, true, false));
        assert!(scheduler_may_stop_transport(true, false, false));
    }

    #[test]
    fn direct_playhead_advances_from_player_position_and_keeps_timeline_offset() {
        assert_eq!(playback_playhead_ms(875.0, 500.0, Some(125.0), 0.0), 1000.0);
        assert_eq!(playback_playhead_ms(875.0, 500.0, Some(900.0), 0.0), 1375.0);
    }

    #[test]
    fn direct_playhead_uses_monotonic_fallback_when_backend_has_no_position() {
        assert_eq!(playback_playhead_ms(250.0, 1_000.0, None, 300.0), 550.0);
    }

    #[test]
    fn progressive_playhead_uses_scheduler_clock_and_preserves_nonzero_start() {
        assert_eq!(progressive_playhead_ms(1_250.0, 375.0), 1_625.0);
        // Repeated scheduler positions while waiting keep the GUI playhead
        // frozen instead of advancing from wall-clock time.
        assert_eq!(progressive_playhead_ms(1_250.0, 375.0), 1_625.0);
    }
}
