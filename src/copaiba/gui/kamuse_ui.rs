use super::*;

pub(super) fn draw_kamuse_authoring_ui(
    app: &mut CopaibaToolkitApp,
    audio_player: &mut AudioPlayer,
    ui: &mut egui::Ui,
) {
    let Some(project) = app.kamuse_project.as_ref() else {
        return;
    };
    let recording_count: usize = project
        .cantor
        .tons
        .iter()
        .map(|tone| tone.gravacoes.len())
        .sum();
    let unit_count: usize = project
        .cantor
        .tons
        .iter()
        .flat_map(|tone| &tone.gravacoes)
        .map(|recording| {
            recording
                .fonemas
                .windows(2)
                .filter(|pair| {
                    let cv = !is_vowel_phone(&pair[0]) && is_vowel_phone(&pair[1]);
                    let vc = is_vowel_phone(&pair[0]) && !is_vowel_phone(&pair[1]);
                    cv || vc
                })
                .count()
        })
        .sum();
    egui::Frame::none()
        .fill(visual_tokens::PANEL)
        .stroke(Stroke::new(1.0_f32, visual_tokens::BORDER))
        .rounding(Rounding::same(16.0))
        .inner_margin(egui::Margin::same(18.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new("KAMUSE  /  ESTÚDIO DE VOZ")
                            .strong()
                            .size(10.0)
                            .color(visual_tokens::ACCENT),
                    );
                    ui.add_space(4.0);
                    ui.heading(RichText::new(&project.cantor.nome).strong().size(25.0));
                    ui.label(
                        RichText::new(format!(
                            "{}  ·  {}",
                            project.cantor.id,
                            project.cantor.idiomas.join(" / ")
                        ))
                        .size(12.0)
                        .color(visual_tokens::MUTED),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    egui::Frame::none()
                        .fill(visual_tokens::ACCENT_DARK)
                        .rounding(Rounding::same(20.0))
                        .inner_margin(egui::Margin::symmetric(12.0, 7.0))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new("●  TAINAH · WORLD")
                                    .strong()
                                    .size(10.0)
                                    .color(visual_tokens::ACCENT),
                            );
                        });
                });
            });
            ui.add_space(14.0);
            ui.horizontal_wrapped(|ui| {
                metric_chip(ui, "TONS", project.cantor.tons.len().to_string());
                metric_chip(ui, "GRAVAÇÕES", recording_count.to_string());
                metric_chip(ui, "LIMITES FONÉTICOS", unit_count.to_string());
                if app.kamuse_marks_dirty {
                    metric_chip_color(ui, "ALTERAÇÕES", "não salvas", visual_tokens::WARNING);
                } else {
                    metric_chip_color(ui, "PROJETO", "em dia", visual_tokens::SUCCESS);
                }
            });
        });
    ui.add_space(12.0);
    let available_width = ui.available_width();
    if available_width >= 1024.0 {
        ui.horizontal_top(|ui| {
            let sidebar_width = (ui.available_width() * 0.36).clamp(290.0, 390.0);
            let available_height = ui.available_height();
            ui.allocate_ui_with_layout(
                egui::vec2(sidebar_width, available_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::Frame::none()
                        .fill(visual_tokens::PANEL)
                        .stroke(Stroke::new(1.0_f32, visual_tokens::BORDER))
                        .rounding(Rounding::same(12.0))
                        .inner_margin(egui::Margin::same(12.0))
                        .show(ui, |ui| {
                            egui::ScrollArea::vertical()
                                .id_salt("kamuse_authoring_sidebar_scroll")
                                .show(ui, |ui| {
                                    draw_kamuse_library(app, ui);
                                    ui.add_space(12.0);
                                    draw_kamuse_authoring_tools(app, ui);
                                });
                        });
                },
            );
            ui.separator();
            let editor_width = ui.available_width().max(1.0);
            ui.allocate_ui_with_layout(
                egui::vec2(editor_width, available_height),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    egui::ScrollArea::vertical()
                        .id_salt("kamuse_editor_scroll")
                        .show(ui, |ui| draw_kamuse_editor(app, audio_player, ui));
                },
            );
        });
    } else {
        ui.horizontal_wrapped(|ui| {
            for view in ["Editor", "Gravações", "Preparação"] {
                let selected = app.kamuse_active_view == view;
                if ui.selectable_label(selected, view).clicked() {
                    app.kamuse_active_view = view.into();
                }
            }
        });
        ui.add_space(6.0);
        match app.kamuse_active_view.as_str() {
            "Gravações" => egui::ScrollArea::vertical()
                .id_salt("kamuse_narrow_library")
                .show(ui, |ui| draw_kamuse_library(app, ui)),
            "Preparação" => egui::ScrollArea::vertical()
                .id_salt("kamuse_narrow_tools")
                .show(ui, |ui| draw_kamuse_authoring_tools(app, ui)),
            _ => egui::ScrollArea::vertical()
                .id_salt("kamuse_editor_scroll")
                .show(ui, |ui| draw_kamuse_editor(app, audio_player, ui)),
        };
    }
}

