use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

fn default_true() -> bool {
    true
}

fn default_scale() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KamafeuConfig {
    pub last_voicebank: Option<PathBuf>,
    #[serde(default)]
    pub recent_voicebanks: Vec<PathBuf>,
    #[serde(default)]
    pub recent_projects: Vec<PathBuf>,
    #[serde(default)]
    pub singers_paths: Vec<PathBuf>,
    #[serde(default = "default_true")]
    pub discord_rpc_enabled: bool,
    #[serde(default = "default_scale")]
    pub ui_scale_factor: f32,
}

impl Default for KamafeuConfig {
    fn default() -> Self {
        let mut default_singers = crate::oto::SingerScanner::default_singers_directories();
        default_singers.dedup();
        Self {
            last_voicebank: None,
            recent_voicebanks: Vec::new(),
            recent_projects: Vec::new(),
            singers_paths: default_singers,
            discord_rpc_enabled: true,
            ui_scale_factor: 1.0,
        }
    }
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

    pub fn load() -> Result<Self, String> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path)
            .map_err(|e| format!("Falha ao ler {}: {e}", path.display()))?;
        serde_json::from_str::<KamafeuConfig>(&content)
            .map_err(|e| format!("Configuração inválida em {}: {e}", path.display()))
    }

    pub fn save(&self) -> Result<(), String> {
        let path = Self::config_path();
        let content = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Falha ao serializar configuração: {e}"))?;
        let parent = path
            .parent()
            .ok_or_else(|| "Caminho de configuração inválido".to_string())?;
        fs::create_dir_all(parent)
            .map_err(|e| format!("Falha ao criar {}: {e}", parent.display()))?;
        let mut temporary = tempfile::NamedTempFile::new_in(parent)
            .map_err(|e| format!("Falha ao criar arquivo temporário: {e}"))?;
        temporary
            .write_all(content.as_bytes())
            .and_then(|()| temporary.as_file().sync_all())
            .map_err(|e| format!("Falha ao gravar configuração: {e}"))?;
        temporary
            .persist(&path)
            .map_err(|e| format!("Falha ao substituir {}: {}", path.display(), e.error))?;
        Ok(())
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
        if let Err(error) = self.save() {
            eprintln!("[Kamafeu] {error}");
        }
    }

    pub fn add_recent_project(&mut self, path: PathBuf) {
        if !path.exists() {
            return;
        }
        self.recent_projects.retain(|p| p != &path);
        self.recent_projects.insert(0, path);
        self.recent_projects.truncate(10);
        if let Err(error) = self.save() {
            eprintln!("[Kamafeu] {error}");
        }
    }
}
