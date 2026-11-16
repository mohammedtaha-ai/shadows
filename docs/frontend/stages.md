# Frontend stages

Where frontend work is now. Masada keeps this file; builders read their task
here and set its status. Task shape: [`AGENTS.md`](./AGENTS.md#a-task-in-stagesmd).

## Now

Slice 6's web tasks, F-121 to F-124 below, in order. Their backend (B-152 `160cf9ef`, B-154
`0abefbef`, B-105 `9e5bbc8d`) is committed and `api/openapi.json` carries it.

### Shipped

A shipped task's entry, and its plan once deleted, are in this file in Git at the commit named.

| PR | Work | Tasks | Entries at |
|---|---|---|---|
| #40 | The settings dialog | F-2 to F-12, F-14, F-15 | `1a2b601` |
| #41 | Architecture restructuring, web side | F-1, F-23 to F-25 | `c7e7bac4` |
| #43 | W-1 to W-8 | F-16, F-17, F-26, F-27, F-29 to F-32 | `ca618f89` |
| #44 | W-9, W-10, W-12, W-13 | F-20, F-21, F-33 to F-44 | `4d5c022d` |
| #45 | The context ring shows the summary only | F-22 | `29ddfd88` |
| #46 | Each settings form saves only what changed | F-28 | `e95b79c3` |
| #47 | W-11, with B-14 to B-18 | F-45 to F-49 | `4f23c717` |
| #48 | Web polish | F-50 to F-61 | `ce31036d` |
| #49 | AYLUL agents and panel, and the AI Studio UI migration | F-62 to F-76, F-77 (accepted 2026-11-03) | `b40e077b`, `a7b4009c` |
| #51 | The agent form starts empty; the background tasks panel | F-78, F-79 | `16da6b8c` |
| #52, #53 | Per-tool permissions in the agent form | F-80 | `9c7c7e5a` |
| #54 | Agent groups in the main window | F-81 to F-85 | `9e5b9ab2` |
| #57 | Planner context, the inspection view | F-86 to F-89 | `1b413fc4` |
| #60, #61 | Structure and depth, slices 2 to 5 (tried in the browser 2026-11-07) | F-90 to F-95 | `b64242a7`, `a4f89386` |
| #62 to #64 | Conversation order, Codex, the task page | F-96, F-97, F-98 | `3a443ea0`, `217d7540`, `c71b263d` |
| #65 | Task page review, the Diff tab, the Planner's notices | F-106 to F-113 | `617d806f` |
| #67 | Conversation images, the question card, Settings › Code | F-114 to F-116 | `db760593` |
| #68 | Attempt briefs and contract conflicts on the task page and in the Planner's notices | F-102, F-117 to F-120 (skeleton loading and task-page follow-ups) | `d73bb646` |
| #69 | Checks on the task's Report tab, the done reason under its title, and replies in the conversation | F-103 | `4031168f` |

Dropped without being built: F-99 and F-101 (covered by F-102's brief panel and F-116's server list; the rest is in [future](../core/future.md)), F-100 (with B-102), F-104 and F-105 (with the repair plan; slice 6 replans them). Their entries and plan are at `d997393a`.

PRs #59 and #66 have no `F-` task. Two composer fixes in PR #61 have none either: `1b85b6e6`
(the textarea grows to 192px) and `e6e6e9cf` (`dir="auto"`, so Arabic runs right to left).

### Known and left

- The light palette and the real desktop login-item behaviour need their own app and OS acceptance.
- The image tiles' look in the browser (F-114) is not recorded.

### Before a new task

Its shared request must be accepted and its backend response must exist. Expand the API hooks,
routing, event refresh, contracts and code-map paths first. The old mockup's `Built` label is not
evidence of completion.

## Slice 6: stop control, rechecks and decisions

Shared request: [slice 6](../shared/requests/2026-11-16-slice-6-stop-control.md), accepted
(`9dbb015c`, codes corrected `13d0d9e7`). Backend plan:
[slice 6](../backend/plans/2026-11-16-slice-6-stop-control.md). There is no frontend plan file:
each task below is complete, and the request's Fields, Errors and Frontend reply are binding.

Every task starts by running `npm run gen:api` from `web/` (its backend task has changed
`api/openapi.json`); `src/api/schema.d.ts` is never edited by hand. A task whose generated schema
lacks a field it needs stops and reports. Look: each new card takes the look of the existing card
named in its task (`surface-panel`, the same header row, text sizes and tokens); no new colour.

### F-121 · Interrupted and stopped attempts show in the Planner's conversation

- Status: done
- Plan: the request, "B-152: two outcomes of `AttemptNotice`" and the Frontend reply.
- Read: `web/src/stream/contract.yaml`; `web/src/app/conversation/contract.yaml`;
  `readAttemptNotice` in `web/src/stream/frames.ts`; `NoticeItem` in
  `web/src/app/conversation/notice-item.tsx`.
