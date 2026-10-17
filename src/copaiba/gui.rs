use crate::audio::AudioPlayer;
use crate::copaiba::{CopaibaConfig, CopaibaEntry};
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Rounding, Sense, Stroke, Vec2};
use std::path::PathBuf;

mod visual_tokens {
    use eframe::egui::Color32;

    pub const BACKGROUND: Color32 = Color32::from_rgb(19, 19, 22);
    pub const PANEL: Color32 = Color32::from_rgb(27, 27, 31);
    pub const SURFACE: Color32 = Color32::from_rgb(36, 35, 40);
    pub const SURFACE_HOVER: Color32 = Color32::from_rgb(47, 47, 52);
    pub const BORDER: Color32 = Color32::from_rgb(67, 65, 72);
    pub const TEXT: Color32 = Color32::from_rgb(242, 239, 233);
    pub const MUTED: Color32 = Color32::from_rgb(174, 170, 178);
    pub const ACCENT: Color32 = Color32::from_rgb(195, 232, 120);
    pub const ACCENT_HOVER: Color32 = Color32::from_rgb(211, 244, 151);
    pub const ACCENT_DARK: Color32 = Color32::from_rgb(33, 42, 24);
    pub const SUCCESS: Color32 = Color32::from_rgb(133, 211, 166);
    pub const WARNING: Color32 = Color32::from_rgb(242, 194, 117);
    pub const ERROR: Color32 = Color32::from_rgb(245, 130, 124);
}

pub fn configure_copaiba_theme(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    let mut visuals = egui::Visuals::dark();
    let border = Stroke::new(1.0_f32, visual_tokens::BORDER);
    let rounding = Rounding::same(8.0);

    visuals.panel_fill = visual_tokens::BACKGROUND;
    visuals.window_fill = visual_tokens::PANEL;
    visuals.override_text_color = Some(visual_tokens::TEXT);
    visuals.hyperlink_color = visual_tokens::ACCENT;
    visuals.faint_bg_color = visual_tokens::SURFACE;
    visuals.extreme_bg_color = visual_tokens::BACKGROUND;
    visuals.code_bg_color = visual_tokens::SURFACE;
    visuals.warn_fg_color = visual_tokens::WARNING;
    visuals.error_fg_color = visual_tokens::ERROR;
    visuals.window_rounding = rounding;
    visuals.window_stroke = border;
    visuals.menu_rounding = rounding;
    visuals.button_frame = true;
    visuals.collapsing_header_frame = true;
    visuals.striped = true;
    visuals.selection.bg_fill = visual_tokens::ACCENT_DARK;
    visuals.selection.stroke = Stroke::new(1.0_f32, visual_tokens::ACCENT);

    let widgets = &mut visuals.widgets;
    widgets.noninteractive.bg_fill = visual_tokens::PANEL;
    widgets.noninteractive.weak_bg_fill = visual_tokens::PANEL;
    widgets.noninteractive.bg_stroke = border;
    widgets.noninteractive.rounding = rounding;
    widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, visual_tokens::TEXT);

    widgets.inactive.bg_fill = visual_tokens::SURFACE;
    widgets.inactive.weak_bg_fill = visual_tokens::SURFACE;
    widgets.inactive.bg_stroke = border;
    widgets.inactive.rounding = rounding;
    widgets.inactive.fg_stroke = Stroke::new(1.0_f32, visual_tokens::TEXT);

    widgets.hovered.bg_fill = visual_tokens::SURFACE_HOVER;
    widgets.hovered.weak_bg_fill = visual_tokens::SURFACE_HOVER;
    widgets.hovered.bg_stroke = Stroke::new(1.0_f32, visual_tokens::ACCENT);
    widgets.hovered.rounding = rounding;
    widgets.hovered.fg_stroke = Stroke::new(1.0_f32, visual_tokens::TEXT);

    widgets.active.bg_fill = visual_tokens::ACCENT;
    widgets.active.weak_bg_fill = visual_tokens::ACCENT;
    widgets.active.bg_stroke = Stroke::new(1.0_f32, visual_tokens::ACCENT_HOVER);
    widgets.active.rounding = rounding;
    widgets.active.fg_stroke = Stroke::new(1.0_f32, visual_tokens::BACKGROUND);

    widgets.open.bg_fill = visual_tokens::ACCENT_DARK;
    widgets.open.weak_bg_fill = visual_tokens::ACCENT_DARK;
    widgets.open.bg_stroke = Stroke::new(1.0_f32, visual_tokens::ACCENT);
    widgets.open.rounding = rounding;
    widgets.open.fg_stroke = Stroke::new(1.0_f32, visual_tokens::TEXT);

    style.visuals = visuals;
    style.spacing.item_spacing = Vec2::new(10.0, 10.0);
    style.spacing.button_padding = Vec2::new(12.0, 8.0);
    style.spacing.interact_size = Vec2::new(40.0, 34.0);
    style.spacing.window_margin = egui::Margin::same(16.0);
    style.spacing.menu_margin = egui::Margin::same(10.0);
    style.spacing.slider_width = 150.0;
    ctx.set_style(style);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListFilterMode {
    ByAlias,
    ByWavFile,
    PrefixMap,
}

#[derive(Clone)]
struct PendingClassicMigration {
    profile_path: PathBuf,
    mapping_path: Option<PathBuf>,
    destination_parent: PathBuf,
    preview: crate::voicebank::migration::LegacyMigrationPreview,
}

pub struct CopaibaToolkitApp {
    pub current_dir: Option<PathBuf>,
    pub active_oto_path: Option<PathBuf>,
    pub selected_source_alias: Option<String>,
    pub preserve_oto_entry: bool,
    pub config: CopaibaConfig,
    pub native_project: Option<crate::copaiba::native_project::CopaibaNativeProject>,
    pub native_selected_layer: Option<String>,
    pub native_selected_unit: Option<String>,
    pub native_selected_asset: Option<String>,
    pub native_compare_asset: Option<String>,
    pub native_comparison_a: Option<(Vec<f32>, u32)>,
    pub native_comparison_b: Option<(Vec<f32>, u32)>,
    pub native_new_unit_id: String,
    pub native_new_kind: String,
    pub native_new_phones: String,
    pub native_test_phones: String,
    pub native_phrase_report: Option<String>,
    pub native_create_wizard_open: bool,
    pub native_draft_display_name: String,
    pub native_draft_voice_id: String,
    pub native_draft_bank_id: String,
    pub native_draft_publisher: String,
    pub selected_entry_index: Option<usize>,
    pub search_query: String,
    pub filter_mode: ListFilterMode,
    pub loaded_waveform: Option<(Vec<f32>, u32)>, // samples, sample_rate
    pub loaded_wav_filename: Option<String>,
    pub venus_analysis: Option<crate::dsp::venus_analysis::VenusAnalysis>,
    pub venus_analysis_path: Option<PathBuf>,
    pub previous_waveform: Option<(String, Vec<f32>, u32)>,
    pub next_waveform: Option<(String, Vec<f32>, u32)>,
    pub avatar_texture: Option<egui::TextureHandle>,
    pub zoom_x: f32,
    pub zoom_y: f32,
    pub new_mapping_pitch: String,
    pub new_mapping_prefix: String,
    pub new_mapping_suffix: String,
    pub status_message: String,
    pending_classic_migration: Option<PendingClassicMigration>,
    migration_report: Option<crate::voicebank::migration::MigrationReport>,
}

