# Planner Context Slice 2 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When a Planner session's conversation is compacted, by hand (`/compact`) or
automatically at Claude Code's limit, or when Mohammed asks, the next turn carries the planning
context again. It also carries every instructions and standards change since the session
opened, because the compaction may have dropped those messages too.

**Architecture:**
- `shadows-agent` turns the adapter's compaction report into one event,
  `HarnessEvent::Compacted { trigger }`, sent only when a compaction completes.
- `Harness::forget_context` is the one rule for "the session lost what it heard":
  - it appends a journal entry and a `context_forget` row for the thread;
  - it resets the thread's in-memory instructions baseline to what the session opened with.
- `Harness::heard_context_digest` ignores heard rows prepared before the thread's latest forget.
- `Turns` calls `forget_context` when a turn's events carry `Compacted`.
- The route `POST /api/threads/{id}/context/resend` calls the same method with a command.

**Tech Stack:** Rust 1.94, SQLx 0.9 / SQLite, axum + utoipa. No new dependency.

**Spec:** `docs/backend/specs/2026-11-04-planner-context-design.md` §4.3 ("After a compact", "On
request", "What a compaction or a request forgets"), §6 slice 2, §9 (the plan-approval OPEN).
**Evidence:** `docs/evidence/2026-11-04-planner-context-probe.md` §1.
**Shared request:** `docs/shared/requests/2026-11-04-planning-context-resend.md` (accepted).

## What the adapter sends (read at 0.81.1, the version Shadows pins)

`dist/context-compaction.js`. Shadows declares no `session.compaction` capability, so the adapter
uses its tool-call presentation:

1. `tool_call`: `toolCallId` = the compaction id, `title` "Compact conversation", `kind`
   `think`, `status` `in_progress`, `_meta` `{ contextCompaction: { version: 1 }, claudeCode:
   { toolName: "compact" } }`.
2. Optionally one `tool_call_update` with `status` `in_progress` (a heartbeat), same `_meta`.
3. The terminal `tool_call_update`: `status` `completed` or `failed`, `rawOutput` `{ trigger:
   "manual" | "automatic", preTokens, postTokens?, durationMs? }` when known, and `_meta`
   `{ contextCompaction: { version: 1, trigger, preTokens, … }, claudeCode: { toolName:
   "compact" } }`. A later update for the same id may repeat the metadata without a `status`.
4. If the opening was missed, the first frame is already a terminal `tool_call`.

The provider-neutral key is `_meta.contextCompaction`. The plan reads that key, not the tool name.

## Global Constraints

- CLAUDE.md "Code structure": start at `AppCore`; change only the owning service. A new
  operation is a new method with its contract line in the same commit.
- Migration: `crates/core-db/migrations/0046_context_forget.sql`. Pin it in
  `crates/core-db/tests/fixtures/base_migrations.sha384` in the same task, and raise the count in
  `migration_checksums.rs`.
- New table: add it to `crates/shadows/tests/architecture/table-owners.txt` (owner `core-harness`)
  and to the foundation schema test `crates/core-db/tests/storage_contract/schema.rs`.
- Rust lines ≤ 100 characters (CI checks it); `cargo fmt`; clippy `-D warnings`.
- Never the full gate locally. Each task runs its own tests, plus the checks
  `docs/backend/AGENTS.md` rule 5 lists.
- AYLUL threads get no planning context and cannot be told to forget (`InvalidCommand`). Their
  sessions compact on the same adapter; a compaction there forgets nothing and is no failure.
- Ordering is the journal sequence: a forget's `seq` from `JournalEntry::append_event`, compared
  with `invocation_context.start_seq`. Never `rowid` or `created_at`.

## Review Focus

1. **`/compact` on a session that heard the context.** The `/compact` turn carries nothing (a
   command alone). The next normal turn carries the planning block, and its record is heard.
2. **An automatic compaction inside a turn that itself carried the context.** That turn's heard
   row was prepared before the forget, so it does not count, and the next turn carries it again.
3. **A failed compaction.** Nothing is forgotten, and the next turn carries no block.
4. **Instructions changed mid-session, then a compaction.** The change was sent once as a
   conversation message. After the compaction the next turn sends it again; without one, it does not.
5. **Two compactions before the next turn.** One re-send, not two.
6. **A resend replayed with the same `command_id`.** It answers the first result and appends no
   second forget.
7. **The known limit** (spec §4.3): a daemon restart between a forget and the next turn re-sends
   the planning context, not the earlier instructions changes. No test asserts the opposite.

---

### Task B-55: The adapter's compaction is one event (shadows-agent, fake-acp)

**Files:**
- Modify: `crates/shadows-agent/src/events.rs`: a new variant
  `HarnessEvent::Compacted { trigger: Option<String> }`, documented as "a compaction completed;
  `trigger` is `manual` or `automatic` when the adapter said".
- Modify: `crates/shadows-agent/src/acp/mapping.rs`: `forward` sends the `ToolCall` event as
  today, then `Compacted` when the update is a `tool_call` or `tool_call_update` whose `status` is
  `completed` and whose `_meta` has `contextCompaction`. `trigger` comes from
  `_meta.contextCompaction.trigger`, else `rawOutput.trigger`. A frame without `status` sends no
  `Compacted`, so a repeated metadata frame does not count twice.
- Modify: `crates/core-turns/src/entries.rs`: `Collector::push` writes no entry for `Compacted`
  (add it to the arm that ignores `Task(_)` and similar).
- Modify: `crates/fake-acp/src/prompts.rs`: three prompts that send the adapter's frames above
  for `toolCallId` `compact-1`, then end the turn:
  - `/compact`: manual, `completed`, then the text chunk `compacted`;
  - `compact-auto`: the text chunk `before`, an automatic compaction `completed`, then `after`;
  - `compact-failed`: manual, terminal `failed`, no `rawOutput`.
- Test: `crates/shadows-agent/src/acp_tests.rs`

**Interfaces:**
- Produces: `HarnessEvent::Compacted { trigger: Option<String> }`.

- [ ] **Step 1: tests** in `acp_tests.rs`, feeding `forward` the adapter's JSON:
  - `a_completed_compaction_is_reported_with_its_trigger`: the terminal update gives `ToolCall`
    then `Compacted { trigger: Some("manual") }`;
  - `a_compaction_opened_already_terminal_is_reported`: a single terminal `tool_call`;
  - `a_failed_compaction_is_not_reported`;
  - `a_repeated_metadata_frame_is_not_reported_again`: a `tool_call_update` with `_meta` and no
    `status`;
  - `an_ordinary_completed_tool_is_not_a_compaction`.
- [ ] **Step 2:** run `cargo test -p shadows-agent`; expect FAIL.
- [ ] **Step 3:** implement.
- [ ] **Step 4:** run `cargo test -p shadows-agent` and `cargo test -p core-turns`; expect PASS.
- [ ] **Step 5:** commit `feat(shadows-agent): report a completed compaction`.

### Task B-56: A thread forgets what its session heard (Harness)

**Files:**
- Create: `crates/core-db/migrations/0046_context_forget.sql`: table `context_forget`:
  - `seq INTEGER PRIMARY KEY`: the journal sequence of the forget's event;
  - `thread_id TEXT NOT NULL REFERENCES planning_thread(id)`;
  - `operation_id TEXT REFERENCES operation(id)`: the turn the compaction happened in; `NULL`
    for a request;
  - `reason TEXT NOT NULL CHECK(reason IN ('compaction','request'))`;
  - `trigger TEXT`: `manual`, `automatic` or `NULL`;
  - `created_at TEXT NOT NULL`;
  - an index on `(thread_id, seq)`.
- Modify: `crates/core-harness/src/context_record.rs` (the method),
  `crates/core-harness/src/store/context_record.rs` (its SQL and the changed digest read),
  `crates/core-harness/src/setup.rs` (the baseline reset), `crates/core-harness/src/model.rs`
  (`Forget`), `crates/core-harness/src/error.rs` (a new `HarnessError::InvalidCommand(String)`),
  `crates/core-harness/contract.yaml`
- Modify: `crates/shadows-core/src/error.rs`: map `HarnessError::InvalidCommand` to the
  existing `INVALID_COMMAND` code (HTTP 422)
- Modify: the checksum fixture and count; `crates/shadows/tests/architecture/table-owners.txt`;
  `crates/core-db/tests/storage_contract/schema.rs`; `crates/core-db/contract.yaml` (the table)
- Test: `crates/shadows/tests/context_record.rs` (Harness's record tests through `AppCore`, as in
  slice 1)

**Interfaces:**
- **Type** (`model.rs`): `pub enum Forget { Compaction { operation: OperationId, trigger:
  Option<String> }, Request { command_id: String } }`.
- **The rule:** `Harness::forget_context(&self, thread: &ThreadId, forget: Forget) ->
  Result<(), HarnessError>`, in one write transaction:
  1. read the thread; on an AYLUL thread refuse a `Request` (`HarnessError::InvalidCommand`)
     and answer a `Compaction` with nothing written;
  2. for `Request`: `CommandContext::classify` on aggregate `Thread`, kind
     `thread.context_resend`, params `{thread}`. A replay answers without writing; another param
     set is `CommandConflict`;
  3. `JournalEntry::append_event` of kind `PlannerContextForgotten` with the thread, the
     operation when there is one, and payload `{reason, trigger}`. Its returned `seq` is the
     row's `seq`;
  4. insert the `context_forget` row; for `Request`, `CommandContext::record_command`;
  5. after commit, reset the thread's held setup in `setup.rs`: `withheld =
     heard_first(thread, fork)`, the versions the session opened with (nothing heard for a fork).
     A thread with no held setup has no live session; nothing to reset.
- **Changed read:** `heard_context_digest(session)` answers the newest `heard = 1` row for that
  session whose `start_seq` is greater than the latest `context_forget.seq` of the row's thread
  (none: 0).
- No new event kind reaches the client: `core-events` filters by kind, and this one is not listed.

- [ ] **Step 1: tests** in `crates/shadows/tests/context_record.rs`:
  - `a_forget_makes_a_heard_context_owed`: prepare and hear on session-a, forget, then
    `heard_context_digest("session-a")` is `None`;
  - `a_context_prepared_after_the_forget_counts`;
  - `a_forget_in_the_turn_that_carried_it_still_counts_as_lost`: prepare on op X, forget with
    `Compaction { operation: X }`, mark X heard, and the digest is `None`;
  - `a_replayed_request_forgets_once`: same `command_id` twice, one `context_forget` row;
  - `a_request_with_other_params_conflicts`;
  - `an_aylul_thread_cannot_forget`.
- [ ] **Step 2:** run them; expect FAIL.
- [ ] **Step 3:** implement; pin the migration.
- [ ] **Step 4:** run `cargo test -p core-harness`, `cargo test -p shadows --test context_record`,
  `cargo test -p core-db --test migration_checksums` and `cargo test -p core-db --test
  storage_contract`; expect PASS.
- [ ] **Step 5:** commit `feat(core-harness): forget what a session heard`.

### Task B-57: A completed compaction forgets (Turns)

**Files:**
- Modify: `crates/core-turns/src/turn.rs`: in `accept`, before the event is published, a
  `HarnessEvent::Compacted { trigger }` calls `w.sessions.forget_context(&w.thread_id,
  Forget::Compaction { operation: w.op_id.clone(), trigger })`. A failure is logged as
  `planner.context_forget_failed` and does not fail the turn, as `record_limits` does.
- Modify: `crates/core-turns/contract.yaml` (the obligation and its tests)
- Test: `crates/shadows/tests/planning_context.rs`

**Interfaces:**
- Consumes: B-55's `HarnessEvent::Compacted`, B-56's `Harness::forget_context`.

- [ ] **Step 1: tests** in `planning_context.rs`, each on one thread and session:
  - `a_manual_compact_sends_the_context_again`: `report` (heard), `/compact` (no block, no
    record), `report` (the block, a heard record);
  - `an_automatic_compaction_sends_it_again`: `report`, `compact-auto`, `report` carries it;
  - `a_failed_compaction_forgets_nothing`: `report`, `compact-failed`, `report` has no block;
  - `two_compactions_send_it_once`: `report`, `/compact`, `/compact`, `report` carries it, and
    one more `report` does not;
  - `a_compaction_sends_changed_instructions_again`: `report`, change the project instructions,
    `report` (the change is sent), `/compact`, `report` sends the change again;
  - `without_a_compaction_a_change_is_sent_once`: the same without `/compact`; the last
    `report` has no changes block.
- [ ] **Step 2:** run `cargo test -p shadows --test planning_context`; expect FAIL.
- [ ] **Step 3:** implement.
- [ ] **Step 4:** run `cargo test -p shadows --test planning_context` and
  `cargo test -p core-turns`; expect PASS.
- [ ] **Step 5:** commit `feat(core-turns): a completed compaction makes the context owed`.

### Task B-58: The "send the context again" route (shadows-http)

**Files:**
- Modify: `crates/shadows-http/src/planning_context.rs`: `resend` translates
  `POST /api/threads/{id}/context/resend` with `{ command_id }` into
  `state.core.harness().forget_context(&thread, Forget::Request { command_id })`, inside
  `Transport::detached` like `set_anchor`. It answers `200 { owed: true }`.
- Modify: `crates/shadows-http/src/lib.rs` (the route and its `utoipa` path),
  `crates/shadows-http/src/contract.yaml`
- Modify: `api/openapi.json` and `web/src/api/schema.d.ts`, regenerated:
  `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`
- Test: `crates/shadows/tests/planning_routes.rs`

**Interfaces:**
- Consumes: B-56's `Harness::forget_context` and `Forget::Request`.

- [ ] **Step 1: tests** in `planning_routes.rs`:
  - `a_resend_makes_the_next_turn_carry_the_context`: `report`, `POST …/context/resend`,
    `report` carries the block;
  - `a_resend_on_an_aylul_thread_is_refused` (422 `INVALID_COMMAND`);
  - `a_resend_replay_answers_the_first_result` and a different-params replay is 409.
- [ ] **Step 2:** run them; expect FAIL.
- [ ] **Step 3:** implement; regenerate the API document.
- [ ] **Step 4:** run `cargo test -p shadows --test planning_routes --test openapi`; expect PASS.
- [ ] **Step 5:** commit `feat(shadows-http): send the planning context again`.

## Slice 2 acceptance (live, in this cloud environment)

Claude Code 2.1.318 runs here (checked 2026-11-04), so the controller runs the acceptance
itself, after B-55 to B-58. The live turns run on the Haiku model (Mohammed's choice,
2026-11-04).

1. `npm install` in `harness/claude` (adapter 0.81.1). Do not commit the lockfile it rewrites.
2. Start `shadows serve` on a scratch database. Create a project with a vision and two parts,
   and a Planner thread anchored at one part.
3. **Manual:** send a normal turn and check its record is heard
   (`GET /api/operations/{id}/context`). Send `/compact`. Send a normal turn: it has a record,
   and the daemon log shows `PlannerContextForgotten` with `trigger: manual`.
4. **Automatic:** a second thread whose session runs with a small
   `CLAUDE_CODE_AUTO_COMPACT_WINDOW`, then turns until the adapter reports an automatic
   compaction. The next turn carries the context.
5. **Request:** `POST /api/threads/{id}/context/resend`, then a normal turn carries it.
6. Write the dated results in `docs/evidence/2026-11-04-planner-context-probe.md` as §5. They
   close §1's "Not measured" line.
