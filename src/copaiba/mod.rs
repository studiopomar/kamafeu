pub mod gui;

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;

fn atomic_write(path: &Path, content: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Caminho sem diretório pai".to_string())?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)
        .map_err(|e| format!("Falha ao criar arquivo temporário: {e}"))?;
    temporary
        .write_all(content)
        .and_then(|()| temporary.as_file().sync_all())
        .map_err(|e| format!("Falha ao gravar arquivo temporário: {e}"))?;
    temporary
        .persist(path)
        .map_err(|e| format!("Falha ao substituir {}: {}", path.display(), e.error))?;
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopaibaEntry {
    pub wav_filename: String,
    pub alias: String,
    pub corte_inicial_ms: f64,
    pub consoante_ms: f64,
    #[serde(default)]
    pub loop_inicio_ms: Option<f64>,
    #[serde(default)]
    pub loop_fim_ms: Option<f64>,
    #[serde(default)]
    pub cauda_final_ms: Option<f64>,
    pub corte_final_ms: f64,
    #[serde(default)]
    pub preutterance_ms: f64,
    #[serde(default)]
    pub overlap_ms: f64,
}

impl Default for CopaibaEntry {
    fn default() -> Self {
        Self {
            wav_filename: String::new(),
            alias: String::new(),
            corte_inicial_ms: 0.0,
            consoante_ms: 50.0,
            loop_inicio_ms: None,
            loop_fim_ms: None,
            cauda_final_ms: None,
            corte_final_ms: 0.0,
            preutterance_ms: 30.0,
            overlap_ms: 15.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopaibaConfig {
    pub version: String,
    pub voicebank_name: String,
    pub author: String,
    #[serde(default)]
    pub image_filename: Option<String>,
    pub entries: Vec<CopaibaEntry>,
    #[serde(default)]
    pub prefix_map: std::collections::HashMap<String, (String, String)>,
}

impl Default for CopaibaConfig {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            voicebank_name: "Novo Voicebank".to_string(),
            author: "Desconhecido".to_string(),
            image_filename: None,
            entries: Vec::new(),
            prefix_map: std::collections::HashMap::new(),
        }
    }
}

impl CopaibaConfig {
    pub fn load_from_dir(dir: &Path) -> Result<Self, String> {
        let native_path = dir.join("kamafeu_voicebank.json");
        let config_path = dir.join("copaiba.config");
        let mut config = if native_path.exists() {
            let content = fs::read_to_string(&native_path)
                .map_err(|e| format!("Falha ao ler kamafeu_voicebank.json: {}", e))?;
            serde_json::from_str::<CopaibaConfig>(&content).map_err(|e| {
                format!("Falha ao decodificar JSON do kamafeu_voicebank.json: {}", e)
            })?
        } else if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .map_err(|e| format!("Falha ao ler copaiba.config: {}", e))?;
            let mut cfg = serde_json::from_str::<CopaibaConfig>(&content)
                .map_err(|e| format!("Falha ao decodificar JSON do copaiba.config: {}", e))?;

            for entry in &mut cfg.entries {
                if entry.preutterance_ms == 0.0 {
                    entry.preutterance_ms = 30.0;
                }
                if entry.overlap_ms == 0.0 {
                    entry.overlap_ms = 15.0;
                }
            }

            let pmap_path = dir.join("prefix.map");
            if pmap_path.exists() {
                if let Ok(bytes) = fs::read(&pmap_path) {
                    let (content, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
                    for line in content.lines() {
                        let line = line.trim();
                        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                            continue;
                        }
                        let parts: Vec<&str> = if line.contains('\t') {
                            line.split('\t').collect()
                        } else {
                            line.split_whitespace().collect()
                        };
                        if parts.len() >= 2 {
                            let pitch = parts[0].trim().to_string();
                            let prefix = parts[1]
                                .trim()
                                .trim_matches('"')
                                .trim_matches('\'')
                                .to_string();
                            let suffix = parts
                                .get(2)
                                .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                                .unwrap_or_default();
                            cfg.prefix_map.insert(pitch, (prefix, suffix));
                        }
                    }
                }
            }

            cfg
        } else {
            let mut cfg = Self::default();
            let char_path = dir.join("character.txt");
            if char_path.exists() {
                if let Ok(bytes) = fs::read(&char_path) {
                    let (txt, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
                    for line in txt.lines() {
                        let line = line.trim();
                        if let Some((k, v)) = line.split_once('=') {
                            match k.trim().to_lowercase().as_str() {
                                "name" => cfg.voicebank_name = v.trim().to_string(),
                                "author" => cfg.author = v.trim().to_string(),
                                "image" => cfg.image_filename = Some(v.trim().to_string()),
                                _ => {}
                            }
                        }
                    }
                }
            }

            let pmap_path = dir.join("prefix.map");
            if pmap_path.exists() {
                if let Ok(bytes) = fs::read(&pmap_path) {
                    let (content, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
                    for line in content.lines() {
                        let line = line.trim();
                        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                            continue;
                        }
                        let parts: Vec<&str> = if line.contains('\t') {
                            line.split('\t').collect()
                        } else {
                            line.split_whitespace().collect()
                        };
                        if parts.len() >= 2 {
                            let pitch = parts[0].trim().to_string();
                            let prefix = parts[1]
                                .trim()
                                .trim_matches('"')
                                .trim_matches('\'')
                                .to_string();
                            let suffix = parts
                                .get(2)
                                .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
                                .unwrap_or_default();
                            cfg.prefix_map.insert(pitch, (prefix, suffix));
                        }
                    }
                }
            }

            let oto_path = dir.join("oto.ini");
            if oto_path.exists() {
                if let Ok(bytes) = fs::read(&oto_path) {
                    let (content, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
                    for line in content.lines() {
                        let line = line.trim();
                        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                            continue;
                        }
                        if let Some((file, params_str)) = line.split_once('=') {
                            let parts: Vec<&str> = params_str.split(',').collect();
                            let alias = parts.first().unwrap_or(&file).to_string();
                            let offset: f64 =
                                parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                            let consonant: f64 =
                                parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(50.0);
                            let cutoff: f64 =
                                parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                            let preutterance: f64 =
                                parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(30.0);
                            let overlap: f64 =
                                parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(15.0);

                            cfg.entries.push(CopaibaEntry {
                                wav_filename: file.to_string(),
                                alias: if alias.is_empty() {
                                    file.to_string()
                                } else {
                                    alias
                                },
                                corte_inicial_ms: offset,
                                consoante_ms: consonant,
                                loop_inicio_ms: Some(offset + consonant),
                                loop_fim_ms: Some(offset + consonant + 200.0),
                                cauda_final_ms: None,
                                corte_final_ms: cutoff,
                                preutterance_ms: preutterance,
                                overlap_ms: overlap,
                            });
                        }
                    }
                }
            }

            if cfg.entries.is_empty() {
                if let Ok(read_dir) = fs::read_dir(dir) {
                    for entry in read_dir.flatten() {
                        let path = entry.path();
                        if path.is_file()
                            && path
                                .extension()
                                .is_some_and(|ext| ext.eq_ignore_ascii_case("wav"))
                        {
                            if let Some(name_str) = path.file_name().and_then(|n| n.to_str()) {
                                let alias = path
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or(name_str)
                                    .to_string();
                                cfg.entries.push(CopaibaEntry {
                                    wav_filename: name_str.to_string(),
                                    alias,
                                    corte_inicial_ms: 10.0,
                                    consoante_ms: 60.0,
                                    loop_inicio_ms: Some(100.0),
                                    loop_fim_ms: Some(300.0),
                                    cauda_final_ms: None,
                                    corte_final_ms: 0.0,
                                    preutterance_ms: 30.0,
                                    overlap_ms: 15.0,
                                });
                            }
                        }
                    }
                }
            }

            cfg
        };

        if config.image_filename.is_none() {
            let candidates = [
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
            ];
            for c in candidates {
                if dir.join(c).exists() {
                    config.image_filename = Some(c.to_string());
                    break;
                }
            }
        }

        Ok(config)
    }

    pub fn save_to_dir(&self, dir: &Path) -> Result<(), String> {
        let config_path = dir.join("copaiba.config");
        let json_content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Falha ao serializar copaiba.config: {}", e))?;
        atomic_write(&config_path, json_content.as_bytes())
            .map_err(|e| format!("Falha ao salvar copaiba.config: {}", e))?;

        let native_path = dir.join("kamafeu_voicebank.json");
        let native_json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Falha ao serializar kamafeu_voicebank.json: {}", e))?;
        atomic_write(&native_path, native_json.as_bytes())
            .map_err(|e| format!("Falha ao salvar kamafeu_voicebank.json: {}", e))?;

        self.export_oto_ini(dir)?;
        self.sync_character_txt(dir)?;
        self.export_prefix_map(dir)?;
        Ok(())
    }

    pub fn sync_character_txt(&self, dir: &Path) -> Result<(), String> {
        let char_path = dir.join("character.txt");
        let mut content = format!("name={}\nauthor={}\n", self.voicebank_name, self.author);
        if let Some(ref img) = self.image_filename {
            content.push_str(&format!("image={}\n", img));
        }
        atomic_write(&char_path, content.as_bytes())
            .map_err(|e| format!("Falha ao escrever character.txt: {}", e))?;
        Ok(())
    }

    pub fn export_oto_ini(&self, dir: &Path) -> Result<(), String> {
        let oto_path = dir.join("oto.ini");
        let mut lines = Vec::new();
        for entry in &self.entries {
            let line = format!(
                "{}={},{},{},{},{},{}",
                entry.wav_filename,
                entry.alias,
                entry.corte_inicial_ms,
                entry.consoante_ms,
                entry.corte_final_ms,
                entry.preutterance_ms,
                entry.overlap_ms
            );
            lines.push(line);
        }
        let content = lines.join("\r\n");
        atomic_write(&oto_path, content.as_bytes())
            .map_err(|e| format!("Falha ao exportar oto.ini: {}", e))?;
        Ok(())
    }

    pub fn export_prefix_map(&self, dir: &Path) -> Result<(), String> {
        if self.prefix_map.is_empty() {
            return Ok(());
        }
        let pmap_path = dir.join("prefix.map");
        let mut lines = Vec::new();
        for (pitch, (prefix, suffix)) in &self.prefix_map {
            lines.push(format!("{}\t{}\t{}", pitch, prefix, suffix));
        }
        let content = lines.join("\r\n");
        atomic_write(&pmap_path, content.as_bytes())
            .map_err(|e| format!("Falha ao exportar prefix.map: {}", e))?;
        Ok(())
    }
}
