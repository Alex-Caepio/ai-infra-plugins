//! Hook payload prefiltering, naming checks, loop counters, and subagent write-lock gate tests.
use agent_guardrails::guards::{self, GuardOutput};
use serde_json::{Value, json};
use std::{fs, path::Path, process::Command};

const CHILD_ROOT: &str = "GUARDRAILS_PREFILTER_CHILD_ROOT";
const HOSTS: [&str; 3] = ["claude", "codex", "cursor"];
const ROLES: [&str; 3] = ["bash-guard", "naming-check", "edit-loop"];

fn run(root: &Path, host: &str, role: &str, payload: &Value) -> GuardOutput {
    guards::hook_execute(
        Some(root),
        Some(host),
        role,
        &serde_json::to_vec(payload).unwrap(),
    )
    .unwrap()
}

fn silent(output: GuardOutput) {
    assert_eq!(output.code, 0, "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
}

fn exercise(root: &Path) {
    let temporary = tempfile::tempdir_in(root).unwrap();
    let project = temporary.path().join("project");
    fs::create_dir(&project).unwrap();
    fs::create_dir_all(project.join(".claude/skills/architecture-audit")).unwrap();

    let missing = temporary.path().join("missing-project");
    let invalid_policy = temporary.path().join("invalid-policy");
    fs::create_dir_all(invalid_policy.join(".agent-guardrails")).unwrap();
    fs::write(invalid_policy.join(".agent-guardrails/policy.json"), "{").unwrap();

    let read = json!({"tool_name":"Read","tool_input":{"file_path":"memory-bank/chunks/bad.md"}});
    for host in HOSTS {
        for bound in [&project, &missing, &invalid_policy] {
            for role in ROLES {
                silent(run(bound, host, role, &read));
            }
        }
        for raw in [
            b"".as_slice(),
            b"not json",
            b"null",
            b"false",
            b"42",
            b"\"text\"",
            b"[{\"command\":\"rm -rf /\"}]",
        ] {
            for role in ROLES {
                silent(guards::hook_execute(Some(&project), Some(host), role, raw).unwrap());
            }
        }

        // Hidden recursively nested command triggers refusal across valid & malformed policy roots
        for payload in [
            json!({"tool_name":"Read","command":"","input":[{"cmd":"git push --force origin main"}]}),
            json!({"tool_name":null,"input":{"command":false,"child":{"command":"rm -rf /"}}}),
        ] {
            for bound in [&project, &missing, &invalid_policy] {
                let output = run(bound, host, "bash-guard", &payload);
                assert_eq!(output.code, 2, "{host} {}", bound.display());
                let msg = String::from_utf8(output.stderr).unwrap();
                assert!(msg.contains("BLOCKED:"), "{msg}");
            }
        }

        // File naming checks
        for payload in [
            json!({"tool_input":{"file_path":"memory-bank/chunks/bad.md"}}),
            json!({"tool_name":null,"tool_input":{"file_path":"tasks/TASK-1/architecture-audit-notes.md"}}),
            json!({"tool_name":"Read","nested":[{"tool_name":"provider:Multi_Edit"}],"tool_input":{"file_path":"tasks/TASK-001/unprefixed.md"}}),
            json!({"tool_name":"tools.apply_patch","input":"*** Begin Patch\n*** Add File: memory-bank/chunks/bad.md\n+body\n*** End Patch\n"}),
        ] {
            let output = run(&project, host, "naming-check", &payload);
            assert_eq!(output.code, 2, "{host} {payload}: {output:?}");
            assert!(
                String::from_utf8(output.stderr)
                    .unwrap()
                    .contains("BLOCKED:")
            );
        }

        // Valid accelerator markdown paths pass
        silent(run(
            &project,
            host,
            "naming-check",
            &json!({"tool_name":"Write","tool_input":{"file_path":"tasks/TASK-001/architecture-audit-plan.md"}}),
        ));
        silent(run(
            &project,
            host,
            "naming-check",
            &json!({"tool_name":"Write","tool_input":{"file_path":"memory-bank/chunks/MEM-0001-safe-auth.md"}}),
        ));

        // Edit loop warnings (at 7) and blocks (at 10)
        for (index, mut payload) in [
            json!({"tool_input":{"file_path":"src/App.rs"}}),
            json!({"tool_name":"tools.apply_patch","input":"*** Begin Patch\n*** Update File: src/Lib.rs\n*** End Patch\n"}),
        ]
        .into_iter()
        .enumerate()
        {
            payload["session_id"] = json!(format!("prefilter-{host}-{index}"));
            for _ in 0..6 {
                silent(run(&project, host, "edit-loop", &payload));
            }
            let warning = run(&project, host, "edit-loop", &payload);
            assert_eq!(warning.code, if host == "cursor" { 1 } else { 0 });
            assert!(
                String::from_utf8(warning.stdout)
                    .unwrap()
                    .contains("edited 7 times")
            );
            for _ in 0..2 {
                let w = run(&project, host, "edit-loop", &payload);
                assert_eq!(w.code, if host == "cursor" { 1 } else { 0 });
            }
            let blocked = run(&project, host, "edit-loop", &payload);
            assert_eq!(blocked.code, 2);
            assert!(
                String::from_utf8(blocked.stderr)
                    .unwrap()
                    .contains("BLOCKED:")
            );
        }
    }

    // Subagent write-lock serialization & release test
    fs::create_dir_all(project.join(".claude/agents")).unwrap();
    fs::write(
        project.join(".claude/agents/coder.md"),
        "---\nname: coder\ndescription: writer\nwrites: true\n---\nCoder agent\n",
    )
    .unwrap();
    fs::write(
        project.join(".claude/agents/reviewer.md"),
        "---\nname: reviewer\ndescription: reader\nwrites: false\n---\nReviewer agent\n",
    )
    .unwrap();

    // First write-capable agent succeeds & takes lock
    silent(run(
        &project,
        "claude",
        "subagent-gate",
        &json!({"tool_name":"Agent","tool_input":{"subagent_type":"coder"}}),
    ));
    // Read-only agent passes concurrently
    silent(run(
        &project,
        "claude",
        "subagent-gate",
        &json!({"tool_name":"Agent","tool_input":{"subagent_type":"reviewer"}}),
    ));
    // Unknown agent rejected
    let unknown = run(
        &project,
        "claude",
        "subagent-gate",
        &json!({"tool_name":"Agent","tool_input":{"subagent_type":"Explore"}}),
    );
    assert_eq!(unknown.code, 2);
    // Second concurrent write-capable agent rejected while first holds lock
    let second = run(
        &project,
        "claude",
        "subagent-gate",
        &json!({"tool_name":"Agent","tool_input":{"subagent_type":"coder"}}),
    );
    assert_eq!(second.code, 2);
    // Release lock on SubagentStop
    silent(run(
        &project,
        "claude",
        "subagent-release",
        &json!({"hook_event_name":"SubagentStop","agent_type":"coder"}),
    ));
    // Re-acquiring succeeds after release
    silent(run(
        &project,
        "claude",
        "subagent-gate",
        &json!({"tool_name":"Agent","tool_input":{"subagent_type":"coder"}}),
    ));
}

#[test]
fn guard_prefilter_and_write_locks_work_end_to_end() {
    if let Some(root) = std::env::var_os(CHILD_ROOT) {
        exercise(Path::new(&root));
        return;
    }
    let temporary = tempfile::tempdir().unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap());
    child
        .args([
            "--exact",
            "guard_prefilter_and_write_locks_work_end_to_end",
            "--nocapture",
        ])
        .env(CHILD_ROOT, temporary.path())
        .env("TMPDIR", temporary.path())
        .env("SUBAGENT_WRITE_LOCK_DIR", temporary.path())
        .current_dir(temporary.path());
    for var in ["CLAUDE_PROJECT_DIR", "CODEX_HOME", "CODEX_THREAD_ID"] {
        child.env_remove(var);
    }
    let output = child.output().unwrap();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
