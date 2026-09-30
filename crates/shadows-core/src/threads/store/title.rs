//! One job: replacing a thread's title after its creation (spec §4.2) — by
//! its first message, then by the harness. Each source replaces only the
//! sources below it; a `person` title is replaced by neither.
//!
//! Neither write is a person's command, so neither records a `CommandId`: the
//! first message's rides the turn's own write, and the harness's is its own
//! write, as a turn's agent entries are.

use sqlx::SqliteConnection;

use crate::db::{Storage, StorageError, append_event, now};
use crate::events::{Actor, DurableEvent};
use crate::projects::ProjectId;
use crate::threads::model::{ThreadEntryId, ThreadId};
use crate::threads::title;

/// Inside the turn's write, after `entry` (its user message) was appended:
/// titles the thread after the message when it is the thread's first and the
/// title is still the client's. It happens once: the source it writes is no
/// longer `client`, and a later message is not the first. A fork holds its
/// source's messages, so no message of a fork is its first.
pub(crate) async fn title_from_first_message_in(
    conn: &mut SqliteConnection,
    thread: &ThreadId,
    entry: &ThreadEntryId,
    message: &str,
    actor: Actor,
    ts: &str,
) -> Result<(), StorageError> {
    let Some(title) = title::from_first_message(message) else {
        return Ok(());
    };
    let project: Option<String> = sqlx::query_scalar(
        "UPDATE planning_thread SET title = ?, title_source = 'first_message'
          WHERE id = ? AND title_source = 'client'
            AND NOT EXISTS (SELECT 1 FROM thread_entry
                             WHERE thread_id = planning_thread.id
                               AND kind = 'UserMessage' AND id <> ?)
      RETURNING project_id",
    )
    .bind(&title)
    .bind(thread.as_str())
    .bind(entry.as_str())
    .fetch_optional(&mut *conn)
    .await?;
    if let Some(project) = project {
        retitled(conn, &project, thread, &title, "first_message", actor, ts).await?;
    }
    Ok(())
}

impl Storage {
    /// The harness named its session (§12.3): the thread takes the title,
    /// sanitized, unless a person named it. Its own write; answers whether
    /// the title changed. An empty title, or the one the thread already has,
    /// writes nothing.
    pub async fn title_from_harness(
        &self,
        thread: &ThreadId,
        title: &str,
    ) -> Result<bool, StorageError> {
        let Some(title) = title::from_harness(title) else {
            return Ok(false);
        };
        let (thread, ts) = (thread.clone(), now());
        self.write_txn(move |conn| {
            Box::pin(async move {
                // `harness` is replaced too: the adapter may send the raw
                // first prompt when it cannot generate a title yet, and the
                // title it generates at a later turn must still land.
                let project: Option<String> = sqlx::query_scalar(
                    "UPDATE planning_thread SET title = ?, title_source = 'harness'
                      WHERE id = ? AND title <> ?
                        AND title_source IN ('client', 'first_message', 'harness')
                  RETURNING project_id",
                )
                .bind(&title)
                .bind(thread.as_str())
                .bind(&title)
                .fetch_optional(&mut *conn)
                .await?;
                let Some(project) = project else {
                    return Ok(false);
                };
                retitled(
                    conn,
                    &project,
                    &thread,
                    &title,
                    "harness",
                    Actor::system(),
                    &ts,
                )
                .await?;
                Ok(true)
            })
        })
        .await
    }
}

async fn retitled(
    conn: &mut SqliteConnection,
    project: &str,
    thread: &ThreadId,
    title: &str,
    source: &str,
    actor: Actor,
    ts: &str,
) -> Result<(), StorageError> {
    append_event(
        conn,
        &DurableEvent::new("ThreadRetitled", actor)
            .with_project(&ProjectId::from_stored(project.to_string()))
            .with_thread(thread)
            .with_payload(serde_json::json!({ "title": title, "source": source })),
        ts,
    )
    .await?;
    Ok(())
}
