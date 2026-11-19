---
name: mod-import
description: Dry-run and safely import a context-graph-bundle-v1 knowledge bundle into the target project with collision handling and atomic rollback.
argument-hint: "\"<source> [--strategy skip|remap-ids|archive-superseded]\""
---

Safely import a `context-graph-bundle-v1` package from `$ARGUMENTS`.

**Important**: Pass `project: "<repo-root>"` and explicit `source`.

1. Always dry-run first: call `context_import_bundle` with `{ "project": "<repo-root>", "source": "<path>", "dry_run": true }`.
2. Review any ID collisions or `source_drift` warnings with the user.
3. Apply import with explicit `strategy` (`skip`, `remap-ids`, or `archive-superseded`) and confirm `memory_status`.
