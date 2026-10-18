use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::oto::entry::OtoEntry;
use crate::oto::parser::OtoParser;

use super::diagnostics::format_parse_diagnostics;
use super::{is_safe_relative_path, Voicebank};

impl Voicebank {
    pub(super) fn scan_oto_files(
        root: &Path,
        current_dir: &Path,
        entries: &mut HashMap<String, OtoEntry>,
    ) -> Result<(), std::io::Error> {
        if let Ok(read_dir) = fs::read_dir(current_dir) {
            let mut directory_entries = read_dir.flatten().collect::<Vec<_>>();
            directory_entries.sort_by_key(|entry| entry.file_name());
            // Load the oto.ini owned by this directory before descending into
            // colour/pitch subbanks. `or_insert` below then gives the root
            // voice its documented precedence for duplicate, unsuffixed
            // aliases. The old depth-first walk did the opposite even though
            // its comment claimed root precedence.
            for entry in directory_entries.iter().filter(|entry| {
                entry.file_type().is_ok_and(|kind| kind.is_file())
                    && entry
                        .file_name()
                        .to_str()
                        .is_some_and(|name| name.eq_ignore_ascii_case("oto.ini"))
            }) {
                let path = entry.path();
                if entry.file_type().is_ok_and(|kind| kind.is_symlink()) {
                    continue;
                }
                let (file_entries, parse_errors) = OtoParser::parse_file_lossy_diagnostics(&path)?;
                if !parse_errors.is_empty() {
                    let relative_path = path
                        .strip_prefix(root)
                        .unwrap_or(path.as_path())
                        .display()
                        .to_string();
                    let summary = format_parse_diagnostics(&parse_errors);
                    crate::terminal_log::warn_once(
                        format!("{}:{summary}", path.display()),
                        "Voicebank",
                        format!("{}\n{}", relative_path, summary),
                    );
                }
                let rel_sub_dir = path
                    .parent()
                    .and_then(|parent| parent.strip_prefix(root).ok())
                    .unwrap_or(Path::new(""));
                for (alias, mut oto) in file_entries {
                    if !is_safe_relative_path(Path::new(&oto.wav_filename)) {
                        continue;
                    }
                    if rel_sub_dir != Path::new("") {
                        oto.wav_filename = rel_sub_dir
                            .join(&oto.wav_filename)
                            .to_string_lossy()
                            .to_string();
                    }
                    entries.entry(alias).or_insert(oto);
                }
            }
            for entry in directory_entries {
                if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                    Self::scan_oto_files(root, &entry.path(), entries)?;
                }
            }
        }
        Ok(())
    }
}