fn draw_kamuse_library(app: &mut CopaibaToolkitApp, ui: &mut egui::Ui) {
    let Some(project) = app.kamuse_project.as_ref() else {
        return;
    };
    egui::Frame::none()
        .fill(visual_tokens::SURFACE)
        .stroke(Stroke::new(1.0_f32, visual_tokens::BORDER))
        .rounding(Rounding::same(12.0))
        .inner_margin(egui::Margin::same(12.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading(RichText::new("Gravações").strong().size(16.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(format!(
                            "{} takes",
                            project
                                .cantor
                                .tons
                                .iter()
                                .map(|t| t.gravacoes.len())
                                .sum::<usize>()
                        ))
                        .small()
                        .color(visual_tokens::MUTED),
                    );
                });
            });
            ui.label(
                RichText::new("Selecione um take para revisar fonemas e marcações.")
                    .size(11.0)
                    .color(visual_tokens::MUTED),
            );
            ui.add_space(6.0);
            if project.cantor.tons.iter().all(|t| t.gravacoes.is_empty()) {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::Image::from_bytes(
                            "bytes://kamuse-empty.svg",
                            include_bytes!("../../../assets/kamuse/empty.svg"),
                        )
                        .fit_to_exact_size(egui::vec2(72.0, 72.0)),
                    );
                    ui.label("Ainda não há takes. Vá a Preparação para importar um WAV ou MP3.");
                });
            } else {
                egui::ScrollArea::vertical()
                    .max_height(220.0)
                    .show(ui, |ui| {
                        for tone in &project.cantor.tons {
                            ui.label(
                                RichText::new(format!("{} · MIDI {}", tone.id, tone.midi))
                                    .strong()
                                    .color(visual_tokens::ACCENT),
                            );
                            for take in &tone.gravacoes {
                                let selected = app.kamuse_selected_tone.as_deref()
                                    == Some(&tone.id)
                                    && app.kamuse_selected_recording.as_deref() == Some(&take.id);
                                let label = format!("{}   ·   {}", take.id, take.fonemas.join(" "));
                                if ui
                                    .selectable_label(selected, label)
                                    .on_hover_text("Abrir este take no editor de marcações")
                                    .clicked()
                                {
                                    app.kamuse_selected_tone = Some(tone.id.clone());
                                    app.kamuse_selected_recording = Some(take.id.clone());
                                    app.kamuse_active_view = "Editor".into();
                                }
                            }
                        }
                    });
            }
        });
}

