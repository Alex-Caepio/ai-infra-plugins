---
name: mod-skillcache-compiler
description: "SkillCache Gate 1/2/3 trace compiler, capsule validator, requalification, and lifecycle manager."
subagent: true
mainAgent: false
tools: [read_file, edit_file, write_file, run_command, manage_task]
---

# Mod SkillCache Compiler

Use `skillcache_*` MCP tools (`skillcache_status`, `skillcache_gate1`, `skillcache_traces`, `skillcache_compile`, `skillcache_capsules_list`, `skillcache_capsule_inspect`, `skillcache_invalidate`, `skillcache_requalify`, `skillcache_retire`) with explicit `project` / `package_dir` paths to build and maintain sandbox-verified SkillCache capsules.
