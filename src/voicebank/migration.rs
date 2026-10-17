//! Assisted, non-destructive conversion from classic oto.ini data to a KFV 2 copy.
use crate::oto::{OtoEntry, Voicebank};
use crate::voicebank::native::{
    safe_relative_path, Asset, Context, Defaults, EngineRequirement, Layer, Manifest, NativeBank,
    PhoneticProfile, Quality, RenderTuning, SourceRegion, Style, Unit, UnitFile, UnitTiming,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

const MIGRATION_MAX_ASSET_FRAMES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct LegacyMigrationOptions {
    pub voice_id: String,
    pub bank_id: String,
    pub bank_version: String,
    pub display_name: String,
    pub publisher: String,
    pub profile: PhoneticProfile,
    pub style_id: String,
    pub style_display_name: String,
    pub layer_id: String,
    pub recorded_midi: u8,
    /// Explicit mapping keyed by the exact alias stored in the source oto.ini.
    /// Unmapped aliases are only inferred when every whitespace token is an
    /// exact phone ID in `profile`; all such inferences remain unreviewed.
    pub phone_mappings: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationEntryStatus {
    Migrated,
    Skipped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MappingOrigin {
    Explicit,
    ExactAliasTokens,
    Unmapped,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LegacyEntryMigration {
    pub alias: String,
    pub wav_filename: String,
    pub status: MigrationEntryStatus,
    pub mapping_origin: MappingOrigin,
    pub phones: Vec<String>,
    pub unit_id: Option<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MigrationReport {
    pub source_name: String,
    pub source_author: String,
    pub destination: PathBuf,
    pub content_sha256: String,
    pub migrated_units: usize,
    pub skipped_entries: usize,
    pub copied_assets: usize,
    pub entries: Vec<LegacyEntryMigration>,
    pub warnings: Vec<String>,
}

/// Read-only estimate shown before the graphical migration writes a draft.
/// WAV decoding and source-region validation still happen during migration,
/// so `potential_units` is an upper bound rather than a promise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LegacyMigrationPreview {
    pub total_entries: usize,
    pub explicit_mappings: usize,
    pub inferred_aliases: usize,
    pub unmapped_aliases: usize,
    pub invalid_mappings: usize,
    pub alias_collisions: usize,
    pub potential_units: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug)]
struct PreparedEntry {
    report: LegacyEntryMigration,
    asset_path: Option<String>,
    sample_rate_hz: Option<u32>,
    frame_count: Option<u64>,
}

/// Inspect mapping coverage without creating files or copying audio.
pub fn preview_legacy_voicebank_migration(
    source: &Voicebank,
    profile: &PhoneticProfile,
    phone_mappings: &BTreeMap<String, Vec<String>>,
) -> Result<LegacyMigrationPreview, String> {
    profile.validate()?;
    let source_root = fs::canonicalize(&source.root_path)
        .map_err(|error| format!("cannot resolve classic voicebank root: {error}"))?;
    let (mut aliases, mut warnings) = read_legacy_oto_entries(&source_root, source)?;
    aliases.sort_by(|left, right| {
        left.alias
            .cmp(&right.alias)
            .then_with(|| left.wav_filename.cmp(&right.wav_filename))
            .then_with(|| left.offset.total_cmp(&right.offset))
    });
    if aliases.is_empty() {
        return Err("classic voicebank has no entries in oto.ini to migrate".into());
    }

    let phone_classes: HashMap<&str, &str> = profile
        .symbols
        .iter()
        .map(|symbol| (symbol.id.as_str(), symbol.class.as_str()))
        .collect();
    let mut alias_counts = BTreeMap::<&str, usize>::new();
    for entry in &aliases {
        *alias_counts.entry(&entry.alias).or_default() += 1;
    }
    let mut preview = LegacyMigrationPreview {
        total_entries: aliases.len(),
        explicit_mappings: 0,
        inferred_aliases: 0,
        unmapped_aliases: 0,
        invalid_mappings: 0,
        alias_collisions: alias_counts.values().filter(|count| **count > 1).sum(),
        potential_units: 0,
        warnings: Vec::new(),
    };
    for entry in &aliases {
        let mapping = if let Some(phones) = phone_mappings.get(&entry.alias) {
            preview.explicit_mappings += 1;
            Some(phones.clone())
        } else {
            let inferred: Vec<String> = entry
                .alias
                .split_whitespace()
                .map(str::to_owned)
                .collect();
            if !inferred.is_empty()
                && inferred
                    .iter()
                    .all(|phone| phone_classes.contains_key(phone.as_str()))
            {
                preview.inferred_aliases += 1;
                Some(inferred)
            } else {
                preview.unmapped_aliases += 1;
                None
            }
        };
        if let Some(phones) = mapping {
            if phones.is_empty()
                || phones.len() > 2
                || phones
                    .iter()
                    .any(|phone| !phone_classes.contains_key(phone.as_str()))
                || (phones.len() == 1
                    && !matches!(
                        phone_classes.get(phones[0].as_str()).copied(),
                        Some("vowel" | "breath" | "release")
                    ))
                || entry.alias.trim().is_empty()
            {
                preview.invalid_mappings += 1;
            } else {
                preview.potential_units += 1;
            }
        }
    }
    if preview.alias_collisions > 0 {
        warnings.push(format!(
            "{} entradas pertencem a aliases duplicados; elas serão mantidas como candidatas separadas.",
            preview.alias_collisions
        ));
    }
    warnings.push(
        "A prévia não decodifica WAVs nem valida regiões de origem; a conversão pode ignorar mais entradas.".into(),
    );
    warnings.push(
        "Mapas de prefixo, cores de voz, preutterance e licença não são transferidos integralmente."
            .into(),
    );
    preview.warnings.extend(warnings);
    Ok(preview)
}

/// Convert only entries with safe source WAVs and explicit or exact-token phone
/// mappings. The source bank is read-only; output must be a new sibling/outside
/// directory. The report distinguishes inferred mappings and legacy fields
/// that have no exact native equivalent. Output remains a draft until reviewed
/// and licensed by its creator.
pub fn migrate_legacy_voicebank_to_copy(
    source: &Voicebank,
    destination: &Path,
    options: &LegacyMigrationOptions,
) -> Result<MigrationReport, String> {
    options.profile.validate()?;
    let mut manifest = migration_manifest(source, options);
    manifest.validate()?;
    if options.recorded_midi > 127 {
        return Err("recorded MIDI must be between 0 and 127".into());
    }

    let source_root = fs::canonicalize(&source.root_path)
        .map_err(|error| format!("cannot resolve classic voicebank root: {error}"))?;
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create migration output parent: {error}"))?;
    let parent = fs::canonicalize(parent)
        .map_err(|error| format!("cannot resolve migration output parent: {error}"))?;
    let name = destination
        .file_name()
        .ok_or_else(|| "migration destination must name a new directory".to_string())?;
    let destination = parent.join(name);
    if destination.exists() || fs::symlink_metadata(&destination).is_ok() {
        return Err(format!(
            "migration destination already exists: {}",
            destination.display()
        ));
    }
    if destination.starts_with(&source_root) {
        return Err("migration output must be outside the original voicebank".into());
    }

    let (mut aliases, oto_warnings) = read_legacy_oto_entries(&source_root, source)?;
    aliases.sort_by(|left, right| {
        left.alias
            .cmp(&right.alias)
            .then_with(|| left.wav_filename.cmp(&right.wav_filename))
            .then_with(|| left.offset.total_cmp(&right.offset))
    });
    if aliases.is_empty() {
        return Err("classic voicebank has no entries in oto.ini to migrate".into());
    }

    let mut prepared = Vec::with_capacity(aliases.len());
    let mut unit_files = UnitFile {
        format: "kamafeu.units".into(),
        schema_version: "1.0.0".into(),
        layer_id: options.layer_id.clone(),
        assets: Vec::new(),
        units: Vec::new(),
    };
    let mut copied_assets: HashMap<String, (String, u32, u64, PathBuf)> = HashMap::new();
    let mut warnings = vec![
        "Each migrated unit is marked unreviewed. Check phone mapping and source marks in Copaíba before use.".into(),
        "Preutterance and pitch-dependent prefix.map/voice-color selection are not represented by this one-layer migration.".into(),
        format!("The single layer uses MIDI {} as supplied by the caller; recording pitch was not estimated.", options.recorded_midi),
        "Legacy aliases are retained for traceability; context is generalized to 'any' and must be reviewed.".into(),
        "No license is inferred or copied. Define distribution rights and credits before sharing this draft.".into(),
    ];
    warnings.extend(oto_warnings);
    let mut alias_counts = BTreeMap::<String, usize>::new();
    for entry in &aliases {
        *alias_counts.entry(entry.alias.clone()).or_default() += 1;
    }
    if source.prefix_map.len() > 0 {
        warnings.push(format!(
            "{} pitch prefix/suffix mappings were flattened into the single MIDI {} layer.",
            source.prefix_map.len(),
            options.recorded_midi
        ));
    }
    if !source.prefix_map.colors().collect::<Vec<_>>().is_empty() {
        warnings.push(
            "Source voice colors were not transferred; KFV 2 styles must be authored explicitly."
                .into(),
        );
    }

    let symbols: HashMap<&str, &str> = options
        .profile
        .symbols
        .iter()
        .map(|symbol| (symbol.id.as_str(), symbol.class.as_str()))
        .collect();
    for (index, entry) in aliases.into_iter().enumerate() {
        let mut report = LegacyEntryMigration {
            alias: entry.alias.clone(),
            wav_filename: entry.wav_filename.clone(),
            status: MigrationEntryStatus::Skipped,
            mapping_origin: MappingOrigin::Unmapped,
            phones: Vec::new(),
            unit_id: None,
            warnings: Vec::new(),
        };
        if alias_counts.get(&entry.alias).copied().unwrap_or(0) > 1 {
            report.warnings.push(
                "Alias collision: another entry in oto.ini has the same alias; this recording was retained as a separate candidate.".into(),
            );
        }
        let mapping = if let Some(phones) = options.phone_mappings.get(&entry.alias) {
            report.mapping_origin = MappingOrigin::Explicit;
            report.warnings.push(
                "Phone sequence came from the supplied alias map; verify it against the original reclist before use.".into(),
            );
            Some(phones.clone())
        } else {
            let inferred: Vec<String> = entry.alias.split_whitespace().map(str::to_owned).collect();
            if !inferred.is_empty()
                && inferred
                    .iter()
                    .all(|phone| symbols.contains_key(phone.as_str()))
            {
                report.mapping_origin = MappingOrigin::ExactAliasTokens;
                report.warnings.push(
                    "Phone IDs were inferred by exact whitespace-token match; human review is required.".into(),
                );
                Some(inferred)
            } else {
                None
            }
        };
        let Some(phones) = mapping else {
            report.warnings.push(
                "No unambiguous phone mapping was supplied or inferred; entry was omitted.".into(),
            );
            prepared.push(PreparedEntry {
                report,
                asset_path: None,
                sample_rate_hz: None,
                frame_count: None,
            });
            continue;
        };
        report.phones = phones.clone();
        if phones.is_empty()
            || phones.len() > 2
            || phones
                .iter()
                .any(|phone| !symbols.contains_key(phone.as_str()))
        {
            report.warnings.push(
                "Mapping must contain one or two phone IDs declared by the selected profile; entry was omitted.".into(),
            );
            prepared.push(PreparedEntry {
                report,
                asset_path: None,
                sample_rate_hz: None,
                frame_count: None,
            });
            continue;
        }
        let classes: Vec<&str> = phones.iter().map(|phone| symbols[phone.as_str()]).collect();
        let is_vowel = |class: &str| class == "vowel";
        let kind = match classes.as_slice() {
            [class] if is_vowel(class) => Some("V"),
            [class] if *class == "breath" => Some("breath"),
            [class] if *class == "release" => Some("release"),
            [left, right] if !is_vowel(left) && is_vowel(right) => Some("CV"),
            [left, right] if is_vowel(left) && !is_vowel(right) => Some("VC"),
            [left, right] if is_vowel(left) && is_vowel(right) => Some("VV"),
            [_, _] => Some("CC"),
            [_] => {
                report.warnings.push(
                    "A standalone consonant has no KFV 2 unit kind in the selected mapping; entry was omitted.".into(),
                );
                prepared.push(PreparedEntry {
                    report,
                    asset_path: None,
                    sample_rate_hz: None,
                    frame_count: None,
                });
                continue;
            }
            _ => unreachable!("mapping length is checked"),
        };
        let kind = kind.expect("supported one- and two-phone mappings return a unit kind");
        if entry.alias.trim().is_empty() {
            report
                .warnings
                .push("Empty legacy alias retained only as a trace note; unit was omitted.".into());
            prepared.push(PreparedEntry {
                report,
                asset_path: None,
                sample_rate_hz: None,
                frame_count: None,
            });
            continue;
        }
        if !safe_relative_path(&entry.wav_filename) {
            report
                .warnings
                .push("Unsafe WAV path referenced by oto.ini; entry was omitted.".into());
            prepared.push(PreparedEntry {
                report,
                asset_path: None,
                sample_rate_hz: None,
                frame_count: None,
            });
            continue;
        }
        let source_path = source_root.join(&entry.wav_filename);
        let canonical_source = match fs::canonicalize(&source_path) {
            Ok(path) if path.starts_with(&source_root) && path.is_file() => path,
            _ => {
                report.warnings.push("WAV is missing, not a regular file, or resolves outside the source bank; entry was omitted.".into());
                prepared.push(PreparedEntry {
                    report,
                    asset_path: None,
                    sample_rate_hz: None,
                    frame_count: None,
                });
                continue;
            }
        };
        let reader = match hound::WavReader::open(&canonical_source) {
            Ok(reader) => reader,
            Err(error) => {
                report.warnings.push(format!(
                    "WAV cannot be decoded ({error}); entry was omitted."
                ));
                prepared.push(PreparedEntry {
                    report,
                    asset_path: None,
                    sample_rate_hz: None,
                    frame_count: None,
                });
                continue;
            }
        };
        let spec = reader.spec();
        let frame_count = u64::from(reader.duration());
        if spec.channels != 1
            || spec.sample_rate == 0
            || frame_count == 0
            || frame_count > MIGRATION_MAX_ASSET_FRAMES
        {
            report.warnings.push("KFV 2 migration requires a non-empty mono WAV under the frame limit; entry was omitted.".into());
            prepared.push(PreparedEntry {
                report,
                asset_path: None,
                sample_rate_hz: None,
                frame_count: None,
            });
            continue;
        }
        let start_frame = match oto_start_frame(entry.offset, spec.sample_rate, frame_count) {
            Some(frame) if frame < frame_count => frame,
            _ => {
                report.warnings.push(
                    "Offset in oto.ini is non-finite or outside the source WAV; entry was omitted.".into(),
                );
                prepared.push(PreparedEntry {
                    report,
                    asset_path: None,
                    sample_rate_hz: None,
                    frame_count: None,
                });
                continue;
            }
        };
        let end_frame = match oto_end_frame(&entry, start_frame, frame_count, spec.sample_rate) {
            Some(frame) if frame > start_frame => frame,
            _ => {
                report.warnings.push(
                    "Cutoff in oto.ini is invalid or leaves an empty source region; entry was omitted."
                        .into(),
                );
                prepared.push(PreparedEntry {
                    report,
                    asset_path: None,
                    sample_rate_hz: None,
                    frame_count: None,
                });
                continue;
            }
        };
        let consonant_frames =
            match milliseconds_to_frames(entry.consonant.max(0.0), spec.sample_rate) {
                Some(frames) => frames,
                None => {
                    report
                        .warnings
                        .push("Consonant mark in oto.ini is non-finite; entry was omitted.".into());
                    prepared.push(PreparedEntry {
                        report,
                        asset_path: None,
                        sample_rate_hz: None,
                        frame_count: None,
                    });
                    continue;
                }
            };
        let fixed_end_frame = match start_frame.checked_add(consonant_frames) {
            Some(frame) if frame < end_frame => frame,
            _ => {
                report.warnings.push("Consonant mark in oto.ini reaches or exceeds the cropped source end; entry was omitted.".into());
                prepared.push(PreparedEntry {
                    report,
                    asset_path: None,
                    sample_rate_hz: None,
                    frame_count: None,
                });
                continue;
            }
        };

        let mut source_region = SourceRegion {
            start_frame,
            end_frame,
            anchor_frame: fixed_end_frame,
            fixed_end_frame,
            sustain: None,
            release_start_frame: None,
        };
        match (entry.loop_start, entry.loop_end) {
            (Some(start), Some(end)) => match (
                milliseconds_to_frames(start, spec.sample_rate),
                milliseconds_to_frames(end, spec.sample_rate),
            ) {
                (Some(start_frame), Some(end_frame))
                    if fixed_end_frame <= start_frame
                        && start_frame < end_frame
                        && end_frame <= source_region.end_frame =>
                {
                    source_region.sustain = Some(crate::voicebank::native::FrameRange {
                        start_frame,
                        end_frame,
                    });
                    report.warnings.push("Legacy loop marks were mapped to a native sustain region; audition and recheck loop quality.".into());
                }
                _ => report.warnings.push("Legacy loop marks could not be represented as a valid KFV 2 sustain region and were dropped.".into()),
            },
            (Some(_), None) | (None, Some(_)) => {
                report.warnings.push("Only one legacy loop boundary exists; incomplete loop metadata was dropped.".into());
            }
            (None, None) => {}
        }
        if let Some(tail_start) = entry.tail_start {
            match milliseconds_to_frames(tail_start, spec.sample_rate) {
                Some(frame)
                    if frame >= source_region.start_frame
                        && frame < source_region.end_frame
                        && source_region
                            .sustain
                            .as_ref()
                            .is_none_or(|sustain| frame >= sustain.end_frame) =>
                {
                    source_region.release_start_frame = Some(frame);
                    report.warnings.push("Legacy tail start was mapped to a release marker; confirm that it does not duplicate another final.".into());
                }
                _ => report.warnings.push("Legacy tail start could not be represented as a valid native release and was dropped.".into()),
            }
        }

        let asset_path = copied_assets
            .get(&entry.wav_filename)
            .map(|(path, _, _, _)| path.clone())
            .unwrap_or_else(|| format!("audio/legacy/asset-{:06}.wav", copied_assets.len() + 1));
        let asset_id = copied_assets
            .entry(entry.wav_filename.clone())
            .or_insert_with(|| {
                (
                    asset_path.clone(),
                    spec.sample_rate,
                    frame_count,
                    canonical_source.clone(),
                )
            });
        if asset_id.1 != spec.sample_rate || asset_id.2 != frame_count {
            report.warnings.push(
                "Aliased WAV metadata changed while preparing migration; entry was omitted.".into(),
            );
            prepared.push(PreparedEntry {
                report,
                asset_path: None,
                sample_rate_hz: None,
                frame_count: None,
            });
            continue;
        }
        let unit_id = format!("legacy-{:06}", index + 1);
        let overlap = if entry.overlap.is_finite() {
            if entry.overlap < 0.0 {
                report.warnings.push(
                    "Negative overlap from oto.ini has no equivalent native crossfade and was clamped to zero.".into(),
                );
            }
            entry.overlap.max(0.0)
        } else {
            report
                .warnings
                .push("Non-finite overlap from oto.ini was replaced by a zero preferred crossfade.".into());
            0.0
        };
        let duration_ms = (end_frame - start_frame) as f64 * 1000.0 / f64::from(spec.sample_rate);
        let unit = Unit {
            id: unit_id.clone(),
            asset_id: format!(
                "asset-{:06}",
                copied_assets
                    .keys()
                    .position(|path| path == &entry.wav_filename)
                    .unwrap_or(0)
                    + 1
            ),
            kind: kind.into(),
            phones,
            left_context: Context {
                kind: "any".into(),
                phone: None,
            },
            right_context: Context {
                kind: "any".into(),
                phone: None,
            },
            legacy_aliases: vec![entry.alias.clone()],
            source: source_region,
            timing: UnitTiming {
                min_target_ms: 1.0,
                preferred_crossfade_ms: overlap.min(duration_ms),
                max_crossfade_ms: overlap.min(duration_ms),
            },
            render: RenderTuning::default(),
            quality: Quality::default(),
        };
        report.status = MigrationEntryStatus::Migrated;
        report.unit_id = Some(unit_id);
        unit_files.units.push(unit.clone());
        if entry.preutterance != 0.0 {
            report.warnings.push(
                "Preutterance from oto.ini is not represented by KFV 2 source regions and was omitted."
                    .into(),
            );
        }
        report
            .warnings
            .push("Context was generalized to 'any'; review it in Copaíba.".into());
        report.warnings.push(
            "A 1 ms minimum target duration was selected as a neutral native default.".into(),
        );
        prepared.push(PreparedEntry {
            report,
            asset_path: Some(asset_path),
            sample_rate_hz: Some(spec.sample_rate),
            frame_count: Some(frame_count),
        });
    }

    if unit_files.units.is_empty() {
        return Err(
            "no entries in oto.ini had a valid phone mapping and migratable mono source WAV".into(),
        );
    }
    let mut asset_id_by_path = BTreeMap::new();
    let mut next_asset = 1usize;
    for entry in &prepared {
        if let (Some(path), Some(sample_rate_hz), Some(frame_count)) =
            (&entry.asset_path, entry.sample_rate_hz, entry.frame_count)
        {
            if !asset_id_by_path.contains_key(path) {
                let id = format!("asset-{next_asset:06}");
                asset_id_by_path.insert(path.clone(), (id, sample_rate_hz, frame_count));
                next_asset += 1;
            }
        }
    }
    for entry in &prepared {
        if let Some(path) = entry.asset_path.as_ref() {
            if let Some((asset_id, _, _)) = asset_id_by_path.get(path) {
                if let Some(unit) = unit_files
                    .units
                    .iter_mut()
                    .find(|unit| entry.report.unit_id.as_deref() == Some(unit.id.as_str()))
                {
                    unit.asset_id = asset_id.clone();
                }
            }
        }
    }
    unit_files.assets = asset_id_by_path
        .iter()
        .map(|(path, (id, sample_rate_hz, frame_count))| Asset {
            id: id.clone(),
            path: path.clone(),
            sample_rate_hz: *sample_rate_hz,
            channels: 1,
            frame_count: *frame_count,
        })
        .collect();
    unit_files.validate(&options.layer_id, &options.profile)?;

    let staging = tempfile::Builder::new()
        .prefix("kamafeu_migration_")
        .tempdir_in(&parent)
        .map_err(|error| format!("cannot create migration staging directory: {error}"))?;
    fs::create_dir_all(staging.path().join("phonetics"))
        .and_then(|()| fs::create_dir_all(staging.path().join("units")))
        .map_err(|error| format!("cannot create KFV 2 staging folders: {error}"))?;
    for (old_path, (asset_path, _, _, source_path)) in &copied_assets {
        let target = staging.path().join(asset_path);
        fs::create_dir_all(target.parent().ok_or("asset output path has no parent")?)
            .map_err(|error| format!("cannot create migrated audio folder: {error}"))?;
        fs::copy(source_path, &target)
            .map_err(|error| format!("cannot copy source WAV '{}': {error}", old_path))?;
    }

    let profile_path = "phonetics/profile.json";
    let units_path = "units/layer.json";
    let profile_file = staging.path().join(profile_path);
    let units_file_path = staging.path().join(units_path);
    write_json(&profile_file, &options.profile)?;
    write_json(&units_file_path, &unit_files)?;
    manifest.phonetic_profiles[0].path = profile_path.into();
    manifest.layers[0].units_path = units_path.into();
    if unit_files
        .units
        .iter()
        .any(|unit| unit.source.sustain.is_some())
        && !manifest
            .engine
            .required_features
            .iter()
            .any(|feature| feature == "sustain-region")
    {
        manifest
            .engine
            .required_features
            .push("sustain-region".into());
    }
    manifest.validate()?;
    write_json(&staging.path().join("kamafeu_voicebank.json"), &manifest)?;
    let checked = NativeBank::load_dir(staging.path())?;
    let content_sha256 = checked.content_sha256()?;
    let entries = prepared
        .into_iter()
        .map(|entry| entry.report)
        .collect::<Vec<_>>();
    let skipped_entries = entries
        .iter()
        .filter(|entry| entry.status == MigrationEntryStatus::Skipped)
        .count();
    let copied_assets = asset_id_by_path.len();
    let report = MigrationReport {
        source_name: source.name.clone(),
        source_author: source.author.clone(),
        destination: destination.clone(),
        content_sha256,
        migrated_units: unit_files.units.len(),
        skipped_entries,
        copied_assets,
        entries,
        warnings,
    };
    // Keep the per-entry inference/loss evidence beside the authoring folder.
    // It is intentionally not part of the manifest content hash or distributable allowlist.
    write_json(&staging.path().join("migration_report.json"), &report)?;
    if !options.phone_mappings.is_empty() {
        write_json(
            &staging.path().join("migration_phone_mappings.json"),
            &options.phone_mappings,
        )?;
    }
    fs::rename(staging.path(), &destination).map_err(|error| {
        format!(
            "cannot atomically move KFV 2 migration copy into place at {}: {error}",
            destination.display()
        )
    })?;
    Ok(report)
}

fn migration_manifest(source: &Voicebank, options: &LegacyMigrationOptions) -> Manifest {
    Manifest {
        format: "kamafeu.voicebank".into(),
        schema_version: "2.0.0".into(),
        voice_id: options.voice_id.clone(),
        bank_id: options.bank_id.clone(),
        bank_version: options.bank_version.clone(),
        package_revision: 1,
        display_name: options.display_name.clone(),
        publisher: options.publisher.clone(),
        description: format!(
            "Assisted migration draft from classic voicebank '{}' by '{}'. Review every converted unit and define a license before distribution.",
            source.name, source.author
        ),
        languages: vec![options.profile.language.clone()],
        phonetic_profiles: vec![crate::voicebank::native::PhoneticProfileRef {
            id: options.profile.id.clone(),
            language: options.profile.language.clone(),
            path: "phonetics/profile.json".into(),
        }],
        engine: EngineRequirement {
            id: "kamafeu.concat".into(),
            api_major: 1,
            required_features: vec!["explicit-anchor".into(), "finite-units".into()],
        },
        styles: vec![Style {
            id: options.style_id.clone(),
            display_name: options.style_display_name.clone(),
        }],
        layers: vec![Layer {
            id: options.layer_id.clone(),
            style_id: options.style_id.clone(),
            recorded_midi: options.recorded_midi,
            units_path: "units/layer.json".into(),
        }],
        defaults: Defaults {
            phonetic_profile_id: options.profile.id.clone(),
            style_id: options.style_id.clone(),
            layer_policy: "nearest-recorded".into(),
            missing_unit_policy: "error".into(),
        },
        presentation: None,
        license: None,
    }
}

fn read_legacy_oto_entries(
    source_root: &Path,
    source: &Voicebank,
) -> Result<(Vec<OtoEntry>, Vec<String>), String> {
    fn scan(
        root: &Path,
        directory: &Path,
        entries: &mut Vec<OtoEntry>,
        warnings: &mut Vec<String>,
        found_oto: &mut bool,
    ) -> Result<(), String> {
        let mut children = fs::read_dir(directory)
            .map_err(|error| {
                format!(
                    "cannot scan classic bank '{}': {error}",
                    directory.display()
                )
            })?
            .map(|entry| entry.map_err(|error| error.to_string()))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| {
                format!(
                    "cannot list classic bank '{}': {error}",
                    directory.display()
                )
            })?;
        children.sort_by_key(|entry| entry.file_name());
        for child in children {
            let path = child.path();
            let metadata = fs::symlink_metadata(&path).map_err(|error| {
                format!(
                    "cannot inspect classic bank entry '{}': {error}",
                    path.display()
                )
            })?;
            if metadata.file_type().is_symlink() {
                warnings.push(format!(
                    "Skipped symlink in classic source tree: {}",
                    path.strip_prefix(root).unwrap_or(&path).display()
                ));
                continue;
            }
            if metadata.is_dir() {
                scan(root, &path, entries, warnings, found_oto)?;
                continue;
            }
            if !metadata.is_file()
                || !child
                    .file_name()
                    .to_str()
                    .is_some_and(|name| name.eq_ignore_ascii_case("oto.ini"))
            {
                continue;
            }
            *found_oto = true;
            let relative_oto = path.strip_prefix(root).unwrap_or(&path);
            let bytes = fs::read(&path)
                .map_err(|error| format!("cannot read '{}': {error}", relative_oto.display()))?;
            let text = match std::str::from_utf8(&bytes) {
                Ok(text) => text.to_owned(),
                Err(_) => encoding_rs::SHIFT_JIS.decode(&bytes).0.into_owned(),
            };
            let relative_directory = relative_oto.parent().unwrap_or_else(|| Path::new(""));
            for (line_number, line) in text.trim_start_matches('\u{feff}').lines().enumerate() {
                if line.trim().is_empty()
                    || line.trim_start().starts_with(';')
                    || line.trim_start().starts_with('#')
                {
                    continue;
                }
                match parse_migration_oto_line(line) {
                    Ok(mut entry) => {
                        if !relative_directory.as_os_str().is_empty() {
                            entry.wav_filename = relative_directory
                                .join(&entry.wav_filename)
                                .to_string_lossy()
                                .into_owned();
                        }
                        entries.push(entry);
                    }
                    Err(reason) => warnings.push(format!(
                        "{}:{}: {reason}",
                        relative_oto.display(),
                        line_number + 1
                    )),
                }
            }
        }
        Ok(())
    }

    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    let mut found_oto = false;
    scan(
        source_root,
        source_root,
        &mut entries,
        &mut warnings,
        &mut found_oto,
    )?;
    if !found_oto {
        entries.extend(source.entries.values().cloned());
        entries.sort_by(|left, right| {
            left.alias
                .cmp(&right.alias)
                .then_with(|| left.wav_filename.cmp(&right.wav_filename))
        });
        warnings.push(
            "No oto.ini was found; migration used the classic loader's indexed entries, which may already collapse duplicate aliases.".into(),
        );
    }
    Ok((entries, warnings))
}

/// Migration only uses the first three timing marks. It still validates all
/// five numeric columns against classic oto.ini rules so malformed or absent
/// values are reported consistently; present blank columns parse as zero. The
/// original file remains untouched and the report identifies timing losses.
fn parse_migration_oto_line(line: &str) -> Result<OtoEntry, String> {
    crate::oto::OtoParser::parse_str(line)?
        .into_values()
        .next()
        .ok_or_else(|| "empty line in oto.ini".to_string())
}

fn milliseconds_to_frames(milliseconds: f64, sample_rate_hz: u32) -> Option<u64> {
    if !milliseconds.is_finite() || milliseconds < 0.0 || sample_rate_hz == 0 {
        return None;
    }
    let frames = milliseconds * f64::from(sample_rate_hz) / 1_000.0;
    // Rendering from oto.ini converts millisecond marks to sample indices by truncation.
    // Keep that rule for converted boundaries to avoid moving a mark by a frame.
    frames.is_finite().then(|| frames as u64)
}

fn oto_start_frame(offset_ms: f64, sample_rate_hz: u32, frame_count: u64) -> Option<u64> {
    if !offset_ms.is_finite() {
        return None;
    }
    milliseconds_to_frames(offset_ms.max(0.0), sample_rate_hz).map(|frame| frame.min(frame_count))
}

fn oto_end_frame(
    entry: &OtoEntry,
    start_frame: u64,
    frame_count: u64,
    sample_rate_hz: u32,
) -> Option<u64> {
    if !entry.cutoff.is_finite() {
        return None;
    }
    if entry.cutoff > 0.0 {
        let tail = milliseconds_to_frames(entry.cutoff, sample_rate_hz)?;
        Some(
            frame_count
                .saturating_sub(tail)
                .max(start_frame)
                .min(frame_count),
        )
    } else if entry.cutoff < 0.0 {
        let source_length = milliseconds_to_frames(-entry.cutoff, sample_rate_hz)?;
        Some(start_frame.saturating_add(source_length).min(frame_count))
    } else {
        Some(frame_count)
    }
}

fn write_json(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(value)
        .map_err(|error| format!("cannot serialize KFV 2 migration file: {error}"))?;
    fs::write(path, bytes).map_err(|error| format!("cannot write {}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migration_parser_matches_classic_oto_empty_and_invalid_field_rules() {
        let entry = parse_migration_oto_line("take.wav=ka,10,50,-100,20,").unwrap();
        assert_eq!(entry.alias, "ka");
        assert_eq!(entry.offset, 10.0);
        assert_eq!(entry.preutterance, 20.0);
        assert_eq!(entry.overlap, 0.0);

        let entry = parse_migration_oto_line("take.wav=ka,10,50,-100,,").unwrap();
        assert_eq!(entry.preutterance, 0.0);
        assert_eq!(entry.overlap, 0.0);
        assert!(parse_migration_oto_line("take.wav=ka,10,50,-100,not-used,0").is_err());
        assert!(parse_migration_oto_line("take.wav=ka,not-a-number,50,-100,20,").is_err());
        assert!(parse_migration_oto_line("take.wav=ka,10,50,-100,20").is_err());
    }
    use crate::oto::PrefixMap;
    use crate::voicebank::native::{PhoneSymbol, PhoneticProfile};
    use std::sync::OnceLock;

    fn options() -> LegacyMigrationOptions {
        LegacyMigrationOptions {
            voice_id: "org.example.voice".into(),
            bank_id: "org.example.voice.native".into(),
            bank_version: "1.0.0".into(),
            display_name: "Example Native Draft".into(),
            publisher: "Example Author".into(),
            profile: PhoneticProfile {
                format: "kamafeu.phonetics".into(),
                schema_version: "1.0.0".into(),
                id: "test-phones".into(),
                language: "pt-BR".into(),
                symbols: vec![
                    PhoneSymbol {
                        id: "k".into(),
                        ipa: "k".into(),
                        class: "stop".into(),
                        voiced: false,
                    },
                    PhoneSymbol {
                        id: "a".into(),
                        ipa: "a".into(),
                        class: "vowel".into(),
                        voiced: true,
                    },
                ],
                lexicon: BTreeMap::new(),
                unknown_word_policy: "error".into(),
            },
            style_id: "natural".into(),
            style_display_name: "Natural".into(),
            layer_id: "natural-c4".into(),
            recorded_midi: 60,
            phone_mappings: BTreeMap::from([("ka".into(), vec!["k".into(), "a".into()])]),
        }
    }

    fn source_bank(root: &Path) -> Voicebank {
        let mut entries = HashMap::new();
        entries.insert(
            "ka".into(),
            OtoEntry {
                wav_filename: "ka.wav".into(),
                alias: "ka".into(),
                offset: 10.0,
                consonant: 20.0,
                cutoff: 100.0,
                preutterance: 35.0,
                overlap: 12.0,
                loop_start: Some(50.0),
                loop_end: Some(80.0),
                tail_start: Some(90.0),
            },
        );
        entries.insert(
            "k a".into(),
            OtoEntry {
                alias: "k a".into(),
                ..entries["ka"].clone()
            },
        );
        entries.insert(
            "ka missing".into(),
            OtoEntry {
                wav_filename: "missing.wav".into(),
                alias: "ka missing".into(),
                ..entries["ka"].clone()
            },
        );
        Voicebank {
            root_path: root.to_path_buf(),
            name: "Legacy Test Voice".into(),
            author: "Original Author".into(),
            character_info: String::new(),
            readme_info: String::new(),
            image_path: None,
            entries,
            case_insensitive_entries: OnceLock::new(),
            prefix_map: PrefixMap::default(),
            temp_dir: None,
        }
    }

    #[test]
    fn migration_preview_reports_mapping_coverage_without_writing_files() {
        let temp = tempfile::tempdir().unwrap();
        let source = source_bank(temp.path());
        let mut options = options();
        options.phone_mappings.insert("ka".into(), vec!["missing".into()]);

        let preview = preview_legacy_voicebank_migration(
            &source,
            &options.profile,
            &options.phone_mappings,
        )
        .unwrap();

        assert_eq!(preview.total_entries, 3);
        assert_eq!(preview.explicit_mappings, 1);
        assert_eq!(preview.invalid_mappings, 1);
        assert_eq!(preview.inferred_aliases, 1);
        assert_eq!(preview.unmapped_aliases, 1);
        assert_eq!(preview.potential_units, 1);
        assert_eq!(preview.alias_collisions, 0);
        assert!(std::fs::read_dir(temp.path()).unwrap().next().is_none());
    }

    fn write_source_wav(path: &Path) {
        let mut writer = hound::WavWriter::create(
            path,
            hound::WavSpec {
                channels: 1,
                sample_rate: 48_000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        for index in 0..48_000 {
            let sample = ((f64::from(index) * std::f64::consts::TAU * 220.0 / 48_000.0).sin()
                * 12_000.0) as i16;
            writer.write_sample(sample).unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn oto_boundary_conversion_matches_classic_sample_index_truncation() {
        let sample_count = 101_u64;
        let sample_rate = 44_100_u32;
        for (offset_ms, cutoff_ms) in [
            (0.05, 0.05),
            (0.05, -0.05),
            (0.05, 0.0),
            (-0.05, 0.05),
            (0.0, -100.0),
        ] {
            let (classic_start, classic_end) = crate::dsp::oto_source_bounds(
                sample_count as usize,
                sample_rate,
                offset_ms,
                cutoff_ms,
            );
            let migrated_start = oto_start_frame(offset_ms, sample_rate, sample_count).unwrap();
            let migrated_end = oto_end_frame(
                &OtoEntry {
                    wav_filename: "test.wav".into(),
                    alias: "test".into(),
                    offset: offset_ms,
                    consonant: 0.0,
                    cutoff: cutoff_ms,
                    preutterance: 0.0,
                    overlap: 0.0,
                    loop_start: None,
                    loop_end: None,
                    tail_start: None,
                },
                migrated_start,
                sample_count,
                sample_rate,
            )
            .unwrap();
            assert_eq!(
                (migrated_start as usize, migrated_end as usize),
                (classic_start, classic_end)
            );
        }
    }

    #[test]
    fn migration_creates_a_separate_draft_with_per_alias_loss_report() {
        let root = tempfile::tempdir().unwrap();
        let source_root = root.path().join("legacy");
        fs::create_dir_all(&source_root).unwrap();
        let source_wav = source_root.join("ka.wav");
        write_source_wav(&source_wav);
        let source_bytes = fs::read(&source_wav).unwrap();
        let source = source_bank(&source_root);
        let destination = root.path().join("native-copy");

        let report = migrate_legacy_voicebank_to_copy(&source, &destination, &options()).unwrap();
        assert_eq!(report.migrated_units, 2);
        assert_eq!(report.skipped_entries, 1);
        assert_eq!(report.copied_assets, 1);
        let saved_report: MigrationReport =
            serde_json::from_slice(&fs::read(destination.join("migration_report.json")).unwrap())
                .unwrap();
        assert_eq!(saved_report, report);
        assert!(report.entries.iter().any(|entry| {
            entry.alias == "k a" && entry.mapping_origin == MappingOrigin::ExactAliasTokens
        }));
        assert!(report.entries.iter().any(|entry| {
            entry.alias == "ka missing" && entry.status == MigrationEntryStatus::Skipped
        }));
        assert_eq!(fs::read(&source_wav).unwrap(), source_bytes);

        let bank = NativeBank::load_dir(&destination).unwrap();
        assert_eq!(bank.layers[0].1.units.len(), 2);
        let unit = bank.layers[0]
            .1
            .units
            .iter()
            .find(|unit| unit.legacy_aliases == vec!["ka".to_string()])
            .unwrap();
        assert_eq!(unit.source.start_frame, 480);
        assert_eq!(unit.source.anchor_frame, 1_440);
        assert_eq!(unit.source.end_frame, 43_200);
        assert_eq!(unit.source.sustain.as_ref().unwrap().start_frame, 2_400);
        assert_eq!(unit.source.release_start_frame, Some(4_320));
        assert!(!unit.quality.reviewed);

        let existing = fs::read(destination.join("kamafeu_voicebank.json")).unwrap();
        assert!(
            migrate_legacy_voicebank_to_copy(&source, &destination, &options())
                .unwrap_err()
                .contains("already exists")
        );
        assert_eq!(
            fs::read(destination.join("kamafeu_voicebank.json")).unwrap(),
            existing
        );
        assert_eq!(fs::read(&source_wav).unwrap(), source_bytes);
    }

    #[test]
    fn migration_rereads_root_and_subbank_otos_and_preserves_alias_collisions() {
        let root = tempfile::tempdir().unwrap();
        let source_root = root.path().join("legacy");
        let subbank = source_root.join("Soft");
        fs::create_dir_all(&subbank).unwrap();
        for path in [
            source_root.join("take-a.wav"),
            source_root.join("take-b.wav"),
            subbank.join("take-c.wav"),
        ] {
            write_source_wav(&path);
        }
        fs::write(
            source_root.join("oto.ini"),
            "take-a.wav=k a,10,20,100,35,12\ntake-b.wav=k a,10,20,100,35,12\n",
        )
        .unwrap();
        fs::write(subbank.join("oto.ini"), "take-c.wav=k a,10,20,100,35,12\n").unwrap();
        let source = Voicebank::new(&source_root).unwrap();
        let destination = root.path().join("native-copy");

        let report = migrate_legacy_voicebank_to_copy(&source, &destination, &options()).unwrap();
        assert_eq!(report.migrated_units, 3);
        assert_eq!(report.copied_assets, 3);
        assert!(report.entries.iter().all(|entry| {
            entry.alias == "k a"
                && entry.status == MigrationEntryStatus::Migrated
                && entry
                    .warnings
                    .iter()
                    .any(|warning| warning.contains("Alias collision"))
        }));
        let bank = NativeBank::load_dir(&destination).unwrap();
        assert_eq!(bank.layers[0].1.units.len(), 3);
        assert!(bank.layers[0]
            .1
            .assets
            .iter()
            .any(|asset| asset.path.contains("asset-000003")));
    }

    #[test]
    fn unsupported_single_consonant_mapping_is_reported_without_panicking() {
        let root = tempfile::tempdir().unwrap();
        let source_root = root.path().join("legacy");
        fs::create_dir_all(&source_root).unwrap();
        write_source_wav(&source_root.join("take.wav"));
        fs::write(
            source_root.join("oto.ini"),
            "take.wav=k a,0,20,0,0,0\ntake.wav=k-alone,0,20,0,0,0\n",
        )
        .unwrap();
        let source = Voicebank::new(&source_root).unwrap();
        let mut options = options();
        options
            .phone_mappings
            .insert("k-alone".into(), vec!["k".into()]);
        let report =
            migrate_legacy_voicebank_to_copy(&source, &root.path().join("native-copy"), &options)
                .unwrap();
        assert_eq!(report.migrated_units, 1);
        assert_eq!(report.skipped_entries, 1);
        let consonant = report
            .entries
            .iter()
            .find(|entry| entry.alias == "k-alone")
            .unwrap();
        assert!(consonant
            .warnings
            .iter()
            .any(|warning| warning.contains("standalone consonant")));
    }
}
