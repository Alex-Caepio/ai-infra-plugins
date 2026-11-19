//! Hook execution handlers, CLI check operations, and subagent write-lock gates.
use crate::{
    counters, custody,
    error::{GuardError, Result},
    policy::{self, LoadedPolicy},
    presets::Preset,
    shell,
};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

const EDIT_TOOLS: &[&str] = &[
    "write",
    "edit",
    "multiedit",
    "multi_edit",
    "notebookedit",
    "notebook_edit",
    "apply_patch",
];

#[derive(Clone, Debug, Default)]
pub struct GuardOutput {
    pub code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl GuardOutput {
    pub fn text(value: impl Into<String>) -> Self {
        Self {
            code: 0,
            stdout: value.into().into_bytes(),
            stderr: Vec::new(),
        }
    }

    pub fn json(value: &Value) -> Result<Self> {
        let mut bytes = serde_json::to_vec(value)?;
        bytes.push(b'\n');
        Ok(Self {
            code: 0,
            stdout: bytes,
            stderr: Vec::new(),
        })
    }

    pub fn error(code: i32, message: impl AsRef<str>) -> Self {
        Self {
            code,
            stdout: Vec::new(),
            stderr: format!("{}\n", message.as_ref()).into_bytes(),
        }
    }
}

/// Recursively extract first non-empty string value for `key`.
pub fn first<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    match value {
        Value::Object(map) => map
            .get(key)
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
            .or_else(|| map.values().find_map(|v| first(v, key))),
        Value::Array(items) => items.iter().find_map(|v| first(v, key)),
        _ => None,
    }
}

/// Recursively collect unique non-empty strings matching any of `keys`.
pub fn strings(value: &Value, keys: &[&str]) -> Vec<String> {
    fn visit(value: &Value, keys: &[&str], output: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                for key in keys {
                    if let Some(v) = map
                        .get(*key)
                        .and_then(Value::as_str)
                        .filter(|s| !s.is_empty())
                        && !output.iter().any(|prior| prior == v)
                    {
                        output.push(v.into());
                    }
                }
                for child in map.values() {
                    visit(child, keys, output);
                }
            }
            Value::Array(items) => {
                for child in items {
                    visit(child, keys, output);
                }
            }
            _ => {}
        }
    }
    let mut output = Vec::new();
    visit(value, keys, &mut output);
    output
}

fn allowed_tool(payload: &Value, names: &[&str]) -> bool {
    let tools = strings(payload, &["tool_name"]);
    tools.is_empty()
        || tools.iter().any(|tool| {
            let normalized = tool
                .rsplit(['.', ':', '/'])
                .next()
                .unwrap_or(tool)
                .to_ascii_lowercase();
            names.contains(&normalized.as_str())
        })
}

fn payload_noop(role: &str, payload: &Value) -> bool {
    match role {
        "bash-guard" => first(payload, "command").is_none() && first(payload, "cmd").is_none(),
        "naming-check" | "edit-loop" => !allowed_tool(payload, EDIT_TOOLS),
        _ => false,
    }
}

fn hook_context_json(event: &str, message: &str) -> Result<GuardOutput> {
    GuardOutput::json(&json!({
        "hookSpecificOutput": {
            "hookEventName": event,
            "additionalContext": message
        }
    }))
}

fn debugger_hint(host: &str) -> &'static str {
    if host == "codex" {
        "systematic-debugger"
    } else {
        "/debugger"
    }
}

/// Infer client host (`claude`, `codex`, or `cursor`) when not passed explicitly.
pub fn detect_host(explicit: Option<&str>, payload: &Value) -> String {
    if let Some(host) = explicit {
        return host.to_ascii_lowercase();
    }
    if payload.get("cursor_version").is_some() || payload.get("workspace_roots").is_some() {
        return "cursor".into();
    }
    if std::env::var_os("CODEX_HOME").is_some()
        || std::env::var_os("CODEX_THREAD_ID").is_some()
        || std::env::var_os("CODEX_CI").is_some()
    {
        return "codex".into();
    }
    "claude".into()
}

/// Infer project root from explicit `--root`, host env/payload `cwd`, and ancestor scan.
pub fn resolve_root(explicit: Option<&Path>, payload: Option<&Value>) -> PathBuf {
    if let Some(path) = explicit {
        return custody::discover_repo_root(path);
    }
    if let Some(p) = payload {
        if let Some(cwd) = p.get("cwd").and_then(Value::as_str).filter(|s| !s.is_empty()) {
            return custody::discover_repo_root(Path::new(cwd));
        }
        if let Some(first_ws) = p
            .get("workspace_roots")
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())
            .and_then(Value::as_str)
            .filter(|s| !s.is_empty())
        {
            return custody::discover_repo_root(Path::new(first_ws));
        }
    }
    if let Some(dir) = std::env::var_os("CLAUDE_PROJECT_DIR").filter(|v| !v.is_empty()) {
        return custody::discover_repo_root(Path::new(&dir));
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    custody::discover_repo_root(&cwd)
}

