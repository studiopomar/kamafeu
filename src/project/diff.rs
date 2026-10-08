//! Semantic comparison of project revisions.
//!
//! This deliberately compares musical model values instead of serialized JSON
//! bytes, so formatting, field order and APS migration metadata do not create
//! false changes.

use super::model::UProject;
use serde::Serialize;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ProjectDiff {
    pub changed_project_metadata: bool,
    pub changed_arrangement: bool,
    pub changed_render_settings: bool,
    pub added_notes: usize,
    pub removed_notes: usize,
    pub changed_notes: usize,
    pub changed_note_indices: Vec<usize>,
    pub changed_phoneme_overrides: usize,
    pub added_wave_parts: usize,
    pub removed_wave_parts: usize,
    pub changed_wave_parts: usize,
    pub changed_wave_part_indices: Vec<usize>,
    pub changed_marker_indices: Vec<usize>,
    pub changed_section_indices: Vec<usize>,
}

impl ProjectDiff {
    pub fn between(before: &UProject, after: &UProject) -> Self {
        let before_notes = before
            .parts
            .iter()
            .flat_map(|part| part.notes.iter())
            .collect::<Vec<_>>();
        let after_notes = after
            .parts
            .iter()
            .flat_map(|part| part.notes.iter())
            .collect::<Vec<_>>();
        let mut changed_notes = 0;
        let mut changed_note_indices = Vec::new();
        let mut changed_phoneme_overrides = 0;
        for (index, (before_note, after_note)) in
            before_notes.iter().zip(after_notes.iter()).enumerate()
        {
            if before_note != after_note {
                changed_notes += 1;
                changed_note_indices.push(index);
            }
            if before_note.phoneme_overrides != after_note.phoneme_overrides {
                changed_phoneme_overrides += 1;
            }
        }

        let common_waves = before.wave_parts.len().min(after.wave_parts.len());
        Self {
            changed_project_metadata: before.name != after.name
                || before.bpm != after.bpm
                || before.time_signature_numerator != after.time_signature_numerator
                || before.time_signature_denominator != after.time_signature_denominator
                || before.voicebank != after.voicebank
                || before.voicebank_path != after.voicebank_path,
            changed_arrangement: before.tracks != after.tracks
                || before.parts.len() != after.parts.len()
                || before
                    .parts
                    .iter()
                    .zip(after.parts.iter())
                    .any(|(left, right)| {
                        left.name != right.name
                            || left.track_index != right.track_index
                            || (left.position_ms - right.position_ms).abs() > f64::EPSILON
                    })
                || before.markers != after.markers
                || before.sections != after.sections,
            changed_render_settings: before.phonemizer != after.phonemizer
                || before.resampler != after.resampler
                || before.wavtool != after.wavtool
                || before.flags != after.flags
                || before.sample_rate != after.sample_rate
                || before.render_threads != after.render_threads
                || before.extensions != after.extensions,
            added_notes: after_notes.len().saturating_sub(before_notes.len()),
            removed_notes: before_notes.len().saturating_sub(after_notes.len()),
            changed_notes,
            changed_phoneme_overrides,
            added_wave_parts: after
                .wave_parts
                .len()
                .saturating_sub(before.wave_parts.len()),
            removed_wave_parts: before
                .wave_parts
                .len()
                .saturating_sub(after.wave_parts.len()),
            changed_wave_parts: before
                .wave_parts
                .iter()
                .zip(after.wave_parts.iter())
                .take(common_waves)
                .filter(|(left, right)| left != right)
                .count(),
            changed_note_indices,
            changed_wave_part_indices: before
                .wave_parts
                .iter()
                .zip(after.wave_parts.iter())
                .take(common_waves)
                .enumerate()
                .filter_map(|(index, (left, right))| (left != right).then_some(index))
                .collect(),
            changed_marker_indices: before
                .markers
                .iter()
                .zip(after.markers.iter())
                .enumerate()
                .filter_map(|(index, (left, right))| (left != right).then_some(index))
                .collect(),
            changed_section_indices: before
                .sections
                .iter()
                .zip(after.sections.iter())
                .enumerate()
                .filter_map(|(index, (left, right))| (left != right).then_some(index))
                .collect(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// Restores selected note values from `source` into `current` while retaining
/// the current project's arrangement, audio parts and render configuration.
/// The caller owns the transaction/undo boundary; this function never mutates
/// either input.
pub fn restore_note_indices(
    current: &UProject,
    source: &UProject,
    indices: &[usize],
) -> Result<UProject, String> {
    let mut restored = current.clone();
    let mut current_notes = restored
        .parts
        .iter_mut()
        .flat_map(|part| part.notes.iter_mut())
        .collect::<Vec<_>>();
    let source_notes = source
        .parts
        .iter()
        .flat_map(|part| part.notes.iter())
        .collect::<Vec<_>>();
    for &index in indices {
        let Some(destination) = current_notes.get_mut(index) else {
            return Err(format!("índice de nota fora do projeto atual: {index}"));
        };
        let Some(origin) = source_notes.get(index) else {
            return Err(format!("índice de nota fora do snapshot: {index}"));
        };
        **destination = (**origin).clone();
    }
    Ok(restored)
}

/// Restores selected audio-part descriptors from `source` while retaining all
/// vocal material and render settings from `current`.
pub fn restore_wave_part_indices(
    current: &UProject,
    source: &UProject,
    indices: &[usize],
) -> Result<UProject, String> {
    let mut restored = current.clone();
    for &index in indices {
        let Some(destination) = restored.wave_parts.get_mut(index) else {
            return Err(format!("índice de áudio fora do projeto atual: {index}"));
        };
        let Some(origin) = source.wave_parts.get(index) else {
            return Err(format!("índice de áudio fora do snapshot: {index}"));
        };
        *destination = origin.clone();
    }
    Ok(restored)
}

pub fn restore_marker_indices(
    current: &UProject,
    source: &UProject,
    indices: &[usize],
) -> Result<UProject, String> {
    let mut restored = current.clone();
    for &index in indices {
        let Some(destination) = restored.markers.get_mut(index) else {
            return Err(format!("índice de marcador fora do projeto atual: {index}"));
        };
        let Some(origin) = source.markers.get(index) else {
            return Err(format!("índice de marcador fora do snapshot: {index}"));
        };
        *destination = origin.clone();
    }
    Ok(restored)
}

pub fn restore_section_indices(
    current: &UProject,
    source: &UProject,
    indices: &[usize],
) -> Result<UProject, String> {
    let mut restored = current.clone();
    for &index in indices {
        let Some(destination) = restored.sections.get_mut(index) else {
            return Err(format!("índice de seção fora do projeto atual: {index}"));
        };
        let Some(origin) = source.sections.get(index) else {
            return Err(format!("índice de seção fora do snapshot: {index}"));
        };
        *destination = origin.clone();
    }
    Ok(restored)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::model::{UNote, UPhonemeOverride, UProject};

    #[test]
    fn semantic_diff_reports_note_and_phoneme_changes_without_json_order() {
        let mut before = UProject::default();
        before.parts[0]
            .notes
            .push(UNote::new("a", "C4", 0.0, 400.0));
        let mut after = before.clone();
        after.parts[0].notes[0].duration_ms = 500.0;
        after.parts[0].notes[0]
            .phoneme_overrides
            .push(UPhonemeOverride {
                phoneme: Some("a".into()),
                ..Default::default()
            });

        let diff = ProjectDiff::between(&before, &after);
        assert_eq!(diff.changed_notes, 1);
        assert_eq!(diff.changed_note_indices, vec![0]);
        assert_eq!(diff.changed_phoneme_overrides, 1);
        assert!(!diff.is_empty());
    }

    #[test]
    fn identical_projects_have_an_empty_diff() {
        let project = UProject::default();
        assert!(ProjectDiff::between(&project, &project).is_empty());
    }

    #[test]
    fn selective_note_restore_preserves_current_arrangement_and_is_non_mutating() {
        let mut current = UProject::default();
        current.name = "current".into();
        current.parts[0]
            .notes
            .push(UNote::new("la", "C4", 0.0, 400.0));
        let mut source = current.clone();
        source.name = "snapshot".into();
        source.parts[0].notes[0].lyric = "li".into();

        let restored = restore_note_indices(&current, &source, &[0]).expect("restore");
        assert_eq!(restored.name, "current");
        assert_eq!(restored.parts[0].notes[0].lyric, "li");
        assert_eq!(current.parts[0].notes[0].lyric, "la");
    }

    #[test]
    fn selective_audio_restore_preserves_vocal_material_and_is_non_mutating() {
        let mut current = UProject::default();
        current
            .wave_parts
            .push(crate::project::model::UWavePart::new(
                "current",
                "current.wav",
                0,
            ));
        let mut source = current.clone();
        source.wave_parts[0].name = "snapshot".into();

        let restored = restore_wave_part_indices(&current, &source, &[0]).expect("restore");
        assert_eq!(restored.wave_parts[0].name, "snapshot");
        assert!(restored.parts[0].notes.is_empty());
        assert_eq!(current.wave_parts[0].name, "current");
    }

    #[test]
    fn selective_arrangement_restore_preserves_notes_and_audio() {
        let mut current = UProject::default();
        current
            .markers
            .push(crate::project::model::UProjectMarker::new("current", 10.0));
        current
            .sections
            .push(crate::project::model::UProjectSection::new(
                "current", 0.0, 100.0,
            ));
        let mut source = current.clone();
        source.markers[0].name = "snapshot".into();
        source.sections[0].name = "snapshot".into();

        let restored = restore_marker_indices(&current, &source, &[0]).expect("marker restore");
        let restored = restore_section_indices(&restored, &source, &[0]).expect("section restore");
        assert_eq!(restored.markers[0].name, "snapshot");
        assert_eq!(restored.sections[0].name, "snapshot");
        assert!(restored.parts[0].notes.is_empty());
        assert_eq!(current.markers[0].name, "current");
    }
}
