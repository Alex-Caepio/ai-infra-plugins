use crate::{
    backends::{BackendKind, discover_all, locate_binary},
    paths::{env_configured_root, parse_file_uri, prepare_tool_arguments, restricted_mode},
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeSet, HashMap, VecDeque},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

const PROTOCOL_VERSION: &str = "2025-06-18";

struct BackendProcess {
    kind: BackendKind,
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
    tools: Vec<Value>,
}

impl BackendProcess {
    fn spawn(kind: BackendKind) -> Result<Self, String> {
        let bin = locate_binary(kind)?;
        let mut child = Command::new(&bin)
            .arg("mcp")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("Failed spawning `{} mcp`: {e}", bin.display()))?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| format!("{} missing stdin pipe", kind.binary_name()))?;
        let stdout = BufReader::new(
            child
                .stdout
                .take()
                .ok_or_else(|| format!("{} missing stdout pipe", kind.binary_name()))?,
        );
        let mut proc = Self {
            kind,
            child,
            stdin,
            stdout,
            next_id: 1,
            tools: Vec::new(),
        };
        proc.handshake()?;
        Ok(proc)
    }

    fn send_msg(&mut self, value: &Value) -> Result<(), String> {
        let line = serde_json::to_string(value).map_err(|e| format!("JSON encode error: {e}"))?;
        writeln!(self.stdin, "{line}")
            .and_then(|_| self.stdin.flush())
            .map_err(|e| format!("{} write failed: {e}", self.kind.binary_name()))
    }

    fn read_response(&mut self, expected_id: u64) -> Result<Value, String> {
        let mut line = String::new();
        loop {
            line.clear();
            let n = self
                .stdout
                .read_line(&mut line)
                .map_err(|e| format!("{} read failed: {e}", self.kind.binary_name()))?;
            if n == 0 {
                return Err(format!("{} exited unexpectedly", self.kind.binary_name()));
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let val: Value = serde_json::from_str(trimmed)
                .map_err(|e| format!("{} returned invalid JSON: {e}", self.kind.binary_name()))?;
            if val.get("id").and_then(Value::as_u64) == Some(expected_id) {
                return Ok(val);
            }
        }
    }

    fn rpc(&mut self, method: &str, params: Value) -> Result<Value, String> {
        let id = self.next_id;
        self.next_id += 1;
        self.send_msg(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }))?;
        let resp = self.read_response(id)?;
        if let Some(err) = resp.get("error") {
            return Err(format!("{} RPC error: {err}", self.kind.binary_name()));
        }
        Ok(resp.get("result").cloned().unwrap_or(Value::Null))
    }

    fn handshake(&mut self) -> Result<(), String> {
        let _ = self.rpc(
            "initialize",
            json!({
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": { "name": "ai-infra-mod", "version": env!("CARGO_PKG_VERSION") }
            }),
        )?;
        self.send_msg(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized"
        }))?;
        let list = self.rpc("tools/list", json!({}))?;
        let tools = list["tools"]
            .as_array()
            .ok_or_else(|| format!("{} tools/list missing `tools` array", self.kind.binary_name()))?
            .clone();
        self.tools = tools;
        Ok(())
    }

    fn call_tool(&mut self, name: &str, arguments: Value) -> Result<Value, String> {
        self.rpc(
            "tools/call",
            json!({
                "name": name,
                "arguments": arguments
            }),
        )
    }

    fn shutdown(&mut self) {
        let _ = self.stdin.flush();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for BackendProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[derive(Clone)]
struct ToolMeta {
    kind: BackendKind,
    backend_tool_name: String,
    schema_props: BTreeSet<String>,
}

pub struct Multiplexer {
    backends: HashMap<BackendKind, BackendProcess>,
    routes: HashMap<String, ToolMeta>,
    ordered_tools: Vec<Value>,
}

fn enrich_annotations(name: &str, existing: Option<&Value>) -> Value {
    let base_ro = existing
        .and_then(|v| v.get("readOnlyHint"))
        .and_then(Value::as_bool);
    let base_destr = existing
        .and_then(|v| v.get("destructiveHint"))
        .and_then(Value::as_bool);
    let base_open = existing
        .and_then(|v| v.get("openWorldHint"))
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let (ro, destr) = match (base_ro, base_destr) {
        (Some(r), Some(d)) => (r, d),
        _ => match name {
            "mod_status"
            | "system_catalog"
            | "system_locate"
            | "system_impact"
            | "system_run_status"
            | "system_graph_status"
            | "orch_session_status"
            | "orch_context_draft"
            | "orch_delivery_preview"
            | "orch_working_memory_read"
            | "omnigraph_status"
            | "omnigraph_atoms"
            | "omnigraph_branches_diff" => (true, false),
            "system_execute" | "orch_session_cancel" => (false, true),
            _ => (base_ro.unwrap_or(false), base_destr.unwrap_or(false)),
        },
    };
    json!({
        "readOnlyHint": ro,
        "destructiveHint": destr,
        "openWorldHint": base_open
    })
}

fn mod_status_tool() -> Value {
    json!({
        "name": "mod_status",
        "description": "Report availability, binary paths, versions, tool counts, and active root containment status across all 6 AI-Infrastructure subsystems (graph-map, system-orchestration, context-graph, skill-foundry, agent-guardrails, skillcache).",
        "inputSchema": {
            "type": "object",
            "properties": {
                "project": {
                    "type": "string",
                    "description": "Optional repository root to inspect."
                },
                "root": {
                    "type": "string",
                    "description": "Alias for `project`."
                }
            },
            "additionalProperties": false
        },
        "annotations": {
            "readOnlyHint": true,
            "destructiveHint": false,
            "openWorldHint": false
        }
    })
}

impl Multiplexer {
    pub fn start() -> Result<Self, String> {
        let mut results: Vec<Result<BackendProcess, String>> = thread::scope(|s| {
            let handles: Vec<_> = BackendKind::ALL
                .into_iter()
                .map(|kind| s.spawn(move || BackendProcess::spawn(kind)))
                .collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        });

        let mut backends = HashMap::new();
        let mut routes = HashMap::new();
        let mut ordered_tools = vec![mod_status_tool()];

        for res in results.drain(..) {
            let proc = res?;
            for mut tool in proc.tools.clone() {
                let backend_tool_name = tool["name"]
                    .as_str()
                    .ok_or_else(|| format!("{} returned nameless tool", proc.kind.binary_name()))?
                    .to_string();
                let schema_props = tool["inputSchema"]["properties"]
                    .as_object()
                    .map(|m| m.keys().cloned().collect::<BTreeSet<_>>())
                    .unwrap_or_default();
                let exposed_name = if routes.contains_key(&backend_tool_name) {
                    let aliased = format!("{}_{}", proc.kind.cli_namespace(), backend_tool_name);
                    if routes.contains_key(&aliased) {
                        return Err(format!("Duplicate MCP tool name `{aliased}` across backends"));
                    }
                    tool["name"] = json!(aliased);
                    let existing_desc = tool["description"].as_str().unwrap_or("").trim_end();
                    let tag = if proc.kind == BackendKind::Context {
                        "[context-graph-engine omnigraph.sqlite3]"
                    } else {
                        proc.kind.binary_name()
                    };
                    tool["description"] = if existing_desc.is_empty() {
                        json!(tag)
                    } else if proc.kind == BackendKind::Context {
                        json!(format!("{existing_desc} {tag}"))
                    } else {
                        json!(format!("{existing_desc} [{tag}]"))
                    };
                    aliased
                } else {
                    backend_tool_name.clone()
                };
                let ann = enrich_annotations(&exposed_name, tool.get("annotations"));
                tool["annotations"] = ann;
                routes.insert(
                    exposed_name,
                    ToolMeta {
                        kind: proc.kind,
                        backend_tool_name,
                        schema_props,
                    },
                );
                ordered_tools.push(tool);
            }
            backends.insert(proc.kind, proc);
        }

        Ok(Self {
            backends,
            routes,
            ordered_tools,
        })
    }

    pub fn tools_list(&self) -> Value {
        json!({ "tools": self.ordered_tools })
    }

    pub fn call_tool(
        &mut self,
        name: &str,
        args: Value,
        active_root: Option<&Path>,
        restricted_root: Option<&Path>,
    ) -> Value {
        if name == "mod_status" {
            return self.handle_mod_status(args, active_root, restricted_root);
        }
        let Some(meta) = self.routes.get(name).cloned() else {
            return err_result(format!("Unknown MCP tool `{name}`"));
        };
        let prepared = match prepare_tool_arguments(
            name,
            &meta.schema_props,
            args,
            active_root,
            restricted_root,
        ) {
            Ok(v) => v,
            Err(msg) => return err_result(msg),
        };
        let call_res = {
            let Some(backend) = self.backends.get_mut(&meta.kind) else {
                return err_result(format!("Backend `{}` is not running", meta.kind.id()));
            };
            match backend.call_tool(&meta.backend_tool_name, prepared.clone()) {
                Ok(v) => Ok(v),
                Err(_) => {
                    // Restart once on unexpected backend EOF/crash.
                    if let Ok(mut respawned) = BackendProcess::spawn(meta.kind) {
                        let out = respawned.call_tool(&meta.backend_tool_name, prepared);
                        *backend = respawned;
                        out
                    } else {
                        Err(format!("Backend `{}` crashed and failed to restart", meta.kind.id()))
                    }
                }
            }
        };
        match call_res {
            Ok(res) => res,
            Err(msg) => err_result(msg),
        }
    }

    fn handle_mod_status(
        &self,
        args: Value,
        active_root: Option<&Path>,
        restricted_root: Option<&Path>,
    ) -> Value {
        let mut props = BTreeSet::new();
        props.insert("project".to_string());
        props.insert("root".to_string());

        let inspect_root = match prepare_tool_arguments(
            "mod_status",
            &props,
            args,
            active_root,
            restricted_root,
        ) {
            Ok(prepared) => prepared
                .get("project")
                .or_else(|| prepared.get("root"))
                .and_then(Value::as_str)
                .map(PathBuf::from),
            Err(msg) => return err_result(msg),
        };

        let subsystems: Vec<Value> = discover_all()
            .into_iter()
            .map(|info| {
                let tool_count = self
                    .backends
                    .get(&info.kind)
                    .map(|b| b.tools.len())
                    .unwrap_or(0);
                json!({
                    "id": info.kind.id(),
                    "namespace": info.kind.cli_namespace(),
                    "binary": info.path,
                    "available": info.available,
                    "version": info.version,
                    "tool_count": tool_count
                })
            })
            .collect();

        let repo_markers = inspect_root.as_ref().map(|root| {
            json!({
                "path": root,
                "exists": root.is_dir(),
                "git": root.join(".git").exists(),
                "system_json": root.join("system.json").is_file(),
                "project_brain": root.join("project-brain").is_dir(),
                "memory_bank": root.join("memory-bank").is_dir(),
                "agent_guardrails": root.join(".agent-guardrails").is_dir(),
            })
        });

        let payload = json!({
            "mod": "ai-infra-mod",
            "version": env!("CARGO_PKG_VERSION"),
            "protocol_version": PROTOCOL_VERSION,
            "total_tools": self.ordered_tools.len(),
            "restricted_mode": restricted_root.is_some(),
            "active_root": active_root,
            "subsystems": subsystems,
            "workspace": repo_markers
        });
        json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&payload).unwrap_or_default()
            }],
            "structuredContent": payload,
            "isError": false
        })
    }

    pub fn shutdown_all(&mut self) {
        for backend in self.backends.values_mut() {
            backend.shutdown();
        }
    }
}

