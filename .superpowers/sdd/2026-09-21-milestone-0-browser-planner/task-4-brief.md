## Task 4: Runtime instance lifecycle and startup orphan reconciliation

**Files:**
- Create: `src/runtime/mod.rs`, `src/runtime/recovery.rs`, `src/storage/sqlite/runtime.rs`
- Modify: `src/lib.rs`, `src/storage/mod.rs`
- Test: `tests/recovery.rs`

**Interfaces:**
- Consumes: `Storage::write_txn`, `DurableEvent` from Task 3.
- Produces: `Storage::register_runtime_instance(version: &str) -> Result<RuntimeInstanceId, StorageError>`; `Storage::stop_runtime_instance(id, StopKind) -> Result<(), StorageError>`; `Storage::reconcile_orphans(current: &RuntimeInstanceId) -> Result<ReconcileReport, StorageError>`; `ReconcileReport { interrupted: Vec<String>, anomalies: Vec<String> }`; `StopKind::{Graceful, Escalated}`.

- [ ] **Step 1: Write the failing tests**

`tests/recovery.rs`:

```rust
use shadows::storage::{Storage, StopKind};

async fn seed_operation(
    storage: &Storage,
    op_id: &str,
    runtime_id: &str,
    status: &str,
) {
    let started = if status == "Pending" { None } else { Some("2026-09-21T00:00:00Z") };
    storage
        .write_txn(|conn| {
            let (op_id, runtime_id, status) = (op_id.to_string(), runtime_id.to_string(), status.to_string());
            Box::pin(async move {
                sqlx::query(
                    "INSERT INTO operation (id, kind, status_kind, runtime_instance_id, created_at, started_at)
                     VALUES (?, 'PlannerTurn', ?, ?, '2026-09-21T00:00:00Z', ?)",
                )
                .bind(&op_id).bind(&status).bind(&runtime_id).bind(started)
                .execute(&mut *conn).await?;
                Ok(())
            })
        })
        .await
        .unwrap();
}

/// Spec §8.6. A runtime that was lost leaves its operations non-terminal; the
/// next runtime resolves them to Interrupted by exact CAS, and never claims
/// they succeeded, failed, or were cancelled.
#[tokio::test]
async fn a_lost_runtimes_operations_become_interrupted() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();

    let old = storage.register_runtime_instance("0.1.0-old").await.unwrap();
    seed_operation(&storage, "op-pending", &old, "Pending").await;
    seed_operation(&storage, "op-running", &old, "Running").await;
    // `old` is never stopped: it was lost.

    let new = storage.register_runtime_instance("0.1.0-new").await.unwrap();
    let report = storage.reconcile_orphans(&new).await.unwrap();

    assert_eq!(report.interrupted.len(), 2);
    assert!(report.anomalies.is_empty());

    let rows: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, status_kind, interrupt_reason FROM operation ORDER BY id",
    )
    .fetch_all(storage.reader()).await.unwrap();
    assert_eq!(rows[0].1, "Interrupted");
    assert_eq!(rows[0].2.as_deref(), Some("PreviousRuntimeEndedBeforeStart"));
    assert_eq!(rows[1].1, "Interrupted");
    assert_eq!(rows[1].2.as_deref(), Some("PreviousRuntimeEndedDuringRun"));
}

/// Spec §8.5 and §8.6. An Escalated shutdown records `stopped_at` on purpose
/// and leaves work non-terminal. Recovery selects by ownership, not by
/// `stopped_at`, so that work must still be reconciled. This is the bug the
/// review caught; the test is what stops it coming back.
#[tokio::test]
async fn an_escalated_shutdowns_operations_are_not_stranded() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();

    let old = storage.register_runtime_instance("0.1.0-old").await.unwrap();
    seed_operation(&storage, "op-abandoned", &old, "Running").await;
    storage.stop_runtime_instance(&old, StopKind::Escalated).await.unwrap();

    let new = storage.register_runtime_instance("0.1.0-new").await.unwrap();
    let report = storage.reconcile_orphans(&new).await.unwrap();

    assert_eq!(report.interrupted, vec!["op-abandoned".to_string()]);
    let status: String = sqlx::query_scalar("SELECT status_kind FROM operation WHERE id='op-abandoned'")
        .fetch_one(storage.reader()).await.unwrap();
    assert_eq!(status, "Interrupted");
}

/// Spec §8.5: `Graceful` is a claim only written when true. §8.6: finding one
/// that owns a non-terminal Operation is a defect, reconciled and reported,
/// never stranded and never passed over silently.
#[tokio::test]
async fn a_graceful_runtime_owning_unfinished_work_is_reported_as_an_anomaly() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();

    let old = storage.register_runtime_instance("0.1.0-old").await.unwrap();
    seed_operation(&storage, "op-leaked", &old, "Running").await;
    storage.stop_runtime_instance(&old, StopKind::Graceful).await.unwrap();

    let new = storage.register_runtime_instance("0.1.0-new").await.unwrap();
    let report = storage.reconcile_orphans(&new).await.unwrap();

    assert_eq!(report.interrupted, vec!["op-leaked".to_string()]);
    assert_eq!(report.anomalies, vec!["op-leaked".to_string()]);
}

/// The current runtime's own live work is never reconciled out from under it.
#[tokio::test]
async fn the_current_runtimes_own_operations_are_left_alone() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();

    let me = storage.register_runtime_instance("0.1.0").await.unwrap();
    seed_operation(&storage, "op-mine", &me, "Running").await;

    let report = storage.reconcile_orphans(&me).await.unwrap();
    assert!(report.interrupted.is_empty());
    let status: String = sqlx::query_scalar("SELECT status_kind FROM operation WHERE id='op-mine'")
        .fetch_one(storage.reader()).await.unwrap();
    assert_eq!(status, "Running");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test recovery`
