---
name: feedback-background-agents
description: Mohammed wants frontend-builder and frontend-fixer always run in the background, never blocking the conversation
metadata:
  type: feedback
---

Run frontend-builder and frontend-fixer in the background (run_in_background: true).

**Why:** Mohammed asked for it explicitly on 2026-10-19 when U-W1 started.

**How to apply:** Every builder/fixer dispatch is backgrounded; keep talking with him while it runs, and check the commit and test when the notice arrives.
