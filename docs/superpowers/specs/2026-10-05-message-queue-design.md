# Section 20 — Writing While a Turn Runs: the Queue and Send Now

- **Date:** 2026-10-05.
- **Status:** Design agreed with Mohammed section by section on 2026-10-05;
  awaiting his review of this file before the implementation plan.
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
| `command_id`, `fingerprint` | the `turn.queue` command that created it, for replay |
| `prompt`, `model`, `mode`, `effort`, `focus_json`, `plan_id` | `SendTurn`'s fields, as the composer held them when the message was written |
| `last_error` | why its last send failed, or `NULL` |
| `created_at` | when it was queued |

A waiting message is **not** a thread entry. It becomes one only when it is
sent: as the first entry of a new turn (20.3), or inside the running turn
(20.4). Several waiting messages stay separate, each its own turn, in
`position` order. A waiting message cannot be edited; the person removes it
and writes another.

**Adding** (`turn.queue`) checks the thread's busyness inside the same
`BEGIN IMMEDIATE` transaction that would insert the row. A busy thread gets
the row. An idle thread gets no row: the message is started as a turn through
`send`'s own path, with `send`'s refusals, and the answer says it started.
Without this a message queued in the instant a turn ended would wait for a
`Completed` that never comes.

## 20.3 Sending the next one when a turn ends

Only a turn that ends **`Completed`** sends the next waiting message. After
`Cancelled` (the person pressed Stop), `Failed` or `Interrupted` (a daemon
restart) nothing is sent: the messages wait until the person sends or removes
them. A daemon start sends nothing either.

When the watcher records `Completed`, it takes the thread's first waiting
message and starts it through `send`'s existing path. **Removing the row and
committing the new turn happen in one transaction**, so a message is sent once
or not at all. If the start is refused (the harness is unavailable, the
thread was removed …), the row stays, its `last_error` names the refusal, and
nothing more is sent from that thread until the person acts.

## 20.4 Send now

`turn.send_now` on a waiting message:

1. If a turn runs on the thread, Shadows sends `_session/steering` with the
   message's prompt and `idleBehavior: "promptRequired"` on that turn's
   session.
2. On `injected`, **one transaction** removes the row and writes the message
   as a user entry of the running turn, at the thread's next ordinal. The
   steering request is sent **before** the entry is written: a failed steer
   then leaves no entry the Planner never saw. (The probe's answer came in
   4 ms, before any reply chunk.)
3. On `promptRequired`, or when no turn runs, the message is started as a
   turn exactly as in 20.3, and the answer says so.
4. A steer that fails (the session ended, a JSON-RPC error) leaves the row,
   sets `last_error`, and writes nothing to the thread.

The running turn keeps its own operation and ends as §12.3 says; the steered
reply is part of it. The message's own `model`, `mode` and `effort` do not
apply to a steer: the running turn's settings stand.

`shadows-agent`'s `Connection` gains one call, the steering request; nothing
else in the harness changes.

## 20.5 Interface

Each route calls one `Turns` method (§14.5). No MCP tool: nothing uses one yet.

| Route | Method | Answer |
|---|---|---|
| `POST /api/threads/{id}/queue` | `queue` | the waiting message, or the turn it started (20.2) |
| `GET /api/threads/{id}/queue` | `queued` | the thread's waiting messages in `position` order |
| `DELETE /api/threads/{id}/queue/{qid}` | `unqueue` | nothing |
| `POST /api/threads/{id}/queue/{qid}/send-now` | `send_now` | `Steered`, or the turn it started |

`queue`, `unqueue` and `send_now` are commands with a `CommandId`, a kind and
a fingerprint; a replay answers what the first call answered. `unqueue` or
`send_now` on a message already sent or removed is `404 QUEUED_MESSAGE_GONE`.

Durable events, so every open tab follows: `MessageQueued`,
`QueuedMessageRemoved`, `QueuedMessageSent` (carrying whether it was steered
or started a turn) and `QueuedMessageFailed` (carrying `last_error`).

**Web client.** While a turn runs the composer stays enabled next to Stop;
Enter queues. Waiting messages show below the running turn, dimmed, marked
waiting, each with **Send now** and a remove button. With no turn running the
button reads **Send**. A message with `last_error` shows the reason.

## 20.6 Acceptance

Tests, one per behaviour (`fake-acp` learns `_session/steering`, answering
`injected` while a prompt hangs and `promptRequired` otherwise):

1. A `Completed` turn starts the next waiting message exactly once. This is
   the rule the section exists for: break it and watch the test fail.
2. Stop leaves the queue as it was and starts nothing.
3. Queueing on an idle thread starts a turn and leaves no row.
4. Send now during a running turn writes the entry into that turn, removes the
   row, and the turn ends once.
5. Web: the composer is enabled while a turn runs; a waiting message shows
   Send now and remove.

Before the PR, on a copy of the dev database, in the browser against the real
adapter: queue two messages, Send now one, Stop, then send the other by hand;
and run two conversations in two projects at the same time.
