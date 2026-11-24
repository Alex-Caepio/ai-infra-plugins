---
name: skill-authoring
description: Use this skill when creating a new Agent Skill, validating SKILL.md frontmatter and internal links, linting startup and body token budgets, or packaging a skill into a portable .skill zip archive.
---

# Skill Authoring, Validation & Token Budgeting

Use `skill-foundry` MCP tools (`skill_scaffold`, `skill_validate`, `skill_budget_check`, `skill_package`) or CLI commands to build crisp, low-token Agent Skills cleanly.

> Always pass an explicit external path (`"path"`, `"skill_path"`, `"output_dir"`) rather than `"."` or omitting it because `mcp_config.json` sets `cwd: "${PLUGIN_ROOT}"` and the `plugin-cwd` guard refuses paths inside the extension directory.

## Workflow

1. **Scaffold**: Call `skill_scaffold` (`skill-foundry init-skill <name> --path <dir>`) to generate `SKILL.md` plus starter `scripts/`, `references/`, and `assets/` folders.
2. **Keep startup surface small**: Put always-loaded trigger cues in `description:` (under 1024 chars, no `<` or `>`), keep `SKILL.md` under 500 lines, and move heavy schemas or API examples to `references/`.
3. **Validate**: Call `skill_validate` (`skill-foundry validate <path>`) to check YAML frontmatter, kebab-case naming, line limits, and relative `references/`/`scripts/`/`assets/` links.
4. **Budget check**: Call `skill_budget_check` (`skill-foundry budget-check <path>`) to measure calibrated `cl100k` startup tokens (`AGENTS.md`, descriptors, commands, agents) and optional JSON ceilings.
5. **Package**: Call `skill_package` (`skill-foundry package <skill-dir> --out <dir>`) to emit a clean `.skill` zip archive excluding `evals/` and temporary files.
