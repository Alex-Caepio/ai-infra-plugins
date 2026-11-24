//! End-to-end tests for CLI subcommands, custom policy rules, hook aliases, and MCP stdio server.
use agent_guardrails::{guards, mcp, policy, presets::Preset};
use serde_json::{Value, json};
use std::{fs, io::Cursor, io::Write, process::{Command, Stdio}};

#[test]
fn policy_init_custom_rules_and_secret_globs_enforced() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    let path = policy::init_policy(root, Preset::Node, false).unwrap();
    assert!(path.ends_with(".agent-guardrails/policy.json"));
    // Duplicate init without force fails with conflict
    assert!(policy::init_policy(root, Preset::Rust, false).is_err());

    // Extend generated policy.json with a custom deny rule + additional secret glob
    let custom = json!({
        "schema_version": 1,
        "preset": "node",
        "approved_tools": ["terraform"],
        "project_command_prefixes": [
            {
                "tool": "terraform",
                "argv": ["terraform"],
                "options_with_values": ["-chdir"],
                "short_options_with_values": ""
            }
        ],
        "deny_rules": [
            {
                "id": "deny_terraform_destroy",
                "tool": "terraform",
                "command": ["destroy"],
                "matching": "exact",
                "condition": {"kind": "no_option", "patterns": ["--dry-run"]},
                "reason": "destructive_command"
            }
        ],
        "loop_thresholds": {"cmd_warn": 3, "cmd_block": 5, "edit_warn": 4, "edit_block": 6},
        "naming_rules": {
            "enforce_accelerator_conventions": true,
            "blocked_globs": ["*.secret.md"]
        },
        "subagent_gate": {"enabled": true, "block_builtin_codex": false, "allowed_agents": [], "writer_agents": []},
        "outside_repo_rm": "block",
        "additional_secret_globs": ["*.pem", "vault-token.txt"]
    });
    fs::write(&path, serde_json::to_string_pretty(&custom).unwrap()).unwrap();

    // Custom rule triggers
    let tf_block = guards::check_command(root, "terraform -chdir=infra destroy -auto-approve", None).unwrap();
    assert_eq!(tf_block["blocked"], json!(true));

    // Additional secret glob triggers
    let pem_block = guards::check_command(root, "cat deploy-key.pem", None).unwrap();
    assert_eq!(pem_block["blocked"], json!(true));

    // Node preset rule also remains active
    let npm_block = guards::check_command(root, "npm publish", None).unwrap();
    assert_eq!(npm_block["blocked"], json!(true));

    // Safe command passes
    let ok = guards::check_command(root, "git status", None).unwrap();
    assert_eq!(ok["allowed"], json!(true));

    // Naming rule glob blocks *.secret.md
    let naming = guards::hook_execute(
        Some(root),
        Some("claude"),
        "naming-check",
        &serde_json::to_vec(&json!({
            "tool_name": "Write",
            "tool_input": {"file_path": "docs/prod.secret.md"}
        }))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(naming.code, 2);
}

#[test]
fn mcp_stdio_protocol_covers_init_list_and_all_four_tools() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    let requests = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"guardrails_init_policy","arguments":{"preset":"python","root":root.display().to_string()}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"guardrails_inspect_policy","arguments":{"root":root.display().to_string()}}}),
        json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"guardrails_check_command","arguments":{"command":"alembic downgrade base","root":root.display().to_string()}}}),
        json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"guardrails_check_command","arguments":{"command":"git status","root":root.display().to_string()}}}),
        json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"guardrails_loop_status","arguments":{"root":root.display().to_string(),"session_id":"mcp1","reset":true}}}),
    ];

    let mut stdin_buf = String::new();
    for req in requests {
        stdin_buf.push_str(&serde_json::to_string(&req).unwrap());
        stdin_buf.push('\n');
    }

    let mut out = Vec::new();
    mcp::serve(root, Cursor::new(stdin_buf.as_bytes()), &mut out).unwrap();
    let lines: Vec<Value> = String::from_utf8(out)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();

    assert_eq!(lines.len(), 7); // notifications/initialized produces no response
    assert_eq!(lines[0]["result"]["protocolVersion"], "2025-06-18");

    let tool_names: Vec<&str> = lines[1]["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        tool_names,
        vec![
            "guardrails_check_command",
            "guardrails_inspect_policy",
            "guardrails_init_policy",
            "guardrails_loop_status"
        ]
    );

    assert_eq!(lines[2]["result"]["structuredContent"]["initialized"], true);
    assert_eq!(lines[3]["result"]["structuredContent"]["preset"], "python");
    assert_eq!(lines[4]["result"]["structuredContent"]["blocked"], true);
    assert_eq!(lines[5]["result"]["structuredContent"]["allowed"], true);
    assert_eq!(
        lines[6]["result"]["structuredContent"]["session_id"],
        "mcp1"
    );
}

