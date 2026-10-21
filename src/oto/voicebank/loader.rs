use encoding_rs::SHIFT_JIS;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use super::super::entry::OtoEntry;
use super::super::prefix_map::PrefixMap;
use super::{decode_text_autodetect, extract_kfv, is_safe_relative_path, Voicebank};

impl Voicebank {
    pub fn new<P: AsRef<Path>>(root_path: P) -> Result<Self, std::io::Error> {
        let root_path = root_path.as_ref().to_path_buf();
        let mut temp_dir = None;
        let real_root_path =
            if root_path.is_file() && root_path.extension().is_some_and(|e| e == "kfv") {
                let td = tempfile::Builder::new().prefix("kamafeu_vb_").tempdir()?;
                extract_kfv(&root_path, td.path())?;
                let path = td.path().to_path_buf();
                temp_dir = Some(Arc::new(td));
                path
            } else if root_path.is_dir() {
                root_path.clone()
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!(
                        "voicebank path not found or not a directory/.kfv file: {}",
                        root_path.display()
                    ),
                ));
            };

        let native_config_path = real_root_path.join("kamafeu_voicebank.json");
        if real_root_path.join("cantor.toml").is_file() {
            let kind = if real_root_path.join("oto.ini").is_file() {
                std::io::ErrorKind::InvalidData
            } else {
                std::io::ErrorKind::Unsupported
            };
            let message = if kind == std::io::ErrorKind::InvalidData {
                "conflito de formato: o banco contém cantor.toml e oto.ini"
            } else {
                "cantor.toml é um banco Kamafeu nativo; vincule-o pela galeria de Cantores"
            };
            return Err(std::io::Error::new(kind, message));
        }
        // KFV 2 has an explicit format discriminator. Never reinterpret a
        // malformed or newer native manifest as a classic oto.ini voicebank.
        if native_config_path.is_file() {
            let content = fs::read_to_string(&native_config_path)?;
            let header: serde_json::Value = serde_json::from_str(&content).map_err(|e| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("manifest inválido: {e}"),
                )
            })?;
            if header.get("format").and_then(|v| v.as_str()) == Some("kamafeu.voicebank") {
                let manifest = crate::voicebank::native::Manifest::parse_json(&content)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Unsupported,
                    format!(
                        "{} usa KFV 2; abra pela galeria nativa para vinculá-lo à faixa do projeto",
                        manifest.display_name
                    ),
                ));
            }
        }
        // oto.ini is the authoritative UTAU source. A generated native
        // index may be stale after a voicebank update, so never let it hide
        // aliases that are present in the user's oto.ini.
        if native_config_path.exists() && !real_root_path.join("oto.ini").is_file() {
            if let Ok(content) = fs::read_to_string(&native_config_path) {
                if let Ok(cfg) = serde_json::from_str::<crate::copaiba::CopaibaConfig>(&content) {
                    let mut pm = PrefixMap::new();
                    for (pitch, (pref, suff)) in cfg.prefix_map {
                        pm.insert(pitch, pref, suff);
                    }

                    let mut entries = HashMap::new();
                    for e in cfg.entries {
                        if !is_safe_relative_path(Path::new(&e.wav_filename)) {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("caminho WAV inválido no voicebank: {}", e.wav_filename),
                            ));
                        }
                        let entry = OtoEntry {
                            wav_filename: e.wav_filename.clone(),
                            alias: e.alias.clone(),
                            offset: e.corte_inicial_ms,
                            consonant: e.consoante_ms,
                            cutoff: e.corte_final_ms,
                            preutterance: e.preutterance_ms,
                            overlap: e.overlap_ms,
                            loop_start: e.loop_inicio_ms,
                            loop_end: e.loop_fim_ms,
                            tail_start: e.cauda_final_ms,
                        };
                        entries.insert(e.alias.clone(), entry);
                    }

                    let image_path = match cfg.image_filename.as_deref() {
                        Some(img) if is_safe_relative_path(Path::new(img)) => {
                            Some(real_root_path.join(img))
                        }
                        Some(img) => {
                            return Err(std::io::Error::new(
                                std::io::ErrorKind::InvalidData,
                                format!("caminho de imagem inválido no voicebank: {img}"),
                            ));
                        }
                        None => None,
                    };

                    let readme_path = real_root_path.join("readme.txt");
                    let mut readme_info = String::new();
                    if readme_path.exists() {
                        if let Ok(bytes) = fs::read(&readme_path) {
                            readme_info = decode_text_autodetect(&bytes);
                        }
                    }

                    return Ok(Self {
                        root_path: real_root_path,
                        name: cfg.voicebank_name,
                        author: cfg.author,
                        character_info: format!("Versão: {}", cfg.version),
                        readme_info,
                        image_path,
                        entries,
                        case_insensitive_entries: OnceLock::new(),
                        prefix_map: pm,
                        temp_dir,
                    });
                }
            }
        }

        let mut name = real_root_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown Voicebank")
            .to_string();
        let mut author = "Unknown".to_string();
        let mut character_info = String::new();
        let mut readme_info = String::new();
        let mut image_relative_path: Option<String> = None;
        let mut portrait_relative_path: Option<String> = None;

        let char_path = real_root_path.join("character.txt");
        if char_path.exists() {
            if let Ok(bytes) = fs::read(&char_path) {
                let text = decode_text_autodetect(&bytes);
                character_info = text.clone();
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

        let char_yaml_path = real_root_path.join("character.yaml");
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

                // OpenUtau's YAML is UTF-8 and may contain quoted values,
                // comments, localized names and fields in any order. Use the
                // YAML parser for the fields that are useful to Kamafeu while
                // keeping the legacy character.txt fallback above.
                if let Ok(value) = yaml_serde::from_str::<yaml_serde::Value>(&content) {
                    let yaml_string = |key: &str| {
                        value
                            .get(key)
                            .and_then(|item| item.as_str())
                            .map(str::to_owned)
                            .or_else(|| {
                                value
                                    .get(key)
                                    .and_then(|item| item.as_f64())
                                    .map(|number| number.to_string())
                            })
                    };

                    if let Some(value) = yaml_string("name") {
                        name = value;
                    }
                    if let Some(value) = yaml_string("author") {
                        author = value;
                    }
                    image_relative_path = image_relative_path.or_else(|| yaml_string("image"));
                    portrait_relative_path = yaml_string("portrait");

                    let mut details = Vec::new();
                    for key in ["voice", "version", "singer_type", "web"] {
                        if let Some(value) = yaml_string(key) {
                            details.push(format!("{key}: {value}"));
                        }
                    }
                    if !details.is_empty() {
                        character_info = details.join("\n");
                    }
                }
            }
        }

        if image_relative_path.is_none() {
            image_relative_path = portrait_relative_path;
        }

        let image_path = if let Some(ref rel) = image_relative_path {
            if !is_safe_relative_path(Path::new(rel)) {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("caminho de imagem inválido no voicebank: {rel}"),
                ));
            }
            let p = real_root_path.join(rel);
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
                "image.png",
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
                "IMAGE.PNG",
                "CHARACTER.PNG",
                "ICON.PNG",
                "AVATAR.PNG",
                "PORTRAIT.PNG",
            ];
            for name in default_names {
                let candidate = real_root_path.join(name);
                if candidate.exists() {
                    return Some(candidate);
                }
            }
            None
        });

        let readme_path = real_root_path.join("readme.txt");
        if readme_path.exists() {
            if let Ok(bytes) = fs::read(&readme_path) {
                let (text, _, _) = SHIFT_JIS.decode(&bytes);
                readme_info = text.to_string();
            }
        }

        let prefix_map = {
            let candidates = [
                real_root_path.join("prefix.map"),
                real_root_path.join("PREFIX.MAP"),
                real_root_path.join("Prefix.map"),
                real_root_path.join("Prefix.Map"),
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
                let yaml_path = real_root_path.join("character.yaml");
                if yaml_path.exists() {
                    if let Ok(content) = fs::read_to_string(&yaml_path) {
                        loaded = Some(PrefixMap::parse_yaml_str(&content));
                    }
                }
            }
            match loaded {
                Some(map) => {
                    if let Ok(yaml) = fs::read_to_string(real_root_path.join("character.yaml")) {
                        map.with_voicecolors(&yaml)
                    } else {
                        map
                    }
                }
                None => PrefixMap::default(),
            }
        };

        let mut entries = HashMap::new();
        Self::scan_oto_files(&real_root_path, &real_root_path, &mut entries)?;

        Ok(Self {
            root_path: real_root_path,
            name,
            author,
            character_info,
            readme_info,
            image_path,
            entries,
            case_insensitive_entries: OnceLock::new(),
            prefix_map,
            temp_dir,
        })
    }
}
