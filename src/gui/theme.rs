use eframe::egui::{self, Color32, Rounding, Stroke};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemePreset {
    PomarNeon,
    MelodyneGold,
    Cyberpunk,
    NordicSlate,
    #[serde(alias = "MikuTeal")]
    MikanTeal,
    SolarizedDark,
    CleanLight,
    Custom,
}

impl ThemePreset {
    pub const ALL: [ThemePreset; 7] = [
        ThemePreset::PomarNeon,
        ThemePreset::MelodyneGold,
        ThemePreset::Cyberpunk,
        ThemePreset::NordicSlate,
        ThemePreset::MikanTeal,
        ThemePreset::SolarizedDark,
        ThemePreset::CleanLight,
    ];

    pub fn display_name(&self) -> &'static str {
        self.display_name_for(crate::config::AppLanguage::PtBr)
    }

    pub fn display_name_for(&self, lang: crate::config::AppLanguage) -> &'static str {
        match lang {
            crate::config::AppLanguage::PtBr => match self {
                ThemePreset::PomarNeon => "Pomar Neon (Esmeralda)",
                ThemePreset::MelodyneGold => "Melodyne Classic (Âmbar)",
                ThemePreset::Cyberpunk => "Cyberpunk (Synthwave)",
                ThemePreset::NordicSlate => "Nordic Slate (Clean Dark)",
                ThemePreset::MikanTeal => "Voz-a-loide Teal (Mikan)",
                ThemePreset::SolarizedDark => "Solarized Dark",
                ThemePreset::CleanLight => "Clean Light (Estúdio Claro)",
                ThemePreset::Custom => "Personalizado",
            },
            crate::config::AppLanguage::EnUs => match self {
                ThemePreset::PomarNeon => "Pomar Neon (Emerald)",
                ThemePreset::MelodyneGold => "Melodyne Classic (Amber)",
                ThemePreset::Cyberpunk => "Cyberpunk (Synthwave)",
                ThemePreset::NordicSlate => "Nordic Slate (Clean Dark)",
                ThemePreset::MikanTeal => "Vocaloid Teal (Mikan)",
                ThemePreset::SolarizedDark => "Solarized Dark",
                ThemePreset::CleanLight => "Clean Light (Studio Light)",
                ThemePreset::Custom => "Custom",
            },
        }
    }

    pub fn description(&self) -> &'static str {
        self.description_for(crate::config::AppLanguage::PtBr)
    }

    pub fn description_for(&self, lang: crate::config::AppLanguage) -> &'static str {
        match lang {
            crate::config::AppLanguage::PtBr => match self {
                ThemePreset::PomarNeon => {
                    "Violeta escuro com notas em verde menta fluorescente e acentos lavanda."
                }
                ThemePreset::MelodyneGold => {
                    "Estilo clássico e elegante com notas douradas e âmbar quente."
                }
                ThemePreset::Cyberpunk => {
                    "Paleta noturna com notas magenta neon e linhas ciano elétrico."
                }
                ThemePreset::NordicSlate => {
                    "Azul ardósia minimalista com notas em azul ártico suave para longas sessões."
                }
                ThemePreset::MikanTeal => {
                    "Turquesa icônica inspirada em voz-a-loide com destaques em rosa pastel."
                }
                ThemePreset::SolarizedDark => {
                    "Azul petróleo clássico com notas douradas de alto contraste."
                }
                ThemePreset::CleanLight => {
                    "Interface clara de estúdio com notas vivas para trabalho diurno."
                }
                ThemePreset::Custom => "Esquema totalmente configurado pelo usuário.",
            },
            crate::config::AppLanguage::EnUs => match self {
                ThemePreset::PomarNeon => {
                    "Dark violet with fluorescent mint green notes and lavender accents."
                }
                ThemePreset::MelodyneGold => {
                    "Classic elegant style with gold notes and warm amber accents."
                }
                ThemePreset::Cyberpunk => {
                    "Night palette with neon magenta notes and electric cyan lines."
                }
                ThemePreset::NordicSlate => {
                    "Minimalist slate blue with soft arctic blue notes for long sessions."
                }
                ThemePreset::MikanTeal => {
                    "Iconic vocaloid-inspired turquoise with pastel pink accents."
                }
                ThemePreset::SolarizedDark => {
                    "Classic petrol blue with high-contrast golden notes."
                }
                ThemePreset::CleanLight => {
                    "Clean studio light interface with vivid notes for daytime work."
                }
                ThemePreset::Custom => "Fully customized user color scheme.",
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeConfig {
    pub preset: ThemePreset,
    pub bg_canvas: [u8; 3],
    pub bg_panel: [u8; 3],
    pub bg_header: [u8; 3],
    pub bg_row_white_key: [u8; 3],
    pub bg_row_black_key: [u8; 3],
    pub bg_keyboard_white: [u8; 3],
    pub bg_keyboard_black: [u8; 3],
    pub grid_line_bar: [u8; 3],
    pub grid_line_sub: [u8; 3],
    pub note_fill: [u8; 3],
    pub note_stroke: [u8; 3],
    pub note_selected_fill: [u8; 3],
    pub note_selected_stroke: [u8; 3],
    pub note_hover: [u8; 3],
    pub note_corner_radius: f32,
    pub note_stroke_width: f32,
    pub note_opacity: f32,
    pub note_waveform_opacity: f32,
    pub note_waveform_color: [u8; 3],
    pub pitch_curve_color: [u8; 3],
    pub pitch_anchor_color: [u8; 3],
    pub accent_color: [u8; 3],
    pub playhead_color: [u8; 3],
    pub text_primary: [u8; 3],
    pub text_muted: [u8; 3],
    pub text_note_tag: [u8; 3],
    pub ui_corner_radius: f32,
    pub panel_opacity: f32,
}

impl Default for ThemeConfig {
    fn default() -> Self {
        Self::from_preset(ThemePreset::PomarNeon)
    }
}

impl ThemeConfig {
    pub fn from_preset(preset: ThemePreset) -> Self {
        match preset {
            ThemePreset::PomarNeon => Self {
                preset: ThemePreset::PomarNeon,
                bg_canvas: [20, 18, 28],
                bg_panel: [28, 25, 38],
                bg_header: [36, 32, 48],
                bg_row_white_key: [26, 23, 36],
                bg_row_black_key: [18, 16, 25],
                bg_keyboard_white: [215, 212, 225],
                bg_keyboard_black: [24, 21, 32],
                grid_line_bar: [48, 42, 64],
                grid_line_sub: [32, 28, 44],
                note_fill: [34, 180, 126],
                note_stroke: [45, 205, 145],
                note_selected_fill: [52, 212, 155],
                note_selected_stroke: [220, 245, 235],
                note_hover: [40, 195, 138],
                note_corner_radius: 6.0,
                note_stroke_width: 1.5,
                note_opacity: 1.0,
                note_waveform_opacity: 0.30,
                note_waveform_color: [10, 40, 28],
                pitch_curve_color: [185, 155, 230],
                pitch_anchor_color: [165, 130, 215],
                accent_color: [160, 130, 215],
                playhead_color: [45, 205, 145],
                text_primary: [230, 228, 238],
                text_muted: [140, 132, 160],
                text_note_tag: [12, 28, 20],
                ui_corner_radius: 4.0,
                panel_opacity: 1.0,
            },
            ThemePreset::MelodyneGold => Self {
                preset: ThemePreset::MelodyneGold,
                bg_canvas: [26, 26, 30],
                bg_panel: [34, 34, 40],
                bg_header: [44, 44, 52],
                bg_row_white_key: [38, 38, 44],
                bg_row_black_key: [26, 26, 30],
                bg_keyboard_white: [218, 218, 222],
                bg_keyboard_black: [22, 22, 26],
                grid_line_bar: [58, 58, 68],
                grid_line_sub: [40, 40, 48],
                note_fill: [205, 150, 52],
                note_stroke: [228, 172, 70],
                note_selected_fill: [235, 185, 90],
                note_selected_stroke: [250, 240, 210],
                note_hover: [218, 162, 60],
                note_corner_radius: 8.0,
                note_stroke_width: 1.5,
                note_opacity: 1.0,
                note_waveform_opacity: 0.30,
                note_waveform_color: [50, 30, 0],
                pitch_curve_color: [225, 115, 80],
                pitch_anchor_color: [230, 140, 65],
                accent_color: [205, 150, 52],
                playhead_color: [220, 95, 75],
                text_primary: [235, 235, 240],
                text_muted: [145, 145, 158],
                text_note_tag: [30, 20, 6],
                ui_corner_radius: 5.0,
                panel_opacity: 1.0,
            },
            ThemePreset::Cyberpunk => Self {
                preset: ThemePreset::Cyberpunk,
                bg_canvas: [18, 16, 30],
                bg_panel: [26, 22, 42],
                bg_header: [36, 30, 56],
                bg_row_white_key: [28, 24, 46],
                bg_row_black_key: [18, 15, 30],
                bg_keyboard_white: [222, 216, 238],
                bg_keyboard_black: [20, 16, 32],
                grid_line_bar: [50, 45, 78],
                grid_line_sub: [34, 30, 54],
                note_fill: [210, 55, 120],
                note_stroke: [235, 85, 145],
                note_selected_fill: [240, 100, 160],
                note_selected_stroke: [100, 225, 240],
                note_hover: [225, 70, 135],
                note_corner_radius: 4.0,
                note_stroke_width: 1.8,
                note_opacity: 0.95,
                note_waveform_opacity: 0.35,
                note_waveform_color: [240, 240, 250],
                pitch_curve_color: [70, 205, 230],
                pitch_anchor_color: [225, 185, 70],
                accent_color: [70, 205, 230],
                playhead_color: [80, 215, 235],
                text_primary: [240, 235, 248],
                text_muted: [150, 135, 175],
                text_note_tag: [255, 255, 255],
                ui_corner_radius: 2.0,
                panel_opacity: 0.95,
            },
            ThemePreset::NordicSlate => Self {
                preset: ThemePreset::NordicSlate,
                bg_canvas: [34, 38, 46],
                bg_panel: [43, 48, 59],
                bg_header: [52, 58, 70],
                bg_row_white_key: [40, 45, 55],
                bg_row_black_key: [28, 32, 40],
                bg_keyboard_white: [216, 222, 233],
                bg_keyboard_black: [26, 30, 38],
                grid_line_bar: [60, 68, 84],
                grid_line_sub: [44, 50, 62],
                note_fill: [115, 172, 190],
                note_stroke: [136, 192, 208],
                note_selected_fill: [130, 185, 188],
                note_selected_stroke: [236, 239, 244],
                note_hover: [125, 182, 200],
                note_corner_radius: 6.0,
                note_stroke_width: 1.5,
                note_opacity: 1.0,
                note_waveform_opacity: 0.30,
                note_waveform_color: [20, 35, 45],
                pitch_curve_color: [165, 132, 160],
                pitch_anchor_color: [215, 185, 125],
                accent_color: [120, 175, 195],
                playhead_color: [185, 95, 105],
                text_primary: [225, 230, 238],
                text_muted: [140, 150, 170],
                text_note_tag: [20, 30, 40],
                ui_corner_radius: 6.0,
                panel_opacity: 1.0,
            },
            ThemePreset::MikanTeal => Self {
                preset: ThemePreset::MikanTeal,
                bg_canvas: [20, 30, 34],
                bg_panel: [26, 40, 46],
                bg_header: [34, 50, 58],
                bg_row_white_key: [28, 44, 50],
                bg_row_black_key: [18, 26, 30],
                bg_keyboard_white: [218, 230, 234],
                bg_keyboard_black: [16, 24, 28],
                grid_line_bar: [42, 65, 74],
                grid_line_sub: [28, 45, 52],
                note_fill: [48, 168, 160],
                note_stroke: [70, 195, 185],
                note_selected_fill: [85, 208, 198],
                note_selected_stroke: [240, 252, 250],
                note_hover: [60, 180, 172],
                note_corner_radius: 8.0,
                note_stroke_width: 1.6,
                note_opacity: 1.0,
                note_waveform_opacity: 0.30,
                note_waveform_color: [10, 45, 42],
                pitch_curve_color: [225, 115, 150],
                pitch_anchor_color: [230, 145, 175],
                accent_color: [55, 175, 166],
                playhead_color: [225, 115, 150],
                text_primary: [230, 242, 245],
                text_muted: [128, 165, 175],
                text_note_tag: [8, 28, 26],
                ui_corner_radius: 6.0,
                panel_opacity: 1.0,
            },
            ThemePreset::SolarizedDark => Self {
                preset: ThemePreset::SolarizedDark,
                bg_canvas: [4, 44, 54],
                bg_panel: [10, 54, 66],
                bg_header: [18, 66, 80],
                bg_row_white_key: [12, 58, 70],
                bg_row_black_key: [4, 38, 48],
                bg_keyboard_white: [225, 218, 198],
                bg_keyboard_black: [2, 32, 40],
                grid_line_bar: [24, 82, 98],
                grid_line_sub: [14, 58, 70],
                note_fill: [170, 130, 15],
                note_stroke: [195, 150, 25],
                note_selected_fill: [190, 75, 28],
                note_selected_stroke: [245, 238, 220],
                note_hover: [182, 140, 20],
                note_corner_radius: 5.0,
                note_stroke_width: 1.5,
                note_opacity: 1.0,
                note_waveform_opacity: 0.30,
                note_waveform_color: [40, 25, 0],
                pitch_curve_color: [45, 150, 142],
                pitch_anchor_color: [100, 106, 180],
                accent_color: [42, 130, 190],
                playhead_color: [205, 58, 54],
                text_primary: [240, 234, 215],
                text_muted: [125, 142, 142],
                text_note_tag: [240, 234, 215],
                ui_corner_radius: 4.0,
                panel_opacity: 1.0,
            },
            ThemePreset::CleanLight => Self {
                preset: ThemePreset::CleanLight,
                bg_canvas: [244, 245, 248],
                bg_panel: [234, 236, 242],
                bg_header: [222, 226, 234],
                bg_row_white_key: [250, 251, 254],
                bg_row_black_key: [238, 240, 246],
                bg_keyboard_white: [255, 255, 255],
                bg_keyboard_black: [70, 75, 86],
                grid_line_bar: [198, 204, 216],
                grid_line_sub: [220, 225, 234],
                note_fill: [45, 135, 172],
                note_stroke: [32, 115, 150],
                note_selected_fill: [55, 155, 195],
                note_selected_stroke: [15, 75, 100],
                note_hover: [50, 145, 182],
                note_corner_radius: 6.0,
                note_stroke_width: 1.6,
                note_opacity: 1.0,
                note_waveform_opacity: 0.30,
                note_waveform_color: [255, 255, 255],
                pitch_curve_color: [205, 70, 110],
                pitch_anchor_color: [145, 60, 175],
                accent_color: [45, 135, 172],
                playhead_color: [210, 55, 75],
                text_primary: [38, 44, 56],
                text_muted: [112, 120, 136],
                text_note_tag: [255, 255, 255],
                ui_corner_radius: 6.0,
                panel_opacity: 1.0,
            },
            ThemePreset::Custom => Self::from_preset(ThemePreset::PomarNeon),
        }
    }

    pub fn apply_preset(&mut self, preset: ThemePreset) {
        let new_theme = Self::from_preset(preset);
        *self = new_theme;
        self.preset = preset;
    }

    #[inline]
    pub fn c32(&self, rgb: [u8; 3]) -> Color32 {
        Color32::from_rgb(rgb[0], rgb[1], rgb[2])
    }

    #[inline]
    pub fn c32_alpha(&self, rgb: [u8; 3], alpha: f32) -> Color32 {
        Color32::from_rgba_unmultiplied(
            rgb[0],
            rgb[1],
            rgb[2],
            (alpha.clamp(0.0, 1.0) * 255.0) as u8,
        )
    }

    #[inline]
    pub fn rgb_to_c32(rgb: [u8; 3]) -> Color32 {
        Color32::from_rgb(rgb[0], rgb[1], rgb[2])
    }

    #[inline]
    pub fn rgb_to_c32_alpha(rgb: [u8; 3], alpha: f32) -> Color32 {
        Color32::from_rgba_unmultiplied(
            rgb[0],
            rgb[1],
            rgb[2],
            (alpha.clamp(0.0, 1.0) * 255.0) as u8,
        )
    }

    pub fn bg_canvas_c32(&self) -> Color32 {
        self.c32(self.bg_canvas)
    }

    pub fn bg_panel_c32(&self) -> Color32 {
        self.c32_alpha(self.bg_panel, self.panel_opacity)
    }

    pub fn bg_header_c32(&self) -> Color32 {
        self.c32(self.bg_header)
    }

    pub fn bg_row_white_key_c32(&self) -> Color32 {
        self.c32(self.bg_row_white_key)
    }

    pub fn bg_row_black_key_c32(&self) -> Color32 {
        self.c32(self.bg_row_black_key)
    }

    pub fn bg_keyboard_white_c32(&self) -> Color32 {
        self.c32(self.bg_keyboard_white)
    }

    pub fn bg_keyboard_black_c32(&self) -> Color32 {
        self.c32(self.bg_keyboard_black)
    }

    pub fn grid_line_bar_c32(&self) -> Color32 {
        self.c32(self.grid_line_bar)
    }

    pub fn grid_line_sub_c32(&self) -> Color32 {
        self.c32(self.grid_line_sub)
    }

    pub fn note_fill_c32(&self) -> Color32 {
        self.c32_alpha(self.note_fill, self.note_opacity)
    }

    pub fn note_stroke_c32(&self) -> Color32 {
        self.c32(self.note_stroke)
    }

    pub fn note_selected_fill_c32(&self) -> Color32 {
        self.c32_alpha(self.note_selected_fill, self.note_opacity)
    }

    pub fn note_selected_stroke_c32(&self) -> Color32 {
        self.c32(self.note_selected_stroke)
    }

    pub fn note_hover_c32(&self) -> Color32 {
        self.c32_alpha(self.note_hover, self.note_opacity)
    }

    pub fn note_rounding(&self) -> Rounding {
        Rounding::same(self.note_corner_radius.max(0.0))
    }

    pub fn note_stroke(&self, selected: bool) -> Stroke {
        let color = if selected {
            self.note_selected_stroke_c32()
        } else {
            self.note_stroke_c32()
        };
        Stroke::new(self.note_stroke_width.max(0.5), color)
    }

    pub fn note_highlight_c32(&self, is_selected: bool) -> Color32 {
        let base = if is_selected {
            self.note_selected_fill
        } else {
            self.note_fill
        };
        let r = base[0].saturating_add(30);
        let g = base[1].saturating_add(30);
        let b = base[2].saturating_add(30);
        Color32::from_rgba_unmultiplied(r, g, b, 140)
    }

    pub fn note_shadow_c32(&self, is_selected: bool) -> Color32 {
        let base = if is_selected {
            self.note_selected_fill
        } else {
            self.note_fill
        };
        let r = base[0].saturating_sub(35);
        let g = base[1].saturating_sub(35);
        let b = base[2].saturating_sub(35);
        Color32::from_rgba_unmultiplied(r, g, b, 180)
    }

    pub fn note_glow_c32(&self) -> Color32 {
        self.c32_alpha(self.accent_color, 0.20)
    }

    pub fn key_active_c32(&self) -> Color32 {
        self.c32_alpha(self.accent_color, 0.60)
    }

    pub fn scale_out_of_key_tint(&self) -> Color32 {
        Color32::from_rgba_unmultiplied(10, 8, 16, 80)
    }

    pub fn scale_tonic_tint(&self) -> Color32 {
        self.c32_alpha(self.accent_color, 0.08)
    }

    pub fn pitch_curve_c32(&self) -> Color32 {
        self.c32(self.pitch_curve_color)
    }

    pub fn pitch_anchor_c32(&self) -> Color32 {
        self.c32(self.pitch_anchor_color)
    }

    pub fn accent_c32(&self) -> Color32 {
        self.c32(self.accent_color)
    }

    pub fn playhead_c32(&self) -> Color32 {
        self.c32(self.playhead_color)
    }

    pub fn text_primary_c32(&self) -> Color32 {
        self.c32(self.text_primary)
    }

    pub fn text_muted_c32(&self) -> Color32 {
        self.c32(self.text_muted)
    }

    pub fn text_note_tag_c32(&self) -> Color32 {
        self.c32(self.text_note_tag)
    }

    pub fn ui_rounding(&self) -> Rounding {
        Rounding::same(self.ui_corner_radius.max(0.0))
    }

    pub fn is_light(&self) -> bool {
        self.preset == ThemePreset::CleanLight
    }

    pub fn card_bg_c32(&self) -> Color32 {
        if self.is_light() {
            self.c32(self.bg_header)
        } else {
            self.c32_alpha(self.bg_canvas, 0.9)
        }
    }

    pub fn card_stroke(&self) -> Stroke {
        Stroke::new(1.0, self.grid_line_sub_c32())
    }

    pub fn card_border_c32(&self) -> Color32 {
        self.grid_line_bar_c32()
    }

    pub fn active_border_stroke(&self) -> Stroke {
        Stroke::new(1.5, self.accent_c32())
    }

    pub fn create_egui_visuals(&self) -> egui::Visuals {
        let is_light = self.is_light();
        let mut visuals = if is_light {
            egui::Visuals::light()
        } else {
            egui::Visuals::dark()
        };

        let panel_bg = self.bg_panel_c32();
        let header_bg = self.bg_header_c32();
        let canvas_bg = self.bg_canvas_c32();
        let accent = self.accent_c32();
        let text_p = self.text_primary_c32();
        let text_m = self.text_muted_c32();
        let grid_bar = self.grid_line_bar_c32();
        let grid_sub = self.grid_line_sub_c32();
        let round = self.ui_rounding();

        visuals.panel_fill = panel_bg;
        visuals.window_fill = panel_bg;
        visuals.window_rounding = round;
        visuals.menu_rounding = round;
        visuals.window_stroke = Stroke::new(1.0, grid_bar);
        visuals.extreme_bg_color = canvas_bg;
        visuals.faint_bg_color = header_bg;
        visuals.code_bg_color = canvas_bg;
        visuals.hyperlink_color = accent;

        visuals.widgets.noninteractive.bg_fill = panel_bg;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, grid_sub);
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, text_m);
        visuals.widgets.noninteractive.rounding = round;

        visuals.widgets.inactive.bg_fill = header_bg;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, grid_sub);
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, text_p);
        visuals.widgets.inactive.rounding = round;

        let hovered_bg = if is_light {
            Color32::from_rgb(
                self.bg_header[0].saturating_sub(15),
                self.bg_header[1].saturating_sub(15),
                self.bg_header[2].saturating_sub(15),
            )
        } else {
            Color32::from_rgb(
                self.bg_header[0].saturating_add(15),
                self.bg_header[1].saturating_add(15),
                self.bg_header[2].saturating_add(15),
            )
        };
        visuals.widgets.hovered.bg_fill = hovered_bg;
        visuals.widgets.hovered.bg_stroke =
            Stroke::new(1.0, self.c32_alpha(self.accent_color, 0.8));
        visuals.widgets.hovered.fg_stroke = Stroke::new(
            1.0,
            if is_light {
                Color32::BLACK
            } else {
                Color32::WHITE
            },
        );
        visuals.widgets.hovered.rounding = round;

        visuals.widgets.active.bg_fill = self.c32_alpha(self.accent_color, 0.35);
        visuals.widgets.active.bg_stroke = Stroke::new(1.2, accent);
        visuals.widgets.active.fg_stroke = Stroke::new(1.2, text_p);
        visuals.widgets.active.rounding = round;

        visuals.widgets.open.bg_fill = header_bg;
        visuals.widgets.open.bg_stroke = Stroke::new(1.0, accent);
        visuals.widgets.open.fg_stroke = Stroke::new(1.0, text_p);
        visuals.widgets.open.rounding = round;

        visuals.selection.bg_fill = self.c32_alpha(self.accent_color, 0.25);
        visuals.selection.stroke = Stroke::new(1.0, accent);

        visuals.override_text_color = None;
        visuals.warn_fg_color = Color32::from_rgb(235, 170, 60);
        visuals.error_fg_color = Color32::from_rgb(235, 85, 85);

        visuals
    }
}

