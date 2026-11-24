---
name: cg-export
description: Export a privacy-filtered, SHA-256 attested context-graph-bundle-v1 directory or .zip archive.
---

# Context Graph Export Workflow

Apply this workflow to the destination path and export options (`--format zip|dir`, `--subgraph-root NODE`) requested by the user:

Follow the `context-export` skill (`skills/context-export/SKILL.md`) and export a `context-graph-bundle-v1` bundle for the target repository (passing `"project": "<repo_root>"` in `context_export_bundle` over MCP or `context-graph-engine export --root .`). Report artifact counts, excluded private records, bundle path, and `schema_version`.
