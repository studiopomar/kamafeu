//! KFV 2 package inspection and atomic installation primitives.
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use super::native::{Manifest, NativeBank};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kfv2PackageSummary {
    pub bank_id: String,
    pub bank_version: String,
    pub display_name: String,
    pub publisher: String,
    pub languages: Vec<String>,
    pub styles: Vec<String>,
    pub layer_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kfv2InstallReport {
    pub installed_path: PathBuf,
    pub bank_id: String,
    pub bank_version: String,
    pub content_sha256: String,
}

/// Read only the root manifest from a `.kfv` ZIP. Audio data is not inflated
/// or decoded, so this is suitable for a metadata catalog but does not certify
/// the package's referenced files; installation performs full validation.
pub fn inspect_package_metadata(path: &Path) -> Result<Kfv2PackageSummary, String> {
    if !path
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("kfv"))
    {
        return Err("KFV 2 package must use the .kfv extension".into());
    }
    let file = fs::File::open(path).map_err(|error| format!("cannot open KFV package: {error}"))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|error| format!("invalid KFV ZIP container: {error}"))?;
    if archive.len() > 10_000 {
        return Err("KFV package contains too many entries".into());
    }
    let mut seen = HashSet::new();
    let mut total_size = 0u64;
    let mut manifest_count = 0usize;
    for index in 0..archive.len() {
        let entry = archive
            .by_index(index)
            .map_err(|error| format!("invalid KFV directory entry: {error}"))?;
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            return Err("KFV packages cannot contain symbolic links".into());
        }
        let name = entry.name();
        if name.contains('\\') || name.contains('\0') {
            return Err("KFV package contains an unsafe entry path".into());
        }
        let enclosed = entry
            .enclosed_name()
            .ok_or_else(|| format!("KFV package contains an unsafe path: {name}"))?;
        let normalized = enclosed.to_string_lossy().into_owned();
        if !seen.insert(normalized) {
            return Err(format!("KFV package contains a duplicate path: {name}"));
        }
        total_size = total_size
            .checked_add(entry.size())
            .ok_or_else(|| "KFV declared size overflow".to_string())?;
        if entry.size() > 512 * 1024 * 1024 || total_size > 4 * 1024 * 1024 * 1024 {
            return Err("KFV package exceeds the declared size limits".into());
        }
        if name == "kamafeu_voicebank.json" {
            manifest_count += 1;
        }
    }
    if manifest_count != 1 {
        return Err("KFV package must contain exactly one root manifest".into());
    }
    let mut manifest_entry = archive
        .by_name("kamafeu_voicebank.json")
        .map_err(|_| "KFV package is missing kamafeu_voicebank.json".to_string())?;
    if manifest_entry.size() > 2 * 1024 * 1024 {
        return Err("KFV manifest exceeds the 2 MiB metadata limit".into());
    }
    let mut bytes = Vec::with_capacity(manifest_entry.size() as usize);
    (&mut manifest_entry)
        .take(2 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read KFV manifest: {error}"))?;
    if bytes.len() > 2 * 1024 * 1024 {
        return Err("KFV manifest exceeds the 2 MiB metadata limit".into());
    }
    let text = std::str::from_utf8(&bytes)
        .map_err(|error| format!("KFV manifest is not UTF-8: {error}"))?;
    let manifest = Manifest::parse_json(text)?;
    Ok(summary(&manifest))
}

/// Install a validated KFV 2 package into a new directory. Extraction and
/// validation happen in a sibling staging directory; the final rename is
/// atomic on the destination filesystem. Existing destinations are rejected.
pub fn install_package_atomic(
    package: &Path,
    destination: &Path,
) -> Result<Kfv2InstallReport, String> {
    if !package
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("kfv"))
    {
        return Err("KFV 2 package must use the .kfv extension".into());
    }
    let package = fs::canonicalize(package)
        .map_err(|error| format!("cannot resolve KFV package: {error}"))?;
    if !package.is_file() {
        return Err("KFV package path is not a regular file".into());
    }
    let parent = destination
        .parent()
        .ok_or_else(|| "installation destination has no parent directory".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("cannot create installation parent: {error}"))?;
    let parent = fs::canonicalize(parent)
        .map_err(|error| format!("cannot resolve installation parent: {error}"))?;
    let name = destination
        .file_name()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| "installation destination must name a directory".to_string())?;
    let destination = parent.join(name);
    if fs::symlink_metadata(&destination).is_ok() {
        return Err(format!(
            "installation destination already exists: {}",
            destination.display()
        ));
    }

    let staging = tempfile::Builder::new()
        .prefix(".kfv2-install-")
        .tempdir_in(&parent)
        .map_err(|error| format!("cannot create KFV staging directory: {error}"))?;
    crate::oto::voicebank::extract_kfv(&package, staging.path())
        .map_err(|error| format!("KFV extraction rejected the package: {error}"))?;
    let bank = NativeBank::load_dir(staging.path())?;
    let content_sha256 = bank.content_sha256()?;
    let report = Kfv2InstallReport {
        installed_path: destination.clone(),
        bank_id: bank.manifest.bank_id.clone(),
        bank_version: bank.manifest.bank_version.clone(),
        content_sha256,
    };
    fs::rename(staging.path(), &destination)
        .map_err(|error| format!("cannot atomically install KFV package: {error}"))?;
    // The staging path has been moved. Disarm TempDir cleanup for portability.
    let _ = staging.keep();
    Ok(report)
}

fn summary(manifest: &Manifest) -> Kfv2PackageSummary {
    Kfv2PackageSummary {
        bank_id: manifest.bank_id.clone(),
        bank_version: manifest.bank_version.clone(),
        display_name: manifest.display_name.clone(),
        publisher: manifest.publisher.clone(),
        languages: manifest.languages.clone(),
        styles: manifest
            .styles
            .iter()
            .map(|style| style.display_name.clone())
            .collect(),
        layer_count: manifest.layers.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn failed_install_leaves_no_partial_destination() {
        let temp = tempfile::tempdir().unwrap();
        let package = temp.path().join("invalid.kfv");
        let mut zip = zip::ZipWriter::new(fs::File::create(&package).unwrap());
        zip.start_file("kamafeu_voicebank.json", zip::write::FileOptions::default())
            .unwrap();
        zip.write_all(br#"{"format":"kamafeu.voicebank","schema_version":"2.0.0"}"#)
            .unwrap();
        zip.finish().unwrap();
        let destination = temp.path().join("installed");

        assert!(install_package_atomic(&package, &destination).is_err());
        assert!(!destination.exists());
        assert_eq!(
            fs::read_dir(temp.path())
                .unwrap()
                .filter_map(Result::ok)
                .filter(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".kfv2-install-"))
                .count(),
            0
        );
    }
}
