use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum GridSnapOption {
    #[default]
    Auto,
    Freeform,
    Snap1_1,
    Snap1_2,
    Snap1_4,
    Snap1_8,
    Snap1_16,
    Snap1_32,
    Snap1_64,
    Snap1_128,
    // Tercinas / Triplets
    Snap1_4T,
    Snap1_8T,
    Snap1_16T,
    Snap1_32T,
    Snap1_64T,
}

impl GridSnapOption {
    pub fn step_ms(&self, bpm: f64) -> Option<f64> {
        self.step_ms_with_zoom(bpm, 0.25)
    }

    pub fn step_ms_with_zoom(&self, bpm: f64, px_per_ms: f32) -> Option<f64> {
        let beat_ms = 60000.0 / bpm;
        match self {
            GridSnapOption::Auto => {
                let candidates = [
                    beat_ms * 4.0,  // 1/1 (Bar)
                    beat_ms * 2.0,  // 1/2
                    beat_ms,        // 1/4 (Beat)
                    beat_ms / 2.0,  // 1/8
                    beat_ms / 4.0,  // 1/16
                    beat_ms / 8.0,  // 1/32
                    beat_ms / 16.0, // 1/64
                    beat_ms / 32.0, // 1/128
                ];
                let px_zoom = (px_per_ms as f64).max(0.001);
                let min_px = 18.0;
                let mut chosen = candidates[0];
                for &cand in &candidates {
                    if cand * px_zoom >= min_px {
                        chosen = cand;
                    }
                }
                Some(chosen)
            }
            GridSnapOption::Freeform => None,
            GridSnapOption::Snap1_1 => Some(beat_ms * 4.0),
            GridSnapOption::Snap1_2 => Some(beat_ms * 2.0),
            GridSnapOption::Snap1_4 => Some(beat_ms),
            GridSnapOption::Snap1_8 => Some(beat_ms / 2.0),
            GridSnapOption::Snap1_16 => Some(beat_ms / 4.0),
            GridSnapOption::Snap1_32 => Some(beat_ms / 8.0),
            GridSnapOption::Snap1_64 => Some(beat_ms / 16.0),
            GridSnapOption::Snap1_128 => Some(beat_ms / 32.0),
            GridSnapOption::Snap1_4T => Some(beat_ms * 4.0 / 6.0),
            GridSnapOption::Snap1_8T => Some(beat_ms * 4.0 / 12.0),
            GridSnapOption::Snap1_16T => Some(beat_ms * 4.0 / 24.0),
            GridSnapOption::Snap1_32T => Some(beat_ms * 4.0 / 48.0),
            GridSnapOption::Snap1_64T => Some(beat_ms * 4.0 / 96.0),
        }
    }

    pub fn label(&self) -> &'static str {
        self.label_for(crate::config::AppLanguage::PtBr)
    }

    pub fn label_for(&self, lang: crate::config::AppLanguage) -> &'static str {
        match self {
            GridSnapOption::Auto => "Auto",
            GridSnapOption::Freeform => {
                if lang.is_en() {
                    "Off"
                } else {
                    "Livre"
                }
            }
            GridSnapOption::Snap1_1 => "1/1",
            GridSnapOption::Snap1_2 => "1/2",
            GridSnapOption::Snap1_4 => "1/4",
            GridSnapOption::Snap1_8 => "1/8",
            GridSnapOption::Snap1_16 => "1/16",
            GridSnapOption::Snap1_32 => "1/32",
            GridSnapOption::Snap1_64 => "1/64",
            GridSnapOption::Snap1_128 => "1/128",
            GridSnapOption::Snap1_4T => "1/4T",
            GridSnapOption::Snap1_8T => "1/8T",
            GridSnapOption::Snap1_16T => "1/16T",
            GridSnapOption::Snap1_32T => "1/32T",
            GridSnapOption::Snap1_64T => "1/64T",
        }
    }

    pub fn resolved_label(&self, bpm: f64, px_per_ms: f32) -> String {
        self.resolved_label_for(bpm, px_per_ms, crate::config::AppLanguage::PtBr)
    }

    pub fn resolved_label_for(&self, bpm: f64, px_per_ms: f32, lang: crate::config::AppLanguage) -> String {
        if *self == GridSnapOption::Auto {
            if let Some(step) = self.step_ms_with_zoom(bpm, px_per_ms) {
                let beat_ms = 60000.0 / bpm;
                let ratio = beat_ms / step;
                let fraction_name = if (ratio - 0.25).abs() < 1e-3 {
                    "1/1"
                } else if (ratio - 0.5).abs() < 1e-3 {
                    "1/2"
                } else if (ratio - 1.0).abs() < 1e-3 {
                    "1/4"
                } else if (ratio - 2.0).abs() < 1e-3 {
                    "1/8"
                } else if (ratio - 4.0).abs() < 1e-3 {
                    "1/16"
                } else if (ratio - 8.0).abs() < 1e-3 {
                    "1/32"
                } else if (ratio - 16.0).abs() < 1e-3 {
                    "1/64"
                } else if (ratio - 32.0).abs() < 1e-3 {
                    "1/128"
                } else {
                    ""
                };
                if !fraction_name.is_empty() {
                    return format!("Auto ({})", fraction_name);
                }
            }
            "Auto".to_string()
        } else {
            self.label_for(lang).to_string()
        }
    }
}

pub struct TransportState {
    pub bpm: f64,
    pub voicebank_name: String,
    pub voicebank_path: Option<PathBuf>,
    pub status_message: String,
    pub grid_snap: GridSnapOption,
    pub playhead_time_str: String,
    pub render_progress: f32,
    pub loop_enabled: bool,
    pub loop_start_ms: f64,
    pub loop_end_ms: f64,
    pub metronome_enabled: bool,
    pub count_in_bars: u8,
    pub preview_selection_only: bool,
    pub master_volume: f32,
    pub vu_level_l: f32,
    pub vu_level_r: f32,
    pub vu_peak_l: f32,
    pub vu_peak_r: f32,
}

impl Default for TransportState {
    fn default() -> Self {
        Self {
            bpm: 120.0,
            voicebank_name: String::new(),
            voicebank_path: None,
            status_message: "Pronto".to_string(),
            grid_snap: GridSnapOption::default(),
            playhead_time_str: "00:00.000".to_string(),
            render_progress: 1.0,
            loop_enabled: false,
            loop_start_ms: 0.0,
            loop_end_ms: 8000.0,
            metronome_enabled: false,
            count_in_bars: 0,
            preview_selection_only: false,
            master_volume: 1.0,
            vu_level_l: 0.0,
            vu_level_r: 0.0,
            vu_peak_l: 0.0,
            vu_peak_r: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EditTool {
    #[default]
    Pointer, // Select / Move / Resize
    Pencil,    // Draw new notes
    PitchDraw, // Draw pitch curve splines
    Slice,     // Split note into two
    Eraser,    // Delete notes
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PitchSubTool {
    #[default]
    Freehand, // Desenho livre com suavização
    Line,    // Reta / Glissando linear
    Vibrato, // Pincel de vibrato senoidal
    Smooth,  // Pincel suavizador (moving average)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AutoScrollMode {
    #[default]
    PageScroll, // Rolagem de Página (pula de página ao atingir o fim da tela)
    StationaryCursor, // Cursor Estacionário (mantém cursor visível e rola continuamente)
    Off,              // Desligar (rolagem manual livre)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RightSidebarTab {
    #[default]
    SingerTrack,
    Note,
    Phonemes,
    Engine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub enum ExportAudioScope {
    #[default]
    VocalsAndAudio,
    VocalsOnly,
    SeparateTrackStems,
}
