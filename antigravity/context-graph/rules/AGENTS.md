# Context Graph Antigravity Rules

1. **Always pass an explicit target repository path (`"project"`) to MCP tools**:
   - Antigravity runs `./bin/context-graph-engine mcp` with `cwd: "${PLUGIN_ROOT}"`.
   - Always pass `"project": "<absolute-workspace-root>"` on `memory_*`, `context_*`, and `omnigraph_*` MCP calls so the `plugin-cwd` guard operates strictly on your target repository.
2. **Verify against canonical project sources**:
   - Retrieved `Memory Bank` chunks (`auto-promoted` or `needs-review`) and graph edges are discovery aids only; verify cited source files before stating claims as facts.
   - Obtain exact file digests with `memory_source_fingerprints` and pass `revision` on `memory_record_result`.
3. **Secret hygiene & safe bundle transfer**:
   - Never include secrets, tokens, PII, or raw transcripts in handoffs or promoted Omnigraph atoms.
   - Always run `context_import_bundle` with `"dry_run": true` before applying imports.
4. **Cross-extension non-collision**:
   - Use skill `cg-omnigraph-sync` (not `orch-omnigraph-sync`) when syncing Project Brain and Memory Bank via this extension's `omnigraph_sync_canonical`, `omnigraph_file_context`, `omnigraph_atoms`, `omnigraph_branch_diff`, and `omnigraph_merge_keep` MCP tools.
