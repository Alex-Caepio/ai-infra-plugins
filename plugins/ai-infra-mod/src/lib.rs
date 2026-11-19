pub mod backends;
pub mod hooks;
pub mod mcp;
pub mod paths;

use backends::{BackendKind, discover_all, locate_binary};
use serde_json::json;
use std::{
    env,
    path::PathBuf,
    process::{Command, Stdio},
};

fn usage() -> &'static str {
    concat!(
        "ai-infra-mod ",
        env!("CARGO_PKG_VERSION"),
        " — Unified AI-Infrastructure suite CLI, 56-tool MCP multiplexer, & lifecycle hook router\n\n",
        "USAGE:\n",
        "  ai-infra-mod status [--project <dir>] [--json]\n",
        "  ai-infra-mod mcp\n",
        "  ai-infra-mod hook [claude|codex|cursor] <role> [OPTIONS]\n",
        "  ai-infra-mod <graph|orch|context|skill|guard> [SUBSYSTEM_ARGS...]\n",
        "  ai-infra-mod --help | --version\n"
    )
}

fn take_opt(args: &[String], names: &[&str]) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        for name in names {
            let prefix = format!("{name}=");
            if args[i] == *name {
                return args.get(i + 1).cloned();
            }
            if let Some(val) = args[i].strip_prefix(&prefix) {
                return Some(val.to_string());
            }
        }
        i += 1;
    }
    None
}

pub fn run_status(args: &[String]) -> Result<i32, String> {
    if args.iter().any(|a| matches!(a.as_str(), "-h" | "--help")) {
        println!("Usage: ai-infra-mod status [--project <dir>] [--json]");
        return Ok(0);
    }
    let as_json = args.iter().any(|a| a == "--json");
    let explicit = take_opt(args, &["--project", "--root"]);
    let env_root = paths::env_configured_root();
    let restricted = paths::restricted_mode();

    let resolved_root = if let Some(raw) = explicit {
        let resolved = paths::resolve_clean(&raw, env_root.as_deref())?;
        if restricted
            && let Some(bound) = env_root.as_deref()
        {
            paths::ensure_contained(&resolved, bound)?;
        }
        Some(resolved)
    } else if let Some(root) = env_root {
        Some(root)
    } else {
        let cwd = env::current_dir().map_err(|e| format!("Cannot read cwd: {e}"))?;
        if paths::is_plugin_dir(&cwd) {
            None
        } else {
            Some(std::fs::canonicalize(&cwd).unwrap_or(cwd))
        }
    };

    let backends = discover_all();
    let markers = resolved_root.as_ref().map(|r| {
        json!({
            "path": r,
            "exists": r.is_dir(),
            "git": r.join(".git").exists(),
            "system_json": r.join("system.json").is_file(),
            "project_brain": r.join("project-brain").is_dir(),
            "memory_bank": r.join("memory-bank").is_dir(),
            "agent_guardrails": r.join(".agent-guardrails").is_dir(),
        })
    });

    if as_json {
        let out = json!({
            "mod": "ai-infra-mod",
            "version": env!("CARGO_PKG_VERSION"),
            "restricted_mode": restricted,
            "root": markers,
            "backends": backends.iter().map(|b| json!({
                "id": b.kind.id(),
                "namespace": b.kind.cli_namespace(),
                "binary": b.path,
                "available": b.available,
                "version": b.version,
            })).collect::<Vec<_>>()
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?
        );
    } else {
        println!(
            "ai-infra-mod {} (restricted_root={})",
            env!("CARGO_PKG_VERSION"),
            restricted
        );
        match &resolved_root {
            Some(r) => println!("Project root: {}", r.display()),
            None => println!("Project root: <unspecified — pass --project or set AI_INFRA_ROOT>"),
        }
        for b in &backends {
            println!(
                "  [{:<5}] {:<22} {} ({})",
                b.kind.cli_namespace(),
                b.kind.id(),
                if b.available { "ready" } else { "missing" },
                b.version.as_deref().unwrap_or("unknown")
            );
        }
    }
    Ok(0)
}

fn passthrough(kind: BackendKind, args: &[String]) -> Result<i32, String> {
    let bin = locate_binary(kind)?;
    let status = Command::new(&bin)
        .args(args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| format!("Failed executing {}: {e}", bin.display()))?;
    Ok(status.code().unwrap_or(1))
}

pub fn dispatch(args: &[String]) -> Result<i32, String> {
    let Some(first) = args.first().map(String::as_str) else {
        print!("{}", usage());
        return Ok(0);
    };
    if matches!(first, "-h" | "--help" | "help") {
        print!("{}", usage());
        return Ok(0);
    }
    if matches!(first, "-V" | "--version" | "version") {
        println!("ai-infra-mod {}", env!("CARGO_PKG_VERSION"));
        return Ok(0);
    }
    let rest = &args[1..];
    match first {
        "status" => run_status(rest),
        "mcp" => {
            mcp::serve_stdio()?;
            Ok(0)
        }
        "hook" => hooks::run(rest),
        "graph" => passthrough(BackendKind::Graph, rest),
        "orch" => passthrough(BackendKind::Orch, rest),
        "context" => passthrough(BackendKind::Context, rest),
        "skill" => passthrough(BackendKind::Skill, rest),
        "guard" => passthrough(BackendKind::Guard, rest),
        other => Err(format!(
            "Unknown subcommand `{other}`. Run `ai-infra-mod --help`."
        )),
    }
}

pub fn normalize_root_for_status(raw: &str, base: Option<&PathBuf>) -> Result<PathBuf, String> {
    paths::resolve_clean(raw, base.map(PathBuf::as_path))
}
