//! CLI, Hook dispatcher, and MCP server binary `guardrails`.
use agent_guardrails::{
    counters, custody,
    error::{GuardError, Result},
    guards, mcp,
    policy::{self, LoadedPolicy},
    presets::Preset,
};
use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    io::{self, BufReader, Read, Write},
    path::PathBuf,
    process::ExitCode,
};

fn help_text() -> &'static str {
    concat!(
        "guardrails ",
        env!("CARGO_PKG_VERSION"),
        " - Portable AST shell guard, loop breaker, naming check, and write-lock gate\n\n",
        "USAGE:\n",
        "  guardrails <COMMAND> [OPTIONS]\n\n",
        "COMMANDS:\n",
        "  check-command [--root PATH] [--preset NAME] [--json] <COMMAND>\n",
        "      Validate a shell command without executing it (exit 0=allow, 2=block).\n",
        "  check-naming [--root PATH] [--host HOST] [--json] <PATH>...\n",
        "      Validate target paths against naming conventions & policy globs.\n",
        "  policy-init [--root PATH] [--preset NAME] [--force] [--json]\n",
        "      Write .agent-guardrails/policy.json with preset: generic|laravel|symfony|wordpress|node|python|rust.\n",
        "  status [--root PATH] [--session ID] [--reset] [--json]\n",
        "      Inspect active policy, presets, loop counters in $TMPDIR, and write locks.\n",
        "  hook [HOST] <ROLE> [--root PATH] [--host HOST]\n",
        "      Run hook handler on stdin JSON. Roles: bash-guard | edit-loop | naming-check | subagent-gate | subagent-release.\n",
        "  mcp [--root PATH]\n",
        "      Run stdio JSON-RPC 2.0 MCP server.\n",
        "  --help, -h | --version, -V\n"
    )
}

fn require_value(args: &[String], index: &mut usize, flag: &str) -> Result<String> {
    *index += 1;
    args.get(*index)
        .cloned()
        .ok_or_else(|| GuardError::invalid(format!("Missing value after {flag}")))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            let _ = writeln!(io::stderr(), "error: {error}");
            ExitCode::from(if error.kind == agent_guardrails::error::ErrorKind::Policy {
                2
            } else {
                1
            })
        }
    }
}

fn write_streams(stdout: &[u8], stderr: &[u8]) -> Result<()> {
    if !stdout.is_empty() {
        let mut out = io::stdout().lock();
        out.write_all(stdout)?;
        out.flush()?;
    }
    if !stderr.is_empty() {
        let mut err = io::stderr().lock();
        err.write_all(stderr)?;
        err.flush()?;
    }
    Ok(())
}

fn print_json(value: &Value) -> Result<()> {
    let mut text = serde_json::to_string_pretty(value)?;
    text.push('\n');
    write_streams(text.as_bytes(), &[])
}

pub fn run(args: &[String]) -> Result<u8> {
    let Some(first) = args.first().map(String::as_str) else {
        write_streams(help_text().as_bytes(), &[])?;
        return Ok(0);
    };
    match first {
        "--help" | "-h" | "help" => {
            write_streams(help_text().as_bytes(), &[])?;
            Ok(0)
        }
        "--version" | "-V" => {
            let v = format!("guardrails {}\n", env!("CARGO_PKG_VERSION"));
            write_streams(v.as_bytes(), &[])?;
            Ok(0)
        }
        "check-command" => cmd_check_command(&args[1..]),
        "check-naming" => cmd_check_naming(&args[1..]),
        "policy-init" => cmd_policy_init(&args[1..]),
        "status" => cmd_status(&args[1..]),
        "hook" => cmd_hook(&args[1..]),
        "mcp" => cmd_mcp(&args[1..]),
        other => Err(GuardError::invalid(format!(
            "Unknown command '{other}'. Run `guardrails --help`."
        ))),
    }
}

