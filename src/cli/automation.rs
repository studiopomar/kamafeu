use super::*;

/// Serves one JSON object per input line and writes exactly one JSON response
/// per request. Keeping this protocol on stdio makes it usable from scripts,
/// editor integrations and an eventual MCP adapter without opening a network
/// port or granting ambient filesystem access.
pub(super) fn run_automation() -> Result<(), Box<dyn std::error::Error>> {
    for (line_number, line) in std::io::stdin().lock().lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<AutomationRequest>(&line) {
            Ok(request) => match handle_automation_request(request) {
                Ok(result) => json!({ "ok": true, "result": result }),
                Err(error) => json!({ "ok": false, "error": error.to_string() }),
            },
            Err(error) => json!({
                "ok": false,
                "error": format!("invalid request on line {}: {error}", line_number + 1),
            }),
        };
        println!("{}", serde_json::to_string(&response)?);
    }
    Ok(())
}

pub(super) fn handle_automation_request(
    request: AutomationRequest,
) -> Result<Value, Box<dyn std::error::Error>> {
    match request {
        AutomationRequest::CreateProject { output, name, bpm } => {
            let mut project = UProject::default();
            if let Some(name) = name.filter(|name| !name.trim().is_empty()) {
                project.name = name;
            }
            if let Some(bpm) = bpm {
                project.set_bpm_preserving_beats(bpm).ok_or_else(|| {
                    Error::new(ErrorKind::InvalidInput, "BPM deve ser um número positivo")
                })?;
            }
            save_project(&project, &output)?;
            Ok(json!({ "output": output, "name": project.name, "bpm": project.bpm }))
        }
        AutomationRequest::ProjectInfo { path } => project_info_json(&path),
        AutomationRequest::Diff { before, after } => {
            let before_project = load_project(&before)?;
            let after_project = load_project(&after)?;
            Ok(serde_json::to_value(
                kamafeu::project::ProjectDiff::between(&before_project, &after_project),
            )?)
        }
        AutomationRequest::ListVoicebanks {
            paths,
            include_default,
        } => Ok(voicebank_catalog_json(
            paths.unwrap_or_default(),
            include_default,
        )),
        AutomationRequest::ValidateProject {
            path,
            extensions_path,
        } => conversion::project_validation_json(&path, extensions_path.as_ref()),
        AutomationRequest::Convert { input, output } => {
            conversion::convert_project(&input, &output)
        }
        AutomationRequest::AddNote {
            input,
            output,
            lyric,
            pitch,
            position_ms,
            duration_ms,
            track_index,
        } => {
            if !position_ms.is_finite() || position_ms < 0.0 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "position_ms deve ser um número finito maior ou igual a zero",
                )
                .into());
            }
            if !duration_ms.is_finite() || duration_ms <= 0.0 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "duration_ms deve ser um número finito maior que zero",
                )
                .into());
            }
            let mut project = load_project(&input)?;
            project.normalize();
            let track_index = track_index.unwrap_or(0);
            while project.tracks.len() <= track_index {
                project.tracks.push(kamafeu::project::UTrack {
                    name: format!("Track {}", project.tracks.len() + 1),
                    ..kamafeu::project::UTrack::default()
                });
            }
            let part_index = project
                .parts
                .iter()
                .position(|part| part.track_index == track_index)
                .unwrap_or_else(|| {
                    project.parts.push(kamafeu::project::UVoicePart::new(
                        format!("Parte Vocal {}", track_index + 1),
                        track_index,
                    ));
                    project.parts.len() - 1
                });
            project.parts[part_index].notes.push(UNote::new(
                lyric,
                pitch,
                position_ms,
                duration_ms,
            ));
            project.parts[part_index].notes.sort_by(|left, right| {
                left.position_ms
                    .partial_cmp(&right.position_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            project.normalize();
            save_project(&project, &output)?;
            Ok(json!({
                "input": input,
                "output": output,
                "track_index": track_index,
                "part_index": part_index,
                "note_count": project.parts[part_index].notes.len(),
            }))
        }
        AutomationRequest::AddAudioPart {
            input,
            output,
            file_path,
            position_ms,
            track_index,
            name,
            volume_db,
        } => {
            if !file_path.is_file() {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    format!("arquivo de áudio não encontrado: {}", file_path.display()),
                )
                .into());
            }
            if !position_ms.is_finite() || position_ms < 0.0 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "position_ms deve ser um número finito maior ou igual a zero",
                )
                .into());
            }
            if volume_db.is_some_and(|value| !value.is_finite()) {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "volume_db deve ser um número finito",
                )
                .into());
            }
            let mut project = load_project(&input)?;
            project.normalize();
            let track_index = track_index.unwrap_or(0);
            while project.tracks.len() <= track_index {
                project.tracks.push(kamafeu::project::UTrack {
                    name: format!("Track {}", project.tracks.len() + 1),
                    ..kamafeu::project::UTrack::default()
                });
            }
            let default_name = file_path
                .file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("Audio")
                .to_string();
            let mut wave = kamafeu::project::UWavePart::new(
                name.filter(|name| !name.trim().is_empty())
                    .unwrap_or(default_name),
                file_path.to_string_lossy(),
                track_index,
            );
            wave.position_ms = position_ms;
            if let Some(volume_db) = volume_db {
                wave.volume_db = volume_db;
            }
            let duration_ms = wave.duration_ms;
            project.wave_parts.push(wave);
            project.normalize();
            save_project(&project, &output)?;
            Ok(json!({
                "input": input,
                "output": output,
                "track_index": track_index,
                "wave_part_count": project.wave_parts.len(),
                "duration_ms": duration_ms,
            }))
        }
        AutomationRequest::SetTempo { input, output, bpm } => {
            let mut project = load_project(&input)?;
            let scale = project.set_bpm_preserving_beats(bpm).ok_or_else(|| {
                Error::new(ErrorKind::InvalidInput, "BPM deve ser um número positivo")
            })?;
            save_project(&project, &output)?;
            Ok(json!({ "input": input, "output": output, "bpm": project.bpm, "time_scale": scale }))
        }
        AutomationRequest::SetNote {
            input,
            output,
            part_index,
            note_index,
            lyric,
            pitch,
        } => {
            let mut project = load_project(&input)?;
            let note = automation_note_mut(&mut project, part_index, note_index)?;
            if let Some(lyric) = lyric {
                note.lyric = lyric;
            }
            if let Some(pitch) = pitch {
                let valid = kamafeu::dsp::pitch::note_name_to_midi(&pitch)
                    .or_else(|| pitch.parse::<u8>().ok())
                    .is_some();
                if !valid {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        "pitch deve ser uma nota como C4 ou uma chave MIDI entre 0 e 127",
                    )
                    .into());
                }
                note.pitch = pitch;
            }
            save_project(&project, &output)?;
            Ok(
                json!({ "input": input, "output": output, "part_index": part_index, "note_index": note_index }),
            )
        }
        AutomationRequest::RemoveNote {
            input,
            output,
            part_index,
            note_index,
        } => {
            let mut project = load_project(&input)?;
            project.normalize();
            let part = project.parts.get_mut(part_index).ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!("parte vocal {part_index} não existe"),
                )
            })?;
            if note_index >= part.notes.len() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("nota {note_index} não existe na parte vocal {part_index}"),
                )
                .into());
            }
            let removed = part.notes.remove(note_index);
            project.normalize();
            save_project(&project, &output)?;
            Ok(json!({
                "input": input,
                "output": output,
                "part_index": part_index,
                "note_index": note_index,
                "removed": {
                    "lyric": removed.lyric,
                    "pitch": removed.pitch,
                    "position_ms": removed.position_ms,
                    "duration_ms": removed.duration_ms,
                },
            }))
        }
        AutomationRequest::ApplyLyrics {
            input,
            output,
            part_index,
            text,
            mode,
            start_note_index,
        } => {
            let split_mode = automation_lyrics_split_mode(mode.as_deref())?;
            let tokens = kamafeu::gui::lyrics_dialog::split_lyrics(&text, split_mode);
            if tokens.is_empty() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "text deve conter ao menos uma palavra ou sílaba",
                )
                .into());
            }
            let mut project = load_project(&input)?;
            project.normalize();
            let part = project.parts.get_mut(part_index).ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!("parte vocal {part_index} não existe"),
                )
            })?;
            let start_note_index = start_note_index.unwrap_or(0);
            if start_note_index >= part.notes.len() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "nota inicial {start_note_index} não existe na parte vocal {part_index}"
                    ),
                )
                .into());
            }
            let mut target_indices = (0..part.notes.len()).collect::<Vec<_>>();
            target_indices.sort_by(|left, right| {
                part.notes[*left]
                    .position_ms
                    .partial_cmp(&part.notes[*right].position_ms)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let start_at = target_indices
                .iter()
                .position(|index| *index == start_note_index)
                .expect("the requested note index is already validated");
            let target_indices = &target_indices[start_at..];
            let applied = tokens.len().min(target_indices.len());
            for (token, note_index) in tokens.iter().zip(target_indices.iter()) {
                part.notes[*note_index].lyric = token.clone();
            }
            save_project(&project, &output)?;
            Ok(json!({
                "input": input,
                "output": output,
                "part_index": part_index,
                "start_note_index": start_note_index,
                "mode": mode.unwrap_or_else(|| "hyphens_and_spaces".to_string()),
                "token_count": tokens.len(),
                "applied_count": applied,
                "unassigned_tokens": tokens.len().saturating_sub(applied),
                "unchanged_notes": target_indices.len().saturating_sub(applied),
            }))
        }
        AutomationRequest::AddMarker {
            input,
            output,
            name,
            position_ms,
            color,
        } => {
            if !position_ms.is_finite() || position_ms < 0.0 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "position_ms deve ser um número finito maior ou igual a zero",
                )
                .into());
            }
            if name.trim().is_empty() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "name do marcador não pode estar vazio",
                )
                .into());
            }
            let mut project = load_project(&input)?;
            let mut marker = kamafeu::project::UProjectMarker::new(name, position_ms);
            marker.color = color;
            let marker_name = marker.name.trim().to_string();
            project.markers.push(marker);
            project.normalize();
            let marker_index = project
                .markers
                .iter()
                .rposition(|marker| {
                    (marker.position_ms - position_ms).abs() < f64::EPSILON
                        && marker.name == marker_name
                })
                .unwrap_or_else(|| project.markers.len().saturating_sub(1));
            save_project(&project, &output)?;
            Ok(json!({
                "input": input,
                "output": output,
                "marker_index": marker_index,
                "marker": project.markers.get(marker_index).map(|marker| json!({
                    "name": marker.name,
                    "position_ms": marker.position_ms,
                    "color": marker.color,
                })),
            }))
        }
        AutomationRequest::RemoveMarker {
            input,
            output,
            marker_index,
        } => {
            let mut project = load_project(&input)?;
            if marker_index >= project.markers.len() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("marcador {marker_index} não existe"),
                )
                .into());
            }
            let marker = project.markers.remove(marker_index);
            save_project(&project, &output)?;
            Ok(json!({
                "input": input,
                "output": output,
                "marker_index": marker_index,
                "removed": {
                    "name": marker.name,
                    "position_ms": marker.position_ms,
                    "color": marker.color,
                },
            }))
        }
        AutomationRequest::AddSection {
            input,
            output,
            name,
            start_ms,
            end_ms,
            color,
        } => {
            if !start_ms.is_finite() || !end_ms.is_finite() || start_ms < 0.0 || end_ms < start_ms {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "start_ms e end_ms devem ser finitos, com end_ms maior ou igual a start_ms",
                )
                .into());
            }
            if name.trim().is_empty() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "name da seção não pode estar vazio",
                )
                .into());
            }
            let mut project = load_project(&input)?;
            let mut section = kamafeu::project::UProjectSection::new(name, start_ms, end_ms);
            section.color = color;
            let section_name = section.name.trim().to_string();
            project.sections.push(section);
            project.normalize();
            let section_index = project
                .sections
                .iter()
                .rposition(|section| {
                    section.name == section_name
                        && (section.start_ms - start_ms).abs() < f64::EPSILON
                        && (section.end_ms - end_ms).abs() < f64::EPSILON
                })
                .unwrap_or_else(|| project.sections.len().saturating_sub(1));
            save_project(&project, &output)?;
            Ok(json!({
                "input": input,
                "output": output,
                "section_index": section_index,
                "section": project.sections.get(section_index).map(|section| json!({
                    "name": section.name,
                    "start_ms": section.start_ms,
                    "end_ms": section.end_ms,
                    "color": section.color,
                })),
            }))
        }
        AutomationRequest::RemoveSection {
            input,
            output,
            section_index,
        } => {
            let mut project = load_project(&input)?;
            if section_index >= project.sections.len() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("seção {section_index} não existe"),
                )
                .into());
            }
            let section = project.sections.remove(section_index);
            save_project(&project, &output)?;
            Ok(json!({
                "input": input,
                "output": output,
                "section_index": section_index,
                "removed": {
                    "name": section.name,
                    "start_ms": section.start_ms,
                    "end_ms": section.end_ms,
                    "color": section.color,
                },
            }))
        }
        AutomationRequest::SetPitchBend {
            input,
            output,
            part_index,
            note_index,
            points,
            snap_first,
        } => {
            if points.iter().any(|point| {
                !point.time_offset_ms.is_finite() || !point.pitch_offset_cents.is_finite()
            }) {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "pontos de pitch bend devem conter valores finitos",
                )
                .into());
            }
            let mut project = load_project(&input)?;
            let note = automation_note_mut(&mut project, part_index, note_index)?;
            note.pitch_bend.points = points
                .into_iter()
                .map(|point| kamafeu::project::UPitchBendPoint {
                    time_offset_ms: point.time_offset_ms,
                    pitch_offset_cents: point.pitch_offset_cents,
                    shape: point.shape,
                })
                .collect();
            if let Some(snap_first) = snap_first {
                note.pitch_bend.snap_first = snap_first;
            }
            project.normalize();
            save_project(&project, &output)?;
            Ok(json!({
                "input": input,
                "output": output,
                "part_index": part_index,
                "note_index": note_index,
                "point_count": project.parts[part_index].notes[note_index].pitch_bend.points.len(),
            }))
        }
        AutomationRequest::SetExpression {
            input,
            output,
            part_index,
            note_index,
            dynamics,
            pitch_delta,
            gender,
            breathiness,
            velocity,
            modulation,
            volume,
            attack,
            decay,
        } => {
            let values = [
                dynamics,
                pitch_delta,
                gender,
                breathiness,
                velocity,
                modulation,
                volume,
                attack,
                decay,
            ];
            if values.iter().flatten().any(|value| !value.is_finite()) {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "expressões devem conter apenas valores finitos",
                )
                .into());
            }
            let mut project = load_project(&input)?;
            let note = automation_note_mut(&mut project, part_index, note_index)?;
            if let Some(value) = dynamics {
                note.expressions.dynamics = value;
            }
            if let Some(value) = pitch_delta {
                note.expressions.pitch_delta = value;
            }
            if let Some(value) = gender {
                note.expressions.gender = value;
            }
            if let Some(value) = breathiness {
                note.expressions.breathiness = value;
            }
            if let Some(value) = velocity {
                note.expressions.velocity = value;
            }
            if let Some(value) = modulation {
                note.expressions.modulation = value;
            }
            if let Some(value) = volume {
                note.expressions.volume = value;
            }
            if let Some(value) = attack {
                note.expressions.attack = value;
            }
            if let Some(value) = decay {
                note.expressions.decay = value;
            }
            project.normalize();
            let expressions = &project.parts[part_index].notes[note_index].expressions;
            save_project(&project, &output)?;
            Ok(json!({
                "input": input,
                "output": output,
                "part_index": part_index,
                "note_index": note_index,
                "expressions": {
                    "dynamics": expressions.dynamics,
                    "pitch_delta": expressions.pitch_delta,
                    "gender": expressions.gender,
                    "breathiness": expressions.breathiness,
                    "velocity": expressions.velocity,
                    "modulation": expressions.modulation,
                    "volume": expressions.volume,
                    "attack": expressions.attack,
                    "decay": expressions.decay,
                },
            }))
        }
        AutomationRequest::Render {
            voicebank,
            input,
            output,
            sample_rate,
            venus,
            vocal_only,
            dither,
        } => {
            let summary = render::render_project_to_file(
                &voicebank,
                &input,
                &output,
                RenderOutputFormat::Auto,
                dither,
                sample_rate.unwrap_or(44_100),
                venus,
                None,
                vocal_only,
            )?;
            Ok(render::render_summary_json(&summary))
        }
        AutomationRequest::ValidateVoicebank { path } => voicebank_validation_json(&path),
        AutomationRequest::Extensions {
            path,
            verify_wasm,
            grant_permissions,
            format_filter,
            phonemizer_filter,
            invoke_id,
            invoke_export,
            invoke_argument,
            invoke_bytes,
            invoke_parameters,
        } => {
            let path = path.unwrap_or_else(|| PathBuf::from("extensions"));
            if let Some(id) = invoke_id {
                extensions::invoke_extension_json(
                    &path,
                    &id,
                    invoke_export.as_deref().unwrap_or("analyze"),
                    invoke_argument.unwrap_or(0),
                    invoke_bytes,
                    &grant_permissions,
                    &invoke_parameters,
                )
            } else {
                Ok(extensions::extension_catalog_json(
                    &path,
                    verify_wasm,
                    &grant_permissions,
                    format_filter.as_deref(),
                    phonemizer_filter.as_deref(),
                ))
            }
        }
        AutomationRequest::CacheInfo => Ok(cache_info_json()),
        AutomationRequest::CachePrune { days } => {
            let days = days.unwrap_or(30);
            let removed = kamafeu::renderer::resampler_cache::cleanup_older_than(days)?;
            Ok(json!({ "days": days, "removed": removed }))
        }
    }
}