#[test]
fn cli_binary_handles_help_init_check_status_and_stdin_hooks() {
    let bin = env!("CARGO_BIN_EXE_guardrails");
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    let help = Command::new(bin).arg("--help").output().unwrap();
    assert!(help.status.success());
    let help_text = String::from_utf8(help.stdout).unwrap();
    assert!(help_text.contains("check-command"));
    assert!(help_text.contains("policy-init"));
    assert!(help_text.contains("hook"));
    assert!(help_text.contains("mcp"));

    let init = Command::new(bin)
        .args(["policy-init", "--root"])
        .arg(root)
        .args(["--preset", "rust", "--json"])
        .output()
        .unwrap();
    assert!(init.status.success());
    let init_json: Value = serde_json::from_slice(&init.stdout).unwrap();
    assert_eq!(init_json["preset"], "rust");

    let allow = Command::new(bin)
        .args(["check-command", "--root"])
        .arg(root)
        .args(["cargo test --workspace"])
        .output()
        .unwrap();
    assert_eq!(allow.status.code(), Some(0));

    let deny = Command::new(bin)
        .args(["check-command", "--root"])
        .arg(root)
        .args(["cargo publish"])
        .output()
        .unwrap();
    assert_eq!(deny.status.code(), Some(2));

    let status = Command::new(bin)
        .args(["status", "--root"])
        .arg(root)
        .arg("--json")
        .env("TMPDIR", root)
        .output()
        .unwrap();
    assert!(status.status.success());
    let st: Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(st["policy"]["preset"], "rust");

    // Stdin hook tests matching verifier smoke checks
    for (cmd, expected_code) in [
        ("rm -rf /", 2),
        ("dropdb prod", 2),
        ("git status", 0),
    ] {
        let mut child = Command::new(bin)
            .args(["hook", "bash-guard", "--root"])
            .arg(root)
            .env("TMPDIR", root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let payload = json!({"tool_name":"Bash","tool_input":{"command":cmd}});
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(payload.to_string().as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert_eq!(out.status.code(), Some(expected_code), "cmd={cmd}");
    }
}

#[test]
fn mcp_refuses_init_into_plugin_cache_dir_without_explicit_root() {
    let dir = tempfile::tempdir().unwrap();
    let plugin_dir = dir.path().join("plugin-cache");
    let target_repo = dir.path().join("target-repo");
    fs::create_dir_all(plugin_dir.join(".codex-plugin")).unwrap();
    fs::create_dir_all(plugin_dir.join("bin")).unwrap();
    fs::write(plugin_dir.join(".codex-plugin/plugin.json"), "{}").unwrap();
    fs::write(plugin_dir.join("bin/guardrails"), b"elf").unwrap();
    fs::create_dir_all(&target_repo).unwrap();

    let requests = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"guardrails_init_policy","arguments":{"preset":"generic"}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"guardrails_init_policy","arguments":{"preset":"generic","root":"."}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"guardrails_init_policy","arguments":{"preset":"generic","root":"./"}}}),
        json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"guardrails_inspect_policy","arguments":{}}}),
        json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"guardrails_inspect_policy","arguments":{"root":"."}}}),
        json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"guardrails_check_command","arguments":{"command":"git status"}}}),
        json!({"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"guardrails_check_command","arguments":{"command":"git status","root":"."}}}),
        json!({"jsonrpc":"2.0","id":9,"method":"tools/call","params":{"name":"guardrails_loop_status","arguments":{}}}),
        json!({"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"guardrails_loop_status","arguments":{"root":"."}}}),
        json!({"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"guardrails_init_policy","arguments":{"preset":"generic","root":target_repo.display().to_string()}}}),
    ];

    let mut stdin_buf = String::new();
    for req in requests {
        stdin_buf.push_str(&serde_json::to_string(&req).unwrap());
        stdin_buf.push('\n');
    }

    let mut out = Vec::new();
    mcp::serve(&plugin_dir, Cursor::new(stdin_buf.as_bytes()), &mut out).unwrap();
    let lines: Vec<Value> = String::from_utf8(out)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();

    assert_eq!(lines.len(), 11);
    for (idx, line) in lines[1..=9].iter().enumerate() {
        assert_eq!(
            line["result"]["isError"], true,
            "expected refusal for line {}: {line}",
            idx + 1
        );
    }
    assert!(!plugin_dir.join(".agent-guardrails").exists());
    assert_eq!(lines[10]["result"]["isError"], false);
    assert!(target_repo.join(".agent-guardrails/policy.json").is_file());
}

#[test]
fn mcp_binary_subcommand_blocks_plugin_cwd_bypass_even_under_ancestor_git() {
    let bin = env!("CARGO_BIN_EXE_guardrails");
    let dir = tempfile::tempdir().unwrap();
    let fake_home = dir.path().join("fake-home");
    let plugin_dir = fake_home.join("plugins/agent-guardrails");
    let target_repo = dir.path().join("target-repo");

    // Create ancestor .git (reproducing ~/.git above plugin cache or workspace)
    fs::create_dir_all(fake_home.join(".git")).unwrap();
    fs::create_dir_all(plugin_dir.join(".codex-plugin")).unwrap();
    fs::create_dir_all(plugin_dir.join(".claude-plugin")).unwrap();
    fs::create_dir_all(plugin_dir.join("bin")).unwrap();
    fs::write(plugin_dir.join(".codex-plugin/plugin.json"), "{}").unwrap();
    fs::write(plugin_dir.join(".claude-plugin/plugin.json"), "{}").unwrap();
    fs::write(plugin_dir.join("bin/guardrails"), b"elf").unwrap();
    fs::create_dir_all(&target_repo).unwrap();

    let mut child = Command::new(bin)
        .arg("mcp")
        .current_dir(&plugin_dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let requests = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"guardrails_init_policy","arguments":{"preset":"rust"}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"guardrails_init_policy","arguments":{"preset":"rust","root":"."}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"guardrails_init_policy","arguments":{"preset":"rust","root":"./"}}}),
        json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"guardrails_inspect_policy","arguments":{"root":"."}}}),
        json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"guardrails_check_command","arguments":{"command":"git status","root":"."}}}),
        json!({"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"guardrails_loop_status","arguments":{"root":"."}}}),
        json!({"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"guardrails_init_policy","arguments":{"preset":"rust","root":target_repo.display().to_string()}}}),
    ];
    {
        let stdin = child.stdin.as_mut().unwrap();
        for req in requests {
            writeln!(stdin, "{}", serde_json::to_string(&req).unwrap()).unwrap();
        }
    }
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let lines: Vec<Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();

    assert_eq!(lines.len(), 8);
    for (idx, line) in lines[1..=6].iter().enumerate() {
        assert_eq!(
            line["result"]["isError"], true,
            "expected refusal for line {}: {line}",
            idx + 1
        );
    }
    assert!(!fake_home.join(".agent-guardrails").exists());
    assert!(!plugin_dir.join(".agent-guardrails").exists());
    assert_eq!(lines[7]["result"]["isError"], false);
    assert!(target_repo.join(".agent-guardrails/policy.json").is_file());
}

#[test]
fn cli_reports_read_only_alignment_and_signature_loop_lifecycle() {
    let bin = env!("CARGO_BIN_EXE_guardrails");
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join(".git")).unwrap();

    // Read-only classification via `check-command --json`
    let ro = Command::new(bin)
        .args(["check-command", "--root"])
        .arg(root)
        .args(["--json", "git status | head -n 10"])
        .output()
        .unwrap();
    assert!(ro.status.success());
    let ro_json: Value = serde_json::from_slice(&ro.stdout).unwrap();
    assert_eq!(ro_json["allowed"], true);
    assert_eq!(ro_json["read_only"], true);

    let rw = Command::new(bin)
        .args(["check-command", "--root"])
        .arg(root)
        .args(["--json", "sort -o out.txt names.txt"])
        .output()
        .unwrap();
    assert!(rw.status.success());
    let rw_json: Value = serde_json::from_slice(&rw.stdout).unwrap();
    assert_eq!(rw_json["allowed"], true);
    assert_eq!(rw_json["read_only"], false);

    // Track failing bash check signature via PostToolUse edit-loop, inspect in status, then reset on pass.
    let session = "cliloopsig1";
    let run_hook = |role: &str, payload: &Value| {
        let mut child = Command::new(bin)
            .args(["hook", "claude", role, "--root"])
            .arg(root)
            .env("TMPDIR", root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(payload.to_string().as_bytes())
            .unwrap();
        child.wait_with_output().unwrap()
    };

    let fail_check = json!({
        "session_id": session,
        "tool_name": "Bash",
        "command": "cargo test foo | head -n 20",
        "exit_code": 1,
        "stderr": "error[E0308]: mismatched types"
    });
    assert_eq!(run_hook("edit-loop", &fail_check).status.code(), Some(0));

    let st1 = Command::new(bin)
        .args(["status", "--root"])
        .arg(root)
        .args(["--session", session, "--json"])
        .env("TMPDIR", root)
        .output()
        .unwrap();
    assert!(st1.status.success());
    let st1_json: Value = serde_json::from_slice(&st1.stdout).unwrap();
    let active_sig = st1_json["runtime"]["active_signature"].as_str().unwrap_or("");
    assert!(active_sig.contains("cargo test foo"), "{st1_json}");
    assert!(active_sig.contains("mismatched types"), "{st1_json}");

    let pass_check = json!({
        "session_id": session,
        "tool_name": "Bash",
        "command": "cargo test foo",
        "exit_code": 0
    });
    assert_eq!(run_hook("edit-loop", &pass_check).status.code(), Some(0));

    let st2 = Command::new(bin)
        .args(["status", "--root"])
        .arg(root)
        .args(["--session", session, "--json"])
        .env("TMPDIR", root)
        .output()
        .unwrap();
    assert!(st2.status.success());
    let st2_json: Value = serde_json::from_slice(&st2.stdout).unwrap();
    assert!(st2_json["runtime"]["active_signature"].is_null(), "{st2_json}");
}

#[test]
fn antigravity_native_hooks_contract_and_loop_lifecycle() {
    let bin = env!("CARGO_BIN_EXE_guardrails");
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().join("repo");
    let locks = dir.path().join("locks");
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::create_dir_all(root.join(".claude/agents")).unwrap();
    fs::create_dir_all(&locks).unwrap();
    fs::write(
        root.join(".claude/agents/coder.md"),
        "---\nname: coder\ndescription: writer\nwrites: true\n---\nCoder\n",
    )
    .unwrap();

    let run_agy = |args: &[&str], payload: &Value| {
        let mut child = Command::new(bin)
            .args(args)
            .env("TMPDIR", dir.path())
            .env("SUBAGENT_WRITE_LOCK_DIR", &locks)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        child
            .stdin
            .as_mut()
            .unwrap()
            .write_all(payload.to_string().as_bytes())
            .unwrap();
        let out = child.wait_with_output().unwrap();
        assert_eq!(
            out.status.code(),
            Some(0),
            "Antigravity hook must always exit 0: args={args:?} stderr={}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            out.stderr.is_empty(),
            "Antigravity stderr must stay empty: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        serde_json::from_slice::<Value>(&out.stdout).expect("stdout must be JSON")
    };

    // 1. bash-guard via explicit `agy` host + auto-detected `hookEventName`/`workspacePaths`
    let deny_cmd = json!({
        "conversationId": "agyconv1",
        "workspacePaths": [root.display().to_string()],
        "hookEventName": "PreToolUse",
        "toolCall": {"id": "t1", "name": "run_command", "args": {"CommandLine": "rm -rf /"}}
    });
    let deny_out = run_agy(&["hook", "agy", "bash-guard"], &deny_cmd);
    assert_eq!(deny_out["decision"], "deny");
    assert!(deny_out["reason"].as_str().unwrap().contains("BLOCKED:"));

    let allow_cmd = json!({
        "conversationId": "agyconv1",
        "workspacePaths": [root.display().to_string()],
        "hookEventName": "PreToolUse",
        "toolCall": {"id": "t2", "name": "run_command", "args": {"CommandLine": "git status"}}
    });
    // Omit `--host` to verify auto-detection from camelCase Antigravity fields
    let allow_out = run_agy(&["hook", "bash-guard"], &allow_cmd);
    assert_eq!(allow_out, json!({"decision": "allow"}));

    // Loop warning adds `reason` alongside `decision: "allow"`
    let repeat_cmd = json!({
        "conversationId": "agyconvloop",
        "cwd": root.display().to_string(),
        "hookEventName": "PreToolUse",
        "toolCall": {"id": "t3", "name": "run_command", "args": {"CommandLine": "cargo test agy_warn"}}
    });
    for _ in 0..5 {
        assert_eq!(
            run_agy(&["hook", "antigravity", "bash-guard"], &repeat_cmd),
            json!({"decision": "allow"})
        );
    }
    let warn_out = run_agy(&["hook", "antigravity", "bash-guard"], &repeat_cmd);
    assert_eq!(warn_out["decision"], "allow");
    assert!(warn_out["reason"].as_str().unwrap().contains("WARNING:"));

    // 2. naming-check: `write_to_file` TargetFile, `multi_replace_file_content` TargetFiles, `find_by_name`
    let bad_naming = json!({
        "conversationId": "agyconv1",
        "cwd": root.display().to_string(),
        "hookEventName": "PreToolUse",
        "toolCall": {"name": "write_to_file", "args": {"TargetFile": "memory-bank/chunks/bad.md"}}
    });
    assert_eq!(
        run_agy(&["hook", "naming-check"], &bad_naming)["decision"],
        "deny"
    );

    let bad_multi = json!({
        "conversationId": "agyconv1",
        "cwd": root.display().to_string(),
        "hookEventName": "PreToolUse",
        "toolCall": {
            "name": "multi_replace_file_content",
            "args": {"TargetFiles": ["src/main.rs", "memory-bank/chunks/also-bad.md"]}
        }
    });
    assert_eq!(
        run_agy(&["hook", "naming-check"], &bad_multi)["decision"],
        "deny"
    );

    let ok_naming = json!({
        "conversationId": "agyconv1",
        "cwd": root.display().to_string(),
        "hookEventName": "PreToolUse",
        "toolCall": {
            "name": "write_to_file",
            "args": {"TargetFile": "memory-bank/chunks/MEM-0001-safe-auth.md"}
        }
    });
    assert_eq!(
        run_agy(&["hook", "naming-check"], &ok_naming),
        json!({"decision": "allow"})
    );

    let find_noop = json!({
        "conversationId": "agyconv1",
        "cwd": root.display().to_string(),
        "hookEventName": "PreToolUse",
        "toolCall": {"name": "find_by_name", "args": {"SearchDirectory": "src", "Pattern": "*.rs"}}
    });
    assert_eq!(
        run_agy(&["hook", "naming-check"], &find_noop),
        json!({"decision": "allow"})
    );

    // 3. edit-loop PostToolUse emits `{}` and tracks failing signature + edit counters -> pass resets
    let agy_fail = json!({
        "conversationId": "agyedit1",
        "cwd": root.display().to_string(),
        "hookEventName": "PostToolUse",
        "error": "exit status 1",
        "toolCall": {"name": "run_command", "args": {"CommandLine": "cargo test foo | head -n 5"}}
    });
    assert_eq!(run_agy(&["hook", "edit-loop"], &agy_fail), json!({}));

    let agy_edit = json!({
        "conversationId": "agyedit1",
        "cwd": root.display().to_string(),
        "hookEventName": "PostToolUse",
        "toolCall": {"name": "replace_file_content", "args": {"TargetFile": "src/lib.rs"}}
    });
    assert_eq!(run_agy(&["hook", "edit-loop"], &agy_edit), json!({}));

    let st_mid: Value = serde_json::from_slice(
        &Command::new(bin)
            .args(["status", "--root"])
            .arg(&root)
            .args(["--session", "agyedit1", "--json"])
            .env("TMPDIR", dir.path())
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    assert!(
        st_mid["runtime"]["active_signature"]
            .as_str()
            .is_some_and(|s| s.contains("cargo test foo")),
        "{st_mid}"
    );
    assert_eq!(st_mid["runtime"]["edit_counters"], 1);

    let agy_pass = json!({
        "conversationId": "agyedit1",
        "cwd": root.display().to_string(),
        "hookEventName": "PostToolUse",
        "toolCall": {"name": "run_command", "args": {"CommandLine": "cargo test foo"}}
    });
    assert_eq!(run_agy(&["hook", "edit-loop"], &agy_pass), json!({}));

    let st_reset: Value = serde_json::from_slice(
        &Command::new(bin)
            .args(["status", "--root"])
            .arg(&root)
            .args(["--session", "agyedit1", "--json"])
            .env("TMPDIR", dir.path())
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    assert!(st_reset["runtime"]["active_signature"].is_null());
    assert_eq!(st_reset["runtime"]["edit_counters"], 0);

    // 4. subagent-gate & subagent-release (`invoke_subagent`)
    let spawn_coder = json!({
        "conversationId": "agyconv1",
        "cwd": root.display().to_string(),
        "hookEventName": "PreToolUse",
        "toolCall": {"name": "invoke_subagent", "args": {"Role": "coder", "Prompt": "edit file"}}
    });
    assert_eq!(
        run_agy(&["hook", "subagent-gate"], &spawn_coder),
        json!({"decision": "allow"})
    );
    let overlap = run_agy(&["hook", "subagent-gate"], &spawn_coder);
    assert_eq!(overlap["decision"], "deny");
    assert!(overlap["reason"].as_str().unwrap().contains("already running"));

    let unknown_agent = json!({
        "conversationId": "agyconv1",
        "cwd": root.display().to_string(),
        "hookEventName": "PreToolUse",
        "toolCall": {"name": "invoke_subagent", "args": {"TypeName": "rogue", "Prompt": "x"}}
    });
    assert_eq!(
        run_agy(&["hook", "subagent-gate"], &unknown_agent)["decision"],
        "deny"
    );

    let release_coder = json!({
        "conversationId": "agyconv1",
        "cwd": root.display().to_string(),
        "hookEventName": "PostToolUse",
        "toolCall": {"name": "invoke_subagent", "args": {"Role": "coder"}}
    });
    assert_eq!(run_agy(&["hook", "subagent-release"], &release_coder), json!({}));
    assert_eq!(
        run_agy(&["hook", "subagent-gate"], &spawn_coder),
        json!({"decision": "allow"})
    );
}

#[test]
fn mcp_blocks_antigravity_extension_cache_dirs_without_explicit_root() {
    let bin = env!("CARGO_BIN_EXE_guardrails");
    let dir = tempfile::tempdir().unwrap();
    let agy_plugin = dir.path().join(".gemini/config/plugins/agent-guardrails");
    let target_repo = dir.path().join("target-repo");
    fs::create_dir_all(agy_plugin.join("bin")).unwrap();
    fs::write(agy_plugin.join("plugin.json"), "{}").unwrap();
    fs::write(agy_plugin.join("mcp_config.json"), "{}").unwrap();
    fs::write(agy_plugin.join("bin/guardrails"), b"elf").unwrap();
    fs::create_dir_all(&target_repo).unwrap();

    let mut child = Command::new(bin)
        .arg("mcp")
        .current_dir(&agy_plugin)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let requests = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"agy-test","version":"1.0"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"guardrails_init_policy","arguments":{"preset":"generic"}}}),
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"guardrails_check_command","arguments":{"command":"git status","root":"."}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"guardrails_init_policy","arguments":{"preset":"generic","root":target_repo.display().to_string()}}}),
    ];
    {
        let stdin = child.stdin.as_mut().unwrap();
        for req in requests {
            writeln!(stdin, "{}", serde_json::to_string(&req).unwrap()).unwrap();
        }
    }
    let out = child.wait_with_output().unwrap();
    assert!(out.status.success(), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    let lines: Vec<Value> = String::from_utf8(out.stdout)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();

    assert_eq!(lines.len(), 4);
    assert_eq!(lines[1]["result"]["isError"], true);
    assert_eq!(lines[2]["result"]["isError"], true);
    assert!(!agy_plugin.join(".agent-guardrails").exists());
    assert_eq!(lines[3]["result"]["isError"], false);
    assert!(target_repo.join(".agent-guardrails/policy.json").is_file());
}
