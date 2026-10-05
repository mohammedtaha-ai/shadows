# The Queue and Send Now (§20) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A person writes while a Planner turn runs; the message waits in a daemon-side queue, is sent as the next turn when the running one ends `Completed`, or is injected into the running turn with Send now (`_session/steering`).

**Architecture:** A `queued_message` table owned by the `turns` service. `Turns::send` gains an internal variant that can dequeue a row inside `start_turn`'s one transaction. The turn watcher calls back into `Turns` after it records `Completed`. Send now steers through a new `Connection::steer`, and the watcher, which owns every entry a turn writes, records the steered user entry after flushing the reply text it holds.

**Tech Stack:** Rust 1.94+, SQLx 0.9 on SQLite, axum + utoipa (`shadows-http`), `agent-client-protocol` 2.2.0 (`UntypedMessage` for the extension request), React + TanStack Query + vitest (`web/`).

**Spec:** [`docs/superpowers/specs/2026-10-05-message-queue-design.md`](../specs/2026-10-05-message-queue-design.md) (§20). Facts: [`docs/evidence/2026-10-05-steering-and-commands-probe.md`](../../evidence/2026-10-05-steering-and-commands-probe.md).

## Global Constraints

- Build into `CARGO_TARGET_DIR=C:/Users/Mohammed/AppData/Local/shadows-target` with `CARGO_INCREMENTAL=0`. Never into `E:\...\shadows\target`.
- Work on branch `next/polish` in `E:\Globalprojects\shadows`. No git worktrees. Do not push.
- While working, run only the test file you touch. Run the full gate once, in Task 6.
- The gate (CLAUDE.md): `cargo fmt --all --check`; `cargo clippy --workspace --all-targets --features fake-acp/test-support -- -D warnings`; `cargo test --workspace`; `cargo clippy --workspace -- -D warnings`; `cargo tree -e features,no-dev --workspace | grep test-support` prints nothing; `git diff --exit-code api/` only after the openapi file is regenerated in the same commit.
- No Rust line under `crates/` over 100 columns. Check before each commit: `LC_ALL=C git ls-files 'crates/**/*.rs' | xargs awk '{gsub(/[\200-\377]/,""); sub(/\r$/,"")} length($0)>100{print FILENAME":"FNR}'` prints nothing.
- Never edit an existing migration file (SQLx checksums them). The new one is `0019_queued_message.sql`.
- A service change updates `crates/shadows-core/src/turns/contract.yaml` in the same commit (methods, obligations, tests). A new module gets its row in `docs/codebase/README.md` (`cargo test -p shadows --test codemap` checks it).
- An adapter (`shadows-http`) reads the request, calls one `Turns` method, shapes the answer. No rule lives in it.
- Ordering is explicit: `position` for the queue, the thread's entry ordinal for entries. No `rowid`.
- Agents: find a symbol with the LSP or Grep, then Read only its line range. Read `docs/codebase/README.md` and `turns/contract.yaml` first.

## Review Focus

1. **A message sent twice.** The watcher's next-send and a Send now race for the same row; each must start or steer it at most once. Pinned by the derived command id `queued:<row id>` plus the guarded `DELETE` in `start_turn` (Task 3 test `the_watcher_and_send_now_never_send_one_message_twice`).
2. **A message that waits forever.** Queueing in the instant a turn ended (no `Completed` will come). Pinned by `queue` starting the turn itself on an idle thread (Task 2 test `queueing_on_an_idle_thread_starts_a_turn_and_leaves_no_row`).
3. **A replayed queue request that queues a second copy** after its idle-path turn started. Pinned by asking the derived `queue:<command id>` start's replay first (Task 2 test `a_replayed_queue_on_an_idle_thread_answers_its_turn`).
4. **Reply text and the steered message out of order** in the stored thread. Pinned by the watcher flushing its open message before the steered entry (Task 4 test `send_now_steers_the_running_turn_after_its_streamed_text`).
5. **Send now while a Stop is pending** must not steer. Pinned by the `cancel_requested` check (Task 4 test `send_now_after_stop_is_thread_busy_and_keeps_the_row`).

---

## File Structure

| File | Job |
|---|---|
| `crates/shadows-core/migrations/0019_queued_message.sql` (new) | the `queued_message` table |
| `crates/shadows-core/src/turns/model.rs` (modify) | `QueuedMessageId`, `QueuedMessage`, `Queued`, `SentNow` |
| `crates/shadows-core/src/turns/store/queue.rs` (new) | the queue's SQLite queries |
| `crates/shadows-core/src/turns/store/turn.rs` (modify) | `start_turn` dequeues a row in its transaction |
| `crates/shadows-core/src/turns/queue.rs` (new) | the `Turns` methods for waiting messages |
| `crates/shadows-core/src/turns/mod.rs` (modify) | `send` split into `send_with`; `Turns: Clone` |
| `crates/shadows-core/src/turns/spawn.rs`, `turn.rs`, `handles.rs`, `entries.rs` (modify) | the completed callback and the steer channel |
| `crates/shadows-core/src/db/mod.rs`, `error.rs` (modify) | `StorageError::QueuedMessageGone`, `ErrorCode::QueuedMessageGone` |
| `crates/shadows-agent/src/acp.rs` (modify) | `Connection::steer` |
| `crates/fake-acp/src/steer.rs` (new), `main.rs` (modify) | the `steerable` prompt and `_session/steering` |
| `crates/shadows-http/src/queue.rs` (new), `lib.rs`, `failure.rs` (modify) | the four routes |
| `crates/shadows/tests/message_queue.rs` (new) | every Rust test of §20 |
| `web/src/api/client.ts`, `queries.ts` (modify) | the four calls and the queue query |
| `web/src/app/conversation/waiting-messages.tsx` (new) | the waiting list with Send now and Remove |
| `web/src/app/conversation/composer.tsx`, `conversation.tsx`, `use-conversation.ts` (modify) | Enter queues while running; the list; event refetch |
| `web/src/app/conversation/waiting-messages.test.tsx` (new) | the web test |

`turns/mod.rs` is 483 lines; the new methods go in `turns/queue.rs` so it does not cross 500. `fake-acp/src/main.rs` is already 636 lines; the steering code goes in `steer.rs`. `turns/turn.rs` grows from 386 to about 440: its one job stays "the recorded ending of a live Planner turn", and recording a steered entry in its place among the turn's entries is part of that job (say so in the Task 4 commit message).

---

### Task 1: The table, the model and the store

