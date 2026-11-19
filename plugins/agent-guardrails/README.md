# Agent Guardrails Plugin

Standalone, zero-dependency Rust safety & loop-control plugin for **Claude Code** and **OpenAI Codex CLI**.

Repository: <https://github.com/Alex-Caepio/agent-guardrails>  
Marketplace: <https://github.com/Alex-Caepio/ai-infra-plugins>

## Features

- **POSIX AST Shell Guard (`bash-guard`)**: Parses shell pipelines, subshells, `$(...)`, backticks, heredocs, and nested launchers (`ssh`, `docker`, `sail`, `ddev`, `kubectl`, `bash -c`, `eval`, etc.) without executing commands. Blocks:
  - Destructive Git operations (`git push --force`/`-f`/`+refspec`/`--force-with-lease`/`--mirror`, `git reset --hard`, `git clean -f`, `git branch -D`, `git commit --no-verify`/`-n`, `git config core.hooksPath`).
  - Broad or out-of-repo recursive deletions (`rm -rf /`, `~`, `.`, `*`, system roots `/etc/*`, `/usr/*`, and absolute paths outside the repository root except `/tmp`/`/var/tmp`/`$TMPDIR`).
  - Destructive SQL (`DROP TABLE|DATABASE|SCHEMA`, `TRUNCATE`, unfiltered `DELETE FROM`), `dropdb`, `mysqladmin drop`, `gh repo delete/archive`, `gh api -X DELETE`.
  - Secret file exposure (`.env`, `.env.local`, `.npmrc`, `.pypirc`, `wp-config.php`, Cargo credentials, and custom `additional_secret_globs`) across file readers while permitting `.env.example` templates and `-l`/`-q`/`--files-with-matches` searches.
- **Session Loop Breakers (`edit-loop` & `bash-guard`)**: Tracks repeated identical shell commands (`warn >= 6`, `block >= 12`, excluding read-only polling like `git status`) and repeated file edits (`warn >= 7`, `block >= 10`) in `$TMPDIR/claude-loop-detection-{uid}-{crc32(root)}`.
- **Repository File-Naming Validator (`naming-check`)**: Enforces `memory-bank/chunks/MEM-0001-slug.md`, `tasks/TASK-001/<skill>-*.md`, `specs/<skill>-*.md` (when skills exist), and customizable `blocked_globs`.
- **Advisory Subagent Write-Lock Gate (`subagent-gate` / `subagent-release`)**: Enforces project agent rosters and serializes write-capable subagents (`writes: true` frontmatter or `writer_agents` config) using `flock` + `O_NOFOLLOW` lock files in `/tmp`.
- **Selectable Presets & Custom `.agent-guardrails/policy.json`**:
  `generic` | `laravel` | `symfony` | `wordpress` | `node` | `python` | `rust`

## Installation

### Claude Code

Install from GitHub via the dual marketplace:

```bash
claude plugin marketplace add Alex-Caepio/ai-infra-plugins
claude plugin install agent-guardrails@ai-infra-plugins --scope user
```

Or clone the standalone plugin repository directly and load or validate it:

```bash
git clone https://github.com/Alex-Caepio/agent-guardrails.git
claude plugin validate --strict ./agent-guardrails
claude --plugin-dir ./agent-guardrails
```

### Codex CLI

Install from the GitHub marketplace repository:

```bash
codex plugin marketplace add https://github.com/Alex-Caepio/ai-infra-plugins.git
codex plugin add agent-guardrails@ai-infra-plugins
codex plugin list --json
```

Or from a local marketplace checkout (`./` inside the `ai-infra-plugins` repository root):

```bash
codex plugin marketplace add .
codex plugin add agent-guardrails@ai-infra-plugins
```

## CLI Usage

```bash
./bin/guardrails --help
./bin/guardrails policy-init --preset node
./bin/guardrails check-command --json "npx prisma migrate reset"
./bin/guardrails check-naming --json tasks/TASK-001/skill-notes.md
./bin/guardrails status --json
./bin/guardrails mcp
```

### Hook Entry Points (stdin JSON)

```bash
echo '{"tool_name":"Bash","tool_input":{"command":"dropdb prod"}}' | ./bin/guardrails hook bash-guard
echo '{"tool_name":"Edit","tool_input":{"file_path":"src/main.rs"}}' | ./bin/guardrails hook edit-loop
echo '{"tool_name":"Write","tool_input":{"file_path":"memory-bank/chunks/bad.md"}}' | ./bin/guardrails hook naming-check
echo '{"tool_name":"Agent","tool_input":{"subagent_type":"coder"}}' | ./bin/guardrails hook subagent-gate
```

Explicit host overrides are also supported: `guardrails hook <claude|codex|cursor> <role>`.

## MCP Tools (`guardrails mcp`)

All four MCP tools accept an optional `"root"` argument pointing to the target repository. Under Codex CLI (`.codex-mcp.json` runs with `"cwd": "."` relative to the cached plugin directory), pass `"root"` explicitly; `guardrails_init_policy` refuses to write into the plugin directory when `"root"` is omitted.

- `guardrails_check_command` — AST-check a shell command (`command`, `root?`, `preset?`)
- `guardrails_inspect_policy` — Show active preset, `.agent-guardrails/policy.json`, rule counts, and thresholds (`root?`, `preset?`)
- `guardrails_init_policy` — Initialize `.agent-guardrails/policy.json` (`preset`, `root?`, `force?`)
- `guardrails_loop_status` — Inspect or reset (`reset: true`) `$TMPDIR` session counters and active writer locks (`root?`, `session_id?`, `reset?`)

## Building from Source

The plugin commits the prebuilt stripped Linux x86-64 release binary at `./bin/guardrails`. To rebuild or run the full test suite locally without leaving build artifacts inside the plugin directory:

```bash
CARGO_TARGET_DIR=/tmp/build-agent-guardrails \
CARGO_INCREMENTAL=0 \
cargo test --offline --locked

CARGO_TARGET_DIR=/tmp/build-agent-guardrails \
CARGO_INCREMENTAL=0 \
cargo build --release --offline --locked
cp /tmp/build-agent-guardrails/release/guardrails ./bin/guardrails
strip ./bin/guardrails
chmod +x ./bin/guardrails
```

## Security Model

Hooks are advisory guardrails, not an operating-system sandbox, DLP engine, or authorization boundary. A malformed `.agent-guardrails/policy.json` causes `bash-guard` to fail closed (exit 2), while non-command/non-edit tools pass with zero overhead.
