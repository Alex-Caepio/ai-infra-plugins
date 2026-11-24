# Skill Foundry (Antigravity Extension)

Engineering workbench for Agent Skills — scaffolding, frontmatter/link validation, calibrated `cl100k` token budgeting, `.skill` packaging, cross-CLI trigger evaluation/optimization, 8+4 gate security vetting, and Claude/Codex BTE session cost auditing packaged as an Antigravity Extension.

## Included Components

- **`mcp_config.json`**: Registers the `skill-foundry` MCP server (`./bin/skill-foundry mcp`, `cwd: "${PLUGIN_ROOT}"`) exposing 8 tools:
  - `skill_scaffold`, `skill_validate`, `skill_package`, `skill_eval_triggers`, `skill_optimize_description`, `skill_budget_check`, `skill_vet`, `skill_cost_audit`
- **`rules/AGENTS.md`**: Enforces explicit external target paths (`plugin-cwd` guard compatibility), lean skill surfaces, and pre-adoption security vetting.
- **`skills/`**: `session-cost-audit`, `skill-authoring`, `skill-eval-optimize`, `skill-security-vetting`, `cost-audit`, `skill-optimize`, `skill-validate`, `skill-vet`.
- **`agents/`**: `sf-skill-architect`, `sf-security-auditor`.

## Installation

Copy this directory alongside the compiled `bin/skill-foundry` binary from `plugins/skill-foundry/` (released in [Alex-Caepio/skill-foundry](https://github.com/Alex-Caepio/skill-foundry)) into your Antigravity plugins directory (`~/.gemini/config/plugins/skill-foundry/`), then enable `Skill Foundry` on the Customizations page (`disabled: true` by default).

## License

Apache-2.0 — see [LICENSE](./LICENSE).
