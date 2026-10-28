//! Read-only DiffSinger bank inspection. This deliberately does not route a
//! neural bank through the classic oto.ini renderer.
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

static INSPECTION_CACHE: OnceLock<Mutex<BTreeMap<PathBuf, Result<DiffSingerBank, String>>>> =
    OnceLock::new();

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Layout {
    Classic,
    V17,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionProvider {
    Cpu,
    CoreMl,
}

/// DiffSinger phone input strategy. BRAPA phonetic accepts phones directly in
/// the lyric (with or without spaces); automatic mode treats ordinary lyrics
/// as text and applies the bank language's grapheme mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffSingerPhonemizer {
    Automatic,
    BrapaPhonetic,
}

impl Default for DiffSingerPhonemizer {
    fn default() -> Self {
        Self::Automatic
    }
}

impl Default for ExecutionProvider {
    fn default() -> Self {
        Self::Cpu
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffSingerBank {
    pub root: PathBuf,
    pub layout: Layout,
    pub name: String,
    pub author: Option<String>,
    pub languages: Vec<String>,
    pub speakers: Vec<String>,
    pub dictionaries: BTreeMap<String, PathBuf>,
    pub dictionary_symbols: BTreeMap<String, BTreeSet<String>>,
    /// DiffSinger dictionary symbol classes used by the duration aligner.
    pub phone_types: BTreeMap<String, String>,
    /// Model token to the integer ID expected by the acoustic graph.
    pub phonemes: BTreeMap<String, i64>,
    /// Language tag to the integer ID expected by the acoustic graph.
    pub language_ids: BTreeMap<String, i64>,
    /// Speaker identifiers as declared by dsconfig.yaml (paths are relative
    /// to the bank root, as in OpenUtau).
    pub speaker_embeddings: BTreeMap<String, PathBuf>,
    pub predictor_models: BTreeMap<String, PathBuf>,
    pub duration_speaker_embeddings: BTreeMap<String, PathBuf>,
    pub predict_dur: bool,
    pub duration_uses_speakers: bool,
    pub sample_rate: u32,
    pub hop_size: usize,
    pub hidden_size: usize,
    pub mel_bins: usize,
    pub use_lang_id: bool,
    pub use_key_shift_embed: bool,
    pub use_speed_embed: bool,
    pub use_continuous_acceleration: bool,
    pub use_variable_depth: bool,
    pub acoustic_model: Option<PathBuf>,
    pub vocoder_model: Option<PathBuf>,
    pub loadable: bool,
    pub diagnostic: Option<String>,
}

impl DiffSingerBank {
    pub fn inspect(root: impl AsRef<Path>) -> Result<Self, String> {
        let root = root.as_ref();
        let key = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
        let cache = INSPECTION_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
        if let Ok(cache) = cache.lock() {
            if let Some(result) = cache.get(&key) {
                return result.clone();
            }
        }
        let result = Self::inspect_uncached(&key);
        if let Ok(mut cache) = cache.lock() {
            cache.insert(key, result.clone());
        }
        result
    }

    /// Clear cached bank metadata after the user requests a library refresh.
    pub fn clear_inspection_cache() {
        if let Some(cache) = INSPECTION_CACHE.get() {
            if let Ok(mut cache) = cache.lock() {
                cache.clear();
            }
        }
    }

    fn inspect_uncached(root: &Path) -> Result<Self, String> {
        if !root.is_dir() {
            return Err(format!(
                "DiffSinger bank directory not found: {}",
                root.display()
            ));
        }
        let character_path = root.join("character.yaml");
        let v17_config = root.join("configs/dsconfig.yaml");
        let classic_config = root.join("dsconfig.yaml");
        if !character_path.is_file() || (!classic_config.is_file() && !v17_config.is_file()) {
            return Err("DiffSinger requires character.yaml and dsconfig.yaml".into());
        }
        let character = parse_yaml(&character_path)?;
        let layout = if v17_config.is_file() {
            Layout::V17
        } else {
            Layout::Classic
        };
        let config_path = if layout == Layout::V17 {
            v17_config
        } else {
            classic_config
        };
        let config = parse_yaml(&config_path)?;
        let config_base = if layout == Layout::V17 {
            root.join("configs")
        } else {
            root.to_path_buf()
        };
        let acoustic = string(&config, "acoustic").map(|s| resolve(&config_base, s));
        let vocab = string(&config, "phonemes").map(|s| resolve(&config_base, s));
        let languages_file = string(&config, "languages").map(|s| resolve(&config_base, s));
        let vocoder_dir = string(&config, "vocoder").map(|s| resolve(&config_base, s));
        let vocoder_config = vocoder_dir.as_ref().map(|p| p.join("vocoder.yaml"));
        let vocoder_model = vocoder_config.as_ref().and_then(|p| {
            parse_yaml(p).ok().and_then(|value| {
                string(&value, "model").map(|s| resolve(p.parent().unwrap_or(root), s))
            })
        });
        let phonemes = vocab
            .as_ref()
            .and_then(|p| read_id_map(p).ok())
            .unwrap_or_default();
        let language_ids = languages_file
            .as_ref()
            .and_then(|p| read_id_map(p).ok())
            .unwrap_or_default();
        let mut required = vec![
            ("acoustic model", acoustic.as_ref()),
            ("phoneme vocabulary", vocab.as_ref()),
            ("language vocabulary", languages_file.as_ref()),
            ("vocoder configuration", vocoder_config.as_ref()),
            ("vocoder model", vocoder_model.as_ref()),
        ];
        // Newer banks may keep model paths and vocoder settings in root-level
        // companion configs; the main acoustic and vocoder paths remain the
        // loadability gate for the first supported contract.
        let missing: Vec<String> = required
            .drain(..)
            .filter_map(|(label, path)| {
                path.filter(|p| !p.is_file())
                    .map(|p| format!("{label} missing: {}", p.display()))
            })
            .collect();
        let languages = languages_file
            .as_ref()
            .and_then(|p| fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
            .map(|v| json_tokens(&v))
            .unwrap_or_default();
        let speaker_embeddings: BTreeMap<String, PathBuf> = config
            .get("speakers")
            .and_then(|v| v.as_sequence())
            .map(|xs| {
                xs.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| {
                        let base = resolve(&config_base, s);
                        let embedding = if base.is_file() {
                            base
                        } else {
                            base.with_extension("emb")
                        };
                        (speaker_name(s), embedding)
                    })
                    .collect()
            })
            .unwrap_or_default();
        let speakers: Vec<String> = config
            .get("speakers")
            .and_then(|v| v.as_sequence())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(speaker_name)
                    .collect()
            })
            .unwrap_or_else(|| speaker_embeddings.keys().cloned().collect());
        let dictionaries = collect_dictionaries(root, &config_base);
        let mut dictionary_symbols = BTreeMap::new();
        let mut phone_types = BTreeMap::new();
        let name = string(&character, "name")
            .map(str::to_owned)
            .unwrap_or_else(|| {
                root.file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("DiffSinger")
                    .to_owned()
            });
        let author = string(&character, "author").map(str::to_owned);
        let mut diagnostics = missing;
        if phonemes.is_empty() {
            diagnostics.push("phoneme vocabulary missing, invalid, or empty".into());
        }
        if language_ids.is_empty() {
            diagnostics.push("language vocabulary missing, invalid, or empty".into());
        }
        for (lang, path) in &dictionaries {
            if !path.is_file() {
                diagnostics.push(format!("dictionary for {lang} missing: {}", path.display()));
            } else {
                match read_dictionary_symbols(path) {
                    Ok(symbols) if !symbols.is_empty() => {
                        dictionary_symbols.insert(lang.clone(), symbols);
                        if let Ok(types) = read_dictionary_types(path) {
                            phone_types.extend(types);
                        }
                    }
                    Ok(_) => diagnostics.push(format!(
                        "dictionary for {lang} has no symbols: {}",
                        path.display()
                    )),
                    Err(error) => diagnostics.push(error),
                }
            }
        }
        for (speaker, path) in &speaker_embeddings {
            if !path.is_file() {
                diagnostics.push(format!(
                    "speaker embedding '{speaker}' missing: {}",
                    path.display()
                ));
            }
        }
        let predictor_models = collect_predictors(root, &config_base, &config);
        let duration_config_path = [
            config_base.join("dsdur/dsconfig.yaml"),
            root.join("dsdur/dsconfig.yaml"),
        ].into_iter().find(|path| path.is_file());
        let duration_config = duration_config_path.as_deref().and_then(|p| parse_yaml(p).ok());
        let duration_speaker_embeddings = duration_config.as_ref()
            .and_then(|cfg| cfg.get("speakers").and_then(|v| v.as_sequence()))
            .map(|items| items.iter().filter_map(|v| v.as_str()).map(|s| {
                let base = resolve(duration_config_path.as_deref().unwrap().parent().unwrap_or(root), s);
                let path = if base.is_file() { base } else { base.with_extension("emb") };
                (speaker_name(s), path)
            }).collect()).unwrap_or_default();
        let predict_dur = duration_config.as_ref().and_then(|v| v.get("predict_dur").and_then(|x| x.as_bool())).unwrap_or(false);
        let duration_uses_speakers = duration_config.as_ref().is_some_and(|v| v.get("speakers").and_then(|x| x.as_sequence()).is_some());
        for (name, path) in &predictor_models {
            if !path.is_file() {
                diagnostics.push(format!(
                    "predictor model '{name}' missing: {}",
                    path.display()
                ));
            } else if !looks_like_onnx(path) {
                diagnostics.push(format!(
                    "invalid predictor ONNX model '{name}': {}",
                    path.display()
                ));
            }
        }
        if speakers.is_empty() {
            diagnostics.push("no speakers declared in dsconfig.yaml".into());
        }
        if languages.is_empty() {
            diagnostics.push("language vocabulary missing or unreadable".into());
        }
        if dictionaries.is_empty() {
            diagnostics.push("no language dictionaries found".into());
        }
        // Parsing and path checks are not evidence that ONNX graphs can execute.
        if let Some(path) = acoustic.as_ref().filter(|p| p.is_file()) {
            if !looks_like_onnx(path) {
                diagnostics.push(format!("invalid acoustic ONNX model: {}", path.display()));
            }
        }
        if let Some(path) = vocoder_model.as_ref().filter(|p| p.is_file()) {
            if !looks_like_onnx(path) {
                diagnostics.push(format!("invalid vocoder ONNX model: {}", path.display()));
            }
        }
        let usable = diagnostics.is_empty();
        let int = |key: &str, default: u64| {
            config.get(key).and_then(|v| v.as_u64()).unwrap_or(default) as usize
        };
        Ok(Self {
            root: root.to_path_buf(),
            layout,
            name,
            author,
            languages,
            speakers,
            dictionaries,
            dictionary_symbols,
            phone_types,
            phonemes,
            language_ids,
            speaker_embeddings,
            predictor_models,
            duration_speaker_embeddings,
            predict_dur,
            duration_uses_speakers,
            sample_rate: int("sample_rate", 44_100) as u32,
            hop_size: int("hop_size", 512),
            hidden_size: int("hidden_size", 256),
            mel_bins: int("num_mel_bins", 128),
            use_lang_id: config
                .get("use_lang_id")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            use_key_shift_embed: config
                .get("use_key_shift_embed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            use_speed_embed: config
                .get("use_speed_embed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            use_continuous_acceleration: config
                .get("use_continuous_acceleration")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            use_variable_depth: config
                .get("use_variable_depth")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            acoustic_model: acoustic,
            vocoder_model,
            loadable: usable,
            diagnostic: (!usable).then(|| diagnostics.join("; ")),
        })
    }

    /// Parse a lyric token in the bank's phone vocabulary. Explicit phones
    /// may carry a language prefix (for example `pt/a` or `en/eh`).
    pub fn validate_phone<'a>(&self, token: &'a str) -> Result<&'a str, String> {
        let phone = token.trim();
        if self.phonemes.contains_key(phone) {
            let (language, symbol) = phone.split_once('/').unwrap_or(("", phone));
            let dictionary = self.dictionary_symbols.get(language).or_else(|| {
                (language == "pt")
                    .then(|| self.dictionary_symbols.get("brapa"))
                    .flatten()
            });
            if dictionary
                .is_some_and(|symbols| !symbols.contains(phone) && !symbols.contains(symbol))
            {
                return Err(format!(
                    "phone '{phone}' is not defined by this DiffSinger dictionary"
                ));
            }
            return Ok(phone);
        }
        Err(format!(
            "phone '{phone}' is not present in the DiffSinger vocabulary"
        ))
    }