- Do:
  - `readAttemptNotice` accepts `outcome` `interrupted` and `stopped` (today it returns `null`
    for anything but `committed` and `failed`, so these cards would draw as bare text), and reads
    `left_files` as `string[] | null` (any other value is `null`).
  - `NoticeItem` heads these two cards "attempt N interrupted" and "attempt N stopped by you"
    (`noticeTitle` gives the same words). The body lists `left_files` in mono, one per line,
    under "Uncommitted files"; `[]` shows "No uncommitted files"; `null` shows
    "Uncommitted files: unknown"; then one line: "A new Start continues from these files."
    `interrupted` uses the warning tone on its outcome word; `stopped` the muted one.
  - The Planner-facing body text is never shown for these cards.
- Touch: `web/src/api/schema.d.ts` (generated); `web/src/stream/frames.ts`;
  `web/src/stream/frames.test.ts`; `web/src/stream/contract.yaml`;
  `web/src/app/conversation/notice-item.tsx`; `web/src/app/conversation/notice-item.test.tsx`;
  `web/src/app/conversation/contract.yaml`; `docs/frontend/stages.md` (this entry).
- Needs: B-152 done.
- Done when: new tests pass: in `frames.test.ts`
  `reads_an_interrupted_notice_with_its_left_files` and `left_files_of_another_shape_is_null`; in
  `notice-item.test.tsx` `an_interrupted_notice_lists_the_files_it_left`,
  `a_stopped_notice_with_no_files_says_so`, `unknown_left_files_say_unknown`. Run
  `npx vitest run src/stream/frames.test.ts src/app/conversation/notice-item.test.tsx`, then the
  checks of AGENTS rule 5. One commit, `feat(web): F-121 interrupted and stopped attempts`.

### F-122 · The recheck card: done tasks whose checks no longer pass

- Status: todo
- Plan: the request, "B-154: the recheck card" and the Frontend reply.
- Read: `web/src/stream/contract.yaml`; `web/src/app/conversation/contract.yaml`; `noticeOf` in
  `web/src/app/conversation/tool-text.ts`; `NoticeItem`; the checks row of the task's Report tab
  (`web/src/app/tasks/checks-panel.tsx`); the task route in `web/src/router.tsx`
  (`/projects/$projectId/workflows/$workflowId/versions/$version/tasks/$task`).
- Do:
  - `web/src/api/runs.ts` exports `RecheckNotice` and `RecheckFailure` from the generated schema.
  - `readRecheckNotice(value): RecheckNotice | null` in `frames.ts`, exported from
    `web/src/stream/index.ts`: a card with a string `recheck_id` and an array `failed`; anything
    else is `null`.
  - `noticeOf` gives a Shadows notice its recheck card when `readRecheckNotice` reads one (today a
    Shadows notice always has `card: null`). Its return type widens to carry it; `copyText`,
    `turn-blocks.ts` and `messages.tsx` keep working unchanged through `noticeOf`.
  - New `RecheckNoticeItem` in `recheck-notice.tsx`, drawn by `NoticeItem` for that card. Header:
    "Shadows · recheck after T{cause_task} (version {cause_version})". Body: "These done tasks
    no longer pass:", then per failure a link "T{task} · {title}" to the task route with the
    failure's own `workflow_id` and `version`, and its checks drawn as the task page draws them
    (reuse the checks row; if it cannot be reused without changing `checks-panel.tsx`, stop and
    report). "Their done marks stand." ends it.
- Touch: `web/src/api/schema.d.ts` (generated); `web/src/api/runs.ts`;
  `web/src/api/contract.yaml`; `web/src/stream/frames.ts`; `web/src/stream/frames.test.ts`;
  `web/src/stream/index.ts`; `web/src/stream/contract.yaml`;
  `web/src/app/conversation/tool-text.ts`; `web/src/app/conversation/notice-item.tsx`;
  `web/src/app/conversation/recheck-notice.tsx` (new);
  `web/src/app/conversation/recheck-notice.test.tsx` (new);
  `web/src/app/conversation/contract.yaml`; `docs/frontend/codebase/README.md` (the new file's
  row); `docs/frontend/stages.md` (this entry).
- Needs: B-154 done; F-121 (same `frames.ts` and `notice-item.tsx`).
- Done when: new tests pass: in `frames.test.ts` `reads_a_recheck_card_by_its_recheck_id`; in
  `recheck-notice.test.tsx` `a_recheck_lists_each_failing_task_with_its_checks` and
  `a_failing_task_links_to_its_own_version`; the existing `notice-item.test.tsx` and
  `conversation.test.tsx` stay green. Run
  `npx vitest run src/stream/frames.test.ts src/app/conversation/recheck-notice.test.tsx src/app/conversation/notice-item.test.tsx src/app/conversation/conversation.test.tsx`,
  then the checks of AGENTS rule 5. One commit, `feat(web): F-122 the recheck card`.

