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
    RockPower,
    RnBSoul,
    VocaloidRetro,
    HardTuneModern,
    FolkAcoustic,
    LyricalOpera,
}

impl AutoPitchPreset {
    pub fn name(&self) -> &'static str {
        self.display_name(crate::config::AppLanguage::PtBr)
    }

    pub fn display_name(&self, lang: crate::config::AppLanguage) -> &'static str {
        match self {
            Self::NaturalPop => lang.tr("Natural / Pop", "Natural / Pop"),
            Self::JPopAnime => lang.tr("J-Pop / Anime", "J-Pop / Anime"),
            Self::BalladExpressive => lang.tr("Balada / Emotivo", "Ballad / Expressive"),
            Self::SubtleClean => lang.tr("Sutil / Limpo", "Subtle / Clean"),
            Self::EnkaTraditional => lang.tr("Enka / Tradicional", "Enka / Traditional"),
            Self::RockPower => lang.tr("Rock / Belting", "Rock / Belting"),
            Self::RnBSoul => lang.tr("R&B / Soul", "R&B / Soul"),
            Self::VocaloidRetro => lang.tr("Vocaloid Clássico", "Vocaloid Classic"),
            Self::HardTuneModern => lang.tr("Hard Tune / Trap", "Hard Tune / Trap"),
            Self::FolkAcoustic => lang.tr("Folk / Acústico", "Folk / Acoustic"),
            Self::LyricalOpera => lang.tr("Lírico / Ópera", "Lyrical / Opera"),
        }
    }

    pub fn icon(&self) -> &'static str {
        ""
    }

    pub fn description(&self) -> &'static str {
        self.description_for(crate::config::AppLanguage::PtBr)
    }

    pub fn description_for(&self, lang: crate::config::AppLanguage) -> &'static str {
        match self {
            Self::NaturalPop => lang.tr("Equilibrado: scoops suaves, overshoot natural e vibrato dinâmico.", "Balanced: smooth scoops, natural overshoot, and dynamic vibrato."),
            Self::JPopAnime => {
                lang.tr("Ágil e brilhante: transições rápidas, overshoots acentuados e vibrato veloz.", "Bright and agile: fast transitions, accented overshoots, and quick vibrato.")
            }
            Self::BalladExpressive => {
                lang.tr("Emotivo e profundo: ataques lentos com scoop acentuado e vibrato gradual rico.", "Emotional and deep: slow attack with pronounced scoop and rich gradual vibrato.")
            }
            Self::SubtleClean => {
                lang.tr("Moderno e polido: micro-detalhes transparentes com afinação precisa.", "Modern and polished: transparent micro-details with precise tuning.")
            }
            Self::EnkaTraditional => {
                lang.tr("Expressivo clássico: ornamentos kobushi e vibrato ondulante profundo.", "Classic expressive: kobushi ornaments and deep undulating vibrato.")
            }
            Self::RockPower => {
                lang.tr("Agressivo e potente: ataque enérgico, vibrato profundo e dinâmicas com pegada.", "Aggressive and powerful: energetic attack, deep vibrato, and punchy dynamics.")
            }
            Self::RnBSoul => {
                lang.tr("Aveludado e flexível: scoops longos, vibrato tardio com fade-in e ar suave.", "Smooth and flexible: long scoops, late fade-in vibrato, and soft breath.")
            }
            Self::VocaloidRetro => {
                lang.tr("Digital clássico: portamento ágil reto, vibrato senoidal rápido e resposta imediata.", "Classic digital: straight agile portamento, fast sine vibrato, and instant response.")
            }
            Self::HardTuneModern => {
                lang.tr("Afinação instantânea snap: sem scoops, transições a 0ms e tom ultra-travado.", "Instant pitch snap: zero scoops, 0ms transitions, and ultra-locked pitch.")
            }
            Self::FolkAcoustic => {
                lang.tr("Íntimo e delicado: scoops orgânicos, transições lentas, vibrato suave e respiração presente.", "Intimate and delicate: organic scoops, slow transitions, soft vibrato, and present breath.")
            }
            Self::LyricalOpera => {
                lang.tr("Portamento amplo clássico: transições dramáticas, vibrato encorpado contínuo e sustentação.", "Classic wide portamento: dramatic transitions, full continuous vibrato, and sustain.")
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
            Self::RockPower,
            Self::RnBSoul,
            Self::VocaloidRetro,
            Self::HardTuneModern,
            Self::FolkAcoustic,
            Self::LyricalOpera,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum AutoPitchScope {
    #[default]
    SelectedOnly,
    AllNotes,
}

fn default_intensity() -> f64 {
    1.0
}

fn default_true() -> bool {
    true
}

fn default_portamento_speed_ms() -> f64 {
    0.0
}

fn default_multiplier() -> f64 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoPitchOptions {
    pub preset: AutoPitchPreset,
    #[serde(default = "default_intensity")]
    pub intensity: f64,
    #[serde(default = "default_true")]
    pub enable_attack_scoop: bool,
    #[serde(default = "default_true")]
    pub enable_overshoot: bool,
    #[serde(default = "default_true")]
    pub enable_release_drop: bool,
    #[serde(default = "default_true")]
    pub enable_vibrato: bool,

    // Parâmetros de Expressão Vocal
    #[serde(default)]
    pub enable_dynamics_shaping: bool,
    #[serde(default)]
    pub enable_breathiness: bool,
    #[serde(default)]
    pub enable_consonant_velocity: bool,
    #[serde(default)]
    pub enable_attack_decay: bool,

    // Ajustes finos adicionais
    #[serde(default = "default_portamento_speed_ms")]
    pub portamento_speed_ms: f64, // 0.0 = automático pelo preset, >0 = tempo em ms
    #[serde(default = "default_multiplier")]
    pub vibrato_depth_mult: f64, // Multiplicador de profundidade (0.0 a 2.0)
    #[serde(default = "default_multiplier")]
    pub vibrato_period_mult: f64, // Multiplicador de velocidade (0.5 a 2.0)
    #[serde(default = "default_multiplier")]
    pub expression_amount: f64, // Intensidade dos parâmetros de expressão (0.0 a 2.0)
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
            enable_dynamics_shaping: false,
            enable_breathiness: false,
            enable_consonant_velocity: false,
            enable_attack_decay: false,
            portamento_speed_ms: 0.0,
            vibrato_depth_mult: 1.0,
            vibrato_period_mult: 1.0,
            expression_amount: 1.0,
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

            // Aplicação de Parâmetros de Expressão Vocal
            Self::apply_expressions_to_note(
                &mut notes[idx],
                current_midi,
                dur_ms,
                is_prev_adjacent,
                is_next_adjacent,
                options,
            );
        }
    }

    pub fn apply_expressions_to_note(
        note: &mut UNote,
        midi: u8,
        duration_ms: f64,
        is_prev_adjacent: bool,
        is_next_adjacent: bool,
        options: &AutoPitchOptions,
    ) {
        let exp_amount = options.expression_amount.clamp(0.0, 2.5);

        // 1. Dinâmica e Volume
        if options.enable_dynamics_shaping && exp_amount > 0.01 {
            let pitch_factor = ((midi as f64 - 60.0) * 1.5).clamp(-30.0, 45.0);
            let preset_dyn_base = match options.preset {
                AutoPitchPreset::RockPower => 35.0,
                AutoPitchPreset::LyricalOpera => 25.0,
                AutoPitchPreset::JPopAnime => 15.0,
                AutoPitchPreset::HardTuneModern => 10.0,
                AutoPitchPreset::RnBSoul => 8.0,
                AutoPitchPreset::NaturalPop => 5.0,
                AutoPitchPreset::EnkaTraditional => 12.0,
                AutoPitchPreset::BalladExpressive => -5.0,
                AutoPitchPreset::FolkAcoustic => -15.0,
                AutoPitchPreset::SubtleClean => 0.0,
                AutoPitchPreset::VocaloidRetro => 0.0,
            };

            let phrase_start_boost = if !is_prev_adjacent { 8.0 } else { 0.0 };
            let phrase_end_dip = if !is_next_adjacent && duration_ms >= 300.0 {
                -10.0
            } else {
                0.0
            };

            let target_dyn =
                (preset_dyn_base + pitch_factor + phrase_start_boost + phrase_end_dip) * exp_amount;
            note.expressions.dynamics = target_dyn.clamp(-120.0, 120.0);
        }

        // 2. Breathiness (Ar / Respiração)
        if options.enable_breathiness && exp_amount > 0.01 {
            let preset_bre_base = match options.preset {
                AutoPitchPreset::RnBSoul => 25.0,
                AutoPitchPreset::FolkAcoustic => 22.0,
                AutoPitchPreset::BalladExpressive => 18.0,
                AutoPitchPreset::SubtleClean => 5.0,
                AutoPitchPreset::NaturalPop => 8.0,
                AutoPitchPreset::RockPower => -10.0,
                AutoPitchPreset::HardTuneModern => -20.0,
                AutoPitchPreset::VocaloidRetro => -15.0,
                _ => 0.0,
            };

            let start_air = if !is_prev_adjacent { 12.0 } else { 0.0 };
            let end_air = if !is_next_adjacent && duration_ms >= 250.0 {
                15.0
            } else {
                0.0
            };

            let target_bre = (preset_bre_base + start_air + end_air) * exp_amount;
            note.expressions.breathiness = target_bre.clamp(-100.0, 100.0);
        }

        // 3. Velocidade de Consoante (Consonant Velocity)
        if options.enable_consonant_velocity && exp_amount > 0.01 {
            let preset_vel_offset = match options.preset {
                AutoPitchPreset::HardTuneModern => 30.0,
                AutoPitchPreset::RockPower => 25.0,
                AutoPitchPreset::JPopAnime => 20.0,
                AutoPitchPreset::VocaloidRetro => 15.0,
                AutoPitchPreset::NaturalPop => 5.0,
                AutoPitchPreset::SubtleClean => 0.0,
                AutoPitchPreset::EnkaTraditional => 10.0,
                AutoPitchPreset::RnBSoul => -5.0,
                AutoPitchPreset::BalladExpressive => -10.0,
                AutoPitchPreset::FolkAcoustic => -12.0,
                AutoPitchPreset::LyricalOpera => -8.0,
            };

            // Notas curtas ganham consoantes mais rápidas para manter articulação limpa
            let speed_boost = if duration_ms < 180.0 { 15.0 } else { 0.0 };
            let target_cvel = 100.0 + (preset_vel_offset + speed_boost) * exp_amount;
            note.expressions.consonant_velocity = target_cvel.clamp(20.0, 200.0);
        }

        // 4. Ataque e Decaimento (Attack / Decay)
        if options.enable_attack_decay && exp_amount > 0.01 {
            let (atk_offset, dec_val) = match options.preset {
                AutoPitchPreset::RockPower => (30.0, 0.0),
                AutoPitchPreset::HardTuneModern => (25.0, 0.0),
                AutoPitchPreset::JPopAnime => (15.0, 0.0),
                AutoPitchPreset::VocaloidRetro => (10.0, 0.0),
                AutoPitchPreset::LyricalOpera => (5.0, 15.0),
                AutoPitchPreset::EnkaTraditional => (10.0, 20.0),
                AutoPitchPreset::RnBSoul => (-5.0, 25.0),
                AutoPitchPreset::BalladExpressive => (-10.0, 22.0),
                AutoPitchPreset::FolkAcoustic => (-15.0, 30.0),
                _ => (0.0, 0.0),
            };

            let target_atk = 100.0 + atk_offset * exp_amount;
            note.expressions.attack = target_atk.clamp(20.0, 200.0);

            if !is_next_adjacent && duration_ms >= 250.0 {
                note.expressions.decay = (dec_val * exp_amount).clamp(0.0, 100.0);
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
            vib_fade_in_pct,
            portamento_ms_base,
            portamento_shape,
        ): (
            f64,
            f64,
            f64,
            f64,
            f64,
            f64,
            f64,
            f64,
            f64,
            f64,
            &'static str,
        ) = match options.preset {
            AutoPitchPreset::NaturalPop => (
                30.0, 55.0, 16.0, 25.0, 70.0, 65.0, 48.0, 175.0, 25.0, 80.0, "io",
            ),
            AutoPitchPreset::JPopAnime => (
                24.0, 40.0, 26.0, 18.0, 50.0, 60.0, 55.0, 155.0, 20.0, 65.0, "io",
            ),
            AutoPitchPreset::BalladExpressive => (
                48.0, 80.0, 20.0, 38.0, 100.0, 75.0, 65.0, 185.0, 35.0, 110.0, "io",
            ),
            AutoPitchPreset::SubtleClean => (
                15.0, 35.0, 8.0, 12.0, 45.0, 50.0, 28.0, 170.0, 30.0, 50.0, "io",
            ),
            AutoPitchPreset::EnkaTraditional => (
                40.0, 70.0, 32.0, 45.0, 90.0, 80.0, 80.0, 195.0, 20.0, 95.0, "j",
            ),
            AutoPitchPreset::RockPower => (
                35.0, 45.0, 30.0, 30.0, 60.0, 65.0, 70.0, 160.0, 20.0, 70.0, "io",
            ),
            AutoPitchPreset::RnBSoul => (
                55.0, 90.0, 24.0, 40.0, 110.0, 70.0, 52.0, 175.0, 45.0, 120.0, "s",
            ),
            AutoPitchPreset::VocaloidRetro => (
                20.0, 30.0, 15.0, 15.0, 40.0, 60.0, 50.0, 145.0, 15.0, 55.0, "linear",
            ),
            AutoPitchPreset::HardTuneModern => (
                0.0, 0.0, 0.0, 0.0, 0.0, 30.0, 15.0, 130.0, 10.0, 20.0, "linear",
            ),
            AutoPitchPreset::FolkAcoustic => (
                38.0, 75.0, 12.0, 32.0, 85.0, 55.0, 36.0, 180.0, 40.0, 85.0, "io",
            ),
            AutoPitchPreset::LyricalOpera => (
                42.0, 85.0, 22.0, 35.0, 95.0, 85.0, 75.0, 190.0, 25.0, 130.0, "io",
            ),
        };

        // Custom Portamento Override
        let actual_portamento_len = if options.portamento_speed_ms > 0.0 {
            options.portamento_speed_ms.clamp(10.0, 300.0)
        } else {
            portamento_ms_base
        };

        if is_prev_adjacent && prev_midi.is_some() {
            let pm = prev_midi.unwrap();
            let delta_semitones = current_midi as f64 - pm as f64;
            let portamento_start = -(actual_portamento_len * 0.5);

            if options.preset == AutoPitchPreset::HardTuneModern {
                // Hard tune instant transition
                points.push(UPitchBendPoint {
                    time_offset_ms: 0.0,
                    pitch_offset_cents: 0.0,
                    shape: "linear".to_string(),
                });
            } else if delta_semitones > 0.5 {
                points.push(UPitchBendPoint {
                    time_offset_ms: portamento_start,
                    pitch_offset_cents: -delta_semitones * 100.0,
                    shape: portamento_shape.to_string(),
                });

                if options.enable_overshoot && intensity > 0.01 && overshoot_cents_base > 0.0 {
                    let os_cents = (overshoot_cents_base * intensity).min(45.0);
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
                        time_offset_ms: (actual_portamento_len * 0.5).min(duration_ms * 0.3),
                        pitch_offset_cents: 0.0,
                        shape: portamento_shape.to_string(),
                    });
                }
            } else if delta_semitones < -0.5 {
                points.push(UPitchBendPoint {
                    time_offset_ms: portamento_start,
                    pitch_offset_cents: -delta_semitones * 100.0,
                    shape: portamento_shape.to_string(),
                });

                if options.enable_overshoot && intensity > 0.01 && overshoot_cents_base > 0.0 {
                    let under_cents = (-overshoot_cents_base * 0.5 * intensity).max(-30.0);
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
                        time_offset_ms: (actual_portamento_len * 0.5).min(duration_ms * 0.3),
                        pitch_offset_cents: 0.0,
                        shape: portamento_shape.to_string(),
                    });
                }
            } else {
                if intensity > 0.1 && options.preset != AutoPitchPreset::HardTuneModern {
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
            if options.enable_attack_scoop
                && intensity > 0.01
                && duration_ms >= 80.0
                && scoop_cents_base > 0.0
            {
                let scoop_cents = -scoop_cents_base * intensity;
                let scoop_dur = scoop_dur_ms_base.min(duration_ms * 0.35);

                points.push(UPitchBendPoint {
                    time_offset_ms: 0.0,
                    pitch_offset_cents: scoop_cents,
                    shape: if options.preset == AutoPitchPreset::RnBSoul {
                        "s".to_string()
                    } else {
                        "j".to_string()
                    },
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

        // Ornamento Tradicional Enka (Kobushi)
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

        // Release Drop no fim de frases
        if options.enable_release_drop
            && !is_next_adjacent
            && duration_ms >= 250.0
            && intensity > 0.05
            && release_cents_base > 0.0
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
            portamento_start_ms: -(actual_portamento_len * 0.5),
            portamento_length_ms: actual_portamento_len,
            portamento_shape: portamento_shape.to_string(),
        };

        let vibrato = if options.enable_vibrato
            && duration_ms >= 240.0
            && intensity > 0.1
            && vib_depth_cents > 0.0
        {
            let depth_mult = options.vibrato_depth_mult.clamp(0.0, 2.5);
            let period_mult = options.vibrato_period_mult.clamp(0.4, 2.5);
            let adjusted_depth = (vib_depth_cents * intensity * depth_mult).clamp(5.0, 150.0);
            let adjusted_period = (vib_period_ms * period_mult).clamp(40.0, 500.0);

            VibratoParam {
                length_pct: vib_length_pct,
                period_ms: adjusted_period,
                depth_cents: adjusted_depth,
                fade_in_ms: 0.0,
                fade_in_pct: vib_fade_in_pct,
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
            ..Default::default()
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
            ..Default::default()
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

    #[test]
    fn test_autopitch_rock_and_rnb_presets() {
        let mut notes_rock = vec![UNote::new("a", "E4", 0.0, 500.0)];
        let mut notes_rnb = vec![UNote::new("a", "E4", 0.0, 500.0)];

        let opt_rock = AutoPitchOptions {
            preset: AutoPitchPreset::RockPower,
            enable_dynamics_shaping: true,
            enable_consonant_velocity: true,
            ..Default::default()
        };
        let opt_rnb = AutoPitchOptions {
            preset: AutoPitchPreset::RnBSoul,
            enable_dynamics_shaping: true,
            enable_breathiness: true,
            ..Default::default()
        };

        AutoPitchEngine::apply_to_notes(&mut notes_rock, None, &opt_rock);
        AutoPitchEngine::apply_to_notes(&mut notes_rnb, None, &opt_rnb);

        assert!(
            notes_rock[0].expressions.dynamics > 10.0,
            "Rock deve ter boost de dinamica"
        );
        assert!(
            notes_rock[0].expressions.consonant_velocity > 110.0,
            "Rock deve ter consoantes ageis"
        );
        assert!(
            notes_rnb[0].expressions.breathiness > 15.0,
            "RnB deve ter ar/breathiness"
        );
    }

    #[test]
    fn test_autopitch_hard_tune_preset() {
        let mut notes = vec![
            UNote::new("la", "C4", 0.0, 300.0),
            UNote::new("le", "G4", 300.0, 300.0),
        ];

        let opt_hard = AutoPitchOptions {
            preset: AutoPitchPreset::HardTuneModern,
            ..Default::default()
        };

        AutoPitchEngine::apply_to_notes(&mut notes, None, &opt_hard);
        assert_eq!(
            notes[0].pitch_bend.points[0].pitch_offset_cents, 0.0,
            "Hard tune nao tem scoop"
        );
        assert_eq!(
            notes[1].pitch_bend.portamento_length_ms, 20.0,
            "Hard tune portamento instantaneo"
        );
    }
}
