# A task's run state for the task page

- Status: accepted
- Asked by: Controller for the Core web client, 2026-11-08
- Screen: the task page, [executors design](../../backend/specs/2026-11-08-executors-design.md) §4
  (mockup A, tabs); plan `docs/backend/plans/2026-11-08-executors.md`, F-98.
- Basis: Mohammed approved the executors design on 2026-11-08.

## Need

The task page's header shows each task this one waits on with its state, and a Start button
that is enabled or disabled with a reason. The web must not work out readiness from links,
marks or attempts; Core answers "may Start run now, and if not, why" and the web draws it.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `GET .../versions/{version}/tasks/{task}` | — | `task`, `done`, `done_at`, `latest` (an `Attempt` or `null`), plus `needs: [{ task, title, done }]` in link order, `start: { allowed: boolean, code: string \| null, reason: string \| null }`, `suggested_model: string \| null` |
| `POST .../tasks/{task}/attempts` | `{ command_id, model }` | `Attempt`; a refusal is 409 with `{ code, reason }` |

`start.code` is an opaque id Core owns, the same set a refused Start returns: `waiting`,
`task_busy`, `overlap`, `not_git`, `dirty_scope`, `not_frozen`, `superseded`, `archived`.
`start.reason` is the text to show. `allowed` true means `code` and `reason` are `null`.

## Errors

| When | The screen shows |
|---|---|
| `start.allowed` false | Start disabled, `start.reason` beside it |
| Start refused on send (the state moved since the read) | Core's `reason` on the error line; the model choice stays; the read is refetched |
| a replayed send with another model (`CommandConflict`) | the error line; the page keeps Core's attempt |

## Backend answer

Accepted: `Runs::can_start` answers `start` with the same checks as `Runs::start`, without
writing; `needs` comes from the version's `needs` links and Runs' done marks. Owner `core-runs`,
routes in `shadows-http` (B-93). No other public field is added.

## Frontend reply

Accepted by the controller for F-98: Start's state and reason come only from `start`; the page
keeps no readiness rule.
