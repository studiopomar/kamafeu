use eframe::egui::{self, Color32, Pos2, Rect, RichText, Rounding, Sense, Stroke, Vec2};
use std::path::PathBuf;
use crate::copaiba::{CopaibaConfig, CopaibaEntry};
use crate::audio::AudioPlayer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListFilterMode {
    ByAlias,
    ByWavFile,
}

pub struct CopaibaToolkitApp {
    pub current_dir: Option<PathBuf>,
    pub config: CopaibaConfig,
    pub selected_entry_index: Option<usize>,
    pub search_query: String,
    pub filter_mode: ListFilterMode,
    pub audio_player: AudioPlayer,
    pub loaded_waveform: Option<(Vec<f32>, u32)>, // samples, sample_rate
    pub loaded_wav_filename: Option<String>,
    pub zoom_x: f32,
    pub zoom_y: f32,
}

impl Default for CopaibaToolkitApp {
    fn default() -> Self {
        Self {
            current_dir: None,
            config: CopaibaConfig::default(),
            selected_entry_index: None,
            search_query: String::new(),
            filter_mode: ListFilterMode::ByAlias,
            audio_player: AudioPlayer::new(),
            loaded_waveform: None,
            loaded_wav_filename: None,
            zoom_x: 1.0,
            zoom_y: 1.0,
        }
    }
}

impl CopaibaToolkitApp {
    pub fn open_dir(&mut self, path: PathBuf) {
        if let Ok(cfg) = CopaibaConfig::load_from_dir(&path) {
            self.current_dir = Some(path);
            self.config = cfg;
            if !self.config.entries.is_empty() {
                self.selected_entry_index = Some(0);
            } else {
                self.selected_entry_index = None;
            }
            self.load_selected_wav_samples();
        }
    }

    pub fn selected_entry(&self) -> Option<&CopaibaEntry> {
        self.selected_entry_index.and_then(|idx| self.config.entries.get(idx))
    }

    pub fn selected_entry_mut(&mut self) -> Option<&mut CopaibaEntry> {
        if let Some(idx) = self.selected_entry_index {
            self.config.entries.get_mut(idx)
        } else {
            None
        }
    }