fn draw_kamuse_authoring_tools(app: &mut CopaibaToolkitApp, ui: &mut egui::Ui) {
    section_title(
        ui,
        "01",
        "Preparar gravação",
        "Prompts, perfil e importação.",
    );
    egui::Frame::none()
        .fill(visual_tokens::SURFACE)
        .stroke(Stroke::new(1.0_f32, visual_tokens::BORDER))
        .rounding(Rounding::same(12.0))
        .inner_margin(egui::Margin::same(12.0))
        .show(ui, |ui| {
            ui.label(
                RichText::new("PERFIL E AJUSTES")
                    .strong()
                    .size(10.0)
                    .color(visual_tokens::MUTED),
            );
            ui.horizontal_wrapped(|ui| {
                ui.label("Perfil de idioma");
                egui::ComboBox::from_id_salt("kamuse_language_profile")
                    .selected_text(&app.kamuse_language)
                    .show_ui(ui, |ui| {
                        for language in ["pt-BR", "en-US", "ja-JP"] {
                            ui.selectable_value(
                                &mut app.kamuse_language,
                                language.into(),
                                language,
                            );
                        }
                    });
                ui.label("Engine nativa");
                let current_engine = app
                    .kamuse_project
                    .as_ref()
                    .map(|project| project.cantor.engine.clone())
                    .unwrap_or_default();
                let mut selected_engine = current_engine.clone();
                egui::ComboBox::from_id_salt("kamuse_native_engine")
                    .selected_text(if current_engine == "tainah" {
                        "Tainah · WORLD"
                    } else {
                        &current_engine
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(
                            &mut selected_engine,
                            "tainah".into(),
                            "Tainah · WORLD",
                        );
                    });
                if selected_engine != current_engine {
                    app.status_message = match app
                        .kamuse_project
                        .as_mut()
                        .unwrap()
                        .set_engine(&selected_engine)
                    {
                        Ok(()) => "Tainah WORLD selecionado para o banco nativo.".into(),
                        Err(error) => error,
                    };
                }
            });
            let (mut gain_db, mut transition_ms) = app
                .kamuse_project
                .as_ref()
                .map(|p| (p.cantor.ajustes.ganho_db, p.cantor.ajustes.transicao_ms))
                .unwrap_or_default();
            let mut tuning_changed = false;
            ui.horizontal_wrapped(|ui| {
                ui.label("Ganho do banco (dB)");
                tuning_changed |= ui
                    .add(
                        egui::DragValue::new(&mut gain_db)
                            .speed(0.1)
                            .range(-24.0..=24.0),
                    )
                    .changed();
                ui.label("Transição (ms)");
                tuning_changed |= ui
                    .add(
                        egui::DragValue::new(&mut transition_ms)
                            .speed(1.0)
                            .range(0.0..=500.0),
                    )
                    .changed();
            });
            if tuning_changed {
                app.status_message = match app
                    .kamuse_project
                    .as_mut()
                    .unwrap()
                    .set_adjustments(gain_db, transition_ms)
                {
                    Ok(()) => "Ajustes do cantor salvos.".into(),
                    Err(error) => format!("Ajustes não salvos: {error}"),
                };
            }
        });
    ui.add_space(6.0);
    ui.collapsing("Lista de gravação por perfil", |ui| {
        ui.label(RichText::new("Cada prompt é CVC. O KAMUSE deriva as unidades CV e VC das marcações da mesma gravação.").color(visual_tokens::MUTED));
        ui.horizontal(|ui| { ui.label("Filtrar prompts"); ui.text_edit_singleline(&mut app.kamuse_prompt_filter); });
        let prompts = crate::copaiba::kamuse::KamuseProject::recording_prompts(&app.kamuse_language);
        match prompts {
            Ok(prompts) => {
                let filter = app.kamuse_prompt_filter.to_lowercase();
                egui::ScrollArea::vertical().max_height(138.0).show(ui, |ui| {
                    let mut shown = 0;
                    ui.horizontal_wrapped(|ui| {
                        for prompt in prompts.iter().filter(|p| filter.is_empty() || p.label().to_lowercase().contains(&filter)).take(150) {
                            shown += 1;
                            let label = prompt.label();
                            if ui.add(egui::Button::new(RichText::new(label.clone()).monospace().strong()).fill(visual_tokens::SURFACE).rounding(Rounding::same(8.0))).on_hover_text("Usar este prompt no campo de fonemas").clicked() {
                                app.kamuse_recording_phones = format!("{} {} {}", prompt.onset, prompt.vowel, prompt.coda);
                            }
                        }
                        if shown == 0 { ui.label("Nenhum prompt corresponde ao filtro."); }
                    });
                });
                ui.small(format!("{} combinações no perfil; mostrando até 150 resultados filtrados.", prompts.len()));
            }
            Err(error) => { ui.colored_label(Color32::YELLOW, error); }
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.label("Adicionar tom MIDI");
        ui.add(egui::DragValue::new(&mut app.kamuse_new_tone_midi).range(0..=127));
        if ui
            .button("+ Tom")
            .on_hover_text("Cria uma nova faixa de altura MIDI no banco.")
            .clicked()
        {
            let result = app
                .kamuse_project
                .as_mut()
                .ok_or_else(|| "nenhum banco KAMUSE aberto".to_string())
                .and_then(|project| project.add_tone(app.kamuse_new_tone_midi));
            match result {
                Ok(tone_id) => {
                    app.kamuse_selected_tone = Some(tone_id);
                    app.kamuse_recording_midi = app.kamuse_new_tone_midi;
                    app.kamuse_selected_recording = None;
                    app.kamuse_waveform = None;
                    app.kamuse_editing_tone_id = None;
                    app.kamuse_editing_recording = None;
                    app.status_message = "Tom criado. Adicione uma gravação para cobri-lo.".into();
                }
                Err(error) => app.status_message = error,
            }
        }
    });
    ui.add_space(10.0);
    egui::Frame::none()
        .fill(visual_tokens::SURFACE)
        .stroke(Stroke::new(1.0_f32, visual_tokens::BORDER))
        .rounding(Rounding::same(12.0))
        .inner_margin(egui::Margin::same(12.0))
        .show(ui, |ui| {
    ui.heading(RichText::new("Importar gravação").strong().size(17.0));
    ui.horizontal_wrapped(|ui| {
        ui.label("Fonemas");
        ui.add(
            egui::TextEdit::singleline(&mut app.kamuse_recording_phones)
                .hint_text("ex.: k a m"),
        );
        ui.label("Tom MIDI"); ui.add(egui::DragValue::new(&mut app.kamuse_recording_midi).range(0..=127));
        ui.add(egui::Image::from_bytes("bytes://kamuse-import.svg", include_bytes!("../../../assets/kamuse/import.svg")).fit_to_exact_size(egui::vec2(18.0, 18.0)));
        if ui.button("Importar WAV/MP3...").on_hover_text("Escolha o áudio original e informe os fonemas e o tom antes da importação.").clicked() {
            if let Some(path) = crate::dialogs::FileDialog::new().add_filter("Áudio WAV ou MP3", &["wav", "mp3"]).pick_file() {
                app.status_message = match app.kamuse_import_recording(path) {
                    Ok(()) => {
                        app.kamuse_recording_phones.clear();
                        app.kamuse_editing_tone_id = None;
                        app.kamuse_editing_recording = None;
                        "Gravação importada; revise as marcações provisórias no editor.".into()
                    }
                    Err(error) => error,
                };
            }
        }
    });
    ui.label("A origem permanece intacta. As marcações iniciais são provisórias até a revisão no waveform.");
    if app.kamuse_project.is_some() {
        let (has_avatar, has_portrait) = app
            .kamuse_project
            .as_ref()
            .map(|p| (p.cantor.imagem.is_some(), p.cantor.portrait.is_some()))
            .unwrap_or_default();
        ui.horizontal(|ui| {
            if ui.button("Imagem 200×200...").clicked() {
                if let Some(path) = crate::dialogs::FileDialog::new()
                    .add_filter("Imagem PNG/JPEG", &["png", "jpg", "jpeg"])
                    .pick_file()
                {
                    app.status_message = match app
                        .kamuse_project
                        .as_mut()
                        .unwrap()
                        .import_presentation_asset(&path, true)
                    {
                        Ok(()) => "Imagem de apresentação adicionada.".into(),
                        Err(error) => error,
                    };
                }
            }
            if ui.button("Portrait transparente...").clicked() {
                if let Some(path) = crate::dialogs::FileDialog::new()
                    .add_filter("PNG transparente", &["png"])
                    .pick_file()
                {
                    app.status_message = match app
                        .kamuse_project
                        .as_mut()
                        .unwrap()
                        .import_presentation_asset(&path, false)
                    {
                        Ok(()) => "Portrait transparente adicionado.".into(),
                        Err(error) => error,
                    };
                }
            }
            if has_avatar {
                ui.label("Imagem OK");
            }
            if has_portrait {
                ui.label("Portrait OK");
            }
        });
    }
        });
}

fn draw_kamuse_editor(
    app: &mut CopaibaToolkitApp,
    audio_player: &mut AudioPlayer,
    ui: &mut egui::Ui,
) {
    section_title(
        ui,
        "02",
        "Revisar marcações",
        "Ajuste as fronteiras com a forma de onda e confira os tempos abaixo.",
    );
    let mut changed_selection = false;
    if let Some(project) = app.kamuse_project.as_ref() {
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("kamuse_tone_select")
                .selected_text(app.kamuse_selected_tone.as_deref().unwrap_or("Tom"))
                .show_ui(ui, |ui| {
                    for tone in &project.cantor.tons {
                        changed_selection |= ui
                            .selectable_value(
                                &mut app.kamuse_selected_tone,
                                Some(tone.id.clone()),
                                format!("{} · MIDI {}", tone.id, tone.midi),
                            )
                            .changed();
                    }
                });
            if let Some(tone) = app
                .kamuse_selected_tone
                .as_ref()
                .and_then(|id| project.cantor.tons.iter().find(|tone| &tone.id == id))
            {
                egui::ComboBox::from_id_salt("kamuse_recording_select")
                    .selected_text(
                        app.kamuse_selected_recording
                            .as_deref()
                            .unwrap_or("Gravação"),
                    )
                    .show_ui(ui, |ui| {
                        for recording in &tone.gravacoes {
                            changed_selection |= ui
                                .selectable_value(
                                    &mut app.kamuse_selected_recording,
                                    Some(recording.id.clone()),
                                    recording.id.clone(),
                                )
                                .changed();
                        }
                    });
            }
        });
    }
    if changed_selection {
        app.kamuse_drag_mark = None;
        app.kamuse_editing_tone_id = None;
        app.kamuse_editing_recording = None;
        if let Some(midi) = app
            .kamuse_project
            .as_ref()
            .and_then(|project| {
                project
                    .cantor
                    .tons
                    .iter()
                    .find(|tone| Some(tone.id.as_str()) == app.kamuse_selected_tone.as_deref())
            })
            .map(|tone| tone.midi)
        {
            app.kamuse_recording_midi = midi;
        }
        if app.kamuse_marks_dirty {
            if let Some(project) = app.kamuse_project.as_mut() {
                match project.save() {
                    Ok(()) => app.kamuse_marks_dirty = false,
                    Err(error) => {
                        app.status_message =
                            format!("Marcações ainda não salvas no arquivo: {error}");
                    }
                }
            }
        }
    }
    let selected_tone = app.kamuse_project.as_ref().and_then(|project| {
        app.kamuse_selected_tone.as_ref().and_then(|id| {
            project
                .cantor
                .tons
                .iter()
                .find(|tone| &tone.id == id)
                .map(|tone| (tone.id.clone(), tone.midi))
        })
    });
    if let Some((tone_id, tone_midi)) = selected_tone {
        if app.kamuse_editing_tone_id.as_deref() != Some(tone_id.as_str()) {
            app.kamuse_editing_tone_id = Some(tone_id.clone());
            app.kamuse_selected_tone_midi_edit = tone_midi;
        }
        egui::Frame::none()
            .fill(visual_tokens::SURFACE)
            .stroke(Stroke::new(1.0_f32, visual_tokens::BORDER))
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::symmetric(10.0, 7.0))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(format!("Tom {tone_id}")).strong());
                    ui.label("MIDI");
                    ui.add(
                        egui::DragValue::new(&mut app.kamuse_selected_tone_midi_edit)
                            .range(0..=127),
                    );
                    if ui.button("Aplicar tom").clicked() {
                        app.status_message = match app
                            .kamuse_project
                            .as_mut()
                            .ok_or_else(|| "nenhum banco KAMUSE aberto".to_string())
                            .and_then(|project| {
                                project.set_tone_midi(&tone_id, app.kamuse_selected_tone_midi_edit)
                            }) {
                            Ok(()) => "Tom MIDI atualizado.".into(),
                            Err(error) => error,
                        };
                    }
                    if ui.button("Excluir tom…").clicked() {
                        app.kamuse_pending_delete_tone = Some(tone_id.clone());
                    }
                });
            });
    }
    let selection = app.kamuse_project.as_ref().and_then(|project| {
        app.kamuse_selected_tone
            .as_ref()
            .and_then(|id| project.cantor.tons.iter().find(|tone| &tone.id == id))
            .and_then(|tone| {
                app.kamuse_selected_recording
                    .as_ref()
                    .and_then(|id| tone.gravacoes.iter().find(|r| &r.id == id))
            })
            .cloned()
            .map(|recording| (project.root.clone(), recording))
    });
    if let Some((root, recording)) = selection {
        let tone_id = app.kamuse_selected_tone.clone().unwrap_or_default();
        let recording_key = (tone_id.clone(), recording.id.clone());
        if app.kamuse_editing_recording.as_ref() != Some(&recording_key) {
            app.kamuse_editing_recording = Some(recording_key);
            app.kamuse_recording_alias_edit = recording.id.clone();
            app.kamuse_recording_phones_edit = recording.fonemas.join(" ");
        }
        let waveform_path = root.join(&recording.arquivo);
        if app
            .kamuse_waveform
            .as_ref()
            .is_none_or(|(path, _, _)| path != &waveform_path)
        {
            app.kamuse_waveform = match crate::audio::load_audio_file(&waveform_path) {
                Ok(audio) => {
                    let duration_ms = audio.duration_ms;
                    let channels = usize::from(audio.channels.max(1));
                    let mono = audio
                        .samples
                        .chunks_exact(channels)
                        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
                        .collect();
                    Some((waveform_path.clone(), mono, duration_ms))
                }
                Err(error) => {
                    app.status_message = format!("Falha ao decodificar waveform: {error}");
                    None
                }
            };
        }
        let fallback_duration_ms = recording.marcacoes_ms.last().copied().unwrap_or(0.0);
        let waveform_duration_ms = app
            .kamuse_waveform
            .as_ref()
            .map(|(_, _, duration_ms)| *duration_ms)
            .unwrap_or(fallback_duration_ms);
        ui.label(format!(
            "Fonte original: {} · {} · {} fonemas · {:.0} ms",
            recording.arquivo,
            recording.id,
            recording.fonemas.len(),
            waveform_duration_ms
        ));
        ui.colored_label(
            egui::Color32::from_rgb(242, 194, 117),
            "Revise as marcações provisórias sobre o áudio antes de usar o banco.",
        );
        ui.horizontal(|ui| {
            ui.add(
                egui::Image::from_bytes(
                    "bytes://kamuse-mark.svg",
                    include_bytes!("../../../assets/kamuse/mark.svg"),
                )
                .fit_to_exact_size(egui::vec2(18.0, 18.0)),
            );
            ui.label(
                "Marcas sobre a waveform: arraste as linhas para ajustar os limites dos fonemas.",
            );
        });
        egui::Frame::none()
            .fill(visual_tokens::SURFACE)
            .stroke(Stroke::new(1.0_f32, visual_tokens::BORDER))
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::same(10.0))
            .show(ui, |ui| {
                ui.label(RichText::new("TAKE CVC E UNIDADES DERIVADAS").strong().size(10.0).color(visual_tokens::MUTED));
                ui.horizontal_wrapped(|ui| {
                    ui.label("ID do take");
                    ui.add(egui::TextEdit::singleline(&mut app.kamuse_recording_alias_edit).desired_width(150.0));
                    if ui.button("Salvar ID").clicked() {
                        let new_id = app.kamuse_recording_alias_edit.trim().to_owned();
                        app.status_message = match app
                            .kamuse_project
                            .as_mut()
                            .ok_or_else(|| "nenhum banco KAMUSE aberto".to_string())
                            .and_then(|project| project.rename_recording(&tone_id, &recording.id, &new_id))
                        {
                            Ok(()) => {
                                app.kamuse_selected_recording = Some(new_id);
                                app.kamuse_editing_recording = None;
                                "ID do take atualizado.".into()
                            }
                            Err(error) => error,
                        };
                    }
                });
                ui.label(RichText::new("O ID nomeia este take; as unidades CV/VC são geradas a partir da sequência de fonemas.").size(10.0).color(visual_tokens::MUTED));
                ui.horizontal_wrapped(|ui| {
                    ui.label("Fonemas");
                    ui.add(egui::TextEdit::singleline(&mut app.kamuse_recording_phones_edit).desired_width(200.0));
                    if ui.button("Atualizar CV/VC").clicked() {
                        let phones: Vec<String> = app.kamuse_recording_phones_edit
                            .split_whitespace()
                            .map(str::to_owned)
                            .collect();
                        app.status_message = match app
                            .kamuse_project
                            .as_mut()
                            .ok_or_else(|| "nenhum banco KAMUSE aberto".to_string())
                            .and_then(|project| project.set_recording_phones(&tone_id, &recording.id, &phones))
                        {
                            Ok(()) => {
                                app.kamuse_editing_recording = None;
                                "Fonemas atualizados; os aliases CV/VC foram recalculados.".into()
                            }
                            Err(error) => error,
                        };
                    }
                    if ui.button("Duplicar take").clicked() {
                        match app
                            .kamuse_project
                            .as_mut()
                            .ok_or_else(|| "nenhum banco KAMUSE aberto".to_string())
                            .and_then(|project| project.duplicate_recording(&tone_id, &recording.id))
                        {
                            Ok(new_id) => {
                                app.kamuse_selected_recording = Some(new_id);
                                app.kamuse_editing_recording = None;
                                app.status_message = "Take duplicado; o WAV de origem continua compartilhado e intacto.".into();
                            }
                            Err(error) => app.status_message = error,
                        }
                    }
                    if ui.button("Excluir take…").clicked() {
                        app.kamuse_pending_delete_recording = Some((tone_id.clone(), recording.id.clone()));
                    }
                });
            });
        ui.label(format!(
            "Marcação 0 = início · {} = fim · cada marca intermediária encerra o fonema anterior.",
            recording.marcacoes_ms.len().saturating_sub(1)
        ));
        ui.horizontal_wrapped(|ui| {
            for (index, phone) in recording.fonemas.iter().enumerate() {
                ui.monospace(format!("{}: {}", index + 1, phone));
                if index + 1 < recording.fonemas.len() {
                    ui.label("·");
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label("Unidades derivadas da mesma gravação:");
            for (index, pair) in recording.fonemas.windows(2).enumerate() {
                let is_cv = !is_vowel_phone(&pair[0]) && is_vowel_phone(&pair[1]);
                let is_vc = is_vowel_phone(&pair[0]) && !is_vowel_phone(&pair[1]);
                if is_cv || is_vc {
                    ui.strong(format!(
                        "{} · {}{}",
                        if is_cv { "CV" } else { "VC" },
                        pair[0],
                        pair[1]
                    ));
                    ui.label(format!(
                        "{:.0}–{:.0} ms",
                        recording.marcacoes_ms[index],
                        recording.marcacoes_ms[index + 2]
                    ));
                    ui.label("·");
                }
            }
            ui.label(format!("fonte: {}", recording.arquivo));
        });
        ui.add(
            egui::Image::from_bytes(
                "bytes://kamuse-cvc-units.svg",
                include_bytes!("../../../assets/kamuse/cvc-units.svg"),
            )
            .fit_to_exact_size(egui::vec2(ui.available_width().min(640.0), 92.0)),
        );
        ui.horizontal(|ui| {
            ui.add(egui::Image::from_bytes("bytes://kamuse-play.svg", include_bytes!("../../../assets/kamuse/play.svg")).fit_to_exact_size(egui::vec2(18.0, 18.0)));
            if ui
                .button(format!("Ouvir {}", recording.arquivo))
                .on_hover_text("Reproduz o áudio CVC original deste take.")
                .clicked()
            {
                match crate::audio::load_audio_file(root.join(&recording.arquivo)) {
                    Ok(audio) => audio_player.play_samples_with_channels(
                        audio.samples,
                        audio.sample_rate,
                        audio.channels,
                    ),
                    Err(error) => {
                        app.status_message = format!("Falha ao reproduzir gravação: {error}")
                    }
                }
            }
            if ui.button("Prévia Tainah WORLD").on_hover_text("Gera uma frase de prévia usando as unidades CV deste take.").clicked() {
                let result = (|| -> Result<(crate::voicebank::render::RenderedPhrase, Vec<String>), String> {
                    let bank = crate::voicebank::cantor::CantorBank::load_path(&root)?;
                    let midi = bank.cantor.tons.iter().find(|tone| Some(tone.id.as_str()) == app.kamuse_selected_tone.as_deref()).map(|tone| tone.midi).ok_or("tom selecionado não encontrado")?;
                    let mut notes = Vec::new();
                    for pair in recording.fonemas.windows(2) {
                        if !is_vowel_phone(&pair[0]) && is_vowel_phone(&pair[1]) {
                            notes.push(crate::voicebank::cantor::NotaCantor { fonemas_cv:[pair[0].clone(),pair[1].clone()], inicio_ms:notes.len() as f64 * 250.0, duracao_ms:250.0, midi });
                        }
                    }
                    if notes.is_empty() { return Err("a gravação ainda não contém uma sequência CV para a prévia".into()); }
                    let planned = bank.plan_notes(&notes)?;
                    let warnings = planned.avisos.clone();
                    let rendered = crate::voicebank::render::render_phrase_world(&bank.native_snapshot()?, &planned.plano, 44_100).map_err(|e| format!("Tainah WORLD falhou: {e}"))?;
                    Ok((rendered, warnings))
                })();
                match result {
                    Ok((rendered, warnings)) => {
                        audio_player.play_samples(rendered.samples, rendered.sample_rate_hz);
                        app.status_message = if warnings.is_empty() { "Prévia gerada por Tainah WORLD.".into() } else { format!("Prévia Tainah WORLD · {}", warnings.join(" ")) };
                    }
                    Err(error) => app.status_message = error,
                }
            }
            ui.label("Edite cada unidade na tabela; as alterações são salvas automaticamente.");
        });
        // The selected project's manifest is the single source of truth.
        // Live waveform drags update it in memory; the file is written when
        // the gesture ends.
        let mut marks = recording.marcacoes_ms.clone();
        let mut table_changed = false;
        ui.label("Unidades e recortes · os três tempos de cada par definem início, centro e fim.");
        egui::Grid::new("kamuse_unit_marks_table")
            .striped(true)
            .num_columns(4)
            .show(ui, |ui| {
                ui.strong("Unidade");
                ui.strong("Início (ms)");
                ui.strong("Centro (ms)");
                ui.strong("Fim (ms)");
                ui.end_row();
                for (index, pair) in recording.fonemas.windows(2).enumerate() {
                    let is_cv = !is_vowel_phone(&pair[0]) && is_vowel_phone(&pair[1]);
                    let is_vc = is_vowel_phone(&pair[0]) && !is_vowel_phone(&pair[1]);
                    if !is_cv && !is_vc {
                        continue;
                    }
                    ui.strong(format!(
                        "{} · {}{}",
                        if is_cv { "CV" } else { "VC" },
                        pair[0],
                        pair[1]
                    ));
                    for mark_index in index..=index + 2 {
                        table_changed |=
                            draw_mark_value(ui, &mut marks, mark_index, waveform_duration_ms);
                    }
                    ui.end_row();
                }
            });
        let mut waveform_edit_finished = false;
        if let Some((_, samples, duration_ms)) = app.kamuse_waveform.as_ref() {
            waveform_edit_finished = draw_kamuse_waveform(
                ui,
                samples,
                *duration_ms,
                &mut marks,
                &recording.fonemas,
                &mut app.kamuse_drag_mark,
            );
        }
        let marks_changed = marks != recording.marcacoes_ms;
        let should_save = table_changed || waveform_edit_finished;
        if marks_changed {
            let update_result = app
                .kamuse_project
                .as_mut()
                .ok_or_else(|| "nenhum banco KAMUSE aberto".to_string())
                .and_then(|project| {
                    project.update_marks_in_memory(
                        app.kamuse_selected_tone
                            .as_deref()
                            .ok_or_else(|| "nenhum tom selecionado".to_string())?,
                        app.kamuse_selected_recording
                            .as_deref()
                            .ok_or_else(|| "nenhuma gravação selecionada".to_string())?,
                        marks,
                    )
                });
            if let Err(error) = update_result {
                app.status_message = format!("Não foi possível atualizar as marcações: {error}");
            } else {
                app.kamuse_marks_dirty = true;
            }
        }
        if should_save && app.kamuse_marks_dirty {
            let save_result = app
                .kamuse_project
                .as_mut()
                .ok_or_else(|| "nenhum banco KAMUSE aberto".to_string())
                .and_then(|project| project.save());
            match save_result {
                Ok(()) => {
                    app.kamuse_marks_dirty = false;
                    app.status_message = "Marcações salvas automaticamente em cantor.toml.".into()
                }
                Err(error) => {
                    app.status_message = format!(
                        "Marcações atualizadas nesta sessão, mas não salvas no arquivo: {error}"
                    );
                }
            }
        }
        egui::CollapsingHeader::new("Biblioteca de gravações")
            .default_open(false)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .max_height(220.0)
                    .show(ui, |ui| {
                        egui::Grid::new("kamuse_recording_list")
                            .striped(true)
                            .show(ui, |ui| {
                                for tone in &app.kamuse_project.as_ref().unwrap().cantor.tons {
                                    ui.strong(format!("{} · MIDI {}", tone.id, tone.midi));
                                    ui.end_row();
                                    for take in &tone.gravacoes {
                                        ui.label(format!(
                                            "{} · {}",
                                            take.id,
                                            take.fonemas.join(" ")
                                        ));
                                        ui.label(format!("{} marcações", take.marcacoes_ms.len()));
                                        ui.end_row();
                                    }
                                }
                            });
                    });
            });
    } else {
        ui.horizontal(|ui| {
            ui.add(
                egui::Image::from_bytes(
                    "bytes://kamuse-empty.svg",
                    include_bytes!("../../../assets/kamuse/empty.svg"),
                )
                .fit_to_exact_size(egui::vec2(112.0, 112.0)),
            );
            ui.vertical(|ui| {
                ui.heading("Nenhum take selecionado");
                ui.label("Escolha uma gravação na biblioteca ou importe um áudio em Preparação.");
            });
        });
    }
    draw_kamuse_delete_confirmations(app, ui);
    if !app.status_message.is_empty() {
        ui.colored_label(Color32::LIGHT_BLUE, &app.status_message);
    }
}

