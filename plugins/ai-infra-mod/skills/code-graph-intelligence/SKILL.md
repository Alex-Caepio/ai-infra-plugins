---
name: code-graph-intelligence
description: Explore codebase dependencies, blast radius, architectural layers, cycles, scenarios, test reach, git hotspots, and agent run audits via graph-map tools.
---

# Code Graph Intelligence (`graph-map`)

Use the `graph_*` tools inside `ai-infra-mod` to explore structure, change impact, layering, and recorded AI runs.

## Target Root Rule

Pass `project: "<repo-root>"` on every `graph_*` tool call (unless already fixed by `AI_INFRA_ROOT`).

## Workflows

- **Availability & Indexing**: Call `graph_status` (`check: true`). Build or import via `graph_build` (`import: "<graph.json>"` or `rebuild: true`).
- **Search & Traversal**:
  - `graph_find` (`query`, `target?`, `notests?`)
  - `graph_neighbors` (`node`, `out?`, `into?`)
  - `graph_link` (`from`, `to`) and `graph_path` (`from`, `to`)
  - `graph_shared` (`from`, `to`, `side: "both_use" | "uses_both"`)
  - `graph_scenario` (`entry`, `skip?`)
- **Change Impact & Test Reach**:
  - `graph_change_impact` (`target`: `"work"`, `"uncommitted"`, `"commit:<sha>"`, `"branch:<name>"`, `"pull:<num>"`, `"run:<id>"`)
  - `graph_test_reach` (`target`: `"work"`)
- **Architecture, Cycles & History**:
  - `graph_architecture` (`scope?`, `pattern?`, `modules?`)
  - `graph_cycle` (`cycle`)
  - `graph_history_hotspots` (`branch?`, `query?`)
- **Agent Run Audit**:
  - `graph_run_audit` (omit `session` to list Claude Code, Codex, and Harness runs; pass `session` + optional `launch`/`compare_launch` to inspect unopened dependents).
- **Interactive UI**:
  Run `ai-infra-mod graph --project "<repo-root>" serve --host 127.0.0.1 --port 7840`.
