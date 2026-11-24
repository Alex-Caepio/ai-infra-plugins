---
name: cg-load
description: Load the latest or named portable session handoff and report git/file drift.
---

# Context Graph Handoff Load Workflow

Apply this workflow to the task ID or handoff file requested by the user:

Follow the `context-handoff` skill (`skills/context-handoff/SKILL.md`) and load the matching handoff for the target repository (`context_handoff_load` with explicit `"project": "<repo_root>"` over MCP, or `context-graph-engine memory context-load --root .`). Present restored goal, decisions, next steps, and any detected branch, commit, or file digest drift.
