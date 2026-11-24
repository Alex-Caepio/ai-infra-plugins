use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
use tempfile::tempdir;

fn mod_bin() -> &'static str {
    env!("CARGO_BIN_EXE_ai-infra-mod")
}

fn staged_bin_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("bin")
}

fn init_git_repo(dir: &Path) {
    for args in [
        &["init"][..],
        &["config", "user.name", "Tester"][..],
        &["config", "user.email", "tester@example.com"][..],
    ] {
        let out = Command::new("git").args(args).current_dir(dir).output().unwrap();
        assert!(out.status.success(), "{:?}", out);
    }
    std::fs::write(dir.join("README.md"), "# Demo Repo\n").unwrap();
    for args in [&["add", "."][..], &["commit", "-m", "init"][..]] {
        let out = Command::new("git").args(args).current_dir(dir).output().unwrap();
        assert!(out.status.success(), "{:?}", out);
    }
}

fn rpc(stdin: &mut impl Write, reader: &mut impl BufRead, req: Value) -> Value {
    writeln!(stdin, "{req}").unwrap();
    stdin.flush().unwrap();
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    serde_json::from_str(&line).unwrap()
}

#[test]
fn mcp_aggregates_all_80_tools_with_annotations_and_routes_all_6_backends() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path().join("repo");
    let xdg_state = tmp.path().join("xdg-state");
    std::fs::create_dir(&repo).unwrap();
    std::fs::create_dir(&xdg_state).unwrap();
    init_git_repo(&repo);

    // Create a valid minimal skill inside repo so skill_validate & skillcache_gate1 have a real target.
    let skill_dir = repo.join("demo-skill");
    std::fs::create_dir(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: demo-skill\ndescription: Inspect repository layout cleanly.\n---\n\n# Demo Skill\n",
    )
    .unwrap();

    let mut mcp = Command::new(mod_bin())
        .arg("mcp")
        .env("AI_INFRA_BIN_DIR", staged_bin_dir())
        .env("HOME", tmp.path())
        .env("XDG_STATE_HOME", &xdg_state)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = mcp.stdin.take().unwrap();
    let mut stdout = BufReader::new(mcp.stdout.take().unwrap());

    let init = rpc(
        &mut stdin,
        &mut stdout,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": { "protocolVersion": "2025-06-18", "capabilities": {} }
        }),
    );
    assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
    writeln!(
        stdin,
        "{}",
        json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })
    )
    .unwrap();
    stdin.flush().unwrap();

    let list = rpc(
        &mut stdin,
        &mut stdout,
        json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {} }),
    );
    let tools = list["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 80, "expected 79 backend tools + mod_status");
    let names: BTreeSet<String> = tools
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(names.len(), 80, "expected 80 unique MCP tool names");
    for expected in [
        "mod_status",
        "graph_file_impact",
        "graph_memory_links",
        "system_graph_status",
        "orch_permission_mode",
        "omnigraph_status",
        "omnigraph_atoms",
        "omnigraph_publish",
        "omnigraph_branches_diff",
        "omnigraph_merge_keep",
        "omnigraph_sync_canonical",
        "omnigraph_file_context",
        "omnigraph_branch_diff",
        "context_omnigraph_atoms",
        "context_omnigraph_merge_keep",
        "context_memory_links_for_file",
        "skillcache_status",
        "skillcache_capsules_list",
        "skillcache_gate1",
        "skillcache_compile",
    ] {
        assert!(names.contains(expected), "missing expected tool `{expected}`");
    }
    for t in tools {
        let ann = &t["annotations"];
        assert!(
            ann["readOnlyHint"].is_boolean() && ann["destructiveHint"].is_boolean(),
            "missing annotations on {}",
            t["name"]
        );
    }

    // 1. Meta tool `mod_status`
    let st = rpc(
        &mut stdin,
        &mut stdout,
        json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/call",
            "params": { "name": "mod_status", "arguments": { "project": repo } }
        }),
    );
    assert_eq!(st["result"]["isError"], false, "{st}");
    assert_eq!(st["result"]["structuredContent"]["total_tools"], 80);
    assert_eq!(
        st["result"]["structuredContent"]["subsystems"]
            .as_array()
            .unwrap()
            .len(),
        6
    );

    // 2. graph-map (`graph_status`)
    let gr = rpc(
        &mut stdin,
        &mut stdout,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": { "name": "graph_status", "arguments": { "project": repo } }
        }),
    );
    assert_eq!(gr["result"]["isError"], false, "{gr}");

    // 3. system-orchestration (`orch_working_memory_*`, `omnigraph_atoms`)
    let wm_w = rpc(
        &mut stdin,
        &mut stdout,
        json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "orch_working_memory_write",
                "arguments": { "project": repo, "scope_id": "s1", "key": "k1", "text": "hello mod" }
            }
        }),
    );
    assert_eq!(wm_w["result"]["isError"], false, "{wm_w}");
    let wm_r = rpc(
        &mut stdin,
        &mut stdout,
        json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "orch_working_memory_read",
                "arguments": { "project": repo, "scope_id": "s1" }
            }
        }),
    );
    assert_eq!(wm_r["result"]["isError"], false, "{wm_r}");

    let orch_og = rpc(
        &mut stdin,
        &mut stdout,
        json!({
            "jsonrpc": "2.0",
            "id": 7,
            "method": "tools/call",
            "params": {
                "name": "omnigraph_atoms",
                "arguments": { "project": repo }
            }
        }),
    );
    assert_eq!(orch_og["result"]["isError"], false, "{orch_og}");
    assert!(
        orch_og["result"]["structuredContent"]["active_branch"].is_string(),
        "expected orch-engine omnigraph_atoms payload: {orch_og}"
    );

    // 4. context-graph (`memory_status` + disambiguated `context_omnigraph_atoms`)
    let mem = rpc(
        &mut stdin,
        &mut stdout,
        json!({
            "jsonrpc": "2.0",
            "id": 8,
            "method": "tools/call",
            "params": { "name": "memory_status", "arguments": { "project": repo } }
        }),
    );
    assert_eq!(mem["result"]["isError"], false, "{mem}");

    let ctx_og = rpc(
        &mut stdin,
        &mut stdout,
        json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "tools/call",
            "params": { "name": "context_omnigraph_atoms", "arguments": { "project": repo } }
        }),
    );
    assert_eq!(ctx_og["result"]["isError"], false, "{ctx_og}");

    // 5. skill-foundry (`skill_validate`)
    let sk = rpc(
        &mut stdin,
        &mut stdout,
        json!({
            "jsonrpc": "2.0",
            "id": 10,
            "method": "tools/call",
            "params": { "name": "skill_validate", "arguments": { "path": skill_dir } }
        }),
    );
    assert_eq!(sk["result"]["isError"], false, "{sk}");

    // 6. agent-guardrails (`guardrails_check_command`)
    let grd_ok = rpc(
        &mut stdin,
        &mut stdout,
        json!({
            "jsonrpc": "2.0",
            "id": 11,
            "method": "tools/call",
            "params": { "name": "guardrails_check_command", "arguments": { "root": repo, "command": "git status" } }
        }),
    );
    assert_eq!(grd_ok["result"]["isError"], false, "{grd_ok}");

    // 7. skillcache (`skillcache_status` + `skillcache_capsules_list`)
    let sc_state = tmp.path().join("sc-state");
    std::fs::create_dir(&sc_state).unwrap();
    let sc_st = rpc(
        &mut stdin,
        &mut stdout,
        json!({
            "jsonrpc": "2.0",
            "id": 12,
            "method": "tools/call",
            "params": { "name": "skillcache_status", "arguments": { "project": repo, "state_dir": sc_state } }
        }),
    );
    assert_eq!(sc_st["result"]["isError"], false, "{sc_st}");

    let sc_list = rpc(
        &mut stdin,
        &mut stdout,
        json!({
            "jsonrpc": "2.0",
            "id": 13,
            "method": "tools/call",
            "params": { "name": "skillcache_capsules_list", "arguments": { "project": repo, "state_dir": sc_state } }
        }),
    );
    assert_eq!(sc_list["result"]["isError"], false, "{sc_list}");

    drop(stdin);
    assert!(mcp.wait().unwrap().success());
}

