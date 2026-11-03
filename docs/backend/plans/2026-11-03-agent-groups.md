# Agent Groups Implementation Plan (messaging slice 1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Mohammed creates a group of AYLUL agents, writes to it with `@mentions`, and the mentioned agents answer in the group, inside bounds, with every line recorded by Core.

**Architecture:** A new service `Messages` (`crates/core-messages`) owns groups, their members and their messages. Each member gets one hidden AYLUL thread per group; a delivery is a turn queued on that thread through `Turns::queue` (§20). `Turns` announces every turn's end on a broadcast; `Messages` listens, reads the member's reply from `Threads`, records it, and routes its mentions. Each message is a durable `MessageAdded` journal event that the AYLUL project's subscription carries live.

**Tech Stack:** Rust 1.94, SQLx 0.9 / SQLite, tokio broadcast, axum + utoipa.

**Spec:** `docs/backend/specs/2026-11-03-agent-messaging-design.md` (§6.2, §6.3, §6.6, §6.7, §7, §8 slice 1).

## Global Constraints

- CLAUDE.md "Code structure": start at `AppCore`; `Messages` is built only in `AppCore::start`; services get their dependencies in the constructor, never `AppCore`.
- New crate shape: `Cargo.toml`, `src/lib.rs`, `src/model.rs` (no `sqlx`), `src/store/`, `src/<job>.rs`, `contract.yaml`, `tests/`.
- Every changed module updates its `contract.yaml` in the same commit; the new crate gets a code-map row (`cargo test -p shadows --test codemap`).
- Ordering by an explicit `seq` per group, never `rowid` or insertion order.
- Mutations carry `CommandId` (`CommandContext::user`), with a normalized fingerprint.
- Bounds (spec decision 11): per person message, at most **3 rounds** and **10 member messages**. One constant block in `core-messages/src/bounds.rs`.
- Lines ≤ 100 characters; `cargo fmt`; clippy `-D warnings`. Never the full gate locally; each task runs its own tests.
- `api/openapi.json` regenerated in the task that adds routes.

## Review Focus

1. **A member mentions a non-member or itself.** Ignored as an address; the text is kept.
2. **A member's turn fails or is cancelled.** A `notice` from Shadows says which member and why; the round still counts; nothing hangs as "thinking".
3. **Mohammed writes while members are still answering.** His message starts a new budget; queued deliveries go in order (§20 queue), none is lost.
4. **Core restarts mid-round.** Recorded messages stay; a reply whose turn ended before the restart is not recorded twice (keyed by operation id).
5. **A deleted agent in a group.** It stops receiving; its past lines stay with its name.

---

### Task B-40: Hidden member threads (Threads)

