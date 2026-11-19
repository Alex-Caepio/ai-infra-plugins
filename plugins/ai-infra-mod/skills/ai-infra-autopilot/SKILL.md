---
name: ai-infra-autopilot
description: End-to-end AI Infrastructure engineering workflow chaining graph impact, context graph retrieval, system orchestration, guardrails validation, and skill/session auditing.
---

# AI Infrastructure Autopilot

Use this flagship workflow for complex, multi-file, or cross-service tasks using all five subsystems exposed by `ai-infra-mod`.

## Target Root Rule

Always pass an explicit `project` / `root` repository directory path to MCP tools unless `AI_INFRA_ROOT` is already bound via the Claude Desktop Extension. Call `mod_status` first to confirm active subsystems and configured root.

## End-to-End Pipeline

1. **Inspect Environment (`mod_status`)**:
   Call `mod_status` (`{ "project": "<repo-root>" }`) to verify all 5 backend subsystems (`graph-map`, `orch-engine`, `context-graph-engine`, `skill-foundry`, `guardrails`).
2. **Map Impact & Test Reach (`graph-map`)**:
   - Call `graph_status`, then `graph_change_impact` (`target: "work"`) and `graph_test_reach` (`target: "work"`), plus `graph_architecture` if touching module/layer boundaries.
3. **Retrieve Context & Memory (`context-graph`)**:
   - Call `memory_retrieve` (`project: "<repo-root>"`, `task_id: "<TASK-ID>"`, `query: "<task>"`) and `context_graph_query` (`project: "<repo-root>"`, `node: "<id>"`, `kinds: [...]`) to surface relevant Brain tasks, Memory Bank chunks, handoffs, and superseding decisions.
   - Check `memory_source_fingerprints` (`project: "<repo-root>"`, `sources: ["<path>"]`) on stale citations.
4. **Plan & Orchestrate (`system-orchestration`)**:
   - For multi-service changes (`system.json` / `.orch/`), run `system_catalog` -> `system_impact` -> `system_plan` (`task`, `change_id`) -> `system_execute` (`plan`, `run_dir`) / `system_run_status` (`run_dir`), or run managed sessions/reviews via `orch_session_start`, `orch_session_status`, `orch_fleet_review`, and `orch_clash_debate`.
   - Use `orch_working_memory_write` (`scope_id`, `key`, `text`) / `orch_working_memory_read` (`scope_id`) for working memory notes between turns.
5. **Verify Safety (`agent-guardrails`)**:
   - Run `guardrails_inspect_policy` (`root: "<repo-root>"`) and validate proposed shell commands via `guardrails_check_command` before running risky commands.
6. **Checkpoint & Audit (`context-graph` + `skill-foundry`)**:
   - Persist resolved work via `memory_checkpoint` (`task_id`, `goal?`) or `context_handoff_save` (`goal`, `summary`, `next_steps`), or run `skill_cost_audit` to review token efficiency.
