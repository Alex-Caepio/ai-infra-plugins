//! Hook execution handlers, CLI check operations, and subagent write-lock gates.
use crate::{
    counters, custody,
    error::{GuardError, Result},
    policy::{self, LoadedPolicy},
    presets::Preset,
    shell,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::UNIX_EPOCH,
};

const EDIT_TOOLS: &[&str] = &[
    "write",
    "edit",
    "multiedit",
    "multi_edit",
    "notebookedit",
    "notebook_edit",
    "apply_patch",
    "write_to_file",
    "write_file",
    "replace_file_content",
    "multi_replace_file_content",
    "edit_file",
    "find_by_name",
];
const CHECK_TOOLS: &[&str] = &["bash", "shell", "terminal", "run_command"];
const COMMAND_KEYS: &[&str] = &["command", "cmd", "CommandLine", "command_line"];
const PATH_KEYS: &[&str] = &["file_path", "path", "TargetFile", "AbsolutePath", "TargetFiles"];
const SKILL_CACHE_RACY_NS: i128 = 2_000_000_000;

#[cfg(test)]
thread_local! {
    static SKILL_DIR_SCANS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static ROSTER_DIR_SCANS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn bump_skill_dir_scans() {
    SKILL_DIR_SCANS.with(|cell| cell.set(cell.get() + 1));
}

#[cfg(test)]
fn bump_roster_dir_scans() {
    ROSTER_DIR_SCANS.with(|cell| cell.set(cell.get() + 1));
}

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

/// Recursively collect unique non-empty strings (including string array elements) matching `keys`.
pub fn strings(value: &Value, keys: &[&str]) -> Vec<String> {
    fn push_unique(output: &mut Vec<String>, candidate: &str) {
        if !candidate.is_empty() && !output.iter().any(|prior| prior == candidate) {
            output.push(candidate.into());
        }
    }
    fn visit(value: &Value, keys: &[&str], output: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                for key in keys {
                    if let Some(found) = map.get(*key) {
                        match found {
                            Value::String(v) => push_unique(output, v),
                            Value::Array(items) => {
                                for s in items.iter().filter_map(Value::as_str) {
                                    push_unique(output, s);
                                }
                            }
                            _ => {}
                        }
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

fn tool_names(payload: &Value) -> Vec<String> {
    let mut tools = strings(payload, &["tool_name"]);
    if let Some(tc_name) = payload
        .get("toolCall")
        .and_then(|tc| tc.get("name"))
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        && !tools.iter().any(|prior| prior == tc_name)
    {
        tools.push(tc_name.into());
    }
    tools
}

fn allowed_tool(payload: &Value, names: &[&str]) -> bool {
    let tools = tool_names(payload);
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

fn integer_field(value: &Value, keys: &[&str]) -> Option<i64> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(found) = map.get(*key).and_then(Value::as_i64) {
                    return Some(found);
                }
            }
            map.values().find_map(|child| integer_field(child, keys))
        }
        Value::Array(items) => items.iter().find_map(|child| integer_field(child, keys)),
        _ => None,
    }
}

fn bool_field(value: &Value, keys: &[&str]) -> Option<bool> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(found) = map.get(*key).and_then(Value::as_bool) {
                    return Some(found);
                }
            }
            map.values().find_map(|child| bool_field(child, keys))
        }
        Value::Array(items) => items.iter().find_map(|child| bool_field(child, keys)),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CheckOutcome {
    PreExecution,
    Passing,
    Failing,
}

fn check_outcome(payload: &Value, command: &str, err_sig: &str) -> CheckOutcome {
    let exit_code = integer_field(
        payload,
        &["exit_code", "returncode", "exitCode", "status_code"],
    );
    let explicit_ok = bool_field(payload, &["passed", "ok", "success"]);
    let status_str = first(payload, "status").map(str::to_ascii_lowercase);
    let has_error_field = ["error", "stderr", "last_error", "tool_error"]
        .iter()
        .any(|k| first(payload, k).is_some());
    let agy_post =
        payload.get("hookEventName").and_then(Value::as_str) == Some("PostToolUse");
    if !err_sig.is_empty()
        || has_error_field
        || exit_code.is_some_and(|code| code != 0)
        || explicit_ok == Some(false)
        || matches!(
            status_str.as_deref(),
            Some("failed" | "failure" | "error" | "errored")
        )
    {
        CheckOutcome::Failing
    } else if !shell::polling(command)
        && (exit_code == Some(0)
            || explicit_ok == Some(true)
            || matches!(
                status_str.as_deref(),
                Some("passed" | "pass" | "ok" | "success" | "succeeded")
            )
            || (agy_post
                && exit_code.is_none()
                && explicit_ok.is_none()
                && status_str.is_none()))
    {
        CheckOutcome::Passing
    } else {
        CheckOutcome::PreExecution
    }
}

fn combine_command_error_signature(normalized_cmd: &str, err_sig: &str) -> String {
    if normalized_cmd.is_empty() && err_sig.is_empty() {
        String::new()
    } else if err_sig.is_empty() {
        format!("norm:{normalized_cmd}")
    } else if normalized_cmd.is_empty() {
        format!("err:{err_sig}")
    } else {
        format!("norm:{normalized_cmd}\0{err_sig}")
    }
}