pub(super) fn voicebank_catalog_json(
    mut directories: Vec<PathBuf>,
    include_default: bool,
) -> Value {
    if directories.is_empty() || include_default {
        for directory in SingerScanner::default_singers_directories() {
            if !directories.contains(&directory) {
                directories.push(directory);
            }
        }
    }

    let voicebanks = SingerScanner::scan_directories(&directories)
        .into_iter()
        .map(|singer| {
            json!({
                "name": singer.name,
                "author": singer.author,
                "voice_type": singer.voice_type,
                "path": singer.path,
                "website": singer.web,
                "image_path": singer.image_path,
            })
        })
        .collect::<Vec<_>>();
    json!({
        "directories": directories,
        "count": voicebanks.len(),
        "voicebanks": voicebanks,
    })
}

pub(super) fn cache_info_json() -> Value {
    let directory = kamafeu::renderer::resampler_cache::persistent_cache_dir();
    let (disk_file_count, disk_bytes) = kamafeu::renderer::resampler_cache::get_disk_cache_stats();
    let (memory_entries, memory_samples) =
        kamafeu::renderer::resampler_cache::get_memory_cache_stats();
    let stats = kamafeu::renderer::resampler_cache::cache_stats();
    json!({
        "directory": directory,
        "disk_file_count": disk_file_count,
        "disk_bytes": disk_bytes,
        "memory_entries": memory_entries,
        "memory_samples": memory_samples,
        "hits": stats.hits,
        "misses": stats.misses,
        "renders": stats.renders,
    })
}

fn automation_note_mut(
    project: &mut UProject,
    part_index: usize,
    note_index: usize,
) -> Result<&mut UNote, Box<dyn std::error::Error>> {
    let part = project.parts.get_mut(part_index).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("part_index inválido: {part_index}"),
        )
    })?;
    part.notes.get_mut(note_index).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("note_index inválido para a parte {part_index}: {note_index}"),
        )
        .into()
    })
}
