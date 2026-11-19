---
name: mod-vet
description: Security-vet and validate Agent Skills or plugins across 8 binary security gates, 4 scored quality gates, and token ceilings.
argument-hint: "\"<skill-or-plugin-path>\""
---

Security-vet, validate, and budget-check the skill or plugin at `$ARGUMENTS`.

**Important**: Pass an explicit absolute or repo-relative `path` outside any plugin cache directory.

1. Call `skill_vet` with `{ "path": "<skill-or-plugin-path>" }`.
2. Call `skill_validate` and `skill_budget_check` on the same path.
3. Report binary gate pass/fail outcomes, quality score, and token usage cleanly.
