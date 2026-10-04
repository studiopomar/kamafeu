//! Metadata and asset discovery for OpenUtau DiffSinger voicebanks.
//!
//! DiffSinger banks are phrase-level neural singers.  They deliberately do
//! not use `oto.ini`; this module keeps their files and configuration
//! separate from the classic UTAU voicebank path so the renderer can select
//! the correct engine.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffSingerConfig {
    pub root: PathBuf,
    pub phonemes: PathBuf,
    pub acoustic: PathBuf,
    pub vocoder: PathBuf,
    pub languages: Option<PathBuf>,
    pub language_ids: HashMap<String, i64>,
    pub speakers: Vec<String>,
    pub hidden_size: Option<usize>,
    pub sample_rate: u32,
    pub vocoder_sample_rate: u32,
    pub hop_size: u32,
    pub num_mel_bins: u32,
    pub mel_base: String,
}

impl DiffSingerConfig {
    pub fn load(root: &Path) -> Result<Self, String> {
        let config_path = root.join("dsconfig.yaml");
        let content = fs::read_to_string(&config_path)
            .map_err(|error| format!("não foi possível ler {}: {error}", config_path.display()))?;
        let value: yaml_serde::Value = yaml_serde::from_str(&content)
            .map_err(|error| format!("dsconfig.yaml inválido: {error}"))?;

        let path_value = |key: &str, default: Option<&str>| -> Result<PathBuf, String> {
            let value = value
                .get(key)
                .and_then(|item| item.as_str())
                .or(default)
                .ok_or_else(|| format!("dsconfig.yaml não contém o campo '{key}'"))?;
            let path = Path::new(value);
            if !is_safe_relative_path(path) {
                return Err(format!("caminho inválido no campo '{key}': {value}"));
            }
            let resolved = root.join(path);
            if !resolved.is_file() {
                return Err(format!(
                    "arquivo de DiffSinger não encontrado: {}",
                    resolved.display()
                ));
            }
            Ok(resolved)
        };

        let phonemes = path_value("phonemes", Some("phonemes.txt"))?;
        let acoustic = path_value("acoustic", Some("acoustic.onnx"))?;
        // Some OpenUtau banks name a shared dependency (for example
        // `nsf-hifigan`) instead of pointing to a local ONNX file. Prefer a
        // bank-local dsvocoder/model.onnx when present and preserve the
        // dependency name as an error until the dependency resolver exists.
        let vocoder_value = value
            .get("vocoder")
            .and_then(|item| item.as_str())
            .ok_or_else(|| "dsconfig.yaml não contém o campo 'vocoder'".to_string())?;
        if !is_safe_relative_path(Path::new(vocoder_value)) {
            return Err(format!(
                "caminho inválido no campo 'vocoder': {vocoder_value}"
            ));
        }
        let vocoder_path = root.join(vocoder_value);
        let mut vocoder_sample_rate = 44_100;
        let local_vocoder = if vocoder_path.is_file() {
            Some(vocoder_path)
        } else {
            let yaml_path = root.join("dsvocoder/vocoder.yaml");
            if yaml_path.is_file() {
                resolve_vocoder_yaml(&yaml_path, &mut vocoder_sample_rate)?
            } else {
                let mut candidates = vec![
                    root.join("dsvocoder/model.onnx"),
                    root.join("dsvocoder/vocoder.onnx"),
                ];
                if let Some(parent) = root.parent() {
                    candidates.push(parent.join("Dependencies").join(vocoder_value));
                }
                if let Ok(cwd) = std::env::current_dir() {
                    candidates.push(cwd.join("Dependencies").join(vocoder_value));
                }
                if let Ok(dependencies) = std::env::var("KAMAFEU_DEPENDENCIES") {
                    candidates.push(PathBuf::from(dependencies).join(vocoder_value));
                }
                let mut resolved = None;
                for candidate in candidates {
                    if candidate.is_dir() {
                        let yaml_path = candidate.join("vocoder.yaml");
                        if yaml_path.is_file() {
                            resolved = resolve_vocoder_yaml(&yaml_path, &mut vocoder_sample_rate)?;
                            if resolved.is_some() {
                                break;
                            }
                        }
                    } else if candidate.is_file() {
                        resolved = Some(candidate);
                        break;
                    }
                }
                resolved
            }
        };
        let vocoder = local_vocoder.ok_or_else(|| {
            format!(
                "vocoder DiffSinger não encontrado localmente ({vocoder_value}); dependências compartilhadas ainda não são resolvidas"
            )
        })?;
        let languages = value
            .get("languages")
            .and_then(|item| item.as_str())
            .map(|path| {
                let path = Path::new(path);
                if !is_safe_relative_path(path) {
                    return Err(format!("caminho inválido no campo 'languages': {path:?}"));
                }
                let resolved = root.join(path);
                if !resolved.is_file() {
                    return Err(format!(
                        "arquivo de idiomas não encontrado: {}",
                        resolved.display()
                    ));
                }
                Ok(resolved)
            })
            .transpose()?;
        let language_ids = if let Some(path) = &languages {
            let content = fs::read_to_string(path)
                .map_err(|error| format!("não foi possível ler {}: {error}", path.display()))?;
            serde_json::from_str(&content)
                .map_err(|error| format!("arquivo de idiomas DiffSinger inválido: {error}"))?
        } else {
            HashMap::new()
        };
        let speakers: Vec<String> = value
            .get("speakers")
            .and_then(|items| items.as_sequence())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default();

        let number = |key: &str, default: u32| {
            value
                .get(key)
                .and_then(|item| item.as_i64())
                .map(|value| value.max(1) as u32)
                .unwrap_or(default)
        };
        let mel_base = value
            .get("mel_base")
            .and_then(|item| item.as_str())
            .unwrap_or("10")
            .to_string();
        let hidden_size = value
            .get("hiddenSize")
            .or_else(|| value.get("hidden_size"))
            .and_then(|item| item.as_i64())
            .map(|value| value.max(1) as usize)
            .or_else(|| (!speakers.is_empty()).then_some(256));

        Ok(Self {
            root: root.to_path_buf(),
            phonemes,
            acoustic,
            vocoder,
            languages,
            language_ids,
            speakers,
            hidden_size,
            sample_rate: number("sample_rate", 44_100),
            vocoder_sample_rate,
            hop_size: number("hop_size", 512),
            num_mel_bins: number("num_mel_bins", 128),
            mel_base,
        })
    }
}