pub struct MelodyneTheme;

impl MelodyneTheme {
    pub const BG_CANVAS: Color32 = Color32::from_rgb(20, 18, 28);
    pub const BG_ROW_WHITE_KEY: Color32 = Color32::from_rgb(26, 23, 36);
    pub const BG_ROW_BLACK_KEY: Color32 = Color32::from_rgb(18, 16, 25);
    pub const BG_PANEL: Color32 = Color32::from_rgb(28, 25, 38);
    pub const BG_HEADER: Color32 = Color32::from_rgb(36, 32, 48);
    pub const BG_KEYBOARD_BLACK: Color32 = Color32::from_rgb(24, 21, 32);
    pub const BG_KEYBOARD_WHITE: Color32 = Color32::from_rgb(215, 212, 225);

    pub const GRID_LINE_BAR: Color32 = Color32::from_rgb(48, 42, 64);
    pub const GRID_LINE_SUB: Color32 = Color32::from_rgb(32, 28, 44);

    pub const NOTE_GOLD_FILL: Color32 = Color32::from_rgb(34, 180, 126);
    pub const NOTE_GOLD_HOVER: Color32 = Color32::from_rgb(40, 195, 138);
    pub const NOTE_GOLD_STROKE: Color32 = Color32::from_rgb(45, 205, 145);
    pub const NOTE_SELECTED_GOLD: Color32 = Color32::from_rgb(52, 212, 155);

