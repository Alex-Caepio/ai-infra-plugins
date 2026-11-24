---
name: cg-save
description: Save a curated, secret-checked portable session handoff for continuing work cleanly.
---

# Context Graph Handoff Save Workflow

Apply this workflow to the task ID and detail level (`summary|topic|full`) requested by the user:

Follow the `context-handoff` skill (`skills/context-handoff/SKILL.md`) and save a portable context handoff in the target repository (`context_handoff_save` with explicit `"project": "<repo_root>"` over MCP, or `context-graph-engine memory context-save --root .`). Ensure `goal`, `summary`, `next_steps`, and cited `files` contain zero secrets or raw transcripts.
