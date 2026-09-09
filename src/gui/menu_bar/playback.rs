use super::*;

impl KamafeuStudioApp {
    pub(super) fn menu_playback(&mut self, ui: &mut egui::Ui) {
        let lang = self.config.language;
        ui.menu_button(lang.tr("Reprodução", "Playback"), |ui| {
            let is_playing = self.audio_player.is_playing() || self.render_rx.is_some();
            if ui
                .button(if is_playing {
                    lang.tr("Pausar Reprodução (Espaço)", "Pause Playback (Space)")
                } else {
                    lang.tr("Tocar / Iniciar (Espaço)", "Play / Start (Space)")
                })
                .clicked()
            {
                if is_playing {
                    self.pause_audio();
                } else {
                    self.play_current_track();
                }
                ui.close_menu();
            }
            if ui
                .button(lang.tr(
                    "Parar e Ir para o Início (Esc)",
                    "Stop and Go to Start (Esc)",
                ))
                .clicked()
            {
                self.stop_audio();
                ui.close_menu();
            }
            ui.separator();
            if ui
                .button(lang.tr("Rebobinar para o Início (0ms)", "Rewind to Start (0ms)"))
                .clicked()
            {
                self.piano_roll_state.playhead_ms = 0.0;
                ui.close_menu();
            }
            if ui
                .button(lang.tr("Ir para o Final da Música", "Go to End of Project"))
                .clicked()
            {
                let max_end = self
                    .project
                    .parts
                    .iter()
                    .flat_map(|p| p.notes.iter())
                    .map(|n| n.position_ms + n.duration_ms)
                    .fold(0.0f64, f64::max);
                self.piano_roll_state.playhead_ms = max_end;
                ui.close_menu();
            }
            ui.separator();
            ui.menu_button(
                lang.tr("Velocidade de Reprodução", "Playback Speed"),
                |ui| {
                    let speeds = [
                        (
                            lang.tr("0.5x (Lenta / Estudo)", "0.5x (Slow / Practice)"),
                            0.5f64,
                        ),
                        (lang.tr("0.75x (Moderada)", "0.75x (Moderate)"), 0.75f64),
                        (lang.tr("1.0x (Normal)", "1.0x (Normal)"), 1.0f64),
                        (lang.tr("1.25x (Rápida)", "1.25x (Fast)"), 1.25f64),
                        (lang.tr("1.5x (Muito Rápida)", "1.5x (Very Fast)"), 1.5f64),
                    ];
                    for (label, spd) in speeds {
                        let is_active = (self.playback_speed_rate - spd).abs() < 0.01;
                        if ui.selectable_label(is_active, label).clicked() {
                            self.playback_speed_rate = spd;
                            self.audio_player.set_speed(spd as f32);
                            self.transport_state.status_message = if lang.is_en() {
                                format!("Playback speed set to {:.2}x", spd)
                            } else {
                                format!("Velocidade ajustada para {:.2}x", spd)
                            };
                            ui.close_menu();
                        }
                    }
                },
            );
            ui.separator();
            if ui
                .button(lang.tr("Forçar Pré-renderização do Áudio", "Force Audio Pre-render"))
                .clicked()
            {
                self.play_current_track();
                ui.close_menu();
            }
        });
    }
}
