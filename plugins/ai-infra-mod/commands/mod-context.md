---
name: mod-context
description: Retrieve Memory Bank & Project Brain context, sync/query embedded Omnigraph, inspect file links, or save checkpoints and handoffs.
argument-hint: "\"<query|checkpoint|handoff|graph-node|path>\""
---

Query governed project context, SQLite working memory, embedded Omnigraph, and the multi-relation context graph for `$ARGUMENTS`.

**Important**: Pass `project: "<repo-root>"` unless `AI_INFRA_ROOT` is active.

1. Check `memory_status` (`project: "<repo-root>"`) and `memory_source_fingerprints` (`project: "<repo-root>"`, `sources: [...]`).
2. Run `memory_retrieve` (`project: "<repo-root>"`, `task_id: "<TASK-ID>"`, `query: "$ARGUMENTS"`), traverse `context_graph_query` / `context_graph_links`, or check `omnigraph_sync_canonical`, `omnigraph_file_context`, `context_omnigraph_atoms`, and `context_memory_links_for_file` when targeting workspace files.
3. If saving progress or transferring context across agents, run `memory_checkpoint` (`task_id`, `goal?`) or `context_handoff_save` (`goal`, `summary`, `next_steps`).
