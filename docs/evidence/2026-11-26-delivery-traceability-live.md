# Delivery traceability, run live on a new project

Date: 2026-11-26. Shadows at `e8a72ae6` (branch `feature/delivery-traceability`), daemon built in
place and started from `.claude/run-daemon.cmd`, web on Vite. Answers B-242
([plan Task 10](../backend/plans/2026-11-26-delivery-traceability.md)). One run.

## Setup

- Project `testing9` (`2a743d53-c1da-49e5-b69b-118eaf4f7dfb`) at
  `F:\shadows_projects_testing\testing9`, an empty git repository. A Go `todo` CLI.
- Planner: Claude Code, Sonnet 5.5, medium, `auto`. Spoken to in English.
- Vision (7 cards), map and roadmap proposed by the Planner and approved by the person. The
  standards require a database part, so the map holds three parts, not the two first asked for:
  Todo CLI (`todo/`, backend, `d4ad1383`), Storage (`todo/internal/store`, database, nested,
  `4df1a7d6`) and Fixtures (`fixtures/`, read only, `b4a8c38c`); api and frontend waived.
- Roadmap: one outcome, "Add and list todos" (`743c5fd8-1966-4f38-98f9-d033e30b145b`), four
  acceptance items.

## Observed

| Check | Result |
|---|---|
| Plan from here | Roadmap's link opened a new conversation (`f8d784ec`) showing "Anchored at Add and list todos". The person asked for a one-task plan and named no outcome. |
| Link at draft start | Plan `ff7d371c`, version `ea89dd03` v1 (Draft). Events 6560 `WorkflowDraftStarted` and 6561 `ProjectDesignChanged` (`changed_outcomes: [743c5fd8]`, revision 10) came from the one start. The outcome's revision went 1 → 2 and listed the plan. |
| Before approval | Roadmap "Related tasks" showed "Todo add and list · Draft". `delivery.approval_missing` was empty: parts came from the task's scope (reads `fixtures/**` → Fixtures; writes `todo/**` → Todo CLI, `fixtures/two-tasks.json` → Fixtures). |
| Ordinary draft | A conversation with no anchor (`e7a531f0`) drafted "Todo count" (`ae36b41b`). `approval_missing: ["outcome"]`. A person approval answered `STAGE_BLOCKED` "plan has no roadmap outcome: link it on the Roadmap page". |
| Link from the editor | The outcome editor, opened after the anchored draft, already showed "Todo add and list" ticked. Ticking "Todo count" and saving, then approving, froze it (19:03:18 UTC). Roadmap then listed its T1 as Not started without a reload. |
| Last link kept | Unticking "Todo count" and saving answered `STAGE_BLOCKED` "plan 18a44dbb-… has an approved version and keeps at least one roadmap outcome". |
| Planner approval | The person asked the anchored Planner to approve and run. `approved_by.agent` is `planner:claude-code` with the request entry; the gate passed. |
| Execution | T1 attempt `c2e695b6` committed `2cce258` in testing9; the Planner marked T1 done at 19:06:07 UTC. `go test ./...` in `todo/`: `cli` and `infrastructure/jsonstore` ok. `git status fixtures` clean. |
| Executor brief | `/api/attempts/c2e695b6…/brief` context: `outcomes` = [Add and list todos], `ownership` = the same reads and writes as the plan read. |
| State, outcome | `outcome_id` selector: 2 tasks, 1 done, 1 not started; "Todo add and list" T1 done (committed), "Todo count" T1 not done. Project `task_totals` the same, counted once. |
| State, parts | Todo CLI: T1 `write`. Fixtures: T1 `read_write`. Storage: the plan listed (explicit link), no task. |
| Planner tool | Asked to read `project_state` with the outcome id, the Planner reported the same counts and plans, and said on its own that "Todo count" covers none of the outcome's acceptance items. |
| Roadmap view | "1 of 2 tasks done · 0 started · 1 not started"; task links go to `/workflows/{id}/versions/1/tasks/1`. |

Outcome acceptance is not recorded by Shadows (the OPEN block in §18.3). Read by the controller
against the four items: add and list are covered by the `cli` and `jsonstore` tests; the
missing-file and fixtures-read-only items are covered by those tests and by the clean fixtures
folder. This is an observation, not an accepted outcome.

## Findings

1. **The Approve bar contradicts itself.** For "Todo count" it showed "✓ Nothing blocks approval."
   above "Link a roadmap outcome". The first line reads the plan's own blockers only.
2. **The last-link refusal names the plan by id**, not by title.
3. **A broad write scope does not reach a nested part.** T1 writes `todo/**`; `layout.owners`
   gives Todo CLI only, so Storage (`todo/internal/store`) shows no task although the task could
   write there. Here the executor wrote the store under `todo/internal/infrastructure/jsonstore`,
   not the Storage part's folder; that drift is the map's, not this feature's.
4. **Linking to pass the gate is visible but allowed.** "Todo count" was linked to an outcome it
   does not serve, only for this test. The gate accepted it; the Planner flagged it when asked.
   B-243's prompt rule asks the Planner not to do this; nothing stops a person.
5. "Todo count"'s T1 writes `**`, which owns no part; the plan read shows it unmapped, as designed.
