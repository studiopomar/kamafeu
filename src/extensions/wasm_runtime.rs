//! Sandboxed WebAssembly verification runtime for Kamafeu extensions.
//!
//! The initial ABI intentionally exposes no host imports. A module can prove
//! its API compatibility through `kamafeu_extension_api_version() -> i32`, but
//! it cannot access files, processes or the network through Kamafeu.

use super::{DiscoveredExtension, ExtensionPermissionPolicy};
#[cfg(not(target_arch = "wasm32"))]
use super::{ExtensionManifest, EXTENSION_API_VERSION};
use std::path::PathBuf;

#[cfg(not(target_arch = "wasm32"))]
const WASM_FUEL_LIMIT: u64 = 100_000;
#[cfg(not(target_arch = "wasm32"))]
const WASM_MEMORY_LIMIT_BYTES: usize = 64 * 1024 * 1024;
#[cfg(not(target_arch = "wasm32"))]
const WASM_OUTPUT_LIMIT_BYTES: usize = 1024 * 1024;

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
struct SandboxState {
    limits: wasmi::StoreLimits,
}

#[cfg(not(target_arch = "wasm32"))]
fn sandbox_store(engine: &wasmi::Engine) -> Result<wasmi::Store<SandboxState>, String> {
    use wasmi::StoreLimitsBuilder;

    let limits = StoreLimitsBuilder::new()
        .memory_size(WASM_MEMORY_LIMIT_BYTES)
        .instances(1)
        .tables(8)
        .memories(8)
        .build();
    let mut store = wasmi::Store::new(engine, SandboxState { limits });
    store.limiter(|state| &mut state.limits);
    store
        .add_fuel(WASM_FUEL_LIMIT)
        .map_err(|error| format!("não foi possível configurar limite WASM: {error}"))?;
    Ok(store)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WasmExtensionInfo {
    pub id: String,
    pub entrypoint: PathBuf,
    pub api_version: u32,
}

#[cfg(not(target_arch = "wasm32"))]
pub fn verify_wasm_extension(extension: &DiscoveredExtension) -> Result<WasmExtensionInfo, String> {
    verify_wasm_extension_with_policy(extension, &ExtensionPermissionPolicy::default())
}

/// Verifies and authorizes a WASM extension. Verification never grants a
/// permission by itself; callers must provide a policy representing explicit
/// user consent.
#[cfg(not(target_arch = "wasm32"))]
pub fn verify_wasm_extension_with_policy(
    extension: &DiscoveredExtension,
    policy: &ExtensionPermissionPolicy,
) -> Result<WasmExtensionInfo, String> {
    policy.check(&extension.manifest)?;
    ensure_platform_compatible(&extension.manifest)?;
    use wasmi::{Config, Engine, Linker, Module};

    let entrypoint = resolve_entrypoint(&extension.manifest_path, &extension.manifest)?;
    let bytes = std::fs::read(&entrypoint)
        .map_err(|error| format!("não foi possível ler {}: {error}", entrypoint.display()))?;
    let mut config = Config::default();
    config.consume_fuel(true);
    let engine = Engine::new(&config);
    let module = Module::new(&engine, &mut bytes.as_slice())
        .map_err(|error| format!("módulo WASM inválido em {}: {error}", entrypoint.display()))?;
    let mut store = sandbox_store(&engine)?;
    let linker = Linker::<SandboxState>::new(&engine);
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

/// Invokes a small, explicitly named WASM entrypoint using the same sandbox
/// rules as verification. This is the first executable ABI primitive for
/// analysis/preset plugins; richer interfaces can be added without granting
/// imports to the module.
#[cfg(not(target_arch = "wasm32"))]
pub fn invoke_wasm_i32(
    extension: &DiscoveredExtension,
    policy: &ExtensionPermissionPolicy,
    export_name: &str,
    argument: i32,
) -> Result<i32, String> {
    invoke_wasm_i32_with_parameters(
        extension,
        policy,
        export_name,
        argument,
        &serde_json::Map::new(),
    )
}

#[cfg(not(target_arch = "wasm32"))]
pub fn invoke_wasm_i32_with_parameters(
    extension: &DiscoveredExtension,
    policy: &ExtensionPermissionPolicy,
    export_name: &str,
    argument: i32,
    parameters: &serde_json::Map<String, serde_json::Value>,
) -> Result<i32, String> {
    use wasmi::{Config, Engine, Linker, Module};

    policy.check(&extension.manifest)?;
    ensure_platform_compatible(&extension.manifest)?;
    extension.manifest.validate_parameter_values(parameters)?;
    let entrypoint = resolve_entrypoint(&extension.manifest_path, &extension.manifest)?;
    let bytes = std::fs::read(&entrypoint)
        .map_err(|error| format!("não foi possível ler {}: {error}", entrypoint.display()))?;
    let mut config = Config::default();
    config.consume_fuel(true);
    let engine = Engine::new(&config);
    let module = Module::new(&engine, &mut bytes.as_slice())
        .map_err(|error| format!("módulo WASM inválido: {error}"))?;
    let mut store = sandbox_store(&engine)?;
    let linker = Linker::<SandboxState>::new(&engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|error| format!("imports WASM não permitidos: {error}"))?
        .start(&mut store)
        .map_err(|error| format!("falha ao iniciar extensão WASM: {error}"))?;
    let api_version = instance
        .get_typed_func::<(), i32>(&store, "kamafeu_extension_api_version")
        .map_err(|_| "extensão não exporta a versão da ABI".to_string())?
        .call(&mut store, ())
        .map_err(|error| format!("falha ao consultar ABI: {error}"))?;
    if api_version != EXTENSION_API_VERSION as i32 {
        return Err(format!(
            "extensão declara API {api_version}, host requer {EXTENSION_API_VERSION}"
        ));
    }
    instance
        .get_typed_func::<i32, i32>(&store, export_name)
        .map_err(|_| format!("extensão não exporta {export_name}(i32) -> i32"))?
        .call(&mut store, argument)
        .map_err(|error| format!("falha ao executar {export_name}: {error}"))
}

/// Calls an analysis/transform export with `(i32) -> i64` and reads a bounded
/// byte buffer from the module's exported `memory`. The return value packs the
/// pointer in the high 32 bits and the length in the low 32 bits.
#[cfg(not(target_arch = "wasm32"))]
pub fn invoke_wasm_bytes(
    extension: &DiscoveredExtension,
    policy: &ExtensionPermissionPolicy,
    export_name: &str,
    argument: i32,
    parameters: &serde_json::Map<String, serde_json::Value>,
) -> Result<Vec<u8>, String> {
    use wasmi::{Config, Engine, Linker, Module};

    policy.check(&extension.manifest)?;
    ensure_platform_compatible(&extension.manifest)?;
    extension.manifest.validate_parameter_values(parameters)?;
    let entrypoint = resolve_entrypoint(&extension.manifest_path, &extension.manifest)?;
    let bytes = std::fs::read(&entrypoint)
        .map_err(|error| format!("não foi possível ler {}: {error}", entrypoint.display()))?;
    let mut config = Config::default();
    config.consume_fuel(true);
    let engine = Engine::new(&config);
    let module = Module::new(&engine, &mut bytes.as_slice())
        .map_err(|error| format!("módulo WASM inválido: {error}"))?;
    let mut store = sandbox_store(&engine)?;
    let linker = Linker::<SandboxState>::new(&engine);
    let instance = linker
        .instantiate(&mut store, &module)
        .map_err(|error| format!("imports WASM não permitidos: {error}"))?
        .start(&mut store)
        .map_err(|error| format!("falha ao iniciar extensão WASM: {error}"))?;
    let api_version = instance
        .get_typed_func::<(), i32>(&store, "kamafeu_extension_api_version")
        .map_err(|_| "extensão não exporta a versão da ABI".to_string())?
        .call(&mut store, ())
        .map_err(|error| format!("falha ao consultar ABI: {error}"))?;
    if api_version != EXTENSION_API_VERSION as i32 {
        return Err(format!(
            "extensão declara API {api_version}, host requer {EXTENSION_API_VERSION}"
        ));
    }
    let packed = instance
        .get_typed_func::<i32, i64>(&store, export_name)
        .map_err(|_| format!("extensão não exporta {export_name}(i32) -> i64"))?
        .call(&mut store, argument)
        .map_err(|error| format!("falha ao executar {export_name}: {error}"))?
        as u64;
    let pointer = (packed >> 32) as usize;
    let length = (packed & u64::from(u32::MAX)) as usize;
    if length > WASM_OUTPUT_LIMIT_BYTES {
        return Err(format!(
            "saída WASM excede o limite de {} bytes",
            WASM_OUTPUT_LIMIT_BYTES
        ));
    }
    let memory = instance
        .get_memory(&store, "memory")
        .ok_or_else(|| "extensão não exporta memória linear 'memory'".to_string())?;
    let mut output = vec![0_u8; length];
    memory
        .read(&store, pointer, &mut output)
        .map_err(|error| format!("buffer WASM fora da memória exportada: {error}"))?;
    Ok(output)
}

#[cfg(not(target_arch = "wasm32"))]
fn ensure_platform_compatible(manifest: &ExtensionManifest) -> Result<(), String> {
    let platform = super::current_platform();
    if manifest.supports_platform(platform) {
        Ok(())
    } else {
        Err(format!(
            "extensão '{}' não suporta a plataforma atual '{}'; declaradas: {:?}",
            manifest.id, platform, manifest.platforms
        ))
    }
}

#[cfg(target_arch = "wasm32")]
pub fn verify_wasm_extension(
    _extension: &DiscoveredExtension,
) -> Result<WasmExtensionInfo, String> {
    Err("o host de extensões WASM ainda não está disponível na compilação web".to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn verify_wasm_extension_with_policy(
    _extension: &DiscoveredExtension,
    _policy: &ExtensionPermissionPolicy,
) -> Result<WasmExtensionInfo, String> {
    Err("o host de extensões WASM ainda não está disponível na compilação web".to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn invoke_wasm_i32(
    _extension: &DiscoveredExtension,
    _policy: &ExtensionPermissionPolicy,
    _export_name: &str,
    _argument: i32,
) -> Result<i32, String> {
    Err("o host de extensões WASM ainda não está disponível na compilação web".to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn invoke_wasm_i32_with_parameters(
    _extension: &DiscoveredExtension,
    _policy: &ExtensionPermissionPolicy,
    _export_name: &str,
    _argument: i32,
    _parameters: &serde_json::Map<String, serde_json::Value>,
) -> Result<i32, String> {
    Err("o host de extensões WASM ainda não está disponível na compilação web".to_string())
}

#[cfg(target_arch = "wasm32")]
pub fn invoke_wasm_bytes(
    _extension: &DiscoveredExtension,
    _policy: &ExtensionPermissionPolicy,
    _export_name: &str,
    _argument: i32,
    _parameters: &serde_json::Map<String, serde_json::Value>,
) -> Result<Vec<u8>, String> {
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
    use crate::extensions::{
        ExtensionKind, ExtensionManifest, ExtensionParameter, ExtensionPermissionPolicy,
        EXTENSION_API_VERSION,
    };

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
            abi_min_version: None,
            abi_max_version: None,
            kind: ExtensionKind::Effect,
            capabilities: Default::default(),
            formats: Default::default(),
            phonemizers: Default::default(),
            permissions: Default::default(),
            platforms: Default::default(),
            limitations: Default::default(),
            parameters: Default::default(),
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
                abi_min_version: None,
                abi_max_version: None,
                kind: ExtensionKind::Effect,
                capabilities: Default::default(),
                formats: Default::default(),
                phonemizers: Default::default(),
                permissions: Default::default(),
                platforms: Default::default(),
                limitations: Default::default(),
                parameters: Default::default(),
                description: String::new(),
                entrypoint: Some("extension.wasm".to_string()),
            },
        };
        let error = verify_wasm_extension(&extension).expect_err("API mismatch");
        assert!(error.contains("declara API 2"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn wasm_activation_requires_explicit_manifest_permissions() {
        let directory = tempfile::tempdir().expect("temporary extension directory");
        let manifest_path = directory.path().join("kamafeu-extension.json");
        let entrypoint = directory.path().join("extension.wasm");
        let module = [
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x01, 0x60, 0x00, 0x01,
            0x7f, 0x03, 0x02, 0x01, 0x00, 0x07, 0x21, 0x01, 0x1d, b'k', b'a', b'm', b'a', b'f',
            b'e', b'u', b'_', b'e', b'x', b't', b'e', b'n', b's', b'i', b'o', b'n', b'_', b'a',
            b'p', b'i', b'_', b'v', b'e', b'r', b's', b'i', b'o', b'n', 0x00, 0x00, 0x0a, 0x06,
            0x01, 0x04, 0x00, 0x41, 0x01, 0x0b,
        ];
        std::fs::write(&entrypoint, module).expect("WASM module");
        let mut permissions = std::collections::BTreeSet::new();
        permissions.insert("read_voicebank".to_string());
        let mut extension = DiscoveredExtension {
            manifest_path,
            manifest: ExtensionManifest {
                id: "org.kamafeu.permission-test".to_string(),
                name: "Permission test".to_string(),
                version: "0.1.0".to_string(),
                api_version: EXTENSION_API_VERSION,
                abi_min_version: None,
                abi_max_version: None,
                kind: ExtensionKind::Phonemizer,
                capabilities: Default::default(),
                formats: Default::default(),
                phonemizers: Default::default(),
                permissions,
                platforms: Default::default(),
                limitations: Default::default(),
                parameters: Default::default(),
                description: String::new(),
                entrypoint: Some("extension.wasm".to_string()),
            },
        };
        let denied =
            verify_wasm_extension_with_policy(&extension, &ExtensionPermissionPolicy::default())
                .expect_err("permission must be denied");
        assert!(denied.contains("read_voicebank"));

        let mut policy = ExtensionPermissionPolicy::default();
        policy.granted.insert("read_voicebank".to_string());
        assert!(verify_wasm_extension_with_policy(&extension, &policy).is_ok());

        extension.manifest.platforms.insert("wasm".to_string());
        let platform_error = verify_wasm_extension_with_policy(&extension, &policy)
            .expect_err("desktop host must reject a wasm-only extension");
        assert!(platform_error.contains("plataforma atual"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn invocation_rejects_an_export_with_the_wrong_signature() {
        let directory = tempfile::tempdir().expect("temporary extension directory");
        let manifest_path = directory.path().join("kamafeu-extension.json");
        let entrypoint = directory.path().join("extension.wasm");
        // Minimal module exporting only kamafeu_extension_api_version() -> i32.
        let module = [
            0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00, 0x01, 0x05, 0x01, 0x60, 0x00, 0x01,
            0x7f, 0x03, 0x02, 0x01, 0x00, 0x07, 0x21, 0x01, 0x1d, b'k', b'a', b'm', b'a', b'f',
            b'e', b'u', b'_', b'e', b'x', b't', b'e', b'n', b's', b'i', b'o', b'n', b'_', b'a',
            b'p', b'i', b'_', b'v', b'e', b'r', b's', b'i', b'o', b'n', 0x00, 0x00, 0x0a, 0x06,
            0x01, 0x04, 0x00, 0x41, 0x01, 0x0b,
        ];
        std::fs::write(&entrypoint, module).expect("WASM module");
        let extension = DiscoveredExtension {
            manifest_path,
            manifest: ExtensionManifest {
                id: "org.kamafeu.signature-test".to_string(),
                name: "Signature test".to_string(),
                version: "0.1.0".to_string(),
                api_version: EXTENSION_API_VERSION,
                abi_min_version: None,
                abi_max_version: None,
                kind: ExtensionKind::Analysis,
                capabilities: Default::default(),
                formats: Default::default(),
                phonemizers: Default::default(),
                permissions: Default::default(),
                platforms: Default::default(),
                limitations: Default::default(),
                parameters: Default::default(),
                description: String::new(),
                entrypoint: Some("extension.wasm".to_string()),
            },
        };
        let mut parameterized = extension.clone();
        parameterized.manifest.parameters.insert(
            "required_value".to_string(),
            ExtensionParameter {
                kind: "integer".to_string(),
                default: None,
                minimum: Some(0.0),
                maximum: Some(10.0),
                required: true,
            },
        );
        let contract_error = invoke_wasm_i32_with_parameters(
            &parameterized,
            &ExtensionPermissionPolicy::default(),
            "kamafeu_extension_api_version",
            1,
            &serde_json::Map::new(),
        )
        .expect_err("required parameters must be checked before WASM execution");
        assert!(contract_error.contains("obrigatório ausente"));
        let error = invoke_wasm_i32(
            &extension,
            &ExtensionPermissionPolicy::default(),
            "kamafeu_extension_api_version",
            1,
        )
        .expect_err("the version export has no i32 parameter");
        assert!(error.contains("kamafeu_extension_api_version"));

        let bytes_error = invoke_wasm_bytes(
            &extension,
            &ExtensionPermissionPolicy::default(),
            "kamafeu_extension_api_version",
            1,
            &serde_json::Map::new(),
        )
        .expect_err("the version export does not implement the bytes ABI");
        assert!(bytes_error.contains("kamafeu_extension_api_version"));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn bytes_invocation_reads_the_plugin_buffer_with_bounds() {
        let directory = tempfile::tempdir().expect("temporary extension directory");
        let manifest_path = directory.path().join("kamafeu-extension.json");
        let entrypoint = directory.path().join("extension.wasm");
        let module = wat::parse_str(
            r#"(module
                (memory (export "memory") 1)
                (data (i32.const 0) "hello")
                (func (export "kamafeu_extension_api_version") (result i32) i32.const 1)
                (func (export "analyze") (param i32) (result i64) i64.const 5)
            )"#,
        )
        .expect("valid WAT fixture");
        std::fs::write(&entrypoint, module).expect("WASM module");
        let extension = DiscoveredExtension {
            manifest_path,
            manifest: ExtensionManifest {
                id: "org.kamafeu.bytes-test".to_string(),
                name: "Bytes test".to_string(),
                version: "0.1.0".to_string(),
                api_version: EXTENSION_API_VERSION,
                abi_min_version: None,
                abi_max_version: None,
                kind: ExtensionKind::Analysis,
                capabilities: Default::default(),
                formats: Default::default(),
                phonemizers: Default::default(),
                permissions: Default::default(),
                platforms: Default::default(),
                limitations: Default::default(),
                parameters: Default::default(),
                description: String::new(),
                entrypoint: Some("extension.wasm".to_string()),
            },
        };
        let output = invoke_wasm_bytes(
            &extension,
            &ExtensionPermissionPolicy::default(),
            "analyze",
            0,
            &serde_json::Map::new(),
        )
        .expect("bytes ABI invocation");
        assert_eq!(output, b"hello");

        let oversized_module = wat::parse_str(
            r#"(module
                (func (export "kamafeu_extension_api_version") (result i32) i32.const 1)
                (func (export "analyze") (param i32) (result i64) i64.const 1048577)
            )"#,
        )
        .expect("valid oversized WAT fixture");
        std::fs::write(&entrypoint, oversized_module).expect("oversized WASM module");
        let error = invoke_wasm_bytes(
            &extension,
            &ExtensionPermissionPolicy::default(),
            "analyze",
            0,
            &serde_json::Map::new(),
        )
        .expect_err("oversized plugin output must be rejected");
        assert!(error.contains("excede o limite"));
    }
}
