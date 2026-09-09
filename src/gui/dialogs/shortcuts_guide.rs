use crate::gui::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(super) fn show_shortcuts_guide(&mut self, ctx: &egui::Context) {
        if self.shortcuts_guide_open {
            let lang = self.config.language;
            let mut is_open = self.shortcuts_guide_open;
            ctx.show_viewport_immediate(
                egui::ViewportId::from_hash_of("shortcuts_guide_native_viewport"),
                egui::ViewportBuilder::default()
                    .with_title(lang.tr("Guia de Teclas de Atalho - Kamafeu Studio", "Keyboard Shortcuts Guide - Kamafeu Studio"))
                    .with_inner_size([580.0, 520.0])
                    .with_min_inner_size([460.0, 360.0]),
                |ctx, _class| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        ui.horizontal(|ui| {
                            ui.heading(
                                egui::RichText::new(lang.tr("Teclas de Atalho do Kamafeu Studio", "Kamafeu Studio Keyboard Shortcuts"))
                                    .strong()
                                    .color(egui::Color32::from_rgb(0, 255, 157)),
                            );
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.button(lang.tr("Fechar", "Close")).clicked() {
                                    is_open = false;
                                }
                            });
                        });
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);

                        egui::ScrollArea::vertical()
                            .id_salt("shortcuts_guide_scroll")
                            .show(ui, |ui| {
                                egui::Grid::new("shortcuts_grid")
                                    .striped(true)
                                    .spacing([20.0, 8.0])
                                    .show(ui, |ui| {
                                        ui.label(
                                            egui::RichText::new(lang.tr("Atalho", "Shortcut"))
                                                .strong()
                                                .color(egui::Color32::from_rgb(255, 215, 0)),
                                        );
                                        ui.label(
                                            egui::RichText::new(lang.tr("Ação / Funcionalidade", "Action / Functionality"))
                                                .strong()
                                                .color(egui::Color32::from_rgb(255, 215, 0)),
                                        );
                                        ui.end_row();

                                        ui.label(lang.tr("Espaço (Space)", "Space"));
                                        ui.label(lang.tr("Tocar / Pausar Reprodução", "Play / Pause Playback"));
                                        ui.end_row();
                                        ui.label("Esc");
                                        ui.label(lang.tr("Parar e Reiniciar Cursor no Início (0ms)", "Stop and Return Playhead to Start (0ms)"));
                                        ui.end_row();
                                        ui.label("V / 1");
                                        ui.label(lang.tr(
                                            "Ferramenta Ponteiro (Seleção / Mover / Redimensionar)",
                                            "Pointer Tool (Select / Move / Resize)",
                                        ));
                                        ui.end_row();
                                        ui.label("N / 2");
                                        ui.label(lang.tr("Ferramenta Lápis (Desenhar Notas)", "Pencil Tool (Draw Notes)"));
                                        ui.end_row();
                                        ui.label("P / 3");
                                        ui.label(lang.tr("Ferramenta Pitch (Livre / Reta / Vibrato / Suave)", "Pitch Tool (Free / Line / Vibrato / Smooth)"));
                                        ui.end_row();
                                        ui.label("Shift + P");
                                        ui.label(lang.tr("Alternar Submodo de Pitch (Livre / Reta / Vibrato / Suave)", "Toggle Pitch Submode (Free / Line / Vibrato / Smooth)"));
                                        ui.end_row();
                                        ui.label("Ctrl+Alt+P / Cmd+Alt+P");
                                        ui.label(lang.tr("Abrir Pre-tunning (Afinador Orgânico)", "Open Pre-tunning (Organic Tuner)"));
                                        ui.end_row();
                                        ui.label("Ctrl+Alt+T / Cmd+Alt+T");
                                        ui.label(lang.tr("Personalizar Tema, Cores e Cantos da Interface", "Customize UI Theme, Colors & Corners"));
                                        ui.end_row();
                                        ui.label("Ctrl+, / Cmd+,");
                                        ui.label(lang.tr("Abrir Preferências & Configurações Avançadas", "Open Preferences & Advanced Settings"));
                                        ui.end_row();
                                        ui.label(lang.tr("Duplo-clique / Shift+Click na curva", "Double-click / Shift+Click on curve"));
                                        ui.label(lang.tr("Adicionar novo ponto de ancoragem no pitch", "Add new pitch anchor point"));
                                        ui.end_row();
                                        ui.label(lang.tr("Alt+Click / Clique Direito na âncora", "Alt+Click / Right Click on anchor"));
                                        ui.label(lang.tr("Deletar ponto de ancoragem específico do pitch", "Delete specific pitch anchor point"));
                                        ui.end_row();
                                        ui.label("E / 4");
                                        ui.label(lang.tr("Ferramenta Borracha (Apagar Notas)", "Eraser Tool (Erase Notes)"));
                                        ui.end_row();
                                        ui.label("Ctrl+Z / Cmd+Z");
                                        ui.label(lang.tr("Desfazer Ação", "Undo Action"));
                                        ui.end_row();
                                        ui.label("Ctrl+Y / Cmd+Shift+Z");
                                        ui.label(lang.tr("Refazer Ação", "Redo Action"));
                                        ui.end_row();
                                        ui.label("Ctrl+X / Cmd+X");
                                        ui.label(lang.tr("Recortar Nota(s) Selecionada(s)", "Cut Selected Note(s)"));
                                        ui.end_row();
                                        ui.label("Ctrl+C / Cmd+C");
                                        ui.label(lang.tr("Copiar Nota(s) Selecionada(s)", "Copy Selected Note(s)"));
                                        ui.end_row();
                                        ui.label("Ctrl+V / Cmd+V");
                                        ui.label(lang.tr("Colar Nota(s) na Posição do Cursor", "Paste Note(s) at Cursor Position"));
                                        ui.end_row();
                                        ui.label("Ctrl+D / Cmd+D");
                                        ui.label(lang.tr("Duplicar Nota(s)", "Duplicate Note(s)"));
                                        ui.end_row();
                                        ui.label("Ctrl+A / Cmd+A");
                                        ui.label(lang.tr("Selecionar Todas as Notas", "Select All Notes"));
                                        ui.end_row();
                                        ui.label("Ctrl+Shift+A / Cmd+Shift+A");
                                        ui.label(lang.tr("Desmarcar Seleção de Notas", "Deselect All Notes"));
                                        ui.end_row();
                                        ui.label("Tab");
                                        ui.label(lang.tr("Alternar Painel de Parâmetros / Automações (ON/OFF)", "Toggle Parameter / Automation Drawer (ON/OFF)"));
                                        ui.end_row();
                                        ui.label("M");
                                        ui.label(lang.tr("Alternar Mute na Faixa Ativa", "Toggle Mute on Active Track"));
                                        ui.end_row();
                                        ui.label("F1 / Cmd+?");
                                        ui.label(lang.tr("Abrir este Guia de Teclas de Atalho", "Open this Keyboard Shortcuts Guide"));
                                        ui.end_row();
                                        ui.label("Delete / Backspace");
                                        ui.label(lang.tr("Excluir Nota(s) Selecionada(s)", "Delete Selected Note(s)"));
                                        ui.end_row();
                                        ui.label(lang.tr("Seta Cima / Baixo", "Up / Down Arrow"));
                                        ui.label(lang.tr("Transpor Nota +1 / -1 Semitom", "Transpose Note +1 / -1 Semitone"));
                                        ui.end_row();
                                        ui.label(lang.tr("Shift + Seta Cima / Baixo", "Shift + Up / Down Arrow"));
                                        ui.label(lang.tr("Transpor Nota +1 / -1 Oitava (+12 / -12 semitones)", "Transpose Note +1 / -1 Octave (+12 / -12 semitones)"));
                                        ui.end_row();
                                        ui.label(lang.tr("Seta Esquerda / Direita", "Left / Right Arrow"));
                                        ui.label(lang.tr("Mover Posição da Nota (-50ms / +50ms)", "Move Note Position (-50ms / +50ms)"));
                                        ui.end_row();
                                        ui.label(lang.tr("Shift + Esquerda / Direita", "Shift + Left / Right Arrow"));
                                        ui.label(lang.tr("Redimensionar Duração da Nota (-50ms / +50ms)", "Resize Note Duration (-50ms / +50ms)"));
                                        ui.end_row();
                                        ui.label("Ctrl+N / Cmd+N");
                                        ui.label(lang.tr("Novo Projeto (Limpar / Criar projeto vazio)", "New Project (Clear / Create empty project)"));
                                        ui.end_row();
                                        ui.label("Ctrl+O / Cmd+O");
                                        ui.label(lang.tr("Abrir Projeto (.aps, .ustx, .ust, .mid)", "Open Project (.aps, .ustx, .ust, .mid)"));
                                        ui.end_row();
                                        ui.label("Ctrl+S / Cmd+S");
                                        ui.label(lang.tr("Salvar Projeto (.aps)", "Save Project (.aps)"));
                                        ui.end_row();
                                        ui.label("Ctrl+E / Cmd+E");
                                        ui.label(lang.tr("Exportar Áudio WAV", "Export WAV Audio"));
                                        ui.end_row();
                                        ui.label("Ctrl+L / Cmd+L");
                                        ui.label(lang.tr("Alternar Janela de Log do Engine (ON/OFF)", "Toggle Engine Log Window (ON/OFF)"));
                                        ui.end_row();
                                        ui.label("Ctrl+= / Cmd+=");
                                        ui.label(lang.tr("Aumentar Zoom Horizontal", "Zoom In Horizontal"));
                                        ui.end_row();
                                        ui.label("Ctrl+- / Cmd+-");
                                        ui.label(lang.tr("Diminuir Zoom Horizontal", "Zoom Out Horizontal"));
                                        ui.end_row();
                                        ui.label("Ctrl+0 / Cmd+0");
                                        ui.label(lang.tr("Redefinir Zoom Padrão", "Reset Default Zoom"));
                                        ui.end_row();
                                    });
                            });
                    });
                    if ctx.input(|i| i.viewport().close_requested()) {
                        is_open = false;
                    }
                },
            );
            self.shortcuts_guide_open = is_open;
        }
    }
}
