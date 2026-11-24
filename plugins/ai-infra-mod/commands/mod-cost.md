---
name: mod-cost
description: Audit Claude Code and Codex CLI token usage, cache efficiency, BTE cost metrics, and SkillCache capsule amortization across local sessions.
argument-hint: "\"[--host all|claude|codex] [--project substr] [--since-days N]\""
---

Audit AI session costs, token efficiency, and SkillCache amortization for `$ARGUMENTS`.

1. Call `skill_cost_audit` with appropriate filters (`host`, `project`, `since_days`).
2. Optionally inspect `skillcache_capsules_list` (`project: "<repo-root>"`) to compare recurring cold turns vs compiled SkillCache capsule runs.
3. Summarize per-host and per-project session counts, input/cache/output tokens, and BTE cost metrics.
