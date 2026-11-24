---
name: cg-import
description: Dry-run validate or import a context-graph-bundle-v1 bundle with collision and drift handling.
---

# Context Graph Import Workflow

Apply this workflow to the bundle path and collision resolution options (`--strategy skip|remap-ids|archive-superseded`, `--apply`) requested by the user:

Follow the `context-import` skill (`skills/context-import/SKILL.md`). First run a dry-run (`"dry_run": true`) against the target repository (`"project": "<repo_root>"` over MCP or `context-graph-engine import --root . --dry-run`) to report SHA-256 validation, collisions, and target `source_drift`. Apply changes only when requested or after confirming strategy (`skip`, `remap-ids`, or `archive-superseded`).
