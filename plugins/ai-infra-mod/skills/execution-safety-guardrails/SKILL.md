---
name: execution-safety-guardrails
description: Inspect and configure execution safety guardrails (POSIX AST bash protection, preset selection, file-naming rules, loop breakers, and subagent write-lock status).
---

# Execution Safety Guardrails (`agent-guardrails`)

Use the `guardrails_*` tools inside `ai-infra-mod` to check shell command safety, inspect `.agent-guardrails/policy.json`, initialize stack presets, and inspect loop counters.

## Target Root Rule

Pass explicit `root: "<repo-root>"` to `guardrails_*` tools (unless `AI_INFRA_ROOT` is set). Never omit `root` when launched from a plugin directory.

## Workflows

1. **Inspect Policy (`guardrails_inspect_policy`)**:
   Review active rules, preset, `blocked_globs`, protected paths (`MEM-*`, `tasks/TASK-NNN/`, `specs/`), and credential/SQL/destructive bash guards for `root`.
2. **Initialize / Switch Preset (`guardrails_init_policy`)**:
   Write or customize `.agent-guardrails/policy.json` using presets: `generic`, `laravel`, `symfony`, `wordpress`, `node`, `python`, or `rust`.
3. **Dry-Run Shell Command Checks (`guardrails_check_command`)**:
   Parse arbitrary shell strings through the full POSIX AST inspector (`bash -c`, `eval`, `ssh`, `docker`, `kubectl`, heredocs, subshells, `DROP`/`TRUNCATE`/`DELETE`, out-of-repo `rm`/`git`/`gh`) before running them live.
4. **Loop & Lock Diagnostics (`guardrails_loop_status`)**:
   Inspect command (`6`/`12`) and edit (`7`/`10`) loop detection counters and active subagent write-lock state.