fn draw_kamuse_delete_confirmations(app: &mut CopaibaToolkitApp, ui: &mut egui::Ui) {
    if let Some(tone_id) = app.kamuse_pending_delete_tone.clone() {
        let mut confirm = false;
        let mut cancel = false;
        egui::Window::new("Excluir tom")
            .id(egui::Id::new("kamuse_delete_tone_confirm"))
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.label(format!("Remover o tom {tone_id} do cantor?"));
                ui.label("Os arquivos de áudio permanecem na pasta do banco.");
                ui.horizontal(|ui| {
                    confirm = ui.button("Remover tom").clicked();
                    cancel = ui.button("Cancelar").clicked();
                });
            });
        if cancel {
            app.kamuse_pending_delete_tone = None;
        } else if confirm {
            app.status_message = match app
                .kamuse_project
                .as_mut()
                .ok_or_else(|| "nenhum banco KAMUSE aberto".to_string())
                .and_then(|project| project.delete_tone(&tone_id))
            {
                Ok(()) => {
                    app.kamuse_selected_tone = app
                        .kamuse_project
                        .as_ref()
                        .and_then(|project| project.cantor.tons.first())
                        .map(|tone| tone.id.clone());
                    if let Some(midi) = app
                        .kamuse_project
                        .as_ref()
                        .and_then(|project| project.cantor.tons.first())
                        .map(|tone| tone.midi)
                    {
                        app.kamuse_recording_midi = midi;
                    }
                    app.kamuse_selected_recording = app
                        .kamuse_project
                        .as_ref()
                        .and_then(|project| project.cantor.tons.first())
                        .and_then(|tone| tone.gravacoes.first())
                        .map(|recording| recording.id.clone());
                    app.kamuse_waveform = None;
                    app.kamuse_editing_tone_id = None;
                    app.kamuse_editing_recording = None;
                    "Tom removido do banco; os WAVs foram preservados.".into()
                }
                Err(error) => error,
            };
            app.kamuse_pending_delete_tone = None;
        }
    }

    if let Some((tone_id, recording_id)) = app.kamuse_pending_delete_recording.clone() {
        let mut confirm = false;
        let mut cancel = false;
        egui::Window::new("Excluir gravação")
            .id(egui::Id::new("kamuse_delete_recording_confirm"))
            .collapsible(false)
            .resizable(false)
            .show(ui.ctx(), |ui| {
                ui.label(format!("Remover o take {recording_id} do banco?"));
                ui.label("O WAV de origem não será apagado.");
                ui.horizontal(|ui| {
                    confirm = ui.button("Remover take").clicked();
                    cancel = ui.button("Cancelar").clicked();
                });
            });
        if cancel {
            app.kamuse_pending_delete_recording = None;
        } else if confirm {
            app.status_message = match app
                .kamuse_project
                .as_mut()
                .ok_or_else(|| "nenhum banco KAMUSE aberto".to_string())
                .and_then(|project| project.delete_recording(&tone_id, &recording_id))
            {
                Ok(()) => {
                    app.kamuse_selected_recording = app
                        .kamuse_project
                        .as_ref()
                        .and_then(|project| {
                            project.cantor.tons.iter().find(|tone| tone.id == tone_id)
                        })
                        .and_then(|tone| tone.gravacoes.first())
                        .map(|recording| recording.id.clone());
                    app.kamuse_waveform = None;
                    app.kamuse_editing_recording = None;
                    "Take removido do banco; o WAV foi preservado.".into()
                }
                Err(error) => error,
            };
            app.kamuse_pending_delete_recording = None;
        }
    }
}

