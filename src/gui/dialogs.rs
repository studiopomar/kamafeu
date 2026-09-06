use super::KamafeuStudioApp;
use eframe::egui;

impl KamafeuStudioApp {
    pub(crate) fn render_dialogs(&mut self, ctx: &egui::Context) {
        if self.copaiba_window_open {
            let mut is_open = self.copaiba_window_open;
            egui::Window::new("Copaiba Voicebank Toolkit")
                .open(&mut is_open)
                .default_size([1000.0, 600.0])
                .show(ctx, |ui| {
                    crate::copaiba::gui::draw_copaiba_toolkit_ui(&mut self.copaiba_app, ui);
                });
            self.copaiba_window_open = is_open;
        }

        if self.shortcuts_guide_open {
            let mut is_open = self.shortcuts_guide_open;
            egui::Window::new("Guia de Teclas de Atalho")
            .open(&mut is_open)
            .default_size([550.0, 480.0])
            .show(ctx, |ui| {
                ui.heading(
                    egui::RichText::new("Teclas de Atalho do Kamafeu Studio")
                        .strong()
                        .color(egui::Color32::from_rgb(0, 255, 157)),
                );
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
                                    egui::RichText::new("Atalho")
                                        .strong()
                                        .color(egui::Color32::from_rgb(255, 215, 0)),
                                );
                                ui.label(
                                    egui::RichText::new("Ação / Funcionalidade")
                                        .strong()
                                        .color(egui::Color32::from_rgb(255, 215, 0)),
                                );
                                ui.end_row();

                                ui.label("Espaço (Space)");
                                ui.label("Tocar / Pausar Reprodução");
                                ui.end_row();
                                ui.label("Esc");
                                ui.label("Parar e Reiniciar Cursor no Início (0ms)");
                                ui.end_row();
                                ui.label("V ou 1");
                                ui.label(
                                    "Ferramenta Ponteiro (Seleção / Mover / Redimensionar)",
                                );
                                ui.end_row();
                                ui.label("N ou 2");
                                ui.label("Ferramenta Lápis (Desenhar Notas)");
                                ui.end_row();
                                ui.label("P ou 3");
                                ui.label("Ferramenta Pitch (✏ Livre / 📏 Reta / 〰 Vibrato / 🪄 Suave)");
                                ui.end_row();
                                ui.label("Shift + P");
                                ui.label("Alternar Submodo de Pitch (Livre / Reta / Vibrato / Suave)");
                                ui.end_row();
                                ui.label("Ctrl+Alt+P / Cmd+Alt+P");
                                ui.label("Abrir AutoPitch Vocal Studio (Afinador Orgânico)");
                                ui.end_row();
                                ui.label("Duplo-clique / Shift+Click na curva");
                                ui.label("Adicionar novo ponto de ancoragem no pitch");
                                ui.end_row();
                                ui.label("Alt+Click / Clique Direito na âncora");
                                ui.label("Deletar ponto de ancoragem específico do pitch");
                                ui.end_row();
                                ui.label("E ou 4");
                                ui.label("Ferramenta Borracha (Apagar Notas)");
                                ui.end_row();
                                ui.label("Ctrl+Z / Cmd+Z");
                                ui.label("Desfazer Ação");
                                ui.end_row();
                                ui.label("Ctrl+Y / Cmd+Shift+Z");
                                ui.label("Refazer Ação");
                                ui.end_row();
                                ui.label("Ctrl+X / Cmd+X");
                                ui.label("Recortar Nota(s) Selecionada(s)");
                                ui.end_row();
                                ui.label("Ctrl+C / Cmd+C");
                                ui.label("Copiar Nota(s) Selecionada(s)");
                                ui.end_row();
                                ui.label("Ctrl+V / Cmd+V");
                                ui.label("Colar Nota(s) na Posição do Cursor");
                                ui.end_row();
                                ui.label("Ctrl+D / Cmd+D");
                                ui.label("Duplicar Nota(s)");
                                ui.end_row();
                                ui.label("Ctrl+A / Cmd+A");
                                ui.label("Selecionar Todas as Notas");
                                ui.end_row();
                                ui.label("Ctrl+Shift+A / Cmd+Shift+A");
                                ui.label("Desmarcar Seleção de Notas");
                                ui.end_row();
                                ui.label("Tab");
                                ui.label("Alternar Painel de Parâmetros / Automações (ON/OFF)");
                                ui.end_row();
                                ui.label("M");
                                ui.label("Alternar Mute na Faixa Ativa");
                                ui.end_row();
                                ui.label("F1 / Cmd+?");
                                ui.label("Abrir este Guia de Teclas de Atalho");
                                ui.end_row();
                                ui.label("Delete / Backspace");
                                ui.label("Excluir Nota(s) Selecionada(s)");
                                ui.end_row();
                                ui.label("Seta Cima / Baixo");
                                ui.label("Transpor Nota +1 / -1 Semitom");
                                ui.end_row();
                                ui.label("Shift + Seta Cima / Baixo");
                                ui.label("Transpor Nota +1 / -1 Oitava (+12 / -12 semitones)");
                                ui.end_row();
                                ui.label("Seta Esquerda / Direita");
                                ui.label("Mover Posição da Nota (-50ms / +50ms)");
                                ui.end_row();
                                ui.label("Shift + Esquerda / Direita");
                                ui.label("Redimensionar Duração da Nota (-50ms / +50ms)");
                                ui.end_row();
                                ui.label("Ctrl+N / Cmd+N");
                                ui.label("Novo Projeto (Limpar / Criar projeto vazio)");
                                ui.end_row();
                                ui.label("Ctrl+O / Cmd+O");
                                ui.label("Abrir Projeto (.aps, .ustx, .ust, .mid)");
                                ui.end_row();
                                ui.label("Ctrl+S / Cmd+S");
                                ui.label("Salvar Projeto (.aps)");
                                ui.end_row();
                                ui.label("Ctrl+E / Cmd+E");
                                ui.label("Exportar Áudio WAV");
                                ui.end_row();
                                ui.label("Ctrl+L / Cmd+L");
                                ui.label("Alternar Janela de Log do Engine (ON/OFF)");
                                ui.end_row();
                                ui.label("Ctrl+= / Cmd+=");
                                ui.label("Aumentar Zoom Horizontal");
                                ui.end_row();
                                ui.label("Ctrl+- / Cmd+-");
                                ui.label("Diminuir Zoom Horizontal");
                                ui.end_row();
                                ui.label("Ctrl+0 / Cmd+0");
                                ui.label("Redefinir Zoom Padrão");
                                ui.end_row();
                            });
                    });
            });
            self.shortcuts_guide_open = is_open;
        }

        if self.batch_lyrics_open {
            let mut is_open = self.batch_lyrics_open;
            let mut apply_lyrics = false;
            let mut close_modal = false;

            egui::Window::new("✍️ Inserir Letras em Lote (Batch Lyrics)")
                .open(&mut is_open)
                .default_size([460.0, 240.0])
                .resizable(true)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(
                        "Cole ou digite o texto com palavras ou sílabas separadas por espaço:",
                    );
                    ui.add_space(4.0);
                    ui.add(
                        egui::TextEdit::multiline(&mut self.batch_lyrics_buffer)
                            .desired_rows(6)
                            .desired_width(f32::INFINITY)
                            .hint_text("ex: quem te viu quem te ve"),
                    );
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        if ui.button("✨ Distribuir Letras pelas Notas").clicked() {
                            apply_lyrics = true;
                        }
                        if ui.button("Cancelar").clicked() {
                            close_modal = true;
                        }
                    });
                });

            if apply_lyrics {
                let syllables: Vec<String> = self
                    .batch_lyrics_buffer
                    .split_whitespace()
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();

                if !syllables.is_empty() {
                    let undo_before = self.project.clone();
                    self.undo_manager.push_state(undo_before);

                    let sel = self.piano_roll_state.selected_note_indices.clone();
                    let notes = self.current_notes_mut();
                    let target_indices: Vec<usize> = if !sel.is_empty() {
                        let mut v: Vec<usize> = sel.into_iter().collect();
                        v.sort_by(|&a, &b| {
                            notes
                                .get(a)
                                .map(|n| n.position_ms)
                                .unwrap_or(0.0)
                                .partial_cmp(&notes.get(b).map(|n| n.position_ms).unwrap_or(0.0))
                                .unwrap_or(std::cmp::Ordering::Equal)
                        });
                        v
                    } else {
                        (0..notes.len()).collect()
                    };

                    for (i, &idx) in target_indices.iter().enumerate() {
                        if let Some(syl) = syllables.get(i) {
                            if let Some(note) = notes.get_mut(idx) {
                                note.lyric = syl.clone();
                            }
                        }
                    }
                    self.piano_roll_state.phoneme_cache.clear();
                    self.is_dirty = true;
                    self.transport_state.status_message = format!(
                        "Letras distribuídas por {} notas",
                        syllables.len().min(target_indices.len())
                    );
                    close_modal = true;
                }
            }

            if close_modal {
                is_open = false;
            }
            self.batch_lyrics_open = is_open;
        }

        if self.singers_gallery_window_open {
            let mut is_open = self.singers_gallery_window_open;
            let mut singer_to_load: Option<std::path::PathBuf> = None;
            let mut trigger_add_dir = false;
            let mut trigger_reload = false;

            egui::Window::new("🎭 Galeria de Cantores (OpenUtau / UTAU)")
            .open(&mut is_open)
            .default_size([720.0, 520.0])
            .min_size([500.0, 380.0])
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(
                        egui::RichText::new("🎭 Cantores Instalados")
                            .strong()
                            .size(16.0)
                            .color(egui::Color32::from_rgb(0, 255, 180)),
                    );
                    ui.label(
                        egui::RichText::new(format!("({} encontrados)", self.singers_list.len()))
                            .size(11.0)
                            .color(crate::gui::theme::MelodyneTheme::TEXT_MUTED),
                    );
                    ui.add_space(ui.available_width() - 250.0);
                    if ui.button("➕ Registrar Pasta...").clicked() {
                        trigger_add_dir = true;
                    }
                    if ui.button("🔄 Recarregar").clicked() {
                        trigger_reload = true;
                    }
                });

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("🔍").size(14.0));
                    ui.add(
                        egui::TextEdit::singleline(&mut self.singer_search_query)
                            .hint_text("Buscar por nome do cantor ou autor...")
                            .desired_width(ui.available_width()),
                    );
                });

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(8.0);

                let query_lower = self.singer_search_query.to_lowercase();
                let filtered: Vec<_> = self
                    .singers_list
                    .iter()
                    .filter(|s| {
                        query_lower.is_empty()
                            || s.name.to_lowercase().contains(&query_lower)
                            || s.author.to_lowercase().contains(&query_lower)
                            || s.voice_type.to_lowercase().contains(&query_lower)
                    })
                    .collect();

                egui::ScrollArea::vertical()
                    .id_salt("singers_gallery_grid_scroll")
                    .show(ui, |ui| {
                        if filtered.is_empty() {
                            ui.vertical_centered(|ui| {
                                ui.add_space(40.0);
                                ui.label(
                                    egui::RichText::new("Nenhum cantor encontrado.")
                                        .size(13.0)
                                        .color(crate::gui::theme::MelodyneTheme::TEXT_MUTED),
                                );
                                ui.label(
                                    egui::RichText::new(
                                        "Clique em 'Registrar Pasta...' para apontar para a pasta 'Singers' do OpenUtau.",
                                    )
                                    .size(10.5)
                                    .italics()
                                    .color(egui::Color32::from_rgb(180, 190, 220)),
                                );
                            });
                        } else {
                            let item_w = 210.0f32;
                            let item_h = 100.0f32;
                            let cols = ((ui.available_width() / (item_w + 12.0)).floor() as usize).max(1);

                            egui::Grid::new("singers_gallery_grid")
                                .spacing([12.0, 12.0])
                                .show(ui, |ui| {
                                    for (idx, singer) in filtered.iter().enumerate() {
                                        let is_current = self
                                            .voicebank
                                            .as_ref()
                                            .is_some_and(|v| v.root_path == singer.path);

                                        let bg_color = if is_current {
                                            egui::Color32::from_rgb(45, 30, 68)
                                        } else {
                                            egui::Color32::from_rgb(24, 18, 34)
                                        };
                                        let stroke_color = if is_current {
                                            egui::Color32::from_rgb(0, 255, 180)
                                        } else {
                                            egui::Color32::from_rgb(52, 40, 72)
                                        };

                                        egui::Frame::none()
                                            .fill(bg_color)
                                            .rounding(egui::Rounding::same(8.0))
                                            .stroke(egui::Stroke::new(1.2_f32, stroke_color))
                                            .inner_margin(egui::Margin::same(8.0))
                                            .show(ui, |ui| {
                                                ui.set_width(item_w);
                                                ui.set_height(item_h);

                                                ui.horizontal(|ui| {
                                                    let (avatar_rect, _) = ui.allocate_exact_size(
                                                        egui::Vec2::new(56.0, 56.0),
                                                        egui::Sense::hover(),
                                                    );
                                                    let painter = ui.painter_at(avatar_rect);
                                                    painter.rect_filled(
                                                        avatar_rect,
                                                        egui::Rounding::same(6.0),
                                                        egui::Color32::from_rgb(36, 26, 52),
                                                    );

                                                    let mut loaded = false;
                                                    if let Some(ref img_p) = singer.image_path {
                                                        if let Some(tex) = crate::gui::image_cache::texture_for_path(ui.ctx(), img_p) {
                                                            painter.image(
                                                                tex.id(),
                                                                avatar_rect,
                                                                egui::Rect::from_min_max(
                                                                    egui::Pos2::new(0.0, 0.0),
                                                                    egui::Pos2::new(1.0, 1.0),
                                                                ),
                                                                egui::Color32::WHITE,
                                                            );
                                                            loaded = true;
                                                        }
                                                    }

                                                    if !loaded {
                                                        let initial = singer.name.chars().next().unwrap_or('V');
                                                        painter.circle_filled(
                                                            avatar_rect.center(),
                                                            18.0,
                                                            egui::Color32::from_rgb(192, 132, 252),
                                                        );
                                                        painter.text(
                                                            avatar_rect.center(),
                                                            egui::Align2::CENTER_CENTER,
                                                            initial.to_string(),
                                                            egui::FontId::proportional(16.0),
                                                            egui::Color32::from_rgb(20, 14, 30),
                                                        );
                                                    }

                                                    ui.add_space(6.0);
                                                    ui.vertical(|ui| {
                                                        ui.label(
                                                            egui::RichText::new(&singer.name)
                                                                .strong()
                                                                .size(11.5)
                                                                .color(if is_current {
                                                                    egui::Color32::from_rgb(0, 255, 180)
                                                                } else {
                                                                    egui::Color32::WHITE
                                                                }),
                                                        );
                                                        ui.label(
                                                            egui::RichText::new(&singer.author)
                                                                .size(9.0)
                                                                .color(crate::gui::theme::MelodyneTheme::TEXT_MUTED),
                                                        );
                                                        ui.label(
                                                            egui::RichText::new(&singer.voice_type)
                                                                .size(8.5)
                                                                .color(egui::Color32::from_rgb(216, 180, 254)),
                                                        );

                                                        ui.add_space(4.0);
                                                        if is_current {
                                                            ui.label(
                                                                egui::RichText::new("✓ Em Uso")
                                                                    .strong()
                                                                    .size(10.0)
                                                                    .color(egui::Color32::from_rgb(0, 255, 180)),
                                                            );
                                                        } else if ui.small_button("🎤 Selecionar").clicked() {
                                                            singer_to_load = Some(singer.path.clone());
                                                        }
                                                    });
                                                });
                                            });

                                        if (idx + 1) % cols == 0 {
                                            ui.end_row();
                                        }
                                    }
                                });
                        }
                    });
            });

            if trigger_add_dir {
                if let Some(folder) = crate::dialogs::FileDialog::new().pick_folder() {
                    if !self.config.singers_paths.contains(&folder) {
                        self.config.singers_paths.push(folder);
                        self.persist_config();
                        self.reload_singers();
                    }
                }
            }

            if trigger_reload {
                self.persist_config();
                self.reload_singers();
            }

            if let Some(path) = singer_to_load {
                if let Ok(vb) = crate::oto::Voicebank::new(&path) {
                    self.transport_state.status_message =
                        format!("Voicebank Carregado: {}", vb.name);
                    self.transport_state.voicebank_name = vb.name.clone();
                    self.transport_state.voicebank_path = Some(vb.root_path.clone());
                    self.config.add_recent_voicebank(vb.root_path.clone());
                    self.voicebank = Some(vb);
                    self.persist_config();
                }
                is_open = false;
            }

            self.singers_gallery_window_open = is_open;
        }

        if self.autopitch_window_open {
            let mut is_open = self.autopitch_window_open;
            let mut trigger_apply = false;
            let mut trigger_close = false;

            egui::Window::new("✨ AutoPitch - Afinador Vocal Orgânico")
                .open(&mut is_open)
                .resizable(false)
                .default_size([520.0, 480.0])
                .show(ctx, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(8.0, 8.0);

                    ui.horizontal(|ui| {
                        ui.heading(
                            egui::RichText::new("✨ AutoPitch Vocal Studio")
                                .strong()
                                .color(egui::Color32::from_rgb(0, 255, 180)),
                        );
                    });
                    ui.label(
                        egui::RichText::new(
                            "Modela curvas humanas realistas com attack scoops, overshoots, release drops e vibrato adaptativo.",
                        )
                        .size(11.0)
                        .color(egui::Color32::from_rgb(180, 175, 200)),
                    );

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    ui.label(
                        egui::RichText::new("ESTILO / PRESET:")
                            .strong()
                            .size(12.0)
                            .color(egui::Color32::from_rgb(255, 215, 0)),
                    );

                    ui.horizontal_wrapped(|ui| {
                        for preset in crate::dsp::AutoPitchPreset::all() {
                            let is_selected = self.autopitch_options.preset == *preset;
                            let (bg_color, stroke_color, text_color) = if is_selected {
                                (
                                    egui::Color32::from_rgb(50, 40, 75),
                                    egui::Stroke::new(1.5_f32, egui::Color32::from_rgb(0, 255, 180)),
                                    egui::Color32::from_rgb(0, 255, 220),
                                )
                            } else {
                                (
                                    egui::Color32::from_rgb(26, 20, 36),
                                    egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(50, 40, 65)),
                                    egui::Color32::from_rgb(200, 195, 215),
                                )
                            };

                            let btn = egui::Button::new(
                                egui::RichText::new(preset.name())
                                    .strong()
                                    .size(11.5)
                                    .color(text_color),
                            )
                            .fill(bg_color)
                            .stroke(stroke_color)
                            .rounding(egui::Rounding::same(4.0))
                            .min_size(egui::vec2(90.0, 28.0));

                            if ui.add(btn).clicked() {
                                self.autopitch_options.preset = *preset;
                            }
                        }
                    });

                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(20, 16, 28))
                        .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(45, 38, 60)))
                        .rounding(egui::Rounding::same(4.0))
                        .inner_margin(egui::Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.label(
                                egui::RichText::new(self.autopitch_options.preset.description())
                                    .size(11.0)
                                    .italics()
                                    .color(egui::Color32::from_rgb(220, 215, 240)),
                            );
                        });

                    ui.add_space(4.0);
                    ui.separator();
                    ui.add_space(4.0);

                    ui.label(
                        egui::RichText::new("PARÂMETROS DE EXPRESSÃO:")
                            .strong()
                            .size(12.0)
                            .color(egui::Color32::from_rgb(255, 215, 0)),
                    );

                    ui.horizontal(|ui| {
                        ui.label("Intensidade Global:");
                        let mut pct = (self.autopitch_options.intensity * 100.0).round() as i32;
                        if ui
                            .add(
                                egui::Slider::new(&mut pct, 0..=200)
                                    .suffix("%")
                                    .show_value(true),
                            )
                            .changed()
                        {
                            self.autopitch_options.intensity = pct as f64 / 100.0;
                        }
                    });

                    ui.add_space(2.0);
                    egui::Grid::new("autopitch_checkboxes_grid")
                        .spacing([24.0, 8.0])
                        .show(ui, |ui| {
                            ui.checkbox(
                                &mut self.autopitch_options.enable_attack_scoop,
                                "🌸 Attack Scoop (Ataque inicial)",
                            )
                            .on_hover_text("Inicia notas de começo de frase ligeiramente abaixo do tom, subindo suavemente.");

                            ui.checkbox(
                                &mut self.autopitch_options.enable_overshoot,
                                "⚡ Overshoot de Portamento",
                            )
                            .on_hover_text("Ultrapassa levemente o tom em saltos ascendentes antes de estabilizar.");
                            ui.end_row();

                            ui.checkbox(
                                &mut self.autopitch_options.enable_release_drop,
                                "🍂 Release Drop (Queda final)",
                            )
                            .on_hover_text("Queda natural de afinação no final de frases/silêncios.");

                            ui.checkbox(
                                &mut self.autopitch_options.enable_vibrato,
                                "〰 Vibrato Inteligente",
                            )
                            .on_hover_text("Aplica vibrato natural com fade-in e período adequado ao estilo.");
                            ui.end_row();
                        });

                    ui.add_space(6.0);
                    ui.separator();
                    ui.add_space(4.0);

                    ui.label(
                        egui::RichText::new("APLICAR EM:")
                            .strong()
                            .size(12.0)
                            .color(egui::Color32::from_rgb(255, 215, 0)),
                    );

                    let sel_count = self.piano_roll_state.selected_note_indices.len();
                    ui.horizontal(|ui| {
                        let sel_label = if sel_count > 0 {
                            format!("🎯 Notas Selecionadas ({} notas)", sel_count)
                        } else if self.piano_roll_state.selected_note_index.is_some() {
                            "🎯 Nota Selecionada (1 nota)".to_string()
                        } else {
                            "🎯 Notas Selecionadas (Nenhuma selecionada)".to_string()
                        };

                        ui.radio_value(
                            &mut self.autopitch_scope,
                            crate::dsp::AutoPitchScope::SelectedOnly,
                            sel_label,
                        );

                        ui.radio_value(
                            &mut self.autopitch_scope,
                            crate::dsp::AutoPitchScope::AllNotes,
                            "🎼 Todas as Notas da Faixa",
                        );
                    });

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        if ui.button("Cancelar").clicked() {
                            trigger_close = true;
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let apply_btn = egui::Button::new(
                                egui::RichText::new("✨ Aplicar AutoPitch")
                                    .strong()
                                    .size(13.0)
                                    .color(egui::Color32::BLACK),
                            )
                            .fill(egui::Color32::from_rgb(0, 255, 180))
                            .rounding(egui::Rounding::same(4.0))
                            .min_size(egui::vec2(140.0, 30.0));

                            if ui.add(apply_btn).clicked() {
                                trigger_apply = true;
                                trigger_close = true;
                            }
                        });
                    });
                });

            if trigger_close {
                is_open = false;
            }

            if trigger_apply {
                self.apply_autopitch();
            }

            self.autopitch_window_open = is_open;
        }

        if self.export_options_dialog_open {
            let mut is_open = self.export_options_dialog_open;
            let mut trigger_export = false;
            let mut trigger_close = false;

            egui::Window::new("📤 Opções de Exportação WAV")
                .open(&mut is_open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .fixed_size([480.0, 260.0])
                .show(ctx, |ui| {
                    ui.add_space(4.0);
                    ui.heading(
                        egui::RichText::new("Selecione o Conteúdo para Exportar")
                            .strong()
                            .size(15.0)
                            .color(egui::Color32::from_rgb(0, 220, 255)),
                    );
                    ui.label(
                        egui::RichText::new("O projeto contém faixas vocais e arquivos de áudio instrumental importados.")
                            .size(11.0)
                            .color(egui::Color32::from_rgb(170, 160, 190)),
                    );

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.radio_value(
                        &mut self.export_audio_scope,
                        crate::gui::types::ExportAudioScope::VocalsAndAudio,
                        egui::RichText::new("🎵 Mix Completa (Vocais + Instrumental/Áudios)")
                            .strong()
                            .size(13.0)
                            .color(egui::Color32::from_rgb(230, 240, 255)),
                    );
                    ui.indent("opt_full_mix", |ui| {
                        ui.label(
                            egui::RichText::new("Exporta a música finalizada, combinando as vozes sintetizadas com as faixas de áudio importadas.")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(150, 145, 175)),
                        );
                    });

                    ui.add_space(6.0);

                    ui.radio_value(
                        &mut self.export_audio_scope,
                        crate::gui::types::ExportAudioScope::VocalsOnly,
                        egui::RichText::new("🎙️ Apenas Vocais (Acapella)")
                            .strong()
                            .size(13.0)
                            .color(egui::Color32::from_rgb(230, 240, 255)),
                    );
                    ui.indent("opt_vocals_only", |ui| {
                        ui.label(
                            egui::RichText::new("Exporta apenas a síntese das vozes (stems/acapella), silenciando as faixas instrumentais.")
                                .size(11.0)
                                .color(egui::Color32::from_rgb(150, 145, 175)),
                        );
                    });

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(8.0);

                    ui.horizontal(|ui| {
                        if ui.button("Cancelar").clicked() {
                            trigger_close = true;
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let export_btn = egui::Button::new(
                                egui::RichText::new("💾 Escolher Destino e Exportar...")
                                    .strong()
                                    .size(12.0)
                                    .color(egui::Color32::BLACK),
                            )
                            .fill(egui::Color32::from_rgb(0, 255, 180))
                            .min_size(egui::vec2(160.0, 28.0));

                            if ui.add(export_btn).clicked() {
                                trigger_export = true;
                                trigger_close = true;
                            }
                        });
                    });
                });

            if trigger_close {
                is_open = false;
            }
            self.export_options_dialog_open = is_open;

            if trigger_export {
                let scope = self.export_audio_scope;
                self.execute_export_wav(scope);
            }
        }

        if self.export_dialog_open {
            let mut is_open = self.export_dialog_open;
            let export_finished = !self.export_in_progress && self.export_result.is_some();
            let has_error = match &self.export_result {
                Some(Err(_)) => true,
                _ => false,
            };

            let title = if export_finished {
                if has_error {
                    "❌ Falha na Exportação WAV"
                } else {
                    "✅ Exportação WAV Concluída"
                }
            } else {
                "📤 Exportando Áudio WAV..."
            };

            let mut trigger_close = false;

            egui::Window::new(title)
                .open(&mut is_open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
                .fixed_size([520.0, 240.0])
                .show(ctx, |ui| {
                    ui.add_space(4.0);

                    let filename_text = self
                        .export_save_path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "output.wav".to_string());

                    ui.horizontal(|ui| {
                        let icon = if export_finished {
                            if has_error {
                                "❌"
                            } else {
                                "🎉"
                            }
                        } else {
                            "💾"
                        };
                        ui.label(egui::RichText::new(icon).size(24.0));
                        ui.vertical(|ui| {
                            ui.heading(
                                egui::RichText::new(&filename_text)
                                    .strong()
                                    .size(15.0)
                                    .color(if has_error {
                                        egui::Color32::from_rgb(255, 100, 100)
                                    } else if export_finished {
                                        egui::Color32::from_rgb(0, 255, 180)
                                    } else {
                                        egui::Color32::from_rgb(0, 220, 255)
                                    }),
                            );
                            if let Some(ref path) = self.export_save_path {
                                ui.label(
                                    egui::RichText::new(path.to_string_lossy())
                                        .size(10.5)
                                        .color(egui::Color32::from_rgb(160, 150, 180)),
                                );
                            }
                        });
                    });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(8.0);

                    let pct = (self.export_progress * 100.0).clamp(0.0, 100.0);
                    let bar_color = if has_error {
                        egui::Color32::from_rgb(255, 80, 80)
                    } else if export_finished {
                        egui::Color32::from_rgb(0, 230, 140)
                    } else {
                        egui::Color32::from_rgb(0, 215, 255)
                    };

                    let progress_bar = egui::ProgressBar::new(self.export_progress)
                        .show_percentage()
                        .text(format!("{:.1}%", pct))
                        .fill(bar_color)
                        .animate(self.export_in_progress);

                    ui.add_sized([ui.available_width(), 24.0], progress_bar);

                    ui.add_space(8.0);

                    let status_color = if has_error {
                        egui::Color32::from_rgb(255, 120, 120)
                    } else if export_finished {
                        egui::Color32::from_rgb(180, 255, 200)
                    } else {
                        egui::Color32::from_rgb(200, 200, 230)
                    };

                    let display_msg = if let Some(Err(ref err)) = self.export_result {
                        format!("Erro: {}", err)
                    } else if export_finished {
                        "Renderização finalizada e gravada em disco com sucesso!".to_string()
                    } else if !self.export_status_detail.is_empty() {
                        self.export_status_detail.clone()
                    } else {
                        "Processando amostras de áudio...".to_string()
                    };

                    ui.label(
                        egui::RichText::new(display_msg)
                            .size(11.5)
                            .color(status_color),
                    );

                    ui.add_space(12.0);
                    ui.separator();
                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        if let Some(ref path) = self.export_save_path {
                            if export_finished && !has_error {
                                let open_folder_btn = egui::Button::new(
                                    egui::RichText::new("📁 Abrir Pasta").size(12.0),
                                )
                                .min_size(egui::vec2(120.0, 28.0));

                                if ui.add(open_folder_btn).clicked() {
                                    open_file_in_folder(path);
                                }
                            }
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let btn_label = if export_finished {
                                "Concluir"
                            } else {
                                "Ocultar em Segundo Plano"
                            };

                            let close_btn = egui::Button::new(
                                egui::RichText::new(btn_label).strong().size(12.0).color(
                                    if export_finished && !has_error {
                                        egui::Color32::BLACK
                                    } else {
                                        egui::Color32::WHITE
                                    },
                                ),
                            )
                            .fill(if export_finished && !has_error {
                                egui::Color32::from_rgb(0, 255, 180)
                            } else {
                                egui::Color32::from_rgb(60, 50, 80)
                            })
                            .min_size(egui::vec2(100.0, 28.0));

                            if ui.add(close_btn).clicked() {
                                trigger_close = true;
                            }
                        });
                    });
                });

            if trigger_close {
                is_open = false;
            }

            self.export_dialog_open = is_open;
        }

        if self.folder_picker_open {
            let mut is_open = self.folder_picker_open;
            let mut selected_dir: Option<std::path::PathBuf> = None;
            let mut trigger_close = false;

            egui::Window::new("📁 Escolher Pasta do Cantor / Voicebank")
                .open(&mut is_open)
                .collapsible(false)
                .resizable(true)
                .default_size([650.0, 420.0])
                .show(ctx, |ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);

                    ui.horizontal(|ui| {
                        ui.heading(
                            egui::RichText::new("📁 Explorador de Pastas (Android / Local)")
                                .strong()
                                .color(egui::Color32::from_rgb(0, 255, 180)),
                        );
                    });
                    ui.add_space(4.0);

                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            egui::RichText::new("Atalhos:")
                                .strong()
                                .color(egui::Color32::from_rgb(255, 215, 0)),
                        );
                        for (label, path_str) in [
                            ("📥 Downloads", "/sdcard/Download"),
                            ("🎵 Músicas", "/sdcard/Music"),
                            ("📂 Documentos", "/sdcard/Documents"),
                            ("🎤 OpenUtau/Singers", "/sdcard/OpenUtau/Singers"),
                            ("📱 /sdcard", "/sdcard"),
                            ("🏠 Início", "."),
                        ] {
                            let p = std::path::PathBuf::from(path_str);
                            if p.exists() {
                                if ui.small_button(label).clicked() {
                                    self.folder_picker_current_dir = p;
                                }
                            }
                        }
                    });
                    ui.separator();

                    ui.horizontal(|ui| {
                        if let Some(parent) = self.folder_picker_current_dir.parent() {
                            if ui.button("⬆ Subir Nível").clicked() {
                                self.folder_picker_current_dir = parent.to_path_buf();
                            }
                        }
                        ui.label(
                            egui::RichText::new(self.folder_picker_current_dir.display().to_string())
                                .monospace()
                                .color(egui::Color32::from_rgb(200, 220, 255)),
                        );
                    });

                    let has_oto = self.folder_picker_current_dir.join("oto.ini").is_file()
                        || self.folder_picker_current_dir.join("character.txt").is_file();
                    if has_oto {
                        ui.colored_label(
                            egui::Color32::from_rgb(0, 255, 150),
                            "✨ Voicebank detectado nesta pasta (oto.ini / character.txt encontrado)!",
                        );
                    }

                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .id_salt("folder_picker_entries_scroll")
                        .max_height(240.0)
                        .show(ui, |ui| {
                            if let Ok(entries) = std::fs::read_dir(&self.folder_picker_current_dir) {
                                let mut dirs = Vec::new();
                                for entry in entries.flatten() {
                                    if let Ok(file_type) = entry.file_type() {
                                        if file_type.is_dir() {
                                            dirs.push(entry.path());
                                        }
                                    }
                                }
                                dirs.sort();
                                if dirs.is_empty() {
                                    ui.label(
                                        egui::RichText::new("(Nenhuma subpasta encontrada aqui)").italics(),
                                    );
                                } else {
                                    for dir in dirs {
                                        let name = dir
                                            .file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or("?");
                                        let is_vb = dir.join("oto.ini").is_file()
                                            || dir.join("character.txt").is_file();
                                        let icon = if is_vb { "🎤" } else { "📁" };
                                        let text = if is_vb {
                                            format!("{icon} {name} (Voicebank)")
                                        } else {
                                            format!("{icon} {name}")
                                        };
                                        let color = if is_vb {
                                            egui::Color32::from_rgb(0, 255, 200)
                                        } else {
                                            egui::Color32::WHITE
                                        };
                                        if ui.button(egui::RichText::new(text).color(color)).clicked() {
                                            self.folder_picker_current_dir = dir;
                                            break;
                                        }
                                    }
                                }
                            } else {
                                ui.colored_label(egui::Color32::RED, "Acesso restrito ou pasta vazia.");
                            }
                        });

                    ui.add_space(8.0);
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("✅ Selecionar Esta Pasta")
                                        .strong()
                                        .color(egui::Color32::from_rgb(0, 255, 180)),
                                )
                                .fill(egui::Color32::from_rgb(20, 60, 45)),
                            )
                            .clicked()
                        {
                            selected_dir = Some(self.folder_picker_current_dir.clone());
                            trigger_close = true;
                        }
                        if ui.button("❌ Cancelar").clicked() {
                            trigger_close = true;
                        }
                    });
                });

            if trigger_close {
                is_open = false;
            }

            self.folder_picker_open = is_open;

            if let Some(folder) = selected_dir {
                if let Ok(vb) = crate::oto::Voicebank::new(&folder) {
                    self.transport_state.status_message =
                        format!("Voicebank Carregado: {}", vb.name);
                    self.transport_state.voicebank_name = vb.name.clone();
                    self.transport_state.voicebank_path = Some(vb.root_path.clone());
                    self.config.add_recent_voicebank(vb.root_path.clone());
                    self.voicebank = Some(vb);
                    self.persist_config();
                } else {
                    self.transport_state.status_message =
                        format!("Pasta adicionada: {}", folder.display());
                }
                if !self.config.singers_paths.contains(&folder) {
                    self.config.singers_paths.push(folder);
                    self.persist_config();
                    self.reload_singers();
                }
            }
        }
    }
}

fn open_file_in_folder<P: AsRef<std::path::Path>>(path: P) {
    let _p = path.as_ref();
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg("-R").arg(_p).spawn();
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(parent) = _p.parent() {
            let _ = std::process::Command::new("explorer").arg(parent).spawn();
        }
    }
    #[cfg(all(
        not(target_os = "macos"),
        not(target_os = "windows"),
        not(target_os = "android")
    ))]
    {
        if let Some(parent) = _p.parent() {
            let _ = std::process::Command::new("xdg-open").arg(parent).spawn();
        }
    }
}
