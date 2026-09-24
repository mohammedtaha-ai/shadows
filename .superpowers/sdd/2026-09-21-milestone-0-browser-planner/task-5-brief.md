## Task 5: Local-directory Project with external command idempotency

**Files:**
- Create: `src/command/mod.rs`, `src/project/mod.rs`, `src/storage/sqlite/project.rs`
- Modify: `src/lib.rs`, `src/storage/mod.rs`, `Cargo.toml` (add `sha2`)
- Test: `tests/project_contract.rs` — **not** `tests/storage_contract.rs`

**Why a new test file.** `tests/storage_contract.rs` is a named accretion point in
CLAUDE.md, and it crossed 300 lines during Task 3 holding one responsibility: the
connection and transaction contracts — pool policy, write serialization, atomicity,
recovery, provenance. Project identity and command idempotency are a different
responsibility, and appending them here is how that file reaches 500 lines by Task 6.
The split is by domain, which is what CLAUDE.md prescribes for this file. Name its one
job without using "and": the project capability's durable contract.

**Interfaces:**
- Consumes: `Storage::write_txn`, `append_event` from Task 3.
- Produces: `command::CommandContext { principal_kind, principal_id, command_id, command_kind, command_schema_ver, request_fingerprint }`; `command::fingerprint(kind: &str, params: &serde_json::Value) -> String`; `project::Project { id, slug, name, created_at }`; `Storage::create_project(ctx: &CommandContext, slug: &str, name: &str) -> Result<Project, StorageError>`; `Storage::list_projects() -> Result<Vec<Project>, StorageError>`; `pub(super) classify(...)` and `pub(super) record_command(...)` for reuse by later tasks.

- [ ] **Step 1: Write the failing tests**

Append to `tests/storage_contract.rs`:

```rust
use shadows::command::{fingerprint, CommandContext};

fn ctx(command_id: &str, params: &serde_json::Value) -> CommandContext {
    ctx_kind(command_id, "project.create", params)
}

fn ctx_kind(command_id: &str, kind: &str, params: &serde_json::Value) -> CommandContext {
    CommandContext {
        principal_kind: "User".into(),
        principal_id: "local".into(),
        command_id: command_id.into(),
        command_kind: kind.into(),
        command_schema_ver: 1,
        request_fingerprint: fingerprint(kind, params),
    }
}

/// Spec section 5.2 and cross-cutting rule 6: an external mutation writes its
/// CommandRecord in the same transaction. Replaying the same command id with
/// the same request returns the stored outcome and creates nothing new.
#[tokio::test]
async fn replaying_an_identical_command_returns_the_stored_outcome() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();
    let params = serde_json::json!({ "slug": "demo", "name": "Demo" });

    let first = storage.create_project(&ctx("cmd-1", &params), "demo", "Demo").await.unwrap();
    let second = storage.create_project(&ctx("cmd-1", &params), "demo", "Demo").await.unwrap();

    assert_eq!(first.id, second.id, "replay must return the same entity");

    let projects: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project")
        .fetch_one(storage.reader()).await.unwrap();
    assert_eq!(projects, 1, "replay must not create a second project");

    let events: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM durable_event")
        .fetch_one(storage.reader()).await.unwrap();
    assert_eq!(events, 1, "replay must not append a second event");
}

/// Replay requires fingerprint equality. The same command id with a different
/// request is CommandConflict and mutates nothing. This is the arm a future
/// contributor most wants to weaken; it stays refused.
#[tokio::test]
async fn the_same_command_id_with_a_different_request_is_a_conflict() {
    let tmp = tempfile::tempdir().unwrap();
    let storage = Storage::open(&tmp.path().join("s.sqlite3")).await.unwrap();

    let first_params = serde_json::json!({ "slug": "demo", "name": "Demo" });
    storage.create_project(&ctx("cmd-1", &first_params), "demo", "Demo").await.unwrap();

    let other_params = serde_json::json!({ "slug": "other", "name": "Other" });
    let err = storage
        .create_project(&ctx("cmd-1", &other_params), "other", "Other")
        .await
        .expect_err("a reused command id with a different request must be refused");
    assert!(matches!(err, shadows::storage::StorageError::CommandConflict));

    let projects: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project")
        .fetch_one(storage.reader()).await.unwrap();
    assert_eq!(projects, 1, "a conflict must mutate nothing");
}

/// Key order carries no meaning, so reordering JSON keys must not turn a
/// replay into a conflict. A different command kind must not collide.
#[test]
fn the_fingerprint_ignores_key_order_but_not_command_kind() {
    let a = serde_json::json!({ "slug": "demo", "name": "Demo" });
    let b = serde_json::json!({ "name": "Demo", "slug": "demo" });
    assert_eq!(fingerprint("project.create", &a), fingerprint("project.create", &b));
    assert_ne!(fingerprint("project.create", &a), fingerprint("thread.create", &a));
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test storage_contract`
Expected: FAIL — `shadows::command` and `create_project` do not exist.

