## Task 10: Cancellation — request, confirmed termination, terminal Cancelled

**Files:**
- Modify: `src/storage/sqlite/operation.rs`
- Create: `src/planner/mod.rs`
- Modify: `src/lib.rs`
- Test: `tests/operation_lifecycle.rs`, `tests/planner_turn.rs`

**Interfaces:**
- Consumes: everything from Tasks 3–9.
- Produces:
  - `Storage::request_cancellation(&self, op_id: &str, by_kind: &str, by_id: &str) -> Result<(), StorageError>`
  - `Storage::mark_operation_cancelled(&self, op_id: &str) -> Result<(), StorageError>`
  - `planner::LiveHandles` — `Default`, with a `pub(crate)` inner `Mutex<HashMap<String, ProcessHandle>>` so `cli::serve` can enumerate this runtime's live operations at shutdown.
  - `planner::PlannerTurn::start(runtime: Arc<Runtime>, handles: Arc<LiveHandles>, harness: Arc<ClaudeHarness>, thread_id: String, prompt: String, cwd: PathBuf, resume_session_id: Option<String>, bus: broadcast::Sender<(String, StreamItem)>) -> Result<String, StorageError>`
  - `planner::PlannerTurn::stop(runtime: Arc<Runtime>, handles: Arc<LiveHandles>, op_id: &str) -> Result<(), StorageError>`

**There is no `mark_operation_interrupted` capability.** Interruption is not a transition anyone requests — it is what startup recovery concludes about work a previous runtime left behind, so it belongs to `reconcile_orphans` in Task 4 and to nothing else. A public method for it would let live code mark its own work interrupted, which is a claim only a later runtime is entitled to make.

**The rule this task exists to enforce.** Spec §2.3: `Cancelled` means Shadows confirmed the managed execution is no longer running and persisted the terminal transition. It never means only that a user requested cancellation.

- [ ] **Step 1: Write the failing tests**

Append to `tests/operation_lifecycle.rs`:

```rust
/// Spec §2.3. The request and the terminal state are two separate facts in two
/// separate transactions. A request alone leaves the operation non-terminal.
#[tokio::test]
async fn a_cancellation_request_does_not_make_an_operation_terminal() {
    let (_t, storage, runtime, thread) = fixture().await;
    let op = storage.create_pending_operation(&thread, &runtime).await.unwrap();
    storage.mark_operation_started(&op, &runtime).await.unwrap();

    storage.request_cancellation(&op, "User", "local").await.unwrap();

    let loaded = storage.get_operation(&op).await.unwrap();
    assert_eq!(loaded.status_kind, "Running", "a request is not a terminal state");
    assert!(loaded.cancel_requested_at.is_some());
    assert!(loaded.finished_at.is_none());
}

/// Spec §2.3. Terminal Cancelled is written only after termination is
/// confirmed, and it is what closes the operation.
#[tokio::test]
async fn cancelled_is_written_after_confirmation_and_closes_the_operation() {
    let (_t, storage, runtime, thread) = fixture().await;
    let op = storage.create_pending_operation(&thread, &runtime).await.unwrap();
    storage.mark_operation_started(&op, &runtime).await.unwrap();
    storage.request_cancellation(&op, "User", "local").await.unwrap();

    storage.mark_operation_cancelled(&op).await.unwrap();

    let loaded = storage.get_operation(&op).await.unwrap();
    assert_eq!(loaded.status_kind, "Cancelled");
    assert!(loaded.finished_at.is_some());
    assert!(loaded.cancel_requested_at.is_some(), "the request is retained as history");

    let kinds: Vec<String> = sqlx::query_scalar(
        "SELECT kind FROM durable_event WHERE operation_id = ? ORDER BY seq",
    )
    .bind(&op)
    .fetch_all(storage.reader()).await.unwrap();
    assert_eq!(
        kinds,
        vec!["OperationCreated", "OperationStarted",
             "OperationCancellationRequested", "OperationCancelled"]
    );
}

/// Spec §8.4 case 4. A process that exited on its own before cancellation took
/// termination ownership is Completed by its own exit, not Cancelled. Shadows
/// does not claim to have stopped something that had already stopped.
#[tokio::test]
async fn a_natural_exit_wins_over_an_in_flight_cancellation() {
    let (_t, storage, runtime, thread) = fixture().await;
    let op = storage.create_pending_operation(&thread, &runtime).await.unwrap();
    storage.mark_operation_started(&op, &runtime).await.unwrap();
    storage.request_cancellation(&op, "User", "local").await.unwrap();

    // The process exits before containment takes ownership.
    storage.mark_operation_completed(&op, serde_json::json!({ "ok": true })).await.unwrap();

    let loaded = storage.get_operation(&op).await.unwrap();
    assert_eq!(loaded.status_kind, "Completed");
    assert!(loaded.cancel_requested_at.is_some(),
            "a terminal non-cancelled status may retain request metadata as history");

    // Spec §8.4 case 5 in reverse: the late Cancelled must not overwrite it.
    let late = storage.mark_operation_cancelled(&op).await;
    assert!(matches!(late, Err(StorageError::TransitionConflict { .. })));
}

/// Spec §8.4 case 6. If termination cannot be confirmed, the request is
/// preserved and Cancelled is NOT written. The operation stays non-terminal
/// until recovery can make an honest Interrupted transition.
#[tokio::test]
async fn unconfirmed_termination_leaves_the_operation_non_terminal() {
    let (_t, storage, runtime, thread) = fixture().await;
    let op = storage.create_pending_operation(&thread, &runtime).await.unwrap();
    storage.mark_operation_started(&op, &runtime).await.unwrap();
    storage.request_cancellation(&op, "User", "local").await.unwrap();

    // Termination failed: the daemon simply does not call mark_operation_cancelled.
    let loaded = storage.get_operation(&op).await.unwrap();
    assert_eq!(loaded.status_kind, "Running");

    // A later runtime resolves it honestly, and to Interrupted, not Cancelled,
    // because the final process outcome is unknown after a crash.
    let next = storage.register_runtime_instance("next").await.unwrap();
    let report = storage.reconcile_orphans(&next).await.unwrap();
    assert_eq!(report.interrupted, vec![op.clone()]);
    let after = storage.get_operation(&op).await.unwrap();
    assert_eq!(after.status_kind, "Interrupted");
    assert!(after.cancel_requested_at.is_some(),
            "the request stays visible on the interrupted record");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test operation_lifecycle`
