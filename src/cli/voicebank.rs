use super::*;

pub(super) fn handle(
    command: Commands,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Commands::VoicebankInfo { path, alias, pitch } => {
            let vb = Voicebank::new(&path)?;
            if json_output {
                let aliases = vb
                    .entries
                    .iter()
                    .take(10)
                    .map(|(alias, entry)| {
                        json!({
                            "alias": alias,
                            "wav": entry.wav_filename,
                            "offset_ms": entry.offset,
                            "preutterance_ms": entry.preutterance,
                        })
                    })
                    .collect::<Vec<_>>();
                let resolution = alias.as_deref().map(|alias| {
                    serde_json::to_value(vb.alias_resolution_report(alias, &pitch))
                        .expect("alias resolution report is serializable")
                });
                print_json(&json!({
                    "path": path,
                    "name": vb.name,
                    "author": vb.author,
                    "entry_count": vb.entries.len(),
                    "aliases": aliases,
                    "resolution": resolution,
                }))?;
            } else {
                kamafeu::terminal_log::out_header("Voicebank", &vb.name);
                kamafeu::terminal_log::out_kv("Author", &vb.author);
                kamafeu::terminal_log::out_kv("Entries", vb.entries.len());
                if let Some(alias) = alias {
                    let report = vb.alias_resolution_report(alias, &pitch);
                    match report.resolved_alias {
                        Some(resolved) => {
                            kamafeu::terminal_log::out_status(
                                "OK",
                                "Resolution",
                                format!(
                                    "{} @ {} -> {} / {} (preutter {}ms, overlap {}ms)",
                                    report.requested_alias,
                                    report.pitch,
                                    resolved,
                                    report.wav_filename.unwrap_or_default(),
                                    report.preutterance_ms.unwrap_or_default(),
                                    report.overlap_ms.unwrap_or_default()
                                ),
                            );
                        }
                        None => kamafeu::terminal_log::out_status(
                            "WARN",
                            "Resolution",
                            format!(
                                "{} @ {} -> {}",
                                report.requested_alias,
                                report.pitch,
                                report.reason.unwrap_or_else(|| "not resolved".to_string())
                            ),
                        ),
                    }
                }
            }
        }

        Commands::Voicebanks {
            paths,
            include_default,
        } => {
            let catalog = automation::voicebank_catalog_json(paths, include_default);
            if json_output {
                print_json(&catalog)?;
            } else {
                let directories = catalog["directories"].as_array();
                let singers = catalog["voicebanks"].as_array();
                kamafeu::terminal_log::out_header(
                    "Voicebank catalog",
                    format!("{} directories scanned", directories.map_or(0, Vec::len)),
                );
                if let Some(directories) = directories {
                    for directory in directories {
                        if let Some(path) = directory.as_str() {
                            kamafeu::terminal_log::out_list(path);
                        }
                    }
                }
                kamafeu::terminal_log::out_kv("Voicebanks", singers.map_or(0, Vec::len));
                if let Some(singers) = singers {
                    for singer in singers {
                        kamafeu::terminal_log::out_status(
                            "OK",
                            singer["name"].as_str().unwrap_or("Unknown"),
                            format!(
                                "{} | {} | {}",
                                singer["voice_type"].as_str().unwrap_or("Unknown"),
                                singer["author"].as_str().unwrap_or("Unknown"),
                                singer["path"].as_str().unwrap_or(""),
                            ),
                        );
                    }
                }
            }
        }

        Commands::ValidateVoicebank { path } => {
            let vb = Voicebank::new(&path)?;
            let report = vb.diagnostic_report();
            if json_output {
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
                print_json(&json!({
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
                }))?;
            } else {
                kamafeu::terminal_log::out_header("Voicebank validation", &vb.name);
                kamafeu::terminal_log::out_kv("Entries", report.entry_count);
                kamafeu::terminal_log::out_kv("Unique WAV", report.unique_wav_count);
                kamafeu::terminal_log::out_kv("Readable WAV", report.readable_wav_count);
                kamafeu::terminal_log::out_kv("Missing WAV", report.missing_wav_count);
                kamafeu::terminal_log::out_kv("Invalid WAV", report.invalid_wav_count);
                kamafeu::terminal_log::out_kv("Empty WAV", report.empty_wav_count);
                kamafeu::terminal_log::out_kv("Invalid oto", report.invalid_oto_timing_count);
                if report.sample_rates.is_empty() {
                    kamafeu::terminal_log::out_kv("Sample rates", "none");
                } else {
                    let sample_rates = report
                        .sample_rates
                        .iter()
                        .map(|(rate, count)| format!("{rate} Hz ({count})"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    kamafeu::terminal_log::out_kv("Sample rates", sample_rates);
                }
                if report.is_healthy() {
                    kamafeu::terminal_log::out_status("OK", "Result", "healthy");
                } else {
                    kamafeu::terminal_log::out_status(
                        "WARN",
                        "Result",
                        format!("{} issue(s)", report.issues.len()),
                    );
                    for issue in &report.issues {
                        kamafeu::terminal_log::out_list(format!(
                            "{} -> {}: {} (hint: {})",
                            issue.alias,
                            issue.wav_filename,
                            issue.detail,
                            issue.suggestion()
                        ));
                    }
                }
            }
            if !report.is_healthy() {
                return Err(
                    Error::new(ErrorKind::InvalidData, "voicebank validation failed").into(),
                );
            }
        }

        Commands::MigrateVoicebank {
            path,
            destination,
            profile,
            mapping,
            midi,
        } => {
            let source = Voicebank::new(&path)?;
            let phonetic_profile: kamafeu::voicebank::native::PhoneticProfile =
                serde_json::from_slice(&fs::read(&profile)?)?;
            let phone_mappings = if let Some(mapping_path) = mapping {
                serde_json::from_slice::<std::collections::BTreeMap<String, Vec<String>>>(
                    &fs::read(mapping_path)?,
                )?
            } else {
                std::collections::BTreeMap::new()
            };
            let slug: String = source
                .name
                .chars()
                .map(|character| {
                    if character.is_ascii_alphanumeric() {
                        character.to_ascii_lowercase()
                    } else {
                        '-'
                    }
                })
                .collect();
            let slug = slug.trim_matches('-');
            let slug = if slug.is_empty() { "voicebank" } else { slug };
            let options = kamafeu::voicebank::migration::LegacyMigrationOptions {
                voice_id: format!("org.kamafeu.voice.{slug}"),
                bank_id: format!("org.kamafeu.voice.{slug}.native"),
                bank_version: "0.1.0".into(),
                display_name: format!("{} · Nativa (rascunho)", source.name),
                publisher: if source.author.trim().is_empty() {
                    "Autor não informado".into()
                } else {
                    source.author.clone()
                },
                profile: phonetic_profile,
                style_id: "natural".into(),
                style_display_name: "Natural".into(),
                layer_id: format!("natural-midi-{midi}"),
                recorded_midi: midi,
                phone_mappings,
            };
            let report = kamafeu::voicebank::migration::migrate_legacy_voicebank_to_copy(
                &source,
                &destination,
                &options,
            )?;
            if json_output {
                print_json(&serde_json::to_value(report)?)?;
            } else {
                println!(
                    "KFV 2 draft: {}\nUnits: {} | skipped entries: {} | copied assets: {}\nContent SHA-256: {}\nReview migration_report.json in the output directory before using or distributing this draft.",
                    report.destination.display(),
                    report.migrated_units,
                    report.skipped_entries,
                    report.copied_assets,
                    report.content_sha256,
                );
            }
        }

        _ => unreachable!("voicebank command dispatcher received another command"),
    }
    Ok(())
}
