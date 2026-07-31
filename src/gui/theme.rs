use eframe::egui::Color32;

pub struct MelodyneTheme;

impl MelodyneTheme {
    // Cyberpunk High-Contrast Dark Mode Backgrounds (Deep Midnight & Royal Purple)
    pub const BG_CANVAS: Color32 = Color32::from_rgb(18, 14, 28);
    pub const BG_PANEL: Color32 = Color32::from_rgb(26, 20, 38);
    pub const BG_HEADER: Color32 = Color32::from_rgb(36, 27, 53);
    pub const BG_KEYBOARD_BLACK: Color32 = Color32::from_rgb(22, 17, 34);
    pub const BG_KEYBOARD_WHITE: Color32 = Color32::from_rgb(232, 227, 245);

    // Ultra-Clear High Contrast Grid lines
    pub const GRID_LINE_BAR: Color32 = Color32::from_rgb(61, 46, 84);
    pub const GRID_LINE_SUB: Color32 = Color32::from_rgb(37, 28, 54);

    // Note Blobs (Electric Neon Mint Green)
    pub const NOTE_GOLD_FILL: Color32 = Color32::from_rgb(0, 230, 138);
    pub const NOTE_GOLD_HOVER: Color32 = Color32::from_rgb(51, 255, 166);
    pub const NOTE_GOLD_STROKE: Color32 = Color32::from_rgb(0, 255, 157);
    pub const NOTE_SELECTED_GOLD: Color32 = Color32::from_rgb(0, 255, 157);

    // Pitch Curves & Arms (Bright Lavender & Royal Purple)
    pub const PITCH_ARM_GOLD: Color32 = Color32::from_rgb(216, 180, 254);
    pub const PITCH_ANCHOR_CYAN: Color32 = Color32::from_rgb(192, 132, 252);

    // High-Contrast Text Colors
    pub const TEXT_GOLD_LABEL: Color32 = Color32::from_rgb(255, 255, 255);
    pub const TEXT_NOTE_TAG: Color32 = Color32::from_rgb(10, 28, 18);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(165, 148, 201);

    // Accents
    pub const ACCENT_GOLD: Color32 = Color32::from_rgb(192, 132, 252);
    pub const PLAYHEAD_RED: Color32 = Color32::from_rgb(0, 255, 157);
}
