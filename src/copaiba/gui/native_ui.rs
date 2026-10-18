use super::*;

type NativeLayerLists = ((), Vec<(String, String)>, Vec<(String, String)>);

pub(super) fn draw_copaiba_native_ui(
    app: &mut CopaibaToolkitApp,
    audio_player: &mut AudioPlayer,
    ui: &mut egui::Ui,
) {
    let layers = app
        .native_project
        .as_ref()
        .map(|project| {
            project
                .layers
                .iter()
                .map(|(layer, _)| {
                    (
                        layer.id.clone(),
                        layer.style_id.clone(),
                        layer.recorded_midi,
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if layers.is_empty() {
        ui.colored_label(Color32::LIGHT_RED, "KFV 2 não possui camadas para editar.");
        return;
    }
    let mut selected_layer = app
        .native_selected_layer
        .clone()
        .unwrap_or_else(|| layers[0].0.clone());
    if let Some(project) = app.native_project.as_ref() {
        match project.layer_coverage(&selected_layer) {
            Ok(coverage) => {
                let color = if coverage.missing_symbols.is_empty() {
                    visual_tokens::SUCCESS
                } else {
                    Color32::from_rgb(255, 190, 80)
                };
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(
                        color,
                        format!(
                            "Cobertura de símbolos · {}/{} · {} unidades · {}/{} revisadas · {} gravações",
                            coverage.covered_symbol_count,
                            coverage.symbol_count,
                            coverage.unit_count,
                            coverage.reviewed_unit_count,
                            coverage.unit_count,
                            coverage.asset_count
                        ),
                    );
                    if !coverage.missing_symbols.is_empty() {
                        ui.label(format!(
                            "Sem unidade: {}",
                            coverage.missing_symbols.join(", ")
                        ));
                    }
                });
                ui.small("A cobertura de símbolos é apenas inventário. O teste de frase abaixo verifica seleção estrutural, não adequação acústica.");
            }
            Err(error) => {
                ui.colored_label(
                    Color32::LIGHT_RED,
                    format!("Falha ao calcular cobertura: {error}"),
                );
            }
        }
    }
    if let Some(report) = app.migration_report.as_ref() {
        ui.collapsing(
            format!(
                "Relatório da migração · {} convertidas · {} ignoradas",
                report.migrated_units, report.skipped_entries
            ),
            |ui| {
                ui.small(format!(
                    "Origem: {} · {} gravações copiadas · SHA-256 {}",
                    report.source_name, report.copied_assets, report.content_sha256
                ));
                egui::ScrollArea::vertical()
                    .max_height(190.0)
                    .show(ui, |ui| {
                        for entry in &report.entries {
                            let (status, color) = match &entry.status {
                                crate::voicebank::migration::MigrationEntryStatus::Migrated => {
                                    ("convertida", visual_tokens::SUCCESS)
                                }
                                crate::voicebank::migration::MigrationEntryStatus::Skipped => {
                                    ("ignorada", Color32::from_rgb(255, 190, 80))
                                }
                            };
                            let mapping = match &entry.mapping_origin {
                                crate::voicebank::migration::MappingOrigin::Explicit => {
                                    "mapa explícito"
                                }
                                crate::voicebank::migration::MappingOrigin::ExactAliasTokens => {
                                    "inferência por tokens exatos"
                                }
                                crate::voicebank::migration::MappingOrigin::Unmapped => {
                                    "sem mapeamento"
                                }
                            };
                            ui.separator();
                            ui.colored_label(
                                color,
                                format!(
                                    "{} · {} · {} → {}",
                                    status,
                                    entry.alias,
                                    mapping,
                                    if entry.phones.is_empty() {
                                        "(sem fonemas)".to_string()
                                    } else {
                                        entry.phones.join(" ")
                                    }
                                ),
                            );
                            ui.small(format!("WAV: {}", entry.wav_filename));
                            for warning in &entry.warnings {
                                ui.small(warning);
                            }
                        }
                    });
                for warning in &report.warnings {
                    ui.colored_label(Color32::YELLOW, warning);
                }
            },
        );
    }
    ui.horizontal(|ui| {
        ui.label("Teste de frase (fonemas):");
        let changed = ui
            .add(
                egui::TextEdit::singleline(&mut app.native_test_phones)
                    .desired_width(180.0)
                    .hint_text("ex.: k a s a"),
            )
            .changed();
        if changed {
            app.native_phrase_report = None;
        }
        if ui.button("Verificar cobertura").clicked() {
            app.native_selected_layer = Some(selected_layer.clone());
            let result = app
                .native_project
                .as_ref()
                .ok_or_else(|| "Nenhum banco KFV 2 aberto".to_string())
                .and_then(|project| {
                    project.plan_direct_phone_phrase(&selected_layer, &app.native_test_phones)
                });
            app.native_phrase_report = Some(match result {
                Ok(plan) => {
                    let choices = plan
                        .units
                        .iter()
                        .map(|unit| {
                            format!(
                                "{} {}→{}",
                                unit.unit_id,
                                unit.phones.first().map(String::as_str).unwrap_or("?"),
                                unit.phones.last().map(String::as_str).unwrap_or("?")
                            )
                        })
                        .collect::<Vec<_>>()
                        .join(" · ");
                    format!(
                        "Coberta: {}/{} eventos · {choices}",
                        plan.units.len(),
                        plan.units.len()
                    )
                }
                Err(error) => format!("Sem cobertura completa: {error}"),
            });
        }
        if ui.button("▶ Frase · SOLA").clicked() {
            app.native_selected_layer = Some(selected_layer.clone());
            if let Err(error) = app.preview_native_phrase_backend(
                audio_player,
                crate::voicebank::render::PreviewBackend::Sola,
            ) {
                app.native_phrase_report = Some(format!("Preview indisponível: {error}"));
            }
        }
        if ui.button("▶ Frase · WORLD (experimental)").clicked() {
            app.native_selected_layer = Some(selected_layer.clone());
            if let Err(error) = app.preview_native_phrase_backend(
                audio_player,
                crate::voicebank::render::PreviewBackend::World,
            ) {
                app.native_phrase_report = Some(format!("Preview indisponível: {error}"));
            }
        }
    });
    if let Some(report) = &app.native_phrase_report {
        ui.small(report);
        ui.small("Cobertura significa que o planner encontrou uma sequência de unidades. Não estima qualidade acústica ou naturalidade.");
    }
    let layer_changed = egui::ComboBox::from_id_salt("copaiba_kfv2_layer")
        .selected_text(format!(
            "{} · {} · MIDI {}",
            selected_layer,
            layers
                .iter()
                .find(|layer| layer.0 == selected_layer)
                .map(|layer| layer.1.as_str())
                .unwrap_or("style"),
            layers
                .iter()
                .find(|layer| layer.0 == selected_layer)
                .map(|layer| layer.2)
                .unwrap_or(0)
        ))
        .show_ui(ui, |ui| {
            for (id, style, midi) in &layers {
                ui.selectable_value(
                    &mut selected_layer,
                    id.clone(),
                    format!("{id} · {style} · MIDI {midi}"),
                );
            }
        })
        .response
        .changed();
    if layer_changed {
        app.native_selected_layer = Some(selected_layer.clone());
        app.native_phrase_report = None;
        let (_, units, assets) = native_layer_lists(app, &selected_layer);
        app.native_selected_unit = units.first().map(|unit| unit.0.clone());
        app.native_selected_asset = assets.first().map(|asset| asset.0.clone());
        app.native_compare_asset = assets
            .iter()
            .find(|asset| Some(asset.0.as_str()) != app.native_selected_asset.as_deref())
            .map(|asset| asset.0.clone());
        app.native_comparison_a = None;
        app.native_comparison_b = None;
        app.load_selected_wav_samples();
    }

    let (_, units, assets) = native_layer_lists(app, &selected_layer);
    if app.native_selected_asset.is_none() {
        app.native_selected_asset = assets.first().map(|asset| asset.0.clone());
    }
    let asset_a_id = app.native_selected_asset.clone();
    if app.native_compare_asset.as_ref().is_none_or(|asset_b| {
        Some(asset_b.as_str()) == asset_a_id.as_deref()
            || !assets.iter().any(|asset| asset.0 == *asset_b)
    }) {
        app.native_compare_asset = assets
            .iter()
            .find(|asset| Some(asset.0.as_str()) != asset_a_id.as_deref())
            .map(|asset| asset.0.clone());
        app.native_comparison_a = None;
        app.native_comparison_b = None;
    }
    if app.native_selected_asset.is_some()
        && (app.native_comparison_a.is_none()
            || (app.native_compare_asset.is_some() && app.native_comparison_b.is_none()))
    {
        if let Err(error) = app.load_native_comparison_waveforms() {
            app.status_message = format!("Falha ao carregar comparação A/B: {error}");
        }
    }
    ui.horizontal(|ui| {
        ui.label("Unidade:");
        let mut unit_id = app.native_selected_unit.clone().unwrap_or_default();
        let unit_changed = egui::ComboBox::from_id_salt("copaiba_kfv2_unit")
            .selected_text(if unit_id.is_empty() {
                "Selecionar unidade".into()
            } else {
                unit_id.clone()
            })
            .show_ui(ui, |ui| {
                for (id, label) in &units {
                    ui.selectable_value(&mut unit_id, id.clone(), label);
                }
            })
            .response
            .changed();
        if unit_changed {
            app.native_selected_unit = Some(unit_id);
            app.load_selected_wav_samples();
        }
        ui.label("Gravação:");
        let mut asset_id = app.native_selected_asset.clone().unwrap_or_default();
        let asset_changed = egui::ComboBox::from_id_salt("copaiba_kfv2_asset")
            .selected_text(if asset_id.is_empty() {
                "Selecionar asset".into()
            } else {
                asset_id.clone()
            })
            .show_ui(ui, |ui| {
                for (id, path) in &assets {
                    ui.selectable_value(&mut asset_id, id.clone(), path);
                }
            })
            .response
            .changed();
        if asset_changed {
            app.native_selected_asset = Some(asset_id);
            let selected_asset = app.native_selected_asset.as_deref();
            if app.native_compare_asset.as_deref() == selected_asset {
                app.native_compare_asset = assets
                    .iter()
                    .find(|asset| Some(asset.0.as_str()) != selected_asset)
                    .map(|asset| asset.0.clone());
            }
            app.native_comparison_a = None;
            app.native_comparison_b = None;
            if let Err(error) = app.load_native_comparison_waveforms() {
                app.status_message = format!("Falha ao carregar comparação A/B: {error}");
            }
        }
        if ui.button("▶ Preview · SOLA").clicked() {
            if let Err(error) = app.preview_native_unit_backend(
                audio_player,
                crate::voicebank::render::PreviewBackend::Sola,
            ) {
                app.status_message = error;
            }
        }
        if ui.button("▶ Preview · WORLD (experimental)").clicked() {
            if let Err(error) = app.preview_native_unit_backend(
                audio_player,
                crate::voicebank::render::PreviewBackend::World,
            ) {
                app.status_message = error;
            }
        }
    });

    let mut compare_asset = app.native_compare_asset.clone().unwrap_or_default();
    let mut play_a = false;
    let mut play_b = false;
    ui.separator();
    ui.horizontal(|ui| {
        ui.label("Comparar tomadas:");
        egui::ComboBox::from_id_salt("copaiba_kfv2_compare_asset")
            .selected_text(if compare_asset.is_empty() {
                "Importe outra gravação".into()
            } else {
                compare_asset.clone()
            })
            .show_ui(ui, |ui| {
                for (id, path) in &assets {
                    if Some(id.as_str()) != app.native_selected_asset.as_deref() {
                        ui.selectable_value(&mut compare_asset, id.clone(), path);
                    }
                }
            });
        if compare_asset != app.native_compare_asset.clone().unwrap_or_default() {
            app.native_compare_asset = (!compare_asset.is_empty()).then_some(compare_asset);
            app.native_comparison_a = None;
            app.native_comparison_b = None;
            if let Err(error) = app.load_native_comparison_waveforms() {
                app.status_message = format!("Falha ao carregar comparação A/B: {error}");
            }
        }
        play_a = ui.button("▶ A").clicked();
        play_b = ui.button("▶ B").clicked();
    });
    if assets.len() < 2 {
        ui.small("Importe pelo menos duas gravações nesta camada para comparar tomadas.");
    } else {
        ui.small("Áudio de origem; cada forma de onda usa sua própria escala temporal. A comparação não altera os arquivos.");
        let asset_a_label = app.native_selected_asset.as_deref().unwrap_or("A");
        let asset_b_label = app.native_compare_asset.as_deref().unwrap_or("B");
        ui.columns(2, |columns| {
            let draw_take = |column: &mut egui::Ui,
                             label: &str,
                             waveform: Option<&(Vec<f32>, u32)>,
                             color: Color32| {
                column.label(label);
                if let Some((samples, sample_rate)) = waveform {
                    column.small(format!(
                        "{} ms · {} Hz",
                        samples.len() as f64 * 1_000.0 / f64::from(*sample_rate),
                        sample_rate
                    ));
                    let (rect, _) = column.allocate_exact_size(
                        Vec2::new(column.available_width(), 64.0),
                        Sense::hover(),
                    );
                    let painter = column.painter_at(rect);
                    painter.rect_filled(rect, Rounding::same(6.0), visual_tokens::BACKGROUND);
                    draw_waveform_peak_envelope(&painter, rect, samples, color, 0.8, 1.0);
                } else {
                    column.small("Gravação B indisponível");
                }
            };
            draw_take(
                &mut columns[0],
                asset_a_label,
                app.native_comparison_a.as_ref(),
                visual_tokens::ACCENT,
            );
            draw_take(
                &mut columns[1],
                asset_b_label,
                app.native_comparison_b.as_ref(),
                Color32::from_rgb(126, 190, 226),
            );
        });
    }
    if play_a {
        if let Err(error) = app.preview_native_comparison_take(audio_player, 'A') {
            app.status_message = error;
        }
    }
    if play_b {
        if let Err(error) = app.preview_native_comparison_take(audio_player, 'B') {
            app.status_message = error;
        }
    }

    ui.horizontal(|ui| {
        ui.label("Novo ID:");
        ui.add(egui::TextEdit::singleline(&mut app.native_new_unit_id).desired_width(120.0));
        ui.label("Tipo:");
        egui::ComboBox::from_id_salt("copaiba_kfv2_unit_kind")
            .selected_text(&app.native_new_kind)
            .show_ui(ui, |ui| {
                for kind in ["CV", "VC", "V", "VV", "CC", "breath", "release"] {
                    ui.selectable_value(&mut app.native_new_kind, kind.to_string(), kind);
                }
            });
        ui.label("Fonemas:");
        ui.add(
            egui::TextEdit::singleline(&mut app.native_new_phones)
                .desired_width(130.0)
                .hint_text("ex.: k a"),
        );
        if ui.button("Criar unidade da gravação selecionada").clicked() {
            app.native_selected_layer = Some(selected_layer.clone());
            match app.add_native_unit_from_selected_asset() {
                Ok(()) => {
                    app.status_message = format!("Unidade criada: {}", app.native_new_unit_id)
                }
                Err(error) => app.status_message = format!("Falha ao criar unidade: {error}"),
            }
        }
    });

    let selected = app.native_project.as_ref().and_then(|project| {
        let unit_id = app.native_selected_unit.as_deref()?;
        let unit = project.unit(&selected_layer, unit_id)?.clone();
        let (_, file) = project
            .layers
            .iter()
            .find(|(layer, _)| layer.id == selected_layer)?;
        let asset = file
            .assets
            .iter()
            .find(|asset| asset.id == unit.asset_id)?
            .clone();
        Some((unit, asset))
    });
    let Some((unit, asset)) = selected else {
        ui.separator();
        ui.label("Importe um WAV mono e selecione ou crie uma unidade KFV 2.");
        return;
    };
    ui.separator();
    ui.label(
        RichText::new(format!(
            "{} · {} · {} · {} Hz · {} frames",
            unit.id,
            unit.kind,
            unit.phones.join(" "),
            asset.sample_rate_hz,
            asset.frame_count
        ))
        .strong()
        .color(visual_tokens::ACCENT),
    );
    let mut reviewed = unit.quality.reviewed;
    if ui
        .checkbox(&mut reviewed, "Revisada pelo autor")
        .on_hover_text("Marca uma revisão humana explícita; essa informação é salva com a unidade e pode desempatar a seleção do planner.")
        .changed()
    {
        match app
            .native_project
            .as_mut()
            .ok_or_else(|| "Nenhum banco KFV 2 aberto".to_string())
            .and_then(|project| {
                project.set_unit_reviewed(&selected_layer, &unit.id, reviewed)
            })
        {
            Ok(()) => {
                app.status_message = if reviewed {
                    format!("Unidade marcada como revisada · {}", unit.id)
                } else {
                    format!("Revisão removida · {}", unit.id)
                }
            }
            Err(error) => app.status_message = format!("Falha ao atualizar revisão: {error}"),
        }
    }
    let sample_rate = f64::from(asset.sample_rate_hz);
    let duration_ms = asset.frame_count as f64 * 1_000.0 / sample_rate;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), 170.0), Sense::hover());
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, Rounding::same(6.0), visual_tokens::BACKGROUND);
    painter.rect_stroke(
        rect,
        Rounding::same(6.0),
        Stroke::new(1.0_f32, visual_tokens::BORDER),
    );
    if let Some((samples, _)) = &app.loaded_waveform {
        draw_waveform_peak_envelope(&painter, rect, samples, visual_tokens::ACCENT, 0.8, 1.0);
    }
    if duration_ms > 0.0 {
        let x_at = |frame: u64| {
            rect.left() + (frame as f64 * rect.width() as f64 / asset.frame_count as f64) as f32
        };
        let mark = |frame: u64, color: Color32, label: &str| {
            let x = x_at(frame);
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(1.5_f32, color),
            );
            painter.text(
                Pos2::new(x + 3.0, rect.top() + 2.0),
                egui::Align2::LEFT_TOP,
                label,
                egui::FontId::proportional(9.0),
                color,
            );
        };
        mark(
            unit.source.start_frame,
            Color32::from_rgb(126, 190, 226),
            "início",
        );
        mark(unit.source.anchor_frame, visual_tokens::ACCENT, "âncora");
        mark(unit.source.fixed_end_frame, visual_tokens::WARNING, "fixo");
        mark(unit.source.end_frame, visual_tokens::ERROR, "fim");
        if let Some(sustain) = &unit.source.sustain {
            mark(
                sustain.start_frame,
                Color32::from_rgb(126, 190, 226),
                "sustain início",
            );
            mark(
                sustain.end_frame,
                Color32::from_rgb(110, 150, 205),
                "sustain fim",
            );
        }
        if let Some(release) = unit.source.release_start_frame {
            mark(release, Color32::from_rgb(192, 150, 218), "release");
        }
    }

    let frame_ms = 1_000.0 / sample_rate;
    let mut source = unit.source.clone();
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label("Início");
        changed |= ui
            .add(
                egui::Slider::new(&mut source.start_frame, 0..=source.anchor_frame)
                    .custom_formatter(|v, _| format!("{:.2} ms", v * frame_ms)),
            )
            .changed();
        ui.label("Âncora");
        let anchor_max = source
            .fixed_end_frame
            .min(source.end_frame.saturating_sub(1))
            .min(
                source
                    .sustain
                    .as_ref()
                    .map(|region| region.start_frame)
                    .unwrap_or(source.fixed_end_frame),
            );
        changed |= ui
            .add(
                egui::Slider::new(&mut source.anchor_frame, source.start_frame..=anchor_max)
                    .custom_formatter(|v, _| format!("{:.2} ms", v * frame_ms)),
            )
            .changed();
        ui.label("Fim fixo");
        let fixed_max = source
            .sustain
            .as_ref()
            .map(|region| region.start_frame)
            .unwrap_or(source.end_frame);
        changed |= ui
            .add(
                egui::Slider::new(&mut source.fixed_end_frame, source.anchor_frame..=fixed_max)
                    .custom_formatter(|v, _| format!("{:.2} ms", v * frame_ms)),
            )
            .changed();
        ui.label("Fim");
        let end_min = source
            .fixed_end_frame
            .max(source.anchor_frame.saturating_add(1));
        changed |= ui
            .add(
                egui::Slider::new(&mut source.end_frame, end_min..=asset.frame_count)
                    .custom_formatter(|v, _| format!("{:.2} ms", v * frame_ms)),
            )
            .changed();
    });
    let can_enable_sustain = source.end_frame > source.fixed_end_frame.saturating_add(1);
    let mut has_sustain = source.sustain.is_some();
    if ui
        .add_enabled(
            can_enable_sustain || has_sustain,
            egui::Checkbox::new(&mut has_sustain, "Região de sustain"),
        )
        .changed()
    {
        if has_sustain {
            let start = (source.fixed_end_frame + 1).min(source.end_frame - 1);
            let end = (start + ((source.end_frame - start) / 2).max(1)).min(source.end_frame);
            source.sustain = Some(crate::voicebank::native::FrameRange {
                start_frame: start,
                end_frame: end,
            });
        } else {
            source.sustain = None;
        }
        changed = true;
    }
    if let Some(sustain) = source.sustain.as_mut() {
        ui.horizontal(|ui| {
            ui.label("Sustain início");
            let start_max = sustain
                .end_frame
                .saturating_sub(1)
                .min(
                    source
                        .release_start_frame
                        .unwrap_or(source.end_frame.saturating_sub(1)),
                )
                .max(source.fixed_end_frame);
            changed |= ui
                .add(
                    egui::Slider::new(&mut sustain.start_frame, source.fixed_end_frame..=start_max)
                        .custom_formatter(|v, _| format!("{:.2} ms", v * frame_ms)),
                )
                .changed();
            ui.label("Sustain fim");
            let end_max = source.release_start_frame.unwrap_or(source.end_frame);
            changed |= ui
                .add(
                    egui::Slider::new(&mut sustain.end_frame, (sustain.start_frame + 1)..=end_max)
                        .custom_formatter(|v, _| format!("{:.2} ms", v * frame_ms)),
                )
                .changed();
        });
    }
    let mut has_release = source.release_start_frame.is_some();
    if ui
        .add_enabled(
            source.end_frame > source.fixed_end_frame + 1 || has_release,
            egui::Checkbox::new(&mut has_release, "Início de release"),
        )
        .changed()
    {
        source.release_start_frame = if has_release {
            Some(
                source
                    .sustain
                    .as_ref()
                    .map(|region| region.end_frame)
                    .unwrap_or(source.fixed_end_frame)
                    .min(source.end_frame - 1),
            )
        } else {
            None
        };
        changed = true;
    }
    if let Some(release) = source.release_start_frame.as_mut() {
        let release_min = source
            .sustain
            .as_ref()
            .map(|region| region.end_frame)
            .unwrap_or(source.fixed_end_frame);
        changed |= ui
            .add(
                egui::Slider::new(release, release_min..=source.end_frame.saturating_sub(1))
                    .custom_formatter(|v, _| format!("{:.2} ms", v * frame_ms)),
            )
            .changed();
    }
    if changed {
        match app.native_project.as_mut().unwrap().set_source_region(
            &selected_layer,
            &unit.id,
            source,
        ) {
            Ok(()) => app.status_message = format!("Marcas atualizadas · {}", unit.id),
            Err(error) => app.status_message = format!("Marcas inválidas: {error}"),
        }
    }
    ui.label("As gravações originais ficam intactas; os imports são copiados para audio/imports.");
}

fn native_layer_lists(app: &CopaibaToolkitApp, layer_id: &str) -> NativeLayerLists {
    let Some(project) = app.native_project.as_ref() else {
        return ((), Vec::new(), Vec::new());
    };
    let Some((_, file)) = project
        .layers
        .iter()
        .find(|(layer, _)| layer.id == layer_id)
    else {
        return ((), Vec::new(), Vec::new());
    };
    (
        (),
        file.units
            .iter()
            .map(|unit| {
                (
                    unit.id.clone(),
                    format!("{} · {}", unit.kind, unit.phones.join(" ")),
                )
            })
            .collect(),
        file.assets
            .iter()
            .map(|asset| (asset.id.clone(), asset.path.clone()))
            .collect(),
    )
}
