use crate::gui::piano_roll::PianoRollState;
use crate::gui::KamafeuStudioApp;
use crate::oto::Voicebank;
use crate::renderer::ProjectRenderer;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;

impl KamafeuStudioApp {
    pub fn play_current_track(&mut self) {
        if self.render_rx.is_some() {
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
        let vocal_mode_params = self.vocal_mode_params.clone();
        let render_threads = self.render_threads.clamp(1, 16) as usize;
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

        self.render_log_window_open = false;
        self.render_progress = 0.0;
        self.render_status_title = format!("{} • {:.0}ms", resampler_driver.name(), playhead_ms);

        let (tx, rx) = std::sync::mpsc::channel();
        self.render_log_channel_rx = Some(rx);
        let (audio_tx, audio_rx) = std::sync::mpsc::sync_channel(2);
        self.render_rx = Some(audio_rx);
        let cancel = Arc::new(AtomicBool::new(false));
        self.render_cancel = Some(cancel.clone());
        self.playback_start_offset_ms = playhead_ms;
        self.piano_roll_state.is_playing = false;
        self.piano_roll_state.rendered_waveform_peaks.clear();
        self.playback_start_instant = None;
        self.progressive_playback_started = false;
        let tx = Arc::new(std::sync::Mutex::new(tx));
        let tx_cb = tx.clone();
        std::thread::spawn(move || {
            let report_progress = move |progress, message: &str| {
                if let Ok(guard) = tx_cb.lock() {
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
                    2000.0,
                    resampler_driver.as_ref(),
                    wavtool_driver.as_ref(),
                    &vocal_mode_params,
                    Some(&report_progress),
                    Some(cancel.as_ref()),
                    &audio_tx,
                );
            };
            let render_pool = rayon::ThreadPoolBuilder::new()
                .num_threads(render_threads)
                .thread_name(|index| format!("kamafeu-render-{index}"))
                .build();
            match render_pool {
                Ok(pool) => pool.install(render),
                Err(_) => render(),
            };
        });
    }

    pub fn pause_audio(&mut self) {
        if let Some(cancel) = self.render_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.audio_player.stop();
        self.piano_roll_state.is_playing = false;
        self.playback_start_instant = None;
        self.render_rx = None;
        self.progressive_playback_started = false;
        self.transport_state.status_message = "Pausado".to_string();
    }

    pub fn stop_audio(&mut self) {
        self.pause_audio();
        self.piano_roll_state.playhead_ms = 0.0;
        self.transport_state.status_message = "Parado".to_string();
    }

    pub fn preview_tone(&mut self, freq: f64) {
        let sample_rate = 44100;
        let num_samples = (sample_rate as f64 * 0.3) as usize;
        let mut raw_samples: Vec<f32> = (0..num_samples)
            .map(|i| {
                (i as f64 * 2.0 * std::f64::consts::PI * freq / sample_rate as f64).sin() as f32
                    * 0.4
            })
            .collect();

        let len = raw_samples.len();
        for (i, sample) in raw_samples.iter_mut().enumerate() {
            let fade = (len - i) as f32 / len as f32;
            *sample *= fade;
        }

        self.audio_player.play_samples(raw_samples, sample_rate);
    }

    pub fn compute_preview_waveform_hash(&self) -> u64 {
        use std::hash::{DefaultHasher, Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.active_track_index.hash(&mut hasher);
        self.sample_rate.hash(&mut hasher);
        (self.project.bpm * 1000.0)
            .round()
            .to_bits()
            .hash(&mut hasher);
        if let Some(vb) = &self.voicebank {
            vb.name.hash(&mut hasher);
            vb.root_path.hash(&mut hasher);
            vb.entries.len().hash(&mut hasher);
        }
        for (p_idx, part) in self.project.parts.iter().enumerate() {
            p_idx.hash(&mut hasher);
            part.track_index.hash(&mut hasher);
            (part.position_ms * 10.0)
                .round()
                .to_bits()
                .hash(&mut hasher);
            for n in &part.notes {
                n.lyric.hash(&mut hasher);
                n.pitch.hash(&mut hasher);
                (n.position_ms * 10.0).round().to_bits().hash(&mut hasher);
                (n.duration_ms * 10.0).round().to_bits().hash(&mut hasher);
                n.expressions.pitch_delta.to_bits().hash(&mut hasher);
                n.expressions.consonant_velocity.to_bits().hash(&mut hasher);
            }
        }
        for w in &self.project.wave_parts {
            w.track_index.hash(&mut hasher);
            w.file_path.hash(&mut hasher);
            (w.position_ms * 10.0).round().to_bits().hash(&mut hasher);
            (w.duration_ms * 10.0).round().to_bits().hash(&mut hasher);
        }
        hasher.finish()
    }

    pub fn trigger_background_waveform_preview(&mut self) {
        if self.piano_roll_state.is_playing || self.render_rx.is_some() || self.export_rx.is_some()
        {
            return;
        }

        let has_notes = self.project.parts.iter().any(|p| !p.notes.is_empty());
        let has_waves = !self.project.wave_parts.is_empty();
        if !has_notes && !has_waves {
            if !self.piano_roll_state.rendered_waveform_peaks.is_empty() {
                self.piano_roll_state.rendered_waveform_peaks.clear();
            }
            self.preview_waveform_cache_hash = 0;
            return;
        }

        let current_hash = self.compute_preview_waveform_hash();
        if current_hash == self.preview_waveform_cache_hash
            && (!self.piano_roll_state.rendered_waveform_peaks.is_empty()
                || self.preview_waveform_rx.is_some())
        {
            return;
        }

        if let Some(cancel) = self.preview_waveform_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }

        self.preview_waveform_cache_hash = current_hash;
        let cancel = Arc::new(AtomicBool::new(false));
        self.preview_waveform_cancel = Some(cancel.clone());

        let (tx, rx) = std::sync::mpsc::channel();
        self.preview_waveform_rx = Some(rx);

        let project = self.project.clone();
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
        let vocal_mode_params = self.vocal_mode_params.clone();
        let render_threads = self.render_threads.max(1) as usize;

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

            let render_pool = rayon::ThreadPoolBuilder::new()
                .num_threads(render_threads)
                .thread_name(|idx| format!("kamafeu-preview-wf-{idx}"))
                .build();

            let rendered_audio = match render_pool {
                Ok(pool) => pool.install(render),
                Err(_) => render(),
            };

            if cancel.load(Ordering::Relaxed) {
                return;
            }

            if !rendered_audio.samples.is_empty() {
                let mut temp_state = PianoRollState::default();
                temp_state.update_rendered_waveform(
                    &rendered_audio.samples,
                    rendered_audio.sample_rate,
                    rendered_audio.channels,
                    0.0,
                );
                let _ = tx.send((current_hash, temp_state.rendered_waveform_peaks));
            }
        });
    }
}
