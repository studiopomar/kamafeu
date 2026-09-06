use encoding_rs::SHIFT_JIS;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SingerInfo {
    pub name: String,
    pub author: String,
    pub path: PathBuf,
    pub image_path: Option<PathBuf>,
    pub voice_type: String,
}

pub struct SingerScanner;

impl SingerScanner {
    pub fn default_singers_directories() -> Vec<PathBuf> {
        let mut dirs = Vec::new();

        if let Ok(home) = std::env::var("HOME") {
            let home_p = PathBuf::from(&home);

            let mac_openutau = home_p.join("Library/Application Support/OpenUtau/Singers");
            if mac_openutau.exists() {
                dirs.push(mac_openutau);
            }

            let linux_openutau = home_p.join(".config/OpenUtau/Singers");
            if linux_openutau.exists() && !dirs.contains(&linux_openutau) {
                dirs.push(linux_openutau);
            }

            let local_openutau = home_p.join("OpenUtau/Singers");
            if local_openutau.exists() && !dirs.contains(&local_openutau) {
                dirs.push(local_openutau);
            }
        }

        if let Ok(appdata) = std::env::var("APPDATA") {
            let win_openutau = PathBuf::from(appdata).join("OpenUtau/Singers");
            if win_openutau.exists() && !dirs.contains(&win_openutau) {
                dirs.push(win_openutau);
            }
        }

        dirs
    }

