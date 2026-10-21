//! One job: appending to the durable journal (spec §6.18). Reading it back is
//! `Events`' (`events/store.rs`); appending is the writer's act, inside the
//! transaction that writes the state the event describes (§14.4).

use sqlx::SqliteConnection;

use super::StorageError;

#[derive(Debug, Clone, serde::Serialize, utoipa::ToSchema)]
pub struct Actor {
    pub kind: String,
    pub id: String,
}

impl Actor {
    pub fn system() -> Self {
        Self {
            kind: "System".into(),
            id: "daemon".into(),
        }
    }
    pub fn user(id: impl Into<String>) -> Self {
        Self {
            kind: "User".into(),
            id: id.into(),
        }
    }
}

/// `durable_event` carries `causation_kind` and `causation_ref` as a pair:
/// its own `CHECK ((causation_kind IS NULL) = (causation_ref IS NULL))`
/// requires both or neither. Modeling them as one `Option<Causation>` here
/// rather than two independent `Option<String>` fields makes the invalid
/// state (kind without reference, or reference without kind) unrepresentable
/// in Rust, instead of merely unrepresentable in the schema.
#[derive(Debug, Clone)]
pub struct Causation {
    pub kind: String,
    pub reference: String,
}

/// What the journal stores for one event. Subject ids are plain strings: the
/// service that owns an id converts it, so the journal names no service type.
#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub event_id: String,
    pub kind: String,
    pub project_id: Option<String>,
    pub thread_id: Option<String>,
    pub operation_id: Option<String>,
    pub actor: Actor,
    pub causation: Option<Causation>,
    pub correlation_id: Option<String>,
    pub payload_json: String,
}

/// Private on purpose. Cross-cutting rule 10 forbids a public raw
/// `append_event`: an event is appended only inside a capability that also
/// writes the state it describes.
///
/// Visibility is `pub(crate)`, never `pub`: every service's store calls it
/// inside its own writes (spec §14.6), and `append_event_for_test` (the
/// atomicity contract test) below.
pub(crate) async fn append_event(
    conn: &mut SqliteConnection,
    event: impl Into<JournalEntry>,
    now: &str,
) -> Result<i64, StorageError> {
    let event: JournalEntry = event.into();
    let (causation_kind, causation_ref) = match &event.causation {
        Some(c) => (Some(c.kind.as_str()), Some(c.reference.as_str())),
        None => (None, None),
    };

    let seq: i64 = sqlx::query_scalar(
        "INSERT INTO durable_event
           (event_id, kind, project_id, thread_id, operation_id,
            actor_kind, actor_id, causation_kind, causation_ref, correlation_id,
            payload_json, created_at)
         VALUES (?,?,?,?,?,?,?,?,?,?,?,?)
         RETURNING seq",
    )
    .bind(&event.event_id)
    .bind(&event.kind)
    .bind(&event.project_id)
    .bind(&event.thread_id)
    .bind(&event.operation_id)
    .bind(&event.actor.kind)
    .bind(&event.actor.id)
    .bind(causation_kind)
    .bind(causation_ref)
    .bind(&event.correlation_id)
    .bind(&event.payload_json)
    .bind(now)
    .fetch_one(&mut *conn)
    .await?;
    Ok(seq)
}

/// Test-only access to a private capability: `append_event` without a state
/// write. Compiled in only with the `test-support` feature, which a crate
/// enables only in its `[dev-dependencies]`, so this capability does not ship
/// (§14.6). Tests reach it as `shadows_core::testing::append_event_for_test`.
#[cfg(feature = "test-support")]
pub async fn append_event_for_test(
    conn: &mut SqliteConnection,
    event: impl Into<JournalEntry>,
    now: &str,
) -> Result<i64, StorageError> {
    append_event(conn, event, now).await
}
