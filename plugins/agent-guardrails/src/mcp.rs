//! Bounded stdio JSON-RPC 2.0 MCP server exposing the four `guardrails_*` tools.
use crate::{
    counters, custody,
    error::{GuardError, Result},
    guards, policy,
    presets::Preset,
};
use serde_json::{Value, json, value::RawValue};
use std::{
    collections::BTreeMap,
    io::{BufRead, Read, Write},
    path::{Path, PathBuf},
};

pub const PROTOCOL_VERSION: &str = "2025-06-18";
pub const MAX_MESSAGE: usize = 256 * 1024;
pub const MAX_DEPTH: usize = 64;

/// Tool declarations returned by `tools/list`.
pub fn tools() -> Vec<Value> {
    let spec = |name: &str, description: &str, properties: Value, required: Vec<&str>, read: bool| {
        json!({
            "name": name,
            "description": description,
            "inputSchema": {
                "type": "object",
                "properties": properties,
                "required": required,
                "additionalProperties": false
            },
            "annotations": {
                "readOnlyHint": read,
                "destructiveHint": false,
                "openWorldHint": false
            }
        })
    };
    vec![
        spec(
            "guardrails_check_command",
            "Inspect a shell command using the POSIX AST shell guard and active preset/policy without executing it.",
            json!({
                "command": {"type": "string", "description": "Shell command line to validate."},
                "root": {"type": "string", "description": "Project root path (defaults to server cwd)."},
                "preset": {"type": "string", "enum": Preset::PUBLIC_NAMES, "description": "Optional preset override."}
            }),
            vec!["command"],
            true,
        ),
        spec(
            "guardrails_inspect_policy",
            "Inspect the effective guardrails policy, active preset, rule counts, loop thresholds, naming rules, and subagent gate settings.",
            json!({
                "root": {"type": "string", "description": "Project root path (defaults to server cwd)."},
                "preset": {"type": "string", "enum": Preset::PUBLIC_NAMES, "description": "Optional preset override."}
            }),
            vec![],
            true,
        ),
        spec(
            "guardrails_init_policy",
            "Initialize .agent-guardrails/policy.json with a selectable preset (generic, laravel, symfony, wordpress, node, python, rust).",
            json!({
                "preset": {"type": "string", "enum": Preset::PUBLIC_NAMES, "description": "Rule preset to initialize."},
                "root": {"type": "string", "description": "Project root path (defaults to server cwd)."},
                "force": {"type": "boolean", "description": "Overwrite an existing policy file."}
            }),
            vec!["preset"],
            false,
        ),
        spec(
            "guardrails_loop_status",
            "Inspect (or reset) $TMPDIR session command/edit loop counters and active subagent write locks for a repository.",
            json!({
                "root": {"type": "string", "description": "Project root path (defaults to server cwd)."},
                "session_id": {"type": "string", "description": "Session identifier (defaults to 'shared')."},
                "reset": {"type": "boolean", "description": "Clear session loop counters before reporting status."}
            }),
            vec![],
            false,
        ),
    ]
}

