---
name: skillcache-compiler
description: Compile cold agent turn traces or candidate packages into sandbox-verified SkillCache capsules, run Gate 1/2/3 checks (--unshare-net), manage capsule lifecycle (requalify, invalidate, retire), and execute hybrid plans via ai-infra-mod skillcache.
---

# SkillCache Trace Compilation & Capsule Lifecycle (`skillcache-engine`)

Use the `skillcache_*` MCP tools and `ai-infra-mod skillcache ...` CLI inside `ai-infra-mod` to compile repeatable skill workflows into sealed, gate-verified hybrid or textual capsules stored in private local state (`$SKILLCACHE_STATE_DIR` / `$XDG_STATE_HOME/skillcache` / `~/.local/state/skillcache`).

## Target Root Rule

Always pass explicit `"project": "<repository-root>"` to `skillcache_*` tools (or `"package_dir": "<abs-package-dir>"` to `skillcache_gate1`), unless `AI_INFRA_ROOT` is configured via the Desktop Extension.

## 3-Gate Verification Model

1. **Gate 1 (`skillcache_gate1` / `ai-infra-mod skillcache gate1`)**: Static validation of `capsule.json`, schema version, compile rules, binding dataflow, runtime allowlists (`python3`, `php`, `node`, `bash`, `sh`), network declarations, and secret-redaction checks.
2. **Gate 2 (Sandboxed Program Check)**: Executes each hybrid program's `--check` contract in an isolated empty directory via `bubblewrap --unshare-net` (Linux) or `sandbox-exec` (macOS), failing closed without confinement.
3. **Gate 3 (Replay Verification)**: Replays the compiled capsule against the captured workspace snapshot and observed trace values inside the sandbox to confirm structural equivalence.

## Core Workflows

1. **Check Sandbox & Compiler Readiness**:
   - Call `skillcache_status` (`project: "<repo-root>"`) or run `ai-infra-mod skillcache status --project "<repo-root>"`.
2. **Ingest & Inspect Traces**:
   - Call `skillcache_traces` (`project: "<repo-root>"`, `action: "list"` or `action: "ingest"`, `trace_path: "<trace.json>"`).
3. **Compile & Qualify a Capsule**:
   - From a recorded trace generation: `skillcache_compile` (`project`, `generation`, `provider: "claude" | "codex"`).
   - From a trace JSON file directly: `skillcache_compile` (`project`, `trace_path`, `provider?`).
   - From a pre-authored candidate JSON or package dir (direct Gate 1/2/3 qualification): `skillcache_compile` (`project`, `candidate_path`).
   - Standalone Gate 1 pre-flight on a package folder: `skillcache_gate1` (`package_dir`, `examples_path?`).
4. **Manage Capsule Lifecycle**:
   - `skillcache_capsules_list` (`project`, optional `skill`, `status`, `form`, `include_retired`) — list capsules and amortization/staleness metrics.
   - `skillcache_capsule_inspect` (`project`, `capsule_id`) — inspect gate verdicts, capabilities, and dependency manifest.
   - `skillcache_invalidate` (`project`, `capsule_id`) — mark `active` / `needs_recompile` capsules `stale` so warm sessions fall back safely.
   - `skillcache_requalify` (`project`, `capsule_id`) — re-run Gates 1/2/3 without re-invoking an LLM (when `SKILL.md` tree SHA matches).
   - `skillcache_retire` (`project`, `capsule_id`, `reason?`) — permanently retire superseded capsules.
5. **CLI & Warm Execution**:
   - Compile / manage: `ai-infra-mod skillcache compile --project "<repo-root>" --candidate "<candidate.json>"` (or `capsules`, `inspect`, `requalify`, `invalidate`, `retire`, `traces`, `gate1`).
   - Execute a sealed warm package: `ai-infra-mod skillcache run '{"key":"val"}'` / `continue` / `tool <program> --check`.
