# Partial person update

- Status: asked
- Asked by: Masada, 2026-10-26
- Screen: [docs/frontend/plans/2026-10-25-web-architecture.md](../../frontend/plans/2026-10-25-web-architecture.md) "Decisions" (3); the settings dialog's You card and General preferences

## Need

The settings dialog saves the person record from two forms: the You card (name, what AYLUL
calls the person) and General preferences (language, theme). Today each form sends the whole
record (`PUT /api/me` with all four fields), built from the snapshot it read. Two saves close
together can therefore write back an older value of the field the other form just changed.
Mohammed decided (2026-10-25) that each form sends only the fields it changed and Core merges
them into the stored record, so the two saves cannot overwrite each other.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `PATCH /api/me` (or `PUT` with optional fields: the backend's choice) | `{ command_id, name?, aylul_calls_me?, language?, theme? }`: only the changed fields; an absent field keeps its stored value; `null` clears a name | the whole record, as `GET /api/me` |

The idempotency fingerprint covers the fields sent, so a replay with a different set of
fields is a `CommandConflict`.

## Errors

| When | The screen shows |
|---|---|
| No field sent | The form's save is disabled; a request without fields is a validation error |
| A field fails today's validation (length, line break, unknown theme or language) | The same message as today, on that field |

## Backend answer

<Hamoud: as asked / with a change / not possible, and why. Name the service
and contract it touches.>

## Frontend reply

<Masada: accepted, or what must stay and why.>

## Ruling

