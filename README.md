# ai-infra-plugins

Standalone, dual-compatible **Claude Code (`2.1.278`+)** and **Codex CLI (`0.169.0`)** plugin marketplace (plus unified `.mcpb` Claude Desktop Extension) extracted from `AI-Infrastructure` (`PR #98–#112`).

Every plugin in `./plugins/` is self-contained (zero path/runtime dependencies back to `AI-Infrastructure`, zero `target/` or root `CLAUDE.md`), ships stripped Linux x86_64 Rust binaries in `bin/`, exposes a stdio JSON-RPC `2025-06-18` MCP server (`<bin> mcp`), and provides dual `.claude-plugin/` + `.codex-plugin/` manifests, `skills/`, `commands/`, and (where applicable) `hooks/`.

## Plugins Summary

| Plugin | Version | Binary (`bin/`, stripped ELF) | Verified Tests | MCP Tools | Skills / Commands / Hooks | Highlights |
| :--- | :---: | :--- | :---: | :---: | :---: | :--- |
| **`graph-map`** | `0.2.0` | `graph-map` (`42M`) | `615/615` | `16` | `4` / `3` / — | 13-language native AST graph engine (`tree-sitter` IR, zero external Python required), Cargo/git/PR blast-radius, layers/cycles/test-reach, Claude/Codex/Harness run audit, embedded WASM Canvas2D/SVG UI (`1.7M` `graph_map_ui_bg.wasm`), CSRF-protected HTTP/SSE API. |
| **`system-orchestration`** | `0.1.0` | `orch-engine` (`38M`) | `958/959` (`1` gated) | `25` | `6` / `6` / `3` | Multi-repo `system` plan/execute, embedded **Omnigraph** (`system_graph` + interactive explorer), live SSE stream & permission prompts/rules, session scheduler & inbox, fleet review, clash debate, SDD delivery previews, SQLite `RunWorkingMemory`, `attach`, embedded WASM UI (`6.5M` `orch_ui_bg.wasm`). |
| **`context-graph`** | `0.1.0` | `context-graph-engine` (`13M`) | `166/166` | `17` | `6` / `8` / `6` (`3` matchers) | Project Brain, Memory Bank, SQLite FTS5 retrieval, unified N-hop `graph_nodes`/`graph_edges`, **Omnigraph** atoms & branch `merge-keep` integration, session handoffs, SHA-256 `context-graph-bundle-v1` export/import, memory hooks (`hooks/hooks.json` auto-loaded). |
| **`skill-foundry`** | `0.1.0` | `skill-foundry` (`7.1M`) | `22/22` | `8` | `4` / `4` / — | Skill scaffold/validate/package, SQLite WAL `QueryMemoCache` + Gate-1 prefilter, ROI-tracked optimization, SkillCache multi-arm economics `benchmark` (`model_calls`, `deopt_rate`, `list_price_usd`, `compile_usd`, `breakeven_reuses`), SHA-256 `cl100k` budget linter, `8+4` `vet-skill`, dual-host BTE `cost-audit`, hardened standalone HTML/OOXML review UI. |
| **`agent-guardrails`** | `0.1.0` | `guardrails` (`3.0M`) | `18/18` | `4` | `2` / `2` / `5` | POSIX AST `bash-guard` (`rg --pre`/dangerous commands), `$TMPDIR` command & error-signature-scoped `edit-loop` breakers (`PostToolUse` `Bash\|Write\|Edit\|MultiEdit\|NotebookEdit`), settled naming checks, `/tmp` `flock` + `O_NOFOLLOW` write locks, 7 presets + custom `.agent-guardrails/policy.json`. |
| **`skillcache`** | `0.1.0` | `skillcache-engine` (`9.1M`) | `136/136` | `9` | `3` / `4` / — | Standalone SkillCache trace compiler (`active`/`hybrid`/`textual` capsules), static Gate-1 dataflow/secret validator, network-isolated Gate-2/Gate-3 `bwrap`/`--unshare-net` sandbox executor, private SQLite capsule lifecycle (`invalidate`, `requalify`, `retire`), multicall `skillcache-run`/`skillcache-tool`. |
| **`ai-infra-mod`** *(Unified Suite + `.mcpb`)* | `0.2.0` | `ai-infra-mod` (`740K`) + `6` sibling ELFs in `bin/` | `6/6` | `80` (`79` + `mod_status`) | `7` / `12` (`mod-*`) / `9` | Single multiplexed 80-tool MCP router, 9-role unified hook router, CLI suite router across all 6 engines, `AI_INFRA_ROOT` + `AI_INFRA_RESTRICT_ROOT=1` containment, and Claude Desktop `.mcpb` v0.2.0 bundle (`dist/ai-infra-mod.mcpb`, `38.3 MiB`). |
| **Marketplace Total (`7` plugins)** | — | **7 standalone + unified binaries** | **`1,921` passed (`1` ignored)** | **`79` modular / `80` suite** | **`25`+`7` / `27`+`12` / `14`+`9`** | **Plugin-dir `cwd` guard & containment across all servers; Claude + Codex + `.mcpb` validated.** |

