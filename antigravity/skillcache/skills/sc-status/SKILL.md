---
name: sc-status
description: Inspect SkillCache sandbox readiness, compiler CLIs, trace count, capsule status counts, and amortization for a repository.
---

# SkillCache Status Workflow

Apply this workflow to the repository requested by the user:

1. Determine the explicit repository root `<repo-root>` from the user's request or active workspace root (must point outside the extension directory).
2. Call MCP tool `skillcache_status` with `{"project": "<repo-root>"}` (always pass `"project"` explicitly so `cwd: "${PLUGIN_ROOT}"` does not trigger the `plugin-cwd` guard).
3. If MCP is unavailable, run:
   ```sh
   skillcache-engine status --project "<repo-root>"
   ```
4. Summarize sandbox availability, available compiler CLIs (`claude`, `codex`), capsule status counts, stale reasons, trace count, and estimated break-even reuse amortization.
