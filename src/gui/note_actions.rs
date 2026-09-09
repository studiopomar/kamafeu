use crate::gui::KamafeuStudioApp;
use crate::oto::Voicebank;
use crate::project::model::UNote;
use std::path::PathBuf;

impl KamafeuStudioApp {
    pub fn apply_autopitch(&mut self) {
        self.push_history();

        let selected: Option<Vec<usize>> = match self.autopitch_scope {
            crate::dsp::AutoPitchScope::SelectedOnly => {
                if self.piano_roll_state.selected_note_indices.is_empty() {
                    if let Some(idx) = self.piano_roll_state.selected_note_index {
                        Some(vec![idx])
                    } else {
                        None
                    }
                } else {
                    Some(
                        self.piano_roll_state
                            .selected_note_indices
                            .iter()
                            .copied()
                            .collect(),
                    )
                }
            }
            crate::dsp::AutoPitchScope::AllNotes => None,
        };

        let options = self.autopitch_options.clone();
        let notes = self.current_notes_mut();
        crate::dsp::AutoPitchEngine::apply_to_notes(notes, selected.as_deref(), &options);
        self.piano_roll_state.continuous_edit_dirty = true;
        self.is_dirty = true;
        self.transport_state.status_message =
            format!("Pre-tunning {} ({})", self.config.language.tr("aplicado com sucesso", "applied successfully"), options.preset.display_name(self.config.language));
    }

    pub fn current_notes_mut(&mut self) -> &mut Vec<UNote> {
        if self.project.tracks.is_empty() {
            self.project
                .tracks
                .push(crate::project::model::UTrack::default());
        }
        if self.active_track_index >= self.project.tracks.len() {
            self.active_track_index = 0;
        }
        let track_idx = self.active_track_index;

        if let Some(part_idx) = self
            .project
            .parts
            .iter()
            .position(|p| p.track_index == track_idx)
        {
            &mut self.project.parts[part_idx].notes
        } else {
            let part_name = format!("Part Track {}", track_idx + 1);
            let new_part = crate::project::model::UVoicePart::new(part_name, track_idx);
            self.project.parts.push(new_part);
            let last_idx = self.project.parts.len() - 1;
            &mut self.project.parts[last_idx].notes
        }
    }

    pub fn current_notes(&self) -> &[UNote] {
        let track_idx = self.active_track_index;
        if let Some(part) = self
            .project
            .parts
            .iter()
            .find(|p| p.track_index == track_idx)
        {
            &part.notes
        } else if !self.project.parts.is_empty() {
            &self.project.parts[0].notes
        } else {
            &[]
        }
    }

    pub fn push_history(&mut self) {
        self.undo_manager.push_state(self.project.clone());
        self.is_dirty = true;
    }

    pub fn humanize_selection(&mut self) {
        self.push_history();
        let sel = self.piano_roll_state.selected_note_indices.clone();
        let notes = self.current_notes_mut();
        let target_indices: Vec<usize> = if !sel.is_empty() {
            sel.into_iter().collect()
        } else {
            (0..notes.len()).collect()
        };

        let mut rng_seed = 123456789u64;
        for &idx in &target_indices {
            if let Some(note) = notes.get_mut(idx) {
                rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let time_offset = ((rng_seed % 21) as f64 - 10.0) * 0.8;
                rng_seed = rng_seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                let vel_offset = (rng_seed % 17) as f64 - 8.0;

                note.position_ms = (note.position_ms + time_offset).max(0.0);
                note.expressions.velocity =
                    (note.expressions.velocity + vel_offset).clamp(30.0, 180.0);
            }
        }
        self.is_dirty = true;
        self.transport_state.status_message =
            format!("Humanização aplicada em {} notas", target_indices.len());
    }

