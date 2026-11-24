---
name: mod-map
description: Analyze dependency blast radius via native 13-language AST + SQLite IR, A..B ranges, per-file impact, linked memory, layers, cycles, scenarios, test reach, or launch the Canvas2D WASM UI.
argument-hint: "\"[work|commit:sha|from..to|path|serve|search-term]\""
---

Inspect dependency topology and change impact for `$ARGUMENTS` via `graph_*` MCP tools.

**Important**: Pass `project: "<repo-root>"` unless `AI_INFRA_ROOT` is active.

1. Run `graph_status` (`project: "<repo-root>"`, `check: true`). If unbuilt and no `fallback_graph` exists, run `graph_build` (native 13-language tree-sitter AST + SQLite IR).
2. Based on `$ARGUMENTS`:
   - `"serve"`: run `./bin/ai-infra-mod graph --project "<repo-root>" serve --host 127.0.0.1 --port 7840` (Canvas2D WASM UI, 5 lenses, Commit DAG, SSE).
   - `work` / `uncommitted` / `commit:...` / `ref:...` / `pr:...` / `range:...` / `run:...` / `<from>..<to>`: run `graph_change_impact` (supporting `lod`, `min_weight`, `from`/`to`, `focus_session`, `catalog_only`) and `graph_test_reach`.
   - File path: inspect `graph_file_impact`, `graph_memory_links`, and `graph_neighbors` (`depth: 1..6`).
   - `"layers"` / `"cycles"`: run `graph_architecture` and `graph_cycle`.
   - Anything else: search via `graph_find` and inspect `graph_neighbors`.
