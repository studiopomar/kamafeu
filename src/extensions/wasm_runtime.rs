//! Sandboxed WebAssembly verification runtime for Kamafeu extensions.
//!
//! The initial ABI intentionally exposes no host imports. A module can prove
//! its API compatibility through `kamafeu_extension_api_version() -> i32`, but
//! it cannot access files, processes or the network through Kamafeu.

use super::DiscoveredExtension;
#[cfg(not(target_arch = "wasm32"))]
use super::{ExtensionManifest, EXTENSION_API_VERSION};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmExtensionInfo {
    pub id: String,
    pub entrypoint: PathBuf,
    pub api_version: u32,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn verify_wasm_extension(extension: &DiscoveredExtension) -> Result<WasmExtensionInfo, String> {
    use wasmi::{Config, Engine, Linker, Module, Store};

    let entrypoint = resolve_entrypoint(&extension.manifest_path, &extension.manifest)?;
    let bytes = std::fs::read(&entrypoint)
        .map_err(|error| format!("não foi possível ler {}: {error}", entrypoint.display()))?;
    let mut config = Config::default();
    config.consume_fuel(true);
    let engine = Engine::new(&config);
    let module = Module::new(&engine, &mut bytes.as_slice())
        .map_err(|error| format!("módulo WASM inválido em {}: {error}", entrypoint.display()))?;
    let mut store = Store::new(&engine, ());
    store
        .add_fuel(100_000)
        .map_err(|error| format!("não foi possível configurar limite WASM: {error}"))?;
    let linker = Linker::<()>::new(&engine);
    let pre_instance = linker
        .instantiate(&mut store, &module)
        .map_err(|error| format!("imports WASM não permitidos: {error}"))?;
    let instance = pre_instance
        .start(&mut store)
        .map_err(|error| format!("falha ao iniciar extensão WASM: {error}"))?;
    let api_version = instance
        .get_typed_func::<(), i32>(&store, "kamafeu_extension_api_version")
        .map_err(|_| {
            "extensão WASM não exporta kamafeu_extension_api_version() -> i32".to_string()
        })?
        .call(&mut store, ())
        .map_err(|error| format!("falha ao consultar API da extensão WASM: {error}"))?;
    if api_version != EXTENSION_API_VERSION as i32 {
        return Err(format!(
            "extensão WASM declara API {api_version}; Kamafeu requer {}",
            EXTENSION_API_VERSION
        ));
    }
    Ok(WasmExtensionInfo {
        id: extension.manifest.id.clone(),
        entrypoint,
        api_version: api_version as u32,
    })
}

#[cfg(target_arch = "wasm32")]
pub fn verify_wasm_extension(
    _extension: &DiscoveredExtension,
) -> Result<WasmExtensionInfo, String> {
    Err("o host de extensões WASM ainda não está disponível na compilação web".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_entrypoint(
    manifest_path: &std::path::Path,
    manifest: &ExtensionManifest,
) -> Result<PathBuf, String> {
    let entrypoint = manifest
        .entrypoint
        .as_deref()
        .ok_or_else(|| "manifesto não declara entrypoint WASM".to_string())?;
    if !entrypoint.to_ascii_lowercase().ends_with(".wasm") {
        return Err("entrypoint deve apontar para um arquivo .wasm".to_string());
    }
    let root = manifest_path
        .parent()
        .ok_or_else(|| format!("caminho de manifesto inválido: {}", manifest_path.display()))?;
    let resolved = root.join(entrypoint);
    if !resolved.is_file() {
        return Err(format!(
            "entrypoint WASM não encontrado: {}",
            resolved.display()
        ));
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extensions::{ExtensionKind, ExtensionManifest, EXTENSION_API_VERSION};

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn verifies_a_minimal_wasm_module_with_matching_api() {
        let directory = tempfile::tempdir().expect("temporary extension directory");
        let manifest_path = directory.path().join("kamafeu-extension.json");
        let entrypoint = directory.path().join("extension.wasm");
        // (module (func (export "kamafeu_extension_api_version") (result i32) i32.const 1))
        let module = [
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x01, 0x60, 0x00, 0x01,
            0x7f, 0x03, 0x02, 0x01, 0x00, 0x07, 0x21, 0x01, 0x1d, b'k', b'a', b'm', b'a', b'f',
            b'e', b'u', b'_', b'e', b'x', b't', b'e', b'n', b's', b'i', b'o', b'n', b'_', b'a',
            b'p', b'i', b'_', b'v', b'e', b'r', b's', b'i', b'o', b'n', 0x00, 0x00, 0x0a, 0x06,
            0x01, 0x04, 0x00, 0x41, 0x01, 0x0b,
        ];
        std::fs::write(&entrypoint, module).expect("WASM module");
        let manifest = ExtensionManifest {
            id: "org.kamafeu.runtime-test".to_string(),
            name: "Runtime test".to_string(),
            version: "0.1.0".to_string(),
            api_version: EXTENSION_API_VERSION,
            kind: ExtensionKind::Effect,
            description: String::new(),
            entrypoint: Some("extension.wasm".to_string()),
        };
        let extension = DiscoveredExtension {
            manifest_path,
            manifest,
        };
        let info = verify_wasm_extension(&extension).expect("verify module");
        assert_eq!(info.api_version, EXTENSION_API_VERSION);
        assert_eq!(info.entrypoint, entrypoint);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn rejects_a_wasm_module_for_a_different_api_version() {
        let directory = tempfile::tempdir().expect("temporary extension directory");
        let manifest_path = directory.path().join("kamafeu-extension.json");
        let entrypoint = directory.path().join("extension.wasm");
        let mut module = [
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x01, 0x60, 0x00, 0x01,
            0x7f, 0x03, 0x02, 0x01, 0x00, 0x07, 0x21, 0x01, 0x1d, b'k', b'a', b'm', b'a', b'f',
            b'e', b'u', b'_', b'e', b'x', b't', b'e', b'n', b's', b'i', b'o', b'n', b'_', b'a',
            b'p', b'i', b'_', b'v', b'e', b'r', b's', b'i', b'o', b'n', 0x00, 0x00, 0x0a, 0x06,
            0x01, 0x04, 0x00, 0x41, 0x01, 0x0b,
        ];
        module[module.len() - 2] = 2;
        std::fs::write(&entrypoint, module).expect("WASM module");
        let extension = DiscoveredExtension {
            manifest_path,
            manifest: ExtensionManifest {
                id: "org.kamafeu.runtime-mismatch".to_string(),
                name: "Runtime mismatch".to_string(),
                version: "0.1.0".to_string(),
                api_version: EXTENSION_API_VERSION,
                kind: ExtensionKind::Effect,
                description: String::new(),
                entrypoint: Some("extension.wasm".to_string()),
            },
        };
        let error = verify_wasm_extension(&extension).expect_err("API mismatch");
        assert!(error.contains("declara API 2"));
    }
}
