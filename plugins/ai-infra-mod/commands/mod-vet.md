---
name: mod-vet
description: Security-vet and validate Agent Skills, plugins, or SkillCache capsule packages across binary security gates, quality scores, and token ceilings.
argument-hint: "\"<skill-plugin-or-package-path>\""
---

Security-vet, validate, and budget-check the skill, plugin, or SkillCache capsule package at `$ARGUMENTS`.

**Important**: Pass an explicit absolute or repo-relative `path` (or `package_dir`) outside any plugin cache directory.

1. For skills/plugins: call `skill_vet`, `skill_validate`, and `skill_budget_check` with `{ "path": "<target-path>" }`.
2. For SkillCache capsule packages (`capsule.json`): also run `skillcache_gate1` with `{ "package_dir": "<package-dir>" }`.
3. Report binary gate pass/fail outcomes, quality score, and token usage cleanly.