    pub fn scan_directories(directories: &[PathBuf]) -> Vec<SingerInfo> {
        let mut singers = Vec::new();
        let mut seen_paths = std::collections::HashSet::new();

        for dir in directories {
            if !dir.exists() || !dir.is_dir() {
                continue;
            }

            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() && !seen_paths.contains(&path) {
                        if let Some(singer) = Self::inspect_singer_directory(&path) {
                            seen_paths.insert(path.clone());
                            singers.push(singer);
                        }
                    }
                }
            }
        }

        singers.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        singers
    }

    pub fn find_singer_by_name_or_path(
        query: &str,
        recent_paths: &[PathBuf],
        custom_singer_dirs: &[PathBuf],
    ) -> Option<PathBuf> {
        let q = query.trim();
        if q.is_empty() {
            return None;
        }

        let direct_path = PathBuf::from(q);
        if direct_path.exists() && direct_path.is_dir() {
            return Some(direct_path);
        }

        let q_lower = q.to_lowercase();

        for recent in recent_paths {
            if recent.exists() && recent.is_dir() {
                if let Some(info) = Self::inspect_singer_directory(recent) {
                    if info.name.to_lowercase() == q_lower
                        || recent
                            .file_name()
                            .and_then(|n| n.to_str())
                            .is_some_and(|n| n.to_lowercase() == q_lower)
                    {
                        return Some(recent.clone());
                    }
                }
            }
        }

        let mut all_dirs = custom_singer_dirs.to_vec();
        for d in Self::default_singers_directories() {
            if !all_dirs.contains(&d) {
                all_dirs.push(d);
            }
        }
        if let Ok(home) = std::env::var("HOME") {
            let dl = PathBuf::from(home).join("Downloads");
            if dl.exists() && !all_dirs.contains(&dl) {
                all_dirs.push(dl);
            }
        }

        let singers = Self::scan_directories(&all_dirs);
        for s in singers {
            if s.name.to_lowercase() == q_lower
                || s.path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.to_lowercase() == q_lower)
            {
                return Some(s.path);
            }
        }

        None
    }

    pub fn inspect_singer_directory(dir: &Path) -> Option<SingerInfo> {
        let has_oto = dir.join("oto.ini").exists();
        let has_char = dir.join("character.txt").exists();
        let has_char_yaml = dir.join("character.yaml").exists();
        let has_native = dir.join("kamafeu_voicebank.json").exists();

        let mut sub_has_oto = false;
        if !has_oto && !has_char && !has_char_yaml && !has_native {
            if let Ok(entries) = fs::read_dir(dir) {
                for e in entries.flatten() {
                    let sub_p = e.path();
                    if sub_p.is_dir() && sub_p.join("oto.ini").exists() {
                        sub_has_oto = true;
                        break;
                    }
                }
            }
        }

        if !has_oto && !has_char && !has_char_yaml && !has_native && !sub_has_oto {
            return None;
        }

        let mut name = dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Cantor Desconhecido")
            .to_string();
        let mut author = "Desconhecido".to_string();
        let mut image_path: Option<PathBuf> = None;
        let mut image_rel: Option<String> = None;

        let char_path = dir.join("character.txt");
        if char_path.exists() {
            if let Ok(bytes) = fs::read(&char_path) {
                let (text, _, _) = SHIFT_JIS.decode(&bytes);
                for line in text.lines() {
                    let l = line.trim();
                    if l.starts_with("name=") {
                        name = l.trim_start_matches("name=").trim().to_string();
                    } else if l.starts_with("author=") {
                        author = l.trim_start_matches("author=").trim().to_string();
                    } else if l.starts_with("image=") {
                        image_rel = Some(l.trim_start_matches("image=").trim().to_string());
                    } else if l.starts_with("icon=") && image_rel.is_none() {
                        image_rel = Some(l.trim_start_matches("icon=").trim().to_string());
                    } else if l.starts_with("portrait=") && image_rel.is_none() {
                        image_rel = Some(l.trim_start_matches("portrait=").trim().to_string());
                    }
                }
            }
        }

        let yaml_path = dir.join("character.yaml");
        if yaml_path.exists() {
            if let Ok(content) = fs::read_to_string(&yaml_path) {
                for line in content.lines() {
                    let l = line.trim();
                    if l.starts_with("name:") {
                        let parsed_name = l.trim_start_matches("name:").trim().trim_matches('"');
                        if !parsed_name.is_empty() {
                            name = parsed_name.to_string();
                        }
                    } else if l.starts_with("author:") {
                        let parsed_author =
                            l.trim_start_matches("author:").trim().trim_matches('"');
                        if !parsed_author.is_empty() {
                            author = parsed_author.to_string();
                        }
                    } else if l.starts_with("image:") && image_rel.is_none() {
                        let img = l.trim_start_matches("image:").trim().trim_matches('"');
                        if !img.is_empty() {
                            image_rel = Some(img.to_string());
                        }
                    } else if l.starts_with("portrait:") && image_rel.is_none() {
                        let img = l.trim_start_matches("portrait:").trim().trim_matches('"');
                        if !img.is_empty() {
                            image_rel = Some(img.to_string());
                        }
                    }
                }
            }
        }

        if let Some(rel) = image_rel {
            let img_candidate = dir.join(rel);
            if img_candidate.exists() {
                image_path = Some(img_candidate);
            }
        }

        if image_path.is_none() {
            let common_images = [
                "image.png",
                "image.jpg",
                "image.jpeg",
                "image.bmp",
                "character.png",
                "character.jpg",
                "icon.png",
                "portrait.png",
                "char.png",
                "avatar.png",
            ];
            for img_name in common_images {
                let p = dir.join(img_name);
                if p.exists() {
                    image_path = Some(p);
                    break;
                }
            }
        }

        let voice_type = if dir.join("kamafeu_voicebank.json").exists() {
            "Kamafeu Studio Nativo"
        } else if sub_has_oto {
            "Multipitch VCV/CVC"
        } else {
            "UTAU Standard"
        }
        .to_string();

        Some(SingerInfo {
            name,
            author,
            path: dir.to_path_buf(),
            image_path,
            voice_type,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inspect_missing_dir_returns_none() {
        let path = PathBuf::from("non_existent_voicebank_dir_12345");
        assert!(SingerScanner::inspect_singer_directory(&path).is_none());
    }
}
