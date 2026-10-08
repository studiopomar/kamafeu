use encoding_rs::SHIFT_JIS;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use super::entry::OtoEntry;
use super::parser::OtoParser;
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
    /// Returns true when this folder is an OpenUtau DiffSinger bank. These
    /// banks must be rendered as complete phrases with ONNX models and must
    /// never fall through to the classic oto.ini renderer.
    pub fn is_diffsinger(&self) -> bool {
        // An UTAU bank may ship auxiliary folders named "dsvocoder"; that
        // folder alone must not bypass its oto.ini renderer. A DiffSinger
        // bank needs its explicit configuration, or characteristic model
        // assets without a classic oto.ini.
        if self.root_path.join("oto.ini").is_file() {
            return false;
        }
        if self.root_path.join("dsconfig.yaml").is_file() {
            return true;
        }
        self.root_path.join("acoustic.onnx").is_file()
            || (self.root_path.join("dsvocoder").is_dir()
                && self.root_path.join("phonemes.txt").is_file())
    }

    pub fn diffsinger_config(&self) -> Result<crate::oto::DiffSingerConfig, String> {
        if !self.is_diffsinger() {
            return Err("o voicebank ativo não é DiffSinger".to_string());
        }
        crate::oto::DiffSingerConfig::load(&self.root_path)
    }

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
        Self::scan_oto_files(&real_root_path, &real_root_path, &mut entries);

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

    fn scan_oto_files(root: &Path, current_dir: &Path, entries: &mut HashMap<String, OtoEntry>) {
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
                if let Ok(file_entries) = OtoParser::parse_file(&path) {
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
            }
            for entry in directory_entries {
                if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                    Self::scan_oto_files(root, &entry.path(), entries);
                }
            }
        }
    }

    /// Exact phonetic lookup with pitch and candidate mapping. Never changes case or drops a VC/VCV prefix.
    pub fn find_mapped_entry(&self, lyric: &str, pitch_name: &str) -> Option<&OtoEntry> {
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
            let raw_alias = self.prefix_map.get_alias(cand_lyric, pitch_name);
            candidates.push(raw_alias);

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

            candidates.push(cand_lyric.clone());

            if !pitch_name.is_empty() {
                candidates.push(format!("{}_{}", cand_lyric, pitch_name));
                candidates.push(format!("{}_{}", pitch_name, cand_lyric));
                candidates.push(format!("{} {}", pitch_name, cand_lyric));
                candidates.push(format!("{} {}", cand_lyric, pitch_name));
                candidates.push(format!("{}{}", pitch_name, cand_lyric));
            }
        }

        for cand in &candidates {
            if let Some(entry) = self.entries.get(cand) {
                return Some(entry);
            }
        }

        // Exact authored aliases remain the final local fallback. This keeps
        // aliases such as "k a" available, while allowing a mapped pitch
        // subbank such as "a s_C4" to win when the current note has one.
        if let Some(entry) = self.entries.get(lyric_trimmed) {
            return Some(entry);
        }

        if let Some(target_midi) = crate::dsp::pitch::note_name_to_midi(pitch_name) {
            let mut alt_pitches: Vec<(&str, i32)> = self
                .prefix_map
                .mapped_pitches()
                .filter(|&p| p != pitch_name)
                .filter_map(|p| {
                    crate::dsp::pitch::note_name_to_midi(p)
                        .map(|m| (p, (m as i32 - target_midi as i32).abs()))
                })
                .collect();

            alt_pitches.sort_by_key(|&(_, dist)| dist);

            for (alt_pitch, _) in alt_pitches {
                if let Some((pfx, sfx)) = self.prefix_map.get_prefix_suffix(alt_pitch) {
                    for cand_lyric in &lyric_cands {
                        let cand = format!("{}{}{}", pfx.trim(), cand_lyric, sfx.trim());
                        if let Some(entry) = self.entries.get(&cand) {
                            return Some(entry);
                        }
                    }
                }
            }
        }

        None
    }

    /// Content identity for render provenance and cache diagnostics. It uses
    /// sorted aliases, oto timing and WAV bytes, so replacing a sample at the
    /// same path cannot preserve the old identity.
    pub fn content_fingerprint(&self) -> String {
        let mut hash = 0xcbf29ce484222325u64;
        let mut entries = self.entries.iter().collect::<Vec<_>>();
        entries.sort_by(|left, right| left.0.cmp(right.0));
        for (alias, entry) in entries {
            for value in [
                alias.as_bytes(),
                entry.wav_filename.as_bytes(),
                format!(
                    "{:.9},{:.9},{:.9},{:.9},{:.9},{:?},{:?},{:?}",
                    entry.offset,
                    entry.consonant,
                    entry.cutoff,
                    entry.preutterance,
                    entry.overlap,
                    entry.loop_start,
                    entry.loop_end,
                    entry.tail_start
                )
                .as_bytes(),
            ] {
                for byte in value {
                    hash ^= u64::from(*byte);
                    hash = hash.wrapping_mul(0x100000001b3);
                }
                hash ^= 0xff;
                hash = hash.wrapping_mul(0x100000001b3);
            }
            if let Ok(bytes) = fs::read(self.root_path.join(&entry.wav_filename)) {
                for byte in bytes {
                    hash ^= u64::from(byte);
                    hash = hash.wrapping_mul(0x100000001b3);
                }
            }
            hash ^= 0xfe;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{hash:016x}")
    }

    /// Resolves an alias through the same prefix/suffix and case rules used by
    /// rendering, while returning a stable non-destructive diagnostic.
    pub fn alias_resolution_report(
        &self,
        lyric: impl Into<String>,
        pitch: impl Into<String>,
    ) -> AliasResolutionReport {
        let requested_alias = lyric.into();
        let pitch = pitch.into();
        match self.find_mapped_entry(&requested_alias, &pitch) {
            Some(entry) => AliasResolutionReport {
                requested_alias,
                pitch,
                resolved: true,
                resolved_alias: Some(entry.alias.clone()),
                wav_filename: Some(entry.wav_filename.clone()),
                offset_ms: Some(entry.offset),
                consonant_ms: Some(entry.consonant),
                preutterance_ms: Some(entry.preutterance),
                overlap_ms: Some(entry.overlap),
                reason: None,
            },
            None => AliasResolutionReport {
                requested_alias,
                pitch,
                resolved: false,
                resolved_alias: None,
                wav_filename: None,
                offset_ms: None,
                consonant_ms: None,
                preutterance_ms: None,
                overlap_ms: None,
                reason: Some("alias ausente ou sem entrada compatível no oto.ini".to_string()),
            },
        }
    }

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
            let raw_alias = self.prefix_map.get_alias(cand_lyric, pitch_name);
            candidates.push(raw_alias);

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

            candidates.push(cand_lyric.clone());

            if !pitch_name.is_empty() {
                candidates.push(format!("{}_{}", cand_lyric, pitch_name));
                candidates.push(format!("{}_{}", pitch_name, cand_lyric));
                candidates.push(format!("{} {}", pitch_name, cand_lyric));
                candidates.push(format!("{} {}", cand_lyric, pitch_name));
                candidates.push(format!("{}{}", pitch_name, cand_lyric));
            }
        }

        for cand in &candidates {
            if let Some(entry) = self.entries.get(cand) {
                return Some(entry);
            }
        }

        if let Some(entry) = self.entries.get(lyric_trimmed) {
            return Some(entry);
        }

        // Try case-insensitive match using a single, lazily-built index. The
        // previous nested scan was O(candidates × aliases) for every note.
        let case_insensitive_entries = self.case_insensitive_entries.get_or_init(|| {
            self.entries
                .keys()
                .map(|alias| (alias.to_lowercase(), alias.clone()))
                .collect()
        });
        for cand in &candidates {
            if let Some(original_alias) = case_insensitive_entries.get(&cand.to_lowercase()) {
                if let Some(entry) = self.entries.get(original_alias) {
                    return Some(entry);
                }
            }
        }

        if let Some(target_midi) = crate::dsp::pitch::note_name_to_midi(pitch_name) {
            let mut alt_pitches: Vec<(&str, i32)> = self
                .prefix_map
                .mapped_pitches()
                .filter(|&p| p != pitch_name)
                .filter_map(|p| {
                    crate::dsp::pitch::note_name_to_midi(p)
                        .map(|m| (p, (m as i32 - target_midi as i32).abs()))
                })
                .collect();

            alt_pitches.sort_by_key(|&(_, dist)| dist);

            for (alt_pitch, _) in alt_pitches {
                if let Some((pfx, sfx)) = self.prefix_map.get_prefix_suffix(alt_pitch) {
                    let cand = format!("{}{}{}", pfx.trim(), lyric_trimmed, sfx.trim());
                    if let Some(entry) = self.entries.get(&cand) {
                        return Some(entry);
                    }
                }
            }
        }

        if let Some(space_idx) = lyric_trimmed.find(' ') {
            let cv_part = lyric_trimmed[space_idx + 1..].trim();
            if !cv_part.is_empty() && cv_part != lyric_trimmed {
                return self.find_entry(cv_part, pitch_name);
            }
        }

        None
    }

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

