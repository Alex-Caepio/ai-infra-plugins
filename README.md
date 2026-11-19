# ai-infra-plugins

Standalone, dual-compatible **Claude Code (`2.1.278`)** and **Codex CLI (`0.169.0`)** plugin marketplace extracted from the native Rust runtime of `AI-Infrastructure`.

Every plugin in `./plugins/` is self-contained (zero path/runtime dependencies back to `AI-Infrastructure`, zero `target/` or root `CLAUDE.md`), ships a prebuilt stripped Linux x86_64 Rust binary in `bin/`, exposes a stdio JSON-RPC `2025-06-18` MCP server (`<bin> mcp`), and provides dual `.claude-plugin/` + `.codex-plugin/` manifests, `skills/`, `commands/`, and (where applicable) `hooks/`.

## Plugins Summary

| Plugin | Binary (`bin/`, stripped ELF) | Verified Tests | MCP Tools | Skills / Commands / Hooks | Highlights |
| :--- | :--- | :---: | :---: | :---: | :--- |
| **`graph-map`** | `graph-map` (`7.3M`) | `243/243` | `14` | `4` / `3` / — | Streaming code-graph engine (`graphify`), Cargo/git/PR blast-radius, layers/cycles/test-reach, Claude/Codex/Harness run audit, embedded WASM SVG UI (`924K` `graph_map_ui_bg.wasm`). |
| **`system-orchestration`** | `orch-engine` (`15M`) | `227/227` | `18` | `5` / `5` / `3` | Multi-repo `system` plan/execute, session scheduler & atomic inbox, fleet review, clash debate, SDD delivery previews, SQLite `RunWorkingMemory`, `attach`, embedded WASM UI (`2.1M` `orch_ui_bg.wasm`). |
| **`context-graph`** | `context-graph-engine` (`12M`) | `134/134` | `11` | `5` / `5` / `6` | Project Brain, Memory Bank, SQLite FTS5 retrieval, unified N-hop `graph_nodes`/`graph_edges`, session handoffs, SHA-256 `context-graph-bundle-v1` export/import, memory hooks. |
| **`skill-foundry`** | `skill-foundry` (`4.7M`) | `8/8` | `8` | `4` / `4` / — | Skill scaffold/validate/package, cross-CLI trigger & `routing-eval --skills-dir`, MT19937 seed-42 optimizer, `cl100k` budget linter, `8+4` `vet-skill`, dual-host Claude/Codex BTE `cost-audit`, HTML/OOXML review UI. |
| **`agent-guardrails`** | `guardrails` (`2.9M`) | `11/11` | `4` | `2` / `2` / `5` | POSIX AST `bash-guard`, `$TMPDIR` command (`6`/`12`) & edit (`7`/`10`) loop breakers, file-naming check, `/tmp` `flock` + `O_NOFOLLOW` subagent write locks, 7 presets + custom `.agent-guardrails/policy.json`. |
| **Total (5 plugins)** | **5 standalone binaries** | **`623/623`** | **`55`** | **`20` / `19` / `14`** | **Plugin-dir `cwd` guard across all 5 MCP servers; dual Claude + Codex validated.** |

> Подробный архитектурный анализ и матрица декуплинга (на русском): [`ANALYSIS.md`](./ANALYSIS.md).

## Standalone GitHub Repositories

- Marketplace: <https://github.com/Alex-Caepio/ai-infra-plugins>
- `graph-map`: <https://github.com/Alex-Caepio/graph-map>
- `system-orchestration`: <https://github.com/Alex-Caepio/system-orchestration>
- `context-graph`: <https://github.com/Alex-Caepio/context-graph>
- `skill-foundry`: <https://github.com/Alex-Caepio/skill-foundry>
- `agent-guardrails`: <https://github.com/Alex-Caepio/agent-guardrails>

## Claude Code Installation & Validation

Verified against `claude` (`2.1.278`):

```bash
# Register the GitHub marketplace and install any plugin
claude plugin marketplace add Alex-Caepio/ai-infra-plugins
claude plugin install graph-map@ai-infra-plugins --scope user
claude plugin install system-orchestration@ai-infra-plugins --scope user
claude plugin install context-graph@ai-infra-plugins --scope user
claude plugin install skill-foundry@ai-infra-plugins --scope user
claude plugin install agent-guardrails@ai-infra-plugins --scope user

# Or validate / load from a local checkout
claude plugin validate --strict ./.claude-plugin/marketplace.json
claude plugin validate --strict ./plugins/skill-foundry
claude --plugin-dir ./plugins/agent-guardrails
```

## Codex CLI Installation & Validation

Verified against `codex` (`0.169.0`) (Codex CLI installs plugins via marketplace rather than a directory flag):

