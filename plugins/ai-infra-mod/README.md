# AI Infrastructure Mod (`ai-infra-mod`)

Unified dual-compatible suite combining all five **AI Infrastructure** subsystems into a single native Rust CLI, unified lifecycle hook router, **56-tool stdio JSON-RPC `2025-06-18` MCP server**, Claude Code plugin, Codex CLI plugin, and `.mcpb` Claude Desktop Extension.

## Included Subsystems

| Subsystem | Binary (`bin/`) | Capabilities |
| --- | --- | --- |
| **Graph Map** | `graph-map` | Dependency map, change impact, test reach, layers, cycles, scenarios, hotspots, run audits, WASM UI |
| **System Orchestration** | `orch-engine` | Multi-service planning (`system.json`), supervised sessions, dispatch mailboxes, fleet/clash review, WASM UI |
| **Context Graph** | `context-graph-engine` | Project Brain, Memory Bank, unified 8-relation SQLite context graph, `context-graph-bundle-v1` export/import |
| **Skill Foundry** | `skill-foundry` | Skill scaffolding, validation, packaging, trigger evals, MT19937 optimization, security vetting, BTE cost audit |
| **Agent Guardrails** | `guardrails` | POSIX AST shell inspection, stack presets, file-naming validation, loop breakers, subagent write locks |
| **Unified Mod Router** | `ai-infra-mod` | Aggregates 55 backend MCP tools + `mod_status` (56 total), routes unified hooks, dispatches CLI subcommands |

## Installation

### Claude Code

```bash
claude plugin marketplace add Alex-Caepio/ai-infra-plugins
claude plugin install ai-infra-mod@ai-infra-plugins
```

### Codex CLI

```bash
codex plugin marketplace add https://github.com/Alex-Caepio/ai-infra-plugins.git
codex plugin add ai-infra-mod@ai-infra-plugins
```

### Claude Desktop (`.mcpb` Extension)

Install `dist/ai-infra-mod.mcpb` via **Claude Desktop -> Settings -> Extensions** and select your workspace folder (`AI_INFRA_ROOT`, with `AI_INFRA_RESTRICT_ROOT=1` enforced by default).

Source repositories:
- Suite repository: [Alex-Caepio/ai-infra-mod](https://github.com/Alex-Caepio/ai-infra-mod)
- Marketplace catalog: [Alex-Caepio/ai-infra-plugins](https://github.com/Alex-Caepio/ai-infra-plugins)

## Unified CLI & Hooks

```bash
./bin/ai-infra-mod status --project /path/to/repo --json
./bin/ai-infra-mod mcp
./bin/ai-infra-mod graph --project /path/to/repo serve --host 127.0.0.1 --port 7840
./bin/ai-infra-mod orch --project /path/to/repo serve
./bin/ai-infra-mod context --project /path/to/repo graph sync
./bin/ai-infra-mod skill validate ./skills/ai-infra-autopilot
./bin/ai-infra-mod guard status --root /path/to/repo
```

Unified lifecycle hook router roles (`ai-infra-mod hook [claude|codex|cursor] <role>`):
`session-start`, `user-prompt`, `pre-bash`, `pre-write-edit`, `pre-subagent`, `post-write-edit`, `post-subagent`, `subagent-stop`, `stop`.

## Target Root & Security

- **Desktop Extension (`.mcpb`)**: Sets `AI_INFRA_ROOT` + `AI_INFRA_RESTRICT_ROOT=1`; all path arguments are canonicalized inside `AI_INFRA_ROOT` and escapes are rejected.
- **Claude Code / Codex CLI (`cwd: "."`)**: Pass explicit `project` / `root` / `path` arguments targeting your repository root; calls omitting paths while `cwd` is inside a plugin or cache directory fail closed (`isError: true`).

## Skills & Non-Colliding Slash Commands

- **Skills (`skills/`)**: `ai-infra-autopilot`, `code-graph-intelligence`, `context-memory-transfer`, `multi-agent-orchestration`, `skill-engineering-suite`, `execution-safety-guardrails`
- **Slash Commands (`commands/`)**: `/mod-status`, `/mod-autopilot`, `/mod-map`, `/mod-plan`, `/mod-context`, `/mod-export`, `/mod-import`, `/mod-guard`, `/mod-vet`, `/mod-cost`

## License

Apache-2.0 — see [LICENSE](LICENSE).