Expected: FAIL — `request_cancellation` and `mark_operation_cancelled` do not exist.

- [ ] **Step 3: Extend `src/storage/sqlite/operation.rs`**

```rust
impl Storage {
    /// TX #1 of cancellation. Spec §2.3: this records that a stop was asked
    /// for. It does not stop anything and it does not make the operation
    /// terminal.
    pub async fn request_cancellation(
        &self,
        op_id: &str,
        by_kind: &str,
        by_id: &str,
    ) -> Result<(), StorageError> {
        let (op_id, by_kind, by_id, ts) =
            (op_id.to_string(), by_kind.to_string(), by_id.to_string(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                let affected = sqlx::query(
                    "UPDATE operation
                        SET cancel_requested_at = ?, cancel_requested_by_kind = ?,
                            cancel_requested_by_id = ?
                      WHERE id = ? AND status_kind IN ('Pending','Running')
                        AND cancel_requested_at IS NULL",
                )
                .bind(&ts).bind(&by_kind).bind(&by_id).bind(&op_id)
                .execute(&mut *conn).await?
                .rows_affected();
                if affected == 0 {
                    // Already requested, or already terminal. Spec §2.3: a
                    // repeat records its own idempotent AlreadyTerminal result
                    // and runs no process effects. Treated as success here.
                    return Ok(());
                }
                append_event(
                    conn,
                    &DurableEvent::new("OperationCancellationRequested",
                                       Actor { kind: by_kind.clone(), id: by_id.clone() })
                        .with_operation(&op_id),
                    &ts,
                ).await?;
                Ok(())
            })
        })
        .await
    }

    /// TX #2 of cancellation, written ONLY after the process tree is confirmed
    /// gone. Spec §2.3. The CAS refuses to overwrite a terminal state, which is
    /// what makes a natural exit win the race (§8.4 case 4 and 5).
    pub async fn mark_operation_cancelled(&self, op_id: &str) -> Result<(), StorageError> {
        let (op_id, ts) = (op_id.to_string(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                let affected = sqlx::query(
                    "UPDATE operation SET status_kind = 'Cancelled', finished_at = ?
                      WHERE id = ? AND status_kind IN ('Pending','Running')",
                )
                .bind(&ts).bind(&op_id)
                .execute(&mut *conn).await?
                .rows_affected();
                if affected == 0 {
                    return Err(StorageError::TransitionConflict {
                        expected: "Pending or Running".into(),
                        found: "already terminal".into(),
                    });
                }
                append_event(
                    conn,
                    &DurableEvent::new("OperationCancelled", Actor::system())
                        .with_operation(&op_id),
                    &ts,
                ).await?;
                Ok(())
            })
        })
        .await
    }
}
```

- [ ] **Step 4: Write `src/planner/mod.rs`**

This is where the two-phase spawn and the interlock actually run.

