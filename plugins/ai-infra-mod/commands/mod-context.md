---
name: mod-context
description: Retrieve Memory Bank & Project Brain context, traverse the multi-relation context graph, or save checkpoints and handoffs.
argument-hint: "\"<query|checkpoint|handoff|graph-node>\""
---

Query governed project context, SQLite working memory, and the multi-relation context graph for `$ARGUMENTS`.

**Important**: Pass `project: "<repo-root>"` unless `AI_INFRA_ROOT` is active.

1. Check `memory_status` (`project: "<repo-root>"`) and `memory_source_fingerprints` (`project: "<repo-root>"`, `sources: [...]`).
2. Run `memory_retrieve` (`project: "<repo-root>"`, `task_id: "<TASK-ID>"`, `query: "$ARGUMENTS"`) and traverse `context_graph_query` (`node?`, `kinds?`) / `context_graph_links` (`target`) for provenance.
3. If saving progress or transferring context across agents, run `memory_checkpoint` (`task_id`, `goal?`) or `context_handoff_save` (`goal`, `summary`, `next_steps`).
