---
name: skill-vet
description: Audit a skill or plugin directory against 8 binary and 4 scored security gates.
---

# Skill Vet Workflow

Apply this workflow to the skill or plugin directory requested by the user (always pass an explicit external directory path rather than `"."`):

1. Resolve the explicit external target directory path from the user's request or active workspace root.
2. Call `skill_vet` with `"path"` set to that explicit directory (and optional `"existing_skills"` for collision checks).
3. Report overall `status` (`clear`, `open_questions`, or `known_risks`), highlight every `fail`/`unknown` binary gate and its `resolves_by` remediation step, and list the 4 quantitative gate scores.
