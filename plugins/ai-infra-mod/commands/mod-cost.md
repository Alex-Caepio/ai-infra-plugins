---
name: mod-cost
description: Audit Claude Code and Codex CLI token usage, cache efficiency, and BTE cost metrics across local session transcripts.
argument-hint: "\"[--host all|claude|codex] [--project substr] [--since-days N]\""
---

Audit AI session costs and token efficiency for `$ARGUMENTS`.

1. Call `skill_cost_audit` with appropriate filters (`host`, `project`, `since_days`).
2. Summarize per-host and per-project session counts, input/cache/output tokens, and BTE cost metrics.
