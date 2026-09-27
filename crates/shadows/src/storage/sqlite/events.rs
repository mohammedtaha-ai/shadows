use sqlx::SqliteConnection;

use super::StorageError;
use crate::events::DurableEvent;

/// Private on purpose. Cross-cutting rule 10 forbids a public raw
/// `append_event`: an event is appended only inside a capability that also
/// writes the state it describes.
///
/// Visibility is `pub(in crate::storage)` rather than `pub(super)`: besides
/// its Task 4-9 callers in sibling modules under `storage::sqlite` (which
/// `pub(super)` alone would reach), `storage::test_support` — a sibling of
/// `sqlite` itself, not a descendant of it — also needs to call this for the
/// atomicity contract test. `pub(in crate::storage)` is the narrowest
/// visibility that reaches both without making the function `pub`.
pub(in crate::storage) async fn append_event(
    conn: &mut SqliteConnection,
    event: &DurableEvent,
    now: &str,
) -> Result<i64, StorageError> {
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
    // Converted here rather than by deriving `sqlx::Type` on the id newtypes:
    // CLAUDE.md keeps domain types free of persistence imports, so the
    // domain-to-column step belongs at this boundary and nowhere else.
    .bind(event.project_id.as_ref().map(|i| i.as_str()))
    .bind(event.thread_id.as_ref().map(|i| i.as_str()))
    .bind(event.operation_id.as_ref().map(|i| i.as_str()))
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
