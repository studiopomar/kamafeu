use encoding_rs::SHIFT_JIS;
use std::collections::{HashMap, HashSet};
#[cfg(test)]
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

mod archive;
mod diagnostics;
mod loader;
mod lookup;
mod scanner;
pub(crate) use archive::extract_kfv;
#[cfg(test)]
use diagnostics::format_line_ranges;

use super::entry::OtoEntry;
use super::prefix_map::PrefixMap;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct AliasResolutionReport {
    pub requested_alias: String,
    pub pitch: String,
    pub resolved: bool,
    pub resolved_alias: Option<String>,
    pub wav_filename: Option<String>,
    pub offset_ms: Option<f64>,
    pub consonant_ms: Option<f64>,
    pub preutterance_ms: Option<f64>,
    pub overlap_ms: Option<f64>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Voicebank {
    pub root_path: PathBuf,
    pub name: String,
    pub author: String,
    pub character_info: String,
    pub readme_info: String,
    pub image_path: Option<PathBuf>,
    pub entries: HashMap<String, OtoEntry>,
    pub case_insensitive_entries: OnceLock<HashMap<String, String>>,
    pub prefix_map: PrefixMap,
    pub temp_dir: Option<Arc<tempfile::TempDir>>,
}

impl Voicebank {
    pub fn get_subfolders(&self) -> Vec<String> {
        let mut subfolders = HashSet::new();
        subfolders.insert("All Folders".to_string());
        subfolders.insert("Root".to_string());

        for entry in self.entries.values() {
            let path = Path::new(&entry.wav_filename);
            if let Some(parent) = path.parent() {
                let p_str = parent.to_string_lossy().to_string();
                if !p_str.is_empty() {
                    subfolders.insert(p_str);
                }
            }
        }

        let mut list: Vec<String> = subfolders.into_iter().collect();
        list.sort();
        list
    }

    pub fn search_entries<'a>(
        &'a self,
        search_query: &str,
        folder_filter: &str,
    ) -> Vec<(&'a String, &'a OtoEntry)> {
        let q = search_query.trim().to_lowercase();

        self.entries
            .iter()
            .filter(|(alias, entry)| {
                let folder_matches = if folder_filter == "All Folders" {
                    true
                } else if folder_filter == "Root" {
                    !entry.wav_filename.contains('/') && !entry.wav_filename.contains('\\')
                } else {
                    entry.wav_filename.starts_with(folder_filter)
                };

                if !folder_matches {
                    return false;
                }

                if q.is_empty() {
                    true
                } else {
                    alias.to_lowercase().contains(&q)
                        || entry.wav_filename.to_lowercase().contains(&q)
                }
            })
            .collect()
    }
}

