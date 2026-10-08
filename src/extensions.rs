//! Registry and manifest validation for Kamafeu extensions.
//!
//! Extensions are declared before they are executed. This lets the editor and
//! command-line tools enumerate compatible engines, formats, phonemizers and
//! effects safely, while keeping the execution runtime replaceable.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub mod project_formats;
pub mod wasm_runtime;
pub use project_formats::{
    default_project_format_registry, ProjectFormatAdapter, ProjectFormatRegistry,
};
pub use wasm_runtime::{
    invoke_wasm_bytes, invoke_wasm_i32, invoke_wasm_i32_with_parameters, verify_wasm_extension,
    verify_wasm_extension_with_policy, WasmExtensionInfo,
};

pub const EXTENSION_MANIFEST_FILE: &str = "kamafeu-extension.json";
pub const EXTENSION_API_VERSION: u32 = 1;
pub const EXTENSION_ABI_MIN_VERSION: u32 = 1;
pub const EXTENSION_ABI_MAX_VERSION: u32 = 1;

/// Host-side consent state. Discovery may list requested permissions, but an
/// extension is only eligible for activation when every request is present in
/// this allow-list. The default is deliberately deny-all.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionPermissionPolicy {
    #[serde(default)]
    pub granted: BTreeSet<String>,
}

impl ExtensionPermissionPolicy {
    pub fn allows(&self, permission: &str) -> bool {
        self.granted.contains(permission)
    }

