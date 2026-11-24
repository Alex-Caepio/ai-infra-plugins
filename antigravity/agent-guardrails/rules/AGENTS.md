# Agent Guardrails Antigravity Rules

1. **Always pass an explicit target repository root (`"root"`) to `guardrails_*` MCP tools**:
   - Antigravity runs `./bin/guardrails mcp` with `cwd: "${PLUGIN_ROOT}"`.
   - Always pass `"root": "<absolute-workspace-root>"` on `guardrails_init_policy`, `guardrails_inspect_policy`, `guardrails_check_command`, and `guardrails_loop_status` so the `plugin-cwd` guard validates against the target repository.
2. **Never bypass shell, loop, naming, or write-lock hooks**:
   - When `bash-guard`, `naming-check`, `edit-loop`, or `subagent-gate` denies a tool call, inspect the exact `reason` and fix the underlying command, file path, or failing edit rather than wrapping commands in nested shells or disabling hooks.
3. **Validate policy edits**:
   - After creating or editing `.agent-guardrails/policy.json`, always run `guardrails_inspect_policy` (`{"root": "<repo-root>"}`) to ensure schema validity, because malformed policy files trigger fail-closed denial.
4. **Trust boundary**:
   - Guardrails hooks are deterministic advisory runtime checks, not an OS sandbox or secret-management vault. Keep credentials out of prompts and files.