fn validate_args(spec: &Value, input: &Value) -> Result<()> {
    let obj = input
        .as_object()
        .ok_or_else(|| GuardError::invalid("Tool arguments must be a JSON object"))?;
    let props = spec["inputSchema"]["properties"]
        .as_object()
        .ok_or_else(|| GuardError::invalid("Invalid tool schema"))?;
    if obj.keys().any(|k| !props.contains_key(k)) {
        return Err(GuardError::invalid("Unknown tool argument supplied"));
    }
    if spec["inputSchema"]["required"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .any(|req| !obj.contains_key(req))
    {
        return Err(GuardError::invalid("Missing required tool argument"));
    }
    Ok(())
}

/// Dispatch one MCP tool invocation.
pub fn call_tool(default_root: &Path, name: &str, input: &Value) -> Result<Value> {
    let spec = tools()
        .into_iter()
        .find(|s| s["name"] == name)
        .ok_or_else(|| GuardError::invalid(format!("Unknown guardrails tool: {name}")))?;
    validate_args(&spec, input)?;

    let explicit_root = input
        .get("root")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty());
    let plugin_cwd_without_root =
        explicit_root.is_none() && custody::inside_plugin_dir(default_root);

    let root = explicit_root
        .map(PathBuf::from)
        .map(|p| custody::discover_repo_root(&p))
        .unwrap_or_else(|| default_root.to_path_buf());

    match name {
        "guardrails_check_command" => {
            let cmd = input["command"]
                .as_str()
                .ok_or_else(|| GuardError::invalid("command must be a string"))?;
            let preset = input
                .get("preset")
                .and_then(Value::as_str)
                .map(Preset::parse)
                .transpose()?;
            guards::check_command(&root, cmd, preset)
        }
        "guardrails_inspect_policy" => {
            let preset = input
                .get("preset")
                .and_then(Value::as_str)
                .map(Preset::parse)
                .transpose()?;
            let mut val = policy::inspect_policy(&root, preset)?;
            if plugin_cwd_without_root {
                val["warning"] = json!(
                    "MCP server cwd is inside the plugin directory; pass 'root' explicitly set to your target repository path."
                );
            }
            Ok(val)
        }
        "guardrails_init_policy" => {
            if plugin_cwd_without_root {
                return Err(GuardError::invalid(
                    "Missing 'root' argument: MCP server cwd is inside the plugin directory; pass 'root' set to the target repository path.",
                ));
            }
            let preset_raw = input["preset"]
                .as_str()
                .ok_or_else(|| GuardError::invalid("preset must be a string"))?;
            let preset = Preset::parse(preset_raw)?;
            let force = input.get("force").and_then(Value::as_bool).unwrap_or(false);
            let path = policy::init_policy(&root, preset, force)?;
            Ok(json!({
                "initialized": true,
                "preset": preset.as_str(),
                "policy_path": path.display().to_string(),
            }))
        }
        "guardrails_loop_status" => {
            let session = input.get("session_id").and_then(Value::as_str);
            let reset = input.get("reset").and_then(Value::as_bool).unwrap_or(false);
            let removed = if reset {
                counters::reset(&root, &counters::sanitize_session(session))?
            } else {
                0
            };
            let status = counters::inspect_status(&root, session)?;
            let mut val = serde_json::to_value(status)?;
            if reset {
                val["reset_removed_counters"] = json!(removed);
            }
            if plugin_cwd_without_root {
                val["warning"] = json!(
                    "MCP server cwd is inside the plugin directory; pass 'root' explicitly set to your target repository path."
                );
            }
            Ok(val)
        }
        _ => Err(GuardError::invalid(format!("Unknown guardrails tool: {name}"))),
    }
}

/// Serve JSON-RPC 2.0 MCP messages line-by-line over stdio streams.
pub fn serve<R: BufRead, W: Write>(default_root: &Path, mut incoming: R, mut outgoing: W) -> Result<()> {
    let (mut initialized, mut ready) = (false, false);
    loop {
        let mut raw = Vec::new();
        let count = Read::take(&mut incoming, MAX_MESSAGE as u64 + 1)
            .read_until(b'\n', &mut raw)?;
        if count == 0 {
            break;
        }
        let oversized = raw.len() > MAX_MESSAGE;
        let mut identifier = "null".to_string();
        let outcome = (|| -> std::result::Result<Option<Value>, i64> {
            if oversized || too_deep(&raw) {
                return Err(-32700);
            }
            let members = rpc_members(&raw).map_err(|_| -32700)?;
            let version = members
                .get("jsonrpc")
                .and_then(|v| serde_json::from_str::<String>(v.get()).ok());
            let method = members
                .get("method")
                .and_then(|v| serde_json::from_str::<String>(v.get()).ok());
            if version.as_deref() != Some("2.0") || method.is_none() {
                return Err(-32600);
            }
            if let Some(id) = members.get("id") {
                let raw_id = id.get();
                if !valid_identifier(raw_id) {
                    return Err(-32600);
                }
                identifier = raw_id.into();
            }
            let method = method.unwrap_or_default();
            let params = members
                .get("params")
                .map(|v| {
                    custody::strict_parse_json(v.get().as_bytes(), MAX_MESSAGE, MAX_DEPTH, MAX_MESSAGE)
                })
                .transpose()
                .map_err(|_| -32700)?
                .unwrap_or_else(|| json!({}));
            if !params.is_object() || params.get("_meta").is_some_and(|v| !v.is_object()) {
                return Err(-32602);
            }
            if method == "notifications/initialized" && !members.contains_key("id") {
                ready = initialized;
                return Ok(None);
            }
            if !members.contains_key("id") {
                return Ok(None);
            }
            let value = match method.as_str() {
                "initialize" => {
                    if initialized || !params["protocolVersion"].is_string() {
                        return Err(-32602);
                    }
                    initialized = true;
                    json!({
                        "protocolVersion": PROTOCOL_VERSION,
                        "capabilities": {"tools": {}},
                        "serverInfo": {"name": "agent-guardrails", "version": env!("CARGO_PKG_VERSION")},
                        "instructions": "Always pass the target repository path in 'root' when invoking tools from plugin-scoped environments (such as Codex where cwd='.'). Use guardrails_check_command to validate dangerous shell pipelines safely without executing them, guardrails_inspect_policy to view active rules/thresholds, guardrails_init_policy to configure .agent-guardrails/policy.json, and guardrails_loop_status to inspect or reset session loop counters."
                    })
                }
                "ping" => json!({}),
                _ if !ready => return Err(-32602),
                "tools/list" => {
                    if params
                        .as_object()
                        .is_some_and(|map| map.keys().any(|key| key != "_meta"))
                    {
                        return Err(-32602);
                    }
                    json!({"tools": tools()})
                }
                "tools/call" => {
                    let name = params["name"].as_str().ok_or(-32602)?;
                    let arguments = params
                        .get("arguments")
                        .cloned()
                        .unwrap_or_else(|| json!({}));
                    match call_tool(default_root, name, &arguments) {
                        Ok(value) => json!({
                            "content": [{"type": "text", "text": serde_json::to_string_pretty(&value).map_err(|_| -32602)?}],
                            "structuredContent": value,
                            "isError": false
                        }),
                        Err(err) => json!({
                            "content": [{"type": "text", "text": err.message}],
                            "isError": true
                        }),
                    }
                }
                _ => return Err(-32601),
            };
            Ok(Some(json!({"result": value})))
        })();

        let response = match outcome {
            Ok(None) => continue,
            Ok(Some(value)) => format!(
                "{{\"jsonrpc\":\"2.0\",\"id\":{identifier},\"result\":{}}}\n",
                custody::canonical_json(&value["result"])?
            ),
            Err(code) => {
                let message = match code {
                    -32600 => "Invalid JSON-RPC request",
                    -32601 => "Method not found",
                    -32602 => "Invalid request parameters or initialization state",
                    _ => "Invalid JSON-RPC message",
                };
                format!(
                    "{{\"jsonrpc\":\"2.0\",\"id\":{identifier},\"error\":{{\"code\":{code},\"message\":\"{message}\"}}}}\n"
                )
            }
        };
        outgoing.write_all(response.as_bytes())?;
        outgoing.flush()?;
        if oversized {
            break;
        }
    }
    Ok(())
}