impl Default for CopaibaToolkitApp {
    fn default() -> Self {
        Self {
            current_dir: None,
            active_oto_path: None,
            selected_source_alias: None,
            preserve_oto_entry: false,
            config: CopaibaConfig::default(),
            native_project: None,
            native_selected_layer: None,
            native_selected_unit: None,
            native_selected_asset: None,
            native_compare_asset: None,
            native_comparison_a: None,
            native_comparison_b: None,
            native_new_unit_id: "unit-001".into(),
            native_new_kind: "V".into(),
            native_new_phones: String::new(),
            native_test_phones: "k a".into(),
            native_phrase_report: None,
            native_create_wizard_open: false,
            native_draft_display_name: "Nova voz Kamafeu".into(),
            native_draft_voice_id: "org.kamafeu.voice.nova".into(),
            native_draft_bank_id: "org.kamafeu.voice.nova.natural".into(),
            native_draft_publisher: "Autor não definido".into(),
            selected_entry_index: None,
            search_query: String::new(),
            filter_mode: ListFilterMode::ByAlias,
            loaded_waveform: None,
            loaded_wav_filename: None,
            venus_analysis: None,
            venus_analysis_path: None,
            previous_waveform: None,
            next_waveform: None,
            avatar_texture: None,
            zoom_x: 1.0,
            zoom_y: 1.0,
            new_mapping_pitch: String::new(),
            new_mapping_prefix: String::new(),
            new_mapping_suffix: String::new(),
            status_message: String::new(),
            pending_classic_migration: None,
            migration_report: None,
        }
    }
}

impl CopaibaToolkitApp {
    /// Opens a voicebank and selects the exact alias requested by the editor.
    /// This is used by the Kamafeu phoneme ruler so manual oto.ini fixes start on
    /// the offending sample instead of making the user search the bank again.
    pub fn open_alias(&mut self, path: PathBuf, alias: &str) {
        let oto_path = crate::copaiba_bridge::oto_path_for_alias(&path, alias).ok();
        let load_dir = oto_path
            .as_ref()
            .and_then(|oto| oto.parent().map(PathBuf::from))
            .unwrap_or(path);
        self.open_dir(load_dir);
        self.active_oto_path = oto_path;
        self.selected_source_alias = Some(alias.trim().to_string());
        self.preserve_oto_entry = self.active_oto_path.is_some();
        let wanted = alias.trim();
        if let Some(index) = self
            .config
            .entries
            .iter()
            .position(|entry| entry.alias.trim().eq_ignore_ascii_case(wanted))
        {
            self.selected_entry_index = Some(index);
            self.search_query.clear();
            self.filter_mode = ListFilterMode::ByAlias;
            self.load_selected_wav_samples();
            self.status_message = format!("Alias selecionado: {wanted}");
        } else {
            self.search_query = wanted.to_string();
            self.status_message = format!(
                "Alias '{wanted}' não foi encontrado; verifique o banco e crie ou ajuste a entrada."
            );
        }
    }