### F-123 · A proposed decision with Accept in the Planner's conversation

- Status: todo
- Plan: the request, "B-105: decisions", its Errors and the Frontend reply.
- Read: `web/src/api/contract.yaml`; `web/src/app/conversation/contract.yaml`;
  `web/src/stream/contract.yaml`; `VisionProposalCard` and `useProposalApproval` /
  `ProposalApproval` (`web/src/app/conversation/vision-proposal-card.tsx`,
  `proposal-approval.tsx`); `classifyRefresh` in `web/src/stream/refresh.ts`.
- Do:
  - In `web/src/api/design.ts`: `decisionsQuery(projectId)` (`GET /api/projects/{id}/decisions`)
    and `acceptDecision(projectId, decisionId, version, commandId)`
    (`POST …/decisions/{decision}/accept`), with a key under the design keys; `DecisionRecord`
    from the generated schema.
  - `classifyRefresh` refetches the decisions on `DecisionProposed` and `DecisionAccepted`.
  - New `DecisionProposalCard` (`decision-proposal-card.tsx`), drawn by `entry.tsx` for
    `entry.kind === 'DecisionProposal'`, with the look of `VisionProposalCard`: "Decision
    {id} · version {version}", its title, decision, rationale and source (`dir="auto"`). The
    live state comes from `decisionsQuery`, never from the card: `proposed` and the latest
    revision → an Accept button; `accepted` → "Accepted by {approved_by}" (`approved-by.tsx`);
    `superseded` → "Superseded". `DECISION_STALE` (409) shows "This decision changed. Reload to
    see its latest version." and refetches; any other error shows Core's message. One
    `command_id` per click, kept across a retry (`web/src/api/command-id.ts`).
  - Reuse `useProposalApproval` / `ProposalApproval` when their signature fits. Changing them for
    the version argument is allowed in `proposal-approval.tsx` only if the vision, map and
    roadmap card tests stay green.
- Touch: `web/src/api/schema.d.ts` (generated); `web/src/api/design.ts`;
  `web/src/api/design.test.ts`; `web/src/api/contract.yaml`; `web/src/stream/refresh.ts`;
  `web/src/stream/refresh.test.tsx`; `web/src/stream/contract.yaml`;
  `web/src/app/conversation/entry.tsx`; `web/src/app/conversation/proposal-approval.tsx`;
  `web/src/app/conversation/decision-proposal-card.tsx` (new);
  `web/src/app/conversation/decision-proposal-card.test.tsx` (new);
  `web/src/app/conversation/contract.yaml`; `docs/frontend/codebase/README.md` (the new file's
  row); `docs/frontend/stages.md` (this entry).
- Needs: B-105 done.
- Done when: new tests pass: in `design.test.ts` `accept_sends_the_version_and_command_id`; in
  `refresh.test.tsx` `decision_events_refetch_the_decisions`; in `decision-proposal-card.test.tsx`
  `a_proposed_decision_can_be_accepted`, `an_accepted_decision_shows_who_accepted_it`,
  `a_superseded_decision_has_no_accept`, `a_stale_accept_asks_to_reload`; the vision, map and
  roadmap card tests stay green. Run
  `npx vitest run src/api/design.test.ts src/stream/refresh.test.tsx src/app/conversation/decision-proposal-card.test.tsx src/app/conversation/vision-proposal-card.test.tsx src/app/conversation/map-proposal-card.test.tsx src/app/conversation/roadmap-proposal-card.test.tsx`,
  then the checks of AGENTS rule 5. One commit, `feat(web): F-123 proposed decisions with Accept`.

### F-124 · The attempt brief shows the decisions its task follows

- Status: todo
- Plan: the request, "B-105: decisions", row `context.decisions`.
- Read: `web/src/app/tasks/contract.yaml`; `BriefPanel` in `web/src/app/tasks/brief-panel.tsx`.
- Do: under the resolved contracts, a "Decisions" list when `context.decisions` is present:
  "{id} v{version} · {title}" with the decision text below and the revision's state as saved in
  the brief (`superseded` shown as "superseded since", so a frozen pin to an older revision is
  plain). Absent means no section.
- Touch: `web/src/app/tasks/brief-panel.tsx`; `web/src/app/tasks/brief-panel.test.tsx`;
  `web/src/app/tasks/contract.yaml`; `docs/frontend/stages.md` (this entry).
- Needs: B-105 done; F-123 (it ran `npm run gen:api`).
- Done when: new tests pass in `brief-panel.test.tsx`: `the_brief_lists_its_pinned_decisions` and
  `a_superseded_pin_is_labelled`. Run `npx vitest run src/app/tasks/brief-panel.test.tsx`, then the
  checks of AGENTS rule 5. One commit, `feat(web): F-124 decisions in the attempt brief`.
