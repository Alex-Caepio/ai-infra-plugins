# System Orchestration Antigravity Extension (`system-orchestration`)

Multi-service orchestration, supervised session control, fleet/clash review, Omnigraph bridge, and subagent dispatch hooks packaged for Google Antigravity (`disabled: true` by default).

## Included Capabilities

- **MCP Server (`mcp_config.json`)**: `25` tools (`system_*`, `orch_*`, `omnigraph_*`, `system_graph_status`, `orch_permission_mode`) served by `./bin/orch-engine mcp`.
- **Native Antigravity Hooks (`hooks.json`)**: `PreToolUse` (`subagent-gate`) and `PostToolUse` (`subagent-dispatch`) on `invoke_subagent`.
- **Embedded WASM Workbench**: Launch `./bin/orch-engine serve --project "<repo-root>" --port 8766`.
- **Skills (`12` in `skills/`)**: `system-plan`, `system-run`, `multi-agent-dispatch`, `clash-debate`, `fleet-review`, `orch-omnigraph-sync`, `orch-plan`, `orch-run`, `orch-fleet-review`, `orch-clash`, `orch-ui`, `orch-omnigraph`.
- **Subagents (`2` in `agents/`)**: `orch-planner`, `orch-reviewer`.
- **Always-On Rule**: `rules/AGENTS.md`.

## License

MIT — see [LICENSE](LICENSE).
