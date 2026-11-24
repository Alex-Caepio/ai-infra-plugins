# AI Infrastructure — Native Antigravity Extensions

Native **Google Antigravity** extension bundles for all 7 AI-Infrastructure plugins, built from source-only definitions under `antigravity/` and packaged with the stripped x86-64 ELF binaries from `plugins/*/bin/`.

## Extension Lineup (`7` Extensions, `0` Collisions)

| Extension | Version | Default State | MCP Tools | Hooks (`hooks.json`) | Skills | Agents |
| --- | --- | --- | --- | --- | --- | --- |
| **`ai-infra-mod`** | `0.2.0` | Enabled (`disabled: false`) | `80` (`ai-infra-mod mcp`) | `SessionStart`, `PreInvocation`, `PreToolUse`, `PostToolUse`, `Stop` | `19` | `6` |
| **`graph-map`** | `0.2.0` | Disabled (`disabled: true`) | `16` (`graph-map mcp`) | `—` | `7` | `2` |
| **`system-orchestration`** | `0.1.0` | Disabled (`disabled: true`) | `25` (`orch-engine mcp`) | `PreToolUse`, `PostToolUse` (`invoke_subagent`) | `12` | `2` |
| **`context-graph`** | `0.1.0` | Disabled (`disabled: true`) | `17` (`context-graph-engine mcp`) | `SessionStart`, `PreInvocation`, `Stop` | `14` | `2` |
| **`skill-foundry`** | `0.1.0` | Disabled (`disabled: true`) | `8` (`skill-foundry mcp`) | `—` | `8` | `2` |
| **`agent-guardrails`** | `0.1.0` | Disabled (`disabled: true`) | `4` (`guardrails mcp`) | `PreToolUse`, `PostToolUse` | `4` | `1` |
| **`skillcache`** | `0.1.0` | Disabled (`disabled: true`) | `9` (`skillcache-engine mcp`) | `—` | `7` | `2` |

`ai-infra-mod` ships enabled by default and provides the unified **80-tool MCP multiplexer** (`context_omnigraph_atoms` & `context_omnigraph_merge_keep` disambiguated cleanly alongside `orch-engine` `omnigraph_*`). All `6` modular extensions ship with `"disabled": true` so users can toggle individual subsystems cleanly inside Antigravity without duplicate MCP tools or hook double-firing.

## Anatomy of Each Extension

- `plugin.json`: Antigravity manifest (`name`, `displayName`, `version`, `description`, `logo`, `suggestedPrompts`, `author`, `homepage`, `repository`, `disabled`).
- `mcp_config.json`: Local stdio MCP transport (`./bin/<binary> mcp` with `"cwd": "${PLUGIN_ROOT}"`).
- `hooks.json` *(on `ai-infra-mod`, `system-orchestration`, `context-graph`, `agent-guardrails`)*: Native Antigravity lifecycle hooks reading camelCase `toolCall` / `workspacePaths` / `conversationId` stdin JSON and outputting Antigravity `decision` / `injectSteps` / `{}` contracts.
- `rules/AGENTS.md`: Always-on plain-Markdown rules enforcing explicit workspace root arguments (aligned with the fail-closed `plugin-cwd` MCP guard) and `omnigraph_*` disambiguation.
- `skills/<skill>/SKILL.md`: `71` globally unique skills converted from plugin skills and slash commands.
- `agents/<agent>.md`: `17` subagents using native Antigravity toolsets (`read_file`, `edit_file`, `write_file`, `run_command`, `manage_task`, `search_web`).

## Building & Installing

### Local Install & Reproducible `.zip` Packaging

Run from the repository root:

```bash
# Install all 7 self-contained extensions into ~/.gemini/config/plugins/
# and emit reproducible release archives with 0755 binary permissions:
./antigravity/build-antigravity.sh --install --dist-dir ./dist/antigravity
```

`build-antigravity.sh` stages files cleanly into `$HOME/.gemini/config/plugins/<name>/` and preserves Unix `0755` executable mode on `bin/*` (`0644` on source files) without altering `~/.gemini/config/config.json`.

### Installing from GitHub Release (`marketplace.json`)

```bash
agy plugin validate ./antigravity/marketplace.json
agy plugin install ai-infra-mod
```

Or install any individual archive directly with `agy plugin install <archive-url-or-zip>`, and toggle extensions live on the Antigravity Customizations page (or via `agy plugin enable` / `agy plugin disable`).
