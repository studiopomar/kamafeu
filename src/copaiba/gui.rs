use crate::audio::AudioPlayer;
use crate::copaiba::{CopaibaConfig, CopaibaEntry};
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Rounding, Sense, Stroke, Vec2};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListFilterMode {
    ByAlias,
    ByWavFile,
    PrefixMap,
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
    pub previous_waveform: Option<(String, Vec<f32>, u32)>,
    pub next_waveform: Option<(String, Vec<f32>, u32)>,
    pub avatar_texture: Option<egui::TextureHandle>,
    pub zoom_x: f32,
    pub zoom_y: f32,
    pub new_mapping_pitch: String,
    pub new_mapping_prefix: String,
    pub new_mapping_suffix: String,
    pub status_message: String,
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
            previous_waveform: None,
            next_waveform: None,
            avatar_texture: None,
            zoom_x: 1.0,
            zoom_y: 1.0,
            new_mapping_pitch: String::new(),
            new_mapping_prefix: String::new(),
            new_mapping_suffix: String::new(),
            status_message: String::new(),
        }
    }
}

impl CopaibaToolkitApp {
    pub fn open_dir(&mut self, path: PathBuf) {
        match CopaibaConfig::load_from_dir(&path) {
            Ok(cfg) => {
                self.current_dir = Some(path);
                self.config = cfg;
                self.avatar_texture = None;
                self.status_message.clear();
                if !self.config.entries.is_empty() {
                    self.selected_entry_index = Some(0);
                } else {
                    self.selected_entry_index = None;
                }
                self.load_selected_wav_samples();
            }
            Err(err) => {
                self.status_message = format!("Erro ao abrir voicebank: {err}");
            }
        }
    }

    pub fn reload_avatar_texture(&mut self, ctx: &egui::Context) {
        if let Some(ref dir) = self.current_dir {
            let mut found_path = None;
            if let Some(ref name) = self.config.image_filename {
                let p = dir.join(name);
                if p.exists() {
                    found_path = Some(p);
                }
            }
            if found_path.is_none() {
                let candidates = [
                    "character.png",
                    "icon.png",
                    "avatar.png",
                    "portrait.png",
                    "character.bmp",
                    "icon.bmp",
                    "avatar.bmp",
                    "portrait.bmp",
                    "character.jpg",
                    "icon.jpg",
                    "avatar.jpg",
                    "portrait.jpg",
                ];
                for c in candidates {
                    let p = dir.join(c);
                    if p.exists() {
                        found_path = Some(p);
                        break;
                    }
                }
            }

            if let Some(path) = found_path {
                if let Ok(img) = image::open(&path) {
                    let rgba = img.to_rgba8();
                    let color_img = egui::ColorImage::from_rgba_unmultiplied(
                        [rgba.width() as usize, rgba.height() as usize],
                        rgba.as_flat_samples().as_slice(),
                    );
                    self.avatar_texture = Some(ctx.load_texture(
                        "copaiba_avatar_preview",
                        color_img,
                        egui::TextureOptions::LINEAR,
                    ));
                    return;
                }
            }
        }
        self.avatar_texture = None;
    }

    pub fn import_avatar_image(
        &mut self,
        ctx: &egui::Context,
        source_path: PathBuf,
    ) -> Result<(), String> {
        if let Some(ref dir) = self.current_dir {
            let img = image::open(&source_path)
                .map_err(|e| format!("Falha ao carregar imagem: {}", e))?;

            let resized = img.resize_exact(100, 100, image::imageops::FilterType::Lanczos3);
            let dest_name = "character.png".to_string();
            let dest_path = dir.join(&dest_name);

            resized
                .save(&dest_path)
                .map_err(|e| format!("Falha ao salvar character.png: {}", e))?;

            self.config.image_filename = Some(dest_name);
            self.save_config()?;
            self.reload_avatar_texture(ctx);
            Ok(())
        } else {
            Err("Nenhum Voicebank aberto".to_string())
        }
    }

    pub fn remove_avatar_image(&mut self, _ctx: &egui::Context) -> Result<(), String> {
        if let Some(dir) = &self.current_dir {
            let fallback_names = [
                "character.png",
                "icon.png",
                "avatar.png",
                "portrait.png",
                "character.bmp",
                "icon.bmp",
                "avatar.bmp",
                "portrait.bmp",
                "character.jpg",
                "icon.jpg",
                "avatar.jpg",
                "portrait.jpg",
            ];
            let image_path = self
                .config
                .image_filename
                .as_deref()
                .map(|name| dir.join(name))
                .filter(|path| path.exists())
                .or_else(|| {
                    fallback_names
                        .iter()
                        .map(|name| dir.join(name))
                        .find(|path| path.exists())
                });

            if let Some(image_path) = image_path {
                let canonical_dir = dir
                    .canonicalize()
                    .map_err(|e| format!("Falha ao validar pasta do voicebank: {e}"))?;
                let canonical_image = image_path
                    .canonicalize()
                    .map_err(|e| format!("Falha ao validar caminho do avatar: {e}"))?;
                if !canonical_image.starts_with(&canonical_dir) {
                    return Err("O avatar está fora da pasta do voicebank".to_string());
                }
                std::fs::remove_file(&canonical_image)
                    .map_err(|e| format!("Falha ao remover avatar: {e}"))?;
            }
        }
        self.config.image_filename = None;
        self.avatar_texture = None;
        self.save_config()?;
        Ok(())
    }

    pub fn selected_entry(&self) -> Option<&CopaibaEntry> {
        self.selected_entry_index
            .and_then(|idx| self.config.entries.get(idx))
    }

    pub fn load_wav_file(&self, filename: &str) -> Option<(Vec<f32>, u32)> {
        self.current_dir.as_ref().and_then(|directory| {
            crate::renderer::TrackRenderer::load_wav_samples(directory.join(filename)).ok()
        })
    }

