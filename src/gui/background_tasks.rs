use crate::gui::KamafeuStudioApp;
use eframe::egui;
use std::time::Instant;

impl KamafeuStudioApp {
    pub(super) fn update_background_tasks(&mut self, ctx: &egui::Context) {
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
                    self.render_log_messages.remove(0);
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
            }
        }

        let mut drop_rx = false;
        if let Some(ref rx) = self.render_rx {
            // Drain the complete ready batch.  Limiting this to two chunks let
            // a busy piano-roll frame leave the sink without the next block.
            for _ in 0..4 {
                match rx.try_recv() {
                    Ok(mut chunk) => {
                        if let Some(error) = chunk.audio.error.take() {
                            self.audio_player.stop();
                            self.piano_roll_state.is_playing = false;
                            self.progressive_playback_started = false;
                            self.transport_state.status_message =
                                format!("Erro no render: {error}");
                            self.render_log_messages.push(error);
                            drop_rx = true;
                            break;
                        }

                        let is_first = !self.progressive_playback_started;
                        let mut samples = std::mem::take(&mut chunk.audio.samples);

                        // Mix metronome clicks if enabled
                        if self.transport_state.metronome_enabled {
                            crate::audio::apply_metronome_clicks(
                                &mut samples,
                                chunk.audio.sample_rate,
                                chunk.audio.channels,
                                chunk.chunk_start_ms,
                                self.transport_state.bpm,
                            );
                        }

                        if is_first {
                            self.piano_roll_state.update_rendered_waveform(
                                &samples,
                                chunk.audio.sample_rate,
                                chunk.audio.channels,
                                chunk.chunk_start_ms,
                            );
                            self.audio_player.play_samples_with_channels(
                                samples,
                                chunk.audio.sample_rate,
                                chunk.audio.channels,
                            );
                            self.piano_roll_state.is_playing = true;
                            self.playback_start_instant = Some(Instant::now());
                            self.progressive_playback_started = true;
                            self.transport_state.status_message =
                                "Reproduzindo áudio...".to_string();
                        } else {
                            self.piano_roll_state.append_rendered_waveform(
                                &samples,
                                chunk.audio.sample_rate,
                                chunk.audio.channels,
                                chunk.chunk_start_ms,
                            );
                            self.audio_player.append_samples_with_channels(
                                samples,
                                chunk.audio.sample_rate,
                                chunk.audio.channels,
                            );
                        }

                        if chunk.is_final {
                            self.transport_state.render_progress = 1.0;
                            self.render_progress = 1.0;
                            drop_rx = true;
                            break;
                        }
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => break,
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => {
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
            if let Some(start_t) = self.playback_start_instant {
                let elapsed_ms =
                    start_t.elapsed().as_secs_f64() * 1000.0 * self.playback_speed_rate;
                self.piano_roll_state.playhead_ms = self.playback_start_offset_ms + elapsed_ms;

                let max_end_ms = self
                    .project
                    .parts
                    .iter()
                    .flat_map(|part| {
                        part.notes
                            .iter()
                            .map(move |note| part.position_ms + note.position_ms + note.duration_ms)
                    })
                    .fold(0.0f64, f64::max);

                if elapsed_ms > 1000.0
                    && self.piano_roll_state.playhead_ms > max_end_ms + 1000.0
                    && !self.audio_player.is_playing()
                {
                    self.pause_audio();
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

        // Keep background preview waveform freshly updated before pressing play

        let cur_ms = self.piano_roll_state.playhead_ms.max(0.0);
        let total_sec = (cur_ms / 1000.0) as u32;
        let mins = total_sec / 60;
        let secs = total_sec % 60;
        let ms_rem = (cur_ms % 1000.0) as u32;
        self.transport_state.playhead_time_str = format!("{:02}:{:02}.{:03}", mins, secs, ms_rem);
    }
}
