---
name: feedback-same-agent-related-tasks
description: Related tasks go to the same builder agent (SendMessage to it) instead of a fresh agent
metadata:
  type: feedback
---

When tasks are related (same files or one builds directly on the other), give the next task to the agent that did the previous one: `SendMessage` to it with the same one line `Do task U-<n> in docs/aylul/ui/stages.md`, not a new `Agent` call.

**Why:** Mohammed asked for it on 2026-10-19: the agent already holds the context, so a fresh one re-reading everything is waste.

**How to apply:** Start a new agent only for an unrelated task, or when the previous agent's context is long or its work went wrong. First applied: U-W1c then U-W2 (the same shell files).
