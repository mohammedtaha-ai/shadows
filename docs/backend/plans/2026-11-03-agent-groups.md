# Agent Groups Implementation Plan (messaging slice 1, Core side)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Mohammed creates a group of AYLUL agents, writes to it with `@mentions`, and the mentioned agents answer in the group, inside bounds. Core records every line, and a reply cannot be lost, even across a restart.

**Architecture:**
- A new service `Messages` (`crates/core-messages`) owns groups, members, messages and **deliveries**.
- Each member gets one hidden AYLUL thread per group. A delivery is a durable row that `Messages` turns into a turn with `Turns::send_with_default_effort`; it does not use §20's queue.
- `Messages` learns that a turn ended by reading the durable journal through `Events`, after a cursor it stores. It records the reply and advances the cursor in one transaction, so a restart resumes where it stopped.
- Each message is a durable `MessageAdded` event, carried by the AYLUL project's stream in its existing `durable` frame.

**Tech Stack:** Rust 1.94, SQLx 0.9 / SQLite, tokio, axum + utoipa.

**Spec:** `docs/backend/specs/2026-11-03-agent-messaging-design.md` (§6.2, §6.3, §6.6, §7, §8 slice 1).

## Global Constraints

- CLAUDE.md "Code structure": start at `AppCore`. `Messages` is built only in `AppCore::start`, gets its dependencies in its constructor, and never gets `AppCore`.
- New crate shape: `Cargo.toml`, `src/lib.rs`, `src/model.rs` (no `sqlx`), `src/store/`, `src/<job>.rs`, `contract.yaml`, `tests/`.
- Every changed module updates its `contract.yaml` in the same commit. The new crate gets a code-map row (`cargo test -p shadows --test codemap`).
- Ordering by explicit `seq` (per group) and journal `seq`, never `rowid` or insertion order.
- Mutations carry `CommandContext::user` with a normalized fingerprint.
- Bounds (spec decision 11), per person message (its **cause**): at most **3 rounds** and **10 member deliveries**. One constant block in `core-messages/src/bounds.rs`.
- Migrations go in `crates/core-db/migrations/`, next numbers `0041`, `0042`.
- Lines ≤ 100 characters; `cargo fmt`; clippy `-D warnings`. Never the full gate locally; each task runs its own tests.
- `api/openapi.json` is regenerated in B-45 only.

## Routing rules (B-43 and B-44 both follow these)

| Message from | Addressed to |
|---|---|
| Mohammed, with member mentions | the mentioned members |
| Mohammed, no recognized address | every member |
| Mohammed, only `@mohammed` | Mohammed only |
| A member, with other members mentioned | those members only; never itself |
| A member, no other member mentioned | nobody (it is recorded, it wakes no one) |

- `@mohammed` in any message adds `Address::Person`. It wakes no member, and the group's `needs_you` becomes true.
- A mention is `@<slug>`, a whole token, case-insensitive. A mention of a non-member is not an address; the text stays as written.

## Delivery rules (B-43 schedules, B-44 completes)

- **A delivery row** holds `id, group_id, slug, cause_seq, round, trigger_seq, state, operation_id`
  and, once prepared, the exact inputs of its turn: `prompt, model, mode`. Its `state` is
  `waiting`, `starting`, `running`, `answered` or `failed`.
- **One turn per delivery.** Deliveries are never merged, so each reply belongs to exactly one
  delivery, one cause and one round.
- **Budget is reserved when a delivery is scheduled, not when a reply arrives.** Scheduling is
  refused when the cause already has 10 deliveries, or when the new round would be 4. A refusal
  writes one `Notice` from `Shadows`: "The group stopped here: the limit for one message was
  reached." The notice is written once per cause.