const INVALID_POLICY: &str = "BLOCKED: guardrails policy data or custody is invalid. Restore .agent-guardrails/policy.json (or project-brain/config/hook-policy.json) before running shell commands.";

fn refusal(command: &str, policy: &shell::Policy, root: Option<&Path>) -> Option<GuardOutput> {
    let category = match shell::validate_in_repo(command, policy, root) {
        Ok(category) => category,
        Err(error) => Some(error.to_string()),
    };
    category.map(|category| {
        GuardOutput::error(
            2,
            format!(
                "BLOCKED: Command refused (rule category: {category}).\n   This operation is blocked by agent-guardrails."
            ),
        )
    })
}

fn parse_payload(raw: &[u8]) -> Value {
    serde_json::from_slice::<Value>(raw)
        .ok()
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({}))
}

fn bash_guard(root: &Path, host: &str, payload: &Value) -> Result<GuardOutput> {
    let command = strings(payload, &["command", "cmd"]).join("\n");
    if command.is_empty() {
        return Ok(GuardOutput::default());
    }
    let Ok(loaded) = policy::load_policy(root, None) else {
        return Ok(GuardOutput::error(2, INVALID_POLICY));
    };
    let shell_policy = shell::Policy::from_loaded(&loaded);
    if let Some(blocked) = refusal(&command, &shell_policy, Some(root)) {
        return Ok(blocked);
    }
    if shell::polling(&command) {
        return Ok(GuardOutput::default());
    }
    let count = match counters::advance(root, payload, "cmd", &command, true) {
        Ok(c) => c,
        Err(_) => return Ok(GuardOutput::default()),
    };
    let hint = debugger_hint(host);
    if count >= loaded.policy.loop_thresholds.cmd_block {
        return Ok(GuardOutput::error(
            2,
            format!(
                "BLOCKED: this exact command has run {count} times this session.\n   Repeating it again is not a new attempt. Either change the\n   command (narrow it, add the failing case, read the output\n   differently) or escalate to {hint} for a root cause."
            ),
        ));
    }
    if count >= loaded.policy.loop_thresholds.cmd_warn {
        let message = format!(
            "WARNING: this exact command has run {count} times this session. If it keeps failing, use {hint} instead of another rerun."
        );
        return if host == "cursor" {
            Ok(GuardOutput::error(1, message))
        } else {
            hook_context_json("PreToolUse", &message)
        };
    }
    Ok(GuardOutput::default())
}

fn patch_paths(value: &Value, paths: &mut BTreeSet<String>) {
    match value {
        Value::String(text) => {
            for line in text.lines() {
                for prefix in ["*** Add File: ", "*** Update File: ", "*** Move to: "] {
                    if let Some(path) = line.strip_prefix(prefix) {
                        paths.insert(path.trim_end_matches('\r').into());
                    }
                }
            }
        }
        Value::Object(map) => {
            for child in map.values() {
                patch_paths(child, paths);
            }
        }
        Value::Array(items) => {
            for child in items {
                patch_paths(child, paths);
            }
        }
        _ => {}
    }
}

fn skill_prefixes(root: &Path, host: &str) -> Vec<String> {
    let primary = match host {
        "cursor" => root.join(".cursor/skills"),
        "codex" => root.join(".agents/skills"),
        _ => root.join(".claude/skills"),
    };
    let mut dirs = vec![
        primary,
        root.join(".agents/skills"),
        root.join(".claude/skills"),
        root.join(".cursor/skills"),
        root.join("skills"),
    ];
    let mut seen = BTreeSet::new();
    dirs.retain(|d| seen.insert(d.clone()));
    let mut prefixes = Vec::new();
    for dir in dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.filter_map(std::result::Result::ok) {
                if entry.path().is_dir()
                    && let Some(name) = entry.file_name().to_str()
                {
                    let p = format!("{name}-");
                    if !prefixes.contains(&p) {
                        prefixes.push(p);
                    }
                }
            }
        }
    }
    prefixes
}

