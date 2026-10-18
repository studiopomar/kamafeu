use super::*;

pub(super) fn handle(
    command: Commands,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Commands::ProjectInfo { path } => {
            let project = load_project(&path)?;
            let note_count: usize = project.parts.iter().map(|part| part.notes.len()).sum();
            let duration_ms = project
                .parts
                .iter()
                .flat_map(|part| {
                    part.notes.iter().map(move |note| {
                        part.position_ms + note.position_ms + note.duration_ms.max(0.0)
                    })
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
                    (index, &track.name, track_notes)
                })
                .collect::<Vec<_>>();
            if json_output {
                print_json(&json!({
                    "path": path,
                    "name": project.name,
                    "bpm": project.bpm,
                    "time_signature": format!("{}/{}", project.time_signature_numerator, project.time_signature_denominator),
                    "track_count": project.tracks.len(),
                    "voice_part_count": project.parts.len(),
                    "wave_part_count": project.wave_parts.len(),
                    "section_count": project.sections.len(),
                    "note_count": note_count,
                    "duration_ms": duration_ms,
                    "tracks": tracks.iter().map(|(index, name, notes)| json!({
                        "index": index,
                        "name": name,
                        "note_count": notes,
                    })).collect::<Vec<_>>(),
                }))?;
            } else {
                kamafeu::terminal_log::out_header("Project", &project.name);
                kamafeu::terminal_log::out_kv("Tempo", format!("{:.2} BPM", project.bpm));
                kamafeu::terminal_log::out_kv(
                    "Time signature",
                    format!(
                        "{}/{}",
                        project.time_signature_numerator, project.time_signature_denominator
                    ),
                );
                kamafeu::terminal_log::out_kv("Tracks", project.tracks.len());
                kamafeu::terminal_log::out_kv("Voice parts", project.parts.len());
                kamafeu::terminal_log::out_kv("Wave parts", project.wave_parts.len());
                kamafeu::terminal_log::out_kv("Sections", project.sections.len());
                kamafeu::terminal_log::out_kv("Notes", note_count);
                kamafeu::terminal_log::out_kv("Duration", format!("{:.2}s", duration_ms / 1000.0));
                for (index, name, track_notes) in tracks {
                    kamafeu::terminal_log::out_list(format!(
                        "{}. {} ({} notes)",
                        index + 1,
                        name,
                        track_notes
                    ));
                }
            }
        }

        Commands::Diff { before, after } => {
            let before_project = load_project(&before)?;
            let after_project = load_project(&after)?;
            let diff = kamafeu::project::ProjectDiff::between(&before_project, &after_project);
            if json_output {
                print_json(&serde_json::to_value(&diff)?)?;
            } else {
                kamafeu::terminal_log::out_header(
                    "Project diff",
                    format!("{} -> {}", before.display(), after.display()),
                );
                kamafeu::terminal_log::out_kv("Metadata", diff.changed_project_metadata);
                kamafeu::terminal_log::out_kv("Arrangement", diff.changed_arrangement);
                kamafeu::terminal_log::out_kv("Render settings", diff.changed_render_settings);
                kamafeu::terminal_log::out_kv("Notes added", diff.added_notes);
                kamafeu::terminal_log::out_kv("Notes removed", diff.removed_notes);
                kamafeu::terminal_log::out_kv("Notes changed", diff.changed_notes);
                kamafeu::terminal_log::out_kv("Phoneme overrides", diff.changed_phoneme_overrides);
                kamafeu::terminal_log::out_kv("Wave parts added", diff.added_wave_parts);
                kamafeu::terminal_log::out_kv("Wave parts removed", diff.removed_wave_parts);
                kamafeu::terminal_log::out_kv("Wave parts changed", diff.changed_wave_parts);
                kamafeu::terminal_log::out_status(
                    "OK",
                    "Result",
                    if diff.is_empty() {
                        "identical"
                    } else {
                        "changed"
                    },
                );
            }
        }

        Commands::ProvenanceInfo { path } => {
            let contents = fs::read_to_string(&path)?;
            let provenance: RenderProvenance = serde_json::from_str(&contents)
                .map_err(|error| Error::new(ErrorKind::InvalidData, error))?;
            if json_output {
                print_json(&serde_json::to_value(&provenance)?)?;
            } else {
                kamafeu::terminal_log::out_header("Render provenance", path.display());
                kamafeu::terminal_log::out_kv("Pipeline", provenance.pipeline_version);
                kamafeu::terminal_log::out_kv("Schema", provenance.schema);
                kamafeu::terminal_log::out_kv("Resampler", provenance.resampler);
                kamafeu::terminal_log::out_kv("Wavtool", provenance.wavtool);
                kamafeu::terminal_log::out_kv("Cache", provenance.cache_hit);
                kamafeu::terminal_log::out_kv("Artifacts", provenance.artifacts.len());
                kamafeu::terminal_log::out_kv(
                    "Timing records",
                    provenance.timing_diagnostics.len(),
                );
                kamafeu::terminal_log::out_kv(
                    "Timing boundaries",
                    provenance.timing_boundary_diagnostics.len(),
                );
                kamafeu::terminal_log::out_kv("Warnings", provenance.warnings.len());
            }
        }

        Commands::ValidateProject {
            path,
            fail_on_warning,
            extensions_path,
        } => {
            let report = conversion::project_validation_json(&path, extensions_path.as_ref())?;
            if json_output {
                print_json(&report)?;
            } else {
                kamafeu::terminal_log::out_header("Project validation", path.display());
                kamafeu::terminal_log::out_kv("Tracks", &report["track_count"]);
                kamafeu::terminal_log::out_kv("Voice parts", &report["voice_part_count"]);
                kamafeu::terminal_log::out_kv("Wave parts", &report["wave_part_count"]);
                kamafeu::terminal_log::out_kv("Notes", &report["note_count"]);
                if report["valid"].as_bool() == Some(true) {
                    kamafeu::terminal_log::out_status("OK", "Result", "valid");
                } else {
                    kamafeu::terminal_log::out_status("ERROR", "Result", "invalid");
                    for issue in report["issues"].as_array().into_iter().flatten() {
                        kamafeu::terminal_log::out_list(format!(
                            "[{}] {}: {}{}",
                            issue["severity"].as_str().unwrap_or("unknown"),
                            issue["location"].as_str().unwrap_or("project"),
                            issue["detail"].as_str().unwrap_or("unknown issue"),
                            issue["suggestion"]
                                .as_str()
                                .map(|s| format!(" (hint: {s})"))
                                .unwrap_or_default()
                        ));
                    }
                }
                for diagnostic in report["extension_diagnostics"]
                    .as_array()
                    .into_iter()
                    .flatten()
                {
                    kamafeu::terminal_log::out_list(format!("[warning] extension: {diagnostic}"));
                }
            }
            let has_warnings = report["issues"].as_array().is_some_and(|issues| {
                issues
                    .iter()
                    .any(|issue| issue["severity"].as_str() == Some("warning"))
            }) || report["extension_diagnostics"]
                .as_array()
                .is_some_and(|diagnostics| !diagnostics.is_empty());
            if report["valid"].as_bool() != Some(true) || (fail_on_warning && has_warnings) {
                return Err(Error::new(ErrorKind::InvalidData, "project validation failed").into());
            }
        }

        _ => unreachable!("project command dispatcher received another command"),
    }
    Ok(())
}