```rust
use std::sync::Arc;

use tokio::io::AsyncBufReadExt;
use tokio::sync::Mutex;

use crate::agent::{claude::ClaudeHarness, AgentHarness, AgentInvocation, StreamItem};
use crate::operation::FailureStage;
use crate::process::{spawn, ProcessHandle};
use crate::runtime::Runtime;
use crate::storage::StorageError;

/// Live handles for operations this runtime owns. Spec §8.3: termination goes
/// through the containment handle that owns the tree, never through a PID read
/// from the database, because the OS reuses PIDs.
#[derive(Default)]
pub struct LiveHandles(Mutex<std::collections::HashMap<String, ProcessHandle>>);

pub struct PlannerTurn;

impl PlannerTurn {
    /// Returns the operation id as soon as Pending is committed. The turn keeps
    /// running; the caller subscribes to the stream separately.
    pub async fn start(
        runtime: Arc<Runtime>,
        handles: Arc<LiveHandles>,
        harness: Arc<ClaudeHarness>,
        thread_id: String,
        prompt: String,
        cwd: std::path::PathBuf,
        resume_session_id: Option<String>,
        bus: tokio::sync::broadcast::Sender<(String, StreamItem)>,
    ) -> Result<String, StorageError> {
        // TX #1: the durable attempt exists before anything spawns.
        let op_id = runtime
            .storage
            .create_pending_operation(&thread_id, &runtime.instance_id)
            .await?;

        let invocation = AgentInvocation {
            operation_id: op_id.clone(),
            role: "Planner".into(),
            model: "sonnet".into(),
            prompt,
            cwd,
            resume_session_id,
            session_id: uuid::Uuid::new_v4().to_string(),
        };

        // Prepare: resolve, build the environment, ready the workspace. A
        // failure here is not a spawn failure — no process ever existed.
        let spec = harness.to_process_spec(&invocation);
        if !spec.executable.exists() && spec.executable.components().count() > 1 {
            runtime
                .storage
                .mark_operation_failed(
                    &op_id,
                    FailureStage::Prepare,
                    &format!("harness executable not found: {}", spec.executable.display()),
                )
                .await?;
            return Ok(op_id);
        }

        let mut handle = match spawn(spec) {
            Ok(h) => h,
            Err(e) => {
                runtime
                    .storage
                    .mark_operation_failed(&op_id, FailureStage::Spawn, &e.to_string())
                    .await?;
                return Ok(op_id);
            }
        };

        // Spec §8.3: register the handle, THEN commit Running. A committed
        // transition without a registered handle is a state we must not produce.
        let lines = handle.stdout_lines().take();
        {
            let mut map = handles.0.lock().await;
            map.insert(op_id.clone(), handle);
        }

        // TX #2.
        runtime
            .storage
            .mark_operation_started(&op_id, &runtime.instance_id)
            .await?;

        let reader_op = op_id.clone();
        let reader_runtime = runtime.clone();
        let reader_handles = handles.clone();
        let reader_harness = harness.clone();
        tokio::spawn(async move {
            let mut outcome = serde_json::json!({ "stop_reason": null });
            if let Some(mut lines) = lines {
                while let Ok(Some(line)) = lines.next_line().await {
                    let item = reader_harness.classify(&line);
                    match &item {
                        // Durable: write before forwarding. The UI may drop a
                        // frame; the record may not.
                        StreamItem::Entry { uuid, role, text } => {
                            let _ = reader_runtime
                                .storage
                                .append_thread_entry(
                                    &thread_id, "AgentMessage", role, uuid, text, &[],
                                )
                                .await;
                        }
                        StreamItem::TurnEnd { subtype, stop_reason } => {
                            outcome = serde_json::json!({
                                "subtype": subtype, "stop_reason": stop_reason
                            });
                        }
                        _ => {}
                    }
                    let _ = bus.send((reader_op.clone(), item));
                }
            }

            // The process ended. Take the handle back and let its containment
            // drop, then record the terminal state. If a cancellation already
            // won, this CAS is refused and that is correct (§8.4 case 5).
            let _ = reader_handles.0.lock().await.remove(&reader_op);
            let _ = reader_runtime
                .storage
                .mark_operation_completed(&reader_op, outcome)
                .await;
        });

        Ok(op_id)
    }

    /// Spec §2.3: request, terminate, confirm, then write Cancelled. If
    /// termination cannot be confirmed, Cancelled is not written and the
    /// operation is left for recovery (§8.4 case 6).
    pub async fn stop(
        runtime: Arc<Runtime>,
        handles: Arc<LiveHandles>,
        op_id: &str,
    ) -> Result<(), StorageError> {
        runtime.storage.request_cancellation(op_id, "User", "local").await?;

        let mut map = handles.0.lock().await;
        let Some(mut handle) = map.remove(op_id) else {
            // No live handle: either it already exited, or this runtime does
            // not own it. Either way we cannot confirm termination, so we do
            // not write Cancelled.
            return Ok(());
        };
        drop(map);

        if handle.terminate_tree().is_err() {
            return Ok(()); // unconfirmed; left non-terminal on purpose
        }
        // `wait` returning is the confirmation that the tree is reaped.
        let _ = handle.wait().await;
        runtime.storage.mark_operation_cancelled(op_id).await
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test operation_lifecycle`
Expected: PASS, all nine tests.

- [ ] **Step 6: Commit**

```bash
git add src/planner src/storage src/lib.rs tests/operation_lifecycle.rs
git commit -m "feat(planner): cancellation requires confirmed termination before Cancelled"
```

---

