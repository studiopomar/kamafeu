use super::super::KamafeuStudioApp;
use crate::formats::ApsFormat;
use eframe::egui;
use std::path::{Path, PathBuf};
use std::sync::mpsc;

fn copy_reference_audio(source: &Path, aps_path: &Path) -> Result<PathBuf, String> {
    let stem = aps_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("vocal");
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("wav");
    for suffix in 0..1000 {
        let file_name = if suffix == 0 {
            format!("{stem}_referencia.{extension}")
        } else {
            format!("{stem}_referencia_{suffix}.{extension}")
        };
        let destination = aps_path.with_file_name(file_name);
        if !destination.exists() {
            std::fs::copy(source, &destination).map_err(|error| error.to_string())?;
            return Ok(destination);
        }
    }
    Err("não foi possível escolher um nome livre para o áudio de referência".into())
}

impl KamafeuStudioApp {
    pub(crate) fn show_game_transcription(&mut self, ctx: &egui::Context) {
        let state = &mut self.game_transcription;
        if let Some(receiver) = state.receiver.take() {
            match receiver.try_recv() {
                Ok(Ok(notes)) => {
                    state.status = Some(format!(
                        "{} notas estimadas. Revise a prévia antes de criar o APS.",
                        notes.len()
                    ));
                    state.notes = Some(notes);
                    state.pitch_shift_semitones = 0;
                }
                Ok(Err(error)) => {
                    state.notes = None;
                    state.status = Some(error);
                }
                Err(mpsc::TryRecvError::Empty) => state.receiver = Some(receiver),
                Err(mpsc::TryRecvError::Disconnected) => {
                    state.status =
                        Some("A tarefa de transcrição foi encerrada sem resultado.".into());
                }
            }
        }
        if !state.open {
            return;
        }

        let mut open = state.open;
        let mut open_created_project: Option<PathBuf> = None;
        egui::Window::new("Vocal para APS — GAME")
            .open(&mut open)
            .default_size([650.0, 580.0])
            .min_width(460.0)
            .show(ctx, |ui| {
                super::visual::header(
                    ui,
                    super::visual::Art::Vocal,
                    "Vocal para APS — GAME",
                    "Estime notas, revise a prévia e salve um projeto APS.",
                    self.config.theme.success_c32(),
                );
                ui.add_space(8.0);
                let state = &mut self.game_transcription;
                ui.small("GAME estima notas; não reconhece a letra cantada. As letras provisórias poderão ser editadas no piano roll.");
                ui.add_space(8.0);

                ui.horizontal(|ui| {
                    if ui.button("Selecionar áudio…").clicked() {
                        state.audio_path = crate::dialogs::FileDialog::new()
                            .set_title("Selecionar vocal para transcrição")
                            .add_filter("Áudio (*.wav, *.mp3, *.ogg, *.flac)", &["wav", "mp3", "ogg", "flac"])
                            .pick_file();
                        state.notes = None;
                        state.pitch_shift_semitones = 0;
                    }
                    ui.label(state.audio_path.as_ref().map_or_else(
                        || "Nenhum áudio selecionado".to_string(),
                        |path| path.display().to_string(),
                    ));
                });
                ui.horizontal(|ui| {
                    ui.label("Comando GAME:");
                    ui.add(egui::TextEdit::singleline(&mut state.command).desired_width(240.0));
                    if ui.button("Localizar…").clicked() {
                        if let Some(path) = crate::dialogs::FileDialog::new()
                            .set_title("Localizar o executável do backend GAME")
                            .pick_file()
                        {
                            state.command = path.display().to_string();
                        }
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Modelo:");
                    ui.add(egui::TextEdit::singleline(&mut state.model).desired_width(180.0));
                    ui.label("Idioma (opcional):");
                    ui.add(egui::TextEdit::singleline(&mut state.language).desired_width(100.0));
                });
                ui.horizontal(|ui| {
                    ui.label("Letra provisória por nota:");
                    ui.add(egui::TextEdit::singleline(&mut state.lyric).desired_width(90.0));
                });
                ui.small("Na primeira execução, o backend pode baixar o modelo selecionado. O Kamafeu não inclui nem baixa modelos automaticamente.");

                let can_analyze = cfg!(not(any(target_arch = "wasm32", target_os = "android")))
                    && state.audio_path.is_some()
                    && state.receiver.is_none();
                if ui.add_enabled(can_analyze, egui::Button::new(if state.receiver.is_some() { "Analisando…" } else { "Analisar gravação" })).clicked() {
                    if let Some(audio_path) = state.audio_path.clone() {
                        let command = state.command.clone();
                        let model = state.model.clone();
                        let language = state.language.clone();
                        let (sender, receiver) = mpsc::channel();
                        state.receiver = Some(receiver);
                        state.notes = None;
                        state.status = Some("Executando GAME no backend selecionado…".into());
                        let repaint = ctx.clone();
                        std::thread::spawn(move || {
                            let result = crate::transcription::transcribe_with_game_command(
                                &command,
                                &audio_path,
                                &model,
                                (!language.trim().is_empty()).then_some(language.as_str()),
                            );
                            let _ = sender.send(result);
                            repaint.request_repaint();
                        });
                    }
                }

                if let Some(status) = &state.status {
                    super::visual::card(ui, |ui| {
                        let color = if state.receiver.is_some() {
                            ui.visuals().hyperlink_color
                        } else if state.notes.is_some() {
                            self.config.theme.success_c32()
                        } else {
                            ui.visuals().warn_fg_color
                        };
                        ui.label(egui::RichText::new(status).color(color));
                    });
                }
                if let Some(notes) = &mut state.notes {
                    ui.separator();
                    ui.heading("Prévia da transcrição");
                    ui.label(format!("{} notas; a afinação em cents será preservada no APS.", notes.len()));
                    let previous_shift = state.pitch_shift_semitones;
                    ui.horizontal(|ui| {
                        ui.label("Transposição global:");
                        ui.add(egui::Slider::new(&mut state.pitch_shift_semitones, -24..=24).suffix(" semitons"));
                    });
                    if state.pitch_shift_semitones != previous_shift {
                        let delta = f64::from(state.pitch_shift_semitones - previous_shift);
                        for note in notes.iter_mut() {
                            note.pitch_midi = (note.pitch_midi + delta).clamp(0.0, 127.0);
                        }
                    }
                    ui.small("Ajuste início, duração e afinação por nota na tabela antes de salvar.");
                    egui::ScrollArea::vertical().max_height(250.0).show(ui, |ui| {
                        egui::Grid::new("game_transcription_preview")
                            .striped(true)
                            .num_columns(4)
                            .show(ui, |ui| {
                                for heading in ["Início (s)", "Duração (s)", "Nota MIDI", "Letra"] {
                                    ui.strong(heading);
                                }
                                ui.end_row();
                                for note in notes.iter_mut() {
                                    let old_duration = (note.end_seconds - note.start_seconds).max(0.01);
                                    let start_changed = ui.add(egui::DragValue::new(&mut note.start_seconds).speed(0.01).suffix(" s").range(0.0..=86_400.0)).changed();
                                    if start_changed {
                                        note.end_seconds = note.start_seconds + old_duration;
                                    }
                                    let mut duration = note.end_seconds - note.start_seconds;
                                    if ui.add(egui::DragValue::new(&mut duration).speed(0.01).suffix(" s").range(0.01..=86_400.0)).changed() {
                                        note.end_seconds = note.start_seconds + duration;
                                    }
                                    ui.add(egui::DragValue::new(&mut note.pitch_midi).speed(0.01).range(0.0..=127.0));
                                    ui.label(&state.lyric);
                                    ui.end_row();
                                }
                            });
                    });
                    if ui.button("Criar e salvar projeto .aps…").clicked() {
                        if let (Some(audio_source), Some(notes)) = (state.audio_path.clone(), state.notes.as_ref()) {
                    let default_name = format!(
                        "{}.aps",
                        audio_source
                            .file_stem()
                            .and_then(|name| name.to_str())
                            .unwrap_or("vocal")
                    );
                    if let Some(mut aps_path) = crate::dialogs::FileDialog::new()
                                .set_title("Salvar projeto transcrito")
                                .set_file_name(&default_name)
                                .add_filter("Projeto Kamafeu (*.aps)", &["aps"])
                                .save_file()
                            {
                                if aps_path.extension().is_none_or(|extension| extension != "aps") {
                                    aps_path.set_extension("aps");
                                }
                                let result = (|| {
                                    let reference = copy_reference_audio(&audio_source, &aps_path)?;
                                    let mut project = crate::transcription::project_from_game_notes(
                                        audio_source.file_stem().and_then(|name| name.to_str()).unwrap_or("Vocal transcrito"),
                                        &reference,
                                        notes,
                                        &state.lyric,
                                    )?;
                                    project.wave_parts[0].file_path = reference.to_string_lossy().into_owned();
                                    if let Err(error) = ApsFormat::save_file(&project, &aps_path) {
                                        let _ = std::fs::remove_file(reference);
                                        return Err(error.to_string());
                                    }
                                    Ok(aps_path.clone())
                                })();
                                match result {
                                    Ok(path) => {
                                        state.status = Some(format!("Projeto criado: {}", path.display()));
                                        open_created_project = Some(path);
                                    }
                                    Err(error) => state.status = Some(format!("Falha ao criar APS: {error}")),
                                }
                            }
                        }
                    }
                }
            });
        self.game_transcription.open = open;
        if let Some(path) = open_created_project {
            self.open_project_from_path(&path);
        }
    }
}
