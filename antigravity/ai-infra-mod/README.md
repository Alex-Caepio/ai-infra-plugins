# AI Infrastructure Mod Antigravity Extension (`ai-infra-mod`)

Flagship unified Antigravity Extension (`v0.2.0`, enabled by default) bundling all 6 **AI Infrastructure** subsystems via an **80-tool stdio MCP multiplexer**, native Antigravity lifecycle hooks, **19** skills, **6** specialized subagents, and always-on workspace rules.

## Included Capabilities

- **MCP Multiplexer (`mcp_config.json`, `80` tools)**: `mod_status` (`1`) + `graph-map` (`16`) + `orch-engine` (`25`) + `context-graph-engine` (`17`, with `context_omnigraph_atoms` & `context_omnigraph_merge_keep` disambiguated) + `skill-foundry` (`8`) + `guardrails` (`4`) + `skillcache-engine` (`9`).
- **Native Antigravity Hooks (`hooks.json`)**: `SessionStart`, `PreInvocation`, `PreToolUse` (`run_command`, file editors, `invoke_subagent`), `PostToolUse` (`run_command`/file editors, `invoke_subagent`), and `Stop`.
- **Skills (`19` in `skills/`)**: `ai-infra-autopilot`, `code-graph-intelligence`, `context-memory-transfer`, `multi-agent-orchestration`, `skill-engineering-suite`, `execution-safety-guardrails`, `skillcache-compiler`, plus `mod-status`, `mod-autopilot`, `mod-map`, `mod-plan`, `mod-context`, `mod-omnigraph`, `mod-export`, `mod-import`, `mod-guard`, `mod-vet`, `mod-cost`, `mod-skillcache`.
- **Subagents (`6` in `agents/`)**: `mod-graph-analyst`, `mod-orchestrator`, `mod-context-curator`, `mod-skill-engineer`, `mod-safety-auditor`, `mod-skillcache-compiler`.
- **Always-On Rule**: `rules/AGENTS.md`.

## License

Apache-2.0 — see [LICENSE](LICENSE).
