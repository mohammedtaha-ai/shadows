# The active-projects setting carries its bounds

- Status: asked
- Asked by: Masada, 2026-10-29
- Screen: docs/frontend/plans/2026-10-25-web-architecture.md (W-11 packet 2 `active-limit`)

## Need

Settings → Active projects lets the person choose how many projects stay active.
The number input needs its lowest and highest value before the person types, and
today `web/src/app/settings/active-projects.tsx` copies them (`MIN = 1`,
`MAX = 20`) from Core's check in `core-code`. The settings read must carry the
bounds Core enforces, so a change in Core reaches the input.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/code/settings` | nothing | adds `active_limit_min: integer` and `active_limit_max: integer` |
| `PUT /api/code/settings` | unchanged | unchanged; a value outside the bounds stays a 400 whose message names them |

## Errors

| When | The screen shows |
|---|---|
| The person types a value outside the bounds | the save is disabled and the bounds are shown |
| Core refuses the value anyway | Core's message under the input |

## Backend answer

## Frontend reply

## Ruling