    pub fn quantize_positions(&mut self) {
        self.push_history();
        let snap = self.transport_state.grid_snap;
        let bpm = self.transport_state.bpm;
        let px_per_ms = self.piano_roll_state.px_per_ms;
        let sel = self.piano_roll_state.selected_note_indices.clone();
        let notes = self.current_notes_mut();
        let target_indices: Vec<usize> = if !sel.is_empty() {
            sel.into_iter().collect()
        } else {
            (0..notes.len()).collect()
        };

        for &idx in &target_indices {
            if let Some(note) = notes.get_mut(idx) {
                note.position_ms = crate::gui::piano_roll::state::apply_snap_with_zoom(
                    note.position_ms,
                    snap,
                    bpm,
                    px_per_ms,
                )
                .max(0.0);
            }
        }
        self.is_dirty = true;
        self.transport_state.status_message = format!(
            "Posições quantizadas para {}",
            snap.resolved_label(bpm, px_per_ms)
        );
    }

    pub fn quantize_durations(&mut self) {
        self.push_history();
        let snap = self.transport_state.grid_snap;
        let bpm = self.transport_state.bpm;
        let px_per_ms = self.piano_roll_state.px_per_ms;
        let sel = self.piano_roll_state.selected_note_indices.clone();
        let notes = self.current_notes_mut();
        let target_indices: Vec<usize> = if !sel.is_empty() {
            sel.into_iter().collect()
        } else {
            (0..notes.len()).collect()
        };

        for &idx in &target_indices {
            if let Some(note) = notes.get_mut(idx) {
                let step = snap.step_ms_with_zoom(bpm, px_per_ms).unwrap_or(125.0);
                note.duration_ms = (note.duration_ms / step).round().max(1.0) * step;
            }
        }
        self.is_dirty = true;
        self.transport_state.status_message = format!(
            "Durações quantizadas para {}",
            snap.resolved_label(bpm, px_per_ms)
        );
    }

