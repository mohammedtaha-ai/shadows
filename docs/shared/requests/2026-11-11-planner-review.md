# The Planner's review on the task page

- Status: accepted
- Asked by: Hamoud (backend), 2026-11-11, for Mohammed's slice 1b
- Screen: the task page ([task run request](2026-11-08-task-run-page.md)); the conversation
- Serves: [executors design §5.1](../../backend/specs/2026-11-08-executors-design.md#51-the-planner-judges-mohammed-2026-11-11),
  [plan](../../backend/plans/2026-11-11-slice-1b-planner-runs-tasks.md) (B-126 builds the field)

## Need

The Planner now starts tasks, judges each committed attempt and either marks the task done or
sends the executor an **addition**: what is missing or must be extended. The executor carries on
in its own session as a new attempt. The person must see, on the task page, which attempt began
from Start and which carried on with an addition, and the addition's text beside the report it
answered. Nothing is sent from the web for this: the Planner acts through its tools.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/workflows/{workflow}/versions/{version}/tasks/{task}/attempts` (and every route that answers an `Attempt`) | nothing new | `Attempt` gains `addition: string \| null`: the Planner's addition this attempt carried on with, in the session of the attempt before it; `null` for an attempt Start began |

What the page can derive, with no new field:

- **Waiting for the Planner's judgment:** the task's `latest` is `committed` and `done` is false.
  While it waits, Start answers `task_busy` (its scope is held) and the run's session stays open.
- **Judged:** `done` true. The Planner's mark and the person's mark are the same mark.
- **Additions on the task:** the attempts with a non-null `addition`, in `number` order.

The conversation also shows the Planner's notices: a message on the plan's conversation whose
text starts with `[Shadows] T<n> "<title>"`, written as the person's side of a turn. Styling it
apart from the person's own messages is the web's choice; nothing marks it but that prefix.

## Errors

None new on any route the web calls. `start.code` can now be `task_busy` for a committed task
that waits for judgment; its `reason` text is Runs' own.

## Backend answer

Proposed by the backend with the slice. Owner: Runs (`crates/core-runs/contract.yaml`). Only
B-126 changes `api/openapi.json` (`Attempt.addition`), and it is not built until this request is
`accepted`. B-123 to B-125 change nothing the web reads in shape.

## Frontend reply

Masada, 2026-11-11: **accepted** as asked. B-126 may build `Attempt.addition`.

- **Addition (F-108, blocked on B-126).** The Report tab marks each attempt "Began from Start"
  (`addition` null) or "Carried on with the Planner's addition" and shows the text as plain
  preserved text, right under that attempt's heading. The attempts are newest first, so the
  report it answered is the next card down.
- **Waiting for judgment (F-106, built now).** The page reads it exactly as stated here
  (`latest.state == committed` and `done == false`), in one function in `web/src/api/runs.ts`
  and nowhere else. While it holds, the page reads the task again every two seconds (there
  is no event for a run), so the Planner's mark or its next attempt shows without a reload.
  If Core ever adds a state to that reading, it must come as a field, not a new rule for the
  web.
- **The cap of 3 additions (Mohammed, 2026-11-11).** The web counts nothing. When a task
  stops after its third addition and goes back to Mohammed, the page must read that from Core:
  name here the field or the `start.code`/`reason` that says it before B-126 is built, and F-108
  shows it. If it is only `start.reason` on a task that Start refuses, nothing more is needed
  from the web; if Start is allowed again, the page needs a field to say why the task stopped.
- **The Planner's notices in the conversation.** Not styled apart for now. The web will not
  read a text prefix to style a message (that is a rule kept in the text). If Mohammed wants
  them styled, Core marks them (for example who sent them) under a new request.

## Ruling
