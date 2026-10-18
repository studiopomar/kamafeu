use super::{UProject, UTrack, UVoicePart, CURRENT_PROJECT_SCHEMA_VERSION};

impl UProject {
    /// Drop imported phonetic decisions when the user explicitly requests a
    /// full recalculation. Musical timing, expressions and flags are kept.
    pub fn recalculate_imported_phonemes(&mut self) {
        for part in &mut self.parts {
            for note in &mut part.notes {
                note.phonetic_hint = None;
                note.phoneme_overrides.clear();
                note.phoneme_durations_ms.clear();
            }
        }
    }

    /// Change the project tempo while keeping every event on the same musical beat.
    ///
    /// Timeline values are stored in milliseconds, so changing only `bpm` would
    /// leave playback speed unchanged. Rescaling positions and durations by the
    /// inverse tempo ratio preserves their beat positions and makes the tempo
    /// change audible.
    pub fn set_bpm_preserving_beats(&mut self, new_bpm: f64) -> Option<f64> {
        if !new_bpm.is_finite() || new_bpm <= 0.0 {
            return None;
        }

        let old_bpm = if self.bpm.is_finite() && self.bpm > 0.0 {
            self.bpm
        } else {
            120.0
        };
        let new_bpm = new_bpm.clamp(20.0, 999.0);
        let time_scale = old_bpm / new_bpm;

        if (old_bpm - new_bpm).abs() <= f64::EPSILON {
            self.bpm = new_bpm;
            return Some(1.0);
        }

        for part in &mut self.parts {
            part.position_ms *= time_scale;
            for note in &mut part.notes {
                note.position_ms *= time_scale;
                note.duration_ms *= time_scale;
                for point in &mut note.pitch_bend.points {
                    point.time_offset_ms *= time_scale;
                }
            }
        }
        for wave in &mut self.wave_parts {
            wave.position_ms *= time_scale;
        }
        for marker in &mut self.markers {
            marker.position_ms *= time_scale;
        }
        for section in &mut self.sections {
            section.start_ms *= time_scale;
            section.end_ms *= time_scale;
        }

        self.bpm = new_bpm;
        Some(time_scale)
    }

