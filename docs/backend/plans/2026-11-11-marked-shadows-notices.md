# Executor notices marked in the conversation — Implementation Plan

> **For agentic workers:** each task is built by `backend-builder` from its entry in
> [stages.md](../stages.md); the entry's Touch and Done when are binding, and so are this
> file's Interfaces, Rules and steps. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** The `[Shadows]` notice Runs sends the Planner about an attempt is recorded as the
message of that attempt's executor, never the person's: the web draws it as an item of the
Planner's activity with its header facts and the executor's full report, the person cannot
remove it from the queue, and nothing in the turn it starts can approve in the person's name.
The model still receives the same text as the message of its turn.

**Wish (Mohammed, 2026-11-11):** each notice drawn like Claude Code's "Message from subagent
<name>": a header such as "Executor (sonnet) · T2 · attempt 3 committed" and the full report
under it as markdown, cut with Show more. It reopens "Shadows' notices marked in the
conversation" (`docs/core/future.md`).

**Rulings (Mohammed, 2026-11-11, after the first version of this plan):**
1. The notice arrives in the executor's name: not the person's, not a plain "Shadows". The
   turn it starts does not run as the person, and Core refuses an approval (vision, map,
   roadmap, plan) from anyone but the person.
2. Core refuses removing a waiting notice from the queue, with a code. The person's own
   waiting messages stay removable.

**Shape (decided here):** the entry keeps kind `UserMessage`; its **author** is the executor
and its **card** carries the facts.
- `author: { kind: "Executor", id: <the attempt's executor thread id> }`. An executor has no
  agent identity of its own (it is a thread opened for one task), so its thread is its
  identity; the web opens that conversation from it.
- `card`: Runs' `AttemptNotice` (below). It names the executor's `model`, which the web shows
  as the executor's name in the header: "Executor ({model})".
- `body`: unchanged, the full notice text the model reads.
- The turn's `turn.queue` and `turn.start` commands are recorded with principal
  `Executor/<thread id>`, not `User/local`.

**Why not a new entry kind:** the notice is the prompt of a turn, and Core reads a turn's
prompt entry as `UserMessage` in several places: `Threads::user_operations_in` (a thread's
turns by their prompt entries), the first-message title, the fork's first turn. A new kind
would have to be taught to each; the author changes nothing they read.

**Owners:**
- Runs owns the notice and its card (`AttemptNotice`, built in `notify.rs`).
- Turns owns making a queued message a turn and its prompt entry. It gains
  `Turns::queue_notice`, which records the executor and the card on the waiting row, writes the
  entry with them, records the commands under the executor's principal, and refuses `unqueue`
  of that row.
- Grants owns approval authority (`Grants::authorise_in`, which all four approvals call). It
  refuses an approval whose turn holds no message from the person (B-135).
- Threads stores entries as given; only its request read (`request_entry_in`) learns to
  pick the person's message.

**Tech Stack:** Rust 1.94+, SQLite migration 0057, utoipa.

