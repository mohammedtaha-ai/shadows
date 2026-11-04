# Defects seen in two Planner sessions: 2026-11-04

Dated facts, to fix later; this file decides nothing. Each fix goes to the owner of the
behaviour named, with its own test.

Seen in:
- testing55, conversation "Shadows project idea" (`35c8625b-2fdd-4957-b17b-d2a65df66ab7`),
  planned on 2026-11-04 for the Planner context probe
  (`docs/evidence/2026-11-04-planner-context-probe.md` §3);
- testing4, conversation "فكرة مشروع Shadows" (`5309a8bc-d755-4eb3-a88e-3d59dedb4aaf`).

Read through the running daemon: `GET /api/threads/{id}/entries`, and
`GET /api/projects/{id}/design/vision` and `/design/parts`.

## 1. Every refused tool answers `INVALID_COMMAND`, with a short message that does not say what to do

Mohammed's report: when a tool call fails, the error is not clear; every error reads the same.

The three refused `map_propose` calls in testing55 (entries 70, 72, 74):

| Entry | Error as the agent and the person saw it |
|---|---|
| 70 | `INVALID_COMMAND: file outside its part's folders: composer.json` |
| 72 | `INVALID_COMMAND: folder ownership conflict: app, app/services` |
| 74 | `INVALID_COMMAND: folder ownership conflict: src, src` |

- **The code says nothing.** All three carry the same code, `INVALID_COMMAND`, whatever the
  rule that refused them.
- **The message omits the facts.** It does not name the part, and does not say which rule or what
  is allowed:
  - entry 70 does not say which part's folders `composer.json` must sit in;
  - entry 74 names the folder `src` twice, but not the two parts that both claim it.
- **The agent had to guess the fix,** and needed three attempts.
- **The card shows the error inside a code fence** (```` ``` ````), so the person reads raw text.

Wanted, for whoever fixes it:
- a code per rule (for example "file outside part folders", "folder claimed twice");
- the parts and paths involved;
- one line saying what is allowed.

## 2. Vision IDs follow the approval order, not the proposal order

- **Proposed:** the Planner proposed the technical directions in this order and named them in the
  conversation:
  - TD1 Laravel;
  - TD2 Telegram Bot API server;
  - TD3 Mercure;
  - TD4 FCM;
  - TD5 Flutter and dashboard;
  - then OpenAPI and sign-in.
- **Approved:** the cards were approved in another order.
- **Stored:** the IDs follow the approval order:
  - TD1 = OpenAPI, TD2 = sign-in, TD3 = Flutter and dashboard;
  - TD4 = Bot API server, TD5 = Laravel, TD6 = Mercure, TD7 = FCM.
- **The result:**
  - The conversation's labels and the stored IDs disagree.
  - The part cards' `Serves` lists use the stored IDs; for example the API part serves
    `TD1 TD2 TD3`.
  - So a person who reads the conversation and then a part card sees numbers that look wrong.

Whoever fixes it decides between:
- giving an ID when the card is proposed;
- or having the Planner show the stored IDs after approval.

## 3. The Telegram part does not serve the self-hosted Bot API direction

- The Telegram bot integration part serves `PU1 U1 G1 G5 B2`.
- It does not serve TD4, "Self-hosted Telegram Bot API server", although its own text says it
  connects through that server.
- Seen once; it may be a Planner slip rather than a rule, but no check caught it.

## 4. A Read of a file that does not exist, in testing4

- In testing4's conversation, entry 4, a `Read` failed with "File does not exist" (working
  directory `F:\testing4`).
- Likely harmless, the agent looking before the structure existed. It is recorded so the fix
  for §1 can check that harness errors and Shadows refusals are told apart.
