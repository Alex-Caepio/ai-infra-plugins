---
name: guardrails-check
description: Inspect guardrails policy, check a shell command with AST safety rules, or view session loop counters.
---

# Guardrails Check Workflow

Apply this workflow to inspect the active policy or validate a specific shell command requested by the user:

1. Always pass `"root": "<repo-root>"` explicitly to MCP tools (required because `mcp_config.json` sets `"cwd": "${PLUGIN_ROOT}"`).
2. If the user supplied a command to validate, check it strictly read-only without executing it:
   - Prefer MCP tool `guardrails_check_command` with `{"command": "<cmd>", "root": "<repo-root>"}`.
   - Or run `guardrails check-command --root "<repo-root>" --json "<cmd>"`.
3. Also inspect the active policy and loop counter status via `guardrails_inspect_policy` (`{"root": "<repo-root>"}`) + `guardrails_loop_status` (`{"root": "<repo-root>"}`) (or `guardrails status --root "<repo-root>" --json`).
4. Report whether the command is allowed or blocked (with exact rule category), active preset, and current session counter usage.
