use crate::audio::AudioPlayer;
use crate::copaiba::{CopaibaConfig, CopaibaEntry};
use eframe::egui::{self, Color32, Pos2, Rect, RichText, Rounding, Sense, Stroke, Vec2};
use std::path::PathBuf;

#[path = "gui/legacy_bank.rs"]
mod legacy_bank;
#[path = "gui/legacy_editor.rs"]
mod legacy_editor;
#[path = "gui/migration.rs"]
mod migration;
#[path = "gui/native_assets.rs"]
mod native_assets;
#[path = "gui/native_authoring.rs"]
mod native_authoring;
#[path = "gui/native_preview.rs"]
mod native_preview;
#[path = "gui/native_ui.rs"]
mod native_ui;
#[path = "gui/toolbar.rs"]
mod toolbar;
#[path = "gui/welcome.rs"]
mod welcome;
#[path = "gui/wizard.rs"]
mod wizard;

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
    egui_extras::install_image_loaders(ctx);
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
    pub kamuse_project: Option<crate::copaiba::kamuse::KamuseProject>,
    pub kamuse_name: String,
    pub kamuse_id: String,
    pub kamuse_language: String,
    pub kamuse_recording_phones: String,
    pub kamuse_recording_midi: u8,
    pub kamuse_new_tone_midi: u8,
    pub kamuse_selected_tone_midi_edit: u8,
    pub kamuse_recording_alias_edit: String,
    pub kamuse_recording_phones_edit: String,
    pub kamuse_editing_tone_id: Option<String>,
    pub kamuse_editing_recording: Option<(String, String)>,
    pub kamuse_pending_delete_tone: Option<String>,
    pub kamuse_pending_delete_recording: Option<(String, String)>,
    pub kamuse_selected_tone: Option<String>,
    pub kamuse_selected_recording: Option<String>,
    pub kamuse_prompt_filter: String,
    pub kamuse_active_view: String,
    pub kamuse_waveform: Option<(PathBuf, Vec<f32>, f64)>,
    pub kamuse_drag_mark: Option<usize>,
    pub kamuse_marks_dirty: bool,
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
            kamuse_project: None,
            kamuse_name: "Nova voz Kamafeu".into(),
            kamuse_id: "org.kamafeu.voice.nova".into(),
            kamuse_language: "pt-BR".into(),
            kamuse_recording_phones: String::new(),
            kamuse_recording_midi: 60,
            kamuse_new_tone_midi: 60,
            kamuse_selected_tone_midi_edit: 60,
            kamuse_recording_alias_edit: String::new(),
            kamuse_recording_phones_edit: String::new(),
            kamuse_editing_tone_id: None,
            kamuse_editing_recording: None,
            kamuse_pending_delete_tone: None,
            kamuse_pending_delete_recording: None,
            kamuse_selected_tone: None,
            kamuse_selected_recording: None,
            kamuse_prompt_filter: String::new(),
            kamuse_active_view: "Editor".into(),
            kamuse_waveform: None,
            kamuse_drag_mark: None,
            kamuse_marks_dirty: false,
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
            let end_s = ((px_i + 1) * visible_samples)
                .div_ceil(px_width)
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

pub fn draw_copaiba_toolkit_ui(
    app: &mut CopaibaToolkitApp,
    audio_player: &mut AudioPlayer,
    ui: &mut egui::Ui,
) {
    ui.vertical(|ui| {
        toolbar::draw_toolbar(app, audio_player, ui);

        if app.kamuse_project.is_some() {
            ui.add_space(12.0);
            kamuse_ui::draw_kamuse_authoring_ui(app, audio_player, ui);
            return;
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(12.0);

        if app.current_dir.is_none() {
            welcome::draw_empty_state(app, ui);
            return;
        }

        if app.native_project.is_some() {
            native_ui::draw_copaiba_native_ui(app, audio_player, ui);
            return;
        }

        if app.native_create_wizard_open {
            wizard::draw_native_create_wizard(app, ui);
            return;
        }

        legacy_editor::draw_legacy_editor(app, audio_player, ui);
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

mod kamuse_ui;

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
