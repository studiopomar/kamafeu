//! In-process project-format extension point.
//!
//! The registry owns format selection instead of scattering filename matches
//! through the application. Third-party runtimes can later register adapters
//! here after their manifest is accepted by the extension host.

use crate::formats::{
    ApsFormat, MidiFormat, SvpFormat, UfdataFormat, UstFormat, UstxFormat, VsqxFormat,
};
use crate::project::model::{UNote, UProject};
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::Path;

pub type FormatResult<T> = Result<T, Box<dyn std::error::Error>>;

pub trait ProjectFormatAdapter: Send + Sync {
    fn id(&self) -> &'static str;
    fn extensions(&self) -> &'static [&'static str];
    fn load(&self, path: &Path) -> FormatResult<UProject>;
    fn save(&self, project: &UProject, path: &Path) -> FormatResult<()>;
}

#[derive(Default)]
pub struct ProjectFormatRegistry {
    adapters: Vec<Box<dyn ProjectFormatAdapter>>,
}

impl ProjectFormatRegistry {
    pub fn register(&mut self, adapter: impl ProjectFormatAdapter + 'static) -> Result<(), String> {
        let duplicate = adapter.extensions().iter().any(|extension| {
            self.adapters.iter().any(|registered| {
                registered
                    .extensions()
                    .iter()
                    .any(|existing| existing.eq_ignore_ascii_case(extension))
            })
        });
        if duplicate {
            return Err(format!(
                "uma extensão de arquivo de '{}' já está registrada",
                adapter.id()
            ));
        }
        self.adapters.push(Box::new(adapter));
        Ok(())
    }

    pub fn load_file(&self, path: impl AsRef<Path>) -> FormatResult<UProject> {
        let path = path.as_ref();
        self.adapter_for_path(path)?.load(path)
    }

    pub fn save_file(&self, project: &UProject, path: impl AsRef<Path>) -> FormatResult<()> {
        let path = path.as_ref();
        self.adapter_for_path(path)?.save(project, path)
    }

    pub fn supported_extensions(&self) -> Vec<&'static str> {
        let mut extensions = self
            .adapters
            .iter()
            .flat_map(|adapter| adapter.extensions().iter().copied())
            .collect::<Vec<_>>();
        extensions.sort_unstable();
        extensions
    }

    fn adapter_for_path(&self, path: &Path) -> FormatResult<&dyn ProjectFormatAdapter> {
        let extension = path
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        self.adapters
            .iter()
            .find(|adapter| {
                adapter
                    .extensions()
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(extension))
            })
            .map(|adapter| adapter.as_ref())
            .ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!("formato de projeto não suportado: .{extension}"),
                )
                .into()
            })
    }
}

pub fn default_project_format_registry() -> ProjectFormatRegistry {
    let mut registry = ProjectFormatRegistry::default();
    for adapter in [
        BuiltInFormat::Aps,
        BuiltInFormat::Ust,
        BuiltInFormat::Ustx,
        BuiltInFormat::Midi,
        BuiltInFormat::Vsqx,
        BuiltInFormat::Svp,
        BuiltInFormat::Ufdata,
        BuiltInFormat::Json,
    ] {
        registry
            .register(adapter)
            .expect("unique built-in format extensions");
    }
    registry
}

#[derive(Clone, Copy)]
enum BuiltInFormat {
    Aps,
    Ust,
    Ustx,
    Midi,
    Vsqx,
    Svp,
    Ufdata,
    Json,
}

impl ProjectFormatAdapter for BuiltInFormat {
    fn id(&self) -> &'static str {
        match self {
            Self::Aps => "org.kamafeu.aps",
            Self::Ust => "org.kamafeu.ust",
            Self::Ustx => "org.kamafeu.ustx",
            Self::Midi => "org.kamafeu.midi",
            Self::Vsqx => "org.kamafeu.vsqx",
            Self::Svp => "org.kamafeu.svp",
            Self::Ufdata => "org.kamafeu.ufdata",
            Self::Json => "org.kamafeu.json",
        }
    }

    fn extensions(&self) -> &'static [&'static str] {
        match self {
            Self::Aps => &["aps"],
            Self::Ust => &["ust"],
            Self::Ustx => &["ustx"],
            Self::Midi => &["mid", "midi"],
            Self::Vsqx => &["vsqx"],
            Self::Svp => &["svp"],
            Self::Ufdata => &["ufdata"],
            Self::Json => &["json"],
        }
    }

    fn load(&self, path: &Path) -> FormatResult<UProject> {
        match self {
            Self::Aps => ApsFormat::load_file(path),
            Self::Ust => UstFormat::load_file(path),
            Self::Ustx => UstxFormat::load_file(path),
            Self::Midi => MidiFormat::load_file(path),
            Self::Vsqx => VsqxFormat::load_file(path),
            Self::Svp => SvpFormat::load_file(path),
            Self::Ufdata => UfdataFormat::load_file(path),
            Self::Json => {
                let content = fs::read_to_string(path)?;
                if let Ok(project) = serde_json::from_str::<UProject>(&content) {
                    return Ok(project);
                }
                let notes = serde_json::from_str::<Vec<UNote>>(&content)?;
                let mut project = UProject::default();
                project.parts[0].notes = notes;
                Ok(project)
            }
        }
    }

    fn save(&self, project: &UProject, path: &Path) -> FormatResult<()> {
        match self {
            Self::Aps => ApsFormat::save_file(project, path),
            Self::Ust => UstFormat::save_file(project, path),
            Self::Ustx => UstxFormat::save_file(project, path),
            Self::Midi => MidiFormat::save_file(project, path),
            Self::Vsqx => VsqxFormat::save_file(project, path),
            Self::Svp => SvpFormat::save_file(project, path),
            Self::Ufdata => UfdataFormat::save_file(project, path),
            Self::Json => {
                fs::write(path, serde_json::to_string_pretty(project)?)?;
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_registry_roundtrips_native_project() {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("project.aps");
        let mut project = UProject::default();
        project.parts[0]
            .notes
            .push(UNote::new("a", "C4", 0.0, 300.0));
        let registry = default_project_format_registry();
        registry.save_file(&project, &path).expect("save APS");
        let restored = registry.load_file(&path).expect("load APS");
        assert_eq!(restored.parts[0].notes.len(), 1);
        assert!(registry.supported_extensions().contains(&"vsqx"));
    }
}
