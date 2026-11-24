---
name: sc-gate1
description: Run static Gate 1 structural, allowlist, dataflow, and secret-safety checks on a SkillCache package directory.
---

# SkillCache Gate 1 Workflow

Apply this workflow to the external `<package-dir>` and optional `<examples.json>` requested by the user (paths must point outside the extension directory):

1. Extract `<package-dir>` and optional examples file path from the user's request.
2. Call MCP tool `skillcache_gate1` with `{"package_dir": "<package-dir>"}` (and optional `"examples_path": "<examples.json>"`).
3. If MCP is unavailable, run:
   ```sh
   skillcache-engine gate1 "<package-dir>" --examples "<examples.json>"
   ```
4. Report whether Gate 1 passed and list every violation code, field, and remediation message if any check failed.
