---
description: Inspect guardrails policy, check a command with AST safety rules, or view session loop counters
argument-hint: "[command-to-check]"
allowed-tools: ["Bash", "Read"]
---

# Check Guardrails Policy & Commands

User arguments: `$ARGUMENTS`

1. Always pass `"root": "<repo-root>"` explicitly to MCP tools (required under Codex where `.codex-mcp.json` sets `"cwd": "."`).
2. If `$ARGUMENTS` is non-empty, validate the exact shell command strictly read-only without executing it:
   - Prefer MCP tool `guardrails_check_command` with `{"command": "$ARGUMENTS", "root": "<repo-root>"}`.
   - Or run `guardrails check-command --root "<repo-root>" --json "$ARGUMENTS"`.
3. Also inspect the active policy and loop counter status via `guardrails_inspect_policy` (`{"root": "<repo-root>"}`) + `guardrails_loop_status` (`{"root": "<repo-root>"}`) (or `guardrails status --root "<repo-root>" --json`).
4. Report whether the command is allowed or blocked (with exact rule category), active preset, and current session counter usage.
