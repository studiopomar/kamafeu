use crate::gui::KamafeuStudioApp;
use eframe::egui;

#[derive(Clone, Copy)]
enum PaletteAction {
    NewProject,
    ProjectTemplates,
    ExportAudio,
    ValidateProject,
    VoicebankDiagnostics,
    Autopitch,
    FxRack,
    Preferences,
    ThemeCustomizer,
    KeyboardShortcuts,
    QuantizePositions,
    QuantizeDurations,
    ConnectLegato,
    FixOverlaps,
    SmoothPitchCurves,
    OptimizeEnvelopes,
    Rephonemize,
    ZoomFitProject,
    AddMarkerAtPlayhead,
    ManageMarkers,
}

impl PaletteAction {
    fn title(self, lang: crate::config::AppLanguage) -> &'static str {
        match self {
            Self::NewProject => lang.tr("Novo projeto", "New project"),
            Self::ProjectTemplates => lang.tr("Criar a partir de modelo", "Create from template"),
            Self::ExportAudio => lang.tr("Exportar áudio", "Export audio"),
            Self::ValidateProject => lang.tr("Validar projeto", "Validate project"),
            Self::VoicebankDiagnostics => {
                lang.tr("Diagnóstico do voicebank", "Voicebank diagnostics")
            }
            Self::Autopitch => lang.tr("Pre-tunning / Auto-Pitch", "Pre-tuning / Auto-Pitch"),
            Self::FxRack => lang.tr("Abrir rack de efeitos", "Open effects rack"),
            Self::Preferences => lang.tr("Preferências", "Preferences"),
            Self::ThemeCustomizer => lang.tr("Personalizar tema", "Customize theme"),
            Self::KeyboardShortcuts => {
                lang.tr("Guia de teclas de atalho", "Keyboard shortcuts guide")
            }
            Self::QuantizePositions => lang.tr("Quantizar posições", "Quantize positions"),
            Self::QuantizeDurations => lang.tr("Quantizar durações", "Quantize durations"),
            Self::ConnectLegato => lang.tr("Conectar notas em legato", "Connect notes in legato"),
            Self::FixOverlaps => lang.tr("Corrigir sobreposições", "Fix overlapping notes"),
            Self::SmoothPitchCurves => lang.tr("Suavizar curvas de pitch", "Smooth pitch curves"),
            Self::OptimizeEnvelopes => lang.tr("Otimizar envelopes", "Optimize envelopes"),
            Self::Rephonemize => lang.tr("Refazer fonemas", "Rephonemize notes"),
            Self::ZoomFitProject => lang.tr("Enquadrar todo o projeto", "Fit entire project"),
            Self::AddMarkerAtPlayhead => {
                lang.tr("Adicionar marcador no playhead", "Add marker at playhead")
            }
            Self::ManageMarkers => lang.tr("Gerenciar marcadores", "Manage markers"),
        }
    }

    fn subtitle(self, lang: crate::config::AppLanguage) -> &'static str {
        match self {
            Self::NewProject => lang.tr("Começar uma sessão limpa", "Start a clean session"),
            Self::ProjectTemplates => lang.tr(
                "Pop, balada, rock, coral e mais",
                "Pop, ballad, rock, choir, and more",
            ),
            Self::ExportAudio => lang.tr("Renderizar WAV ou FLAC", "Render WAV or FLAC"),
            Self::ValidateProject => lang.tr(
                "Encontrar notas e faixas que precisam de atenção",
                "Find notes and tracks that need attention",
            ),
            Self::VoicebankDiagnostics => lang.tr(
                "Conferir OTO, arquivos e modelos DiffSinger",
                "Check OTO, files, and DiffSinger models",
            ),
            Self::Autopitch => lang.tr(
                "Preparar curvas de pitch para as notas",
                "Prepare pitch curves for notes",
            ),
            Self::FxRack => lang.tr("Efeitos da faixa ativa", "Effects for the active track"),
            Self::Preferences => lang.tr(
                "Áudio, interface e comportamento",
                "Audio, interface, and behavior",
            ),
            Self::ThemeCustomizer => lang.tr(
                "Cores, contraste e densidade",
                "Colors, contrast, and density",
            ),
            Self::KeyboardShortcuts => lang.tr(
                "Ver todos os atalhos disponíveis",
                "See every available shortcut",
            ),
            Self::QuantizePositions => lang.tr(
                "Alinhar inícios à grade atual",
                "Align note starts to the current grid",
            ),
            Self::QuantizeDurations => lang.tr(
                "Alinhar fins e durações à grade atual",
                "Align note ends and durations to the current grid",
            ),
            Self::ConnectLegato => lang.tr(
                "Encostar notas selecionadas sem abrir lacunas",
                "Join selected notes without gaps",
            ),
            Self::FixOverlaps => lang.tr(
                "Resolver colisões entre notas da faixa",
                "Resolve collisions between notes on this track",
            ),
            Self::SmoothPitchCurves => lang.tr(
                "Suavizar os pontos de pitch das notas selecionadas",
                "Smooth pitch points on selected notes",
            ),
            Self::OptimizeEnvelopes => lang.tr(
                "Ajustar ataques e solturas à duração das notas",
                "Fit attacks and releases to note durations",
            ),
            Self::Rephonemize => lang.tr(
                "Atualizar a prévia fonética da faixa ativa",
                "Refresh phoneme preview for the active track",
            ),
            Self::ZoomFitProject => lang.tr(
                "Mostrar todas as notas na timeline",
                "Show all notes in the timeline",
            ),
            Self::AddMarkerAtPlayhead => lang.tr(
                "Criar uma seção na posição atual de reprodução",
                "Create a section at the current playback position",
            ),
            Self::ManageMarkers => lang.tr(
                "Renomear, reposicionar, colorir ou remover seções",
                "Rename, move, color, or remove sections",
            ),
        }
    }

    fn keywords(self) -> &'static str {
        match self {
            Self::NewProject => "novo criar projeto new create",
            Self::ProjectTemplates => "modelo template pop balada rock coral",
            Self::ExportAudio => "exportar renderizar audio wav flac export render",
            Self::ValidateProject => "validar diagnostico projeto errors validate",
            Self::VoicebankDiagnostics => "voicebank cantor oto diffsinger diagnostico singer",
            Self::Autopitch => "pitch afinar tuning pre-tunning autopitch",
            Self::FxRack => "efeitos fx rack plugin effects",
            Self::Preferences => "preferencias configuracoes settings audio",
            Self::ThemeCustomizer => "tema cores aparencia theme colors",
            Self::KeyboardShortcuts => "atalhos teclas ajuda shortcuts keyboard help",
            Self::QuantizePositions => "quantizar quantize grade grid inicio posição timing",
            Self::QuantizeDurations => "quantizar quantize duração fim grade grid timing",
            Self::ConnectLegato => "legato conectar notas gap lacuna join",
            Self::FixOverlaps => "sobreposição overlap colisão notas fix",
            Self::SmoothPitchCurves => "pitch curva suavizar smooth tuning",
            Self::OptimizeEnvelopes => "envelope ataque soltura release optimize opt",
            Self::Rephonemize => "fonema fonemizar phoneme rephonemize letra",
            Self::ZoomFitProject => "zoom enquadrar timeline projeto fit all",
            Self::AddMarkerAtPlayhead => {
                "marcador marker seção section playhead arranjo arrangement"
            }
            Self::ManageMarkers => "marcadores markers seções sections arranjo arrangement editar",
        }
    }
}

