use crate::gui::piano_roll::state::{MusicalScale, ROOT_NOTE_NAMES};
use crate::gui::theme::ThemeConfig;
use crate::gui::types::{AutoScrollMode, EditTool, GridSnapOption, PitchSubTool, TransportState};
use eframe::egui::{self, Color32, Frame, Margin, Pos2, Rect, RichText, Rounding, Stroke, Vec2};

fn toolbar_card<R>(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    Frame::none()
        .fill(theme.card_bg_c32())
        .rounding(theme.ui_rounding())
        .stroke(theme.card_stroke())
        .inner_margin(Margin::symmetric(5.0, 2.0))
        .show(ui, add_contents)
        .inner
}

fn draw_vu_meter(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    state: &mut TransportState,
    _is_playing: bool,
) {
    let (vu_rect, _) = ui.allocate_exact_size(Vec2::new(72.0, 22.0), egui::Sense::hover());
    let painter = ui.painter_at(vu_rect);

    // Frame
    painter.rect_filled(vu_rect, Rounding::same(3.0), theme.bg_canvas_c32());
    painter.rect_stroke(
        vu_rect,
        Rounding::same(3.0),
        Stroke::new(1.0, theme.grid_line_sub_c32()),
    );

    let bar_h = 5.0;
    let bar_max_w = vu_rect.width() - 18.0;

    for (ch_idx, (level, peak, label)) in [
        (state.vu_level_l, state.vu_peak_l, "L"),
        (state.vu_level_r, state.vu_peak_r, "R"),
    ]
    .iter()
    .enumerate()
    {
        let bar_y = vu_rect.min.y + 4.0 + ch_idx as f32 * (bar_h + 3.0);
        let bar_x_start = vu_rect.min.x + 12.0;

        // Label
        painter.text(
            Pos2::new(vu_rect.min.x + 3.0, bar_y + bar_h * 0.5),
            egui::Align2::LEFT_CENTER,
            *label,
            egui::FontId::monospace(8.0),
            theme.text_muted_c32(),
        );

        // Track
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(bar_x_start, bar_y),
                Pos2::new(bar_x_start + bar_max_w, bar_y + bar_h),
            ),
            Rounding::same(1.5),
            theme.c32_alpha(theme.bg_header, 0.7),
        );

        // Active meter
        let fill_w = bar_max_w * level.clamp(0.0, 1.0);
        if fill_w > 0.5 {
            let fill_rect = Rect::from_min_max(
                Pos2::new(bar_x_start, bar_y),
                Pos2::new(bar_x_start + fill_w, bar_y + bar_h),
            );
            let fill_color = if *level > 0.85 {
                Color32::from_rgb(255, 65, 85)
            } else if *level > 0.65 {
                Color32::from_rgb(255, 205, 45)
            } else {
                theme.note_fill_c32()
            };
            painter.rect_filled(fill_rect, Rounding::same(1.5), fill_color);
        }

        // Peak Hold
        let peak_x = bar_x_start + bar_max_w * peak.clamp(0.0, 1.0);
        if peak_x > bar_x_start + 1.0 {
            let peak_color = if *peak > 0.85 {
                Color32::from_rgb(255, 80, 100)
            } else {
                theme.playhead_c32()
            };
            painter.line_segment(
                [Pos2::new(peak_x, bar_y), Pos2::new(peak_x, bar_y + bar_h)],
                Stroke::new(1.5, peak_color),
            );
        }
    }
}

