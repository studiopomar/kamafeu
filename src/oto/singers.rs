use encoding_rs::SHIFT_JIS;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SingerInfo {
    pub name: String,
    pub author: String,
    pub web: Option<String>,
    pub path: PathBuf,
    pub image_path: Option<PathBuf>,
    pub voice_type: String,
    /// False for formats that can be catalogued but have no project render route.
    pub selectable: bool,
    /// KFV 2 uses the native project binding path instead of the oto.ini loader.
    pub is_native_kfv2: bool,
    pub diagnostic: Option<String>,
}

pub struct SingerScanner;

fn diffsinger_portrait(dir: &Path) -> Option<PathBuf> {
    let content = fs::read_to_string(dir.join("character.yaml")).ok()?;
    let value =
        yaml_serde::from_str::<yaml_serde::Value>(content.trim_start_matches('\u{feff}')).ok()?;
    let relative = value
        .get("image")
        .or_else(|| value.get("portrait"))
        .and_then(|value| value.as_str())?;
    let relative_path = Path::new(relative);
    if relative_path.is_absolute()
        || relative_path.components().any(|component| {
            !matches!(
                component,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        })
    {
        return None;
    }
    let candidate = dir.join(relative_path);
    candidate.is_file().then_some(candidate)
}

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
        // DiffSinger metadata is independent of oto.ini. Never route a neural
        // bank through the classic sampler merely because it also ships one.
        if dir.join("character.yaml").is_file()
            && (dir.join("dsconfig.yaml").is_file() || dir.join("configs/dsconfig.yaml").is_file())
        {
            return Some(Self::inspect_diffsinger(dir));
        }
        let cantor_manifest = dir.join("cantor.toml");
        if cantor_manifest.is_file() {
            let fallback_name = dir
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("Cantor nativo")
                .to_string();
            let cantor = crate::voicebank::cantor::Cantor::load_dir(dir);
            return Some(match cantor {
                Ok(cantor) if !dir.join("oto.ini").exists() => SingerInfo {
                    name: cantor.nome,
                    author: "Kamafeu KAMUSE".into(),
                    web: None,
                    path: dir.to_path_buf(),
                    image_path: cantor.imagem.map(|path| dir.join(path)),
                    voice_type: "Kamafeu · Tainah WORLD".into(),
                    selectable: true,
                    is_native_kfv2: true,
                    diagnostic: None,
                },
                Ok(_) => SingerInfo {
                    name: fallback_name,
                    author: "Conflito de formatos".into(),
                    web: None,
                    path: dir.to_path_buf(),
                    image_path: None,
                    voice_type: "Cantor nativo · conflito".into(),
                    selectable: false,
                    is_native_kfv2: true,
                    diagnostic: Some("o banco contém cantor.toml e oto.ini".into()),
                },
                Err(error) => SingerInfo {
                    name: fallback_name,
                    author: "cantor.toml inválido".into(),
                    web: None,
                    path: dir.to_path_buf(),
                    image_path: None,
                    voice_type: "Cantor nativo · não carregável".into(),
                    selectable: false,
                    is_native_kfv2: true,
                    diagnostic: Some(error),
                },
            });
        }

        let has_oto = dir.join("oto.ini").exists();
        let has_char = dir.join("character.txt").exists();
        let has_char_yaml = dir.join("character.yaml").exists();
        let has_native = dir.join("kamafeu_voicebank.json").exists();

        if has_native {
            if let Ok(content) = fs::read_to_string(dir.join("kamafeu_voicebank.json")) {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                    if value.get("format").and_then(|value| value.as_str())
                        == Some("kamafeu.voicebank")
                    {
                        return Some(Self::inspect_kfv2_manifest(dir, &content));
                    }
                }
            }
        }

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
        let mut web: Option<String> = None;
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

                // character.yaml is the authoritative OpenUtau metadata file.
                // Parse it as YAML as well as supporting the legacy line
                // fallback above, so quoted values and comments are handled
                // correctly and portrait-only singers appear in the gallery.
                if let Ok(value) = yaml_serde::from_str::<yaml_serde::Value>(&content) {
                    let yaml_string = |key: &str| {
                        value
                            .get(key)
                            .and_then(|item| item.as_str())
                            .map(str::to_owned)
                    };
                    if let Some(value) = yaml_string("name") {
                        if !value.is_empty() {
                            name = value;
                        }
                    }
                    if let Some(value) = yaml_string("author") {
                        if !value.is_empty() {
                            author = value;
                        }
                    }
                    web = yaml_string("web");
                    image_rel = yaml_string("image")
                        .or_else(|| yaml_string("portrait"))
                        .or(image_rel);
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
                "IMAGE.PNG",
                "CHARACTER.PNG",
                "ICON.PNG",
                "PORTRAIT.PNG",
                "AVATAR.PNG",
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
            web,
            path: dir.to_path_buf(),
            image_path,
            voice_type,
            selectable: true,
            is_native_kfv2: false,
            diagnostic: None,
        })
    }

    fn inspect_diffsinger(dir: &Path) -> SingerInfo {
        let fallback_name = dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("DiffSinger")
            .to_owned();
        match crate::voicebank::diffsinger::DiffSingerBank::inspect(dir) {
            Ok(bank) => SingerInfo {
                name: bank.name,
                author: bank.author.unwrap_or_else(|| "Desconhecido".into()),
                web: None,
                path: dir.to_path_buf(),
                image_path: diffsinger_portrait(dir),
                voice_type: match bank.layout {
                    crate::voicebank::diffsinger::Layout::Classic => "DiffSinger · clássico".into(),
                    crate::voicebank::diffsinger::Layout::V17 => "DiffSinger · v1.7".into(),
                },
                selectable: bank.loadable,
                is_native_kfv2: false,
                diagnostic: bank.diagnostic,
            },
            Err(error) => SingerInfo {
                name: fallback_name,
                author: "Configuração DiffSinger inválida".into(),
                web: None,
                path: dir.to_path_buf(),
                image_path: None,
                voice_type: "DiffSinger · não carregável".into(),
                selectable: false,
                is_native_kfv2: false,
                diagnostic: Some(error),
            },
        }
    }

    fn inspect_kfv2_manifest(dir: &Path, content: &str) -> SingerInfo {
        let fallback_name = dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("KFV 2 voice")
            .to_string();
        let manifest = match crate::voicebank::native::Manifest::parse_json(content) {
            Ok(manifest) => manifest,
            Err(error) => {
                return SingerInfo {
                    name: fallback_name,
                    author: "Manifesto KFV 2 inválido".into(),
                    web: None,
                    path: dir.to_path_buf(),
                    image_path: None,
                    voice_type: "KFV 2 · não carregável".into(),
                    selectable: false,
                    is_native_kfv2: true,
                    diagnostic: Some(error),
                };
            }
        };
        let image_path = manifest
            .presentation
            .as_ref()
            .and_then(|presentation| {
                presentation
                    .avatar
                    .as_ref()
                    .or(presentation.portrait.as_ref())
            })
            .and_then(|relative| manifest.resolve_existing(dir, relative).ok());
        SingerInfo {
            name: manifest.display_name,
            author: manifest.publisher,
            web: None,
            path: dir.to_path_buf(),
            image_path,
            voice_type: "Voz Kamafeu · Nativa (preview SOLA experimental)".into(),
            selectable: true,
            is_native_kfv2: true,
            diagnostic: Some(
                "Preview experimental via SOLA. Use phones explícitos no campo de pronúncia; ainda não há G2P nativo nem A/B acústico aprovado.".into(),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kfv2_manifest() -> crate::voicebank::native::Manifest {
        use crate::voicebank::native::{
            Defaults, EngineRequirement, Layer, Manifest, PhoneticProfileRef, Style,
        };
        Manifest {
            format: "kamafeu.voicebank".into(),
            schema_version: "2.0.0".into(),
            voice_id: "com.example.voice".into(),
            bank_id: "com.example.voice.natural".into(),
            bank_version: "1.0.0".into(),
            package_revision: 1,
            display_name: "Voz Nativa de Teste".into(),
            publisher: "Autoria de Teste".into(),
            description: String::new(),
            languages: vec!["pt-BR".into()],
            phonetic_profiles: vec![PhoneticProfileRef {
                id: "pt-br-v1".into(),
                language: "pt-BR".into(),
                path: "phonetics/pt-BR.json".into(),
            }],
            engine: EngineRequirement {
                id: "kamafeu.concat".into(),
                api_major: 1,
                required_features: vec![],
            },
            styles: vec![Style {
                id: "natural".into(),
                display_name: "Natural".into(),
            }],
            layers: vec![Layer {
                id: "natural-c4".into(),
                style_id: "natural".into(),
                recorded_midi: 60,
                units_path: "units/c4.json".into(),
            }],
            defaults: Defaults {
                phonetic_profile_id: "pt-br-v1".into(),
                style_id: "natural".into(),
                layer_policy: "nearest-recorded".into(),
                missing_unit_policy: "error".into(),
            },
            presentation: None,
            license: None,
        }
    }

    #[test]
    fn test_inspect_missing_dir_returns_none() {
        let path = PathBuf::from("non_existent_voicebank_dir_12345");
        assert!(SingerScanner::inspect_singer_directory(&path).is_none());
    }

    #[test]
    fn diffsinger_portrait_comes_from_its_own_character_metadata() {
        let directory = tempfile::tempdir().expect("temporary DiffSinger directory");
        let portrait = directory.path().join("singer.png");
        fs::write(&portrait, b"placeholder image bytes").expect("portrait");
        fs::write(
            directory.path().join("character.yaml"),
            "name: Test DiffSinger\nportrait: singer.png\n",
        )
        .expect("character metadata");
        assert_eq!(diffsinger_portrait(directory.path()), Some(portrait));

        fs::write(
            directory.path().join("character.yaml"),
            "name: Test DiffSinger\nportrait: ../unrelated-utau.png\n",
        )
        .expect("unsafe character metadata");
        assert_eq!(diffsinger_portrait(directory.path()), None);
    }

    #[test]
    fn openutau_yaml_metadata_is_used_by_singer_gallery() {
        let directory = tempfile::tempdir().expect("temporary singer directory");
        fs::write(
            directory.path().join("oto.ini"),
            "ka.wav=ka,0,100,-400,80,30\n",
        )
        .expect("oto.ini");
        fs::write(directory.path().join("portrait.png"), b"not-an-image").expect("portrait");
        fs::write(
            directory.path().join("character.yaml"),
            "name: Gallery Singer\nauthor: YAML Author\nportrait: portrait.png\nweb: https://example.test\n",
        )
        .expect("character metadata");

        let singer = SingerScanner::inspect_singer_directory(directory.path())
            .expect("voicebank should be listed");
        assert_eq!(singer.name, "Gallery Singer");
        assert_eq!(singer.author, "YAML Author");
        assert_eq!(singer.web.as_deref(), Some("https://example.test"));
        assert_eq!(
            singer.image_path.as_deref(),
            Some(directory.path().join("portrait.png").as_path())
        );
    }

    #[test]
    fn finds_uppercase_png_portrait_without_metadata() {
        let directory = tempfile::tempdir().expect("temporary singer directory");
        fs::write(
            directory.path().join("oto.ini"),
            "ka.wav=ka,0,100,-400,80,30\n",
        )
        .expect("oto.ini");
        fs::write(directory.path().join("PORTRAIT.PNG"), b"not-an-image").expect("portrait");

        let singer = SingerScanner::inspect_singer_directory(directory.path())
            .expect("voicebank should be listed");

        let portrait = singer.image_path.expect("PNG portrait should be detected");
        assert!(portrait.exists());
        assert_eq!(
            portrait
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.to_ascii_lowercase()),
            Some("portrait.png".to_string())
        );
    }

    #[test]
    fn kfv2_catalog_reads_manifest_metadata_without_loading_audio_and_enables_native_binding() {
        let directory = tempfile::tempdir().expect("temporary KFV 2 folder");
        fs::write(
            directory.path().join("kamafeu_voicebank.json"),
            serde_json::to_vec(&kfv2_manifest()).unwrap(),
        )
        .unwrap();

        let singer = SingerScanner::inspect_singer_directory(directory.path())
            .expect("valid KFV 2 manifest should be catalogued");
        assert_eq!(singer.name, "Voz Nativa de Teste");
        assert_eq!(singer.author, "Autoria de Teste");
        assert!(singer.voice_type.contains("Voz Kamafeu · Nativa"));
        assert!(singer.selectable);
        assert!(singer.is_native_kfv2);
        assert!(singer.diagnostic.unwrap().contains("phones explícitos"));
    }

    #[test]
    fn invalid_kfv2_manifest_is_not_misclassified_as_a_classic_singer() {
        let directory = tempfile::tempdir().expect("temporary invalid KFV 2 folder");
        fs::write(
            directory.path().join("kamafeu_voicebank.json"),
            r#"{"format":"kamafeu.voicebank","schema_version":"2.0.0"}"#,
        )
        .unwrap();
        fs::write(
            directory.path().join("oto.ini"),
            "sample.wav=ka,0,100,-400,80,30\n",
        )
        .unwrap();

        let singer = SingerScanner::inspect_singer_directory(directory.path())
            .expect("invalid native bank should remain visible as an error");
        assert_eq!(singer.voice_type, "KFV 2 · não carregável");
        assert!(!singer.selectable);
        assert!(singer.diagnostic.is_some());
    }
}