fn is_safe_relative_path(path: &Path) -> bool {
    !path.is_absolute()
        && !path
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
}

fn resolve_vocoder_yaml(
    yaml_path: &Path,
    sample_rate: &mut u32,
) -> Result<Option<PathBuf>, String> {
    let yaml = fs::read_to_string(yaml_path)
        .map_err(|error| format!("não foi possível ler {}: {error}", yaml_path.display()))?;
    let yaml_value: yaml_serde::Value =
        yaml_serde::from_str(&yaml).map_err(|error| format!("vocoder.yaml inválido: {error}"))?;
    let model = yaml_value
        .get("model")
        .and_then(|item| item.as_str())
        .unwrap_or("model.onnx");
    *sample_rate = yaml_value
        .get("sample_rate")
        .and_then(|item| item.as_i64())
        .map(|value| value.max(1) as u32)
        .unwrap_or(44_100);
    let model_path = Path::new(model);
    if !is_safe_relative_path(model_path) {
        return Err(format!("caminho inválido no campo 'model': {model}"));
    }
    let resolved = yaml_path
        .parent()
        .ok_or_else(|| format!("diretório inválido para {}", yaml_path.display()))?
        .join(model_path);
    Ok(resolved.is_file().then_some(resolved))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_required_diffsinger_assets() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("phonemes.txt"), "a 1\n").unwrap();
        fs::write(dir.path().join("acoustic.onnx"), b"model").unwrap();
        fs::write(dir.path().join("vocoder.onnx"), b"model").unwrap();
        fs::write(
            dir.path().join("dsconfig.yaml"),
            "phonemes: phonemes.txt\nacoustic: acoustic.onnx\nvocoder: vocoder.onnx\nspeakers: [main]\n",
        )
        .unwrap();

        let config = DiffSingerConfig::load(dir.path()).unwrap();
        assert_eq!(config.speakers, vec!["main"]);
        assert_eq!(config.hidden_size, Some(256));
        assert_eq!(config.acoustic, dir.path().join("acoustic.onnx"));
        assert_eq!(config.sample_rate, 44_100);
        assert_eq!(config.hop_size, 512);
        assert_eq!(config.num_mel_bins, 128);
    }

    #[test]
    fn reads_diffsinger_hidden_size_alias() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("phonemes.txt"), "SP\n").unwrap();
        fs::write(dir.path().join("acoustic.onnx"), b"model").unwrap();
        fs::write(dir.path().join("vocoder.onnx"), b"model").unwrap();
        fs::write(
            dir.path().join("dsconfig.yaml"),
            "acoustic: acoustic.onnx\nvocoder: vocoder.onnx\nhiddenSize: 256\n",
        )
        .unwrap();
        let config = DiffSingerConfig::load(dir.path()).unwrap();
        assert_eq!(config.hidden_size, Some(256));
    }

    #[test]
    fn resolves_local_dsvocoder_yaml_model() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("dsvocoder")).unwrap();
        fs::write(dir.path().join("phonemes.txt"), "SP\n").unwrap();
        fs::write(dir.path().join("acoustic.onnx"), b"model").unwrap();
        fs::write(dir.path().join("dsvocoder/model.onnx"), b"model").unwrap();
        fs::write(
            dir.path().join("dsvocoder/vocoder.yaml"),
            "model: model.onnx\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("dsconfig.yaml"),
            "acoustic: acoustic.onnx\nvocoder: nsf_hifigan\n",
        )
        .unwrap();
        let config = DiffSingerConfig::load(dir.path()).unwrap();
        assert_eq!(config.vocoder, dir.path().join("dsvocoder/model.onnx"));
    }

    #[test]
    fn rejects_parent_paths() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("dsconfig.yaml"),
            "phonemes: ../phones.txt\nacoustic: ../model.onnx\nvocoder: ../vocoder.onnx\n",
        )
        .unwrap();
        let error = DiffSingerConfig::load(dir.path()).unwrap_err();
        assert!(error.contains("caminho inválido"));
    }
}
