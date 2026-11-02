# Context use per operation

- Status: asked
- Asked by: Masada, 2026-11-02
- Screen: [docs/frontend/plans/2026-11-02-ai-studio-ui-migration.md](../../frontend/plans/2026-11-02-ai-studio-ui-migration.md), F-76 (Agent Mode board, context section)

## Need

The board's context section shows the turn's summary today: `context_used` of
`context_window` from the latest operation's `InvocationView`. The design also shows what one
operation spent: tokens in, tokens out, tokens read from and written to cache, so the person
sees which turn was costly. That block sits empty inside `Planned`. The figures must come from
the harness's own usage report for the turn. They must not be read by sending `/context`: the
[summary-only ring](2026-10-27-summary-only-context-ring.md) removed that because it is slow and
enters what the model is sent.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/threads/{id}/operations` | as today | `InvocationView` adds `usage: { input_tokens, output_tokens, cache_read_tokens, cache_write_tokens } \| null`, each `integer \| null`, as the harness reported them for this operation |
| `GET /api/subscribe` | — | the figures of a running turn when the harness reports them (on an existing frame or a transient `usage` frame), so the board fills before the turn ends |

The web shows each figure as given and adds or derives none.

## Errors

| When | The screen shows |
|---|---|
| `usage` is `null` (the harness reports none, or an older operation) | the block stays inside `Planned` with dashes |
| one figure is `null` | that figure shows "—" |

## Backend answer

<Hamoud: as asked / with a change / not possible, and why. Name the service
and contract it touches.>

## Frontend reply

<Masada: accepted, or what must stay and why.>

## Ruling

<The teacher, only when disputed.>
