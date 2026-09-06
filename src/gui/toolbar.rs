use crate::gui::types::{AutoScrollMode, EditTool, GridSnapOption, PitchSubTool, TransportState};
use eframe::egui::{self, Color32, Frame, Margin, RichText, Rounding, Stroke, Vec2};

fn toolbar_card<R>(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui) -> R) -> R {
    Frame::none()
        .fill(Color32::from_rgb(20, 15, 29))
        .rounding(Rounding::same(4.0))
        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(46, 36, 66)))
        .inner_margin(Margin::symmetric(6.0, 3.0))
        .show(ui, add_contents)
        .inner
}

pub fn draw_unified_toolbar(
    ui: &mut egui::Ui,
    state: &mut TransportState,
    is_playing: bool,
    log_open: &mut bool,
    current_tool: &mut EditTool,
    pitch_sub_tool: &mut PitchSubTool,
    auto_scroll_mode: &mut AutoScrollMode,
    px_per_ms: &mut f32,
    row_height: &mut f32,
    on_play: &mut dyn FnMut(),
    on_stop: &mut dyn FnMut(),
    on_export_wav: &mut dyn FnMut(),
    on_open_copaiba: &mut dyn FnMut(),
    on_open_autopitch: &mut dyn FnMut(),
) {
    ui.spacing_mut().item_spacing = Vec2::new(6.0, 4.0);

    ui.horizontal(|ui| {
        ui.add_space(2.0);

        toolbar_card(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

            let (play_bg, play_text, play_color, play_stroke) = if is_playing {
                (
                    Color32::from_rgb(36, 27, 53),
                    "⏸",
                    Color32::from_rgb(216, 180, 254),
                    Color32::from_rgb(192, 132, 252),
                )
            } else {
                (
                    Color32::from_rgb(12, 44, 28),
                    "▶",
                    Color32::from_rgb(0, 255, 157),
                    Color32::from_rgb(0, 255, 157),
                )
            };

            let play_btn = egui::Button::new(
                RichText::new(play_text)
                    .strong()
                    .size(13.0)
                    .color(play_color),
            )
            .min_size(Vec2::new(26.0, 22.0))
            .fill(play_bg)
            .stroke(Stroke::new(1.2_f32, play_stroke))
            .rounding(Rounding::same(4.0));

            if ui
                .add(play_btn)
                .on_hover_text("Tocar / Pausar (Space)")
                .clicked()
            {
                on_play();
            }

            let stop_btn = egui::Button::new(
                RichText::new("⏹")
                    .strong()
                    .size(13.0)
                    .color(Color32::from_rgb(255, 110, 110)),
            )
            .min_size(Vec2::new(26.0, 22.0))
            .fill(Color32::from_rgb(38, 18, 22))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(85, 30, 40)))
            .rounding(Rounding::same(4.0));

            if ui
                .add(stop_btn)
                .on_hover_text("Parar e retornar ao início (Esc)")
                .clicked()
            {
                on_stop();
            }

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            ui.label(
                RichText::new(&state.playhead_time_str)
                    .monospace()
                    .strong()
                    .size(12.0)
                    .color(Color32::from_rgb(0, 255, 200)),
            );

            ui.add_space(4.0);
            ui.separator();
            ui.add_space(4.0);

            let (loop_bg, loop_color, loop_stroke) = if state.loop_enabled {
                (
                    Color32::from_rgb(36, 28, 55),
                    Color32::from_rgb(200, 150, 255),
                    Stroke::new(1.2_f32, Color32::from_rgb(180, 120, 255)),
                )
            } else {
                (
                    Color32::from_rgb(26, 21, 36),
                    Color32::from_rgb(160, 155, 175),
                    Stroke::new(1.0_f32, Color32::from_rgb(45, 36, 60)),
                )
            };
            let loop_btn = egui::Button::new(
                RichText::new("🔁")
                    .size(11.0)
                    .color(loop_color),
            )
            .min_size(Vec2::new(24.0, 22.0))
            .fill(loop_bg)
            .stroke(loop_stroke)
            .rounding(Rounding::same(3.0));

            if ui
                .add(loop_btn)
                .on_hover_text(if state.loop_enabled {
                    "Loop Ativo (clique para desativar)"
                } else {
                    "Loop Desativado (clique para ativar)"
                })
                .clicked()
            {
                state.loop_enabled = !state.loop_enabled;
            }

            let (sel_bg, sel_color, sel_stroke) = if state.preview_selection_only {
                (
                    Color32::from_rgb(18, 42, 36),
                    Color32::from_rgb(0, 255, 200),
                    Stroke::new(1.2_f32, Color32::from_rgb(0, 255, 180)),
                )
            } else {
                (
                    Color32::from_rgb(26, 21, 36),
                    Color32::from_rgb(160, 155, 175),
                    Stroke::new(1.0_f32, Color32::from_rgb(45, 36, 60)),
                )
            };
            let sel_btn = egui::Button::new(
                RichText::new("🎯")
                    .size(11.0)
                    .color(sel_color),
            )
            .min_size(Vec2::new(24.0, 22.0))
            .fill(sel_bg)
            .stroke(sel_stroke)
            .rounding(Rounding::same(3.0));

            if ui
                .add(sel_btn)
                .on_hover_text(if state.preview_selection_only {
                    "Tocar Apenas Seleção (Ativo)"
                } else {
                    "Tocar Projeto Todo (Clique para tocar apenas notas selecionadas)"
                })
                .clicked()
            {
                state.preview_selection_only = !state.preview_selection_only;
            }
        });

        toolbar_card(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);

            let tools = [
                (EditTool::Pointer, "↖", "Selecionar / Mover (V)"),
                (EditTool::Pencil, "✎", "Inserir / Desenhar Nota (N)"),
                (EditTool::PitchDraw, "⌁", "Desenhar Pitch / Curva (P)"),
                (EditTool::Slice, "✂", "Cortar / Dividir Nota (C)"),
                (EditTool::Eraser, "⌫", "Apagar Notas (E)"),
            ];

            for (tool, label, tooltip) in tools {
                let is_selected = *current_tool == tool;
                let (bg_color, stroke_color, text_color) = if is_selected {
                    (
                        Color32::from_rgb(48, 38, 62),
                        Stroke::new(1.2_f32, Color32::from_rgb(255, 215, 0)),
                        Color32::from_rgb(255, 215, 0),
                    )
                } else {
                    (
                        Color32::from_rgb(26, 21, 36),
                        Stroke::new(1.0_f32, Color32::from_rgb(45, 36, 60)),
                        Color32::from_rgb(180, 175, 195),
                    )
                };

                let btn =
                    egui::Button::new(RichText::new(label).strong().size(12.0).color(text_color))
                        .min_size(Vec2::new(24.0, 22.0))
                        .fill(bg_color)
                        .stroke(stroke_color)
                        .rounding(Rounding::same(3.0));

                if ui.add(btn).on_hover_text(tooltip).clicked() {
                    *current_tool = tool;
                }
            }

            if *current_tool == EditTool::PitchDraw {
                ui.add_space(3.0);
                ui.separator();
                ui.add_space(2.0);

                for (subtool, label, tip) in [
                    (PitchSubTool::Freehand, "🖌", "Desenho livre"),
                    (PitchSubTool::Smooth, "🪄", "Suavizador"),
                    (PitchSubTool::Line, "📏", "Linha reta"),
                    (PitchSubTool::Vibrato, "〰", "Vibrato"),
                ] {
                    ui.selectable_value(pitch_sub_tool, subtool, label)
                        .on_hover_text(tip);
                }
            }

            ui.add_space(3.0);
            ui.separator();
            ui.add_space(2.0);

            let autopitch_btn = egui::Button::new(
                RichText::new("✨ AutoPitch")
                    .strong()
                    .size(11.0)
                    .color(Color32::from_rgb(0, 255, 180)),
            )
            .min_size(Vec2::new(72.0, 22.0))
            .fill(Color32::from_rgb(22, 34, 38))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0, 200, 150)))
            .rounding(Rounding::same(3.0));

            if ui
                .add(autopitch_btn)
                .on_hover_text("✨ AutoPitch: Gerar curvas de afinação orgânicas, overshoots e vibrato natural")
                .clicked()
            {
                on_open_autopitch();
            }
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(2.0);

            let log_bg = if *log_open {
                Color32::from_rgb(12, 44, 28)
            } else {
                Color32::from_rgb(26, 21, 36)
            };
            let log_stroke = if *log_open {
                Stroke::new(1.2_f32, Color32::from_rgb(0, 255, 157))
            } else {
                Stroke::new(1.0_f32, Color32::from_rgb(55, 44, 75))
            };
            let log_text_color = if *log_open {
                Color32::from_rgb(0, 255, 157)
            } else {
                Color32::from_rgb(180, 170, 200)
            };

            let log_btn = egui::Button::new(
                RichText::new(format!("Logs ({:.0}%)", state.render_progress * 100.0))
                    .size(11.0)
                    .color(log_text_color),
            )
            .fill(log_bg)
            .stroke(log_stroke)
            .rounding(Rounding::same(4.0));

            if ui
                .add(log_btn)
                .on_hover_text("Abrir/Fechar painel de logs de síntese")
                .clicked()
            {
                *log_open = !*log_open;
            }

            let is_exporting = state.render_progress < 0.99;
            let (export_label, export_bg, export_stroke, export_text_color) = if is_exporting {
                (
                    format!("⏳ Exportando ({:.0}%)", state.render_progress * 100.0),
                    Color32::from_rgb(20, 50, 60),
                    Stroke::new(1.2_f32, Color32::from_rgb(0, 220, 255)),
                    Color32::from_rgb(0, 255, 230),
                )
            } else {
                (
                    "⤓ Exportar WAV".to_string(),
                    Color32::from_rgb(38, 28, 56),
                    Stroke::new(1.0_f32, Color32::from_rgb(70, 52, 98)),
                    Color32::from_rgb(235, 230, 250),
                )
            };

            let export_btn = egui::Button::new(
                RichText::new(export_label)
                    .size(11.0)
                    .color(export_text_color),
            )
            .fill(export_bg)
            .stroke(export_stroke)
            .rounding(Rounding::same(4.0));

            if ui
                .add(export_btn)
                .on_hover_text("Exportar áudio renderizado para arquivo WAV")
                .clicked()
            {
                on_export_wav();
            }

            let copaiba_btn = egui::Button::new(
                RichText::new("🌿 Copaiba")
                    .size(11.0)
                    .color(Color32::from_rgb(200, 245, 210)),
            )
            .fill(Color32::from_rgb(18, 38, 28))
            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(38, 85, 58)))
            .rounding(Rounding::same(4.0));

            if ui
                .add(copaiba_btn)
                .on_hover_text("Assistente IA Vocal Copaiba")
                .clicked()
            {
                on_open_copaiba();
            }
        });
    });

    ui.horizontal(|ui| {
        ui.add_space(2.0);

        toolbar_card(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

            ui.add(
                egui::DragValue::new(&mut state.bpm)
                    .range(40.0..=300.0)
                    .speed(0.5)
                    .prefix("♩ ")
                    .suffix(" BPM"),
            )
            .on_hover_text("Andamento (Batidas por Minuto)");

            ui.separator();

            let snap_options = [
                (GridSnapOption::Freeform, "Livre"),
                (GridSnapOption::Snap1_1, "1/1"),
                (GridSnapOption::Snap1_2, "1/2"),
                (GridSnapOption::Snap1_4, "1/4"),
                (GridSnapOption::Snap1_8, "1/8"),
                (GridSnapOption::Snap1_16, "1/16"),
                (GridSnapOption::Snap1_32, "1/32"),
                (GridSnapOption::Snap1_64, "1/64"),
                (GridSnapOption::Snap1_128, "1/128"),
                (GridSnapOption::Snap1_4T, "1/4T (1/6)"),
                (GridSnapOption::Snap1_8T, "1/8T (1/12)"),
                (GridSnapOption::Snap1_16T, "1/16T (1/24)"),
                (GridSnapOption::Snap1_32T, "1/32T (1/48)"),
                (GridSnapOption::Snap1_64T, "1/64T (1/96)"),
            ];

            egui::ComboBox::from_id_salt("grid_snap_combo_unified")
                .selected_text(format!("Grade: {}", state.grid_snap.label()))
                .show_ui(ui, |ui| {
                    for (opt, label) in snap_options {
                        ui.selectable_value(&mut state.grid_snap, opt, label);
                    }
                });

            ui.separator();

            ui.toggle_value(&mut state.metronome_enabled, "🔔 Metrônomo")
                .on_hover_text("Ativar metrônomo durante a reprodução");

            ui.label(
                RichText::new("Contagem:")
                    .size(10.5)
                    .color(Color32::from_rgb(160, 150, 180)),
            );
            ui.add_sized(
                [42.0, 18.0],
                egui::DragValue::new(&mut state.count_in_bars)
                    .range(0..=4)
                    .suffix(" comp."),
            )
            .on_hover_text("Compassos de contagem prévia");
        });

        toolbar_card(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);

            ui.label(
                RichText::new("Loop A:")
                    .size(10.5)
                    .color(Color32::from_rgb(160, 150, 180)),
            );
            ui.add_sized(
                [64.0, 18.0],
                egui::DragValue::new(&mut state.loop_start_ms)
                    .range(0.0..=3_600_000.0)
                    .speed(10.0)
                    .suffix("ms"),
            )
            .on_hover_text("Início do Loop (milissegundos)");

            ui.label(
                RichText::new("B:")
                    .size(10.5)
                    .color(Color32::from_rgb(160, 150, 180)),
            );
            ui.add_sized(
                [64.0, 18.0],
                egui::DragValue::new(&mut state.loop_end_ms)
                    .range(1.0..=3_600_000.0)
                    .speed(10.0)
                    .suffix("ms"),
            )
            .on_hover_text("Fim do Loop (milissegundos)");

            if state.loop_end_ms <= state.loop_start_ms {
                state.loop_end_ms = state.loop_start_ms + 1.0;
            }
        });

        toolbar_card(ui, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);

            ui.label(
                RichText::new("Zoom X:")
                    .size(10.5)
                    .color(Color32::from_rgb(160, 150, 180)),
            );
            if ui
                .small_button("−")
                .on_hover_text("Diminuir Zoom Horizontal")
                .clicked()
            {
                *px_per_ms = (*px_per_ms * 0.8).max(0.05);
            }
            ui.add_sized(
                [50.0, 16.0],
                egui::Slider::new(px_per_ms, 0.05..=1.0).show_value(false),
            );
            if ui
                .small_button("+")
                .on_hover_text("Aumentar Zoom Horizontal")
                .clicked()
            {
                *px_per_ms = (*px_per_ms * 1.25).min(1.0);
            }

            ui.separator();

            ui.label(
                RichText::new("Zoom Y:")
                    .size(10.5)
                    .color(Color32::from_rgb(160, 150, 180)),
            );
            if ui
                .small_button("−")
                .on_hover_text("Diminuir Altura das Notas")
                .clicked()
            {
                *row_height = (*row_height * 0.85).max(12.0);
            }
            ui.add_sized(
                [50.0, 16.0],
                egui::Slider::new(row_height, 12.0..=48.0).show_value(false),
            );
            if ui
                .small_button("+")
                .on_hover_text("Aumentar Altura das Notas")
                .clicked()
            {
                *row_height = (*row_height * 1.15).min(48.0);
            }

            ui.separator();

            egui::ComboBox::from_id_salt("autoscroll_combo_unified")
                .selected_text(match auto_scroll_mode {
                    AutoScrollMode::Off => "Rolagem: Off",
                    AutoScrollMode::StationaryCursor => "Rolagem: Cursor",
                    AutoScrollMode::PageScroll => "Rolagem: Página",
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(auto_scroll_mode, AutoScrollMode::Off, "Desligada");
                    ui.selectable_value(
                        auto_scroll_mode,
                        AutoScrollMode::StationaryCursor,
                        "Cursor estacionário",
                    );
                    ui.selectable_value(auto_scroll_mode, AutoScrollMode::PageScroll, "Por página");
                });
        });

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(2.0);
            toolbar_card(ui, |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                ui.label(
                    RichText::new("🔊 Master:")
                        .size(10.5)
                        .color(Color32::from_rgb(180, 175, 195)),
                );
                ui.add_sized(
                    [80.0, 18.0],
                    egui::Slider::new(&mut state.master_volume, 0.0..=2.0)
                        .show_value(true)
                        .suffix("×"),
                )
                .on_hover_text("Volume Geral de Saída");
            });
        });
    });
}
