use super::*;

pub(super) struct RenderSummary {
    pub(super) output: PathBuf,
    pub(super) format: AudioExportFormat,
    pub(super) sample_rate: u32,
    pub(super) channels: u16,
    pub(super) frame_count: usize,
    pub(super) note_count: usize,
    pub(super) diagnostics: AudioDiagnostics,
    pub(super) silent: bool,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn render_project_to_file(
    voicebank_path: &PathBuf,
    input: &PathBuf,
    output: &PathBuf,
    requested_format: RenderOutputFormat,
    dither: bool,
    sample_rate: u32,
    venus: bool,
    external_resampler: Option<PathBuf>,
    vocal_only: bool,
) -> Result<RenderSummary, Box<dyn std::error::Error>> {
    let voicebank = Voicebank::new(voicebank_path)?;
    let mut project = load_project(input)?;
    project.normalize();
    if project.imported_phoneme_policy
        == kamafeu::project::model::ImportedPhonemePolicy::Recalculate
    {
        kamafeu::terminal_log::debug(
            "Import",
            format!("phonemes: source={} policy=recalculate", input.display()),
        );
        project.recalculate_imported_phonemes();
    } else {
        kamafeu::terminal_log::debug(
            "Import",
            format!("phonemes: source={} policy=preserve", input.display()),
        );
    }
    if vocal_only {
        project.wave_parts.clear();
    }
    let note_count: usize = project.parts.iter().map(|part| part.notes.len()).sum();
    if note_count == 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "o projeto não contém notas renderizáveis",
        )
        .into());
    }

    let classic_resampler = NativeResamplerDriver;
    let world_resampler = NativeWorldResamplerDriver;
    let external_resampler = external_resampler.map(ExternalResamplerDriver::new);
    let resampler: &dyn ResamplerDriver = if let Some(ref external) = external_resampler {
        external
    } else if venus {
        &world_resampler
    } else {
        &classic_resampler
    };
    let native_wavtool = NativeWavtoolDriver;
    let render_options = RenderOptions {
        flags: project.flags.clone(),
        phonemizer_mode: project.phonemizer.unwrap_or_default(),
        ..RenderOptions::default()
    };
    let timing_diagnostics = std::sync::Arc::new(std::sync::Mutex::new(Vec::<
        kamafeu::renderer::timing::PhonemeTimingDiagnostic,
    >::new()));
    let timing_boundary_diagnostics = std::sync::Arc::new(std::sync::Mutex::new(Vec::<
        kamafeu::renderer::provenance::TimingBoundaryDiagnostic,
    >::new()));
    let render_commands = std::sync::Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    let phoneme_renders = std::sync::Arc::new(std::sync::Mutex::new(Vec::<
        kamafeu::renderer::PhonemeRenderDiagnostic,
    >::new()));
    let cache_stats = std::sync::Arc::new(std::sync::Mutex::new((0usize, 0usize)));
    let timing_capture = std::sync::Arc::clone(&timing_diagnostics);
    let timing_boundary_capture = std::sync::Arc::clone(&timing_boundary_diagnostics);
    let commands_capture = std::sync::Arc::clone(&render_commands);
    let phoneme_renders_capture = std::sync::Arc::clone(&phoneme_renders);
    let cache_capture = std::sync::Arc::clone(&cache_stats);
    let progress_callback = move |_progress: f32, message: &str| {
        if let Some(payload) = message.strip_prefix("[Timing] ") {
            if let Ok(diagnostic) = serde_json::from_str(payload) {
                if let Ok(mut captured) = timing_capture.lock() {
                    captured.push(diagnostic);
                }
            }
        }
        if let Some(diagnostic) = kamafeu::renderer::provenance::parse_timing_boundary_log(message)
        {
            if let Ok(mut captured) = timing_boundary_capture.lock() {
                captured.push(diagnostic);
            }
        }
        if let Some(command) = message
            .strip_prefix("  [Resampler Command] ")
            .or_else(|| message.strip_prefix("  [Wavtool Command] "))
        {
            if let Ok(mut commands) = commands_capture.lock() {
                commands.push(command.to_string());
            }
        }
        if let Some(payload) = message.strip_prefix("  [Phoneme Render] ") {
            if let Ok(diagnostic) = serde_json::from_str(payload) {
                if let Ok(mut captured) = phoneme_renders_capture.lock() {
                    captured.push(diagnostic);
                }
            }
        }
        if let Ok(mut stats) = cache_capture.lock() {
            if message.contains("[Resampler Cache] hit") {
                stats.0 += 1;
            } else if message.contains("[Resampler Cache] miss") {
                stats.1 += 1;
            }
        }
    };
    let rendered = ProjectRenderer::render_project_with_drivers(
        &project,
        &voicebank,
        sample_rate,
        0.0,
        resampler,
        &native_wavtool,
        &render_options,
        Some(&progress_callback),
    );
    if let Some(error) = rendered.error {
        return Err(std::io::Error::other(error).into());
    }

    let voicebank_ref = &voicebank;
    let phoneme_resolutions = project
        .parts
        .iter()
        .enumerate()
        .flat_map(|(part_index, part)| {
            let notes = &part.notes;
            let phones = kamafeu::phonemizer::JapanesePhonemizer::apply_phonemizer(
                notes,
                &voicebank,
                render_options.phonemizer_mode,
            );
            phones.into_iter().map(move |phone| {
                let (oto_wav, oto_preutter_ms, oto_overlap_ms) = phone
                    .find_entry(voicebank_ref)
                    .map(|item| {
                        (
                            Some(item.wav_filename.clone()),
                            Some(item.preutterance),
                            Some(item.overlap),
                        )
                    })
                    .unwrap_or_default();
                kamafeu::renderer::PhonemeResolutionDiagnostic {
                    part_index,
                    phonemizer_implementation: format!(
                        "{:?}",
                        kamafeu::phonemizer::OpenUtauPhonemizer::implementation(
                            render_options.phonemizer_mode
                        )
                    ),
                    note_index: phone.note_index,
                    lyric: phone.lyric,
                    pitch: phone.pitch,
                    requested_timbre: notes
                        .get(phone.note_index)
                        .and_then(|note| note.timbre.clone()),
                    resolved_alias: phone.resolved_alias,
                    resolved_timbre: phone.resolved_timbre,
                    oto_wav,
                    oto_preutter_ms,
                    oto_overlap_ms,
                    position_ms: phone.position_ms,
                    duration_ms: phone.duration_ms,
                    flags: phone.flags,
                }
            })
        })
        .collect::<Vec<_>>();
    for diagnostic in &phoneme_resolutions {
        kamafeu::terminal_log::debug(
            "PhonemeResolution",
            serde_json::to_string(diagnostic).unwrap_or_else(|_| "{}".to_string()),
        );
    }

    let format = requested_format
        .resolve(output)
        .map_err(|error| Error::new(ErrorKind::InvalidInput, error))?;
    AudioExporter::export_audio_with_dither(
        output,
        &rendered.samples,
        sample_rate,
        rendered.channels,
        format,
        if dither {
            DitherMode::Tpdf
        } else {
            DitherMode::None
        },
    )
    .map_err(Error::other)?;
    let project_bytes = serde_json::to_vec(&project)?;
    let timing_diagnostics = timing_diagnostics
        .lock()
        .map(|diagnostics| diagnostics.clone())
        .unwrap_or_default();
    let timing_boundary_diagnostics = timing_boundary_diagnostics
        .lock()
        .map(|diagnostics| diagnostics.clone())
        .unwrap_or_default();
    let render_commands = render_commands
        .lock()
        .map(|commands| commands.clone())
        .unwrap_or_default();
    let phoneme_renders = phoneme_renders
        .lock()
        .map(|diagnostics| diagnostics.clone())
        .unwrap_or_default();
    let (cache_hits, cache_misses) = cache_stats.lock().map(|stats| *stats).unwrap_or_default();
    let mut provenance = RenderProvenance::new(
        kamafeu::renderer::provenance::content_fingerprint(&project_bytes),
        voicebank.content_fingerprint(),
        resampler.name(),
        native_wavtool.name(),
        sample_rate,
        rendered.channels,
        render_options.clone(),
    )
    .with_import_context(
        input.display().to_string(),
        input
            .extension()
            .and_then(|extension| extension.to_str())
            .unwrap_or_default()
            .to_ascii_lowercase(),
        match project.imported_phoneme_policy {
            kamafeu::project::model::ImportedPhonemePolicy::Preserve => "preserve",
            kamafeu::project::model::ImportedPhonemePolicy::Recalculate => "recalculate",
        },
        project
            .voicebank_path
            .clone()
            .or_else(|| project.voicebank.clone()),
        voicebank.root_path.display().to_string(),
    )
    .with_timing_diagnostics(timing_diagnostics)
    .with_timing_boundary_diagnostics(timing_boundary_diagnostics)
    .with_phoneme_resolutions(phoneme_resolutions)
    .with_phoneme_renders(phoneme_renders)
    .with_commands(render_commands)
    .with_cache_stats(cache_hits, cache_misses)
    .with_extension_details(project.extensions.iter().map(|extension| {
        kamafeu::renderer::RenderExtension {
            id: extension.id.clone(),
            version: extension.version.clone(),
            origin: extension.origin.clone(),
            manifest_fingerprint: extension.manifest_fingerprint.clone(),
        }
    }))
    .with_artifact_bytes("project", input.display().to_string(), &project_bytes);
    provenance
        .warnings
        .extend(provenance.timing_boundary_warnings());
    provenance.warnings.sort();
    provenance.warnings.dedup();
    let oto_path = voicebank.root_path.join("oto.ini");
    if oto_path.is_file() {
        provenance = provenance.with_artifact_file("oto", &oto_path)?;
    }
    let mut wav_paths = std::collections::BTreeSet::new();
    for entry in voicebank.entries.values() {
        let wav_path = voicebank.root_path.join(&entry.wav_filename);
        if wav_path.is_file() {
            wav_paths.insert(wav_path);
        }
    }
    for wav_path in wav_paths {
        provenance = provenance.with_artifact_file("voicebank_wav", wav_path)?;
    }
    AudioExporter::write_provenance_sidecar(output, &provenance).map_err(Error::other)?;
    let diagnostics = AudioDiagnostics::analyze(&rendered.samples, rendered.channels);
    Ok(RenderSummary {
        output: output.clone(),
        format,
        sample_rate,
        channels: rendered.channels,
        frame_count: rendered.frame_count(),
        note_count,
        silent: diagnostics.peak < 1e-5,
        diagnostics,
    })
}

pub(super) fn render_summary_json(summary: &RenderSummary) -> Value {
    json!({
        "output": summary.output,
        "format": summary.format.display_name(),
        "sample_rate": summary.sample_rate,
        "channels": summary.channels,
        "frame_count": summary.frame_count,
        "note_count": summary.note_count,
        "duration_ms": summary.frame_count as f64 / f64::from(summary.sample_rate.max(1)) * 1000.0,
        "audio": {
            "peak": summary.diagnostics.peak,
            "rms": summary.diagnostics.rms,
            "clipped_samples": summary.diagnostics.clipped_samples,
            "non_finite_samples": summary.diagnostics.non_finite_samples,
            "max_step": summary.diagnostics.max_step,
            "max_step_frame": summary.diagnostics.max_step_frame,
            "safe": summary.diagnostics.is_safe(),
            "silent": summary.silent,
        },
    })
}
