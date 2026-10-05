//! Non-mutating structural checks for imported and scripted projects.

use super::model::UProject;
use crate::dsp::pitch::note_name_to_midi;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectIssueSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectIssueKind {
    InvalidTempo,
    MissingTracks,
    InvalidTrackReference,
    InvalidPosition,
    InvalidDuration,
    InvalidPitch,
    EmptyLyric,
    MissingWaveFile,
    InvalidMarkerPosition,
    InvalidSectionRange,
    VeryShortNote,
    NoteOverlap,
    ExtremePhonemeTiming,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectIssue {
    pub severity: ProjectIssueSeverity,
    pub kind: ProjectIssueKind,
    pub location: String,
    pub detail: String,
}

#[derive(Debug, Clone, Default)]
pub struct ProjectDiagnosticReport {
    pub track_count: usize,
    pub voice_part_count: usize,
    pub wave_part_count: usize,
    pub note_count: usize,
    pub issues: Vec<ProjectIssue>,
}

impl ProjectDiagnosticReport {
    pub fn is_valid(&self) -> bool {
        self.issues
            .iter()
            .all(|issue| issue.severity != ProjectIssueSeverity::Error)
    }

    pub fn error_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == ProjectIssueSeverity::Error)
            .count()
    }

    pub fn warning_count(&self) -> usize {
        self.issues
            .iter()
            .filter(|issue| issue.severity == ProjectIssueSeverity::Warning)
            .count()
    }

    /// A concise health signal for the editor and automation clients. It is
    /// intentionally advisory: errors have stronger weight than vocal review
    /// warnings, and the full issue list remains the authoritative diagnosis.
    pub fn vocal_health_score(&self) -> u8 {
        let penalty =
            self.error_count().saturating_mul(35) + self.warning_count().saturating_mul(8);
        100u8.saturating_sub(penalty.min(100) as u8)
    }
}

