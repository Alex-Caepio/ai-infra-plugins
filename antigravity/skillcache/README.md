# SkillCache (Antigravity Extension)

Standalone SkillCache compiler, Gate 1/2/3 sandbox verifier, warm capsule executor, and stdio MCP server packaged as an Antigravity Extension.

## Included Components

- **`mcp_config.json`**: Registers the `skillcache` MCP server (`./bin/skillcache-engine mcp`, `cwd: "${PLUGIN_ROOT}"`) exposing 9 tools:
  - `skillcache_status`, `skillcache_capsules_list`, `skillcache_capsule_inspect`, `skillcache_gate1`, `skillcache_compile`, `skillcache_requalify`, `skillcache_retire`, `skillcache_invalidate`, `skillcache_traces`
- **`rules/AGENTS.md`**: Requires explicit `"project": "<repo-root>"` targets (enforced by the `plugin-cwd` guard) and sandbox confinement.
- **`skills/`**: `skillcache-capsules`, `skillcache-compile`, `skillcache-warm-run`, `sc-capsules`, `sc-compile`, `sc-gate1`, `sc-status`.
- **`agents/`**: `sc-compiler`, `sc-capsule-curator`.

## Installation

Copy this directory alongside the compiled `bin/skillcache-engine` binary from `plugins/skillcache/` (released in [Alex-Caepio/skillcache](https://github.com/Alex-Caepio/skillcache)) into your Antigravity plugins directory (`~/.gemini/config/plugins/skillcache/`), then enable `SkillCache` on the Customizations page (`disabled: true` by default).

## License

Apache-2.0 — see [LICENSE](./LICENSE).
