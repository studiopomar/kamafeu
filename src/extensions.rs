//! Registry and manifest validation for Kamafeu extensions.
//!
//! Extensions are declared before they are executed. This lets the editor and
//! command-line tools enumerate compatible engines, formats, phonemizers and
//! effects safely, while keeping the execution runtime replaceable.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

pub mod project_formats;
pub mod wasm_runtime;
pub use project_formats::{
    default_project_format_registry, ProjectFormatAdapter, ProjectFormatRegistry,
};
pub use wasm_runtime::{verify_wasm_extension, WasmExtensionInfo};

pub const EXTENSION_MANIFEST_FILE: &str = "kamafeu-extension.json";
pub const EXTENSION_API_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionKind {
    SynthesisEngine,
    ProjectFormat,
    Phonemizer,
    Effect,
    Instrument,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionManifest {
    /// Globally unique reverse-DNS-like identifier, for example
    /// `org.example.neural-engine`.
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: u32,
    pub kind: ExtensionKind,
    #[serde(default)]
    pub description: String,
    /// Optional entrypoint relative to the manifest directory. A future
    /// runtime may load this file, but discovery never executes it.
    #[serde(default)]
    pub entrypoint: Option<String>,
}

impl ExtensionManifest {
    pub fn validate(&self) -> Result<(), String> {
        if !is_valid_extension_id(&self.id) {
            return Err("id deve usar segmentos minúsculos separados por ponto ou hífen".into());
        }
        if self.name.trim().is_empty() {
            return Err("name não pode ser vazio".into());
        }
        if self.version.trim().is_empty() {
            return Err("version não pode ser vazia".into());
        }
        if self.api_version != EXTENSION_API_VERSION {
            return Err(format!(
                "api_version {} não é compatível; esta versão do Kamafeu aceita {}",
                self.api_version, EXTENSION_API_VERSION
            ));
        }
        if let Some(entrypoint) = &self.entrypoint {
            let path = Path::new(entrypoint);
            if entrypoint.trim().is_empty() || path.is_absolute() || entrypoint.contains("..") {
                return Err("entrypoint deve ser um caminho relativo dentro da extensão".into());
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredExtension {
    pub manifest_path: PathBuf,
    pub manifest: ExtensionManifest,
}

/// Reads extension manifests in `directory` and its direct child directories.
/// Invalid manifests are returned as diagnostics; they never prevent healthy
/// extensions from being listed.
pub fn discover(directory: impl AsRef<Path>) -> ExtensionDiscovery {
    let directory = directory.as_ref();
    let mut candidates = vec![directory.join(EXTENSION_MANIFEST_FILE)];
    match fs::read_dir(directory) {
        Ok(entries) => {
            for entry in entries.flatten() {
                if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
                    candidates.push(entry.path().join(EXTENSION_MANIFEST_FILE));
                }
            }
        }
        Err(error) => {
            return ExtensionDiscovery {
                extensions: Vec::new(),
                diagnostics: vec![format!(
                    "não foi possível ler {}: {error}",
                    directory.display()
                )],
            };
        }
    }

    candidates.sort();
    candidates.dedup();
    let mut extensions = Vec::new();
    let mut diagnostics = Vec::new();
    let mut ids = BTreeSet::new();
    for path in candidates.into_iter().filter(|path| path.is_file()) {
        let parsed = fs::read_to_string(&path)
            .map_err(|error| error.to_string())
            .and_then(|content| {
                serde_json::from_str::<ExtensionManifest>(&content)
                    .map_err(|error| error.to_string())
            })
            .and_then(|manifest| manifest.validate().map(|()| manifest));
        match parsed {
            Ok(manifest) if ids.insert(manifest.id.clone()) => {
                extensions.push(DiscoveredExtension {
                    manifest_path: path,
                    manifest,
                })
            }
            Ok(manifest) => diagnostics.push(format!(
                "{}: id duplicado '{}'",
                path.display(),
                manifest.id
            )),
            Err(error) => diagnostics.push(format!("{}: {error}", path.display())),
        }
    }
    ExtensionDiscovery {
        extensions,
        diagnostics,
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtensionDiscovery {
    pub extensions: Vec<DiscoveredExtension>,
    pub diagnostics: Vec<String>,
}

/// A capability shipped with Kamafeu itself. Built-ins use the same kinds and
/// stable identifiers as third-party extensions, so callers can present one
/// unified catalog without pretending that built-ins are loaded from disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuiltInCapability {
    pub id: &'static str,
    pub name: &'static str,
    pub kind: ExtensionKind,
    pub description: &'static str,
}

pub fn built_in_capabilities() -> Vec<BuiltInCapability> {
    let capabilities = vec![
        BuiltInCapability {
            id: "org.kamafeu.venus",
            name: "VENUS",
            kind: ExtensionKind::SynthesisEngine,
            description: "Resampler nativo para bancos UTAU.",
        },
        BuiltInCapability {
            id: "org.kamafeu.andromeda",
            name: "Andromeda",
            kind: ExtensionKind::SynthesisEngine,
            description: "Wavtool e mixer nativos para envelopes e crossfades UTAU.",
        },
        BuiltInCapability {
            id: "org.kamafeu.aps",
            name: "APS",
            kind: ExtensionKind::ProjectFormat,
            description: "Formato nativo de projeto do Kamafeu.",
        },
        BuiltInCapability {
            id: "org.kamafeu.utau-formats",
            name: "UTAU and OpenUtau",
            kind: ExtensionKind::ProjectFormat,
            description: "Importação e exportação UST e USTX.",
        },
        BuiltInCapability {
            id: "org.kamafeu.interchange-formats",
            name: "Interchange formats",
            kind: ExtensionKind::ProjectFormat,
            description: "Importação e exportação MIDI, VSQX, SVP e UFData.",
        },
        BuiltInCapability {
            id: "org.kamafeu.phonemizers",
            name: "Built-in phonemizers",
            kind: ExtensionKind::Phonemizer,
            description: "Fonemizadores japonês, português, inglês e manual.",
        },
        BuiltInCapability {
            id: "org.kamafeu.fx-rack",
            name: "FX Rack",
            kind: ExtensionKind::Effect,
            description: "Equalização, compressão, chorus, delay, reverb e mais.",
        },
    ];
    #[cfg(target_arch = "wasm32")]
    {
        capabilities
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut capabilities = capabilities;
        capabilities.push(BuiltInCapability {
            id: "org.kamafeu.diffsinger",
            name: "DiffSinger ONNX",
            kind: ExtensionKind::SynthesisEngine,
            description: "Renderização de frases DiffSinger com modelos ONNX locais.",
        });
        capabilities
    }
}

fn is_valid_extension_id(id: &str) -> bool {
    let segments = id.split('.').collect::<Vec<_>>();
    segments.len() >= 2
        && segments.iter().all(|segment| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
                && !segment.starts_with('-')
                && !segment.ends_with('-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = r#"{
        "id": "org.kamafeu.test-engine",
        "name": "Test engine",
        "version": "0.1.0",
        "api_version": 1,
        "kind": "synthesis_engine",
        "entrypoint": "engine.wasm"
    }"#;

    #[test]
    fn discovers_a_valid_child_extension_and_reports_invalid_ones() {
        let directory = tempfile::tempdir().expect("temp directory");
        let valid = directory.path().join("valid");
        let invalid = directory.path().join("invalid");
        fs::create_dir_all(&valid).expect("valid directory");
        fs::create_dir_all(&invalid).expect("invalid directory");
        fs::write(valid.join(EXTENSION_MANIFEST_FILE), VALID).expect("valid manifest");
        fs::write(invalid.join(EXTENSION_MANIFEST_FILE), "{not json}").expect("invalid manifest");

        let discovery = discover(directory.path());
        assert_eq!(discovery.extensions.len(), 1);
        assert_eq!(
            discovery.extensions[0].manifest.id,
            "org.kamafeu.test-engine"
        );
        assert_eq!(discovery.diagnostics.len(), 1);
    }

    #[test]
    fn rejects_unsafe_entrypoints_and_invalid_ids() {
        let mut manifest: ExtensionManifest = serde_json::from_str(VALID).expect("manifest");
        manifest.id = "Kamafeu".to_string();
        assert!(manifest.validate().is_err());
        manifest.id = "org.kamafeu.safe".to_string();
        manifest.entrypoint = Some("../engine.wasm".to_string());
        assert!(manifest.validate().is_err());
    }

    #[test]
    fn built_in_capabilities_use_valid_extension_ids() {
        let capabilities = built_in_capabilities();
        assert!(!capabilities.is_empty());
        assert!(capabilities
            .iter()
            .all(|capability| is_valid_extension_id(capability.id)));
    }
}
