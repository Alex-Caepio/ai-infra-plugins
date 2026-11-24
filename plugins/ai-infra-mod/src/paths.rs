use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    env, fs,
    path::{Component, Path, PathBuf},
};

const WORKSPACE_KEYS: [&str; 3] = ["project", "root", "project_root"];
const PATH_KEYS: &[&str] = &[
    "project",
    "root",
    "project_root",
    "path",
    "previous_path",
    "skill_path",
    "package_dir",
    "examples_path",
    "candidate_path",
    "trace_path",
    "eval_set",
    "bundle_path",
    "output_dir",
    "results_dir",
    "report",
    "dossier",
    "destination",
    "input",
    "run_dir",
    "plan",
    "system",
    "config_path",
    "output",
    "graph",
    "import",
    "ceiling",
    "claude_dir",
    "codex_dir",
    "state_dir",
];

pub fn is_plugin_dir(path: &Path) -> bool {
    if path.join(".codex-plugin").is_dir()
        || path.join(".claude-plugin").is_dir()
        || (path.join(".mcp.json").is_file() && path.join("bin").is_dir())
        || (path.join("bin/ai-infra-mod").is_file() && path.join("Cargo.toml").is_file())
    {
        return true;
    }
    let normalized = path.to_string_lossy().replace('\\', "/");
    normalized.contains(".codex/plugins/cache")
}

pub fn env_configured_root() -> Option<PathBuf> {
    for var in ["AI_INFRA_ROOT", "ROOT_DIR"] {
        if let Some(raw) = env::var_os(var) {
            let s = raw.to_string_lossy().trim().to_string();
            if !s.is_empty() {
                let p = PathBuf::from(s);
                return Some(fs::canonicalize(&p).unwrap_or(p));
            }
        }
    }
    None
}

pub fn restricted_mode() -> bool {
    env::var("AI_INFRA_RESTRICT_ROOT")
        .ok()
        .is_some_and(|v| matches!(v.trim(), "1" | "true" | "TRUE" | "yes"))
}

pub fn parse_file_uri(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let raw_path = if let Some(after_host) = rest.strip_prefix("localhost/") {
        format!("/{after_host}")
    } else if rest.starts_with('/') {
        rest.to_string()
    } else {
        return None;
    };
    let decoded = percent_decode(&raw_path)?;
    Some(PathBuf::from(decoded))
}

fn percent_decode(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return None;
            }
            let hi = (bytes[i + 1] as char).to_digit(16)?;
            let lo = (bytes[i + 2] as char).to_digit(16)?;
            out.push((hi * 16 + lo) as u8);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Canonicalize existing path cleanly, or canonicalize deepest existing ancestor + join clean remainder.
pub fn resolve_clean(raw: &str, base: Option<&Path>) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.contains('\0') {
        return Err("Invalid empty or NUL path".to_string());
    }
    let p = Path::new(trimmed);
    let abs = if p.is_absolute() {
        p.to_path_buf()
    } else if let Some(root) = base {
        root.join(p)
    } else {
        env::current_dir()
            .map_err(|e| format!("Cannot read working directory: {e}"))?
            .join(p)
    };
    canonicalize_flex(&abs)
}

pub fn canonicalize_flex(abs: &Path) -> Result<PathBuf, String> {
    if let Ok(canon) = fs::canonicalize(abs) {
        return Ok(canon);
    }
    let mut existing: Option<&Path> = None;
    let mut tail_rev: Vec<Component<'_>> = Vec::new();
    let mut cur = abs;
    loop {
        if fs::symlink_metadata(cur).is_ok() {
            existing = Some(cur);
            break;
        }
        match (cur.parent(), cur.components().next_back()) {
            (Some(parent), Some(comp)) => {
                tail_rev.push(comp);
                cur = parent;
            }
            _ => break,
        }
    }
    let mut resolved = match existing {
        Some(anc) => fs::canonicalize(anc)
            .map_err(|e| format!("Failed to canonicalize `{}`: {e}", anc.display()))?,
        None => PathBuf::from("/"),
    };
    for comp in tail_rev.into_iter().rev() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if !resolved.pop() {
                    return Err("Path traversal escapes filesystem root".to_string());
                }
            }
            Component::Normal(seg) => resolved.push(seg),
            Component::RootDir | Component::Prefix(_) => {}
        }
    }
    Ok(resolved)
}

