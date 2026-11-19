---
name: policy-authoring
description: Use this skill when writing or customizing .agent-guardrails/policy.json rules, adding custom deny_rules or project_command_prefixes, adding additional_secret_globs, tuning loop_thresholds, or configuring file-naming rules and subagent_gate writer rosters.
version: 0.1.0
license: Apache-2.0
---

# Authoring `.agent-guardrails/policy.json`

Use this skill to customize guardrail rules declaratively in `.agent-guardrails/policy.json` without modifying binaries or hook wiring.

## Configuration Structure

Create or edit `.agent-guardrails/policy.json` (start with `guardrails_init_policy` or `guardrails policy-init`):

```json
{
  "schema_version": 1,
  "preset": "node",
  "approved_tools": ["terraform"],
  "project_command_prefixes": [
    {
      "tool": "terraform",
      "argv": ["terraform"],
      "options_with_values": ["-chdir"],
      "short_options_with_values": ""
    }
  ],
  "deny_rules": [
    {
      "id": "deny_terraform_destroy",
      "tool": "terraform",
      "command": ["destroy"],
      "matching": "exact",
      "condition": { "kind": "no_option", "patterns": ["--dry-run"] },
      "reason": "destructive_command"
    }
  ],
  "loop_thresholds": {
    "cmd_warn": 6,
    "cmd_block": 12,
    "edit_warn": 7,
    "edit_block": 10
  },
  "naming_rules": {
    "enforce_accelerator_conventions": true,
    "blocked_globs": ["*.secret.md"]
  },
  "subagent_gate": {
    "enabled": true,
    "block_builtin_codex": false,
    "allowed_agents": [],
    "writer_agents": []
  },
  "outside_repo_rm": "block",
  "additional_secret_globs": ["*.pem", "*.key"]
}
```

## Schema Rules

1. **`preset`**: One of `generic`, `laravel`, `symfony`, `wordpress`, `node`, `python`, `rust`. Custom `deny_rules` and `additional_secret_globs` layer cleanly on top of the chosen preset.
2. **`approved_tools` + `project_command_prefixes` + `deny_rules`**:
   - Every `tool` in `approved_tools` (lowercase slug) must have a matching `project_command_prefixes` entry (`argv`, optional `options_with_values`).
   - Each `deny_rules` item requires unique slug `id`, registered `tool`, `command` word list, `matching` (`"exact"` or `"console"`), `reason` (`"destructive_command"`, `"secret_exposure"`, or `"verification_bypass"`), and `condition`:
     - `{"kind": "always"}`
     - `{"kind": "any_option", "patterns": [...]}`
     - `{"kind": "no_option", "patterns": [...]}`
     - `{"kind": "first_argument", "patterns": [...]}`
     - `{"kind": "argument_contains", "patterns": [...]}`
3. **`additional_secret_globs`**: Filename globs blocked across common file readers (`cat`, `head`, `tail`, `less`, `rg`, `grep`, `sed`, `awk`, etc.).
4. **`outside_repo_rm`**: `"block"` (default) refuses recursive `rm -r` targeting paths outside the repository root and outside `/tmp`/`/var/tmp`/`$TMPDIR`; `"allow"` falls back to global system-root / broad-target checks only.
5. **`subagent_gate`**: Add allowed agent names to `allowed_agents` and write-serialized agents to `writer_agents` (also auto-reads `writes: true` from `.claude/agents/*.md` and `.cursor/agents/*.md` frontmatter).

## Verification Workflow

After editing `.agent-guardrails/policy.json`:

1. Call MCP `guardrails_inspect_policy` with `{"root": "<repository-root>"}` (or run `guardrails status --root "<repository-root>" --json`) to confirm schema validation succeeds (malformed `policy.json` causes `bash-guard` to fail closed!).
2. Test blocked & allowed commands via `guardrails_check_command` (`{"command": "<cmd>", "root": "<repository-root>"}`) or `guardrails check-command --root "<repository-root>"`.
3. Test naming rules if customized via `guardrails check-naming --root "<repository-root>" <path>`.
