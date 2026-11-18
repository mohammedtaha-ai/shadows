# Executor choices: remaining acceptance (B-159)

The rest of this plan (B-155 to B-158, and the web task F-125) is on `main` at `0e281585` (PR #71).
Its rulings are in [executors design](../specs/2026-11-08-executors-design.md) and its web API is in
the accepted request (`docs/shared/requests/2026-11-18-executor-choices.md` at `e78e07dc`). The full plan is in Git at `a1561e20`.

## Task B-159: acceptance in the browser

Needs: B-155 to B-158 and F-125, all on `main` at `0e281585`.

**Files:**
- Modify: `docs/backend/stages.md`
- Create: `docs/evidence/2026-11-18-executor-choices.md`

**Steps:**
- [ ] On a disposable project:
  1. A new conversation shows Claude Code's models and efforts before the first message, starting on the remembered ones.
  2. The Planner plans two tasks and saves a choice on each.
  3. Mohammed changes T2's model and effort on its page.
  4. Mohammed starts T1, and the Planner's `task_start` starts T2. Each attempt's Run tab and the logs show the saved CLI, model and effort.
  5. With Codex configured, one task runs on codex and commits only its writes.
- [ ] Run the signed-in live tests by name, with `--include-ignored`: both `live_..._catalog_lists_each_models_efforts` and `live_a_codex_executor_commits_only_its_writes`. Record each result, or "not run" with its reason.
- [ ] Record the dates, commits and log lines in the evidence file, set B-159 to done, and commit.