fn is_check_event(payload: &Value) -> bool {
    let has_cmd = COMMAND_KEYS.iter().any(|k| first(payload, k).is_some());
    if !has_cmd {
        return false;
    }
    let tools = tool_names(payload);
    (!tools.is_empty() && allowed_tool(payload, CHECK_TOOLS))
        || integer_field(
            payload,
            &["exit_code", "returncode", "exitCode", "status_code"],
        )
        .is_some()
        || bool_field(payload, &["passed", "ok", "success"]).is_some()
        || first(payload, "status").is_some()
        || !normalized_error_signature(payload).is_empty()
        || first(payload, "error").is_some()
}

fn payload_noop(role: &str, payload: &Value) -> bool {
    match role {
        "bash-guard" => COMMAND_KEYS.iter().all(|k| first(payload, k).is_none()),
        "naming-check" => !allowed_tool(payload, EDIT_TOOLS),
        "edit-loop" => !allowed_tool(payload, EDIT_TOOLS) && !is_check_event(payload),
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

fn normalize_host(raw: &str) -> String {
    match raw.to_ascii_lowercase().as_str() {
        "agy" | "antigravity" => "antigravity".into(),
        other => other.into(),
    }
}

/// Infer client host (`antigravity`, `claude`, `codex`, or `cursor`) when not passed explicitly.
pub fn detect_host(explicit: Option<&str>, payload: &Value) -> String {
    if let Some(host) = explicit {
        return normalize_host(host);
    }
    if payload.get("hookEventName").is_some()
        || payload.get("toolCall").is_some()
        || payload.get("workspacePaths").is_some()
        || payload.get("conversationId").is_some()
    {
        return "antigravity".into();
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

/// Infer project root from explicit `--root`, host env/payload `cwd` or `workspacePaths`, and ancestor scan.
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
        if let Some(first_ws) = p
            .get("workspacePaths")
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

/// Normalizes a shell command into a canonical signature so trivial output paging/redirection
/// tweaks (`2>&1`, `| head -n 20`, extra whitespace) still register as repeated retries.
pub fn normalize_command_signature(command: &str) -> String {
    let mut trimmed = command.trim();
    loop {
        let next = trimmed.trim_end();
        let stripped = if let Some(idx) = next.rfind('|') {
            let tail = next[idx + 1..].trim_start();
            let head_word = tail.split_whitespace().next().unwrap_or("");
            if [
                "head", "tail", "cat", "less", "more", "wc", "grep", "rg", "tee",
            ]
            .contains(&head_word)
            {
                &next[..idx]
            } else {
                next
            }
        } else {
            next
        };
        let without_redir = stripped
            .trim_end()
            .strip_suffix("2>&1")
            .or_else(|| stripped.trim_end().strip_suffix("1>/dev/null"))
            .or_else(|| stripped.trim_end().strip_suffix(">/dev/null"))
            .map(str::trim_end);
        match without_redir {
            Some(shorter) if shorter.len() < next.len() => trimmed = shorter,
            _ if stripped.len() < next.len() => trimmed = stripped,
            _ => break,
        }
    }
    trimmed.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn normalized_error_signature(payload: &Value) -> String {
    for key in ["error", "stderr", "last_error", "tool_error"] {
        if let Some(raw) = first(payload, key) {
            let sig: String = raw
                .split_whitespace()
                .filter(|w| !w.chars().all(|c| c.is_ascii_digit()))
                .take(12)
                .collect::<Vec<_>>()
                .join(" ");
            if !sig.is_empty() {
                return sig;
            }
        }
    }
    String::new()
}

fn diagnostic_hint(normalized: &str, err_sig: &str, debugger_name: &str) -> String {
    let lower = normalized.to_ascii_lowercase();
    let base = if lower.contains("cargo test")
        || lower.contains("cargo check")
        || lower.contains("cargo clippy")
    {
        "Hint: isolate one failing test with `-- --nocapture` or inspect the first compiler diagnostic instead of rerunning the full workspace."
    } else if lower.contains("phpunit")
        || lower.contains("pest")
        || lower.contains("phpstan")
        || lower.contains("composer")
    {
        "Hint: narrow to the failing test/file (`--filter <Name>` or single path) and verify dependencies/config before retrying."
    } else if lower.contains("pytest") || lower.contains("python") {
        "Hint: isolate the failing test with `-k <name> --tb=short` to inspect the underlying assertion or traceback."
    } else if lower.contains("npm ")
        || lower.contains("pnpm ")
        || lower.contains("yarn ")
        || lower.contains("vitest")
        || lower.contains("jest")
        || lower.contains("tsc")
    {
        "Hint: run a single test file or standalone typecheck to read the first failing diagnostic cleanly."
    } else if lower.starts_with("git ") {
        "Hint: inspect `git status` and `git diff --stat` to resolve conflicts or hooks before retrying."
    } else {
        "Hint: narrow the scope, read full stderr without truncation, or check recent edits before retrying."
    };
    if err_sig.is_empty() {
        format!("{base} Escalate to {debugger_name} if the failure repeats.")
    } else {
        format!(
            "{base} Repeated error signature: `{}`. Escalate to {debugger_name} if unresolved.",
            err_sig.chars().take(80).collect::<String>()
        )
    }
}

fn bash_guard(root: &Path, host: &str, payload: &Value) -> Result<GuardOutput> {
    let command = strings(payload, COMMAND_KEYS).join("\n");
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
    let normalized = normalize_command_signature(&command);
    let err_sig = normalized_error_signature(payload);
    if check_outcome(payload, &command, &err_sig) == CheckOutcome::Passing {
        let _ = counters::reset(root, &counters::session_id(payload));
        return Ok(GuardOutput::default());
    }
    let norm_identity = combine_command_error_signature(&normalized, &err_sig);
    let _ = counters::record_signature(root, payload, &norm_identity);
    let exact_identity = if err_sig.is_empty() {
        command.clone()
    } else {
        format!("{command}\0{err_sig}")
    };
    let exact_count = match counters::advance(root, payload, "cmd", &exact_identity, true) {
        Ok(c) => c,
        Err(_) => return Ok(GuardOutput::default()),
    };
    let norm_count = if normalized != command.trim() || !err_sig.is_empty() {
        counters::advance(root, payload, "cmd", &norm_identity, true).unwrap_or(exact_count)
    } else {
        exact_count
    };
    let count = exact_count.max(norm_count);
    let debugger = debugger_hint(host);
    let hint = diagnostic_hint(&normalized, &err_sig, debugger);
    if count >= loaded.policy.loop_thresholds.cmd_block {
        return Ok(GuardOutput::error(
            2,
            format!(
                "BLOCKED: this {} has run {count} times this session.\n   Repeating it again is not a new attempt. Either change the\n   command (narrow it, add the failing case, read the output\n   differently) or escalate to {debugger} for a root cause.\n   {hint}",
                if exact_count >= loaded.policy.loop_thresholds.cmd_block {
                    "exact command"
                } else {
                    "normalized command signature"
                }
            ),
        ));
    }
    if count >= loaded.policy.loop_thresholds.cmd_warn {
        let kind_label = if exact_count >= loaded.policy.loop_thresholds.cmd_warn {
            "exact command"
        } else {
            "command signature"
        };
        let message = format!(
            "WARNING: this {kind_label} has run {count} times this session. {hint}"
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

fn memory_filename_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r"^MEM-[0-9]{4,}-[a-z0-9]+(?:-[a-z0-9]+)*\.md$")
            .expect("constant memory filename regex")
    })
}

fn task_directory_re() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r"^TASK-[0-9]{3,}$").expect("constant task directory regex")
    })
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct EntryStamp {
    mtime_ns: i128,
    ctime_ns: i128,
    size: u64,
    ino: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct SkillDirCacheEntry {
    skills_dirs: Vec<(String, Option<EntryStamp>)>,
    prefixes: Vec<String>,
}

fn entry_stamp(path: &Path, expect_dir: bool) -> Option<(EntryStamp, i128)> {
    let meta = fs::symlink_metadata(path).ok()?;
    if custody::linked(&meta)
        || (expect_dir && !meta.is_dir())
        || (!expect_dir && !meta.is_file())
    {
        return None;
    }
    let mtime_ns = i128::try_from(
        meta.modified()
            .ok()?
            .duration_since(UNIX_EPOCH)
            .ok()?
            .as_nanos(),
    )
    .ok()?;
    #[cfg(unix)]
    let (ctime_ns, size, ino) = {
        use std::os::unix::fs::MetadataExt;
        let ctime = i128::from(meta.ctime()) * 1_000_000_000 + i128::from(meta.ctime_nsec());
        (ctime, meta.size(), meta.ino())
    };
    #[cfg(not(unix))]
    let (ctime_ns, size, ino) = (mtime_ns, meta.len(), 0u64);
    let settled = mtime_ns.max(ctime_ns);
    Some((
        EntryStamp {
            mtime_ns,
            ctime_ns,
            size,
            ino,
        },
        settled,
    ))
}

fn current_unix_ns() -> i128 {
    i128::try_from(
        std::time::SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos()),
    )
    .unwrap_or(0)
}

fn write_settled_cache_file(cache_path: &Path, bytes: &[u8]) {
    if cache_path.parent().is_some_and(|parent| {
        fs::symlink_metadata(parent)
            .is_ok_and(|meta| !custody::linked(&meta) && meta.is_dir())
    }) && custody::admit_directory(cache_path.parent().unwrap_or(cache_path)).is_ok()
    {
        let _ = custody::atomic_write(cache_path, bytes);
    }
}

fn skill_dirs(root: &Path, host: &str) -> Vec<PathBuf> {
    let primary = match host {
        "cursor" => root.join(".cursor/skills"),
        "codex" => root.join(".agents/skills"),
        "antigravity" => root.join(".gemini/skills"),
        _ => root.join(".claude/skills"),
    };
    let mut dirs = vec![
        primary,
        root.join(".gemini/skills"),
        root.join(".antigravity/skills"),
        root.join(".agents/skills"),
        root.join(".claude/skills"),
        root.join(".cursor/skills"),
        root.join("skills"),
    ];
    let mut seen = BTreeSet::new();
    dirs.retain(|d| seen.insert(d.clone()));
    dirs
}

fn scan_skill_prefixes(dirs: &[PathBuf]) -> Vec<String> {
    #[cfg(test)]
    bump_skill_dir_scans();
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

fn skill_prefixes_with_now(dirs: &[PathBuf], cache_file: Option<&Path>, now_ns: i128) -> Vec<String> {
    static CACHE: OnceLock<Mutex<BTreeMap<Vec<PathBuf>, SkillDirCacheEntry>>> = OnceLock::new();
    let mut dir_stamps = Vec::with_capacity(dirs.len());
    let mut max_settled = 0i128;
    let mut any_existing = false;
    for dir in dirs {
        if let Some((stamp, settled_ns)) = entry_stamp(dir, true) {
            any_existing = true;
            max_settled = max_settled.max(settled_ns);
            dir_stamps.push((dir.to_string_lossy().into_owned(), Some(stamp)));
        } else {
            dir_stamps.push((dir.to_string_lossy().into_owned(), None));
        }
    }
    if !any_existing {
        return Vec::new();
    }
    let racy_settled = now_ns > SKILL_CACHE_RACY_NS && now_ns - max_settled >= SKILL_CACHE_RACY_NS;
    let memory = CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    if racy_settled
        && let Ok(guard) = memory.lock()
        && let Some(entry) = guard.get(dirs)
        && entry.skills_dirs == dir_stamps
    {
        return entry.prefixes.clone();
    }
    if racy_settled
        && let Some(cache_path) = cache_file
        && custody::owned_by_current_user(cache_path)
        && let Ok(raw) = custody::read_regular(cache_path, 512 * 1024)
        && let Ok(entry) = serde_json::from_slice::<SkillDirCacheEntry>(&raw)
        && entry.skills_dirs == dir_stamps
    {
        if let Ok(mut guard) = memory.lock() {
            guard.insert(dirs.to_vec(), entry.clone());
        }
        return entry.prefixes;
    }

    let prefixes = scan_skill_prefixes(dirs);
    if racy_settled {
        let entry = SkillDirCacheEntry {
            skills_dirs: dir_stamps,
            prefixes: prefixes.clone(),
        };
        if let Ok(mut guard) = memory.lock() {
            guard.insert(dirs.to_vec(), entry.clone());
        }
        if let Some(cache_path) = cache_file
            && let Ok(bytes) = serde_json::to_vec(&entry)
        {
            write_settled_cache_file(cache_path, &bytes);
        }
    }
    prefixes
}

fn skill_prefixes(root: &Path, host: &str) -> Vec<String> {
    let dirs = skill_dirs(root, host);
    let cache_file = counters::directory(root)
        .ok()
        .map(|dir| dir.join(format!("skill-naming-cache-{host}.json")));
    skill_prefixes_with_now(&dirs, cache_file.as_deref(), current_unix_ns())
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
    let mut cached_prefixes: Option<Vec<String>> = None;
    let memory_filename = memory_filename_re();
    let task_directory = task_directory_re();
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
            if !["README.md", "CHANGELOG.md", "MANIFEST.md"].contains(&file) {
                let prefixes = cached_prefixes.get_or_insert_with(|| skill_prefixes(root, host));
                if !prefixes.is_empty() && !prefixes.iter().any(|prefix| file.starts_with(prefix)) {
                    return Ok(Some(format!(
                        "'{file}' must start with a discovered skill name followed by '-'"
                    )));
                }
            }
        }
    }
    Ok(None)
}

