use crate::oto::{OtoEntry, Voicebank};
use std::fs;
use std::path::Path;

pub(super) fn read_legacy_oto_entries(
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
pub(super) fn parse_migration_oto_line(line: &str) -> Result<OtoEntry, String> {
    crate::oto::OtoParser::parse_str(line)?
        .into_values()
        .next()
        .ok_or_else(|| "empty line in oto.ini".to_string())
}

pub(super) fn milliseconds_to_frames(milliseconds: f64, sample_rate_hz: u32) -> Option<u64> {
    if !milliseconds.is_finite() || milliseconds < 0.0 || sample_rate_hz == 0 {
        return None;
    }
    let frames = milliseconds * f64::from(sample_rate_hz) / 1_000.0;
    // Rendering from oto.ini converts millisecond marks to sample indices by truncation.
    // Keep that rule for converted boundaries to avoid moving a mark by a frame.
    frames.is_finite().then_some(frames as u64)
}

pub(super) fn oto_start_frame(
    offset_ms: f64,
    sample_rate_hz: u32,
    frame_count: u64,
) -> Option<u64> {
    if !offset_ms.is_finite() {
        return None;
    }
    milliseconds_to_frames(offset_ms.max(0.0), sample_rate_hz).map(|frame| frame.min(frame_count))
}

pub(super) fn oto_end_frame(
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
