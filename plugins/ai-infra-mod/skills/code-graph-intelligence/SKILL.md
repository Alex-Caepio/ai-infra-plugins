---
name: code-graph-intelligence
description: Explore codebase dependencies via native 13-language tree-sitter AST + SQLite IR, progressive LOD slices, per-file blast radius, linked project memory, architectural layers, cycles, scenarios, test reach, git hotspots, and agent run audits via graph-map tools.
---

# Code Graph Intelligence (`graph-map`)

Use the `graph_*` tools inside `ai-infra-mod` to build/query the native 13-language tree-sitter AST + incremental SQLite IR index and explore dependency topology, change impact, layering, linked memory, and recorded AI runs.

## Target Root Rule

Pass `project: "<repo-root>"` on every `graph_*` tool call (unless already fixed by `AI_INFRA_ROOT`).

## Workflows

- **Availability & Native Indexing**: Call `graph_status` (`check: true`). Run `graph_build` (`rebuild?`, `wait?`) to index Rust, PHP, TypeScript/TSX, JavaScript, Python, Go, Java, C#, C, C++, Ruby, HTML, and CSS natively into SQLite IR (no Python/`graphify` required; `import: "<graph.json>"` remains supported).
- **Search & Progressive LOD Traversal**:
  - `graph_find` (`query`, `target?`, `notests?`)
  - `graph_neighbors` (`node`, `depth?: 1..6`, `out?`, `into?`, `no_incoming?`, `lod?: "overview" | "folder" | "file" | "symbol"`, `min_weight?`, `whole?`, `offset?`, `limit?`)
  - `graph_link` (`from`, `to`, `base?`, `lod?`, `min_weight?`) and `graph_path` (`from`, `to`, `longer?`, `lod?`, `min_weight?`)
  - `graph_shared` (`from`, `to`, `side: "both_use" | "uses_both"`, `skip?`, `offset?`, `limit?`)
  - `graph_scenario` (`entry`, `skip?`, `lod?`, `min_weight?`)
- **Change Impact, `A..B` Ranges, Per-File Blast & Memory Links**:
  - `graph_change_impact` (`target`: `"work"`, `"uncommitted"`, `"commit:<sha>"`, `"ref:<name>"`, `"pr:<num>"`, `"range:<from>..<to>"`, `"run:<id>"`, or explicit `from`+`to` range params; plus optional `focus_session`, `agent`, `catalog_only`, `lod`, `min_weight`)
  - `graph_file_impact` (`path`, `target?` or `from`+`to`, `focus_session?`, `agent?`, `certain?`) — per-file blast radius, symbol diffs, callers/callees, and reaching tests
  - `graph_memory_links` (`path` or batch `files: [...]`, `previous_path?`) — Project Brain records and Memory Bank citations/hygiene warnings
  - `graph_test_reach` (`target`: `"work"` or `from`+`to`)
- **Architecture, Cycles & History**:
  - `graph_architecture` (`scope?`, `pattern?`, `modules?`, `no_incoming?`, `lod?`, `min_weight?`, `from?`, `to?`)
  - `graph_cycle` (`cycle`, `base?`, `lod?`, `min_weight?`)
  - `graph_history_hotspots` (`branch?`, `query?`, `skip?`, `lod?`, `min_weight?`)
- **Agent Run Audit**:
  - `graph_run_audit` (omit `session` to list Claude Code, Codex, and Harness runs; pass `session` + optional `launch`/`compare_launch` to inspect unopened dependents).
- **Interactive Canvas2D WASM UI (5 Lenses & Commit DAG + SSE)**:
  Run `ai-infra-mod graph --project "<repo-root>" serve --host 127.0.0.1 --port 7840`.