pub fn ensure_contained(resolved: &Path, allowed_root: &Path) -> Result<(), String> {
    let canon_root = canonicalize_flex(allowed_root)?;
    if resolved == canon_root || resolved.starts_with(&canon_root) {
        Ok(())
    } else {
        Err(format!(
            "Path `{}` escapes configured root `{}` (AI_INFRA_RESTRICT_ROOT=1)",
            resolved.display(),
            canon_root.display()
        ))
    }
}

fn is_dot_self(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return false;
    }
    let normalized = trimmed.replace('\\', "/");
    let stripped = normalized.trim_end_matches('/');
    !stripped.is_empty()
        && Path::new(stripped)
            .components()
            .all(|c| matches!(c, Component::CurDir))
}

fn check_and_maybe_rewrite_path(
    raw: &str,
    trim_hash_fragment: bool,
    rewrite_relative: bool,
    resolution_base: Option<&Path>,
    restricted_root: Option<&Path>,
) -> Result<Option<String>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let path_part = if trim_hash_fragment {
        trimmed.split('#').next().unwrap_or(trimmed)
    } else {
        trimmed
    };
    let resolved = resolve_clean(path_part, resolution_base)?;
    if let Some(bound) = restricted_root {
        ensure_contained(&resolved, bound)?;
    }
    if rewrite_relative && Path::new(trimmed).is_relative() {
        Ok(Some(resolved.to_string_lossy().into_owned()))
    } else {
        Ok(None)
    }
}

fn should_rewrite_key(key: &str, accepts_workspace_key: bool) -> bool {
    match key {
        // Standalone `skill_*` tools have no `project`/`root` parameter and run relative to server cwd,
        // so relative `path` must rewrite against `active_root`; workspace-scoped tools (`graph_file_impact`,
        // `graph_memory_links`, `omnigraph_file_context`, `context_memory_links_for_file`) require
        // repository-relative `path`/`previous_path`. `system_graph_status` requires relative `config_path`.
        "path" => !accepts_workspace_key,
        "previous_path" | "config_path" => false,
        _ => true,
    }
}

