use crate::gui::piano_roll::state::{MusicalScale, ROOT_NOTE_NAMES};
use crate::gui::theme::ThemeConfig;
use crate::gui::types::{AutoScrollMode, EditTool, GridSnapOption, PitchSubTool, TransportState};
use eframe::egui::{self, Color32, Frame, Margin, Pos2, Rect, RichText, Rounding, Stroke, Vec2};

use crate::gui::visual_fx as fx;

/// Duration used for state colour transitions. Follows the global egui
/// animation time, which `app_frame` sets to zero when animations are off.
fn transition_time(ui: &egui::Ui) -> f32 {
    ui.style().animation_time.max(0.0) * 1.25
}

fn toolbar_card<R>(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    id_salt: &str,
    add_contents: impl FnOnce(&mut egui::Ui) -> R,
) -> R {
    // Reserved before the frame so the hover halo sits behind the card.
    let glow_slot = fx::reserve(ui.painter());
    let radius = theme.ui_corner_radius.max(5.0);
    // Toolbar groups should read as one continuous control surface, rather
    // than a row of heavy outlined boxes.
    let frame = Frame::none()
        .fill(theme.c32_alpha(theme.bg_header, 0.58))
        .rounding(Rounding::same(radius))
        .stroke(Stroke::new(
            0.6_f32,
            theme.c32_alpha(theme.grid_line_sub, 0.34),
        ))
        .inner_margin(Margin::symmetric(4.0, 2.0))
        .show(ui, add_contents);

    let rect = frame.response.rect;
    let hovered = ui.rect_contains_pointer(rect);
    let t = ui.ctx().animate_bool_with_time(
        egui::Id::new("toolbar_card_hover").with(id_salt),
        hovered,
        transition_time(ui) * 1.4,
    );
    if t > 0.001 {
        let accent = theme.accent_c32();
        fx::paint_glow_into(
            ui.painter(),
            glow_slot,
            rect,
            radius,
            fx::with_alpha(accent, 0.22 * t),
            10.0,
        );
        ui.painter().rect_stroke(
            rect,
            radius,
            Stroke::new(0.9_f32, fx::with_alpha(accent, 0.55 * t)),
        );
    }
    frame.inner
}

/// Smoothly interpolated (fill, stroke, text) for an on/off control.
fn toggle_style(
    ui: &egui::Ui,
    theme: &ThemeConfig,
    id: egui::Id,
    active: bool,
    active_color: Color32,
    active_fill_alpha: f32,
) -> (Color32, Stroke, Color32) {
    let t = ui
        .ctx()
        .animate_bool_with_time(id.with("toggle_anim"), active, transition_time(ui));
    let fill = fx::lerp_color(
        theme.bg_header_c32(),
        fx::with_alpha(active_color, active_fill_alpha),
        t,
    );
    let stroke = Stroke::new(
        1.0 + 0.2 * t,
        fx::lerp_color(theme.grid_line_sub_c32(), active_color, t),
    );
    let text = fx::lerp_color(theme.text_muted_c32(), active_color, t);
    (fill, stroke, text)
}

/// Glowing underline that slides between the selected buttons of a group.
fn paint_selection_slider(ui: &egui::Ui, theme: &ThemeConfig, id: &str, target: Option<Rect>) {
    let Some(target) = target else {
        return;
    };
    let rect = fx::animate_rect(
        ui.ctx(),
        egui::Id::new("toolbar_slider").with(id),
        target,
        transition_time(ui) * 1.6,
    );
    let bar = Rect::from_min_max(
        Pos2::new(rect.left() + 4.0, rect.bottom() - 2.4),
        Pos2::new(rect.right() - 4.0, rect.bottom() - 0.6),
    );
    if bar.width() <= 0.0 {
        return;
    }
    let accent = theme.accent_c32();
    fx::paint_glow(ui.painter(), bar, 1.0, fx::with_alpha(accent, 0.75), 6.0);
    ui.painter().rect_filled(bar, 1.0, fx::lighten(accent, 0.25));
}

