# AI Infrastructure Mod Rules

- **Explicit Workspace Root**: Always pass `project`, `root`, `path`, `skill_path`, or `package_dir` pointing to the active workspace repository root on every MCP tool call (`mod_status`, `graph_*`, `system_*`, `orch_*`, `omnigraph_*`, `memory_*`, `context_*`, `skill_*`, `guardrails_*`, `skillcache_*`). Omitted or relative `.` paths when running from the extension cache are rejected fail-closed.
- **Omnigraph Routing (`80`-tool multiplexer)**:
  - Use `omnigraph_status`, `omnigraph_atoms`, `omnigraph_publish`, `omnigraph_branches_diff` (plural), and `omnigraph_merge_keep` for the `orch-engine` orchestration Omnigraph (`mode=off` until configured).
  - Use `omnigraph_sync_canonical`, `omnigraph_file_context`, `context_omnigraph_atoms`, `omnigraph_branch_diff` (singular), `context_omnigraph_merge_keep`, and `context_memory_links_for_file` for `context-graph-engine`'s embedded offline SQLite Omnigraph (`omnigraph.sqlite3`).
- **Repo-Relative vs Path Arguments**: `path` / `previous_path` on `graph_file_impact`, `graph_memory_links`, `omnigraph_file_context`, and `context_memory_links_for_file` expect repository-relative file paths; branch names (`source`, `target`, `from`, `to`) on Omnigraph diff/merge calls are ref names, not filesystem paths.
