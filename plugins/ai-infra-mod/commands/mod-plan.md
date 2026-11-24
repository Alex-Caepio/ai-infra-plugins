---
name: mod-plan
description: Discover system services & cross-service graphs, plan orchestration changes, manage session permission modes, diff Omnigraph branches, or trigger fleet/clash review.
argument-hint: "\"[system-task-or-subcommand]\""
---

Coordinate multi-service planning, orchestration execution, live permissions, and verification reviews for `$ARGUMENTS`.

**Important**: Pass `project: "<repo-root>"` unless `AI_INFRA_ROOT` is active.

1. Inspect services, service graphs, and contracts via `system_catalog`, `system_graph_status` (`check?`), `system_locate` (`task`), and `system_impact` (`service?`, `contract?`).
2. Build and execute an impact-scoped plan via `system_plan` (`task`, `change_id`) and `system_execute` (`plan`, `run_dir`), checking progress via `system_run_status` (`run_dir`).
3. Coordinate supervised sessions (`orch_session_start`, `orch_permission_mode`), Omnigraph atoms/diffs (`omnigraph_status`, `omnigraph_branches_diff`, `omnigraph_merge_keep`), and cross-reviewer gates (`orch_fleet_review`, `orch_clash_debate`, `orch_delivery_preview`).
