---
name: guardrails-init
description: Initialize .agent-guardrails/policy.json with a stack preset (generic, laravel, symfony, wordpress, node, python, rust).
---

# Guardrails Init Workflow

Apply this workflow to initialize `.agent-guardrails/policy.json` for the repository and optional stack preset (`generic`, `laravel`, `symfony`, `wordpress`, `node`, `python`, `rust`, or `--force`) requested by the user:

Follow the `guardrails-setup` skill:

1. Determine the target repository root `<repo-root>` and preset from the user's request (if omitted, detect the project stack from root manifests like `package.json`, `pyproject.toml`, `Cargo.toml`, `composer.json`/`artisan`/`bin/console`/`wp-config*`, or default to `generic`).
2. Call MCP tool `guardrails_init_policy` with `{"preset": "<preset>", "root": "<repo-root>"}` (always pass `"root"` explicitly so extension `cwd: "${PLUGIN_ROOT}"` does not target the plugin cache; include `"force": true` only if requested). If MCP is unavailable, run:
   ```sh
   guardrails policy-init --root "<repo-root>" --preset <preset>
   ```
3. Run `guardrails_inspect_policy` with `{"root": "<repo-root>"}` (or `guardrails status --root "<repo-root>" --json`) and summarize the active preset, rule count, loop thresholds, and policy file path.