- **Dispatch, per member:** if the member has no delivery in `starting` or `running`, its oldest
  `waiting` delivery starts in two steps.
  1. **Prepare (one write):** build the prompt from the messages in
     `(delivered_seq, trigger_seq]`. Store `prompt`, `model` and `mode` on the row. Set `state` to
     `starting`, and leave the member's cursor unchanged until the turn is accepted.
  2. **Start:** call `Turns::send_with_default_effort(thread, SendTurn { command_id: "delivery:<id>", prompt, model,
     mode, .. })` with the **stored** values. On `Ok(op)`, one write sets `running` and
     `operation_id = op` and advances `delivered_seq` to `trigger_seq`.
- **Re-dispatch:** a `starting` row (after a crash, or after `ThreadBusy`) calls `send` again with
  its stored values. A replay returns the same operation; it cannot conflict, because the inputs
  are byte for byte the same. The prompt is never rebuilt, and a later change of the agent's
  model does not touch a prepared row.
- **`ThreadBusy` from `send`:** the row stays `starting`. A terminal event or a 250 ms retry
  wakes dispatch, including when a settings lease is released without a journal write.
- **Any other refusal from `send`:** the row becomes `failed`, with a `Notice`: "`<name>` could
  not answer: `<error>`". The member's next `waiting` delivery starts.
- **The prompt's cut:** past the limits, the oldest messages are dropped first. The message at
  `trigger_seq` is always kept, whole up to its own line limit.

## The listener's order (B-44)

One worker owns dispatch and terminal consumption. It reconciles prepared starts before each
journal batch, then reads endings in sequence order. A start or link storage error retries the
batch before consuming any endings. Busy attempts remain prepared and can retry after the
current turn's durable ending. A completed link and its context cursor commit together.
Unmatched endings advance the journal cursor. The worker takes the commit watch before reading,
accepts wakes from newly scheduled sends, and stops before Core closes storage.
Shutdown signals the worker first, then runs the existing bounded turn shutdown. An unfinished
delivery link stays recoverable by its command id. Harness settings RPCs use the setup deadline.

## Review Focus

1. **A member's turn ends before its delivery is linked, while the listener runs.** The listener waits at that event; the reply is recorded once, after the link.
2. **Core stops after `send` accepted a turn and before the link was written, and a new message arrives before the restart.** The re-dispatch replays with the stored inputs: no `CommandConflict`, no second turn.
3. **A member is busy when a second message reaches it, then its running turn fails.** The failure writes a notice, and the waiting delivery starts next. The group never hangs.
4. **Three members reply in parallel and each mentions the other two.** Deliveries stop at 10 for that cause, and exactly one limit notice is written.
5. **A long history pushes the triggering message past the prompt's limit.** That message is still in the prompt.

---

### Task B-40: Hidden member threads (Threads)

**Files:**
- Create: `crates/core-db/migrations/0041_thread_hidden.sql` (`ALTER TABLE planning_thread ADD COLUMN hidden INTEGER NOT NULL DEFAULT 0`)
- Modify: `crates/core-threads/src/lib.rs`, `src/store/`, `contract.yaml`
- Test: the crate's existing thread tests file under `crates/core-threads/tests/`

**Interfaces:**
- Produces: `Threads::create_hidden_in(conn: &mut SqliteConnection, project: &ProjectId, title: &str, profile: &str, now: &str) -> Result<PlanningThread, ThreadsError>`. It creates an `aylul` thread for `profile` with `hidden = 1`, inside the caller's transaction. It records no command; the caller's command covers it.
- `list` and `list_for_profile` (what clients see) leave hidden threads out.
- `list_for_profile_in` is **unchanged** and still returns hidden threads. Harness's `close_agent` uses it, so a deleted agent's group sessions close too.
- `get`, `entries` and turns work on hidden threads as on any thread.

- [ ] **Step 1: Write the failing tests.**
  - `a_hidden_thread_is_left_out_of_the_agent_list`: one normal and one hidden thread for `ay`; `list_for_profile("ay")` returns only the normal one; `get(hidden.id)` succeeds.
  - `a_hidden_thread_is_still_closed_with_its_agent`: `list_for_profile_in(tx, "ay")` returns both threads.
