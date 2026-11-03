//! Persistence of a Planner conversation's selected design item.
use super::DesignStore;
use crate::{Design, DesignAnchor, DesignError, OutcomeId, PartId, ThreadId};
use core_db::{CommandContext, Storage, StorageError};
use core_threads::Threads;

impl DesignStore {
    pub async fn set_thread_anchor(
        &self,
        command_id: String,
        thread: &ThreadId,
        anchor: Option<DesignAnchor>,
    ) -> Result<Option<DesignAnchor>, DesignError> {
        let thread = thread.clone();
        let command = CommandContext::user(
            command_id,
            "thread.anchor",
            serde_json::json!({ "thread": thread, "anchor": anchor }),
        );
        self.storage
            .write_txn_as(move |conn| {
                Box::pin(async move {
                    if let Some(answer) =
                        CommandContext::classify(conn, &command, "Thread", thread.as_str()).await?
                    {
                        return Ok(serde_json::from_str(&answer)?);
                    }
                    let context = Threads::turn_context_in(conn, &thread).await?;
                    if context.harness == "aylul" {
                        return Err(DesignError::InvalidCommand(
                            "AYLUL conversations cannot have a planning anchor".into(),
                        ));
                    }
                    let ts = Storage::now();
                    match &anchor {
                        Some(anchor) => {
                            let (kind, id) = match anchor {
                                DesignAnchor::Part(id) => {
                                    Design::part_in(conn, &context.project_id, id).await?;
                                    ("part", id.as_str())
                                }
                                DesignAnchor::Outcome(id) => {
                                    Design::design_outcome_in(conn, &context.project_id, id)
                                        .await?;
                                    ("outcome", id.as_str())
                                }
                            };
                            sqlx::query(
                                "INSERT INTO design_thread_anchor(thread_id,kind,item_id,set_at) \
                        VALUES (?,?,?,?) ON CONFLICT(thread_id) DO UPDATE SET \
                        kind=excluded.kind,item_id=excluded.item_id,set_at=excluded.set_at",
                            )
                            .bind(thread.as_str())
                            .bind(kind)
                            .bind(id)
                            .bind(&ts)
                            .execute(&mut *conn)
                            .await?;
                        }
                        None => {
                            sqlx::query("DELETE FROM design_thread_anchor WHERE thread_id=?")
                                .bind(thread.as_str())
                                .execute(&mut *conn)
                                .await?;
                        }
                    }
                    CommandContext::record_command(
                        conn,
                        &command,
                        "Thread",
                        thread.as_str(),
                        "DesignAnchor",
                        &serde_json::to_string(&anchor)?,
                        &ts,
                    )
                    .await?;
                    Ok(anchor)
                })
            })
            .await
    }

    pub async fn thread_anchor(
        &self,
        thread: &ThreadId,
    ) -> Result<Option<DesignAnchor>, StorageError> {
        let mut tx = self.storage.reader().begin().await?;
        Threads::require_live_in(&mut tx, thread).await?;
        let row: Option<(String, String)> =
            sqlx::query_as("SELECT kind,item_id FROM design_thread_anchor WHERE thread_id=?")
                .bind(thread.as_str())
                .fetch_optional(&mut *tx)
                .await?;
        Ok(row.map(|(kind, id)| match kind.as_str() {
            "part" => DesignAnchor::Part(PartId::from_recorded(id)),
            _ => DesignAnchor::Outcome(OutcomeId::from_recorded(id)),
        }))
    }
}