fn is_safe_relative_path(path: &Path) -> bool {
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path.components().all(|component| {
            matches!(
                component,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        })
}

fn decode_text_autodetect(bytes: &[u8]) -> String {
    let clean = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &bytes[3..]
    } else {
        bytes
    };
    if let Ok(utf8) = std::str::from_utf8(clean) {
        utf8.to_string()
    } else {
        let (decoded, _, _) = SHIFT_JIS.decode(clean);
        decoded.into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn kfv_extractor_rejects_traversal_duplicate_and_symlink_entries() {
        for entries in [
            vec![("../outside.txt", 0o100644), ("ok.txt", 0o100644)],
            vec![("same.txt", 0o100644), ("same.txt", 0o100644)],
        ] {
            let case = entries
                .iter()
                .map(|(name, _)| *name)
                .collect::<Vec<_>>()
                .join(",");
            let workspace = tempdir().unwrap();
            let package = workspace.path().join("bad.kfv");
            let mut zip = zip::ZipWriter::new(fs::File::create(&package).unwrap());
            for (name, mode) in entries {
                let options = zip::write::FileOptions::default().unix_permissions(mode);
                zip.start_file(name, options).unwrap();
                zip.write_all(b"payload").unwrap();
            }
            zip.finish().unwrap();
            let destination = workspace.path().join("extracted");
            fs::create_dir(&destination).unwrap();
            let error = match extract_kfv(&package, &destination) {
                Err(error) => error,
                Ok(()) => panic!("unexpectedly accepted malicious archive case: {case}"),
            };
            assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
            assert!(!workspace.path().join("outside.txt").exists());
        }

        // zip 0.6's writer masks Unix file-type bits, so mark one generated
        // central-directory entry as a symlink to exercise the real archive
        // metadata the reader consumes.
        let workspace = tempdir().unwrap();
        let package = workspace.path().join("symlink.kfv");
        let mut zip = zip::ZipWriter::new(fs::File::create(&package).unwrap());
        zip.start_file("link", zip::write::FileOptions::default())
            .unwrap();
        zip.write_all(b"target").unwrap();
        zip.finish().unwrap();
        let mut bytes = fs::read(&package).unwrap();
        let central = bytes.windows(4).position(|v| v == b"PK\x01\x02").unwrap();
        bytes[central + 5] = 3; // Unix creator OS.
        bytes[central + 38..central + 42].copy_from_slice(&(0o120777u32 << 16).to_le_bytes());
        fs::write(&package, bytes).unwrap();
        let destination = workspace.path().join("extracted");
        fs::create_dir(&destination).unwrap();
        let error = extract_kfv(&package, &destination).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
    }

    #[test]
    fn kfv2_discriminator_never_falls_back_to_root_oto() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("kamafeu_voicebank.json"),
            r#"{"format":"kamafeu.voicebank","schema_version":"2.0.0"}"#,
        )
        .unwrap();
        fs::write(
            dir.path().join("oto.ini"),
            "sample.wav=ka,0,100,-400,80,30\n",
        )
        .unwrap();
        let error = Voicebank::new(dir.path()).unwrap_err();
        assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
        assert!(error.to_string().contains("manifest fields"));
    }

    #[test]
    fn formats_diagnostic_lines_as_compact_ranges() {
        assert_eq!(
            format_line_ranges(&[19, 20, 21, 23, 24, 31]),
            "19-21, 23-24, 31"
        );
    }

    #[test]
    fn test_native_voicebank_loading() {
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("kamafeu_voicebank.json");

        let json_content = r#"{
            "version": "1.0",
            "voicebank_name": "Test Native Voicebank",
            "author": "Antigravity",
            "image_filename": "avatar.png",
            "entries": [
                {
                    "wav_filename": "ka.wav",
                    "alias": "ka",
                    "corte_inicial_ms": 10.0,
                    "consoante_ms": 50.0,
                    "corte_final_ms": -100.0,
                    "preutterance_ms": 30.0,
                    "overlap_ms": 15.0

                }
            ],
            "prefix_map": {
                "C4": ["", "_C4"]
            }
        }"#;
        fs::write(&config_path, json_content).unwrap();

        let vb = Voicebank::new(dir.path()).unwrap();
        assert_eq!(vb.name, "Test Native Voicebank");
        assert_eq!(vb.author, "Antigravity");
        assert_eq!(vb.entries.len(), 1);

        let entry = vb.entries.get("ka").unwrap();
        assert_eq!(entry.wav_filename, "ka.wav");
        assert_eq!(entry.offset, 10.0);
        assert_eq!(entry.consonant, 50.0);
        assert_eq!(entry.cutoff, -100.0);
        assert_eq!(entry.preutterance, 30.0);
        assert_eq!(entry.overlap, 15.0);

        let (pref, suff) = vb.prefix_map.get_prefix_suffix("C4").unwrap();
        assert_eq!(pref, "");
        assert_eq!(suff, "_C4");
    }

    #[test]
    fn oto_ini_wins_over_a_stale_native_voicebank_index() {
        let dir = tempdir().unwrap();
        fs::write(
            dir.path().join("kamafeu_voicebank.json"),
            r#"{
                "version": "1.0",
                "voicebank_name": "Indexed",
                "entries": [{
                    "wav_filename": "old.wav",
                    "alias": "old",
                    "corte_inicial_ms": 0.0,
                    "consoante_ms": 50.0,
                    "corte_final_ms": -100.0,
                    "preutterance_ms": 20.0,
                    "overlap_ms": 10.0
                }]
            }"#,
        )
        .unwrap();
        fs::write(dir.path().join("oto.ini"), "ka.wav=k a,0,100,-400,80,30\n").unwrap();

        let vb = Voicebank::new(dir.path()).unwrap();
        assert!(vb.entries.contains_key("k a"));
        assert!(!vb.entries.contains_key("old"));
    }

    #[test]
    fn root_oto_wins_over_duplicate_aliases_in_subbanks() {
        let dir = tempdir().unwrap();
        let subbank = dir.path().join("A3");
        fs::create_dir(&subbank).unwrap();
        fs::write(dir.path().join("oto.ini"), "root.wav=ka,0,100,-400,80,30\n").unwrap();
        fs::write(
            subbank.join("oto.ini"),
            "sub.wav=ka,0,200,-500,160,60\nsub.wav=ka_A3,0,200,-500,160,60\n",
        )
        .unwrap();

        let vb = Voicebank::new(dir.path()).unwrap();
        assert_eq!(vb.entries["ka"].wav_filename, "root.wav");
        assert_eq!(vb.entries["ka"].preutterance, 80.0);
        assert_eq!(vb.entries["ka_A3"].wav_filename, "A3/sub.wav");
    }

    #[test]
    fn reads_openutau_character_yaml_metadata_and_portrait() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("oto.ini"), "ka.wav=ka,0,100,-400,80,30\n").unwrap();
        fs::write(dir.path().join("portrait.png"), b"not-an-image").unwrap();
        fs::write(
            dir.path().join("character.yaml"),
            "name: Test Singer\nauthor: Test Author\nportrait: portrait.png\nvoice: Test Voice\nversion: 1.5\nsinger_type: utau\nweb: https://example.test\n",
        )
        .unwrap();

        let vb = Voicebank::new(dir.path()).unwrap();
        assert_eq!(vb.name, "Test Singer");
        assert_eq!(vb.author, "Test Author");
        assert_eq!(
            vb.image_path.as_deref(),
            Some(dir.path().join("portrait.png").as_path())
        );
        assert!(vb.character_info.contains("voice: Test Voice"));
        assert!(vb.character_info.contains("version: 1.5"));
        assert!(vb.character_info.contains("web: https://example.test"));
    }

    #[test]
    fn finds_conventional_png_portrait_without_metadata() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("oto.ini"), "ka.wav=ka,0,100,-400,80,30\n").unwrap();
        fs::write(dir.path().join("image.png"), b"not-an-image").unwrap();

        let vb = Voicebank::new(dir.path()).unwrap();

        assert_eq!(
            vb.image_path.as_deref(),
            Some(dir.path().join("image.png").as_path())
        );
    }

    #[test]
    fn oto_bank_with_auxiliary_dsvocoder_stays_on_classic_renderer() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("oto.ini"), "ka.wav=k a,0,100,-400,80,30\n").unwrap();
        fs::create_dir(dir.path().join("dsvocoder")).unwrap();

        let vb = Voicebank::new(dir.path()).unwrap();
        assert_eq!(vb.find_mapped_entry("k a", "C4").unwrap().alias, "k a");
        let report = vb.alias_resolution_report("k a", "C4");
        assert!(report.resolved);
        assert_eq!(report.resolved_alias.as_deref(), Some("k a"));
        assert_eq!(report.preutterance_ms, Some(80.0));
        fs::write(dir.path().join("ka.wav"), b"first sample").unwrap();
        let first_fingerprint = vb.content_fingerprint();
        fs::write(dir.path().join("ka.wav"), b"replacement sample").unwrap();
        assert_ne!(first_fingerprint, vb.content_fingerprint());
    }
}
