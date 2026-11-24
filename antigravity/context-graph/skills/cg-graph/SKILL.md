---
name: cg-graph
description: Query the unified Context Graph (nodes, edges, N-hop provenance, impact, or DOT export).
---

# Context Graph Query Workflow

Apply this workflow to the node, file path, or traversal options (`--depth N`, `--direction out|in|both`, `--dot`) requested by the user:

Follow the `context-graph` skill (`skills/context-graph/SKILL.md`) and run a Context Graph query against the target repository (pass `"project": "<repo_root>"` when invoking `context_graph_query` / `context_graph_links` over MCP, or use `context-graph-engine graph ... --root .`). Report matching nodes, typed edges (`cites`, `touches`, `conflicts`, `supersedes`, `promoted_from`, `consolidated_from`, `handoff_of`, `session_merge`), freshness status, and provenance paths.
