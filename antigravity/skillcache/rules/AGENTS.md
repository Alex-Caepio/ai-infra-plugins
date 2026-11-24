# SkillCache Antigravity Rules

1. **Always pass an explicit target repository path (`"project"`) to `skillcache_*` MCP tools**:
   - Antigravity runs `./bin/skillcache-engine mcp` with `cwd: "${PLUGIN_ROOT}"`.
   - The built-in `plugin-cwd` guard refuses omitted, empty, or `"."` targets when invoked inside an Antigravity extension directory.
   - Pass `"project": "<absolute-workspace-root>"` on `skillcache_status`, `skillcache_capsules_list`, `skillcache_capsule_inspect`, `skillcache_compile`, `skillcache_requalify`, `skillcache_retire`, `skillcache_invalidate`, and `skillcache_traces`, and pass an explicit external `"package_dir"` to `skillcache_gate1`.
2. **Preserve sandbox confinement (Gate 1 / Gate 2 / Gate 3)**:
   - Never bypass `--unshare-net` / `sandbox-exec` verification or mutate sealed `warm/` capsule packages directly on disk.
   - Use `skillcache_requalify` / `skillcache-engine requalify` after inspecting stale capsules instead of relaxing Gate checks.
3. **Keep target repositories clean**:
   - SkillCache stores compilation SQLite databases and warm capsule artifacts under `$SKILLCACHE_STATE_DIR` (`$XDG_STATE_HOME/skillcache` or `~/.local/state/skillcache`), never inside target git repositories.
