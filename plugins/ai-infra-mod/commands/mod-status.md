---
name: mod-status
description: Check status and active root configuration across all 6 AI Infrastructure Mod subsystems (graph-map, system-orchestration, context-graph, skill-foundry, agent-guardrails, skillcache).
argument-hint: "[project-path]"
---

Inspect the unified status of `ai-infra-mod` for `$ARGUMENTS` (pass `project: "<repo-root>"` unless `AI_INFRA_ROOT` is configured).

1. Call `mod_status` with `{ "project": "<repo-root>" }` to verify all 6 subsystems (`80` MCP tools total).
2. Also check `graph_status` (`project: "<repo-root>"`, `check: true`), `system_graph_status`, `memory_status`, `guardrails_inspect_policy` (`root: "<repo-root>"`), and `skillcache_status` (`project: "<repo-root>"`).
3. Report subsystem health, configured root, graph node/edge counts, active memory/Omnigraph state, guardrails preset, and SkillCache sandbox readiness.
