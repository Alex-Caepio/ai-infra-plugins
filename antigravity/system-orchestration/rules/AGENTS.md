# System Orchestration Extension Rules

- Always pass explicit `project` (or `root`) pointing to the target workspace root on every `system_*`, `orch_*`, and `omnigraph_*` MCP tool call. Omitted or relative `.` paths inside the extension directory fail closed.
- Use `system_graph_status` and `omnigraph_status` before cross-service execution or Omnigraph branch operations (`mode=off` by default unless configured).
- Compare divergent Omnigraph session branches with `omnigraph_branches_diff` and settle conflicts explicitly via `omnigraph_merge_keep`.
- Execution tools never auto-push commits; inspect `orch_delivery_preview` before staging or committing locally.