    pub fn load_selected_wav_samples(&mut self) {
        let target_wav = self.selected_entry().map(|e| e.wav_filename.clone());
        if let (Some(ref dir), Some(ref wav_name)) = (&self.current_dir, &target_wav) {
            if let Ok(waveform) =
                crate::renderer::TrackRenderer::load_wav_samples(dir.join(wav_name))
            {
                self.loaded_waveform = Some(waveform);
                self.loaded_wav_filename = Some(wav_name.clone());
            } else {
                self.loaded_waveform = None;
                self.loaded_wav_filename = None;
            }
        } else {
            self.loaded_waveform = None;
            self.loaded_wav_filename = None;
        }

        self.previous_waveform = self
            .selected_entry_index
            .and_then(|idx| idx.checked_sub(1))
            .and_then(|idx| self.config.entries.get(idx))
            .and_then(|entry| {
                self.load_wav_file(&entry.wav_filename)
                    .map(|(s, sr)| (entry.wav_filename.clone(), s, sr))
            });
        self.next_waveform = self
            .selected_entry_index
            .and_then(|idx| self.config.entries.get(idx + 1))
            .and_then(|entry| {
                self.load_wav_file(&entry.wav_filename)
                    .map(|(s, sr)| (entry.wav_filename.clone(), s, sr))
            });
    }

    pub fn duplicate_selected_entry(&mut self) {
        if let Some(idx) = self.selected_entry_index {
            if let Some(entry) = self.config.entries.get(idx).cloned() {
                let mut dup = entry;
                dup.alias = format!("{}_copia", dup.alias);
                let new_idx = idx + 1;
                self.config.entries.insert(new_idx, dup);
                self.selected_entry_index = Some(new_idx);
                self.load_selected_wav_samples();
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
                self.load_selected_wav_samples();
            }
        }
    }

    pub fn save_config(&mut self) -> Result<(), String> {
        if let Some(ref dir) = self.current_dir {
            self.config.save_to_dir(dir)
        } else {
            Err("Nenhum diretório aberto".to_string())
        }
    }

    pub fn compile_and_package_kfv(&self, dest_path: &std::path::Path) -> Result<(), String> {
        use std::fs;
        let src_dir = self.current_dir.as_ref().ok_or("Nenhum diretório aberto")?;

        self.config.save_to_dir(src_dir)?;

        let file = fs::File::create(dest_path)
            .map_err(|e| format!("Falha ao criar arquivo .kfv: {}", e))?;
        let mut zip = zip::ZipWriter::new(file);

        let options =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        fn walk_rec(
            src_dir: &std::path::Path,
            dir: &std::path::Path,
            zip: &mut zip::ZipWriter<fs::File>,
            options: zip::write::FileOptions,
        ) -> Result<(), String> {
            let read_dir =
                fs::read_dir(dir).map_err(|e| format!("Falha ao ler diretório: {}", e))?;
            for entry in read_dir {
                let entry = entry.map_err(|e| format!("Falha ao ler entrada do diretório: {e}"))?;
                let path = entry.path();
                let name = path
                    .strip_prefix(src_dir)
                    .map_err(|e| format!("Erro no strip_prefix: {}", e))?
                    .to_string_lossy()
                    .to_string();

                let file_type = entry
                    .file_type()
                    .map_err(|e| format!("Falha ao identificar tipo do arquivo: {e}"))?;
                if file_type.is_symlink() {
                    continue;
                } else if file_type.is_dir() {
                    zip.add_directory(&name, options)
                        .map_err(|e| format!("Falha ao adicionar pasta ao zip: {}", e))?;
                    walk_rec(src_dir, &path, zip, options)?;
                } else if file_type.is_file() {
                    if path
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("kfv"))
                    {
                        continue;
                    }
                    zip.start_file(&name, options)
                        .map_err(|e| format!("Falha ao iniciar arquivo no zip: {}", e))?;
                    let mut f = fs::File::open(&path)
                        .map_err(|e| format!("Falha ao abrir arquivo: {}", e))?;
                    std::io::copy(&mut f, zip)
                        .map_err(|e| format!("Falha ao copiar dados para zip: {}", e))?;
                }
            }
            Ok(())
        }

        walk_rec(src_dir, src_dir, &mut zip, options)?;

        zip.finish()
            .map_err(|e| format!("Falha ao finalizar zip: {}", e))?;
        Ok(())
    }
}

fn draw_waveform_peak_envelope(
    painter: &egui::Painter,
    canvas_rect: Rect,
    samples: &[f32],
    color: Color32,
    zoom_y: f32,
    zoom_x: f32,
) {
    let mid_y = canvas_rect.center().y;
    let wave_h = canvas_rect.height();
    let px_width = canvas_rect.width() as usize;
    let num_samples = samples.len();
    if px_width > 0 && num_samples > 0 {
        let visible_samples =
            ((num_samples as f32 / zoom_x.max(1.0)).ceil() as usize).clamp(1, num_samples);
        for px_i in 0..px_width {
            let start_s = px_i * visible_samples / px_width;
            let end_s = (((px_i + 1) * visible_samples + px_width - 1) / px_width)
                .min(visible_samples)
                .max(start_s + 1);
            let chunk = &samples[start_s..end_s];
            let mut min_val = 0.0f32;
            let mut max_val = 0.0f32;
            for &s in chunk {
                if s < min_val {
                    min_val = s;
                }
                if s > max_val {
                    max_val = s;
                }
            }
            let x_pos = canvas_rect.min.x + px_i as f32;
            let y_top = mid_y - (max_val * (wave_h * 0.45) * zoom_y);
            let y_bot = mid_y - (min_val * (wave_h * 0.45) * zoom_y);
            painter.line_segment(
                [Pos2::new(x_pos, y_top), Pos2::new(x_pos, y_bot)],
                Stroke::new(1.0_f32, color),
            );
        }
    }
}

