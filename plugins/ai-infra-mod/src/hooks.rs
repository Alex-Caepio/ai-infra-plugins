use crate::backends::{BackendKind, locate_binary};
use serde_json::{Value, json};
use std::{
    env,
    io::{Read, Write},
    process::{Command, Stdio},
};

struct SubHookOutcome {
    code: i32,
    stdout: String,
    stderr: String,
}

fn detect_host(explicit: Option<&str>) -> String {
    if let Some(h) = explicit {
        return h.to_string();
    }
    if env::var_os("CLAUDE_PROJECT_DIR").is_some() || env::var_os("CLAUDE_PLUGIN_ROOT").is_some() {
        "claude".to_string()
    } else if env::var_os("CODEX_THREAD_ID").is_some() {
        "codex".to_string()
    } else {
        "claude".to_string()
    }
}

fn extract_flag_value(args: &[String], flag: &str) -> Option<String> {
    let prefix = format!("{flag}=");
    let mut i = 0;
    while i < args.len() {
        if args[i] == flag {
            return args.get(i + 1).cloned();
        }
        if let Some(rest) = args[i].strip_prefix(&prefix) {
            return Some(rest.to_string());
        }
        i += 1;
    }
    None
}

fn infer_root(extra_args: &[String], stdin_bytes: &[u8]) -> String {
    if let Some(r) = extract_flag_value(extra_args, "--root")
        .or_else(|| extract_flag_value(extra_args, "--project"))
    {
        return r;
    }
    if let Ok(val) = serde_json::from_slice::<Value>(stdin_bytes)
        && let Some(cwd) = val.get("cwd").and_then(Value::as_str).filter(|s| !s.is_empty())
    {
        return cwd.to_string();
    }
    if let Some(r) = crate::paths::env_configured_root() {
        return r.to_string_lossy().into_owned();
    }
    ".".to_string()
}

fn invoke_sub(kind: BackendKind, argv: &[String], stdin_bytes: &[u8]) -> Result<SubHookOutcome, String> {
    let bin = locate_binary(kind)?;
    let mut child = Command::new(&bin)
        .args(argv)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn {}: {e}", bin.display()))?;
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(stdin_bytes);
    }
    let out = child
        .wait_with_output()
        .map_err(|e| format!("Failed waiting on {}: {e}", bin.display()))?;
    Ok(SubHookOutcome {
        code: out.status.code().unwrap_or(1),
        stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&out.stderr).into_owned(),
    })
}

fn merge_context_outputs(outcomes: Vec<SubHookOutcome>) -> (String, String) {
    let mut json_chunks: Vec<Value> = Vec::new();
    let mut additional_parts: Vec<String> = Vec::new();
    let mut plain_parts: Vec<String> = Vec::new();
    let mut stderr_parts: Vec<String> = Vec::new();
    let mut event_name: Option<String> = None;

    for out in outcomes {
        if !out.stderr.trim().is_empty() {
            stderr_parts.push(out.stderr.trim_end().to_string());
        }
        let s = out.stdout.trim();
        if s.is_empty() {
            continue;
        }
        if let Ok(val) = serde_json::from_str::<Value>(s) {
            if let Some(ctx) = val
                .pointer("/hookSpecificOutput/additionalContext")
                .and_then(Value::as_str)
            {
                if event_name.is_none() {
                    event_name = val
                        .pointer("/hookSpecificOutput/hookEventName")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                }
                if !ctx.trim().is_empty() {
                    additional_parts.push(ctx.trim_end().to_string());
                }
            }
            json_chunks.push(val);
        } else {
            plain_parts.push(s.to_string());
        }
    }

    let stderr = if stderr_parts.is_empty() {
        String::new()
    } else {
        format!("{}\n", stderr_parts.join("\n"))
    };

    if !json_chunks.is_empty() || !additional_parts.is_empty() {
        let mut combined_ctx = additional_parts;
        combined_ctx.extend(plain_parts);
        let mut merged = json_chunks.into_iter().next().unwrap_or_else(|| json!({}));
        if !combined_ctx.is_empty() {
            let mut hook_out = serde_json::Map::new();
            if let Some(ev) = event_name {
                hook_out.insert("hookEventName".to_string(), json!(ev));
            }
            hook_out.insert(
                "additionalContext".to_string(),
                json!(combined_ctx.join("\n\n")),
            );
            merged["hookSpecificOutput"] = Value::Object(hook_out);
        }
        return (format!("{merged}\n"), stderr);
    }

    let stdout = if plain_parts.is_empty() {
        String::new()
    } else {
        format!("{}\n", plain_parts.join("\n\n"))
    };
    (stdout, stderr)
}

fn emit_and_return(code: i32, stdout: &str, stderr: &str) -> i32 {
    if !stdout.is_empty() {
        print!("{stdout}");
    }
    if !stderr.is_empty() {
        eprint!("{stderr}");
    }
    code
}

