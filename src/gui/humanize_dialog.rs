use crate::config::AppLanguage;
use crate::dsp::pitch::VibratoParam;
use crate::gui::theme::ThemeConfig;
use crate::project::model::UNote;
use eframe::egui::{self, Color32, Frame, RichText, Window};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct HumanizeParams {
    pub timing_jitter_ms: f64,
    pub pitch_cents_jitter: f64,
    pub volume_jitter_pct: f64,
    pub breathiness_jitter_pct: f64,
}

impl Default for HumanizeParams {
    fn default() -> Self {
        Self {
            timing_jitter_ms: 8.0,
            pitch_cents_jitter: 10.0,
            volume_jitter_pct: 8.0,
            breathiness_jitter_pct: 5.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AutoVibratoParams {
    pub min_duration_ms: f64,
    pub length_pct: f64,
    pub depth_cents: f64,
    pub period_ms: f64,
    pub fade_in_pct: f64,
}

impl Default for AutoVibratoParams {
    fn default() -> Self {
        Self {
            min_duration_ms: 350.0,
            length_pct: 65.0,
            depth_cents: 60.0,
            period_ms: 160.0,
            fade_in_pct: 25.0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HumanizeDialogState {
    pub is_open: bool,
    pub humanize_params: HumanizeParams,
    pub vibrato_params: AutoVibratoParams,
    pub apply_to_all_if_none_selected: bool,
}

impl Default for HumanizeDialogState {
    fn default() -> Self {
        Self {
            is_open: false,
            humanize_params: HumanizeParams::default(),
            vibrato_params: AutoVibratoParams::default(),
            apply_to_all_if_none_selected: true,
        }
    }
}

/// Simple pseudo-random number generator for deterministic humanization
fn pseudo_rand(seed: u64) -> f64 {
    let mut x = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    x ^= x >> 18;
    let val = (x >> 32) as u32;
    (val as f64) / (u32::MAX as f64) * 2.0 - 1.0 // Range -1.0 .. +1.0
}

pub fn apply_humanize(
    notes: &mut [UNote],
    target_indices: &[usize],
    params: &HumanizeParams,
    seed_base: u64,
) {
    for (_i, &idx) in target_indices.iter().enumerate() {
        if idx < notes.len() {
            let s = seed_base.wrapping_add((idx as u64).wrapping_mul(1013));
            let r_time = pseudo_rand(s);
            let r_pitch = pseudo_rand(s.wrapping_add(31));
            let r_vol = pseudo_rand(s.wrapping_add(67));
            let r_breath = pseudo_rand(s.wrapping_add(109));

            let delta_ms = r_time * params.timing_jitter_ms;
            notes[idx].position_ms = (notes[idx].position_ms + delta_ms).max(0.0);

            let delta_pitch = r_pitch * params.pitch_cents_jitter;
            notes[idx].expressions.pitch_delta =
                (notes[idx].expressions.pitch_delta + delta_pitch).clamp(-200.0, 200.0);

            let delta_vol = r_vol * params.volume_jitter_pct;
            notes[idx].expressions.volume =
                (notes[idx].expressions.volume + delta_vol).clamp(0.0, 200.0);

            let delta_breath = r_breath * params.breathiness_jitter_pct;
            notes[idx].expressions.breathiness =
                (notes[idx].expressions.breathiness + delta_breath).clamp(0.0, 100.0);
        }
    }
}

pub fn apply_smart_vibrato(
    notes: &mut [UNote],
    target_indices: &[usize],
    params: &AutoVibratoParams,
) {
    for &idx in target_indices {
        if idx < notes.len() {
            if notes[idx].duration_ms >= params.min_duration_ms {
                notes[idx].vibrato = VibratoParam {
                    length_pct: params.length_pct,
                    period_ms: params.period_ms,
                    depth_cents: params.depth_cents,
                    fade_in_ms: 0.0,
                    fade_in_pct: params.fade_in_pct,
                    fade_out_pct: 10.0,
                    shift_pct: 0.0,
                    drift_pct: 0.0,
                    volume_link_pct: 0.0,
                };
            }
        }
    }
}

pub fn draw_humanize_dialog(
    ctx: &egui::Context,
    lang: AppLanguage,
    theme: &ThemeConfig,
    state: &mut HumanizeDialogState,
    notes: &mut [UNote],
    selected_indices: &HashSet<usize>,
    on_applied: &mut dyn FnMut(),
) {
    if !state.is_open {
        return;
    }

    let mut window_open = state.is_open;
    let mut apply_humanize_clicked = false;
    let mut apply_vibrato_clicked = false;
    let mut apply_all_clicked = false;
    let mut close_clicked = false;

    let target_indices: Vec<usize> = if !selected_indices.is_empty() {
        selected_indices.iter().copied().collect()
    } else if state.apply_to_all_if_none_selected {
        (0..notes.len()).collect()
    } else {
        Vec::new()
    };

    Window::new(
        RichText::new(lang.tr(
            "Humanizador & Vibrato Natural",
            "Humanizer & Natural Vibrato",
        ))
        .strong()
        .color(theme.accent_c32()),
    )
    .open(&mut window_open)
    .resizable(true)
    .default_width(420.0)
    .frame(
        Frame::window(&ctx.style())
            .fill(theme.bg_panel_c32())
            .stroke(theme.card_stroke()),
    )
    .show(ctx, |ui| {
        ui.vertical(|ui| {
            let note_count = target_indices.len();
            ui.label(
                RichText::new(format!(
                    "{}: {} {}",
                    lang.tr("Alvo", "Target"),
                    note_count,
                    lang.tr("notas selecionadas", "selected notes")
                ))
                .strong()
                .size(11.0)
                .color(theme.accent_c32()),
            );
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            // --- Section 1: Humanize ---
            Frame::none()
                .fill(theme.card_bg_c32())
                .rounding(theme.ui_rounding())
                .stroke(theme.card_stroke())
                .inner_margin(egui::Margin::same(8.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(lang.tr(
                            "Variação Orgânica / Humanização",
                            "Organic Variation / Humanization",
                        ))
                        .strong()
                        .size(11.5)
                        .color(theme.accent_c32()),
                    );
                    ui.label(
                        RichText::new(lang.tr(
                            "Adiciona micro-imperfeições sutis para soar mais humano e natural:",
                            "Adds subtle micro-imperfections for a more human and natural sound:",
                        ))
                        .size(9.5)
                        .color(theme.text_muted_c32()),
                    );
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label(lang.tr("Jitter de Tempo (ms):", "Timing Jitter (ms):"));
                        ui.add(
                            egui::Slider::new(
                                &mut state.humanize_params.timing_jitter_ms,
                                0.0..=30.0,
                            )
                            .suffix(" ms"),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label(lang.tr("Jitter de Pitch (cents):", "Pitch Jitter (cents):"));
                        ui.add(
                            egui::Slider::new(
                                &mut state.humanize_params.pitch_cents_jitter,
                                0.0..=35.0,
                            )
                            .suffix(" c"),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label(lang.tr("Jitter de Volume (%):", "Volume Jitter (%):"));
                        ui.add(
                            egui::Slider::new(
                                &mut state.humanize_params.volume_jitter_pct,
                                0.0..=25.0,
                            )
                            .suffix(" %"),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label(lang.tr("Jitter de Soprosidade (%):", "Breathiness Jitter (%):"));
                        ui.add(
                            egui::Slider::new(
                                &mut state.humanize_params.breathiness_jitter_pct,
                                0.0..=20.0,
                            )
                            .suffix(" %"),
                        );
                    });

                    ui.add_space(4.0);
                    if ui
                        .button(
                            RichText::new(
                                lang.tr("Aplicar Apenas Humanização", "Apply Humanization Only"),
                            )
                            .size(11.0),
                        )
                        .clicked()
                    {
                        apply_humanize_clicked = true;
                    }
                });

            ui.add_space(8.0);

            // --- Section 2: Smart Vibrato ---
            Frame::none()
                .fill(theme.card_bg_c32())
                .rounding(theme.ui_rounding())
                .stroke(theme.card_stroke())
                .inner_margin(egui::Margin::same(8.0))
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(lang.tr("Auto-Vibrato Inteligente", "Smart Auto-Vibrato"))
                            .strong()
                            .size(11.5)
                            .color(theme.accent_c32()),
                    );
                    ui.label(
                        RichText::new(lang.tr(
                            "Detecta notas sustentadas e aplica vibrato musical progressivo:",
                            "Detects sustained notes and applies progressive musical vibrato:",
                        ))
                        .size(9.5)
                        .color(theme.text_muted_c32()),
                    );
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label(lang.tr("Duração Mínima (ms):", "Minimum Duration (ms):"));
                        ui.add(
                            egui::Slider::new(
                                &mut state.vibrato_params.min_duration_ms,
                                150.0..=1000.0,
                            )
                            .suffix(" ms"),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label(lang.tr("Profundidade (cents):", "Depth (cents):"));
                        ui.add(
                            egui::Slider::new(&mut state.vibrato_params.depth_cents, 10.0..=150.0)
                                .suffix(" c"),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label(lang.tr("Período / Velocidade (ms):", "Period / Speed (ms):"));
                        ui.add(
                            egui::Slider::new(&mut state.vibrato_params.period_ms, 80.0..=300.0)
                                .suffix(" ms"),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label(lang.tr("Extensão do Vibrato (%):", "Vibrato Length (%):"));
                        ui.add(
                            egui::Slider::new(&mut state.vibrato_params.length_pct, 20.0..=100.0)
                                .suffix(" %"),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label(lang.tr("Fade-in Suave (%):", "Smooth Fade-in (%):"));
                        ui.add(
                            egui::Slider::new(&mut state.vibrato_params.fade_in_pct, 5.0..=60.0)
                                .suffix(" %"),
                        );
                    });

                    ui.add_space(4.0);
                    if ui
                        .button(
                            RichText::new(
                                lang.tr("Aplicar Apenas Auto-Vibrato", "Apply Auto-Vibrato Only"),
                            )
                            .size(11.0),
                        )
                        .clicked()
                    {
                        apply_vibrato_clicked = true;
                    }
                });

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                if ui
                    .button(
                        RichText::new(lang.tr(
                            "Aplicar Ambos (Humanizar + Vibrato)",
                            "Apply Both (Humanize + Vibrato)",
                        ))
                        .strong()
                        .size(11.5)
                        .color(Color32::WHITE),
                    )
                    .clicked()
                {
                    apply_all_clicked = true;
                }
                if ui
                    .button(RichText::new(lang.tr("Fechar", "Close")).size(11.0))
                    .clicked()
                {
                    close_clicked = true;
                }
            });

            let seed = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(42);

            let has_changes = apply_humanize_clicked || apply_vibrato_clicked || apply_all_clicked;
            if has_changes {
                on_applied();
            }

            if apply_humanize_clicked || apply_all_clicked {
                apply_humanize(notes, &target_indices, &state.humanize_params, seed);
            }

            if apply_vibrato_clicked || apply_all_clicked {
                apply_smart_vibrato(notes, &target_indices, &state.vibrato_params);
            }

            if has_changes {
                close_clicked = true;
            }
        });
    });

    state.is_open = window_open && !close_clicked;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_humanize() {
        let mut notes = vec![
            UNote::new("a", "C4", 100.0, 400.0),
            UNote::new("ka", "D4", 500.0, 400.0),
        ];
        let params = HumanizeParams {
            timing_jitter_ms: 10.0,
            pitch_cents_jitter: 15.0,
            volume_jitter_pct: 10.0,
            breathiness_jitter_pct: 5.0,
        };
        apply_humanize(&mut notes, &[0, 1], &params, 12345);
        assert_ne!(notes[0].position_ms, 100.0);
    }

    #[test]
    fn test_apply_smart_vibrato() {
        let mut notes = vec![
            UNote::new("short", "C4", 0.0, 100.0),
            UNote::new("long", "C4", 200.0, 600.0),
        ];
        let params = AutoVibratoParams {
            min_duration_ms: 300.0,
            length_pct: 70.0,
            depth_cents: 50.0,
            period_ms: 170.0,
            fade_in_pct: 20.0,
        };
        apply_smart_vibrato(&mut notes, &[0, 1], &params);
        assert_eq!(notes[0].vibrato.length_pct, 0.0);
        assert_eq!(notes[1].vibrato.length_pct, 70.0);
    }
}