- [ ] **Step 3: Write `src/command/mod.rs`**

```rust
use sha2::{Digest, Sha256};

/// Spec section 5.2, external write origin: everything needed to decide whether
/// a submission is new, a replay, or a conflict.
#[derive(Debug, Clone)]
pub struct CommandContext {
    pub principal_kind: String,
    pub principal_id: String,
    pub command_id: String,
    pub command_kind: String,
    pub command_schema_ver: i64,
    pub request_fingerprint: String,
}

/// Canonicalise before hashing, so that key order — which carries no meaning —
/// cannot turn a replay into a conflict. The command kind is mixed in so the
/// same params under a different command are not interchangeable.
pub fn fingerprint(command_kind: &str, params: &serde_json::Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(command_kind.as_bytes());
    hasher.update(b"\0");
    hasher.update(canonical(params).as_bytes());
    format!("{:x}", hasher.finalize())
}

fn canonical(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let inner: Vec<String> =
                keys.iter().map(|k| format!("{}:{}", k, canonical(&map[*k]))).collect();
            format!("{{{}}}", inner.join(","))
        }
        serde_json::Value::Array(items) => {
            let inner: Vec<String> = items.iter().map(canonical).collect();
            format!("[{}]", inner.join(","))
        }
        other => other.to_string(),
    }
}
```

Add `sha2 = "0.10"` to `[dependencies]`.

- [ ] **Step 4: Write `src/project/mod.rs`**

```rust
/// Spec section 11.1 requires selecting a local directory without a path
/// becoming the project's identity. The id is a UUID and the slug is the
/// stable human key; the directory is configuration, not identity.
#[derive(Debug, Clone, serde::Serialize)]
pub struct Project {
    pub id: String,
    pub slug: String,
    pub name: String,
    pub created_at: String,
}
```

- [ ] **Step 5: Write `src/storage/sqlite/project.rs`**

