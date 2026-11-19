---
name: context-memory-transfer
description: Manage Project Brain, Memory Bank, unified multi-relation SQLite context graphs, portable context-graph-bundle-v1 export/import, and session handoffs.
---

# Context Memory & Portable Graph Transfer (`context-graph`)

Use the `memory_*` and `context_*` tools inside `ai-infra-mod` for governed memory continuity, N-hop graph traversal, and cross-repo/cross-tool knowledge transfer.

## Target Root Rule

Pass `project: "<repo-root>"` on every tool call (unless fixed by `AI_INFRA_ROOT`).

## Core Workflows

1. **Retrieve & Check Status**:
   - `memory_status` (`project?`, `root?`)
   - `memory_retrieve` (`task_id`, `query`, `paths?`, `project?`, `root?`) — both `task_id` and `query` are required.
   - `memory_source_fingerprints` (`sources`, `project?`, `root?`) — `sources` (array of file paths) is required.
2. **N-Hop Graph & Provenance**:
   - `context_graph_query` (`node?`, `root_id?`, `direction`: `"out" | "in" | "both"`, `depth?`, `kinds?`, `limit?`, `include_archived?`, `format?`, `project?`, `root?`) across all 8 edge kinds (`cites`, `touches`, `conflicts`, `supersedes`, `promoted_from`, `consolidated_from`, `handoff_of`, `session_merge`).
   - `context_graph_links` (`target`, `include_archived?`, `project?`, `root?`) — `target` is required to trace lineage and reverse impact.
3. **Continuity & Checkpoints**:
   - `memory_checkpoint` (`task_id`, `goal?`, `project?`, `root?`) and `memory_record_result` (`task_id`, `result_id`, `revision`, `progress?`, `learnings?`, `next_steps?`, `verified?`).
   - `context_handoff_save` (`goal`, `summary`, `next_steps`, `task_id?`, `files?`, `decisions?`, `constraints?`) and `context_handoff_load` (`handoff_id?`, `task_id?`, `latest?`, `input?`, `include_transcript?`).
4. **Portable Bundle Export / Import (`context-graph-bundle-v1`)**:
   - `context_export_bundle` (`destination`, `subgraph_root?`, `node?`, `depth?`, `format?`, `force?`, `include_archive?`, `include_episodes?`, `include_handoffs?`, `include_superseded?`) — `destination` is required.
   - `context_import_bundle` (`source`, `dry_run?`, `strategy`: `"skip" | "remap-ids" | "archive-superseded"`, `include_archive?`, `include_episodes?`, `include_handoffs?`) — `source` is required; always dry-run (`dry_run: true`) first.