    pub const PITCH_ARM_GOLD: Color32 = Color32::from_rgb(185, 155, 230);
    pub const PITCH_ANCHOR_CYAN: Color32 = Color32::from_rgb(165, 130, 215);

    pub const TEXT_GOLD_LABEL: Color32 = Color32::from_rgb(230, 228, 238);
    pub const TEXT_NOTE_TAG: Color32 = Color32::from_rgb(12, 28, 20);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(140, 132, 160);

    pub const ACCENT_GOLD: Color32 = Color32::from_rgb(160, 130, 215);
    pub const ACCENT_CYAN: Color32 = Color32::from_rgb(70, 205, 230);
    pub const PLAYHEAD_RED: Color32 = Color32::from_rgb(45, 205, 145);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_presets_all_valid() {
        for preset in ThemePreset::ALL {
            let theme = ThemeConfig::from_preset(preset);
            assert_eq!(theme.preset, preset);
            assert!(theme.note_corner_radius >= 0.0);
            assert!(theme.note_stroke_width >= 0.5);
            assert!(theme.note_opacity >= 0.2 && theme.note_opacity <= 1.0);
            let _visuals = theme.create_egui_visuals();
        }
    }

    #[test]
    fn test_theme_config_serialization_roundtrip() {
        let theme = ThemeConfig::from_preset(ThemePreset::Cyberpunk);
        let json = serde_json::to_string(&theme).expect("serialize");
        let deserialized: ThemeConfig = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(deserialized.preset, ThemePreset::Cyberpunk);
        assert_eq!(deserialized.note_fill, [210, 55, 120]);
        assert_eq!(deserialized.accent_color, [70, 205, 230]);
    }
}
