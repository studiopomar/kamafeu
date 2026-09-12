use crate::gui::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    #[allow(dead_code)]
    pub(super) fn draw_led_marquee(&self, ui: &mut egui::Ui) {
        let theme = &self.config.theme;
        let rect = ui.available_rect_before_wrap();

        ui.painter()
            .rect_filled(rect, theme.ui_rounding(), theme.card_bg_c32());
        ui.painter()
            .rect_stroke(rect, theme.ui_rounding(), theme.card_stroke());

        let is_rendering = self.render_progress < 0.99 && self.render_rx.is_some();
        let is_playing = self.audio_player.is_playing();
        let lang = self.config.language;

        let total_w = rect.width();
        let right_w = 260.0_f32.min(total_w * 0.40).max(160.0);
        let left_w = (total_w - right_w).max(40.0);

        let left_rect = egui::Rect::from_min_size(rect.min, egui::Vec2::new(left_w, rect.height()));
        let right_rect = egui::Rect::from_min_size(
            egui::Pos2::new(rect.min.x + left_w, rect.min.y),
            egui::Vec2::new(right_w, rect.height()),
        );

        // 1. LEFT ZONE: Status Badge, Voicebank, Engine, Phonemizer, Playhead, Metronome
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(left_rect), |ui| {
            ui.set_clip_rect(left_rect);
            ui.horizontal(|ui| {
                ui.add_space(8.0);

                let (status_dot_color, status_text) = if is_rendering {
                    (theme.accent_c32(), lang.tr("RENDERIZANDO", "RENDERING"))
                } else if is_playing {
                    (theme.accent_c32(), lang.tr("REPRODUZINDO", "PLAYING"))
                } else {
                    (theme.text_muted_c32(), lang.tr("● PRONTO", "● READY"))
                };

                ui.label(
                    egui::RichText::new(status_text)
                        .strong()
                        .size(10.5)
                        .color(status_dot_color),
                );

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                let vb_full = self
                    .voicebank
                    .as_ref()
                    .map(|v| v.name.as_str())
                    .unwrap_or(lang.tr("Nenhum Voicebank", "No Voicebank"));

                let vb_short = if vb_full.chars().count() > 20 {
                    let s: String = vb_full.chars().take(18).collect();
                    format!("{s}...")
                } else {
                    vb_full.to_string()
                };

                ui.label(
                    egui::RichText::new("VB:")
                        .size(10.0)
                        .color(theme.text_muted_c32()),
                );
                ui.label(
                    egui::RichText::new(vb_short)
                        .strong()
                        .size(10.0)
                        .color(theme.text_primary_c32()),
                )
                .on_hover_text(format!("Voicebank: {vb_full}"));

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                let resampler_short = if self.selected_resampler.contains("straycat") {
                    "straycat"
                } else if self.selected_resampler.contains("Hifisampler")
                    || self.selected_resampler.contains("hifisampler")
                {
                    "Hifisampler"
                } else if self.selected_resampler.contains("Venus")
                    || self.selected_resampler.contains("venus")
                {
                    "Venus"
                } else if self.selected_resampler.contains("TD-PSOLA") {
                    "TD-PSOLA"
                } else if self.selected_resampler.contains("SOLA") {
                    "SOLA"
                } else if self.selected_resampler.contains("Organum")
                    || self.selected_resampler.contains("organum")
                {
                    "Organum"
                } else {
                    self.selected_resampler
                        .split(&['/', '\\'][..])
                        .last()
                        .unwrap_or(&self.selected_resampler)
                };

                ui.label(
                    egui::RichText::new("Engine:")
                        .size(10.0)
                        .color(theme.text_muted_c32()),
                );
                ui.label(
                    egui::RichText::new(resampler_short)
                        .strong()
                        .size(10.0)
                        .color(theme.text_primary_c32()),
                )
                .on_hover_text(format!("Resampler: {}", self.selected_resampler));

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                let (phonemizer_short, phonemizer_full) = match self
                    .vocal_mode_params
                    .phonemizer_mode
                {
                    crate::phonemizer::PhonemizerMode::None => (
                        "Manual",
                        lang.tr("Manual (Sem Fonemizador)", "Manual (No Phonemizer)"),
                    ),
                    crate::phonemizer::PhonemizerMode::BasicCV => ("JA: CV", "JA: Basic CV"),
                    crate::phonemizer::PhonemizerMode::VCV => ("JA: VCV", "JA: VCV"),
                    crate::phonemizer::PhonemizerMode::CVVC => ("JA: CVVC", "JA: CVVC"),
                    crate::phonemizer::PhonemizerMode::EnglishArpasing => (
                        "EN: Arpasing",
                        lang.tr("EN: Arpasing (Fonética)", "EN: Arpasing (Phonetic)"),
                    ),
                    crate::phonemizer::PhonemizerMode::EnglishVCCV => (
                        "EN: VCCV",
                        lang.tr("EN: VCCV (Fonética)", "EN: VCCV (Phonetic)"),
                    ),
                    crate::phonemizer::PhonemizerMode::EnglishG2P => ("EN: G2P", "EN: English G2P"),
                    crate::phonemizer::PhonemizerMode::PortugueseBrapaVCCV => {
                        ("PT: VCCV", "PT: VCCV BRAPA (xiao)")
                    }
                    crate::phonemizer::PhonemizerMode::PortugueseBrapaCVC => (
                        "PT: CVC",
                        lang.tr("PT: BRAPA CVC (Fonética)", "PT: BRAPA CVC (Phonetic)"),
                    ),
                    crate::phonemizer::PhonemizerMode::PortugueseCVVC => (
                        "PT: CVVC",
                        lang.tr("PT: CVVC (Fonética)", "PT: CVVC (Phonetic)"),
                    ),
                    crate::phonemizer::PhonemizerMode::PortugueseVCV => (
                        "PT: VCV",
                        lang.tr("PT: VCV (Fonética)", "PT: VCV (Phonetic)"),
                    ),
                    crate::phonemizer::PhonemizerMode::PortugueseG2P => (
                        "PT: G2P",
                        lang.tr("PT: Português G2P", "PT: Portuguese G2P"),
                    ),
                };

                ui.label(
                    egui::RichText::new(lang.tr("Fonet:", "Phon:"))
                        .size(10.0)
                        .color(theme.text_muted_c32()),
                );
                ui.label(
                    egui::RichText::new(phonemizer_short)
                        .strong()
                        .size(10.0)
                        .color(theme.accent_c32()),
                )
                .on_hover_text(format!(
                    "{}: {phonemizer_full}",
                    lang.tr("Fonetizador", "Phonemizer")
                ));

                if is_playing {
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);
                    let cur_ms = self.piano_roll_state.playhead_ms.max(0.0);
                    let mins = (cur_ms / 60000.0) as usize;
                    let secs = ((cur_ms % 60000.0) / 1000.0) as usize;
                    let ms_rem = (cur_ms % 1000.0) as usize;
                    ui.label(
                        egui::RichText::new(format!("{:02}:{:02}.{:03}", mins, secs, ms_rem))
                            .strong()
                            .size(10.0)
                            .color(theme.accent_c32()),
                    );
                }

                if self.transport_state.metronome_enabled {
                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);
                    let beat_ms = 60_000.0 / self.transport_state.bpm.max(1.0);
                    let beat = ((self.piano_roll_state.playhead_ms / beat_ms).floor() as i64)
                        .rem_euclid(4)
                        + 1;
                    let accent = if beat == 1 { "●" } else { "○" };
                    ui.label(
                        egui::RichText::new(format!("{accent} {beat}/4"))
                            .size(9.5)
                            .color(if beat == 1 {
                                theme.accent_c32()
                            } else {
                                theme.text_muted_c32()
                            }),
                    )
                    .on_hover_text(lang.tr("Metrônomo ativo", "Metronome active"));
                }
            });
        });

        // 2. RIGHT ZONE: Progress Bar, Percentage, and FPS Counter (Always clipped & bounded)
        ui.allocate_new_ui(egui::UiBuilder::new().max_rect(right_rect), |ui| {
            ui.set_clip_rect(right_rect);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(8.0);

                let fps = if self.frame_time_ema_ms > 0.0 {
                    1_000.0 / self.frame_time_ema_ms
                } else {
                    0.0
                };
                let fps_color = if fps >= 55.0 {
                    theme.accent_c32()
                } else if fps >= 30.0 {
                    theme.text_primary_c32()
                } else {
                    theme.text_muted_c32()
                };
                ui.label(
                    egui::RichText::new(format!("{fps:.0} FPS"))
                        .monospace()
                        .size(9.5)
                        .color(fps_color),
                )
                .on_hover_text(format!(
                    "{}: {:.1} ms ({fps:.0} FPS)",
                    lang.tr("Tempo de quadro", "Frame time"),
                    self.frame_time_ema_ms
                ));

                ui.add_space(4.0);
                ui.separator();
                ui.add_space(4.0);

                let (progress_fraction, progress_color, pct_text) = if is_rendering {
                    let p = self.render_progress.clamp(0.0, 1.0) as f32;
                    (p, theme.accent_c32(), format!("{:.0}%", p * 100.0))
                } else if is_playing {
                    let total_ms = self
                        .project
                        .parts
                        .iter()
                        .flat_map(|p| p.notes.iter())
                        .map(|n| n.position_ms + n.duration_ms)
                        .fold(0.0_f64, f64::max)
                        .max(1000.0);
                    let cur_ms = self.piano_roll_state.playhead_ms.max(0.0);
                    let p = (cur_ms / total_ms).clamp(0.0, 1.0) as f32;
                    (p, theme.accent_c32(), format!("{:.0}%", p * 100.0))
                } else {
                    (1.0, theme.text_muted_c32(), "100%".to_string())
                };

                ui.label(
                    egui::RichText::new(pct_text)
                        .strong()
                        .size(10.0)
                        .color(theme.text_primary_c32()),
                );

                ui.add_space(4.0);

                let bar_width = 70.0_f32;
                let bar_height = 6.0_f32;
                let (bar_rect, _) = ui.allocate_exact_size(
                    egui::Vec2::new(bar_width, bar_height),
                    egui::Sense::hover(),
                );

                ui.painter()
                    .rect_filled(bar_rect, theme.ui_rounding(), theme.card_bg_c32());

                if progress_fraction > 0.001 {
                    let filled_width = bar_width * progress_fraction;
                    let filled_rect = egui::Rect::from_min_size(
                        bar_rect.min,
                        egui::Vec2::new(filled_width, bar_height),
                    );
                    ui.painter()
                        .rect_filled(filled_rect, theme.ui_rounding(), progress_color);
                }

                ui.painter()
                    .rect_stroke(bar_rect, theme.ui_rounding(), theme.card_stroke());

                if is_rendering {
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("[Render]")
                            .size(9.0)
                            .strong()
                            .color(theme.accent_c32()),
                    )
                    .on_hover_text(&self.render_status_title);
                }
            });
        });
    }
}
