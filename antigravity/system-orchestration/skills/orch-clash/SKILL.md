---
name: orch-clash
description: "Run an adversarial multi-round clash debate between two independent providers"
---

Start or inspect an adversarial clash debate session for the user's request.

1. Pass `"project"` pointing to the target repository root on every MCP tool call.
2. Call `orch_clash_debate` with distinct `provider` and `challenger`, `workflow` (`review` or `native`), `rounds` (`1..=3`), and `prompt` (or pass `session_id` to inspect an active debate ledger).
3. Summarize contested objections, resolved items, and the final verdict.
