---
name: sc-compile
description: Compile or qualify a SkillCache capsule from an observed trace, trace JSON, or candidate package through Gate 1/2/3.
---

# SkillCache Compile Workflow

Apply this workflow to the target repository and compilation input (`--generation <id>`, `--trace-path <file>`, `--candidate <path>`, `--skill <name>`, `--provider`, `--model`, or `--dry-run`) requested by the user:

Follow the `skillcache-compile` skill:

1. Resolve the explicit repository root `<repo-root>` and compilation source (`generation`, `trace_path`, `candidate_path`, or `skill`) plus optional `provider` (`claude`|`codex`), `model`, and `round_dry_run`.
2. Call MCP tool `skillcache_compile` passing `"project": "<repo-root>"` explicitly along with the supplied arguments.
3. If MCP is unavailable, run:
   ```sh
   skillcache-engine compile --project "<repo-root>" ...
   ```
4. Report the resulting `compile_id`, `capsule_id`, final status (`completed`/`failed`), per-gate verification outcomes (Gate 1, Gate 2, Gate 3), and any diagnostics on failure.
