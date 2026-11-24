---
name: skill-optimize
description: Evaluate and iteratively optimize a skill description using train/holdout trigger queries.
---

# Skill Optimize Workflow

Apply this workflow to the skill directory and eval-set JSON (`[{query, should_trigger}]`) requested by the user:

Follow the `skill-eval-optimize` skill:

1. Ensure an eval set JSON (`[{query, should_trigger}]`) and explicit external `<skill-dir>` / `<project_root>` are specified (or help the user generate one using `skill-eval-optimize`).
2. Call `skill_eval_triggers` first for baseline trigger precision and recall.
3. If failures occur, run `skill_optimize_description` with `"holdout": 0.4` and report the train/test improvement summary.