**Files:**
- Create: `crates/shadows-core/migrations/0019_queued_message.sql`
- Create: `crates/shadows-core/src/turns/store/queue.rs`
- Modify: `crates/shadows-core/src/turns/store/mod.rs` (add `mod queue; pub(crate) use queue::*;` following the file's existing pattern)
- Modify: `crates/shadows-core/src/turns/model.rs`
- Modify: `crates/shadows-core/src/db/mod.rs:40-60` (StorageError)
- Modify: `crates/shadows-core/src/error.rs:10-60` (ErrorCode)
- Modify: `crates/shadows-http/src/failure.rs:77` (mapping)
- Modify: `docs/superpowers/specs/2026-10-05-message-queue-design.md` (20.2's table and command ids, see Step 1)
- Test: `crates/shadows/tests/message_queue.rs`

**Interfaces:**
- Produces:
  - `newtype_id!` `QueuedMessageId` in `turns/model.rs`, re-exported from `turns` and from the crate root beside `OperationId`.
  - `pub struct QueuedMessage { pub id: QueuedMessageId, pub thread_id: ThreadId, pub position: i64, pub prompt: String, pub model: String, pub mode: String, pub effort: Option<String>, pub focus: Option<Focus>, pub plan: Option<PlanId>, pub last_error: Option<String>, pub created_at: String }` — `Debug, Clone, Serialize, Deserialize, utoipa::ToSchema`.
  - `#[serde(tag = "status", rename_all = "snake_case")] pub enum Queued { Waiting { message: QueuedMessage }, Started { operation_id: OperationId } }`
  - `#[serde(tag = "status", rename_all = "snake_case")] pub enum SentNow { Steered { entry_id: ThreadEntryId }, Started { operation_id: OperationId } }`
  - `pub(crate) struct NewQueued<'a> { pub prompt: &'a str, pub model: &'a str, pub mode: &'a str, pub effort: Option<&'a str>, pub focus: Option<&'a Focus>, pub plan: Option<&'a PlanId> }`
  - `pub(crate) enum QueueAnswer { Waiting(QueuedMessage), Idle }`
  - `Storage::queue_message(&self, ctx: &CommandContext, thread: &ThreadId, new: NewQueued<'_>) -> Result<QueueAnswer, StorageError>`
  - `Storage::queued_messages(&self, thread: &ThreadId) -> Result<Vec<QueuedMessage>, StorageError>`
  - `Storage::queued_message(&self, thread: &ThreadId, id: &QueuedMessageId) -> Result<Option<QueuedMessage>, StorageError>`
  - `Storage::unqueue_message(&self, ctx: &CommandContext, thread: &ThreadId, id: &QueuedMessageId) -> Result<(), StorageError>`
  - `Storage::fail_queued(&self, thread: &ThreadId, id: &QueuedMessageId, reason: &str) -> Result<(), StorageError>`
  - `pub(crate) async fn take_queued_in(conn: &mut SqliteConnection, thread: &ThreadId, id: &QueuedMessageId) -> Result<(), StorageError>` — deletes the row or answers `QueuedMessageGone`; Task 3 and Task 4 call it inside their transactions.
  - `StorageError::QueuedMessageGone` → `ErrorCode::QueuedMessageGone` → HTTP 404.

- [ ] **Step 1: Amend the spec where the code proves it wrong**

`classify` (`db/command.rs:19`) allows one command kind per command id in a scope, so one caller id cannot be both a `turn.queue` and a `turn.start`, and the creating command lives in `command_record`, not in the row. In `docs/superpowers/specs/2026-10-05-message-queue-design.md`:
- In 20.2's table, replace the row `| \`command_id\`, \`fingerprint\` | ... |` with:
  `| (no column) | the creating \`turn.queue\` command is in \`command_record\` (§3.2), its outcome the message as first answered |`
- In 20.2, replace "That turn is recorded as `turn.start` under the caller's `command_id`, with `turn.start`'s fingerprint, and a replay" with "That turn is a `turn.start` under the derived id `queue:<command_id>`, with `turn.start`'s fingerprint, and a replay".
- In 20.3, replace "derived from the row's id (as §13.5 derives one for `draft_start`)" with "`queued:<row id>` (as §13.5 derives one for `draft_start`); Send now's start uses the same id, so whichever comes second is a replay".

- [ ] **Step 2: Write the migration**

`crates/shadows-core/migrations/0019_queued_message.sql`:

```sql
-- §20: a message the person wrote while a turn ran, waiting to be sent.
CREATE TABLE queued_message (
    id          TEXT PRIMARY KEY,
    thread_id   TEXT NOT NULL REFERENCES planning_thread(id),
    position    INTEGER NOT NULL,
    prompt      TEXT NOT NULL,
    model       TEXT NOT NULL,
    mode        TEXT NOT NULL,
    effort      TEXT,
    focus_json  TEXT,
    plan_id     TEXT,
    last_error  TEXT,
    created_at  TEXT NOT NULL,
    UNIQUE (thread_id, position)
);
```

- [ ] **Step 3: Add the error**

In `db/mod.rs` beside `ThreadBusy`:

```rust
    /// Spec §20.5: the waiting message was already sent or removed.
    #[error("the waiting message was already sent or removed")]
    QueuedMessageGone,
```

In `error.rs`, after `ThreadBusy`:

```rust
    /// The waiting message was already sent or removed (§20.5); 404.
    QueuedMessageGone,
```

In `shadows-http/src/failure.rs`, beside the `ThreadBusy` arm:

```rust
            StorageError::QueuedMessageGone => {
                return own(StatusCode::NOT_FOUND, ErrorCode::QueuedMessageGone);
            }
```

If `shadows-mcp/src/refusal.rs` matches `StorageError` exhaustively, add the same variant there as `INVALID_COMMAND` (it has no MCP caller).

- [ ] **Step 4: Write the failing store test**

`crates/shadows/tests/message_queue.rs`:

```rust
//! Spec §20: writing while a turn runs — the queue and Send now.

use serde_json::{Value, json};

#[path = "fixtures/app.rs"]
mod app;

use app::{ctx, test_app};
use shadows_core::StorageError;
use shadows_core::testing::queue::{QueueAnswer, new_queued};

#[tokio::test]
async fn waiting_messages_keep_their_order_and_a_removed_one_is_gone() {
    let app = test_app().await;
    // A queue needs a busy thread: hold one turn open at the store.
    app.storage
        .start_turn(
            &shadows_core::testing::turn::turn_command("t0", &app.thread, "hang", &app::settings()),
            shadows_core::testing::turn::new_turn(&app.thread, &app.runtime, "hang", &app::settings()),
        )
        .await
        .unwrap();
    let first = match app
        .storage
        .queue_message(&ctx("q1", "turn.queue"), &app.thread, new_queued("one"))
        .await
        .unwrap()
    {
        QueueAnswer::Waiting(m) => m,
        QueueAnswer::Idle => panic!("a busy thread queues"),
    };
    app.storage
        .queue_message(&ctx("q2", "turn.queue"), &app.thread, new_queued("two"))
        .await
        .unwrap();
    let listed = app.storage.queued_messages(&app.thread).await.unwrap();
    let prompts: Vec<&str> = listed.iter().map(|m| m.prompt.as_str()).collect();
    assert_eq!(prompts, ["one", "two"]);

    app.storage
        .unqueue_message(&ctx("u1", "turn.unqueue"), &app.thread, &first.id)
        .await
        .unwrap();
    let again = app
        .storage
        .unqueue_message(&ctx("u2", "turn.unqueue"), &app.thread, &first.id)
        .await;
    assert!(matches!(again, Err(StorageError::QueuedMessageGone)));
}
```

Add to `crates/shadows-core/src/testing/` a `queue.rs` (exported as `testing::queue`, `test-support` only) with:

```rust
use crate::turns::NewQueued;

/// A waiting message with the fixture's settings (`fake-small`, `acceptEdits`, `high`).
pub fn new_queued(prompt: &'static str) -> NewQueued<'static> {
    NewQueued {
        prompt,
        model: "fake-small",
        mode: "acceptEdits",
        effort: Some("high"),
        focus: None,
        plan: None,
    }
}
```

and `pub use crate::turns::{NewQueued, QueueAnswer};` in that same `testing/queue.rs`. If `fixtures/app.rs` has no `settings()` helper returning `TurnSettings { model: "fake-small", mode: "acceptEdits", effort: Some("high") }`, add one there.

**Visibility.** The test is an integration test in another crate, so the `Storage` methods it calls (`queue_message`, `queued_messages`, `unqueue_message`) are `pub`, as `start_turn` is, and `NewQueued`/`QueueAnswer` are `pub` types re-exported from `turns` (`pub use store::{NewQueued, QueueAnswer};`) and reached by tests through `shadows_core::testing::queue`, the way `StartedTurn` reaches `testing`. Where Step 7 writes `pub(crate)` on these three methods and two types, write `pub`.

- [ ] **Step 5: Run it to see it fail**

Run: `cargo test -p shadows --test message_queue`
Expected: compile errors naming `queue_message`, `QueueAnswer`, `new_queued`.

- [ ] **Step 6: Write the model types**

In `turns/model.rs`, add `newtype_id! { /// Spec §20.2. QueuedMessageId }` and the `QueuedMessage`, `Queued`, `SentNow` types from Interfaces. Give `QueuedMessageId` a `utoipa::ToSchema` the same way `OperationId` gets one (find it with Grep `impl.*ToSchema.*OperationId` or the `#[schema(value_type = String)]` use at its call sites, and copy that).

- [ ] **Step 7: Write the store**

`crates/shadows-core/src/turns/store/queue.rs`:

```rust
//! One job: the queue's SQLite queries (spec §20.2).

use sqlx::SqliteConnection;

use super::turn::has_open_operation;
use crate::command::CommandContext;
use crate::db::{Storage, StorageError, classify, now, record_command};
use crate::events::{Actor, DurableEvent, append_event};
use crate::plans::{Focus, PlanId};
use crate::threads::ThreadId;
use crate::turns::model::{QueuedMessage, QueuedMessageId};

const SCOPE: &str = "Thread";

pub(crate) struct NewQueued<'a> {
    pub prompt: &'a str,
    pub model: &'a str,
    pub mode: &'a str,
    pub effort: Option<&'a str>,
    pub focus: Option<&'a Focus>,
    pub plan: Option<&'a PlanId>,
}

pub(crate) enum QueueAnswer {
    Waiting(QueuedMessage),
    /// The thread runs no turn: nothing was written; the caller starts one.
    Idle,
}

type Row = (
    String, String, i64, String, String, String,
    Option<String>, Option<String>, Option<String>, Option<String>, String,
);

const COLUMNS: &str = "q.id, q.thread_id, q.position, q.prompt, q.model, q.mode, \
                       q.effort, q.focus_json, q.plan_id, q.last_error, q.created_at";

fn message(r: Row) -> Result<QueuedMessage, StorageError> {
    Ok(QueuedMessage {
        id: QueuedMessageId::from_stored(r.0),
        thread_id: ThreadId::from_stored(r.1),
        position: r.2,
        prompt: r.3,
        model: r.4,
        mode: r.5,
        effort: r.6,
        focus: r.7.map(|f| serde_json::from_str(&f)).transpose()?,
        plan: r.8.map(PlanId::from_stored),
        last_error: r.9,
        created_at: r.10,
    })
}

async fn event(
    conn: &mut SqliteConnection,
    kind: &str,
    thread: &ThreadId,
    actor: Actor,
    payload: serde_json::Value,
    ts: &str,
) -> Result<(), StorageError> {
    append_event(
        conn,
        &DurableEvent::new(kind, actor).with_thread(thread).with_payload(payload),
        ts,
    )
    .await
}

/// Removes a waiting message inside the caller's transaction, or answers
/// `QueuedMessageGone`: of two writers racing for one row, one wins.
pub(crate) async fn take_queued_in(
    conn: &mut SqliteConnection,
    thread: &ThreadId,
    id: &QueuedMessageId,
) -> Result<(), StorageError> {
    let gone = sqlx::query("DELETE FROM queued_message WHERE id = ? AND thread_id = ?")
        .bind(id.as_str())
        .bind(thread.as_str())
        .execute(&mut *conn)
        .await?
        .rows_affected()
        == 0;
    if gone {
        return Err(StorageError::QueuedMessageGone);
    }
    Ok(())
}

impl Storage {
    /// §20.2: a replay answers the message as first answered; a removed
    /// thread is `NotFound`; an idle thread writes nothing and is `Idle`;
    /// a busy one gets the row, last in `position` order.
    pub(crate) async fn queue_message(
        &self,
        ctx: &CommandContext,
        thread: &ThreadId,
        new: NewQueued<'_>,
    ) -> Result<QueueAnswer, StorageError> {
        let (ctx, thread, ts) = (ctx.clone(), thread.clone(), now());
        let id = QueuedMessageId::generate();
        let fields = (
            new.prompt.to_owned(),
            new.model.to_owned(),
            new.mode.to_owned(),
            new.effort.map(str::to_owned),
            new.focus.map(serde_json::to_string).transpose()?,
            new.plan.map(|p| p.as_str().to_owned()),
        );
        self.write_txn(move |conn| {
            Box::pin(async move {
                if let Some(outcome) = classify(conn, &ctx, SCOPE, thread.as_str()).await? {
                    return Ok(QueueAnswer::Waiting(serde_json::from_str(&outcome)?));
                }
                let live: Option<i64> = sqlx::query_scalar(
                    "SELECT 1 FROM planning_thread WHERE id = ? AND removed_at IS NULL",
                )
                .bind(thread.as_str())
                .fetch_optional(&mut *conn)
                .await?;
                live.ok_or(StorageError::NotFound("planning_thread"))?;
                if !has_open_operation(conn, &thread).await? {
                    return Ok(QueueAnswer::Idle);
                }
                let position: i64 = sqlx::query_scalar(
                    "SELECT COALESCE(MAX(position), 0) + 1 FROM queued_message WHERE thread_id = ?",
                )
                .bind(thread.as_str())
                .fetch_one(&mut *conn)
                .await?;
                let (prompt, model, mode, effort, focus, plan) = &fields;
                sqlx::query(
                    "INSERT INTO queued_message
                       (id, thread_id, position, prompt, model, mode, effort,
                        focus_json, plan_id, last_error, created_at)
                     VALUES (?,?,?,?,?,?,?,?,?,NULL,?)",
                )
                .bind(id.as_str())
                .bind(thread.as_str())
                .bind(position)
                .bind(prompt)
                .bind(model)
                .bind(mode)
                .bind(effort)
                .bind(focus)
                .bind(plan)
                .bind(&ts)
                .execute(&mut *conn)
                .await?;
                let row: Row = sqlx::query_as(&format!(
                    "SELECT {COLUMNS} FROM queued_message q WHERE q.id = ?"
                ))
                .bind(id.as_str())
                .fetch_one(&mut *conn)
                .await?;
                let queued = message(row)?;
                let actor = Actor::user(&ctx.principal_id);
                let payload = serde_json::json!({ "queued_id": id.as_str() });
                event(conn, "MessageQueued", &thread, actor, payload, &ts).await?;
                let outcome = serde_json::to_string(&queued)?;
                record_command(conn, &ctx, SCOPE, thread.as_str(), "QueuedMessage", &outcome, &ts)
                    .await?;
                Ok(QueueAnswer::Waiting(queued))
            })
        })
        .await
    }

    /// The thread's waiting messages in `position` order; none for a removed thread.
    pub async fn queued_messages(
        &self,
        thread: &ThreadId,
    ) -> Result<Vec<QueuedMessage>, StorageError> {
        let rows: Vec<Row> = sqlx::query_as(&format!(
            "SELECT {COLUMNS} FROM queued_message q
               JOIN planning_thread t ON t.id = q.thread_id AND t.removed_at IS NULL
              WHERE q.thread_id = ? ORDER BY q.position"
        ))
        .bind(thread.as_str())
        .fetch_all(self.reader())
        .await?;
        rows.into_iter().map(message).collect()
    }

    pub async fn queued_message(
        &self,
        thread: &ThreadId,
        id: &QueuedMessageId,
    ) -> Result<Option<QueuedMessage>, StorageError> {
        let row: Option<Row> = sqlx::query_as(&format!(
            "SELECT {COLUMNS} FROM queued_message q
               JOIN planning_thread t ON t.id = q.thread_id AND t.removed_at IS NULL
              WHERE q.thread_id = ? AND q.id = ?"
        ))
        .bind(thread.as_str())
        .bind(id.as_str())
        .fetch_optional(self.reader())
        .await?;
        row.map(message).transpose()
    }

    /// §20.5: a replay is judged first; a new command on a gone row is
    /// `QueuedMessageGone`.
    pub async fn unqueue_message(
        &self,
        ctx: &CommandContext,
        thread: &ThreadId,
        id: &QueuedMessageId,
    ) -> Result<(), StorageError> {
        let (ctx, thread, id, ts) = (ctx.clone(), thread.clone(), id.clone(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                if classify(conn, &ctx, SCOPE, thread.as_str()).await?.is_some() {
                    return Ok(());
                }
                take_queued_in(conn, &thread, &id).await?;
                let actor = Actor::user(&ctx.principal_id);
                let payload = serde_json::json!({ "queued_id": id.as_str() });
                event(conn, "QueuedMessageRemoved", &thread, actor, payload, &ts).await?;
                record_command(conn, &ctx, SCOPE, thread.as_str(), "QueuedMessage", "{}", &ts)
                    .await
            })
        })
        .await
    }

    /// §20.3, §20.4: why the last send of a waiting message failed.
    pub async fn fail_queued(
        &self,
        thread: &ThreadId,
        id: &QueuedMessageId,
        reason: &str,
    ) -> Result<(), StorageError> {
        let (thread, id, reason, ts) = (thread.clone(), id.clone(), reason.to_owned(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                let found = sqlx::query(
                    "UPDATE queued_message SET last_error = ? WHERE id = ? AND thread_id = ?",
                )
                .bind(&reason)
                .bind(id.as_str())
                .bind(thread.as_str())
                .execute(&mut *conn)
                .await?
                .rows_affected();
                if found == 0 {
                    return Ok(());
                }
                let payload = serde_json::json!({ "queued_id": id.as_str(), "last_error": reason });
                event(conn, "QueuedMessageFailed", &thread, Actor::system(), payload, &ts).await
            })
        })
        .await
    }
}
```

Adjust imports to where `append_event`, `DurableEvent` and `write_txn`'s closure type really live (Grep `pub(crate) async fn append_event` and an existing `write_txn` caller such as `plans/store/plan.rs:55`). `has_open_operation` is already `pub(crate)` in `turns/store/turn.rs:103`. If `PlanId::from_stored` is not reachable here, read `plan_id` the way `turns/store/operation_read.rs` reads ids. Keep `record_command`'s argument order exactly as `turns/store/turn.rs:230` calls it.

- [ ] **Step 8: Run the test to see it pass**

Run: `cargo test -p shadows --test message_queue`
Expected: `waiting_messages_keep_their_order_and_a_removed_one_is_gone ... ok`.

- [ ] **Step 9: Contract, codemap, commit**

- `turns/contract.yaml`: under `shapes`, add `QueuedMessage`, `Queued`, `SentNow` (fields as in Interfaces); under the store functions, add `queue_message`, `queued_messages`, `queued_message`, `unqueue_message`, `fail_queued`, `take_queued_in` with their signatures and one-line rules (copy the rule sentences from the doc comments above); add the test name.
- `docs/codebase/README.md`: a row for `crates/shadows-core/src/turns/store/queue.rs` — job "the queue's SQLite queries" — under the existing `turns/store/` row style.
- `cargo test -p shadows --test codemap` and `cargo test -p shadows-core --test contracts`, then the 100-column check.

```bash
git add crates/shadows-core docs crates/shadows-http/src/failure.rs crates/shadows/tests/message_queue.rs crates/shadows-mcp
git commit -m "feat(turns): the queued_message table and its store (§20.2)"
```

---

### Task 2: `queue`, `queued`, `unqueue` and their routes

**Files:**
- Create: `crates/shadows-core/src/turns/queue.rs`
- Modify: `crates/shadows-core/src/turns/mod.rs` (`mod queue;`, `#[derive(Clone)]` on `Turns`, `SendTurn: Clone`, `start_command` helper)
- Create: `crates/shadows-http/src/queue.rs`
- Modify: `crates/shadows-http/src/lib.rs` (router and `#[openapi(paths(...))]`)
- Modify: `crates/shadows/tests/openapi.rs` (the route list near line 105)
- Modify: `api/openapi.json` (regenerated)
- Test: `crates/shadows/tests/message_queue.rs`

**Interfaces:**
- Consumes: Task 1's store and types.
- Produces:
  - `Turns::queue(&self, thread: ThreadId, turn: SendTurn) -> Result<Queued, CoreError>`
  - `Turns::queued(&self, thread: &ThreadId) -> Result<Vec<QueuedMessage>, CoreError>`
  - `Turns::unqueue(&self, thread: &ThreadId, id: &QueuedMessageId, command_id: String) -> Result<(), CoreError>`
  - `fn start_command(command_id: String, thread: &ThreadId, turn: &SendTurn) -> CommandContext` in `turns/mod.rs`, used by `send` and `queue`, so a derived start replays with `send`'s exact fingerprint.
  - Routes: `POST /api/threads/{id}/queue` (body `StartTurn`, 202 `Queued`), `GET /api/threads/{id}/queue` (200 `Vec<QueuedMessage>`), `DELETE /api/threads/{id}/queue/{qid}?command_id=…` (204).

- [ ] **Step 1: Write the failing tests**

Append to `crates/shadows/tests/message_queue.rs` (add `use app::{call, default_settings, fresh_command, post};` — import only what the file uses, since clippy denies unused imports):

```rust
fn queue_body(prompt: &str) -> Value {
    let mut body = default_settings();
    body["command_id"] = json!(fresh_command());
    body["prompt"] = json!(prompt);
    body
}

#[tokio::test]
async fn queueing_on_an_idle_thread_starts_a_turn_and_leaves_no_row() {
    let app = test_app().await;
    let path = format!("/api/threads/{}/queue", app.thread.as_str());
    let (status, answer) = post(&app, &path, queue_body("hello")).await;
    assert_eq!(status, 202, "{answer}");
    assert_eq!(answer["status"], "started");
    assert!(answer["operation_id"].is_string());
    let (_, listed) = call(&app, "GET", &path, None).await;
    assert_eq!(listed, json!([]));
}

#[tokio::test]
async fn a_replayed_queue_on_an_idle_thread_answers_its_turn() {
    let app = test_app().await;
    let path = format!("/api/threads/{}/queue", app.thread.as_str());
    let mut body = queue_body("wait-for-release");
    body["command_id"] = json!("same");
    let (_, first) = post(&app, &path, body.clone()).await;
    // The turn it started still runs: a replay must not queue a copy behind it.
    let (status, again) = post(&app, &path, body).await;
    assert_eq!(status, 202, "{again}");
    assert_eq!(again["status"], "started");
    assert_eq!(again["operation_id"], first["operation_id"]);
    let (_, listed) = call(&app, "GET", &path, None).await;
    assert_eq!(listed, json!([]));
    std::fs::write(app.dir().join("release"), "").unwrap();
}

#[tokio::test]
async fn a_busy_thread_queues_and_remove_answers_gone_the_second_time() {
    let app = test_app().await;
    let path = format!("/api/threads/{}/queue", app.thread.as_str());
    // On the idle thread this starts the turn that holds it busy.
    let (_, started) = post(&app, &path, queue_body("wait-for-release")).await;
    assert_eq!(started["status"], "started");
    let (status, queued) = post(&app, &path, queue_body("next one")).await;
    assert_eq!(status, 202, "{queued}");
    assert_eq!(queued["status"], "waiting");
    assert_eq!(queued["message"]["prompt"], "next one");
    let qid = queued["message"]["id"].as_str().unwrap();
    let remove = format!("{path}/{qid}?command_id={}", fresh_command());
    assert_eq!(call(&app, "DELETE", &remove, None).await.0, 204);
    let again = format!("{path}/{qid}?command_id={}", fresh_command());
    let (status, gone) = call(&app, "DELETE", &again, None).await;
    assert_eq!(status, 404);
    assert_eq!(gone["code"], "QUEUED_MESSAGE_GONE");
    std::fs::write(app.dir().join("release"), "").unwrap();
}
```

`app.dir()` is the project folder fake-acp runs in (where `wait-for-release` looks for `release`). If the fixture names it differently, use that name; Grep `"release"` in `crates/shadows/tests` for an existing use.

- [ ] **Step 2: Run to see them fail**

Run: `cargo test -p shadows --test message_queue`
Expected: the three new tests fail with 404/405 (no route).

- [ ] **Step 3: Split the start command out of `send`**

In `turns/mod.rs` add `#[derive(Clone)]` to `SendTurn` and to `Turns` (every field is an `Arc`, the `Bus` sender or `Code`, all `Clone`). Move the params building at lines 213-227 into:

```rust
/// `turn.start`'s command for `turn` on `thread` (§12.7): `send` asks its
/// replay first, and `queue` asks the replay of the start it derived.
fn start_command(command_id: String, thread: &ThreadId, turn: &SendTurn) -> CommandContext {
    let mut params = serde_json::json!({
        "thread_id": thread,
        "prompt": turn.prompt,
        "model": turn.model,
        "mode": turn.mode,
        "effort": turn.effort,
    });
    // Absent without a focus, so a turn recorded before §13.9 replays as it did.
    if let Some(focus) = &turn.focus {
        params["focus"] = serde_json::json!(focus);
    }
    if let Some(plan) = &turn.plan {
        params["plan"] = serde_json::json!(plan);
    }
    user_command(command_id, "turn.start", params)
}
```

and in `send` replace the inline building with `let command = start_command(turn.command_id.clone(), &thread_id, &turn);` before destructuring `turn`. Behaviour is unchanged: run `cargo test -p shadows --test turn_command` and see it pass.

- [ ] **Step 4: Write `turns/queue.rs`**

```rust
//! One job: the waiting messages of a conversation (spec §20) — queueing,
//! listing and removing them, sending the next, and Send now.

use super::{SendTurn, Turns, start_command};
use crate::app::user_command;
use crate::db::StorageError;
use crate::error::CoreError;
use crate::threads::ThreadId;
use crate::turns::model::{Queued, QueuedMessage, QueuedMessageId};
use crate::turns::store::{NewQueued, QueueAnswer};

/// The id of the turn a `turn.queue` starts on an idle thread (§20.2).
fn idle_start_id(command_id: &str) -> String {
    format!("queue:{command_id}")
}

impl Turns {
    /// §20.2: a waiting message on a busy thread; on an idle one, the turn
    /// it starts. A replay answers what the first call answered.
    pub async fn queue(&self, thread: ThreadId, turn: SendTurn) -> Result<Queued, CoreError> {
        let derived = start_command(idle_start_id(&turn.command_id), &thread, &turn);
        if let Some(replay) = self.storage.replayed_turn(&derived, &thread).await? {
            return Ok(Queued::Started { operation_id: replay.operation_id });
        }
        let params = serde_json::json!({
            "thread_id": thread,
            "prompt": turn.prompt,
            "model": turn.model,
            "mode": turn.mode,
            "effort": turn.effort,
            "focus": turn.focus,
            "plan": turn.plan,
        });
        let command = user_command(turn.command_id.clone(), "turn.queue", params);
        let new = || NewQueued {
            prompt: &turn.prompt,
            model: &turn.model,
            mode: &turn.mode,
            effort: turn.effort.as_deref(),
            focus: turn.focus.as_ref(),
            plan: turn.plan.as_ref(),
        };
        // Twice at most: a turn may take the thread between `Idle` and the start.
        for _ in 0..2 {
            match self.storage.queue_message(&command, &thread, new()).await? {
                QueueAnswer::Waiting(message) => return Ok(Queued::Waiting { message }),
                QueueAnswer::Idle => {
                    let start = SendTurn {
                        command_id: idle_start_id(&turn.command_id),
                        ..turn.clone()
                    };
                    match self.send(thread.clone(), start).await {
                        Err(CoreError::Storage(StorageError::ThreadBusy)) => continue,
                        other => {
                            return other.map(|operation_id| Queued::Started { operation_id });
                        }
                    }
                }
            }
        }
        Err(StorageError::ThreadBusy.into())
    }

    /// §20.5: the thread's waiting messages in `position` order.
    pub async fn queued(&self, thread: &ThreadId) -> Result<Vec<QueuedMessage>, CoreError> {
        Ok(self.storage.queued_messages(thread).await?)
    }

    /// §20.5: removes a waiting message; a replay answers as the first did.
    pub async fn unqueue(
        &self,
        thread: &ThreadId,
        id: &QueuedMessageId,
        command_id: String,
    ) -> Result<(), CoreError> {
        let params = serde_json::json!({ "thread_id": thread, "queued_id": id });
        let command = user_command(command_id, "turn.unqueue", params);
        Ok(self.storage.unqueue_message(&command, thread, id).await?)
    }
}
```

`send` uses `start_command` with the id it is given, so the derived `queue:<id>` start and its replay share one fingerprint.

- [ ] **Step 5: Write the routes**

`crates/shadows-http/src/queue.rs`, following `conversation.rs`'s `start_turn` (state, `detached`, `Failure`, the `utoipa::path` attribute with every status the method can answer):

```rust
//! One job: the routes of a conversation's waiting messages (spec §20.5).

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use shadows_core::{Queued, QueuedMessage, QueuedMessageId, SendTurn, ThreadId};

use crate::{AppState, Failure, detached};
use crate::conversation::StartTurn;

#[derive(serde::Deserialize, utoipa::IntoParams)]
pub(super) struct CommandParam {
    /// The idempotency key (spec §3.2).
    command_id: String,
}

/// Queues a message while a turn runs; on an idle thread starts it as a turn.
#[utoipa::path(
    post, path = "/api/threads/{id}/queue", tag = "turns",
    params(("id" = ThreadId, Path, description = "The thread")),
    request_body = StartTurn,
    responses(
        (status = 202, body = Queued),
        (status = 404, description = "INVALID_COMMAND: no such thread", body = crate::ErrorBody),
        (status = 409, description = "THREAD_BUSY; COMMAND_CONFLICT", body = crate::ErrorBody),
        (status = 422, description = "as POST /api/threads/{id}/turns", body = crate::ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = crate::ErrorBody),
        (status = 502, description = "HARNESS_START_FAILED", body = crate::ErrorBody),
        (status = 503, description = "RUNTIME_STOPPING", body = crate::ErrorBody),
    )
)]
pub(super) async fn queue(
    State(s): State<AppState>,
    Path(thread): Path<ThreadId>,
    Json(body): Json<StartTurn>,
) -> Result<(StatusCode, Json<Queued>), Failure> {
    let turn: SendTurn = body.into();
    let core = s.core.clone();
    let answer =
        detached(async move { core.turns().queue(thread, turn).await.map_err(Failure::from) })
            .await?;
    Ok((StatusCode::ACCEPTED, Json(answer)))
}

/// The thread's waiting messages, oldest first.
#[utoipa::path(
    get, path = "/api/threads/{id}/queue", tag = "turns",
    params(("id" = ThreadId, Path, description = "The thread")),
    responses(
        (status = 200, body = Vec<QueuedMessage>),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = crate::ErrorBody),
    )
)]
pub(super) async fn queued(
    State(s): State<AppState>,
    Path(thread): Path<ThreadId>,
) -> Result<Json<Vec<QueuedMessage>>, Failure> {
    Ok(Json(s.core.turns().queued(&thread).await?))
}

/// Removes a waiting message.
#[utoipa::path(
    delete, path = "/api/threads/{id}/queue/{qid}", tag = "turns",
    params(
        ("id" = ThreadId, Path, description = "The thread"),
        ("qid" = QueuedMessageId, Path, description = "The waiting message"),
        CommandParam,
    ),
    responses(
        (status = 204),
        (status = 404, description = "QUEUED_MESSAGE_GONE", body = crate::ErrorBody),
        (status = 409, description = "COMMAND_CONFLICT", body = crate::ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = crate::ErrorBody),
    )
)]
pub(super) async fn unqueue(
    State(s): State<AppState>,
    Path((thread, qid)): Path<(ThreadId, QueuedMessageId)>,
    Query(p): Query<CommandParam>,
) -> Result<StatusCode, Failure> {
    s.core.turns().unqueue(&thread, &qid, p.command_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

`StartTurn` → `SendTurn`: if `conversation.rs` has no `From<StartTurn> for SendTurn`, add one there and make `start_turn` use it (it currently destructures field by field at lines 145-163). Make `StartTurn` `pub(super)` if it is private. Re-export `Queued`, `QueuedMessage`, `QueuedMessageId`, `SentNow` from the `shadows_core` root beside `SendTurn`. Register the three handlers in `lib.rs`'s router (`.route("/api/threads/{id}/queue", post(queue::queue).get(queue::queued))`, `.route("/api/threads/{id}/queue/{qid}", delete(queue::unqueue))`) and in the `#[openapi(paths(...))]` list; add `"POST /api/threads/{id}/queue"`, `"GET /api/threads/{id}/queue"`, `"DELETE /api/threads/{id}/queue/{qid}"` to `tests/openapi.rs`'s route list.

- [ ] **Step 6: Regenerate the protocol and run the tests**

Run: `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`, then `cargo test -p shadows --test openapi` and `cargo test -p shadows --test message_queue`.
Expected: all pass; `api/openapi.json` gained the three paths and the `Queued`, `QueuedMessage` schemas.

- [ ] **Step 7: Contract, codemap, commit**

- `turns/contract.yaml`: `functions` gains `queue`, `queued`, `unqueue` with signatures, their rules (the doc comments), refusals (`queue`: `send`'s refusals plus `ThreadBusy` after two tries; `unqueue`: `QueuedMessageGone`, `CommandConflict`) and `tested_by`.
- `docs/codebase/README.md`: rows for `crates/shadows-core/src/turns/queue.rs` ("the waiting messages of a conversation") and `crates/shadows-http/src/queue.rs` ("the routes of a conversation's waiting messages"), in the style of their neighbours.
- `cargo test -p shadows --test codemap`, `cargo test -p shadows-core --test contracts`, 100-column check.

```bash
git add crates api docs
git commit -m "feat(turns): queue, list and remove waiting messages over HTTP (§20.2, §20.5)"
```

---

### Task 3: Sending the next one when a turn completes

**Files:**
- Modify: `crates/shadows-core/src/turns/store/turn.rs` (`NewTurn.dequeue`, the take in `start_turn`)
- Modify: `crates/shadows-core/src/turns/mod.rs` (`send_with`, `on_completed`)
- Modify: `crates/shadows-core/src/turns/spawn.rs`, `turn.rs` (the callback)
- Modify: `crates/shadows-core/src/turns/queue.rs` (`send_next`, `send_queued`)
- Modify: `crates/shadows-core/src/testing/turn.rs:99`, `crates/shadows/tests/plan_boundaries.rs:176`, `crates/shadows/tests/turn_command.rs:576` (new fields `None`)
- Test: `crates/shadows/tests/message_queue.rs`

**Interfaces:**
- Consumes: Task 1 `take_queued_in`; Task 2 `start_command`, `Turns: Clone`.
- Produces:
  - `pub struct Dequeue<'a> { pub id: &'a QueuedMessageId, pub also: Option<&'a CommandContext> }` in `turns/store/turn.rs`; `NewTurn` gains `pub dequeue: Option<Dequeue<'a>>`.
  - `pub type OnCompleted = Arc<dyn Fn(ThreadId) + Send + Sync>` in `turns/spawn.rs`; `PlannerTurnRequest` gains `pub on_completed: Option<OnCompleted>`.
  - `Turns::send_with(&self, thread_id: ThreadId, turn: SendTurn, dequeue: Option<(QueuedMessageId, Option<CommandContext>)>) -> Result<OperationId, CoreError>` (private; `send` calls it with `None`).
  - `Turns::send_queued(&self, thread: &ThreadId, row: &QueuedMessage, also: Option<CommandContext>) -> Result<OperationId, CoreError>` (`pub(crate)`).
  - `Turns::send_next(&self, thread: &ThreadId)` (`pub(crate)`).
  - Event `QueuedMessageSent` with payload `{ "queued_id", "how": "turn", "operation_id" }`.

- [ ] **Step 1: Write the failing tests**

Append to `message_queue.rs`:

```rust
async fn operations(app: &app::App) -> Vec<shadows_core::Operation> {
    app.storage.list_operations_for_thread(&app.thread).await.unwrap()
}

async fn until_ops(app: &app::App, n: usize) -> Vec<shadows_core::Operation> {
    for _ in 0..500 {
        let ops = operations(app).await;
        if ops.len() >= n && ops.iter().all(|o| o.finished_at.is_some()) {
            return ops;
        }
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    }
    panic!("the thread never reached {n} finished turns");
}

#[tokio::test]
async fn a_completed_turn_starts_the_next_waiting_message_exactly_once() {
    let app = test_app().await;
    let path = format!("/api/threads/{}/queue", app.thread.as_str());
    post(&app, &path, queue_body("wait-for-release")).await;
    let (_, queued) = post(&app, &path, queue_body("after it")).await;
    assert_eq!(queued["status"], "waiting");
    std::fs::write(app.dir().join("release"), "").unwrap();

    let ops = until_ops(&app, 2).await;
    assert_eq!(ops.len(), 2);
    assert!(ops.iter().all(|o| o.status_kind == "Completed"), "{ops:?}");
    let sent = app::entries(&app)
        .await
        .into_iter()
        .filter(|e| e.body == "after it")
        .count();
    assert_eq!(sent, 1);
    let (_, listed) = call(&app, "GET", &path, None).await;
    assert_eq!(listed, json!([]));
}

#[tokio::test]
async fn stop_leaves_the_queue_and_starts_nothing() {
    let app = test_app().await;
    let path = format!("/api/threads/{}/queue", app.thread.as_str());
    let (_, first) = post(&app, &path, queue_body("hang")).await;
    post(&app, &path, queue_body("waits")).await;
    let op = first["operation_id"].as_str().unwrap();
    post(&app, &format!("/api/operations/{op}/stop"), json!({})).await;

    let ops = until_ops(&app, 1).await;
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    assert_eq!(operations(&app).await.len(), 1, "{ops:?}");
    assert_eq!(ops[0].status_kind, "Cancelled");
    let (_, listed) = call(&app, "GET", &path, None).await;
    assert_eq!(listed[0]["prompt"], "waits");
    assert!(listed[0]["last_error"].is_null());
}
```

Use the stop route exactly as `conversation.test.tsx` and `tests/planner_turn.rs` call it (`POST /api/operations/{id}/stop`; Grep for its body).

- [ ] **Step 2: Run to see them fail**

Run: `cargo test -p shadows --test message_queue completed_turn`
Expected: FAIL — only one operation; "after it" waits.

- [ ] **Step 3: Dequeue inside `start_turn`**

In `turns/store/turn.rs`:

```rust
/// A waiting message the turn sends (§20.3, §20.4): taken in the turn's own
/// transaction, so it is sent once or not at all. `also` is Send now's own
/// command, recorded with the turn so its replay answers the turn.
#[derive(Debug, Clone, Copy)]
pub struct Dequeue<'a> {
    pub id: &'a QueuedMessageId,
    pub also: Option<&'a CommandContext>,
}
```

Add `pub dequeue: Option<Dequeue<'a>>,` to `NewTurn`. In `start_turn` clone it out before the closure (`let dequeue = turn.dequeue.map(|d| (d.id.clone(), d.also.cloned()));`) and, inside the transaction right after the `has_open_operation` check:

```rust
                    if let Some((id, _)) = &dequeue {
                        super::queue::take_queued_in(conn, &thread, id).await?;
                    }
```

and after `record_command(...)` for the turn:

```rust
                    if let Some((id, also)) = &dequeue {
                        let payload = serde_json::json!({
                            "queued_id": id.as_str(), "how": "turn", "operation_id": op.as_str(),
                        });
                        append_event(
                            conn,
                            &DurableEvent::new("QueuedMessageSent", Actor::user(&ctx.principal_id))
                                .with_thread(&thread)
                                .with_payload(payload),
                            &ts,
                        )
                        .await?;
                        if let Some(also) = also {
                            record_command(
                                conn, also, SCOPE, thread.as_str(), "Operation",
                                &outcome.to_string(), &ts,
                            )
                            .await?;
                        }
                    }
```

Every other `NewTurn { .. }` literal (`turns/mod.rs`, `testing/turn.rs`) gets `dequeue: None`.

- [ ] **Step 4: The completed callback**

In `turns/spawn.rs`:

```rust
/// Called once a turn's `Completed` is recorded (§20.3): sends the thread's
/// next waiting message.
pub type OnCompleted = Arc<dyn Fn(ThreadId) + Send + Sync>;
```

`PlannerTurnRequest` gains `pub on_completed: Option<OnCompleted>`, passed into `TurnWatch` (new field `pub on_completed: Option<OnCompleted>`). Add `on_completed: None` to the three other `PlannerTurnRequest { .. }` literals listed under Files.

In `turns/turn.rs`, after `let result = match answer { .. };`, replace the trailing error log with:

```rust
            let completed = matches!(result, Ok(())) && w.completed_answer;
            if let Err(error) = result {
                tracing::error!(%error, "planner.terminal_transition_failed");
            }
            if completed && let Some(next) = &w.on_completed {
                next(w.thread_id.clone());
            }
```

where `completed_answer` is not a field: compute `let ended = matches!(answer, Ok(TurnEnd::Ended));` before `match answer` consumes it and use `ended` in place of `w.completed_answer`. (`mark_operation_completed` returns `Result<(), StorageError>`; if its `Ok` type differs, match `Ok(_)`.)

- [ ] **Step 5: `send_with`, `send_queued`, `send_next`**

In `turns/mod.rs`, rename the body of `send` to

```rust
    pub(crate) async fn send_with(
        &self,
        thread_id: ThreadId,
        turn: SendTurn,
        dequeue: Option<(QueuedMessageId, Option<CommandContext>)>,
    ) -> Result<OperationId, CoreError> {
```

and make `send` call `self.send_with(thread_id, turn, None).await`. Thread `dequeue` through `Turn` (new field `dequeue: Option<Dequeue<'a>>`, built as `dequeue.as_ref().map(|(id, also)| Dequeue { id, also: also.as_ref() })`) into `start`'s `NewTurn { .., dequeue: turn.dequeue }`. In the `PlannerTurnRequest` literal add `on_completed: Some(self.on_completed())`, with:

```rust
    /// §20.3: after a `Completed`, the thread's next waiting message is sent
    /// by a task of its own, run to its end as `send` must be.
    fn on_completed(&self) -> spawn::OnCompleted {
        let turns = self.clone();
        Arc::new(move |thread| {
            let turns = turns.clone();
            tokio::spawn(async move { turns.send_next(&thread).await });
        })
    }
```

In `turns/queue.rs`:

```rust
/// The id of the turn a waiting message starts (§20.3): the watcher's start
/// and Send now's share it, so the second is a replay.
fn queued_start_id(id: &QueuedMessageId) -> String {
    format!("queued:{}", id.as_str())
}

impl Turns {
    /// Starts `row` as a turn, taking it from the queue in the turn's own
    /// transaction (§20.3).
    pub(crate) async fn send_queued(
        &self,
        thread: &ThreadId,
        row: &QueuedMessage,
        also: Option<CommandContext>,
    ) -> Result<OperationId, CoreError> {
        let turn = SendTurn {
            command_id: queued_start_id(&row.id),
            prompt: row.prompt.clone(),
            model: row.model.clone(),
            mode: row.mode.clone(),
            effort: row.effort.clone(),
            focus: row.focus.clone(),
            plan: row.plan.clone(),
            client_tab: None,
        };
        self.send_with(thread.clone(), turn, Some((row.id.clone(), also))).await
    }

    /// §20.3: the first waiting message, unless it carries an error and so
    /// waits for the person. `ThreadBusy` is not a failure: the turn that took
    /// the thread sends it when it completes. A gone row was sent by Send now.
    pub(crate) async fn send_next(&self, thread: &ThreadId) {
        let first = match self.storage.queued_messages(thread).await {
            Ok(list) => list.into_iter().next(),
            Err(error) => {
                tracing::error!(%error, thread_id = %thread, "queue.read_failed");
                return;
            }
        };
        let Some(row) = first.filter(|row| row.last_error.is_none()) else {
            return;
        };
        match self.send_queued(thread, &row, None).await {
            Ok(_) => {}
            Err(CoreError::Storage(StorageError::ThreadBusy | StorageError::QueuedMessageGone)) => {}
            Err(error) => {
                if let Err(e) = self.storage.fail_queued(thread, &row.id, &error.to_string()).await {
                    tracing::error!(error = %e, "queue.fail_record_failed");
                }
            }
        }
    }
}
```

- [ ] **Step 6: Run the tests**

Run: `cargo test -p shadows --test message_queue`
Expected: all pass.

- [ ] **Step 7: Prove the main test bites**

Comment out `next(w.thread_id.clone());` in `turn.rs`, run `cargo test -p shadows --test message_queue completed_turn`, see it FAIL (one operation). Restore it, run again, see it pass. Say in the commit message that this was done.

- [ ] **Step 8: Contract, commit**

`turns/contract.yaml`: `send_next` and `send_queued` (pub(crate), their rules), `NewTurn.dequeue`, the `on_completed` obligation ("only `Completed` calls it, after the transition committed"), and an agreement `between: [send_next, start_turn]`: the row is taken inside the turn's transaction. Add both test names.

```bash
git add crates docs
git commit -m "feat(turns): a completed turn sends the next waiting message (§20.3)

Proven to bite: with the watcher's callback removed, the completed-turn test fails."
```

---

### Task 4: Send now

**Files:**
- Modify: `crates/shadows-agent/src/acp.rs` (`Steer`, `Connection::steer`)
- Create: `crates/fake-acp/src/steer.rs`; Modify: `crates/fake-acp/src/main.rs`
- Modify: `crates/shadows-core/src/turns/handles.rs` (`LiveTurn.steer`, `steer_target`)
- Modify: `crates/shadows-core/src/turns/entries.rs` (`Collector::cut`)
- Modify: `crates/shadows-core/src/turns/spawn.rs`, `turn.rs` (the steer channel)
- Modify: `crates/shadows-core/src/turns/store/queue.rs` (`steered_entry`, `replayed_send_now`)
- Modify: `crates/shadows-core/src/turns/queue.rs` (`send_now`)
- Modify: `crates/shadows-http/src/queue.rs`, `lib.rs`, `tests/openapi.rs`, `api/openapi.json`
- Test: `crates/shadows/tests/message_queue.rs`

**Interfaces:**
- Consumes: Tasks 1-3.
- Produces:
  - `pub enum Steer { Injected, PromptRequired }` and `Connection::steer(&self, session: &str, text: &str) -> Result<Steer, AcpError>` in `shadows-agent`.
  - `pub(crate) struct Steered { pub command: CommandContext, pub row: QueuedMessageId, pub prompt: String, pub reply: oneshot::Sender<Result<ThreadEntryId, StorageError>> }` in `turns/turn.rs`.
  - `LiveTurn.steer: mpsc::UnboundedSender<Steered>`; `LiveHandles::steer_target(&self, thread: &ThreadId) -> Option<SteerTarget>` with `pub(crate) struct SteerTarget { pub op: OperationId, pub session: OpenSession, pub cancel_requested: Arc<AtomicBool>, pub steer: mpsc::UnboundedSender<Steered> }`.
  - `Collector::cut(&mut self) -> Vec<Durable>`: the message being streamed, written now.
  - `Storage::steered_entry(&self, ctx: &CommandContext, thread: &ThreadId, op: &OperationId, row: &QueuedMessageId, prompt: &str) -> Result<ThreadEntryId, StorageError>`
  - `Storage::replayed_send_now(&self, ctx: &CommandContext, thread: &ThreadId) -> Result<Option<SentNow>, StorageError>`
  - `Turns::send_now(&self, thread: &ThreadId, id: &QueuedMessageId, command_id: String) -> Result<SentNow, CoreError>`
  - Route `POST /api/threads/{id}/queue/{qid}/send-now`, body `{ "command_id": string }`, 202 `SentNow`.
  - fake-acp prompt `steerable`: chunk `m1` "waiting", waits for a steer, then chunk `m2` "steered: <text>" and `end_turn`. `_session/steering` answers `injected` while `steerable` waits, `promptRequired` otherwise.

- [ ] **Step 1: Write the failing tests**

Append to `message_queue.rs`:

```rust
#[tokio::test]
async fn send_now_steers_the_running_turn_after_its_streamed_text() {
    let app = test_app().await;
    let path = format!("/api/threads/{}/queue", app.thread.as_str());
    let (_, first) = post(&app, &path, queue_body("steerable")).await;
    let (_, queued) = post(&app, &path, queue_body("turn left")).await;
    let qid = queued["message"]["id"].as_str().unwrap();
    // Let the turn stream its first message before the steer.
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    let now = format!("{path}/{qid}/send-now");
    let (status, answer) = post(&app, &now, json!({ "command_id": fresh_command() })).await;
    assert_eq!(status, 202, "{answer}");
    assert_eq!(answer["status"], "steered");

    let ops = until_ops(&app, 1).await;
    assert_eq!(ops.len(), 1, "a steer starts no turn of its own");
    assert_eq!(ops[0].id.as_str(), first["operation_id"].as_str().unwrap());
    let bodies: Vec<String> = app::entries(&app).await.into_iter().map(|e| e.body).collect();
    assert_eq!(bodies, ["steerable", "waiting", "turn left", "steered: turn left"]);
    let (_, listed) = call(&app, "GET", &path, None).await;
    assert_eq!(listed, json!([]));
}

#[tokio::test]
async fn send_now_after_stop_is_thread_busy_and_keeps_the_row() {
    let app = test_app().await;
    let path = format!("/api/threads/{}/queue", app.thread.as_str());
    let (_, first) = post(&app, &path, queue_body("ignore-cancel")).await;
    let (_, queued) = post(&app, &path, queue_body("later")).await;
    let qid = queued["message"]["id"].as_str().unwrap();
    let op = first["operation_id"].as_str().unwrap();
    // `ignore-cancel` never confirms, so the Stop stays pending a while.
    let app2 = app.clone_handle();
    let stop = tokio::spawn(async move {
        post(&app2, &format!("/api/operations/{op}/stop"), json!({})).await
    });
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let now = format!("{path}/{qid}/send-now");
    let (status, answer) = post(&app, &now, json!({ "command_id": fresh_command() })).await;
    assert_eq!(status, 409, "{answer}");
    assert_eq!(answer["code"], "THREAD_BUSY");
    stop.await.unwrap();
    let (_, listed) = call(&app, "GET", &path, None).await;
    assert_eq!(listed[0]["prompt"], "later");
    assert!(listed[0]["last_error"].is_null());
}

#[tokio::test]
async fn the_watcher_and_send_now_never_send_one_message_twice() {
    let app = test_app().await;
    let path = format!("/api/threads/{}/queue", app.thread.as_str());
    post(&app, &path, queue_body("wait-for-release")).await;
    let (_, queued) = post(&app, &path, queue_body("once")).await;
    let qid = queued["message"]["id"].as_str().unwrap().to_string();
    std::fs::write(app.dir().join("release"), "").unwrap();
    // Send now as the turn completes. `wait-for-release` is not steerable, so
    // the adapter answers promptRequired: Send now starts it (202), finds the
    // turn still holding the thread (409), or finds it already sent (404, or
    // 202 replaying the watcher's start under the shared `queued:<id>`).
    let now = format!("{path}/{qid}/send-now");
    let (status, _) = post(&app, &now, json!({ "command_id": fresh_command() })).await;
    assert!([202, 404, 409].contains(&status), "{status}");
    let ops = until_ops(&app, 2).await;
    assert_eq!(ops.len(), 2);
    let sent = app::entries(&app).await.into_iter().filter(|e| e.body == "once").count();
    assert_eq!(sent, 1);
}
```

`app.clone_handle()`: if `App` cannot be shared with a spawned task, issue the stop through `tokio::join!` on the two requests instead of `tokio::spawn`, sleeping 100 ms at the start of the send-now future. Use whichever the fixture allows; the assertion is unchanged.

- [ ] **Step 2: Run to see them fail**

Run: `cargo test -p shadows --test message_queue send_now`
Expected: FAIL — 404/405, no route; `steerable` unknown to fake-acp.

- [ ] **Step 3: `Connection::steer`**

In `shadows-agent/src/acp.rs`, beside `TurnEnd`:

```rust
/// What `_session/steering` answered (spec §20.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Steer {
    /// The message went into the running prompt; that prompt answers for it.
    Injected,
    /// No prompt was running; nothing was started.
    PromptRequired,
}
```

and in `impl Connection`, after `prompt`:

```rust
    /// Sends `text` into the session's running prompt (§20.4), asking the
    /// adapter to start nothing when none runs.
    pub async fn steer(&self, session: &str, text: &str) -> Result<Steer, AcpError> {
        let params = serde_json::json!({
            "sessionId": session,
            "prompt": [{ "type": "text", "text": text }],
            "_meta": { "steering": { "idleBehavior": "promptRequired" } },
        });
        let request = agent_client_protocol::UntypedMessage::new("_session/steering", params)
            .map_err(rpc)?;
        let answer = self.cx.send_request(request).block_task().await.map_err(rpc)?;
        match answer.get("outcome").and_then(Value::as_str) {
            Some("injected") => Ok(Steer::Injected),
            Some("promptRequired") => Ok(Steer::PromptRequired),
            other => Err(AcpError::Rpc(format!("unexpected steering answer {other:?}"))),
        }
    }
```

- [ ] **Step 4: fake-acp learns to be steered**

Read `crates/fake-acp/src/main.rs` around its session state struct (the type holding `cancel`, Grep `cancel:`) and the handler chain ending near line 400-560. Add to the session state:

```rust
    /// The text of a `_session/steering` while `steerable` waits (`steer.rs`).
    steer: Arc<tokio::sync::watch::Sender<Option<String>>>,
    /// Whether a `steerable` prompt is waiting to be steered.
    steerable: Arc<std::sync::atomic::AtomicBool>,
```

initialised where `cancel` is (`Arc::new(tokio::sync::watch::channel(None).0)`, `Arc::new(AtomicBool::new(false))`).

`crates/fake-acp/src/steer.rs`:

```rust
//! One job: `_session/steering` (spec §20.1) and the `steerable` prompt that
//! waits for it.

use std::sync::atomic::Ordering;

use serde_json::{Value, json};

use crate::Session;

/// The answer to a `_session/steering` for `s`: `injected` while a
/// `steerable` prompt waits (it then streams the text), else `promptRequired`.
pub fn answer(s: &Session, params: &Value) -> Value {
    if !s.steerable.load(Ordering::SeqCst) {
        return json!({ "outcome": "promptRequired", "reason": "noRunningTurn" });
    }
    let text = params
        .pointer("/prompt/0/text")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    s.steerable.store(false, Ordering::SeqCst);
    let _ = s.steer.send(Some(text));
    json!({ "outcome": "injected" })
}

/// The `steerable` prompt: marks the session steerable, waits, and answers
/// with the steered text as a second message.
pub async fn wait(s: &Session) -> String {
    let mut steered = s.steer.subscribe();
    s.steerable.store(true, Ordering::SeqCst);
    loop {
        if let Some(text) = steered.borrow_and_update().clone() {
            return text;
        }
        if steered.changed().await.is_err() {
            return String::new();
        }
    }
}
```

(`Session` is whatever the state struct is called; make it and the two fields `pub(crate)`.) In `main.rs`'s prompt `match`, add before `"hang"`:

```rust
                            "steerable" => {
                                chunk(&cx, &id, "m1", "waiting")?;
                                let text = steer::wait(&s).await;
                                chunk(&cx, &id, "m2", &format!("steered: {text}"))?;
                            }
```

and register, as the **last** handler of the chain (an `UntypedMessage` request matches every method, so anything after it is never reached):

```rust
        .on_receive_request(
            {
                let state = state.clone();
                async move |r: agent_client_protocol::UntypedMessage,
                            responder: Responder<serde_json::Value>,
                            _cx: ConnectionTo<Client>| {
                    if r.method() != "_session/steering" {
                        return responder.respond_with_error(
                            agent_client_protocol::Error::new(-32601, "Method not found"),
                        );
                    }
                    let id = r.params().get("sessionId").and_then(Value::as_str).unwrap_or("");
                    let s = state.lock().unwrap().sessions.get(id).cloned();
                    match s {
                        Some(s) => responder.respond(steer::answer(&s, r.params())),
                        None => responder.respond_with_error(
                            agent_client_protocol::Error::new(-32603, "Session not found"),
                        ),
                    }
                }
            },
            agent_client_protocol::on_receive_request!(),
        )
```

Update the module doc at the top of `main.rs` to list `steerable` and `_session/steering`. If the crate's builder tries handlers by registration order, this being last is enough; if `cargo test -p shadows --test message_queue` shows ordinary prompts answered "Method not found", the untyped handler is shadowing the typed ones: move it after all of them, or use the crate's fallback hook if one exists (Grep `fallback` in `agent-client-protocol-2.2.0/src/jsonrpc.rs`).

- [ ] **Step 5: The steer channel in the watcher**

`entries.rs`, in `impl Collector`:

```rust
    /// The message being streamed, written now (§20.4): a steered message
    /// goes after the text the person already saw, and the reply continues
    /// as a new message.
    pub(crate) fn cut(&mut self) -> Vec<Durable> {
        let mut out = Vec::new();
        self.flush_message(&mut out);
        self.message_id = None;
        out
    }
```

`turn.rs`, beside `TurnWatch`:

```rust
/// A Send now the adapter took (§20.4): the watcher records it in its place
/// among the turn's entries and answers the sender.
pub(crate) struct Steered {
    pub command: CommandContext,
    pub row: QueuedMessageId,
    pub prompt: String,
    pub reply: oneshot::Sender<Result<ThreadEntryId, StorageError>>,
}

async fn record_steer(w: &TurnWatch, collector: &mut Collector, s: Steered) {
    persist(w, collector.cut()).await;
    let recorded = w
        .runtime
        .storage
        .steered_entry(&s.command, &w.thread_id, &w.op_id, &s.row, &s.prompt)
        .await;
    let _ = s.reply.send(recorded);
}
```

`watch_turn` takes a fourth argument `mut steers: mpsc::UnboundedReceiver<Steered>`. Make the `select!` `biased;` with the branches in the order prompt, events, steers, and add:

```rust
                        Some(s) = steers.recv() => record_steer(&w, &mut collector, s).await,
```

After the existing `while let Ok(event) = rx.try_recv()` drain, add `while let Ok(s) = steers.try_recv() { record_steer(&w, &mut collector, s).await; }` before `collector.finish()`.

`handles.rs`: `LiveTurn` gains `pub(crate) steer: mpsc::UnboundedSender<Steered>`, and:

```rust
pub(crate) struct SteerTarget {
    pub(crate) op: OperationId,
    pub(crate) session: OpenSession,
    pub(crate) cancel_requested: Arc<AtomicBool>,
    pub(crate) steer: mpsc::UnboundedSender<Steered>,
}

impl LiveHandles {
    /// The running turn of `thread`, as Send now reaches it (§20.4).
    pub(crate) async fn steer_target(&self, thread: &ThreadId) -> Option<SteerTarget> {
        let r = self.0.lock().await;
        r.turns.iter().find(|(_, t)| &t.thread_id == thread).map(|(op, t)| SteerTarget {
            op: op.clone(),
            session: t.session.clone(),
            cancel_requested: t.cancel_requested.clone(),
            steer: t.steer.clone(),
        })
    }
}
```

`spawn.rs`: `let (steer, steers) = mpsc::unbounded_channel();` before `register`, `steer` into `LiveTurn`, `steers` into `watch_turn(.., events, steers, bus)`.

- [ ] **Step 6: The store's steered entry**

In `turns/store/queue.rs`:

```rust
impl Storage {
    /// §20.4: the steered message as a user entry of the running turn, and
    /// its row taken, in one transaction; a replay answers the entry.
    pub(crate) async fn steered_entry(
        &self,
        ctx: &CommandContext,
        thread: &ThreadId,
        op: &OperationId,
        row: &QueuedMessageId,
        prompt: &str,
    ) -> Result<ThreadEntryId, StorageError> {
        let (ctx, thread, op, row, prompt, ts) =
            (ctx.clone(), thread.clone(), op.clone(), row.clone(), prompt.to_owned(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                if let Some(outcome) = classify(conn, &ctx, SCOPE, thread.as_str()).await? {
                    let v: serde_json::Value = serde_json::from_str(&outcome)?;
                    let id = v["entry_id"].as_str().ok_or(StorageError::NotFound("entry_id"))?;
                    return Ok(ThreadEntryId::from_stored(id.to_owned()));
                }
                take_queued_in(conn, &thread, &row).await?;
                let entry = append_entry_in(
                    conn,
                    &thread,
                    NewThreadEntry {
                        kind: ThreadEntryKind::UserMessage,
                        author: Actor::user(&ctx.principal_id),
                        body: &prompt,
                        refs: &[],
                        operation_id: Some(&op),
                    },
                    &ts,
                )
                .await?;
                let payload = serde_json::json!({
                    "queued_id": row.as_str(), "how": "steered", "entry_id": entry.id.as_str(),
                });
                let actor = Actor::user(&ctx.principal_id);
                event(conn, "QueuedMessageSent", &thread, actor, payload, &ts).await?;
                let outcome = serde_json::json!({ "entry_id": entry.id.as_str() }).to_string();
                record_command(conn, &ctx, SCOPE, thread.as_str(), "ThreadEntry", &outcome, &ts)
                    .await?;
                Ok(entry.id)
            })
        })
        .await
    }

    /// Read-only: what an earlier `turn.send_now` with this command answered.
    pub(crate) async fn replayed_send_now(
        &self,
        ctx: &CommandContext,
        thread: &ThreadId,
    ) -> Result<Option<SentNow>, StorageError> {
        let mut conn = self.reader().acquire().await?;
        let Some(outcome) = classify(&mut conn, ctx, SCOPE, thread.as_str()).await? else {
            return Ok(None);
        };
        let v: serde_json::Value = serde_json::from_str(&outcome)?;
        if let Some(op) = v["operation_id"].as_str() {
            let operation_id = OperationId::from_stored(op.to_owned());
            return Ok(Some(SentNow::Started { operation_id }));
        }
        let id = v["entry_id"].as_str().ok_or(StorageError::NotFound("entry_id"))?;
        Ok(Some(SentNow::Steered { entry_id: ThreadEntryId::from_stored(id.to_owned()) }))
    }
}
```

`start_turn` records the `also` command with the turn's outcome `{operation_id, entry_id}` (Task 3), which is why `operation_id` is read first.

- [ ] **Step 7: `Turns::send_now`**

In `turns/queue.rs`:

```rust
impl Turns {
    /// §20.4: into the running turn when the adapter takes it; as a turn of
    /// its own when none runs; refused while a Stop is pending.
    pub async fn send_now(
        &self,
        thread: &ThreadId,
        id: &QueuedMessageId,
        command_id: String,
    ) -> Result<SentNow, CoreError> {
        let params = serde_json::json!({ "thread_id": thread, "queued_id": id });
        let command = user_command(command_id, "turn.send_now", params);
        if let Some(replay) = self.storage.replayed_send_now(&command, thread).await? {
            return Ok(replay);
        }
        let row = self
            .storage
            .queued_message(thread, id)
            .await?
            .ok_or(StorageError::QueuedMessageGone)?;
        if let Some(target) = self.handles.steer_target(thread).await {
            if target.cancel_requested.load(Ordering::SeqCst) {
                return Err(StorageError::ThreadBusy.into());
            }
            let connection = target.session.connection();
            match connection.steer(&target.session.session_id, &row.prompt).await {
                Ok(Steer::Injected) => return self.record_steer(target, command, row).await,
                Ok(Steer::PromptRequired) => {}
                Err(error) => {
                    let reason = format!("the message did not reach the turn: {error}");
                    self.storage.fail_queued(thread, id, &reason).await?;
                    return Err(CoreError::HarnessStartFailed(reason));
                }
            }
        }
        let operation_id = self.send_queued(thread, &row, Some(command)).await?;
        Ok(SentNow::Started { operation_id })
    }

    /// Hands the steered message to the turn's watcher, which writes it after
    /// the text already streamed; a watcher already gone has written all its
    /// entries, so the message is written directly.
    async fn record_steer(
        &self,
        target: SteerTarget,
        command: CommandContext,
        row: QueuedMessage,
    ) -> Result<SentNow, CoreError> {
        let (reply, answer) = tokio::sync::oneshot::channel();
        let steered = Steered {
            command: command.clone(),
            row: row.id.clone(),
            prompt: row.prompt.clone(),
            reply,
        };
        let entry_id = match target.steer.send(steered) {
            Ok(()) => match answer.await {
                Ok(recorded) => recorded?,
                Err(_) => self.write_steered(&command, &target.op, &row).await?,
            },
            Err(_) => self.write_steered(&command, &target.op, &row).await?,
        };
        Ok(SentNow::Steered { entry_id })
    }

    async fn write_steered(
        &self,
        command: &CommandContext,
        op: &OperationId,
        row: &QueuedMessage,
    ) -> Result<ThreadEntryId, StorageError> {
        self.storage
            .steered_entry(command, &row.thread_id, op, &row.id, &row.prompt)
            .await
    }
}
```

Imports: `std::sync::atomic::Ordering`, `shadows_agent::acp::Steer`, `super::handles::SteerTarget`, `super::turn::Steered`, `CommandContext`, `OperationId`, `ThreadEntryId`, `SentNow`.

- [ ] **Step 8: The route**

In `crates/shadows-http/src/queue.rs`:

```rust
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub(super) struct SendNowBody {
    /// The idempotency key (spec §3.2).
    command_id: String,
}

/// Send now: into the running turn, or as a turn when none runs.
#[utoipa::path(
    post, path = "/api/threads/{id}/queue/{qid}/send-now", tag = "turns",
    params(
        ("id" = ThreadId, Path, description = "The thread"),
        ("qid" = QueuedMessageId, Path, description = "The waiting message"),
    ),
    request_body = SendNowBody,
    responses(
        (status = 202, body = SentNow),
        (status = 404, description = "QUEUED_MESSAGE_GONE", body = crate::ErrorBody),
        (status = 409, description = "THREAD_BUSY: a Stop is pending; COMMAND_CONFLICT",
         body = crate::ErrorBody),
        (status = 422, description = "as POST /api/threads/{id}/turns", body = crate::ErrorBody),
        (status = 500, description = "STORAGE_UNAVAILABLE", body = crate::ErrorBody),
        (status = 502, description = "HARNESS_START_FAILED: the steer failed; the message \
                                      keeps the reason", body = crate::ErrorBody),
        (status = 503, description = "RUNTIME_STOPPING", body = crate::ErrorBody),
    )
)]
pub(super) async fn send_now(
    State(s): State<AppState>,
    Path((thread, qid)): Path<(ThreadId, QueuedMessageId)>,
    Json(body): Json<SendNowBody>,
) -> Result<(StatusCode, Json<SentNow>), Failure> {
    let core = s.core.clone();
    let answer = detached(async move {
        core.turns().send_now(&thread, &qid, body.command_id).await.map_err(Failure::from)
    })
    .await?;
    Ok((StatusCode::ACCEPTED, Json(answer)))
}
```

Register it (`.route("/api/threads/{id}/queue/{qid}/send-now", post(queue::send_now))`), add it to the openapi paths and to `tests/openapi.rs`'s list, then `UPDATE_OPENAPI=1 cargo test -p shadows --test openapi`.

- [ ] **Step 9: Run the tests**

Run: `cargo test -p shadows --test message_queue` and `cargo test -p shadows-core --lib entries`.
Expected: all pass. Then `cargo test -p shadows --test planner_turn` (the watcher changed) and see it pass.

- [ ] **Step 10: Contract, codemap, commit**

- `turns/contract.yaml`: `send_now` (rules: replay first; Stop pending is `ThreadBusy`; steer before write; `promptRequired` starts a turn; a failed steer sets `last_error`), `steered_entry`, `replayed_send_now`, `LiveTurn.steer`, `Collector::cut`, the agreement `between: [watch_turn, send_now]` (the watcher writes the steered entry after the text it holds), tests.
- `crates/shadows-agent`: if it has a contract or codemap row naming `Connection`'s calls, add `steer`.
- `docs/codebase/README.md`: a row for `crates/fake-acp/src/steer.rs` ("`_session/steering` and the prompt that waits for it").

```bash
git add crates api docs
git commit -m "feat(turns): Send now steers the running turn (§20.4)

turn.rs grows to record a steered entry in its place among the turn's
entries, which is part of its one job, the recorded ending of a live turn."
```

---

### Task 5: The web client

**Files:**
- Modify: `web/src/api/schema.d.ts` (regenerated: `cd web && npm run gen:api`)
- Modify: `web/src/api/client.ts`, `web/src/api/queries.ts`
- Create: `web/src/app/conversation/waiting-messages.tsx`
- Modify: `web/src/app/conversation/composer.tsx`, `conversation.tsx`, `use-conversation.ts`
- Test: `web/src/app/conversation/waiting-messages.test.tsx`

**Interfaces:**
- Consumes: the four routes and the `Queued`, `QueuedMessage`, `SentNow` schemas.
- Produces:
  - `queueMessage(threadId: string, commandId: string, prompt: string, settings: TurnSettings, extra: { focus?: Focus | null; plan?: string | null; clientTab: string }): Promise<Queued>`
  - `listQueued(threadId: string): Promise<QueuedMessage[]>`
  - `unqueueMessage(threadId: string, id: string, commandId: string): Promise<void>`
  - `sendQueuedNow(threadId: string, id: string, commandId: string): Promise<SentNow>`
  - `queuedQuery(threadId: string)` in `queries.ts`
  - `<WaitingMessages threadId={string} running={boolean} />`

- [ ] **Step 1: Write the failing test**

`web/src/app/conversation/waiting-messages.test.tsx`:

```tsx
// @vitest-environment happy-dom
//
// Writing while a turn runs (§20): Enter queues, and a waiting message offers
// Send now and Remove.

import { act } from 'react'
import { afterEach, expect, it } from 'vitest'
import { type TestApp, startApp, typeInto, until } from '../test-app'

const operation = {
  id: 'op1',
  kind: 'PlannerTurn',
  status_kind: 'Running',
  thread_id: 't1',
  runtime_instance_id: 'r',
  created_at: '2026-10-05T00:00:00Z',
  invocation: null,
}

const waiting = {
  id: 'q1',
  thread_id: 't1',
  position: 1,
  prompt: 'And the tests too',
  model: 'fake-small',
  mode: 'acceptEdits',
  effort: 'high',
  focus: null,
  plan: null,
  last_error: null,
  created_at: 'x',
}

const DAEMON = {
  'GET /api/projects': [
    { id: 'p1', slug: 'demo', name: 'Demo', directory: 'C:\\work\\demo', created_at: 'x' },
  ],
  'GET /api/projects/p1/threads': [
    { id: 't1', project_id: 'p1', title: 'Conversation 1', status: 'Open', created_at: 'x' },
  ],
  'GET /api/threads/t1/entries': [],
  'GET /api/threads/t1/operations': [operation],
  'GET /api/threads/t1/queue': [waiting],
  'POST /api/threads/t1/queue': { status: 'waiting', message: { ...waiting, id: 'q2' } },
  'POST /api/threads/t1/queue/q1/send-now': { status: 'steered', entry_id: 'e9' },
  'DELETE /api/threads/t1/queue/q1': new Response(null, { status: 204 }),
}

let app: TestApp | null = null
afterEach(() => {
  app?.unmount()
  app = null
})

it('queues on Enter while a turn runs, and offers Send now and Remove', async () => {
  const a = (app = await startApp('/projects/p1/threads/t1', DAEMON))
  await until(() => a.button('Stop') !== undefined)
  await until(() => a.text().includes('And the tests too'))
  expect(a.text()).toContain('Waiting')

  const box = a.container.querySelector('textarea')
  if (box === null) throw new Error('the composer is not enabled while a turn runs')
  await act(async () => {
    typeInto(box, 'One more thing')
    box.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }))
  })
  await until(() => a.calls.includes('POST /api/threads/t1/queue'))

  await act(async () => a.button('Send now')?.click())
  await until(() => a.calls.includes('POST /api/threads/t1/queue/q1/send-now'))

  await act(async () => a.button('Remove')?.click())
  await until(() => a.calls.some((c) => c.startsWith('DELETE /api/threads/t1/queue/q1')))
})
```

If the fake daemon (`web/src/test/fake-daemon.ts` / `test-app.tsx`) needs the session routes the composer asks for (`POST /api/threads/t1/session`), copy them from `answers()` in `@/test/fake-daemon` exactly as `conversation.test.tsx` does with `answers({...})`, and keep the keys above. If `calls` records the path without the query string, the `startsWith` check still holds.

- [ ] **Step 2: Run to see it fail**

Run: `cd web && npx vitest run src/app/conversation/waiting-messages.test.tsx`
Expected: FAIL — no waiting message drawn; no `POST /api/threads/t1/queue`.

- [ ] **Step 3: The client calls and the query**

`cd web && npm run gen:api`. In `client.ts`, beside `startTurn`, following its `unwrap(client.POST(...))` form:

```ts
export type Queued = Schemas['Queued']
export type QueuedMessage = Schemas['QueuedMessage']
export type SentNow = Schemas['SentNow']

export async function queueMessage(
  threadId: string,
  commandId: string,
  prompt: string,
  settings: TurnSettings,
  { focus = null, plan, clientTab }: { focus?: Focus | null; plan?: string | null; clientTab: string },
): Promise<Queued> {
  return unwrap(
    client.POST('/api/threads/{id}/queue', {
      params: { path: { id: threadId } },
      body: {
        command_id: commandId,
        prompt,
        model: settings.model,
        mode: settings.mode,
        effort: settings.effort,
        focus,
        plan: plan ?? null,
        client_tab: clientTab,
      },
    }),
  )
}

export function listQueued(threadId: string): Promise<QueuedMessage[]> {
  return unwrap(client.GET('/api/threads/{id}/queue', { params: { path: { id: threadId } } }))
}

export async function unqueueMessage(threadId: string, id: string, commandId: string): Promise<void> {
  await unwrap(
    client.DELETE('/api/threads/{id}/queue/{qid}', {
      params: { path: { id: threadId, qid: id }, query: { command_id: commandId } },
    }),
  )
}

export function sendQueuedNow(threadId: string, id: string, commandId: string): Promise<SentNow> {
  return unwrap(
    client.POST('/api/threads/{id}/queue/{qid}/send-now', {
      params: { path: { id: threadId, qid: id } },
      body: { command_id: commandId },
    }),
  )
}
```

Match `startTurn`'s body field names exactly (read its body at `client.ts:167`); if `unwrap` does not accept a 204, follow how another `DELETE` in `client.ts` is unwrapped. In `queries.ts`, beside `entriesQuery`:

```ts
export function queuedQuery(threadId: string) {
  return queryOptions({ queryKey: ['threads', threadId, 'queue'], queryFn: () => listQueued(threadId) })
}
```

using the key prefix `entriesQuery` uses for the same thread.

- [ ] **Step 4: The waiting list**

`web/src/app/conversation/waiting-messages.tsx`:

```tsx
// One job: a conversation's waiting messages (§20), each with Send now (Send
// when no turn runs) and Remove, and the reason a send failed.

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { sendQueuedNow, unqueueMessage } from '@/api/client'
import { queuedQuery } from '@/api/queries'
import { Button } from '@/components/ui/button'

export function WaitingMessages({ threadId, running }: { threadId: string; running: boolean }) {
  const queue = useQuery(queuedQuery(threadId))
  const queryClient = useQueryClient()
  const refetch = () => queryClient.invalidateQueries({ queryKey: queuedQuery(threadId).queryKey })
  const sendNow = useMutation({
    mutationFn: (id: string) => sendQueuedNow(threadId, id, crypto.randomUUID()),
    onSettled: refetch,
  })
  const remove = useMutation({
    mutationFn: (id: string) => unqueueMessage(threadId, id, crypto.randomUUID()),
    onSettled: refetch,
  })
  const waiting = queue.data ?? []
  if (waiting.length === 0) return null
  return (
    <ul className="mx-auto max-w-3xl space-y-2 px-6 pb-2" aria-label="Waiting messages">
      {waiting.map((m) => (
        <li key={m.id} className="ml-auto max-w-[80%] rounded-xl border border-border/60 bg-muted/40 px-3 py-2 opacity-70">
          <p dir="auto" className="whitespace-pre-wrap text-sm text-foreground">{m.prompt}</p>
          <div className="mt-1 flex items-center gap-2 text-xs text-muted-foreground">
            <span>Waiting</span>
            {m.last_error !== null && <span className="text-destructive-foreground">{m.last_error}</span>}
            <Button size="sm" variant="ghost" onClick={() => sendNow.mutate(m.id)} disabled={sendNow.isPending}>
              {running ? 'Send now' : 'Send'}
            </Button>
            <Button size="sm" variant="ghost" onClick={() => remove.mutate(m.id)} disabled={remove.isPending}>
              Remove
            </Button>
          </div>
        </li>
      ))}
    </ul>
  )
}
```

Use the project's `Button` import path and token class names as `composer.tsx` uses them. If `crypto.randomUUID` is not how the client mints command ids, use the helper `composer.tsx`'s `attemptFor` uses.

- [ ] **Step 5: Enter queues while a turn runs; the list is drawn; events refetch it**

In `composer.tsx`:
- Add a mutation beside `send`:

```tsx
  const queue = useMutation({
    mutationFn: ({ commandId, text, settings, pointed, planId }: SendArgs) =>
      queueMessage(threadId as string, commandId, text, settings, {
        focus: focusOf(pointed),
        plan: planId ?? null,
        clientTab: tabId(),
      }),
    onSuccess: () => {
      pending.current = null
      setPrompt('')
      void queryClient.invalidateQueries({ queryKey: queuedQuery(threadId as string).queryKey })
    },
  })
```

(`SendArgs` is the type `send.mutate` takes today; name it if it is inline.)
- In `submit`, replace `|| running !== null) return` so a running turn queues instead:

```tsx
    if (text === '' || !ready || settings === null || send.isPending || queue.isPending) return
    ...
    const args = { commandId: pending.current.commandId, text, settings, pointed, planId }
    if (running !== null && threadId !== null) queue.mutate(args)
    else send.mutate(args)
```

- Include `queue.error` in `const error = ...`.

In `conversation.tsx`, render `<WaitingMessages threadId={threadId} running={running !== null} />` between the messages and the composer (read the file for the variable names it holds).

In `use-conversation.ts`, inside the stream callback:

```ts
      if (live && QUEUE_EVENTS.has(event.kind)) {
        void queryClient.invalidateQueries({ queryKey: queuedQuery(threadId).queryKey })
      }
```

with `const QUEUE_EVENTS = new Set(['MessageQueued', 'QueuedMessageRemoved', 'QueuedMessageSent', 'QueuedMessageFailed'])` at module level.

- [ ] **Step 6: Run the web tests**

Run: `cd web && npx vitest run src/app/conversation` then `npm run typecheck` and `npm run lint`.
Expected: all pass, including the existing `conversation.test.tsx` ("shows Running and a Stop …" — Send is still absent while running; Stop stays).

- [ ] **Step 7: Commit**

```bash
git add web
git commit -m "feat(web): write while a turn runs — Enter queues; Send now and Remove (§20.5)"
```

---

### Task 6: The gate and the run in the browser

**Files:**
- Modify: `docs/status.md`
- Modify: `docs/superpowers/specs/README.md` (§20's row: "built")

- [ ] **Step 1: The full gate, once**

From the root, with the Global Constraints' env:

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --features fake-acp/test-support -- -D warnings
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo tree -e features,no-dev --workspace | grep test-support
cd web && npm run typecheck && npm run lint && npm test
```

Expected: every command passes; the `grep` prints nothing. Then the 100-column check and `git status` shows `api/openapi.json` committed.

- [ ] **Step 2: The journey on a copy of the dev database**

Stop any running `shadows.exe`. Copy `%LOCALAPPDATA%\shadows-dev\shadows.sqlite3` to a scratch folder and point a `daemon` run at the copy (as `.claude/launch.json`'s `daemon` entry does, with the copy's path). The daemon migrates the copy to 0019. In the browser, against the real adapter on Sonnet, effort medium:
1. Ask something long ("count to 40 with a sentence each"). While it runs, write two messages and press Enter twice: both show as Waiting.
2. Press Send now on the first: the counting stops, the reply answers it, and in the thread the text streamed before it sits above your message.
3. Press Stop during the turn that follows, if one starts; the second message stays Waiting with **Send**. Press Send: it runs.
4. Open a second project's conversation in another tab and run a turn in both at once: both complete.
5. Read the daemon log for `queue.` and `planner.` errors: none.

Record what you saw (one line per step) in `docs/status.md` under a new dated paragraph for §20, naming the commits.

- [ ] **Step 3: Commit**

```bash
git add docs
git commit -m "docs(status): §20, the queue and Send now, ran in the browser"
```
