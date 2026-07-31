pub mod gui;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

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
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopaibaConfig {
    pub version: String,
    pub voicebank_name: String,
    pub author: String,
    pub entries: Vec<CopaibaEntry>,
}

impl Default for CopaibaConfig {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            voicebank_name: "Novo Voicebank".to_string(),
            author: "Desconhecido".to_string(),
            entries: Vec::new(),
        }
    }
}

impl CopaibaConfig {
    pub fn load_from_dir(dir: &Path) -> Result<Self, String> {
        let config_path = dir.join("copaiba.config");
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .map_err(|e| format!("Falha ao ler copaiba.config: {}", e))?;
            let config: CopaibaConfig = serde_json::from_str(&content)
                .map_err(|e| format!("Falha ao decodificar JSON do copaiba.config: {}", e))?;
            Ok(config)
        } else {
            let mut config = Self::default();
            // Try loading from oto.ini if present
            let oto_path = dir.join("oto.ini");
            if oto_path.exists() {
                if let Ok(content) = fs::read_to_string(&oto_path) {
                    for line in content.lines() {
                        let line = line.trim();
                        if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                            continue;
                        }
                        if let Some((file, params_str)) = line.split_once('=') {
                            let parts: Vec<&str> = params_str.split(',').collect();
                            let alias = parts.get(0).unwrap_or(&file).to_string();
                            let offset: f64 = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0.0);
                            let consonant: f64 = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(50.0);
                            let cutoff: f64 = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(0.0);

                            config.entries.push(CopaibaEntry {
                                wav_filename: file.to_string(),
                                alias: if alias.is_empty() { file.to_string() } else { alias },
                                corte_inicial_ms: offset,
                                consoante_ms: consonant,
                                loop_inicio_ms: Some(offset + consonant),
                                loop_fim_ms: Some(offset + consonant + 200.0),
                                cauda_final_ms: None,
                                corte_final_ms: cutoff,
                            });
                        }
                    }
                }
            }

            if config.entries.is_empty() {
                if let Ok(read_dir) = fs::read_dir(dir) {
                    for entry in read_dir.flatten() {
                        let path = entry.path();
                        if path.is_file() && path.extension().map_or(false, |ext| ext.eq_ignore_ascii_case("wav")) {
                            if let Some(name_str) = path.file_name().and_then(|n| n.to_str()) {
                                let alias = path.file_stem().and_then(|s| s.to_str()).unwrap_or(name_str).to_string();
                                config.entries.push(CopaibaEntry {
                                    wav_filename: name_str.to_string(),
                                    alias,
                                    corte_inicial_ms: 10.0,
                                    consoante_ms: 60.0,
                                    loop_inicio_ms: Some(100.0),
                                    loop_fim_ms: Some(300.0),
                                    cauda_final_ms: None,
                                    corte_final_ms: 0.0,
                                });
                            }
                        }
                    }
                }
            }

            Ok(config)
        }
    }

    pub fn save_to_dir(&self, dir: &Path) -> Result<(), String> {
        let config_path = dir.join("copaiba.config");
        let json_content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Falha ao serializar copaiba.config: {}", e))?;
        fs::write(&config_path, json_content)
            .map_err(|e| format!("Falha ao salvar copaiba.config: {}", e))?;
        
        self.export_oto_ini(dir)?;
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
                entry.corte_inicial_ms + (entry.consoante_ms * 0.5),
                entry.corte_inicial_ms * 0.2
            );
            lines.push(line);
        }
        let content = lines.join("\r\n");
        fs::write(oto_path, content).map_err(|e| format!("Falha ao exportar oto.ini: {}", e))?;
        Ok(())
    }
}
