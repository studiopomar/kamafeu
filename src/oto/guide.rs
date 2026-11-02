//! UTAU Guide: Voicebank acoustic characteristic analysis and automated note calibration.
//!
//! Analyzes an active or installed UTAU voicebank (`oto.ini` entries, preutterance, overlap,
//! consonant lengths, bank style: CV, VCV, CVVC) and calibrates note timing, envelope attacks,
//! consonant velocity, and portamento pitch transitions to match that specific singer.

use super::Voicebank;
use crate::dsp::pitch::note_name_to_midi;
use crate::project::model::{UNote, UPitchBendPoint};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VoicebankStyle {
    Vcv,
    Cv,
    Cvvc,
    Unknown,
}

impl VoicebankStyle {
    pub fn label(self) -> &'static str {
        match self {
            Self::Vcv => "VCV (Vocal Contínuo)",
            Self::Cv => "CV (Consonantal-Vogal Padrão)",
            Self::Cvvc => "CVVC (Dífonos / Rápido)",
            Self::Unknown => "Genérico / Desconhecido",
        }
    }

    pub fn label_en(self) -> &'static str {
        match self {
            Self::Vcv => "VCV (Continuous Vocal)",
            Self::Cv => "CV (Standard Consonant-Vowel)",
            Self::Cvvc => "CVVC (Diphone / Fast)",
            Self::Unknown => "Generic / Unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UtauGuideAnalysis {
    pub style: VoicebankStyle,
    pub total_entries: usize,
    pub median_preutterance_ms: f64,
    pub median_overlap_ms: f64,
    pub median_consonant_ms: f64,
    pub recommended_portamento_start_ratio: f64,
    pub recommended_portamento_length_ms: f64,
    pub recommended_consonant_velocity: f64,
    pub recommended_attack_ms: f64,
    pub recommended_release_ms: f64,
}

impl Default for UtauGuideAnalysis {
    fn default() -> Self {
        Self {
            style: VoicebankStyle::Cv,
            total_entries: 0,
            median_preutterance_ms: 60.0,
            median_overlap_ms: 25.0,
            median_consonant_ms: 120.0,
            recommended_portamento_start_ratio: -0.75,
            recommended_portamento_length_ms: 80.0,
            recommended_consonant_velocity: 110.0,
            recommended_attack_ms: 10.0,
            recommended_release_ms: 35.0,
        }
    }
}

fn median(mut list: Vec<f64>) -> f64 {
    if list.is_empty() {
        return 0.0;
    }
    list.sort_by(|a, b| a.total_cmp(b));
    let mid = list.len() / 2;
    if list.len() % 2 == 0 {
        (list[mid - 1] + list[mid]) * 0.5
    } else {
        list[mid]
    }
}

