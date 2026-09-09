use crate::gui::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(crate) fn render_templates_dialog(&mut self, ctx: &egui::Context) {
        if !self.templates_dialog_open {
            return;
        }

        let lang = self.config.language;
        let mut is_open = self.templates_dialog_open;
        let mut load_template_name: Option<&'static str> = None;

        ctx.show_viewport_immediate(
            egui::ViewportId::from_hash_of("templates_native_viewport"),
            egui::ViewportBuilder::default()
                .with_title(lang.tr("Novo Projeto a partir de Modelo - Kamafeu Studio", "New Project from Template - Kamafeu Studio"))
                .with_inner_size([620.0, 440.0])
                .with_min_inner_size([480.0, 320.0]),
            |ctx, _class| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.heading(
                            egui::RichText::new(lang.tr("Escolha um Modelo de Projeto", "Choose a Project Template"))
                                .strong()
                                .color(egui::Color32::from_rgb(0, 255, 180)),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.button(lang.tr("Cancelar", "Cancel")).clicked() {
                                is_open = false;
                            }
                        });
                    });
                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    let templates = [
                        (
                            "pop",
                            lang.tr("Pop / Voz-a-loide", "Pop / Vocal Synth"),
                            lang.tr("120 BPM · 4/4 · Escala C Maior", "120 BPM · 4/4 · C Major Scale"),
                            lang.tr(
                                "Configuração com faixa de Vocal Principal e faixa de Vocal Guia (acordes de apoio).",
                                "Setup with Lead Vocal track and Guide Vocal track (backing chords).",
                            ),
                        ),
                        (
                            "ballad",
                            lang.tr("Balada Acústica", "Acoustic Ballad"),
                            lang.tr("80 BPM · 4/4 · Escala A Menor", "80 BPM · 4/4 · A Minor Scale"),
                            lang.tr(
                                "Configuração expressiva com Vocal Principal e Faixa de Harmonia Suave em pan estéreo.",
                                "Expressive setup with Lead Vocal and Soft Harmony track with stereo panning.",
                            ),
                        ),
                        (
                            "rock",
                            lang.tr("Rock / Uptempo", "Rock / Uptempo"),
                            lang.tr("160 BPM · 4/4 · Escala E Menor", "160 BPM · 4/4 · E Minor Scale"),
                            lang.tr(
                                "Configuração enérgica com Vocal Lead agressivo e Backing Vocals L/R abertos.",
                                "High-energy setup with aggressive Lead Vocal and wide L/R Backing Vocals.",
                            ),
                        ),
                        (
                            "choir",
                            lang.tr("Arranjo Coral (4 Vozes)", "Choir Arrangement (4 Voices)"),
                            lang.tr("100 BPM · 4/4 · SATB", "100 BPM · 4/4 · SATB"),
                            lang.tr(
                                "Quatro faixas vocais distribuídas em Soprano, Alto, Tenor e Baixo com pan espacial.",
                                "Four vocal tracks spread across Soprano, Alto, Tenor, and Bass with spatial panning.",
                            ),
                        ),
                        (
                            "empty",
                            lang.tr("Projeto Vazio Padrão", "Default Empty Project"),
                            lang.tr("120 BPM · 4/4 · Livre", "120 BPM · 4/4 · Free"),
                            lang.tr(
                                "Uma única pista vocal limpa para começar do zero.",
                                "A single clean vocal track to start from scratch.",
                            ),
                        ),
                    ];

                    for (id, title, meta, desc) in templates {
                        egui::Frame::none()
                            .fill(egui::Color32::from_rgb(26, 20, 38))
                            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(52, 40, 72)))
                            .rounding(egui::Rounding::same(6.0))
                            .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.vertical(|ui| {
                                        ui.horizontal(|ui| {
                                            ui.label(egui::RichText::new(title).strong().size(13.0).color(egui::Color32::WHITE));
                                            ui.label(egui::RichText::new(format!("({})", meta)).size(10.5).color(crate::gui::theme::MelodyneTheme::TEXT_MUTED));
                                        });
                                        ui.label(egui::RichText::new(desc).size(10.5).color(egui::Color32::from_rgb(180, 175, 200)));
                                    });
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if ui.button(egui::RichText::new(lang.tr("Criar Projeto", "Create Project")).strong().color(egui::Color32::from_rgb(0, 255, 180))).clicked() {
                                            load_template_name = Some(id);
                                        }
                                    });
                                });
                            });
                        ui.add_space(6.0);
                    }
                });
                if ctx.input(|i| i.viewport().close_requested()) {
                    is_open = false;
                }
            },
        );

        if let Some(t_id) = load_template_name {
            self.load_project_template(t_id);
            is_open = false;
        }

        self.templates_dialog_open = is_open;
    }
}