/// Validate one list of file paths against naming conventions & policy globs.
pub fn validate_naming_paths(
    root: &Path,
    host: &str,
    paths: &BTreeSet<String>,
    loaded: &LoadedPolicy,
) -> Result<Option<String>> {
    let blocked_globs: Vec<regex::Regex> = loaded
        .policy
        .naming_rules
        .blocked_globs
        .iter()
        .filter_map(|g| policy::compile_glob(g).ok())
        .collect();
    let prefixes = skill_prefixes(root, host);
    let memory_filename = regex::Regex::new(r"^MEM-[0-9]{4,}-[a-z0-9]+(?:-[a-z0-9]+)*\.md$")?;
    let task_directory = regex::Regex::new(r"^TASK-[0-9]{3,}$")?;
    let root_prefix = format!("{}/", root.display());

    for path in paths {
        let normalized = path
            .strip_prefix(&root_prefix)
            .unwrap_or_else(|| path.strip_prefix("./").unwrap_or(path));
        if blocked_globs.iter().any(|re| re.is_match(normalized)) {
            return Ok(Some(format!(
                "Path '{path}' matches a blocked_globs naming rule in policy.json"
            )));
        }
        if !loaded.policy.naming_rules.enforce_accelerator_conventions || !path.ends_with(".md") {
            continue;
        }
        let file = normalized.rsplit('/').next().unwrap_or(normalized);
        if normalized.starts_with("memory-bank/chunks/") {
            if normalized.rsplit_once('/').map(|(parent, _)| parent) != Some("memory-bank/chunks") {
                return Ok(Some(format!(
                    "Memory chunks must be stored directly in memory-bank/chunks/: '{path}'"
                )));
            }
            if !memory_filename.is_match(file) {
                return Ok(Some(format!(
                    "Memory chunks must use memory-bank/chunks/MEM-0001-short-slug.md: '{path}'"
                )));
            }
        } else if normalized.starts_with("tasks/") || normalized.starts_with("specs/") {
            if let Some(relative) = normalized.strip_prefix("tasks/") {
                if let Some((directory, _)) = relative.split_once('/') {
                    if !task_directory.is_match(directory) {
                        return Ok(Some(format!(
                            "Task Markdown must be inside a zero-padded directory such as tasks/TASK-001/: '{path}'"
                        )));
                    }
                } else if !["README.md", "CHANGELOG.md"].contains(&file) {
                    return Ok(Some(format!(
                        "Task Markdown must be inside tasks/TASK-NNN/: '{path}'"
                    )));
                }
            }
            if !prefixes.is_empty()
                && !["README.md", "CHANGELOG.md", "MANIFEST.md"].contains(&file)
                && !prefixes.iter().any(|prefix| file.starts_with(prefix))
            {
                return Ok(Some(format!(
                    "'{file}' must start with a discovered skill name followed by '-'"
                )));
            }
        }
    }
    Ok(None)
}

fn naming_check(root: &Path, host: &str, payload: &Value) -> Result<GuardOutput> {
    if !allowed_tool(payload, EDIT_TOOLS) {
        return Ok(GuardOutput::default());
    }
    let mut paths: BTreeSet<String> = strings(payload, &["file_path", "path"]).into_iter().collect();
    patch_paths(payload, &mut paths);
    if paths.is_empty() {
        return Ok(GuardOutput::default());
    }
    let loaded = policy::load_policy(root, None).unwrap_or_else(|_| LoadedPolicy {
        preset: Preset::Generic,
        policy: policy::HookPolicy::template(Preset::Generic),
        source_path: None,
        strict_custody: false,
    });
    if let Some(reason) = validate_naming_paths(root, host, &paths, &loaded)? {
        return Ok(GuardOutput::error(
            2,
            format!("BLOCKED: {reason}\n   See repository file-naming rules."),
        ));
    }
    Ok(GuardOutput::default())
}

