# The Planner's review on the task page

- Status: asked
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

<Masada>

## Ruling
