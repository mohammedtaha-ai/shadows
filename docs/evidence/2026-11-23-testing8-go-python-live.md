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

Resumed the same day: `0863e02` (report T4), `11a5d85` (CLI T5), `00569c8` (report T5). All 14
tasks judged done (the Planner said 15; the plans hold 14). The Planner ran both programs end to
end: Go wrote the file, Python read it, the file's checksum was unchanged; Go passes in 8
packages, pytest 39 passed.

## Claude Code executor, for comparison

One more plan, one task (`--days N` for the report), executor Claude Code (Sonnet 5.5, medium):
attempt `a0812824`, started 19:43:47, committed `6a662f5` at 19:44:36, notice sent 19:45:46 UTC.
The executor ran pytest itself (66 passed), its new test copied a fixture into pytest's
temporary folder, and `git add` and `git commit` were not denied (its report names only LF→CRLF
warnings). The sandbox findings below are Codex's only.

## After B-200 to B-205 (2026-11-24)

Rebuilt with `2853797a` to `3957395b`. Plan "Water CLI reset-today", two tasks, Codex
`gpt-6-luna`, medium (`gpt-6.2-sol` was not offered: Codex's own cache, fetched from OpenAI at
14:24 UTC, listed only `gpt-6-luna`, `gpt-5.6-terra` and `gpt-5.6-luna`).

| | T1 `a4923097` | T2 `b3fbb5d6` |
|---|---|---|
| Attempt ended (UTC) | 15:04:57 | 15:06:29 |
| Notice sent | 15:05:17 (20 s) | 15:06:33 (3 s) |
| Git | committed first time | committed first time |

No `index.lock` denial and no approval in either report (B-201). T1's mark started no recheck;
T2's, which completed the plan, started one (15:06:42 to 15:12:13: 30 passed, 6 `contract`
`not_run` for want of pyright), which sent no notice (B-200). T1's executor still saw `go test`
rename tests fail with "Access is denied" inside the sandbox; Shadows' own checks passed them.
The Planner waited for a recheck message that a mark-started recheck with no failure never sends.

## The person's recheck on the plan page (2026-11-24)

Same daemon and plan ("Water CLI reset-today"), web on `vite` at 5173. Each row is one recheck
started by the person, rebuilt and restarted before it. Times UTC.

| Built at | Started | Finished | Result | What was seen |
|---|---|---|---|---|
| B-206 `89995c61`, F-137 `41926e85` | 16:06:21 | 16:12:04 | 30 passed, 6 `not_run` | The line read the earlier batch (30 passed, 6 not run), the button gave way to "Rechecking…", and the page updated itself when the batch finished |
| B-207 `0389c34b`, F-138 `47acbc1d` | 16:47:40 | 16:53 | 30 passed, 6 `not_run` | The panel opened on the click, rows turned passed one by one, a finished row showed pytest's output as in a terminal; `contract` rows print nothing |
| B-208 to B-210, F-139 to F-144 (`578f909b`) | 19:59:10 | 20:05:06 | 30 passed, 6 `not_run` | The button and line updated from the project stream; no read polled while the batch ran |

The 6 `not_run` are the Python `contract` checks: pyright is not installed.

**Requests from the plan page.** Before the polling work, an idle plan page read `/api/projects`
and the project's threads every 10 s each, and `/recheck` every 5 s while a batch ran; the tab,
open since 14:22, had sent thousands. After `578f909b`, with the project and thread streams open:

| Plan page | Window | Requests to the daemon |
|---|---|---|
| Idle | 54 s | 0 |
| Recheck running, panel open | 420 s | `/recheck` 43, `/rechecks/{id}` 42, `/rechecks/{id}/output` 2: about one of each per finished check; the rest (2 each) were the page's own refetch when the pane was shown |

Timers ran at their normal rate in the hidden pane (10 ticks of 500 ms in 5 s), so the 0 is not
throttling. On the task page, each full refetch (`/api/me` included) followed a
`visibilitychange` from the app's pane being shown and hidden: React Query's refetch on focus,
not polling.

**Defects found while trying it, fixed on the branch:**

- The all-passed recheck card showed no body: `notice-item.tsx` did not pass it (`224713f5`).
- A batch the person asked for with a failing check and no Planner thread to tell never
  finished: `end_recheck` returned early on `Unavailable`, so "Rechecking…" would never end
  (`18454d14`, found by B-207's builder).
- With the panel open the whole app scrolled up and left a black band: each row's hidden state
  label was placed against `main`, ending at y=1447 (`dba10814`); in a window under about 470 px
  the fixed-height panel overflowed (`d6cd1ea2`). Checked live at 400 and 682 px: no scroll.
- `cargo test -p shadows-core --test contracts` failed on `Only::over` in the codebase contract
  and an unquoted colon in the harness contract, both written earlier on the branch (`45924006`).

**Observed, not fixed:** `GET /api/harnesses/codex/catalog` answered 502 at 20:10 UTC
(`Codex: workspace routing discovery timed out`). Drive E filled to 100% during a test build;
22 older test-binary copies were deleted and Mohammed freed space.

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
