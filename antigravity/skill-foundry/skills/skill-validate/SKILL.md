---
name: skill-validate
description: Validate skills and measure calibrated token budgets for a skill or repository directory.
---

# Skill Validate Workflow

Apply this workflow to the skill or repository directory requested by the user (always pass the explicit external directory path rather than `"."`):

1. Resolve the explicit external directory path from the user's request or active workspace root.
2. Call `skill_validate` with `"path"` set to that explicit directory.
3. Call `skill_budget_check` with `"path"` set to that explicit directory.
4. Summarize any frontmatter/link errors, warnings (such as `SKILL.md` files over 500 lines), and the startup vs per-skill calibrated `cl100k` token breakdown.
