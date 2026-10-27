# Partial person update

- Status: accepted
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

As asked, as `PATCH /api/me`; `PUT /api/me` goes, since the web is its only caller. Task B-13 in
`docs/backend/stages.md`. The owner is `PersonSettings` (`crates/core-person`, its
`contract.yaml`): `set_person` becomes `update_person`, which reads the stored row, applies
the sent fields and writes it back inside one write transaction, so two updates close together
each keep the other's fields. A sent `null` clears a name or the language; `theme` cannot be
null. The fingerprint is the fields as sent.

## Frontend reply

Accepted. Task F-28 in `docs/frontend/stages.md`, on the same delivery branch as B-13.

## Ruling

Mohammed accepted the request on 2026-10-28.