pub fn draw_copaiba_toolkit_ui(app: &mut CopaibaToolkitApp, ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        let mut style = ui.style().as_ref().clone();

        style.visuals.widgets.noninteractive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.inactive.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.hovered.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.active.rounding = egui::Rounding::ZERO;
        style.visuals.widgets.open.rounding = egui::Rounding::ZERO;
        style.visuals.window_rounding = egui::Rounding::ZERO;
        style.visuals.menu_rounding = egui::Rounding::ZERO;

        let border_stroke = Stroke::new(2.0_f32, Color32::BLACK);
        style.visuals.widgets.noninteractive.bg_stroke = border_stroke;
        style.visuals.widgets.inactive.bg_stroke = border_stroke;
        style.visuals.widgets.hovered.bg_stroke = border_stroke;
        style.visuals.widgets.active.bg_stroke = border_stroke;

        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(28, 28, 28);
        style.visuals.widgets.inactive.fg_stroke = Stroke::new(1.5_f32, Color32::from_rgb(240, 240, 240));

        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(180, 255, 0);
        style.visuals.widgets.hovered.fg_stroke = Stroke::new(1.8_f32, Color32::BLACK);

        style.visuals.widgets.active.bg_fill = Color32::from_rgb(0, 230, 255);
        style.visuals.widgets.active.fg_stroke = Stroke::new(1.8_f32, Color32::BLACK);

        ui.set_style(style);


        ui.horizontal(|ui| {
            ui.heading(RichText::new("Copaiba Voicebank Toolkit").strong().color(Color32::from_rgb(180, 255, 0)));
            ui.add_space(15.0);

            if ui.button(RichText::new("Abrir Pasta do Voicebank...").strong().color(Color32::WHITE)).clicked() {
                if let Some(folder) = crate::dialogs::FileDialog::new().pick_folder() {
                    app.open_dir(folder);
                }
            }

            if app.current_dir.is_some() {
                if ui.button(RichText::new("Salvar").strong().color(Color32::WHITE)).clicked() {
                    app.status_message = match app.save_config() {
                        Ok(()) => "Configurações salvas!".to_string(),
                        Err(err) => format!("Erro ao salvar: {err}"),
                    };
                }

                ui.add_space(10.0);
                if ui.button(RichText::new("Compilar e Empacotar (.kfv)").strong().color(Color32::BLACK)).clicked() {
                    let default_name = format!("{}.kfv", app.config.voicebank_name);
                    if let Some(dest_file) = crate::dialogs::FileDialog::new()
                        .set_file_name(&default_name)
                        .add_filter("Kamafeu Voicebank (*.kfv)", &["kfv"])
                        .save_file()
                    {
                        match app.compile_and_package_kfv(&dest_file) {
                            Ok(()) => {
                                app.status_message = format!("Compilado com sucesso: {}", dest_file.file_name().unwrap_or_default().to_string_lossy());
                            }
                            Err(err) => {
                                app.status_message = format!("Erro ao empacotar: {}", err);
                            }
                        }
                    }
                }

                ui.add_space(10.0);
                if ui.button(RichText::new("Duplicar Alias").strong().color(Color32::WHITE)).clicked() {
                    app.duplicate_selected_entry();
                }

                if app.selected_entry_index.is_some()
                    && ui.button(RichText::new("Excluir Alias").strong().color(Color32::from_rgb(255, 100, 100))).clicked() {
                        app.delete_selected_entry();
                    }
            }

            ui.add_space(15.0);
            if let Some(ref dir) = app.current_dir {
                ui.label(RichText::new(format!("Pasta: {}", dir.display())).size(11.0).color(Color32::from_rgb(180, 170, 210)));
            } else {
                ui.label(RichText::new("Nenhuma pasta aberta").size(11.0).color(Color32::from_rgb(150, 140, 170)));
            }

            if !app.status_message.is_empty() {
                ui.add_space(15.0);
                ui.label(RichText::new(&app.status_message).size(11.0).color(Color32::from_rgb(180, 255, 0)));
            }
        });

        ui.add_space(4.0);
        ui.separator();
        ui.add_space(4.0);

        if app.current_dir.is_none() {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    ui.label(RichText::new("Bem-vindo ao Copaiba Voicebank Toolkit").size(18.0).strong().color(Color32::from_rgb(180, 255, 0)));
                    ui.add_space(8.0);
                    ui.label("Abra uma pasta de Voicebank contendo arquivos .wav para configurar seus fonemas e criar seu copaiba.config.");
                    ui.add_space(12.0);
                    if ui.button(RichText::new("Selecionar Pasta do Voicebank").size(14.0)).clicked() {
                        if let Some(folder) = crate::dialogs::FileDialog::new().pick_folder() {
                            app.open_dir(folder);
                        }
                    }
                });
            });
            return;
        }

        egui::Frame::none()
            .fill(Color32::from_rgb(24, 24, 24))
            .rounding(Rounding::ZERO)
            .stroke(Stroke::new(2.5_f32, Color32::BLACK))
            .inner_margin(egui::Margin::same(8.0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (avatar_rect, _) = ui.allocate_exact_size(Vec2::new(100.0, 100.0), Sense::hover());
                    let painter = ui.painter_at(avatar_rect);
                    painter.rect_filled(avatar_rect, Rounding::ZERO, Color32::from_rgb(16, 16, 16));
                    painter.rect_stroke(avatar_rect, Rounding::ZERO, Stroke::new(2.0_f32, Color32::BLACK));


                    if app.avatar_texture.is_none() {
                        app.reload_avatar_texture(ui.ctx());
                    }

                    if let Some(ref texture) = app.avatar_texture {
                        let uv = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0));
                        painter.image(texture.id(), avatar_rect.shrink(2.0), uv, Color32::WHITE);
                    } else {
                        let center = avatar_rect.center();
                        painter.text(
                            Pos2::new(center.x, center.y - 8.0),
                            egui::Align2::CENTER_CENTER,
                            "🖼️",
                            egui::FontId::proportional(22.0),
                            Color32::from_rgb(165, 148, 201),
                        );
                        painter.text(
                            Pos2::new(center.x, center.y + 16.0),
                            egui::Align2::CENTER_CENTER,
                            "Sem Foto",
                            egui::FontId::proportional(10.0),
                            Color32::from_rgb(165, 148, 201),
                        );
                    }

                    ui.add_space(10.0);

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Nome do Voicebank:").size(11.0).strong().color(Color32::from_rgb(0, 255, 157)));
                            if ui.add(egui::TextEdit::singleline(&mut app.config.voicebank_name).desired_width(180.0)).changed() {
                                if let Err(err) = app.save_config() {
                                    app.status_message = format!("Erro ao salvar: {err}");
                                }
                            }

                            ui.add_space(12.0);
                            ui.label(RichText::new("Autor / Criador:").size(11.0).strong().color(Color32::from_rgb(216, 180, 254)));
                            if ui.add(egui::TextEdit::singleline(&mut app.config.author).desired_width(160.0)).changed() {
                                if let Err(err) = app.save_config() {
                                    app.status_message = format!("Erro ao salvar: {err}");
                                }
                            }
                        });

                        ui.add_space(6.0);

                        ui.horizontal(|ui| {
                            let import_btn = egui::Button::new(
                                RichText::new("🖼️ Importar Foto / Avatar (100x100)...")
                                    .size(11.0)
                                    .strong()
                                    .color(Color32::from_rgb(0, 255, 157)),
                            )
                            .fill(Color32::from_rgb(10, 48, 30))
                            .stroke(Stroke::new(1.0_f32, Color32::from_rgb(0, 255, 157)))
                            .rounding(Rounding::same(4.0));

                            if ui.add(import_btn).clicked() {
                                if let Some(file) = crate::dialogs::FileDialog::new()
                                    .add_filter("Imagens (*.png, *.jpg, *.jpeg, *.bmp, *.webp)", &["png", "jpg", "jpeg", "bmp", "webp"])
                                    .pick_file()
                                {
                                    if let Err(err) = app.import_avatar_image(ui.ctx(), file) {
                                        app.status_message = err;
                                    }
                                }
                            }

                            if (app.config.image_filename.is_some() || app.avatar_texture.is_some())
                                && ui.button(RichText::new("🗑️ Remover").size(11.0).color(Color32::from_rgb(255, 100, 100))).clicked() {
                                    app.status_message = match app.remove_avatar_image(ui.ctx()) {
                                        Ok(()) => "Avatar removido".to_string(),
                                        Err(err) => err,
                                    };
                                }

                            if let Some(ref img_name) = app.config.image_filename {
                                ui.label(RichText::new(format!("Arquivo: {}", img_name)).size(10.0).italics().color(Color32::from_rgb(165, 148, 201)));
                            }
                        });
                    });
                });
            });

        ui.add_space(4.0);

        let full_avail_h = ui.available_height();

        ui.horizontal(|ui| {
            ui.allocate_ui_with_layout(Vec2::new(220.0, full_avail_h), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.horizontal(|ui| {
                    if ui.selectable_label(app.filter_mode == ListFilterMode::ByAlias, "Alias").clicked() {
                        app.filter_mode = ListFilterMode::ByAlias;
                    }
                    if ui.selectable_label(app.filter_mode == ListFilterMode::ByWavFile, "WAVs").clicked() {
                        app.filter_mode = ListFilterMode::ByWavFile;
                    }
                    if ui.selectable_label(app.filter_mode == ListFilterMode::PrefixMap, "Tons").clicked() {
                        app.filter_mode = ListFilterMode::PrefixMap;
                    }
                });

                ui.add_space(4.0);
                ui.add(egui::TextEdit::singleline(&mut app.search_query).hint_text("Filtrar alias ou wav...").desired_width(210.0));
                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if ui.button(RichText::new("+ Duplicar").size(10.5).color(Color32::from_rgb(0, 255, 157))).clicked() {
                        app.duplicate_selected_entry();
                    }
                    if ui.button(RichText::new("🗑 Excluir").size(10.5).color(Color32::from_rgb(255, 100, 100))).clicked() {
                        app.delete_selected_entry();
                    }
                });

                ui.add_space(4.0);
                ui.separator();

                let mut new_selection = app.selected_entry_index;

                egui::ScrollArea::vertical()
                    .id_salt("copaiba_alias_scroll_list")
                    .max_height(ui.available_height())
                    .show(ui, |ui| {
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
                                if app.filter_mode == ListFilterMode::PrefixMap {
                                    app.filter_mode = ListFilterMode::ByAlias;
                                }
                            }
                        }
                    });

                if new_selection != app.selected_entry_index {
                    app.selected_entry_index = new_selection;
                    app.load_selected_wav_samples();
                }
            });

            ui.separator();

            if app.filter_mode == ListFilterMode::PrefixMap {
                ui.allocate_ui_with_layout(Vec2::new(ui.available_width(), full_avail_h), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    ui.vertical(|ui| {
                        ui.heading(RichText::new("Mapeamento Multitom (Prefix Map / Subbanks)").strong().color(Color32::from_rgb(180, 255, 0)));
                        ui.label("Configure prefixos e sufixos para tons específicos (subbanks) do seu voicebank.");
                        ui.add_space(10.0);

                        egui::Frame::none()
                            .fill(Color32::from_rgb(24, 24, 24))
                            .rounding(Rounding::ZERO)
                            .stroke(Stroke::new(2.5_f32, Color32::BLACK))
                            .inner_margin(egui::Margin::same(8.0))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.label("Tom (ex: C4):");
                                    ui.add(egui::TextEdit::singleline(&mut app.new_mapping_pitch).desired_width(60.0));
                                    ui.label("Prefixo:");
                                    ui.add(egui::TextEdit::singleline(&mut app.new_mapping_prefix).desired_width(80.0));
                                    ui.label("Sufixo:");
                                    ui.add(egui::TextEdit::singleline(&mut app.new_mapping_suffix).desired_width(80.0));

                                    let add_btn = egui::Button::new(RichText::new("➕ Adicionar").strong().color(Color32::BLACK))
                                        .fill(Color32::from_rgb(180, 255, 0))
                                        .stroke(Stroke::new(2.0_f32, Color32::BLACK));

                                    if ui.add(add_btn).clicked() {

                                        let pitch = app.new_mapping_pitch.trim().to_string();
                                        if !pitch.is_empty() {
                                            app.config.prefix_map.insert(
                                                pitch,
                                                (app.new_mapping_prefix.clone(), app.new_mapping_suffix.clone()),
                                            );
                                            app.new_mapping_pitch.clear();
                                            app.new_mapping_prefix.clear();
                                            app.new_mapping_suffix.clear();
                                            if let Err(err) = app.save_config() {
                                                app.status_message = format!("Erro ao salvar: {err}");
                                            }
                                        }
                                    }

                                    ui.add_space(10.0);
                                    if ui.button("Gerar Oitavas Padrão").clicked() {
                                        let octaves = vec!["C3", "F3", "C4", "F4", "C5", "F5"];
                                        for oct in octaves {
                                            app.config.prefix_map.insert(
                                                oct.to_string(),
                                                (String::new(), format!("_{}", oct)),
                                            );
                                        }
                                        if let Err(err) = app.save_config() {
                                            app.status_message = format!("Erro ao salvar: {err}");
                                        }
                                    }
                                });
                            });

                        ui.add_space(15.0);

                        egui::ScrollArea::vertical()
                            .id_salt("prefix_map_rules_scroll")
                            .show(ui, |ui| {
                                egui::Grid::new("prefix_map_grid")
                                    .striped(true)
                                    .spacing([20.0, 10.0])
                                    .show(ui, |ui| {
                                        ui.label(RichText::new("Tom").strong().color(Color32::from_rgb(255, 215, 0)));
                                        ui.label(RichText::new("Prefixo").strong().color(Color32::from_rgb(216, 180, 254)));
                                        ui.label(RichText::new("Sufixo").strong().color(Color32::from_rgb(216, 180, 254)));
                                        ui.label(RichText::new("Ações").strong().color(Color32::WHITE));
                                        ui.end_row();

                                        let mut to_remove = None;
                                        let mut keys: Vec<String> = app.config.prefix_map.keys().cloned().collect();
                                        keys.sort();

                                        let mut changed = false;
                                        for key in keys {
                                            let mut entry_removed = false;
                                            if let Some((pref, suff)) = app.config.prefix_map.get_mut(&key) {
                                                ui.label(RichText::new(&key).strong().color(Color32::from_rgb(0, 255, 157)));

                                                if ui.add(egui::TextEdit::singleline(pref).desired_width(100.0)).changed() {
                                                    changed = true;
                                                }
                                                if ui.add(egui::TextEdit::singleline(suff).desired_width(100.0)).changed() {
                                                    changed = true;
                                                }

                                                if ui.button(RichText::new("🗑 Excluir").color(Color32::from_rgb(255, 100, 100))).clicked() {
                                                    to_remove = Some(key.clone());
                                                    entry_removed = true;
                                                }
                                                ui.end_row();
                                            }
                                            if entry_removed {
                                                break;
                                            }
                                        }

                                        if let Some(rem_key) = to_remove {
                                            app.config.prefix_map.remove(&rem_key);
                                            changed = true;
                                        }

                                        if changed {
                                            if let Err(err) = app.save_config() {
                                                app.status_message = format!("Erro ao salvar: {err}");
                                            }
                                        }
                                    });
                            });
                    });
                });
            } else {
                let center_w = (ui.available_width() - 270.0).max(300.0);
                ui.allocate_ui_with_layout(Vec2::new(center_w, full_avail_h), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                    let current_idx = app.selected_entry_index.unwrap_or(0);
                    let prev_entry = if current_idx > 0 { app.config.entries.get(current_idx - 1).cloned() } else { None };

                let next_entry = app.config.entries.get(current_idx + 1).cloned();

                ui.horizontal(|ui| {
                    ui.label(RichText::new("Editor de Forma de Onda (Triplo Stack)").strong().size(12.0).color(Color32::from_rgb(0, 255, 157)));
                    ui.add_space(10.0);

                    if let Some((ref samples, sr)) = app.loaded_waveform {
                        if ui.button("▶ Tocar").clicked() {
                            app.audio_player.play_samples(samples.clone(), sr);
                        }
                    }

                    ui.add_space(10.0);
                    ui.label(RichText::new("Zoom X:").size(10.0).color(Color32::from_rgb(180, 170, 190)));
                    ui.add(egui::Slider::new(&mut app.zoom_x, 1.0..=5.0).show_value(false));
                    ui.label(RichText::new("Zoom Y:").size(10.0).color(Color32::from_rgb(180, 170, 190)));
                    ui.add(egui::Slider::new(&mut app.zoom_y, 0.5..=4.0).show_value(false));
                });

                ui.add_space(4.0);

                let available_center_h = ui.available_height() - 10.0;
                let prev_next_h = (available_center_h * 0.18).clamp(55.0, 90.0);
                let main_h = available_center_h - (prev_next_h * 2.0) - 12.0;

                ui.horizontal(|ui| {
                    if let Some(ref p_entry) = prev_entry {
                        ui.label(RichText::new(format!("▲ Anterior: {} ({})", p_entry.alias, p_entry.wav_filename)).size(10.0).color(Color32::from_rgb(150, 140, 170)));
                    } else {
                        ui.label(RichText::new("▲ (Sem alias anterior)").size(10.0).color(Color32::from_rgb(90, 80, 110)));
                    }
                });

                let (prev_rect, prev_resp) = ui.allocate_exact_size(Vec2::new(center_w, prev_next_h), Sense::click());
                let prev_painter = ui.painter_at(prev_rect);
                prev_painter.rect_filled(prev_rect, Rounding::same(4.0), Color32::from_rgb(14, 11, 22));
                prev_painter.rect_stroke(prev_rect, Rounding::same(4.0), Stroke::new(1.0_f32, Color32::from_rgb(35, 28, 48)));

                if let Some(ref p_entry) = prev_entry {
                    if prev_resp.clicked() {
                        app.selected_entry_index = Some(current_idx - 1);
                        app.load_selected_wav_samples();
                    }
                    if let Some((cached_name, samples, _)) = &app.previous_waveform {
                        if cached_name == &p_entry.wav_filename {
                            draw_waveform_peak_envelope(&prev_painter, prev_rect, samples, Color32::from_rgb(0, 150, 100), app.zoom_y * 0.8, 1.0);
                        }
                    }
                }

                ui.add_space(4.0);

                let (canvas_rect, canvas_resp) = ui.allocate_exact_size(Vec2::new(center_w, main_h.max(120.0)), Sense::click_and_drag());
                let painter = ui.painter_at(canvas_rect);

                painter.rect_filled(canvas_rect, Rounding::ZERO, Color32::from_rgb(16, 16, 16));
                painter.rect_stroke(canvas_rect, Rounding::ZERO, Stroke::new(2.5_f32, Color32::BLACK));


                let zoom_y = app.zoom_y;
                let sel_idx = app.selected_entry_index;

                if let (Some((ref samples, sr)), Some(idx)) = (&app.loaded_waveform, sel_idx) {
                    if idx < app.config.entries.len() {
                        let entry = &mut app.config.entries[idx];
                        let total_duration_ms = (samples.len() as f64 / *sr as f64) * 1000.0;

                        let visible_duration_ms = total_duration_ms / app.zoom_x.max(1.0) as f64;
                        let ms_to_x = |ms: f64| -> f32 {
                            let ratio = (ms / visible_duration_ms).clamp(0.0, 1.0) as f32;
                            canvas_rect.min.x + ratio * canvas_rect.width()
                        };

                        let x_to_ms = |x: f32| -> f64 {
                            let ratio = ((x - canvas_rect.min.x) / canvas_rect.width()).clamp(0.0, 1.0) as f64;
                            ratio * visible_duration_ms
                        };

                        let ruler_h = 16.0f32;
                        let ruler_rect = Rect::from_min_size(canvas_rect.min, Vec2::new(canvas_rect.width(), ruler_h));
                        painter.rect_filled(ruler_rect, Rounding::same(2.0), Color32::from_rgb(22, 18, 34));
                        painter.line_segment([Pos2::new(ruler_rect.min.x, ruler_rect.max.y), Pos2::new(ruler_rect.max.x, ruler_rect.max.y)], Stroke::new(1.0_f32, Color32::from_rgb(60, 50, 80)));

                        let step_ms = if total_duration_ms > 2000.0 { 200.0 } else { 50.0 };
                        let mut ms_tick = 0.0f64;
                        while ms_tick <= visible_duration_ms {
                            let tick_x = ms_to_x(ms_tick);
                            if tick_x >= canvas_rect.min.x && tick_x <= canvas_rect.max.x {
                                painter.line_segment(
                                    [Pos2::new(tick_x, ruler_rect.min.y + 8.0), Pos2::new(tick_x, canvas_rect.max.y)],
                                    Stroke::new(1.0_f32, Color32::from_rgba_premultiplied(80, 70, 110, 100)),
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

                        let wave_area_min_y = canvas_rect.min.y + ruler_h;

                        let x_offset = ms_to_x(entry.corte_inicial_ms);
                        let x_consonant = ms_to_x(entry.corte_inicial_ms + entry.consoante_ms);
                        let cutoff_marker_ms = if entry.corte_final_ms > 0.0 {
                            total_duration_ms - entry.corte_final_ms
                        } else if entry.corte_final_ms < 0.0 {
                            entry.corte_inicial_ms - entry.corte_final_ms
                        } else {
                            total_duration_ms
                        };
                        let x_cutoff = ms_to_x(cutoff_marker_ms);
                        let x_tail = entry.cauda_final_ms.map(&ms_to_x);
                        let x_preutterance = ms_to_x(entry.corte_inicial_ms + entry.preutterance_ms);
                        let x_overlap = ms_to_x(entry.corte_inicial_ms + entry.overlap_ms);


                        let left_cut_rect = Rect::from_min_max(Pos2::new(canvas_rect.min.x, wave_area_min_y), Pos2::new(x_offset, canvas_rect.max.y));
                        painter.rect_filled(left_cut_rect, Rounding::ZERO, Color32::from_rgba_premultiplied(0, 60, 150, 45));

                        let cons_rect = Rect::from_min_max(Pos2::new(x_offset, wave_area_min_y), Pos2::new(x_consonant, canvas_rect.max.y));
                        painter.rect_filled(cons_rect, Rounding::ZERO, Color32::from_rgba_premultiplied(0, 150, 80, 25));

                        let right_cut_rect = Rect::from_min_max(Pos2::new(x_cutoff, wave_area_min_y), canvas_rect.max);
                        painter.rect_filled(right_cut_rect, Rounding::ZERO, Color32::from_rgba_premultiplied(150, 40, 40, 45));

                        if let (Some(l_start), Some(l_end)) = (entry.loop_inicio_ms, entry.loop_fim_ms) {
                            let x_lstart = ms_to_x(l_start);
                            let x_lend = ms_to_x(l_end);
                            let loop_rect = Rect::from_min_max(Pos2::new(x_lstart, wave_area_min_y), Pos2::new(x_lend, canvas_rect.max.y));
                            painter.rect_filled(loop_rect, Rounding::ZERO, Color32::from_rgba_premultiplied(160, 140, 0, 25));
                            painter.line_segment([Pos2::new(x_lstart, wave_area_min_y), Pos2::new(x_lstart, canvas_rect.max.y)], Stroke::new(1.5_f32, Color32::from_rgb(220, 200, 0)));
                            painter.line_segment([Pos2::new(x_lend, wave_area_min_y), Pos2::new(x_lend, canvas_rect.max.y)], Stroke::new(1.5_f32, Color32::from_rgb(220, 200, 0)));
                        }

                        if let Some(xt) = x_tail {
                            let tail_rect = Rect::from_min_max(Pos2::new(xt, wave_area_min_y), Pos2::new((xt + 40.0).min(canvas_rect.max.x), canvas_rect.max.y));
                            painter.rect_filled(tail_rect, Rounding::ZERO, Color32::from_rgba_premultiplied(120, 30, 180, 25));
                            painter.line_segment([Pos2::new(xt, wave_area_min_y), Pos2::new(xt, canvas_rect.max.y)], Stroke::new(1.5_f32, Color32::from_rgb(160, 50, 220)));
                            painter.text(Pos2::new(xt + 3.0, wave_area_min_y + 4.0), egui::Align2::LEFT_TOP, "Cauda Final", egui::FontId::proportional(10.0), Color32::from_rgb(180, 60, 240));
                        }

                        draw_waveform_peak_envelope(&painter, Rect::from_min_max(Pos2::new(canvas_rect.min.x, wave_area_min_y), canvas_rect.max), samples, Color32::from_rgb(0, 255, 160), zoom_y, app.zoom_x);


                        painter.line_segment([Pos2::new(x_offset, wave_area_min_y), Pos2::new(x_offset, canvas_rect.max.y)], Stroke::new(2.5_f32, Color32::from_rgb(0, 150, 255)));
                        painter.text(Pos2::new(x_offset + 3.0, wave_area_min_y + 4.0), egui::Align2::LEFT_TOP, "Corte Inicial", egui::FontId::proportional(10.0), Color32::from_rgb(0, 180, 255));

                        painter.line_segment([Pos2::new(x_consonant, wave_area_min_y), Pos2::new(x_consonant, canvas_rect.max.y)], Stroke::new(2.5_f32, Color32::from_rgb(0, 255, 120)));
                        painter.text(Pos2::new(x_consonant + 3.0, wave_area_min_y + 4.0), egui::Align2::LEFT_TOP, "Início Consoante", egui::FontId::proportional(10.0), Color32::from_rgb(0, 255, 120));

                        painter.line_segment([Pos2::new(x_cutoff, wave_area_min_y), Pos2::new(x_cutoff, canvas_rect.max.y)], Stroke::new(2.5_f32, Color32::from_rgb(255, 60, 60)));
                        painter.text(Pos2::new(x_cutoff + 3.0, wave_area_min_y + 4.0), egui::Align2::LEFT_TOP, "Corte Final", egui::FontId::proportional(10.0), Color32::from_rgb(255, 80, 80));

                        painter.line_segment([Pos2::new(x_preutterance, wave_area_min_y), Pos2::new(x_preutterance, canvas_rect.max.y)], Stroke::new(2.5_f32, Color32::from_rgb(255, 0, 180)));
                        painter.text(Pos2::new(x_preutterance + 3.0, wave_area_min_y + 16.0), egui::Align2::LEFT_TOP, "Pre-utterance", egui::FontId::proportional(10.0), Color32::from_rgb(255, 0, 180));

                        painter.line_segment([Pos2::new(x_overlap, wave_area_min_y), Pos2::new(x_overlap, canvas_rect.max.y)], Stroke::new(2.5_f32, Color32::from_rgb(255, 140, 0)));
                        painter.text(Pos2::new(x_overlap + 3.0, wave_area_min_y + 28.0), egui::Align2::LEFT_TOP, "Overlap", egui::FontId::proportional(10.0), Color32::from_rgb(255, 140, 0));

                        let is_typing = ui.ctx().wants_keyboard_input();
                        if !is_typing {
                            if let Some(hover_pos) = ui.input(|i| i.pointer.hover_pos()) {
                                if canvas_rect.contains(hover_pos) {
                                    let hovered_ms = x_to_ms(hover_pos.x);
                                    ui.input(|i| {
                                        if i.key_down(egui::Key::Q) {
                                            entry.corte_inicial_ms = hovered_ms.clamp(0.0, total_duration_ms);
                                        } else if i.key_down(egui::Key::W) {
                                            entry.consoante_ms = (hovered_ms - entry.corte_inicial_ms).max(1.0);
                                        } else if i.key_down(egui::Key::E) {
                                            entry.corte_final_ms = if entry.corte_final_ms < 0.0 {
                                                -(hovered_ms - entry.corte_inicial_ms).max(0.0)
                                            } else {
                                                (total_duration_ms - hovered_ms).max(0.0)
                                            };
                                        } else if i.key_down(egui::Key::R) {
                                            entry.preutterance_ms = (hovered_ms - entry.corte_inicial_ms).max(0.0);
                                        } else if i.key_down(egui::Key::T) {
                                            entry.overlap_ms = (hovered_ms - entry.corte_inicial_ms).max(0.0);
                                        } else if i.key_down(egui::Key::Y) {
                                            if entry.loop_inicio_ms.is_some() {
                                                entry.loop_inicio_ms = Some(hovered_ms.clamp(0.0, total_duration_ms));
                                            } else if entry.cauda_final_ms.is_some() {
                                                entry.cauda_final_ms = Some(hovered_ms.clamp(0.0, total_duration_ms));
                                            }
                                        } else if i.key_down(egui::Key::U)
                                            && entry.loop_fim_ms.is_some()
                                        {
                                            entry.loop_fim_ms =
                                                Some(hovered_ms.clamp(0.0, total_duration_ms));
                                        }
                                    });
                                }
                            }
                        }

                        if let Some(mpos) = canvas_resp.interact_pointer_pos() {
                            if canvas_resp.dragged() || canvas_resp.clicked() {
                                let clicked_ms = x_to_ms(mpos.x);

                                let dist_offset = (mpos.x - x_offset).abs();
                                let dist_cons = (mpos.x - x_consonant).abs();
                                let dist_cutoff = (mpos.x - x_cutoff).abs();
                                let dist_pre = (mpos.x - x_preutterance).abs();
                                let dist_over = (mpos.x - x_overlap).abs();
                                let dist_tail = x_tail.map(|xt| (mpos.x - xt).abs()).unwrap_or(f32::MAX);

                                let min_dist = dist_offset.min(dist_cons).min(dist_cutoff).min(dist_pre).min(dist_over).min(dist_tail);

                                if min_dist == dist_tail && entry.cauda_final_ms.is_some() {
                                    entry.cauda_final_ms = Some(clicked_ms.clamp(0.0, total_duration_ms));
                                } else if min_dist == dist_offset {
                                    entry.corte_inicial_ms = clicked_ms.clamp(0.0, total_duration_ms);
                                } else if min_dist == dist_cons {
                                    entry.consoante_ms = (clicked_ms - entry.corte_inicial_ms).max(1.0);
                                } else if min_dist == dist_cutoff {
                                    entry.corte_final_ms = if entry.corte_final_ms < 0.0 {
                                        -(clicked_ms - entry.corte_inicial_ms).max(0.0)
                                    } else {
                                        (total_duration_ms - clicked_ms).max(0.0)
                                    };
                                } else if min_dist == dist_pre {
                                    entry.preutterance_ms = (clicked_ms - entry.corte_inicial_ms).max(0.0);
                                } else {
                                    entry.overlap_ms = (clicked_ms - entry.corte_inicial_ms).max(0.0);
                                }
                            }
                        }

                    }
                }

                ui.add_space(4.0);

                ui.horizontal(|ui| {
                    if let Some(ref n_entry) = next_entry {
                        ui.label(RichText::new(format!("▼ Próxima: {} ({})", n_entry.alias, n_entry.wav_filename)).size(10.0).color(Color32::from_rgb(150, 140, 170)));
                    } else {
                        ui.label(RichText::new("▼ (Sem próxima alias)").size(10.0).color(Color32::from_rgb(90, 80, 110)));
                    }
                });

                let (next_rect, next_resp) = ui.allocate_exact_size(Vec2::new(center_w, prev_next_h), Sense::click());
                let next_painter = ui.painter_at(next_rect);
                next_painter.rect_filled(next_rect, Rounding::same(4.0), Color32::from_rgb(14, 11, 22));
                next_painter.rect_stroke(next_rect, Rounding::same(4.0), Stroke::new(1.0_f32, Color32::from_rgb(35, 28, 48)));

                if let Some(ref n_entry) = next_entry {
                    if next_resp.clicked() {
                        app.selected_entry_index = Some(current_idx + 1);
                        app.load_selected_wav_samples();
                    }
                    if let Some((cached_name, samples, _)) = &app.next_waveform {
                        if cached_name == &n_entry.wav_filename {
                            draw_waveform_peak_envelope(&next_painter, next_rect, samples, Color32::from_rgb(0, 150, 100), app.zoom_y * 0.8, 1.0);
                        }
                    }
                }
            });

            ui.separator();

            ui.allocate_ui_with_layout(Vec2::new(260.0, full_avail_h), egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.label(RichText::new("Controles & Sliders").strong().size(12.0).color(Color32::from_rgb(0, 255, 157)));
                ui.add_space(6.0);

                egui::ScrollArea::vertical()
                    .id_salt("copaiba_right_controls_scroll")
                    .max_height(ui.available_height())
                    .show(ui, |ui| {
                        let sel_idx = app.selected_entry_index;
                        let wav_duration_ms = app.loaded_waveform.as_ref().map(|(s, sr)| (s.len() as f64 / *sr as f64) * 1000.0);

                        if let (Some(idx), Some(duration_ms)) = (sel_idx, wav_duration_ms) {
                            if idx < app.config.entries.len() {
                                let entry = &mut app.config.entries[idx];

                                ui.label(RichText::new("Nome do Alias:").size(11.0).color(Color32::from_rgb(200, 190, 230)));
                                ui.add(egui::TextEdit::singleline(&mut entry.alias).desired_width(240.0));

                                ui.add_space(4.0);
                                ui.label(RichText::new("Arquivo WAV:").size(10.0).color(Color32::from_rgb(160, 150, 180)));
                                ui.label(RichText::new(&entry.wav_filename).size(11.0).color(Color32::WHITE));

                                ui.add_space(8.0);
                                ui.separator();
                                ui.add_space(6.0);

                                ui.label(RichText::new("Corte Inicial (ms):").size(11.0).color(Color32::from_rgb(0, 150, 255)));
                                ui.add(egui::Slider::new(&mut entry.corte_inicial_ms, 0.0..=duration_ms).suffix(" ms"));

                                ui.add_space(8.0);
                                ui.label(RichText::new("Início Consoante (ms):").size(11.0).color(Color32::from_rgb(0, 255, 120)));
                                ui.add(egui::Slider::new(&mut entry.consoante_ms, 0.0..=duration_ms).suffix(" ms"));

                                ui.add_space(8.0);
                                ui.label(RichText::new("Corte Final (ms):").size(11.0).color(Color32::from_rgb(255, 60, 60)));
                                ui.add(egui::Slider::new(&mut entry.corte_final_ms, -duration_ms..=duration_ms).suffix(" ms"));

                                ui.add_space(8.0);
                                ui.label(RichText::new("Pre-utterance (ms):").size(11.0).color(Color32::from_rgb(255, 0, 180)));
                                ui.add(egui::Slider::new(&mut entry.preutterance_ms, 0.0..=duration_ms).suffix(" ms"));

                                ui.add_space(8.0);
                                ui.label(RichText::new("Overlap (ms):").size(11.0).color(Color32::from_rgb(255, 140, 0)));
                                ui.add(egui::Slider::new(&mut entry.overlap_ms, 0.0..=duration_ms).suffix(" ms"));

                                ui.add_space(10.0);

                                ui.separator();
                                ui.add_space(6.0);

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
        }
    });
});
}