    /// Restore invariants after loading permissive third-party project files.
    pub fn normalize(&mut self) {
        if self.schema_version == 0 {
            self.schema_version = CURRENT_PROJECT_SCHEMA_VERSION;
        }
        // Future documents may be opened read-only by older hosts. The APS
        // writer rejects them instead of silently downgrading their schema.
        if !self.bpm.is_finite() || self.bpm <= 0.0 {
            self.bpm = 120.0;
        }
        self.bpm = self.bpm.clamp(20.0, 999.0);
        self.time_signature_numerator = self.time_signature_numerator.clamp(1, 32);
        self.time_signature_denominator = match self.time_signature_denominator {
            1 | 2 | 4 | 8 | 16 | 32 => self.time_signature_denominator,
            _ => 4,
        };

        for extension in &mut self.extensions {
            extension.id = extension.id.trim().to_string();
            extension.version = extension.version.trim().to_string();
            extension.origin = extension.origin.trim().to_string();
            extension.manifest_fingerprint = extension.manifest_fingerprint.trim().to_string();
        }
        self.extensions.sort_by(|left, right| {
            left.id
                .cmp(&right.id)
                .then_with(|| left.version.cmp(&right.version))
                .then_with(|| left.origin.cmp(&right.origin))
                .then_with(|| left.manifest_fingerprint.cmp(&right.manifest_fingerprint))
        });
        self.extensions.dedup();

        let max_part_track = self
            .parts
            .iter()
            .map(|part| part.track_index.saturating_add(1))
            .max()
            .unwrap_or(1);
        let max_wave_track = self
            .wave_parts
            .iter()
            .map(|wave| wave.track_index.saturating_add(1))
            .max()
            .unwrap_or(1);
        let required_tracks = max_part_track.max(max_wave_track).max(1);

        while self.tracks.len() < required_tracks {
            self.tracks.push(UTrack {
                name: format!("Track {}", self.tracks.len() + 1),
                ..UTrack::default()
            });
        }
        if self.parts.is_empty() {
            self.parts.push(UVoicePart::new("Parte Vocal 1", 0));
        }

        for wave in &mut self.wave_parts {
            if !wave.position_ms.is_finite() {
                wave.position_ms = 0.0;
            }
            wave.position_ms = wave.position_ms.max(0.0);
            if !wave.volume_db.is_finite() {
                wave.volume_db = 0.0;
            }
            wave.volume_db = wave.volume_db.clamp(-60.0, 12.0);
        }

        for (index, marker) in self.markers.iter_mut().enumerate() {
            if !marker.position_ms.is_finite() {
                marker.position_ms = 0.0;
            }
            marker.position_ms = marker.position_ms.max(0.0);
            if marker.name.trim().is_empty() {
                marker.name = format!("Marcador {}", index + 1);
            }
            marker.name = marker.name.trim().chars().take(120).collect();
            marker.color = marker.color.take().filter(|color| {
                let color = color.trim();
                color.len() == 7
                    && color.starts_with('#')
                    && color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
            });
        }
        self.markers.sort_by(|left, right| {
            left.position_ms
                .partial_cmp(&right.position_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.name.cmp(&right.name))
        });

        for (index, section) in self.sections.iter_mut().enumerate() {
            if !section.start_ms.is_finite() {
                section.start_ms = 0.0;
            }
            if !section.end_ms.is_finite() {
                section.end_ms = section.start_ms;
            }
            section.start_ms = section.start_ms.max(0.0);
            section.end_ms = section.end_ms.max(section.start_ms);
            if section.name.trim().is_empty() {
                section.name = format!("Seção {}", index + 1);
            }
            section.name = section.name.trim().chars().take(120).collect();
            section.color = section.color.take().filter(|color| {
                let color = color.trim();
                color.len() == 7
                    && color.starts_with('#')
                    && color.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
            });
        }
        self.sections.sort_by(|left, right| {
            left.start_ms
                .partial_cmp(&right.start_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| left.name.cmp(&right.name))
        });

        for track in &mut self.tracks {
            if !track.volume_db.is_finite() {
                track.volume_db = 0.0;
            }
            if !track.pan.is_finite() {
                track.pan = 0.0;
            }
            track.volume_db = track.volume_db.clamp(-60.0, 12.0);
            track.pan = track.pan.clamp(-1.0, 1.0);
        }

        for part in &mut self.parts {
            if !part.position_ms.is_finite() {
                part.position_ms = 0.0;
            }
            part.position_ms = part.position_ms.max(0.0);
            for note in &mut part.notes {
                if !note.position_ms.is_finite() {
                    note.position_ms = 0.0;
                }
                if !note.duration_ms.is_finite() {
                    note.duration_ms = 1.0;
                }
                note.position_ms = note.position_ms.max(0.0);
                note.duration_ms = note.duration_ms.max(1.0);
                for phoneme_override in &mut note.phoneme_overrides {
                    phoneme_override.normalize(note.duration_ms);
                }
                note.envelope.normalize(note.duration_ms);
                if !note.envelope.crossfade_ms.is_finite() {
                    note.envelope.crossfade_ms = 0.0;
                }
                note.envelope.crossfade_ms = note.envelope.crossfade_ms.clamp(0.0, 500.0);
                note.envelope.p2 = if note.envelope.p2.is_finite() {
                    note.envelope.p2.clamp(0.0, 500.0)
                } else {
                    5.0
                };
                note.envelope.p5 = if note.envelope.p5.is_finite() {
                    note.envelope.p5.clamp(0.0, 500.0)
                } else {
                    35.0
                };
                if note
                    .phoneme_durations_ms
                    .iter()
                    .any(|duration| !duration.is_finite() || *duration <= 0.0)
                {
                    note.phoneme_durations_ms.clear();
                }
                if !note.expressions.consonant_velocity.is_finite() {
                    note.expressions.consonant_velocity = 100.0;
                }
                note.expressions.consonant_velocity =
                    note.expressions.consonant_velocity.clamp(-100.0, 200.0);
                if !note.expressions.velocity.is_finite() {
                    note.expressions.velocity = 100.0;
                }
                if !note.expressions.dynamics.is_finite() {
                    note.expressions.dynamics = 0.0;
                }
                if !note.expressions.pitch_delta.is_finite() {
                    note.expressions.pitch_delta = 0.0;
                }
                if !note.expressions.gender.is_finite() {
                    note.expressions.gender = 0.0;
                }
                if !note.expressions.breathiness.is_finite() {
                    note.expressions.breathiness = 0.0;
                }
                if !note.expressions.modulation.is_finite() {
                    note.expressions.modulation = 0.0;
                }
                if !note.expressions.volume.is_finite() {
                    note.expressions.volume = 100.0;
                }
                if !note.expressions.attack.is_finite() {
                    note.expressions.attack = 100.0;
                }
                if !note.expressions.decay.is_finite() {
                    note.expressions.decay = 0.0;
                }
                note.expressions.velocity = note.expressions.velocity.clamp(0.0, 200.0);
                note.expressions.dynamics = note.expressions.dynamics.clamp(-240.0, 120.0);
                note.expressions.pitch_delta = note.expressions.pitch_delta.clamp(-1200.0, 1200.0);
                note.expressions.gender = note.expressions.gender.clamp(-100.0, 100.0);
                note.expressions.breathiness = note.expressions.breathiness.clamp(0.0, 100.0);
                note.expressions.modulation = note.expressions.modulation.clamp(0.0, 100.0);
                note.expressions.volume = note.expressions.volume.clamp(0.0, 200.0);
                note.expressions.attack = note.expressions.attack.clamp(0.0, 200.0);
                note.expressions.decay = note.expressions.decay.clamp(0.0, 100.0);
                note.vibrato.normalize();
                if !note.pitch_bend.portamento_start_ms.is_finite() {
                    note.pitch_bend.portamento_start_ms = -40.0;
                }
                if !note.pitch_bend.portamento_length_ms.is_finite() {
                    note.pitch_bend.portamento_length_ms = 80.0;
                }
                note.pitch_bend.portamento_start_ms =
                    note.pitch_bend.portamento_start_ms.clamp(-2000.0, 2000.0);
                note.pitch_bend.portamento_length_ms =
                    note.pitch_bend.portamento_length_ms.clamp(1.0, 2000.0);
                if note.pitch_bend.portamento_shape.is_empty() {
                    note.pitch_bend.portamento_shape = "io".to_string();
                }
                note.pitch_bend.points.retain(|point| {
                    point.time_offset_ms.is_finite() && point.pitch_offset_cents.is_finite()
                });
                note.pitch_bend.points.sort_by(|left, right| {
                    left.time_offset_ms
                        .partial_cmp(&right.time_offset_ms)
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
            }
        }
    }
}
