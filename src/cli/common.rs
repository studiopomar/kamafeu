use serde_json::{json, Value};
use std::io::{Error, ErrorKind};
use std::path::PathBuf;

use kamafeu::{
    extensions::default_project_format_registry, oto::Voicebank, project::model::UProject,
};

pub(super) fn automation_lyrics_split_mode(
    mode: Option<&str>,
) -> Result<kamafeu::gui::lyrics_dialog::SplitMode, Box<dyn std::error::Error>> {
    use kamafeu::gui::lyrics_dialog::SplitMode;

    match mode.unwrap_or("hyphens_and_spaces").trim().to_ascii_lowercase().as_str() {
        "spaces" | "space" | "words" => Ok(SplitMode::Spaces),
        "hyphens_and_spaces" | "hyphens" | "hyphen" | "default" => {
            Ok(SplitMode::HyphensAndSpaces)
        }
        "japanese_syllables" | "japanese" | "ja" => Ok(SplitMode::JapaneseSyllables),
        "portuguese_syllables" | "portuguese" | "pt" => Ok(SplitMode::PortugueseSyllables),
        value => Err(Error::new(
            ErrorKind::InvalidInput,
            format!("mode inválido '{value}'; use spaces, hyphens_and_spaces, japanese_syllables ou portuguese_syllables"),
        )
        .into()),
    }
}

pub(super) fn print_json(value: &Value) -> Result<(), serde_json::Error> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

pub(super) fn project_info_json(path: &PathBuf) -> Result<Value, Box<dyn std::error::Error>> {
    let project = load_project(path)?;
    let note_count: usize = project.parts.iter().map(|part| part.notes.len()).sum();
    let duration_ms = project
        .parts
        .iter()
        .flat_map(|part| {
            part.notes
                .iter()
                .map(move |note| part.position_ms + note.position_ms + note.duration_ms.max(0.0))
        })
        .chain(
            project
                .wave_parts
                .iter()
                .map(|wave| wave.position_ms + wave.duration_ms.max(0.0)),
        )
        .fold(0.0_f64, f64::max);
    let tracks = project
        .tracks
        .iter()
        .enumerate()
        .map(|(index, track)| {
            let track_notes: usize = project
                .parts
                .iter()
                .filter(|part| part.track_index == index)
                .map(|part| part.notes.len())
                .sum();
            json!({"index": index, "name": track.name, "note_count": track_notes})
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "path": path,
        "name": project.name,
        "bpm": project.bpm,
        "time_signature": format!("{}/{}", project.time_signature_numerator, project.time_signature_denominator),
        "track_count": project.tracks.len(),
        "voice_part_count": project.parts.len(),
        "wave_part_count": project.wave_parts.len(),
        "marker_count": project.markers.len(),
        "section_count": project.sections.len(),
        "sections": project.sections.iter().enumerate().map(|(index, section)| json!({"index": index, "name": section.name, "start_ms": section.start_ms, "end_ms": section.end_ms, "color": section.color})).collect::<Vec<_>>(),
        "markers": project.markers.iter().enumerate().map(|(index, marker)| json!({"index": index, "name": marker.name, "position_ms": marker.position_ms, "color": marker.color})).collect::<Vec<_>>(),
        "note_count": note_count,
        "duration_ms": duration_ms,
        "tracks": tracks,
    }))
}

pub(super) fn voicebank_validation_json(
    path: &PathBuf,
) -> Result<Value, Box<dyn std::error::Error>> {
    let vb = Voicebank::new(path)?;
    let report = vb.diagnostic_report();
    let issues = report
        .issues
        .iter()
        .map(|issue| {
            json!({
                "alias": issue.alias,
                "wav": issue.wav_filename,
                "detail": issue.detail,
                "kind": format!("{:?}", issue.kind),
                "suggestion": issue.suggestion(),
            })
        })
        .collect::<Vec<_>>();
    Ok(json!({
        "path": path,
        "name": vb.name,
        "healthy": report.is_healthy(),
        "entries": report.entry_count,
        "unique_wavs": report.unique_wav_count,
        "readable_wavs": report.readable_wav_count,
        "missing_wavs": report.missing_wav_count,
        "invalid_wavs": report.invalid_wav_count,
        "empty_wavs": report.empty_wav_count,
        "invalid_oto_timing": report.invalid_oto_timing_count,
        "sample_rates": report.sample_rates,
        "issues": issues,
    }))
}

pub(super) fn load_project(path: &PathBuf) -> Result<UProject, Box<dyn std::error::Error>> {
    default_project_format_registry().load_file(path)
}

pub(super) fn save_project(
    project: &UProject,
    path: &PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    default_project_format_registry().save_file(project, path)
}
