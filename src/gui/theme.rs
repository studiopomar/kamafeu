use eframe::egui::Color32;

pub struct MelodyneTheme;

impl MelodyneTheme {
    pub const BG_CANVAS: Color32 = Color32::from_rgb(14, 11, 22);
    pub const BG_ROW_WHITE_KEY: Color32 = Color32::from_rgb(27, 21, 41);
    pub const BG_ROW_BLACK_KEY: Color32 = Color32::from_rgb(15, 11, 23);
    pub const BG_PANEL: Color32 = Color32::from_rgb(26, 20, 38);
    pub const BG_HEADER: Color32 = Color32::from_rgb(36, 27, 53);
    pub const BG_KEYBOARD_BLACK: Color32 = Color32::from_rgb(18, 14, 28);
    pub const BG_KEYBOARD_WHITE: Color32 = Color32::from_rgb(232, 227, 245);

    pub const GRID_LINE_BAR: Color32 = Color32::from_rgb(68, 52, 95);
    pub const GRID_LINE_SUB: Color32 = Color32::from_rgb(42, 32, 60);

    pub const NOTE_GOLD_FILL: Color32 = Color32::from_rgb(0, 230, 138);
    pub const NOTE_GOLD_HOVER: Color32 = Color32::from_rgb(51, 255, 166);
    pub const NOTE_GOLD_STROKE: Color32 = Color32::from_rgb(0, 255, 157);
    pub const NOTE_SELECTED_GOLD: Color32 = Color32::from_rgb(0, 255, 157);

    pub const PITCH_ARM_GOLD: Color32 = Color32::from_rgb(216, 180, 254);
    pub const PITCH_ANCHOR_CYAN: Color32 = Color32::from_rgb(192, 132, 252);

    pub const TEXT_GOLD_LABEL: Color32 = Color32::from_rgb(255, 255, 255);
    pub const TEXT_NOTE_TAG: Color32 = Color32::from_rgb(10, 28, 18);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(165, 148, 201);

    pub const ACCENT_GOLD: Color32 = Color32::from_rgb(192, 132, 252);
    pub const PLAYHEAD_RED: Color32 = Color32::from_rgb(0, 255, 157);
}
