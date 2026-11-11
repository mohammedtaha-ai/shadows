---
name: commit-only-paths
description: With another task's tree open, commit docs with `git commit -- <paths>`, never a bare commit after git add
metadata:
  type: feedback
---

Commit my docs with `git commit -m ... -- <each path>`, not `git add` then a bare `git commit`.

**Why:** on 2026-11-11 the index already held B-121's staged rename (`executor.txt -> executor.md`); a bare commit took it in, which left HEAD unbuildable. I redid it with `reset --soft` and a path-limited commit.

**How to apply:** whenever `git status` shows a staged entry (R, A, M in the first column) that is not mine. Also never run `python -` in Bash here: it hangs on the Windows store stub.
