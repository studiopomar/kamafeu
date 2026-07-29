use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use encoding_rs::SHIFT_JIS;

use super::entry::OtoEntry;
use super::parser::OtoParser;
use super::prefix_map::PrefixMap;

#[derive(Debug, Clone)]
pub struct Voicebank {
    pub root_path: PathBuf,
    pub name: String,
    pub author: String,
    pub character_info: String,
    pub readme_info: String,
    pub entries: HashMap<String, OtoEntry>,
    pub prefix_map: PrefixMap,
}

impl Voicebank {
    pub fn new<P: AsRef<Path>>(root_path: P) -> Result<Self, std::io::Error> {
        let root_path = root_path.as_ref().to_path_buf();
        let mut name = root_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown Voicebank")
            .to_string();
        let mut author = "Unknown".to_string();
        let mut character_info = String::new();
        let mut readme_info = String::new();

        // Load character.txt metadata if present
        let char_path = root_path.join("character.txt");
        if char_path.exists() {
            if let Ok(bytes) = fs::read(&char_path) {
                let (text, _, _) = SHIFT_JIS.decode(&bytes);
                character_info = text.to_string();
                for line in text.lines() {
                    let line = line.trim();
                    if line.starts_with("name=") {
                        name = line.trim_start_matches("name=").trim().to_string();
                    } else if line.starts_with("author=") {
                        author = line.trim_start_matches("author=").trim().to_string();
                    }
                }
            }
        }

        // Load readme.txt if present
        let readme_path = root_path.join("readme.txt");
        if readme_path.exists() {
            if let Ok(bytes) = fs::read(&readme_path) {
                let (text, _, _) = SHIFT_JIS.decode(&bytes);
                readme_info = text.to_string();
            }
        }

        // Load prefix.map if present
        let prefix_path = root_path.join("prefix.map");
        let prefix_map = if prefix_path.exists() {
            PrefixMap::parse_file(&prefix_path).unwrap_or_default()
        } else {
            PrefixMap::default()
        };

        // Recursively find all oto.ini files in the voicebank directory tree
        let mut entries = HashMap::new();
        Self::scan_oto_files(&root_path, &root_path, &mut entries);

        Ok(Self {
            root_path,
            name,
            author,
            character_info,
            readme_info,
            entries,
            prefix_map,
        })
    }

    fn scan_oto_files(root: &Path, current_dir: &Path, entries: &mut HashMap<String, OtoEntry>) {
        if let Ok(read_dir) = fs::read_dir(current_dir) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    Self::scan_oto_files(root, &path, entries);
                } else if path.is_file() {
                    if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                        if filename.eq_ignore_ascii_case("oto.ini") {
                            if let Ok(file_entries) = OtoParser::parse_file(&path) {
                                let rel_sub_dir = path
                                    .parent()
                                    .and_then(|p| p.strip_prefix(root).ok())
                                    .unwrap_or(Path::new(""));

                                for (alias, mut entry) in file_entries {
                                    if rel_sub_dir != Path::new("") {
                                        let full_wav = rel_sub_dir.join(&entry.wav_filename);
                                        entry.wav_filename = full_wav.to_string_lossy().to_string();
                                    }
                                    entries.insert(alias, entry);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Lookup OtoEntry by lyric and target pitch name (e.g., "ka", "C4", "- そ")
    pub fn find_entry(&self, lyric: &str, pitch_name: &str) -> Option<&OtoEntry> {
        let prefixed_alias = self.prefix_map.get_alias(lyric, pitch_name);
        if let Some(entry) = self.entries.get(&prefixed_alias).or_else(|| self.entries.get(lyric)) {
            return Some(entry);
        }

        // VCV fallback stripping (e.g. "- そ" -> "そ", "o そ" -> "そ", "a か" -> "か")
        let cleaned_lyric = lyric.trim();
        if let Some(space_idx) = cleaned_lyric.find(' ') {
            let cv_part = &cleaned_lyric[space_idx + 1..];
            let cv_prefixed = self.prefix_map.get_alias(cv_part, pitch_name);
            if let Some(entry) = self.entries.get(&cv_prefixed).or_else(|| self.entries.get(cv_part)) {
                return Some(entry);
            }
        }

        None
    }

    /// Retrieve all subfolders containing oto.ini entries
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

    /// Filter phoneme entries by search query and subfolder selection
    pub fn search_entries<'a>(&'a self, search_query: &str, folder_filter: &str) -> Vec<(&'a String, &'a OtoEntry)> {
        let q = search_query.trim().to_lowercase();

        self.entries
            .iter()
            .filter(|(alias, entry)| {
                // Folder filter
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

                // Query search filter
                if q.is_empty() {
                    true
                } else {
                    alias.to_lowercase().contains(&q) || entry.wav_filename.to_lowercase().contains(&q)
                }
            })
            .collect()
    }
}
