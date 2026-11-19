---
description: Initialize .agent-guardrails/policy.json with a stack preset (generic, laravel, symfony, wordpress, node, python, rust)
argument-hint: "[preset] [--force]"
allowed-tools: ["Bash", "Read"]
---

# Initialize Guardrails Policy

User arguments: `$ARGUMENTS`

Follow the `agent-guardrails:guardrails-setup` skill:

1. Determine the target repository root `<repo-root>` and preset from `$ARGUMENTS` (if omitted, detect the project stack from root manifests like `package.json`, `pyproject.toml`, `Cargo.toml`, `composer.json`/`artisan`/`bin/console`/`wp-config*`, or default to `generic`).
2. Call MCP tool `guardrails_init_policy` with `{"preset": "<preset>", "root": "<repo-root>"}` (always pass `"root"` explicitly so Codex plugin `cwd: "."` does not target the plugin cache; include `"force": true` only if `$ARGUMENTS` contains `--force`). If MCP is unavailable, run:
   ```sh
   guardrails policy-init --root "<repo-root>" --preset <preset>
   ```
3. Run `guardrails_inspect_policy` with `{"root": "<repo-root>"}` (or `guardrails status --root "<repo-root>" --json`) and summarize the active preset, rule count, loop thresholds, and policy file path.
