# Prepared context against a plain agent: one feature, built twice

Date: 2026-11-26. Shadows at `c596c3d0` (branch `feature/after-77`), Claude Code 2.1.332.
Answers in part Stage 4's exit check "compare equivalent accepted work with and without
prepared context" (`docs/core/roadmap.md`, "Stage exits"). One run each; not repeated.

## Setup

- **Start:** `testing8` (`F:\shadows_projects_testing\testing8`, Go CLI `water/` + Python
  report `report/`) at `9c10bb2`. A clone of it at the same commit, `testing8-baseline`, for
  the plain run.
- **Feature, same text for both:** "Streaks for the daily water goal": `water streak` prints the
  current run of goal-met days ending today (or yesterday when today is not met yet), `No goal
  set` with exit 0 when there is no goal; the Python report's summary prints `Longest streak: N
  days` when a goal is set; missing days count as not met; existing outputs and the data file
  format unchanged; both suites cover the behaviour.
- **With Shadows:** sent to testing8's Planner thread (Claude Code, Sonnet 5.5, medium, `auto`),
  asking it to plan and run the work with Claude Code executors on Sonnet 5.5 at medium effort.
- **Without:** one Claude Code subagent on Sonnet, given the same feature text, told to work only
  in `testing8-baseline`, run both suites and commit.
- Both were started at 00:43:58 UTC and ran at the same time on the same machine. Nobody
  intervened in either run.

## Results

| | With Shadows | Plain agent |
|---|---|---|
| Wall time | 5 min 40 s (00:43:58 to 00:49:38, the Planner's last turn) | 3 min 34 s |
| Shape | Planner: 7 turns; plan of 6 tasks; executors in three waves (3, then 2 with one overlapping, then 1) | one session, 29 tool calls |
| Executor time per task | 26 to 38 s | — |
| Attempts | 6 of 6 tasks done on the first attempt, no additions | — |
| Failures fixed on the way | 0 reported by the executors | 5: a patch script broke Go string literals (several builds), a usage-text test, three pytest failures (one CRLF patch miss, two wrong expectations in its own tests); it also ran `gofmt -w` over `internal/` by mistake and restored the files |
| Person's interventions | 0 | 0 |
| Commits | 6, one per task | 1 |
| Diff | 16 files, +322 −17 | 13 files, +233 −4 |
| Go tests (`go test -count=1 ./...`) | 8 packages ok | 8 packages ok |
| Python tests (`python -m pytest -q`) | 105 passed | 103 passed |
| Tokens | Planner context 110k to 174k per turn over 7 turns (about 970k, mostly cached); executors not counted | about 102k in all |

Both suites were rerun by the controller, not taken from the reports.

### The same scenario on both built programs

No goal; then goal 3 with three met days and today at 1; two `add`s; `streak extra`; the report
with `--days 5`; a lone met day two days ago; a corrupt file. Both programs printed the same,
line for line, except the usage line: Shadows' lists `streak` (`usage: water
<add|today|reset-today|goal [set N]|streak>`), the plain run kept the old one.

### Code, read by the controller

- **Same rules:** both count the current streak the same way (today unmet falls back to
  yesterday; missing days are not met), and both compute the longest run over the report's
  rows.
- **With Shadows:** the streak use case returns the streak and whether a goal is set, so the CLI
  reads the goal once; the rule has its own file (`domain/streak.go`) with its tests; the
  AppCore wiring and the core have tests; `USAGE.md` and the usage line name the command. The
  changed usage line differs from "keep existing outputs unchanged" read strictly.
- **Plain agent:** the CLI asks `Goal()` and then `Streak()`, which loads the goal again, so the
  "no goal" decision is made in two places; the rule sits in `counts.go`; no wiring test.

## Reading

- On a feature this small the plain agent was faster (about 2 minutes) and far cheaper in
  tokens; the Planner's long thread (more than 500 entries) carries most of Shadows' cost.
- Shadows' executors needed no retries and no fixes, where the plain agent needed five; the
  plan's split put each rule in its layer and gave each its tests.
- The visible result was the same. The quality difference is in structure and tests, not in
  behaviour.
- Not measured: a feature large enough for the plain agent to lose the thread, a Planner in a
  fresh thread, and repeated runs.