**Files:**
- Create: `crates/core-db/migrations/0041_thread_hidden.sql` (`ALTER TABLE planning_thread ADD COLUMN hidden INTEGER NOT NULL DEFAULT 0`)
- Modify: `crates/core-threads/src/lib.rs`, `src/store/`, `contract.yaml`
- Test: `crates/core-threads/tests/` (the crate's existing thread tests file)

**Interfaces:**
- Produces: `Threads::create_hidden(&self, command_id: String, project: &ProjectId, title: &str, profile: &str) -> Result<PlanningThread, ThreadsError>`: an `aylul` thread for `profile`, command kind `thread.create_hidden`.
- `list` and `list_for_profile` leave hidden threads out. `get`, `entries` and turns work on them as on any thread.

- [ ] **Step 1:** test `a_hidden_thread_is_left_out_of_the_agent_list`: create one normal and one hidden thread for `ay`; `list_for_profile("ay")` has only the normal one; `get(hidden.id)` succeeds.
- [ ] **Step 2:** run `cargo test -p core-threads a_hidden_thread`; expect FAIL (no `create_hidden`).
- [ ] **Step 3:** add the column, the method and the filter.
- [ ] **Step 4:** run the test; expect PASS. Run `cargo test -p core-threads`.
- [ ] **Step 5:** commit `feat(core-threads): hidden threads for group members`.

### Task B-41: Turns announce a turn's end

**Files:**
- Modify: `crates/core-turns/src/lib.rs`, `src/turn.rs` (the terminal match, after the write), `src/model.rs`, `contract.yaml`
- Test: `crates/core-turns/tests/` (beside the existing turn-end tests, fake-acp)

**Interfaces:**
- Produces: `pub struct TurnEnded { pub thread_id: ThreadId, pub operation_id: OperationId, pub outcome: TurnOutcome }`, `pub enum TurnOutcome { Completed, Failed(String), Cancelled }` in `model.rs`.
- Produces: `Turns::subscribe_ended(&self) -> tokio::sync::broadcast::Receiver<TurnEnded>`; capacity 256. Sent once per turn, only after its terminal row is written; a failed terminal write sends nothing.

- [ ] **Step 1:** test `a_completed_turn_is_announced_once`: subscribe, run one fake-acp turn, receive exactly one `TurnEnded` with `Completed` and the turn's operation id. Test `a_refused_turn_is_announced_as_failed` with the refusal text.
- [ ] **Step 2:** run them; expect FAIL.
- [ ] **Step 3:** add the sender to `Turns` (built in `Turns::new`, passed to the watcher like `on_completed`); send after each successful terminal write.
- [ ] **Step 4:** run them; expect PASS. Run `cargo test -p core-turns`.
- [ ] **Step 5:** commit `feat(core-turns): announce each turn's end`.

### Task B-42: The `Messages` service, groups and their log

**Files:**
- Create: `crates/core-messages/` (`Cargo.toml`: core-db, core-threads, core-turns, core-agents, core-projects; `src/lib.rs`, `src/model.rs`, `src/error.rs`, `src/store/mod.rs`, `src/groups.rs`, `contract.yaml`, `tests/groups.rs`)
- Create: `crates/core-db/migrations/0042_messages.sql`
- Modify: `crates/shadows-core/src/app.rs` (owner row, field, accessor `messages()`), `crates/shadows-core/Cargo.toml`, `docs/codebase/README.md` (code-map row)

**Tables:**
- `message_group(id TEXT PK, name TEXT NOT NULL, created_at TEXT NOT NULL)`
- `message_group_member(group_id, slug, thread_id, delivered_seq INTEGER NOT NULL DEFAULT 0, joined_at, PRIMARY KEY(group_id, slug))`
- `message(id TEXT PK, group_id, seq INTEGER NOT NULL, speaker_kind TEXT, speaker_slug TEXT NULL, to_json TEXT, kind TEXT, body TEXT, cause_seq INTEGER NULL, operation_id TEXT NULL UNIQUE, created_at TEXT, UNIQUE(group_id, seq))`

**Interfaces:**
- Produces (`model.rs`): `GroupId`, `MessageId` (`newtype_id!`); `Group { id, name, members: Vec<Member>, needs_you: bool, created_at }`; `Member { slug, name, thread_id }`; `Speaker { Person, Agent(String), Shadows }`; `MessageKind { Chat, Notice }`; `Message { id, group_id, seq, speaker, to: Vec<Address>, kind, body, created_at }`; `Address { Person, Agent(String) }`.
- Produces: `Messages::new(storage, threads, turns, agents, projects) -> Arc<Messages>`.
- Produces: `create_group(command_id, name: &str, slugs: &[String]) -> Result<Group, MessagesError>`: 1 to 6 live agents, no duplicates; one `Threads::create_hidden` per member in the AYLUL project, titled `"<group name>"`.
- Produces: `groups() -> Result<Vec<Group>, _>` (name order, then id), `group(&GroupId)`, `messages(&GroupId, after: Option<i64>) -> Result<Vec<Message>, _>` (`seq` order).
- `needs_you`: true when the group's last message addressed to `Person` comes after the person's last message.
- Each written message appends a `MessageAdded` journal event in the same transaction: `project_id` = AYLUL project, payload `{group_id, seq}`.

- [ ] **Step 1:** tests `a_group_gets_one_hidden_thread_per_member`, `a_group_refuses_seven_members`, `a_group_refuses_a_deleted_agent`, `messages_come_back_in_seq_order`.
- [ ] **Step 2:** run `cargo test -p core-messages`; expect FAIL.
- [ ] **Step 3:** build the crate, the migration and the wiring.
- [ ] **Step 4:** run them and `cargo test -p shadows --test codemap`; expect PASS.
- [ ] **Step 5:** commit `feat(core-messages): groups and their message log`.

### Task B-43: Mohammed writes to a group

**Files:**
- Create: `crates/core-messages/src/send.rs`, `src/mentions.rs`, `src/bounds.rs`, `src/prompt.rs`
- Test: `crates/core-messages/tests/send.rs` (fake-acp)

**Interfaces:**
- Produces: `Messages::send(command_id, group: &GroupId, text: &str) -> Result<Message, MessagesError>`: records the person's `Chat` message, then delivers it.
- Mentions: `@<slug>` of a member, matched as a whole token, case-insensitive; `@mohammed` is `Address::Person`. No member mentioned: every member is addressed (spec §6.6).
- Delivery to a member: `Turns::queue(member.thread_id, SendTurn { command_id: "<message id>:<slug>", prompt, model: agent.default_model, mode: policy::default_mode("aylul"), effort: None, .. })`.
- Prompt (`prompt.rs`): the group's name, the member list with slugs, the rule "answer in 1-3 sentences; mention `@slug` only when you need that member; `@mohammed` when you need him", then every message since the member's last delivered `seq`, oldest first, at most 200 messages and 32 000 characters, the cut stated (`[N earlier messages omitted]`).
- After a delivery is queued, the member's `delivered_seq` is the last `seq` its prompt carried.

- [ ] **Step 1:** tests `a_mention_reaches_only_that_member`, `no_mention_reaches_every_member`, `a_mention_of_a_stranger_is_not_an_address`, `the_prompt_carries_only_what_is_new`.
- [ ] **Step 2:** run; expect FAIL.
- [ ] **Step 3:** implement.
- [ ] **Step 4:** run `cargo test -p core-messages`; expect PASS.
- [ ] **Step 5:** commit `feat(core-messages): send to a group with mentions`.

### Task B-44: Members answer, inside bounds

**Files:**
- Create: `crates/core-messages/src/replies.rs`
- Modify: `src/lib.rs` (start the listener in `Messages::new`'s caller: `AppCore::start` calls `messages.listen()` once)
- Test: `crates/core-messages/tests/replies.rs` (fake-acp)

**Interfaces:**
- Consumes: `Turns::subscribe_ended` (B-41), `Threads::entries` (filter `operation_id`, kind `AgentMessage`).
- Produces: `Messages::listen(self: &Arc<Self>)`: one task over `subscribe_ended`. For a member thread:
  - `Completed`: the turn's `AgentMessage` entries joined by a blank line become one `Chat` message from `Agent(slug)`, `operation_id` set (unique, so a repeat is ignored). Its mentions are routed as in B-43, under the budget of the person message that caused it (`cause_seq`).
  - `Failed(reason)` / `Cancelled`: a `Notice` from `Shadows`: "`<name>` could not answer: `<reason>`".
- Bounds: a round is one wave of deliveries; a reply's deliveries are the next round. Over 3 rounds or 10 member messages for one cause, no delivery; one `Notice`: "The group stopped here: the limit for one message was reached."
- A lagged receiver (`RecvError::Lagged`) logs `messages.lagged` and continues.

- [ ] **Step 1:** tests `a_reply_is_recorded_once`, `a_reply_mentioning_another_member_reaches_it`, `a_mention_loop_stops_at_the_bound`, `a_failed_member_turn_writes_a_notice`, `a_reply_to_mohammed_marks_needs_you`.
- [ ] **Step 2:** run; expect FAIL.
- [ ] **Step 3:** implement.
- [ ] **Step 4:** run `cargo test -p core-messages`; expect PASS.
- [ ] **Step 5:** commit `feat(core-messages): members answer inside bounds`.

### Task B-45: Routes and the live frame

**Files:**
- Create: `crates/shadows-http/src/groups.rs`
- Modify: `crates/shadows-http/src/` router and `openapi.rs`, `sse.rs` (frame `message`); `crates/core-events/src/store.rs` (allow `MessageAdded` on project subscriptions); contracts of both; `api/openapi.json`
- Test: `crates/shadows/tests/group_routes.rs`

**Interfaces:**
- `POST /api/groups` `{command_id, name, members: [slug]}` → `Group`
- `GET /api/groups` → `[Group]`; `GET /api/groups/{id}` → `Group`
- `GET /api/groups/{id}/messages?after=<seq>` → `[Message]`
- `POST /api/groups/{id}/messages` `{command_id, text}` → `Message`
- SSE: the AYLUL project's stream carries `message` frames `{group_id, seq}`; the client reads the message by route.
- Each route calls one `Messages` method and shapes the answer.

- [ ] **Step 1:** test `a_group_is_created_written_to_and_read_back` (route level, fake-acp) and `a_message_reaches_the_project_stream`.
- [ ] **Step 2:** run `cargo test -p shadows --test group_routes`; expect FAIL.
- [ ] **Step 3:** implement; regenerate `api/openapi.json`.
- [ ] **Step 4:** run them; expect PASS; `git diff --exit-code api/` shows only the new routes.
- [ ] **Step 5:** commit `feat(shadows-http): group routes and the message frame`.

---

## Frontend request (for the AYLUL desktop UI, not in this plan)

A group screen in the AYLUL desktop: create a group (name, pick up to 6 agents), the group's log with each speaker's colour, a composer with `@` completion over members, a "needs you" mark on the group, and an alert when a message is addressed to Mohammed. It reads the routes and the `message` frame of B-45. The frontend lead writes its request and U-tasks.

## Whole-branch check

The full gate runs once before the PR, in CI. Before the PR, Mohammed runs `shadows serve`, creates a group of two agents, writes `@a hello`, and reads the reply in the log.