fn cmd_check_command(args: &[String]) -> Result<u8> {
    let mut root: Option<PathBuf> = None;
    let mut preset: Option<Preset> = None;
    let mut json_out = false;
    let mut command_arg: Option<String> = None;
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                write_streams(help_text().as_bytes(), &[])?;
                return Ok(0);
            }
            "--root" => root = Some(PathBuf::from(require_value(args, &mut i, "--root")?)),
            "--preset" => preset = Some(Preset::parse(&require_value(args, &mut i, "--preset")?)?),
            "--command" => command_arg = Some(require_value(args, &mut i, "--command")?),
            "--json" => json_out = true,
            "--" => {
                positional.extend_from_slice(&args[i + 1..]);
                break;
            }
            arg if arg.starts_with('-') => {
                return Err(GuardError::invalid(format!("Unknown option '{arg}'")));
            }
            _ => positional.push(args[i].clone()),
        }
        i += 1;
    }
    let command = if let Some(c) = command_arg {
        c
    } else if !positional.is_empty() {
        positional.join(" ")
    } else {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        buf.trim_end_matches('\n').to_string()
    };
    if command.is_empty() {
        return Err(GuardError::invalid(
            "Provide a command argument, --command, or stdin text",
        ));
    }
    let resolved_root = guards::resolve_root(root.as_deref(), None);
    let result = guards::check_command(&resolved_root, &command, preset)?;
    let blocked = result["blocked"].as_bool().unwrap_or(false);
    if json_out {
        print_json(&result)?;
    } else if blocked {
        let cat = result["category"].as_str().unwrap_or("policy violation");
        write_streams(&[], format!("BLOCKED: {cat}\n").as_bytes())?;
    } else {
        write_streams(
            format!("ALLOWED (preset: {})\n", result["preset"].as_str().unwrap_or("generic"))
                .as_bytes(),
            &[],
        )?;
    }
    Ok(if blocked { 2 } else { 0 })
}

fn cmd_check_naming(args: &[String]) -> Result<u8> {
    let mut root: Option<PathBuf> = None;
    let mut host = String::from("claude");
    let mut json_out = false;
    let mut paths = BTreeSet::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                write_streams(help_text().as_bytes(), &[])?;
                return Ok(0);
            }
            "--root" => root = Some(PathBuf::from(require_value(args, &mut i, "--root")?)),
            "--host" => host = require_value(args, &mut i, "--host")?.to_ascii_lowercase(),
            "--path" => {
                paths.insert(require_value(args, &mut i, "--path")?);
            }
            "--json" => json_out = true,
            arg if arg.starts_with('-') => {
                return Err(GuardError::invalid(format!("Unknown option '{arg}'")));
            }
            arg => {
                paths.insert(arg.to_string());
            }
        }
        i += 1;
    }
    if paths.is_empty() {
        return Err(GuardError::invalid("Provide at least one path to check"));
    }
    let resolved_root = guards::resolve_root(root.as_deref(), None);
    let loaded = policy::load_policy(&resolved_root, None).unwrap_or_else(|_| LoadedPolicy {
        preset: Preset::Generic,
        policy: policy::HookPolicy::template(Preset::Generic),
        source_path: None,
        strict_custody: false,
    });
    let reason = guards::validate_naming_paths(&resolved_root, &host, &paths, &loaded)?;
    let blocked = reason.is_some();
    if json_out {
        print_json(&json!({
            "allowed": !blocked,
            "blocked": blocked,
            "reason": reason,
            "checked_paths": paths.iter().collect::<Vec<_>>(),
            "root": resolved_root.display().to_string(),
        }))?;
    } else if let Some(msg) = reason {
        write_streams(&[], format!("BLOCKED: {msg}\n").as_bytes())?;
    } else {
        write_streams(b"OK: all paths satisfy naming rules\n", &[])?;
    }
    Ok(if blocked { 2 } else { 0 })
}

fn cmd_policy_init(args: &[String]) -> Result<u8> {
    let mut root: Option<PathBuf> = None;
    let mut preset = Preset::Generic;
    let mut force = false;
    let mut json_out = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                write_streams(help_text().as_bytes(), &[])?;
                return Ok(0);
            }
            "--root" => root = Some(PathBuf::from(require_value(args, &mut i, "--root")?)),
            "--preset" => preset = Preset::parse(&require_value(args, &mut i, "--preset")?)?,
            "--force" => force = true,
            "--json" => json_out = true,
            arg if arg.starts_with('-') => {
                return Err(GuardError::invalid(format!("Unknown option '{arg}'")));
            }
            positional => preset = Preset::parse(positional)?,
        }
        i += 1;
    }
    let target_root = root
        .map(|p| custody::absolute_lexical(&p))
        .transpose()?
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    let written = policy::init_policy(&target_root, preset, force)?;
    if json_out {
        print_json(&json!({
            "initialized": true,
            "preset": preset.as_str(),
            "policy_path": written.display().to_string(),
        }))?;
    } else {
        write_streams(
            format!(
                "Initialized {} with preset '{}'\n",
                written.display(),
                preset.as_str()
            )
            .as_bytes(),
            &[],
        )?;
    }
    Ok(0)
}