fn edit_loop(root: &Path, host: &str, payload: &Value) -> Result<GuardOutput> {
    if !allowed_tool(payload, EDIT_TOOLS) {
        return Ok(GuardOutput::default());
    }
    let mut paths = BTreeSet::new();
    if let Some(path) = first(payload, "file_path")
        .or_else(|| {
            if host == "claude" {
                None
            } else {
                first(payload, "path")
            }
        })
        .or_else(|| first(payload, "notebook_path"))
    {
        paths.insert(path.to_string());
    } else {
        patch_paths(payload, &mut paths);
    }
    if paths.is_empty() {
        return Ok(GuardOutput::default());
    }
    let loaded = policy::load_policy(root, None).unwrap_or_else(|_| LoadedPolicy {
        preset: Preset::Generic,
        policy: policy::HookPolicy::template(Preset::Generic),
        source_path: None,
        strict_custody: false,
    });
    let warn_at = loaded.policy.loop_thresholds.edit_warn;
    let block_at = loaded.policy.loop_thresholds.edit_block;
    let hint = debugger_hint(host);
    let mut warnings = Vec::new();
    let mut blocked = Vec::new();
    for path in paths {
        let count = match counters::advance(root, payload, "edit", &path, false) {
            Ok(c) => c,
            Err(_) => continue,
        };
        let shown = path.replace(['\\', '"'], "");
        if count >= block_at {
            blocked.push(format!(
                "BLOCKED: File '{shown}' edited {count} times this session.\n   This looks like a repeated-edit loop. Consider:\n   - {hint} to investigate the root cause\n   - Reassessing your approach"
            ));
        } else if count >= warn_at {
            warnings.push(format!(
                "WARNING: File '{shown}' edited {count} times this session. If you're stuck in a loop, consider using {hint}."
            ));
        }
    }
    if !blocked.is_empty() {
        blocked.extend(warnings);
        return Ok(GuardOutput::error(2, blocked.join("\n")));
    }
    if warnings.is_empty() {
        return Ok(GuardOutput::default());
    }
    let message = warnings.join(" ");
    if host == "cursor" {
        let mut out = GuardOutput::text(format!("{message}\n"));
        out.code = 1;
        Ok(out)
    } else {
        hook_context_json("PostToolUse", &message)
    }
}

fn roster(root: &Path, host: &str, loaded: &LoadedPolicy) -> BTreeMap<String, bool> {
    let mut result = BTreeMap::new();
    for name in &loaded.policy.subagent_gate.allowed_agents {
        result.entry(name.clone()).or_insert(false);
    }
    for name in &loaded.policy.subagent_gate.writer_agents {
        result.insert(name.clone(), true);
    }
    let dir = root.join(if host == "cursor" {
        ".cursor/agents"
    } else {
        ".claude/agents"
    });
    if dir.is_dir()
        && let Ok(entries) = fs::read_dir(dir)
    {
        for entry in entries.filter_map(std::result::Result::ok) {
            let p = entry.path();
            if p.extension().is_none_or(|ext| ext != "md")
                || p.file_name().is_some_and(|name| name == "README.md")
            {
                continue;
            }
            let Ok(text) = custody::read_text(&p, 256 * 1024) else {
                continue;
            };
            let mut lines = text.lines();
            if lines.next() != Some("---") {
                continue;
            }
            let front: Vec<&str> = lines.take_while(|line| *line != "---").collect();
            let field = |key: &str| {
                front.iter().find_map(|line| {
                    line.strip_prefix(&format!("{key}:"))
                        .map(|v| v.trim().replace('"', ""))
                })
            };
            if let Some(name) = field("name").filter(|n| !n.is_empty()) {
                let writes = field("writes").as_deref() == Some("true");
                result
                    .entry(name)
                    .and_modify(|prev| *prev |= writes)
                    .or_insert(writes);
            }
        }
    }
    result
}

fn safe_type(value: &str) -> String {
    value
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || " _.:/-".contains(*c))
        .take(64)
        .collect()
}

fn gate_response(host: &str, allow: bool, message: &str) -> Result<GuardOutput> {
    if host == "cursor" {
        if allow {
            GuardOutput::json(&json!({"permission": "allow"}))
        } else {
            GuardOutput::json(&json!({"permission": "deny", "user_message": message}))
        }
    } else if allow {
        Ok(GuardOutput::default())
    } else {
        Ok(GuardOutput::error(2, message))
    }
}

fn codex_multi_agent_disabled(root: &Path, loaded: &LoadedPolicy) -> bool {
    if loaded.policy.subagent_gate.block_builtin_codex {
        return true;
    }
    let cfg = root.join(".codex/config.toml");
    if let Ok(text) = custody::read_text(&cfg, 128 * 1024)
        && (text.contains("multi_agent = false") || text.contains("enabled = false"))
    {
        return true;
    }
    false
}

