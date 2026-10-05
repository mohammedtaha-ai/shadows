# Section 20 — Writing While a Turn Runs: the Queue and Send Now

- **Date:** 2026-10-05.
- **Status:** Accepted by Mohammed on 2026-10-05, after an independent review
  (`e94bf08`) and his two rulings in 20.4. Next: the implementation plan.
- **Evidence:** [`2026-10-05-steering-and-commands-probe.md`](../../evidence/2026-10-05-steering-and-commands-probe.md)
  (adapter 0.81.1, Claude Code 2.1.289).
- **Related owners:** §12.3 and §12.7 (a turn and its one start command),
  §14.9 (`send`'s order), §2.3 (`Cancelled` means confirmed).

This section owns what happens to a message the person writes while a turn
runs in the same conversation: it waits in a queue, it is sent as the next
turn when the running one completes, or the person sends it into the running
turn at once ("Send now"). It copies the behaviour of Claude Code's desktop
app. It does not change §12.7's rule that a thread has at most one open turn:
a second `turn.start` while one runs is still `THREAD_BUSY`. The queue sits in
front of that rule; it never goes around it.

## 20.1 What the adapter gives

Measured, not assumed (evidence above):

- `_session/steering` during a running turn answers `{"outcome":"injected"}`
  within milliseconds and pre-empts the current generation, or slots between
  tool calls. The steered message gets **no turn of its own**: the running
  `session/prompt` resolves when the steered reply is done, with its usage.
- Sent with `_meta.steering.idleBehavior: "promptRequired"` while no turn runs,
  it answers `{"outcome":"promptRequired"}` and starts nothing.
- **The steered text is never echoed** in `session/update`. Shadows records it
  itself.
- The adapter also queues a second `session/prompt` on its own. Shadows does
  not use that: an adapter-held queue dies with the adapter, cannot be
  removed from, and would put two open turns on one thread.

## 20.2 The queue

A waiting message is a row of a new table `queued_message`, owned by the
`turns` service, in migration 0019:

| Column | Meaning |
|---|---|
| `id` | the waiting message's id |
| `thread_id` | its conversation |
| `position` | an explicit, stable order within the thread; a new message goes last |
| `command_id`, `fingerprint` | the `turn.queue` command that created it, for replay (§3.2) |
| `prompt`, `model`, `mode`, `effort`, `focus_json`, `plan_id` | `SendTurn`'s fields, as the composer held them when the message was written |
| `last_error` | why its last send failed, or `NULL` |
| `created_at` | when it was queued |

A waiting message is **not** a thread entry. It becomes one only when it is
sent: as the first entry of a new turn (20.3), or inside the running turn
(20.4). Several waiting messages stay separate, each its own turn, in
`position` order. A waiting message cannot be edited; the person removes it
and writes another. A fork does not copy waiting messages. Those of a removed
thread are never sent and never listed: `send` refuses a removed thread.

**Adding** (`turn.queue`) checks the thread's busyness inside the same
`BEGIN IMMEDIATE` transaction that would insert the row. A busy thread gets
the row. An idle thread gets no row: the message is started as a turn through
`send`'s own path, with `send`'s refusals, and the answer says it started.
That turn is recorded as `turn.start` under the caller's `command_id`, with
`turn.start`'s fingerprint, and a replay of the `turn.queue` call asks
`send`'s replay step first, so it answers that turn and does not queue a
second message behind it. Without this a message queued in the instant a turn ended would wait for a
`Completed` that never comes.

## 20.3 Sending the next one when a turn ends

Only a turn that ends **`Completed`** sends the next waiting message. After
`Cancelled` (the person pressed Stop), `Failed` or `Interrupted` (a daemon
restart) nothing is sent: the messages wait until the person sends or removes
them. A daemon start sends nothing either.

When the watcher has recorded `Completed` (after it gave the session's events
back, `turns/turn.rs` `watch_turn`), it takes the thread's first waiting
message and starts it through `send`'s existing path: the same order, the same
refusals, run to its end as `turns/contract.yaml`'s first trap demands, so the
watcher does not call a half of it. The turn's `turn.start` command id is
derived from the row's id (as §13.5 derives one for `draft_start`), never the
`turn.queue`'s own. **Removing the row and committing the new turn happen in
one `start_turn` transaction**, so a message is sent once or not at all. The
row is removed only if it is still there: the `Send now` of 20.4 and this start
race for the same row, and the one that finds it gone does nothing.

If the start is refused (the harness is unavailable, `RuntimeStopping`, a
setting no longer offered …), the row stays, its `last_error` names the
refusal, and nothing more is sent from that thread until the person acts.
`THREAD_BUSY` is not a failure: the person's own turn took the thread between
the `Completed` and this start, so the row stays without `last_error` and that
turn's `Completed` sends it.

## 20.4 Send now

`turn.send_now` on a waiting message:

1. If a turn runs on the thread, Shadows sends `_session/steering` with the
   message's prompt and `idleBehavior: "promptRequired"` on that turn's
   session. If a Stop was already asked for that turn (its cancellation is
   recorded, §12.3), nothing is sent: the row stays without `last_error` and
   the answer is `409 THREAD_BUSY`. Once the turn is `Cancelled` the person
   sends it with **Send**, as after any Stop (20.3). Decided by Mohammed on
   2026-10-05.
2. On `injected`, **one transaction** removes the row and writes the message
   as a user entry of the running turn, at the thread's next ordinal. The
   steering request is sent **before** the entry is written: a failed steer
   then leaves no entry the Planner never saw. (The probe's answer came in
   4 ms, before any reply chunk.) Entries take their ordinal from the thread's
   counter (`threads/store/entry.rs` `append_entry_in`), so this write cannot
   collide with the running turn's own entries.
3. On `promptRequired`, or when no turn runs, the message is started as a
   turn exactly as in 20.3, and the answer says so.
4. A steer that fails (the session ended, a JSON-RPC error) leaves the row,
   sets `last_error`, and writes nothing to the thread.

**Order in the thread.** The entries keep the order the person saw. §12.3
writes an agent message only once it is complete, so before the steered user
entry is written, the running turn's message still being streamed is written
first with the text it has so far; the reply continues as a new message after
the steered entry. Decided by Mohammed on 2026-10-05.

> **OPEN — ACP v2 prompt.** The ACP "prompt" RFD for v2 (draft, read through
> Context7 on 2026-10-05) has `session/prompt` answer when a message is
> *accepted*, the agent emit `userMessage/accepted` with an agent-owned id,
> and clients submit, edit or cancel queued messages before acceptance. Then
> the steered entry would be written on that event instead of on `injected`,
> and the adapter's own queue might replace `_session/steering`. Closed when
> the pinned adapter advertises ACP v2 prompts: re-run the probe of 20.1 and
> amend this section.

The running turn keeps its own operation and ends as §12.3 says; the steered
reply is part of it. The message's own `model`, `mode` and `effort` do not
apply to a steer: the running turn's settings stand.

`shadows-agent`'s `Connection` (`acp.rs`) gains one call, the steering request,
sent as an extension method beside the typed `prompt` and `cancel`; nothing
else in the harness changes.

## 20.5 Interface

Each route calls one `Turns` method (§14.5). No MCP tool: nothing uses one yet.

| Route | Method | Answer |
|---|---|---|
| `POST /api/threads/{id}/queue` | `queue` | the waiting message, or the turn it started (20.2) |
| `GET /api/threads/{id}/queue` | `queued` | the thread's waiting messages in `position` order |
| `DELETE /api/threads/{id}/queue/{qid}` | `unqueue` | nothing |
| `POST /api/threads/{id}/queue/{qid}/send-now` | `send_now` | `Steered`, or the turn it started |

`queue`, `unqueue` and `send_now` are commands with a `CommandId`, a kind
(`turn.queue`, `turn.unqueue`, `turn.send_now`) and a fingerprint over the
thread and the message (`queue`: `turn.start`'s parameters). The id travels as
`command_id` in the body, and as a query parameter on `DELETE`. A replay is
judged first and answers what the first call answered; only a new command on a
message already sent or removed is `404 QUEUED_MESSAGE_GONE`, a new
`ErrorCode`. Each new command has its line in the `turns` contract, and the
routes change `api/openapi.json`.

Durable events, so every open tab follows: `MessageQueued`,
`QueuedMessageRemoved`, `QueuedMessageSent` (carrying whether it was steered
or started a turn) and `QueuedMessageFailed` (carrying `last_error`).

**Web client.** Today Enter does nothing while a turn runs and Stop replaces
Send (`composer.tsx` `submit`); the model and effort menus are frozen. Now the
composer stays enabled next to Stop and Enter queues, with the settings the
menus hold. Waiting messages show below the running turn, dimmed, marked
waiting, each with **Send now** and a remove button. With no turn running the
button reads **Send**. A message with `last_error` shows the reason.

## 20.6 Acceptance

Tests, one per behaviour (`fake-acp` learns `_session/steering`, answering
`injected` while a prompt hangs and `promptRequired` otherwise):

1. A `Completed` turn starts the next waiting message exactly once. This is
   the rule the section exists for: break it and watch the test fail.
2. Stop leaves the queue as it was and starts nothing.
3. Queueing on an idle thread starts a turn and leaves no row.
4. Send now during a running turn writes the entry into that turn after the
   reply text streamed before it, removes the row, and the turn ends once.
5. Web: the composer is enabled while a turn runs; a waiting message shows
   Send now and remove.

Before the PR, on a copy of the dev database, in the browser against the real
adapter: queue two messages, Send now one, Stop, then send the other by hand;
and run two conversations in two projects at the same time.
