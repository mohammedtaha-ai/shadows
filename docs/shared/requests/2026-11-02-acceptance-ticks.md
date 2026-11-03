# Saved acceptance ticks per task

- Status: asked
- Asked by: Masada, 2026-11-02
- Screen: `docs/frontend/plans/2026-11-02-ai-studio-ui-migration.md` at `a7b4009c`, F-75 (`docs/frontend/stages.md` at `b40e077b`; inspect panel, Acceptance section)

## Need

In a plan task's inspect panel the person reads the task's acceptance items and ticks each one
as it is met; a bar shows how many are ticked. The ticks must be saved by Core and read back on
reload and by every other client. Today `AcceptanceItem` is `{ number, text }` with no tick, so
the list shows disabled ticks and an empty bar inside `Planned`. The web keeps no local tick.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| the plan read (as for the task list) | as today | each `AcceptanceItem` adds `met: boolean` and `met_at: string \| null` (RFC 3339) |
| a new call to tick or untick one item | `{ command_id, plan task id or number, acceptance number, met: boolean }` | the item with `met` and `met_at` |

The bar shows the count of `met` items over the count of items, both from Core. Core decides
whether ticks carry over when a new plan version keeps the same acceptance item, and says so in
its answer.

## Errors

| When | The screen shows |
|---|---|
| `met` absent | the tick is disabled, as today |
| the plan is `Archived` | ticks are read-only |
| a tick is refused | the tick returns to Core's value; the error line shows the reason |

## Backend answer

<Hamoud: as asked / with a change / not possible, and why. Name the service
and contract it touches.>

## Frontend reply

<Masada: accepted, or what must stay and why.>

## Ruling

<The teacher, only when disputed.>
