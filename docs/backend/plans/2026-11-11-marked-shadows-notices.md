# Shadows' notices marked in the conversation — Implementation Plan

> **For agentic workers:** the task is built by `backend-builder` from its entry in
> [stages.md](../stages.md); the entry's Touch and Done when are binding, and so are this
> file's Interfaces, Rules and steps. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A `[Shadows]` notice Runs sends the Planner is recorded so the web can draw it as an
item of the Planner's activity, never as the person's bubble, with its header facts and the
executor's full report; the model still receives it as the message of its turn.

**Wish (Mohammed, 2026-11-11):** each notice drawn like Claude Code's "Message from subagent
<name>": a header such as "Shadows · T2 · attempt 3 committed" and the full report under it
as markdown, cut with Show more. It reopens "Shadows' notices marked in the conversation"
(`docs/core/future.md`, deferred by Masada on 2026-11-11).

**Shape (decided here):** the entry keeps kind `UserMessage`; its **author** says who wrote it
and its **card** carries the facts.
- `author: { kind: "Shadows", id: "runs" }` instead of `{ kind: "User", id: "local" }`.
- `card`: Runs' `AttemptNotice` (below).
- `body`: unchanged, the full notice text the model reads.

**Why not a new entry kind:** the notice is the prompt of a turn, and Core reads a turn's
prompt entry as `UserMessage` in several places: `Threads::user_operations_in` (a thread's
turns by their prompt entries), the first-message title, the fork's first turn. A new kind
would have to be taught to each; the author changes nothing they read. The web already branches on `author` for agents.

**Owners:** Runs owns the notice and its card (`AttemptNotice`, built in `notify.rs`). Turns
owns turning a queued message into a turn and its prompt entry: it gains
`Turns::queue_notice`, which records who wrote the message and its card on the waiting row and
on the entry. Threads stores the entry as it is given (`NewThreadEntry` already takes `author`
and `card`); it does not change. Turns never reads the card; Runs never writes an entry.

**Tech Stack:** Rust 1.94+, SQLite migration 0057, utoipa.