#[test]
fn mcp_restrict_root_injects_root_and_blocks_path_escapes_from_plugin_cwd() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path().join("allowed-repo");
    let outside = tmp.path().join("outside-secret");
    std::fs::create_dir(&repo).unwrap();
    std::fs::create_dir(&outside).unwrap();
    init_git_repo(&repo);

    let skill_dir = repo.join("my-skill");
    std::fs::create_dir(&skill_dir).unwrap();
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: my-skill\ndescription: Valid test skill.\n---\n",
    )
    .unwrap();
    let out_skill = outside.join("evil-skill");
    std::fs::create_dir(&out_skill).unwrap();
    std::fs::write(
        out_skill.join("SKILL.md"),
        "---\nname: evil-skill\ndescription: Outside test skill.\n---\n",
    )
    .unwrap();

    // Run with cwd inside plugin root, simulating MCPB / Codex `cwd: "."`.
    let mut mcp = Command::new(mod_bin())
        .arg("mcp")
        .current_dir(PathBuf::from(env!("CARGO_MANIFEST_DIR")))
        .env("AI_INFRA_BIN_DIR", staged_bin_dir())
        .env("AI_INFRA_ROOT", &repo)
        .env("AI_INFRA_RESTRICT_ROOT", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = mcp.stdin.take().unwrap();
    let mut stdout = BufReader::new(mcp.stdout.take().unwrap());

    let _ = rpc(
        &mut stdin,
        &mut stdout,
        json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": { "protocolVersion": "2025-06-18", "capabilities": {} }
        }),
    );
    writeln!(
        stdin,
        "{}",
        json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })
    )
    .unwrap();
    stdin.flush().unwrap();

    // Omitted `project`/`root` auto-injects AI_INFRA_ROOT cleanly!
    let status = rpc(
        &mut stdin,
        &mut stdout,
        json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "mod_status", "arguments": {} } }),
    );
    assert_eq!(status["result"]["isError"], false, "{status}");
    assert_eq!(
        status["result"]["structuredContent"]["restricted_mode"],
        true
    );

    let mem = rpc(
        &mut stdin,
        &mut stdout,
        json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": { "name": "memory_status", "arguments": {} } }),
    );
    assert_eq!(mem["result"]["isError"], false, "{mem}");

    // Relative subpath inside AI_INFRA_ROOT succeeds even when server cwd is plugin root!
    let rel_ok = rpc(
        &mut stdin,
        &mut stdout,
        json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": { "name": "skill_validate", "arguments": { "path": "my-skill" } }
        }),
    );
    assert_eq!(rel_ok["result"]["isError"], false, "{rel_ok}");

    let outside_json = outside.join("outside.json");
    std::fs::write(&outside_json, "{}").unwrap();

    // Branch names (`source`/`target`) on `omnigraph_branch_diff` & `omnigraph_branches_diff` MUST NOT be treated as paths under AI_INFRA_RESTRICT_ROOT!
    let bd = rpc(
        &mut stdin,
        &mut stdout,
        json!({
            "jsonrpc": "2.0",
            "id": 16,
            "method": "tools/call",
            "params": {
                "name": "omnigraph_branch_diff",
                "arguments": { "source": "feature/x", "target": "main" }
            }
        }),
    );
    let bd_txt = bd["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(
        !bd_txt.contains("AI_INFRA_RESTRICT_ROOT")
            && !bd_txt.contains(repo.to_string_lossy().as_ref())
            && bd_txt.contains("Omnigraph branch 'feature/x' not found"),
        "expected verbatim branch name `feature/x` passed to backend without path rewriting: {bd}"
    );

    let obd = rpc(
        &mut stdin,
        &mut stdout,
        json!({
            "jsonrpc": "2.0",
            "id": 20,
            "method": "tools/call",
            "params": {
                "name": "omnigraph_branches_diff",
                "arguments": { "source": "feature/x", "target": "main" }
            }
        }),
    );
    let obd_txt = obd["result"]["content"][0]["text"].as_str().unwrap_or("");
    assert!(
        !obd_txt.contains("AI_INFRA_RESTRICT_ROOT")
            && !obd_txt.contains(repo.to_string_lossy().as_ref()),
        "expected orch omnigraph_branches_diff not to treat branch names as paths: {obd}"
    );

    // Repo-relative `path` / `previous_path` / `files` / `sources` on workspace-scoped tools MUST stay relative (not rewritten to /abs/...) under AI_INFRA_RESTRICT_ROOT=1!
    for (rid, tool, args) in [
        (21, "graph_file_impact", json!({ "path": "README.md" })),
        (22, "graph_memory_links", json!({ "path": "README.md", "previous_path": "OLD.md", "files": [{ "path": "README.md" }] })),
        (23, "omnigraph_file_context", json!({ "path": "README.md" })),
        (24, "context_memory_links_for_file", json!({ "path": "README.md", "previous_path": "OLD.md" })),
        (25, "memory_source_fingerprints", json!({ "sources": ["README.md"] })),
    ] {
        let resp = rpc(
            &mut stdin,
            &mut stdout,
            json!({
                "jsonrpc": "2.0",
                "id": rid,
                "method": "tools/call",
                "params": { "name": tool, "arguments": args }
            }),
        );
        assert_eq!(resp["result"]["isError"], false, "{tool} failed under AI_INFRA_RESTRICT_ROOT=1: {resp}");
    }

    // Path escapes (traversal `..`, absolute sibling, or external project/import/ceiling/sources/merge_from/skillcache/context_import_bundle) MUST fail!
    for (id, tool, args) in [
        (5, "skill_validate", json!({ "path": "../outside-secret/evil-skill" })),
        (6, "skill_validate", json!({ "path": out_skill })),
        (7, "graph_status", json!({ "project": outside })),
        (8, "orch_working_memory_read", json!({ "project": outside, "scope_id": "x" })),
        (9, "context_export_bundle", json!({ "destination": "../escaped.cgb.zip" })),
        (10, "graph_build", json!({ "import": outside_json })),
        (11, "graph_build", json!({ "import": "../outside-secret/outside.json" })),
        (12, "skill_budget_check", json!({ "path": "my-skill", "ceiling": outside_json })),
        (13, "memory_source_fingerprints", json!({ "sources": ["../outside-secret/outside.json"] })),
        (14, "context_handoff_save", json!({ "goal": "g", "summary": "s", "next_steps": ["n"], "files": [outside_json], "merge_from": ["../outside-secret/h.md"] })),
        (15, "skill_cost_audit", json!({ "claude_dir": outside })),
        (17, "context_import_bundle", json!({ "source": "../outside-secret/escaped.cgb.zip" })),
        (18, "skillcache_gate1", json!({ "package_dir": "../outside-secret/evil-skill" })),
        (19, "skillcache_compile", json!({ "candidate_path": "../outside-secret/outside.json" })),
        (26, "graph_file_impact", json!({ "path": "../outside-secret/x.php" })),
        (27, "graph_memory_links", json!({ "files": [{ "path": "../outside-secret/x.php" }] })),
        (28, "system_graph_status", json!({ "config_path": "../outside-secret/system.json" })),
    ] {
        let resp = rpc(
            &mut stdin,
            &mut stdout,
            json!({
                "jsonrpc": "2.0",
                "id": id,
                "method": "tools/call",
                "params": { "name": tool, "arguments": args }
            }),
        );
        assert_eq!(resp["result"]["isError"], true, "{tool}: {resp}");
        assert!(
            resp["result"]["content"][0]["text"]
                .as_str()
                .unwrap()
                .contains("AI_INFRA_RESTRICT_ROOT"),
            "{resp}"
        );
    }

    drop(stdin);
    assert!(mcp.wait().unwrap().success());
}

