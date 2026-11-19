---
name: mod-autopilot
description: Run the end-to-end AI Infrastructure Mod workflow (graph impact, context retrieval, orchestration planning, guardrails verification, and checkpoint).
argument-hint: "\"<task description or target>\""
---

Execute the flagship end-to-end engineering workflow for `$ARGUMENTS` using the `ai-infra-autopilot` skill.

**Important**: Pass explicit `project` / `root` repository paths unless `AI_INFRA_ROOT` is bound.

1. Verify subsystem readiness via `mod_status`.
2. Analyze blast radius & test coverage via `graph_change_impact` and `graph_test_reach`.
3. Pull relevant decisions, Brain records, and handoffs via `memory_retrieve` (`task_id`, `query`) and `context_graph_query` (`node`, `kinds`).
4. For multi-service changes, inspect `system_catalog` / `system_impact` and plan/execute via `system_plan` (`task`, `change_id`) + `system_execute` (`plan`, `run_dir`); validate shell steps via `guardrails_check_command`.
5. Save continuity state via `memory_checkpoint` (`task_id`, `goal?`) or `context_handoff_save` (`goal`, `summary`, `next_steps`) when complete.