```bash
# Register the GitHub (or local checkout) marketplace (.agents/plugins/marketplace.json)
codex plugin marketplace add https://github.com/Alex-Caepio/ai-infra-plugins.git
# Or from a cloned local directory:
codex plugin marketplace add .

# Install plugins and verify loaded state
codex plugin add graph-map@ai-infra-plugins
codex plugin add system-orchestration@ai-infra-plugins
codex plugin add context-graph@ai-infra-plugins
codex plugin add skill-foundry@ai-infra-plugins
codex plugin add agent-guardrails@ai-infra-plugins
codex plugin list --json
```

**Important note on Codex MCP `cwd` safety**: Every `.codex-mcp.json` runs `"command": "./bin/<binary>"` with `"cwd": "."` (the cached plugin directory). Because that directory may sit beneath an ancestor `.git` / `.claude` / `.agents`, all 5 MCP servers detect plugin/cache `cwd` BEFORE walking ancestors and reject omitted or `"."` target arguments (`isError: true`) rather than silently mutating `~/` or the plugin cache. Always pass explicit `"project"` / `"root"` / `"path"` pointing to your target repository when calling MCP tools.

## Direct CLI & MCP Smoke Checks

```bash
./plugins/graph-map/bin/graph-map --help
./plugins/system-orchestration/bin/orch-engine --help
./plugins/context-graph/bin/context-graph-engine --help
./plugins/skill-foundry/bin/skill-foundry --help
./plugins/agent-guardrails/bin/guardrails --help

# Stdio MCP JSON-RPC 2.0 handshake + tool discovery
printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"check","version":"0.1"}}}\n{"jsonrpc":"2.0","method":"notifications/initialized"}\n{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}\n' | ./plugins/skill-foundry/bin/skill-foundry mcp
```

## Building All 5 Plugins from Source

Each plugin ships its own `Cargo.lock` so tests and release builds run cleanly `--offline --locked` without polluting the plugin tree (use an external `CARGO_TARGET_DIR`):

### Pure-Rust & Single-Crate / Workspace Plugins (`agent-guardrails`, `skill-foundry`, `context-graph`)

```bash
# Single-crate plugins: agent-guardrails, skill-foundry
for p in agent-guardrails skill-foundry; do
  (
    cd "plugins/$p"
    CARGO_TARGET_DIR="/tmp/ai-infra-build/$p" CARGO_INCREMENTAL=0 cargo test --offline --locked
    CARGO_TARGET_DIR="/tmp/ai-infra-build/$p" CARGO_INCREMENTAL=0 cargo build --release --offline --locked
  )
done

# Multi-crate workspace plugin: context-graph
(
  cd plugins/context-graph
  CARGO_TARGET_DIR=/tmp/ai-infra-build/context-graph CARGO_INCREMENTAL=0 cargo test --workspace --offline --locked
  CARGO_TARGET_DIR=/tmp/ai-infra-build/context-graph CARGO_INCREMENTAL=0 cargo build --release --offline --locked --bin context-graph-engine
)
```

### WASM-Embedded Plugins (`graph-map`, `system-orchestration`)

Both `graph-map` and `system-orchestration` include precompiled WASM + JS assets on disk (`crates/graph-map-cli/web/assets/` and `web/assets/`) embedded automatically at compile time. Rebuilding the native binary works directly with `--offline --locked`; to recompile the frontend WASM bundles themselves (`rustup target add wasm32-unknown-unknown` + `wasm-bindgen-cli` `0.2.130`):

```bash
# graph-map (rebuild optional WASM bundle + workspace test/release binary)
(
  cd plugins/graph-map
  CARGO_TARGET_DIR=/tmp/ai-infra-build/graph-map cargo build -p graph-map-ui --target wasm32-unknown-unknown --release --offline --locked
  wasm-bindgen --target web --no-typescript --out-name graph_map_ui --out-dir crates/graph-map-cli/web/assets \
    /tmp/ai-infra-build/graph-map/wasm32-unknown-unknown/release/graph_map_ui.wasm
  CARGO_TARGET_DIR=/tmp/ai-infra-build/graph-map CARGO_INCREMENTAL=0 cargo test --workspace --offline --locked
  CARGO_TARGET_DIR=/tmp/ai-infra-build/graph-map CARGO_INCREMENTAL=0 cargo build --release --offline --locked --bin graph-map
)

# system-orchestration (rebuild optional WASM bundle + workspace test/release binary)
(
  cd plugins/system-orchestration
  CARGO_TARGET_DIR=/tmp/ai-infra-build/system-orchestration cargo build -p orch-ui --target wasm32-unknown-unknown --release --offline --locked
  wasm-bindgen --target web --no-typescript --out-name orch_ui --out-dir web/assets \
    /tmp/ai-infra-build/system-orchestration/wasm32-unknown-unknown/release/orch_ui.wasm
  CARGO_TARGET_DIR=/tmp/ai-infra-build/system-orchestration CARGO_INCREMENTAL=0 cargo test --workspace --offline --locked
  CARGO_TARGET_DIR=/tmp/ai-infra-build/system-orchestration CARGO_INCREMENTAL=0 cargo build --release --offline --locked --bin orch-engine
)
```
