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
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"guardrails_inspect_policy","arguments":{}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"guardrails_init_policy","arguments":{"preset":"generic","root":target_repo.display().to_string()}}}),
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

    assert_eq!(lines.len(), 4);
    assert_eq!(lines[1]["result"]["isError"], true);
    assert!(
        lines[1]["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Missing 'root' argument")
    );
    assert!(!plugin_dir.join(".agent-guardrails").exists());
    assert!(
        lines[2]["result"]["structuredContent"]["warning"]
            .as_str()
            .unwrap()
            .contains("pass 'root'")
    );
    assert_eq!(lines[3]["result"]["isError"], false);
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
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"guardrails_inspect_policy","arguments":{}}}),
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"guardrails_init_policy","arguments":{"preset":"rust","root":target_repo.display().to_string()}}}),
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
    assert!(
        lines[1]["result"]["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Missing 'root' argument")
    );
    assert!(!fake_home.join(".agent-guardrails").exists());
    assert!(!plugin_dir.join(".agent-guardrails").exists());
    assert!(
        lines[2]["result"]["structuredContent"]["warning"]
            .as_str()
            .unwrap()
            .contains("pass 'root'")
    );
    assert_eq!(lines[3]["result"]["isError"], false);
    assert!(target_repo.join(".agent-guardrails/policy.json").is_file());
}
