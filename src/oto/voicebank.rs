use encoding_rs::SHIFT_JIS;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

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
    pub image_path: Option<PathBuf>,
    pub entries: HashMap<String, OtoEntry>,
    pub prefix_map: PrefixMap,
}

impl Voicebank {
    pub fn new<P: AsRef<Path>>(root_path: P) -> Result<Self, std::io::Error> {
        let root_path = root_path.as_ref().to_path_buf();
        if !root_path.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("voicebank directory not found: {}", root_path.display()),
            ));
        }
        let mut name = root_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown Voicebank")
            .to_string();
        let mut author = "Unknown".to_string();
        let mut character_info = String::new();
        let mut readme_info = String::new();
        let mut image_relative_path: Option<String> = None;

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
                    } else if line.starts_with("image=") {
                        image_relative_path =
                            Some(line.trim_start_matches("image=").trim().to_string());
                    } else if line.starts_with("icon=") {
                        if image_relative_path.is_none() {
                            image_relative_path =
                                Some(line.trim_start_matches("icon=").trim().to_string());
                        }
                    } else if line.starts_with("portrait=") && image_relative_path.is_none() {
                        image_relative_path =
                            Some(line.trim_start_matches("portrait=").trim().to_string());
                    }
                }
            }
        }

        // Load character.yaml metadata if present
        let char_yaml_path = root_path.join("character.yaml");
        if char_yaml_path.exists() {
            if let Ok(content) = fs::read_to_string(&char_yaml_path) {
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with("name:") && (name.starts_with("Unknown") || name.is_empty())
                    {
                        name = line
                            .trim_start_matches("name:")
                            .trim()
                            .trim_matches('"')
                            .to_string();
                    } else if line.starts_with("author:") && author == "Unknown" {
                        author = line
                            .trim_start_matches("author:")
                            .trim()
                            .trim_matches('"')
                            .to_string();
                    } else if (line.starts_with("image:")
                        || line.starts_with("portrait:")
                        || line.starts_with("icon:"))
                        && image_relative_path.is_none()
                    {
                        let parts: Vec<&str> = line.splitn(2, ':').collect();
                        if parts.len() == 2 {
                            image_relative_path =
                                Some(parts[1].trim().trim_matches('"').to_string());
                        }
                    }
                }
            }
        }

        // Resolve absolute image_path with fallback to common image files in voicebank root
        let image_path = if let Some(ref rel) = image_relative_path {
            let p = root_path.join(rel);
            if p.exists() {
                Some(p)
            } else {
                None
            }
        } else {
            None
        }
        .or_else(|| {
            let default_names = [
                "character.png",
                "icon.png",
                "avatar.png",
                "portrait.png",
                "character.bmp",
                "icon.bmp",
                "avatar.bmp",
                "portrait.bmp",
                "character.jpg",
                "icon.jpg",
                "avatar.jpg",
                "portrait.jpg",
                "CHARACTER.PNG",
                "ICON.PNG",
                "AVATAR.PNG",
                "PORTRAIT.PNG",
            ];
            for name in default_names {
                let candidate = root_path.join(name);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
            None
        });

        // Load readme.txt if present
        let readme_path = root_path.join("readme.txt");
        if readme_path.exists() {
            if let Ok(bytes) = fs::read(&readme_path) {
                let (text, _, _) = SHIFT_JIS.decode(&bytes);
                readme_info = text.to_string();
            }
        }

        // Load prefix.map if present (with fallback for case variations or subfolder placement)
        let prefix_map = {
            let candidates = [
                root_path.join("prefix.map"),
                root_path.join("PREFIX.MAP"),
                root_path.join("Prefix.map"),
                root_path.join("Prefix.Map"),
            ];
            let mut loaded = None;
            for p in candidates {
                if p.exists() {
                    if let Ok(pm) = PrefixMap::parse_file(&p) {
                        loaded = Some(pm);
                        break;
                    }
                }
            }
            if loaded.is_none() {
                let yaml_path = root_path.join("character.yaml");
                if yaml_path.exists() {
                    if let Ok(content) = fs::read_to_string(&yaml_path) {
                        loaded = Some(PrefixMap::parse_yaml_str(&content));
                    }
                }
            }
            loaded.unwrap_or_default()
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
            image_path,
            entries,
            prefix_map,
        })
    }

    fn scan_oto_files(root: &Path, current_dir: &Path, entries: &mut HashMap<String, OtoEntry>) {
        if let Ok(read_dir) = fs::read_dir(current_dir) {
            for entry in read_dir.flatten() {
                let path = entry.path();
                if entry
                    .file_type()
                    .map(|kind| kind.is_symlink())
                    .unwrap_or(true)
                {
                    continue;
                }
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
        let lyric_trimmed = lyric.trim();
        if lyric_trimmed.is_empty() {
            return None;
        }

        let (prefix, suffix) = self
            .prefix_map
            .get_prefix_suffix(pitch_name)
            .unwrap_or(("", ""));

        let clean_p = prefix.trim_matches('"').trim_matches('\'');
        let clean_s = suffix.trim_matches('"').trim_matches('\'');

        let lyric_cands = crate::phonemizer::romaji::lyric_candidates(lyric_trimmed);

        let mut candidates = Vec::new();

        for cand_lyric in &lyric_cands {
            // 1. Direct get_alias from prefix_map
            let raw_alias = self.prefix_map.get_alias(cand_lyric, pitch_name);
            candidates.push(raw_alias);

            // 2. Prefix & Suffix combinations (with & without spaces / underscores)
            if !clean_p.is_empty() || !clean_s.is_empty() {
                candidates.push(format!("{}{}{}", clean_p, cand_lyric, clean_s));

                if !clean_p.is_empty() {
                    let p_trimmed = clean_p.trim();
                    candidates.push(format!("{} {}{}", p_trimmed, cand_lyric, clean_s));
                    candidates.push(format!("{}_{}{}", p_trimmed, cand_lyric, clean_s));
                    candidates.push(format!("{}{}{}", p_trimmed, cand_lyric, clean_s));
                }

                if !clean_s.is_empty() {
                    let s_trimmed = clean_s.trim();
                    candidates.push(format!("{}{}_{}", clean_p, cand_lyric, s_trimmed));
                    candidates.push(format!("{}{}{}", clean_p, cand_lyric, s_trimmed));
                }
            }

            // 3. Raw lyric candidate
            candidates.push(cand_lyric.clone());

            // 4. Pitch fallbacks directly on candidate lyric
            if !pitch_name.is_empty() {
                candidates.push(format!("{}_{}", cand_lyric, pitch_name));
                candidates.push(format!("{}_{}", pitch_name, cand_lyric));
                candidates.push(format!("{} {}", pitch_name, cand_lyric));
                candidates.push(format!("{} {}", cand_lyric, pitch_name));
                candidates.push(format!("{}{}", pitch_name, cand_lyric));
            }
        }

        // Try exact match for candidate strings
        for cand in &candidates {
            if let Some(entry) = self.entries.get(cand) {
                return Some(entry);
            }
        }

        // Try case-insensitive match for candidate strings
        for cand in &candidates {
            let cand_lower = cand.to_lowercase();
            for (key, entry) in &self.entries {
                if key.to_lowercase() == cand_lower {
                    return Some(entry);
                }
            }
        }

        // 5. VCV / CVVC Fallback (e.g. "e ら" -> "ら", "a か" -> "か")
        if let Some(space_idx) = lyric_trimmed.find(' ') {
            let cv_part = lyric_trimmed[space_idx + 1..].trim();
            if !cv_part.is_empty() && cv_part != lyric_trimmed {
                return self.find_entry(cv_part, pitch_name);
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
    pub fn search_entries<'a>(
        &'a self,
        search_query: &str,
        folder_filter: &str,
    ) -> Vec<(&'a String, &'a OtoEntry)> {
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
                    alias.to_lowercase().contains(&q)
                        || entry.wav_filename.to_lowercase().contains(&q)
                }
            })
            .collect()
    }
}
