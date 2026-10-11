//! Persisted vision versions (§23.3).
use crate::{ProjectId, StorageError, VisionContent};
use sqlx::SqliteConnection;

pub(super) async fn insert_version(
    conn: &mut SqliteConnection,
    project: &ProjectId,
    number: i64,
    content: &VisionContent,
    proposal: Option<&str>,
    ts: &str,
) -> Result<(), StorageError> {
    sqlx::query(
        "INSERT INTO design_vision_version
        (project_id,number,content_json,proposal_id,created_at) VALUES (?,?,?,?,?)",
    )
    .bind(project.as_str())
    .bind(number)
    .bind(serde_json::to_string(content)?)
    .bind(proposal)
    .bind(ts)
    .execute(conn)
    .await?;
    Ok(())
}
