use eframe::egui::{self, Color32, Pos2, Rect, RichText, Rounding, Sense, Stroke, Vec2};
use std::path::PathBuf;
use crate::copaiba::CopaibaConfig;
use crate::audio::AudioPlayer;

pub struct CopaibaToolkitApp {
    pub current_dir: Option<PathBuf>,
    pub config: CopaibaConfig,
    pub selected_wav: Option<String>,
    pub search_query: String,
    pub audio_player: AudioPlayer,
    pub loaded_waveform: Option<(Vec<f32>, u32)>, // samples, sample_rate
    pub is_loop_enabled: bool,
    pub is_tail_enabled: bool,
}

impl Default for CopaibaToolkitApp {
    fn default() -> Self {
        Self {
            current_dir: None,
            config: CopaibaConfig::default(),
            selected_wav: None,
            search_query: String::new(),
            audio_player: AudioPlayer::new(),
            loaded_waveform: None,
            is_loop_enabled: false,
            is_tail_enabled: false,
        }
    }
}

impl CopaibaToolkitApp {
    pub fn open_dir(&mut self, path: PathBuf) {
        if let Ok(cfg) = CopaibaConfig::load_from_dir(&path) {
            self.current_dir = Some(path);
            self.config = cfg;
            if self.selected_wav.is_none() {
                self.selected_wav = self.config.entries.keys().next().cloned();
            }
            self.load_selected_wav_samples();
        }
    }

    pub fn load_selected_wav_samples(&mut self) {
        self.loaded_waveform = None;
        if let (Some(ref dir), Some(ref wav_name)) = (&self.current_dir, &self.selected_wav) {
            let wav_path = dir.join(wav_name);
            if let Ok(reader) = hound::WavReader::open(&wav_path) {
                let spec = reader.spec();
                let samples: Vec<f32> = match spec.sample_format {
                    hound::SampleFormat::Int => {
                        let max_val = (1 << (spec.bits_per_sample - 1)) as f32;
                        reader.into_samples::<i32>()
                            .filter_map(Result::ok)
                            .map(|s| s as f32 / max_val)
                            .collect()
                    }
                    hound::SampleFormat::Float => {
                        reader.into_samples::<f32>()
                            .filter_map(Result::ok)
                            .collect()
                    }
                };
                let mono_samples = if spec.channels > 1 {
                    samples.chunks(spec.channels as usize)
                        .map(|chunk| chunk.iter().sum::<f32>() / spec.channels as f32)
                        .collect()
                } else {
                    samples
                };
                self.loaded_waveform = Some((mono_samples, spec.sample_rate));
            }
        }
    }

    pub fn save_config(&mut self) {
        if let Some(ref dir) = self.current_dir {
            let _ = self.config.save_to_dir(dir);
        }
    }
}

