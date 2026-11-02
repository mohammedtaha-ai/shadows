# Task status on a plan task

- Status: asked
- Asked by: Masada, 2026-11-02
- Screen: [docs/frontend/plans/2026-11-02-ai-studio-ui-migration.md](../../frontend/plans/2026-11-02-ai-studio-ui-migration.md), F-75 (plan page header); look in [visual direction](../../frontend/specs/2026-11-01-visual-direction.md)

## Need

On a plan's page the person sees how far the plan has come: "N tasks · M done", a progress
bar, and one small toggle per task (`T1`, `T2`, ...) that shows whether that task is done and
lets the person mark it done or not done. Today `PlanTask` (`TaskContent` plus `id`) carries no
status, so the header shows the real task count, "—" for done, an empty bar and disabled
toggles inside `Planned`. The web must not work a status out from threads, operations or
links; Core says each task's status and the web draws it. `TaskPreview` (a linked task's
preview) already carries a `state`; the plan read needs the same for its own tasks.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/workflows/...` plan read (the call `planVersionsQuery` uses) | as today | each `PlanTask` adds `status: string`, an opaque id Core owns (for example `pending`, `in_progress`, `done`), and `status_label: string` to show; and the plan adds `done: integer` and `total: integer`, Core's own count of the latest version |
| a new call to set one task's status by hand, if Core allows it | `{ command_id, status }` | the task with its new `status` |

The web shows `done` / `total` as given and computes no percentage beyond `done / total` for the
bar's width. Whether a person may set a status by hand, and which statuses count as done, is
Core's rule; if hand-marking is not allowed, the toggles stay read-only marks.

## Errors

| When | The screen shows |
|---|---|
| `status` absent (an older version) | the toggle shows "—" and stays disabled |
| setting a status is refused | the toggle returns to Core's status and the error line shows Core's reason |
| replay with another status (`CommandConflict`) | the error line; the task keeps Core's status |

## Backend answer

<Hamoud: as asked / with a change / not possible, and why. Name the service
and contract it touches.>

## Frontend reply

<Masada: accepted, or what must stay and why.>

## Ruling

<The teacher, only when disputed.>