    pub fn legato_connect_notes(&mut self) {
        self.push_history();
        let sel = self.piano_roll_state.selected_note_indices.clone();
        let notes = self.current_notes_mut();
        let mut target_indices: Vec<usize> = if !sel.is_empty() {
            sel.into_iter().collect()
        } else {
            (0..notes.len()).collect()
        };
        target_indices.sort_by(|&a, &b| {
            notes
                .get(a)
                .map(|n| n.position_ms)
                .unwrap_or(0.0)
                .partial_cmp(&notes.get(b).map(|n| n.position_ms).unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for i in 0..target_indices.len().saturating_sub(1) {
            let curr_idx = target_indices[i];
            let next_idx = target_indices[i + 1];
            let next_pos = notes.get(next_idx).map(|n| n.position_ms).unwrap_or(0.0);
            if let Some(curr_note) = notes.get_mut(curr_idx) {
                if next_pos > curr_note.position_ms {
                    curr_note.duration_ms = next_pos - curr_note.position_ms;
                }
            }
        }
        self.is_dirty = true;
        self.transport_state.status_message = "Legato conectado entre notas adjacentes".to_string();
    }

    pub fn fix_overlapping_notes(&mut self) {
        self.push_history();
        let sel = self.piano_roll_state.selected_note_indices.clone();
        let notes = self.current_notes_mut();
        if notes.len() < 2 {
            return;
        }

        let mut target_indices: Vec<usize> = if !sel.is_empty() {
            sel.into_iter().collect()
        } else {
            (0..notes.len()).collect()
        };

        target_indices.sort_by(|&a, &b| {
            notes
                .get(a)
                .map(|n| n.position_ms)
                .unwrap_or(0.0)
                .partial_cmp(&notes.get(b).map(|n| n.position_ms).unwrap_or(0.0))
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut fixed_count = 0;
        for i in 0..target_indices.len().saturating_sub(1) {
            let curr_idx = target_indices[i];
            let next_idx = target_indices[i + 1];
            let next_pos = notes.get(next_idx).map(|n| n.position_ms).unwrap_or(0.0);
            if let Some(curr_note) = notes.get_mut(curr_idx) {
                let curr_end = curr_note.position_ms + curr_note.duration_ms;
                if curr_end > next_pos && next_pos > curr_note.position_ms {
                    curr_note.duration_ms = (next_pos - curr_note.position_ms).max(30.0);
                    fixed_count += 1;
                }
            }
        }

        self.is_dirty = true;
        self.piano_roll_state.continuous_edit_dirty = true;
        self.transport_state.status_message = format!(
            "Sobreposição de notas corrigida ({} ajustada(s))",
            fixed_count
        );
    }

    pub fn select_short_notes(&mut self, threshold_ms: f64) {
        let matched: Vec<usize> = self
            .current_notes()
            .iter()
            .enumerate()
            .filter(|(_, n)| n.duration_ms < threshold_ms)
            .map(|(i, _)| i)
            .collect();
        self.piano_roll_state.selected_note_indices = matched.into_iter().collect();
        self.piano_roll_state.selected_note_index = self
            .piano_roll_state
            .selected_note_indices
            .iter()
            .next()
            .copied();
        self.transport_state.status_message = format!(
            "{} notas curtas (< {:.0}ms) selecionadas",
            self.piano_roll_state.selected_note_indices.len(),
            threshold_ms
        );
    }

    pub fn select_overlapping_notes(&mut self) {
        let notes = self.current_notes();
        let mut matched = std::collections::HashSet::new();
        for i in 0..notes.len() {
            for j in (i + 1)..notes.len() {
                let n1 = &notes[i];
                let n2 = &notes[j];
                let overlap = (n1.position_ms < n2.position_ms + n2.duration_ms)
                    && (n2.position_ms < n1.position_ms + n1.duration_ms);
                if overlap {
                    matched.insert(i);
                    matched.insert(j);
                }
            }
        }
        self.piano_roll_state.selected_note_indices = matched;
        self.piano_roll_state.selected_note_index = self
            .piano_roll_state
            .selected_note_indices
            .iter()
            .next()
            .copied();
        self.transport_state.status_message = format!(
            "{} notas sobrepostas selecionadas",
            self.piano_roll_state.selected_note_indices.len()
        );
    }

    pub fn select_out_of_scale_notes(&mut self) {
        let scale = self.piano_roll_state.active_scale;
        let root = self.piano_roll_state.scale_root_key;
        if scale == crate::gui::piano_roll::state::MusicalScale::Chromatic {
            self.transport_state.status_message =
                "Escala Cromática ativa: todas as notas estão na escala".to_string();
            return;
        }
        let matched: Vec<usize> = self
            .current_notes()
            .iter()
            .enumerate()
            .filter(|(_, n)| !scale.is_in_scale(root, n.midi_key()))
            .map(|(i, _)| i)
            .collect();
        self.piano_roll_state.selected_note_indices = matched.into_iter().collect();
        self.piano_roll_state.selected_note_index = self
            .piano_roll_state
            .selected_note_indices
            .iter()
            .next()
            .copied();
        self.transport_state.status_message = format!(
            "{} notas fora da escala selecionadas",
            self.piano_roll_state.selected_note_indices.len()
        );
    }

    pub fn invert_melody_retrograde(&mut self) {
        self.push_history();
        let sel = self.piano_roll_state.selected_note_indices.clone();
        let notes = self.current_notes_mut();
        let target_indices: Vec<usize> = if !sel.is_empty() {
            sel.into_iter().collect()
        } else {
            (0..notes.len()).collect()
        };
        if target_indices.len() < 2 {
            return;
        }

        let min_pos = target_indices
            .iter()
            .map(|&i| notes[i].position_ms)
            .fold(f64::INFINITY, f64::min);
        let max_end = target_indices
            .iter()
            .map(|&i| notes[i].position_ms + notes[i].duration_ms)
            .fold(0.0f64, f64::max);

        for &i in &target_indices {
            let orig_start = notes[i].position_ms;
            let orig_dur = notes[i].duration_ms;
            notes[i].position_ms = max_end - (orig_start - min_pos) - orig_dur;
        }
        self.is_dirty = true;
        self.transport_state.status_message =
            "Melodia invertida horizontalmente (Retrógrado)".to_string();
    }

    pub fn invert_melody_intervals(&mut self) {
        self.push_history();
        let sel = self.piano_roll_state.selected_note_indices.clone();
        let notes = self.current_notes_mut();
        let target_indices: Vec<usize> = if !sel.is_empty() {
            sel.into_iter().collect()
        } else {
            (0..notes.len()).collect()
        };
        if target_indices.is_empty() {
            return;
        }

        let pivot_midi = notes[target_indices[0]].midi_key();
        for &i in &target_indices {
            let curr_midi = notes[i].midi_key() as i32;
            let diff = curr_midi - pivot_midi as i32;
            let inverted = (pivot_midi as i32 - diff).clamp(12, 120) as u8;
            notes[i].set_midi_key(inverted);
        }
        self.is_dirty = true;
        self.transport_state.status_message =
            "Intervalos melódicos espelhados verticalmente".to_string();
    }

    pub fn clean_pitch_suffixes_from_lyrics(&mut self) {
        self.push_history();
        let notes = self.current_notes_mut();
        let mut count = 0;
        for note in notes.iter_mut() {
            if let Some(pos) = note.lyric.rfind('_') {
                let suffix = &note.lyric[pos + 1..];
                if suffix.chars().any(|c| c.is_ascii_digit()) {
                    note.lyric = note.lyric[..pos].to_string();
                    count += 1;
                }
            }
        }
        self.piano_roll_state.phoneme_cache.clear();
        self.is_dirty = true;
        self.transport_state.status_message =
            format!("Sufixos de afinação removidos de {} notas", count);
    }

    pub fn apply_vocal_preset(&mut self, preset_name: &str) {
        self.push_history();
        let sel = self.piano_roll_state.selected_note_indices.clone();
        let notes = self.current_notes_mut();
        let target_indices: Vec<usize> = if !sel.is_empty() {
            sel.into_iter().collect()
        } else {
            (0..notes.len()).collect()
        };

        for &i in &target_indices {
            if let Some(note) = notes.get_mut(i) {
                match preset_name {
                    "whisper" => {
                        note.expressions.breathiness = 65.0;
                        note.expressions.velocity = 80.0;
                        note.expressions.gender = 15.0;
                    }
                    "belting" => {
                        note.expressions.breathiness = -20.0;
                        note.expressions.velocity = 140.0;
                        note.expressions.gender = -10.0;
                        note.expressions.dynamics = 30.0;
                    }
                    "robotic" => {
                        note.pitch_bend.points.clear();
                        note.pitch_bend.portamento_length_ms = 1.0;
                        note.vibrato.length_pct = 0.0;
                    }
                    "natural" => {
                        note.expressions.breathiness = 5.0;
                        note.expressions.velocity = 100.0;
                        note.expressions.gender = 0.0;
                        note.expressions.dynamics = 0.0;
                    }
                    _ => {}
                }
            }
        }
        self.is_dirty = true;
        self.transport_state.status_message = format!(
            "Preset vocal '{}' aplicado em {} notas",
            preset_name,
            target_indices.len()
        );
    }

    pub fn zoom_fit_all_notes(&mut self) {
        let mut min_pos = f64::INFINITY;
        let mut max_end = 0.0f64;
        let mut has_notes = false;

        for part in &self.project.parts {
            for note in &part.notes {
                has_notes = true;
                min_pos = min_pos.min(note.position_ms);
                max_end = max_end.max(note.position_ms + note.duration_ms);
            }
        }

        if has_notes && max_end > min_pos {
            let total_dur = (max_end - min_pos).max(2000.0);
            let target_px = (1200.0 / total_dur).clamp(0.05, 1.5);
            self.piano_roll_state.px_per_ms = target_px as f32;
            self.piano_roll_state.horizontal_scroll_offset = (min_pos * target_px).max(0.0) as f32;
            self.transport_state.status_message =
                "Zoom ajustado para enquadrar todo o projeto".to_string();
        } else {
            self.piano_roll_state.px_per_ms = 0.25;
            self.piano_roll_state.horizontal_scroll_offset = 0.0;
        }
    }

    pub fn smooth_selected_pitch_curves(&mut self) {
        self.push_history();
        let sel = self.piano_roll_state.selected_note_indices.clone();
        let notes = self.current_notes_mut();
        let target_indices: Vec<usize> = if !sel.is_empty() {
            sel.into_iter().collect()
        } else {
            (0..notes.len()).collect()
        };

        let mut smoothed_count = 0;
        for &idx in &target_indices {
            if let Some(note) = notes.get_mut(idx) {
                let pts = &mut note.pitch_bend.points;
                if pts.len() >= 3 {
                    let old_vals: Vec<f64> = pts.iter().map(|p| p.pitch_offset_cents).collect();
                    for i in 1..pts.len() - 1 {
                        pts[i].pitch_offset_cents =
                            (old_vals[i - 1] + old_vals[i] * 2.0 + old_vals[i + 1]) / 4.0;
                    }
                    smoothed_count += 1;
                }
            }
        }
        self.is_dirty = true;
        self.transport_state.status_message = format!(
            "Curvas de pitch suavizadas (Gaussiano 3-pts) em {} notas",
            smoothed_count
        );
    }

    pub fn copy_selected_notes(&mut self) {
        let selected_indices = self.piano_roll_state.selected_note_indices.clone();
        let notes = self.current_notes();

        let indices_to_copy: Vec<usize> = if !selected_indices.is_empty() {
            let mut v: Vec<usize> = selected_indices.into_iter().collect();
            v.sort();
            v
        } else if let Some(sel_idx) = self.piano_roll_state.selected_note_index {
            if sel_idx < notes.len() {
                vec![sel_idx]
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        if !indices_to_copy.is_empty() {
            self.clipboard = indices_to_copy
                .iter()
                .filter_map(|&idx| notes.get(idx).cloned())
                .collect();
            self.transport_state.status_message =
                format!("{} nota(s) copiada(s)", self.clipboard.len());
        }
    }

    pub fn cut_selected_notes(&mut self) {
        self.copy_selected_notes();
        if !self.clipboard.is_empty() {
            let count = self.clipboard.len();
            self.delete_selected_notes();
            self.transport_state.status_message = format!("{} nota(s) recortada(s)", count);
        }
    }

    pub fn delete_selected_notes(&mut self) {
        let selected_indices = self.piano_roll_state.selected_note_indices.clone();
        let sel_idx_opt = self.piano_roll_state.selected_note_index;
        let total_notes = self.current_notes().len();

        let mut to_delete: Vec<usize> = if !selected_indices.is_empty() {
            selected_indices.into_iter().collect()
        } else if let Some(sel_idx) = sel_idx_opt {
            if sel_idx < total_notes {
                vec![sel_idx]
            } else {
                vec![]
            }
        } else {
            vec![]
        };

        if !to_delete.is_empty() {
            self.push_history();
            to_delete.sort_by(|a, b| b.cmp(a));
            let notes = self.current_notes_mut();
            for d_idx in to_delete {
                if d_idx < notes.len() {
                    notes.remove(d_idx);
                }
            }
            self.piano_roll_state.selected_note_index = None;
            self.piano_roll_state.selected_note_indices.clear();
        }
    }

    pub fn paste_notes(&mut self) {
        if self.clipboard.is_empty() {
            return;
        }

        self.push_history();
        let target_pos = self.piano_roll_state.playhead_ms.max(0.0);
        let min_pos = self
            .clipboard
            .iter()
            .map(|n| n.position_ms)
            .fold(f64::INFINITY, f64::min);
        let offset = if min_pos.is_finite() {
            target_pos - min_pos
        } else {
            0.0
        };

        let start_idx = self.current_notes().len();
        let mut pasted_notes = Vec::new();

        for note in &self.clipboard {
            let mut pasted = note.clone();
            pasted.position_ms = (pasted.position_ms + offset).max(0.0);
            pasted_notes.push(pasted);
        }

        let new_count = pasted_notes.len();
        self.current_notes_mut().extend(pasted_notes);

        self.piano_roll_state.selected_note_indices.clear();
        for i in 0..new_count {
            self.piano_roll_state
                .selected_note_indices
                .insert(start_idx + i);
        }
        if new_count > 0 {
            self.piano_roll_state.selected_note_index = Some(start_idx);
        }

        self.transport_state.status_message = format!("{} nota(s) colada(s)", new_count);
    }

    pub fn transpose_selected_note(&mut self, semitones: i32) {
        let selected_indices = self.piano_roll_state.selected_note_indices.clone();
        let min_m = self.piano_roll_state.min_midi as i32;
        let max_m = self.piano_roll_state.max_midi as i32;

        let target_indices: Vec<usize> = if !selected_indices.is_empty() {
            selected_indices.into_iter().collect()
        } else if let Some(sel_idx) = self.piano_roll_state.selected_note_index {
            vec![sel_idx]
        } else {
            vec![]
        };

        if !target_indices.is_empty() {
            self.push_history();
            let notes = self.current_notes_mut();
            for idx in target_indices {
                if idx < notes.len() {
                    let cur_m = notes[idx].midi_key() as i32;
                    let new_m = (cur_m + semitones).clamp(min_m, max_m) as u8;
                    notes[idx].set_midi_key(new_m);
                }
            }
        }
    }

    pub fn reset_selected_notes_parameters(&mut self) {
        let selected_indices = self.piano_roll_state.selected_note_indices.clone();
        let target_indices: Vec<usize> = if !selected_indices.is_empty() {
            selected_indices.into_iter().collect()
        } else if let Some(sel_idx) = self.piano_roll_state.selected_note_index {
            vec![sel_idx]
        } else {
            let len = self.current_notes().len();
            (0..len).collect()
        };

        if !target_indices.is_empty() {
            self.push_history();
            let count = target_indices.len();
            let notes = self.current_notes_mut();
            for idx in target_indices {
                if idx < notes.len() {
                    notes[idx].reset_all_parameters();
                }
            }
            self.piano_roll_state.phoneme_cache_hash = 0;
            self.piano_roll_state.phoneme_cache.clear();
            self.piano_roll_state.note_phonemes_cache.clear();
            self.is_dirty = true;
            self.transport_state.status_message =
                format!("Parâmetros de {} nota(s) resetados para o padrão", count);
        }
    }

    pub fn rephonemize_all_notes(&mut self) {
        self.push_history();
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
        let vb = self.voicebank.as_ref().unwrap_or(&dummy_vb);
        let mode = self.vocal_mode_params.phonemizer_mode;

        for part in self.project.parts.iter_mut() {
            let phones =
                crate::phonemizer::JapanesePhonemizer::apply_phonemizer(&part.notes, vb, mode);
            for (idx, phone) in phones.iter().enumerate() {
                if idx < part.notes.len() {
                    part.notes[idx].lyric = phone.lyric.clone();
                }
            }
        }
        self.piano_roll_state.phoneme_cache.clear();
        self.transport_state.status_message =
            "Todas as notas foram refonetizadas com sucesso!".to_string();
    }

    pub fn clear_all_pitch_curves(&mut self) {
        self.push_history();
        for part in self.project.parts.iter_mut() {
            for note in part.notes.iter_mut() {
                note.pitch_bend.points.clear();
                note.vibrato.length_pct = 0.0;
            }
        }
        self.transport_state.status_message = "Curvas de pitch e vibrato limpas!".to_string();
    }

    pub fn reset_all_volume_envelopes(&mut self) {
        self.push_history();
        for part in self.project.parts.iter_mut() {
            for note in part.notes.iter_mut() {
                note.envelope = crate::dsp::envelope::UtauEnvelope::default();
            }
        }
        self.transport_state.status_message =
            "Envelopes de volume resetados para o padrão!".to_string();
    }

    pub fn apply_autopitch_all(&mut self) {
        self.push_history();
        for part in self.project.parts.iter_mut() {
            let all_indices: std::collections::HashSet<usize> = (0..part.notes.len()).collect();
            crate::gui::piano_roll::apply_autopitch_to_selection(
                &mut part.notes,
                &all_indices,
                crate::gui::piano_roll::AutoPitchStyle::SmoothPop,
            );
        }
        self.transport_state.status_message = self.config.language.tr(
            "Pre-tunning Suave/Pop aplicado a todas as notas!",
            "Soft/Pop Pre-tunning applied to all notes!",
        ).to_string();
    }
}
