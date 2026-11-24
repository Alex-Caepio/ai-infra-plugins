---
name: mod-skillcache
description: Inspect SkillCache sandbox status, validate capsule packages with Gate 1, ingest traces, compile candidate workflows, and manage compiled capsule lifecycle.
argument-hint: "[status | gate1 <dir> | compile <candidate-or-trace> | capsules | inspect <id> | requalify <id>]"
---

Manage SkillCache compilation and capsule lifecycle for `$ARGUMENTS` via `skillcache_*` MCP tools (or `ai-infra-mod skillcache ...`).

**Important**: Pass explicit `project: "<repo-root>"` (or `package_dir` for `skillcache_gate1`) unless `AI_INFRA_ROOT` is configured.

1. Check sandbox & compiler availability via `skillcache_status` (`project: "<repo-root>"`).
2. Based on `$ARGUMENTS`:
   - `gate1`: call `skillcache_gate1` (`package_dir`, optional `examples_path`).
   - `traces`: call `skillcache_traces` (`project`, `action: "list"` or `action: "ingest"` + `trace_path`).
   - `compile`: call `skillcache_compile` with `project` and `candidate_path` (direct Gate 1/2/3), `trace_path`, or `generation` + `provider`.
   - `capsules` / `inspect`: call `skillcache_capsules_list` (`project`) or `skillcache_capsule_inspect` (`project`, `capsule_id`).
   - `requalify` / `invalidate` / `retire`: call `skillcache_requalify`, `skillcache_invalidate`, or `skillcache_retire` (`project`, `capsule_id`).
3. Summarize gate verdicts, active/stale counts, and amortization metrics cleanly.
