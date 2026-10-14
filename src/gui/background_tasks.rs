use crate::gui::KamafeuStudioApp;
use eframe::egui;
use web_time::Instant;

impl KamafeuStudioApp {
    pub(super) fn update_background_tasks(&mut self, ctx: &egui::Context) {
        // Progressive audio still crosses the UI boundary before reaching the
        // dedicated scheduler. Keep egui polling while rendering so an idle or
        // expensive piano-roll frame cannot starve the sink.
        if self.render_rx.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(8));
        }
        #[cfg(not(target_arch = "wasm32"))]
        let mut scheduler_finished = false;
        #[cfg(not(target_arch = "wasm32"))]
        let mut scheduler_stopped = false;
        #[cfg(not(target_arch = "wasm32"))]
        for event in self.audio_player.drain_scheduler_events() {
            let render_state = self
                .project
                .render_state
                .get_or_insert_with(Default::default);
            let (generation, state, played_ms) = match &event {
                crate::audio::PlaybackSchedulerEvent::PlaybackStarted { generation, .. } => {
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
        }
        #[cfg(not(target_arch = "wasm32"))]
        if scheduler_finished || scheduler_stopped {
            // The scheduler is the authority for progressive playback. Do
            // not leave the UI transport marked as playing after the sink
            // reached the final timestamp or the producer failed.
            self.piano_roll_state.is_playing = false;
            self.progressive_playback_started = false;
            self.playback_start_instant = None;
        }
        if let Some(ref rx) = self.render_log_channel_rx {
            // Keep UI frames responsive even if a renderer emits a burst of logs.
            for _ in 0..64 {
                let Ok((prog, msg)) = rx.try_recv() else {
                    break;
                };
                self.render_progress = prog;
                self.transport_state.render_progress = prog;
                if self.export_in_progress {
                    self.export_progress = prog;
                    self.export_status_detail = msg.clone();
                } else if self.render_rx.is_some() {
                    self.transport_state.status_message = msg.clone();
                    self.render_status_title = format!("{:.0}% • {}", prog * 100.0, msg);
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

        let mut drop_rx = false;
        if let Some(ref rx) = self.render_rx {
            // Drain the complete ready batch.  Limiting this to two chunks let
            // a busy piano-roll frame leave the sink without the next block.
            // Drain the whole ready window in one frame. The producer may
            // already have rendered ahead while egui was busy scrolling.
            for _ in 0..64 {
                match rx.try_recv() {
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

                            self.piano_roll_state.merge_rendered_waveform(
                                &waveform_samples,
                                chunk.audio.sample_rate,
                                chunk.audio.channels,
                                chunk.chunk_start_ms,
                            );

                            // The dedicated audio bridge already sends the
                            // playback copy to the scheduler. The UI only
                            // merges the untouched renderer output into the
                            // waveform and observes scheduler state.
                            if !self.progressive_playback_started
                                && self.audio_player.scheduler_is_running()
                            {
                                self.piano_roll_state.is_playing = true;
                                self.playback_start_instant = Some(Instant::now());
                                self.progressive_playback_started = true;
                                self.transport_state.status_message =
                                    "Reproduzindo áudio...".to_string();
                            }

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
                #[cfg(not(target_arch = "wasm32"))]
                let elapsed_ms = self.audio_player.scheduler_position_ms();
                #[cfg(target_arch = "wasm32")]
                let elapsed_ms =
                    _start_t.elapsed().as_secs_f64() * 1000.0 * self.playback_speed_rate;
                self.piano_roll_state.playhead_ms = self.playback_start_offset_ms + elapsed_ms;

                // Finding the final note is O(all notes). It is only needed
                // after the audio sink has stopped, not on every live playback
                // frame where the sink is still advancing normally.
                if elapsed_ms > 1000.0 && !self.audio_player.is_playing() {
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
            let target_l = (amp * 1.0).clamp(0.0, 1.0);
            let target_r = (amp * 0.98).clamp(0.0, 1.0);
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

            ctx.request_repaint();
        } else {
            self.transport_state.vu_level_l = (self.transport_state.vu_level_l - 0.08).max(0.0);
            self.transport_state.vu_level_r = (self.transport_state.vu_level_r - 0.08).max(0.0);
            self.transport_state.vu_peak_l = (self.transport_state.vu_peak_l - 0.02).max(0.0);
            self.transport_state.vu_peak_r = (self.transport_state.vu_peak_r - 0.02).max(0.0);

            if self.piano_roll_state.is_scrubbing_ruler {
                ctx.request_repaint();
            }
        }

        if self.render_rx.is_some()
            || self.export_rx.is_some()
            || self.preview_waveform_rx.is_some()
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
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
                ctx.request_repaint();
            }
        }

        // Keep the waveform fresh while editing only when the user opted in.
        // The preview method is hash-gated, so this inexpensive call does not
        // start a new render on every UI frame.
        #[cfg(not(target_arch = "wasm32"))]
        if self.config.dsp.background_prerender {
            self.trigger_background_waveform_preview();
        } else if let Some(cancel) = self.preview_waveform_cancel.take() {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            self.preview_waveform_rx = None;
        }

        let cur_ms = self.piano_roll_state.playhead_ms.max(0.0);
        let total_sec = (cur_ms / 1000.0) as u32;
        let mins = total_sec / 60;
        let secs = total_sec % 60;
        let ms_rem = (cur_ms % 1000.0) as u32;
        self.transport_state.playhead_time_str = format!("{:02}:{:02}.{:03}", mins, secs, ms_rem);
    }
}
