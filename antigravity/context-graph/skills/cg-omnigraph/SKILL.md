---
name: cg-omnigraph
description: Sync canonical knowledge to Omnigraph, inspect file context, diff branches, or resolve merge conflicts.
---

# Context Graph Omnigraph Workflow

Apply this workflow to the requested Omnigraph operation (`sync-canonical|file-context|atoms|branch-diff|merge-keep`) and arguments provided by the user:

Follow the `cg-omnigraph-sync` skill (`skills/cg-omnigraph-sync/SKILL.md`) and execute the requested Omnigraph operation against the target repository (`"project": "<repo_root>"` over MCP with `omnigraph_sync_canonical`, `omnigraph_file_context`, `omnigraph_atoms`, `omnigraph_branch_diff`, `omnigraph_merge_keep`, or CLI `context-graph-engine omnigraph ... --root .`). Always inspect `contradiction_warnings` and preview `omnigraph_branch_diff` before resolving conflicts with `keep_source`, `keep_target`, or verbatim per-atom `keep`.