    pub fn validate_language(&self, language: &str) -> Result<i64, String> {
        self.language_ids
            .get(language)
            .copied()
            .ok_or_else(|| format!("language '{language}' is not declared by this DiffSinger bank"))
    }

    pub fn validate_speaker(&self, speaker: &str) -> Result<&Path, String> {
        self.speaker_embeddings
            .get(speaker)
            .map(PathBuf::as_path)
            .ok_or_else(|| format!("speaker '{speaker}' is not declared by this DiffSinger bank"))
    }
}

fn read_dictionary_symbols(path: &Path) -> Result<BTreeSet<String>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    if path.extension().and_then(|ext| ext.to_str()) == Some("txt") {
        return Ok(text
            .lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    None
                } else {
                    line.split_whitespace().next().map(str::to_owned)
                }
            })
            .collect());
    }
    // These G2P lexicons legitimately contain repeated `entries` keys for
    // alternate pronunciations; parse only the symbol declarations we need.
    Ok(text
        .lines()
        .filter_map(|line| {
            let (_, value) = line.split_once("symbol:")?;
            let value = value
                .split([',', '}'])
                .next()?
                .trim()
                .trim_matches(['\'', '"']);
            (!value.is_empty()).then(|| value.to_owned())
        })
        .collect())
}

fn read_dictionary_types(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(text.lines().filter_map(|line| {
        let symbol = line.split_once("symbol:")?.1.split([',', '}']).next()?.trim().trim_matches(['\'', '"']);
        let kind = line.split_once("type:")?.1.split([',', '}']).next()?.trim().trim_matches(['\'', '"']);
        (!symbol.is_empty() && !kind.is_empty()).then(|| (symbol.to_owned(), kind.to_owned()))
    }).collect())
}

