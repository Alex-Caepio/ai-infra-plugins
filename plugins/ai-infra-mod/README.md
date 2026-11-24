# AI Infrastructure Mod (`ai-infra-mod`)

Unified dual-compatible suite combining all six **AI Infrastructure** subsystems into a single native Rust CLI, unified lifecycle hook router, **80-tool stdio JSON-RPC `2025-06-18` MCP server**, Claude Code plugin, Codex CLI plugin, and `.mcpb` Claude Desktop Extension.

## Included Subsystems

| Subsystem | Binary (`bin/`) | Tools | Capabilities |
| --- | --- | --- | --- |
| **Graph Map** | `graph-map` | `16` | Native 13-language tree-sitter AST + SQLite IR, LOD slices, `A..B` ranges, per-file blast, linked memory, layers, cycles, scenarios, test reach, hotspots, run audits, Canvas2D WASM UI + SSE |
| **System Orchestration** | `orch-engine` | `25` | Multi-service planning (`system.json`), cross-service dependency bridges (`system_graph`), supervised sessions, live permission modes, Omnigraph server/branch atoms, fleet/clash review, WASM UI |
| **Context Graph** | `context-graph-engine` | `17` | Project Brain, Memory Bank, unified 8-relation SQLite graph, embedded offline Omnigraph (`knowledge/canonical`, `SymbolRef`), graph-to-memory links, IR-epoch snapshots, `context-graph-bundle-v1` export/import |
| **Skill Foundry** | `skill-foundry` | `8` | Skill scaffolding, validation, packaging, trigger evals, MT19937 optimization, 8+4 security vetting, BTE cost audit |
| **Agent Guardrails** | `guardrails` | `4` | POSIX AST shell inspection (`rg --pre`, `core.hooksPath`, `read_only` alignment), 7 stack presets, file-naming validation, normalized bash/edit loop breakers, subagent write locks |
| **SkillCache** | `skillcache-engine` | `9` | Gate 1 static checks, Gate 2 sandboxed (`bubblewrap --unshare-net` / `sandbox-exec`) `--check`, Gate 3 replay determinism, trace ingest, capsule lifecycle, warm execution |
| **Unified Mod Router** | `ai-infra-mod` | `1` (`mod_status`) | Aggregates all 79 backend MCP tools + `mod_status` (**80 total**, prefixing `context_omnigraph_atoms` & `context_omnigraph_merge_keep`), routes unified hooks, dispatches CLI subcommands |

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

You can also inspect or toggle plugins interactively in Codex TUI via `/plugins` or `~/.codex/config.toml` (`[plugins."ai-infra-mod@ai-infra-plugins"] enabled = true`).

### Claude Desktop (`.mcpb` Extension)

Install `dist/ai-infra-mod.mcpb` via **Claude Desktop -> Settings -> Extensions** and select your workspace folder (`AI_INFRA_ROOT`, with `AI_INFRA_RESTRICT_ROOT=1` enforced by default).

