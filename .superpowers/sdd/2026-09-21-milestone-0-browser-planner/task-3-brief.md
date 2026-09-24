## Task 3: Serialized write transactions and atomic state + event

**Files:**
- Modify: `src/storage/sqlite/mod.rs`
- Create: `src/storage/sqlite/events.rs`, `src/events/mod.rs`
- Modify: `src/lib.rs` (add `pub mod events;`)
- Test: `tests/storage_contract.rs`

**Interfaces:**
- Consumes: `Storage` from Task 2.
- Produces: `Storage::write_txn<F, T>(&self, f: F) -> Result<T, StorageError>` where `F: for<'a> FnOnce(&'a mut SqliteConnection) -> BoxFuture<'a, Result<T, StorageError>>`; `events::DurableEvent { event_id, kind, project_id, thread_id, operation_id, actor, causation, correlation_id, payload_json }`; `events::EventCursor(i64)`; private `append_event(conn, &DurableEvent, now) -> Result<i64, StorageError>`.

- [ ] **Step 1: Write the failing tests**

Append to `tests/storage_contract.rs`:

```rust
use shadows::events::{Actor, DurableEvent};

/// Spec §2.4 and cross-cutting rule 5: state and event commit together or not
/// at all. A live publication failure must never roll back committed truth, and
/// a rolled-back transaction must leave no event behind.
#[tokio::test]
async fn state_and_event_commit_atomically_or_not_at_all() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();

    // A transaction that fails after appending its event leaves nothing behind.
    let outcome = storage
        .write_txn(|conn| {
            Box::pin(async move {
                sqlx::query(
                    "INSERT INTO project (id, slug, name, created_at) VALUES (?,?,?,?)",
                )
                .bind("p-1").bind("demo").bind("Demo").bind("2026-09-21T00:00:00Z")
                .execute(&mut *conn)
                .await?;
                shadows::storage::test_support::append_event_for_test(
                    conn,
                    &DurableEvent::new("ProjectCreated", Actor::system())
                        .with_project("p-1")
                        .with_payload(serde_json::json!({})),
                    "2026-09-21T00:00:00Z",
                )
                .await?;
                Err::<(), _>(shadows::storage::StorageError::NotFound("forced"))
            })
        })
        .await;
    assert!(outcome.is_err());

    let projects: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project")
        .fetch_one(storage.reader()).await.unwrap();
    let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM durable_event")
        .fetch_one(storage.reader()).await.unwrap();
    assert_eq!((projects, events), (0, 0), "rollback must leave neither behind");
}

/// The measured writer policy, encoded as a regression test. Concurrent
/// read-then-write transactions must all succeed. If someone later replaces the
/// serialized write connection with a pool, this test is what fails.
#[tokio::test]
async fn concurrent_read_then_write_transactions_all_succeed() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = std::sync::Arc::new(
        Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap(),
    );

    let mut handles = Vec::new();
    for w in 0..16 {
        let storage = storage.clone();
        handles.push(tokio::spawn(async move {
            for i in 0..25 {
                let id = format!("p-{w}-{i}");
                storage
                    .write_txn(|conn| {
                        let id = id.clone();
                        Box::pin(async move {
                            // Read first, then write: this is the shape that
                            // forces a lock upgrade under deferred BEGIN.
                            let _: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project")
                                .fetch_one(&mut *conn).await?;
                            sqlx::query(
                                "INSERT INTO project (id, slug, name, created_at) VALUES (?,?,?,?)",
                            )
                            .bind(&id).bind(&id).bind("x").bind("2026-09-21T00:00:00Z")
                            .execute(&mut *conn).await?;
                            Ok(())
                        })
                    })
                    .await
                    .expect("no write transaction may fail under the serialized policy");
            }
        }));
    }
    for h in handles { h.await.unwrap(); }

    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project")
        .fetch_one(storage.reader()).await.unwrap();
    assert_eq!(count, 400);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test storage_contract`
Expected: FAIL — `write_txn`, `events`, and `test_support` do not exist.

- [ ] **Step 3: Write `src/events/mod.rs`**

