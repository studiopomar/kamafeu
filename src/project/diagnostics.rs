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
    InvalidPhonemeOverride,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectIssue {
    pub severity: ProjectIssueSeverity,
    pub kind: ProjectIssueKind,
    pub location: String,
    pub detail: String,
}

impl ProjectIssue {
    /// Suggested action only; diagnostics never mutate the project.
    pub fn suggestion(&self) -> Option<&'static str> {
        Some(match self.kind {
            ProjectIssueKind::InvalidTempo => "Defina um BPM entre 20 e 999.",
            ProjectIssueKind::MissingTracks => "Crie uma faixa antes de inserir material.",
            ProjectIssueKind::InvalidTrackReference => "Associe a parte a uma faixa existente.",
            ProjectIssueKind::InvalidPosition | ProjectIssueKind::InvalidMarkerPosition => {
                "Ajuste a posição para um valor finito não negativo."
            }
            ProjectIssueKind::InvalidDuration => "Ajuste a duração para um valor positivo.",
            ProjectIssueKind::InvalidPitch => "Escolha uma nota como C4 ou uma chave MIDI válida.",
            ProjectIssueKind::EmptyLyric => "Digite uma letra ou fonema renderizável.",
            ProjectIssueKind::MissingWaveFile => "Selecione um arquivo WAV existente.",
            ProjectIssueKind::InvalidSectionRange => "Ajuste início e fim da seção.",
            ProjectIssueKind::VeryShortNote => "Aumente a duração ou una a nota à vizinha.",
            ProjectIssueKind::NoteOverlap => {
                "Use Legato ou corrija as sobreposições se não forem intencionais."
            }
            ProjectIssueKind::ExtremePhonemeTiming => {
                "Revise preutterance, overlap e timing de consoante."
            }
            ProjectIssueKind::InvalidPhonemeOverride => {
                "Limpe o override ou restaure a herança do fonema."
            }
        })
    }
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
                for (override_index, phoneme_override) in note.phoneme_overrides.iter().enumerate()
                {
                    let override_location =
                        format!("{location} / phoneme override {}", override_index + 1);
                    if note.phoneme_durations_ms.len() > phoneme_override.index {
                        // The index is structurally meaningful when authored
                        // sub-phoneme durations are present.
                    } else if !note.phoneme_durations_ms.is_empty() {
                        report.issues.push(warning(
                            ProjectIssueKind::InvalidPhonemeOverride,
                            &override_location,
                            "o índice do override não corresponde a uma divisão de subfonema persistida",
                        ));
                    }
                    if phoneme_override
                        .phoneme
                        .as_deref()
                        .is_some_and(|phoneme| phoneme.trim().is_empty())
                    {
                        report.issues.push(warning(
                            ProjectIssueKind::InvalidPhonemeOverride,
                            &override_location,
                            "alias/fonema vazio será tratado como herança",
                        ));
                    }
                    let numeric_invalid =
                        phoneme_override.envelope.as_ref().is_some_and(|envelope| {
                            [
                                envelope.p1,
                                envelope.p2,
                                envelope.p3,
                                envelope.p4,
                                envelope.p5,
                                envelope.v1,
                                envelope.v2,
                                envelope.v3,
                                envelope.v4,
                                envelope.v5,
                                envelope.crossfade_ms,
                            ]
                            .iter()
                            .any(|value| !value.is_finite())
                        }) || phoneme_override
                            .consonant_timing_offset_ms
                            .is_some_and(|value| !value.is_finite() || value.abs() > 300.0)
                            || phoneme_override.vibrato.as_ref().is_some_and(|vibrato| {
                                [
                                    vibrato.length_pct,
                                    vibrato.period_ms,
                                    vibrato.depth_cents,
                                    vibrato.fade_in_ms,
                                    vibrato.fade_in_pct,
                                    vibrato.fade_out_pct,
                                    vibrato.shift_pct,
                                    vibrato.drift_pct,
                                    vibrato.volume_link_pct,
                                ]
                                .iter()
                                .any(|value| !value.is_finite())
                            })
                            || phoneme_override.pitch_bend.as_ref().is_some_and(|bend| {
                                !bend.portamento_start_ms.is_finite()
                                    || !bend.portamento_length_ms.is_finite()
                                    || bend.points.iter().any(|point| {
                                        !point.time_offset_ms.is_finite()
                                            || !point.pitch_offset_cents.is_finite()
                                    })
                            });
                    if numeric_invalid {
                        report.issues.push(warning(
                            ProjectIssueKind::InvalidPhonemeOverride,
                            &override_location,
                            "o override contém valores não finitos e será normalizado ao salvar",
                        ));
                    }
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
    fn reports_invalid_phoneme_override_without_mutating_project() {
        let mut project = UProject::default();
        let mut note = UNote::new("a", "C4", 0.0, 200.0);
        note.phoneme_durations_ms = vec![100.0];
        note.phoneme_overrides
            .push(crate::project::model::UPhonemeOverride {
                index: 4,
                phoneme: Some("   ".to_string()),
                vibrato: Some(crate::dsp::pitch::VibratoParam {
                    period_ms: f64::NAN,
                    ..Default::default()
                }),
                ..Default::default()
            });
        project.parts[0].notes.push(note);

        let report = project.diagnostic_report();
        assert!(
            report
                .issues
                .iter()
                .filter(|issue| issue.kind == ProjectIssueKind::InvalidPhonemeOverride)
                .count()
                >= 3
        );
        assert_eq!(project.parts[0].notes[0].phoneme_overrides[0].index, 4);
        assert!(project.parts[0].notes[0].phoneme_overrides[0]
            .vibrato
            .as_ref()
            .unwrap()
            .period_ms
            .is_nan());
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
        assert!(report.issues[0].suggestion().is_some());
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
