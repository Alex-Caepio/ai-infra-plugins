# Agent Guardrails (Antigravity Extension)

Portable POSIX AST shell safety guard, `$TMPDIR` command/edit loop breakers, repository file-naming enforcement, and advisory subagent write-lock gates packaged as an Antigravity Extension.

## Included Components

- **`mcp_config.json`**: Registers the `agent-guardrails` MCP server (`./bin/guardrails mcp`, `cwd: "${PLUGIN_ROOT}"`) exposing 4 tools:
  - `guardrails_init_policy`, `guardrails_inspect_policy`, `guardrails_check_command`, `guardrails_loop_status`
- **`hooks.json`**: Registers Antigravity `PreToolUse` (`bash-guard`, `naming-check`, `subagent-gate`) and `PostToolUse` (`edit-loop`, `subagent-release`) handlers (`./bin/guardrails hook antigravity ...`).
- **`rules/AGENTS.md`**: Requires explicit `"root": "<repo-root>"` targets (`plugin-cwd` guard compatibility) and validates `.agent-guardrails/policy.json` changes.
- **`skills/`**: `guardrails-setup`, `policy-authoring`, `guardrails-check`, `guardrails-init`.
- **`agents/`**: `ag-policy-advisor`.

## Installation

Copy this directory alongside the compiled `bin/guardrails` binary from `plugins/agent-guardrails/` (released in [Alex-Caepio/agent-guardrails](https://github.com/Alex-Caepio/agent-guardrails)) into your Antigravity plugins directory (`~/.gemini/config/plugins/agent-guardrails/`), then enable `Agent Guardrails` on the Customizations page (`disabled: true` by default).

## License

Apache-2.0 — see [LICENSE](./LICENSE).
