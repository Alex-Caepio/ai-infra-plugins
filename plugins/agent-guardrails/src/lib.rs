//! Standalone POSIX AST guardrails, session loop counters, naming checks,
//! subagent write locks, and stdio MCP server for Claude Code and Codex CLI.
pub mod counters;
pub mod custody;
pub mod error;
pub mod guards;
pub mod mcp;
pub mod policy;
pub mod presets;
pub mod shell;
