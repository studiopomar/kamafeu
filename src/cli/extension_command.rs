use super::*;

pub(super) fn handle(
    command: Commands,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        Commands::Extensions {
            path,
            verify_wasm,
            grant_permissions,
            format_filter,
            phonemizer_filter,
            invoke_id,
            invoke_export,
            invoke_argument,
            invoke_bytes,
            invoke_parameters,
        } => {
            let discovery = discover(&path);
            let built_ins = built_in_capabilities();
            let policy = extensions::permission_policy(grant_permissions);
            if let Some(invoke_id) = invoke_id {
                let extension = discovery
                    .extensions
                    .iter()
                    .find(|extension| extension.manifest.id == invoke_id)
                    .ok_or_else(|| {
                        Error::new(
                            ErrorKind::NotFound,
                            format!("extensão não encontrada: {invoke_id}"),
                        )
                    })?;
                let parameters = extensions::parse_extension_parameters(&invoke_parameters)?;
                if invoke_bytes {
                    let output = kamafeu::extensions::invoke_wasm_bytes(
                        extension,
                        &policy,
                        &invoke_export,
                        invoke_argument,
                        &parameters,
                    )
                    .map_err(Error::other)?;
                    print_json(&json!({
                        "id": invoke_id,
                        "export": invoke_export,
                        "argument": invoke_argument,
                        "bytes_length": output.len(),
                        "utf8": String::from_utf8_lossy(&output),
                        "hex": output.iter().map(|byte| format!("{byte:02x}")).collect::<String>(),
                    }))?;
                } else {
                    let output = kamafeu::extensions::invoke_wasm_i32_with_parameters(
                        extension,
                        &policy,
                        &invoke_export,
                        invoke_argument,
                        &parameters,
                    )
                    .map_err(Error::other)?;
                    print_json(&json!({
                        "id": invoke_id,
                        "export": invoke_export,
                        "argument": invoke_argument,
                        "result": output,
                    }))?;
                }
                return Ok(());
            }
            let matches_filter = |extension: &&DiscoveredExtension| {
                format_filter
                    .as_deref()
                    .is_none_or(|value| extension.manifest.formats.contains(value))
                    && phonemizer_filter
                        .as_deref()
                        .is_none_or(|value| extension.manifest.phonemizers.contains(value))
            };
            let wasm_verification = verify_wasm
                .then(|| extensions::wasm_verification_json(&discovery.extensions, &policy));
            if json_output {
                print_json(&json!({
                    "path": path,
                    "built_in_capabilities": built_ins.iter().map(|capability| json!({
                        "id": capability.id,
                        "name": capability.name,
                        "kind": capability.kind,
                        "description": capability.description,
                    })).collect::<Vec<_>>(),
                    "format_filter": format_filter,
                    "phonemizer_filter": phonemizer_filter,
                    "extensions": discovery.extensions.iter().filter(matches_filter).map(|extension| json!({
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
                        "entrypoint_status": extensions::extension_entrypoint_status(extension),
                        "platform_compatible": extension
                            .manifest
                            .supports_platform(kamafeu::extensions::current_platform()),
                        "description": extension.manifest.description,
                    })).collect::<Vec<_>>(),
                    "diagnostics": discovery.diagnostics,
                    "diagnostic_records": discovery.diagnostic_records,
                    "wasm_verification": wasm_verification,
                }))?;
            } else {
                kamafeu::terminal_log::out_header("Kamafeu extensions", path.display());
                kamafeu::terminal_log::out_status("INFO", "Built-in", "capabilities");
                for capability in built_ins {
                    kamafeu::terminal_log::out_list(format!(
                        "{} ({:?}) [{}]",
                        capability.name, capability.kind, capability.id
                    ));
                }
                if discovery.extensions.is_empty() {
                    kamafeu::terminal_log::out_status("INFO", "Extensions", "none found");
                }
                for extension in discovery.extensions.iter().filter(matches_filter) {
                    kamafeu::terminal_log::out_status(
                        "OK",
                        &extension.manifest.name,
                        format!(
                            "{} ({:?}) [{}]",
                            extension.manifest.version,
                            extension.manifest.kind,
                            extension.manifest.id
                        ),
                    );
                    kamafeu::terminal_log::out_kv(
                        "Entrypoint",
                        format!(
                            "{} ({})",
                            extension
                                .manifest
                                .entrypoint
                                .as_deref()
                                .unwrap_or("não declarado"),
                            extensions::extension_entrypoint_status(extension)
                        ),
                    );
                    if !extension.manifest.limitations.is_empty() {
                        kamafeu::terminal_log::out_kv(
                            "Limitations",
                            extension.manifest.limitations.join("; "),
                        );
                    }
                }
                if !discovery.diagnostics.is_empty() {
                    kamafeu::terminal_log::out_status("WARN", "Diagnostics", "extension issues");
                    for diagnostic in discovery.diagnostics {
                        kamafeu::terminal_log::out_list(diagnostic);
                    }
                }
                if let Some(results) = wasm_verification {
                    kamafeu::terminal_log::out_status("INFO", "WASM", "verification");
                    for result in results {
                        let id = result["id"].as_str().unwrap_or("unknown");
                        if result["ok"].as_bool() == Some(true) {
                            kamafeu::terminal_log::out_status("OK", id, "verified");
                        } else {
                            kamafeu::terminal_log::out_status(
                                "ERROR",
                                id,
                                result["error"].as_str().unwrap_or("unknown error"),
                            );
                        }
                    }
                }
            }
        }

        _ => unreachable!("extension command dispatcher received another command"),
    }
    Ok(())
}
