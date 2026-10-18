use super::*;

/// Minimal MCP server backed by the same safe project operations used by the
/// automation protocol. It intentionally has no network listener: the caller
/// owns process creation and filesystem permissions.
pub(super) fn run_mcp() -> Result<(), Box<dyn std::error::Error>> {
    for line in std::io::stdin().lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let message: Value = match serde_json::from_str(&line) {
            Ok(message) => message,
            Err(error) => {
                println!(
                    "{}",
                    serde_json::to_string(&mcp_error(
                        Value::Null,
                        -32700,
                        &format!("parse error: {error}")
                    ))?
                );
                continue;
            }
        };
        let id = message.get("id").cloned();
        let method = message.get("method").and_then(Value::as_str);
        let response = match method {
            Some("initialize") => Some(mcp_result(
                id.unwrap_or(Value::Null),
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": { "listChanged": false } },
                    "serverInfo": { "name": "kamafeu-studio", "version": kamafeu::APP_VERSION },
                }),
            )),
            Some("notifications/initialized") => None,
            Some("tools/list") => Some(mcp_result(
                id.unwrap_or(Value::Null),
                json!({ "tools": mcp_tools() }),
            )),
            Some("tools/call") => Some(mcp_tool_call(
                id.unwrap_or(Value::Null),
                message.get("params").cloned().unwrap_or(Value::Null),
            )),
            Some(method) => {
                id.map(|id| mcp_error(id, -32601, &format!("method not found: {method}")))
            }
            None => id.map(|id| mcp_error(id, -32600, "invalid JSON-RPC request")),
        };
        if let Some(response) = response {
            println!("{}", serde_json::to_string(&response)?);
        }
    }
    Ok(())
}

fn mcp_result(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn mcp_error(id: Value, code: i32, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

pub(super) fn mcp_tool_call(id: Value, params: Value) -> Value {
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return mcp_error(id, -32602, "tools/call requires params.name");
    };
    let Some(command) = name.strip_prefix("kamafeu_") else {
        return mcp_tool_result(id, Err(format!("unknown Kamafeu tool: {name}")));
    };
    let mut arguments = params
        .get("arguments")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    arguments.insert("command".to_string(), Value::String(command.to_string()));
    let request = serde_json::from_value::<AutomationRequest>(Value::Object(arguments))
        .map_err(|error| format!("invalid arguments for {name}: {error}"));
    let result = request.and_then(|request| {
        automation::handle_automation_request(request).map_err(|error| error.to_string())
    });
    mcp_tool_result(id, result)
}

fn mcp_tool_result(id: Value, result: Result<Value, String>) -> Value {
    match result {
        Ok(result) => mcp_result(
            id,
            json!({
                "content": [{ "type": "text", "text": serde_json::to_string_pretty(&result).unwrap_or_default() }],
                "structuredContent": result,
                "isError": false,
            }),
        ),
        Err(error) => mcp_result(
            id,
            json!({
                "content": [{ "type": "text", "text": error }],
                "isError": true,
            }),
        ),
    }
}

pub(super) fn mcp_tools() -> Vec<Value> {
    [
        (
            "project_info",
            "Read project metadata, tracks, notes and duration.",
        ),
        (
            "diff",
            "Compare two project revisions without modifying either input.",
        ),
        (
            "list_voicebanks",
            "Scan singer folders and list UTAU and OpenUTAU voicebanks.",
        ),
        (
            "validate_project",
            "Check a project for invalid tempo, tracks, notes and audio parts.",
        ),
        (
            "create_project",
            "Create a new APS, USTX, UST, MIDI, VSQX, SVP, UFData or JSON project.",
        ),
        (
            "add_note",
            "Add a vocal note to a project and save to an explicit output path.",
        ),
        ("set_note", "Change lyric and/or pitch of a specific note."),
        (
            "remove_note",
            "Remove one note and save the resulting project to an explicit output path.",
        ),
        (
            "apply_lyrics",
            "Split and distribute lyrics across a vocal part in musical time order.",
        ),
        (
            "add_marker",
            "Add a named arrangement marker at an explicit timeline position.",
        ),
        (
            "remove_marker",
            "Remove a project marker and save the resulting project to an explicit output path.",
        ),
        (
            "add_section",
            "Add a named arrangement section with explicit start and end positions.",
        ),
        (
            "remove_section",
            "Remove a project section and save the resulting project to an explicit output path.",
        ),
        (
            "set_pitch_bend",
            "Replace a note pitch-bend curve with time/cents points.",
        ),
        (
            "set_expression",
            "Set vocal expressions such as dynamics, breathiness and gender.",
        ),
        (
            "add_audio_part",
            "Add an existing audio file to the arrangement.",
        ),
        (
            "render",
            "Render a project with a voicebank to WAV, FLAC or RAW.",
        ),
        ("validate_voicebank", "Validate UTAU voicebank assets."),
        (
            "extensions",
            "List extensions and optionally verify their WASM modules.",
        ),
        ("cache_info", "Inspect renderer cache location and usage."),
        ("cache_prune", "Remove stale renderer-cache entries."),
    ]
    .into_iter()
    .map(|(name, description)| {
        json!({
            "name": format!("kamafeu_{name}"),
            "description": description,
            "inputSchema": { "type": "object", "additionalProperties": true },
        })
    })
    .collect()
}
