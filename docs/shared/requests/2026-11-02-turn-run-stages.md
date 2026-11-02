# Run stages of an agent turn

- Status: asked
- Asked by: Masada, 2026-11-02
- Screen: [docs/frontend/plans/2026-11-02-ai-studio-ui-migration.md](../../frontend/plans/2026-11-02-ai-studio-ui-migration.md), F-76 (`docs/frontend/stages.md` at `b40e077b`; Agent Mode board stepper) and F-74 (`docs/frontend/stages.md` at `b40e077b`; AYLUL agents home, stage strip)

## Need

In Agent Mode the board shows a stepper of the turn's stages (the design names Plan, Analyze,
Execute, Verify, Deliver) with the current one lit and the finished ones marked; the AYLUL
agents home shows the same pipeline as an overview. Today Core sends no stage, so both sit inside
`Planned` with no marks, and the design's heuristics (a stage guessed from tool kinds) are not
ported. The web needs Core's list of stages and, for a turn, which stage it is in, live and after
a reload. Stage names are Core's: the web draws the list it gets, in Core's order, and holds no
stage set of its own.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| a stage list (on an existing read, or `GET /api/...`) | nothing | `[{ id: string, label: string }]` in order |
| `GET /api/threads/{id}/operations` | as today | each `Operation` adds `stages: [{ id, status: "pending" \| "active" \| "done" \| "skipped" \| "failed" }] \| null`; `null` when the harness reports none |
| `GET /api/subscribe` | — | a transient `stage` frame `{ op, id, status }` on each change of a running turn; after `caught-up`, one frame with the current stage of each running turn |

## Errors

| When | The screen shows |
|---|---|
| `stages` is `null` | the stepper stays inside `Planned`, empty |
| a `stage` frame cannot be read | the stream's existing rule (the frame in `problem`) |
| the turn ends with a stage still `active` | Core's final status for it; the web marks nothing itself |

## Backend answer

<Hamoud: as asked / with a change / not possible, and why. Name the service
and contract it touches.>

## Frontend reply

<Masada: accepted, or what must stay and why.>

## Ruling

<The teacher, only when disputed.>