**Spec:** [executors design §5.1](../specs/2026-11-08-executors-design.md#51-the-planner-judges-mohammed-2026-11-11),
"The Planner hears how each attempt ended" (amended with this plan). Request:
[`docs/shared/requests/2026-11-11-shadows-notices.md`](../../shared/requests/2026-11-11-shadows-notices.md).

## Global Constraints

- The same as [slice 1b](2026-11-11-slice-1b-planner-runs-tasks.md#global-constraints).
- Built only once the request is `accepted`: it changes `api/openapi.json`.
- The person's own turns, queued messages and Send now do not change: author `User/local`,
  no card, the same command fingerprints.
- The next migration number is checked at dispatch (0057 when written).

## Review Focus

1. **A notice that waits** while the Planner is busy shows in the queue: the row carries the
   notice, so the web need not draw it as the person's waiting message. Held by
   `a_notice_waits_on_a_busy_planner_as_shadows`.
2. **Send now on a waiting notice** steers it into the running turn: the steered entry keeps
   the notice's author and card (`store/steer.rs`). No new test (it needs a running turn on the
   fake adapter); the review reads it.
3. **A replayed notice** (`notify-<attempt>` sent twice): the notice joins the command's
   fingerprint, so the replay answers the first, as today.
4. **A failed attempt with no report**: `report` null, `reason` set; the header still reads.
   Held by the live failure test.
5. **The person's message after a notice** is still `User/local` with no card. Held by
   `a_notice_waits_on_a_busy_planner_as_shadows` (a person's message queued after it).

---

## Task B-134: Shadows' notices carry their author and their facts

**Files:**
- Modify: `crates/core-turns/src/model.rs` (`Notice`; `QueuedMessage.notice`)
- Modify: `crates/core-turns/src/lib.rs` (re-export `Notice`)
- Modify: `crates/core-turns/src/queue.rs` (`queue_notice`; the shared path takes the notice)
- Modify: `crates/core-turns/src/store/queue.rs` (the `notice_json` column)
- Modify: `crates/core-turns/src/store/turn.rs` (the prompt entry's author and card)
- Modify: `crates/core-turns/src/store/steer.rs` (a steered notice keeps them)
- Modify: `crates/core-turns/contract.yaml`
- Create: `crates/core-db/migrations/0057_queued_notice.sql`
- Modify: `crates/core-db/tests/fixtures/base_migrations.sha384`, `crates/core-db/tests/migration_checksums.rs` (if asked), `crates/core-db/contract.yaml` (if it lists migrations)
- Modify: `crates/core-runs/src/model.rs` (`AttemptNotice`); `crates/core-runs/src/lib.rs` (re-export)
- Modify: `crates/core-runs/src/notify.rs` (builds the card, calls `queue_notice`)
- Modify: `crates/core-runs/contract.yaml`
- Modify: `crates/shadows-core/src/lib.rs` (re-export `Notice`, `AttemptNotice`)
- Modify: `crates/shadows-http/src/openapi.rs` (list `AttemptNotice` and `Notice` as schemas)
- Modify: `crates/shadows-http/src/contract.yaml`
- Test: `crates/shadows-core/tests/runs_attempts/planner.rs`
- Test: `crates/shadows/tests/planner_runs_live.rs`
- Regenerate: `api/openapi.json`

**Interfaces:**
- Produces in `core-turns`:
  - `pub struct Notice { pub from: String, pub card: serde_json::Value }`, deriving `Debug,
    Clone, Serialize, Deserialize, utoipa::ToSchema` (`card` documented as an object):
    a message Shadows wrote, not the person. `from` names the service (`runs`).
  - `QueuedMessage.notice: Option<Notice>`, `null` for the person's.
  - `impl Turns { pub async fn queue_notice(&self, thread: ThreadId, turn: SendTurn, notice: Notice) -> Result<Queued, TurnsError> }`:
    `queue` with the notice; `queue` itself passes none. The private path that `queue`,
    the watcher's dequeue and Send now share carries `Option<Notice>`; `SendTurn` does not
    change, so no caller of `send` or `queue` changes.
- Produces in `core-runs`:
  - `pub struct AttemptNotice { pub workflow_id: WorkflowId, pub version: i64, pub task: u32,
    pub title: String, pub attempt: u32, pub attempt_id: AttemptId, pub outcome: AttemptState,
    pub commit: Option<String>, pub report: Option<String>, pub reason: Option<String>,
    pub last_addition: bool }`, deriving `Debug, Clone, Serialize, utoipa::ToSchema`.

**Rules:**
- A turn started from a row with a notice writes its prompt entry with author
  `Actor { kind: "Shadows", id: notice.from }` and `card: Some(notice.card)`; the title rule is
  called with that author. Without a notice, nothing changes.
- The notice is stored as `queued_message.notice_json` (JSON of `Notice`), read back into
  `QueuedMessage.notice`, and joins the `turn.queue` and the derived `turn.start` fingerprints
  only when present, as `focus` does.
- A steered row with a notice writes its steering entry with the same author and card.
- `notify` builds one `AttemptNotice` per notice: `outcome` the attempt's state (`committed`
  or `failed`, as `AttemptState` serializes); `commit` the sha or null; `report` the executor's
  full report when committed, its last message (as B-133 reads it) when failed, else null;
  `reason` the failure reason or null; `last_addition` as the "third and last addition"
  sentence decides. Then `queue_notice(thread, turn, Notice { from: "runs", card })`. The
  body text does not change.

- [ ] **Step 1: The failing no-agent test** in `runs_attempts/planner.rs`:

```rust
#[tokio::test]
async fn a_notice_waits_on_a_busy_planner_as_shadows() {
    // The plan's Planner thread is made busy by a Pending operation written in a write
    // (Operations::insert_pending_in, as a turn start writes it).
    // queue_notice(thread, turn("[Shadows] T3 ..."), Notice { from: "runs",
    //   card: json!({ "task": 3, "attempt": 1, "outcome": "committed" }) })
    //   -> Queued::Waiting { message } with message.notice == Some(that notice).
    // queue(thread, turn("my own words")) -> Waiting with message.notice == None.
    // queued_messages(thread): both, in order, the first with its notice, the second none.
}
```

  Run `cargo test -p shadows-core --test runs_attempts a_notice_waits`: it fails to compile.
- [ ] **Step 2: Migration and Turns.** `0057_queued_notice.sql`: `ALTER TABLE queued_message
  ADD COLUMN notice_json TEXT;`. Then `Notice`, the column, `queue_notice`, the shared path,
  the prompt entry and the steer by the Rules. The test passes; `cargo test -p core-db --test
  migration_checksums` passes after its fixture is updated.
- [ ] **Step 3: Runs.** `AttemptNotice` and `notify` by the Rules.
- [ ] **Step 4: The live tests** in `planner_runs_live.rs`: in
  `live_a_committed_attempt_reaches_its_planner_and_waits_for_its_judgment` and
  `live_an_attempt_without_a_commit_reaches_its_planner`, the notice entry (found as
  `planner_entry` finds it, returning the entry) has kind `UserMessage`, `author.kind ==
  "Shadows"`, `author.id == "runs"`, and a card whose `task` is 1, `attempt` 1 and `outcome`
  `committed` (with `report` non-empty and `commit` the sha) or `failed` (with `reason`
  non-empty). Each prints `live notice card: <card>`.
- [ ] **Step 5: API.** `shadows-core` re-exports `Notice` and `AttemptNotice`; `openapi.rs`
  lists both; `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`. Only `QueuedMessage.notice`,
  `Notice` and `AttemptNotice` change in `api/openapi.json`.
- [ ] **Step 6: Contracts.** `core-turns`: `queue_notice`, `Notice`, the column, the entry's
  author and card, the steer, an obligation tested by the new test. `core-runs`:
  `AttemptNotice`, `notify` names the card and `queue_notice`, an obligation tested by the two
  live tests. `core-db`: migration 0057. `shadows-http`: the two schemas. `docs/core/future.md`'s
  row is left to `docs-sync` after the merge.
- [ ] **Step 7:** run what the entry names; commit `feat(backend): B-134 <goal>`; the request's
  backend answer gains one line: built.

## Left for Mohammed to decide

- **A notice turn and approval.** A turn a notice starts runs under the person's command
  principal (`User/local`), as every queued turn does. If the Planner calls `plan_approve` in
  it, the approval is recorded as asked in that turn, though the person wrote nothing. The
  instructions forbid approving unasked; nothing refuses it. Refusing approvals in a turn whose
  prompt is a notice is a Grants rule and needs a ruling.
- **The person removing a waiting notice.** Today the queue lets the person remove any waiting
  message, a notice too; the Planner then never hears that attempt. Whether a notice may be
  removed is the web's choice of controls unless Mohammed wants Core to refuse it.
