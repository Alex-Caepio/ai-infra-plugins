---
name: mod-map
description: Analyze dependency blast radius, layers, cycles, scenarios, test reach, or launch the interactive localhost WASM code-graph UI.
argument-hint: "\"[work|commit:sha|path|serve|search-term]\""
---

Inspect dependency topology and change impact for `$ARGUMENTS` via `graph_*` MCP tools.

**Important**: Pass `project: "<repo-root>"` unless `AI_INFRA_ROOT` is active.

1. Run `graph_status` (`project: "<repo-root>"`, `check: true`). If unbuilt and no `fallback_graph` exists, run `graph_build`.
2. Based on `$ARGUMENTS`:
   - `"serve"`: run `./bin/ai-infra-mod graph --project "<repo-root>" serve --host 127.0.0.1 --port 7840`.
   - `work` / `uncommitted` / `commit:...` / `branch:...` / `pull:...` / `run:...`: run `graph_change_impact` and `graph_test_reach`.
   - `"layers"` / `"cycles"`: run `graph_architecture` and `graph_cycle`.
   - Anything else: search via `graph_find` and inspect `graph_neighbors`.
