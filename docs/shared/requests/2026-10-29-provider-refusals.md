# Provider refusals name the field

- Status: asked
- Asked by: Masada, 2026-10-29
- Screen: docs/frontend/plans/2026-10-25-web-architecture.md (W-11 packet 1 `model-picks`)

## Need

The AYLUL provider form lets the person list the models a provider serves, each
with a context window. Today the web refuses a bad list itself, with a copy of
Core's rules in `web/src/app/agents/model-picks.ts` (1 to 20 models, ids of 1 to
128 printable characters with no spaces, `default` reserved, each id once, a
context window from 1000 to 10000000), plus a label of at most 64 characters in
`provider-form.tsx` and a context window of 200000 when a listed model names none,
in `provider-models.tsx`. Core refuses the same input with one message, "invalid
provider configuration", which does not say what is wrong, and fills the same
200000 default itself. The form must show Core's own reason next to the field Core
refused and leave an unknown context window empty for Core to fill, so the web
copies can go.

## Fields

| Call | Sends | Gets back |
|---|---|---|
| `POST /api/aylul/providers` (create) and `PUT /api/aylul/providers/{slug}` (edit, and the retry after a create whose key save failed) | each model's `context_window` may be absent; Core fills its default | the same 400 `PROFILE_INVALID` from both, with `field: string` (`models`, `models[i].id`, `models[i].context_window`, `name`, `label`, `base_url`) and a `message` that states the rule, e.g. "Models must be 1 to 20 ids." |

## Errors

| When | The screen shows |
|---|---|
| Core refuses one field | Core's message under that field; the form keeps the person's input |
| Core refuses with no field | Core's message above the save button |

## Backend answer

## Frontend reply

## Ruling