pub fn draw_unified_toolbar(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    lang: crate::config::AppLanguage,
    state: &mut TransportState,
    is_playing: bool,
    log_open: &mut bool,
    current_tool: &mut EditTool,
    pitch_sub_tool: &mut PitchSubTool,
    auto_scroll_mode: &mut AutoScrollMode,
    active_scale: &mut MusicalScale,
    scale_root_key: &mut u8,
    px_per_ms: &mut f32,
    _row_height: &mut f32,
    show_arrangement: &mut bool,
    show_drawer: &mut bool,
    show_phonemes: &mut bool,
    show_inspector: &mut bool,
    is_maximized: &mut bool,
    on_play: &mut dyn FnMut(),
    on_stop: &mut dyn FnMut(),
    on_export_wav: &mut dyn FnMut(),
    on_open_autopitch: &mut dyn FnMut(),
    on_quantize_snap: &mut dyn FnMut(),
    on_fix_overlaps: &mut dyn FnMut(),
) {
    ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);

    // ==========================================
    // SINGLE COMPACT LINE
    // ==========================================
    egui::ScrollArea::horizontal()
        .id_salt("toolbar_compact_scroll")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(2.0);

                // ── Card 1: Transport (Play / Stop / Time) ──
                toolbar_card(ui, theme, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);

                    let (play_bg, play_text, play_color, play_stroke) = if is_playing {
                        (
                            theme.c32_alpha(theme.accent_color, 0.2),
                            "⏸",
                            theme.accent_c32(),
                            theme.accent_c32(),
                        )
                    } else {
                        (
                            theme.c32_alpha(theme.note_fill, 0.2),
                            "▶",
                            theme.note_fill_c32(),
                            theme.note_stroke_c32(),
                        )
                    };

                    let play_btn = egui::Button::new(
                        RichText::new(play_text)
                            .strong()
                            .size(12.0)
                            .color(play_color),
                    )
                    .min_size(Vec2::new(24.0, 20.0))
                    .fill(play_bg)
                    .stroke(Stroke::new(1.0_f32, play_stroke))
                    .rounding(Rounding::same(3.0));

                    if ui
                        .add(play_btn)
                        .on_hover_text(lang.tr("Tocar / Pausar (Space)", "Play / Pause (Space)"))
                        .clicked()
                    {
                        on_play();
                    }

                    let stop_btn = egui::Button::new(
                        RichText::new("⏹")
                            .strong()
                            .size(12.0)
                            .color(Color32::from_rgb(255, 110, 110)),
                    )
                    .min_size(Vec2::new(24.0, 20.0))
                    .fill(Color32::from_rgba_unmultiplied(255, 70, 70, 35))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(180, 50, 50)))
                    .rounding(Rounding::same(3.0));

                    if ui
                        .add(stop_btn)
                        .on_hover_text(lang.tr(
                            "Parar e retornar ao início (Esc)",
                            "Stop and return to start (Esc)",
                        ))
                        .clicked()
                    {
                        on_stop();
                    }

                    ui.label(
                        RichText::new(&state.playhead_time_str)
                            .monospace()
                            .strong()
                            .size(11.0)
                            .color(theme.accent_c32()),
                    );

                    // Transport actions must not depend on the trailing
                    // overflow menu: it disappears first on compact windows.
                    // Keep the three playback modes next to Play/Stop, where
                    // they remain reachable without horizontal scrolling.
                    let transport_toggles = [
                        (
                            &mut state.loop_enabled,
                            "Loop",
                            lang.tr("Repetir intervalo A–B", "Repeat A–B range"),
                            theme.accent_c32(),
                        ),
                        (
                            &mut state.preview_selection_only,
                            "Sel",
                            lang.tr("Tocar apenas a seleção", "Play selection only"),
                            theme.note_fill_c32(),
                        ),
                        (
                            &mut state.metronome_enabled,
                            "Metr",
                            lang.tr("Ativar metrônomo", "Enable metronome"),
                            theme.playhead_c32(),
                        ),
                    ];
                    for (enabled, label, tooltip, active_color) in transport_toggles {
                        let (fill, stroke, text_color) = if *enabled {
                            (
                                Color32::from_rgba_unmultiplied(
                                    active_color.r(),
                                    active_color.g(),
                                    active_color.b(),
                                    64,
                                ),
                                Stroke::new(1.2, active_color),
                                active_color,
                            )
                        } else {
                            (
                                theme.bg_header_c32(),
                                Stroke::new(1.0, theme.grid_line_sub_c32()),
                                theme.text_muted_c32(),
                            )
                        };
                        let button =
                            egui::Button::new(RichText::new(label).size(9.5).color(text_color))
                                .min_size(Vec2::new(30.0, 20.0))
                                .fill(fill)
                                .stroke(stroke)
                                .rounding(Rounding::same(3.0));
                        if ui.add(button).on_hover_text(tooltip).clicked() {
                            *enabled = !*enabled;
                        }
                    }

                    // Keep the active loop range visible in the main toolbar;
                    // users should not have to open the overflow menu to know
                    // which part of the track will repeat.
                    ui.separator();
                    ui.label(
                        RichText::new("A")
                            .size(9.0)
                            .strong()
                            .color(theme.accent_c32()),
                    );
                    ui.add_sized(
                        [48.0, 20.0],
                        egui::DragValue::new(&mut state.loop_start_ms)
                            .range(0.0..=3_600_000.0)
                            .speed(10.0)
                            .suffix("ms"),
                    );
                    ui.label(
                        RichText::new("B")
                            .size(9.0)
                            .strong()
                            .color(theme.accent_c32()),
                    );
                    ui.add_sized(
                        [48.0, 20.0],
                        egui::DragValue::new(&mut state.loop_end_ms)
                            .range(1.0..=3_600_000.0)
                            .speed(10.0)
                            .suffix("ms"),
                    );
                    if state.loop_end_ms <= state.loop_start_ms {
                        state.loop_end_ms = state.loop_start_ms + 1.0;
                    }
                });

                // ── Card 2: Edit Tools ──
                toolbar_card(ui, theme, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);

                    let tools = [
                        (
                            EditTool::Pointer,
                            "Pnt",
                            lang.tr("Selecionar / Mover (V)", "Select / Move (V)"),
                        ),
                        (
                            EditTool::Pencil,
                            "Pen",
                            lang.tr("Inserir / Desenhar Nota (N)", "Insert / Draw Note (N)"),
                        ),
                        (
                            EditTool::PitchDraw,
                            "Pit",
                            lang.tr("Desenhar Pitch / Curva (P)", "Draw Pitch / Curve (P)"),
                        ),
                        (
                            EditTool::Slice,
                            "Cut",
                            lang.tr("Cortar / Dividir Nota (C)", "Cut / Split Note (C)"),
                        ),
                        (
                            EditTool::Eraser,
                            "Del",
                            lang.tr("Apagar Notas (E)", "Delete Notes (E)"),
                        ),
                    ];

                    for (tool, label, tooltip) in tools {
                        let is_selected = *current_tool == tool;
                        let (bg_color, stroke_color, text_color) = if is_selected {
                            (
                                theme.c32_alpha(theme.accent_color, 0.25),
                                Stroke::new(1.2_f32, theme.accent_c32()),
                                theme.accent_c32(),
                            )
                        } else {
                            (
                                theme.bg_header_c32(),
                                Stroke::new(1.0_f32, theme.grid_line_sub_c32()),
                                theme.text_muted_c32(),
                            )
                        };

                        let btn = egui::Button::new(
                            RichText::new(label).strong().size(10.0).color(text_color),
                        )
                        .min_size(Vec2::new(26.0, 20.0))
                        .fill(bg_color)
                        .stroke(stroke_color)
                        .rounding(Rounding::same(3.0));

                        if ui.add(btn).on_hover_text(tooltip).clicked() {
                            *current_tool = tool;
                        }
                    }

                    if *current_tool == EditTool::PitchDraw {
                        ui.add_space(2.0);
                        ui.separator();
                        ui.add_space(1.0);

                        for (subtool, label, tip) in [
                            (
                                PitchSubTool::Freehand,
                                lang.tr("Livre", "Free"),
                                lang.tr("Desenho livre", "Freehand drawing"),
                            ),
                            (
                                PitchSubTool::Smooth,
                                lang.tr("Suave", "Smooth"),
                                lang.tr("Suavizador", "Smoother"),
                            ),
                            (
                                PitchSubTool::Line,
                                lang.tr("Reta", "Line"),
                                lang.tr("Linha reta", "Straight line"),
                            ),
                            (PitchSubTool::Vibrato, "Vibrato", "Vibrato"),
                        ] {
                            ui.selectable_value(pitch_sub_tool, subtool, label)
                                .on_hover_text(tip);
                        }
                    }
                });

                // ── Card 3: BPM + Grid Snap ──
                toolbar_card(ui, theme, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);

                    ui.add(
                        egui::DragValue::new(&mut state.bpm)
                            .range(40.0..=300.0)
                            .speed(0.5)
                            .suffix(" BPM"),
                    )
                    .on_hover_text(
                        lang.tr("Andamento (Batidas por Minuto)", "Tempo (Beats Per Minute)"),
                    );

                    let snap_options = [
                        (
                            GridSnapOption::Auto,
                            lang.tr("Auto (Adaptativo ao Zoom)", "Auto (Adaptive to Zoom)"),
                        ),
                        (GridSnapOption::Freeform, lang.tr("Livre", "Off")),
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
                        .selected_text(
                            state
                                .grid_snap
                                .resolved_label_for(state.bpm, *px_per_ms, lang),
                        )
                        .width(60.0)
                        .show_ui(ui, |ui| {
                            for (opt, label) in snap_options {
                                ui.selectable_value(&mut state.grid_snap, opt, label);
                            }
                        });
                });

                // ── Card 4: Pre-tunning + Export ──
                toolbar_card(ui, theme, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);

                    let autopitch_btn = egui::Button::new(
                        RichText::new("Pre-tunning")
                            .strong()
                            .size(10.0)
                            .color(theme.accent_c32()),
                    )
                    .min_size(Vec2::new(65.0, 20.0))
                    .fill(theme.c32_alpha(theme.accent_color, 0.15))
                    .stroke(Stroke::new(1.0_f32, theme.accent_c32()))
                    .rounding(Rounding::same(3.0));

                    if ui
                        .add(autopitch_btn)
                        .on_hover_text(lang.tr(
                            "Pre-tunning: Gerar curvas de afinação orgânicas",
                            "Pre-tunning: Generate organic pitch curves",
                        ))
                        .clicked()
                    {
                        on_open_autopitch();
                    }

                    let export_btn = egui::Button::new(
                        RichText::new(lang.tr("Exportar", "Export"))
                            .size(10.0)
                            .color(theme.text_primary_c32()),
                    )
                    .min_size(Vec2::new(50.0, 20.0))
                    .fill(theme.bg_header_c32())
                    .stroke(Stroke::new(1.0_f32, theme.grid_line_bar_c32()))
                    .rounding(Rounding::same(3.0));

                    if ui
                        .add(export_btn)
                        .on_hover_text(lang.tr(
                            "Exportar áudio (WAV, FLAC, RAW PCM)",
                            "Export audio (WAV, FLAC, RAW PCM)",
                        ))
                        .clicked()
                    {
                        on_export_wav();
                    }
                });

                // ── Card 5: Panel Toggles ──
                toolbar_card(ui, theme, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);

                    let toggles: [(&mut bool, &str, &str, &str); 4] = [
                        (
                            show_arrangement,
                            lang.tr("Fxs", "Trk"),
                            lang.tr("Faixas / Arrangement (A)", "Tracks / Arrangement (A)"),
                            "panel_btn_arrangement",
                        ),
                        (
                            show_drawer,
                            lang.tr("Exp", "Exp"),
                            lang.tr(
                                "Expressões / Parâmetros (Tab)",
                                "Expressions / Parameters (Tab)",
                            ),
                            "panel_btn_drawer",
                        ),
                        (
                            show_phonemes,
                            lang.tr("Fon", "Pho"),
                            lang.tr("Fonemas / OTO (Alt+O)", "Phonemes / OTO (Alt+O)"),
                            "panel_btn_phonemes",
                        ),
                        (
                            show_inspector,
                            lang.tr("Ins", "Ins"),
                            lang.tr("Inspetor (Cmd+B / Ctrl+B)", "Inspector (Cmd+B / Ctrl+B)"),
                            "panel_btn_inspector",
                        ),
                    ];

                    for (flag, label, tooltip, id_str) in toggles {
                        let active = *flag;
                        let (bg, stroke, color) = if active {
                            (
                                theme.c32_alpha(theme.accent_color, 0.25),
                                Stroke::new(1.2_f32, theme.accent_c32()),
                                theme.accent_c32(),
                            )
                        } else {
                            (
                                theme.bg_header_c32(),
                                Stroke::new(1.0_f32, theme.grid_line_sub_c32()),
                                theme.text_muted_c32(),
                            )
                        };

                        let btn = egui::Button::new(RichText::new(label).size(9.5).color(color))
                            .min_size(Vec2::new(24.0, 20.0))
                            .fill(bg)
                            .stroke(stroke)
                            .rounding(Rounding::same(3.0));

                        let resp = ui.add(btn).on_hover_text(tooltip);
                        ui.ctx()
                            .data_mut(|d| d.insert_temp(egui::Id::new(id_str), resp.rect));

                        if resp.clicked() {
                            *flag = !*flag;
                        }
                    }

                    // Maximize
                    let max_active = *is_maximized;
                    let (max_bg, max_stroke, max_color) = if max_active {
                        (
                            theme.c32_alpha(theme.accent_color, 0.35),
                            Stroke::new(1.2_f32, theme.accent_c32()),
                            theme.accent_c32(),
                        )
                    } else {
                        (
                            theme.bg_header_c32(),
                            Stroke::new(1.0_f32, theme.grid_line_sub_c32()),
                            theme.text_muted_c32(),
                        )
                    };
                    let max_btn = egui::Button::new(RichText::new("⛶").size(10.0).color(max_color))
                        .min_size(Vec2::new(20.0, 20.0))
                        .fill(max_bg)
                        .stroke(max_stroke)
                        .rounding(Rounding::same(3.0));

                    if ui
                        .add(max_btn)
                        .on_hover_text(lang.tr(
                            "Maximizar Piano Roll (F11 / Shift+F)",
                            "Maximize Piano Roll (F11 / Shift+F)",
                        ))
                        .clicked()
                    {
                        *is_maximized = !*is_maximized;
                    }
                });

                // ── Card 6: ⋯ More (Popover) ──
                let more_btn = egui::Button::new(
                    RichText::new("⋯")
                        .size(12.0)
                        .strong()
                        .color(theme.text_muted_c32()),
                )
                .min_size(Vec2::new(24.0, 20.0))
                .fill(theme.bg_header_c32())
                .stroke(Stroke::new(1.0_f32, theme.grid_line_sub_c32()))
                .rounding(Rounding::same(3.0));

                let more_response = ui.add(more_btn).on_hover_text(lang.tr(
                    "Mais: Loop, Metrônomo, Escalas, Rolagem…",
                    "More: Loop, Metronome, Scales, Scroll…",
                ));

                let popup_id = ui.make_persistent_id("toolbar_more_popup");
                if more_response.clicked() {
                    ui.memory_mut(|mem| mem.toggle_popup(popup_id));
                }

                egui::popup::popup_below_widget(
                    ui,
                    popup_id,
                    &more_response,
                    egui::PopupCloseBehavior::CloseOnClickOutside,
                    |ui| {
                        ui.set_min_width(280.0);
                        ui.spacing_mut().item_spacing = Vec2::new(4.0, 4.0);

                        // --- Loop ---
                        ui.horizontal(|ui| {
                            let (loop_bg, loop_color, loop_stroke) = if state.loop_enabled {
                                (
                                    theme.c32_alpha(theme.accent_color, 0.25),
                                    theme.accent_c32(),
                                    Stroke::new(1.2_f32, theme.accent_c32()),
                                )
                            } else {
                                (
                                    theme.bg_header_c32(),
                                    theme.text_muted_c32(),
                                    Stroke::new(1.0_f32, theme.grid_line_sub_c32()),
                                )
                            };
                            let loop_btn = egui::Button::new(
                                RichText::new("Loop").size(10.5).color(loop_color),
                            )
                            .min_size(Vec2::new(34.0, 20.0))
                            .fill(loop_bg)
                            .stroke(loop_stroke)
                            .rounding(Rounding::same(3.0));
                            if ui.add(loop_btn).clicked() {
                                state.loop_enabled = !state.loop_enabled;
                            }

                            let (sel_bg, sel_color, sel_stroke) = if state.preview_selection_only {
                                (
                                    theme.c32_alpha(theme.note_fill, 0.25),
                                    theme.note_fill_c32(),
                                    Stroke::new(1.2_f32, theme.note_stroke_c32()),
                                )
                            } else {
                                (
                                    theme.bg_header_c32(),
                                    theme.text_muted_c32(),
                                    Stroke::new(1.0_f32, theme.grid_line_sub_c32()),
                                )
                            };
                            let sel_btn =
                                egui::Button::new(RichText::new("Sel").size(10.5).color(sel_color))
                                    .min_size(Vec2::new(28.0, 20.0))
                                    .fill(sel_bg)
                                    .stroke(sel_stroke)
                                    .rounding(Rounding::same(3.0));
                            if ui
                                .add(sel_btn)
                                .on_hover_text(
                                    lang.tr("Tocar Apenas Seleção", "Play Selection Only"),
                                )
                                .clicked()
                            {
                                state.preview_selection_only = !state.preview_selection_only;
                            }
                        });

                        // --- Loop A / B ---
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Loop A:")
                                    .size(10.0)
                                    .color(theme.text_muted_c32()),
                            );
                            ui.add_sized(
                                [52.0, 18.0],
                                egui::DragValue::new(&mut state.loop_start_ms)
                                    .range(0.0..=3_600_000.0)
                                    .speed(10.0)
                                    .suffix("ms"),
                            );
                            ui.label(RichText::new("B:").size(10.0).color(theme.text_muted_c32()));
                            ui.add_sized(
                                [52.0, 18.0],
                                egui::DragValue::new(&mut state.loop_end_ms)
                                    .range(1.0..=3_600_000.0)
                                    .speed(10.0)
                                    .suffix("ms"),
                            );
                            if state.loop_end_ms <= state.loop_start_ms {
                                state.loop_end_ms = state.loop_start_ms + 1.0;
                            }
                        });

                        ui.separator();

                        // --- Metronome ---
                        ui.horizontal(|ui| {
                            ui.toggle_value(
                                &mut state.metronome_enabled,
                                lang.tr("Metrônomo", "Metronome"),
                            );
                            ui.label(
                                RichText::new(lang.tr("Contagem:", "Count-in:"))
                                    .size(10.0)
                                    .color(theme.text_muted_c32()),
                            );
                            ui.add_sized(
                                [38.0, 18.0],
                                egui::DragValue::new(&mut state.count_in_bars)
                                    .range(0..=4)
                                    .suffix(" c."),
                            );
                        });

                        ui.separator();

                        // --- Quantize & Fix Overlaps ---
                        ui.horizontal(|ui| {
                            let quant_btn = egui::Button::new(
                                RichText::new("Snap").size(10.5).color(theme.accent_c32()),
                            )
                            .min_size(Vec2::new(40.0, 19.0))
                            .fill(theme.c32_alpha(theme.accent_color, 0.15))
                            .stroke(Stroke::new(1.0_f32, theme.accent_c32()))
                            .rounding(Rounding::same(3.0));

                            if ui
                                .add(quant_btn)
                                .on_hover_text(
                                    lang.tr("Alinhar notas para a grade", "Align notes to grid"),
                                )
                                .clicked()
                            {
                                on_quantize_snap();
                            }

                            let overlap_btn = egui::Button::new(
                                RichText::new(lang.tr("Sobreposição", "Overlap"))
                                    .size(10.5)
                                    .color(theme.note_fill_c32()),
                            )
                            .min_size(Vec2::new(40.0, 19.0))
                            .fill(theme.c32_alpha(theme.note_fill, 0.15))
                            .stroke(Stroke::new(1.0_f32, theme.note_stroke_c32()))
                            .rounding(Rounding::same(3.0));

                            if ui
                                .add(overlap_btn)
                                .on_hover_text(
                                    lang.tr("Corrigir notas sobrepostas", "Fix overlapping notes"),
                                )
                                .clicked()
                            {
                                on_fix_overlaps();
                            }
                        });

                        ui.separator();

                        // --- Auto Scroll ---
                        ui.horizontal(|ui| {
                            egui::ComboBox::from_id_salt("autoscroll_combo_popup")
                                .selected_text(match auto_scroll_mode {
                                    AutoScrollMode::Off => lang.tr("Rolagem: Off", "Scroll: Off"),
                                    AutoScrollMode::StationaryCursor => {
                                        lang.tr("Rolagem: Cursor", "Scroll: Cursor")
                                    }
                                    AutoScrollMode::PageScroll => {
                                        lang.tr("Rolagem: Página", "Scroll: Page")
                                    }
                                })
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(
                                        auto_scroll_mode,
                                        AutoScrollMode::Off,
                                        lang.tr("Desligada", "Off"),
                                    );
                                    ui.selectable_value(
                                        auto_scroll_mode,
                                        AutoScrollMode::StationaryCursor,
                                        lang.tr("Cursor estacionário", "Stationary cursor"),
                                    );
                                    ui.selectable_value(
                                        auto_scroll_mode,
                                        AutoScrollMode::PageScroll,
                                        lang.tr("Por página", "Page scroll"),
                                    );
                                });
                        });

                        // --- Scale Guide ---
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(lang.tr("Guia:", "Guide:"))
                                    .size(10.5)
                                    .strong()
                                    .color(theme.accent_c32()),
                            );

                            egui::ComboBox::from_id_salt("toolbar_scale_root_popup")
                                .selected_text(
                                    ROOT_NOTE_NAMES
                                        .get(*scale_root_key as usize)
                                        .copied()
                                        .unwrap_or("C"),
                                )
                                .width(36.0)
                                .show_ui(ui, |ui| {
                                    for (k_idx, k_name) in ROOT_NOTE_NAMES.iter().enumerate() {
                                        ui.selectable_value(scale_root_key, k_idx as u8, *k_name);
                                    }
                                });

                            egui::ComboBox::from_id_salt("toolbar_scale_type_popup")
                                .selected_text(active_scale.display_name())
                                .width(120.0)
                                .show_ui(ui, |ui| {
                                    for scale in MusicalScale::ALL {
                                        ui.selectable_value(
                                            active_scale,
                                            scale,
                                            scale.display_name(),
                                        );
                                    }
                                });
                        });

                        ui.separator();

                        // --- Console ---
                        ui.horizontal(|ui| {
                            let log_bg = if *log_open {
                                theme.c32_alpha(theme.accent_color, 0.25)
                            } else {
                                theme.bg_header_c32()
                            };
                            let log_text_color = if *log_open {
                                theme.accent_c32()
                            } else {
                                theme.text_muted_c32()
                            };

                            let log_btn = egui::Button::new(
                                RichText::new(if *log_open { "Console ●" } else { "Console" })
                                    .size(10.5)
                                    .color(log_text_color),
                            )
                            .min_size(Vec2::new(60.0, 20.0))
                            .fill(log_bg)
                            .rounding(Rounding::same(3.0));

                            if ui
                                .add(log_btn)
                                .on_hover_text(lang.tr(
                                    "Abrir/Fechar console de renderização",
                                    "Open/Close render console",
                                ))
                                .clicked()
                            {
                                *log_open = !*log_open;
                            }
                        });
                    },
                );

                // ── Card 7: Master Volume + VU ──
                toolbar_card(ui, theme, |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                    draw_vu_meter(ui, theme, state, is_playing);
                    ui.separator();
                    ui.label(
                        RichText::new("Vol:")
                            .size(9.0)
                            .color(theme.text_muted_c32()),
                    );
                    ui.add_sized(
                        [48.0, 16.0],
                        egui::Slider::new(&mut state.master_volume, 0.0..=2.0)
                            .show_value(true)
                            .suffix("×"),
                    )
                    .on_hover_text(lang.tr("Volume Geral de Saída", "Master Output Volume"));
                });

                // ── Inline status (replaces marquee) ──
                if !state.voicebank_name.is_empty() {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(format!("🎤 {}", &state.voicebank_name))
                            .size(9.5)
                            .color(theme.text_muted_c32()),
                    )
                    .on_hover_text(lang.tr("Voicebank carregado", "Loaded voicebank"));
                }
            });
        });
}