fn speaker_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(path)
        .to_owned()
}

fn read_id_map(path: &Path) -> Result<BTreeMap<String, i64>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("invalid vocabulary {}: {e}", path.display()))?;
    let object = value
        .as_object()
        .ok_or_else(|| format!("vocabulary must be a JSON object: {}", path.display()))?;
    object
        .iter()
        .map(|(key, value)| {
            value
                .as_i64()
                .map(|id| (key.clone(), id))
                .ok_or_else(|| format!("invalid ID for '{key}' in {}", path.display()))
        })
        .collect()
}

fn collect_predictors(
    root: &Path,
    config_base: &Path,
    config: &yaml_serde::Value,
) -> BTreeMap<String, PathBuf> {
    let mut out = BTreeMap::new();
    for key in [
        "dur",
        "linguistic_dur",
        "pitch",
        "linguistic_pitch",
        "variance",
        "linguistic_variance",
    ] {
        if let Some(path) = string(config, key) {
            out.insert(key.to_owned(), resolve(config_base, path));
        }
    }
    for name in ["dsdur", "dspitch", "dsvariance"] {
        let path = [
            config_base.join(name).join("dsconfig.yaml"),
            root.join(name).join("dsconfig.yaml"),
        ]
        .into_iter()
        .find(|path| path.is_file())
        .unwrap_or_else(|| config_base.join(name).join("dsconfig.yaml"));
        if let Ok(cfg) = parse_yaml(&path) {
            for (key, value) in cfg.as_mapping().into_iter().flatten() {
                if let (Some(key), Some(rel)) = (key.as_str(), value.as_str()) {
                    if ["dur", "pitch", "variance", "linguistic"].contains(&key) {
                        out.insert(
                            format!("{name}.{key}"),
                            resolve(path.parent().unwrap_or(root), rel),
                        );
                    }
                }
            }
        }
    }
    out
}

