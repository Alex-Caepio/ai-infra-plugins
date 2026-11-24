---
name: mod-autopilot
description: Run the end-to-end AI Infrastructure Mod workflow across all 6 engines (graph impact, context & Omnigraph retrieval, orchestration planning, guardrails verification, SkillCache compilation, and checkpoint).
argument-hint: "\"<task description or target>\""
---

Execute the flagship end-to-end engineering workflow for `$ARGUMENTS` using the `ai-infra-autopilot` skill.

**Important**: Pass explicit `project` / `root` repository paths unless `AI_INFRA_ROOT` is bound.

1. Verify all 6 subsystems via `mod_status`.
2. Analyze blast radius, per-file impact, linked memory, & test reach via `graph_change_impact`, `graph_file_impact`, `graph_memory_links`, and `graph_test_reach`.
3. Pull relevant decisions, Brain records, and Omnigraph file context via `memory_retrieve` (`task_id`, `query`), `context_graph_query` (`node`, `kinds`), `omnigraph_file_context`, and `context_memory_links_for_file`.
4. For multi-service changes, inspect `system_catalog`, `system_graph_status`, and `system_impact`, plan/execute via `system_plan` + `system_execute`, or manage supervised sessions (`orch_session_start`, `orch_permission_mode`, `omnigraph_status`, `omnigraph_publish`); validate shell steps via `guardrails_check_command`.
5. Save continuity state via `memory_checkpoint` / `context_handoff_save`, compile recurring workflows with `skillcache_compile` / `skillcache_capsules_list`, or audit efficiency via `skill_cost_audit`.
