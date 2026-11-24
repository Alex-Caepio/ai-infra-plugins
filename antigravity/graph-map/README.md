# Graph Map Antigravity Extension (`graph-map`)

Standalone interactive code-graph explorer (`v0.2.0`) packaged for Google Antigravity (`disabled: true` by default so modular users can enable it independently or use `ai-infra-mod`).

## Included Capabilities

- **MCP Server (`mcp_config.json`)**: `16` read/index tools (`graph_status`, `graph_build`, `graph_find`, `graph_neighbors`, `graph_link`, `graph_path`, `graph_shared`, `graph_cycle`, `graph_change_impact`, `graph_architecture`, `graph_scenario`, `graph_test_reach`, `graph_history_hotspots`, `graph_run_audit`, `graph_file_impact`, `graph_memory_links`) backed by `./bin/graph-map mcp`.
- **Native 13-Language Tree-Sitter AST + SQLite IR**: Indexes Rust, PHP, TypeScript/TSX, JavaScript, Python, Go, Java, C#, C, C++, Ruby, HTML, and CSS without Python or external `graphify`.
- **Interactive Canvas2D WASM UI**: Launch `./bin/graph-map --project "<repo-root>" serve --host 127.0.0.1 --port 7840`.
- **Skills (`7` in `skills/`)**: `change-impact`, `architecture-audit`, `commit-work-map`, `trace-scenario`, `graph-map`, `graph-impact`, `graph-layers`.
- **Subagents (`2` in `agents/`)**: `gm-graph-analyst`, `gm-impact-reviewer`.
- **Always-On Rule**: `rules/AGENTS.md` enforces explicit `project` root passing and edge-certainty discipline.

## License

Apache-2.0 — see [LICENSE](LICENSE).