fn subagent_gate(root: &Path, host: &str, payload: &Value) -> Result<GuardOutput> {
    let loaded = policy::load_policy(root, None).unwrap_or_else(|_| LoadedPolicy {
        preset: Preset::Generic,
        policy: policy::HookPolicy::template(Preset::Generic),
        source_path: None,
        strict_custody: false,
    });
    if !loaded.policy.subagent_gate.enabled {
        return gate_response(host, true, "");
    }
    if host == "codex" {
        let forbidden = [
            "spawn_agent",
            "agent",
            "send_input",
            "resume_agent",
            "wait_agent",
            "close_agent",
        ];
        if codex_multi_agent_disabled(root, &loaded)
            && strings(payload, &["tool_name"])
                .iter()
                .any(|name| forbidden.contains(&name.to_ascii_lowercase().as_str()))
        {
            return gate_response(
                host,
                false,
                "BLOCKED: Codex built-in subagents are disabled by project policy.\n   Use project skills in .agents/skills/ or work in the main session.",
            );
        }
        return gate_response(host, true, "");
    }
    if host != "cursor" && !allowed_tool(payload, &["agent", "task"]) {
        return gate_response(host, true, "");
    }
    let Some(agent) = first(payload, "subagent_type").or_else(|| first(payload, "agent_type")) else {
        return gate_response(host, true, "");
    };
    let map = roster(root, host, &loaded);
    if map.is_empty() {
        return gate_response(host, true, "");
    }
    let Some(&writes) = map.get(agent) else {
        return gate_response(
            host,
            false,
            &format!(
                "BLOCKED: subagent type \"{}\" is not listed in this project's guardrails roster.",
                safe_type(agent)
            ),
        );
    };
    if !writes {
        return gate_response(host, true, "");
    }
    match counters::take_writer(root, host, agent) {
        Ok(None) => gate_response(host, true, ""),
        Ok(Some(holder)) => gate_response(
            host,
            false,
            &format!(
                "BLOCKED: write-capable agent \"{}\" is already running.\n   Only one write-capable subagent may run per repository at a time.",
                safe_type(&holder)
            ),
        ),
        Err(error) => gate_response(host, false, &format!("BLOCKED: {error}")),
    }
}

fn subagent_release(root: &Path, payload: &Value) -> Result<GuardOutput> {
    let agent = first(payload, "agent_type")
        .or_else(|| first(payload, "subagent_type"))
        .unwrap_or("");
    let _ = counters::release_writer(root, agent);
    Ok(GuardOutput::default())
}

/// Normalize canonical & alias hook role strings.
pub fn normalize_role(role: &str) -> Option<&'static str> {
    match role {
        "bash-guard" | "bash-validator" => Some("bash-guard"),
        "edit-loop" | "loop-detection" => Some("edit-loop"),
        "naming-check" | "file-naming-validator" => Some("naming-check"),
        "subagent-gate" => Some("subagent-gate"),
        "subagent-release" | "subagent-dispatch" => Some("subagent-release"),
        _ => None,
    }
}

/// Execute any hook role against raw stdin JSON bytes and host (`claude`/`codex`/`cursor`).
pub fn hook_execute(
    explicit_root: Option<&Path>,
    explicit_host: Option<&str>,
    raw_role: &str,
    raw_stdin: &[u8],
) -> Result<GuardOutput> {
    let role = normalize_role(raw_role)
        .ok_or_else(|| GuardError::invalid(format!("Unknown hook role: {raw_role}")))?;
    let payload = parse_payload(raw_stdin);
    if payload_noop(role, &payload) {
        return Ok(GuardOutput::default());
    }
    let host = detect_host(explicit_host, &payload);
    if !["claude", "codex", "cursor"].contains(&host.as_str()) {
        return Err(GuardError::invalid(format!("Unsupported hook host: {host}")));
    }
    let root = resolve_root(explicit_root, Some(&payload));
    match role {
        "bash-guard" => bash_guard(&root, &host, &payload),
        "edit-loop" => edit_loop(&root, &host, &payload),
        "naming-check" => naming_check(&root, &host, &payload),
        "subagent-gate" => subagent_gate(&root, &host, &payload),
        "subagent-release" => subagent_release(&root, &payload),
        _ => Ok(GuardOutput::default()),
    }
}

/// Direct CLI / MCP check of a single command without touching loop counters.
pub fn check_command(
    root: &Path,
    command: &str,
    preset_override: Option<Preset>,
) -> Result<Value> {
    let loaded = policy::load_policy(root, preset_override)?;
    let shell_policy = shell::Policy::from_loaded(&loaded);
    let outcome = shell::validate_in_repo(command, &shell_policy, Some(root));
    match outcome {
        Ok(None) => Ok(json!({
            "allowed": true,
            "blocked": false,
            "category": Value::Null,
            "preset": loaded.preset.as_str(),
            "root": root.display().to_string(),
            "polling_exempt": shell::polling(command),
        })),
        Ok(Some(category)) => Ok(json!({
            "allowed": false,
            "blocked": true,
            "category": category,
            "preset": loaded.preset.as_str(),
            "root": root.display().to_string(),
            "polling_exempt": false,
        })),
        Err(error) => Ok(json!({
            "allowed": false,
            "blocked": true,
            "category": error.message,
            "preset": loaded.preset.as_str(),
            "root": root.display().to_string(),
            "polling_exempt": false,
        })),
    }
}
