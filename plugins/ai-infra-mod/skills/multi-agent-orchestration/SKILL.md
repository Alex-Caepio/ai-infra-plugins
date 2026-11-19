---
name: multi-agent-orchestration
description: Plan multi-service system changes, coordinate agent sessions and fleet/clash reviews, inspect delivery gates, and manage SQLite working memory via system_* and orch_* tools.
---

# Multi-Agent & System Orchestration (`system-orchestration`)

Use the `system_*` and `orch_*` tools in `ai-infra-mod` to coordinate multi-service changes, supervised sessions, fleet/clash verification, and working memory.

## Target Root Rule

Pass explicit `project` / `root` repository paths (unless `AI_INFRA_ROOT` is configured via Desktop Extension).

## Workflows

1. **System Discovery & Multi-Service Planning**:
   - Initialize or inspect services and contracts via `system_init` (`name?`), `system_catalog` (`system?`, `allow_root?`), `system_locate` (`task`), and `system_impact` (`service?`, `contract?`, `impact_depth?`).
   - Build and run an impact-scoped plan via `system_plan` (`task`, `change_id`, `service?`, `contract?`, `context_budget?`, `impact_depth?`, `output?`) -> `system_execute` (`plan`, `run_dir`) -> `system_run_status` (`run_dir`) or `system_resume` (`run_dir`).
2. **Supervised Agent Sessions & Delivery**:
   - Start and manage sessions via `orch_session_start` (`prompt`, `provider?`, `workflow?`, `mode?`, `workspace?`), `orch_session_send` (`session_id`, `prompt`), `orch_session_status` (`session_id?`, `after?`), and `orch_session_cancel` (`session_id`).
   - Draft follow-up context via `orch_context_draft` (`source_session_id`, `destination_session_id?`) and preview commits/merges via `orch_delivery_preview` (`session_id`, `action?`).
3. **Fleet Review & Challenger Clash**:
   - Trigger multi-lens parallel review via `orch_fleet_review` (`thread?`, `lenses?`, `scope?`, `approve?`, `reject?`).
   - Launch adversarial challenger verification via `orch_clash_debate` (`prompt?`, `provider?`, `challenger?`, `rounds?`).
4. **SQLite Working Memory**:
   - Read and persist structured working notes via `orch_working_memory_read` (`scope_id`, `include_resolved?`, `run_dir?`) and `orch_working_memory_write` (`scope_id`, `key`, `text`, `kind?`, `status?`, `sources?`, `ttl_seconds?`).
   - Launch the orchestration Web UI via `ai-infra-mod orch --project "<repo-root>" serve`.