fn cmd_status(args: &[String]) -> Result<u8> {
    let mut root: Option<PathBuf> = None;
    let mut session: Option<String> = None;
    let mut reset = false;
    let mut json_out = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                write_streams(help_text().as_bytes(), &[])?;
                return Ok(0);
            }
            "--root" => root = Some(PathBuf::from(require_value(args, &mut i, "--root")?)),
            "--session" => session = Some(require_value(args, &mut i, "--session")?),
            "--reset" => reset = true,
            "--json" => json_out = true,
            arg => return Err(GuardError::invalid(format!("Unknown argument '{arg}'"))),
        }
        i += 1;
    }
    let resolved_root = guards::resolve_root(root.as_deref(), None);
    let policy_info = policy::inspect_policy(&resolved_root, None)?;
    let removed = if reset {
        counters::reset(&resolved_root, &counters::sanitize_session(session.as_deref()))?
    } else {
        0
    };
    let loop_status = counters::inspect_status(&resolved_root, session.as_deref())?;
    let report = json!({
        "policy": policy_info,
        "runtime": loop_status,
        "reset_removed_counters": if reset { json!(removed) } else { Value::Null },
    });
    if json_out {
        print_json(&report)?;
    } else {
        let pf = report["policy"]["policy_file"]
            .as_str()
            .unwrap_or("(built-in default)");
        let out = format!(
            "root: {}\npreset: {} ({})\npolicy_file: {}\nrules: {} builtin + {} custom\ncounters ({}): cmd={} (max={}) edit={} (max={})\nwriter_lock: {}\n",
            report["policy"]["root"].as_str().unwrap_or(""),
            report["policy"]["preset"].as_str().unwrap_or("generic"),
            report["policy"]["preset_description"].as_str().unwrap_or(""),
            pf,
            report["policy"]["builtin_rule_count"],
            report["policy"]["custom_rule_count"],
            loop_status.session_id,
            loop_status.command_counters,
            loop_status.max_command_count,
            loop_status.edit_counters,
            loop_status.max_edit_count,
            loop_status.active_writer_lock.as_deref().unwrap_or("none"),
        );
        write_streams(out.as_bytes(), &[])?;
    }
    Ok(0)
}

fn cmd_hook(args: &[String]) -> Result<u8> {
    let mut root: Option<PathBuf> = None;
    let mut host: Option<String> = None;
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                write_streams(help_text().as_bytes(), &[])?;
                return Ok(0);
            }
            "--root" => root = Some(PathBuf::from(require_value(args, &mut i, "--root")?)),
            "--host" => host = Some(require_value(args, &mut i, "--host")?),
            arg if arg.starts_with('-') => {
                return Err(GuardError::invalid(format!("Unknown hook option '{arg}'")));
            }
            arg => positional.push(arg.to_string()),
        }
        i += 1;
    }
    let (resolved_host, role) = match positional.as_slice() {
        [r] => (host, r.as_str()),
        [h, r] => (Some(h.clone()), r.as_str()),
        _ => {
            return Err(GuardError::invalid(
                "Usage: guardrails hook [claude|codex|cursor] <bash-guard|edit-loop|naming-check|subagent-gate|subagent-release>",
            ));
        }
    };
    let mut stdin_bytes = Vec::new();
    io::stdin()
        .take(1024 * 1024)
        .read_to_end(&mut stdin_bytes)?;
    let out = guards::hook_execute(
        root.as_deref(),
        resolved_host.as_deref(),
        role,
        &stdin_bytes,
    )?;
    write_streams(&out.stdout, &out.stderr)?;
    Ok(u8::try_from(out.code.clamp(0, 255)).unwrap_or(1))
}

fn cmd_mcp(args: &[String]) -> Result<u8> {
    let mut root: Option<PathBuf> = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--help" | "-h" => {
                write_streams(help_text().as_bytes(), &[])?;
                return Ok(0);
            }
            "--root" => root = Some(PathBuf::from(require_value(args, &mut i, "--root")?)),
            arg => return Err(GuardError::invalid(format!("Unknown mcp option '{arg}'"))),
        }
        i += 1;
    }
    let resolved_root = match root.as_deref() {
        Some(explicit) => guards::resolve_root(Some(explicit), None),
        None => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
            if custody::inside_plugin_dir(&cwd) {
                std::fs::canonicalize(&cwd).unwrap_or(cwd)
            } else {
                guards::resolve_root(None, None)
            }
        }
    };
    let stdin = io::stdin();
    let stdout = io::stdout();
    mcp::serve(&resolved_root, BufReader::new(stdin.lock()), stdout.lock())?;
    Ok(0)
}