> Подробный архитектурный анализ и матрица декуплинга (`PR #98–#112`, на русском): [`ANALYSIS.md`](./ANALYSIS.md).

## Standalone GitHub Repositories

- Marketplace: <https://github.com/Alex-Caepio/ai-infra-plugins>
- `ai-infra-mod` *(all-in-one suite + `.mcpb`)*: <https://github.com/Alex-Caepio/ai-infra-mod>
- `graph-map`: <https://github.com/Alex-Caepio/graph-map>
- `system-orchestration`: <https://github.com/Alex-Caepio/system-orchestration>
- `context-graph`: <https://github.com/Alex-Caepio/context-graph>
- `skill-foundry`: <https://github.com/Alex-Caepio/skill-foundry>
- `agent-guardrails`: <https://github.com/Alex-Caepio/agent-guardrails>
- `skillcache`: <https://github.com/Alex-Caepio/skillcache>

## Claude Code Installation & Validation

Verified against `claude` (`2.1.278`+):

```bash
# Register the GitHub marketplace and install the unified suite or individual plugins
claude plugin marketplace add Alex-Caepio/ai-infra-plugins
claude plugin install ai-infra-mod@ai-infra-plugins --scope user

# Or install individual modular plugins
claude plugin install graph-map@ai-infra-plugins --scope user
claude plugin install system-orchestration@ai-infra-plugins --scope user
claude plugin install context-graph@ai-infra-plugins --scope user
claude plugin install skill-foundry@ai-infra-plugins --scope user
claude plugin install agent-guardrails@ai-infra-plugins --scope user
claude plugin install skillcache@ai-infra-plugins --scope user

# Validate / load from a local checkout
claude plugin validate --strict ./.claude-plugin/marketplace.json
claude plugin validate --strict ./plugins/ai-infra-mod
claude --plugin-dir ./plugins/ai-infra-mod
```

> **Note on `context-graph` hooks manifest**: Claude Code automatically loads `hooks/hooks.json`, `commands/`, `skills/`, and `.mcp.json` when present at standard plugin paths; `.claude-plugin/plugin.json` intentionally omits redundant pointers to those default paths so `claude plugin list` loads hooks without duplicate-hook errors.

## Claude Desktop Extension (`.mcpb`)

Install the prebuilt Desktop Extension (`0.2.0`, `38.3 MiB` / `40,150,506` bytes) from [`plugins/ai-infra-mod/dist/ai-infra-mod.mcpb`](./plugins/ai-infra-mod/dist/ai-infra-mod.mcpb) via **Claude Desktop -> Settings -> Extensions**:

- Configures `server.type: "binary"` (`${__dirname}/bin/ai-infra-mod mcp`) with all 6 sibling ELFs (`graph-map`, `orch-engine`, `context-graph-engine`, `skill-foundry`, `guardrails`, `skillcache-engine`) bundled in `bin/`.
- Prompts once for `user_config.projectDir`, exporting `AI_INFRA_ROOT` and `AI_INFRA_RESTRICT_ROOT=1` so all 80 tools canonicalize paths inside `AI_INFRA_ROOT` and reject path-escape attempts (`isError: true`).

## Codex CLI Installation & Validation

Verified against `codex` (`0.169.0`) (Codex CLI installs plugins via marketplace rather than a directory flag):

```bash
# Register the GitHub (or local checkout) marketplace (.agents/plugins/marketplace.json)
codex plugin marketplace add https://github.com/Alex-Caepio/ai-infra-plugins.git
# Or from a cloned local directory:
codex plugin marketplace add .

# Install the unified suite or individual plugins and verify loaded state
codex plugin add ai-infra-mod@ai-infra-plugins
codex plugin add graph-map@ai-infra-plugins
codex plugin add system-orchestration@ai-infra-plugins
codex plugin add context-graph@ai-infra-plugins
codex plugin add skill-foundry@ai-infra-plugins
codex plugin add agent-guardrails@ai-infra-plugins
codex plugin add skillcache@ai-infra-plugins
codex plugin list --json
```