#[test]
fn mcp_negotiates_roots_list_and_guards_plugin_cwd_without_roots() {
    let tmp = tempdir().unwrap();
    let ancestor = tmp.path().join("marketplace");
    std::fs::create_dir(&ancestor).unwrap();
    init_git_repo(&ancestor);
    let plugin = ancestor.join("plugins").join("ai-infra-mod");
    std::fs::create_dir_all(plugin.join(".codex-plugin")).unwrap();
    std::fs::create_dir_all(plugin.join("bin")).unwrap();
    std::fs::write(plugin.join("bin/ai-infra-mod"), "").unwrap();

    // Case A: Client without roots inside plugin cwd -> must refuse omitted/dot project/root!
    {
        let mut mcp = Command::new(mod_bin())
            .arg("mcp")
            .current_dir(&plugin)
            .env("AI_INFRA_BIN_DIR", staged_bin_dir())
            .env_remove("AI_INFRA_ROOT")
            .env_remove("ROOT_DIR")
            .env_remove("AI_INFRA_RESTRICT_ROOT")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = mcp.stdin.take().unwrap();
        let mut stdout = BufReader::new(mcp.stdout.take().unwrap());

        let _ = rpc(
            &mut stdin,
            &mut stdout,
            json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2025-06-18", "capabilities": {} } }),
        );
        writeln!(
            stdin,
            "{}",
            json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })
        )
        .unwrap();
        stdin.flush().unwrap();

        for (id, tool, args) in [
            (2, "graph_status", json!({})),
            (3, "graph_status", json!({ "project": "." })),
            (4, "memory_status", json!({})),
            (5, "orch_working_memory_read", json!({ "scope_id": "x" })),
            (6, "skillcache_status", json!({})),
            (7, "skillcache_status", json!({ "project": "./." })),
            (8, "skillcache_gate1", json!({})),
            (9, "skillcache_gate1", json!({ "package_dir": "." })),
        ] {
            let resp = rpc(
                &mut stdin,
                &mut stdout,
                json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call", "params": { "name": tool, "arguments": args } }),
            );
            assert_eq!(resp["result"]["isError"], true, "{tool}: {resp}");
        }
        drop(stdin);
        assert!(mcp.wait().unwrap().success());
    }

    // Case B: Client advertises `capabilities.roots`, responds to `roots/list` -> auto-injects negotiated root!
    {
        let target = tmp.path().join("target-repo");
        std::fs::create_dir(&target).unwrap();
        init_git_repo(&target);

        let mut mcp = Command::new(mod_bin())
            .arg("mcp")
            .current_dir(&plugin)
            .env("AI_INFRA_BIN_DIR", staged_bin_dir())
            .env_remove("AI_INFRA_ROOT")
            .env_remove("ROOT_DIR")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = mcp.stdin.take().unwrap();
        let mut stdout = BufReader::new(mcp.stdout.take().unwrap());

        let _ = rpc(
            &mut stdin,
            &mut stdout,
            json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2025-06-18",
                    "capabilities": { "roots": { "listChanged": true } }
                }
            }),
        );
        writeln!(
            stdin,
            "{}",
            json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })
        )
        .unwrap();
        stdin.flush().unwrap();

        // Read server->client `roots/list` request and respond!
        let mut req_line = String::new();
        stdout.read_line(&mut req_line).unwrap();
        let roots_req: Value = serde_json::from_str(&req_line).unwrap();
        assert_eq!(roots_req["method"], "roots/list");
        writeln!(
            stdin,
            "{}",
            json!({
                "jsonrpc": "2.0",
                "id": roots_req["id"],
                "result": {
                    "roots": [{ "uri": format!("file://{}", target.display()), "name": "target-repo" }]
                }
            })
        )
        .unwrap();
        stdin.flush().unwrap();

        let status = rpc(
            &mut stdin,
            &mut stdout,
            json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": { "name": "mod_status", "arguments": {} } }),
        );
        assert_eq!(status["result"]["isError"], false, "{status}");
        assert_eq!(
            status["result"]["structuredContent"]["workspace"]["path"],
            target.to_string_lossy().as_ref()
        );
        drop(stdin);
        assert!(mcp.wait().unwrap().success());
    }
}