Expected: FAIL — `register_runtime_instance`, `stop_runtime_instance`, and `reconcile_orphans` do not exist.

- [ ] **Step 3: Write `src/storage/sqlite/runtime.rs`**

```rust
use sqlx::SqliteConnection;

use crate::events::{Actor, DurableEvent};
use super::{Storage, StorageError, events::append_event};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopKind {
    Graceful,
    Escalated,
}

impl StopKind {
    fn as_str(self) -> &'static str {
        match self {
            StopKind::Graceful => "Graceful",
            StopKind::Escalated => "Escalated",
        }
    }
}

#[derive(Debug, Default)]
pub struct ReconcileReport {
    pub interrupted: Vec<String>,
    /// Operations found under a `Graceful` runtime. Spec §8.5 says that cannot
    /// happen; if it does, §8.6 requires it be reported, not silently handled.
    pub anomalies: Vec<String>,
}

fn now() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .expect("RFC3339 formatting cannot fail")
}

impl Storage {
    pub async fn register_runtime_instance(&self, version: &str) -> Result<String, StorageError> {
        let id = uuid::Uuid::new_v4().to_string();
        let ts = now();
        let (id2, version, ts2) = (id.clone(), version.to_string(), ts.clone());
        self.write_txn(move |conn| {
            Box::pin(async move {
                sqlx::query(
                    "INSERT INTO runtime_instance (id, version, started_at) VALUES (?,?,?)",
                )
                .bind(&id2).bind(&version).bind(&ts2)
                .execute(&mut *conn).await?;
                append_event(
                    conn,
                    &DurableEvent::new("RuntimeStarted", Actor::system())
                        .with_payload(serde_json::json!({ "runtime_instance_id": id2 })),
                    &ts2,
                ).await?;
                Ok(())
            })
        })
        .await?;
        Ok(id)
    }

    pub async fn stop_runtime_instance(
        &self,
        id: &str,
        kind: StopKind,
    ) -> Result<(), StorageError> {
        let (id, ts) = (id.to_string(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                let affected = sqlx::query(
                    "UPDATE runtime_instance SET stopped_at = ?, stop_kind = ?
                     WHERE id = ? AND stopped_at IS NULL",
                )
                .bind(&ts).bind(kind.as_str()).bind(&id)
                .execute(&mut *conn).await?
                .rows_affected();
                if affected == 0 {
                    return Err(StorageError::TransitionConflict {
                        expected: "runtime with stopped_at IS NULL".into(),
                        found: "already stopped or missing".into(),
                    });
                }
                append_event(
                    conn,
                    &DurableEvent::new("RuntimeStopped", Actor::system())
                        .with_payload(serde_json::json!({
                            "runtime_instance_id": id, "stop_kind": kind.as_str()
                        })),
                    &ts,
                ).await?;
                Ok(())
            })
        })
        .await
    }

    /// Spec §8.6. Selects on **ownership alone**: every non-terminal Operation
    /// whose owning runtime is not the current one, regardless of how that
    /// runtime ended. Filtering on `stopped_at IS NULL` would permanently
    /// strand everything an Escalated shutdown left behind.
    pub async fn reconcile_orphans(
        &self,
        current: &str,
    ) -> Result<ReconcileReport, StorageError> {
        let (current, ts) = (current.to_string(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                let rows: Vec<(String, String, Option<String>)> = sqlx::query_as(
                    "SELECT o.id, o.status_kind, r.stop_kind
                       FROM operation o
                       JOIN runtime_instance r ON r.id = o.runtime_instance_id
                      WHERE o.status_kind IN ('Pending','Running')
                        AND o.runtime_instance_id <> ?
                      ORDER BY o.id",
                )
                .bind(&current)
                .fetch_all(&mut *conn)
                .await?;

                let mut report = ReconcileReport::default();
                for (op_id, status, stop_kind) in rows {
                    let reason = match status.as_str() {
                        "Pending" => "PreviousRuntimeEndedBeforeStart",
                        _ => "PreviousRuntimeEndedDuringRun",
                    };
                    // Exact CAS on id, expected status, and previous runtime.
                    let affected = sqlx::query(
                        "UPDATE operation
                            SET status_kind = 'Interrupted',
                                interrupt_reason = ?,
                                finished_at = ?
                          WHERE id = ? AND status_kind = ? AND runtime_instance_id <> ?",
                    )
                    .bind(reason).bind(&ts).bind(&op_id).bind(&status).bind(&current)
                    .execute(&mut *conn).await?
                    .rows_affected();
                    if affected == 0 {
                        continue;
                    }
                    append_event(
                        conn,
                        &DurableEvent::new("OperationInterrupted", Actor::system())
                            .with_operation(&op_id)
                            .with_payload(serde_json::json!({ "reason": reason })),
                        &ts,
                    ).await?;
                    report.interrupted.push(op_id.clone());
                    if stop_kind.as_deref() == Some("Graceful") {
                        report.anomalies.push(op_id);
                    }
                }
                Ok(report)
            })
        })
        .await
    }
}
```