- [ ] **Step 2:** run `cargo test -p core-threads hidden`. Expect FAIL: `create_hidden_in` does not exist.
- [ ] **Step 3:** add the column, the method and the client-list filter.
- [ ] **Step 4:** run the tests, then `cargo test -p core-threads`. Expect PASS.
- [ ] **Step 5:** commit `feat(core-threads): hidden threads for group members`.

### Task B-41: Ended turns from the journal (Events)

**Files:**
- Modify: `crates/core-events/src/lib.rs`, `src/store.rs`, `src/model.rs`, `contract.yaml`
- Test: `crates/core-events/tests/` (the crate's journal-read tests)

**Interfaces:**
- Produces (`model.rs`): `EndedTurn { seq: i64, thread_id: ThreadId, operation_id: OperationId, outcome: EndedOutcome }` and `enum EndedOutcome { Completed, Failed, Cancelled, Interrupted }`.
- Produces: `Events::ended_turns_after(&self, after: i64, limit: i64) -> Result<Vec<EndedTurn>, StorageError>`. It reads journal rows of kind `OperationCompleted`, `OperationFailed`, `OperationCancelled` and `OperationInterrupted` that have a `thread_id`, with `seq > after`, in `seq` order.
- The caller waits on `Storage::watch_committed()` between reads. `Events` adds no task.

- [ ] **Step 1: Write the failing tests.**
  - `ended_turns_come_back_in_seq_order_after_the_cursor`: append started, completed and failed events for two threads; `ended_turns_after(cursor)` returns only the ended ones after the cursor, in order.
  - `an_interrupted_turn_is_an_ended_turn`.
- [ ] **Step 2:** run them. Expect FAIL.
- [ ] **Step 3:** implement as a read in `store.rs`.
- [ ] **Step 4:** run them, then `cargo test -p core-events`. Expect PASS.
- [ ] **Step 5:** commit `feat(core-events): read ended turns after a cursor`.

### Task B-42: The `Messages` service, groups and their log

**Files:**
- Create: `crates/core-messages/` (`Cargo.toml` with core-db, core-threads, core-turns, core-events, core-agents, core-projects; `src/lib.rs`, `src/model.rs`, `src/error.rs`, `src/store/mod.rs`, `src/groups.rs`, `contract.yaml`, `tests/groups.rs`)
- Create: `crates/core-db/migrations/0042_messages.sql`
- Modify: `crates/shadows-core/src/app.rs` (owner row, field, accessor `messages()`), `crates/shadows-core/Cargo.toml`, `docs/codebase/README.md` (code-map row)

**Tables:**
- `message_group(id TEXT PK, name TEXT NOT NULL, created_at TEXT NOT NULL)`
- `message_group_member(group_id, slug, thread_id, delivered_seq INTEGER NOT NULL DEFAULT 0, joined_at, PRIMARY KEY(group_id, slug))`
- `message(id TEXT PK, group_id, seq INTEGER NOT NULL, speaker_kind TEXT, speaker_slug TEXT NULL, to_json TEXT, kind TEXT, body TEXT, cause_seq INTEGER NULL, operation_id TEXT NULL UNIQUE, created_at TEXT, UNIQUE(group_id, seq))`
- `message_delivery(id TEXT PK, group_id, slug, cause_seq INTEGER NOT NULL, round INTEGER NOT NULL, trigger_seq INTEGER NOT NULL, state TEXT NOT NULL, prompt TEXT NULL, model TEXT NULL, mode TEXT NULL, operation_id TEXT NULL UNIQUE, created_at TEXT)`
- `message_cursor(name TEXT PK, seq INTEGER NOT NULL)`: one row, `turn_ends`.

**Interfaces:**
- Produces (`model.rs`):
  - ids: `GroupId`, `MessageId`, `DeliveryId` (`newtype_id!`);
  - `Group { id, name, members: Vec<Member>, needs_you: bool, created_at }`;
  - `Member { slug, name, thread_id }`;
  - `Speaker { Person, Agent(String), Shadows }`, `MessageKind { Chat, Notice }`, `Address { Person, Agent(String) }`;
  - `Message { id, group_id, seq, speaker, to: Vec<Address>, kind, body, created_at }`.
- Produces: `Messages::new(storage, agents) -> Arc<Messages>`. AppCore passes the existing
  Turns, Threads and Events instances when it starts the worker.
- Produces: `create_group(command_id, name: &str, slugs: &[String]) -> Result<Group, MessagesError>`. It takes 1 to 6 live agents, with no duplicates. **One write transaction** holds the command record, the group row, one `Threads::create_hidden_in` per member (AYLUL project, title = group name) and the member rows. Any failure leaves nothing.
- Produces: `groups()` (by name, then id), `group(&GroupId)`, `messages(&GroupId, after: Option<i64>)` (in `seq` order).
- `needs_you` is true when the group's last message addressed to `Person` comes after Mohammed's last message.
- Every message written appends a `MessageAdded` journal event in the same transaction, with `project_id` = the AYLUL project and payload `{group_id, seq}`.

- [ ] **Step 1: Write the failing tests.**
  - `a_group_gets_one_hidden_thread_per_member`
  - `a_group_refuses_seven_members`
  - `a_group_refuses_a_deleted_agent`
  - `a_failed_group_creation_leaves_no_thread` (the third slug is unknown; no hidden thread and no member row remain)
  - `messages_come_back_in_seq_order`
- [ ] **Step 2:** run `cargo test -p core-messages`. Expect FAIL.
- [ ] **Step 3:** build the crate, the migration and the wiring.
- [ ] **Step 4:** run the tests and `cargo test -p shadows --test codemap`. Expect PASS.
- [ ] **Step 5:** commit `feat(core-messages): groups and their message log`.

### Task B-43: Mohammed writes; deliveries are scheduled and dispatched

**Files:**
- Create: `crates/core-messages/src/send.rs`, `src/mentions.rs`, `src/bounds.rs`, `src/deliveries.rs`, `src/prompt.rs`
- Test: `crates/core-messages/tests/send.rs`; the tests that need a running turn go in
  `crates/shadows/tests/group_routes.rs` (AppCore and fake-acp)

**Interfaces:**
- Produces: `Messages::send(command_id, group: &GroupId, text: &str) -> Result<Message, MessagesError>`. It records Mohammed's `Chat` message (its `cause_seq` is its own `seq`), schedules deliveries by the routing rules at round 1 and wakes the dispatcher.
- `bounds::schedule` takes the triggering message, cause and round in the same transaction.
  `Messages::dispatch_ready` reconciles eligible deliveries in the worker.
- `SendTurn` for a delivery:
  - `prompt` from `prompt.rs`;
  - `model` = the agent's `default_model`;
  - `mode` = `policy::default_mode("aylul")`;
  - effort is selected by Turns from the selected model's current offer, with no focus, plan or attachments. The command fingerprint identifies automatic effort selection; replay returns the accepted operation before resolving the offer again.
- Prompt (`prompt.rs`):
  - the group's name and its members with their slugs;
  - the rule: "Answer in 1-3 sentences. Mention `@slug` only when you need that member, `@mohammed` when you need Mohammed.";
  - then the messages in `(delivered_seq, trigger_seq]`, oldest first, at most 200 messages and 32 000 characters, the message at `trigger_seq` always kept;
  - the cut, stated as `[N earlier messages omitted]`.

- [ ] **Step 1: Write the failing tests.**
  - `a_mention_reaches_only_that_member`
  - `no_mention_reaches_every_member`
  - `a_mention_of_a_stranger_is_not_an_address`
  - `the_prompt_carries_only_what_is_new`
  - `the_trigger_survives_the_cut`: 300 earlier messages; the triggering message is in the prompt
  - `a_second_message_to_a_busy_member_gets_its_own_turn`: the member's first turn runs; a second message waits; when the first ends, a second turn carries the second message
  - `a_redispatch_uses_the_stored_inputs`: prepare and start a delivery, drop its link, add a group message and change the agent's model; the re-dispatch returns the same operation with no `CommandConflict`
- [ ] **Step 2:** run them. Expect FAIL.
- [ ] **Step 3:** implement.
- [ ] **Step 4:** run `cargo test -p core-messages`. Expect PASS.
- [ ] **Step 5:** commit `feat(core-messages): send to a group and dispatch deliveries`.

### Task B-44: Members answer, from the journal, inside bounds

**Files:**
- Create: `crates/core-messages/src/replies.rs`
- Modify: `crates/shadows-core/src/app.rs` (`AppCore::start` calls `messages.listen()` once, after building)
- Test: `crates/shadows/tests/group_routes.rs` (Core, router and fake-acp)

**Interfaces:**
- Consumes: `Events::ended_turns_after` (B-41), `Storage::watch_committed`, `Threads::entries`
  and `Turns::operations` (to classify the failure without exposing provider diagnostics).
- Produces: `Messages::listen(self: &Arc<Self>, turns, threads, events)`, one task:
  1. A missing cursor starts at zero, so pre-existing durable endings are eligible on restart.
  2. Reconcile prepared or waiting deliveries before reading each journal batch.
  3. Read `ended_turns_after(cursor, 100)`. For a matching running delivery, one transaction:
     - **if `Completed`:** writes the turn's `AgentMessage` entries, joined by a blank line, as one `Chat` from `Agent(slug)` with `operation_id`, `cause_seq` and the next round, and schedules its addresses by the routing rules (B-43's `schedule`);
     - **if `Failed`, `Cancelled` or `Interrupted`:** writes a `Notice` from `Shadows`: "`<name>` could not answer: `<reason>`";
     - marks the delivery `answered` or `failed`;
     - advances the cursor to this event's `seq`.

     After the batch, the worker dispatches eligible members on its next pass.
  4. When no work remains it waits on the commit watch, the sender's notification or shutdown.