    pub fn check(&self, manifest: &ExtensionManifest) -> Result<(), String> {
        if let Some(permission) = manifest
            .permissions
            .iter()
            .find(|permission| !self.allows(permission))
        {
            return Err(format!(
                "permissão '{}' não foi concedida ao host para a extensão '{}'",
                permission, manifest.id
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionKind {
    SynthesisEngine,
    ProjectFormat,
    Phonemizer,
    Effect,
    Instrument,
    /// Non-rendering analysis or inspection tool.
    Analysis,
    /// Non-rendering musical transformation.
    Transform,
    /// Shareable parameter/pipeline preset provider.
    Preset,
    /// UI contribution that remains optional and capability-declared.
    EditorPanel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionParameter {
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExtensionManifest {
    /// Globally unique reverse-DNS-like identifier, for example
    /// `org.example.neural-engine`.
    pub id: String,
    pub name: String,
    pub version: String,
    pub api_version: u32,
    /// Inclusive ABI range supported by the extension. Older manifests use
    /// `api_version` and are interpreted as an exact range.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abi_min_version: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub abi_max_version: Option<u32>,
    pub kind: ExtensionKind,
    /// Stable capability identifiers exposed by the extension. Keeping this
    /// list in the manifest lets clients filter extensions before loading them.
    #[serde(default)]
    pub capabilities: BTreeSet<String>,
    /// Project/audio formats understood by the extension.
    #[serde(default)]
    pub formats: BTreeSet<String>,
    /// Phonemizer identifiers implemented by the extension.
    #[serde(default)]
    pub phonemizers: BTreeSet<String>,
    /// Requested host capabilities. Discovery reports these, while runtime
    /// activation remains responsible for enforcing the actual policy.
    #[serde(default)]
    pub permissions: BTreeSet<String>,
    /// Optional platform identifiers. An empty set means no restriction.
    #[serde(default)]
    pub platforms: BTreeSet<String>,
    /// Declarative limitations shown before activation (for example required
    /// model families or unsupported voicebank features). These strings are
    /// informational and never weaken the host sandbox.
    #[serde(default)]
    pub limitations: Vec<String>,
    /// Host-discoverable controls. Plugins must still validate values at the
    /// ABI boundary; this schema only describes the public contract.
    #[serde(default)]
    pub parameters: BTreeMap<String, ExtensionParameter>,
    #[serde(default)]
    pub description: String,
    /// Optional entrypoint relative to the manifest directory. Discovery never
    /// executes it; activation validates it through the sandboxed WASM host.
    #[serde(default)]
    pub entrypoint: Option<String>,
}

impl ExtensionManifest {
    pub fn supports_platform(&self, platform: &str) -> bool {
        self.platforms.is_empty() || self.platforms.contains(platform)
    }

    /// Validates host-provided parameter values without executing the plugin.
    /// Unknown keys are rejected so a typo cannot silently select a default.
    pub fn validate_parameter_values(
        &self,
        values: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<(), String> {
        if let Some(name) = values
            .keys()
            .find(|name| !self.parameters.contains_key(*name))
        {
            return Err(format!("parâmetro não declarado pelo plugin: '{name}'"));
        }
        for (name, parameter) in &self.parameters {
            let Some(value) = values.get(name) else {
                if parameter.required {
                    return Err(format!("parâmetro obrigatório ausente: '{name}'"));
                }
                continue;
            };
            match parameter.kind.as_str() {
                "number" if !value.is_number() => {
                    return Err(format!("parâmetro '{name}' deve ser numérico"))
                }
                "integer" if value.as_i64().is_none() && value.as_u64().is_none() => {
                    return Err(format!("parâmetro '{name}' deve ser inteiro"))
                }
                "boolean" if !value.is_boolean() => {
                    return Err(format!("parâmetro '{name}' deve ser booleano"))
                }
                "string" if !value.is_string() => {
                    return Err(format!("parâmetro '{name}' deve ser texto"))
                }
                _ => {}
            }
            if let Some(number) = value.as_f64() {
                if parameter.minimum.is_some_and(|minimum| number < minimum)
                    || parameter.maximum.is_some_and(|maximum| number > maximum)
                {
                    return Err(format!("parâmetro '{name}' está fora da faixa declarada"));
                }
            }
        }
        Ok(())
    }

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
        let min = self.abi_min_version.unwrap_or(self.api_version);
        let max = self.abi_max_version.unwrap_or(self.api_version);
        if min > max || min > EXTENSION_ABI_MAX_VERSION || max < EXTENSION_ABI_MIN_VERSION {
            return Err(format!(
                "ABI incompatível: extensão aceita {min}..={max}, host oferece {}..={}",
                EXTENSION_ABI_MIN_VERSION, EXTENSION_ABI_MAX_VERSION
            ));
        }
        for (label, values) in [
            ("capability", &self.capabilities),
            ("format", &self.formats),
            ("phonemizer", &self.phonemizers),
            ("permission", &self.permissions),
            ("platform", &self.platforms),
        ] {
            if let Some(value) = values.iter().find(|value| !is_valid_manifest_label(value)) {
                return Err(format!(
                    "{label} '{value}' deve usar apenas letras minúsculas, números, ponto, hífen ou sublinhado"
                ));
            }
        }
        if self
            .limitations
            .iter()
            .any(|limitation| limitation.trim().is_empty())
        {
            return Err("limitations não pode conter texto vazio".into());
        }
        for (name, parameter) in &self.parameters {
            if !is_valid_manifest_label(name) {
                return Err(format!(
                    "parâmetro '{name}' deve usar apenas letras minúsculas, números, ponto, hífen ou sublinhado"
                ));
            }
            if !matches!(
                parameter.kind.as_str(),
                "number" | "integer" | "boolean" | "string"
            ) {
                return Err(format!(
                    "parâmetro '{name}' possui tipo desconhecido '{}', esperado number, integer, boolean ou string",
                    parameter.kind
                ));
            }
            if let (Some(minimum), Some(maximum)) = (parameter.minimum, parameter.maximum) {
                if !minimum.is_finite() || !maximum.is_finite() || minimum > maximum {
                    return Err(format!("parâmetro '{name}' possui faixa inválida"));
                }
            }
            if let Some(default) = &parameter.default {
                let valid_type = match parameter.kind.as_str() {
                    "number" => default.is_number(),
                    "integer" => default.as_i64().is_some() || default.as_u64().is_some(),
                    "boolean" => default.is_boolean(),
                    "string" => default.is_string(),
                    _ => false,
                };
                let in_range = default.as_f64().is_none_or(|number| {
                    number.is_finite()
                        && parameter.minimum.is_none_or(|minimum| number >= minimum)
                        && parameter.maximum.is_none_or(|maximum| number <= maximum)
                });
                if !valid_type || !in_range {
                    return Err(format!(
                        "parâmetro '{name}' possui valor padrão fora da faixa"
                    ));
                }
            }
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

/// Stable platform label used in manifests and diagnostics.
pub fn current_platform() -> &'static str {
    #[cfg(target_arch = "wasm32")]
    {
        return "wasm";
    }
    #[cfg(target_os = "android")]
    {
        return "android";
    }
    #[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
    {
        "desktop"
    }
}

/// Capability contract shared by the UI, automation clients and the
/// compatibility documentation.  Keeping this in code prevents the platform
/// matrix from becoming a promise that drifts away from the actual cfg gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct PlatformCapability {
    pub id: &'static str,
    pub desktop: bool,
    pub android: bool,
    pub wasm: bool,
    pub note: &'static str,
}

pub fn platform_capability_matrix() -> Vec<PlatformCapability> {
    vec![
        PlatformCapability {
            id: "project-formats",
            desktop: true,
            android: true,
            wasm: true,
            note: "APS, UST/USTX e formatos de intercâmbio sem processos externos",
        },
        PlatformCapability {
            id: "utau-native-render",
            desktop: true,
            android: true,
            wasm: true,
            note: "resampler e wavtool nativos",
        },
        PlatformCapability {
            id: "external-process-engines",
            desktop: true,
            android: false,
            wasm: false,
            note: "executáveis externos e Wine",
        },
        PlatformCapability {
            id: "diffsinger-onnx",
            desktop: true,
            android: false,
            wasm: false,
            note: "modelos ONNX locais; indisponível sem runtime nativo",
        },
        PlatformCapability {
            id: "wasm-extensions",
            desktop: true,
            android: true,
            wasm: true,
            note: "manifesto validado e execução sandboxed quando o runtime está disponível",
        },
        PlatformCapability {
            id: "local-snapshots",
            desktop: true,
            android: true,
            wasm: false,
            note: "autosave e snapshots no sistema de arquivos local",
        },
    ]
}

#[derive(Debug, Clone, PartialEq)]
pub struct DiscoveredExtension {
    pub manifest_path: PathBuf,
    pub manifest: ExtensionManifest,
}

impl DiscoveredExtension {
    /// Produces the stable identity recorded by render provenance. Reading the
    /// manifest again is intentional: the identity must reflect its current
    /// bytes, even when a plugin keeps the same version string.
    pub fn render_identity(&self) -> Result<crate::renderer::RenderExtension, String> {
        let bytes = fs::read(&self.manifest_path).map_err(|error| {
            format!(
                "não foi possível ler o manifesto da extensão '{}': {error}",
                self.manifest.id
            )
        })?;
        Ok(crate::renderer::RenderExtension {
            id: self.manifest.id.clone(),
            version: self.manifest.version.clone(),
            origin: self.manifest_path.to_string_lossy().into_owned(),
            manifest_fingerprint: crate::renderer::provenance::content_fingerprint(&bytes),
        })
    }
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
                diagnostic_records: vec![ExtensionDiscoveryDiagnostic {
                    manifest_path: directory.to_string_lossy().into_owned(),
                    reason: error.to_string(),
                }],
            };
        }
    }

    candidates.sort();
    candidates.dedup();
    let mut extensions = Vec::new();
    let mut diagnostics = Vec::new();
    let mut diagnostic_records = Vec::new();
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
            Ok(manifest) => {
                let reason = format!("id duplicado '{}'", manifest.id);
                diagnostics.push(format!("{}: {reason}", path.display()));
                diagnostic_records.push(ExtensionDiscoveryDiagnostic {
                    manifest_path: path.to_string_lossy().into_owned(),
                    reason,
                });
            }
            Err(error) => {
                diagnostics.push(format!("{}: {error}", path.display()));
                diagnostic_records.push(ExtensionDiscoveryDiagnostic {
                    manifest_path: path.to_string_lossy().into_owned(),
                    reason: error,
                });
            }
        }
    }
    ExtensionDiscovery {
        extensions,
        diagnostics,
        diagnostic_records,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionDiscoveryDiagnostic {
    pub manifest_path: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct ExtensionDiscovery {
    pub extensions: Vec<DiscoveredExtension>,
    pub diagnostics: Vec<String>,
    /// Structured counterpart of `diagnostics`; the text field remains for
    /// backwards compatibility with existing CLI consumers.
    pub diagnostic_records: Vec<ExtensionDiscoveryDiagnostic>,
}

/// Compares extension references persisted in a project with the discovered
/// catalog. This is deliberately metadata-only: it never loads or executes a
/// WASM module, and reports every mismatch so the UI can keep incompatible
/// entries visible with an actionable explanation.
pub fn diagnose_project_extensions(
    project_extensions: &[crate::project::model::UProjectExtension],
    discovery: &ExtensionDiscovery,
) -> Vec<String> {
    diagnose_project_extensions_structured(project_extensions, discovery)
        .into_iter()
        .map(|diagnostic| match diagnostic.kind.as_str() {
            "missing" => format!(
                "extensão do projeto '{}' não foi encontrada no catálogo",
                diagnostic.extension_id
            ),
            "version_mismatch" => format!(
                "extensão '{}' {}",
                diagnostic.extension_id, diagnostic.detail
            ),
            "manifest_changed" => format!(
                "manifesto da extensão '{}' {}",
                diagnostic.extension_id, diagnostic.detail
            ),
            _ => format!("{}: {}", diagnostic.extension_id, diagnostic.detail),
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectExtensionDiagnostic {
    pub extension_id: String,
    pub kind: String,
    pub detail: String,
}

pub fn diagnose_project_extensions_structured(
    project_extensions: &[crate::project::model::UProjectExtension],
    discovery: &ExtensionDiscovery,
) -> Vec<ProjectExtensionDiagnostic> {
    let mut diagnostics = Vec::new();
    for required in project_extensions {
        let Some(found) = discovery
            .extensions
            .iter()
            .find(|extension| extension.manifest.id == required.id)
        else {
            diagnostics.push(ProjectExtensionDiagnostic {
                extension_id: required.id.clone(),
                kind: "missing".to_string(),
                detail: "não foi encontrada no catálogo".to_string(),
            });
            continue;
        };
        if found.manifest.version != required.version {
            diagnostics.push(ProjectExtensionDiagnostic {
                extension_id: required.id.clone(),
                kind: "version_mismatch".to_string(),
                detail: format!(
                    "requer versão {}, mas foi encontrada a versão {}",
                    required.version, found.manifest.version
                ),
            });
        }
        if let Ok(identity) = found.render_identity() {
            if identity.manifest_fingerprint != required.manifest_fingerprint {
                diagnostics.push(ProjectExtensionDiagnostic {
                    extension_id: required.id.clone(),
                    kind: "manifest_changed".to_string(),
                    detail: "manifesto foi alterado desde o último uso".to_string(),
                });
            }
        }
    }
    diagnostics
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

fn is_valid_manifest_label(value: &str) -> bool {
    !value.is_empty()
        && value.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'-' | b'_')
        })
        && !value.starts_with(['.', '-', '_'])
        && !value.ends_with(['.', '-', '_'])
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
        assert_eq!(discovery.diagnostic_records.len(), 1);
        assert!(discovery.diagnostic_records[0]
            .manifest_path
            .ends_with("invalid/kamafeu-extension.json"));
        assert!(!discovery.diagnostic_records[0].reason.is_empty());
    }

    #[test]
    fn discovered_extension_identity_tracks_manifest_content() {
        let directory = tempfile::tempdir().expect("temp directory");
        let manifest_path = directory.path().join(EXTENSION_MANIFEST_FILE);
        fs::write(&manifest_path, VALID).expect("manifest");
        let extension = discover(directory.path()).extensions.pop().unwrap();
        let identity = extension.render_identity().expect("identity");
        assert_eq!(identity.id, "org.kamafeu.test-engine");
        assert_eq!(identity.version, "0.1.0");
        assert_eq!(identity.origin, manifest_path.to_string_lossy());

        fs::write(&manifest_path, format!("{VALID}\n")).expect("changed manifest");
        let changed = extension.render_identity().expect("changed identity");
        assert_ne!(identity.manifest_fingerprint, changed.manifest_fingerprint);
    }

    #[test]
    fn project_extension_diagnostics_report_missing_version_and_manifest_changes() {
        let directory = tempfile::tempdir().expect("temp directory");
        let manifest_path = directory.path().join(EXTENSION_MANIFEST_FILE);
        fs::write(&manifest_path, VALID).expect("manifest");
        let discovery = discover(directory.path());
        let identity = discovery.extensions[0].render_identity().expect("identity");
        let project = vec![
            crate::project::model::UProjectExtension {
                id: identity.id.clone(),
                version: "9.0.0".into(),
                origin: identity.origin.clone(),
                manifest_fingerprint: identity.manifest_fingerprint.clone(),
            },
            crate::project::model::UProjectExtension {
                id: "org.example.missing".into(),
                version: "1.0.0".into(),
                origin: String::new(),
                manifest_fingerprint: String::new(),
            },
        ];
        fs::write(&manifest_path, format!("{VALID}\n")).expect("changed manifest");
        let diagnostics = diagnose_project_extensions(&project, &discovery);
        assert_eq!(diagnostics.len(), 3);
        assert!(diagnostics.iter().any(|value| value.contains("versão")));
        assert!(diagnostics
            .iter()
            .any(|value| value.contains("não foi encontrada")));
        assert!(diagnostics
            .iter()
            .any(|value| value.contains("foi alterado")));
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

    #[test]
    fn manifest_capabilities_are_optional_and_roundtrip() {
        let mut manifest: ExtensionManifest = serde_json::from_str(VALID).expect("manifest");
        assert!(manifest.capabilities.is_empty());
        assert!(manifest.permissions.is_empty());
        manifest
            .capabilities
            .insert("phonemizer.japanese".to_string());
        manifest.permissions.insert("read_voicebank".to_string());
        manifest
            .limitations
            .push("modelo externo obrigatório".to_string());
        let encoded = serde_json::to_string(&manifest).expect("serialize manifest");
        let decoded: ExtensionManifest = serde_json::from_str(&encoded).expect("deserialize");
        assert_eq!(decoded.capabilities, manifest.capabilities);
        assert_eq!(decoded.permissions, manifest.permissions);
        assert_eq!(decoded.limitations, manifest.limitations);
    }

    #[test]
    fn manifest_rejects_empty_limitations_but_accepts_explanatory_text() {
        let mut manifest: ExtensionManifest = serde_json::from_str(VALID).expect("manifest");
        manifest.limitations.push("   ".to_string());
        assert!(manifest.validate().unwrap_err().contains("texto vazio"));
        manifest.limitations = vec!["requer modelo externo".to_string()];
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn permission_policy_denies_by_default_and_allows_explicit_consent() {
        let mut manifest: ExtensionManifest = serde_json::from_str(VALID).expect("manifest");
        manifest.permissions.insert("read_voicebank".to_string());
        let denied = ExtensionPermissionPolicy::default();
        assert!(denied
            .check(&manifest)
            .unwrap_err()
            .contains("read_voicebank"));

        let mut granted = ExtensionPermissionPolicy::default();
        granted.granted.insert("read_voicebank".to_string());
        assert!(granted.check(&manifest).is_ok());
    }

    #[test]
    fn new_non_rendering_extension_kinds_are_discoverable() {
        for kind in ["analysis", "transform", "preset", "editor_panel"] {
            let json = VALID.replace("synthesis_engine", kind);
            let manifest: ExtensionManifest = serde_json::from_str(&json).expect("manifest");
            assert!(manifest.validate().is_ok(), "kind {kind}");
        }
    }

    #[test]
    fn platform_restrictions_are_optional_and_exact() {
        let mut manifest: ExtensionManifest = serde_json::from_str(VALID).expect("manifest");
        assert!(manifest.supports_platform("desktop"));
        manifest.platforms.insert("wasm".to_string());
        assert!(manifest.supports_platform("wasm"));
        assert!(!manifest.supports_platform("desktop"));
    }

    #[test]
    fn platform_matrix_has_stable_ids_and_expected_boundaries() {
        let matrix = platform_capability_matrix();
        assert!(matrix
            .iter()
            .any(|item| item.id == "project-formats" && item.wasm));
        assert!(matrix
            .iter()
            .any(|item| item.id == "external-process-engines" && item.desktop && !item.wasm));
        assert!(matrix
            .iter()
            .any(|item| item.id == "diffsinger-onnx" && item.desktop && !item.android));
        assert!(matrix
            .iter()
            .all(|item| { !item.id.is_empty() && !item.note.is_empty() }));
    }

    #[test]
    fn validates_typed_parameter_ranges_and_roundtrips_them() {
        let mut manifest: ExtensionManifest = serde_json::from_str(VALID).expect("manifest");
        manifest.parameters.insert(
            "mix".to_string(),
            ExtensionParameter {
                kind: "number".to_string(),
                default: Some(serde_json::json!(0.5)),
                minimum: Some(0.0),
                maximum: Some(1.0),
                required: false,
            },
        );
        assert!(manifest.validate().is_ok());
        let encoded = serde_json::to_string(&manifest).expect("serialize manifest");
        let decoded: ExtensionManifest = serde_json::from_str(&encoded).expect("deserialize");
        assert_eq!(decoded.parameters, manifest.parameters);
        let mut invalid = manifest;
        invalid.parameters.get_mut("mix").unwrap().default = Some(serde_json::json!(2.0));
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn validates_values_before_plugin_execution() {
        let mut manifest: ExtensionManifest = serde_json::from_str(VALID).expect("manifest");
        manifest.parameters.insert(
            "enabled".to_string(),
            ExtensionParameter {
                kind: "boolean".to_string(),
                default: Some(serde_json::json!(false)),
                minimum: None,
                maximum: None,
                required: true,
            },
        );
        let mut values = serde_json::Map::new();
        values.insert("enabled".to_string(), serde_json::json!(true));
        assert!(manifest.validate_parameter_values(&values).is_ok());
        values.insert("enabled".to_string(), serde_json::json!("yes"));
        assert!(manifest.validate_parameter_values(&values).is_err());
        values.remove("enabled");
        assert!(manifest.validate_parameter_values(&values).is_err());
        values.insert("unknown".to_string(), serde_json::json!(1));
        assert!(manifest.validate_parameter_values(&values).is_err());
    }

    #[test]
    fn manifest_rejects_malformed_capabilities_and_permissions() {
        let mut manifest: ExtensionManifest = serde_json::from_str(VALID).expect("manifest");
        manifest.capabilities.insert("Read Audio".to_string());
        assert!(manifest.validate().unwrap_err().contains("capability"));
        manifest.capabilities.clear();
        manifest.formats.insert("UST X".to_string());
        assert!(manifest.validate().unwrap_err().contains("format"));
        manifest.formats.clear();
        manifest.permissions.insert("../files".to_string());
        assert!(manifest.validate().unwrap_err().contains("permission"));
    }
}