fn section_title(ui: &mut egui::Ui, number: &str, title: &str, description: &str) {
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        egui::Frame::none()
            .fill(visual_tokens::ACCENT_DARK)
            .rounding(Rounding::same(7.0))
            .inner_margin(egui::Margin::symmetric(8.0, 4.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(number)
                        .monospace()
                        .strong()
                        .color(visual_tokens::ACCENT),
                );
            });
        ui.vertical(|ui| {
            ui.label(RichText::new(title).strong().size(16.0));
            ui.label(
                RichText::new(description)
                    .size(11.0)
                    .color(visual_tokens::MUTED),
            );
        });
    });
    ui.add_space(6.0);
}

fn metric_chip(ui: &mut egui::Ui, label: &str, value: String) {
    metric_chip_color(ui, label, &value, visual_tokens::ACCENT);
}

fn metric_chip_color(ui: &mut egui::Ui, label: &str, value: &str, color: Color32) {
    egui::Frame::none()
        .fill(visual_tokens::SURFACE)
        .stroke(Stroke::new(1.0_f32, visual_tokens::BORDER))
        .rounding(Rounding::same(8.0))
        .inner_margin(egui::Margin::symmetric(10.0, 6.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(label)
                        .size(9.0)
                        .strong()
                        .color(visual_tokens::MUTED),
                );
                ui.label(RichText::new(value).size(11.0).strong().color(color));
            });
        });
}