/// Analyzes an installed voicebank's `oto.ini` entries to detect its singing style and acoustic characteristics.
pub fn analyze_voicebank(bank: &Voicebank) -> UtauGuideAnalysis {
    let mut preutter_vals = Vec::new();
    let mut overlap_vals = Vec::new();
    let mut consonant_vals = Vec::new();
    let mut vcv_indicators = 0usize;
    let mut cvvc_indicators = 0usize;

    for entry in bank.entries.values() {
        if entry.preutterance > 0.0 {
            preutter_vals.push(entry.preutterance);
        }
        if entry.overlap > 0.0 {
            overlap_vals.push(entry.overlap);
        }
        if entry.consonant > 0.0 {
            consonant_vals.push(entry.consonant);
        }

        let alias = entry.alias.trim();
        // VCV indicator: whitespace separating vowel prefix or - prefix, e.g. "- か", "a か", "k か"
        if alias.contains(' ') || alias.starts_with("- ") || alias.starts_with("-_") {
            vcv_indicators += 1;
        } else if alias.ends_with(" -")
            || alias.ends_with(" R")
            || (alias.len() > 2 && alias.ends_with('-'))
        {
            cvvc_indicators += 1;
        }
    }

    let total_entries = bank.entries.len();
    let style = if total_entries > 0 && (vcv_indicators as f64 / total_entries as f64) > 0.25 {
        VoicebankStyle::Vcv
    } else if total_entries > 0 && (cvvc_indicators as f64 / total_entries as f64) > 0.20 {
        VoicebankStyle::Cvvc
    } else if total_entries > 0 {
        VoicebankStyle::Cv
    } else {
        VoicebankStyle::Unknown
    };

    let median_preutter = median(preutter_vals).clamp(10.0, 350.0);
    let median_overlap = median(overlap_vals).clamp(5.0, 200.0);
    let median_consonant = median(consonant_vals).clamp(15.0, 500.0);

    let (port_start_ratio, port_len_ms, vel, attack_ms, release_ms) = match style {
        VoicebankStyle::Vcv => (
            -0.50,
            (median_overlap * 2.5).clamp(60.0, 140.0),
            100.0,
            median_overlap.max(8.0),
            40.0,
        ),
        VoicebankStyle::Cv => (
            -0.80,
            (median_preutter * 1.2).clamp(50.0, 110.0),
            115.0,
            6.0,
            35.0,
        ),
        VoicebankStyle::Cvvc => (
            -0.65,
            (median_preutter * 1.4).clamp(55.0, 120.0),
            105.0,
            8.0,
            30.0,
        ),
        VoicebankStyle::Unknown => (-0.70, 75.0, 100.0, 8.0, 35.0),
    };

    UtauGuideAnalysis {
        style,
        total_entries,
        median_preutterance_ms: median_preutter,
        median_overlap_ms: median_overlap,
        median_consonant_ms: median_consonant,
        recommended_portamento_start_ratio: port_start_ratio,
        recommended_portamento_length_ms: port_len_ms,
        recommended_consonant_velocity: vel,
        recommended_attack_ms: attack_ms,
        recommended_release_ms: release_ms,
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UtauGuideOptions {
    pub strength: f64,
    pub apply_timing: bool,
    pub apply_pitch_portamento: bool,
    pub apply_envelope_shape: bool,
    pub apply_consonant_velocity: bool,
}

impl Default for UtauGuideOptions {
    fn default() -> Self {
        Self {
            strength: 1.0,
            apply_timing: true,
            apply_pitch_portamento: true,
            apply_envelope_shape: true,
            apply_consonant_velocity: true,
        }
    }
}

/// Applies the analyzed UTAU Guide settings to a sequence of notes.
pub fn apply_utau_guide(
    notes: &mut [UNote],
    analysis: &UtauGuideAnalysis,
    options: &UtauGuideOptions,
) {
    if notes.is_empty() {
        return;
    }

    let strength = options.strength.clamp(0.2, 2.0);

    // Sort or preserve chronological order
    for i in 0..notes.len() {
        if options.apply_consonant_velocity {
            let vel = 100.0 + (analysis.recommended_consonant_velocity - 100.0) * strength;
            notes[i].expressions.consonant_velocity = vel.clamp(50.0, 200.0);
        }

        if options.apply_envelope_shape {
            let atk = (analysis.recommended_attack_ms * strength).clamp(2.0, 150.0);
            let rel = (analysis.recommended_release_ms * strength).clamp(5.0, 250.0);
            notes[i].envelope.p2 = atk;
            notes[i].envelope.p5 = rel;
            notes[i].expressions.attack = if analysis.style == VoicebankStyle::Cv {
                (100.0 + 20.0 * strength).clamp(80.0, 160.0)
            } else {
                100.0
            };
        }

        if options.apply_timing {
            // Calibrate preutter offset to voicebank median
            let offset =
                (analysis.median_preutterance_ms * (1.0 - strength) * 0.2).clamp(-30.0, 30.0);
            notes[i].expressions.preutter_offset_ms = offset;
        }

        if options.apply_pitch_portamento && i > 0 {
            let prev_pitch = note_name_to_midi(&notes[i - 1].pitch).unwrap_or(60);
            let cur_pitch = note_name_to_midi(&notes[i].pitch).unwrap_or(60);
            let pitch_delta = (prev_pitch as f64 - cur_pitch as f64) * 100.0;

            let prev_end = notes[i - 1].position_ms + notes[i - 1].duration_ms;
            let gap_ms = notes[i].position_ms - prev_end;

            // If notes are connected or legato (gap < 120ms)
            if gap_ms <= 120.0 && pitch_delta.abs() > 10.0 {
                let port_len =
                    (analysis.recommended_portamento_length_ms * strength).clamp(30.0, 200.0);
                let start_offset = analysis.recommended_portamento_start_ratio * port_len;

                notes[i].pitch_bend.portamento_start_ms = start_offset;
                notes[i].pitch_bend.portamento_length_ms = port_len;
                notes[i].pitch_bend.portamento_shape = if analysis.style == VoicebankStyle::Vcv {
                    "s".to_string() // Smooth S-curve
                } else {
                    "j".to_string() // Ease-in
                };

                // Clear manual points in favor of clean bank-matched portamento curve
                notes[i].pitch_bend.points.clear();
                notes[i].pitch_bend.points.push(UPitchBendPoint {
                    time_offset_ms: start_offset,
                    pitch_offset_cents: pitch_delta,
                    shape: "s".to_string(),
                });
                notes[i].pitch_bend.points.push(UPitchBendPoint {
                    time_offset_ms: start_offset + port_len,
                    pitch_offset_cents: 0.0,
                    shape: "s".to_string(),
                });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oto::OtoEntry;

    #[test]
    fn analyzes_vcv_and_cv_voicebanks_correctly() {
        let dir1 = tempfile::tempdir().unwrap();
        let dir2 = tempfile::tempdir().unwrap();
        let mut vcv_bank = Voicebank::new(dir1.path()).unwrap();
        let mut cv_bank = Voicebank::new(dir2.path()).unwrap();

        for i in 0..10 {
            vcv_bank.entries.insert(
                format!("a ka_{i}"),
                OtoEntry {
                    wav_filename: format!("ka_{i}.wav"),
                    alias: format!("a ka_{i}"),
                    preutterance: 120.0,
                    overlap: 50.0,
                    consonant: 180.0,
                    ..Default::default()
                },
            );
            cv_bank.entries.insert(
                format!("ka_{i}"),
                OtoEntry {
                    wav_filename: format!("ka_{i}.wav"),
                    alias: format!("ka_{i}"),
                    preutterance: 45.0,
                    overlap: 15.0,
                    consonant: 90.0,
                    ..Default::default()
                },
            );
        }

        let vcv_analysis = analyze_voicebank(&vcv_bank);
        assert_eq!(vcv_analysis.style, VoicebankStyle::Vcv);
        assert_eq!(vcv_analysis.median_preutterance_ms, 120.0);
        assert_eq!(vcv_analysis.median_overlap_ms, 50.0);

        let cv_analysis = analyze_voicebank(&cv_bank);
        assert_eq!(cv_analysis.style, VoicebankStyle::Cv);
        assert_eq!(cv_analysis.median_preutterance_ms, 45.0);
        assert_eq!(cv_analysis.median_overlap_ms, 15.0);
    }

    #[test]
    fn apply_utau_guide_configures_timing_and_pitch() {
        let mut notes = vec![
            UNote::new("a", "C4", 0.0, 500.0),
            UNote::new("ka", "G4", 500.0, 500.0),
        ];

        let analysis = UtauGuideAnalysis {
            style: VoicebankStyle::Vcv,
            median_preutterance_ms: 100.0,
            median_overlap_ms: 40.0,
            median_consonant_ms: 150.0,
            recommended_portamento_start_ratio: -0.5,
            recommended_portamento_length_ms: 90.0,
            recommended_consonant_velocity: 100.0,
            recommended_attack_ms: 25.0,
            recommended_release_ms: 45.0,
            total_entries: 100,
        };

        let options = UtauGuideOptions::default();
        apply_utau_guide(&mut notes, &analysis, &options);

        assert_eq!(notes[1].expressions.consonant_velocity, 100.0);
        assert_eq!(notes[1].envelope.p2, 25.0);
        assert_eq!(notes[1].pitch_bend.points.len(), 2);
        // C4 (60) to G4 (67) is -700 cents delta
        assert_eq!(notes[1].pitch_bend.points[0].pitch_offset_cents, -700.0);
        assert_eq!(notes[1].pitch_bend.points[1].pitch_offset_cents, 0.0);
    }
}
