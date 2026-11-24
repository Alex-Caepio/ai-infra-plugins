---
name: ai-infra-autopilot
description: End-to-end AI Infrastructure engineering workflow chaining 13-language code graph impact, context & Omnigraph retrieval, system orchestration, guardrails validation, skill/cost auditing, and SkillCache trace compilation across all 6 engines (80 MCP tools).
---

# AI Infrastructure Autopilot

Use this flagship workflow for complex, multi-file, or cross-service tasks using all six subsystems (80 MCP tools) exposed by `ai-infra-mod`.

## Target Root Rule

Always pass an explicit `project` / `root` repository directory path to MCP tools unless `AI_INFRA_ROOT` is already bound via the Claude Desktop Extension. Call `mod_status` first to confirm active subsystems and configured root.

## End-to-End Pipeline

1. **Inspect Environment (`mod_status`)**:
   Call `mod_status` (`{ "project": "<repo-root>" }`) to verify all 6 backend subsystems (`graph-map`, `orch-engine`, `context-graph-engine`, `skill-foundry`, `guardrails`, `skillcache-engine`).
2. **Map Impact, File Blast & Memory Links (`graph-map`)**:
   - Call `graph_status` (`check: true`); run `graph_build` if unbuilt to index the 13-language native tree-sitter AST + SQLite IR.
   - Run `graph_change_impact` (`target: "work"` or `from`+`to`, optional `lod`/`min_weight`), `graph_file_impact` (`path`) for key modified files, `graph_memory_links` (`path` or `files`), `graph_test_reach`, and `graph_architecture` when crossing layers.
3. **Retrieve Context & Omnigraph Knowledge (`context-graph`)**:
   - Call `memory_retrieve` (`project: "<repo-root>"`, `task_id: "<TASK-ID>"`, `query: "<task>"`), `context_graph_query`, and `context_memory_links_for_file` (`path`, `previous_path?`).
   - Sync and inspect embedded Omnigraph knowledge via `omnigraph_sync_canonical`, `omnigraph_file_context`, and `context_omnigraph_atoms` (settling session branch conflicts with `omnigraph_branch_diff` / `context_omnigraph_merge_keep`).
4. **Plan & Orchestrate (`system-orchestration`)**:
   - For multi-service changes (`system.json` / `.orch/`), run `system_catalog` -> `system_graph_status` -> `system_impact` -> `system_plan` (`task`, `change_id`) -> `system_execute` (`plan`, `run_dir`) / `system_run_status`, or run supervised sessions/reviews via `orch_session_start`, `orch_permission_mode`, `orch_fleet_review`, `orch_clash_debate`, and `omnigraph_status` / `omnigraph_publish` / `omnigraph_branches_diff`.
   - Use `orch_working_memory_write` (`scope_id`, `key`, `text`) / `orch_working_memory_read` (`scope_id`) for working memory notes between turns.
5. **Verify Safety (`agent-guardrails`)**:
   - Run `guardrails_inspect_policy` (`root: "<repo-root>"`), check loop diagnostics (`guardrails_loop_status`), and validate proposed shell commands via `guardrails_check_command` before running risky commands.
6. **Checkpoint, Compile & Audit (`context-graph`, `skill-foundry`, `skillcache`)**:
   - Persist resolved work via `memory_checkpoint` (`task_id`, `goal?`) or `context_handoff_save` (`goal`, `summary`, `next_steps`).
   - Compile repeatable workflows into verified capsules via `skillcache_compile` / `skillcache_capsules_list` / `skillcache_requalify`, or run `skill_cost_audit` to review token efficiency.