**Spec:** [executors design §5.1](../specs/2026-11-08-executors-design.md#51-the-planner-judges-mohammed-2026-11-11),
"The Planner hears how each attempt ended" (amended with this plan). Request:
[`docs/shared/requests/2026-11-11-shadows-notices.md`](../../shared/requests/2026-11-11-shadows-notices.md).

## Global Constraints

- The same as [slice 1b](2026-11-11-slice-1b-planner-runs-tasks.md#global-constraints).
- The person's own turns, queued messages, Send now and removal do not change: author
  `User/local`, no card, the same command principal and fingerprints.
- The next migration number is checked at dispatch (0057 when written).
- Order: B-134, then B-135.

## Review Focus

1. **A notice that waits** while the Planner is busy shows in the queue with its card, and
   cannot be removed. Held by `a_notice_waits_on_a_busy_planner_in_the_executors_name`.
2. **Send now on a waiting notice** steers it into the running turn: the steered entry keeps
   the executor as author and the card (`store/steer.rs`). No new test (it needs a running turn
   on the fake adapter); the review reads it.
3. **The Planner approves in a notice turn** (it was told a task is done and approves the
   plan): refused, `GRANT_SCOPE` with the reason. Held by
   `an_approval_needs_a_message_from_the_person_in_its_turn`.
4. **The person steers into a notice turn** ("approve it now"): the request is the person's
   steered message, and the approval is allowed and records it. Held by the same test.
5. **A failed attempt with no report**: `report` null, `reason` set; the header still reads.
   Held by the live failure test.

---

## Task B-134: An executor's notice is recorded in its name, with its facts, and stays queued

**Files:**
- Modify: `crates/core-turns/src/model.rs` (`Notice`; `QueuedMessage.notice`)
- Modify: `crates/core-turns/src/lib.rs` (re-export `Notice`; `start_command` takes the principal)
- Modify: `crates/core-turns/src/queue.rs` (`queue_notice`; the shared path takes the notice; `unqueue` refuses a notice)
- Modify: `crates/core-turns/src/error.rs` (`QueuedNoticeKept`)
- Modify: `crates/core-turns/src/store/queue.rs` (the `notice_json` column; the refusal in the removing write)
- Modify: `crates/core-turns/src/store/turn.rs` (the prompt entry's author and card)
- Modify: `crates/core-turns/src/store/steer.rs` (a steered notice keeps them)
- Modify: `crates/core-turns/contract.yaml`
- Create: `crates/core-db/migrations/0057_queued_notice.sql`
- Modify: `crates/core-db/tests/fixtures/base_migrations.sha384`, `crates/core-db/tests/migration_checksums.rs` (if asked), `crates/core-db/contract.yaml` (if it lists migrations)
- Modify: `crates/core-runs/src/model.rs` (`AttemptNotice`); `crates/core-runs/src/lib.rs` (re-export)
- Modify: `crates/core-runs/src/notify.rs` (builds the card, calls `queue_notice`)
- Modify: `crates/core-runs/contract.yaml`
- Modify: `crates/shadows-core/src/lib.rs` (re-export `Notice`, `AttemptNotice`)
- Modify: `crates/shadows-core/src/error.rs` (`ErrorCode::QueuedNoticeKept` and its translation)
- Modify: `crates/shadows-http/src/failure/cases.rs`, `crates/shadows-http/src/failure/mod.rs` (409), `crates/shadows-http/src/failure/mapping_tests.rs` (if it lists every code)
- Modify: `crates/shadows-mcp/src/refusal.rs` (only if it matches every `ErrorCode`)
- Modify: `crates/shadows-http/src/openapi.rs` (list `AttemptNotice` and `Notice`)
- Modify: `crates/shadows-http/src/contract.yaml`
- Test: `crates/shadows-core/tests/runs_attempts/planner.rs`
- Test: `crates/shadows/tests/planner_runs_live.rs`
- Regenerate: `api/openapi.json`

**Interfaces:**
- Produces in `core-turns`:
  - `pub struct Notice { pub executor: ThreadId, pub card: serde_json::Value }`, deriving
    `Debug, Clone, PartialEq, Serialize, Deserialize, utoipa::ToSchema` (`card` documented as
    an object): a message an executor's attempt sends the Planner through Shadows.
  - `QueuedMessage.notice: Option<Notice>`, `null` for the person's.
  - `impl Turns { pub async fn queue_notice(&self, thread: ThreadId, turn: SendTurn, notice: Notice) -> Result<Queued, TurnsError> }`:
    `queue` with the notice; `queue` itself passes none. The private path that `queue`, the
    watcher's dequeue and Send now share carries `Option<Notice>`; `SendTurn` does not change,
    so no caller of `send` or `queue` changes.
  - `TurnsError::QueuedNoticeKept`, translated to `ErrorCode::QueuedNoticeKept`
    (`QUEUED_NOTICE_KEPT`), HTTP 409, message "An executor's notice stays queued: the Planner
    must hear it."
- Produces in `core-runs`:
  - `pub struct AttemptNotice { pub workflow_id: WorkflowId, pub version: i64, pub task: u32,
    pub title: String, pub attempt: u32, pub attempt_id: AttemptId, pub model: String,
    pub outcome: AttemptState, pub commit: Option<String>, pub report: Option<String>,
    pub reason: Option<String>, pub last_addition: bool }`, deriving `Debug, Clone, Serialize,
    utoipa::ToSchema`. `task`, `attempt`, `title`, `outcome` and `model` are always set
    (Masada's acceptance); `title` is the task's title, or `T{task}` when the plan cannot be
    read.

**Rules:**
- A turn started from a row with a notice writes its prompt entry with author
  `Actor { kind: "Executor", id: notice.executor }` and `card: Some(notice.card)`; the title
  rule is called with that author. The `turn.queue` and the derived `turn.start` commands are
  recorded with principal kind `Executor` and id the executor thread id; the notice joins both
  fingerprints. Without a notice, nothing changes.
- The notice is stored as `queued_message.notice_json` (JSON of `Notice`) and read back into
  `QueuedMessage.notice`.
- `unqueue` of a row with a notice is `QueuedNoticeKept` and removes nothing; a replay of the
  same command answers the same refusal. A row without one is removed as today.
- A steered row with a notice writes its steering entry with the same author and card.
- `notify` builds one `AttemptNotice` per notice: `outcome` the attempt's state (`committed`
  or `failed`, as `AttemptState` serializes); `model` the attempt's model; `commit` the sha or
  null; `report` the executor's full report when committed, its last message (as B-133 reads
  it) when failed, else null; `reason` the failure reason or null; `last_addition` as the
  "third and last addition" sentence decides. Then
  `queue_notice(thread, turn, Notice { executor: attempt.thread_id, card })`. The body does not
  change.

- [ ] **Step 1: The failing no-agent test** in `runs_attempts/planner.rs`:

```rust
#[tokio::test]
async fn a_notice_waits_on_a_busy_planner_in_the_executors_name() {
    // The plan's Planner thread is made busy by a Pending operation written in a write
    // (Operations::insert_pending_in, as a turn start writes it); executor = executor_thread(3).
    // queue_notice(planner, turn("[Shadows] T3 ..."), Notice { executor,
    //   card: json!({ "task": 3, "attempt": 1, "outcome": "committed" }) })
    //   -> Queued::Waiting { message } with message.notice == Some(that notice).
    // queue(planner, turn("my own words")) -> Waiting with message.notice == None.
    // queued(planner): both, in order, the first with its notice, the second none.
    // unqueue(planner, &notice_row, "u1") -> Err(TurnsError::QueuedNoticeKept); still queued.
    // unqueue(planner, &own_row, "u2") -> Ok; queued(planner) holds only the notice.
    // The turn.queue command of the notice is recorded with principal ("Executor", executor):
    //   SELECT principal_kind, principal_id FROM command_record WHERE command_id = <its id>.
}
```

  Run `cargo test -p shadows-core --test runs_attempts a_notice_waits`: it fails to compile.
- [ ] **Step 2: Migration and Turns.** `0057_queued_notice.sql`: `ALTER TABLE queued_message
  ADD COLUMN notice_json TEXT;`. Then `Notice`, the column, `queue_notice`, the principal, the
  shared path, the prompt entry, the steer and the refusal by the Rules; the error code through
  `shadows-core` and `shadows-http`. The test passes; `cargo test -p core-db --test
  migration_checksums` passes after its fixture is updated.
- [ ] **Step 3: Runs.** `AttemptNotice` and `notify` by the Rules.
- [ ] **Step 4: The live tests** in `planner_runs_live.rs`: in
  `live_a_committed_attempt_reaches_its_planner_and_waits_for_its_judgment` and
  `live_an_attempt_without_a_commit_reaches_its_planner`, the notice entry (found as
  `planner_entry` finds it, returning the entry) has kind `UserMessage`, `author.kind ==
  "Executor"`, `author.id` the attempt's `thread_id`, and a card whose `task` is 1, `attempt`
  1, `model` `sonnet`, and `outcome` `committed` (with `report` non-empty and `commit` the sha)
  or `failed` (with `reason` non-empty). Each prints `live notice card: <card>`.
- [ ] **Step 5: API.** `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`. Only
  `QueuedMessage.notice`, the `Notice` and `AttemptNotice` schemas and the `QUEUED_NOTICE_KEPT`
  code change in `api/openapi.json`.
- [ ] **Step 6: Contracts.** `core-turns`: `queue_notice`, `Notice`, the column, the entry's
  author and card, the principal, the steer, `unqueue`'s new refusal, an obligation tested by
  the new test. `core-runs`: `AttemptNotice`, `notify` names the card and `queue_notice`, an
  obligation tested by the two live tests. `core-db`: migration 0057. `shadows-http`: the two
  schemas and the 409.
- [ ] **Step 7:** run what the entry names; commit `feat(backend): B-134 <goal>`; the request's
  backend answer gains one line: built.

---

## Task B-135: Only the person's message in a turn lets the Planner approve

**Files:**
- Modify: `crates/core-threads/src/store/records.rs` (`request_entry_in` reads only the person's message)
- Modify: `crates/core-threads/contract.yaml`
- Modify: `crates/core-grants/src/approval.rs` (`authorise_in` refuses when there is none)
- Modify: `crates/core-grants/contract.yaml`
- Test: `crates/shadows/tests/grants.rs`

**Interfaces:**
- `Threads::request_entry_in(conn, thread, turn) -> Result<Option<ThreadEntryId>, StorageError>`
  keeps its signature: the latest `UserMessage` of the turn whose `author_kind` is `User`; an
  entry by an executor (or anything but the person) is never the request. Its one caller is
  `Grants::authorise_in`.
- `Grants::authorise_in`: `None` from `request_entry_in` is
  `GrantsError::GrantScopeMessage("Only the person approves: this turn holds no message from the person.")`
  instead of `StorageError::NotFound("request message")`. Vision, map, roadmap and plan
  approvals all reach it (`core-design` `proposal_approval`, `map_approval`,
  `roadmap_approval`; `core-plans` `edit`), so none of them changes.

- [ ] **Step 1: The failing no-agent test** in `crates/shadows/tests/grants.rs`, through
  `Grants::authorise_in` in a write (no turn runs; `authorise_in` does not check one):

```rust
#[tokio::test]
async fn an_approval_needs_a_message_from_the_person_in_its_turn() {
    // A project, its Planner thread and its live thread grant; Writer::from_grant of it;
    // Approver::Planner { writer, turn: "op-1", harness: "claude-code" }.
    // Entry 1: UserMessage of turn op-1 by Actor { kind: "Executor", id: <an executor thread> }.
    // authorise_in -> Err(GrantsError::GrantScopeMessage(m)) with m starting "Only the person
    //   approves".
    // Entry 2: UserMessage of op-1 by Actor::user("local") (the person steered).
    // authorise_in -> Ok(Some(entry 2's id)).
    // Entry 3: UserMessage of op-1 by the executor again, after it.
    // authorise_in -> Ok(Some(entry 2's id)): the person's latest message is the request.
}
```

  Run `cargo test -p shadows --test grants an_approval_needs`: it fails on the first assertion.
- [ ] **Step 2:** change `request_entry_in` and `authorise_in` by the Interfaces. The test
  passes.
- [ ] **Step 3:** `cargo test -p shadows --test workspace_mcp --test planner_mcp --test
  plan_routes --test design_routes` pass unchanged (the person's approvals and the Planner's
  in a person's turn).
- [ ] **Step 4: Contracts.** `core-grants`: `authorise_in`'s refusal and an obligation "Only
  the person's message in a turn lets the Planner approve" tested by the new test.
  `core-threads`: `request_entry_in` reads only the person's messages.
- [ ] **Step 5:** run what the entry names; commit `feat(backend): B-135 <goal>`.