fn rpc_members(raw: &[u8]) -> std::result::Result<BTreeMap<String, Box<RawValue>>, ()> {
    use serde::{
        Deserialize, Deserializer,
        de::{Error as _, MapAccess, Visitor},
    };
    std::str::from_utf8(raw).map_err(|_| ())?;
    struct Envelope(BTreeMap<String, Box<RawValue>>);
    impl<'de> Deserialize<'de> for Envelope {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
            struct Fields;
            impl<'de> Visitor<'de> for Fields {
                type Value = Envelope;
                fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                    f.write_str("a JSON-RPC object")
                }
                fn visit_map<A: MapAccess<'de>>(
                    self,
                    mut input: A,
                ) -> std::result::Result<Envelope, A::Error> {
                    let mut members = BTreeMap::new();
                    while let Some(key) = input.next_key::<String>()? {
                        if members.contains_key(&key) {
                            return Err(A::Error::custom("Duplicate JSON-RPC field."));
                        }
                        let value = input.next_value::<Box<RawValue>>()?;
                        members.insert(key, value);
                    }
                    Ok(Envelope(members))
                }
            }
            deserializer.deserialize_map(Fields)
        }
    }
    let mut parser = serde_json::Deserializer::from_slice(raw);
    let Envelope(members) = Envelope::deserialize(&mut parser).map_err(|_| ())?;
    parser.end().map_err(|_| ())?;
    for (key, value) in &members {
        if !["id", "jsonrpc", "method"].contains(&key.as_str()) {
            custody::strict_parse_json(value.get().as_bytes(), MAX_MESSAGE, MAX_DEPTH, MAX_MESSAGE)
                .map_err(|_| ())?;
        }
    }
    Ok(members)
}

fn valid_identifier(value: &str) -> bool {
    if value == "null" {
        return true;
    }
    if value.starts_with('"') {
        return value.ends_with('"');
    }
    let digits = value.strip_prefix('-').unwrap_or(value);
    !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
}

fn too_deep(raw: &[u8]) -> bool {
    let (mut depth, mut string, mut escaped) = (0usize, false, false);
    for byte in raw {
        if string {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                string = false;
            }
        } else if *byte == b'"' {
            string = true;
        } else if *byte == b'[' || *byte == b'{' {
            depth += 1;
            if depth > MAX_DEPTH {
                return true;
            }
        } else if *byte == b']' || *byte == b'}' {
            depth = depth.saturating_sub(1);
        }
    }
    false
}
