# A turn's status carries its journal order

- Status: accepted
- Asked by: Masada, 2026-10-30
- Screen: docs/frontend/plans/2026-10-25-web-architecture.md (W-11 packet 5 `turn-state`)

## Need

The conversation shows whether a turn is waiting, running or ended, and whether a stop was
asked, so it knows when to offer Stop. It learns this from two sources that arrive in either
order: the operations snapshot (read on opening a thread and at each `caught-up`) and the
stream's operation events. Today `web/src/app/conversation/turn-state.ts` copies Core's
lifecycle to merge them: it ranks Pending below Running below every terminal status, keeps the
higher rank, and maps each event kind to the status it leads to. The screen needs each source
to say where it stands in the journal and what status it leaves the turn in, so the newer one
wins by sequence and the copied lifecycle can go.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/threads/{id}/operations` | nothing | each `Operation` adds `seq: integer`, the journal sequence of the latest event of that operation, read with the row |
| stream, durable `Operation*` events (`OperationCreated`, `OperationStarted`, `OperationCompleted`, `OperationFailed`, `OperationCancelled`, `OperationInterrupted`, `OperationCancellationRequested`) | — | the payload adds `status_kind: string`, the operation's status after the event, and `cancel_requested: boolean`, whether a stop has been asked by then; existing payload fields stay |

The client keeps, per turn, the source with the higher `seq` (a stream event's `seq` is the
frame's). The start route's answer has no `seq`, so any snapshot row or event for that turn
replaces it.

## Errors

| When | The screen shows |
|---|---|
| The operations snapshot cannot be read | the existing operations error; the stream's events still apply |
| A `status_kind` the client does not know | the turn is treated as not ended, so Stop stays offered, as today |

## Backend answer

Controller for Hamoud, 2026-10-30: as asked. `Operations` owns both
(`crates/core-operations`, contract `crates/core-operations/contract.yaml`): every lifecycle
transition writes its event through `Transition::record`, which adds `status_kind` and
`cancel_requested` beside any payload the transition supplies, and
`list_operations_for_thread_in` reads each row's latest operation event sequence in the same
query. `Turns` (`crates/core-turns`) carries that `seq` onto `Operation`. Task: B-18.

## Frontend reply

Masada, 2026-10-30: accepted. F-49 merges by `seq` and deletes the status rank and the
event-kind table from `turn-state.ts`.

## Ruling