fn draw_kamuse_waveform(
    ui: &mut egui::Ui,
    samples: &[f32],
    duration_ms: f64,
    marks: &mut [f64],
    phones: &[String],
    drag_mark: &mut Option<usize>,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 190.0),
        egui::Sense::click_and_drag(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, Rounding::same(10.0), visual_tokens::BACKGROUND);
    let handle_inset = 14.0_f32.min(rect.width() * 0.1);
    let usable_left = rect.left() + handle_inset;
    let usable_right = rect.right() - handle_inset;
    let usable_width = (usable_right - usable_left).max(1.0);
    let time_to_x = |time_ms: f64| {
        let fraction = if duration_ms > 0.0 {
            (time_ms / duration_ms).clamp(0.0, 1.0) as f32
        } else {
            0.0
        };
        usable_left + fraction * usable_width
    };
    for index in 0..marks.len().saturating_sub(1) {
        let left = time_to_x(marks[index]);
        let right = time_to_x(marks[index + 1]);
        let color = if index % 3 == 0 {
            visual_tokens::ACCENT
        } else if index % 3 == 1 {
            Color32::from_rgb(129, 183, 226)
        } else {
            Color32::from_rgb(216, 164, 116)
        };
        painter.rect_filled(
            Rect::from_min_max(
                egui::pos2(left, rect.top()),
                egui::pos2(right, rect.bottom()),
            ),
            0.0,
            Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 16),
        );
        if let Some(phone) = phones.get(index) {
            if right - left > 34.0 {
                painter.text(
                    egui::pos2((left + right) * 0.5, rect.top() + 10.0),
                    egui::Align2::CENTER_TOP,
                    phone,
                    egui::FontId::proportional(10.0),
                    color,
                );
            }
        }
    }
    // Instant hover feedback respects the application's reduce-motion setting.
    let hover = if response.hovered() { 1.0 } else { 0.0 };
    painter.rect_stroke(
        rect,
        Rounding::same(10.0),
        Stroke::new(
            1.0 + hover * 0.5,
            visual_tokens::BORDER.lerp_to_gamma(visual_tokens::ACCENT, hover),
        ),
    );
    for tick in 0..=10 {
        let x = usable_left + usable_width * tick as f32 / 10.0;
        painter.line_segment(
            [
                egui::pos2(x, rect.bottom() - 14.0),
                egui::pos2(x, rect.bottom() - 8.0),
            ],
            Stroke::new(1.0_f32, visual_tokens::MUTED),
        );
    }
    let middle = rect.center().y;
    painter.line_segment(
        [
            egui::pos2(rect.left(), middle),
            egui::pos2(rect.right(), middle),
        ],
        egui::Stroke::new(1.0_f32, ui.visuals().weak_text_color()),
    );
    if !samples.is_empty() {
        let peak = samples
            .iter()
            .fold(0.02f32, |peak, sample| peak.max(sample.abs()));
        let width = rect.width().max(1.0);
        for x in 0..width as usize {
            let start = x * samples.len() / width as usize;
            let end =
                (((x + 1) * samples.len() / width as usize).max(start + 1)).min(samples.len());
            let low = samples[start..end].iter().copied().fold(0.0f32, f32::min);
            let high = samples[start..end].iter().copied().fold(0.0f32, f32::max);
            let xp = rect.left() + x as f32;
            painter.line_segment(
                [
                    egui::pos2(xp, middle - high / peak * rect.height() * 0.42),
                    egui::pos2(xp, middle - low / peak * rect.height() * 0.42),
                ],
                egui::Stroke::new(1.0_f32, ui.visuals().hyperlink_color),
            );
        }
    }
    for (index, mark) in marks.iter().enumerate() {
        let x = time_to_x(*mark);
        let color = if *drag_mark == Some(index) {
            egui::Color32::YELLOW
        } else {
            ui.visuals().selection.stroke.color
        };
        painter.line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(
                if index == 0 || index + 1 == marks.len() {
                    3.0_f32
                } else {
                    2.0_f32
                },
                color,
            ),
        );
        if index == 0 || index + 1 == marks.len() {
            painter.circle_filled(egui::pos2(x, rect.bottom() - 7.0), 5.0, color);
            painter.text(
                egui::pos2(x, rect.top() + 4.0),
                egui::Align2::CENTER_TOP,
                if index == 0 { "INÍCIO" } else { "FIM" },
                egui::FontId::proportional(9.0),
                color,
            );
        } else {
            painter.text(
                egui::pos2(x + 3.0, rect.top() + 4.0),
                egui::Align2::LEFT_TOP,
                format!("{mark:.0}"),
                egui::FontId::proportional(9.0),
                color,
            );
        }
    }
    if response.drag_started() {
        if let Some(pointer) = response.interact_pointer_pos() {
            *drag_mark = marks
                .iter()
                .enumerate()
                .min_by(|(_, a), (_, b)| {
                    let xa = time_to_x(**a);
                    let xb = time_to_x(**b);
                    (xa - pointer.x).abs().total_cmp(&(xb - pointer.x).abs())
                })
                .and_then(|(index, mark)| {
                    let x = time_to_x(*mark);
                    ((x - pointer.x).abs() <= 16.0).then_some(index)
                });
        }
    }
    if response.dragged() {
        if let (Some(index), Some(pointer)) = (*drag_mark, response.interact_pointer_pos()) {
            let fraction = ((pointer.x - usable_left) / usable_width).clamp(0.0, 1.0) as f64;
            let lower = if index == 0 {
                0.0
            } else {
                marks[index - 1] + 0.1
            };
            let upper = if index + 1 == marks.len() {
                duration_ms
            } else {
                marks[index + 1] - 0.1
            };
            if lower < upper {
                marks[index] = (fraction * duration_ms).clamp(lower, upper);
            }
        }
    }
    let drag_stopped = response.drag_stopped();
    if drag_stopped {
        *drag_mark = None;
    }
    response.on_hover_text(
        "Arraste as linhas verticais, inclusive INÍCIO e FIM, para ajustar os limites fonéticos.",
    );
    drag_stopped
}

fn draw_mark_value(ui: &mut egui::Ui, marks: &mut [f64], index: usize, duration_ms: f64) -> bool {
    let minimum = if index == 0 {
        0.0
    } else {
        marks[index - 1] + 0.1
    };
    let maximum = if index + 1 == marks.len() {
        duration_ms
    } else {
        marks[index + 1] - 0.1
    };
    let response = ui.add(
        egui::DragValue::new(&mut marks[index])
            .speed(1.0)
            .range(minimum..=maximum.max(minimum)),
    );
    response.changed()
}

fn is_vowel_phone(phone: &str) -> bool {
    matches!(
        phone.to_lowercase().as_str(),
        "a" | "e"
            | "eh"
            | "i"
            | "o"
            | "oh"
            | "u"
            | "an"
            | "en"
            | "in"
            | "on"
            | "un"
            | "ae"
            | "ih"
            | "iy"
            | "aa"
            | "ao"
            | "uh"
            | "uw"
            | "ah"
            | "er"
            | "ay"
            | "aw"
            | "oy"
            | "ow"
    )
}
