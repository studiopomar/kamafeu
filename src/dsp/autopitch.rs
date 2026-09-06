use crate::dsp::pitch::VibratoParam;
use crate::dsp::pitch_bend::PitchBendSolver;
use crate::project::model::{UNote, UPitchBend, UPitchBendPoint};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AutoPitchPreset {
    #[default]
    NaturalPop,
    JPopAnime,
    BalladExpressive,
    SubtleClean,
    EnkaTraditional,
}

impl AutoPitchPreset {
    pub fn name(&self) -> &'static str {
        match self {
            Self::NaturalPop => "🌸 Natural / Pop",
            Self::JPopAnime => "⚡ J-Pop / Anime",
            Self::BalladExpressive => "🎭 Balada / Emotivo",
            Self::SubtleClean => "🍃 Sutil / Limpo",
            Self::EnkaTraditional => "⛩ Enka / Tradicional",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::NaturalPop => "Equilibrado: scoops suaves, overshoot natural e vibrato dinâmico.",
            Self::JPopAnime => {
                "Ágil e brilhante: transições rápidas, overshoots acentuados e vibrato veloz."
            }
            Self::BalladExpressive => {
                "Emotivo e profundo: ataques lentos com scoop acentuado e vibrato gradual rico."
            }
            Self::SubtleClean => {
                "Moderno e polido: micro-detalhes transparentes com afinação precisa."
            }
            Self::EnkaTraditional => {
                "Expressivo clássico: ornamentos kobushi e vibrato ondulante profundo."
            }
        }
    }

    pub fn all() -> &'static [AutoPitchPreset] {
        &[
            Self::NaturalPop,
            Self::JPopAnime,
            Self::BalladExpressive,
            Self::SubtleClean,
            Self::EnkaTraditional,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AutoPitchScope {
    #[default]
    SelectedOnly,
    AllNotes,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoPitchOptions {
    pub preset: AutoPitchPreset,
    pub intensity: f64,
    pub enable_attack_scoop: bool,
    pub enable_overshoot: bool,
    pub enable_release_drop: bool,
    pub enable_vibrato: bool,
}

impl Default for AutoPitchOptions {
    fn default() -> Self {
        Self {
            preset: AutoPitchPreset::NaturalPop,
            intensity: 1.0,
            enable_attack_scoop: true,
            enable_overshoot: true,
            enable_release_drop: true,
            enable_vibrato: true,
        }
    }
}

pub struct AutoPitchEngine;

impl AutoPitchEngine {
    pub fn apply_to_notes(
        notes: &mut [UNote],
        selected_indices: Option<&[usize]>,
        options: &AutoPitchOptions,
    ) {
        if notes.is_empty() {
            return;
        }

        let should_process = |idx: usize| -> bool {
            match selected_indices {
                Some(sel) => sel.contains(&idx),
                None => true,
            }
        };

        let note_count = notes.len();
        let note_context: Vec<(u8, f64, f64)> = notes
            .iter()
            .map(|n| (n.midi_key(), n.position_ms, n.duration_ms))
            .collect();

        for idx in 0..note_count {
            if !should_process(idx) {
                continue;
            }

            let (current_midi, pos_ms, dur_ms) = note_context[idx];

            let (prev_midi, is_prev_adjacent) = if idx > 0 {
                let (pm, pp, pd) = note_context[idx - 1];
                let adj = (pp + pd - pos_ms).abs() <= 2.0;
                (Some(pm), adj)
            } else {
                (None, false)
            };

            let is_next_adjacent = if idx + 1 < note_count {
                let (_nm, np, _nd) = note_context[idx + 1];
                (pos_ms + dur_ms - np).abs() <= 2.0
            } else {
                false
            };

            let (new_pitch_bend, new_vibrato) = Self::generate_pitch_for_note(
                current_midi,
                dur_ms,
                prev_midi,
                is_prev_adjacent,
                is_next_adjacent,
                options,
            );

            notes[idx].pitch_bend = new_pitch_bend;
            if options.enable_vibrato {
                notes[idx].vibrato = new_vibrato;
            }
        }
    }

    pub fn generate_pitch_for_note(
        current_midi: u8,
        duration_ms: f64,
        prev_midi: Option<u8>,
        is_prev_adjacent: bool,
        is_next_adjacent: bool,
        options: &AutoPitchOptions,
    ) -> (UPitchBend, VibratoParam) {
        let mut points: Vec<UPitchBendPoint> = Vec::new();
        let intensity = options.intensity.clamp(0.0, 2.5);

        let (
            scoop_cents_base,
            scoop_dur_ms_base,
            overshoot_cents_base,
            release_cents_base,
            release_dur_ms_base,
            vib_length_pct,
            vib_depth_cents,
            vib_period_ms,
        ): (f64, f64, f64, f64, f64, f64, f64, f64) = match options.preset {
            AutoPitchPreset::NaturalPop => (30.0, 55.0, 16.0, 25.0, 70.0, 65.0, 48.0, 175.0),
            AutoPitchPreset::JPopAnime => (24.0, 40.0, 26.0, 18.0, 50.0, 60.0, 55.0, 155.0),
            AutoPitchPreset::BalladExpressive => (48.0, 80.0, 20.0, 38.0, 100.0, 75.0, 65.0, 185.0),
            AutoPitchPreset::SubtleClean => (15.0, 35.0, 8.0, 12.0, 45.0, 50.0, 28.0, 170.0),
            AutoPitchPreset::EnkaTraditional => (40.0, 70.0, 32.0, 45.0, 90.0, 80.0, 80.0, 195.0),
        };

        if is_prev_adjacent && prev_midi.is_some() {
            let pm = prev_midi.unwrap();
            let delta_semitones = current_midi as f64 - pm as f64;
            let portamento_start = -40.0;

            if delta_semitones > 0.5 {
                points.push(UPitchBendPoint {
                    time_offset_ms: portamento_start,
                    pitch_offset_cents: -delta_semitones * 100.0,
                    shape: "io".to_string(),
                });

                if options.enable_overshoot && intensity > 0.01 {
                    let os_cents = (overshoot_cents_base * intensity).min(40.0);
                    let os_time = (20.0 * (1.0 + delta_semitones * 0.05)).min(duration_ms * 0.25);
                    points.push(UPitchBendPoint {
                        time_offset_ms: os_time,
                        pitch_offset_cents: os_cents,
                        shape: "s".to_string(),
                    });
                    let settle_time = (os_time + 35.0).min(duration_ms * 0.45);
                    points.push(UPitchBendPoint {
                        time_offset_ms: settle_time,
                        pitch_offset_cents: 0.0,
                        shape: "s".to_string(),
                    });
                } else {
                    points.push(UPitchBendPoint {
                        time_offset_ms: (35.0f64).min(duration_ms * 0.3),
                        pitch_offset_cents: 0.0,
                        shape: "io".to_string(),
                    });
                }
            } else if delta_semitones < -0.5 {
                points.push(UPitchBendPoint {
                    time_offset_ms: portamento_start,
                    pitch_offset_cents: -delta_semitones * 100.0,
                    shape: "io".to_string(),
                });

                if options.enable_overshoot && intensity > 0.01 {
                    let under_cents = (-overshoot_cents_base * 0.5 * intensity).max(-25.0);
                    let under_time = (18.0f64).min(duration_ms * 0.25);
                    points.push(UPitchBendPoint {
                        time_offset_ms: under_time,
                        pitch_offset_cents: under_cents,
                        shape: "s".to_string(),
                    });
                    let settle_time = (under_time + 30.0).min(duration_ms * 0.4);
                    points.push(UPitchBendPoint {
                        time_offset_ms: settle_time,
                        pitch_offset_cents: 0.0,
                        shape: "s".to_string(),
                    });
                } else {
                    points.push(UPitchBendPoint {
                        time_offset_ms: (35.0f64).min(duration_ms * 0.3),
                        pitch_offset_cents: 0.0,
                        shape: "io".to_string(),
                    });
                }
            } else {
                if intensity > 0.1 {
                    points.push(UPitchBendPoint {
                        time_offset_ms: -15.0,
                        pitch_offset_cents: 0.0,
                        shape: "s".to_string(),
                    });
                    points.push(UPitchBendPoint {
                        time_offset_ms: (12.0f64).min(duration_ms * 0.2),
                        pitch_offset_cents: -12.0 * intensity,
                        shape: "s".to_string(),
                    });
                    points.push(UPitchBendPoint {
                        time_offset_ms: (30.0f64).min(duration_ms * 0.35),
                        pitch_offset_cents: 0.0,
                        shape: "s".to_string(),
                    });
                } else {
                    points.push(UPitchBendPoint {
                        time_offset_ms: 0.0,
                        pitch_offset_cents: 0.0,
                        shape: "s".to_string(),
                    });
                }
            }
        } else {
            if options.enable_attack_scoop && intensity > 0.01 && duration_ms >= 80.0 {
                let scoop_cents = -scoop_cents_base * intensity;
                let scoop_dur = scoop_dur_ms_base.min(duration_ms * 0.35);

                points.push(UPitchBendPoint {
                    time_offset_ms: 0.0,
                    pitch_offset_cents: scoop_cents,
                    shape: "j".to_string(),
                });
                points.push(UPitchBendPoint {
                    time_offset_ms: scoop_dur,
                    pitch_offset_cents: 0.0,
                    shape: "s".to_string(),
                });
            } else {
                points.push(UPitchBendPoint {
                    time_offset_ms: 0.0,
                    pitch_offset_cents: 0.0,
                    shape: "s".to_string(),
                });
            }
        }

        if options.preset == AutoPitchPreset::EnkaTraditional
            && duration_ms >= 300.0
            && intensity > 0.3
        {
            let kobushi_center = duration_ms * 0.45;
            let wave_amp = 35.0 * intensity;
            points.push(UPitchBendPoint {
                time_offset_ms: kobushi_center - 40.0,
                pitch_offset_cents: 0.0,
                shape: "s".to_string(),
            });
            points.push(UPitchBendPoint {
                time_offset_ms: kobushi_center - 15.0,
                pitch_offset_cents: wave_amp,
                shape: "io".to_string(),
            });
            points.push(UPitchBendPoint {
                time_offset_ms: kobushi_center + 15.0,
                pitch_offset_cents: -wave_amp * 0.7,
                shape: "io".to_string(),
            });
            points.push(UPitchBendPoint {
                time_offset_ms: kobushi_center + 40.0,
                pitch_offset_cents: 0.0,
                shape: "s".to_string(),
            });
        }

        if options.enable_release_drop
            && !is_next_adjacent
            && duration_ms >= 250.0
            && intensity > 0.05
        {
            let rel_drop = -release_cents_base * intensity;
            let rel_dur = release_dur_ms_base.min(duration_ms * 0.3);
            let rel_start = duration_ms - rel_dur;

            points.push(UPitchBendPoint {
                time_offset_ms: rel_start,
                pitch_offset_cents: 0.0,
                shape: "r".to_string(), // decaimento logarítmico
            });
            points.push(UPitchBendPoint {
                time_offset_ms: duration_ms,
                pitch_offset_cents: rel_drop,
                shape: "s".to_string(),
            });
        } else {
            let last_t = points.last().map(|p| p.time_offset_ms).unwrap_or(0.0);
            if last_t < duration_ms - 20.0 {
                points.push(UPitchBendPoint {
                    time_offset_ms: duration_ms,
                    pitch_offset_cents: 0.0,
                    shape: "s".to_string(),
                });
            }
        }

        points.sort_by(|a, b| {
            a.time_offset_ms
                .partial_cmp(&b.time_offset_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let simplified = PitchBendSolver::simplify_pitch_points(&points, 1.5);

        let pitch_bend = UPitchBend {
            points: simplified,
            snap_first: is_prev_adjacent,
            portamento_start_ms: -40.0,
            portamento_length_ms: 80.0,
            portamento_shape: "io".to_string(),
        };

        let vibrato = if options.enable_vibrato && duration_ms >= 240.0 && intensity > 0.1 {
            let adjusted_depth = (vib_depth_cents * intensity).clamp(10.0, 120.0);
            VibratoParam {
                length_pct: vib_length_pct,
                period_ms: vib_period_ms,
                depth_cents: adjusted_depth,
                fade_in_ms: 0.0,
                fade_in_pct: 25.0,
                fade_out_pct: 15.0,
                shift_pct: 0.0,
                drift_pct: 0.0,
                volume_link_pct: 0.0,
            }
        } else {
            VibratoParam::default()
        };

        (pitch_bend, vibrato)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_autopitch_isolated_note_attack_scoop() {
        let note = UNote::new("a", "C4", 0.0, 500.0);
        let options = AutoPitchOptions {
            preset: AutoPitchPreset::NaturalPop,
            intensity: 1.0,
            enable_attack_scoop: true,
            enable_overshoot: true,
            enable_release_drop: true,
            enable_vibrato: true,
        };

        let mut notes = vec![note];
        AutoPitchEngine::apply_to_notes(&mut notes, None, &options);

        assert!(!notes[0].pitch_bend.points.is_empty());
        let first_pt = &notes[0].pitch_bend.points[0];
        assert!(
            first_pt.pitch_offset_cents < -10.0,
            "Ataque isolado deve ter scoop negativo, got {}",
            first_pt.pitch_offset_cents
        );

        assert!(notes[0].vibrato.depth_cents > 20.0);
    }

    #[test]
    fn test_autopitch_leap_overshoot() {
        let options = AutoPitchOptions {
            preset: AutoPitchPreset::JPopAnime,
            intensity: 1.0,
            enable_attack_scoop: true,
            enable_overshoot: true,
            enable_release_drop: false,
            enable_vibrato: false,
        };

        let mut notes = vec![
            UNote::new("a", "C4", 0.0, 400.0),   // C4 = 60
            UNote::new("e", "G4", 400.0, 400.0), // G4 = 67 (+7 semitons)
        ];

        AutoPitchEngine::apply_to_notes(&mut notes, None, &options);

        let note_g4 = &notes[1];
        let has_positive_overshoot = note_g4
            .pitch_bend
            .points
            .iter()
            .any(|p| p.pitch_offset_cents > 10.0);
        assert!(
            has_positive_overshoot,
            "Salto ascendente C4 -> G4 deve gerar overshoot positivo, pontos: {:?}",
            note_g4.pitch_bend.points
        );
    }

    #[test]
    fn test_autopitch_presets_differ() {
        let note_pop = UNote::new("a", "C4", 0.0, 600.0);
        let note_ballad = UNote::new("a", "C4", 0.0, 600.0);

        let opt_pop = AutoPitchOptions {
            preset: AutoPitchPreset::NaturalPop,
            ..Default::default()
        };
        let opt_ballad = AutoPitchOptions {
            preset: AutoPitchPreset::BalladExpressive,
            ..Default::default()
        };

        let mut notes_pop = vec![note_pop];
        let mut notes_ballad = vec![note_ballad];

        AutoPitchEngine::apply_to_notes(&mut notes_pop, None, &opt_pop);
        AutoPitchEngine::apply_to_notes(&mut notes_ballad, None, &opt_ballad);

        assert!(
            notes_ballad[0].vibrato.depth_cents > notes_pop[0].vibrato.depth_cents,
            "Ballad vibrato deve ser mais profundo que Pop"
        );
        let scoop_pop = notes_pop[0].pitch_bend.points[0].pitch_offset_cents;
        let scoop_ballad = notes_ballad[0].pitch_bend.points[0].pitch_offset_cents;
        assert!(
            scoop_ballad < scoop_pop,
            "Ballad scoop deve ser mais profundo (mais negativo) que Pop: {} vs {}",
            scoop_ballad,
            scoop_pop
        );
    }
}
