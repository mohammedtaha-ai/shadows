# Send the planning context again

- Status: accepted
- Asked by: Claude for Mohammed, 2026-11-04. The button comes with the inspection view
  (Planner context slice 3); slice 2's acceptance calls the route directly.
- Screen: `docs/backend/specs/2026-11-04-planner-context-design.md` §4.3 ("On request", "What a
  compaction or a request forgets"); plan `docs/backend/plans/2026-11-04-planner-context-slice-2.md`
  at `024432fc`, B-58 (`docs/backend/stages.md` at `024432fc`).

## Need

In a Planner conversation, Mohammed presses "send the context again" when he thinks the session
lost what it was given, for a case the compaction rule misses. The next turn then carries the
planning context and every instructions and standards change since the session opened.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `POST /api/threads/{id}/context/resend` | `{ command_id: string }` | `{ owed: true }` |

## Errors

| When | The screen shows |
|---|---|
| The thread does not exist (404) | The usual missing-conversation line. |
| The conversation is an AYLUL agent's (422 `INVALID_COMMAND`) | The action is not offered for AYLUL conversations. |
| A replay with the same `command_id` and other params (409 `COMMAND_CONFLICT`) | The usual conflict line. |

## Backend answer

As asked. `Harness` owns what a session heard, so `Harness::forget_context` answers it; a
completed compaction calls the same method. A missing thread answers 404 with the code
`INVALID_COMMAND`, the API's code for a missing record.

## Frontend reply

Accepted by Mohammed's decision of 2026-11-04 to ship the route with slice 2 and the button
with the inspection view.

## Ruling

<The teacher, only when disputed.>
