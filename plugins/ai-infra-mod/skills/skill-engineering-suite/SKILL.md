---
name: skill-engineering-suite
description: Scaffold, validate, package, evaluate triggers, optimize descriptions, enforce token budgets, vet security gates, and audit Claude/Codex session costs with skill-foundry.
---

# Skill Engineering & Cost Audit (`skill-foundry`)

Use the `skill_*` tools inside `ai-infra-mod` to author, validate, benchmark, optimize, and vet Agent Skills and plugins, and audit cross-CLI session costs.

## Target Path Rule

Always pass explicit `path` / `skill_path` / `project_root` / `output_dir` arguments inside the target workspace (omitted or `"."` paths inside a plugin directory are rejected).

## Workflows

1. **Authoring & Validation**:
   - `skill_scaffold`: create a new skill skeleton (`SKILL.md` + template structure).
   - `skill_validate`: verify YAML frontmatter, structure, and strict naming rules.
   - `skill_budget_check`: enforce token (`cl100k`) and JSON payload ceilings across `skills/` and `commands/`.
   - `skill_package`: validate and pack distributable skill artifacts into `output_dir`.
2. **Evaluation & Description Optimization**:
   - `skill_eval_triggers`: run trigger precision/recall evaluation on held-out prompts (`claude`/`codex` routing).
   - `skill_optimize_description`: iteratively refine `description` frontmatter against deterministic train/holdout splits (`MT19937`).
3. **Security Vetting & Session Cost Audits**:
   - `skill_vet`: execute the 8 binary security gates and 4 scored quality gates before adopting third-party skills.
   - `skill_cost_audit`: analyze local `~/.claude/projects` and `~/.codex/sessions` BTE cost efficiency and token usage.
