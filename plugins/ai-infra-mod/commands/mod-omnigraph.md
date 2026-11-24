---
name: mod-omnigraph
description: Sync Project Brain & Memory Bank to embedded Omnigraph, inspect file context and memory links, publish/query atoms, diff branches, and settle merge conflicts.
argument-hint: "[status | sync | file <path> | diff <source>..<target> | publish]"
---

Operate embedded and orchestration Omnigraph graphs for `$ARGUMENTS`.

**Important**: Pass explicit `project: "<repo-root>"` unless `AI_INFRA_ROOT` is active.

1. **Embedded Offline Omnigraph (`context-graph-engine`)**:
   - Call `omnigraph_sync_canonical` (`project`, `prune?`) to index active Brain records & Memory Bank chunks (`knowledge/canonical`, `SymbolRef`).
   - Call `omnigraph_file_context` (`project`, `path`) or `context_memory_links_for_file` (`project`, `path`, `previous_path?`) for file-level atoms, declarations, hygiene warnings, and `contradiction_warnings`.
   - Query/diff/merge via `context_omnigraph_atoms`, `omnigraph_branch_diff` (`source`, `target?`), and `context_omnigraph_merge_keep`.
2. **Orchestration Omnigraph (`orch-engine`)**:
   - Inspect mode & branches via `omnigraph_status` (`project`, `branch?`) and `system_graph_status` (`project`, `check?`).
   - Browse, publish, diff, or settle orchestration atoms via `omnigraph_atoms`, `omnigraph_publish` (`title`, `files?`, ...), `omnigraph_branches_diff` (`source`, `target?`), and `omnigraph_merge_keep` (`atom_id`, `from`, `to?`).
3. Report conflict details, contradiction warnings, and settled state clearly.