fn draw_vu_meter(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    state: &mut TransportState,
    _is_playing: bool,
) {
    let (vu_rect, _) = ui.allocate_exact_size(Vec2::new(84.0, 22.0), egui::Sense::hover());
    let painter = ui.painter_at(vu_rect);

    fx::paint_vertical_gradient(
        &painter,
        vu_rect.shrink(0.5),
        theme.c32_alpha(theme.bg_canvas, 0.95),
        theme.c32_alpha(theme.bg_header, 0.95),
    );
    painter.rect_stroke(
        vu_rect,
        theme.ui_rounding(),
        Stroke::new(1.0_f32, theme.c32_alpha(theme.grid_line_sub, 0.6)),
    );

    let palette = fx::MeterPalette {
        low: theme.meter_c32(),
        mid: theme.warning_c32(),
        high: theme.danger_c32(),
        off: theme.c32_alpha(theme.grid_line_sub, 0.22),
    };
    let bar_h = 6.0;
    for (ch_idx, (level, peak, label)) in [
        (state.vu_level_l, state.vu_peak_l, "L"),
        (state.vu_level_r, state.vu_peak_r, "R"),
    ]
    .iter()
    .enumerate()
    {
        let bar_y = vu_rect.min.y + 3.5 + ch_idx as f32 * (bar_h + 2.5);
        painter.text(
            Pos2::new(vu_rect.min.x + 3.0, bar_y + bar_h * 0.5),
            egui::Align2::LEFT_CENTER,
            *label,
            egui::FontId::monospace(8.0),
            theme.text_muted_c32(),
        );
        let bar = Rect::from_min_size(
            Pos2::new(vu_rect.min.x + 12.0, bar_y),
            Vec2::new(vu_rect.width() - 15.0, bar_h),
        );
        fx::paint_led_meter(&painter, bar, *level, *peak, &palette);
    }
}

/// Compact oscilloscope of the audio that just played.
fn draw_live_scope(ui: &mut egui::Ui, theme: &ThemeConfig, state: &TransportState) {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(74.0, 22.0), egui::Sense::hover());
    let painter = ui.painter_at(rect);
    fx::paint_vertical_gradient(
        &painter,
        rect.shrink(0.5),
        theme.c32_alpha(theme.bg_canvas, 0.95),
        theme.c32_alpha(theme.bg_header, 0.95),
    );
    // Faint graticule.
    for i in 1..4 {
        let x = egui::lerp(rect.left()..=rect.right(), i as f32 / 4.0);
        painter.line_segment(
            [Pos2::new(x, rect.top() + 2.0), Pos2::new(x, rect.bottom() - 2.0)],
            Stroke::new(0.5_f32, theme.c32_alpha(theme.grid_line_sub, 0.25)),
        );
    }
    let peak = state
        .scope
        .iter()
        .map(|(min, max)| min.abs().max(max.abs()))
        .fold(0.0f32, f32::max);
    let gain = if peak > 1e-4 { (0.9 / peak).clamp(1.0, 4.0) } else { 1.0 };
    let color = theme.waveform_c32();
    if peak > 0.01 {
        fx::paint_glow(&painter, rect.shrink(6.0), 2.0, fx::with_alpha(color, 0.10), 8.0);
    }
    fx::paint_scope(&painter, rect.shrink2(Vec2::new(2.0, 2.0)), &state.scope, color, gain);
    painter.rect_stroke(
        rect,
        theme.ui_rounding(),
        Stroke::new(1.0_f32, theme.c32_alpha(theme.grid_line_sub, 0.6)),
    );
    response.on_hover_text("Osciloscópio · 1,6 s / Scope · 1.6 s");
}

