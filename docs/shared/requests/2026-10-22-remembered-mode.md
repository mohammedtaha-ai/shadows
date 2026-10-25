# The mode a conversation keeps

- Status: accepted
- Asked by: Mohammed, 2026-10-22 (the controller wrote it from his ruling)
- Screen: the composer's Mode menu in `web/src/app/conversation/` (also in the AYLUL
  desktop, which packages the same client). Owner spec:
  [harness controls §12.4](../../superpowers/specs/2026-09-24-harness-controls-design.md#124-choices-come-from-the-harness)

## Need

Mohammed picks Auto in a conversation's Mode menu. Today the choice lives in React state
only, and Core reopens sessions at the policy's default, so a reload or leaving the
conversation shows Accept edits again. The screen must save the mode the moment it is
picked, and every later open of that conversation must show the mode Core holds for it.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `PUT /api/threads/{id}/session/mode` | `{ command_id: string, mode: string }` | the session's choices, as `POST /api/threads/{id}/session` answers them, with `current.mode` set |
| `POST /api/threads/{id}/session` | unchanged | `current.mode` is the conversation's remembered mode when it is still allowed |

## Errors

| When | The screen shows |
|---|---|
| The project does not allow the mode (`ModeNotAllowed`) | the menu keeps Core's mode and says the project does not allow it |
| The harness has no such mode | the menu keeps Core's mode; the error text from Core |
| A replay with another mode (`CommandConflict`) | Core's mode; no retry with the same command id |
| Core unreachable | the menu returns to the last mode Core answered, with the usual connection notice |

## Backend answer

As asked. Owner: `Harness` (`crates/shadows-core/src/harness/`, its contract and store),
which already owns the session's choices and the remembered model and effort. Built in
B-10 after A-17 reshapes the harness boundary.

## Frontend reply

Accepted: F-23 saves on pick and shows Core's answer.