Source repositories:
- Suite repository: [Alex-Caepio/ai-infra-mod](https://github.com/Alex-Caepio/ai-infra-mod)
- Marketplace catalog: [Alex-Caepio/ai-infra-plugins](https://github.com/Alex-Caepio/ai-infra-plugins)

## Unified CLI & Hooks

```bash
./bin/ai-infra-mod status --project /path/to/repo --json
./bin/ai-infra-mod mcp
./bin/ai-infra-mod graph --project /path/to/repo build
./bin/ai-infra-mod graph --project /path/to/repo serve --host 127.0.0.1 --port 7840
./bin/ai-infra-mod orch --project /path/to/repo serve
./bin/ai-infra-mod context --project /path/to/repo omnigraph sync-canonical --root /path/to/repo
./bin/ai-infra-mod skill validate ./skills/ai-infra-autopilot
./bin/ai-infra-mod guard status --root /path/to/repo
./bin/ai-infra-mod skillcache status --project /path/to/repo
./bin/ai-infra-mod skillcache compile --project /path/to/repo --candidate /path/to/candidate.json
```

Unified lifecycle hook router roles (`ai-infra-mod hook [claude|codex|cursor] <role>`):
`session-start`, `user-prompt`, `pre-bash`, `pre-write-edit`, `pre-subagent`, `post-write-edit` (Claude `PostToolUse` matches `Bash|Write|Edit|MultiEdit|NotebookEdit`), `post-bash`, `post-subagent`, `subagent-stop`, `stop`.

## MCP Tools (80)

| Subsystem | Tools |
| --- | --- |
| **Router (`1`)** | `mod_status` |
| **Graph Map (`16`)** | `graph_status`, `graph_build`, `graph_find`, `graph_neighbors`, `graph_link`, `graph_path`, `graph_shared`, `graph_cycle`, `graph_change_impact`, `graph_architecture`, `graph_scenario`, `graph_test_reach`, `graph_history_hotspots`, `graph_run_audit`, `graph_file_impact`, `graph_memory_links` |
| **System Orchestration (`25`)** | `system_init`, `system_catalog`, `system_locate`, `system_impact`, `system_plan`, `system_execute`, `system_resume`, `system_run_status`, `system_graph_status`, `orch_session_start`, `orch_session_send`, `orch_session_status`, `orch_session_cancel`, `orch_permission_mode`, `orch_context_draft`, `orch_fleet_review`, `orch_clash_debate`, `orch_delivery_preview`, `orch_working_memory_read`, `orch_working_memory_write`, `omnigraph_status`, `omnigraph_atoms`, `omnigraph_publish`, `omnigraph_branches_diff`, `omnigraph_merge_keep` |
| **Context Graph (`17`)** | `memory_status`, `memory_retrieve`, `memory_source_fingerprints`, `memory_checkpoint`, `memory_record_result`, `context_graph_query`, `context_graph_links`, `context_export_bundle`, `context_import_bundle`, `context_handoff_save`, `context_handoff_load`, `omnigraph_sync_canonical`, `omnigraph_file_context`, `context_omnigraph_atoms`, `omnigraph_branch_diff`, `context_omnigraph_merge_keep`, `context_memory_links_for_file` |
| **Skill Foundry (`8`)** | `skill_scaffold`, `skill_validate`, `skill_package`, `skill_eval_triggers`, `skill_optimize_description`, `skill_budget_check`, `skill_vet`, `skill_cost_audit` |
| **Agent Guardrails (`4`)** | `guardrails_inspect_policy`, `guardrails_init_policy`, `guardrails_check_command`, `guardrails_loop_status` |
| **SkillCache (`9`)** | `skillcache_status`, `skillcache_gate1`, `skillcache_traces`, `skillcache_compile`, `skillcache_capsules_list`, `skillcache_capsule_inspect`, `skillcache_invalidate`, `skillcache_requalify`, `skillcache_retire` |

## Target Root & Security

- **Desktop Extension (`.mcpb`)**: Sets `AI_INFRA_ROOT` + `AI_INFRA_RESTRICT_ROOT=1`; workspace path arguments are canonicalized inside `AI_INFRA_ROOT` (while non-path parameters such as Omnigraph git/branch refs are left untouched).
- **Claude Code / Codex CLI (`cwd: "."`)**: Pass explicit `project` / `root` / `path` arguments targeting your repository root; calls omitting paths while `cwd` is inside a plugin or cache directory fail closed (`isError: true`).

## Skills & Non-Colliding Slash Commands

- **Skills (`7` in `skills/`)**: `ai-infra-autopilot`, `code-graph-intelligence`, `context-memory-transfer`, `multi-agent-orchestration`, `skill-engineering-suite`, `execution-safety-guardrails`, `skillcache-compiler`
- **Slash Commands (`12` in `commands/`)**: `/mod-status`, `/mod-autopilot`, `/mod-map`, `/mod-plan`, `/mod-context`, `/mod-omnigraph`, `/mod-export`, `/mod-import`, `/mod-guard`, `/mod-vet`, `/mod-cost`, `/mod-skillcache`

## License

Apache-2.0 — see [LICENSE](LICENSE).
