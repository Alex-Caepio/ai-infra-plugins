---
name: cost-audit
description: Audit Claude Code & Codex BTE token usage by project and skill.
---

# Session Cost Audit Workflow

Apply this workflow to the local session log filters (`host: all|claude|codex`, optional `project` substring, optional `since_days`) requested by the user:

1. Call MCP tool `skill_cost_audit` with the requested filters (defaulting to `{"host": "all"}`).
2. Summarize total BTE spend, Claude vs Codex split, main vs subagent share, and the top projects, skills, MCP servers, and tools consuming tokens.