- `UNIQUE(message.operation_id)` stays as a second guard against a double record.

- [ ] **Step 1: Write the failing tests.**
  - `a_reply_is_recorded_once`
  - `a_reply_survives_a_restart_before_it_is_recorded`: end a member's turn with the listener stopped; start a new `Messages` over the same database; the reply is recorded once
  - `a_turn_that_ends_before_its_link_is_still_recorded`: refuse the link in the fixture
    database until the turn ends. Restart with a changed model and a newer group message.
    The prepared delivery replays the original turn and its reply is recorded once.
  - `a_reply_without_member_mentions_wakes_no_one`
  - `a_reply_to_mohammed_only_wakes_no_one_and_marks_needs_you`
  - `a_reply_mentioning_another_member_reaches_it`
  - `parallel_mentions_stop_at_ten_deliveries_with_one_notice`
  - `a_failed_turn_writes_a_notice_and_the_next_delivery_starts`
  - `a_busy_delivery_retries_after_a_lease_releases_without_a_write`
  - `shutdown_is_bounded_while_group_settings_are_unanswered`
- [ ] **Step 2:** run them. Expect FAIL.
- [ ] **Step 3:** implement.
- [ ] **Step 4:** run `cargo test -p core-messages`. Expect PASS.
- [ ] **Step 5:** commit `feat(core-messages): members answer from the journal, inside bounds`.