#[test]
fn hooks_and_cli_status_and_passthrough_work_end_to_end() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir(&repo).unwrap();
    init_git_repo(&repo);

    // `hook pre-bash` blocks `rm -rf /` with exit 2 and allows `git status` with exit 0.
    let mut blk = Command::new(mod_bin())
        .args(["hook", "claude", "pre-bash"])
        .env("AI_INFRA_BIN_DIR", staged_bin_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    blk.stdin
        .take()
        .unwrap()
        .write_all(
            json!({ "cwd": repo, "tool_name": "Bash", "tool_input": { "command": "rm -rf /" } })
                .to_string()
                .as_bytes(),
        )
        .unwrap();
    let blk_out = blk.wait_with_output().unwrap();
    assert_eq!(blk_out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&blk_out.stderr).contains("BLOCKED"));

    let mut allow = Command::new(mod_bin())
        .args(["hook", "codex", "pre-bash"])
        .env("AI_INFRA_BIN_DIR", staged_bin_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    allow
        .stdin
        .take()
        .unwrap()
        .write_all(
            json!({ "cwd": repo, "tool_name": "Bash", "tool_input": { "command": "git status" } })
                .to_string()
                .as_bytes(),
        )
        .unwrap();
    let allow_out = allow.wait_with_output().unwrap();
    assert_eq!(allow_out.status.code(), Some(0));

    // `hook claude session-start` runs both context-graph hooks and merges cleanly (fail-open).
    let mut ss = Command::new(mod_bin())
        .args(["hook", "claude", "session-start"])
        .env("AI_INFRA_BIN_DIR", staged_bin_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    ss.stdin
        .take()
        .unwrap()
        .write_all(
            json!({ "cwd": repo, "hook_event_name": "SessionStart" })
                .to_string()
                .as_bytes(),
        )
        .unwrap();
    let ss_out = ss.wait_with_output().unwrap();
    assert_eq!(ss_out.status.code(), Some(0));

    // `hook claude pre-subagent`, `post-subagent`, `subagent-stop`, `post-write-edit`, `post-bash`, `user-prompt`, `stop` work fail-open cleanly.
    for role in [
        "pre-subagent",
        "post-subagent",
        "subagent-stop",
        "post-write-edit",
        "post-bash",
        "user-prompt",
        "stop",
    ] {
        let mut h = Command::new(mod_bin())
            .args(["hook", "claude", role])
            .env("AI_INFRA_BIN_DIR", staged_bin_dir())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap();
        let payload = if role == "post-bash" {
            json!({
                "cwd": repo,
                "tool_name": "Bash",
                "tool_input": { "command": "cargo test" },
                "tool_response": { "exit_code": 0 }
            })
        } else {
            json!({ "cwd": repo })
        };
        h.stdin
            .take()
            .unwrap()
            .write_all(payload.to_string().as_bytes())
            .unwrap();
        let out = h.wait_with_output().unwrap();
        assert_eq!(out.status.code(), Some(0), "hook `{role}` failed");
    }

    // `status --project <repo> --json`
    let st = Command::new(mod_bin())
        .args(["status", "--project"])
        .arg(&repo)
        .arg("--json")
        .env("AI_INFRA_BIN_DIR", staged_bin_dir())
        .output()
        .unwrap();
    assert!(st.status.success());
    let st_json: Value = serde_json::from_slice(&st.stdout).unwrap();
    assert_eq!(st_json["backends"].as_array().unwrap().len(), 6);
    assert!(
        st_json["backends"]
            .as_array()
            .unwrap()
            .iter()
            .all(|b| b["available"] == true)
    );

    // Passthrough namespaces (`guard`, `graph`, `orch`, `context`, `skill`, `skillcache`)
    let g_blk = Command::new(mod_bin())
        .args(["guard", "check-command", "rm -rf /"])
        .env("AI_INFRA_BIN_DIR", staged_bin_dir())
        .output()
        .unwrap();
    assert_eq!(g_blk.status.code(), Some(2));
    for ns in ["graph", "orch", "context", "skill", "guard", "skillcache"] {
        let h = Command::new(mod_bin())
            .args([ns, "--help"])
            .env("AI_INFRA_BIN_DIR", staged_bin_dir())
            .output()
            .unwrap();
        assert!(h.status.success(), "{ns} --help failed");
    }
    let sc_state = tmp.path().join("sc-cli-state");
    std::fs::create_dir(&sc_state).unwrap();
    let sc_cli = Command::new(mod_bin())
        .args(["skillcache", "status", "--project"])
        .arg(&repo)
        .arg("--state-dir")
        .arg(&sc_state)
        .env("AI_INFRA_BIN_DIR", staged_bin_dir())
        .output()
        .unwrap();
    assert!(sc_cli.status.success(), "skillcache status passthrough failed");
}

#[test]
fn antigravity_hooks_and_gemini_plugin_cwd_guard() {
    let tmp = tempdir().unwrap();
    let repo = tmp.path().join("repo");
    std::fs::create_dir_all(repo.join(".antigravity/agents")).unwrap();
    std::fs::create_dir_all(repo.join(".gemini/skills/demo")).unwrap();
    init_git_repo(&repo);
    std::fs::write(
        repo.join(".antigravity/agents/php-writer.md"),
        "---\nname: php-writer\nwrites: true\n---\n\nWrite agent\n",
    )
    .unwrap();

    // 1. Fake `.gemini/config/plugins/ai-infra-mod` with `plugin.json` + `mcp_config.json` + `bin/ai-infra-mod` triggers `plugin-cwd` guard!
    let agy_plugin = tmp
        .path()
        .join(".gemini")
        .join("config")
        .join("plugins")
        .join("ai-infra-mod");
    std::fs::create_dir_all(agy_plugin.join("bin")).unwrap();
    std::fs::write(agy_plugin.join("bin/ai-infra-mod"), "").unwrap();
    std::fs::write(agy_plugin.join("plugin.json"), "{}").unwrap();
    std::fs::write(agy_plugin.join("mcp_config.json"), "{}").unwrap();

    let mut mcp = Command::new(mod_bin())
        .arg("mcp")
        .current_dir(&agy_plugin)
        .env("AI_INFRA_BIN_DIR", staged_bin_dir())
        .env("HOME", tmp.path())
        .env("XDG_STATE_HOME", tmp.path().join("xdg"))
        .env_remove("AI_INFRA_ROOT")
        .env_remove("ROOT_DIR")
        .env_remove("AI_INFRA_RESTRICT_ROOT")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = mcp.stdin.take().unwrap();
    let mut stdout = BufReader::new(mcp.stdout.take().unwrap());
    let _ = rpc(
        &mut stdin,
        &mut stdout,
        json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": { "protocolVersion": "2025-06-18", "capabilities": {} } }),
    );
    writeln!(
        stdin,
        "{}",
        json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })
    )
    .unwrap();
    stdin.flush().unwrap();

    for (id, tool, args) in [
        (2, "graph_status", json!({})),
        (3, "omnigraph_status", json!({ "project": "." })),
        (4, "skillcache_status", json!({ "project": "./." })),
    ] {
        let resp = rpc(
            &mut stdin,
            &mut stdout,
            json!({ "jsonrpc": "2.0", "id": id, "method": "tools/call", "params": { "name": tool, "arguments": args } }),
        );
        assert_eq!(resp["result"]["isError"], true, "{tool}: {resp}");
    }
    let ok_call = rpc(
        &mut stdin,
        &mut stdout,
        json!({ "jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": { "name": "omnigraph_status", "arguments": { "project": repo } } }),
    );
    assert_eq!(ok_call["result"]["isError"], false, "{ok_call}");
    drop(stdin);
    assert!(mcp.wait().unwrap().success());

    // 2. Native Antigravity hooks (`antigravity`, `agy`, and auto-detected camelCase stdin) across all roles
    let run_hook = |args: &[&str], body: &Value| {
        let mut h = Command::new(mod_bin())
            .args(args)
            .env("AI_INFRA_BIN_DIR", staged_bin_dir())
            .env("HOME", tmp.path())
            .env("XDG_STATE_HOME", tmp.path().join("xdg"))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        h.stdin
            .take()
            .unwrap()
            .write_all(serde_json::to_string(body).unwrap().as_bytes())
            .unwrap();
        h.wait_with_output().unwrap()
    };

    // session-start
    let ss = run_hook(
        &["hook", "antigravity", "session-start"],
        &json!({ "hookEventName": "SessionStart", "workspacePaths": [&repo], "conversationId": "c1" }),
    );
    assert_eq!(ss.status.code(), Some(0));
    let ss_j: Value = serde_json::from_slice(&ss.stdout).unwrap();
    assert!(ss_j.is_object(), "{ss_j}");

    // pre-invocation (auto-detected camelCase without explicit host)
    let pi = run_hook(
        &["hook", "pre-invocation"],
        &json!({ "hookEventName": "PreInvocation", "workspacePaths": [&repo], "conversationId": "c1" }),
    );
    assert_eq!(pi.status.code(), Some(0));
    let pi_j: Value = serde_json::from_slice(&pi.stdout).unwrap();
    assert!(pi_j.is_object(), "{pi_j}");

    // pre-bash deny & allow (exit 0 JSON)
    let pb_deny = run_hook(
        &["hook", "agy", "pre-bash"],
        &json!({
            "hookEventName": "PreToolUse",
            "workspacePaths": [&repo],
            "toolCall": { "name": "run_command", "args": { "CommandLine": "rm -rf /" } }
        }),
    );
    assert_eq!(pb_deny.status.code(), Some(0));
    assert_eq!(
        serde_json::from_slice::<Value>(&pb_deny.stdout).unwrap()["decision"],
        "deny"
    );

    let pb_allow = run_hook(
        &["hook", "antigravity", "pre-bash"],
        &json!({
            "hookEventName": "PreToolUse",
            "workspacePaths": [&repo],
            "toolCall": { "name": "run_command", "args": { "CommandLine": "git status" } }
        }),
    );
    assert_eq!(pb_allow.status.code(), Some(0));
    assert_eq!(
        serde_json::from_slice::<Value>(&pb_allow.stdout).unwrap()["decision"],
        "allow"
    );

    // pre-write-edit allow (SKILL.md)
    let pw_ok = run_hook(
        &["hook", "antigravity", "pre-write-edit"],
        &json!({
            "hookEventName": "PreToolUse",
            "workspacePaths": [&repo],
            "toolCall": {
                "name": "write_to_file",
                "args": { "TargetFile": repo.join(".gemini/skills/demo/SKILL.md") }
            }
        }),
    );
    assert_eq!(pw_ok.status.code(), Some(0));
    assert_eq!(
        serde_json::from_slice::<Value>(&pw_ok.stdout).unwrap()["decision"],
        "allow"
    );

    // post-write-edit -> `{}` exit 0
    let pe = run_hook(
        &["hook", "antigravity", "post-write-edit"],
        &json!({
            "hookEventName": "PostToolUse",
            "workspacePaths": [&repo],
            "conversationId": "c1",
            "toolCall": {
                "name": "write_to_file",
                "args": { "TargetFile": repo.join("README.md") }
            }
        }),
    );
    assert_eq!(pe.status.code(), Some(0));
    assert_eq!(serde_json::from_slice::<Value>(&pe.stdout).unwrap(), json!({}));

    // pre-subagent allow -> concurrent deny -> post-subagent `{}` releases both -> allow!
    let sub_pre = json!({
        "hookEventName": "PreToolUse",
        "workspacePaths": [&repo],
        "conversationId": "c1",
        "toolCall": {
            "name": "invoke_subagent",
            "args": { "TypeName": "php-writer", "Prompt": "Edit", "Workspace": "inherit" }
        }
    });
    let sg1 = run_hook(&["hook", "antigravity", "pre-subagent"], &sub_pre);
    assert_eq!(sg1.status.code(), Some(0));
    assert_eq!(
        serde_json::from_slice::<Value>(&sg1.stdout).unwrap()["decision"],
        "allow"
    );

    let sg2 = run_hook(&["hook", "pre-subagent"], &sub_pre);
    assert_eq!(sg2.status.code(), Some(0));
    assert_eq!(
        serde_json::from_slice::<Value>(&sg2.stdout).unwrap()["decision"],
        "deny"
    );

    let sub_post = json!({
        "hookEventName": "PostToolUse",
        "workspacePaths": [&repo],
        "conversationId": "c1",
        "toolCall": {
            "name": "invoke_subagent",
            "args": { "Role": "php-writer", "Prompt": "Done", "Workspace": "inherit" }
        }
    });
    let sp = run_hook(&["hook", "antigravity", "post-subagent"], &sub_post);
    assert_eq!(sp.status.code(), Some(0));
    assert_eq!(serde_json::from_slice::<Value>(&sp.stdout).unwrap(), json!({}));

    let sg3 = run_hook(&["hook", "antigravity", "pre-subagent"], &sub_pre);
    assert_eq!(sg3.status.code(), Some(0));
    assert_eq!(
        serde_json::from_slice::<Value>(&sg3.stdout).unwrap()["decision"],
        "allow"
    );
    let _ = run_hook(&["hook", "antigravity", "post-subagent"], &sub_post);

    // stop -> `{}` exit 0
    let stp = run_hook(
        &["hook", "antigravity", "stop"],
        &json!({ "hookEventName": "Stop", "workspacePaths": [&repo], "conversationId": "c1" }),
    );
    assert_eq!(stp.status.code(), Some(0));
    assert_eq!(serde_json::from_slice::<Value>(&stp.stdout).unwrap(), json!({}));
}
