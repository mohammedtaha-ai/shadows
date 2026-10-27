# The context ring shows the summary only

- Status: accepted
- Asked by: Mohammed, 2026-10-27 (the controller wrote it from his agreement)
- Screen: the context ring in `web/src/app/conversation/context-ring.tsx` (also in the AYLUL
  desktop, which packages the same client). Owner spec:
  [harness controls §12.8](../../superpowers/specs/2026-09-24-harness-controls-design.md#128-context-and-account-limits)

## Need

Mohammed opens the context ring in a conversation and sees, at once, the context used of the
window and the account's five-hour and weekly limits, as the daemon last recorded them. The
ring's second level (the breakdown) goes: it is read by sending `/context` to the session, which
takes 17 to 26 seconds on a fresh session, and the 2026-10-22 control run
(`docs/evidence/2026-10-22-slash-context-probe.md`) shows each reading enters what the model is
sent (892 input tokens for two readings on OpenRouter). That run is the trigger §12.8's OPEN
block names. When Mohammed wants the breakdown he types `/context` in the conversation
himself, and its answer arrives as an ordinary message through A-2's slash-command path.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `GET /api/threads/{id}/context` | removed | removed |
| The ring's summary | unchanged | unchanged: `context_used`, `context_window` and the harness limits Core already sends |
| `/context` typed in the composer | unchanged: sent alone, as A-2 sends a slash command | the breakdown as one message |

## Errors

| When | The screen shows |
|---|---|
| No figures yet | "no figures yet", as today; no loading state |
| `/context` typed with other text in one message | as any slash command today (A-2) |

## Backend answer

As asked. Task A-7b in `docs/backend/stages.md`: the service `AppCore` names as the route's owner
drops it and its breakdown surface, §12.8's OPEN block closes on the control run, OpenAPI is regenerated.

## Frontend reply

Accepted. Task F-22 in `docs/frontend/stages.md`: the ring renders the summary only and makes no
breakdown request; the schema is regenerated; both land on the same delivery branch.
