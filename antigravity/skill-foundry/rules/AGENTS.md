# Skill Foundry Antigravity Rules

1. **Always pass explicit external paths to `skill_*` MCP tools**:
   - Antigravity runs `./bin/skill-foundry mcp` with `cwd: "${PLUGIN_ROOT}"`.
   - Always pass an explicit external directory path (`"path"`, `"skill_path"`, `"output_dir"`, or `"project_root"`) rather than `"."` or omitting it so the `plugin-cwd` guard does not block calls inside the extension directory.
2. **Keep startup skill surfaces lean & valid**:
   - Keep `description` under 1024 characters without `<` or `>` tags, keep `SKILL.md` concise (under 500 lines), move heavy schemas/examples into `references/`, and verify with `skill_validate` + `skill_budget_check`.
3. **Security vetting before adoption**:
   - Run `skill_vet` (8 binary + 4 scored gates) against any third-party skill or extension before enabling or copying it into a repository, passing `existing_skills` to catch naming collisions.
4. **Honest measurement**:
   - Never tune `description` on held-out eval queries (`skill_optimize_description` withholds holdouts cleanly) or extrapolate unprinted BTE numbers in `skill_cost_audit`.
