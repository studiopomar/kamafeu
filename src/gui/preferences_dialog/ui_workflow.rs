use super::help_marker;
use super::section_card;
use super::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(in crate::gui) fn render_ui_workflow_tab(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
    ) {
        let lang = self.config.language;
        section_card(
            ui,
            lang.tr("Idioma & Localização", "Language & Locale"),
            |ui| {
                ui.horizontal(|ui| {
                ui.label(lang.tr("Idioma da Interface:", "Interface Language:"));
                for (l, label) in [
                    (crate::config::AppLanguage::PtBr, lang.tr("Brasileiro (Brasil)", "Portuguese (Brazil)")),
                    (crate::config::AppLanguage::EnUs, lang.tr("Inglês (Global)", "English (Global)")),
                ] {
                    let is_active = self.config.language == l;
                    if ui.selectable_label(is_active, label).clicked() {
                        self.config.language = l;
                        self.persist_config();
                    }
                }
                help_marker(
                    ui,
                    lang.tr(
                        "Define o idioma padrão dos menus, tooltips e painéis de controle do Kamafeu Studio.",
                        "Sets the default language for Kamafeu Studio menus, tooltips, and control panels.",
                    ),
                );
            });
            },
        );

        section_card(
            ui,
            lang.tr(
                "Escala da Interface Gráfica & Acessibilidade",
                "UI Scale & Accessibility",
            ),
            |ui| {
                ui.horizontal(|ui| {
                ui.label(lang.tr("Escala de Zoom da Interface (UI Scale):", "UI Scale Factor:"));
                let mut scale = self.config.ui_scale_factor;
                if ui.add(egui::Slider::new(&mut scale, 0.75..=2.0).step_by(0.05).custom_formatter(|v, _| format!("{:.0}%", v * 100.0))).changed() {
                    self.config.ui_scale_factor = scale;
                    ctx.set_pixels_per_point(scale);
                }
                help_marker(
                    ui,
                    lang.tr(
                        "Aumenta ou reduz o tamanho dos textos, botões e controles para monitores 4K ou telas compactas.",
                        "Increases or decreases the size of texts, buttons, and controls for 4K monitors or compact screens.",
                    ),
                );
            });
            },
        );

        section_card(
            ui,
            lang.tr(
                "Fluxo de Trabalho, Salvamento & Histórico Undo",
                "Workflow, Saving & Undo History",
            ),
            |ui| {
                ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.config.workflow.auto_save_enabled,
                    lang.tr("Ativar Salvamento Automático Periódico (Auto-save)", "Enable Periodic Auto-save"),
                );
                if self.config.workflow.auto_save_enabled {
                    ui.add(egui::Slider::new(&mut self.config.workflow.auto_save_interval_sec, 30..=600).suffix(lang.tr(" seg", " sec")));
                }
                help_marker(
                    ui,
                    lang.tr(
                        "Salva um arquivo de recuperação do projeto a cada intervalo determinado para proteger contra imprevistos.",
                        "Saves a project recovery file at regular intervals to protect against unforeseen crashes.",
                    ),
                );
            });

                ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.config.workflow.backup_on_save,
                    lang.tr("Criar arquivo de backup (.bak) ao salvar o projeto", "Create backup file (.bak) when saving project"),
                );
                help_marker(
                    ui,
                    lang.tr(
                        "Mantém uma cópia da versão anterior sempre que você salvar manualmente o projeto.",
                        "Keeps a copy of the previous version whenever you manually save the project.",
                    ),
                );
            });

                ui.horizontal(|ui| {
                    ui.label(lang.tr(
                        "Passos Máximos de Desfazer (Undo History):",
                        "Maximum Undo Steps:",
                    ));
                    ui.add(
                        egui::Slider::new(&mut self.config.workflow.max_undo_steps, 20..=500)
                            .suffix(lang.tr(" passos", " steps")),
                    );
                    help_marker(
                        ui,
                        lang.tr(
                            "Número máximo de ações que podem ser desfeitas com Ctrl+Z.",
                            "Maximum number of actions that can be undone with Ctrl+Z.",
                        ),
                    );
                });

                ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.config.workflow.note_audition_on_click,
                    lang.tr("Tocar tom da nota ao clicar/arrastar no Piano Roll", "Audition note pitch on click/drag in Piano Roll"),
                );
                help_marker(
                    ui,
                    lang.tr(
                        "Emite o som da nota ao clicar no teclado guia ou selecionar uma nota.",
                        "Plays the note tone when clicking on the piano guide keys or selecting a note.",
                    ),
                );
            });

                ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.config.discord_rpc_enabled,
                    lang.tr("Integração Discord Rich Presence (Exibir projeto e status no Discord)", "Discord Rich Presence Integration (Show project and status in Discord)"),
                );
                help_marker(
                    ui,
                    lang.tr(
                        "Exibe seu status no perfil do Discord mostrando que está trabalhando no Kamafeu Studio.",
                        "Shows your status on your Discord profile indicating you are working in Kamafeu Studio.",
                    ),
                );
            });

                ui.horizontal(|ui| {
                ui.checkbox(
                    &mut self.config.workflow.confirm_on_exit_dirty,
                    lang.tr("Pedir confirmação ao fechar o programa caso haja alterações não salvas", "Confirm before closing if there are unsaved changes"),
                );
                help_marker(
                    ui,
                    lang.tr(
                        "Exibe aviso de confirmação para salvar o projeto antes de fechar a janela.",
                        "Prompts for confirmation to save the project before closing the window.",
                    ),
                );
            });
            },
        );
    }
}
