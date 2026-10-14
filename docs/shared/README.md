# Shared requests

The only place the frontend and the backend agree on what crosses between
them. The agreed shape is decided here, **before** either side writes code, so
nothing is built twice.

`api/openapi.json` stays the real contract: it is generated from the backend's
code. A request is the agreement that code must then satisfy.

## The cycle

1. **Masada** plans a screen with the user and writes
   `requests/<yyyy-mm-dd>-<topic>.md` from [`requests/TEMPLATE.md`](./requests/TEMPLATE.md):
   what the screen needs, field by field. Status: `asked`.
2. She tells **Hamoud** (a message to his session) that the request is waiting.
3. **Hamoud** reads it against the backend's contracts and answers in
   "Backend answer": as asked, with a change, or not possible, and why.
   Status: `answered`.
4. **Masada** accepts the answer, or replies under it. Status: `accepted` or
   `disputed`.
5. A `disputed` request goes to the **teacher** session, which reads both sides
   and writes the "Ruling". Its ruling is final. Status: `accepted`.
6. Only an `accepted` request may be built. Hamoud's tasks build it first;
   Masada's tasks that need it stay `blocked` until his side is done and
   `api/openapi.json` shows it.

A message between sessions is only a notice. What counts is written in the
request file.

## Who writes what

| Section | Written by |
|---|---|
| Need, Fields, Errors | Masada |
| Backend answer | Hamoud |
| Frontend reply | Masada |
| Ruling | the teacher |
| Status | whoever moved it last |
