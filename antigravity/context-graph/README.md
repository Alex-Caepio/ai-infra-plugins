# Context Graph (Antigravity Extension)

Unified Context Graph, Project Brain, Memory Bank, embedded Omnigraph sync/conflict resolution, portable session handoffs, and SHA-256 attested transfer bundles packaged as an Antigravity Extension.

## Included Components

- **`mcp_config.json`**: Registers the `context-graph` MCP server (`./bin/context-graph-engine mcp`, `cwd: "${PLUGIN_ROOT}"`) exposing 17 tools (`memory_*`, `context_*`, `omnigraph_*`).
- **`hooks.json`**: Registers Antigravity `SessionStart`, `PreInvocation`, and `Stop` lifecycle hooks (`./bin/context-graph-engine hook antigravity ...`).
- **`rules/AGENTS.md`**: Requires explicit `"project": "<repo-root>"` on MCP calls, verification against cited files, and secret-free handoffs.
- **`skills/`**: `context-export`, `context-graph`, `context-handoff`, `context-import`, `memory-checkpoint`, `cg-omnigraph-sync`, `cg-checkpoint`, `cg-export`, `cg-graph`, `cg-import`, `cg-links`, `cg-load`, `cg-omnigraph`, `cg-save`.
- **`agents/`**: `cg-graph-analyst`, `cg-memory-curator`.

## Installation

Copy this directory alongside the compiled `bin/context-graph-engine` binary from `plugins/context-graph/` (released in [Alex-Caepio/context-graph](https://github.com/Alex-Caepio/context-graph)) into your Antigravity plugins directory (`~/.gemini/config/plugins/context-graph/`), then enable `Context Graph` on the Customizations page (`disabled: true` by default).

## License

MIT — see [LICENSE](./LICENSE).