    pub fn open_dir(&mut self, path: PathBuf) {
        self.pending_classic_migration = None;
        self.migration_report = None;
        if path.is_file()
            && path
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("kfv"))
        {
            match crate::copaiba::native_project::CopaibaNativeProject::load_path(&path) {
                Ok(project) => self.activate_native_project(project),
                Err(error) => {
                    self.status_message = format!("Erro ao abrir pacote KFV 2 no Copaíba: {error}")
                }
            }
            return;
        }
        let manifest_path = path.join("kamafeu_voicebank.json");
        if manifest_path.is_file() {
            match std::fs::read_to_string(&manifest_path)
                .map_err(|e| format!("cannot read KFV manifest: {e}"))
                .and_then(|text| {
                    serde_json::from_str::<serde_json::Value>(&text)
                        .map_err(|e| format!("invalid KFV manifest JSON: {e}"))
                }) {
                Ok(value)
                    if value.get("format").and_then(|v| v.as_str())
                        == Some("kamafeu.voicebank") =>
                {
                    match crate::copaiba::native_project::CopaibaNativeProject::load_dir(&path) {
                        Ok(project) => self.activate_native_project(project),
                        Err(error) => {
                            self.status_message = format!("Erro ao abrir KFV 2 no Copaíba: {error}")
                        }
                    }
                    return;
                }
                Err(error) => {
                    self.status_message = error;
                    return;
                }
                _ => {}
            }
        }
        match CopaibaConfig::load_from_dir(&path) {
            Ok(cfg) => {
                self.native_project = None;
                self.native_selected_layer = None;
                self.native_selected_unit = None;
                self.native_selected_asset = None;
                self.native_compare_asset = None;
                self.native_comparison_a = None;
                self.native_comparison_b = None;
                self.current_dir = Some(path);
                self.active_oto_path = None;
                self.selected_source_alias = None;
                self.preserve_oto_entry = false;
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

    pub fn create_native_draft(&mut self, parent: &std::path::Path) -> Result<(), String> {
        let project = crate::copaiba::native_project::CopaibaNativeProject::create_new_draft(
            parent,
            &self.native_draft_voice_id,
            &self.native_draft_bank_id,
            &self.native_draft_display_name,
            &self.native_draft_publisher,
            crate::copaiba::native_project::CopaibaNativeProject::starter_pt_br_profile(),
        )?;
        let folder = project.root.clone();
        self.activate_native_project(project);
        self.native_create_wizard_open = false;
        self.status_message = format!(
            "Rascunho KFV 2 criado em {} · inventário fonético sem G2P; licenciamento e gravações ainda precisam ser definidos",
            folder.display()
        );
        Ok(())
    }

    fn activate_native_project(
        &mut self,
        project: crate::copaiba::native_project::CopaibaNativeProject,
    ) {
        self.current_dir = Some(project.root.clone());
        self.active_oto_path = None;
        self.selected_source_alias = None;
        self.preserve_oto_entry = false;
        self.config = CopaibaConfig::default();
        self.native_selected_layer = project.layers.first().map(|(layer, _)| layer.id.clone());
        self.native_selected_unit = project
            .layers
            .first()
            .and_then(|(_, file)| file.units.first())
            .map(|unit| unit.id.clone());
        self.native_selected_asset = project
            .layers
            .first()
            .and_then(|(_, file)| file.assets.first())
            .map(|asset| asset.id.clone());
        let first_asset = self.native_selected_asset.as_deref();
        self.native_compare_asset = project
            .layers
            .first()
            .and_then(|(_, file)| {
                file.assets
                    .iter()
                    .find(|asset| Some(asset.id.as_str()) != first_asset)
            })
            .map(|asset| asset.id.clone());
        self.native_comparison_a = None;
        self.native_comparison_b = None;
        self.native_project = Some(project);
        self.selected_entry_index = None;
        self.avatar_texture = None;
        self.status_message.clear();
        self.load_selected_wav_samples();
        if self.native_selected_asset.is_some() {
            if let Err(error) = self.load_native_comparison_waveforms() {
                self.status_message = format!("Falha ao carregar comparação A/B: {error}");
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
        if self.native_project.is_some() {
            self.load_native_selected_wav_samples();
            return;
        }
        let target_wav = self.selected_entry().map(|e| e.wav_filename.clone());
        if let (Some(ref dir), Some(ref wav_name)) = (&self.current_dir, &target_wav) {
            if let Ok(waveform) =
                crate::renderer::TrackRenderer::load_wav_samples(dir.join(wav_name))
            {
                let wav_path = dir.join(wav_name);
                self.venus_analysis =
                    crate::dsp::venus_analysis::load_or_analyze(&wav_path, &waveform.0, waveform.1)
                        .ok();
                self.venus_analysis_path = self
                    .venus_analysis
                    .as_ref()
                    .map(|_| crate::dsp::venus_analysis::sidecar_path(&wav_path));
                self.loaded_waveform = Some(waveform);
                self.loaded_wav_filename = Some(wav_name.clone());
            } else {
                self.loaded_waveform = None;
                self.loaded_wav_filename = None;
                self.venus_analysis = None;
                self.venus_analysis_path = None;
            }
        } else {
            self.loaded_waveform = None;
            self.loaded_wav_filename = None;
            self.venus_analysis = None;
            self.venus_analysis_path = None;
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

    fn load_native_selected_wav_samples(&mut self) {
        let sample = self.native_project.as_ref().and_then(|project| {
            let layer_id = self.native_selected_layer.as_deref()?;
            let unit_id = self.native_selected_unit.as_deref()?;
            let unit = project.unit(layer_id, unit_id)?;
            let (_, file) = project
                .layers
                .iter()
                .find(|(layer, _)| layer.id == layer_id)?;
            let asset = file.assets.iter().find(|asset| asset.id == unit.asset_id)?;
            Some(project.root.join(&asset.path))
        });
        if let Some(path) = sample {
            match crate::renderer::TrackRenderer::load_wav_samples(&path) {
                Ok(waveform) => {
                    self.loaded_wav_filename = Some(path.to_string_lossy().into_owned());
                    self.loaded_waveform = Some(waveform);
                }
                Err(error) => {
                    self.loaded_wav_filename = None;
                    self.loaded_waveform = None;
                    self.status_message = format!("Falha ao carregar gravação nativa: {error}");
                }
            }
        } else {
            self.loaded_wav_filename = None;
            self.loaded_waveform = None;
        }
    }

    pub fn load_native_comparison_waveforms(&mut self) -> Result<(), String> {
        let project = self
            .native_project
            .as_ref()
            .ok_or("Nenhum banco KFV 2 aberto")?;
        let layer_id = self
            .native_selected_layer
            .as_deref()
            .ok_or("Nenhuma camada selecionada")?;
        let asset_a = self
            .native_selected_asset
            .as_deref()
            .ok_or("Selecione a gravação A")?;
        let asset_b = self.native_compare_asset.as_deref();
        if asset_b == Some(asset_a) {
            return Err("A e B precisam ser gravações diferentes".into());
        }
        let waveform_a = project.load_asset_audio(layer_id, asset_a)?;
        let waveform_b = asset_b
            .map(|asset_id| project.load_asset_audio(layer_id, asset_id))
            .transpose()?;
        self.native_comparison_a = Some(waveform_a);
        self.native_comparison_b = waveform_b;
        Ok(())
    }

    pub fn preview_native_comparison_take(
        &self,
        audio_player: &mut AudioPlayer,
        take: char,
    ) -> Result<(), String> {
        let waveform = match take.to_ascii_uppercase() {
            'A' => self.native_comparison_a.as_ref(),
            'B' => self.native_comparison_b.as_ref(),
            _ => return Err("Selecione a tomada A ou B".into()),
        }
        .ok_or("Waveform da tomada ainda não carregada")?;
        audio_player.play_samples(waveform.0.clone(), waveform.1);
        Ok(())
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
        if let Some(project) = self.native_project.as_mut() {
            let directory = self.current_dir.clone().ok_or("Nenhum diretório aberto")?;
            return project.save_to_dir(&directory);
        }
        if self.preserve_oto_entry {
            let oto_path = self
                .active_oto_path
                .as_ref()
                .ok_or_else(|| "nenhum oto.ini ativo".to_string())?;
            let index = self
                .selected_entry_index
                .ok_or_else(|| "nenhum alias selecionado".to_string())?;
            let entry = self
                .config
                .entries
                .get(index)
                .ok_or_else(|| "alias selecionado inválido".to_string())?;
            let oto_entry = crate::oto::OtoEntry::new(
                entry.wav_filename.clone(),
                entry.alias.clone(),
                entry.corte_inicial_ms,
                entry.consoante_ms,
                entry.corte_final_ms,
                entry.preutterance_ms,
                entry.overlap_ms,
            );
            let original_alias = self
                .selected_source_alias
                .as_deref()
                .unwrap_or(entry.alias.as_str());
            crate::copaiba_bridge::update_oto_entry(oto_path, original_alias, &oto_entry)?;
            self.selected_source_alias = Some(entry.alias.clone());
            self.status_message = format!("Entrada salva em {}", oto_path.display());
            return Ok(());
        }
        if let Some(ref dir) = self.current_dir {
            self.config.save_to_dir(dir)
        } else {
            Err("Nenhum diretório aberto".to_string())
        }
    }

    pub fn reload_current(&mut self) {
        if let Some(alias) = self.selected_source_alias.clone() {
            if let Some(oto_path) = self.active_oto_path.clone() {
                if let Some(dir) = oto_path.parent() {
                    self.open_alias(dir.to_path_buf(), &alias);
                    return;
                }
            }
        }
        if let Some(dir) = self.current_dir.clone() {
            self.open_dir(dir);
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

    pub fn import_native_recording(
        &mut self,
        source: PathBuf,
        asset_id: &str,
    ) -> Result<(), String> {
        let layer_id = self
            .native_selected_layer
            .clone()
            .ok_or("Nenhuma camada selecionada")?;
        let project = self
            .native_project
            .as_mut()
            .ok_or("Nenhum banco KFV 2 aberto")?;
        let path = project.import_recording(&layer_id, &source, asset_id)?;
        self.native_selected_asset = Some(asset_id.to_string());
        self.status_message = format!("Gravação copiada sem modificar o original: {path}");
        Ok(())
    }

    pub fn add_native_unit_from_selected_asset(&mut self) -> Result<(), String> {
        let layer_id = self
            .native_selected_layer
            .clone()
            .ok_or("Nenhuma camada selecionada")?;
        let asset_id = self
            .native_selected_asset
            .clone()
            .ok_or("Nenhuma gravação selecionada")?;
        let phones: Vec<String> = self
            .native_new_phones
            .split_whitespace()
            .map(str::to_string)
            .collect();
        let project = self
            .native_project
            .as_mut()
            .ok_or("Nenhum banco KFV 2 aberto")?;
        project.add_unit_from_asset(
            &layer_id,
            &asset_id,
            &self.native_new_unit_id,
            &self.native_new_kind,
            phones,
        )?;
        self.native_selected_unit = Some(self.native_new_unit_id.clone());
        self.load_native_selected_wav_samples();
        Ok(())
    }

    pub fn package_native_draft(&mut self, destination: &std::path::Path) -> Result<(), String> {
        self.save_config()?;
        self.native_project
            .as_mut()
            .ok_or("Nenhum banco KFV 2 aberto")?
            .package_kfv2_draft(destination)
    }

    pub fn migrate_classic_voicebank(
        &mut self,
        profile_path: &std::path::Path,
        mapping_path: Option<&std::path::Path>,
        destination_parent: &std::path::Path,
    ) -> Result<(), String> {
        let source_root = self
            .current_dir
            .as_deref()
            .ok_or("Nenhum voicebank clássico aberto")?;
        if self.native_project.is_some() {
            return Err("A migração oto.ini só se aplica a um banco clássico".into());
        }
        let source = crate::oto::Voicebank::new(source_root)
            .map_err(|error| format!("Não foi possível abrir o oto.ini de origem: {error}"))?;
        let (profile, phone_mappings) = load_migration_inputs(profile_path, mapping_path)?;

        let slug = migration_slug(&source.name);
        let options = crate::voicebank::migration::LegacyMigrationOptions {
            voice_id: format!("org.kamafeu.voice.{slug}"),
            bank_id: format!("org.kamafeu.voice.{slug}.native"),
            bank_version: "0.1.0".into(),
            display_name: format!("{} · Nativa (rascunho)", source.name),
            publisher: if source.author.trim().is_empty() {
                "Autor não informado".into()
            } else {
                source.author.clone()
            },
            profile,
            style_id: "natural".into(),
            style_display_name: "Natural".into(),
            layer_id: "natural-midi-60".into(),
            recorded_midi: 60,
            phone_mappings,
        };
        let destination = destination_parent.join(format!("{slug}-kfv2-draft"));
        let report = crate::voicebank::migration::migrate_legacy_voicebank_to_copy(
            &source,
            &destination,
            &options,
        )?;
        drop(source);
        let project =
            crate::copaiba::native_project::CopaibaNativeProject::load_dir(&report.destination)
                .map_err(|error| format!("Rascunho criado, mas não reabriu no Copaíba: {error}"))?;
        self.activate_native_project(project);
        self.migration_report = Some(report.clone());
        let inferred = report
            .entries
            .iter()
            .filter(|entry| {
                entry.mapping_origin == crate::voicebank::migration::MappingOrigin::ExactAliasTokens
            })
            .count();
        let explicit = report
            .entries
            .iter()
            .filter(|entry| {
                entry.mapping_origin == crate::voicebank::migration::MappingOrigin::Explicit
            })
            .count();
        let skipped = report
            .entries
            .iter()
            .filter(|entry| {
                entry.status == crate::voicebank::migration::MigrationEntryStatus::Skipped
            })
            .map(|entry| format!("{}: {}", entry.alias, entry.warnings.join("; ")))
            .collect::<Vec<_>>();
        let mut status = format!(
            "Rascunho KFV 2 criado em {} · {} unidades · {} mapas explícitos · {} aliases inferidos · {} ignorados · SHA-256 {}",
            report.destination.display(),
            report.migrated_units,
            explicit,
            inferred,
            report.skipped_entries,
            report.content_sha256
        );
        if !skipped.is_empty() {
            status.push_str("\nIgnorados:\n");
            status.push_str(&skipped.join("\n"));
        }
        status.push_str(
            "\nRevise todas as marcações e licenças no Copaíba antes de usar ou distribuir.",
        );
        self.status_message = status;
        Ok(())
    }

    fn prepare_classic_voicebank_migration(
        &mut self,
        profile_path: PathBuf,
        mapping_path: Option<PathBuf>,
        destination_parent: PathBuf,
    ) -> Result<(), String> {
        let source_root = self
            .current_dir
            .as_deref()
            .ok_or("Nenhum voicebank clássico aberto")?;
        if self.native_project.is_some() {
            return Err("A migração oto.ini só se aplica a um banco clássico".into());
        }
        let source = crate::oto::Voicebank::new(source_root)
            .map_err(|error| format!("Não foi possível abrir o oto.ini de origem: {error}"))?;
        let output_parent = std::fs::canonicalize(&destination_parent)
            .map_err(|error| format!("Não foi possível validar a pasta de destino: {error}"))?;
        let destination = output_parent.join(format!(
            "{}-kfv2-draft",
            migration_slug(&source.name)
        ));
        let canonical_source = std::fs::canonicalize(source_root)
            .map_err(|error| format!("Não foi possível validar o banco de origem: {error}"))?;
        if destination.starts_with(&canonical_source) {
            return Err("A pasta de destino precisa ficar fora do voicebank original".into());
        }
        if destination.exists() || std::fs::symlink_metadata(&destination).is_ok() {
            return Err(format!(
                "A pasta de destino já existe: {}",
                destination.display()
            ));
        }
        let (profile, phone_mappings) =
            load_migration_inputs(&profile_path, mapping_path.as_deref())?;
        let preview = crate::voicebank::migration::preview_legacy_voicebank_migration(
            &source,
            &profile,
            &phone_mappings,
        )?;
        self.pending_classic_migration = Some(PendingClassicMigration {
            profile_path,
            mapping_path,
            destination_parent,
            preview,
        });
        Ok(())
    }

    fn confirm_classic_voicebank_migration(&mut self) {
        let Some(pending) = self.pending_classic_migration.take() else {
            return;
        };
        self.status_message = match self.migrate_classic_voicebank(
            &pending.profile_path,
            pending.mapping_path.as_deref(),
            &pending.destination_parent,
        ) {
            Ok(()) => self.status_message.clone(),
            Err(error) => format!("Falha na migração assistida: {error}"),
        };
    }

    pub fn preview_native_unit(&mut self, audio_player: &mut AudioPlayer) -> Result<(), String> {
        self.preview_native_unit_backend(
            audio_player,
            crate::voicebank::render::PreviewBackend::Sola,
        )
    }

    pub fn preview_native_unit_backend(
        &mut self,
        audio_player: &mut AudioPlayer,
        backend: crate::voicebank::render::PreviewBackend,
    ) -> Result<(), String> {
        let project = self
            .native_project
            .as_ref()
            .ok_or("Nenhum banco KFV 2 aberto")?;
        let layer_id = self
            .native_selected_layer
            .as_deref()
            .ok_or("Nenhuma camada selecionada")?;
        let unit_id = self
            .native_selected_unit
            .as_deref()
            .ok_or("Nenhuma unidade selecionada")?;
        let (layer, _) = project
            .layers
            .iter()
            .find(|(layer, _)| layer.id == layer_id)
            .ok_or("Camada selecionada não existe")?;
        let unit = project
            .unit(layer_id, unit_id)
            .ok_or("Unidade selecionada não existe")?;
        let event = crate::voicebank::planner::PhoneEvent {
            phones: unit.phones.clone(),
            kind: unit.kind.clone(),
            position_ms: 0.0,
            duration_ms: unit.timing.min_target_ms,
            midi: layer.recorded_midi,
            pitch_points: Vec::new(),
            unit_override_id: Some(unit.id.clone()),
            layer_override_id: Some(layer.id.clone()),
        };
        let bank = project.native_bank_snapshot();
        let phrase = crate::voicebank::planner::PhraseRequest {
            style_id: layer.style_id.clone(),
            events: vec![event],
        };
        let plan = crate::voicebank::planner::plan_phrase(&bank, &phrase)?;
        let rendered = match backend {
            crate::voicebank::render::PreviewBackend::Sola => {
                crate::voicebank::render::render_phrase_sola(&bank, &plan, 48_000)?
            }
            crate::voicebank::render::PreviewBackend::World => {
                crate::voicebank::render::render_phrase_world(&bank, &plan, 48_000)?
            }
        };
        if rendered.samples.is_empty() {
            return Err(format!(
                "{} não produziu áudio para a unidade selecionada",
                rendered.engine_id
            ));
        }
        audio_player.play_samples(rendered.samples, rendered.sample_rate_hz);
        let fallback = rendered
            .units
            .iter()
            .flat_map(|unit| unit.fallback_reasons.iter())
            .next()
            .cloned();
        self.status_message = format!(
            "Preview {} · {}{}",
            rendered.engine_id,
            unit.id,
            fallback
                .map(|reason| format!(" · fallback: {reason}"))
                .unwrap_or_default()
        );
        Ok(())
    }

    pub fn preview_native_phrase(&mut self, audio_player: &mut AudioPlayer) -> Result<(), String> {
        self.preview_native_phrase_backend(
            audio_player,
            crate::voicebank::render::PreviewBackend::Sola,
        )
    }

    pub fn preview_native_phrase_backend(
        &mut self,
        audio_player: &mut AudioPlayer,
        backend: crate::voicebank::render::PreviewBackend,
    ) -> Result<(), String> {
        let project = self
            .native_project
            .as_ref()
            .ok_or("Nenhum banco KFV 2 aberto")?;
        let layer_id = self
            .native_selected_layer
            .as_deref()
            .ok_or("Nenhuma camada selecionada")?;
        let plan = project.plan_direct_phone_phrase(layer_id, &self.native_test_phones)?;
        let bank = project.native_bank_snapshot();
        let rendered = match backend {
            crate::voicebank::render::PreviewBackend::Sola => {
                crate::voicebank::render::render_phrase_sola(&bank, &plan, 48_000)?
            }
            crate::voicebank::render::PreviewBackend::World => {
                crate::voicebank::render::render_phrase_world(&bank, &plan, 48_000)?
            }
        };
        if rendered.samples.is_empty() {
            return Err(format!(
                "{} não produziu áudio para a frase de teste",
                rendered.engine_id
            ));
        }
        audio_player.play_samples(rendered.samples, rendered.sample_rate_hz);
        let fallback = rendered
            .units
            .iter()
            .flat_map(|unit| unit.fallback_reasons.iter())
            .next()
            .cloned();
        self.status_message = format!(
            "Preview {} · {} eventos{}",
            rendered.engine_id,
            plan.units.len(),
            fallback
                .map(|reason| format!(" · fallback: {reason}"))
                .unwrap_or_default()
        );
        Ok(())
    }
}

fn migration_slug(value: &str) -> String {
    let slug: String = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    let slug = slug.trim_matches('-');
    if slug.is_empty() {
        "voicebank".into()
    } else {
        slug.chars().take(64).collect()
    }
}

fn load_migration_inputs(
    profile_path: &std::path::Path,
    mapping_path: Option<&std::path::Path>,
) -> Result<(
    crate::voicebank::native::PhoneticProfile,
    std::collections::BTreeMap<String, Vec<String>>,
), String> {
    let profile: crate::voicebank::native::PhoneticProfile = serde_json::from_slice(
        &std::fs::read(profile_path)
            .map_err(|error| format!("Não foi possível ler o perfil fonético: {error}"))?,
    )
    .map_err(|error| format!("Perfil fonético inválido: {error}"))?;
    profile.validate()?;
    let phone_mappings = if let Some(path) = mapping_path {
        serde_json::from_slice::<std::collections::BTreeMap<String, Vec<String>>>(
            &std::fs::read(path)
                .map_err(|error| format!("Não foi possível ler o mapa de aliases: {error}"))?,
        )
        .map_err(|error| {
            format!("Mapa de aliases inválido; esperado objeto alias → lista de IDs: {error}")
        })?
    } else {
        std::collections::BTreeMap::new()
    };
    Ok((profile, phone_mappings))
}

#[cfg(test)]
mod migration_ui_tests {
    use super::*;

    #[test]
    fn classic_migration_action_builds_and_reports_a_separate_kfv2_draft() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("classic-bank");
        let destination_parent = temp.path().join("exports");
        std::fs::create_dir_all(&source).unwrap();
        std::fs::create_dir_all(&destination_parent).unwrap();
        std::fs::write(
            source.join("character.txt"),
            "name=Voz de Teste\nauthor=Autora\n",
        )
        .unwrap();
        std::fs::write(source.join("oto.ini"), "take.wav=syllable,0,20,0,10,5\n").unwrap();
        let wav = source.join("take.wav");
        let mut writer = hound::WavWriter::create(
            &wav,
            hound::WavSpec {
                channels: 1,
                sample_rate: 48_000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for _ in 0..48_000 {
            writer.write_sample(0_i16).unwrap();
        }
        writer.finalize().unwrap();
        let original_wav = std::fs::read(&wav).unwrap();

        let profile_path = temp.path().join("pt-br-profile.json");
        std::fs::write(
            &profile_path,
            include_bytes!("../../docs/kfv2/examples/minimal/phonetics/pt-BR.json"),
        )
        .unwrap();
        let mapping_path = temp.path().join("aliases.json");
        std::fs::write(&mapping_path, r#"{"syllable":["k","a"]}"#).unwrap();
        let mut app = CopaibaToolkitApp {
            current_dir: Some(source.clone()),
            ..CopaibaToolkitApp::default()
        };

        app.migrate_classic_voicebank(&profile_path, Some(&mapping_path), &destination_parent)
            .unwrap();
        let migration_report = app.migration_report.as_ref().unwrap();
        assert_eq!(migration_report.migrated_units, 1);
        assert_eq!(migration_report.entries.len(), 1);
        let output = destination_parent.join("voz-de-teste-kfv2-draft");
        let native = crate::voicebank::native::NativeBank::load_dir(&output).unwrap();
        assert_eq!(native.layers[0].1.units.len(), 1);
        assert!(app.native_project.is_some());
        assert_eq!(
            app.current_dir.as_deref(),
            Some(std::fs::canonicalize(&output).unwrap().as_path())
        );
        assert!(native.layers[0].1.units[0]
            .legacy_aliases
            .contains(&"syllable".into()));
        assert!(app.status_message.contains("1 mapas explícitos"));
        assert!(app.status_message.contains("0 aliases inferidos"));
        assert!(app.status_message.contains("SHA-256"));
        assert!(app.status_message.contains("Revise todas"));
        assert_eq!(std::fs::read(wav).unwrap(), original_wav);
        assert!(source.join("oto.ini").is_file());
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

fn draw_copaiba_native_ui(
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
        draw_waveform_peak_envelope(
            &painter,
            rect,
            samples,
            visual_tokens::ACCENT,
            0.8,
            1.0,
        );
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
        mark(unit.source.start_frame, Color32::from_rgb(126, 190, 226), "início");
        mark(unit.source.anchor_frame, visual_tokens::ACCENT, "âncora");
        mark(
            unit.source.fixed_end_frame,
            visual_tokens::WARNING,
            "fixo",
        );
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

fn native_layer_lists(
    app: &CopaibaToolkitApp,
    layer_id: &str,
) -> ((), Vec<(String, String)>, Vec<(String, String)>) {
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

pub fn draw_copaiba_toolkit_ui(
    app: &mut CopaibaToolkitApp,
    audio_player: &mut AudioPlayer,
    ui: &mut egui::Ui,
) {
    ui.vertical(|ui| {
        egui::Frame::none()
            .fill(visual_tokens::PANEL)
            .stroke(Stroke::new(1.0_f32, visual_tokens::BORDER))
            .rounding(Rounding::same(10.0))
            .inner_margin(egui::Margin::symmetric(16.0, 12.0))
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.vertical(|ui| {
                        ui.heading(
                            RichText::new("COPAÍBA")
                                .strong()
                                .size(22.0)
                                .color(visual_tokens::ACCENT),
                        );
                        ui.label(
                            RichText::new("VOICEBANK WORKSHOP · KAMAFEU")
                                .size(10.5)
                                .strong()
                                .color(visual_tokens::MUTED),
                        );
                    });
                    ui.add_space(10.0);

                    if app.current_dir.is_some() {
                        if ui.button("Abrir pasta").clicked() {
                            if let Some(folder) = crate::dialogs::FileDialog::new().pick_folder() {
                                app.open_dir(folder);
                            }
                        }

                        if ui.button("Abrir pacote .kfv").clicked() {
                            if let Some(package) = crate::dialogs::FileDialog::new()
                                .add_filter("Kamafeu Voicebank (*.kfv)", &["kfv"])
                                .pick_file()
                            {
                                app.open_dir(package);
                            }
                        }
                    }

                    if app.current_dir.is_some()
                        && app.native_project.is_none()
                        && ui.button("Novo banco KFV 2").clicked()
                    {
                        app.native_create_wizard_open = !app.native_create_wizard_open;
                    }

                    if app.native_project.is_some() {
                        if ui.button("Salvar KFV 2").clicked() {
                            app.status_message = match app.save_config() {
                                Ok(()) => "Marcas KFV 2 salvas".into(),
                                Err(error) => format!("Falha ao salvar KFV 2: {error}"),
                            };
                        }
                        if ui.button("Recarregar").clicked() {
                            app.reload_current();
                        }
                        if ui.button("Desfazer").clicked() {
                            let result = app.native_project.as_mut().unwrap().undo();
                            app.status_message = match result {
                                Ok(true) => {
                                    app.load_selected_wav_samples();
                                    "Edição desfeita".into()
                                }
                                Ok(false) => "Não há edição para desfazer".into(),
                                Err(error) => format!("Falha ao desfazer: {error}"),
                            };
                        }
                        if ui.button("Refazer").clicked() {
                            let result = app.native_project.as_mut().unwrap().redo();
                            app.status_message = match result {
                                Ok(true) => {
                                    app.load_selected_wav_samples();
                                    "Edição refeita".into()
                                }
                                Ok(false) => "Não há edição para refazer".into(),
                                Err(error) => format!("Falha ao refazer: {error}"),
                            };
                        }
                        if ui.button("Importar WAV").clicked() {
                            if let Some(source) = crate::dialogs::FileDialog::new()
                                .add_filter("Áudio WAV (*.wav)", &["wav"])
                                .pick_file()
                            {
                                let stem = source
                                    .file_stem()
                                    .and_then(|name| name.to_str())
                                    .unwrap_or("take");
                                let asset_id: String = stem
                                    .chars()
                                    .map(|ch| {
                                        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                                            ch
                                        } else {
                                            '-'
                                        }
                                    })
                                    .collect();
                                if let Err(error) = app.import_native_recording(source, &asset_id) {
                                    app.status_message = error;
                                }
                            }
                        }
                        if ui.button("Ouvir unidade").clicked() {
                            app.status_message = match app.preview_native_unit(audio_player) {
                                Ok(()) => app.status_message.clone(),
                                Err(error) => format!("Preview experimental: {error}"),
                            };
                        }
                        if ui.button("Empacotar .kfv").clicked() {
                            let default_name = app
                                .native_project
                                .as_ref()
                                .map(|project| {
                                    format!("{}.kfv", project.manifest.bank_id.replace('.', "_"))
                                })
                                .unwrap_or_else(|| "voicebank.kfv".into());
                            if let Some(destination) = crate::dialogs::FileDialog::new()
                                .set_file_name(&default_name)
                                .add_filter("Kamafeu Voicebank (*.kfv)", &["kfv"])
                                .save_file()
                            {
                                app.status_message = match app.package_native_draft(&destination) {
                                    Ok(()) => format!(
                                        "Rascunho KFV 2 verificado: {}",
                                        destination.display()
                                    ),
                                    Err(error) => format!("Falha ao empacotar rascunho: {error}"),
                                };
                            }
                        }
                    } else if app.current_dir.is_some() {
                        if ui.button("Salvar").clicked() {
                            app.status_message = match app.save_config() {
                                Ok(()) => "Configurações salvas!".to_string(),
                                Err(err) => format!("Erro ao salvar: {err}"),
                            };
                        }

                        if ui.button("Recarregar").clicked() {
                            app.reload_current();
                        }

                        if ui.button("Compilar pacote .kfv").clicked() {
                            let default_name = format!("{}.kfv", app.config.voicebank_name);
                            if let Some(dest_file) = crate::dialogs::FileDialog::new()
                                .set_file_name(&default_name)
                                .add_filter("Kamafeu Voicebank (*.kfv)", &["kfv"])
                                .save_file()
                            {
                                match app.compile_and_package_kfv(&dest_file) {
                                    Ok(()) => {
                                        app.status_message = format!(
                                            "Compilado com sucesso: {}",
                                            dest_file
                                                .file_name()
                                                .unwrap_or_default()
                                                .to_string_lossy()
                                        );
                                    }
                                    Err(err) => {
                                        app.status_message = format!("Erro ao empacotar: {err}");
                                    }
                                }
                            }
                        }

                        if ui.button("Converter para KFV 2").clicked() {
                            if let Some(profile_path) = crate::dialogs::FileDialog::new()
                                .add_filter("Perfil fonético KFV 2 (*.json)", &["json"])
                                .pick_file()
                            {
                                let mapping_path = crate::dialogs::FileDialog::new()
                                    .add_filter("Mapa JSON alias → fonemas (*.json)", &["json"])
                                    .pick_file();
                                if let Some(destination_parent) =
                                    crate::dialogs::FileDialog::new().pick_folder()
                                {
                                    app.pending_classic_migration = None;
                                    app.status_message = match app.prepare_classic_voicebank_migration(
                                        profile_path,
                                        mapping_path,
                                        destination_parent,
                                    ) {
                                        Ok(()) => {
                                            "Revise a prévia antes de criar o rascunho.".into()
                                        }
                                        Err(error) => {
                                            format!("Falha na migração assistida: {error}")
                                        }
                                    };
                                }
                            }
                        }

                        if ui.button("Duplicar alias").clicked() {
                            app.duplicate_selected_entry();
                        }

                        if app.selected_entry_index.is_some()
                            && ui.button("Excluir alias").clicked()
                        {
                            app.delete_selected_entry();
                        }
                    }

                    if let Some(path) = &app.current_dir {
                        ui.separator();
                        let bank_name = app
                            .native_project
                            .as_ref()
                            .map(|project| project.manifest.display_name.as_str())
                            .unwrap_or(app.config.voicebank_name.as_str());
                        let folder_name = path
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| path.display().to_string());
                        ui.vertical(|ui| {
                            ui.add_sized(
                                Vec2::new(240.0, 20.0),
                                egui::Label::new(
                                    RichText::new(bank_name)
                                        .strong()
                                        .color(visual_tokens::TEXT),
                                )
                                .truncate(),
                            )
                            .on_hover_text(bank_name);
                            ui.add_sized(
                                Vec2::new(240.0, 18.0),
                                egui::Label::new(
                                    RichText::new(folder_name)
                                        .size(10.5)
                                        .color(visual_tokens::MUTED),
                                )
                                .truncate(),
                            )
                            .on_hover_text(path.display().to_string());
                        });
                    }

                    if !app.status_message.is_empty() {
                        let status_color = if app.status_message.contains("Falha")
                            || app.status_message.contains("Erro")
                        {
                            visual_tokens::ERROR
                        } else {
                            visual_tokens::SUCCESS
                        };
                        let summary: String = app
                            .status_message
                            .lines()
                            .next()
                            .unwrap_or_default()
                            .chars()
                            .take(100)
                            .collect();
                        ui.colored_label(status_color, summary)
                            .on_hover_text(&app.status_message);
                    }
                });
            });

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(12.0);

        if app.current_dir.is_none() {
            ui.centered_and_justified(|ui| {
                ui.vertical_centered(|ui| {
                    egui::Frame::none()
                        .fill(visual_tokens::PANEL)
                        .stroke(Stroke::new(1.0_f32, visual_tokens::BORDER))
                        .rounding(Rounding::same(18.0))
                        .inner_margin(egui::Margin::same(32.0))
                        .show(ui, |ui| {
                            ui.set_max_width(640.0);
                            ui.vertical_centered(|ui| {
                                ui.label(
                                    RichText::new("STUDIO POMAR  /  KAMAFEU")
                                        .strong()
                                        .size(11.0)
                                        .color(visual_tokens::ACCENT),
                                );
                                ui.add_space(14.0);
                                ui.heading(
                                    RichText::new("Seus voicebanks,\ncom espaço para respirar.")
                                        .strong()
                                        .size(30.0)
                                        .color(visual_tokens::TEXT),
                                );
                                ui.add_space(10.0);
                                ui.label(
                                    RichText::new(
                                        "Abra um banco UTAU, revise gravações ou crie um rascunho KFV 2.",
                                    )
                                    .size(14.0)
                                    .color(visual_tokens::MUTED),
                                );
                                ui.add_space(24.0);
                                if ui
                                    .add_sized(
                                        Vec2::new(280.0, 46.0),
                                        egui::Button::new(
                                            RichText::new("Abrir pasta de voicebank")
                                                .strong()
                                                .color(visual_tokens::BACKGROUND),
                                        )
                                        .fill(visual_tokens::ACCENT)
                                        .stroke(Stroke::new(1.0_f32, visual_tokens::ACCENT_HOVER))
                                        .rounding(Rounding::same(10.0)),
                                    )
                                    .clicked()
                                {
                                    if let Some(folder) =
                                        crate::dialogs::FileDialog::new().pick_folder()
                                    {
                                        app.open_dir(folder);
                                    }
                                }
                                ui.add_space(10.0);
                                ui.horizontal_wrapped(|ui| {
                                    if ui.button("Abrir pacote .kfv").clicked() {
                                        if let Some(package) = crate::dialogs::FileDialog::new()
                                            .add_filter("Kamafeu Voicebank (*.kfv)", &["kfv"])
                                            .pick_file()
                                        {
                                            app.open_dir(package);
                                        }
                                    }
                                    if ui.button("Criar banco KFV 2").clicked() {
                                        app.native_create_wizard_open = true;
                                    }
                                });
                                if !app.status_message.is_empty() {
                                    ui.add_space(12.0);
                                    ui.colored_label(visual_tokens::WARNING, &app.status_message);
                                }
                            });
                        });
                    if app.native_create_wizard_open {
                        ui.add_space(16.0);
                        draw_native_create_wizard(app, ui);
                    }
                });
            });
            return;
        }

        if app.native_project.is_some() {
            draw_copaiba_native_ui(app, audio_player, ui);
            return;
        }

        if app.native_create_wizard_open {
            draw_native_create_wizard(app, ui);
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
                    painter.rect_filled(avatar_rect, Rounding::same(8.0), visual_tokens::BACKGROUND);
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
                            Pos2::new(center.x, center.y - 6.0),
                            egui::Align2::CENTER_CENTER,
                            "[Foto]",
                            egui::FontId::proportional(12.0),
                            Color32::from_rgb(165, 148, 201),
                        );
                        painter.text(
                            Pos2::new(center.x, center.y + 12.0),
                            egui::Align2::CENTER_CENTER,
                            "Sem Imagem",
                            egui::FontId::proportional(10.0),
                            Color32::from_rgb(165, 148, 201),
                        );
                    }

                    ui.add_space(10.0);

                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("Nome do Voicebank:").size(11.0).strong().color(visual_tokens::ACCENT));
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
                                RichText::new("Importar Foto / Avatar (100x100)...")
                                    .size(11.0)
                                    .strong()
                                    .color(visual_tokens::ACCENT),
                            )
                            .fill(Color32::from_rgb(10, 48, 30))
                            .stroke(Stroke::new(1.0_f32, visual_tokens::ACCENT))
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
                                && ui.button(RichText::new("Remover").size(11.0).color(Color32::from_rgb(255, 100, 100))).clicked() {
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
                    if ui.button(RichText::new("+ Duplicar").size(10.5).color(visual_tokens::ACCENT)).clicked() {
                        app.duplicate_selected_entry();
                    }
                    if ui.button(RichText::new("Excluir").size(10.5).color(Color32::from_rgb(255, 100, 100))).clicked() {
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
                                visual_tokens::ACCENT
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

                                    let add_btn = egui::Button::new(RichText::new("+ Adicionar").strong().color(Color32::BLACK))
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
                                                ui.label(RichText::new(&key).strong().color(visual_tokens::ACCENT));

                                                if ui.add(egui::TextEdit::singleline(pref).desired_width(100.0)).changed() {
                                                    changed = true;
                                                }
                                                if ui.add(egui::TextEdit::singleline(suff).desired_width(100.0)).changed() {
                                                    changed = true;
                                                }

                                                if ui.button(RichText::new("Excluir").color(Color32::from_rgb(255, 100, 100))).clicked() {
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
                    ui.label(RichText::new("Editor de Forma de Onda (Triplo Stack)").strong().size(12.0).color(visual_tokens::ACCENT));
                    ui.add_space(10.0);

                    if let Some((ref samples, sr)) = app.loaded_waveform {
                        if ui.button("▶ Tocar").clicked() {
                            audio_player.play_samples(samples.clone(), sr);
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

                painter.rect_filled(canvas_rect, Rounding::same(6.0), visual_tokens::BACKGROUND);
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

                        draw_waveform_peak_envelope(&painter, Rect::from_min_max(Pos2::new(canvas_rect.min.x, wave_area_min_y), canvas_rect.max), samples, visual_tokens::ACCENT, zoom_y, app.zoom_x);


                        painter.line_segment([Pos2::new(x_offset, wave_area_min_y), Pos2::new(x_offset, canvas_rect.max.y)], Stroke::new(2.5_f32, Color32::from_rgb(0, 150, 255)));
                        painter.text(Pos2::new(x_offset + 3.0, wave_area_min_y + 4.0), egui::Align2::LEFT_TOP, "Corte Inicial", egui::FontId::proportional(10.0), Color32::from_rgb(0, 180, 255));

                        painter.line_segment([Pos2::new(x_consonant, wave_area_min_y), Pos2::new(x_consonant, canvas_rect.max.y)], Stroke::new(2.0_f32, visual_tokens::SUCCESS));
                        painter.text(Pos2::new(x_consonant + 3.0, wave_area_min_y + 4.0), egui::Align2::LEFT_TOP, "Início Consoante", egui::FontId::proportional(10.0), visual_tokens::SUCCESS);

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
                ui.label(RichText::new("Controles & Sliders").strong().size(12.0).color(visual_tokens::ACCENT));
                ui.add_space(6.0);

                egui::ScrollArea::vertical()
                    .id_salt("copaiba_right_controls_scroll")
                    .max_height(ui.available_height())
                    .show(ui, |ui| {
                        let selected_index = app
                            .selected_entry_index
                            .filter(|idx| *idx < app.config.entries.len());
                        let wav_duration_ms = app
                            .loaded_waveform
                            .as_ref()
                            .map(|(samples, sample_rate)| {
                                (samples.len() as f64 / *sample_rate as f64) * 1000.0
                            })
                            .filter(|duration_ms| duration_ms.is_finite() && *duration_ms > 0.0);

                        if let Some(idx) = selected_index {
                            // Keep alias parameters editable even when the referenced WAV is
                            // missing or unreadable. In that case use a temporary range based
                            // on the stored values and clearly label it as an estimate.
                            let has_waveform = wav_duration_ms.is_some();
                            let duration_ms = wav_duration_ms.unwrap_or_else(|| {
                                let entry = &app.config.entries[idx];
                                [
                                    entry.corte_inicial_ms.abs(),
                                    entry.consoante_ms.abs(),
                                    entry.corte_final_ms.abs(),
                                    entry.preutterance_ms.abs(),
                                    entry.overlap_ms.abs(),
                                    entry.loop_inicio_ms.unwrap_or_default().abs(),
                                    entry.loop_fim_ms.unwrap_or_default().abs(),
                                    entry.cauda_final_ms.unwrap_or_default().abs(),
                                ]
                                .into_iter()
                                .fold(1_000.0_f64, f64::max)
                            });
                            let entry = &mut app.config.entries[idx];

                                ui.label(RichText::new("Nome do Alias:").size(11.0).color(Color32::from_rgb(200, 190, 230)));
                                ui.add(egui::TextEdit::singleline(&mut entry.alias).desired_width(240.0));

                                ui.add_space(4.0);
                                ui.label(RichText::new("Arquivo WAV:").size(10.0).color(Color32::from_rgb(160, 150, 180)));
                                ui.label(RichText::new(&entry.wav_filename).size(11.0).color(Color32::WHITE));
                                if !has_waveform {
                                    ui.colored_label(
                                        visual_tokens::WARNING,
                                        "WAV indisponível; faixa dos sliders é provisória.",
                                    )
                                    .on_hover_text(
                                        "Os parâmetros continuam editáveis. Reabra o WAV para ajustar os limites à duração real da gravação.",
                                    );
                                }

                                ui.add_space(8.0);
                                ui.separator();
                                ui.add_space(6.0);

                                ui.collapsing("Análise VENUS (.venus)", |ui| {
                                let mut venus_changed = false;
                                if let Some(analysis) = app.venus_analysis.as_mut() {
                                    let voiced = analysis.frames.iter().filter(|frame| frame.voiced).count();
                                    ui.label(
                                        RichText::new(format!(
                                            "{} frames · {} vozeados · {} marcas",
                                            analysis.frames.len(),
                                            voiced,
                                            analysis.pitch_marks.len()
                                        ))
                                        .size(10.0)
                                        .color(Color32::from_rgb(160, 180, 205)),
                                    );
                                    venus_changed |= ui
                                        .add(
                                            egui::Slider::new(&mut analysis.gain_db, -24.0..=24.0)
                                                .text("Ganho por alias")
                                                .suffix(" dB"),
                                        )
                                        .changed();
                                    venus_changed |= ui
                                        .add(
                                            egui::Slider::new(
                                                &mut analysis.formant_shift_cents,
                                                -2_400.0..=2_400.0,
                                            )
                                            .text("Correção de formante")
                                            .suffix(" cents"),
                                        )
                                        .changed();
                                    venus_changed |= ui
                                        .add(
                                            egui::Slider::new(&mut analysis.breathiness, 0.0..=100.0)
                                                .text("Respiração adicional"),
                                        )
                                        .changed();
                                    venus_changed |= ui
                                        .add(
                                            egui::TextEdit::multiline(&mut analysis.notes)
                                                .hint_text("Notas para este alias")
                                                .desired_rows(2),
                                        )
                                        .changed();
                                    if ui.button("Reanalisar WAV").clicked() {
                                        if let Some((samples, sample_rate)) = app.loaded_waveform.as_ref() {
                                            let mut refreshed = crate::dsp::venus_analysis::analyze_samples(samples, *sample_rate);
                                            refreshed.gain_db = analysis.gain_db;
                                            refreshed.formant_shift_cents = analysis.formant_shift_cents;
                                            refreshed.breathiness = analysis.breathiness;
                                            refreshed.notes.clone_from(&analysis.notes);
                                            *analysis = refreshed;
                                            venus_changed = true;
                                        }
                                    }
                                } else {
                                    ui.label(
                                        RichText::new("Não foi possível criar a análise deste WAV.")
                                            .size(10.0)
                                            .color(Color32::from_rgb(255, 140, 100)),
                                    );
                                }
                                if venus_changed {
                                    if let (Some(analysis), Some(path)) = (
                                        app.venus_analysis.as_ref(),
                                        app.venus_analysis_path.as_ref(),
                                    ) {
                                        let _ = crate::dsp::venus_analysis::save(path, analysis);
                                    }
                                }
                                });

                                ui.add_space(8.0);
                                ui.separator();
                                ui.add_space(6.0);

                                ui.label(
                                    RichText::new("Parâmetros de oto.ini")
                                        .strong()
                                        .size(12.0)
                                        .color(visual_tokens::ACCENT),
                                );
                                ui.label(RichText::new("Corte Inicial (ms):").size(11.0).color(Color32::from_rgb(0, 150, 255)));
                                ui.add(egui::Slider::new(&mut entry.corte_inicial_ms, 0.0..=duration_ms).suffix(" ms"));

                                ui.add_space(8.0);
                                ui.label(RichText::new("Início Consoante (ms):").size(11.0).color(visual_tokens::SUCCESS));
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

                                ui.label(RichText::new("Parte de Loop (Sustentação)").strong().size(11.0).color(visual_tokens::WARNING));
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
                                    ui.label(RichText::new("Loop Início:").size(10.0).color(visual_tokens::WARNING));
                                    ui.add(egui::Slider::new(l_start, 0.0..=duration_ms).suffix(" ms"));

                                    let l_end = entry.loop_fim_ms.get_or_insert(300.0);
                                    ui.label(RichText::new("Loop Fim:").size(10.0).color(visual_tokens::WARNING));
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
                        } else if app.config.entries.is_empty() {
                            ui.label(
                                RichText::new("Nenhum alias disponível neste voicebank.")
                                    .size(11.0)
                                    .color(visual_tokens::MUTED),
                            );
                        } else {
                            ui.label(
                                RichText::new("Selecione um alias para editar seus parâmetros.")
                                    .size(11.0)
                                    .color(visual_tokens::MUTED),
                            );
                        }
                    });
            });
        }
    });
});

if let Some(pending) = app.pending_classic_migration.as_ref() {
    let preview = pending.preview.clone();
    let mut open = true;
    let mut confirm = false;
    let mut cancel = false;
    egui::Window::new("Prévia da migração para KFV 2")
        .collapsible(false)
        .resizable(false)
        .open(&mut open)
        .show(ui.ctx(), |ui| {
            ui.label(format!("Entradas em oto.ini: {}", preview.total_entries));
            ui.label(format!("Mapeamentos explícitos: {}", preview.explicit_mappings));
            ui.label(format!("Aliases inferidos por tokens exatos: {}", preview.inferred_aliases));
            ui.label(format!("Sem mapeamento: {}", preview.unmapped_aliases));
            ui.label(format!("Mapeamentos inválidos: {}", preview.invalid_mappings));
            ui.label(format!("Entradas em aliases duplicados: {}", preview.alias_collisions));
            ui.strong(format!("Unidades potenciais antes de validar WAVs: {}", preview.potential_units));
            ui.add_space(6.0);
            for warning in &preview.warnings {
                ui.label(RichText::new(warning).color(Color32::YELLOW));
            }
            ui.add_space(8.0);
            ui.label("A conversão cria uma pasta editável de autoria e a abre no Copaíba. Depois da revisão, use ‘Empacotar rascunho KFV 2’ para gerar um pacote ZIP com extensão .kfv.");
            ui.horizontal(|ui| {
                if ui.button("Criar pasta de autoria").clicked() {
                    confirm = true;
                }
                if ui.button("Cancelar").clicked() {
                    cancel = true;
                }
            });
        });
    if confirm {
        app.confirm_classic_voicebank_migration();
    } else if cancel || !open {
        app.pending_classic_migration = None;
        app.status_message = "Migração cancelada; nenhum rascunho foi criado.".into();
    }
}
}

fn draw_native_create_wizard(app: &mut CopaibaToolkitApp, ui: &mut egui::Ui) {
    ui.add_space(12.0);
    egui::Frame::group(ui.style()).show(ui, |ui| {
        ui.set_max_width(540.0);
        ui.heading("Criar banco nativo · rascunho");
        ui.label("Começa com estilo Natural, camada MIDI 60 e um inventário PT-BR básico. Não inclui G2P nem gravações.");
        ui.horizontal(|ui| {
            ui.label("Nome");
            ui.text_edit_singleline(&mut app.native_draft_display_name);
        });
        ui.horizontal(|ui| {
            ui.label("Voice ID");
            ui.text_edit_singleline(&mut app.native_draft_voice_id);
        });
        ui.horizontal(|ui| {
            ui.label("Bank ID");
            ui.text_edit_singleline(&mut app.native_draft_bank_id);
        });
        ui.horizontal(|ui| {
            ui.label("Autor");
            ui.text_edit_singleline(&mut app.native_draft_publisher);
        });
        ui.small("O inventário é só uma lista de símbolos fonéticos. As pronúncias devem ser informadas diretamente; defina autorização e licença antes de distribuir.");
        ui.horizontal(|ui| {
            if ui.button("Escolher pasta e criar").clicked() {
                if let Some(parent) = crate::dialogs::FileDialog::new()
                    .set_title("Escolha onde criar o rascunho KFV 2")
                    .pick_folder()
                {
                    if let Err(error) = app.create_native_draft(&parent) {
                        app.status_message = format!("Não foi possível criar o banco: {error}");
                    }
                }
            }
            if ui.button("Cancelar").clicked() {
                app.native_create_wizard_open = false;
            }
        });
    });
}