impl KamafeuStudioApp {
    pub(crate) fn render_command_palette(&mut self, ctx: &egui::Context) {
        if !self.command_palette_open {
            return;
        }

        let lang = self.config.language;
        let input_id = egui::Id::new("command_palette_search");
        let mut is_open = self.command_palette_open;
        let mut selected_action = None;
        let mut escape_requested = false;

        egui::Window::new(lang.tr("Paleta de comandos", "Command Palette"))
            .id(egui::Id::new("command_palette_window"))
            .open(&mut is_open)
            .collapsible(false)
            .resizable(false)
            .default_width(560.0)
            .anchor(egui::Align2::CENTER_TOP, [0.0, 84.0])
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(lang.tr(
                        "Encontre uma ação sem procurar pelos menus.",
                        "Find an action without hunting through menus.",
                    ))
                    .color(self.config.theme.text_muted_c32()),
                );
                ui.add_space(6.0);
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.command_palette_query)
                        .id(input_id)
                        .hint_text(lang.tr("Digite para buscar…", "Type to search…"))
                        .desired_width(f32::INFINITY),
                );
                if !response.has_focus() {
                    ui.memory_mut(|memory| memory.request_focus(input_id));
                }
                if response.changed() {
                    self.command_palette_selected = 0;
                }
                ui.add_space(8.0);

                let query = self.command_palette_query.trim().to_lowercase();
                let actions = [
                    PaletteAction::NewProject,
                    PaletteAction::ProjectTemplates,
                    PaletteAction::ExportAudio,
                    PaletteAction::ValidateProject,
                    PaletteAction::VoicebankDiagnostics,
                    PaletteAction::Autopitch,
                    PaletteAction::FxRack,
                    PaletteAction::Preferences,
                    PaletteAction::ThemeCustomizer,
                    PaletteAction::KeyboardShortcuts,
                    PaletteAction::QuantizePositions,
                    PaletteAction::QuantizeDurations,
                    PaletteAction::ConnectLegato,
                    PaletteAction::FixOverlaps,
                    PaletteAction::SmoothPitchCurves,
                    PaletteAction::OptimizeEnvelopes,
                    PaletteAction::Rephonemize,
                    PaletteAction::ZoomFitProject,
                    PaletteAction::AddMarkerAtPlayhead,
                    PaletteAction::ManageMarkers,
                ];
                let matching: Vec<_> = actions
                    .into_iter()
                    .filter(|action| {
                        query.is_empty()
                            || action.title(lang).to_lowercase().contains(&query)
                            || action.subtitle(lang).to_lowercase().contains(&query)
                            || action.keywords().contains(&query)
                    })
                    .collect();

                if matching.is_empty() {
                    ui.add_space(12.0);
                    ui.centered_and_justified(|ui| {
                        ui.label(
                            egui::RichText::new(
                                lang.tr("Nenhuma ação encontrada.", "No matching action found."),
                            )
                            .color(self.config.theme.text_muted_c32()),
                        );
                    });
                } else {
                    self.command_palette_selected = self
                        .command_palette_selected
                        .min(matching.len().saturating_sub(1));
                    let (move_up, move_down, execute_selected) = ui.input(|input| {
                        (
                            input.key_pressed(egui::Key::ArrowUp),
                            input.key_pressed(egui::Key::ArrowDown),
                            input.key_pressed(egui::Key::Enter),
                        )
                    });
                    if move_up {
                        self.command_palette_selected = self
                            .command_palette_selected
                            .checked_sub(1)
                            .unwrap_or(matching.len() - 1);
                    }
                    if move_down {
                        self.command_palette_selected =
                            (self.command_palette_selected + 1) % matching.len();
                    }
                    if execute_selected {
                        selected_action = Some(matching[self.command_palette_selected]);
                    }

                    egui::ScrollArea::vertical()
                        .max_height(330.0)
                        .show(ui, |ui| {
                            for (index, action) in matching.into_iter().enumerate() {
                                let label =
                                    format!("{}\n{}", action.title(lang), action.subtitle(lang));
                                let is_selected = index == self.command_palette_selected;
                                let response = ui.add_sized(
                                    [ui.available_width(), 43.0],
                                    egui::Button::new(
                                        egui::RichText::new(label)
                                            .size(12.0)
                                            .color(egui::Color32::WHITE),
                                    )
                                    .wrap()
                                    .fill(if is_selected {
                                        egui::Color32::from_rgb(28, 101, 108)
                                    } else {
                                        egui::Color32::TRANSPARENT
                                    }),
                                );
                                if response.clicked() {
                                    selected_action = Some(action);
                                }
                                if is_selected {
                                    response.scroll_to_me(Some(egui::Align::Center));
                                }
                                ui.add_space(3.0);
                            }
                        });
                }
                ui.add_space(5.0);
                ui.label(
                    egui::RichText::new(lang.tr(
                        "↑↓ navegam · Enter executa · Esc fecha · Ctrl/Cmd+K alterna",
                        "↑↓ navigate · Enter runs · Esc closes · Ctrl/Cmd+K toggles",
                    ))
                    .size(10.0)
                    .color(self.config.theme.text_muted_c32()),
                );
                if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
                    escape_requested = true;
                }
            });

        if escape_requested {
            is_open = false;
        }

        if let Some(action) = selected_action {
            match action {
                PaletteAction::NewProject => self.new_project(),
                PaletteAction::ProjectTemplates => self.templates_dialog_open = true,
                PaletteAction::ExportAudio => self.export_dialog_open = true,
                PaletteAction::ValidateProject => self.project_diagnostic_open = true,
                PaletteAction::VoicebankDiagnostics => self.voicebank_diagnostic_open = true,
                PaletteAction::Autopitch => self.autopitch_window_open = true,
                PaletteAction::FxRack => {
                    self.fx_rack_dialog_state.target_track = Some(self.active_track_index);
                    self.fx_rack_dialog_state.is_open = true;
                }
                PaletteAction::Preferences => self.preferences_window_open = true,
                PaletteAction::ThemeCustomizer => self.theme_customizer_open = true,
                PaletteAction::KeyboardShortcuts => self.shortcuts_guide_open = true,
                PaletteAction::QuantizePositions => self.quantize_positions(),
                PaletteAction::QuantizeDurations => self.quantize_durations(),
                PaletteAction::ConnectLegato => self.legato_connect_notes(),
                PaletteAction::FixOverlaps => self.fix_overlapping_notes(),
                PaletteAction::SmoothPitchCurves => self.smooth_selected_pitch_curves(),
                PaletteAction::OptimizeEnvelopes => self.apply_envelope_opt(),
                PaletteAction::Rephonemize => self.rephonemize_all_notes(),
                PaletteAction::ZoomFitProject => self.zoom_fit_all_notes(),
                PaletteAction::AddMarkerAtPlayhead => {
                    self.push_history();
                    let position_ms = self.piano_roll_state.playhead_ms.max(0.0);
                    let marker_number = self.project.markers.len() + 1;
                    self.project
                        .markers
                        .push(crate::project::UProjectMarker::new(
                            format!("{} {}", lang.tr("Marcador", "Marker"), marker_number),
                            position_ms,
                        ));
                    self.project.normalize();
                    self.is_dirty = true;
                    self.transport_state.status_message = format!(
                        "{} {:.0} ms",
                        lang.tr("Marcador criado em", "Marker created at"),
                        position_ms
                    );
                }
                PaletteAction::ManageMarkers => self.markers_dialog_open = true,
            }
            self.command_palette_query.clear();
            is_open = false;
        }

        self.command_palette_open = is_open;
    }
}