```rust
/// Spec §6.18. `seq` is assigned by the INSERT, which on SQLite can only run
/// while holding the write lock, so assignment order equals commit order. That
/// property is SQLite-specific — see the OPEN block in §6.18 before writing
/// backend-neutral cursor code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct EventCursor(pub i64);

#[derive(Debug, Clone)]
pub struct Actor {
    pub kind: String,
    pub id: String,
}

impl Actor {
    pub fn system() -> Self {
        Self { kind: "System".into(), id: "daemon".into() }
    }
    pub fn user(id: impl Into<String>) -> Self {
        Self { kind: "User".into(), id: id.into() }
    }
}

#[derive(Debug, Clone)]
pub struct DurableEvent {
    pub event_id: String,
    pub kind: String,
    pub project_id: Option<String>,
    pub thread_id: Option<String>,
    pub operation_id: Option<String>,
    pub actor: Actor,
    pub correlation_id: Option<String>,
    pub payload_json: String,
}

impl DurableEvent {
    pub fn new(kind: impl Into<String>, actor: Actor) -> Self {
        Self {
            event_id: uuid::Uuid::new_v4().to_string(),
            kind: kind.into(),
            project_id: None,
            thread_id: None,
            operation_id: None,
            actor,
            correlation_id: None,
            payload_json: "{}".into(),
        }
    }
    pub fn with_project(mut self, id: impl Into<String>) -> Self {
        self.project_id = Some(id.into());
        self
    }
    pub fn with_thread(mut self, id: impl Into<String>) -> Self {
        self.thread_id = Some(id.into());
        self
    }
    pub fn with_operation(mut self, id: impl Into<String>) -> Self {
        self.operation_id = Some(id.into());
        self
    }
    pub fn with_payload(mut self, v: serde_json::Value) -> Self {
        self.payload_json = v.to_string();
        self
    }
}
```

- [ ] **Step 4: Write `src/storage/sqlite/events.rs`**

```rust
use sqlx::SqliteConnection;

use crate::events::DurableEvent;
use super::StorageError;

/// Private on purpose. Cross-cutting rule 10 forbids a public raw
/// `append_event`: an event is appended only inside a capability that also
/// writes the state it describes.
pub(super) async fn append_event(
    conn: &mut SqliteConnection,
    event: &DurableEvent,
    now: &str,
) -> Result<i64, StorageError> {
    let seq: i64 = sqlx::query_scalar(
        "INSERT INTO durable_event
           (event_id, kind, project_id, thread_id, operation_id,
            actor_kind, actor_id, payload_json, created_at)
         VALUES (?,?,?,?,?,?,?,?,?)
         RETURNING seq",
    )
    .bind(&event.event_id)
    .bind(&event.kind)
    .bind(&event.project_id)
    .bind(&event.thread_id)
    .bind(&event.operation_id)
    .bind(&event.actor.kind)
    .bind(&event.actor.id)
    .bind(&event.payload_json)
    .bind(now)
    .fetch_one(&mut *conn)
    .await?;
    Ok(seq)
}
```

- [ ] **Step 5: Add `write_txn` to `src/storage/sqlite/mod.rs`**

```rust
use futures_core::future::BoxFuture;

impl Storage {
    /// Every write goes through here. `BEGIN IMMEDIATE` takes the write lock up
    /// front so no transaction has to upgrade mid-flight; the mutex is what
    /// removes contention entirely. Both are required — see §6.23.
    pub async fn write_txn<F, T>(&self, f: F) -> Result<T, StorageError>
    where
        F: for<'a> FnOnce(&'a mut SqliteConnection) -> BoxFuture<'a, Result<T, StorageError>>,
    {
        use sqlx::Executor;
        let mut conn = self.write.lock().await;
        conn.execute("BEGIN IMMEDIATE").await?;
        match f(&mut conn).await {
            Ok(v) => {
                conn.execute("COMMIT").await?;
                Ok(v)
            }
            Err(e) => {
                let _ = conn.execute("ROLLBACK").await;
                Err(e)
            }
        }
    }
}
```

Add `futures-core = "0.3"` to `[dependencies]`.

- [ ] **Step 6: Expose a test-only hook**

In `src/storage/mod.rs`:

```rust
/// Test-only access to a private capability. Not compiled into the library for
/// consumers, and not a public API.
#[doc(hidden)]
pub mod test_support {
    use sqlx::SqliteConnection;
    use crate::events::DurableEvent;
    use super::StorageError;

    pub async fn append_event_for_test(
        conn: &mut SqliteConnection,
        event: &DurableEvent,
        now: &str,
    ) -> Result<i64, StorageError> {
        super::sqlite::events::append_event(conn, event, now).await
    }
}
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cargo test --test storage_contract`
Expected: PASS, both tests.

- [ ] **Step 8: Commit**

```bash
git add src/events src/storage src/lib.rs Cargo.toml tests/storage_contract.rs
git commit -m "feat(storage): serialized write transactions with atomic state and event"
```

---

