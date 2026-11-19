---
name: mod-export
description: Export a portable, privacy-filtered context-graph-bundle-v1 package or subgraph for transfer to another repository or agent host.
argument-hint: "\"<destination> [--subgraph-root <node>] [--depth N]\""
---

Export a verified `context-graph-bundle-v1` archive/directory for `$ARGUMENTS`.

**Important**: Pass `project: "<repo-root>"` and explicit `destination`.

1. Run `memory_status` (`project: "<repo-root>"`) to confirm indexed Project Brain, Memory Bank, and SQLite graph state.
2. Call `context_export_bundle` with `project`, `destination` (plus optional `subgraph_root`, `depth`, `format`, `force`).
3. Summarize manifest SHA-256 digests, exported node/edge counts, and privacy scan status.