fn extract_kfv(zip_path: &Path, dest_dir: &Path) -> Result<(), std::io::Error> {
    const MAX_FILES: usize = 10_000;
    const MAX_FILE_SIZE: u64 = 512 * 1024 * 1024;
    const MAX_TOTAL_SIZE: u64 = 4 * 1024 * 1024 * 1024;

    let file = fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    if archive.len() > MAX_FILES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "arquivo KFV contém entradas demais",
        ));
    }

    let mut total_size = 0u64;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        if file.size() > MAX_FILE_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("entrada KFV excede o limite de tamanho: {}", file.name()),
            ));
        }
        if file.size() > 10 * 1024 * 1024
            && file.compressed_size() > 0
            && file.size() / file.compressed_size() > 1_000
        {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("entrada KFV possui compressão suspeita: {}", file.name()),
            ));
        }
        total_size = total_size.checked_add(file.size()).ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "tamanho KFV inválido")
        })?;
        if total_size > MAX_TOTAL_SIZE {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "conteúdo descompactado do KFV excede o limite permitido",
            ));
        }
        let outpath = match file.enclosed_name() {
            Some(path) => dest_dir.join(path),
            None => continue,
        };

        if (*file.name()).ends_with('/') {
            fs::create_dir_all(&outpath)?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)?;
                }
            }
            let mut outfile = fs::File::create(&outpath)?;
            let copied = std::io::copy(&mut file.by_ref().take(MAX_FILE_SIZE + 1), &mut outfile)?;
            if copied > MAX_FILE_SIZE {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "entrada KFV excedeu o tamanho declarado",
                ));
            }
        }
    }
    Ok(())
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
    use tempfile::tempdir;

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
        assert!(!vb.is_diffsinger());
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

    #[test]
    fn oto_bank_wins_over_diff_singer_metadata() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("oto.ini"), "ka.wav=k a,0,100,-400,80,30\n").unwrap();
        fs::write(
            dir.path().join("dsconfig.yaml"),
            "phonemes: phonemes.txt\nacoustic: acoustic.onnx\nvocoder: vocoder.onnx\n",
        )
        .unwrap();

        let vb = Voicebank::new(dir.path()).unwrap();
        assert!(!vb.is_diffsinger());
        assert_eq!(vb.find_mapped_entry("k a", "D4").unwrap().alias, "k a");
    }
}