impl UProject {
    /// Checks source data before `normalize` repairs permissive imports.
    pub fn diagnostic_report(&self) -> ProjectDiagnosticReport {
        let mut report = ProjectDiagnosticReport {
            track_count: self.tracks.len(),
            voice_part_count: self.parts.len(),
            wave_part_count: self.wave_parts.len(),
            note_count: self.parts.iter().map(|part| part.notes.len()).sum(),
            ..ProjectDiagnosticReport::default()
        };
        if !self.bpm.is_finite() || !(20.0..=999.0).contains(&self.bpm) {
            report.issues.push(error(
                ProjectIssueKind::InvalidTempo,
                "project",
                "BPM deve ser um número finito entre 20 e 999",
            ));
        }
        if self.tracks.is_empty() {
            report.issues.push(error(
                ProjectIssueKind::MissingTracks,
                "project",
                "o projeto não possui faixas",
            ));
        }

        for (part_index, part) in self.parts.iter().enumerate() {
            let location = format!("voice part {}", part_index + 1);
            validate_part_location(
                &mut report,
                &location,
                part.track_index,
                part.position_ms,
                self.tracks.len(),
            );
            for (note_index, note) in part.notes.iter().enumerate() {
                let location = format!("voice part {} / note {}", part_index + 1, note_index + 1);
                if !note.position_ms.is_finite() || note.position_ms < 0.0 {
                    report.issues.push(error(
                        ProjectIssueKind::InvalidPosition,
                        &location,
                        "posição da nota deve ser um número finito maior ou igual a zero",
                    ));
                }
                if !note.duration_ms.is_finite() || note.duration_ms <= 0.0 {
                    report.issues.push(error(
                        ProjectIssueKind::InvalidDuration,
                        &location,
                        "duração da nota deve ser um número finito maior que zero",
                    ));
                }
                if note_name_to_midi(&note.pitch)
                    .or_else(|| note.pitch.parse::<u8>().ok())
                    .is_none()
                {
                    report.issues.push(error(
                        ProjectIssueKind::InvalidPitch,
                        &location,
                        "altura deve ser uma nota como C4 ou uma chave MIDI entre 0 e 127",
                    ));
                }
                if note.lyric.trim().is_empty() {
                    report.issues.push(warning(
                        ProjectIssueKind::EmptyLyric,
                        &location,
                        "letra vazia pode não produzir um fonema renderizável",
                    ));
                }
                if note.duration_ms.is_finite() && note.duration_ms > 0.0 && note.duration_ms < 35.0
                {
                    report.issues.push(warning(
                        ProjectIssueKind::VeryShortNote,
                        &location,
                        "nota abaixo de 35 ms pode não ter tempo suficiente para consoante e transição; una-a ou aumente sua duração",
                    ));
                }
                let timing_values = [
                    note.expressions.consonant_timing_offset_ms,
                    note.expressions.preutter_offset_ms,
                    note.expressions.overlap_offset_ms,
                ];
                if timing_values
                    .iter()
                    .any(|value| !value.is_finite() || value.abs() > 300.0)
                {
                    report.issues.push(warning(
                        ProjectIssueKind::ExtremePhonemeTiming,
                        &location,
                        "offset de consoante, preutterance ou overlap excede ±300 ms; revise a transição antes de renderizar",
                    ));
                }
            }

            let mut positioned_notes = part.notes.iter().enumerate().collect::<Vec<_>>();
            positioned_notes.sort_by(|(_, left), (_, right)| {
                left.position_ms
                    .partial_cmp(&right.position_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            for pair in positioned_notes.windows(2) {
                let (left_index, left) = pair[0];
                let (right_index, right) = pair[1];
                let left_end = left.position_ms + left.duration_ms;
                if left.position_ms.is_finite()
                    && left.duration_ms.is_finite()
                    && right.position_ms.is_finite()
                    && left_end - right.position_ms > 40.0
                {
                    report.issues.push(warning(
                        ProjectIssueKind::NoteOverlap,
                        &format!("voice part {} / note {}", part_index + 1, right_index + 1),
                        &format!(
                            "sobreposição de {:.0} ms com a nota {}; use Legato ou Corrigir Sobreposições se não for intencional",
                            left_end - right.position_ms,
                            left_index + 1,
                        ),
                    ));
                }
            }
        }

        for (wave_index, wave) in self.wave_parts.iter().enumerate() {
            let location = format!("wave part {}", wave_index + 1);
            validate_part_location(
                &mut report,
                &location,
                wave.track_index,
                wave.position_ms,
                self.tracks.len(),
            );
            if wave.file_path.trim().is_empty() {
                report.issues.push(error(
                    ProjectIssueKind::MissingWaveFile,
                    &location,
                    "parte de áudio não possui caminho de arquivo",
                ));
            }
        }
        for (index, marker) in self.markers.iter().enumerate() {
            if !marker.position_ms.is_finite() || marker.position_ms < 0.0 {
                report.issues.push(error(
                    ProjectIssueKind::InvalidMarkerPosition,
                    &format!("marker {}", index + 1),
                    "posição do marcador deve ser um número finito maior ou igual a zero",
                ));
            }
        }
        for (index, section) in self.sections.iter().enumerate() {
            if !section.start_ms.is_finite()
                || !section.end_ms.is_finite()
                || section.start_ms < 0.0
                || section.end_ms < section.start_ms
            {
                report.issues.push(error(
                    ProjectIssueKind::InvalidSectionRange,
                    &format!("section {}", index + 1),
                    "início e fim da seção devem ser números finitos, com fim igual ou posterior ao início",
                ));
            }
        }
        report
    }
}

fn validate_part_location(
    report: &mut ProjectDiagnosticReport,
    location: &str,
    track_index: usize,
    position_ms: f64,
    track_count: usize,
) {
    if track_index >= track_count {
        report.issues.push(error(
            ProjectIssueKind::InvalidTrackReference,
            location,
            "a parte aponta para uma faixa inexistente",
        ));
    }
    if !position_ms.is_finite() || position_ms < 0.0 {
        report.issues.push(error(
            ProjectIssueKind::InvalidPosition,
            location,
            "posição da parte deve ser um número finito maior ou igual a zero",
        ));
    }
}

fn error(kind: ProjectIssueKind, location: &str, detail: &str) -> ProjectIssue {
    ProjectIssue {
        severity: ProjectIssueSeverity::Error,
        kind,
        location: location.to_string(),
        detail: detail.to_string(),
    }
}

fn warning(kind: ProjectIssueKind, location: &str, detail: &str) -> ProjectIssue {
    ProjectIssue {
        severity: ProjectIssueSeverity::Warning,
        kind,
        location: location.to_string(),
        detail: detail.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::model::{UNote, UProject};

    #[test]
    fn reports_import_problems_without_normalizing_them_away() {
        let mut project = UProject::default();
        project.bpm = f64::NAN;
        project.parts[0].track_index = 9;
        project.parts[0]
            .notes
            .push(UNote::new("", "not-a-note", -1.0, 0.0));

        let report = project.diagnostic_report();
        assert!(!report.is_valid());
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.kind == ProjectIssueKind::InvalidTempo));
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.kind == ProjectIssueKind::InvalidTrackReference));
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.kind == ProjectIssueKind::InvalidPitch));
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.kind == ProjectIssueKind::EmptyLyric));
        assert_eq!(report.error_count(), 5);
        assert_eq!(report.warning_count(), 1);
        assert_eq!(report.vocal_health_score(), 0);
    }

    #[test]
    fn flags_vocal_timing_problems_without_changing_notes() {
        let mut project = UProject::default();
        let mut first = UNote::new("a", "C4", 0.0, 200.0);
        first.expressions.preutter_offset_ms = 350.0;
        project.parts[0].notes.push(first);
        project.parts[0]
            .notes
            .push(UNote::new("i", "D4", 120.0, 20.0));

        let report = project.diagnostic_report();
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.kind == ProjectIssueKind::ExtremePhonemeTiming));
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.kind == ProjectIssueKind::VeryShortNote));
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.kind == ProjectIssueKind::NoteOverlap));
    }

    #[test]
    fn health_score_distinguishes_advisory_warnings_from_errors() {
        let mut project = UProject::default();
        project.parts[0]
            .notes
            .push(UNote::new("", "C4", 0.0, 100.0));
        let report = project.diagnostic_report();
        assert_eq!(report.error_count(), 0);
        assert_eq!(report.warning_count(), 1);
        assert_eq!(report.vocal_health_score(), 92);
    }

    #[test]
    fn detects_invalid_marker_and_section_timeline_data() {
        let mut project = UProject::default();
        project
            .markers
            .push(crate::project::UProjectMarker::new("Verse", -1.0));
        project.sections.push(crate::project::UProjectSection::new(
            "Chorus", 2_000.0, 1_000.0,
        ));

        let report = project.diagnostic_report();
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.kind == ProjectIssueKind::InvalidMarkerPosition));
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.kind == ProjectIssueKind::InvalidSectionRange));
    }
}
