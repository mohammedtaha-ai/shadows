---
name: feedback-agent-freedom
description: Mohammed lets Masada edit frontend-builder/fixer definitions and choose task splits herself; builders must screenshot-compare visual tasks
metadata:
  type: feedback
---

Mohammed (2026-10-19): "do what you see fit". Masada may change the builder's and the fixer's settings and instructions (`.claude/agents/frontend-*.md`, git-ignored, local only) and decide herself how to split tasks, without asking first.

**Why:** He wants her to run the side, not to be asked about small process choices.

**How to apply:** Choose and act, then tell him in one line what changed. Lesson behind the first such change: U-W1b's builder checked the layout only through tests and never looked at the mockup, so a visual task must name its target `.png` and the builder screenshots and compares before committing. Masada still takes her own screenshot to verify.
