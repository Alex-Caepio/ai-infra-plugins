---
name: mod-status
description: Check status and active root configuration across all 5 AI Infrastructure Mod subsystems (graph-map, system-orchestration, context-graph, skill-foundry, agent-guardrails).
argument-hint: "[project-path]"
---

Inspect the unified status of `ai-infra-mod` for `$ARGUMENTS` (pass `project: "<repo-root>"` unless `AI_INFRA_ROOT` is configured).

1. Call `mod_status` with `{ "project": "<repo-root>" }`.
2. Also check `graph_status` (`project: "<repo-root>"`, `check: true`), `memory_status` (`project: "<repo-root>"`), and `guardrails_inspect_policy` (`root: "<repo-root>"`).
3. Report subsystem health, configured root, graph node/edge counts, active memory banks/tasks, and guardrails preset.