pub fn run(args: &[String]) -> Result<i32, String> {
    if args.is_empty() || matches!(args[0].as_str(), "-h" | "--help") {
        println!(
            "Usage: ai-infra-mod hook [claude|codex|cursor] <role> [OPTIONS]\n\
             Roles: session-start | user-prompt | pre-bash | pre-write-edit | pre-subagent | post-write-edit | post-subagent | subagent-stop | stop"
        );
        return Ok(0);
    }
    let (explicit_host, rest) = if matches!(args[0].as_str(), "claude" | "codex" | "cursor") {
        (Some(args[0].as_str()), &args[1..])
    } else {
        (None, args)
    };
    let Some(role) = rest.first().map(String::as_str) else {
        return Err("Missing hook role".to_string());
    };
    let extra = &rest[1..];
    let host_flag = extract_flag_value(extra, "--host");
    let host = detect_host(host_flag.as_deref().or(explicit_host));

    let mut stdin_bytes = Vec::new();
    std::io::stdin()
        .read_to_end(&mut stdin_bytes)
        .map_err(|e| format!("Failed reading hook stdin: {e}"))?;
    let root = infer_root(extra, &stdin_bytes);

    match role {
        "session-start" => {
            let mut results = Vec::new();
            for cg_role in ["local-context", "context-continuity"] {
                if let Ok(out) = invoke_sub(
                    BackendKind::Context,
                    &[
                        "hook".into(),
                        host.clone(),
                        cg_role.into(),
                        "--root".into(),
                        root.clone(),
                    ],
                    &stdin_bytes,
                ) && out.code == 0
                {
                    results.push(out);
                }
            }
            let (stdout, stderr) = merge_context_outputs(results);
            Ok(emit_and_return(0, &stdout, &stderr))
        }
        "user-prompt" => {
            let mut results = Vec::new();
            for cg_role in ["working-memory-read", "context-continuity"] {
                if let Ok(out) = invoke_sub(
                    BackendKind::Context,
                    &[
                        "hook".into(),
                        host.clone(),
                        cg_role.into(),
                        "--root".into(),
                        root.clone(),
                    ],
                    &stdin_bytes,
                ) && out.code == 0
                {
                    results.push(out);
                }
            }
            let (stdout, stderr) = merge_context_outputs(results);
            Ok(emit_and_return(0, &stdout, &stderr))
        }
        "stop" => {
            let mut results = Vec::new();
            for cg_role in ["working-memory-write", "context-continuity"] {
                if let Ok(out) = invoke_sub(
                    BackendKind::Context,
                    &[
                        "hook".into(),
                        host.clone(),
                        cg_role.into(),
                        "--root".into(),
                        root.clone(),
                    ],
                    &stdin_bytes,
                ) && out.code == 0
                {
                    results.push(out);
                }
            }
            let (stdout, stderr) = merge_context_outputs(results);
            Ok(emit_and_return(0, &stdout, &stderr))
        }
        "pre-bash" => {
            let out = invoke_sub(
                BackendKind::Guard,
                &["hook".into(), host, "bash-guard".into()],
                &stdin_bytes,
            )?;
            Ok(emit_and_return(out.code, &out.stdout, &out.stderr))
        }
        "pre-write-edit" => {
            let out = invoke_sub(
                BackendKind::Guard,
                &["hook".into(), host, "naming-check".into()],
                &stdin_bytes,
            )?;
            Ok(emit_and_return(out.code, &out.stdout, &out.stderr))
        }
        "post-write-edit" => {
            let out = invoke_sub(
                BackendKind::Guard,
                &["hook".into(), host, "edit-loop".into()],
                &stdin_bytes,
            )?;
            Ok(emit_and_return(out.code, &out.stdout, &out.stderr))
        }
        "pre-subagent" => {
            let gr = invoke_sub(
                BackendKind::Guard,
                &["hook".into(), host.clone(), "subagent-gate".into()],
                &stdin_bytes,
            )?;
            if gr.code != 0 {
                return Ok(emit_and_return(gr.code, &gr.stdout, &gr.stderr));
            }
            let oe = invoke_sub(
                BackendKind::Orch,
                &[
                    "hook".into(),
                    "subagent-gate".into(),
                    "--host".into(),
                    host,
                ],
                &stdin_bytes,
            )?;
            if oe.code != 0 {
                return Ok(emit_and_return(oe.code, &oe.stdout, &oe.stderr));
            }
            let (stdout, stderr) = merge_context_outputs(vec![gr, oe]);
            Ok(emit_and_return(0, &stdout, &stderr))
        }
        "post-subagent" => {
            let oe = invoke_sub(
                BackendKind::Orch,
                &[
                    "hook".into(),
                    "subagent-dispatch".into(),
                    "--host".into(),
                    host,
                ],
                &stdin_bytes,
            )?;
            Ok(emit_and_return(oe.code, &oe.stdout, &oe.stderr))
        }
        "subagent-stop" => {
            let oe = invoke_sub(
                BackendKind::Orch,
                &[
                    "hook".into(),
                    "subagent-dispatch".into(),
                    "--host".into(),
                    host.clone(),
                ],
                &stdin_bytes,
            )?;
            let gr = invoke_sub(
                BackendKind::Guard,
                &["hook".into(), host, "subagent-release".into()],
                &stdin_bytes,
            )?;
            let code = if gr.code != 0 { gr.code } else { oe.code };
            let (stdout, stderr) = merge_context_outputs(vec![oe, gr]);
            Ok(emit_and_return(code, &stdout, &stderr))
        }
        other => Err(format!("Unknown unified hook role `{other}`")),
    }
}
