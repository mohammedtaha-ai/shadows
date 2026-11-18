---
name: stages-after-approval
description: Write a plan's tasks into stages.md only after Mohammed approves the plan, never with the draft
metadata:
  type: feedback
---

Write the plan first and bring it to Mohammed; add its B-tasks to `docs/backend/stages.md` only after he approves it.

**Why:** Mohammed, 2026-11-18, after I wrote the executor-choices tasks into stages.md with the draft plan and then had to rewrite them after Codex's review: "عشان اذا في تعديل لا تتعب مرتين".

**How to apply:** for every new plan, the draft commit touches only `docs/backend/plans/`. Only after his approval is `stages.md` updated, in its own commit.
