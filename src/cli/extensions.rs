use super::*;

pub(super) fn extension_catalog_json(
    path: &PathBuf,
    verify_wasm: bool,
    grant_permissions: &[String],
    format_filter: Option<&str>,
    phonemizer_filter: Option<&str>,
) -> Value {
    let discovery = discover(path);
    let built_ins = built_in_capabilities();
    let policy = permission_policy(grant_permissions.to_vec());
    let matches_filter = |extension: &&DiscoveredExtension| {
        format_filter.is_none_or(|value| extension.manifest.formats.contains(value))
            && phonemizer_filter.is_none_or(|value| extension.manifest.phonemizers.contains(value))
    };
    let filtered_extensions = discovery
        .extensions
        .iter()
        .filter(matches_filter)
        .cloned()
        .collect::<Vec<_>>();
    json!({
        "path": path,
        "format_filter": format_filter,
        "phonemizer_filter": phonemizer_filter,
        "built_in_capabilities": built_ins.iter().map(|capability| json!({
            "id": capability.id,
            "name": capability.name,
            "kind": capability.kind,
            "description": capability.description,
        })).collect::<Vec<_>>(),
        "extensions": filtered_extensions.iter().map(|extension| json!({
            "manifest_path": extension.manifest_path,
            "manifest_fingerprint": extension
                .render_identity()
                .ok()
                .map(|identity| identity.manifest_fingerprint),
            "id": extension.manifest.id,
            "name": extension.manifest.name,
            "version": extension.manifest.version,
            "api_version": extension.manifest.api_version,
            "kind": extension.manifest.kind,
            "capabilities": extension.manifest.capabilities,
            "formats": extension.manifest.formats,
            "phonemizers": extension.manifest.phonemizers,
            "permissions": extension.manifest.permissions,
            "platforms": extension.manifest.platforms,
            "limitations": extension.manifest.limitations,
            "parameters": extension.manifest.parameters,
            "entrypoint": extension.manifest.entrypoint,
            "entrypoint_status": extension_entrypoint_status(extension),
            "platform_compatible": extension
                .manifest
                .supports_platform(kamafeu::extensions::current_platform()),
            "description": extension.manifest.description,
        })).collect::<Vec<_>>(),
        "diagnostics": discovery.diagnostics,
        "diagnostic_records": discovery.diagnostic_records,
        "wasm_verification": verify_wasm.then(|| wasm_verification_json(&filtered_extensions, &policy)),
    })
}

pub(super) fn invoke_extension_json(
    path: &std::path::Path,
    id: &str,
    export_name: &str,
    argument: i32,
    bytes: bool,
    granted_permissions: &[String],
    parameters: &serde_json::Map<String, Value>,
) -> Result<Value, Box<dyn std::error::Error>> {
    let discovery = discover(path);
    let extension = discovery
        .extensions
        .iter()
        .find(|extension| extension.manifest.id == id)
        .ok_or_else(|| {
            Error::new(
                ErrorKind::NotFound,
                format!("extensão não encontrada: {id}"),
            )
        })?;
    let policy = permission_policy(granted_permissions.to_vec());
    if bytes {
        let output = kamafeu::extensions::invoke_wasm_bytes(
            extension,
            &policy,
            export_name,
            argument,
            parameters,
        )
        .map_err(Error::other)?;
        Ok(json!({
            "id": id,
            "export": export_name,
            "argument": argument,
            "bytes_length": output.len(),
            "utf8": String::from_utf8_lossy(&output),
            "hex": output.iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
        }))
    } else {
        let output = kamafeu::extensions::invoke_wasm_i32_with_parameters(
            extension,
            &policy,
            export_name,
            argument,
            parameters,
        )
        .map_err(Error::other)?;
        Ok(json!({
            "id": id,
            "export": export_name,
            "argument": argument,
            "result": output,
        }))
    }
}

pub(super) fn extension_entrypoint_status(extension: &DiscoveredExtension) -> &'static str {
    let Some(entrypoint) = extension.manifest.entrypoint.as_deref() else {
        return "not_declared";
    };
    if !entrypoint.to_ascii_lowercase().ends_with(".wasm") {
        return "not_wasm";
    }
    let Some(root) = extension.manifest_path.parent() else {
        return "missing";
    };
    if root.join(entrypoint).is_file() {
        "available"
    } else {
        "missing"
    }
}

pub(super) fn permission_policy(granted: Vec<String>) -> ExtensionPermissionPolicy {
    ExtensionPermissionPolicy {
        granted: granted.into_iter().collect(),
    }
}

pub(super) fn parse_extension_parameters(
    values: &[String],
) -> Result<serde_json::Map<String, serde_json::Value>, Box<dyn std::error::Error>> {
    let mut parameters = serde_json::Map::new();
    for value in values {
        let (name, raw) = value.split_once('=').ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("parâmetro de extensão inválido '{value}'; use chave=valor JSON"),
            )
        })?;
        if name.trim().is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput, "nome de parâmetro vazio").into());
        }
        let parsed = serde_json::from_str(raw).unwrap_or_else(|_| Value::String(raw.to_string()));
        if parameters.insert(name.to_string(), parsed).is_some() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!("parâmetro de extensão duplicado: '{name}'"),
            )
            .into());
        }
    }
    Ok(parameters)
}

pub(super) fn wasm_verification_json(
    extensions: &[DiscoveredExtension],
    policy: &ExtensionPermissionPolicy,
) -> Vec<Value> {
    extensions
        .iter()
        .filter(|extension| {
            extension
                .manifest
                .entrypoint
                .as_deref()
                .is_some_and(|entrypoint| entrypoint.to_ascii_lowercase().ends_with(".wasm"))
        })
        .map(
            |extension| match verify_wasm_extension_with_policy(extension, policy) {
                Ok(info) => json!({
                    "id": info.id,
                    "ok": true,
                    "entrypoint": info.entrypoint,
                    "api_version": info.api_version,
                }),
                Err(error) => json!({ "id": extension.manifest.id, "ok": false, "error": error }),
            },
        )
        .collect()
}
