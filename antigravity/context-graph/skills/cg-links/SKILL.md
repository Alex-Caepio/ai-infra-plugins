---
name: cg-links
description: Inspect cited Project Brain records, Memory Bank chunks, rename/missing hygiene warnings, and contradictions for files.
---

# Context Graph File Links Workflow

Apply this workflow to the target file path (and optional previous path) requested by the user:

Follow the `cg-omnigraph-sync` skill (`skills/cg-omnigraph-sync/SKILL.md`) and run `context_memory_links_for_file` (with `"project": "<repo_root>"` over MCP, or `context-graph-engine memory-links --root . --path ...`). Report linked Project Brain records, active Memory Bank chunks, `cited-file-missing` / `cited-file-renamed` warnings, and Omnigraph `contradiction_warnings`.
