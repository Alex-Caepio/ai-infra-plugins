---
name: mod-plan
description: Discover system services, plan cross-service orchestration changes, inspect execution status, or trigger fleet/clash review.
argument-hint: "\"[system-task-or-subcommand]\""
---

Coordinate multi-service planning, orchestration execution, and verification reviews for `$ARGUMENTS`.

**Important**: Pass `project: "<repo-root>"` unless `AI_INFRA_ROOT` is active.

1. Inspect services and contracts via `system_catalog`, `system_locate` (`task`), and `system_impact` (`service?`, `contract?`).
2. Build and execute an impact-scoped plan via `system_plan` (`task`, `change_id`) and `system_execute` (`plan`, `run_dir`), checking progress via `system_run_status` (`run_dir`).
3. Coordinate supervised sessions or cross-reviewer gates via `orch_session_start`, `orch_fleet_review`, `orch_clash_debate`, and `orch_delivery_preview`.
