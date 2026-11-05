# Each turn says whether its planning context was heard

- Status: accepted
- Asked by: Claude for Mohammed, 2026-11-05.
- Screen: `docs/backend/specs/2026-11-04-planner-context-design.md` §4.4 ("The conversation
  shows, per turn…"); plan `docs/frontend/plans/2026-11-05-planner-context-view.md`, B-59 and F-89.

## Need

Each turn in a Planner conversation shows whether it carried planning context and whether the
session heard it. The conversation already loads its operations; asking for each turn's record
one by one would be a request per turn.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/threads/{id}/operations` | | each `Operation.invocation` gains `planning_context: "heard" \| "prepared" \| null` |

Every other answer that carries an `InvocationView` gains the same field.

- `"heard"`: a record exists and the turn ended normally.
- `"prepared"`: a record exists and the turn has not completed: it is still running, or it
  ended without completing. The screen says "not confirmed yet".
- `null`: no record.

## Errors

None new.

## Backend answer

As asked. `Harness` owns the invocation row and the context record, so it reads both together
for every answer that carries an `InvocationView`.

## Frontend reply

Accepted by Mohammed, 2026-11-05.

## Ruling

<The teacher, only when disputed.>
