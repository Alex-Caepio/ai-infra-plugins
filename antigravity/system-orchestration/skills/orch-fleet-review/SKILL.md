---
name: orch-fleet-review
description: "Run or resume a parallel multi-lens fleet code review with human approval gate"
---

Run a multi-lens parallel fleet code review for the user's request.

1. Pass `"project"` pointing to the target repository root on every MCP tool call.
2. Call `orch_fleet_review` with appropriate `worker`, `lenses`, and `scope`.
3. When paused at `"awaiting_approval"`, present findings clearly and ask the user whether to approve or reject before calling `orch_fleet_review` with `resume: true` and `approve: true` or `reject: true`.