- [ ] **Step 4: Re-export from `src/storage/mod.rs`**

```rust
mod sqlite;

pub use sqlite::{ReconcileReport, StopKind, Storage, StorageError};
```

- [ ] **Step 5: Write `src/runtime/mod.rs`**

```rust
use std::sync::Arc;

use crate::storage::{ReconcileReport, StopKind, Storage, StorageError};

/// Owns this process's runtime identity for its whole lifetime. Spec §8.1:
/// startup performs migration, ownership, containment setup, and recovery
/// before any work is accepted.
pub struct Runtime {
    pub instance_id: String,
    pub storage: Arc<Storage>,
}

impl Runtime {
    pub async fn start(storage: Arc<Storage>) -> Result<(Self, ReconcileReport), StorageError> {
        let version = env!("CARGO_PKG_VERSION");
        let instance_id = storage.register_runtime_instance(version).await?;
        let report = storage.reconcile_orphans(&instance_id).await?;

        for op in &report.interrupted {
            tracing::info!(operation_id = %op, "recovery.reconcile");
        }
        for op in &report.anomalies {
            tracing::error!(
                operation_id = %op,
                "recovery.anomaly: a Graceful runtime owned a non-terminal operation"
            );
        }
        Ok((Self { instance_id, storage }, report))
    }

    pub async fn stop(&self, kind: StopKind) -> Result<(), StorageError> {
        self.storage.stop_runtime_instance(&self.instance_id, kind).await
    }
}
```

Add `pub mod runtime;` to `src/lib.rs`.

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test recovery`
Expected: PASS, all four tests.

- [ ] **Step 7: Commit**

```bash
git add src/runtime src/storage src/lib.rs tests/recovery.rs
git commit -m "feat(runtime): register the instance and reconcile orphans by ownership"
```

---

