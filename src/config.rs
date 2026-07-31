use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct KamafeuConfig {
    pub last_voicebank: Option<PathBuf>,
    #[serde(default)]
    pub recent_voicebanks: Vec<PathBuf>,
}

impl KamafeuConfig {
    pub fn config_path() -> PathBuf {
        if let Ok(home) = std::env::var("HOME") {
            let dir = PathBuf::from(home).join(".config").join("kamafeu");
            let _ = fs::create_dir_all(&dir);
            dir.join("kamafeu_config.json")
        } else {
            PathBuf::from("kamafeu_config.json")
        }
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(cfg) = serde_json::from_str::<KamafeuConfig>(&content) {
                    return cfg;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, content);
        }
    }

    pub fn add_recent_voicebank(&mut self, path: PathBuf) {
        if !path.exists() {
            return;
        }
        // Remove duplicate entry if present to move to front
        self.recent_voicebanks.retain(|p| p != &path);
        self.recent_voicebanks.insert(0, path.clone());
        self.recent_voicebanks.truncate(10);
        self.last_voicebank = Some(path);
        self.save();
    }
}
