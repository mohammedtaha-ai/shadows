# Slice 6: stop and interrupt cards, the recheck card, project decisions

- Status: accepted
- Asked by: Masada, 2026-11-16
- Screen: the Planner's conversation (`web/src/app/conversation/`), the task page's brief
  (`web/src/app/tasks/brief-panel.tsx`); backend plan
  [`docs/backend/plans/2026-11-16-slice-6-stop-control.md`](../../backend/plans/2026-11-16-slice-6-stop-control.md)
  (B-152, B-154, B-105). Web tasks start at F-121.

## Need

In the Planner's conversation Mohammed must see three new things without reading the Planner's
text:
- **B-152:** an attempt that was interrupted by a restart or stopped by him, with the files it
  left uncommitted.
- **B-154:** the result of a recheck, with each done task whose checks no longer pass.
- **B-105:** a project decision the Planner proposed, with an Accept button.

On the task page's brief he sees the decisions the task follows. The web draws only what the
cards and routes give; it rebuilds nothing from a body text, which is written for the Planner.

## Fields

### B-152: two outcomes of `AttemptNotice`

| Call | Sends | Gets back |
|---|---|---|
| The notice entry (`UserMessage`, author `Executor`), card `AttemptNotice` | — | `outcome: "interrupted" \| "stopped"`, and **one new field**: `left_files: string[] \| null` (the attempt's uncommitted paths inside its writes; `null` when git could not tell) |

Why the new field: today the card has no place for the uncommitted files, and the body is an
instruction to the Planner ("Tell the person; start it again only if they ask"), not text for the
person. The web shows "Interrupted" and "Stopped by you" with the files listed, and a note that a
new Start continues from them. `reason` and `report` stay `null` for these two outcomes.

### B-154: the recheck card

| Call | Sends | Gets back |
|---|---|---|
| The notice entry (`UserMessage`, author `Shadows`) | — | card `RecheckNotice { plan_id, cause_workflow_id, cause_version, cause_task, recheck_id, failed: RecheckFailure[] }`, `RecheckFailure { workflow_id, version, task, title, attempt_id, checks: CheckResult[] }` |

Asked: `RecheckNotice` and `RecheckFailure` named in `api/openapi.json` (`components.schemas`), as
`AttemptNotice` and `AgentReplyNotice` are, so the generated types name them. `Notice.card` stays
an open object. The web tells the card from a Shadows notice by its `recheck_id`, never by the body.
It draws "After T{cause_task} (version {cause_version}) was marked done, these no longer pass",
each failing task with its checks (the existing checks row), and a link to each task.

### B-105: decisions

| Call | Sends | Gets back |
|---|---|---|
| A new thread entry kind `DecisionProposal`, written when `decision_propose` succeeds | — | card `DecisionRecord` |
| `GET /api/projects/{id}/decisions` | — | `DecisionRecord[]`, by id then version |
| `POST /api/projects/{id}/decisions/{decision}/accept` | `{ command_id: string, version: integer }` | `DecisionRecord` (now `accepted`) |
| `GET /api/attempts/{id}/brief` | — | `context.decisions: DecisionRecord[]` (absent when empty) |

`DecisionRecord { id, version, project_id, title, decision, rationale, source, state: "proposed" | "accepted" | "superseded", approved_by: ApprovedBy | null, created_at }`.

Why an entry kind, not the tool call's result: a `ToolCall` entry keeps only the tool's name and
status, not its output, so the web cannot draw a card from it. The other proposals already work
this way (`VisionProposal`, `MapProposal`, `RoadmapProposal`). The card's Accept button reads the
decision's current state from `GET …/decisions`, so a card whose revision is already accepted or
superseded shows that state instead of the button.

Also asked: an event on the project stream when a decision is proposed or accepted, so an open
card and the list refresh. Name it in the answer; the web refetches `…/decisions` on it.

## Errors

| When | The screen shows |
|---|---|
| Accept on a revision that is not the latest, or not `proposed` (`decision_stale`) | "This decision changed. Reload to see its latest version." and the card refetches |
| Accept replayed with the same `command_id` | The same accepted record, no error |
| `left_files` is `null` | "Uncommitted files: unknown" |
| A `RecheckFailure` whose task no longer exists in the plan | The row without its link |

## Backend answer

Hamoud, 2026-11-16: **as asked**, with three precisions. Each change is in its owner, and the
plan's tasks now name the extra files.

**B-152 (Runs, `crates/core-runs/contract.yaml`): as asked.**
- `AttemptNotice.left_files: string[] | null` (serde default, so saved notices still read).
- For `interrupted` and `stopped` it holds the attempt's uncommitted paths inside its writes:
  `[]` when there are none, `null` when git could not tell. For every other outcome it is `null`.
- The card is saved with the notice before its first enqueue, so a resend carries the same list.
- `reason` and `report` stay `null` for these two outcomes; `report` stays `null` even when the
  executor left a last message, which goes only to the Planner's body.
- `AttemptNotice` changes, so `api/openapi.json` is regenerated in B-152.

**B-154 (Runs): as asked.**
- `RecheckNotice` and `RecheckFailure` are registered in `components.schemas`; `Notice.card`
  stays an open object.
- The card is told by `recheck_id`.
- A `RecheckFailure` names its own `workflow_id` and `version`, so its task always exists in
  that version. A link to the plan's latest version may not find it; that is the row without a
  link in your Errors table.

**B-105 (Design, with Threads and Events): as asked.**
- `decision_propose` writes a `DecisionProposal` thread entry (a new `ThreadEntryKind`, owned by
  Threads) in the same write as the revision. Its card is the `DecisionRecord` as proposed; the
  live state is read from `GET …/decisions`, as with `VisionProposal`.
- Project stream: `DecisionProposed` and `DecisionAccepted`, identifiers only:
  `{ project_id, decision_id, version }`. Each is appended inside its own write. Accepting a new
  version also supersedes the old one in that write, and sends no third event: refetch the list
  on `DecisionAccepted`.
- `POST …/accept`: a replay with the same `command_id` answers the same record.
  `decision_stale` is HTTP 409 with code `decision_stale`; `invalid_decision` is 422.
- `context.decisions` is absent when empty, in the brief as in `TaskContext`.

Status: answered.

## Frontend reply

Masada, 2026-11-16: **accepted** as answered.
- `left_files`: `[]` shows "No uncommitted files"; `null` shows "Uncommitted files: unknown".
- The web refetches `…/decisions` on `DecisionProposed` and on `DecisionAccepted`; a superseded
  card learns its state from that refetch.
- `decision_stale` (409) shows the reload message; `invalid_decision` (422) cannot come from the
  person's Accept, and is shown as Core's message if it ever does.
- A recheck row links to the task in its own `workflow_id` and `version`.

The web tasks start at F-121 and stay blocked until B-152, B-154 and B-105 have regenerated
`api/openapi.json`.

## Ruling

<The teacher, only when disputed.>
