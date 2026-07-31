use eframe::egui::{Color32, Stroke};

pub struct MelodyneTheme;

impl MelodyneTheme {
    // 1. Neo-Brutalist Dark Canvas & Surfaces (Obsidian Midnight & Dark Charcoal Violet)
    pub const BG_CANVAS: Color32 = Color32::from_rgb(14, 12, 21);
    pub const BG_PANEL: Color32 = Color32::from_rgb(21, 18, 33);
    pub const BG_HEADER: Color32 = Color32::from_rgb(30, 26, 46);
    pub const BG_CARD: Color32 = Color32::from_rgb(36, 30, 54);
    pub const BG_KEYBOARD_BLACK: Color32 = Color32::from_rgb(18, 15, 27);
    pub const BG_KEYBOARD_WHITE: Color32 = Color32::from_rgb(238, 235, 245);

    // 2. Razor-Sharp Thin Delicate Borders (1.0px - 1.2px)
    pub const BORDER_FINE: Color32 = Color32::from_rgb(58, 49, 84);
    pub const BORDER_BRIGHT: Color32 = Color32::from_rgb(90, 78, 125);
    pub const BORDER_GOLD: Color32 = Color32::from_rgb(255, 208, 0);
    pub const BORDER_CYAN: Color32 = Color32::from_rgb(0, 255, 157);
    pub const BORDER_MAGENTA: Color32 = Color32::from_rgb(255, 42, 133);

    // 3. Grid Lines (High Precision Contrast)
    pub const GRID_LINE_BAR: Color32 = Color32::from_rgb(52, 43, 76);
    pub const GRID_LINE_SUB: Color32 = Color32::from_rgb(28, 24, 42);

    // 4. Note Blocks (Punchy Neo Yellow Fill with Crisp Dark Outline)
    pub const NOTE_GOLD_FILL: Color32 = Color32::from_rgb(255, 208, 0);
    pub const NOTE_GOLD_HOVER: Color32 = Color32::from_rgb(255, 224, 71);
    pub const NOTE_GOLD_STROKE: Color32 = Color32::from_rgb(16, 13, 0);
    pub const NOTE_SELECTED_GOLD: Color32 = Color32::from_rgb(0, 255, 157);

    // 5. Pitch Curves & Control Nodes (Vibrant Cyan / Magenta)
    pub const PITCH_ARM_GOLD: Color32 = Color32::from_rgb(0, 255, 157);
    pub const PITCH_ANCHOR_CYAN: Color32 = Color32::from_rgb(255, 42, 133);

    // 6. Typography Contrast
    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(255, 255, 255);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(213, 206, 232);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(140, 130, 170);
    pub const TEXT_GOLD_LABEL: Color32 = Color32::from_rgb(255, 208, 0);
    pub const TEXT_NOTE_TAG: Color32 = Color32::from_rgb(16, 13, 0);

    // 7. Neo Accents
    pub const ACCENT_GOLD: Color32 = Color32::from_rgb(255, 208, 0);
    pub const ACCENT_CYAN: Color32 = Color32::from_rgb(0, 255, 157);
    pub const ACCENT_MAGENTA: Color32 = Color32::from_rgb(255, 42, 133);
    pub const PLAYHEAD_RED: Color32 = Color32::from_rgb(255, 42, 133);

    // Helper functions for delicate thin strokes
    pub fn stroke_thin() -> Stroke {
        Stroke::new(1.0, Self::BORDER_FINE)
    }

    pub fn stroke_gold() -> Stroke {
        Stroke::new(1.2, Self::BORDER_GOLD)
    }

    pub fn stroke_cyan() -> Stroke {
        Stroke::new(1.2, Self::BORDER_CYAN)
    }
}
