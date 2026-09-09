use super::*;

pub(super) fn draw(
    ui: &mut egui::Ui,
    notes: &mut Vec<UNote>,
    state: &mut PianoRollState,
    lang: crate::config::AppLanguage,
    on_before_change: &mut dyn FnMut(),
    on_note_changed: &mut dyn FnMut(),
) {
    if let Some(menu_pos) = state.context_menu_pos {
        let mut close_menu = false;
        let mut trigger_note_changed = false;

        let menu_idx_opt = state.context_menu_note_idx;
        let sel_count = if !state.selected_note_indices.is_empty() {
            state.selected_note_indices.len()
        } else if menu_idx_opt.is_some() {
            1
        } else {
            0
        };

        let screen_rect = ui.ctx().screen_rect();
        let mut safe_menu_pos = menu_pos;
        let total_w = if state.context_menu_hovered_category.is_some() {
            550.0
        } else {
            275.0
        };
        if safe_menu_pos.x + total_w > screen_rect.max.x - 10.0 {
            safe_menu_pos.x = (screen_rect.max.x - total_w - 10.0).max(10.0);
        }
        if safe_menu_pos.y + 440.0 > screen_rect.max.y - 10.0 {
            safe_menu_pos.y = (screen_rect.max.y - 440.0 - 10.0).max(10.0);
        }

        egui::Area::new(egui::Id::new("piano_roll_note_context_menu"))
            .fixed_pos(safe_menu_pos)
            .order(egui::Order::Tooltip)
            .show(ui.ctx(), |ui| {
                ui.scope(|ui| {
                    let mut visuals = ui.visuals().clone();
                    visuals.widgets.hovered.bg_fill = Color32::from_rgb(38, 48, 72);
                    visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(38, 48, 72);
                    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Color32::WHITE);
                    visuals.widgets.hovered.rounding = Rounding::same(4.0);
                    visuals.widgets.active.bg_fill = Color32::from_rgb(48, 62, 92);
                    visuals.widgets.active.weak_bg_fill = Color32::from_rgb(48, 62, 92);
                    visuals.widgets.active.fg_stroke = Stroke::new(1.2, Color32::WHITE);
                    visuals.widgets.active.rounding = Rounding::same(4.0);
                    visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
                    visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
                    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Color32::from_rgb(220, 225, 240));
                    *ui.visuals_mut() = visuals;

                    ui.horizontal_top(|ui| {
                        // ==========================================
                        // PAINEL PRINCIPAL (ESQUERDA)
                        // ==========================================
                        egui::Frame::menu(ui.style())
                            .fill(MelodyneTheme::BG_PANEL)
                            .stroke(Stroke::new(1.0_f32, MelodyneTheme::ACCENT_GOLD))
                            .rounding(Rounding::same(6.0))
                            .shadow(egui::epaint::Shadow {
                                offset: Vec2::new(0.0, 4.0),
                                blur: 14.0,
                                spread: 0.0,
                                color: Color32::from_black_alpha(200),
                            })
                            .inner_margin(egui::Margin::same(6.0))
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.set_width(245.0);

                                if let Some(n_idx) = menu_idx_opt {
                                    let note_info = if n_idx < notes.len() {
                                        format!("{} '{}' [{}]", lang.tr("Nota", "Note"), notes[n_idx].lyric, notes[n_idx].pitch)
                                    } else {
                                        lang.tr("Nota Selecionada", "Selected Note").to_string()
                                    };
                                    let header = if sel_count > 1 {
                                        format!("{} ({} {})", note_info, sel_count, lang.tr("sel.", "sel."))
                                    } else {
                                        note_info
                                    };
                                    ui.label(
                                        egui::RichText::new(header)
                                            .size(11.5)
                                            .color(MelodyneTheme::TEXT_GOLD_LABEL)
                                            .strong(),
                                    );
                                } else {
                                    let header = if sel_count > 0 {
                                        format!("Piano Roll ({} {})", sel_count, lang.tr(if sel_count > 1 { "selecionadas" } else { "selecionada" }, if sel_count > 1 { "selected" } else { "selected" }))
                                    } else {
                                        lang.tr("Piano Roll — Menu de Edição", "Piano Roll — Edit Menu").to_string()
                                    };
                                    ui.label(
                                        egui::RichText::new(header)
                                            .size(11.5)
                                            .color(MelodyneTheme::TEXT_GOLD_LABEL)
                                            .strong(),
                                    );
                                }
                                ui.separator();

                                // 1. Edição Básica
                                let r_undo = ui.button(lang.tr("Desfazer  (Ctrl+Z)", "Undo  (Ctrl+Z)"));
                                if r_undo.hovered() { state.context_menu_hovered_category = None; }
                                if r_undo.clicked() {
                                    state.request_undo = true;
                                    close_menu = true;
                                }

                                let r_redo = ui.button(lang.tr("Refazer  (Ctrl+Y)", "Redo  (Ctrl+Y)"));
                                if r_redo.hovered() { state.context_menu_hovered_category = None; }
                                if r_redo.clicked() {
                                    state.request_redo = true;
                                    close_menu = true;
                                }

                                ui.separator();

                                let r_cut = ui.button(lang.tr("Recortar  (Ctrl+X)", "Cut  (Ctrl+X)"));
                                if r_cut.hovered() { state.context_menu_hovered_category = None; }
                                if r_cut.clicked() {
                                    state.request_cut = true;
                                    close_menu = true;
                                }

                                let r_copy = ui.button(lang.tr("Copiar  (Ctrl+C)", "Copy  (Ctrl+C)"));
                                if r_copy.hovered() { state.context_menu_hovered_category = None; }
                                if r_copy.clicked() {
                                    state.request_copy = true;
                                    close_menu = true;
                                }

                                let r_paste = ui.button(lang.tr("Colar  (Ctrl+V)", "Paste  (Ctrl+V)"));
                                if r_paste.hovered() { state.context_menu_hovered_category = None; }
                                if r_paste.clicked() {
                                    state.request_paste = true;
                                    close_menu = true;
                                }

                                let r_dup = ui.button(lang.tr("Duplicar Nota(s)  (Ctrl+D)", "Duplicate Note(s)  (Ctrl+D)"));
                                if r_dup.hovered() { state.context_menu_hovered_category = None; }
                                if r_dup.clicked() {
                                    on_before_change();
                                    let mut new_notes = Vec::new();
                                    let targets: Vec<usize> = if !state.selected_note_indices.is_empty() {
                                        state.selected_note_indices.iter().copied().collect()
                                    } else if let Some(idx) = menu_idx_opt {
                                        vec![idx]
                                    } else {
                                        Vec::new()
                                    };
                                    for idx in targets {
                                        if idx < notes.len() {
                                            let mut dup = notes[idx].clone();
                                            dup.position_ms += dup.duration_ms;
                                            new_notes.push(dup);
                                        }
                                    }
                                    let start_new_idx = notes.len();
                                    notes.extend(new_notes);
                                    state.selected_note_indices.clear();
                                    for i in start_new_idx..notes.len() {
                                        state.selected_note_indices.insert(i);
                                    }
                                    if start_new_idx < notes.len() {
                                        state.selected_note_index = Some(start_new_idx);
                                    }
                                    trigger_note_changed = true;
                                    close_menu = true;
                                }

                                let r_del = ui.button(lang.tr("Excluir  (Delete)", "Delete  (Delete)"));
                                if r_del.hovered() { state.context_menu_hovered_category = None; }
                                if r_del.clicked() {
                                    on_before_change();
                                    let mut to_delete: HashSet<usize> = state.selected_note_indices.clone();
                                    if to_delete.is_empty() {
                                        if let Some(idx) = menu_idx_opt {
                                            to_delete.insert(idx);
                                        }
                                    }
                                    let mut indices: Vec<usize> = to_delete.into_iter().collect();
                                    indices.sort_unstable_by(|a, b| b.cmp(a));
                                    for idx in indices {
                                        if idx < notes.len() {
                                            notes.remove(idx);
                                        }
                                    }
                                    state.selected_note_indices.clear();
                                    state.selected_note_index = None;
                                    trigger_note_changed = true;
                                    close_menu = true;
                                }

                                ui.separator();

                                let r_reset_params = ui.button(
                                    egui::RichText::new(lang.tr("Limpar Todos os Parâmetros (Reset Geral)", "Reset All Parameters (Global Reset)"))
                                        .color(Color32::from_rgb(255, 180, 90))
                                        .strong(),
                                );
                                if r_reset_params.hovered() { state.context_menu_hovered_category = None; }
                                if r_reset_params.clicked() {
                                    on_before_change();
                                    let target_indices: Vec<usize> = if !state.selected_note_indices.is_empty() {
                                        state.selected_note_indices.iter().copied().collect()
                                    } else if let Some(idx) = menu_idx_opt {
                                        vec![idx]
                                    } else {
                                        (0..notes.len()).collect()
                                    };
                                    for idx in target_indices {
                                        if idx < notes.len() {
                                            notes[idx].reset_all_parameters();
                                        }
                                    }
                                    state.phoneme_cache_hash = 0;
                                    trigger_note_changed = true;
                                    close_menu = true;
                                }

                                ui.separator();

                                let r_export_demo = ui.button(
                                    egui::RichText::new(lang.tr("Exportar Seleção em Áudio (Demo / WIP)...", "Export Selection Audio (Demo / WIP)..."))
                                        .color(Color32::from_rgb(0, 255, 180))
                                        .strong(),
                                );
                                if r_export_demo.hovered() { state.context_menu_hovered_category = None; }
                                if r_export_demo.clicked() {
                                    state.request_export_selection_audio = true;
                                    close_menu = true;
                                }

                                ui.separator();

                                // Categorias com abertura automática ao passar o mouse (Hover)
                                let mut draw_category_item = |label: &str, cat: ContextMenuCategory| {
                                    let is_active = state.context_menu_hovered_category == Some(cat);
                                    let text_color = if is_active {
                                        Color32::from_rgb(0, 255, 180)
                                    } else {
                                        Color32::from_rgb(220, 225, 240)
                                    };
                                    let text = egui::RichText::new(format!("{} ▶", label))
                                        .size(11.0)
                                        .color(text_color);
                                    let btn = egui::Button::new(if is_active { text.strong() } else { text })
                                        .fill(if is_active {
                                            Color32::from_rgb(36, 46, 70)
                                        } else {
                                            Color32::TRANSPARENT
                                        })
                                        .stroke(if is_active {
                                            Stroke::new(1.0, Color32::from_rgb(0, 200, 160))
                                        } else {
                                            Stroke::NONE
                                        })
                                        .rounding(Rounding::same(4.0));
                                    let r = ui.add_sized([ui.available_width(), 22.0], btn);
                                    if r.hovered() {
                                        state.context_menu_hovered_category = Some(cat);
                                        state.context_menu_category_y_offset = (r.rect.min.y - safe_menu_pos.y).max(0.0);
                                    }
                                };

                                draw_category_item(lang.tr("Seleção", "Selection"), ContextMenuCategory::Selection);
                                draw_category_item(lang.tr("Quantização & Grade", "Quantization & Grid"), ContextMenuCategory::QuantizeGrade);
                                draw_category_item(lang.tr("Letras & Fonemas", "Lyrics & Phonemes"), ContextMenuCategory::LyricsPhonemes);
                                draw_category_item(lang.tr("Transformações Musicais", "Musical Transforms"), ContextMenuCategory::MusicalTransform);
                                draw_category_item(lang.tr("Transposição", "Transpose"), ContextMenuCategory::Transpose);
                                draw_category_item(lang.tr("Pre-tunning & Afinação", "Pre-tunning & Tuning"), ContextMenuCategory::AutoPitch);
                                draw_category_item(lang.tr("Presets de Vibrato Vocal", "Vocal Vibrato Presets"), ContextMenuCategory::Vibrato);

                                if let Some(n_idx) = menu_idx_opt {
                                    ui.separator();
                                    let r_prop = ui.button(
                                        egui::RichText::new(lang.tr("Propriedades Detalhadas da Nota...", "Detailed Note Properties..."))
                                            .color(Color32::from_rgb(230, 220, 240)),
                                    );
                                    if r_prop.hovered() { state.context_menu_hovered_category = None; }
                                    if r_prop.clicked() {
                                        state.properties_window_for_note = Some(n_idx);
                                        close_menu = true;
                                    }
                                }
                            });
                        });

                    // ==========================================
                    // SUB-PAINEL FLYOUT AO PASSAR O MOUSE (DIREITA)
                    // ==========================================
                    if let Some(cat) = state.context_menu_hovered_category {
                        ui.add_space(2.0);
                        ui.vertical(|ui| {
                            let approx_h = match cat {
                                ContextMenuCategory::Selection => 230.0,
                                ContextMenuCategory::QuantizeGrade => 160.0,
                                ContextMenuCategory::LyricsPhonemes => 170.0,
                                ContextMenuCategory::MusicalTransform => 130.0,
                                ContextMenuCategory::Transpose => 160.0,
                                ContextMenuCategory::AutoPitch => 360.0,
                                ContextMenuCategory::Vibrato => 190.0,
                            };
                            let max_offset = (screen_rect.max.y - 15.0 - safe_menu_pos.y - approx_h).max(0.0);
                            let aligned_y = state.context_menu_category_y_offset.min(max_offset).max(0.0);
                            if aligned_y > 0.0 {
                                ui.add_space(aligned_y);
                            }

                            egui::Frame::menu(ui.style())
                                .fill(MelodyneTheme::BG_PANEL)
                                .stroke(Stroke::new(1.0_f32, MelodyneTheme::ACCENT_CYAN))
                                .rounding(Rounding::same(6.0))
                                .shadow(egui::epaint::Shadow {
                                    offset: Vec2::new(0.0, 4.0),
                                    blur: 14.0,
                                    spread: 0.0,
                                    color: Color32::from_black_alpha(200),
                                })
                                .inner_margin(egui::Margin::same(6.0))
                                .show(ui, |ui| {
                                    ui.vertical(|ui| {
                                        ui.set_width(260.0);

                                    match cat {
                                        ContextMenuCategory::Selection => {
                                            ui.label(egui::RichText::new(lang.tr("Seleção", "Selection")).size(11.0).color(MelodyneTheme::ACCENT_CYAN).strong());
                                            ui.separator();
                                            if ui.button(lang.tr("Selecionar Tudo (Ctrl+A)", "Select All (Ctrl+A)")).clicked() {
                                                state.selected_note_indices = (0..notes.len()).collect();
                                                if !notes.is_empty() {
                                                    state.selected_note_index = Some(0);
                                                }
                                                close_menu = true;
                                            }
                                            if ui.button(lang.tr("Desmarcar Seleção (Ctrl+Shift+A)", "Deselect (Ctrl+Shift+A)")).clicked() {
                                                state.selected_note_indices.clear();
                                                state.selected_note_index = None;
                                                close_menu = true;
                                            }
                                            ui.separator();
                                            if ui.button(
                                                egui::RichText::new(lang.tr("Exportar Seleção em Áudio (Demo / WIP)...", "Export Selected Audio (Demo / WIP)..."))
                                                    .color(Color32::from_rgb(0, 255, 180))
                                                    .strong(),
                                            ).clicked() {
                                                state.request_export_selection_audio = true;
                                                close_menu = true;
                                            }
                                            ui.separator();
                                            ui.label(egui::RichText::new(lang.tr("Seleção Inteligente", "Smart Selection")).size(10.0).color(Color32::from_rgb(180, 180, 200)));
                                            if ui.button(lang.tr("Selecionar Notas Curtas (< 120ms)", "Select Short Notes (< 120ms)")).clicked() {
                                                state.selected_note_indices.clear();
                                                for (i, n) in notes.iter().enumerate() {
                                                    if n.duration_ms < 120.0 {
                                                        state.selected_note_indices.insert(i);
                                                    }
                                                }
                                                state.selected_note_index = state.selected_note_indices.iter().next().copied();
                                                close_menu = true;
                                            }
                                            if ui.button(lang.tr("Selecionar Notas Muito Curtas (< 60ms)", "Select Very Short Notes (< 60ms)")).clicked() {
                                                state.selected_note_indices.clear();
                                                for (i, n) in notes.iter().enumerate() {
                                                    if n.duration_ms < 60.0 {
                                                        state.selected_note_indices.insert(i);
                                                    }
                                                }
                                                state.selected_note_index = state.selected_note_indices.iter().next().copied();
                                                close_menu = true;
                                            }
                                            if ui.button(lang.tr("Selecionar Notas Sobrepostas", "Select Overlapping Notes")).clicked() {
                                                state.request_select_overlapping = true;
                                                close_menu = true;
                                            }
                                            if ui.button(lang.tr("Selecionar Fora da Escala Ativa", "Select Out of Active Scale")).clicked() {
                                                state.request_select_out_of_scale = true;
                                                close_menu = true;
                                            }
                                        }

                                        ContextMenuCategory::QuantizeGrade => {
                                            ui.label(egui::RichText::new(lang.tr("Quantização & Grade", "Quantization & Grid")).size(11.0).color(MelodyneTheme::ACCENT_CYAN).strong());
                                            ui.separator();
                                            if ui.button(lang.tr("Alinhar / Quantizar Snap Atual", "Align / Quantize to Current Snap")).clicked() {
                                                state.request_quantize_snap = true;
                                                close_menu = true;
                                            }
                                            if ui.button(lang.tr("Quantizar Durações", "Quantize Durations")).clicked() {
                                                state.request_quantize_durations = true;
                                                close_menu = true;
                                            }
                                            if ui.button(lang.tr("Corrigir Sobreposição (Trim Overlaps)", "Fix Overlaps (Trim Overlaps)")).clicked() {
                                                state.request_fix_overlaps = true;
                                                close_menu = true;
                                            }
                                            if ui.button(lang.tr("Conectar Finais das Notas (Legato)", "Connect Note Ends (Legato)")).clicked() {
                                                state.request_legato = true;
                                                close_menu = true;
                                            }
                                        }

                                        ContextMenuCategory::LyricsPhonemes => {
                                            ui.label(egui::RichText::new(lang.tr("Letras & Fonemas", "Lyrics & Phonemes")).size(11.0).color(MelodyneTheme::ACCENT_CYAN).strong());
                                            ui.separator();
                                            if ui.button(lang.tr("Inserir Letras em Lote... (Ctrl+Shift+L)", "Insert Batch Lyrics... (Ctrl+Shift+L)")).clicked() {
                                                state.request_batch_lyrics = true;
                                                close_menu = true;
                                            }
                                            if ui.button(lang.tr("Limpar Sufixos de Afinação (_A3, _C4...)", "Clean Pitch Suffixes (_A3, _C4...)")).clicked() {
                                                state.request_clean_pitch_suffixes = true;
                                                close_menu = true;
                                            }
                                            if ui.button(lang.tr("Forçar Atualização de Fonemas", "Force Phoneme Update")).clicked() {
                                                state.request_rephonemize = true;
                                                close_menu = true;
                                            }
                                            ui.separator();
                                            if ui.button(
                                                egui::RichText::new(lang.tr("Resetar Tempos dos Fonemas", "Reset Phoneme Timings"))
                                                    .color(Color32::from_rgb(255, 205, 70)),
                                            ).clicked() {
                                                on_before_change();
                                                let target_indices: Vec<usize> = if !state.selected_note_indices.is_empty() {
                                                    state.selected_note_indices.iter().copied().collect()
                                                } else if let Some(idx) = menu_idx_opt {
                                                    vec![idx]
                                                } else {
                                                    (0..notes.len()).collect()
                                                };
                                                for idx in target_indices {
                                                    if idx < notes.len() {
                                                        notes[idx].phoneme_durations_ms.clear();
                                                        notes[idx].expressions.consonant_timing_offset_ms = 0.0;
                                                        notes[idx].expressions.preutter_offset_ms = 0.0;
                                                        notes[idx].expressions.overlap_offset_ms = 0.0;
                                                    }
                                                }
                                                state.phoneme_cache_hash = 0;
                                                trigger_note_changed = true;
                                                close_menu = true;
                                            }
                                            if ui.button(
                                                egui::RichText::new(lang.tr("Limpar Todos os Parâmetros da Nota", "Reset All Note Parameters"))
                                                    .color(Color32::from_rgb(255, 180, 90)),
                                            ).clicked() {
                                                on_before_change();
                                                let target_indices: Vec<usize> = if !state.selected_note_indices.is_empty() {
                                                    state.selected_note_indices.iter().copied().collect()
                                                } else if let Some(idx) = menu_idx_opt {
                                                    vec![idx]
                                                } else {
                                                    (0..notes.len()).collect()
                                                };
                                                for idx in target_indices {
                                                    if idx < notes.len() {
                                                        notes[idx].reset_all_parameters();
                                                    }
                                                }
                                                state.phoneme_cache_hash = 0;
                                                trigger_note_changed = true;
                                                close_menu = true;
                                            }
                                        }

                                        ContextMenuCategory::MusicalTransform => {
                                            ui.label(egui::RichText::new(lang.tr("Transformações Musicais", "Musical Transforms")).size(11.0).color(MelodyneTheme::ACCENT_CYAN).strong());
                                            ui.separator();
                                            if ui.button(lang.tr("Humanizar Posições e Velocities", "Humanize Positions and Velocities")).clicked() {
                                                state.request_humanize = true;
                                                close_menu = true;
                                            }
                                            if ui.button(lang.tr("Inverter Melodia (Retrógrado)", "Invert Melody (Retrograde)")).clicked() {
                                                state.request_invert_retrograde = true;
                                                close_menu = true;
                                            }
                                            if ui.button(lang.tr("Espelhar Intervalos (Inversão)", "Mirror Intervals (Inversion)")).clicked() {
                                                state.request_invert_intervals = true;
                                                close_menu = true;
                                            }
                                        }

                                        ContextMenuCategory::Transpose => {
                                            ui.label(egui::RichText::new(lang.tr("Transposição", "Transpose")).size(11.0).color(MelodyneTheme::ACCENT_CYAN).strong());
                                            ui.separator();
                                            let mut apply_transpose = |semitones: i32| {
                                                on_before_change();
                                                let target_indices: Vec<usize> = if !state.selected_note_indices.is_empty() {
                                                    state.selected_note_indices.iter().copied().collect()
                                                } else if let Some(idx) = menu_idx_opt {
                                                    vec![idx]
                                                } else {
                                                    (0..notes.len()).collect()
                                                };
                                                for idx in target_indices {
                                                    if idx < notes.len() {
                                                        let cur_midi = notes[idx].midi_key() as i32;
                                                        let new_midi = (cur_midi + semitones).clamp(state.min_midi as i32, state.max_midi as i32) as u8;
                                                        notes[idx].pitch = midi_to_note_name(new_midi);
                                                    }
                                                }
                                                trigger_note_changed = true;
                                                close_menu = true;
                                             };

                                            if ui.button(lang.tr("Transpor +1 Semitom (Seta Cima)", "Transpose +1 Semitone (Up Arrow)")).clicked() {
                                                apply_transpose(1);
                                            }
                                            if ui.button(lang.tr("Transpor -1 Semitom (Seta Baixo)", "Transpose -1 Semitone (Down Arrow)")).clicked() {
                                                apply_transpose(-1);
                                            }
                                            if ui.button(lang.tr("Transpor +1 Oitava (+12)", "Transpose +1 Octave (+12)")).clicked() {
                                                apply_transpose(12);
                                            }
                                            if ui.button(lang.tr("Transpor -1 Oitava (-12)", "Transpose -1 Octave (-12)")).clicked() {
                                                apply_transpose(-12);
                                            }
                                        }

                                        ContextMenuCategory::AutoPitch => {
                                            ui.label(egui::RichText::new(lang.tr("Pre-tunning & Afinação", "Pre-tunning & Tuning")).size(11.0).color(MelodyneTheme::ACCENT_CYAN).strong());
                                            ui.separator();
                                            if ui
                                                .button(
                                                    egui::RichText::new(lang.tr("Pre-tunning Suave / Pop", "Smooth / Pop Pre-tunning"))
                                                        .color(Color32::from_rgb(0, 240, 255))
                                                        .strong(),
                                                )
                                                .clicked()
                                            {
                                                apply_autopitch_to_selection(notes, &state.selected_note_indices, AutoPitchStyle::SmoothPop);
                                                trigger_note_changed = true;
                                                close_menu = true;
                                            }
                                            if ui
                                                .button(
                                                    egui::RichText::new(lang.tr("Pre-tunning Natural (Humano)", "Natural Pre-tunning (Human)"))
                                                        .color(MelodyneTheme::NOTE_SELECTED_GOLD),
                                                )
                                                .clicked()
                                            {
                                                apply_autopitch_to_selection(notes, &state.selected_note_indices, AutoPitchStyle::Natural);
                                                trigger_note_changed = true;
                                                close_menu = true;
                                            }
                                            if ui
                                                .button(
                                                    egui::RichText::new(lang.tr("Pre-tunning Balada / Emotivo", "Ballad / Expressive Pre-tunning"))
                                                        .color(Color32::from_rgb(255, 120, 160)),
                                                )
                                                .clicked()
                                            {
                                                apply_autopitch_to_selection(notes, &state.selected_note_indices, AutoPitchStyle::Expressive);
                                                trigger_note_changed = true;
                                                close_menu = true;
                                            }
                                            if ui
                                                .button(
                                                    egui::RichText::new(lang.tr("Pre-tunning Rock / Belting", "Rock / Belting Pre-tunning"))
                                                        .color(Color32::from_rgb(255, 180, 50)),
                                                )
                                                .clicked()
                                            {
                                                apply_autopitch_to_selection(notes, &state.selected_note_indices, AutoPitchStyle::RockPower);
                                                trigger_note_changed = true;
                                                close_menu = true;
                                            }
                                            if ui
                                                .button(
                                                    egui::RichText::new(lang.tr("Pre-tunning R&B / Soul", "R&B / Soul Pre-tunning"))
                                                        .color(Color32::from_rgb(180, 130, 255)),
                                                )
                                                .clicked()
                                            {
                                                apply_autopitch_to_selection(notes, &state.selected_note_indices, AutoPitchStyle::RnBSoul);
                                                trigger_note_changed = true;
                                                close_menu = true;
                                            }
                                            if ui
                                                .button(
                                                    egui::RichText::new(lang.tr("Pre-tunning Hard Tune / Snap", "Hard Tune / Snap Pre-tunning"))
                                                        .color(Color32::from_rgb(0, 255, 180)),
                                                )
                                                .clicked()
                                            {
                                                apply_autopitch_to_selection(notes, &state.selected_note_indices, AutoPitchStyle::HardTune);
                                                trigger_note_changed = true;
                                                close_menu = true;
                                            }
                                            if ui
                                                .button(
                                                    egui::RichText::new(lang.tr("Pre-tunning Folk / Acústico", "Folk / Acoustic Pre-tunning"))
                                                        .color(Color32::from_rgb(220, 200, 150)),
                                                )
                                                .clicked()
                                            {
                                                apply_autopitch_to_selection(notes, &state.selected_note_indices, AutoPitchStyle::AcousticFolk);
                                                trigger_note_changed = true;
                                                close_menu = true;
                                            }
                                            if ui
                                                .button(
                                                    egui::RichText::new(lang.tr("Pre-tunning Lírico / Ópera", "Lyrical / Opera Pre-tunning"))
                                                        .color(Color32::from_rgb(150, 220, 255)),
                                                )
                                                .clicked()
                                            {
                                                apply_autopitch_to_selection(notes, &state.selected_note_indices, AutoPitchStyle::LyricalOpera);
                                                trigger_note_changed = true;
                                                close_menu = true;
                                            }

                                            ui.separator();
                                            if ui.button(lang.tr("Pre-tunning... (Ctrl+Alt+P)", "Pre-tunning... (Ctrl+Alt+P)")).clicked() {
                                                state.request_autopitch_window = true;
                                                close_menu = true;
                                            }
                                            if ui.button(lang.tr("Aplicar Pre-tunning Suave em Tudo", "Apply Soft Pre-tunning to All")).clicked() {
                                                state.request_autopitch_all = true;
                                                close_menu = true;
                                            }

                                            ui.separator();
                                            if ui
                                                .button(
                                                    egui::RichText::new(lang.tr("Limpar Curvas de Pitch e Vibrato", "Clear Pitch Curves and Vibrato"))
                                                        .color(Color32::from_rgb(200, 190, 210)),
                                                )
                                                .clicked()
                                            {
                                                for &idx in &state.selected_note_indices {
                                                    if idx < notes.len() {
                                                        notes[idx].pitch_bend.points.clear();
                                                        notes[idx].vibrato.length_pct = 0.0;
                                                    }
                                                }
                                                trigger_note_changed = true;
                                                close_menu = true;
                                            }
                                            if ui
                                                .button(
                                                    egui::RichText::new(lang.tr("Resetar Envelopes de Volume", "Reset Volume Envelopes"))
                                                        .color(Color32::from_rgb(200, 190, 210)),
                                                )
                                                .clicked()
                                            {
                                                for &idx in &state.selected_note_indices {
                                                    if idx < notes.len() {
                                                        notes[idx].envelope = crate::dsp::envelope::UtauEnvelope::default();
                                                    }
                                                }
                                                trigger_note_changed = true;
                                                close_menu = true;
                                            }
                                        }

                                        ContextMenuCategory::Vibrato => {
                                            ui.label(egui::RichText::new(lang.tr("Presets de Vibrato Vocal", "Vocal Vibrato Presets")).size(11.0).color(MelodyneTheme::ACCENT_CYAN).strong());
                                            ui.separator();
                                            let mut apply_vibrato = |len: f64, depth: f64, period: f64, f_in: f64, f_out: f64| {
                                                let targets: Vec<usize> = if !state.selected_note_indices.is_empty() {
                                                    state.selected_note_indices.iter().copied().collect()
                                                } else if let Some(idx) = menu_idx_opt {
                                                    vec![idx]
                                                } else {
                                                    (0..notes.len()).collect()
                                                };
                                                for idx in targets {
                                                    if idx < notes.len() {
                                                        notes[idx].vibrato.length_pct = len;
                                                        notes[idx].vibrato.depth_cents = depth;
                                                        notes[idx].vibrato.period_ms = period;
                                                        notes[idx].vibrato.fade_in_pct = f_in;
                                                        notes[idx].vibrato.fade_out_pct = f_out;
                                                    }
                                                }
                                                trigger_note_changed = true;
                                                close_menu = true;
                                            };

                                            if ui.button(lang.tr("Pop Suave (65% / 48c / 5.7 Hz)", "Soft Pop (65% / 48c / 5.7 Hz)")).clicked() {
                                                apply_vibrato(65.0, 48.0, 175.0, 25.0, 15.0);
                                            }
                                            if ui.button(lang.tr("Dramático (75% / 75c / 6.2 Hz)", "Dramatic (75% / 75c / 6.2 Hz)")).clicked() {
                                                apply_vibrato(75.0, 75.0, 160.0, 20.0, 10.0);
                                            }
                                            if ui.button(lang.tr("Balada (80% / 50c / 4.5 Hz)", "Ballad (80% / 50c / 4.5 Hz)")).clicked() {
                                                apply_vibrato(80.0, 50.0, 220.0, 35.0, 15.0);
                                            }
                                            if ui.button(lang.tr("Rápido (60% / 60c / 7.0 Hz)", "Fast (60% / 60c / 7.0 Hz)")).clicked() {
                                                apply_vibrato(60.0, 60.0, 140.0, 20.0, 10.0);
                                            }
                                            if ui.button(lang.tr("Desligar Vibrato", "Turn Off Vibrato")).clicked() {
                                                apply_vibrato(0.0, 0.0, 175.0, 0.0, 0.0);
                                            }
                                        }
                                    }
                                });
                            });
                        });
                    }
                });
            });
        });

        if ui.input(|i| i.pointer.primary_clicked() || i.pointer.secondary_clicked()) {
            if let Some(mpos) = ui.input(|i| i.pointer.interact_pos()) {
                let total_w = if state.context_menu_hovered_category.is_some() {
                    570.0
                } else {
                    280.0
                };
                let menu_rect = Rect::from_min_size(
                    safe_menu_pos - Vec2::new(10.0, 10.0),
                    Vec2::new(total_w, 650.0),
                );
                if !menu_rect.contains(mpos) {
                    close_menu = true;
                }
            }
        }

        if close_menu {
            state.context_menu_note_idx = None;
            state.context_menu_pos = None;
            state.context_menu_hovered_category = None;
        }

        if trigger_note_changed {
            state.continuous_edit_dirty = true;
            on_note_changed();
        }
    }
}