pub fn draw_copaiba_toolkit_ui(app: &mut CopaibaToolkitApp, ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        // Top Toolbar
        ui.horizontal(|ui| {
            ui.heading(RichText::new("Copaiba Voicebank Toolkit").strong().color(Color32::from_rgb(0, 255, 157)));
            ui.add_space(15.0);

            if ui.button("Abrir Pasta do Voicebank...").clicked() {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    app.open_dir(folder);
                }
            }

            if app.current_dir.is_some() {
                if ui.button(RichText::new("Salvar copaiba.config").color(Color32::from_rgb(0, 255, 157))).clicked() {
                    app.save_config();
                }
            }

            ui.add_space(15.0);
            if let Some(ref dir) = app.current_dir {
                ui.label(RichText::new(format!("Pasta: {}", dir.display())).size(11.0).color(Color32::from_rgb(180, 170, 210)));
            } else {
                ui.label(RichText::new("Nenhuma pasta aberta").size(11.0).color(Color32::from_rgb(150, 140, 170)));
            }
        });

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        if app.current_dir.is_none() {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("Bem-vindo ao Copaiba Voicebank Toolkit").size(18.0).strong().color(Color32::from_rgb(0, 255, 157)));
                    ui.add_space(8.0);
                    ui.label("Abra uma pasta de Voicebank contendo arquivos .wav para configurar seus fonemas.");
                    ui.add_space(12.0);
                    if ui.button(RichText::new("Selecionar Pasta do Voicebank").size(14.0)).clicked() {
                        if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                            app.open_dir(folder);
                        }
                    }
                });
            });
            return;
        }

        // Main 3-Column Layout: Left (WAV List), Center (Waveform), Right (Controls)
        ui.horizontal(|ui| {
            // Left Panel: Sample List
            ui.allocate_ui_with_layout(Vec2::new(200.0, ui.available_height()), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.label(RichText::new("Arquivos WAV").strong().size(12.0).color(Color32::from_rgb(0, 255, 157)));
                ui.add(egui::TextEdit::singleline(&mut app.search_query).hint_text("Filtrar...").desired_width(190.0));
                ui.add_space(4.0);

                let mut selected_changed = false;
                let mut wav_keys: Vec<String> = app.config.entries.keys().cloned().collect();
                wav_keys.sort();

                egui::ScrollArea::vertical().id_salt("copaiba_wav_list").show(ui, |ui| {
                    for key in wav_keys {
                        if !app.search_query.is_empty() && !key.to_lowercase().contains(&app.search_query.to_lowercase()) {
                            continue;
                        }
                        let is_sel = app.selected_wav.as_ref() == Some(&key);
                        let btn_text = if is_sel {
                            RichText::new(&key).strong().color(Color32::from_rgb(0, 255, 157))
                        } else {
                            RichText::new(&key).color(Color32::WHITE)
                        };
                        if ui.selectable_label(is_sel, btn_text).clicked() {
                            app.selected_wav = Some(key.clone());
                            selected_changed = true;
                        }
                    }
                });

                if selected_changed {
                    app.load_selected_wav_samples();
                }
            });

            ui.separator();

            // Center Panel: Waveform Editor
            let center_w = (ui.available_width() - 250.0).max(300.0);
            ui.allocate_ui_with_layout(Vec2::new(center_w, ui.available_height()), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                let sel_name = app.selected_wav.clone().unwrap_or_default();
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Editor de Onda: {}", sel_name)).strong().size(12.0).color(Color32::from_rgb(0, 255, 157)));
                    ui.add_space(10.0);

                    if let Some((ref samples, sr)) = app.loaded_waveform {
                        if ui.button("Tocar Sample").clicked() {
                            app.audio_player.play_samples(samples.clone(), sr);
                        }
                    }
                });

                ui.add_space(4.0);

                // Waveform Canvas Allocation
                let (canvas_rect, canvas_resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), ui.available_height() - 20.0), Sense::click_and_drag());
                let painter = ui.painter_at(canvas_rect);

                painter.rect_filled(canvas_rect, Rounding::same(4.0), Color32::from_rgb(18, 14, 26));
                painter.rect_stroke(canvas_rect, Rounding::same(4.0), Stroke::new(1.0, Color32::from_rgb(45, 35, 60)));

                if let (Some((ref samples, sr)), Some(ref wav_name)) = (&app.loaded_waveform, &app.selected_wav) {
                    let duration_ms = (samples.len() as f64 / *sr as f64) * 1000.0;

                    // Draw Waveform Overview
                    let mid_y = canvas_rect.center().y;
                    let num_samples = samples.len();
                    let px_width = canvas_rect.width() as usize;
                    if px_width > 0 && num_samples > 0 {
                        let step = (num_samples / px_width).max(1);
                        let mut points = Vec::new();
                        for x in 0..px_width {
                            let idx = (x * step).min(num_samples - 1);
                            let amp = samples[idx];
                            let px = canvas_rect.min.x + x as f32;
                            let py = mid_y - (amp * (canvas_rect.height() * 0.4));
                            points.push(Pos2::new(px, py));
                        }
                        if points.len() >= 2 {
                            for win in points.windows(2) {
                                painter.line_segment([win[0], win[1]], Stroke::new(1.0, Color32::from_rgb(0, 200, 130)));
                            }
                        }
                    }

                    // Get or create entry parameters
                    let entry = app.config.entries.entry(wav_name.clone()).or_default();

                    let ms_to_x = |ms: f64| -> f32 {
                        let ratio = (ms / duration_ms).clamp(0.0, 1.0) as f32;
                        canvas_rect.min.x + ratio * canvas_rect.width()
                    };

                    let x_to_ms = |x: f32| -> f64 {
                        let ratio = ((x - canvas_rect.min.x) / canvas_rect.width()).clamp(0.0, 1.0) as f64;
                        ratio * duration_ms
                    };

                    // Compute Marker Positions
                    let x_offset = ms_to_x(entry.corte_inicial_ms);
                    let x_consonant = ms_to_x(entry.corte_inicial_ms + entry.consoante_ms);
                    let x_cutoff = ms_to_x(if entry.corte_final_ms <= 0.0 {
                        duration_ms + entry.corte_final_ms
                    } else {
                        entry.corte_final_ms
                    });

                    // Shaded Left Offset Cutoff (Blue)
                    let left_cut_rect = Rect::from_min_max(canvas_rect.min, Pos2::new(x_offset, canvas_rect.max.y));
                    painter.rect_filled(left_cut_rect, Rounding::ZERO, Color32::from_rgba_premultiplied(0, 100, 255, 60));

                    // Shaded Fixed Consonant Region (Green)
                    let cons_rect = Rect::from_min_max(Pos2::new(x_offset, canvas_rect.min.y), Pos2::new(x_consonant, canvas_rect.max.y));
                    painter.rect_filled(cons_rect, Rounding::ZERO, Color32::from_rgba_premultiplied(0, 255, 120, 50));

                    // Shaded Right Cutoff (Red)
                    let right_cut_rect = Rect::from_min_max(Pos2::new(x_cutoff, canvas_rect.min.y), canvas_rect.max);
                    painter.rect_filled(right_cut_rect, Rounding::ZERO, Color32::from_rgba_premultiplied(255, 50, 50, 60));

                    // Optional Loop Region (Yellow)
                    if let (Some(l_start), Some(l_end)) = (entry.loop_inicio_ms, entry.loop_fim_ms) {
                        let x_lstart = ms_to_x(l_start);
                        let x_lend = ms_to_x(l_end);
                        let loop_rect = Rect::from_min_max(Pos2::new(x_lstart, canvas_rect.min.y), Pos2::new(x_lend, canvas_rect.max.y));
                        painter.rect_filled(loop_rect, Rounding::ZERO, Color32::from_rgba_premultiplied(240, 220, 0, 40));
                        painter.line_segment([Pos2::new(x_lstart, canvas_rect.min.y), Pos2::new(x_lstart, canvas_rect.max.y)], Stroke::new(1.5, Color32::YELLOW));
                        painter.line_segment([Pos2::new(x_lend, canvas_rect.min.y), Pos2::new(x_lend, canvas_rect.max.y)], Stroke::new(1.5, Color32::YELLOW));
                    }

                    // Optional Final Tail Region (Purple)
                    if let Some(tail) = entry.cauda_final_ms {
                        let x_tail = ms_to_x(tail);
                        painter.line_segment([Pos2::new(x_tail, canvas_rect.min.y), Pos2::new(x_tail, canvas_rect.max.y)], Stroke::new(2.0, Color32::from_rgb(180, 50, 255)));
                    }

                    // Render Vertical Marker Lines
                    painter.line_segment([Pos2::new(x_offset, canvas_rect.min.y), Pos2::new(x_offset, canvas_rect.max.y)], Stroke::new(2.0, Color32::from_rgb(0, 150, 255)));
                    painter.line_segment([Pos2::new(x_consonant, canvas_rect.min.y), Pos2::new(x_consonant, canvas_rect.max.y)], Stroke::new(2.0, Color32::from_rgb(0, 255, 120)));
                    painter.line_segment([Pos2::new(x_cutoff, canvas_rect.min.y), Pos2::new(x_cutoff, canvas_rect.max.y)], Stroke::new(2.0, Color32::from_rgb(255, 60, 60)));

                    // Mouse Drag Markers Interaction
                    if let Some(mpos) = canvas_resp.interact_pointer_pos() {
                        if canvas_resp.dragged() || canvas_resp.clicked() {
                            let clicked_ms = x_to_ms(mpos.x);
                            // Determine closest marker to drag
                            let dist_offset = (mpos.x - x_offset).abs();
                            let dist_cons = (mpos.x - x_consonant).abs();
                            let dist_cutoff = (mpos.x - x_cutoff).abs();

                            if dist_offset <= dist_cons && dist_offset <= dist_cutoff {
                                entry.corte_inicial_ms = clicked_ms.clamp(0.0, duration_ms);
                            } else if dist_cons <= dist_cutoff {
                                entry.consoante_ms = (clicked_ms - entry.corte_inicial_ms).max(1.0);
                            } else {
                                entry.corte_final_ms = clicked_ms.clamp(0.0, duration_ms);
                            }
                        }
                    }
                }
            });

            ui.separator();

            // Right Panel: Numeric Parameter Controls & Tuning
            ui.allocate_ui_with_layout(Vec2::new(230.0, ui.available_height()), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.label(RichText::new("Parâmetros do Fonema").strong().size(12.0).color(Color32::from_rgb(0, 255, 157)));
                ui.add_space(6.0);

                if let Some(ref wav_name) = app.selected_wav {
                    let entry = app.config.entries.entry(wav_name.clone()).or_default();

                    ui.label(RichText::new("Alias / Letra:").size(11.0).color(Color32::from_rgb(200, 190, 230)));
                    ui.add(egui::TextEdit::singleline(&mut entry.alias).desired_width(210.0));

                    ui.add_space(8.0);
                    ui.label(RichText::new("Corte Inicial (ms):").size(11.0).color(Color32::from_rgb(0, 150, 255)));
                    ui.add(egui::DragValue::new(&mut entry.corte_inicial_ms).speed(1.0).range(0.0..=10000.0));

                    ui.add_space(8.0);
                    ui.label(RichText::new("Início Consoante (ms):").size(11.0).color(Color32::from_rgb(0, 255, 120)));
                    ui.add(egui::DragValue::new(&mut entry.consoante_ms).speed(1.0).range(0.0..=10000.0));

                    ui.add_space(8.0);
                    ui.label(RichText::new("Corte Final (ms):").size(11.0).color(Color32::from_rgb(255, 60, 60)));
                    ui.add(egui::DragValue::new(&mut entry.corte_final_ms).speed(1.0));

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(6.0);

                    // Loop Settings Section
                    ui.label(RichText::new("Parte de Loop (Sustentação)").strong().size(11.0).color(Color32::YELLOW));
                    let mut has_loop = entry.loop_inicio_ms.is_some();
                    if ui.checkbox(&mut has_loop, "Habilitar Região de Loop").changed() {
                        if has_loop {
                            entry.loop_inicio_ms = Some(100.0);
                            entry.loop_fim_ms = Some(300.0);
                        } else {
                            entry.loop_inicio_ms = None;
                            entry.loop_fim_ms = None;
                        }
                    }

                    if has_loop {
                        let l_start = entry.loop_inicio_ms.get_or_insert(100.0);
                        ui.horizontal(|ui| {
                            ui.label("Início:");
                            ui.add(egui::DragValue::new(l_start).speed(1.0));
                        });
                        let l_end = entry.loop_fim_ms.get_or_insert(300.0);
                        ui.horizontal(|ui| {
                            ui.label("Fim:");
                            ui.add(egui::DragValue::new(l_end).speed(1.0));
                        });
                    }

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(6.0);

                    // Optional Diphthong Final Tail Section
                    ui.label(RichText::new("Cauda Final (Ditongos)").strong().size(11.0).color(Color32::from_rgb(180, 50, 255)));
                    let mut has_tail = entry.cauda_final_ms.is_some();
                    if ui.checkbox(&mut has_tail, "Habilitar Cauda Final").changed() {
                        if has_tail {
                            entry.cauda_final_ms = Some(entry.corte_inicial_ms + entry.consoante_ms + 100.0);
                        } else {
                            entry.cauda_final_ms = None;
                        }
                    }

                    if has_tail {
                        let tail = entry.cauda_final_ms.get_or_insert(150.0);
                        ui.horizontal(|ui| {
                            ui.label("Posição:");
                            ui.add(egui::DragValue::new(tail).speed(1.0));
                        });
                    }
                }
            });
        });
    });
}