fn err_result(msg: impl Into<String>) -> Value {
    json!({
        "content": [{ "type": "text", "text": msg.into() }],
        "isError": true
    })
}

fn parse_roots_response(msg: &Value) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for entry in msg
        .pointer("/result/roots")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(uri) = entry.get("uri").and_then(Value::as_str)
            && let Some(path) = parse_file_uri(uri)
        {
            out.push(std::fs::canonicalize(&path).unwrap_or(path));
        }
    }
    out
}

pub fn serve_stdio() -> Result<(), String> {
    let mut mux = Multiplexer::start()?;
    let env_root = env_configured_root();
    let restricted_root = if restricted_mode() {
        if env_root.is_none() {
            return Err("AI_INFRA_RESTRICT_ROOT=1 requires AI_INFRA_ROOT or ROOT_DIR to be set".to_string());
        }
        env_root.clone()
    } else {
        None
    };

    let (tx, rx) = mpsc::channel::<Result<String, String>>();
    thread::spawn(move || {
        let stdin = std::io::stdin();
        let reader = BufReader::new(stdin.lock());
        for line in reader.lines() {
            match line {
                Ok(l) => {
                    if tx.send(Ok(l)).is_err() {
                        break;
                    }
                }
                Err(e) => {
                    let _ = tx.send(Err(e.to_string()));
                    break;
                }
            }
        }
    });

    let stdout = std::io::stdout();
    let mut writer = stdout.lock();
    let mut client_supports_roots = false;
    let mut negotiated_roots: Vec<PathBuf> = Vec::new();
    let mut roots_req_seq: u64 = 1;
    let mut pending_roots_id: Option<String> = None;
    let mut pending_msgs: VecDeque<String> = VecDeque::new();

    let send_stdout = |w: &mut std::io::StdoutLock<'_>, v: &Value| -> Result<(), String> {
        let s = serde_json::to_string(v).map_err(|e| format!("JSON serialize failed: {e}"))?;
        writeln!(w, "{s}")
            .and_then(|_| w.flush())
            .map_err(|e| format!("stdout write failed: {e}"))
    };

    let request_roots = |w: &mut std::io::StdoutLock<'_>,
                         seq: &mut u64,
                         pending: &mut Option<String>|
     -> Result<(), String> {
        let req_id = format!("mod-roots-{}", *seq);
        *seq += 1;
        send_stdout(
            w,
            &json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "method": "roots/list",
                "params": {}
            }),
        )?;
        *pending = Some(req_id);
        Ok(())
    };

    loop {
        let line = if let Some(buffered) = pending_msgs.pop_front() {
            buffered
        } else {
            match rx.recv() {
                Ok(Ok(l)) => l,
                Ok(Err(e)) => return Err(format!("stdin read failed: {e}")),
                Err(_) => break,
            }
        };
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let msg: Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(err) => {
                send_stdout(
                    &mut writer,
                    &json!({
                        "jsonrpc": "2.0",
                        "id": Value::Null,
                        "error": { "code": -32700, "message": format!("Parse error: {err}") }
                    }),
                )?;
                continue;
            }
        };

        // Check if this is a client response to our `roots/list` server->client request.
        if msg.get("method").is_none()
            && let Some(id_str) = msg.get("id").and_then(Value::as_str)
            && pending_roots_id.as_deref() == Some(id_str)
        {
            pending_roots_id = None;
            let roots = parse_roots_response(&msg);
            if !roots.is_empty() {
                negotiated_roots = roots;
            }
            continue;
        }

        let method = msg["method"].as_str().unwrap_or("");
        let id = msg.get("id").cloned();

        if id.is_none() {
            if method == "notifications/initialized"
                && client_supports_roots
                && env_root.is_none()
                && pending_roots_id.is_none()
            {
                request_roots(&mut writer, &mut roots_req_seq, &mut pending_roots_id)?;
                // Give a fast client up to 120ms to respond immediately while queueing any early client request.
                let deadline = Instant::now() + Duration::from_millis(120);
                while pending_roots_id.is_some() {
                    let rem = deadline.saturating_duration_since(Instant::now());
                    if rem.is_zero() {
                        break;
                    }
                    match rx.recv_timeout(rem) {
                        Ok(Ok(next_line)) => {
                            let t = next_line.trim();
                            if t.is_empty() {
                                continue;
                            }
                            if let Ok(parsed) = serde_json::from_str::<Value>(t)
                                && parsed.get("method").is_none()
                                && parsed.get("id").and_then(Value::as_str)
                                    == pending_roots_id.as_deref()
                            {
                                pending_roots_id = None;
                                let roots = parse_roots_response(&parsed);
                                if !roots.is_empty() {
                                    negotiated_roots = roots;
                                }
                                break;
                            }
                            pending_msgs.push_back(next_line);
                        }
                        _ => break,
                    }
                }
            } else if method == "notifications/roots/list_changed"
                && client_supports_roots
                && env_root.is_none()
            {
                request_roots(&mut writer, &mut roots_req_seq, &mut pending_roots_id)?;
            }
            continue;
        }

        let req_id = id.unwrap();
        let response = match method {
            "initialize" => {
                client_supports_roots = !msg["params"]["capabilities"]["roots"].is_null();
                json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "protocolVersion": PROTOCOL_VERSION,
                        "capabilities": { "tools": {} },
                        "serverInfo": {
                            "name": "ai-infra-mod",
                            "version": env!("CARGO_PKG_VERSION")
                        },
                        "instructions": "Unified AI-Infrastructure MCP multiplexer (80 tools across graph-map, system-orchestration, context-graph, skill-foundry, agent-guardrails, and skillcache, plus mod_status). Pass explicit `project`/`root` or set AI_INFRA_ROOT / MCP roots."
                    }
                })
            }
            "ping" => json!({ "jsonrpc": "2.0", "id": req_id, "result": {} }),
            "tools/list" => json!({ "jsonrpc": "2.0", "id": req_id, "result": mux.tools_list() }),
            "tools/call" => {
                let params = &msg["params"];
                let name = params["name"].as_str().unwrap_or("");
                let args = params
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                let active_root = env_root
                    .as_deref()
                    .or_else(|| negotiated_roots.first().map(PathBuf::as_path));
                let result = mux.call_tool(name, args, active_root, restricted_root.as_deref());
                json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": result
                })
            }
            _ => json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "error": { "code": -32601, "message": format!("Method not found: {method}") }
            }),
        };
        send_stdout(&mut writer, &response)?;
    }

    mux.shutdown_all();
    Ok(())
}
