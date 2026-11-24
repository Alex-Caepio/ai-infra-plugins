---
name: guardrails-setup
description: Use this skill when initializing agent-guardrails in a repository, choosing an ecosystem preset (generic, laravel, symfony, wordpress, node, python, rust), inspecting active shell/loop/naming/write-lock policy, resetting stuck session loop counters, or diagnosing why a hook blocked an action.
---

# Agent Guardrails Setup & Diagnostics

Use this skill to configure `.agent-guardrails/policy.json`, inspect active protections, or inspect/reset `$TMPDIR` session loop counters without executing untrusted commands.

## Built-In Presets

Choose the preset matching the repository stack:

- `generic` — Cross-stack Git safety (`--force`, `reset --hard`, `clean -f`, `--no-verify`, `core.hooksPath`), recursive `rm` protection (outside repo & system roots), `gh` destructive ops, `dropdb`/`mysqladmin drop`, destructive SQL (`DROP`, `TRUNCATE`, unfiltered `DELETE`), and `.env` / `.env.local` reads.
- `laravel` — `generic` + Artisan destructive migrations/seeds/queues/cache (`migrate:fresh`, `db:wipe`, `db:seed --force`, `env:decrypt`, `config:show`), `.env.*` secret reads, Composer auth config guards.
- `symfony` — `generic` + Doctrine (`doctrine:database:drop`, `doctrine:schema:drop`, down migrations, fixtures without `--append`), Messenger purge, `secrets:reveal`/`debug:dotenv`.
- `wordpress` — `generic` + WP-CLI (`wp db reset/drop`, `wp site delete/empty`, `wp search-replace --all-tables`, `wp config` secrets) and `wp-config*.php` read blocks.
- `node` — `generic` + Prisma (`migrate reset`, `db push --force-reset`), Drizzle (`drop`, `push --force`), Knex/TypeORM/Sequelize resets, `npm`/`pnpm`/`yarn` publish without `--dry-run`, `.npmrc`/`.yarnrc*` secrets.
- `python` — `generic` + Alembic (`downgrade base`/`-1`), Django (`flush`/`sqlflush`/`reset_db`), Flask `db downgrade`, `twine`/`poetry`/`uv` publish checks, `.pypirc`/`.netrc` secrets.
- `rust` — `generic` + `cargo publish` without `--dry-run`, `cargo yank`/`owner`, `sqlx`/`diesel`/`sea-orm-cli` resets, `.cargo/credentials*` secret reads.

## Procedure

> **Important for Extension-Scoped MCP callers:** Pass `"root": "<repository-root>"` explicitly to every `guardrails_*` MCP tool (`guardrails_init_policy`, `guardrails_inspect_policy`, `guardrails_check_command`, `guardrails_loop_status`). Because `mcp_config.json` sets `"cwd": "${PLUGIN_ROOT}"`, the MCP process defaults to the extension directory rather than the target repository.

1. **Inspect existing policy & runtime counters first**:
   - Prefer MCP tool `guardrails_inspect_policy` with `{"root": "<repository-root>"}` (or `guardrails_loop_status` with `{"root": "<repository-root>"}`) when MCP is available.
   - CLI equivalent:
     ```sh
     guardrails status --root "<repository-root>" --json
     ```
2. **Initialize `.agent-guardrails/policy.json`**:
   - Call MCP `guardrails_init_policy` with `{ "preset": "<preset>", "root": "<repository-root>" }` (add `"force": true` only if intentionally replacing an existing config).
   - CLI equivalent:
     ```sh
     guardrails policy-init --root "<repository-root>" --preset <preset>
     ```
3. **Test a command safely before running it**:
   - Call MCP `guardrails_check_command` with `{ "command": "<cmd>", "root": "<repository-root>" }` (read-only AST check; never executes the command).
   - CLI equivalent:
     ```sh
     guardrails check-command --root "<repository-root>" --json "<cmd>"
     ```
4. **Diagnose a hook decision or loop threshold**:
   - **Shell guard denial**: check the printed `reason` (`rule category`). Never bypass by wrapping destructive commands inside `bash -c`, `ssh`, or containers — the POSIX AST parser unwraps nested shells and launchers.
   - **Loop counter warning/block**: check `guardrails_loop_status` (`cmd` warns at 6 / blocks at 12; `edit` warns at 7 / blocks at 10 by default). Stop repeating the identical failing command/edit, inspect root cause first, or pass `reset: true` / `--reset` only after fixing the underlying issue.
   - **Subagent write lock**: `PostToolUse` on `invoke_subagent` (`subagent-release`) releases the `/tmp` write lock automatically. If a crashed run left a stale lock within its 30-minute TTL, verify no writer agent is still running before clearing it.

## Trust Boundaries

Guardrails are deterministic advisory hooks — not an operating-system sandbox, DLP service, or authorization boundary. Keep credentials outside prompts and review database/git targets independently.
