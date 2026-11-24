---
name: multi-agent-orchestration
description: Plan multi-service system changes, coordinate supervised agent sessions and permission modes, manage Omnigraph branches/atoms and cross-service system graphs, trigger fleet/clash reviews, and manage SQLite working memory via system_*, orch_*, and omnigraph_* tools.
---

# Multi-Agent, System & Omnigraph Orchestration (`system-orchestration`)

Use the `system_*`, `orch_*`, and `omnigraph_*` tools in `ai-infra-mod` to coordinate multi-service changes, cross-service dependency bridges, supervised sessions, live approval modes, Omnigraph atom/branch settlement, fleet/clash verification, and working memory.

## Target Root Rule

Pass explicit `project` / `root` repository paths (unless `AI_INFRA_ROOT` is configured via Desktop Extension).

## Workflows

1. **System Discovery, Service Graph & Multi-Service Planning**:
   - Initialize or inspect services and contracts via `system_init` (`name?`), `system_catalog` (`system?`, `allow_root?`), `system_graph_status` (`config_path?`, `check?`, `service?`), `system_locate` (`task`), and `system_impact` (`service?`, `contract?`, `impact_depth?`).
   - Build and run an impact-scoped plan via `system_plan` (`task`, `change_id`, `service?`, `contract?`, `context_budget?`, `impact_depth?`, `output?`) -> `system_execute` (`plan`, `run_dir`) -> `system_run_status` (`run_dir`) or `system_resume` (`run_dir`).
2. **Supervised Agent Sessions, Live Permissions & Delivery**:
   - Start and manage sessions via `orch_session_start` (`prompt`, `provider?`, `workflow?`, `mode?`, `permissions?`, `workspace?`), `orch_session_send` (`session_id`, `prompt`, `permissions?`), `orch_session_status` (`session_id?`, `after?`), and `orch_session_cancel` (`session_id`).
   - Inspect, switch, approve, or revoke live session permissions via `orch_permission_mode` (`session_id`, `mode?: "auto" | "manual" | "bypass" | "default"`, `request_id`+`decision`+`answers?`, `revoke_rule_id` / `revoke_rule_key` / `revoke_all`).
   - Draft follow-up context via `orch_context_draft` (`source_session_id`, `destination_session_id?`) and preview commits/merges via `orch_delivery_preview` (`session_id`, `action?`).
3. **Orchestration Omnigraph (`orch-engine`, `mode=off` by default unless configured)**:
   - Inspect status & branch counts via `omnigraph_status` (`branch?`).
   - Search/filter/paginate atoms via `omnigraph_atoms` (`branch?`, `kind?`, `query`/`q?`, `session_id?`, `limit?`, `offset?`, `cursor?`, `search?`) and record findings/decisions via `omnigraph_publish` (`title`, `kind?`, `summary?`, `body?`, `branch?`, `atom_id?`, `session_id?`, `tags?`, `files?`, `confidence?`, `pin_priority?`, `expected_commit?`).
   - Compare and settle branch divergence via `omnigraph_branches_diff` (`source`, `target?` default `"main"`) and `omnigraph_merge_keep` (`atom_id`, `from`, `to?`, `expected_updated_at?`, `replaced_updated_at?`). (For `context-graph-engine`'s embedded offline `omnigraph.sqlite3`, use `omnigraph_sync_canonical`, `context_omnigraph_atoms`, `omnigraph_branch_diff`, and `context_omnigraph_merge_keep`.)
4. **Fleet Review & Challenger Clash**:
   - Trigger multi-lens parallel review via `orch_fleet_review` (`thread?`, `lenses?`, `scope?`, `approve?`, `reject?`).
   - Launch adversarial challenger verification via `orch_clash_debate` (`prompt?`, `provider?`, `challenger?`, `rounds?`).
5. **SQLite Working Memory & Web UI**:
   - Read and persist structured working notes via `orch_working_memory_read` (`scope_id`, `include_resolved?`, `run_dir?`) and `orch_working_memory_write` (`scope_id`, `key`, `text`, `kind?`, `status?`, `sources?`, `ttl_seconds?`).
   - Launch the orchestration Web UI (with `/api/events/stream` SSE + Omnigraph/System views) via `ai-infra-mod orch --project "<repo-root>" serve`.