**Important note on Codex MCP `cwd` safety**: Every `.codex-mcp.json` runs `"command": "./bin/<binary>"` with `"cwd": "."` (the cached plugin directory). Because that directory may sit beneath an ancestor `.git` / `.claude` / `.agents`, all 7 MCP servers detect plugin/cache `cwd` BEFORE walking ancestors and reject omitted or `"."`/`"./."` target arguments (`isError: true`) rather than silently mutating `~/` or the plugin cache (unless `AI_INFRA_ROOT` or MCP `roots/list` is provided). Always pass explicit `"project"` / `"root"` / `"path"` pointing to your target repository when calling MCP tools.

## Direct CLI & MCP Smoke Checks

```bash
./plugins/ai-infra-mod/bin/ai-infra-mod --help
./plugins/ai-infra-mod/bin/ai-infra-mod status --project . --json
./plugins/graph-map/bin/graph-map --help
./plugins/system-orchestration/bin/orch-engine --help
./plugins/context-graph/bin/context-graph-engine --help
./plugins/skill-foundry/bin/skill-foundry --help
./plugins/agent-guardrails/bin/guardrails --help
./plugins/skillcache/bin/skillcache-engine --help

# Stdio MCP JSON-RPC 2.0 handshake + tool discovery (80 tools on ai-infra-mod)
printf '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"check","version":"0.1"}}}\n{"jsonrpc":"2.0","method":"notifications/initialized"}\n{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}\n' | ./plugins/ai-infra-mod/bin/ai-infra-mod mcp
```

## Building All 7 Plugins from Source

Each plugin ships its own `Cargo.lock` so tests and release builds run cleanly `--offline --locked` without polluting the plugin tree (use an external `CARGO_TARGET_DIR`):

### Single-Crate & Workspace Plugins (`agent-guardrails`, `skill-foundry`, `skillcache`, `ai-infra-mod`, `context-graph`)

```bash
# Single-crate plugins: agent-guardrails, skill-foundry, skillcache, ai-infra-mod
for p in agent-guardrails skill-foundry skillcache ai-infra-mod; do
  (
    cd "plugins/$p"
    CARGO_TARGET_DIR="../../.build-local/$p" CARGO_INCREMENTAL=0 cargo test --offline --locked
    CARGO_TARGET_DIR="../../.build-local/$p" CARGO_INCREMENTAL=0 cargo build --release --offline --locked
  )
done

# Multi-crate workspace plugin: context-graph
(
  cd plugins/context-graph
  CARGO_TARGET_DIR=../../.build-local/context-graph CARGO_INCREMENTAL=0 cargo test --workspace --offline --locked
  CARGO_TARGET_DIR=../../.build-local/context-graph CARGO_INCREMENTAL=0 cargo build --release --offline --locked --bin context-graph-engine
)
```

### WASM-Embedded Plugins (`graph-map`, `system-orchestration`)

Both `graph-map` and `system-orchestration` include precompiled WASM + JS assets on disk (`crates/graph-map-cli/web/assets/` and `web/assets/`) embedded automatically at compile time. Rebuilding the native binary works directly with `--offline --locked`; to recompile the frontend WASM bundles themselves (`rustup target add wasm32-unknown-unknown` + `wasm-bindgen-cli` `0.2.130`):

```bash
# graph-map (rebuild optional WASM bundle + workspace test/release binary)
(
  cd plugins/graph-map
  CARGO_TARGET_DIR=../../.build-local/graph-map cargo build -p graph-map-ui --target wasm32-unknown-unknown --release --offline --locked
  wasm-bindgen --target web --no-typescript --out-name graph_map_ui --out-dir crates/graph-map-cli/web/assets \
    ../../.build-local/graph-map/wasm32-unknown-unknown/release/graph_map_ui.wasm
  CARGO_TARGET_DIR=../../.build-local/graph-map CARGO_INCREMENTAL=0 cargo test --workspace --offline --locked
  CARGO_TARGET_DIR=../../.build-local/graph-map CARGO_INCREMENTAL=0 cargo build --release --offline --locked --bin graph-map
)

# system-orchestration (rebuild optional WASM bundle + workspace test/release binary)
(
  cd plugins/system-orchestration
  CARGO_TARGET_DIR=../../.build-local/system-orchestration cargo build -p orch-ui --target wasm32-unknown-unknown --release --offline --locked
  wasm-bindgen --target web --no-typescript --out-name orch_ui --out-dir web/assets \
    ../../.build-local/system-orchestration/wasm32-unknown-unknown/release/orch_ui.wasm
  CARGO_TARGET_DIR=../../.build-local/system-orchestration CARGO_INCREMENTAL=0 cargo test --workspace --offline --locked
  CARGO_TARGET_DIR=../../.build-local/system-orchestration CARGO_INCREMENTAL=0 cargo build --release --offline --locked --bin orch-engine
)
```