pub fn draw_unified_toolbar(
    ui: &mut egui::Ui,
    theme: &ThemeConfig,
    lang: crate::config::AppLanguage,
    state: &mut TransportState,
    is_playing: bool,
    log_open: &mut bool,
    current_tool: &mut EditTool,
    _pitch_sub_tool: &mut PitchSubTool,
    _pitch_smooth_intensity: &mut f32,
    auto_scroll_mode: &mut AutoScrollMode,
    vertical_pitch_follow: &mut bool,
    vertical_follow_smoothing_ms: &mut f32,
    horizontal_follow_smoothing_ms: &mut f32,
    follow_cursor_pct: &mut f32,
    follow_min_pct: &mut f32,
    follow_max_pct: &mut f32,
    active_scale: &mut MusicalScale,
    scale_root_key: &mut u8,
    px_per_ms: &mut f32,
    _row_height: &mut f32,
    show_arrangement: &mut bool,
    show_drawer: &mut bool,
    show_phonemes: &mut bool,
    show_envelope: &mut bool,
    show_inspector: &mut bool,
    is_maximized: &mut bool,
    on_play: &mut dyn FnMut(),
    on_stop: &mut dyn FnMut(),
    on_export_wav: &mut dyn FnMut(),
    on_open_autopitch: &mut dyn FnMut(),
    on_quantize_snap: &mut dyn FnMut(),
    on_fix_overlaps: &mut dyn FnMut(),
    on_envelope_acpt: &mut dyn FnMut(),
    on_envelope_p2p3: &mut dyn FnMut(),
    on_envelope_p1p4: &mut dyn FnMut(),
    on_envelope_opt: &mut dyn FnMut(),
    on_envelope_reset: &mut dyn FnMut(),
) {
    let touch_first = crate::gui::touch_first_layout(ui.ctx());

    // Alguns controles do toolbar têm um tamanho mínimo próprio. Reforçamos
    // aqui a área de interação para que esses valores não deixem alvos
    // pequenos demais no celular.
    ui.spacing_mut().item_spacing = if touch_first {
        Vec2::new(7.0, 4.0)
    } else {
        Vec2::new(3.0, 0.0)
    };
    ui.spacing_mut().interact_size = if touch_first {
        Vec2::new(44.0, 44.0)
    } else {
        Vec2::new(24.0, 24.0)
    };

    // ==========================================
    // SINGLE COMPACT LINE
    // ==========================================
    egui::ScrollArea::horizontal()
        .id_salt("toolbar_compact_scroll")
        .drag_to_scroll(true)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(2.0);

                // ── Card 1: Transport (Play / Stop / Time) ──
                toolbar_card(ui, theme, "transport", |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);

                    // O estado de reprodução tem um pulso suave: ele comunica
                    // atividade contínua sem virar decoração piscante.
                    let time = ui.input(|input| input.time);
                    let breath = fx::breathe(time, 0.9, state.motion);
                    let play_t = ui.ctx().animate_bool_with_time(
                        egui::Id::new("toolbar_play_state"),
                        is_playing,
                        transition_time(ui) * 1.5,
                    );
                    let idle_bg = theme.c32_alpha(theme.note_fill, 0.2);
                    let live_bg = theme.c32_alpha(theme.accent_color, 0.16 + 0.14 * breath);
                    let play_bg = fx::lerp_color(idle_bg, live_bg, play_t);
                    let play_color =
                        fx::lerp_color(theme.note_fill_c32(), theme.accent_c32(), play_t);
                    let play_stroke =
                        fx::lerp_color(theme.note_stroke_c32(), theme.accent_c32(), play_t);
                    let play_text = if is_playing { "⏸" } else { "▶" };

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

                    let play_glow = fx::reserve(ui.painter());
                    let play_response = ui
                        .add(play_btn)
                        .on_hover_text(lang.tr("Tocar / Pausar (Space)", "Play / Pause (Space)"));
                    let hover_t = ui.ctx().animate_bool_with_time(
                        play_response.id.with("hover_glow"),
                        play_response.hovered(),
                        transition_time(ui),
                    );
                    let glow_strength = (play_t * (0.35 + 0.40 * breath)).max(hover_t * 0.35);
                    if glow_strength > 0.01 {
                        fx::paint_glow_into(
                            ui.painter(),
                            play_glow,
                            play_response.rect,
                            3.0,
                            fx::with_alpha(
                                fx::lerp_color(theme.note_fill_c32(), theme.accent_c32(), play_t),
                                glow_strength,
                            ),
                            8.0 + 4.0 * breath * play_t,
                        );
                    }
                    if play_response.clicked() {
                        on_play();
                    }

                    let stop_btn = egui::Button::new(
                        RichText::new("⏹")
                            .strong()
                            .size(12.0)
                            .color(theme.danger_c32()),
                    )
                    .min_size(Vec2::new(24.0, 20.0))
                    .fill(theme.c32_alpha([235, 85, 85], 0.16))
                    .stroke(Stroke::new(1.0_f32, theme.danger_c32()))
                    .rounding(Rounding::same(3.0));

                    let stop_glow = fx::reserve(ui.painter());
                    let stop_response = ui.add(stop_btn).on_hover_text(lang.tr(
                        "Parar e retornar ao início (Esc)",
                        "Stop and return to start (Esc)",
                    ));
                    let stop_hover = ui.ctx().animate_bool_with_time(
                        stop_response.id.with("hover_glow"),
                        stop_response.hovered(),
                        transition_time(ui),
                    );
                    if stop_hover > 0.01 {
                        fx::paint_glow_into(
                            ui.painter(),
                            stop_glow,
                            stop_response.rect,
                            3.0,
                            fx::with_alpha(theme.danger_c32(), 0.4 * stop_hover),
                            8.0,
                        );
                    }
                    if stop_response.clicked() {
                        on_stop();
                    }

                    let time_color = fx::lerp_color(
                        theme.text_primary_c32(),
                        fx::lighten(theme.accent_c32(), 0.15 * breath),
                        0.55 + 0.45 * play_t,
                    );
                    ui.add_sized(
                        [65.0, 20.0],
                        egui::Label::new(
                            RichText::new(&state.playhead_time_str)
                                .monospace()
                                .strong()
                                .size(11.0)
                                .color(time_color),
                        ),
                    );

                    // Render activity: an orbiting spinner plus percentage
                    // that grows in/out instead of popping.
                    let busy_t = ui.ctx().animate_bool_with_time(
                        egui::Id::new("toolbar_busy"),
                        state.busy,
                        transition_time(ui) * 2.0,
                    );
                    if busy_t > 0.01 {
                        let width = 40.0 * fx::ease_out_cubic(busy_t);
                        let (rect, response) =
                            ui.allocate_exact_size(Vec2::new(width, 20.0), egui::Sense::hover());
                        let painter = ui.painter_at(rect);
                        let accent = fx::with_alpha(theme.accent_c32(), busy_t);
                        fx::paint_orbit_spinner(
                            &painter,
                            Pos2::new(rect.left() + 8.0, rect.center().y),
                            5.5,
                            time,
                            accent,
                            state.motion,
                        );
                        painter.text(
                            Pos2::new(rect.left() + 16.0, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            format!("{:.0}%", state.render_progress.clamp(0.0, 1.0) * 100.0),
                            egui::FontId::monospace(9.0),
                            accent,
                        );
                        response.on_hover_text(lang.tr("Renderizando…", "Rendering…"));
                    }

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
                        let toggle_id = egui::Id::new("transport_toggle").with(label);
                        let (fill, stroke, text_color) =
                            toggle_style(ui, theme, toggle_id, *enabled, active_color, 0.25);
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

                    ui.separator();
                    let mut scroll_selected_rect = None;
                    for (mode, label, tooltip) in [
                        (
                            AutoScrollMode::Off,
                            "H:Off",
                            lang.tr("Rolagem horizontal desligada", "Horizontal scrolling off"),
                        ),
                        (
                            AutoScrollMode::StationaryCursor,
                            "H:Cur",
                            lang.tr(
                                "Cursor horizontal estacionário",
                                "Stationary horizontal cursor",
                            ),
                        ),
                        (
                            AutoScrollMode::PageScroll,
                            "H:Pg",
                            lang.tr("Rolagem horizontal por página", "Horizontal page scrolling"),
                        ),
                    ] {
                        let selected = *auto_scroll_mode == mode;
                        let (fill, stroke, text_color) = toggle_style(
                            ui,
                            theme,
                            egui::Id::new("scroll_mode").with(label),
                            selected,
                            theme.accent_c32(),
                            0.22,
                        );
                        let button = egui::Button::new(RichText::new(label).size(9.5).color(text_color))
                            .min_size(Vec2::new(42.0, 18.0))
                            .fill(fill)
                            .stroke(stroke);
                        let response = ui.add(button).on_hover_text(tooltip);
                        if selected {
                            scroll_selected_rect = Some(response.rect);
                        }
                        if response.clicked() {
                            *auto_scroll_mode = mode;
                        }
                        if mode == AutoScrollMode::StationaryCursor {
                            response.context_menu(|ui| {
                                ui.label(lang.tr("Acompanhamento horizontal", "Horizontal follow"));
                                ui.add(egui::Slider::new(horizontal_follow_smoothing_ms, 0.0..=1000.0).suffix(" ms").text(lang.tr("Suavidade", "Smoothing")));
                                ui.add(egui::Slider::new(follow_cursor_pct, 0.0..=100.0).suffix("%").text(lang.tr("Posição do cursor", "Cursor position")));
                                ui.add(egui::Slider::new(follow_min_pct, 0.0..=98.0).suffix("%").text(lang.tr("Limite inicial", "Start limit")));
                                ui.add(egui::Slider::new(follow_max_pct, 2.0..=100.0).suffix("%").text(lang.tr("Limite final", "End limit")));
                            });
                        }
                    }
                    paint_selection_slider(ui, theme, "scroll_mode", scroll_selected_rect);

                    let (v_fill, v_stroke, v_text) = toggle_style(
                        ui,
                        theme,
                        egui::Id::new("vertical_follow"),
                        *vertical_pitch_follow,
                        theme.note_fill_c32(),
                        0.22,
                    );
                    let vertical_button = egui::Button::new(RichText::new("V:Nt").size(9.5).color(v_text))
                        .min_size(Vec2::new(42.0, 18.0))
                        .fill(v_fill)
                        .stroke(v_stroke);
                    let vertical_response = ui
                        .add(vertical_button)
                        .on_hover_text(lang.tr(
                            "Acompanhar automaticamente as notas na vertical",
                            "Automatically follow notes vertically",
                        ))
                        ;
                    if vertical_response.clicked() {
                        *vertical_pitch_follow = !*vertical_pitch_follow;
                    }
                    vertical_response.context_menu(|ui| {
                        ui.label(lang.tr("Acompanhamento vertical", "Vertical follow"));
                        ui.add(egui::Slider::new(vertical_follow_smoothing_ms, 0.0..=1000.0).suffix(" ms").text(lang.tr("Suavidade", "Smoothing")));
                    });

                });

                // ── Card 2: Edit Tools ──
                toolbar_card(ui, theme, "tools", |ui| {
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

                    let mut tool_selected_rect = None;
                    for (tool, label, tooltip) in tools {
                        let is_selected = *current_tool == tool;
                        let (bg_color, stroke_color, text_color) = toggle_style(
                            ui,
                            theme,
                            egui::Id::new("tool_btn").with(label),
                            is_selected,
                            theme.accent_c32(),
                            0.25,
                        );

                        let btn = egui::Button::new(
                            RichText::new(label).strong().size(10.0).color(text_color),
                        )
                        .min_size(Vec2::new(26.0, 20.0))
                        .fill(bg_color)
                        .stroke(stroke_color)
                        .rounding(Rounding::same(3.0));

                        let resp = ui.add(btn).on_hover_text(tooltip);
                        if is_selected {
                            tool_selected_rect = Some(resp.rect);
                        }
                        if resp.clicked() {
                            *current_tool = tool;
                        }
                    }
                    paint_selection_slider(ui, theme, "edit_tools", tool_selected_rect);

                    if *current_tool == EditTool::PitchDraw {
                        ui.add_space(2.0);
                        ui.separator();
                        ui.add_space(1.0);

                        ui.label(
                            RichText::new(lang.tr("Pontos / segmentos", "Points / segments"))
                                .small()
                                .color(theme.accent_c32()),
                        )
                        .on_hover_text(lang.tr(
                            "Clique na curva para criar um ponto; arraste para editar; clique direito para excluir ou escolher a forma.",
                            "Click the curve to create a point; drag to edit; right-click to delete or choose the shape.",
                        ));
                    }
                });

                // ── Card 3: BPM + Grid Snap ──
                toolbar_card(ui, theme, "tempo_grid", |ui| {
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
                        (GridSnapOption::Snap1_5, "1/5"),
                        (GridSnapOption::Snap1_10, "1/10"),
                        (GridSnapOption::Snap1_20, "1/20"),
                        (GridSnapOption::Snap1_40, "1/40"),
                        (GridSnapOption::Snap1_80, "1/80"),
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
                toolbar_card(ui, theme, "export_actions", |ui| {
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

                // ── Card 5: UTAU Envelopes & Crossfades (ACPT, P2P3, P1P4, OPT, RESET) ──
                toolbar_card(ui, theme, "envelopes", |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);

                    let env_buttons: [(&str, &str, &mut dyn FnMut(), Color32); 5] = [
                        (
                            "ACPT",
                            lang.tr(
                                "ACPT: Ajustar envelopes ao overlap (Auto Crossfade UTAU)",
                                "ACPT: Auto-adjust envelope crossfades to overlap (UTAU Auto Crossfade)",
                            ),
                            on_envelope_acpt,
                            ui.visuals().hyperlink_color,
                        ),
                        (
                            "P2P3",
                            lang.tr(
                                "P2P3: Suavizar transição de overlap entre notas e fonemas VC/CV",
                                "P2P3: Smooth overlap transition between notes and VC/CV phonemes",
                            ),
                            on_envelope_p2p3,
                            ui.visuals().hyperlink_color,
                        ),
                        (
                            "P1P4",
                            lang.tr(
                                "P1P4: Ajustar margens de envelope (p1=0, p4=0)",
                                "P1P4: Zero envelope margins (p1=0, p4=0)",
                            ),
                            on_envelope_p1p4,
                            ui.visuals().text_color().linear_multiply(0.72),
                        ),
                        (
                            "OPT",
                            lang.tr(
                                "OPT: Otimizar envelopes e transições proporcionalmente",
                                "OPT: Optimize envelopes and transitions proportionally",
                            ),
                            on_envelope_opt,
                            ui.visuals().hyperlink_color,
                        ),
                        (
                            "RESET",
                            lang.tr(
                                "RESET: Restaurar envelopes de volume para o padrão UTAU (5/35/0/35)",
                                "RESET: Reset volume envelopes to default UTAU (5/35/0/35)",
                            ),
                            on_envelope_reset,
                            ui.visuals().warn_fg_color,
                        ),
                    ];

                    for (label, tooltip, action, text_color) in env_buttons {
                        let btn = egui::Button::new(
                            RichText::new(label)
                                .strong()
                                .size(9.5)
                                .color(text_color),
                        )
                        .min_size(Vec2::new(24.0, 18.0))
                        .fill(theme.bg_header_c32())
                        .stroke(Stroke::new(1.0_f32, theme.grid_line_sub_c32()))
                        .rounding(Rounding::same(3.0));

                        if ui.add(btn).on_hover_text(tooltip).clicked() {
                            action();
                        }
                    }
                });

                // ── Card 6: Panels ──
                toolbar_card(ui, theme, "panels", |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);

                    let toggles: [(&mut bool, &str, &str, &str); 5] = [
                        (
                            show_arrangement,
                            lang.tr("Arr", "Arr"),
                            lang.tr(
                                "Painel de Arranjo / Multifaixas (A)",
                                "Arrangement / Multitrack Panel (A)",
                            ),
                            "panel_btn_arrangement",
                        ),
                        (
                            show_drawer,
                            lang.tr("Par", "Par"),
                            lang.tr(
                                "Painel de Parâmetros e Expressões (Tab)",
                                "Parameters & Expressions Panel (Tab)",
                            ),
                            "panel_btn_drawer",
                        ),
                        (
                            show_phonemes,
                            lang.tr("Fon", "Pho"),
                            lang.tr(
                                "Régua de Fonemas e Envelopes do oto.ini (Alt+O)",
                                "Phoneme & oto.ini Envelope Ruler (Alt+O)",
                            ),
                            "panel_btn_phonemes",
                        ),
                        (
                            show_envelope,
                            lang.tr("Env", "Env"),
                            lang.tr(
                                "Exibir / Editar Envelopes de Volume (O)",
                                "Show / Edit Volume Envelopes (O)",
                            ),
                            "panel_btn_envelope",
                        ),
                        (
                            show_inspector,
                            lang.tr("Ins", "Ins"),
                            lang.tr(
                                "Inspetor contextual (Cmd+B / Ctrl+B)",
                                "Contextual Inspector (Cmd+B / Ctrl+B)",
                            ),
                            "panel_btn_inspector",
                        ),
                    ];

                    for (flag, label, tooltip, id_str) in toggles {
                        let active = *flag;
                        let (bg, stroke, color) = toggle_style(
                            ui,
                            theme,
                            egui::Id::new("panel_toggle").with(label),
                            active,
                            theme.accent_c32(),
                            0.25,
                        );

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
                            if *flag && *is_maximized {
                                *is_maximized = false;
                            }
                        }
                    }

                    // Maximize
                    let max_active = *is_maximized;
                    let (max_bg, max_stroke, max_color) = toggle_style(
                        ui,
                        theme,
                        egui::Id::new("panel_max_btn"),
                        max_active,
                        theme.accent_c32(),
                        0.35,
                    );
                    let max_btn = egui::Button::new(RichText::new("⛶").size(10.0).color(max_color))
                        .min_size(Vec2::new(20.0, 20.0))
                        .fill(max_bg)
                        .stroke(max_stroke)
                        .rounding(Rounding::same(3.0));

                    if ui
                        .add(max_btn)
                        .on_hover_text(lang.tr(
                            "Maximizar / Focar Piano Roll (Shift+F / Alt+M)",
                            "Maximize / Focus Piano Roll (Shift+F / Alt+M)",
                        ))
                        .clicked()
                    {
                        *is_maximized = !*is_maximized;
                        if !*is_maximized && !*show_arrangement && !*show_inspector && !*show_phonemes {
                            *show_arrangement = true;
                            *show_inspector = true;
                            *show_phonemes = true;
                        }
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

                        // --- Scale Guide ---
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(lang.tr("Guia:", "Guide:"))
                                    .size(10.5)
                                    .strong()
                                    .color(theme.accent_c32()),
                            );

                            let root_label = ROOT_NOTE_NAMES
                                .get(*scale_root_key as usize)
                                .copied()
                                .unwrap_or("C");

                            ui.menu_button(
                                RichText::new(root_label).size(10.5).color(theme.text_primary_c32()),
                                |ui| {
                                    for (k_idx, k_name) in ROOT_NOTE_NAMES.iter().enumerate() {
                                        if ui
                                            .selectable_label(*scale_root_key as usize == k_idx, *k_name)
                                            .clicked()
                                        {
                                            *scale_root_key = k_idx as u8;
                                            ui.close_menu();
                                        }
                                    }
                                },
                            );

                            ui.menu_button(
                                RichText::new(active_scale.display_name())
                                    .size(10.5)
                                    .color(theme.accent_c32()),
                                |ui| {
                                    for scale in MusicalScale::ALL {
                                        if ui
                                            .selectable_label(*active_scale == scale, scale.display_name())
                                            .clicked()
                                        {
                                            *active_scale = scale;
                                            ui.close_menu();
                                        }
                                    }
                                },
                            );
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

                // ── Card 7: Master Volume + VU + Scope ──
                toolbar_card(ui, theme, "master", |ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(3.0, 0.0);
                    draw_live_scope(ui, theme, state);
                    ui.separator();
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
