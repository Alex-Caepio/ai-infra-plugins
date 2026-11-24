# Graph Map Extension Rules

- Always pass `project: "<absolute-workspace-root>"` on every `graph_*` MCP tool call. Calling tools with an omitted or relative path while running inside the extension directory is rejected by the `plugin-cwd` guard.
- Check `graph_status` (`check: true`) before graph traversal; if unbuilt (`state == "off"` and no `fallback_graph`), run `graph_build` to index 13 languages natively via tree-sitter AST into SQLite IR.
- For single-file blast radius and Project Brain / Memory Bank citations, prefer `graph_file_impact` and `graph_memory_links` (passing repository-relative `path` / `previous_path`).
- Clearly distinguish statically extracted code edges (`certain: true`) from heuristic/name-inferred links in reports.