    pub fn load_selected_wav_samples(&mut self) {
        let target_wav = self.selected_entry().map(|e| e.wav_filename.clone());
        if let (Some(ref dir), Some(ref wav_name)) = (&self.current_dir, &target_wav) {
            if self.loaded_wav_filename.as_ref() == Some(wav_name) && self.loaded_waveform.is_some() {
                return;
            }

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
                self.loaded_wav_filename = Some(wav_name.clone());
            } else {
                self.loaded_waveform = None;
                self.loaded_wav_filename = None;
            }
        }
    }

    pub fn duplicate_selected_entry(&mut self) {
        if let Some(idx) = self.selected_entry_index {
            if let Some(entry) = self.config.entries.get(idx).cloned() {
                let mut dup = entry;
                dup.alias = format!("{}_copia", dup.alias);
                let new_idx = idx + 1;
                self.config.entries.insert(new_idx, dup);
                self.selected_entry_index = Some(new_idx);
            }
        }
    }

    pub fn delete_selected_entry(&mut self) {
        if let Some(idx) = self.selected_entry_index {
            if idx < self.config.entries.len() {
                self.config.entries.remove(idx);
                if self.config.entries.is_empty() {
                    self.selected_entry_index = None;
                } else if idx >= self.config.entries.len() {
                    self.selected_entry_index = Some(self.config.entries.len() - 1);
                }
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

                ui.add_space(10.0);
                if ui.button(RichText::new("Duplicar Alias").color(Color32::from_rgb(255, 215, 0))).clicked() {
                    app.duplicate_selected_entry();
                }

                if app.selected_entry_index.is_some() {
                    if ui.button(RichText::new("Excluir Alias").color(Color32::from_rgb(255, 100, 100))).clicked() {
                        app.delete_selected_entry();
                    }
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
                    ui.label("Abra uma pasta de Voicebank contendo arquivos .wav para configurar seus fonemas e criar seu copaiba.config.");
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

        // Main 3-Column Layout: Left (Alias List & Search), Center (High-Legibility Waveform Canvas), Right (Sliders & Parameter Controls)
        ui.horizontal(|ui| {
            // Left Panel: Alias & WAV List
            ui.allocate_ui_with_layout(Vec2::new(230.0, ui.available_height()), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    if ui.selectable_label(app.filter_mode == ListFilterMode::ByAlias, "Lista de Alias").clicked() {
                        app.filter_mode = ListFilterMode::ByAlias;
                    }
                    if ui.selectable_label(app.filter_mode == ListFilterMode::ByWavFile, "Arquivos WAV").clicked() {
                        app.filter_mode = ListFilterMode::ByWavFile;
                    }
                });

                ui.add_space(4.0);
                ui.add(egui::TextEdit::singleline(&mut app.search_query).hint_text("Filtrar alias ou wav...").desired_width(220.0));
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if ui.button(RichText::new("+ Duplicar Alias").size(10.5).color(Color32::from_rgb(0, 255, 157))).clicked() {
                        app.duplicate_selected_entry();
                    }
                    if ui.button(RichText::new("🗑 Excluir").size(10.5).color(Color32::from_rgb(255, 100, 100))).clicked() {
                        app.delete_selected_entry();
                    }
                });

                ui.add_space(4.0);
                ui.separator();

                let mut new_selection = app.selected_entry_index;

                egui::ScrollArea::vertical().id_salt("copaiba_alias_scroll_list").show(ui, |ui| {
                    for (idx, entry) in app.config.entries.iter().enumerate() {
                        let query = app.search_query.to_lowercase();
                        if !query.is_empty() {
                            let match_alias = entry.alias.to_lowercase().contains(&query);
                            let match_wav = entry.wav_filename.to_lowercase().contains(&query);
                            if !match_alias && !match_wav {
                                continue;
                            }
                        }

                        let is_sel = app.selected_entry_index == Some(idx);
                        let display_title = if app.filter_mode == ListFilterMode::ByAlias {
                            format!("{} ({})", entry.alias, entry.wav_filename)
                        } else {
                            format!("{} -> {}", entry.wav_filename, entry.alias)
                        };

                        let text_color = if is_sel {
                            Color32::from_rgb(0, 255, 157)
                        } else {
                            Color32::from_rgb(210, 200, 230)
                        };

                        if ui.selectable_label(is_sel, RichText::new(display_title).size(11.0).color(text_color)).clicked() {
                            new_selection = Some(idx);
                        }
                    }
                });

                if new_selection != app.selected_entry_index {
                    app.selected_entry_index = new_selection;
                    app.load_selected_wav_samples();
                }
            });

            ui.separator();

            // Center Panel: High-Legibility Waveform Visualizer Canvas
            let center_w = (ui.available_width() - 280.0).max(300.0);
            ui.allocate_ui_with_layout(Vec2::new(center_w, ui.available_height()), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                let sel_alias = app.selected_entry().map(|e| e.alias.clone()).unwrap_or_default();
                let sel_wav = app.selected_entry().map(|e| e.wav_filename.clone()).unwrap_or_default();

                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("Waveform: {} [{}]", sel_alias, sel_wav)).strong().size(12.0).color(Color32::from_rgb(0, 255, 157)));
                    ui.add_space(10.0);

                    if let Some((ref samples, sr)) = app.loaded_waveform {
                        if ui.button("▶ Tocar Waveform").clicked() {
                            app.audio_player.play_samples(samples.clone(), sr);
                        }
                    }

                    ui.add_space(10.0);
                    ui.label(RichText::new("Zoom X:").size(10.0).color(Color32::from_rgb(180, 170, 190)));
                    ui.add(egui::Slider::new(&mut app.zoom_x, 0.2..=5.0).show_value(false));
                    ui.label(RichText::new("Zoom Y:").size(10.0).color(Color32::from_rgb(180, 170, 190)));
                    ui.add(egui::Slider::new(&mut app.zoom_y, 0.5..=4.0).show_value(false));
                });

                ui.add_space(4.0);

                // Waveform Display Canvas
                let canvas_h = (ui.available_height() - 10.0).max(200.0);
                let (canvas_rect, canvas_resp) = ui.allocate_exact_size(Vec2::new(ui.available_width(), canvas_h), Sense::click_and_drag());
                let painter = ui.painter_at(canvas_rect);

                // Deep Neon Glowing Canvas Background
                painter.rect_filled(canvas_rect, Rounding::same(4.0), Color32::from_rgb(12, 10, 20));
                painter.rect_stroke(canvas_rect, Rounding::same(4.0), Stroke::new(1.2, Color32::from_rgb(45, 35, 60)));

                let zoom_y = app.zoom_y;
                let sel_idx = app.selected_entry_index;

                if let (Some((ref samples, sr)), Some(idx)) = (&app.loaded_waveform, sel_idx) {
                    if idx < app.config.entries.len() {
                        let entry = &mut app.config.entries[idx];
                        let total_duration_ms = (samples.len() as f64 / *sr as f64) * 1000.0;

                    let ms_to_x = |ms: f64| -> f32 {
                        let ratio = (ms / total_duration_ms).clamp(0.0, 1.0) as f32;
                        canvas_rect.min.x + ratio * canvas_rect.width()
                    };

                    let x_to_ms = |x: f32| -> f64 {
                        let ratio = ((x - canvas_rect.min.x) / canvas_rect.width()).clamp(0.0, 1.0) as f64;
                        ratio * total_duration_ms
                    };

                    // Draw Time Ruler Overlay (ms markers)
                    let ruler_h = 18.0f32;
                    let ruler_rect = Rect::from_min_size(canvas_rect.min, Vec2::new(canvas_rect.width(), ruler_h));
                    painter.rect_filled(ruler_rect, Rounding::same(2.0), Color32::from_rgb(22, 18, 34));
                    painter.line_segment([Pos2::new(ruler_rect.min.x, ruler_rect.max.y), Pos2::new(ruler_rect.max.x, ruler_rect.max.y)], Stroke::new(1.0, Color32::from_rgb(60, 50, 80)));

                    let step_ms = if total_duration_ms > 2000.0 { 200.0 } else { 50.0 };
                    let mut ms_tick = 0.0f64;
                    while ms_tick <= total_duration_ms {
                        let tick_x = ms_to_x(ms_tick);
                        if tick_x >= canvas_rect.min.x && tick_x <= canvas_rect.max.x {
                            painter.line_segment(
                                [Pos2::new(tick_x, ruler_rect.min.y + 8.0), Pos2::new(tick_x, canvas_rect.max.y)],
                                Stroke::new(1.0, Color32::from_rgba_premultiplied(80, 70, 110, 100)),
                            );
                            painter.text(
                                Pos2::new(tick_x + 3.0, ruler_rect.min.y + 2.0),
                                egui::Align2::LEFT_TOP,
                                format!("{:.0}ms", ms_tick),
                                egui::FontId::proportional(9.0),
                                Color32::from_rgb(160, 150, 190),
                            );
                        }
                        ms_tick += step_ms;
                    }

                    // Zero Amplitude Baseline Reference Line
                    let wave_area_min_y = canvas_rect.min.y + ruler_h;
                    let wave_area_h = canvas_rect.height() - ruler_h;
                    let mid_y = wave_area_min_y + wave_area_h * 0.5;

                    painter.line_segment(
                        [Pos2::new(canvas_rect.min.x, mid_y), Pos2::new(canvas_rect.max.x, mid_y)],
                        Stroke::new(1.0, Color32::from_rgba_premultiplied(0, 255, 157, 100)),
                    );

                    // High-Legibility Solid Filled Peak Envelope Waveform Rendering
                    let px_width = canvas_rect.width() as usize;
                    let num_samples = samples.len();
                    if px_width > 0 && num_samples > 0 {
                        let samples_per_pixel = (num_samples as f32 / px_width as f32) as usize;
                        let step = samples_per_pixel.max(1);

                        for px_i in 0..px_width {
                            let start_s = px_i * step;
                            let end_s = ((px_i + 1) * step).min(num_samples);
                            if start_s >= num_samples {
                                break;
                            }

                            let chunk = &samples[start_s..end_s];
                            let mut min_val = 0.0f32;
                            let mut max_val = 0.0f32;
                            for &s in chunk {
                                if s < min_val { min_val = s; }
                                if s > max_val { max_val = s; }
                            }

                            let x_pos = canvas_rect.min.x + px_i as f32;
                            let y_top = mid_y - (max_val * (wave_area_h * 0.45) * zoom_y);
                            let y_bot = mid_y - (min_val * (wave_area_h * 0.45) * zoom_y);

                            // Draw peak vertical line bar
                            painter.line_segment(
                                [Pos2::new(x_pos, y_top), Pos2::new(x_pos, y_bot)],
                                Stroke::new(1.0, Color32::from_rgb(0, 255, 160)),
                            );
                        }
                    }

                    // Compute Marker Positions
                    let x_offset = ms_to_x(entry.corte_inicial_ms);
                    let x_consonant = ms_to_x(entry.corte_inicial_ms + entry.consoante_ms);
                    let x_cutoff = ms_to_x(if entry.corte_final_ms <= 0.0 {
                        total_duration_ms + entry.corte_final_ms
                    } else {
                        entry.corte_final_ms
                    });

                    // Shaded Left Offset Cutoff (Blue)
                    let left_cut_rect = Rect::from_min_max(Pos2::new(canvas_rect.min.x, wave_area_min_y), Pos2::new(x_offset, canvas_rect.max.y));
                    painter.rect_filled(left_cut_rect, Rounding::ZERO, Color32::from_rgba_premultiplied(0, 100, 255, 70));

                    // Shaded Fixed Consonant Region (Green)
                    let cons_rect = Rect::from_min_max(Pos2::new(x_offset, wave_area_min_y), Pos2::new(x_consonant, canvas_rect.max.y));
                    painter.rect_filled(cons_rect, Rounding::ZERO, Color32::from_rgba_premultiplied(0, 255, 120, 50));

                    // Shaded Right Cutoff (Red)
                    let right_cut_rect = Rect::from_min_max(Pos2::new(x_cutoff, wave_area_min_y), canvas_rect.max);
                    painter.rect_filled(right_cut_rect, Rounding::ZERO, Color32::from_rgba_premultiplied(255, 50, 50, 70));

                    // Optional Loop Region (Yellow Shaded)
                    if let (Some(l_start), Some(l_end)) = (entry.loop_inicio_ms, entry.loop_fim_ms) {
                        let x_lstart = ms_to_x(l_start);
                        let x_lend = ms_to_x(l_end);
                        let loop_rect = Rect::from_min_max(Pos2::new(x_lstart, wave_area_min_y), Pos2::new(x_lend, canvas_rect.max.y));
                        painter.rect_filled(loop_rect, Rounding::ZERO, Color32::from_rgba_premultiplied(240, 220, 0, 50));
                        painter.line_segment([Pos2::new(x_lstart, wave_area_min_y), Pos2::new(x_lstart, canvas_rect.max.y)], Stroke::new(2.0, Color32::YELLOW));
                        painter.line_segment([Pos2::new(x_lend, wave_area_min_y), Pos2::new(x_lend, canvas_rect.max.y)], Stroke::new(2.0, Color32::YELLOW));
                    }

                    // Optional Final Tail Region (Purple)
                    if let Some(tail) = entry.cauda_final_ms {
                        let x_tail = ms_to_x(tail);
                        painter.line_segment([Pos2::new(x_tail, wave_area_min_y), Pos2::new(x_tail, canvas_rect.max.y)], Stroke::new(2.2, Color32::from_rgb(190, 60, 255)));
                    }

                    // Render Marker Lines & Labels
                    painter.line_segment([Pos2::new(x_offset, wave_area_min_y), Pos2::new(x_offset, canvas_rect.max.y)], Stroke::new(2.5, Color32::from_rgb(0, 150, 255)));
                    painter.text(Pos2::new(x_offset + 3.0, wave_area_min_y + 4.0), egui::Align2::LEFT_TOP, "Corte Inicial", egui::FontId::proportional(10.0), Color32::from_rgb(0, 180, 255));

                    painter.line_segment([Pos2::new(x_consonant, wave_area_min_y), Pos2::new(x_consonant, canvas_rect.max.y)], Stroke::new(2.5, Color32::from_rgb(0, 255, 120)));
                    painter.text(Pos2::new(x_consonant + 3.0, wave_area_min_y + 4.0), egui::Align2::LEFT_TOP, "Início Consoante", egui::FontId::proportional(10.0), Color32::from_rgb(0, 255, 120));

                    painter.line_segment([Pos2::new(x_cutoff, wave_area_min_y), Pos2::new(x_cutoff, canvas_rect.max.y)], Stroke::new(2.5, Color32::from_rgb(255, 60, 60)));
                    painter.text(Pos2::new(x_cutoff + 3.0, wave_area_min_y + 4.0), egui::Align2::LEFT_TOP, "Corte Final", egui::FontId::proportional(10.0), Color32::from_rgb(255, 80, 80));

                    // Mouse Drag Markers Interaction
                    if let Some(mpos) = canvas_resp.interact_pointer_pos() {
                        if canvas_resp.dragged() || canvas_resp.clicked() {
                            let clicked_ms = x_to_ms(mpos.x);
                            let dist_offset = (mpos.x - x_offset).abs();
                            let dist_cons = (mpos.x - x_consonant).abs();
                            let dist_cutoff = (mpos.x - x_cutoff).abs();

                            if dist_offset <= dist_cons && dist_offset <= dist_cutoff {
                                entry.corte_inicial_ms = clicked_ms.clamp(0.0, total_duration_ms);
                            } else if dist_cons <= dist_cutoff {
                                entry.consoante_ms = (clicked_ms - entry.corte_inicial_ms).max(1.0);
                            } else {
                                entry.corte_final_ms = clicked_ms.clamp(0.0, total_duration_ms);
                            }
                        }
                    }
                }
            }
            });

            ui.separator();

            // Right Panel: Sliders & Numeric Tuning Controls
            ui.allocate_ui_with_layout(Vec2::new(250.0, ui.available_height()), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.label(RichText::new("Controles & Sliders").strong().size(12.0).color(Color32::from_rgb(0, 255, 157)));
                ui.add_space(6.0);

                let sel_idx = app.selected_entry_index;
                let wav_duration_ms = app.loaded_waveform.as_ref().map(|(s, sr)| (s.len() as f64 / *sr as f64) * 1000.0);

                if let (Some(idx), Some(duration_ms)) = (sel_idx, wav_duration_ms) {
                    if idx < app.config.entries.len() {
                        let entry = &mut app.config.entries[idx];

                    ui.label(RichText::new("Nome do Alias:").size(11.0).color(Color32::from_rgb(200, 190, 230)));
                    ui.add(egui::TextEdit::singleline(&mut entry.alias).desired_width(230.0));

                    ui.add_space(4.0);
                    ui.label(RichText::new("Arquivo WAV:").size(10.0).color(Color32::from_rgb(160, 150, 180)));
                    ui.label(RichText::new(&entry.wav_filename).size(11.0).color(Color32::WHITE));

                    ui.add_space(8.0);
                    ui.separator();
                    ui.add_space(6.0);

                    // Corte Inicial (Offset) Slider
                    ui.label(RichText::new("Corte Inicial (ms):").size(11.0).color(Color32::from_rgb(0, 150, 255)));
                    ui.add(egui::Slider::new(&mut entry.corte_inicial_ms, 0.0..=duration_ms).suffix(" ms"));

                    ui.add_space(8.0);
                    // Início Consoante Slider
                    ui.label(RichText::new("Início Consoante (ms):").size(11.0).color(Color32::from_rgb(0, 255, 120)));
                    ui.add(egui::Slider::new(&mut entry.consoante_ms, 0.0..=duration_ms).suffix(" ms"));

                    ui.add_space(8.0);
                    // Corte Final (Cutoff) Slider
                    ui.label(RichText::new("Corte Final (ms):").size(11.0).color(Color32::from_rgb(255, 60, 60)));
                    ui.add(egui::Slider::new(&mut entry.corte_final_ms, -duration_ms..=duration_ms).suffix(" ms"));

                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(6.0);

                    // Loop Region Sliders Section
                    ui.label(RichText::new("Parte de Loop (Sustentação)").strong().size(11.0).color(Color32::YELLOW));
                    let mut has_loop = entry.loop_inicio_ms.is_some();
                    if ui.checkbox(&mut has_loop, "Habilitar Loop").changed() {
                        if has_loop {
                            entry.loop_inicio_ms = Some(entry.corte_inicial_ms + entry.consoante_ms);
                            entry.loop_fim_ms = Some(entry.corte_inicial_ms + entry.consoante_ms + 200.0);
                        } else {
                            entry.loop_inicio_ms = None;
                            entry.loop_fim_ms = None;
                        }
                    }

                    if has_loop {
                        let l_start = entry.loop_inicio_ms.get_or_insert(100.0);
                        ui.label(RichText::new("Loop Início:").size(10.0).color(Color32::YELLOW));
                        ui.add(egui::Slider::new(l_start, 0.0..=duration_ms).suffix(" ms"));

                        let l_end = entry.loop_fim_ms.get_or_insert(300.0);
                        ui.label(RichText::new("Loop Fim:").size(10.0).color(Color32::YELLOW));
                        ui.add(egui::Slider::new(l_end, 0.0..=duration_ms).suffix(" ms"));
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
                        ui.label(RichText::new("Cauda Final:").size(10.0).color(Color32::from_rgb(180, 50, 255)));
                        ui.add(egui::Slider::new(tail, 0.0..=duration_ms).suffix(" ms"));
                    }
                }
                } else {
                    ui.label(RichText::new("Nenhum alias selecionado").size(11.0).color(Color32::from_rgb(150, 140, 170)));
                }
            });
        });
    });
}
