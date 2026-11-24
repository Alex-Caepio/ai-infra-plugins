---
name: cg-checkpoint
description: Run memory health check, retrieve grounded capsule, checkpoint active task, or curate promotions.
---

# Context Graph Checkpoint Workflow

Apply this workflow to the subcommand or task arguments (`status|retrieve|checkpoint|promote-review|validate`) requested by the user:

Follow the `memory-checkpoint` skill (`skills/memory-checkpoint/SKILL.md`) against the target repository (pass `"project": "<repo_root>"` explicitly over MCP or run `context-graph-engine memory ... --root .`). Verify cited canonical files before recording verified findings or applying promotions.
