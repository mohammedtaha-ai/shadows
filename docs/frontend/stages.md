# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

F-62 to F-76 and F-68 shipped in PR #49 (`b40e077b`), on `main`: the web half of the AYLUL
agents and panel design, and the AI Studio UI migration that replaced F-68's look. Their
entries are in this file at `b40e077b`. The AYLUL panel plan (`docs/frontend/plans/2026-11-01-aylul-agents-panel-frontend.md` at `b40e077b`)
and the neon plan (`docs/frontend/plans/2026-11-01-neon-clarity.md` at `b40e077b`) are deleted.
F-77 (the whole web client after the migration) was accepted by Mohammed's visual check on 2026-11-03;
its entry is in this file at `a7b4009c`, and the AI Studio plan (`docs/frontend/plans/2026-11-02-ai-studio-ui-migration.md`) at `a7b4009c`.
The migration's facts are in `docs/evidence/2026-11-02-ai-studio-ui-analysis.md` at `f1459dcc`.

F-78 and F-79 (the agent form starts empty; the background tasks panel) shipped in PR #51
(`2790473b`); their entries are in this file at `16da6b8c`.

F-80 (per-tool permissions in the agent form) shipped in PR #52 (`6b2c66ed`); its entry is in
this file at `9c7c7e5a`.
A click during a save is now queued and built on the saved rules (PR #53, `349606bf`).

F-81 to F-85 (agent groups in the main window) shipped in PR #54 (`9e5b9ab2`), with
B-40 to B-46; their entries are in this file at `9e5b9ab2`. The plan was
`docs/frontend/plans/2026-11-03-agent-groups-ui.md` at `9e5b9ab2`.

Planner context, slice 3 (the inspection view): F-86 to F-89 shipped in PR #57 (`1b413fc4`), with
B-59; their entries are in this file at `1b413fc4`. The plan was
`docs/frontend/plans/2026-11-05-planner-context-view.md` at `1b413fc4`.

Structure and depth, slice 1 (PR #59, `26bf739e`) is backend-only. Its only web change is a test
fixture in `web/src/app/design/create-structure.test.tsx`, with no `F-` task.

Structure and depth, slice 2 (PR #60, `b64242a7`): F-90 and F-91 shipped with B-63 to B-70. Their
entries, and the web fix `fb46cac4` outside their `Touch:` lists, are in this file at `b64242a7`.
The plan is at `a4f89386`, deleted after the merge.

Structure and depth, slices 3 to 5 (PR #61, `a4f89386`): F-92 to F-95 shipped with B-71 to B-82.
Their entries are in this file at `a4f89386`. Mohammed tried them in the browser on 2026-11-07.

Still open outside any task: the light palette and the real desktop login-item behaviour need
their own app and OS acceptance.

Finished work: F-50 to F-61 (branch `feature/web-polish`, PR #48) are in this file at `ce31036d`;
F-45 to F-49 (W-11, with B-14 to B-18, PR #47) are in this file at `4f23c717`;
F-28 (each settings form saves only the fields it changed, with the glass
theme selector, PR #46) is in this file at `e95b79c3`; F-22 (the context ring shows the summary only, PR #45) is in this file at
`29ddfd88`; F-20, F-21 and F-33 to F-44 (W-9, W-10, W-12 and W-13, PR #44) are in this
file at `4d5c022d`; F-16, F-17, F-26, F-27 and F-29 to F-32 (W-1 to W-8, PR #43) at
`ca618f89`; F-1 and F-23 to F-25 (PR #41) at `c7e7bac4`; F-2 to F-12, F-14 and F-15 (the
settings dialog, PR #40) at `1a2b601`.

## Tasks

Two composer fixes in PR #61 have no `F-` task: `1b85b6e6` (the textarea grows to
192px) and `e6e6e9cf` (`dir="auto"`, so Arabic runs right to left).

Structure and planning depth: slices 1 to 5 are on `main`; slice 1 needs no web task. The
[workspace behavior](../backend/specs/2026-11-05-structure-and-depth-design.md#6-what-the-person-sees)
is the review target. Before a new `F-` task is dispatched, its shared
request must be accepted and its backend response must exist. Expand the API hooks,
routing, event refresh, contracts and code-map paths first.
Use Roadmap for outcome coverage and Project map for ownership. The old
mockup's `Built` label cannot be used as evidence of completion.

### F-96 · Refresh conversation order after an assistant reply

- Status: done (automated checks; Mohammed owns manual browser acceptance)
- Plan: Direct user request, 2026-11-07; owner spec §13.11 in `docs/core/specs/2026-09-25-planner-workflow-design.md`.
- Read: `web/src/stream/contract.yaml`, `web/src/app/sidebar/contract.yaml`.
- Touch: `web/src/stream/refresh.ts`, `web/src/stream/refresh.test.tsx`, `web/src/stream/contract.yaml`, `web/src/app/sidebar/sidebar.test.tsx`, `web/src/app/sidebar/contract.yaml`, `web/src/api/threads.ts`, `web/src/api/contract.yaml`, this stages file.
- Needs: B-83; `docs/shared/requests/2026-11-07-conversation-reply-order.md` (accepted).
- Done when: affected sidebar/stream tests, typecheck, lint, build, codemap and architecture checks pass. Manual browser acceptance belongs to Mohammed.
- Verified: 164 tests in sidebar, stream, codemap and architecture; typecheck, lint and production build. Vite retains its existing chunk-size warning.

### F-97 · Use Codex in the existing conversation flow

- Status: done
- Plan: [Codex app-server, F-97](../backend/plans/2026-11-07-codex-app-server.md#f-97-use-the-existing-conversation-ui).
- Read: harness controls spec §12.14; `web/src/app/conversation/contract.yaml`; `web/src/api/contract.yaml`; the accepted [Codex Planner request](../shared/requests/2026-11-07-codex-planner.md).
- Touch: the exact F-97 file list in the plan: conversation picker/session/labels and their tests/contract, fake daemon fixture, frontend code map, this stages file and shared request. No new layout or local permission rules. Widened in the whole-branch review to `web/src/test/contract-fixtures.ts`, whose `codexHarness` carries Core's unconfigured listing (its reason and `read-only` mode), and to `web/src/app/project-settings/project-modes.test.tsx` and its `contract.yaml`, whose undecided-modes test no longer uses Codex.
- Needs: B-87; `docs/shared/requests/2026-11-07-codex-planner.md` (accepted).
- Done when: Codex selection survives first send, the harness locks after a turn, server configuration/errors and per-model efforts are shown, text/tools are not duplicated, unsupported capabilities stay unavailable, and context uses reported last-request figures. Focused conversation tests, typecheck, lint, build, codemap and architecture pass; B-88 owns real browser acceptance, run on `main` after the merge.

### F-98 · The task page

- Status: done
- Plan: [F-98](../backend/plans/2026-11-08-executors.md#f-98-the-task-page); design [executors](../backend/specs/2026-11-08-executors-design.md) §4 (mockup A, tabs).
- Touch: the plan's F-98 file list.
- Needs: B-93's routes and generated types; `docs/shared/requests/2026-11-08-task-run-page.md` (accepted).
- Done when: the plan's F-98 tests, typecheck, lint, build, codemap and architecture pass.


## Approved local delivery UI, 2026-11-09

These tasks are blocked until their named backend API exists; no component implements Core readiness policy.

### F-99 · Files view and server status

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 1](plans/2026-11-09-local-delivery-ui.md#task-1-f-99-files-view-and-server-status).
- Owner: web API and design UI.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 1; no other source paths.
- Needs: B-99; accepted shared API/model agreement. Backend must expose its generated API before this task starts.
- Done when: all named RED/GREEN steps and focused verification commands in Task 1 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### F-100 · Preparation status from the Planner's project

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 2](plans/2026-11-09-local-delivery-ui.md#task-2-f-100-preparation-status-from-the-planners-project).
- Owner: web API and design UI.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 2; no other source paths.
- Needs: B-102, F-99; accepted shared API/model agreement. Backend must expose its generated API before this task starts.
- Done when: all named RED/GREEN steps and focused verification commands in Task 2 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### F-101 · Readable task contract and decision pins

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 3](plans/2026-11-09-local-delivery-ui.md#task-3-f-101-readable-task-contract-and-decision-pins).
- Owner: web API, task and workflow UI.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 3; no other source paths.
- Needs: B-106, F-99; accepted shared API/model agreement. Backend must expose its generated API before this task starts.
- Done when: all named RED/GREEN steps and focused verification commands in Task 3 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### F-102 · Attempt brief and scoped question results

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 4](plans/2026-11-09-local-delivery-ui.md#task-4-f-102-attempt-brief-and-scoped-question-results).
- Owner: web API and task UI.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 4; no other source paths.
- Needs: B-109, F-101; accepted shared API/model agreement. Backend must expose its generated API before this task starts.
- Done when: all named RED/GREEN steps and focused verification commands in Task 4 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### F-103 · Check evidence, critic model and acceptance

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 5](plans/2026-11-09-local-delivery-ui.md#task-5-f-103-check-evidence-critic-model-and-acceptance).
- Owner: web API and task UI.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 5; no other source paths.
- Needs: B-114, F-102; accepted shared API/model agreement. Backend must expose its generated API before this task starts.
- Done when: all named RED/GREEN steps and focused verification commands in Task 5 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### F-104 · Repair state and replan controls

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 6](plans/2026-11-09-local-delivery-ui.md#task-6-f-104-repair-state-and-replan-controls).
- Owner: web API and task UI.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 6; no other source paths.
- Needs: B-117, F-103; accepted shared API/model agreement. Backend must expose its generated API before this task starts.
- Done when: all named RED/GREEN steps and focused verification commands in Task 6 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

### F-105 · Browser acceptance of the complete local journey

- Status: blocked (prerequisites below must ship first)
- Plan: [Task 7](plans/2026-11-09-local-delivery-ui.md#task-7-f-105-browser-acceptance-of-the-complete-local-journey).
- Owner: Frontend/controller.
- Read: approved §17.0; docs/shared/requests/2026-11-09-local-delivery.md; the plan's Global Constraints, Interfaces and named owner contracts.
- Touch: exact Create/Modify/Test file list of Task 7; no other source paths.
- Needs: B-118, F-104; accepted shared API/model agreement. Backend must expose its generated API before this task starts.
- Done when: all named RED/GREEN steps and focused verification commands in Task 7 pass; contract/map/status updated; scoped commit recorded. Browser/log acceptance remains distinct.

## Task page: the Planner's review and the Diff tab, 2026-11-11

Plan: [2026-11-11-task-page-review-and-diff.md](plans/2026-11-11-task-page-review-and-diff.md).
Its Global constraints bind every task here.

### F-106 · The task page says when the task waits for the Planner's judgment

- Status: done
- Plan: [F-106](plans/2026-11-11-task-page-review-and-diff.md#f-106--the-task-page-says-when-the-task-waits-for-the-planners-judgment)
- Read: `web/src/app/tasks/contract.yaml`; the `runs:` block of `web/src/api/contract.yaml`; [the planner review request](../shared/requests/2026-11-11-planner-review.md) "What the page can derive".
- Touch: `web/src/api/runs.ts`, `web/src/api/contract.yaml`, `web/src/app/tasks/use-task-run.ts`, `web/src/app/tasks/task-header.tsx`, `web/src/app/tasks/task-page.test.tsx`, `web/src/app/tasks/contract.yaml`, this stages file.
- Needs: none (B-125 is built; no new field).
- Done when: `npx vitest run src/app/tasks` passes with `a_committed_task_waits_for_the_planners_judgment` and `a_task_marked_done_does_not_wait`; typecheck, lint, build, `npx vitest run src/codemap.test.ts src/architecture` pass. One commit, `feat(web): F-106 <goal>`.

### F-107 · The Diff tab: the chosen file above, the changed files in a small box below

- Status: done
- Plan: [F-107](plans/2026-11-11-task-page-review-and-diff.md#f-107--the-diff-tab-the-chosen-file-above-the-changed-files-in-a-small-box-below)
- Read: `web/src/app/tasks/contract.yaml`; the `tasks` rows of `docs/frontend/codebase/README.md`; `attemptDiffQuery` in `web/src/api/runs.ts`.
- Touch: `web/src/app/tasks/diff-tab.tsx`, `web/src/app/tasks/file-patch.tsx` (new), `web/src/app/tasks/task-page.test.tsx`, `web/src/app/tasks/contract.yaml`, `docs/frontend/codebase/README.md`, this stages file.
- Needs: none (`GET /api/attempts/{id}/diff` as it is).
- Done when: `npx vitest run src/app/tasks` passes with `diff_opens_the_first_file_and_switches_on_click` and `diff_shows_a_cut_marker_as_plain_text`; typecheck, lint, build, `npx vitest run src/codemap.test.ts src/architecture` pass. One commit, `feat(web): F-107 <goal>`.

### F-110 · A task stopped on the cap does not wait for the Planner's judgment

- Status: done
- Plan: main session's ruling, 2026-11-11 (B-126 as planned in `2d48930c`: a fourth `task_amend` stops the waiting attempt, frees its scope, and Start is allowed again). "Waiting for the Planner's judgment" holds only when `run.latest` is committed, `run.done` is false **and** `run.start.code === 'task_busy'`. No new Core field. A capped task (committed, not done, Start allowed) shows nothing extra until F-108, and is not read again every two seconds.
- Read: `awaitingJudgment` in `web/src/api/runs.ts`; the `runs:` block of `web/src/api/contract.yaml`; `web/src/app/tasks/contract.yaml`.
- Touch: `web/src/api/runs.ts` (`awaitingJudgment` and its doc comment), `web/src/api/contract.yaml`, `web/src/app/tasks/task-page.test.tsx` (the F-106 waiting test's fixture gets `start: { allowed: false, code: 'task_busy', reason: ... }`; one new test), `web/src/app/tasks/contract.yaml` (the waiting obligation names the `task_busy` condition and the new test; also delete the duplicated Diff tab obligation F-107 committed twice), this stages file.
- Needs: F-106.
- Done when: `npx vitest run src/app/tasks` passes, with `a_task_stopped_on_the_cap_does_not_wait` (latest committed, `done: false`, `start.allowed: true`, codes null: no badge, and the task is not read again after two seconds) and the existing waiting tests; typecheck, lint, build, `npx vitest run src/codemap.test.ts src/architecture` pass. One commit, `fix(web): F-110 <goal>`.

### F-108 · An attempt's addition on the Report tab

- Status: done. B-126 built at `cecdc0ca`; `npm run gen:api` passed (1 generation). From `web/`: `npx vitest run src/app/tasks src/codemap.test.ts src/architecture` passed (105 tests, 3 files), including the plain-text addition test and cap header checks; `npm run typecheck`, `npm run lint` and `npm run build` passed (1 run each; build transformed 3819 modules, with the chunk-size warning). Automated fixture checks only; no live browser acceptance run.
- Plan: [F-108](plans/2026-11-11-task-page-review-and-diff.md#f-108--an-attempts-addition-on-the-report-tab-blocked-on-b-126)
- Read: `web/src/app/tasks/contract.yaml`; [the planner review request](../shared/requests/2026-11-11-planner-review.md), its backend answer for the cap of 3.
- Touch: `web/src/api/schema.d.ts` (only through `npm run gen:api`), `web/src/app/tasks/report-tab.tsx`, `web/src/app/tasks/task-page.test.tsx`, `web/src/app/tasks/contract.yaml`, this stages file. The cap needs no field: a task whose latest attempt is committed, carries an `addition`, is not done and whose `start.allowed` is true shows "Stopped after the Planner's third addition; back to you" in the header (one function in `web/src/api/runs.ts`, beside `awaitingJudgment`; add `web/src/app/tasks/task-header.tsx` and `web/src/api/contract.yaml` to this list).
- Needs: B-126; `docs/shared/requests/2026-11-11-planner-review.md` (accepted).
- Done when: `npx vitest run src/app/tasks` passes with `the_report_shows_the_planners_addition_as_plain_text`; typecheck, lint, build, codemap and architecture pass. One commit.

### F-109 · The whole file with its changes in the Diff tab

- Status: blocked (B-131 not built)
- Plan: [F-109](plans/2026-11-11-task-page-review-and-diff.md#f-109--the-whole-file-with-its-changes)
- Read: `docs/shared/requests/2026-11-11-attempt-file-content.md` (accepted); `web/src/app/tasks/contract.yaml`; the `runs:` block of `web/src/api/contract.yaml`; the `tasks` rows of `docs/frontend/codebase/README.md`.
- Touch: `web/src/api/schema.d.ts` (only through `npm run gen:api`), `web/src/api/runs.ts`, `web/src/api/contract.yaml`, `web/src/app/tasks/diff-tab.tsx`, `web/src/app/tasks/file-view.tsx` (new), `web/src/app/tasks/file-patch.tsx` (deleted), `web/src/app/tasks/task-page.test.tsx`, `web/src/app/tasks/contract.yaml`, `docs/frontend/codebase/README.md`, this stages file.
- Needs: B-131 committed (`api/openapi.json` holds `AttemptFile`, `GET /api/attempts/{id}/files` and `FileDiff.hunks`); F-107.
- Done when: `npx vitest run src/app/tasks` passes with `diff_opens_the_first_file_and_switches_on_click`, `diff_shows_the_whole_file_with_its_changes_marked`, `diff_shows_no_text_for_a_deleted_file_with_its_hunks`, `diff_says_when_core_shortened_the_file` and `diff_shows_cores_refusal_in_the_file_area`; typecheck, lint, build, `npx vitest run src/codemap.test.ts src/architecture` pass. One commit, `feat(web): F-109 <goal>`.

## Executor notices as the Planner's activity, 2026-11-11

Plan: [2026-11-11-shadows-notices.md](plans/2026-11-11-shadows-notices.md). Its Global
constraints bind both tasks. Backend: B-134 on
[the notices request](../shared/requests/2026-11-11-shadows-notices.md) (accepted, amended at `8b5a5a20` by Mohammed's rulings).
Built by `frontend-builder`.

### F-111 · An executor's notice draws as an item of the Planner's activity

- Status: blocked (B-134 not built)
- Plan: [F-111](plans/2026-11-11-shadows-notices.md#f-111--an-executors-notice-draws-as-an-item-of-the-planners-activity)
- Read: the plan's "What Core gives" and "Where the item sits"; `web/src/app/conversation/contract.yaml`; `web/src/stream/contract.yaml`; the `runs:` block of `web/src/api/contract.yaml`; `Entry` in `entry.tsx`, `EntryRow` in `messages.tsx`, `subagentOf` and `copyText` in `tool-text.ts`, `readSubagentCard` in `web/src/stream/frames.ts`, `ToolGroup` in `tool-group.tsx` (the look to match).
- Touch: `web/src/api/schema.d.ts` (only through `npm run gen:api`), `web/src/api/runs.ts`, `web/src/api/contract.yaml`, `web/src/stream/frames.ts`, `web/src/stream/contract.yaml`, `web/src/app/conversation/tool-text.ts`, `web/src/app/conversation/notice-item.tsx` (new), `web/src/app/conversation/notice-item.test.tsx` (new), `web/src/app/conversation/reply-text.tsx` (the `compact` prop only), `web/src/app/conversation/entry.tsx`, `web/src/app/conversation/messages.tsx` (`EntryRow` footer only), `web/src/app/conversation/contract.yaml`, `docs/frontend/codebase/README.md`, this stages file.
- Needs: B-134 committed (`api/openapi.json` holds `AttemptNotice` with `model`, `Notice` with `executor`, and `QueuedMessage.notice`).
- Done when: `npx vitest run src/app/conversation/notice-item.test.tsx src/app/conversation/todo-card.test.tsx src/app/conversation/reply-text.test.tsx` passes, with `a_notice_draws_as_an_activity_item_not_a_bubble`, `a_failed_notice_shows_its_reason_and_report`, `a_long_report_is_cut_with_show_more`, `a_notice_without_a_readable_card_shows_its_body_under_executor`, `the_header_closes_and_reopens_the_report` and `an_old_notice_from_the_person_stays_a_bubble`; typecheck, lint, build, `npx vitest run src/codemap.test.ts src/architecture` pass. One commit, `feat(web): F-111 <goal>`.

### F-112 · A waiting notice is not drawn as the person's waiting message

- Status: blocked (B-134 not built)
- Plan: [F-112](plans/2026-11-11-shadows-notices.md#f-112--a-waiting-notice-is-not-drawn-as-the-persons-waiting-message)
- Read: `web/src/app/conversation/contract.yaml`; `WaitingMessages` in `waiting-messages.tsx`; `noticeTitle` in `notice-item.tsx`; `readAttemptNotice` in `web/src/stream/frames.ts`.
- Touch: `web/src/app/conversation/waiting-messages.tsx`, `web/src/app/conversation/waiting-messages.test.tsx`, `web/src/app/conversation/contract.yaml`, this stages file.
- Needs: F-111; B-134 (Mohammed ruled 2026-11-11: Core refuses removing a notice with 409 `QUEUED_NOTICE_KEPT`, the web shows no Remove).
- Done when: `npx vitest run src/app/conversation/waiting-messages.test.tsx` passes with `a_waiting_notice_is_not_drawn_as_the_persons_message` and the existing queue test; typecheck, lint, build, `npx vitest run src/codemap.test.ts src/architecture` pass. One commit, `feat(web): F-112 <goal>`.

### F-113 · Each Planner turn is one collapsible box

- Status: blocked (F-111 first)
- Plan: [F-113](plans/2026-11-11-shadows-notices.md#f-113--each-planner-turn-is-one-collapsible-box)
- Read: the plan's F-113; `web/src/app/conversation/contract.yaml`; `Messages`, `ToolRow` and `EntryRow` in `messages.tsx`; `Row`, `activityRows` and `withLiveSteps` in `activity-rows.ts`; `ToolGroup` in `tool-group.tsx` (the open/fold behaviour to match); `noticeOf` in `tool-text.ts`; `noticeName` in `notice-item.tsx` (the summary's name).
- Touch: `web/src/app/conversation/turn-blocks.ts` (new), `web/src/app/conversation/turn-blocks.test.ts` (new), `web/src/app/conversation/turn-box.tsx` (new), `web/src/app/conversation/turn-box.test.tsx` (new), `web/src/app/conversation/messages.tsx`, `web/src/app/conversation/conversation.test.tsx`, `web/src/app/conversation/streamed-reply.test.tsx`, `web/src/app/conversation/tool-group.test.tsx`, `web/src/app/conversation/plan-in-conversation.test.tsx` (these four only to open a finished turn's box before reading inside it), `web/src/app/conversation/contract.yaml`, `docs/frontend/codebase/README.md`, this stages file.
- Needs: F-111. Nothing from Core: entries carry `operation_id`; no shared request.
- Done when: `npx vitest run src/app/conversation` passes, with `a_finished_turn_folds_its_notice_tools_and_text_and_keeps_its_final_answer`, `a_turn_with_no_activity_has_no_box`, `a_proposal_card_stays_outside_the_box`, `the_person_prompt_stays_above_the_box`, `a_finished_turn_shows_one_summary_line_and_opens_on_click` and `a_running_turn_is_open_and_folds_when_it_ends`; typecheck, lint, build, `npx vitest run src/codemap.test.ts src/architecture` pass. One commit, `feat(web): F-113 <goal>`.
