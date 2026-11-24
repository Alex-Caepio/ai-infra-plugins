---
name: mod-guard
description: Inspect guardrails policy, initialize a stack preset, dry-run POSIX AST bash & read-only checks, or inspect normalized loop/lock diagnostics.
argument-hint: "\"[check <cmd>|init <preset>|loops|status]\""
---

Run execution safety diagnostics for `$ARGUMENTS` via `guardrails_*` MCP tools.

**Important**: Pass explicit `root: "<repo-root>"` unless `AI_INFRA_ROOT` is configured.

1. Call `guardrails_inspect_policy` (`root: "<repo-root>"`) and `guardrails_loop_status` (`root: "<repo-root>"`) to review active rules and `active_signature` / command & edit loop counters.
2. If testing a shell command, call `guardrails_check_command` with `{ "root": "<repo-root>", "command": "..." }` (checks `read_only`, `rg --pre`, `core.hooksPath`, secret-write redirections, and destructive patterns).
3. If initializing `.agent-guardrails/policy.json`, call `guardrails_init_policy` with preset (`generic`, `laravel`, `symfony`, `wordpress`, `node`, `python`, `rust`).