### Task B-45: Routes; messages on the project stream

**Files:**
- Create: `crates/shadows-http/src/groups.rs`
- Modify: the `crates/shadows-http/src/` router, `openapi.rs`, `sse.rs` (its frame description names `MessageAdded`); `crates/core-events/src/store.rs` (allow `MessageAdded` on project subscriptions); both contracts; `api/openapi.json`
- Test: `crates/shadows/tests/group_routes.rs`

**Interfaces:**

| Route | Body | Answers |
|---|---|---|
| `POST /api/groups` | `{command_id, name, members: [slug]}` | `Group` |
| `GET /api/groups` | | `[Group]` |
| `GET /api/groups/{id}` | | `Group` (each member's `thread_id` included) |
| `GET /api/groups/{id}/messages?after=<seq>` | | `[Message]` |
| `POST /api/groups/{id}/messages` | `{command_id, text}` | `Message` |

- SSE: no new frame. `MessageAdded` travels in the existing `durable` frame, whose `seq` is the journal's, so a client reconnects with `after` as for any event. Its `payload` is `{group_id, seq}` (the message's place in its group); the client then reads the message by route.
- Each route calls one `Messages` method and shapes the answer.
- Group names accept 1 to 120 trimmed characters. Person messages accept 1 to 8000.
- The web schema is generated from the changed OpenAPI document for the CI contract check.
- A member's permission request is not a new route. It is a request on the member's hidden thread, answered by the existing route with that `thread_id` (AYLUL agents spec §7.8).