```rust
use sqlx::SqliteConnection;

use crate::command::CommandContext;
use crate::events::{Actor, DurableEvent};
use crate::project::Project;
use super::{events::append_event, now, Storage, StorageError};

/// `Some(outcome_ref)` when this exact command was already recorded, `None`
/// when it is new, `Err(CommandConflict)` when the id was reused with a
/// different request. Spec section 5.2.
pub(super) async fn classify(
    conn: &mut SqliteConnection,
    ctx: &CommandContext,
    scope_kind: &str,
    scope_key: &str,
) -> Result<Option<String>, StorageError> {
    let existing: Option<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT command_kind, request_fingerprint, outcome_ref FROM command_record
          WHERE principal_kind = ? AND principal_id = ?
            AND command_scope_kind = ? AND command_scope_key = ?
            AND command_id = ?",
    )
    .bind(&ctx.principal_kind).bind(&ctx.principal_id)
    .bind(scope_kind).bind(scope_key).bind(&ctx.command_id)
    .fetch_optional(&mut *conn)
    .await?;

    match existing {
        None => Ok(None),
        Some((kind, fp, outcome_ref)) => {
            if kind == ctx.command_kind && fp == ctx.request_fingerprint {
                Ok(Some(outcome_ref.ok_or(StorageError::NotFound("outcome_ref"))?))
            } else {
                Err(StorageError::CommandConflict)
            }
        }
    }
}

pub(super) async fn record_command(
    conn: &mut SqliteConnection,
    ctx: &CommandContext,
    scope_kind: &str,
    scope_key: &str,
    entity_kind: &str,
    outcome_ref: &str,
    ts: &str,
) -> Result<(), StorageError> {
    sqlx::query(
        "INSERT INTO command_record
           (principal_kind, principal_id, command_scope_kind, command_scope_key,
            command_id, command_kind, command_schema_ver, request_fingerprint,
            outcome_kind, entity_kind, outcome_ref, recorded_at)
         VALUES (?,?,?,?,?,?,?,?,'Entity',?,?,?)",
    )
    .bind(&ctx.principal_kind).bind(&ctx.principal_id)
    .bind(scope_kind).bind(scope_key)
    .bind(&ctx.command_id).bind(&ctx.command_kind).bind(ctx.command_schema_ver)
    .bind(&ctx.request_fingerprint)
    .bind(entity_kind).bind(outcome_ref).bind(ts)
    .execute(&mut *conn)
    .await?;
    Ok(())
}

impl Storage {
    pub async fn create_project(
        &self,
        ctx: &CommandContext,
        slug: &str,
        name: &str,
    ) -> Result<Project, StorageError> {
        let (ctx, slug, name, ts) = (ctx.clone(), slug.to_string(), name.to_string(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                if let Some(existing_id) = classify(conn, &ctx, "Global", "").await? {
                    return load_project(conn, &existing_id).await;
                }
                let id = uuid::Uuid::new_v4().to_string();
                sqlx::query("INSERT INTO project (id, slug, name, created_at) VALUES (?,?,?,?)")
                    .bind(&id).bind(&slug).bind(&name).bind(&ts)
                    .execute(&mut *conn).await?;

                append_event(
                    conn,
                    &DurableEvent::new("ProjectCreated", Actor::user(&ctx.principal_id))
                        .with_project(&id)
                        .with_payload(serde_json::json!({ "slug": slug, "name": name })),
                    &ts,
                ).await?;

                record_command(conn, &ctx, "Global", "", "Project", &id, &ts).await?;
                load_project(conn, &id).await
            })
        })
        .await
    }

    pub async fn list_projects(&self) -> Result<Vec<Project>, StorageError> {
        let rows: Vec<(String, String, String, String)> = sqlx::query_as(
            "SELECT id, slug, name, created_at FROM project ORDER BY created_at, id",
        )
        .fetch_all(self.reader())
        .await?;
        Ok(rows.into_iter()
            .map(|(id, slug, name, created_at)| Project { id, slug, name, created_at })
            .collect())
    }
}

async fn load_project(conn: &mut SqliteConnection, id: &str) -> Result<Project, StorageError> {
    let row: (String, String, String, String) =
        sqlx::query_as("SELECT id, slug, name, created_at FROM project WHERE id = ?")
            .bind(id)
            .fetch_optional(&mut *conn)
            .await?
            .ok_or(StorageError::NotFound("project"))?;
    Ok(Project { id: row.0, slug: row.1, name: row.2, created_at: row.3 })
}
```

Move `now()` out of `runtime.rs` into `src/storage/sqlite/mod.rs` as `pub(super) fn now() -> String` so every module shares one implementation.

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test --test storage_contract`
Expected: PASS, all five tests.

- [ ] **Step 7: Commit**

```bash
git add src/command src/project src/storage src/lib.rs Cargo.toml tests/storage_contract.rs
git commit -m "feat(project): create a local project under external command idempotency"
```

---