pub fn prepare_tool_arguments(
    tool_name: &str,
    schema_props: &BTreeSet<String>,
    raw_args: Value,
    active_root: Option<&Path>,
    restricted_root: Option<&Path>,
) -> Result<Value, String> {
    let mut obj = match raw_args {
        Value::Null => serde_json::Map::new(),
        Value::Object(m) => m,
        _ => return Err("Tool arguments must be a JSON object".to_string()),
    };

    let raw_cwd = env::current_dir().map_err(|e| format!("Cannot read working directory: {e}"))?;
    let canon_cwd = fs::canonicalize(&raw_cwd).unwrap_or_else(|_| raw_cwd.clone());
    let cwd_is_plugin = is_plugin_dir(&raw_cwd) || is_plugin_dir(&canon_cwd);

    // Check explicit "." / "./" in workspace or path selectors when no active_root exists inside plugin cwd.
    if active_root.is_none() && cwd_is_plugin {
        for key in WORKSPACE_KEYS.iter().chain(
            [
                "path",
                "previous_path",
                "skill_path",
                "package_dir",
                "examples_path",
                "candidate_path",
                "trace_path",
                "config_path",
            ]
            .iter(),
        ) {
            if let Some(val) = obj.get(*key).and_then(Value::as_str)
                && is_dot_self(val)
            {
                return Err(
                    "Refusing relative '.' inside plugin directory; pass explicit `project`/`root` or set AI_INFRA_ROOT."
                        .to_string(),
                );
            }
        }
    }

    let accepts_workspace_key = WORKSPACE_KEYS.iter().any(|k| schema_props.contains(*k));
    let has_explicit_workspace = WORKSPACE_KEYS.iter().any(|k| {
        obj.get(*k)
            .and_then(Value::as_str)
            .is_some_and(|s| !s.trim().is_empty())
    });
    let has_explicit_path = ["path", "skill_path", "dossier", "package_dir"]
        .iter()
        .any(|k| {
            obj.get(*k)
                .and_then(Value::as_str)
                .is_some_and(|s| !s.trim().is_empty())
        });

    // Enforce omitted project/root behavior.
    if accepts_workspace_key && !has_explicit_workspace {
        if let Some(root) = active_root {
            if let Some(primary) = WORKSPACE_KEYS.iter().find(|k| schema_props.contains(**k)) {
                obj.insert((*primary).to_string(), json!(root.to_string_lossy()));
            }
        } else if cwd_is_plugin {
            return Err(
                "Current working directory is inside a plugin directory; pass explicit `project`/`root` or set AI_INFRA_ROOT."
                    .to_string(),
            );
        }
    } else if !accepts_workspace_key
        && matches!(
            tool_name,
            "skill_validate"
                | "skill_budget_check"
                | "skill_vet"
                | "skill_package"
                | "skill_scaffold"
                | "skillcache_gate1"
        )
        && !has_explicit_path
    {
        if let Some(root) = active_root
            && matches!(tool_name, "skill_budget_check" | "skill_vet")
        {
            obj.insert("path".to_string(), json!(root.to_string_lossy()));
        } else if active_root.is_none() && cwd_is_plugin {
            return Err(
                "Current working directory is inside a plugin directory; pass explicit `path`/`skill_path`/`package_dir` or set AI_INFRA_ROOT."
                    .to_string(),
            );
        }
    }

    // Resolve relative paths when active_root is configured, and enforce containment when AI_INFRA_RESTRICT_ROOT=1.
    // Note: `source` is a filesystem bundle path only on `context_import_bundle`; on omnigraph_* it names a branch.
    // `target` is never a filesystem path (it denotes a git diff/range selector or omnigraph branch).
    let resolution_base = restricted_root.or(active_root);
    if resolution_base.is_some() || restricted_root.is_some() {
        for key in PATH_KEYS.iter().copied().chain(
            (tool_name == "context_import_bundle").then_some("source"),
        ) {
            if let Some(raw_val) = obj.get(key).and_then(Value::as_str).map(str::to_string) {
                if key == "report" && matches!(raw_val.trim(), "auto" | "none") {
                    continue;
                }
                let rewrite = should_rewrite_key(key, accepts_workspace_key);
                if let Some(rewritten) = check_and_maybe_rewrite_path(
                    &raw_val,
                    false,
                    rewrite,
                    resolution_base,
                    restricted_root,
                )? {
                    obj.insert(key.to_string(), json!(rewritten));
                }
            }
        }
        for array_key in ["allow_root", "paths", "files", "sources", "merge_from"] {
            if let Some(items) = obj.get(array_key).and_then(Value::as_array).cloned() {
                let mut rewritten = Vec::with_capacity(items.len());
                let trim_hash = array_key == "sources";
                let rewrite_items = array_key == "allow_root";
                for item in items {
                    if let Some(raw_s) = item.as_str() {
                        if let Some(new_s) = check_and_maybe_rewrite_path(
                            raw_s,
                            trim_hash,
                            rewrite_items,
                            resolution_base,
                            restricted_root,
                        )? {
                            rewritten.push(json!(new_s));
                        } else {
                            rewritten.push(item);
                        }
                    } else if array_key == "files"
                        && let Some(file_obj) = item.as_object()
                    {
                        for k in ["path", "previous_path"] {
                            if let Some(s) = file_obj.get(k).and_then(Value::as_str) {
                                let _ = check_and_maybe_rewrite_path(
                                    s,
                                    false,
                                    false,
                                    resolution_base,
                                    restricted_root,
                                )?;
                            }
                        }
                        rewritten.push(item);
                    } else {
                        rewritten.push(item);
                    }
                }
                obj.insert(array_key.to_string(), Value::Array(rewritten));
            }
        }
    }

    Ok(Value::Object(obj))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_and_target_on_omnigraph_and_graph_are_never_treated_as_paths() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        fs::create_dir(&repo).unwrap();
        let repo = fs::canonicalize(&repo).unwrap();

        let props: BTreeSet<String> = ["project", "root", "source", "target", "from", "to"]
            .into_iter()
            .map(str::to_string)
            .collect();

        for tool in [
            "omnigraph_branches_diff",
            "omnigraph_branch_diff",
            "context_omnigraph_merge_keep",
            "graph_change_impact",
        ] {
            let out = prepare_tool_arguments(
                tool,
                &props,
                json!({
                    "source": "feature/x",
                    "target": "main",
                    "from": "feature/x",
                    "to": "main"
                }),
                Some(&repo),
                Some(&repo),
            )
            .unwrap_or_else(|e| panic!("{tool} failed: {e}"));
            assert_eq!(out["source"], "feature/x", "{tool}");
            assert_eq!(out["target"], "main", "{tool}");
            assert_eq!(out["project"], repo.to_string_lossy().as_ref());
        }

        // `context_import_bundle` DOES treat `source` as a filesystem bundle path and blocks escape.
        let bundle_props: BTreeSet<String> = ["project", "root", "source"]
            .into_iter()
            .map(str::to_string)
            .collect();
        let err = prepare_tool_arguments(
            "context_import_bundle",
            &bundle_props,
            json!({ "source": "../outside.cgb.zip" }),
            Some(&repo),
            Some(&repo),
        )
        .unwrap_err();
        assert!(err.contains("AI_INFRA_RESTRICT_ROOT"), "{err}");

        // `skillcache_compile` / `skillcache_traces` restrict `candidate_path` and `trace_path`.
        let sc_props: BTreeSet<String> = ["project", "candidate_path", "trace_path"]
            .into_iter()
            .map(str::to_string)
            .collect();
        let err2 = prepare_tool_arguments(
            "skillcache_compile",
            &sc_props,
            json!({ "candidate_path": "../escape.json", "trace_path": "../escape.jsonl" }),
            Some(&repo),
            Some(&repo),
        )
        .unwrap_err();
        assert!(err2.contains("AI_INFRA_RESTRICT_ROOT"), "{err2}");
    }

    #[test]
    fn workspace_relative_path_keys_are_validated_without_rewriting_to_absolute() {
        let tmp = tempfile::tempdir().unwrap();
        let repo = tmp.path().join("repo");
        fs::create_dir(&repo).unwrap();
        let repo = fs::canonicalize(&repo).unwrap();

        let props: BTreeSet<String> = [
            "project",
            "path",
            "previous_path",
            "config_path",
            "files",
            "paths",
            "sources",
        ]
        .into_iter()
        .map(str::to_string)
        .collect();

        let out = prepare_tool_arguments(
            "graph_memory_links",
            &props,
            json!({
                "path": "src/Order.php",
                "previous_path": "src/OldOrder.php",
                "config_path": "system.json",
                "files": ["src/a.php", { "path": "src/b.php", "previous_path": "src/old_b.php" }],
                "paths": ["src/c.php"],
                "sources": ["src/d.php#L12"]
            }),
            Some(&repo),
            Some(&repo),
        )
        .unwrap();
        assert_eq!(out["path"], "src/Order.php");
        assert_eq!(out["previous_path"], "src/OldOrder.php");
        assert_eq!(out["config_path"], "system.json");
        assert_eq!(out["files"][0], "src/a.php");
        assert_eq!(out["paths"][0], "src/c.php");
        assert_eq!(out["sources"][0], "src/d.php#L12");

        // Escape attempts in object `files` entries and `config_path` MUST still fail!
        let err_obj = prepare_tool_arguments(
            "graph_memory_links",
            &props,
            json!({ "files": [{ "path": "../outside/secret.php" }] }),
            Some(&repo),
            Some(&repo),
        )
        .unwrap_err();
        assert!(err_obj.contains("AI_INFRA_RESTRICT_ROOT"), "{err_obj}");

        let err_cfg = prepare_tool_arguments(
            "system_graph_status",
            &props,
            json!({ "config_path": "../outside/system.json" }),
            Some(&repo),
            Some(&repo),
        )
        .unwrap_err();
        assert!(err_cfg.contains("AI_INFRA_RESTRICT_ROOT"), "{err_cfg}");
    }
}