- [ ] **Step 1: Write the failing tests.**
  - `a_group_is_created_written_to_and_read_back` (route level, fake-acp)
  - `a_message_reaches_the_project_stream`
  - `a_reconnect_resumes_after_messages_of_two_groups`: write to two groups, read the stream to its middle, reconnect with that journal `seq`; the rest arrives once, in order
- [ ] **Step 2:** run `cargo test -p shadows --test group_routes`. Expect FAIL.
- [ ] **Step 3:** implement; regenerate `api/openapi.json`.
- [ ] **Step 4:** run them. Expect PASS; `git diff --exit-code api/` shows only the new routes.
- [ ] **Step 5:** commit `feat(shadows-http): group routes; messages on the project stream`.

---

### Task B-46: When a group started needing Mohammed

Added after the UI plan's review: the desktop pill orders waiting items by when they started
waiting, and `Group.created_at` is not that time.

**Files:**
- Modify: `crates/core-messages/src/model.rs`, `src/store/mod.rs` (the group read),
  `src/prompt.rs` (test fixture), `contract.yaml`
- Test: `crates/core-messages/tests/groups.rs`
- Regenerate: `api/openapi.json`, `web/src/api/schema.d.ts`

**Interfaces:**
- `Group` gains `needs_you_since: Option<String>`, read in the same query as `needs_you`.
- **The first call** is the message with the smallest `seq` that addresses `Person` and comes
  after Mohammed's last message, chosen by `seq` (the group's own order), never by time.
- `needs_you_since` is that message's `created_at`.
- `needs_you` is derived from the same result: true exactly when a first call exists. The two
  fields cannot disagree.

- [x] **Step 1:** test `needs_you_since_is_the_first_unanswered_call`:
  - two member messages call `@mohammed`, then `needs_you_since` is the first one's time;
  - after Mohammed writes, it is `None`;
  - after a new call, it is the new call's time.
- [x] **Step 2:** run `cargo test -p core-messages needs_you_since`; expect FAIL.
- [x] **Step 3:** implement; regenerate the API files.
- [x] **Step 4:** run `cargo test -p core-messages` and `cargo test -p shadows --test openapi`;
  expect PASS.
- [x] **Step 5:** commit `feat(core-messages): when a group started needing Mohammed`.

## Acceptance, in two parts

1. **Core (this plan):**
   - CI passes;
   - Mohammed runs `shadows serve` and, with `curl`, creates a group of two agents and posts `@<slug> hello`;
   - he reads the reply from `GET /api/groups/{id}/messages`, and the logs show `durable` frames of kind `MessageAdded`.
2. **The slice (after the frontend's tasks):** the same, done in the AYLUL desktop. Slice 1 is done only then.

## Frontend request (the AYLUL desktop UI, not in this plan)

The frontend lead writes the request and its U-tasks:

- **A group screen:** create a group (a name, up to 6 agents); the log, with each speaker's colour and Shadows' notices set apart; a composer with `@` completion over the members.
- **Needs you:** a mark on the group, and an AYLUL alert when a message is addressed to Mohammed (a `durable` frame of kind `MessageAdded`, then a read).
- **Permission requests:** a member's request, shown and answered inside the group by its `thread_id`, through the existing answer path. Before building it, check that the panel's waiting-request count includes hidden threads, and decide whether it should.
