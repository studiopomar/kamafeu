use crate::project::model::UNote;
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutoPitchStyle {
    Natural,
    Expressive,
    SmoothPop,
    RockPower,
    RnBSoul,
    HardTune,
    AcousticFolk,
    LyricalOpera,
}

pub fn apply_autopitch_to_selection(
    notes: &mut [UNote],
    selection: &HashSet<usize>,
    style: AutoPitchStyle,
) {
    if notes.is_empty() {
        return;
    }

    let (preset, dyn_shaping, breath, cvel, atk_dec) = match style {
        AutoPitchStyle::Natural => (
            crate::dsp::AutoPitchPreset::NaturalPop,
            false,
            false,
            false,
            false,
        ),
        AutoPitchStyle::Expressive => (
            crate::dsp::AutoPitchPreset::BalladExpressive,
            true,
            true,
            false,
            true,
        ),
        AutoPitchStyle::SmoothPop => (
            crate::dsp::AutoPitchPreset::JPopAnime,
            true,
            false,
            true,
            true,
        ),
        AutoPitchStyle::RockPower => (
            crate::dsp::AutoPitchPreset::RockPower,
            true,
            false,
            true,
            true,
        ),
        AutoPitchStyle::RnBSoul => (crate::dsp::AutoPitchPreset::RnBSoul, true, true, true, true),
        AutoPitchStyle::HardTune => (
            crate::dsp::AutoPitchPreset::HardTuneModern,
            true,
            false,
            true,
            true,
        ),
        AutoPitchStyle::AcousticFolk => (
            crate::dsp::AutoPitchPreset::FolkAcoustic,
            true,
            true,
            false,
            true,
        ),
        AutoPitchStyle::LyricalOpera => (
            crate::dsp::AutoPitchPreset::LyricalOpera,
            true,
            false,
            false,
            true,
        ),
    };

    let options = crate::dsp::AutoPitchOptions {
        preset,
        intensity: 1.0,
        enable_attack_scoop: true,
        enable_overshoot: true,
        enable_release_drop: true,
        enable_vibrato: true,
        enable_dynamics_shaping: dyn_shaping,
        enable_breathiness: breath,
        enable_consonant_velocity: cvel,
        enable_attack_decay: atk_dec,
        portamento_speed_ms: 0.0,
        vibrato_depth_mult: 1.0,
        vibrato_period_mult: 1.0,
        expression_amount: 1.0,
    };

    let sel_vec: Option<Vec<usize>> = if selection.is_empty() {
        None
    } else {
        Some(selection.iter().copied().collect())
    };

    crate::dsp::AutoPitchEngine::apply_to_notes(notes, sel_vec.as_deref(), &options);
}
