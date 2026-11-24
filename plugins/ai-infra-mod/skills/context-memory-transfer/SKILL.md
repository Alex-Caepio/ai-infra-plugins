---
name: context-memory-transfer
description: Manage Project Brain, Memory Bank, embedded offline Omnigraph canonical sync & branch conflicts, graph-to-memory file links, IR-epoch snapshots, and portable context-graph-bundle-v1 export/import.
---

# Context Memory, Embedded Omnigraph & Portable Transfer (`context-graph`)

Use the `memory_*`, `context_*`, and embedded Omnigraph tools inside `ai-infra-mod` for governed memory continuity, N-hop graph traversal, offline `omnigraph.sqlite3` synchronization, `SymbolRef` file-context lookups, and cross-repo bundle transfer.

## Target Root Rule

Pass `project: "<repo-root>"` on every tool call (unless fixed by `AI_INFRA_ROOT`).

## Core Workflows

1. **Retrieve & Check Status (IR-Epoch Aware)**:
   - `memory_status` (`project?`, `root?`)
   - `memory_retrieve` (`task_id`, `query`, `paths?`, `project?`, `root?`) — both `task_id` and `query` are required; snapshot caches automatically invalidate when underlying graph IR revisions advance.
   - `memory_source_fingerprints` (`sources`, `project?`, `root?`) — `sources` (array of file paths) is required.
2. **N-Hop Context Graph & File-to-Memory Bridge**:
   - `context_graph_query` (`node?`, `root_id?`, `direction`: `"out" | "in" | "both"`, `depth?`, `kinds?`, `limit?`, `include_archived?`, `format?`, `project?`, `root?`) across all 8 edge kinds (`cites`, `touches`, `conflicts`, `supersedes`, `promoted_from`, `consolidated_from`, `handoff_of`, `session_merge`).
   - `context_graph_links` (`target`, `include_archived?`, `project?`, `root?`) — trace lineage and reverse impact.
   - `context_memory_links_for_file` (`project`, `path`, `previous_path?`) — inspect Brain records, Memory chunks, Omnigraph atoms, `cited-file-missing`/`cited-file-renamed` hygiene warnings, and `contradiction_warnings`.
3. **Embedded Offline Omnigraph (`omnigraph.sqlite3` + `sync_ledger.sqlite3`)**:
   - `omnigraph_sync_canonical` (`project`, `prune?`) — sync active Project Brain & Memory Bank records into `knowledge/canonical` with `TouchesFile` / `MentionsSymbol` (`SymbolRef`) indexing and retired-atom pruning.
   - `omnigraph_file_context` (`project`, `path`) — inspect atoms, attached symbols, and contradiction warnings for a workspace file.
   - `context_omnigraph_atoms` (`project`, `branch?`, `kind?`, `query?`, `limit?`, `offset?`) — query atoms inside the embedded SQLite Omnigraph (distinct from `orch-engine`'s `omnigraph_atoms`).
   - `omnigraph_branch_diff` (`project`, `source`, `target?`) & `context_omnigraph_merge_keep` (`project`, `atom_id?`, `from?`, `to?`, `strategy?`, `source?`, `target?`) — inspect 3-way `merge_base` diffs and settle conflicts with `keep_source`, `keep_target`, or optimistic per-atom `keep`.
   - CLI for NDJSON export/import & vetted promotion: `ai-infra-mod context omnigraph {promote-atom,export-ndjson,import-ndjson} ...`.
4. **Continuity & Checkpoints**:
   - `memory_checkpoint` (`task_id`, `goal?`, `project?`, `root?`) and `memory_record_result` (`task_id`, `result_id`, `revision`, `progress?`, `learnings?`, `next_steps?`, `verified?`).
   - `context_handoff_save` (`goal`, `summary`, `next_steps`, `task_id?`, `files?`, `decisions?`, `constraints?`) and `context_handoff_load` (`handoff_id?`, `task_id?`, `latest?`, `input?`, `include_transcript?`).
5. **Portable Bundle Export / Import (`context-graph-bundle-v1`)**:
   - `context_export_bundle` (`destination`, `subgraph_root?`, `node?`, `depth?`, `format?`, `force?`, `include_archive?`, `include_episodes?`, `include_handoffs?`, `include_superseded?`) — `destination` is required.
   - `context_import_bundle` (`source`, `dry_run?`, `strategy`: `"skip" | "remap-ids" | "archive-superseded"`, `include_archive?`, `include_episodes?`, `include_handoffs?`) — `source` is required; always dry-run (`dry_run: true`) first.