fn looks_like_onnx(path: &Path) -> bool {
    let Ok(metadata) = fs::metadata(path) else {
        return false;
    };
    if metadata.len() <= 16 {
        return false;
    }
    let Ok(mut file) = fs::File::open(path) else {
        return false;
    };
    let mut first_byte = [0_u8; 1];
    file.read_exact(&mut first_byte).is_ok() && first_byte[0] == 0x08
}

fn parse_yaml(path: &Path) -> Result<yaml_serde::Value, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    yaml_serde::from_str(text.trim_start_matches('\u{feff}'))
        .map_err(|e| format!("invalid YAML in {}: {e}", path.display()))
}
fn string<'a>(value: &'a yaml_serde::Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(|v| v.as_str())
}
fn resolve(base: &Path, rel: &str) -> PathBuf {
    let path = Path::new(rel);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}
fn json_tokens(value: &serde_json::Value) -> Vec<String> {
    match value {
        serde_json::Value::Array(items) => items
            .iter()
            .filter_map(|x| x.as_str().map(str::to_owned))
            .collect(),
        serde_json::Value::Object(map) => map.keys().cloned().collect(),
        _ => Vec::new(),
    }
}
fn collect_dictionaries(root: &Path, config_base: &Path) -> BTreeMap<String, PathBuf> {
    fn walk(dir: &Path, root: &Path, out: &mut BTreeMap<String, PathBuf>, depth: usize) {
        if depth > 4 {
            return;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, root, out, depth + 1);
                continue;
            }
            let Some(file) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            if file.starts_with("dsdict-") && file.ends_with(".yaml") {
                let lang = file
                    .trim_start_matches("dsdict-")
                    .trim_end_matches(".yaml")
                    .to_owned();
                out.insert(lang, path);
            } else if file.starts_with("dictionary-") && file.ends_with(".txt") {
                let lang = file
                    .trim_start_matches("dictionary-")
                    .trim_end_matches(".txt")
                    .to_owned();
                out.insert(lang, path);
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out, 0);
    // v1.7 config dictionaries are relative paths; include these even if they
    // live outside the configs subtree.
    if let Ok(config) = parse_yaml(&config_base.join("dsconfig.yaml")) {
        if let Some(map) = config.get("dictionaries").and_then(|v| v.as_mapping()) {
            for (key, value) in map {
                if let (Some(lang), Some(rel)) = (key.as_str(), value.as_str()) {
                    let direct = resolve(config_base, rel);
                    let candidates = [
                        direct.clone(),
                        resolve(&config_base.join("dsdur"), rel),
                        config_base
                            .parent()
                            .map(|parent| resolve(parent, rel))
                            .unwrap_or(direct.clone()),
                    ];
                    out.insert(
                        lang.to_owned(),
                        candidates
                            .into_iter()
                            .find(|path| path.is_file())
                            .unwrap_or(direct),
                    );
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn reports_layouts_languages_speakers_and_relative_paths() {
        for path in [
            "/Users/xiao/Library/OpenUtau/Singers/Evelyn_DS_v100",
            "/Users/xiao/Library/OpenUtau/Singers/MIRACLE",
            "/Users/xiao/Library/OpenUtau/Singers/SAiFA_v170",
            "/Users/xiao/Library/OpenUtau/Singers/configs",
        ] {
            if Path::new(path).exists() {
                let bank = DiffSingerBank::inspect(path).unwrap();
                assert!(!bank.speakers.is_empty(), "{}", bank.name);
                assert!(!bank.languages.is_empty(), "{}", bank.name);
                assert!(!bank.dictionaries.is_empty(), "{}", bank.name);
                assert!(bank.loadable, "{}: {:?}", bank.name, bank.diagnostic);
                assert!(bank.phonemes.len() > 10, "{}", bank.name);
                assert!(bank.validate_phone("pt/a").is_ok(), "{}", bank.name);
                assert!(bank.validate_phone("zz/not-a-phone").is_err());
                let speaker = bank.speakers.first().unwrap();
                assert!(bank.validate_speaker(speaker).is_ok());
                assert!(bank.validate_speaker("missing-speaker").is_err());
                assert!(bank.diagnostic.is_none());
            }
        }
    }

    #[test]
    fn malformed_config_is_reported() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("character.yaml"), "name: test\n").unwrap();
        fs::write(dir.path().join("dsconfig.yaml"), "[broken\n").unwrap();
        assert!(DiffSingerBank::inspect(dir.path())
            .unwrap_err()
            .contains("invalid YAML"));
    }

    #[test]
    fn inspection_cache_is_reused_until_a_library_refresh() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join("vocoder")).unwrap();
        fs::create_dir_all(root.join("embeds")).unwrap();
        fs::write(root.join("character.yaml"), "name: cached\n").unwrap();
        fs::write(
            root.join("dsconfig.yaml"),
            "phonemes: phones.json\nlanguages: languages.json\nacoustic: acoustic.onnx\nvocoder: vocoder\nspeakers:\n- embeds/test\n",
        )
        .unwrap();
        fs::write(root.join("phones.json"), r#"{"pt/a": 1}"#).unwrap();
        fs::write(root.join("languages.json"), r#"{"pt": 1}"#).unwrap();
        fs::write(
            root.join("acoustic.onnx"),
            [vec![0x08], vec![0; 32]].concat(),
        )
        .unwrap();
        fs::write(root.join("vocoder/vocoder.yaml"), "model: model.onnx\n").unwrap();
        fs::write(
            root.join("vocoder/model.onnx"),
            [vec![0x08], vec![0; 32]].concat(),
        )
        .unwrap();
        fs::write(root.join("embeds/test.emb"), [0; 16]).unwrap();
        fs::write(root.join("dsdict-pt.yaml"), "symbols:\n- {symbol: pt/a}\n").unwrap();

        let bank = DiffSingerBank::inspect(root).unwrap();
        assert!(bank.loadable, "{:?}", bank.diagnostic);
        fs::rename(root.join("character.yaml"), root.join("character.yaml.off")).unwrap();
        assert_eq!(DiffSingerBank::inspect(root).unwrap().name, "cached");
        DiffSingerBank::clear_inspection_cache();
        assert!(DiffSingerBank::inspect(root).is_err());
    }

    #[test]
    fn explicit_phones_and_language_ids_are_checked_against_bank_vocabularies() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("models")).unwrap();
        fs::create_dir_all(dir.path().join("speaker")).unwrap();
        fs::write(dir.path().join("character.yaml"), "name: fixture\n").unwrap();
        fs::write(dir.path().join("dsconfig.yaml"), "phonemes: phones.json\nlanguages: languages.json\nacoustic: acoustic.onnx\nvocoder: vocoder\nspeakers:\n- speaker\n").unwrap();
        fs::write(dir.path().join("phones.json"), r#"{"pt/a": 1, "en/eh": 2}"#).unwrap();
        fs::write(dir.path().join("languages.json"), r#"{"pt": 1, "en": 2}"#).unwrap();
        fs::write(dir.path().join("acoustic.onnx"), "placeholder").unwrap();
        fs::create_dir_all(dir.path().join("vocoder")).unwrap();
        fs::write(
            dir.path().join("vocoder/vocoder.yaml"),
            "model: model.onnx\n",
        )
        .unwrap();
        fs::write(dir.path().join("vocoder/model.onnx"), "placeholder").unwrap();
        let bank = DiffSingerBank::inspect(dir.path()).unwrap();
        assert_eq!(bank.validate_phone("pt/a").unwrap(), "pt/a");
        assert_eq!(bank.validate_phone("en/eh").unwrap(), "en/eh");
        assert!(bank.validate_phone("pt/unknown").is_err());
        assert_eq!(bank.validate_language("en").unwrap(), 2);
        assert!(bank.validate_language("ja").is_err());
        assert_eq!(
            bank.validate_speaker("speaker").unwrap(),
            dir.path().join("speaker.emb")
        );
    }
}
