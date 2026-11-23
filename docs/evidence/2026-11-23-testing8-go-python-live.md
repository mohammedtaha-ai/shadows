# testing8: a Go CLI and a Python report, Codex executors (live, paused)

Date: 2026-11-23 (Asia/Aden). Branch `feature/after-75`, built from `4d69977d` with the
commits named below. Daemon: the `daemon` entry of `.claude/launch.json`, original database.
Project `testing8` at `F:\shadows_projects_testing\testing8`
(`dadfc666-6314-48be-959a-53fc7b07334d`), Planner thread
`22862fa7-5da6-41dd-8161-89f9fe5c180a`. Planner: Claude Code, Sonnet 5.5, medium.
Executors: Codex, `gpt-6.1-sol`, medium (Setup's first attempts ran on `gpt-6-luna`, low).
Python 3.14.5, pytest 9.2.0 (installed during the run), Go 1.26.5, gopls present, pyright absent.

## What the Planner did

- Idea to plans in one conversation. The person changed the language three times (Python, Go,
  then both); the Planner withdrew the superseded purpose card, edited the approved one and
  proposed goals for a Go `water` CLI that owns a JSON file and a read-only Python `report`.
  It proposed the file format as a contract both test suites check (TD3).
- `map_propose` was refused twice (`invalid_path` for two paths in one place,
  `layer_not_used_without_reason`); the Planner corrected both. It stopped at the map's waivers,
  which only the person's button approves.
- Setup (2 tasks) and three feature plans (storage 2, CLI 5, report 5 tasks). Plan approval
  refused two plans for a task using another's output without a link; the Planner added the links.

## Progress when paused

Commits: `d4d5023` initial, `f644227` and `a092e9d` (Setup), `23ba233` and `901629f` (storage),
`4f96d9b`, `f278e9b`, `1923b81`, `36dbd33`, `02a2f3c`, `e14e02f`. 9 of 15 tasks judged done;
CLI T3 and report T3 were running. The Planner was told to finish those and start nothing new.

## Shadows defects found and fixed on the branch

- **Unnoted changes asked about files the Planner never wrote** (B-197, `ac7d1e3d`). Before:
  four turns answered "changed with no reason given" for an executor's files, git-ignored
  caches and build output (`.pytest_cache`, `__pycache__`, `water/water.exe`), folders and
  vanished temp files; `project_change_note` refused most of them. After the fix, with two
  Codex attempts writing at once and the Python tasks running pytest, zero such notices.
- **Codex CLI 0.171.0 refused** (B-198, `d5d7baba`). The update changed `AgentMessageDisplay`,
  which Shadows does not read; the snapshot was adopted after the signed-in live Codex tests
  passed. All 12 feature tasks were then set to Codex one call at a time with no failure.
  The earlier `catalog_unavailable` (os error 32) was the npm update still writing `codex.exe`.

## Observed and not fixed

- The first two Setup attempts failed to open: Codex's model-list refresh timed out on the network
  and passed the 20 s `setup_wait`. The retry opened. The error text is in Codex's own log, not
  in the daemon's.
- Codex's sandbox cannot run the WindowsApps `python` alias ("The file cannot be accessed by the
  system"); `C:/Python314/python.exe` runs but has no pytest. Executors report Python checks
  `not_run`; Shadows' own check runs pass them.
- Every Codex executor's first `git add` is denied on `index.lock`, then approved and retried:
  one extra round per task.
- pyright is not installed, so each Python file's contract is `unchecked`.
- Shadows' Codex model list did not offer `gpt-6.2-sol`, which Codex's cache lists.
- The recheck after each done mark slowed every later notice (B-200). After resuming, the recheck
  caused by CLI T5's mark ran 19:29:12 to 19:35:15 UTC, and Python T4's 19:30:05 to 19:37:57, each
  holding the project's check lock. Python T5 committed at 19:33:37; its notice was sent at
  19:38:44, against 18 s for CLI T5 before the rechecks began.
- `live_the_claude_code_catalog_lists_each_models_efforts` failed during B-198's run
  (`harness_catalog.rs:160`); not examined.