fn naming_check(root: &Path, host: &str, payload: &Value) -> Result<GuardOutput> {
    if !allowed_tool(payload, EDIT_TOOLS) {
        return Ok(GuardOutput::default());
    }
    let mut paths: BTreeSet<String> = strings(payload, PATH_KEYS).into_iter().collect();
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
    if is_check_event(payload) {
        let command = strings(payload, COMMAND_KEYS).join("\n");
        let err_sig = normalized_error_signature(payload);
        let normalized = normalize_command_signature(&command);
        match check_outcome(payload, &command, &err_sig) {
            CheckOutcome::Passing => {
                let _ = counters::reset(root, &counters::session_id(payload));
            }
            CheckOutcome::Failing | CheckOutcome::PreExecution => {
                let sig = combine_command_error_signature(&normalized, &err_sig);
                let _ = counters::record_signature(root, payload, &sig);
            }
        }
        return Ok(GuardOutput::default());
    }
    if !allowed_tool(payload, EDIT_TOOLS) {
        return Ok(GuardOutput::default());
    }
    let mut paths = BTreeSet::new();
    if host == "claude" {
        if let Some(path) = first(payload, "file_path").or_else(|| first(payload, "notebook_path")) {
            paths.insert(path.to_string());
        }
    } else {
        for p in strings(
            payload,
            &[
                "file_path",
                "path",
                "notebook_path",
                "TargetFile",
                "AbsolutePath",
                "TargetFiles",
            ],
        ) {
            paths.insert(p);
        }
    }
    if paths.is_empty() {
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
    let inline_cmd = strings(payload, COMMAND_KEYS).join("\n");
    let inline_err = normalized_error_signature(payload);
    let inline_sig =
        combine_command_error_signature(&normalize_command_signature(&inline_cmd), &inline_err);
    let active_sig = if !inline_sig.is_empty() {
        Some(inline_sig)
    } else {
        counters::active_signature(root, payload)
    };
    let warn_at = loaded.policy.loop_thresholds.edit_warn;
    let block_at = loaded.policy.loop_thresholds.edit_block;
    let hint = debugger_hint(host);
    let mut warnings = Vec::new();
    let mut blocked = Vec::new();
    for path in paths {
        let identity = match active_sig.as_deref() {
            Some(sig) => format!("{path}\0{sig}"),
            None => path.clone(),
        };
        let count = match counters::advance(root, payload, "edit", &identity, false) {
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct AgentFileStamp {
    name: String,
    stamp: EntryStamp,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct AgentRosterCacheEntry {
    agents_dir: String,
    dir_stamp: EntryStamp,
    files: Vec<AgentFileStamp>,
    roster: BTreeMap<String, bool>,
}

fn inspect_agent_roster_dir(
    path: &Path,
) -> Result<Option<(EntryStamp, Vec<AgentFileStamp>, i128)>> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    if custody::linked(&meta) || !meta.is_dir() {
        return Err(GuardError::policy(
            "Agent roster directory is a symlink or non-directory",
        ));
    }
    let Some((d_stamp, mut max_settled)) = entry_stamp(path, true) else {
        return Err(GuardError::policy(
            "Agent roster directory metadata is unsafe or unavailable",
        ));
    };
    let mut entries = fs::read_dir(path)?
        .filter_map(std::result::Result::ok)
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    let mut file_stamps = Vec::new();
    for entry in entries {
        let entry_path = entry.path();
        if entry_path.extension().is_none_or(|ext| ext != "md")
            || entry_path
                .file_name()
                .is_some_and(|name| name == "README.md")
        {
            continue;
        }
        let f_meta = fs::symlink_metadata(&entry_path)?;
        if custody::linked(&f_meta) {
            return Err(GuardError::policy(format!(
                "Agent roster entry contains a symlink: {}",
                entry_path.display()
            )));
        }
        if !f_meta.is_file() {
            continue;
        }
        let Some((f_stamp, f_settled)) = entry_stamp(&entry_path, false) else {
            return Err(GuardError::policy(format!(
                "Agent roster file metadata unavailable: {}",
                entry_path.display()
            )));
        };
        max_settled = max_settled.max(f_settled);
        let name = entry.file_name().to_string_lossy().into_owned();
        file_stamps.push(AgentFileStamp {
            name,
            stamp: f_stamp,
        });
    }
    Ok(Some((d_stamp, file_stamps, max_settled)))
}

fn scan_agent_roster(path: &Path) -> Result<BTreeMap<String, bool>> {
    #[cfg(test)]
    bump_roster_dir_scans();
    let mut result = BTreeMap::new();
    let mut entries = fs::read_dir(path)?
        .filter_map(std::result::Result::ok)
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let p = entry.path();
        if p.extension().is_none_or(|ext| ext != "md")
            || p.file_name().is_some_and(|name| name == "README.md")
        {
            continue;
        }
        let meta = fs::symlink_metadata(&p)?;
        if custody::linked(&meta) {
            return Err(GuardError::policy(format!(
                "Agent roster entry contains a symlink: {}",
                p.display()
            )));
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
    Ok(result)
}

fn roster_with_now(
    path: &Path,
    cache_file: Option<&Path>,
    now_ns: i128,
) -> Result<BTreeMap<String, bool>> {
    static ROSTER_CACHE: OnceLock<Mutex<BTreeMap<PathBuf, AgentRosterCacheEntry>>> =
        OnceLock::new();
    let Some((d_stamp, file_stamps, max_settled)) = inspect_agent_roster_dir(path)? else {
        return Ok(BTreeMap::new());
    };
    let key_str = path.to_string_lossy().into_owned();
    let racy_settled = now_ns > SKILL_CACHE_RACY_NS && now_ns - max_settled >= SKILL_CACHE_RACY_NS;
    let memory = ROSTER_CACHE.get_or_init(|| Mutex::new(BTreeMap::new()));
    if racy_settled
        && let Ok(guard) = memory.lock()
        && let Some(entry) = guard.get(path)
        && entry.dir_stamp == d_stamp
        && entry.files == file_stamps
    {
        return Ok(entry.roster.clone());
    }
    if racy_settled
        && let Some(cache_path) = cache_file
        && custody::owned_by_current_user(cache_path)
        && let Ok(raw) = custody::read_regular(cache_path, 512 * 1024)
        && let Ok(entry) = serde_json::from_slice::<AgentRosterCacheEntry>(&raw)
        && entry.agents_dir == key_str
        && entry.dir_stamp == d_stamp
        && entry.files == file_stamps
    {
        if let Ok(mut guard) = memory.lock() {
            guard.insert(path.to_path_buf(), entry.clone());
        }
        return Ok(entry.roster);
    }

    let result = scan_agent_roster(path)?;
    if racy_settled {
        let entry = AgentRosterCacheEntry {
            agents_dir: key_str,
            dir_stamp: d_stamp,
            files: file_stamps,
            roster: result.clone(),
        };
        if let Ok(mut guard) = memory.lock() {
            guard.insert(path.to_path_buf(), entry.clone());
        }
        if let Some(cache_path) = cache_file
            && let Ok(bytes) = serde_json::to_vec(&entry)
        {
            write_settled_cache_file(cache_path, &bytes);
        }
    }
    Ok(result)
}

fn roster(root: &Path, host: &str, loaded: &LoadedPolicy) -> Result<BTreeMap<String, bool>> {
    let mut result = BTreeMap::new();
    for name in &loaded.policy.subagent_gate.allowed_agents {
        result.entry(name.clone()).or_insert(false);
    }
    for name in &loaded.policy.subagent_gate.writer_agents {
        result.insert(name.clone(), true);
    }
    let dir = match host {
        "cursor" => root.join(".cursor/agents"),
        "antigravity" => {
            if root.join(".gemini/agents").is_dir() {
                root.join(".gemini/agents")
            } else if root.join(".antigravity/agents").is_dir() {
                root.join(".antigravity/agents")
            } else {
                root.join(".claude/agents")
            }
        }
        _ => root.join(".claude/agents"),
    };
    let cache_file = counters::directory(root)
        .ok()
        .map(|folder| folder.join(format!("agent-roster-cache-{host}.json")));
    let discovered = roster_with_now(&dir, cache_file.as_deref(), current_unix_ns())?;
    for (name, writes) in discovered {
        result
            .entry(name)
            .and_modify(|prev| *prev |= writes)
            .or_insert(writes);
    }
    Ok(result)
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
    if host != "cursor" && !allowed_tool(payload, &["agent", "task", "invoke_subagent"]) {
        return gate_response(host, true, "");
    }
    let Some(agent) = first(payload, "subagent_type")
        .or_else(|| first(payload, "agent_type"))
        .or_else(|| first(payload, "TypeName"))
        .or_else(|| first(payload, "Role"))
        .or_else(|| first(payload, "type_name"))
        .or_else(|| first(payload, "role"))
    else {
        return gate_response(host, true, "");
    };
    let map = match roster(root, host, &loaded) {
        Ok(map) => map,
        Err(error) => return gate_response(host, false, &format!("BLOCKED: {error}")),
    };
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
        .or_else(|| first(payload, "TypeName"))
        .or_else(|| first(payload, "Role"))
        .or_else(|| first(payload, "type_name"))
        .or_else(|| first(payload, "role"))
        .unwrap_or("");
    let _ = counters::release_writer(root, agent);
    Ok(GuardOutput::default())
}

fn finalize_host_output(host: &str, role: &str, out: GuardOutput) -> Result<GuardOutput> {
    if host != "antigravity" {
        return Ok(out);
    }
    if matches!(role, "edit-loop" | "subagent-release") {
        return GuardOutput::json(&json!({}));
    }
    if out.code != 0 {
        let stderr_text = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let stdout_text = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let reason = if !stderr_text.is_empty() {
            stderr_text
        } else if !stdout_text.is_empty() {
            stdout_text
        } else {
            "Blocked by agent-guardrails.".into()
        };
        return GuardOutput::json(&json!({
            "decision": "deny",
            "reason": reason,
        }));
    }
    if !out.stdout.is_empty() {
        let raw = String::from_utf8_lossy(&out.stdout);
        let advisory = serde_json::from_str::<Value>(raw.trim())
            .ok()
            .and_then(|v| {
                v.get("hookSpecificOutput")
                    .and_then(|h| h.get("additionalContext"))
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_else(|| raw.trim().to_string());
        if !advisory.is_empty() {
            return GuardOutput::json(&json!({
                "decision": "allow",
                "reason": advisory,
            }));
        }
    }
    GuardOutput::json(&json!({"decision": "allow"}))
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

/// Execute any hook role against raw stdin JSON bytes and host (`antigravity`/`claude`/`codex`/`cursor`).
pub fn hook_execute(
    explicit_root: Option<&Path>,
    explicit_host: Option<&str>,
    raw_role: &str,
    raw_stdin: &[u8],
) -> Result<GuardOutput> {
    let role = normalize_role(raw_role)
        .ok_or_else(|| GuardError::invalid(format!("Unknown hook role: {raw_role}")))?;
    let payload = parse_payload(raw_stdin);
    let host = detect_host(explicit_host, &payload);
    if !["antigravity", "claude", "codex", "cursor"].contains(&host.as_str()) {
        return Err(GuardError::invalid(format!("Unsupported hook host: {host}")));
    }
    if payload_noop(role, &payload) {
        return finalize_host_output(&host, role, GuardOutput::default());
    }
    let root = resolve_root(explicit_root, Some(&payload));
    let out = match role {
        "bash-guard" => bash_guard(&root, &host, &payload)?,
        "edit-loop" => edit_loop(&root, &host, &payload)?,
        "naming-check" => naming_check(&root, &host, &payload)?,
        "subagent-gate" => subagent_gate(&root, &host, &payload)?,
        "subagent-release" => subagent_release(&root, &payload)?,
        _ => GuardOutput::default(),
    };
    finalize_host_output(&host, role, out)
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
    let read_only = shell::is_read_only_cli(command);
    match outcome {
        Ok(None) => Ok(json!({
            "allowed": true,
            "blocked": false,
            "category": Value::Null,
            "preset": loaded.preset.as_str(),
            "root": root.display().to_string(),
            "polling_exempt": shell::polling(command),
            "read_only": read_only,
        })),
        Ok(Some(category)) => Ok(json!({
            "allowed": false,
            "blocked": true,
            "category": category,
            "preset": loaded.preset.as_str(),
            "root": root.display().to_string(),
            "polling_exempt": false,
            "read_only": false,
        })),
        Err(error) => Ok(json!({
            "allowed": false,
            "blocked": true,
            "category": error.message,
            "preset": loaded.preset.as_str(),
            "root": root.display().to_string(),
            "polling_exempt": false,
            "read_only": false,
        })),
    }
}

#[cfg(test)]
mod smart_loop_breaker_tests {
    use super::*;

    #[test]
    fn normalized_bash_signature_warns_with_diagnostic_hints_before_blocking_at_12() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).unwrap();

        let session = format!("smartloop{}", std::process::id());
        let variants = [
            "cargo test foo --lib | head -n 20",
            "cargo test foo --lib 2>&1 | tail -n 10",
            "cargo   test   foo   --lib >/dev/null",
        ];

        // 5 invocations produce zero warning.
        for i in 0..5 {
            let cmd = variants[i % variants.len()];
            let payload = json!({
                "session_id": session,
                "tool_name": "Bash",
                "command": cmd,
                "stderr": "error[E0308]: mismatched types"
            });
            let out = hook_execute(
                Some(root),
                Some("claude"),
                "bash-guard",
                &serde_json::to_vec(&payload).unwrap(),
            )
            .unwrap();
            assert_eq!(out.code, 0, "attempt {i} should pass: {out:?}");
            assert!(out.stdout.is_empty(), "attempt {i} should be silent: {out:?}");
        }

        // Attempts 6..=11 emit deterministic diagnostic hint before blocking.
        for i in 6..=11 {
            let cmd = variants[i % variants.len()];
            let payload = json!({
                "session_id": session,
                "tool_name": "Bash",
                "command": cmd,
                "stderr": "error[E0308]: mismatched types"
            });
            let out = hook_execute(
                Some(root),
                Some("claude"),
                "bash-guard",
                &serde_json::to_vec(&payload).unwrap(),
            )
            .unwrap();
            assert_eq!(out.code, 0, "attempt {i} should warn: {out:?}");
            let stdout = String::from_utf8(out.stdout).unwrap();
            assert!(stdout.contains("WARNING:"), "{stdout}");
            assert!(stdout.contains("Hint:") && stdout.contains("--nocapture"), "{stdout}");
            assert!(stdout.contains("mismatched types"), "{stdout}");
        }

        // Attempt 12 blocks cleanly.
        let payload_12 = json!({
            "session_id": session,
            "tool_name": "Bash",
            "command": variants[0],
            "stderr": "error[E0308]: mismatched types"
        });
        let blocked = hook_execute(
            Some(root),
            Some("claude"),
            "bash-guard",
            &serde_json::to_vec(&payload_12).unwrap(),
        )
        .unwrap();
        assert_eq!(blocked.code, 2);
        let stderr = String::from_utf8(blocked.stderr).unwrap();
        assert!(stderr.contains("BLOCKED:") && stderr.contains("Hint:"), "{stderr}");
        let _ = counters::reset(root, &session);
    }

    #[test]
    fn file_naming_defers_non_markdown_and_reuses_settled_skill_cache() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let skills = root.join(".claude/skills");
        fs::create_dir_all(skills.join("alpha-skill")).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();

        let (_, settled_ns) = entry_stamp(&skills, true).unwrap();
        let cache_dir = root.join("cache");
        fs::create_dir_all(&cache_dir).unwrap();
        let cache_file = cache_dir.join("skill-naming-cache-claude.json");
        let dirs = skill_dirs(root, "claude");

        SKILL_DIR_SCANS.with(|c| c.set(0));
        // Inside racy window (< 2s) always scans and avoids persisting stale cache.
        let racy_prefixes =
            skill_prefixes_with_now(&dirs, Some(&cache_file), settled_ns + SKILL_CACHE_RACY_NS - 1);
        assert_eq!(racy_prefixes, vec!["alpha-skill-".to_string()]);
        assert_eq!(SKILL_DIR_SCANS.with(|c| c.get()), 1);
        assert!(!cache_file.exists());

        // Outside racy window: first call scans once and caches; second call hits cache with 0 scans!
        let settled_now = settled_ns + SKILL_CACHE_RACY_NS + 1;
        let first = skill_prefixes_with_now(&dirs, Some(&cache_file), settled_now);
        assert_eq!(first, vec!["alpha-skill-".to_string()]);
        assert_eq!(SKILL_DIR_SCANS.with(|c| c.get()), 2);
        assert!(cache_file.is_file());

        let second = skill_prefixes_with_now(&dirs, Some(&cache_file), settled_now + 500);
        assert_eq!(second, first);
        assert_eq!(SKILL_DIR_SCANS.with(|c| c.get()), 2);

        // Editing a non-Markdown file through naming-check skips skill dir scanning altogether.
        let payload = json!({
            "session_id": "namingskip",
            "tool_name": "Edit",
            "file_path": root.join("src/App.php")
        });
        let before_scans = SKILL_DIR_SCANS.with(|c| c.get());
        let out = hook_execute(
            Some(root),
            Some("claude"),
            "naming-check",
            &serde_json::to_vec(&payload).unwrap(),
        )
        .unwrap();
        assert_eq!(out.code, 0);
        assert_eq!(SKILL_DIR_SCANS.with(|c| c.get()), before_scans);
    }

    #[test]
    fn edit_loop_discriminates_signatures_resets_on_passing_checks_and_isolates_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        fs::create_dir_all(root.join(".git")).unwrap();

        let sess_a = format!("loopA{}", std::process::id());
        let sess_b = format!("loopB{}", std::process::id());
        let target_file = root.join("src/Service.php").to_string_lossy().into_owned();

        // Fail under normalized signature A (`cargo test foo`), edit 6 times -> silent.
        let fail_a = json!({
            "session_id": sess_a,
            "tool_name": "Bash",
            "command": "cargo test foo",
            "exit_code": 1,
            "stderr": "error[E0308]: mismatch"
        });
        hook_execute(
            Some(root),
            Some("claude"),
            "bash-guard",
            &serde_json::to_vec(&fail_a).unwrap(),
        )
        .unwrap();
        let edit_payload = json!({
            "session_id": sess_a,
            "tool_name": "Edit",
            "file_path": target_file
        });
        for i in 0..6 {
            let out = hook_execute(
                Some(root),
                Some("claude"),
                "edit-loop",
                &serde_json::to_vec(&edit_payload).unwrap(),
            )
            .unwrap();
            assert_eq!(out.code, 0);
            assert!(out.stdout.is_empty(), "edit {i} under sig A should be silent");
        }

        // Switching to distinct signature B tracks `(file, sig B)` independently.
        let fail_b = json!({
            "session_id": sess_a,
            "tool_name": "Bash",
            "command": "cargo test bar",
            "exit_code": 1,
            "stderr": "error[E0425]: missing name"
        });
        hook_execute(
            Some(root),
            Some("claude"),
            "bash-guard",
            &serde_json::to_vec(&fail_b).unwrap(),
        )
        .unwrap();
        for i in 0..6 {
            let out = hook_execute(
                Some(root),
                Some("claude"),
                "edit-loop",
                &serde_json::to_vec(&edit_payload).unwrap(),
            )
            .unwrap();
            assert_eq!(out.code, 0);
            assert!(out.stdout.is_empty(), "edit {i} under sig B should be silent");
        }

        // 7th edit under sig B warns cleanly.
        let warn = hook_execute(
            Some(root),
            Some("claude"),
            "edit-loop",
            &serde_json::to_vec(&edit_payload).unwrap(),
        )
        .unwrap();
        assert!(!warn.stdout.is_empty(), "7th edit under same sig should warn");

        // Passing check via PostToolUse edit-loop resets session counters.
        let pass_check = json!({
            "session_id": sess_a,
            "tool_name": "Bash",
            "command": "cargo test bar",
            "exit_code": 0
        });
        let pass_out = hook_execute(
            Some(root),
            Some("claude"),
            "edit-loop",
            &serde_json::to_vec(&pass_check).unwrap(),
        )
        .unwrap();
        assert_eq!(pass_out.code, 0);
        for i in 0..6 {
            let out = hook_execute(
                Some(root),
                Some("claude"),
                "edit-loop",
                &serde_json::to_vec(&edit_payload).unwrap(),
            )
            .unwrap();
            assert!(out.stdout.is_empty(), "post-reset edit {i} must start fresh");
        }

        // Edits in session B must not reset session A's bash counter.
        let cmd_a = json!({"session_id": sess_a, "tool_name": "Bash", "command": "phpunit --filter A"});
        for _ in 0..6 {
            hook_execute(
                Some(root),
                Some("claude"),
                "bash-guard",
                &serde_json::to_vec(&cmd_a).unwrap(),
            )
            .unwrap();
        }
        std::thread::sleep(std::time::Duration::from_millis(25));
        let edit_b = json!({"session_id": sess_b, "tool_name": "Edit", "file_path": target_file});
        hook_execute(
            Some(root),
            Some("claude"),
            "edit-loop",
            &serde_json::to_vec(&edit_b).unwrap(),
        )
        .unwrap();
        let next_a = hook_execute(
            Some(root),
            Some("claude"),
            "bash-guard",
            &serde_json::to_vec(&cmd_a).unwrap(),
        )
        .unwrap();
        let stdout_a = String::from_utf8_lossy(&next_a.stdout);
        assert!(
            stdout_a.contains("WARNING:"),
            "session B edit leaked into session A reset: {stdout_a}"
        );
        let _ = counters::reset(root, &sess_a);
        let _ = counters::reset(root, &sess_b);
    }

    #[test]
    fn agent_roster_reuses_settled_cache_invalidates_on_edit_and_fails_closed_on_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let agents = root.join(".claude/agents");
        fs::create_dir_all(&agents).unwrap();
        let cache_dir = root.join("cache");
        fs::create_dir_all(&cache_dir).unwrap();
        let reviewer = agents.join("reviewer-agent.md");
        fs::write(&reviewer, "---\nname: reviewer-agent\nwrites: false\n---\n").unwrap();
        let cache_file = cache_dir.join("agent-roster-cache-claude.json");

        let (_, _, settled_ns) = inspect_agent_roster_dir(&agents).unwrap().unwrap();
        ROSTER_DIR_SCANS.with(|c| c.set(0));

        // Inside SKILL_CACHE_RACY_NS -> always scans live and never writes cache.
        let racy = roster_with_now(
            &agents,
            Some(&cache_file),
            settled_ns + SKILL_CACHE_RACY_NS - 1,
        )
        .unwrap();
        assert_eq!(racy.get("reviewer-agent"), Some(&false));
        assert_eq!(ROSTER_DIR_SCANS.with(|c| c.get()), 1);
        assert!(!cache_file.exists());

        // Settled beyond SKILL_CACHE_RACY_NS -> 1st call populates cache, 2nd hits with 0 scans.
        let settled_now = settled_ns + SKILL_CACHE_RACY_NS + 1;
        let first = roster_with_now(&agents, Some(&cache_file), settled_now).unwrap();
        assert_eq!(first, racy);
        assert_eq!(ROSTER_DIR_SCANS.with(|c| c.get()), 2);
        assert!(cache_file.is_file());

        let second = roster_with_now(&agents, Some(&cache_file), settled_now + 500).unwrap();
        assert_eq!(second, first);
        assert_eq!(ROSTER_DIR_SCANS.with(|c| c.get()), 2);

        // In-place file edit invalidates cache immediately.
        fs::write(&reviewer, "---\nname: reviewer-agent\nwrites: true\n---\n").unwrap();
        let (_, _, settled_edit) = inspect_agent_roster_dir(&agents).unwrap().unwrap();
        let third = roster_with_now(
            &agents,
            Some(&cache_file),
            settled_edit + SKILL_CACHE_RACY_NS + 1,
        )
        .unwrap();
        assert_eq!(third.get("reviewer-agent"), Some(&true));
        assert_eq!(ROSTER_DIR_SCANS.with(|c| c.get()), 3);

        // Symlinked entry inside agents dir fails closed.
        #[cfg(unix)]
        {
            let target = root.join("outside.md");
            fs::write(&target, "---\nname: evil\nwrites: false\n---\n").unwrap();
            std::os::unix::fs::symlink(&target, agents.join("evil-agent.md")).unwrap();
            assert!(
                roster_with_now(
                    &agents,
                    Some(&cache_file),
                    settled_edit + SKILL_CACHE_RACY_NS + 2
                )
                .is_err()
            );
        }
    }
}
